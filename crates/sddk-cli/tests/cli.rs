use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use sddk_cli::{
    GENERATED_INVENTORY_DOC, GENERATED_WORKFLOW_DOC, GenerationStatus, Severity,
    generate_inventory, generate_workflow_docs, lint_repository, run_from,
};
use sddk_testkit::TestRepository;
use tempfile::TempDir;

const WORKFLOW: &str = include_str!("fixtures/workflow.yaml");
const WORKFLOW_SCHEMA: &str = include_str!("fixtures/workflow.schema.json");
const DIAGNOSTICS: &str = include_str!("fixtures/diagnostics.md");
const REFERENCES: &str = include_str!("fixtures/references.yaml");

#[test]
fn fixture_diagnostics_have_stable_codes_and_locations() {
    let repository = repository_fixture();
    generate_workflow_docs(repository.path(), false).unwrap();
    write(repository.path().join("docs/check.md"), DIAGNOSTICS);

    let report = lint_repository(repository.path()).unwrap();
    let codes = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<Vec<_>>();

    assert_eq!(codes, ["SDDK001", "SDDK002", "SDDK003", "SDDK004"]);
    assert!(report.diagnostics.iter().all(|diagnostic| {
        diagnostic.severity == Severity::Error
            && diagnostic.file == "docs/check.md"
            && diagnostic.line.is_some()
            && !diagnostic.hint.is_empty()
    }));
}

#[test]
fn typed_yaml_references_cover_repository_owned_entities_and_paths() {
    let repository = repository_fixture();
    generate_workflow_docs(repository.path(), false).unwrap();
    write(repository.path().join("docs/references.yaml"), REFERENCES);

    let report = lint_repository(repository.path()).unwrap();
    let broken = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == "SDDK001")
        .collect::<Vec<_>>();

    assert_eq!(broken.len(), 5);
    for target in [
        "agents/missing-agent",
        "skills/missing-skill",
        "plugins/missing-plugin",
        "prompts/missing-prompt.md",
        "docs/missing-file.md",
    ] {
        assert!(
            broken
                .iter()
                .any(|diagnostic| diagnostic.hint.contains(target)),
            "missing diagnostic for {target}"
        );
    }
}

#[test]
fn workflow_topology_and_stale_docs_use_distinct_codes() {
    let repository = repository_fixture();
    generate_workflow_docs(repository.path(), false).unwrap();
    let invalid = WORKFLOW
        .replacen(
            "      - explore\n      - archive",
            "      - archive\n      - explore",
            1,
        )
        .replacen(
            "      - result\nartifacts:",
            "      - missing-artifact\nartifacts:",
            1,
        )
        .replacen("consumers:\n      - archiver", "consumers: []", 1);
    write(repository.path().join("workflow/workflow.yaml"), &invalid);

    let report = lint_repository(repository.path()).unwrap();
    let codes = report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect::<std::collections::BTreeSet<_>>();

    assert!(codes.contains("SDDK006"));
    assert!(codes.contains("SDDK007"));
    assert!(codes.contains("SDDK008"));
    assert!(codes.contains("SDDK009"));
}

#[test]
fn terminal_artifact_does_not_require_a_consumer() {
    let repository = repository_fixture();
    let workflow = WORKFLOW.replacen(
        "consumers:\n      - archiver\n    required: true",
        "consumers: []\n    required: true\n    terminal: true",
        1,
    );
    write(repository.path().join("workflow/workflow.yaml"), &workflow);
    generate_workflow_docs(repository.path(), false).unwrap();

    let report = lint_repository(repository.path()).unwrap();

    assert!(
        report
            .diagnostics
            .iter()
            .all(|diagnostic| diagnostic.code != "SDDK007")
    );
}

#[test]
fn json_output_is_structured_and_deterministic() {
    let repository = repository_fixture();
    generate_workflow_docs(repository.path(), false).unwrap();
    write(repository.path().join("docs/check.md"), DIAGNOSTICS);

    let first = run_from([
        "sddk",
        "lint",
        "--root",
        repository.path().to_str().unwrap(),
        "--format",
        "json",
    ]);
    let second = run_from([
        "sddk",
        "lint",
        "--root",
        repository.path().to_str().unwrap(),
        "--format",
        "json",
    ]);

    assert_eq!(first.status, 1);
    assert_eq!(first, second);
    let json: serde_json::Value = serde_json::from_str(&first.stdout).unwrap();
    assert_eq!(json["summary"]["errors"], 4);
    assert_eq!(json["summary"]["warnings"], 0);
    assert_eq!(json["diagnostics"][0]["code"], "SDDK001");
    assert_eq!(json["diagnostics"][0]["severity"], "error");
}

