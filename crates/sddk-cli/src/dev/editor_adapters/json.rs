//! JSON agent-map registration core shared by opencode and zcode (ADR-0019).
//! Both editors store agents in `<dir>/<editor>.json` → `agent` map with the
//! same entry schema; the core is parameterized by `IdeKey`.

use super::{AdapterReport, RegistrationContext, is_framework_namespaced, resolve_for_models};
use crate::dev::agent_models::IdeKey;
use crate::dev::common::atomic_write;
use std::path::{Path, PathBuf};

/// OpenCode registration: `opencode.json` agent map upsert + bounded prune.
pub struct OpenCodeAdapter {
    pub dir: PathBuf,
}

/// ZCode registration: `zcode.json` — mirrors the opencode schema.
pub struct ZCodeAdapter {
    pub dir: PathBuf,
}

impl super::EditorAdapter for OpenCodeAdapter {
    fn editor_name(&self) -> &'static str {
        "opencode"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        upsert_json_agents(
            &self.dir.join("opencode.json"),
            IdeKey::Opencode,
            &super::PRIMARY_AGENTS,
            ctx,
        )
    }
}

impl super::EditorAdapter for ZCodeAdapter {
    fn editor_name(&self) -> &'static str {
        "zcode"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        upsert_json_agents(
            &self.dir.join("zcode.json"),
            IdeKey::Zcode,
            &super::PRIMARY_AGENTS,
            ctx,
        )
    }
}

/// Upsert bundle agents into a JSON editor config.
///
/// Invariants (ADR-0018): first-time only (existing entries are skipped
/// byte-untouched); ConfigAbsent omits the `model` key; NoModelConfigured
/// skips the agent; pruning is bounded to framework-namespaced orphans.
pub(super) fn upsert_json_agents(
    config_path: &Path,
    ide: IdeKey,
    primary_agents: &[&str],
    ctx: &RegistrationContext<'_>,
) -> AdapterReport {
    let mut report = AdapterReport {
        editor: ide.as_str().to_owned(),
        ..AdapterReport::default()
    };
    let mut config: serde_json::Value = if config_path.exists() {
        match std::fs::read_to_string(config_path)
            .map_err(std::io::Error::other)
            .and_then(|raw| serde_json::from_str(&raw).map_err(std::io::Error::other))
        {
            Ok(value) => value,
            Err(error) => {
                report
                    .errors
                    .push(format!("{}: invalid JSON: {error}", config_path.display()));
                return report;
            }
        }
    } else {
        serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "agent": {},
            "mcp": {}
        })
    };
    let Some(agents) = config
        .get_mut("agent")
        .and_then(|value| value.as_object_mut())
    else {
        report
            .errors
            .push(format!("{}: no agent map", config_path.display()));
        return report;
    };
    let mut changed = false;
    for agent in ctx.agents {
        // The expected prompt path for this agent under the new framework root.
        let new_prompt = format!(
            "{{file:{}}}",
            ctx.root
                .join("agents")
                .join(format!("{}.md", agent.name))
                .to_string_lossy()
        );
        if let Some(existing) = agents.get_mut(&agent.name).and_then(|v| v.as_object_mut()) {
            // Entry already exists. Refresh the prompt path ONLY if the existing
            // path looks like a previous sddk install (stale), preserving any
            // user customization (model, hidden, mode, custom prompt path).
            let existing_prompt = existing
                .get("prompt")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if existing_prompt == new_prompt {
                report.skipped_existing += 1;
            } else if looks_like_framework_prompt(existing_prompt, &agent.name) {
                existing.insert("prompt".to_owned(), serde_json::Value::String(new_prompt));
                report.updated_stale += 1;
                changed = true;
            } else {
                // Path is a user-customized prompt — leave it alone.
                report.skipped_existing += 1;
            }
            continue;
        }
        match resolve_for_models(ctx.models, &agent.name, ide) {
            Ok(model) => {
                let primary = primary_agents.contains(&agent.name.as_str());
                let mut entry = serde_json::json!({
                    "description": agent.description,
                    "mode": if primary { "primary" } else { "subagent" },
                    "prompt": new_prompt,
                });
                if let Some(model) = model {
                    entry["model"] = serde_json::Value::String(model);
                }
                if !primary {
                    entry["hidden"] = serde_json::Value::Bool(true);
                }
                agents.insert(agent.name.clone(), entry);
                report.registered += 1;
                changed = true;
            }
            Err(()) => report.skipped_unresolved += 1,
        }
    }
    let bundle_names: std::collections::HashSet<&str> =
        ctx.agents.iter().map(|agent| agent.name.as_str()).collect();
    let orphans: Vec<String> = agents
        .keys()
        .filter(|name| is_framework_namespaced(name) && !bundle_names.contains(name.as_str()))
        .cloned()
        .collect();
    for orphan in orphans {
        agents.remove(&orphan);
        report.pruned += 1;
        changed = true;
    }
    if !changed {
        return report;
    }
    match serde_json::to_string_pretty(&config) {
        Ok(serialized) => {
            if let Err(error) = atomic_write(config_path, serialized.as_bytes(), None) {
                report
                    .errors
                    .push(format!("{}: {error}", config_path.display()));
            }
        }
        Err(error) => report.errors.push(format!(
            "{}: serialization failed: {error}",
            config_path.display()
        )),
    }
    report
}

/// Heuristic: detect whether an existing `{file:...}` prompt path looks like it
/// came from a previous sddk install (and is therefore stale-eligible) rather
/// than a user-customized path.
///
/// Recognised framework path shapes:
///   - `<somewhere>/sddk-framework/agents/<name>.md`   (dogfooding / bind-mount)
///   - `<somewhere>/.local/share/sddk/framework/<version-or-current>/agents/<name>.md`
///
/// Anything else (`/custom/path.md`, etc.) is treated as a user customization
/// and is preserved byte-untouched.
fn looks_like_framework_prompt(prompt: &str, expected_name: &str) -> bool {
    // Strip the `{file:...}` wrapper if present.
    let inner = prompt
        .strip_prefix("{file:")
        .and_then(|s| s.strip_suffix('}'))
        .unwrap_or(prompt);
    // Must point to an `<name>.md` file (otherwise it's clearly not the agent
    // we registered).
    let expected_suffix = format!("/agents/{expected_name}.md");
    if !inner.ends_with(&expected_suffix) {
        return false;
    }
    inner.contains("/sddk-framework/agents/")
        || inner.contains("/sddk/framework/")
}

#[cfg(test)]
#[path = "../tests/json_adapter_tests.rs"]
mod json_adapter_tests;
