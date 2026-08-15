//! Skill registry — write idempotent skill registry markdown.

use std::path::{Path, PathBuf};

use crate::{CliEnvironment, CommandOutput};

use super::common::atomic_write;
use super::paths::sddk_data_dir;

/// Skill registry entry: name, trigger/description, scope, and path of one skill.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
struct SkillRegistryEntry {
    name: String,
    trigger: String,
    description: String,
    scope: String,
    path: String,
}

/// Minimal frontmatter extraction (name, description) from a skill SKILL.md.
struct SkillFrontmatter {
    name: String,
    description: String,
}

fn parse_skill_frontmatter(path: &Path) -> Option<SkillFrontmatter> {
    let content = std::fs::read_to_string(path).ok()?;
    let block = content.strip_prefix("---")?.split_once("---")?.0;
    let mut name = String::new();
    let mut description = String::new();
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("name:") {
            name = value.trim().trim_matches('"').to_owned();
        } else if let Some(value) = line.strip_prefix("description:") {
            description = value.trim().trim_matches('"').to_owned();
        }
    }
    if name.is_empty() || description.is_empty() {
        return None;
    }
    Some(SkillFrontmatter { name, description })
}

/// Escape pipes and newlines in a markdown table cell so the table renders correctly.
fn escape_md_cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', "\\n")
}

/// Write an idempotent, deduplicated skill registry to
/// `$SDDK_DATA_DIR/projects/<project_id>/skill-registry.md`.
///
/// Scans skills from three scopes in precedence order (first wins dedupe):
/// 1. Project-level: `{project_root}/.opencode/skills/`, `.agents/skills/`,
///    `.claude/skills/`, `.zcode/skills/`
/// 2. User-level: `$HOME/.config/opencode/skills/`, `claude/skills/`, `zcode/skills/`
/// 3. Framework-level: `{framework_root}/skills/`
///
/// Skips `_shared` and `skill-registry`. Parses frontmatter name + description.
/// Extracts trigger from description (text before first ". "). Renders markdown table.
/// File is written atomically so a second invocation produces byte-identical result.
pub(crate) fn write_skill_registry(
    environment: &CliEnvironment,
    project_root: &Path,
    framework_root: &Path,
) -> anyhow::Result<(PathBuf, usize)> {
    let project_id = resolve_project_id_for_registry(environment, project_root)?;
    let registry_dir = sddk_data_dir(environment)?
        .join("projects")
        .join(&project_id);
    let registry_path = registry_dir.join("skill-registry.md");
    std::fs::create_dir_all(&registry_dir)?;

    // Determine home dir for user-level scans.
    // Prefer environment.home when set (tests, isolated environments);
    // fall back to the system HOME variable.
    let home = environment.home.clone().unwrap_or_else(|| {
        std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"))
    });

    // Define all scopes with their search paths in precedence order.
    // Each entry: (scope_label, base_dir).
    // User-level: documented editor config dirs (AGENTS/layout supports zcode + others from bundle).
    // Project-level: project-root-relative dirs for adopted projects.
    // Framework-level: skills/ inside the active framework bundle.
    let scopes: Vec<(&str, PathBuf)> = vec![
        // Project-level dirs under the adopted project root.
        ("project", project_root.join(".opencode/skills")),
        ("project", project_root.join(".agents/skills")),
        ("project", project_root.join(".claude/skills")),
        ("project", project_root.join(".zcode/skills")),
        ("project", project_root.join(".kilo/skills")),
        ("project", project_root.join(".codex/skills")),
        // User-level dirs (XDG_CONFIG_HOME and HOME-relative documented paths).
        ("user", home.join(".config/opencode/skills")),
        ("user", home.join(".agents/skills")),
        ("user", home.join(".claude/skills")),
        ("user", home.join(".zcode/skills")),
        ("user", home.join(".opencode/skills")),
        ("user", home.join(".config/kilo/skills")),
        ("user", home.join(".codex/skills")),
        // Framework-level dirs under the framework root.
        ("framework", framework_root.join("skills")),
    ];

    let mut entries: Vec<SkillRegistryEntry> = Vec::new();
    let mut seen_names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (scope, skills_base) in scopes {
        if let Ok(skill_dirs) = std::fs::read_dir(&skills_base) {
            // Collect and sort for deterministic processing order.
            let mut dirs: Vec<_> = skill_dirs.flatten().collect();
            dirs.sort_by_key(|e| e.file_name());
            for skill_dir in dirs {
                let dir_name = skill_dir.file_name();
                let dir_name_str = dir_name.to_string_lossy();
                // Skip internal entries that are not user-facing skills.
                if dir_name_str == "_shared" || dir_name_str == "skill-registry" {
                    continue;
                }
                let skill_path = skill_dir.path();
                if !skill_path.is_dir() {
                    continue;
                }
                let skl_md = skill_path.join("SKILL.md");
                if !skl_md.is_file() {
                    continue;
                }
                // Dedupe by frontmatter name (first wins — higher precedence scope wins).
                let frontmatter_name = if let Some(fm) = parse_skill_frontmatter(&skl_md) {
                    fm.name.clone()
                } else {
                    dir_name_str.to_string()
                };
                if seen_names.contains(&frontmatter_name) {
                    continue;
                }
                seen_names.insert(frontmatter_name.clone());

                // Parse frontmatter for trigger and description.
                let (trigger, description) = if let Some(fm) = parse_skill_frontmatter(&skl_md) {
                    // Trigger: text before first ". " or first period in description.
                    let trigger_text = fm
                        .description
                        .split_once(". ")
                        .map(|(t, _)| t.to_string())
                        .or_else(|| fm.description.split_once('.').map(|(t, _)| t.to_string()))
                        .unwrap_or_else(|| fm.description.clone());
                    (trigger_text, fm.description)
                } else {
                    (String::new(), String::new())
                };

                entries.push(SkillRegistryEntry {
                    name: frontmatter_name,
                    trigger,
                    description,
                    scope: scope.to_string(),
                    path: skl_md.to_string_lossy().replace('\\', "/"),
                });
            }
        }
    }

    // Sort alphabetically by name.
    entries.sort_by(|a, b| a.name.cmp(&b.name));

    // Render as markdown table with escaped cells.
    let mut content = String::new();
    content.push_str("# Skill Registry\n\n");
    content.push_str("| Name | Trigger | Description | Scope | Path |\n");
    content.push_str("|------|---------|-------------|-------|------|\n");
    for entry in &entries {
        content.push_str(&format!(
            "| {} | {} | {} | {} | {} |\n",
            escape_md_cell(&entry.name),
            escape_md_cell(&entry.trigger),
            escape_md_cell(&entry.description),
            escape_md_cell(&entry.scope),
            escape_md_cell(&entry.path),
        ));
    }

    atomic_write(&registry_path, content.as_bytes(), None)?;
    Ok((registry_path, entries.len()))
}

