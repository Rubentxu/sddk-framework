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
        if agents.contains_key(&agent.name) {
            report.skipped_existing += 1;
            continue;
        }
        match resolve_for_models(ctx.models, &agent.name, ide) {
            Ok(model) => {
                let primary = primary_agents.contains(&agent.name.as_str());
                let mut entry = serde_json::json!({
                    "description": agent.description,
                    "mode": if primary { "primary" } else { "subagent" },
                    "prompt": format!(
                        "{{file:{}}}",
                        ctx.root
                            .join("agents")
                            .join(format!("{}.md", agent.name))
                            .to_string_lossy()
                    ),
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

#[cfg(test)]
mod json_tests {
    use super::*;
    use crate::dev::editor_adapters::test_fixtures::{self, ctx};

    fn temp_config_dir() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    // I1 — idempotent double registration: second run sees everything existing.
    #[test]
    fn json_idempotent_double_register() {
        let fixture = test_fixtures::build();
        let dir = temp_config_dir();
        let path = dir.path().join("opencode.json");
        let context = ctx(&fixture, Some(&fixture.models));
        let first = upsert_json_agents(
            &path,
            IdeKey::Opencode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        assert_eq!(first.registered, 3);
        assert!(first.errors.is_empty(), "{:?}", first.errors);
        let bytes = std::fs::read(&path).unwrap();
        let second = upsert_json_agents(
            &path,
            IdeKey::Opencode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        assert_eq!(second.registered, 0);
        assert_eq!(second.skipped_existing, 3);
        assert_eq!(second.pruned, 0);
        assert!(second.errors.is_empty(), "{:?}", second.errors);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "file must be byte-identical"
        );
    }

    // I2 — user-set model/description survive byte-identical.
    #[test]
    fn json_user_model_survives() {
        let fixture = test_fixtures::build();
        let dir = temp_config_dir();
        let path = dir.path().join("opencode.json");
        let seeded = serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "agent": {
                "orchestrator": {
                    "description": "my custom description",
                    "mode": "primary",
                    "model": "deepseek/deepseek-v4-pro",
                    "prompt": "{file:/custom/path.md}"
                },
                "sddk-foo": {
                    "description": "user edited foo",
                    "mode": "subagent",
                    "hidden": true,
                    "model": "deepseek/deepseek-reasoner",
                    "prompt": "{file:/custom/foo.md}"
                },
                "gentle-bar": {
                    "description": "user edited bar",
                    "mode": "subagent",
                    "hidden": true,
                    "model": "zai-coding-plan/glm-5-turbo",
                    "prompt": "{file:/custom/bar.md}"
                }
            },
            "mcp": {}
        });
        let seeded_bytes = serde_json::to_string_pretty(&seeded).unwrap();
        std::fs::write(&path, &seeded_bytes).unwrap();
        let context = ctx(&fixture, Some(&fixture.models));
        let report = upsert_json_agents(
            &path,
            IdeKey::Opencode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        assert_eq!(report.skipped_existing, 3);
        assert!(report.errors.is_empty(), "{:?}", report.errors);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            seeded_bytes.as_bytes(),
            "user entries must remain byte-identical (no write at all)"
        );
    }

    // I3 — first-time entry created with model from override resolution.
    #[test]
    fn json_first_time_creates() {
        let fixture = test_fixtures::build();
        let dir = temp_config_dir();
        let path = dir.path().join("opencode.json");
        let context = ctx(&fixture, Some(&fixture.models));
        let report = upsert_json_agents(
            &path,
            IdeKey::Opencode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        assert_eq!(report.registered, 3);
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let foo = &config["agent"]["sddk-foo"];
        assert_eq!(
            foo["model"], "deepseek/deepseek-reasoner",
            "override must win"
        );
        assert_eq!(foo["description"], "Foo explorer");
        assert_eq!(foo["mode"], "subagent");
        assert_eq!(foo["hidden"], true);
        assert_eq!(
            foo["prompt"],
            format!(
                "{{file:{}}}",
                fixture.root.path().join("agents/sddk-foo.md").display()
            )
        );
        let orchestrator = &config["agent"]["orchestrator"];
        assert_eq!(
            orchestrator["mode"], "primary",
            "PRIMARY_AGENTS must be primary"
        );
        assert_eq!(orchestrator["model"], "deepseek/deepseek-chat");
        assert!(orchestrator.get("hidden").is_none());
    }

    // I4 — prune removes only framework-namespaced orphans; user entries kept.
    #[test]
    fn json_prunes_framework_orphan_only() {
        let fixture = test_fixtures::build();
        let dir = temp_config_dir();
        let path = dir.path().join("opencode.json");
        std::fs::write(
            &path,
            serde_json::to_string_pretty(&serde_json::json!({
                "$schema": "https://opencode.ai/config.json",
                "agent": {
                    "sddk-zombie": { "description": "stale", "mode": "subagent", "model": "x" },
                    "my-agent": { "description": "user", "mode": "primary", "model": "y" }
                },
                "mcp": {}
            }))
            .unwrap(),
        )
        .unwrap();
        let context = ctx(&fixture, Some(&fixture.models));
        let report = upsert_json_agents(
            &path,
            IdeKey::Opencode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        assert_eq!(report.pruned, 1);
        assert_eq!(report.registered, 3);
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(config["agent"].get("sddk-zombie").is_none());
        assert_eq!(config["agent"]["my-agent"]["model"], "y");
        assert_eq!(config["agent"]["my-agent"]["description"], "user");
    }

    // I5 — zcode mirrors opencode: same agent set, same schema.
    #[test]
    fn zcode_mirrors_opencode_schema() {
        let fixture = test_fixtures::build();
        let dir = temp_config_dir();
        let opencode_path = dir.path().join("opencode.json");
        let zcode_path = dir.path().join("zcode.json");
        let context = ctx(&fixture, Some(&fixture.models));
        let opencode_report = upsert_json_agents(
            &opencode_path,
            IdeKey::Opencode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        let zcode_report = upsert_json_agents(
            &zcode_path,
            IdeKey::Zcode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        assert_eq!(opencode_report.registered, 3);
        assert_eq!(zcode_report.registered, 3);
        let opencode: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&opencode_path).unwrap()).unwrap();
        let zcode: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&zcode_path).unwrap()).unwrap();
        let opencode_agents = opencode["agent"].as_object().unwrap();
        let zcode_agents = zcode["agent"].as_object().unwrap();
        assert_eq!(opencode_agents.len(), zcode_agents.len());
        for (name, entry) in opencode_agents {
            let mirror = &zcode_agents[name];
            for key in ["description", "mode", "hidden", "prompt"] {
                assert_eq!(
                    entry[key], mirror[key],
                    "agent {name} key {key} must mirror"
                );
            }
            // Model may legitimately differ (per-IDE overrides) but the key
            // must exist in both — same schema.
            assert!(entry.get("model").is_some() && mirror.get("model").is_some());
        }
    }

    // I12 — ConfigAbsent: entries written without a `model` key.
    #[test]
    fn config_absent_omits_model_key() {
        let fixture = test_fixtures::build();
        let dir = temp_config_dir();
        let path = dir.path().join("opencode.json");
        let context = ctx(&fixture, None);
        let report = upsert_json_agents(
            &path,
            IdeKey::Opencode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        assert_eq!(report.registered, 3, "ConfigAbsent still registers agents");
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        for (name, entry) in config["agent"].as_object().unwrap() {
            assert!(
                entry.get("model").is_none(),
                "agent {name} must have no model key when config is absent"
            );
        }
    }

    // I13 — NoModelConfigured skips the agent; others still register.
    #[test]
    fn no_model_configured_skips_agent() {
        let fixture = test_fixtures::build();
        let dir = temp_config_dir();
        let path = dir.path().join("opencode.json");
        // Fast tier table lacks opencode → sddk-foo and gentle-bar unresolvable.
        let yaml = test_fixtures::FIXTURE_YAML.replace(
            "fast:\n    opencode: zai-coding-plan/glm-5-turbo\n    zcode: zai-coding-plan/glm-5-turbo",
            "fast:\n    zcode: zai-coding-plan/glm-5-turbo",
        );
        let mut models = crate::dev::agent_models::AgentModelsConfig::from_yaml(&yaml).unwrap();
        models.clear_override("sddk-foo", IdeKey::Opencode);
        let context = ctx(&fixture, Some(&models));
        let report = upsert_json_agents(
            &path,
            IdeKey::Opencode,
            &super::super::PRIMARY_AGENTS,
            &context,
        );
        assert_eq!(
            report.registered, 1,
            "orchestrator resolves via premium table"
        );
        assert_eq!(report.skipped_unresolved, 2);
        let config: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(config["agent"].get("sddk-foo").is_none());
        assert!(config["agent"].get("orchestrator").is_some());
    }
}
