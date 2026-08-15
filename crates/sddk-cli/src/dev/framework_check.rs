//! Framework agent registry helpers — frontmatter parsing and opencode.json registration.
//! Extracted from link.rs to keep link.rs below its LOC ceiling (ADR-016).

use crate::CliEnvironment;
use crate::dev::common::{sha256_hex, walk_dir};
use crate::dev::paths::resolve_active_framework_root;
use sddk_gateway::PermissionPolicy;
use std::collections::HashSet;
use std::path::Path;

/// Agents that should be marked as "primary" (visible by default) in opencode.json.
const PRIMARY_AGENTS: [&str; 2] = ["orchestrator", "book-orchestrator"];

// ── Agent frontmatter ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct AgentFrontmatter {
    description: String,
    model: Option<String>,
}

fn parse_frontmatter(path: &Path) -> Option<AgentFrontmatter> {
    let content = std::fs::read_to_string(path).ok()?;
    let block = content.strip_prefix("---")?.split_once("---")?.0;
    let mut description = String::new();
    let mut model = None;
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("description:") {
            description = value.trim().trim_matches('"').to_owned();
        } else if let Some(value) = line.strip_prefix("model:") {
            model = Some(value.trim().trim_matches('"').to_owned());
        }
    }
    if description.is_empty() {
        return None;
    }
    Some(AgentFrontmatter { description, model })
}

// ── Agent name resolution ─────────────────────────────────────────────────────

/// Names of framework agents from permissions.yaml or filesystem.
pub(super) fn framework_agent_names(root: &Path) -> Vec<String> {
    let agents_dir = root.join("agents");
    if let Ok(policy) = PermissionPolicy::from_file(root.join("permissions.yaml")) {
        let mut names: Vec<String> = policy
            .agents()
            .filter(|name| agents_dir.join(format!("{name}.md")).exists())
            .map(str::to_owned)
            .collect();
        names.sort();
        return names;
    }
    let mut names: Vec<String> = std::fs::read_dir(&agents_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("md"))
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Upsert framework agent entries into opencode.json.
pub(super) fn register_opencode_agents(root: &Path, opencode_json: &Path) -> anyhow::Result<usize> {
    let mut config: serde_json::Value = if opencode_json.exists() {
        serde_json::from_str(&std::fs::read_to_string(opencode_json)?)
            .map_err(|e| anyhow::anyhow!("opencode.json invalid: {e}"))?
    } else {
        serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "agent": {},
            "mcp": {}
        })
    };
    let agents = config
        .get_mut("agent")
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| anyhow::anyhow!("opencode.json has no agent map"))?;
    let mut registered = 0usize;
    for name in framework_agent_names(root) {
        let md_path = root.join("agents").join(format!("{name}.md"));
        let Some(frontmatter) = parse_frontmatter(&md_path) else {
            continue;
        };
        let model = frontmatter
            .model
            .clone()
            .unwrap_or_else(|| "minimax-coding-plan/MiniMax-M3".to_owned());
        let primary = PRIMARY_AGENTS.contains(&name.as_str());
        let mut entry = serde_json::json!({
            "description": frontmatter.description,
            "mode": if primary { "primary" } else { "subagent" },
            "model": model,
            "prompt": format!("{{file:{}}}", md_path.to_string_lossy()),
        });
        if !primary {
            entry["hidden"] = serde_json::Value::Bool(true);
        }
        agents.insert(name, entry);
        registered += 1;
    }
    let source_agent_names: HashSet<String> = framework_agent_names(root).into_iter().collect();
    let orphans: Vec<String> = agents
        .keys()
        .filter(|name| {
            let name = name.as_str();
            (name.starts_with("sddk-") || name.starts_with("sdd-") || name.starts_with("gentle-"))
                && !source_agent_names.contains(name)
        })
        .cloned()
        .collect();
    for orphan in orphans {
        agents.remove(&orphan);
    }
    let serialized = serde_json::to_string_pretty(&config)?;
    std::fs::write(opencode_json, serialized)?;
    Ok(registered)
}

// ── Link report types (shared with link.rs) ────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct LinkReport {
    pub editor: String,
    pub agents_linked: usize,
    pub skills_linked: usize,
    pub prompts_linked: usize,
    pub workflows_linked: usize,
    pub stale_replaced: usize,
    pub pruned: usize,
    pub errors: Vec<String>,
}

pub(super) fn link_report_text(report: &LinkReport) -> String {
    format!(
        "editor: {}\nagents: {}\nskills: {}\nprompts: {}\nworkflows: {}\nstale_replaced: {}\npruned: {}\nerrors: {}\n",
        report.editor,
        report.agents_linked,
        report.skills_linked,
        report.prompts_linked,
        report.workflows_linked,
        report.stale_replaced,
        report.pruned,
        report.errors.len()
    )
}

// ── Asset sync ────────────────────────────────────────────────────────────────

/// Sync the framework assets tree from source into target (idempotent).
pub(super) fn sync_assets(source: &Path, target: &Path) -> anyhow::Result<usize> {
    let mut copied = 0usize;
    std::fs::create_dir_all(target)?;
    for entry in walk_dir(source) {
        let relative = entry
            .strip_prefix(source)
            .unwrap_or(entry.as_path())
            .to_path_buf();
        let destination = target.join(&relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let needs_copy = match (std::fs::read(&entry), std::fs::read(&destination)) {
            (Ok(src), Ok(dst)) => src != dst,
            _ => true,
        };
        if needs_copy {
            std::fs::copy(&entry, &destination)?;
        }
        copied += 1;
    }
    Ok(copied)
}
