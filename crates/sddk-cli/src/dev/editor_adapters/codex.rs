//! Codex adapter: native TOML agent files in `<codex_dir>/agents/`
//! (ADR-0019). Body translation: markdown body → `developer_instructions`.
//! Fields the framework does not model (e.g. `model_reasoning_effort`,
//! `model_reasoning_summary`) are deliberately not written — documented in
//! docs/adr/ADR-0019 and the apply notes.

use super::{AdapterReport, RegistrationContext, is_framework_namespaced, resolve_for_models};
use crate::dev::agent_models::IdeKey;
use crate::dev::common::atomic_write;
use std::collections::HashSet;
use std::path::PathBuf;

/// Codex registration: one `agents/<name>.toml` per bundle agent.
pub struct CodexAdapter {
    pub dir: PathBuf,
}

impl CodexAdapter {
    fn to_toml(agent: &super::AgentSource, model: Option<String>) -> anyhow::Result<String> {
        let mut table = toml::map::Map::new();
        table.insert("name".to_owned(), toml::Value::String(agent.name.clone()));
        table.insert(
            "description".to_owned(),
            toml::Value::String(agent.description.clone()),
        );
        table.insert(
            "developer_instructions".to_owned(),
            toml::Value::String(agent.body.clone()),
        );
        if let Some(model) = model {
            table.insert("model".to_owned(), toml::Value::String(model));
        }
        Ok(toml::to_string_pretty(&toml::Value::Table(table))?)
    }
}

impl super::EditorAdapter for CodexAdapter {
    fn editor_name(&self) -> &'static str {
        "codex"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        let mut report = AdapterReport {
            editor: "codex".to_owned(),
            ..AdapterReport::default()
        };
        let agents_dir = self.dir.join("agents");
        for agent in ctx.agents {
            let target = agents_dir.join(format!("{}.toml", agent.name));
            if target.exists() {
                report.skipped_existing += 1;
                continue;
            }
            match resolve_for_models(ctx.models, &agent.name, IdeKey::Codex) {
                Ok(model) => match Self::to_toml(agent, model) {
                    Ok(serialized) => match atomic_write(&target, serialized.as_bytes(), None) {
                        Ok(()) => report.registered += 1,
                        Err(error) => report.errors.push(format!("{}: {error}", target.display())),
                    },
                    Err(error) => report
                        .errors
                        .push(format!("{}: serialization failed: {error}", agent.name)),
                },
                Err(()) => report.skipped_unresolved += 1,
            }
        }
        let bundle_names: HashSet<&str> =
            ctx.agents.iter().map(|agent| agent.name.as_str()).collect();
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }
                let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
                    continue;
                };
                if is_framework_namespaced(stem) && !bundle_names.contains(stem) {
                    match std::fs::remove_file(&path) {
                        Ok(()) => report.pruned += 1,
                        Err(error) => report
                            .errors
                            .push(format!("{}: cannot prune: {error}", path.display())),
                    }
                }
            }
        }
        report
    }
}

#[cfg(test)]
#[path = "../tests/codex_adapter_tests.rs"]
mod codex_adapter_tests;
