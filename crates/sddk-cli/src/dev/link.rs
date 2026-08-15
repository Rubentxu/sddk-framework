//! `dev link` — symlink framework assets into an editor.

use super::LinkEditor;
use super::manifest::MANIFEST_FILE;
use crate::dev::common::{sha256_hex, walk_dir};
use crate::dev::manifest::verify_manifest;
use crate::dev::paths::resolve_active_framework_root;
use crate::dev::registry::write_skill_registry;
use crate::{CliEnvironment, CommandOutput, OutputFormat, render_result};
use sddk_gateway::{GitExecutor, PermissionPolicy};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

// ── Private helpers ────────────────────────────────────────────────────────────

/// Agents that should be marked as "primary" (visible by default) in opencode.json.
const PRIMARY_AGENTS: [&str; 2] = ["orchestrator", "book-orchestrator"];

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct LinkReport {
    editor: String,
    agents_linked: usize,
    skills_linked: usize,
    prompts_linked: usize,
    workflows_linked: usize,
    stale_replaced: usize,
    pruned: usize,
    errors: Vec<String>,
}

fn link_text(report: &LinkReport) -> String {
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

fn link_file(source: &Path, target: &Path, stale_replaced: &mut usize) -> std::io::Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(target) {
        if metadata.file_type().is_symlink() {
            if let Ok(current) = std::fs::read_link(target) {
                let source_abs =
                    std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
                let current_abs = if current.is_absolute() {
                    current
                } else {
                    target
                        .parent()
                        .unwrap_or_else(|| Path::new("/"))
                        .join(current)
                };
                let current_abs = std::fs::canonicalize(&current_abs).unwrap_or(current_abs);
                if current_abs == source_abs {
                    return Ok(());
                }
            }
            std::fs::remove_file(target)?;
        } else {
            let backup = target.with_extension("sddk-stale");
            if !backup.exists() {
                std::fs::rename(target, &backup)?;
            } else {
                std::fs::remove_file(target)?;
            }
            *stale_replaced += 1;
        }
    }
    std::fs::create_dir_all(target.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "target has no parent")
    })?)?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)?;
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(source, target)?;
    }
    Ok(())
}

/// Prune editor entries that the framework no longer ships.
fn prune_editor(root: &Path, editor_dir: &Path) -> usize {
    let framework_namespace = |name: &std::ffi::OsStr| {
        let name = name.to_string_lossy();
        name.starts_with("sddk-")
            || name.starts_with("sdd-")
            || name.starts_with("gentle-")
            || name == "orchestrator.md"
    };
    let mut pruned = 0usize;
    let mut prune_entry = |path: &Path, source: &Path, name: &std::ffi::OsStr| -> bool {
        let is_framework_entry = framework_namespace(name);
        let exists_in_source = source.join(name).exists();
        let is_broken_link = path
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink() && !path.exists())
            .unwrap_or(false);
        let is_stale_backup = name.to_string_lossy().ends_with(".sddk-stale");
        let should_remove =
            is_broken_link || is_stale_backup || (is_framework_entry && !exists_in_source);
        if should_remove {
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_dir_all(path);
            pruned += 1;
        }
        should_remove
    };

    let source_agents = root.join("agents");
    let target_agents = editor_dir.join("agents");
    if let Ok(entries) = std::fs::read_dir(&target_agents) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let is_md = entry.path().extension().and_then(|e| e.to_str()) == Some("md");
            if is_md {
                prune_entry(&entry.path(), &source_agents, &name);
            }
        }
    }
    let source_skills = root.join("skills");
    let target_skills = editor_dir.join("skills");
    if let Ok(entries) = std::fs::read_dir(&target_skills) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let path = entry.path();
            let is_skill = entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                || path.extension().and_then(|e| e.to_str()) == Some("md");
            if is_skill {
                prune_entry(&path, &source_skills, &name);
            }
        }
    }
    let source_prompts = root.join("prompts/sddk");
    let target_prompts = editor_dir.join("prompts/sddk");
    if let Ok(entries) = std::fs::read_dir(&target_prompts) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let is_md = entry.path().extension().and_then(|e| e.to_str()) == Some("md");
            if is_md {
                prune_entry(&entry.path(), &source_prompts, &name);
            }
        }
    }
    let source_workflows = root.join("prompts/sddk/workflows");
    let target_workflows = editor_dir.join("workflows");
    if let Ok(entries) = std::fs::read_dir(&target_workflows) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).map(str::to_owned);
            let is_workflow_entry = matches!(ext.as_deref(), Some("yaml") | Some("yml"));
            let is_stale_backup = name.to_string_lossy().ends_with(".sddk-stale");
            if is_workflow_entry || is_stale_backup {
                prune_entry(&path, &source_workflows, &name);
            }
        }
    }
    pruned
}

