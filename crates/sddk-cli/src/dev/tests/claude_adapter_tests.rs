//! Tests for the Claude Code adapter (I6, I7, claude halves of I10/I11).

use super::ClaudeAdapter;
use crate::dev::editor_adapters::EditorAdapter;
use crate::dev::editor_adapters::RegistrationContext;
use crate::dev::editor_adapters::test_fixtures::{self, ctx};

fn register_into(
    fixture: &test_fixtures::Fixture,
    dir: &std::path::Path,
) -> super::super::AdapterReport {
    let adapter = ClaudeAdapter {
        dir: dir.to_path_buf(),
    };
    let context = ctx(fixture, Some(&fixture.models));
    adapter.register(&context)
}

// I6 — claude file written with tier-mapped model.
#[test]
fn claude_writes_tier_mapped_model() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.registered, 3, "{:?}", report.errors);
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let premium = std::fs::read_to_string(dir.path().join("agents/orchestrator.md")).unwrap();
    assert!(premium.starts_with("---\nname: orchestrator\n"));
    assert!(premium.contains("description: Team coordinator\n"));
    assert!(premium.contains("model: sonnet\n"), "{premium}");
    assert!(premium.ends_with("# Orchestrator body\n"));
    assert!(
        !premium.contains("tools:"),
        "no tools in frontmatter for orchestrator"
    );

    let with_tools = std::fs::read_to_string(dir.path().join("agents/sddk-foo.md")).unwrap();
    assert!(with_tools.contains("tools: read, bash\n"), "{with_tools}");
    assert!(with_tools.contains("model: haiku\n"), "fast tier default");

    let body = std::fs::read_to_string(dir.path().join("agents/gentle-bar.md")).unwrap();
    assert!(body.ends_with("# Bar body\n"));
}

// I7 — invalid claude model reported and skipped.
#[test]
fn claude_invalid_model_reported_and_skipped() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let yaml = test_fixtures::FIXTURE_YAML.replace("claude: sonnet", "claude: foo-bar");
    let models = crate::dev::agent_models::AgentModelsConfig::from_yaml(&yaml).unwrap();
    let context = ctx(&fixture, Some(&models));
    let adapter = ClaudeAdapter {
        dir: dir.path().to_path_buf(),
    };
    let report = adapter.register(&context);
    assert_eq!(report.skipped_unresolved, 1, "orchestrator skipped");
    assert_eq!(report.registered, 2);
    assert_eq!(report.errors.len(), 1);
    assert!(report.errors[0].contains("foo-bar"), "{:?}", report.errors);
    assert!(
        report.errors[0].contains("orchestrator"),
        "{:?}",
        report.errors
    );
    assert!(!dir.path().join("agents/orchestrator.md").exists());
}

// I10 (claude half) — first-time only: pre-existing file untouched byte-identical.
#[test]
fn claude_first_time_only() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("agents")).unwrap();
    let target = dir.path().join("agents/orchestrator.md");
    std::fs::write(
        &target,
        "---\nname: orchestrator\ndescription: user edited\nmodel: opus\n---\n# User body\n",
    )
    .unwrap();
    let before = std::fs::read(&target).unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.skipped_existing, 1);
    assert_eq!(report.registered, 2);
    assert_eq!(
        std::fs::read(&target).unwrap(),
        before,
        "user file must be untouched"
    );
}

// I11 (claude half) — prune framework-namespaced orphans, keep user files.
#[test]
fn claude_prune_namespace_files() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("agents")).unwrap();
    std::fs::write(dir.path().join("agents/sddk-zombie.md"), "stale").unwrap();
    std::fs::write(dir.path().join("agents/my-agent.md"), "user").unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.pruned, 1);
    assert!(!dir.path().join("agents/sddk-zombie.md").exists());
    assert!(
        dir.path().join("agents/my-agent.md").exists(),
        "user file kept"
    );
}

// ConfigAbsent (claude): omit the `model:` line entirely.
#[test]
fn claude_config_absent_omits_model_line() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let adapter = ClaudeAdapter {
        dir: dir.path().to_path_buf(),
    };
    let context = RegistrationContext {
        root: fixture.root.path(),
        agents: &fixture.agents,
        models: None,
    };
    let report = adapter.register(&context);
    assert_eq!(report.registered, 3);
    let content = std::fs::read_to_string(dir.path().join("agents/orchestrator.md")).unwrap();
    assert!(!content.contains("model:"), "{content}");
}
