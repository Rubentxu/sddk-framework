//! `dev uninstall` — remove an installed prefix or editor assets.

use crate::dev::common::{
    RECEIPT_FILE, framework_agent_names, read_receipt,
};
use crate::{CommandOutput, render_result};
use sha2::Digest;
use std::path::{Path, PathBuf};

// ── Private helpers ────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct UninstallReport {
    editor: String,
    entries_removed: usize,
    symlinks_removed: usize,
    files_kept: usize,
    errors: Vec<String>,
}

fn uninstall_editor(root: &Path, editor_dir: &Path) -> anyhow::Result<UninstallReport> {
    let mut report = UninstallReport {
        editor: editor_dir.to_string_lossy().into_owned(),
        entries_removed: 0,
        symlinks_removed: 0,
        files_kept: 0,
        errors: Vec::new(),
    };

    // 1. opencode.json agent entries.
    let opencode_json = editor_dir.join("opencode.json");
    if opencode_json.exists()
        && let Ok(content) = std::fs::read_to_string(&opencode_json)
        && let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(agents) = config.get_mut("agent").and_then(|v| v.as_object_mut())
    {
        let framework = framework_agent_names(root);
        let before = agents.len();
        agents.retain(|name, _| !framework.iter().any(|f| f == name));
        report.entries_removed = before - agents.len();
        if report.entries_removed > 0 {
            let serialized = serde_json::to_string_pretty(&config)?;
            std::fs::write(&opencode_json, serialized)?;
        }
    }

    // 2. Framework symlinks (target points into the repo).
    let root_canon = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    for category in ["agents", "skills", "prompts", "workflows"] {
        let dir = editor_dir.join(category);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(target) = std::fs::read_link(&path) {
                let absolute = if target.is_absolute() {
                    target
                } else {
                    path.parent()
                        .map(|parent| parent.join(&target))
                        .unwrap_or(target)
                };
                if absolute.starts_with(&root_canon) {
                    let _ = std::fs::remove_file(&path);
                    report.symlinks_removed += 1;
                } else {
                    report.files_kept += 1;
                }
            } else {
                // Regular file (not a symlink): preserve local-only assets.
                report.files_kept += 1;
            }
        }
    }
    Ok(report)
}

// ── Public subcommand ──────────────────────────────────────────────────────────

pub(super) fn run_dev_uninstall(args: super::UninstallArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let mut output = String::new();

        // Binary prefix removal (existing behavior) — optional when --editor is used.
        if let Some(prefix) = &args.prefix {
            let receipt = read_receipt(prefix)?;
            let binary_path = prefix.join(&receipt.binary_path);
            let bytes = std::fs::read(&binary_path)?;
            let digest = format!("sha256:{:x}", sha2::Sha256::digest(&bytes));
            if digest != receipt.binary_sha256 {
                anyhow::bail!("refusing to uninstall: binary does not match the receipt");
            }
            std::fs::remove_file(&binary_path)?;
            std::fs::remove_file(prefix.join(RECEIPT_FILE))?;
            output.push_str("binary: removed\n");
        }

        // Editor framework removal (optional).
        if let Some(editor) = args.editor {
            let root = std::fs::canonicalize(&args.root)?;
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp"));
            let opencode_dir = args
                .opencode_dir
                .clone()
                .unwrap_or_else(|| home.join(".config/opencode"));
            let zcode_dir = args
                .zcode_dir
                .clone()
                .unwrap_or_else(|| home.join(".zcode"));
            if matches!(editor, super::LinkEditor::OpenCode | super::LinkEditor::All) {
                let report = uninstall_editor(&root, &opencode_dir)?;
                output.push_str(&format!(
                    "opencode: {} entries, {} symlinks removed, {} kept\n",
                    report.entries_removed, report.symlinks_removed, report.files_kept
                ));
            }
            if matches!(editor, super::LinkEditor::ZCode | super::LinkEditor::All) {
                let report = uninstall_editor(&root, &zcode_dir)?;
                output.push_str(&format!(
                    "zcode: {} entries, {} symlinks removed, {} kept\n",
                    report.entries_removed, report.symlinks_removed, report.files_kept
                ));
            }
        }
        Ok(output)
    })();
    render_result(result, format, |output: &String| output.clone())
}