fn link_editor(root: &Path, editor_dir: &Path) -> LinkReport {
    let mut report = LinkReport {
        editor: editor_dir.to_string_lossy().into_owned(),
        agents_linked: 0,
        skills_linked: 0,
        prompts_linked: 0,
        workflows_linked: 0,
        stale_replaced: 0,
        pruned: 0,
        errors: Vec::new(),
    };
    let mut stale = 0usize;

    let agents_source = root.join("agents");
    if let Ok(entries) = std::fs::read_dir(&agents_source) {
        let agents_target = editor_dir.join("agents");
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
                let name = entry.file_name();
                if let Err(error) = link_file(&entry.path(), &agents_target.join(&name), &mut stale)
                {
                    report.errors.push(format!("agents/{name:?}: {error}"));
                } else {
                    report.agents_linked += 1;
                }
            }
        }
    }

    let skills_source = root.join("skills");
    if let Ok(entries) = std::fs::read_dir(&skills_source) {
        let skills_target = editor_dir.join("skills");
        for entry in entries.flatten() {
            let path = entry.path();
            let is_skill_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let is_markdown = path.extension().and_then(|e| e.to_str()) == Some("md");
            if is_skill_dir || is_markdown {
                let name = entry.file_name();
                if let Err(error) = link_file(&path, &skills_target.join(&name), &mut stale) {
                    report.errors.push(format!("skills/{name:?}: {error}"));
                } else {
                    report.skills_linked += 1;
                }
            }
        }
    }

    let prompts_source = root.join("prompts/sddk");
    let prompts_target = editor_dir.join("prompts/sddk");
    if prompts_source.is_dir() {
        for entry in walk_dir(&prompts_source) {
            if entry.is_file() {
                let relative = entry
                    .strip_prefix(&prompts_source)
                    .unwrap_or(entry.as_path());
                let target = prompts_target.join(relative);
                if let Err(error) = link_file(&entry, &target, &mut stale) {
                    report.errors.push(format!("prompts/{relative:?}: {error}"));
                } else {
                    report.prompts_linked += 1;
                }
            }
        }
    }

    let workflows_source = root.join("prompts/sddk/workflows");
    let workflows_target = editor_dir.join("workflows");
    if workflows_source.is_dir() {
        for entry in walk_dir(&workflows_source) {
            if entry.is_file()
                && let Some(name) = entry
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_owned)
            {
                if let Err(error) = link_file(&entry, &workflows_target.join(&name), &mut stale) {
                    report.errors.push(format!("workflows/{name:?}: {error}"));
                } else {
                    report.workflows_linked += 1;
                }
            }
        }
    }

    report.stale_replaced = stale;
    report.pruned = prune_editor(root, editor_dir);
    report
}