#[test]
fn generation_is_deterministic_and_contains_required_sections() {
    let repository = repository_fixture();

    assert_eq!(
        generate_workflow_docs(repository.path(), false).unwrap(),
        GenerationStatus::Written
    );
    let generated_path = repository.path().join(GENERATED_WORKFLOW_DOC);
    let first = fs::read_to_string(&generated_path).unwrap();
    assert_eq!(
        generate_workflow_docs(repository.path(), false).unwrap(),
        GenerationStatus::Written
    );
    let second = fs::read_to_string(&generated_path).unwrap();

    assert_eq!(first, second);
    for section in [
        "## Workflow Metadata",
        "## Statuses",
        "## Phases",
        "## Paths",
        "## Transitions",
        "## Artifacts",
        "## Gates",
        "```mermaid",
    ] {
        assert!(
            first.contains(section),
            "missing generated section {section}"
        );
    }
}

#[test]
fn check_never_writes_and_generation_atomically_replaces() {
    let repository = repository_fixture();
    let generated_path = repository.path().join(GENERATED_WORKFLOW_DOC);

    assert_eq!(
        generate_workflow_docs(repository.path(), true).unwrap(),
        GenerationStatus::Stale
    );
    assert!(!generated_path.exists());

    write(&generated_path, "stale\n");
    assert_eq!(
        generate_workflow_docs(repository.path(), true).unwrap(),
        GenerationStatus::Stale
    );
    assert_eq!(fs::read_to_string(&generated_path).unwrap(), "stale\n");

    assert_eq!(
        generate_workflow_docs(repository.path(), false).unwrap(),
        GenerationStatus::Written
    );
    assert_eq!(
        generate_workflow_docs(repository.path(), true).unwrap(),
        GenerationStatus::Current
    );
    let generated_dir = generated_path.parent().unwrap();
    assert!(fs::read_dir(generated_dir).unwrap().all(|entry| {
        !entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .contains(".tmp-")
    }));
}

#[test]
fn inventory_is_sorted_deterministic_and_checked_by_lint() {
    let repository = repository_fixture();
    repository.write("agents/zeta.md", "# Zeta\n").unwrap();
    repository.write("agents/alpha.md", "# Alpha\n").unwrap();
    repository
        .write("skills/example/SKILL.md", "# Example\n")
        .unwrap();

    assert_eq!(
        generate_inventory(repository.path(), false).unwrap(),
        GenerationStatus::Written
    );
    let generated = fs::read_to_string(repository.path().join(GENERATED_INVENTORY_DOC)).unwrap();
    assert!(generated.contains("| Agents | 2 |"));
    assert!(generated.contains("| Skills | 1 |"));
    assert!(generated.find("agents/alpha.md") < generated.find("agents/zeta.md"));
    assert_eq!(
        generate_inventory(repository.path(), true).unwrap(),
        GenerationStatus::Current
    );

    repository.write("agents/new.md", "# New\n").unwrap();
    generate_workflow_docs(repository.path(), false).unwrap();
    let report = lint_repository(repository.path()).unwrap();
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SDDK010")
    );
}

#[test]
fn real_cli_exit_status_tracks_lint_errors_and_stale_checks() {
    let repository = repository_fixture();
    let binary = env!("CARGO_BIN_EXE_sddk");

    let stale = Command::new(binary)
        .args(["generate", "docs", "--root"])
        .arg(repository.path())
        .arg("--check")
        .output()
        .unwrap();
    assert_eq!(stale.status.code(), Some(1));
    assert!(
        String::from_utf8(stale.stderr)
            .unwrap()
            .contains("missing or stale")
    );

    let generated = Command::new(binary)
        .args(["generate", "docs", "--root"])
        .arg(repository.path())
        .output()
        .unwrap();
    assert!(generated.status.success());

    let clean = Command::new(binary)
        .args(["lint", "--root"])
        .arg(repository.path())
        .output()
        .unwrap();
    assert!(
        clean.status.success(),
        "{}",
        String::from_utf8_lossy(&clean.stdout)
    );

    write(repository.path().join("docs/check.md"), DIAGNOSTICS);
    let invalid = Command::new(binary)
        .args(["lint", "--root"])
        .arg(repository.path())
        .output()
        .unwrap();
    assert_eq!(invalid.status.code(), Some(1));
    assert!(
        String::from_utf8(invalid.stdout)
            .unwrap()
            .contains("SDDK001")
    );
}