/// Resolve project_id for the skill registry writer.
///
/// Uses ONLY `crate::resolve_remote` (git command), `crate::find_persisted_fallback_seed`
/// (adoption receipt), and `sddk_domain::resolve_project_identity` — never fabricates
/// a UUID from a hash.
///
/// Priority:
///  1. Git remote URL → canonical resolver (stable p-* across machines)
///  2. Persisted adoption receipt seed → seeded fallback (stable p-* for adopted dirs)
///  3. Neither → explicit error with instructions to run `sddk adopt`
pub(crate) fn resolve_project_id_for_registry(
    environment: &CliEnvironment,
    project_root: &Path,
) -> anyhow::Result<String> {
    let canonical = std::fs::canonicalize(project_root)?;
    let root_display = canonical.to_string_lossy().to_string();

    // Try git remote first.
    if let Some(remote_url) = crate::resolve_remote(project_root, None)? {
        // Use "." as scope (the CLI default) — remote already provides uniqueness.
        let identity = sddk_domain::resolve_project_identity(Some(&remote_url), ".", None);
        return identity
            .map(|id| id.project_id.to_string())
            .map_err(|e| anyhow::anyhow!("{e}"));
    }

    // No remote — look for a persisted adoption receipt for this workspace.
    if let Some(seed) = crate::find_persisted_fallback_seed(environment, &canonical, ".")? {
        let identity = sddk_domain::resolve_project_identity(None, ".", Some(&seed));
        return identity
            .map(|id| id.project_id.to_string())
            .map_err(|e| anyhow::anyhow!("{e}"));
    }

    // Neither remote nor adoption receipt found — require explicit adoption.
    anyhow::bail!(
        "cannot resolve project identity for registry: \
         no git remote found in {root_display} and no adoption receipt exists. \
         Run `sddk adopt --scope .` first to create a persistent project identity, \
         then retry `sddk dev link --write-registry`."
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod skill_registry_tests {
    use super::*;

    fn temp_project(tag: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("sddk-reg-prj-{tag}-{n}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn temp_framework(tag: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("sddk-reg-frm-{tag}-{n}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Environment with sddk_data_dir and home pointing at the temp location so the
    /// registry is written and user-scope skills are scanned from a directory we control.
    fn test_environment(temp_root: &std::path::Path) -> CliEnvironment {
        CliEnvironment {
            home: Some(temp_root.to_path_buf()),
            data_home: None,
            sddk_data_dir: Some(temp_root.to_path_buf()),
            state_home: None,
            cache_home: None,
            sddk_actor: None,
            user: None,
        }
    }

    /// Create a minimal SKILL.md with name and description frontmatter.
    fn make_skill(dir: &std::path::Path, name: &str, description: &str) {
        let content = format!("---\nname: {name}\ndescription: \"{description}\"\n---\n",);
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("SKILL.md"), content).unwrap();
    }

    /// Initialize a fake git repo with a fake remote so `crate::resolve_remote`
    /// returns a deterministic URL and the registry identity resolver produces a
    /// stable p-*. Uses `git init` so the git command succeeds.
    fn init_fake_git_remote(dir: &std::path::Path) {
        // Run `git init` so git commands work in this temp dir.
        let init_output = std::process::Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(dir)
            .output();
        if init_output.map(|o| !o.status.success()).unwrap_or(true) {
            // git not available or failed — skip.
            return;
        }
        // Use a fixed remote URL so the p-* ID is deterministic for the same dir path.
        let remote_url = "https://test.example.com/sddk-framework.git";
        let _ = std::process::Command::new("git")
            .args(["remote", "add", "origin", remote_url])
            .current_dir(dir)
            .output();
    }

    #[test]
    fn write_skill_registry_is_idempotent_and_dedupes() {
        // project_root drives project-ID computation; framework_root is scanned for skills.
        // Both are temp dirs here so we control the entire outcome.
        let project = temp_project("idempotent");
        let framework = temp_framework("idempotent");
        let env = test_environment(&project);

        // Initialize a fake git remote so resolve_remote returns a deterministic URL.
        init_fake_git_remote(&framework);

        // Skills live in framework scope (mirrors real dogfooding: sddk-framework IS the adopted workspace).
        make_skill(
            &framework.join("skills/sddk-apply"),
            "sddk-apply",
            "Apply SDD tasks",
        );
        make_skill(
            &framework.join("skills/sddk-design"),
            "sddk-design",
            "Design SDD solutions",
        );
        // _shared should be skipped.
        make_skill(
            &framework.join("skills/_shared"),
            "_shared",
            "Shared internal",
        );
        // skill-registry should be skipped.
        make_skill(
            &framework.join("skills/skill-registry"),
            "skill-registry",
            "Registry indexer",
        );

        // Pass framework as project_root so project-ID derives from the dir that holds the skills.
        let (path1, count1) = write_skill_registry(&env, &framework, &framework).unwrap();
        assert_eq!(
            count1, 2,
            "only sddk-apply and sddk-design should be included"
        );

        // Second invocation must produce byte-identical output (idempotent).
        let (path2, count2) = write_skill_registry(&env, &framework, &framework).unwrap();
        assert_eq!(count2, 2);
        let content1 = std::fs::read_to_string(&path1).unwrap();
        let content2 = std::fs::read_to_string(&path2).unwrap();
        assert_eq!(
            content1, content2,
            "second invocation must be byte-identical (idempotent)"
        );

        // Verify schema: table has 5 columns.
        assert!(
            content1.contains("| Name | Trigger | Description | Scope | Path |"),
            "registry must have correct header"
        );

        std::fs::remove_dir_all(&project).ok();
        std::fs::remove_dir_all(&framework).ok();
    }

    #[test]
    fn write_skill_registry_skips_non_skill_dirs() {
        let project = temp_project("skip");
        let framework = temp_framework("skip");
        let env = test_environment(&project);
        init_fake_git_remote(&framework);

        // Create a valid skill in framework scope.
        make_skill(
            &framework.join("skills/sddk-verify"),
            "sddk-verify",
            "Verify SDD implementation",
        );
        // A directory without SKILL.md should be skipped.
        std::fs::create_dir_all(framework.join("skills/sddk-incomplete")).unwrap();
        // A regular file in skills/ should be skipped.
        std::fs::write(framework.join("skills/README.md"), "# readme\n").unwrap();

        let (_, count) = write_skill_registry(&env, &framework, &framework).unwrap();
        assert_eq!(
            count, 1,
            "only sddk-verify with SKILL.md should be included"
        );

        std::fs::remove_dir_all(&project).ok();
        std::fs::remove_dir_all(&framework).ok();
    }

    #[test]
    fn write_skill_registry_project_skips_framework_when_empty() {
        // Skills from both scopes — no dedup needed, both appear.
        let project = temp_project("proj-only");
        let framework = temp_framework("proj-only");
        let env = test_environment(&project);
        init_fake_git_remote(&framework);

        // Project skill must be in a project-scope dir under project_root (framework here).
        make_skill(
            &framework.join(".opencode/skills/sddk-apply"),
            "sddk-apply",
            "Apply",
        );
        // Framework skill in the framework scope.
        make_skill(
            &framework.join("skills/sddk-design"),
            "sddk-design",
            "Design",
        );

        let (_, count) = write_skill_registry(&env, &framework, &framework).unwrap();
        assert_eq!(count, 2, "skills from both scopes should appear");

        std::fs::remove_dir_all(&project).ok();
        std::fs::remove_dir_all(&framework).ok();
    }

    #[test]
    fn write_skill_registry_project_wins_over_framework() {
        // Same skill name in project and framework scopes; project must win.
        let project = temp_project("precedence");
        let framework = temp_framework("precedence");
        let env = test_environment(&project);
        init_fake_git_remote(&framework);

        // Skill in framework scope at `framework/skills/sddk-apply`.
        make_skill(
            &framework.join("skills/sddk-apply"),
            "sddk-apply",
            "Framework apply skill",
        );
        // Same name in project scope — must be under project_root (framework here)
        // at a recognized project-scope path so project scope finds it first.
        make_skill(
            &framework.join(".opencode/skills/sddk-apply"),
            "sddk-apply",
            "Project-level apply skill",
        );

        let (_, count) = write_skill_registry(&env, &framework, &framework).unwrap();
        assert_eq!(count, 1, "only one sddk-apply should appear (project wins)");

        std::fs::remove_dir_all(&project).ok();
        std::fs::remove_dir_all(&framework).ok();
    }

    #[test]
    fn write_skill_registry_user_wins_over_framework() {
        // Same skill name in user and framework scopes; user must win.
        let project = temp_project("user-precedence");
        let framework = temp_framework("user-precedence");
        init_fake_git_remote(&framework);

        // Skill in framework scope.
        make_skill(
            &framework.join("skills/sddk-design"),
            "sddk-design",
            "Framework design skill",
        );
        // Same name in user scope should override.
        let fake_home = temp_project("user-home");
        make_skill(
            &fake_home.join(".config/opencode/skills/sddk-design"),
            "sddk-design",
            "User design skill",
        );

        let mut env_with_home = test_environment(&project);
        env_with_home.home = Some(fake_home.clone());
        let (_, count) = write_skill_registry(&env_with_home, &framework, &framework).unwrap();
        assert_eq!(count, 1, "only one sddk-design should appear (user wins)");

        std::fs::remove_dir_all(&project).ok();
        std::fs::remove_dir_all(&framework).ok();
        std::fs::remove_dir_all(&fake_home).ok();
    }

    #[test]
    fn write_skill_registry_empty_when_no_skills() {
        let project = temp_project("empty");
        let framework = temp_framework("empty");
        let env = test_environment(&project);
        init_fake_git_remote(&framework);

        let (_, count) = write_skill_registry(&env, &framework, &framework).unwrap();
        assert_eq!(count, 0, "no skills means empty registry");

        std::fs::remove_dir_all(&project).ok();
        std::fs::remove_dir_all(&framework).ok();
    }

    #[test]
    fn write_skill_registry_deterministic_p_id() {
        // Same project must produce the same p-* ID across calls.
        let project = temp_project("det-id");
        let framework = temp_framework("det-id");
        let env = test_environment(&project);
        init_fake_git_remote(&framework);

        make_skill(
            &framework.join("skills/sddk-apply"),
            "sddk-apply",
            "Test skill",
        );

        let (path1, _) = write_skill_registry(&env, &framework, &framework).unwrap();
        let (path2, _) = write_skill_registry(&env, &framework, &framework).unwrap();

        // The registry path contains the project ID; both calls must write to same location.
        assert_eq!(
            path1, path2,
            "same project_root must produce same registry path (same p-* ID)"
        );

        std::fs::remove_dir_all(&project).ok();
        std::fs::remove_dir_all(&framework).ok();
    }
}