/// Sync the framework assets tree from source into target (idempotent).
fn sync_assets(source: &Path, target: &Path) -> anyhow::Result<usize> {
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

/// Names of framework agents from permissions.yaml or filesystem.
fn framework_agent_names(root: &Path) -> Vec<String> {
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

/// Upsert framework agent entries into opencode.json.
fn register_opencode_agents(root: &Path, opencode_json: &Path) -> anyhow::Result<usize> {
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

// ── Public subcommand ──────────────────────────────────────────────────────────

pub(super) fn run_dev_link(args: super::LinkArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<LinkReport>> {
        let root = if args.root.as_os_str() == "." {
            resolve_active_framework_root(environment)?
        } else {
            std::fs::canonicalize(&args.root)?
        };
        // Dogfooding: sync assets and regenerate manifest.
        if args.root.as_os_str() != "."
            && let Ok(framework_root) = resolve_active_framework_root(environment)
            && root.join("assets").is_dir()
        {
            let _ = sync_assets(&root.join("assets"), &framework_root.join("assets"));
            let _ = super::manifest::write_manifest(&framework_root);
        }
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
        let mut reports = Vec::new();
        if matches!(args.editor, LinkEditor::OpenCode | LinkEditor::All) {
            reports.push(link_editor(&root, &opencode_dir));
            let opencode_json = opencode_dir.join("opencode.json");
            match register_opencode_agents(&root, &opencode_json) {
                Ok(registered) => {
                    eprintln!("opencode: registered {registered} framework agents in opencode.json")
                }
                Err(error) => {
                    eprintln!("warning: opencode.json registration failed: {error}")
                }
            }
        }
        if matches!(args.editor, LinkEditor::ZCode | LinkEditor::All) {
            reports.push(link_editor(&root, &zcode_dir));
        }

        // Write idempotent skill registry.
        let project_root_for_registry =
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        if args.write_registry {
            match write_skill_registry(environment, &project_root_for_registry, &root) {
                Ok((path, count)) => {
                    eprintln!(
                        "skill registry: {} entries written to {}",
                        count,
                        path.display()
                    )
                }
                Err(error) => {
                    eprintln!("warning: skill registry write failed: {error}");
                }
            }
        }

        Ok(reports)
    })();
    render_result(result, format, |reports: &Vec<LinkReport>| {
        let mut text = String::new();
        for report in reports {
            text.push_str(&link_text(report));
        }
        text
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod reconciliation_tests {
    use super::*;

    fn temp_tree(tag: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sddk-link-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn prune_removes_framework_deprecated_but_keeps_foreign() {
        let root = temp_tree("root");
        let editor = temp_tree("editor");
        // Framework source: one agent + one skill.
        std::fs::create_dir_all(root.join("agents")).unwrap();
        std::fs::write(
            root.join("agents/orchestrator.md"),
            "---\nname: orchestrator\n---\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("skills")).unwrap();
        std::fs::create_dir_all(root.join("skills/sddk-apply")).unwrap();
        std::fs::write(root.join("skills/sddk-apply/SKILL.md"), "# apply\n").unwrap();

        // Editor state: broken framework link, orphan namespaced skill,
        // stale backup, and a foreign (arch-stack) skill that must survive.
        std::fs::create_dir_all(editor.join("agents")).unwrap();
        std::fs::create_dir_all(editor.join("skills")).unwrap();
        std::fs::create_dir_all(editor.join("workflows")).unwrap();
        std::os::unix::fs::symlink(
            "/nonexistent/sddk-deprecated.md",
            editor.join("agents/sddk-deprecated.md"),
        )
        .unwrap();
        std::fs::create_dir_all(editor.join("skills/sddk-continue-options")).unwrap();
        std::fs::write(
            editor.join("skills/sddk-continue-options/SKILL.md"),
            "# orphan\n",
        )
        .unwrap();
        std::fs::write(editor.join("workflows/sddk-a-full.sddk-stale"), "stale\n").unwrap();
        std::fs::create_dir_all(editor.join("skills/architecture-discovery")).unwrap();
        std::fs::write(
            editor.join("skills/architecture-discovery/SKILL.md"),
            "# foreign\n",
        )
        .unwrap();

        let pruned = prune_editor(&root, &editor);
        // 1 broken agent + 1 orphan skill + 1 stale workflow = 3.
        assert_eq!(pruned, 3);
        assert!(!editor.join("agents/sddk-deprecated.md").exists());
        assert!(!editor.join("skills/sddk-continue-options").exists());
        assert!(!editor.join("workflows/sddk-a-full.sddk-stale").exists());
        // Foreign surface untouched.
        assert!(
            editor
                .join("skills/architecture-discovery/SKILL.md")
                .exists()
        );

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&editor).ok();
    }

    #[test]
    fn link_file_is_idempotent_when_target_matches() {
        let dir = temp_tree("link");
        let source = dir.join("source.md");
        let target = dir.join("target.md");
        std::fs::write(&source, "content").unwrap();
        let mut stale = 0usize;
        link_file(&source, &target, &mut stale).unwrap();
        let mtime1 = std::fs::metadata(&target).unwrap().modified().unwrap();
        link_file(&source, &target, &mut stale).unwrap();
        let mtime2 = std::fs::metadata(&target).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "correct symlink must not be recreated");
        assert_eq!(stale, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn link_file_replaces_stale_copy_with_backup() {
        let dir = temp_tree("stale");
        let source = dir.join("source.md");
        let target = dir.join("target.md");
        std::fs::write(&source, "new").unwrap();
        std::fs::write(&target, "old copy").unwrap();
        let mut stale = 0usize;
        link_file(&source, &target, &mut stale).unwrap();
        assert_eq!(stale, 1);
        assert!(dir.join("target.sddk-stale").exists());
        assert!(
            std::fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}