#[test]
fn project_resolve_json_canonicalizes_equivalent_remotes() {
    let fixture = CliFixture::new("project-resolve");
    let https = fixture.run(&[
        "project",
        "resolve",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://Example.COM/acme/repo.git",
        "--format",
        "json",
    ]);
    let ssh = fixture.run(&[
        "project",
        "resolve",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "git@example.com:acme/repo.git",
        "--format",
        "json",
    ]);

    assert!(
        https.status.success(),
        "{}",
        String::from_utf8_lossy(&https.stderr)
    );
    assert!(
        ssh.status.success(),
        "{}",
        String::from_utf8_lossy(&ssh.stderr)
    );
    let https: serde_json::Value = serde_json::from_slice(&https.stdout).unwrap();
    let ssh: serde_json::Value = serde_json::from_slice(&ssh.stdout).unwrap();
    assert_eq!(https["project_id"], ssh["project_id"]);
    assert_eq!(https["workspace_id"], ssh["workspace_id"]);
    assert_eq!(https["remote_url"], "https://example.com/acme/repo");
}

#[test]
fn adopt_json_exit_status_tracks_absent_complete_and_replay() {
    let fixture = CliFixture::new("adopt-remote");
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ];

    let absent = fixture.run_adopt("status", &common);
    assert_eq!(absent.status.code(), Some(1));
    let absent_json: serde_json::Value = serde_json::from_slice(&absent.stdout).unwrap();
    assert_eq!(absent_json["status"], "absent");

    let applied = fixture.run_adopt("apply", &common);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let applied_json: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(applied_json["status"], "complete");

    let replayed = fixture.run_adopt("apply", &common);
    assert!(replayed.status.success());
    assert_eq!(replayed.stdout, applied.stdout);

    let status = fixture.run_adopt("status", &common);
    assert!(status.status.success());
    let status_json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status_json["status"], "complete");
}

#[test]
fn fallback_apply_persists_seed_for_status_without_override() {
    let fixture = CliFixture::new("adopt-fallback");
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ];
    let applied = fixture.run_adopt("apply", &common);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let applied_json: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    let seed = applied_json["receipt"]["fallback_seed"]
        .as_str()
        .unwrap()
        .to_owned();

    let status = fixture.run_adopt("status", &common);
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let status_json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status_json["status"], "complete");
    assert_eq!(status_json["receipt"]["fallback_seed"], seed);
}

#[test]
fn repair_restores_missing_receipt_and_status_reports_corruption() {
    let fixture = CliFixture::new("adopt-repair");
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ];
    let applied = fixture.run_adopt("apply", &common);
    let applied_json: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    let receipt = PathBuf::from(applied_json["receipt_path"].as_str().unwrap());
    fs::remove_file(&receipt).unwrap();

    let partial = fixture.run_adopt("status", &common);
    assert_eq!(partial.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&partial.stdout).unwrap()["status"],
        "ledger_only"
    );
    let repaired = fixture.run_adopt("repair", &common);
    assert!(
        repaired.status.success(),
        "{}",
        String::from_utf8_lossy(&repaired.stderr)
    );

    fs::write(&receipt, "{broken\n").unwrap();
    let corrupt = fixture.run_adopt("status", &common);
    assert_eq!(corrupt.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&corrupt.stdout).unwrap()["status"],
        "corrupt"
    );
}

fn repository_fixture() -> TestRepository {
    let repository = TestRepository::new().unwrap();
    repository
        .write("workflow/workflow.yaml", WORKFLOW)
        .unwrap();
    repository
        .write("schemas/workflow.schema.json", WORKFLOW_SCHEMA)
        .unwrap();
    repository.write("target/ignored.md", DIAGNOSTICS).unwrap();
    repository.write(".git/ignored.md", DIAGNOSTICS).unwrap();
    repository.write("supplied-input.zip", DIAGNOSTICS).unwrap();
    generate_inventory(repository.path(), false).unwrap();
    repository
}

fn write(path: impl Into<PathBuf>, content: &str) {
    let path = path.into();
    fs::create_dir_all(path.parent().unwrap_or_else(|| Path::new("."))).unwrap();
    fs::write(path, content).unwrap();
}

struct CliFixture {
    _directory: TempDir,
    root: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
}

impl CliFixture {
    fn new(name: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join(name);
        fs::create_dir_all(&root).unwrap();
        Self {
            root,
            data: directory.path().join("data"),
            state: directory.path().join("state"),
            cache: directory.path().join("cache"),
            _directory: directory,
        }
    }

    fn run(&self, args: &[&str]) -> std::process::Output {
        let mut command = Command::new(env!("CARGO_BIN_EXE_sddk"));
        command
            .args(args)
            .env_remove("HOME")
            .env("XDG_DATA_HOME", &self.data)
            .env("XDG_STATE_HOME", &self.state)
            .env("XDG_CACHE_HOME", &self.cache);
        command.output().unwrap()
    }

    fn run_adopt(&self, operation: &str, common: &[&str]) -> std::process::Output {
        let mut args = vec!["adopt", operation];
        args.extend_from_slice(common);
        self.run(&args)
    }
}
