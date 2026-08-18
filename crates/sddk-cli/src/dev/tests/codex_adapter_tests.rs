//! Tests for the Codex adapter (I8, I9, codex halves of I10/I11).

use super::CodexAdapter;
use crate::dev::editor_adapters::test_fixtures::{self, ctx};
use crate::dev::editor_adapters::{EditorAdapter, RegistrationContext};

fn register_into(
    fixture: &test_fixtures::Fixture,
    dir: &std::path::Path,
) -> crate::dev::editor_adapters::AdapterReport {
    let adapter = CodexAdapter {
        dir: dir.to_path_buf(),
    };
    let context = ctx(fixture, Some(&fixture.models));
    adapter.register(&context)
}

// I8 — toml written from md body; adversarial content survives escaping.
#[test]
fn codex_toml_from_md_body() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.registered, 3, "{:?}", report.errors);
    assert!(report.errors.is_empty(), "{:?}", report.errors);

    let parsed: toml::Value =
        toml::from_str(&std::fs::read_to_string(dir.path().join("agents/sddk-foo.toml")).unwrap())
            .unwrap();
    assert_eq!(parsed["name"], toml::Value::String("sddk-foo".into()));
    assert_eq!(
        parsed["description"],
        toml::Value::String("Foo explorer".into())
    );
    assert_eq!(
        parsed["model"],
        toml::Value::String("openai/gpt-5.4-fast".into())
    );
    assert_eq!(
        parsed["developer_instructions"],
        toml::Value::String("\n# Foo body\n".into())
    );

    // Adversarial body: newlines, quotes, backslashes, hashes, emoji.
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(root.path().join("agents")).unwrap();
    let body = "# Section\n\"double\" 'single' \\ backslash\n# hash line\nemoji: \u{1f680}\nline\twith\ttabs\n";
    std::fs::write(
        root.path().join("agents/sddk-foo.md"),
        format!("---\nname: sddk-foo\ndescription: Foo\n---\n{body}"),
    )
    .unwrap();
    let sources = crate::dev::editor_adapters::load_agent_sources(root.path());
    let context = RegistrationContext {
        root: root.path(),
        agents: &sources,
        models: Some(&fixture.models),
    };
    let out_dir = tempfile::tempdir().unwrap();
    let adapter = CodexAdapter {
        dir: out_dir.path().to_path_buf(),
    };
    let report = adapter.register(&context);
    assert!(report.errors.is_empty(), "{:?}", report.errors);
    let written = std::fs::read_to_string(out_dir.path().join("agents/sddk-foo.toml")).unwrap();
    let reparsed: toml::Value = toml::from_str(&written).unwrap();
    assert_eq!(
        reparsed["developer_instructions"],
        toml::Value::String(format!("\n{body}")),
        "body must round-trip exactly through TOML escaping"
    );
}

// I9 — unsupported codex fields are omitted.
#[test]
fn codex_omits_unsupported_fields() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    register_into(&fixture, dir.path());
    let raw = std::fs::read_to_string(dir.path().join("agents/orchestrator.toml")).unwrap();
    assert!(!raw.contains("model_reasoning_effort"), "{raw}");
    assert!(!raw.contains("model_reasoning_summary"), "{raw}");
    let parsed: toml::Value = toml::from_str(&raw).unwrap();
    let keys: Vec<&str> = parsed
        .as_table()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec!["description", "developer_instructions", "model", "name"]
    );
}

// I10 (codex half) — first-time only: pre-existing file untouched.
#[test]
fn codex_first_time_only() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("agents")).unwrap();
    let target = dir.path().join("agents/orchestrator.toml");
    std::fs::write(
        &target,
        "name = \"orchestrator\"\ndescription = \"user edited\"\n",
    )
    .unwrap();
    let before = std::fs::read(&target).unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.skipped_existing, 1);
    assert_eq!(report.registered, 2);
    assert_eq!(std::fs::read(&target).unwrap(), before);
}

// I11 (codex half) — prune framework-namespaced orphans, keep user files.
#[test]
fn codex_prune_namespace_files() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("agents")).unwrap();
    std::fs::write(dir.path().join("agents/sddk-zombie.toml"), "stale").unwrap();
    std::fs::write(dir.path().join("agents/my-agent.toml"), "user").unwrap();
    let report = register_into(&fixture, dir.path());
    assert_eq!(report.pruned, 1);
    assert!(!dir.path().join("agents/sddk-zombie.toml").exists());
    assert!(dir.path().join("agents/my-agent.toml").exists());
}

// ConfigAbsent (codex): omit the `model` key.
#[test]
fn codex_config_absent_omits_model_key() {
    let fixture = test_fixtures::build();
    let dir = tempfile::tempdir().unwrap();
    let context = RegistrationContext {
        root: fixture.root.path(),
        agents: &fixture.agents,
        models: None,
    };
    let adapter = CodexAdapter {
        dir: dir.path().to_path_buf(),
    };
    let report = adapter.register(&context);
    assert_eq!(report.registered, 3);
    let parsed: toml::Value = toml::from_str(
        &std::fs::read_to_string(dir.path().join("agents/orchestrator.toml")).unwrap(),
    )
    .unwrap();
    assert!(parsed.get("model").is_none());
}
