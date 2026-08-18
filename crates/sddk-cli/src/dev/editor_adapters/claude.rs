//! Claude Code adapter: native `.md` agent files with YAML frontmatter
//! (ADR-0019). Owns `<claude_dir>/agents` — no symlinks there.

use super::{AdapterReport, RegistrationContext, is_framework_namespaced, resolve_for_models};
use crate::dev::agent_models::IdeKey;
use crate::dev::common::atomic_write;
use std::collections::HashSet;
use std::path::PathBuf;

/// Claude Code model vocabulary: short aliases or full provider/model IDs.
fn claude_model_valid(model: &str) -> bool {
    matches!(model, "sonnet" | "opus" | "haiku" | "inherit") || model.contains('/')
}

/// Claude registration: one `agents/<name>.md` per bundle agent.
pub struct ClaudeAdapter {
    pub dir: PathBuf,
}

impl super::EditorAdapter for ClaudeAdapter {
    fn editor_name(&self) -> &'static str {
        "claude"
    }

    fn register(&self, ctx: &RegistrationContext<'_>) -> AdapterReport {
        let mut report = AdapterReport {
            editor: "claude".to_owned(),
            ..AdapterReport::default()
        };
        let agents_dir = self.dir.join("agents");
        for agent in ctx.agents {
            let target = agents_dir.join(format!("{}.md", agent.name));
            if target.exists() {
                report.skipped_existing += 1;
                continue;
            }
            match resolve_for_models(ctx.models, &agent.name, IdeKey::Claude) {
                Ok(model) => {
                    if let Some(model) = &model
                        && !claude_model_valid(model)
                    {
                        report.errors.push(format!(
                            "agent {}: model '{model}' not in claude vocabulary \
                             (sonnet|opus|haiku|inherit or a full provider/model id)",
                            agent.name
                        ));
                        report.skipped_unresolved += 1;
                        continue;
                    }
                    let mut content = format!(
                        "---\nname: {}\ndescription: {}\n",
                        agent.name, agent.description
                    );
                    if let Some(tools) = &agent.tools {
                        content.push_str(&format!("tools: {tools}\n"));
                    }
                    if let Some(model) = model {
                        content.push_str(&format!("model: {model}\n"));
                    }
                    content.push_str("---\n");
                    content.push_str(&agent.body);
                    match atomic_write(&target, content.as_bytes(), None) {
                        Ok(()) => report.registered += 1,
                        Err(error) => report.errors.push(format!("{}: {error}", target.display())),
                    }
                }
                Err(()) => report.skipped_unresolved += 1,
            }
        }
        let bundle_names: HashSet<&str> =
            ctx.agents.iter().map(|agent| agent.name.as_str()).collect();
        if let Ok(entries) = std::fs::read_dir(&agents_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("md") {
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
#[path = "../tests/claude_adapter_tests.rs"]
mod claude_adapter_tests;
