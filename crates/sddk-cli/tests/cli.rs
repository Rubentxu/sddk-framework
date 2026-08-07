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
const CANONICAL_WORKFLOW: &str = include_str!("../../../workflow/workflow.yaml");

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
fn agent_registry_checks_cover_declaration_orphans_and_names() {
    let repository = repository_fixture();
    generate_workflow_docs(repository.path(), false).unwrap();
    repository
        .write(
            "agents/declared-agent.md",
            "---\nname: declared-agent\n---\n# Agent\n",
        )
        .unwrap();
    repository
        .write(
            "agents/mismatch.md",
            "---\nname: other-name\n---\n# Agent\n",
        )
        .unwrap();
    repository
        .write(
            "permissions.yaml",
            "agents:\n  declared-agent:\n    phases: []\n    capabilities: []\n  orphan-agent:\n    phases: []\n    capabilities: []\n",
        )
        .unwrap();

    let report = lint_repository(repository.path()).unwrap();
    let by_code = |code: &str| {
        report
            .diagnostics
            .iter()
            .filter(|diagnostic| diagnostic.code == code)
            .collect::<Vec<_>>()
    };
    assert_eq!(by_code("SDDK011").len(), 1);
    assert_eq!(by_code("SDDK012").len(), 1);
    assert_eq!(by_code("SDDK013").len(), 1);
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

#[test]
fn adopt_apply_is_non_intrusive_and_does_not_plant_workflow() {
    let fixture = CliFixture::new("adopt-plants-workflow");
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
    let applied = fixture.run_adopt("apply", &common);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let planted = fixture.root.join("workflow/workflow.yaml");
    assert!(
        !planted.exists(),
        "adopt apply must NOT write framework files into the project repo (ADR-0011)"
    );
}

#[test]
fn adopt_apply_preserves_existing_custom_workflow_manifest() {
    let fixture = CliFixture::new("adopt-preserves-workflow");
    let custom = "schema_version: 1\nworkflow:\n  id: project-custom\n  version: 9.9.9\n";
    write(fixture.root.join("workflow/workflow.yaml"), custom);
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
    let applied = fixture.run_adopt("apply", &common);
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    assert_eq!(
        fs::read_to_string(fixture.root.join("workflow/workflow.yaml")).unwrap(),
        custom,
        "adopt apply must not overwrite a project-specific manifest"
    );
}

#[test]
fn cycle_start_falls_back_to_embedded_workflow_when_manifest_absent() {
    let fixture = CliFixture::new("cycle-embedded-workflow");
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
    let adopted = fixture.run_adopt("apply", &common);
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );
    // Non-intrusive (ADR-0011): adopt never creates workflow/ in the repo, so
    // the embedded canonical workflow is the only source here.

    let started = fixture.run(&[
        "cycle",
        "start",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--name",
        "add-auth",
        "--path",
        "a-full",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--lease-owner",
        "agent-a",
        "--lease-ms",
        "3600000",
        "--format",
        "json",
    ]);
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    let started_json: serde_json::Value = serde_json::from_slice(&started.stdout).unwrap();
    assert_eq!(started_json["status"], "OPEN");
}

#[test]
fn cli_walks_cycle_with_fencing_and_rebuilds_state() {
    let fixture = CliFixture::new("cycle-authority");

    let adopted = fixture.run_adopt(
        "apply",
        &[
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
        ],
    );
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );

    let started = fixture.run(&[
        "cycle",
        "start",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--name",
        "add-auth",
        "--path",
        "a-full",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--lease-owner",
        "agent-a",
        "--lease-ms",
        "3600000",
        "--format",
        "json",
    ]);
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    let started_json: serde_json::Value = serde_json::from_slice(&started.stdout).unwrap();
    let cycle_id = started_json["cycle_id"].as_str().unwrap().to_owned();
    assert_eq!(started_json["status"], "OPEN");
    assert_eq!(started_json["phase"], "explore");
    assert_eq!(started_json["lease"]["owner"], "agent-a");
    assert_eq!(started_json["lease"]["fencing_token"], 1);

    let status = fixture.run(&[
        "cycle",
        "status",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        &cycle_id,
        "--format",
        "json",
    ]);
    assert!(status.status.success());
    let status_json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    assert_eq!(status_json["status"], "OPEN");
    assert_eq!(status_json["path"], "A-full");

    let evaluated = fixture.run(&[
        "cycle",
        "evaluate-gate",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        &cycle_id,
        "--transition",
        "phase.explore.complete",
        "--gate",
        "exploration-sufficient",
        "--evaluator",
        "sddk.cli",
        "--evidence",
        r#"{"checked": true}"#,
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ]);
    assert!(
        evaluated.status.success(),
        "{}",
        String::from_utf8_lossy(&evaluated.stderr)
    );
    let evaluated_json: serde_json::Value = serde_json::from_slice(&evaluated.stdout).unwrap();
    let gate_receipt = evaluated_json["receipt_id"].as_str().unwrap().to_owned();
    assert_eq!(evaluated_json["gate"], "exploration-sufficient");

    let unfenced = fixture.run(&[
        "cycle",
        "transition",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        &cycle_id,
        "--transition",
        "phase.explore.complete",
        "--artifact",
        "exploration-report=artifacts/exploration.md",
        "--gate-receipt",
        &gate_receipt,
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ]);
    assert_eq!(unfenced.status.code(), Some(1));

    let transitioned = fixture.run(&[
        "cycle",
        "transition",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        &cycle_id,
        "--transition",
        "phase.explore.complete",
        "--artifact",
        "exploration-report=artifacts/exploration.md",
        "--gate-receipt",
        &gate_receipt,
        "--lease-owner",
        "agent-a",
        "--fencing-token",
        "1",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ]);
    assert!(
        transitioned.status.success(),
        "{}",
        String::from_utf8_lossy(&transitioned.stderr)
    );
    let transition_json: serde_json::Value = serde_json::from_slice(&transitioned.stdout).unwrap();
    assert_eq!(transition_json["outcome"], "succeeded");
    assert_eq!(transition_json["phase"], "specify");
    assert_eq!(transition_json["sequence"], 2);

    let verified = fixture.run(&[
        "ledger",
        "verify",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--format",
        "json",
    ]);
    assert!(verified.status.success());
    let verify_json: serde_json::Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert_eq!(verify_json["event_count"], 2);

    let events = fixture.run(&[
        "ledger",
        "events",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--format",
        "json",
    ]);
    assert!(events.status.success());
    let events_json: serde_json::Value = serde_json::from_slice(&events.stdout).unwrap();
    assert_eq!(events_json.as_array().unwrap().len(), 2);
    let frames = events_json
        .as_array()
        .unwrap()
        .iter()
        .map(|event| event["frame_id"].as_str().unwrap())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(frames.len(), 2);

    let rebuilt = fixture.run(&[
        "cycle",
        "rebuild",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        &cycle_id,
        "--format",
        "json",
    ]);
    assert!(rebuilt.status.success());
    let rebuild_json: serde_json::Value = serde_json::from_slice(&rebuilt.stdout).unwrap();
    assert_eq!(rebuild_json["restored"], false);
    assert_eq!(rebuild_json["phase"], "specify");

    let released = fixture.run(&[
        "cycle",
        "lock",
        "release",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        &cycle_id,
        "--owner",
        "agent-a",
        "--fencing-token",
        "1",
        "--format",
        "json",
    ]);
    assert!(released.status.success());
    let release_json: serde_json::Value = serde_json::from_slice(&released.stdout).unwrap();
    assert_eq!(release_json["released"], true);
}

#[test]
fn cli_capability_gateway_enforces_policy_and_persists_receipts() {
    let fixture = CliFixture::new("capability-gateway");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    let common_root = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
    ];
    let adopted = fixture.run_adopt(
        "apply",
        &[
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
        ],
    );
    assert!(adopted.status.success());

    let denied = run_with_root(
        &fixture,
        &[
            "capability",
            "plan",
            "--capability",
            "shell.exec",
            "--program",
            "echo",
            "--format",
            "json",
        ],
        &common_root,
    );
    assert_eq!(denied.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&denied.stderr).contains("denied by policy"),
        "{}",
        String::from_utf8_lossy(&denied.stderr)
    );

    let unapproved = run_with_root(
        &fixture,
        &[
            "capability",
            "apply",
            "--capability",
            "git.delete_branch",
            "--program",
            "echo",
            "--format",
            "json",
        ],
        &common_root,
    );
    assert_eq!(unapproved.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&unapproved.stderr).contains("requires approval"),
        "{}",
        String::from_utf8_lossy(&unapproved.stderr)
    );

    let applied = run_with_root(
        &fixture,
        &[
            "capability",
            "apply",
            "--capability",
            "git.create_branch",
            "--program",
            "echo",
            "--arg",
            "feature/x",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ],
        &common_root,
    );
    assert!(
        applied.status.success(),
        "{}",
        String::from_utf8_lossy(&applied.stderr)
    );
    let applied_json: serde_json::Value = serde_json::from_slice(&applied.stdout).unwrap();
    assert_eq!(applied_json["status"], "succeeded");

    let approved = run_with_root(
        &fixture,
        &[
            "capability",
            "apply",
            "--capability",
            "git.delete_branch",
            "--program",
            "echo",
            "--arg",
            "feature/x",
            "--approve",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ],
        &common_root,
    );
    assert!(
        approved.status.success(),
        "{}",
        String::from_utf8_lossy(&approved.stderr)
    );
    let approved_json: serde_json::Value = serde_json::from_slice(&approved.stdout).unwrap();
    assert_eq!(approved_json["status"], "succeeded");

    let status = run_with_root(
        &fixture,
        &["capability", "status", "--format", "json"],
        &common_root,
    );
    assert!(status.status.success());
    let status_json: serde_json::Value = serde_json::from_slice(&status.stdout).unwrap();
    let receipts = status_json.as_array().unwrap();
    assert_eq!(receipts.len(), 2);
    let capabilities = receipts
        .iter()
        .map(|receipt| receipt["capability"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(capabilities.contains(&"git.create_branch"));
    assert!(capabilities.contains(&"git.delete_branch"));
}

#[test]
fn cli_metrics_record_aggregate_tuning_and_analytics() {
    let fixture = CliFixture::new("metrics-analytics");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );

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
    let adopted = fixture.run_adopt("apply", &common);
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );

    // Record two metrics entries: one first-pass PASS, one FAIL with corrections.
    let recorded = fixture.run(&[
        "metrics",
        "record",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        "p-1/cycle-alpha",
        "--verdict",
        "PASS",
        "--first-pass",
        "--cost",
        "1.5",
        "--format",
        "json",
    ]);
    assert!(
        recorded.status.success(),
        "{}",
        String::from_utf8_lossy(&recorded.stderr)
    );
    let recorded_json: serde_json::Value = serde_json::from_slice(&recorded.stdout).unwrap();
    assert_eq!(recorded_json["cycle_id"], "p-1/cycle-alpha");
    assert_eq!(recorded_json["verify_verdict"], "PASS");

    let recorded_fail = fixture.run(&[
        "metrics",
        "record",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        "p-1/cycle-beta",
        "--verdict",
        "FAIL",
        "--corrections",
        "3",
        "--cost",
        "4.0",
        "--format",
        "json",
    ]);
    assert!(
        recorded_fail.status.success(),
        "{}",
        String::from_utf8_lossy(&recorded_fail.stderr)
    );

    // Aggregate should show 2 samples, 0.5 first-pass rate, median cost 2.75.
    let aggregated = fixture.run(&[
        "metrics",
        "aggregate",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--window",
        "7d",
        "--format",
        "json",
    ]);
    assert!(
        aggregated.status.success(),
        "{}",
        String::from_utf8_lossy(&aggregated.stderr)
    );
    let aggregate_json: serde_json::Value = serde_json::from_slice(&aggregated.stdout).unwrap();
    assert_eq!(aggregate_json["sample_size"], 2);
    assert_eq!(aggregate_json["first_pass_success_rate"], 0.5);
    assert_eq!(aggregate_json["median_cost_usd"], 2.75);
    assert_eq!(aggregate_json["verdict_distribution"]["PASS"], 1);
    assert_eq!(aggregate_json["verdict_distribution"]["FAIL"], 1);

    // Tuning with sample < 3 should produce no recommendations.
    let tuned = fixture.run(&[
        "metrics",
        "tuning",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--format",
        "json",
    ]);
    assert!(
        tuned.status.success(),
        "{}",
        String::from_utf8_lossy(&tuned.stderr)
    );
    let tuning_json: serde_json::Value = serde_json::from_slice(&tuned.stdout).unwrap();
    assert_eq!(tuning_json["path_bias"], serde_json::Value::Null);
    assert_eq!(tuning_json["recommended_deepen"], serde_json::json!([]));

    // Analytics report (JSON) mirrors the aggregate.
    let report = fixture.run(&[
        "analytics",
        "report",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--window",
        "30d",
        "--format",
        "json",
    ]);
    assert!(
        report.status.success(),
        "{}",
        String::from_utf8_lossy(&report.stderr)
    );
    let report_json: serde_json::Value = serde_json::from_slice(&report.stdout).unwrap();
    assert_eq!(report_json["sample_size"], 2);
    assert_eq!(report_json["first_pass_success_rate"], 0.5);

    // Trends command renders both windows.
    let trends = fixture.run(&[
        "analytics",
        "trends",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--format",
        "json",
    ]);
    assert!(
        trends.status.success(),
        "{}",
        String::from_utf8_lossy(&trends.stderr)
    );
    let trends_json: serde_json::Value = serde_json::from_slice(&trends.stdout).unwrap();
    assert_eq!(trends_json["window_7d"]["sample_size"], 2);
    assert_eq!(trends_json["window_30d"]["sample_size"], 2);
}

#[test]
fn cli_metrics_record_upsert_enriches_with_tokens_and_coherence() {
    let fixture = CliFixture::new("metrics-upsert");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    let remote = "https://example.com/acme/repo.git";
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ];
    let adopted = fixture.run_adopt("apply", &common);
    assert!(adopted.status.success());

    // First record: derived/poor (no tokens).
    let first = fixture.run(&[
        "metrics",
        "record",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--cycle",
        "p-1/cycle-gamma",
        "--verdict",
        "PASS",
        "--first-pass",
        "--format",
        "json",
    ]);
    assert!(first.status.success());

    // Second record for the SAME cycle: upsert replaces, no duplicate row,
    // and enriches with tokens/model/coherence/costs.
    let enriched = fixture.run(&[
        "metrics",
        "record",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--cycle",
        "p-1/cycle-gamma",
        "--verdict",
        "PW",
        "--tokens",
        "200000",
        "--model",
        "mini-m2.7",
        "--coherence",
        "88",
        "--costs",
        r#"{"L1": 0.4, "L2": 1.1}"#,
        "--format",
        "json",
    ]);
    assert!(
        enriched.status.success(),
        "{}",
        String::from_utf8_lossy(&enriched.stderr)
    );
    let enriched_json: serde_json::Value = serde_json::from_slice(&enriched.stdout).unwrap();
    assert_eq!(enriched_json["cycle_id"], "p-1/cycle-gamma");
    assert_eq!(enriched_json["verify_verdict"], "PW");
    assert_eq!(enriched_json["tokens_used"], 200000);
    assert_eq!(enriched_json["teleological_coherence_pct"], 88.0);
    assert_eq!(enriched_json["costs"]["L1"], 0.4);
    assert_eq!(enriched_json["costs"]["L2"], 1.1);

    // Exactly one record for the cycle in the JSONL (upsert, no duplicates).
    let projects_dir = fixture.data.join("sddk/projects");
    let metrics_jsonl = std::fs::read_dir(&projects_dir)
        .unwrap()
        .filter_map(|entry| {
            let path = entry.ok()?.path().join("metrics/metrics.jsonl");
            path.exists().then_some(path)
        })
        .next()
        .expect("metrics.jsonl under the fixture data root");
    let jsonl = std::fs::read_to_string(metrics_jsonl).unwrap();
    let gamma_lines: Vec<&str> = jsonl
        .lines()
        .filter(|l| l.contains("cycle-gamma"))
        .collect();
    assert_eq!(gamma_lines.len(), 1, "upsert must not duplicate records");
    let last: serde_json::Value = serde_json::from_str(gamma_lines[0]).unwrap();
    assert_eq!(last["tokens_used"], 200000);
    assert_eq!(last["teleological_coherence_pct"], 88.0);
}

#[test]
fn cli_closing_cycle_auto_captures_metrics_record() {
    let fixture = CliFixture::new("auto-metrics");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    let remote = "https://example.com/acme/repo.git";
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ];

    let adopted = fixture.run_adopt("apply", &common);
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );

    // Start an A-lite cycle.
    let started = fixture.run(&[
        "cycle",
        "start",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--name",
        "auto-capture-test",
        "--path",
        "a-lite",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--lease-owner",
        "agent-a",
        "--lease-ms",
        "3600000",
        "--format",
        "json",
    ]);
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    let started_json: serde_json::Value = serde_json::from_slice(&started.stdout).unwrap();
    let cycle_id = started_json["cycle_id"].as_str().unwrap().to_owned();

    // Helper closures for gate + transition pairs.
    let evaluate = |transition: &str, gate: &str, evidence: &str| {
        fixture.run(&[
            "cycle",
            "evaluate-gate",
            "--root",
            fixture.root.to_str().unwrap(),
            "--scope",
            ".",
            "--remote",
            remote,
            "--cycle",
            &cycle_id,
            "--transition",
            transition,
            "--gate",
            gate,
            "--evaluator",
            "sddk.cli",
            "--evidence",
            evidence,
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ])
    };
    let transition = |transition: &str, artifacts: &[&str], receipts: &[&str]| {
        let mut args = vec![
            "cycle",
            "transition",
            "--root",
            fixture.root.to_str().unwrap(),
            "--scope",
            ".",
            "--remote",
            remote,
            "--cycle",
            &cycle_id,
            "--transition",
            transition,
            "--lease-owner",
            "agent-a",
            "--fencing-token",
            "1",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ];
        for artifact in artifacts {
            args.push("--artifact");
            args.push(artifact);
        }
        for receipt in receipts {
            args.push("--gate-receipt");
            args.push(receipt);
        }
        fixture.run(&args)
    };

    // explore.complete
    let receipt = evaluate(
        "phase.explore.complete",
        "exploration-sufficient",
        r#"{"ok":true}"#,
    );
    assert!(receipt.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt.stdout).unwrap();
    let receipt_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let step = transition(
        "phase.explore.complete",
        &["exploration-report=artifacts/exploration.md"],
        &[receipt_id.as_str()],
    );
    assert!(
        step.status.success(),
        "{}",
        String::from_utf8_lossy(&step.stderr)
    );

    // specify.complete (A-lite -> design)
    let receipt = evaluate(
        "phase.specify.complete",
        "requirements-testable",
        r#"{"ok":true}"#,
    );
    assert!(receipt.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt.stdout).unwrap();
    let receipt_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let step = transition(
        "phase.specify.complete",
        &["specification=artifacts/spec.md"],
        &[receipt_id.as_str()],
    );
    assert!(
        step.status.success(),
        "{}",
        String::from_utf8_lossy(&step.stderr)
    );

    // design.complete.a-lite (A-lite -> build)
    let receipt = evaluate(
        "phase.design.complete.a-lite",
        "architecture-consistent",
        r#"{"ok":true}"#,
    );
    assert!(receipt.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt.stdout).unwrap();
    let receipt_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let step = transition(
        "phase.design.complete.a-lite",
        &["design=artifacts/design.md"],
        &[receipt_id.as_str()],
    );
    assert!(
        step.status.success(),
        "{}",
        String::from_utf8_lossy(&step.stderr)
    );

    // build.complete
    let receipt = evaluate(
        "phase.build.complete",
        "implementation-complete",
        r#"{"ok":true}"#,
    );
    assert!(receipt.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt.stdout).unwrap();
    let receipt_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let step = transition(
        "phase.build.complete",
        &["implementation-receipt=artifacts/receipt.md"],
        &[receipt_id.as_str()],
    );
    assert!(
        step.status.success(),
        "{}",
        String::from_utf8_lossy(&step.stderr)
    );

    // verify.complete.a-lite (A-lite -> RELEASE_PENDING) with two gates
    let receipt_pass = evaluate(
        "phase.verify.complete.a-lite",
        "tests-pass",
        r#"{"ok":true}"#,
    );
    assert!(receipt_pass.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt_pass.stdout).unwrap();
    let receipt_pass_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let receipt_policy = evaluate(
        "phase.verify.complete.a-lite",
        "policy-compliant",
        r#"{"ok":true}"#,
    );
    assert!(receipt_policy.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt_policy.stdout).unwrap();
    let receipt_policy_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let step = transition(
        "phase.verify.complete.a-lite",
        &["verification-report=artifacts/verify.md"],
        &[receipt_pass_id.as_str(), receipt_policy_id.as_str()],
    );
    assert!(
        step.status.success(),
        "{}",
        String::from_utf8_lossy(&step.stderr)
    );

    // release.complete -> RELEASED
    let receipt = evaluate("release.complete", "no-pending-effects", r#"{"ok":true}"#);
    assert!(receipt.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt.stdout).unwrap();
    let receipt_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let step = transition(
        "release.complete",
        &["merge-receipt=main", "release-receipt=v0.0.1"],
        &[receipt_id.as_str()],
    );
    assert!(
        step.status.success(),
        "{}",
        String::from_utf8_lossy(&step.stderr)
    );

    // archive.complete -> CLOSED (auto-capture fires here)
    let receipt_valid = evaluate("archive.complete", "ledger-valid", r#"{"ok":true}"#);
    assert!(receipt_valid.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt_valid.stdout).unwrap();
    let receipt_valid_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let receipt_vault = evaluate("archive.complete", "vault-index-current", r#"{"ok":true}"#);
    assert!(receipt_vault.status.success());
    let gate_json: serde_json::Value = serde_json::from_slice(&receipt_vault.stdout).unwrap();
    let receipt_vault_id = gate_json["receipt_id"].as_str().unwrap().to_owned();
    let closed = transition(
        "archive.complete",
        &["archive-manifest=artifacts/archive.md"],
        &[receipt_valid_id.as_str(), receipt_vault_id.as_str()],
    );
    assert!(
        closed.status.success(),
        "{}",
        String::from_utf8_lossy(&closed.stderr)
    );
    let closed_json: serde_json::Value = serde_json::from_slice(&closed.stdout).unwrap();
    assert_eq!(closed_json["status"], "CLOSED");

    // Auto-capture: metrics record for the cycle must exist with path a-lite.
    let project_id = cycle_id.split('/').next().unwrap();
    let metrics_dir = fixture
        .data
        .join("sddk/projects")
        .join(project_id)
        .join("metrics");
    let jsonl = fs::read_to_string(metrics_dir.join("metrics.jsonl")).unwrap();
    let record: serde_json::Value = jsonl
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        .find(|record| record["cycle_id"] == cycle_id)
        .expect("auto-captured metrics record for the closed cycle");
    assert_eq!(record["path"], "a-lite");
    assert_eq!(record["verify_verdict"], "PASS");
    assert_eq!(record["tag_version"], "v0.0.1");
    assert_eq!(record["first_pass_success"], true);
    assert_eq!(record["correction_cycles"], 0);
    let durations = record["phase_durations_sec"].as_object().unwrap();
    assert!(
        !durations.is_empty(),
        "phase durations must be derived from ledger events"
    );
    assert!(
        record["lead_time_hours"].as_f64().is_some(),
        "lead time must be derived from created -> archive"
    );

    // Exactly one record for this cycle: capture appended once during close.
    let count = jsonl
        .lines()
        .filter(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .map(|record| record["cycle_id"] == cycle_id)
                .unwrap_or(false)
        })
        .count();
    assert_eq!(
        count, 1,
        "capture must append exactly one record per closed cycle"
    );
}

#[test]
fn cli_metrics_cost_tuning_band_and_backfill() {
    let fixture = CliFixture::new("metrics-v2");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
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
    let adopted = fixture.run_adopt("apply", &common);
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );

    // U3: cost estimation from tokens + model.
    let recorded = fixture.run(&[
        "metrics",
        "record",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        "p-1/cost-cycle",
        "--tokens",
        "1000000",
        "--model",
        "deepseek-v4-pro",
        "--format",
        "json",
    ]);
    assert!(
        recorded.status.success(),
        "{}",
        String::from_utf8_lossy(&recorded.stderr)
    );
    let record_json: serde_json::Value = serde_json::from_slice(&recorded.stdout).unwrap();
    assert_eq!(record_json["tokens_used"], 1000000);
    let cost = record_json["cost_estimate_usd"].as_f64().unwrap();
    assert!(
        (cost - 1.20).abs() < 1e-6,
        "cost should be 1.20 for deepseek-v4-pro, got {cost}"
    );

    // Record two more with different verdicts to move rate into the middle band (0.6-0.85).
    let second = fixture.run(&[
        "metrics",
        "record",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        "p-1/pass-cycle",
        "--verdict",
        "PASS",
        "--first-pass",
        "--format",
        "json",
    ]);
    assert!(second.status.success());
    let third = fixture.run(&[
        "metrics",
        "record",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        "p-1/pass-cycle-2",
        "--verdict",
        "PASS",
        "--first-pass",
        "--format",
        "json",
    ]);
    assert!(third.status.success());

    // U2: tuning with rate 2/3 = 0.67 (middle band) must recommend lens + A-lite bias.
    let tuned = fixture.run(&[
        "metrics",
        "tuning",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--format",
        "json",
    ]);
    assert!(
        tuned.status.success(),
        "{}",
        String::from_utf8_lossy(&tuned.stderr)
    );
    let tuning_json: serde_json::Value = serde_json::from_slice(&tuned.stdout).unwrap();
    assert_eq!(tuning_json["path_bias"], "A-lite");
    let lenses = tuning_json["recommended_lens"].as_array().unwrap();
    assert!(
        lenses.iter().any(|lens| lens == "test-quality"),
        "middle band should recommend test-quality lens: {lenses:?}"
    );

    // U4: backfill is a no-op when records are already enriched (PASS verdict).
    let backfilled = fixture.run(&[
        "metrics",
        "backfill",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--format",
        "json",
    ]);
    assert!(
        backfilled.status.success(),
        "{}",
        String::from_utf8_lossy(&backfilled.stderr)
    );
    let backfill_json: serde_json::Value = serde_json::from_slice(&backfilled.stdout).unwrap();
    assert_eq!(backfill_json.as_array().unwrap().len(), 0);
}

#[test]
fn cli_metrics_dedupe_merged_context_and_tuning_file() {
    let fixture = CliFixture::new("metrics-perfection");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
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
    let adopted = fixture.run_adopt("apply", &common);
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );
    let adopted_json: serde_json::Value = serde_json::from_slice(&adopted.stdout).unwrap();
    let project_id = adopted_json["project_id"].as_str().unwrap();

    let metrics_dir = fixture
        .data
        .join("sddk/projects")
        .join(project_id)
        .join("metrics");

    // U3: set-context persists an override for a cycle.
    let set_ctx = fixture.run(&[
        "metrics",
        "record",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--cycle",
        "p-1/ctx-cycle",
        "--set-context",
        "C0",
        "--format",
        "json",
    ]);
    assert!(set_ctx.status.success());
    let context_file = fs::read_to_string(metrics_dir.join("context.json")).unwrap();
    let context_json: serde_json::Value = serde_json::from_str(&context_file).unwrap();
    assert_eq!(context_json["p-1/ctx-cycle"], "C0");

    // U4: tuning writes tuning.md with the F3 block.
    let tuned = fixture.run(&[
        "metrics",
        "tuning",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--format",
        "json",
    ]);
    assert!(tuned.status.success());
    let tuning_md = fs::read_to_string(metrics_dir.join("tuning.md")).unwrap();
    assert!(
        tuning_md.contains("F3 Tuning"),
        "tuning.md should contain the F3 block header"
    );

    // U1 + U2: backfill dedupes records per cycle and derives merged from RELEASED.
    // (No closed cycles in this fixture, so backfill returns 0 but must not error.)
    let backfilled = fixture.run(&[
        "metrics",
        "backfill",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
        "--format",
        "json",
    ]);
    assert!(
        backfilled.status.success(),
        "{}",
        String::from_utf8_lossy(&backfilled.stderr)
    );
    let backfill_json: serde_json::Value = serde_json::from_slice(&backfilled.stdout).unwrap();
    assert_eq!(backfill_json.as_array().unwrap().len(), 0);
}

#[test]
fn cli_f3_tuning_influences_cycle_start_and_research_packet() {
    let fixture = CliFixture::new("f3-closed-loop");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    let remote = "https://example.com/acme/repo.git";
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--actor",
        "cli-test",
        "--format",
        "json",
    ];
    let adopted = fixture.run_adopt("apply", &common);
    assert!(
        adopted.status.success(),
        "{}",
        String::from_utf8_lossy(&adopted.stderr)
    );
    let adopted_json: serde_json::Value = serde_json::from_slice(&adopted.stdout).unwrap();
    let project_id = adopted_json["project_id"].as_str().unwrap();
    let metrics_dir = fixture
        .data
        .join("sddk/projects")
        .join(project_id)
        .join("metrics");

    // Record enough cycles (rate 1.0 > 0.85) so tuning recommends A-min.
    for (index, name) in ["alpha", "beta", "gamma"].iter().enumerate() {
        let recorded = fixture.run(&[
            "metrics",
            "record",
            "--root",
            fixture.root.to_str().unwrap(),
            "--scope",
            ".",
            "--remote",
            remote,
            "--cycle",
            &format!("{project_id}/{name}"),
            "--verdict",
            "PASS",
            "--first-pass",
            "--format",
            "json",
        ]);
        assert!(
            recorded.status.success(),
            "{index}: {}",
            String::from_utf8_lossy(&recorded.stderr)
        );
    }
    // Generate aggregate + tuning.
    let tuned = fixture.run(&[
        "metrics",
        "tuning",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--format",
        "json",
    ]);
    assert!(tuned.status.success());
    let tuning_md = fs::read_to_string(metrics_dir.join("tuning.md")).unwrap();
    assert!(
        tuning_md.contains("path_bias: A-min"),
        "rate 1.0 should recommend A-min, got: {tuning_md}"
    );

    // U1: cycle start WITHOUT --path uses the tuned path (A-min).
    let started = fixture.run(&[
        "cycle",
        "start",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--name",
        "tuned-cycle",
        "--lease-owner",
        "agent-a",
        "--lease-ms",
        "3600000",
        "--format",
        "json",
    ]);
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    let started_json: serde_json::Value = serde_json::from_slice(&started.stdout).unwrap();
    assert_eq!(started_json["path"], "A-min");

    // Explicit --path still wins.
    let explicit = fixture.run(&[
        "cycle",
        "start",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--name",
        "explicit-cycle",
        "--path",
        "a-full",
        "--lease-owner",
        "agent-a",
        "--lease-ms",
        "3600000",
        "--format",
        "json",
    ]);
    assert!(explicit.status.success());
    let explicit_json: serde_json::Value = serde_json::from_slice(&explicit.stdout).unwrap();
    assert_eq!(explicit_json["path"], "A-full");

    // U2: research packet contains aggregate + cycles + signals.
    let research = fixture.run(&[
        "analytics",
        "research",
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        remote,
        "--window",
        "30d",
        "--format",
        "json",
    ]);
    assert!(
        research.status.success(),
        "{}",
        String::from_utf8_lossy(&research.stderr)
    );
    let packet: serde_json::Value = serde_json::from_slice(&research.stdout).unwrap();
    assert_eq!(packet["aggregate"]["sample_size"], 3);
    assert_eq!(packet["cycles"].as_array().unwrap().len(), 3);
    assert!(
        packet["signals"]
            .as_array()
            .unwrap()
            .iter()
            .any(|signal| signal == "path_bias: A-min")
    );
}

#[test]
fn cli_git_operations_verify_postconditions_and_record_receipts() {
    let fixture = CliFixture::new("git-authority");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    for (key, value) in [("user.name", "SDDK Test"), ("user.email", "test@sddk.dev")] {
        std::process::Command::new("git")
            .args(["config", key, value])
            .current_dir(&fixture.root)
            .output()
            .unwrap();
    }
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
    ];
    let adopted = fixture.run_adopt(
        "apply",
        &[
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
        ],
    );
    assert!(adopted.status.success());

    let branch = run_with_root(
        &fixture,
        &[
            "git",
            "create-branch",
            "--name",
            "feat/cas",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        branch.status.success(),
        "{}",
        String::from_utf8_lossy(&branch.stderr)
    );
    let branch_json: serde_json::Value = serde_json::from_slice(&branch.stdout).unwrap();
    assert_eq!(branch_json["status"], "succeeded");
    assert_eq!(branch_json["result"]["branch"], "feat/cas");

    let unapproved_commit = run_with_root(
        &fixture,
        &["git", "commit", "--message", "wip", "--format", "json"],
        &common,
    );
    assert_eq!(unapproved_commit.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&unapproved_commit.stderr).contains("requires approval"),
        "{}",
        String::from_utf8_lossy(&unapproved_commit.stderr)
    );

    let commit = run_with_root(
        &fixture,
        &[
            "git",
            "commit",
            "--message",
            "initial",
            "--approve",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
    let commit_json: serde_json::Value = serde_json::from_slice(&commit.stdout).unwrap();
    assert_eq!(commit_json["status"], "succeeded");
    let sha = commit_json["result"]["sha"].as_str().unwrap().to_owned();

    let tag = run_with_root(
        &fixture,
        &[
            "git",
            "tag",
            "--name",
            "v0.1.0",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        tag.status.success(),
        "{}",
        String::from_utf8_lossy(&tag.stderr)
    );

    let inspect = run_with_root(&fixture, &["git", "inspect", "--format", "json"], &common);
    assert!(inspect.status.success());
    let inspect_json: serde_json::Value = serde_json::from_slice(&inspect.stdout).unwrap();
    assert_eq!(inspect_json["branch"], "feat/cas");
    assert_eq!(inspect_json["head"], sha);

    let receipts = run_with_root(
        &fixture,
        &["capability", "status", "--format", "json"],
        &common,
    );
    let receipts_json: serde_json::Value = serde_json::from_slice(&receipts.stdout).unwrap();
    assert_eq!(receipts_json.as_array().unwrap().len(), 3);
}

#[test]
fn cli_artifact_store_and_get_verify_digest() {
    let fixture = CliFixture::new("artifact-cas");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
    ];
    let adopted = fixture.run_adopt(
        "apply",
        &[
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
        ],
    );
    assert!(adopted.status.success());

    let source = fixture.root.join("report.md");
    fs::write(&source, "artifact payload\n").unwrap();
    let stored = run_with_root(
        &fixture,
        &[
            "artifact",
            "store",
            "--file",
            source.to_str().unwrap(),
            "--kind",
            "report",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        stored.status.success(),
        "{}",
        String::from_utf8_lossy(&stored.stderr)
    );
    let stored_json: serde_json::Value = serde_json::from_slice(&stored.stdout).unwrap();
    let digest = stored_json["sha256"].as_str().unwrap().to_owned();
    assert!(digest.starts_with("sha256:"));

    let destination = fixture.root.join("restored.md");
    let fetched = run_with_root(
        &fixture,
        &[
            "artifact",
            "get",
            "--digest",
            &digest,
            "--output",
            destination.to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        fetched.status.success(),
        "{}",
        String::from_utf8_lossy(&fetched.stderr)
    );
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        "artifact payload\n"
    );

    let missing = run_with_root(
        &fixture,
        &[
            "artifact",
            "get",
            "--digest",
            "sha256:0000000000000000000000000000000000000000000000000000000000000000",
            "--output",
            destination.to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert_eq!(missing.status.code(), Some(1));
}

#[test]
fn cli_validate_agent_result_and_legacy_conversion() {
    let fixture = CliFixture::new("agent-result");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    write(
        fixture.root.join("schemas/agent-result.schema.json"),
        include_str!("../../../schemas/agent-result.schema.json"),
    );
    write(
        fixture.root.join("schemas/artifact-ref.schema.json"),
        include_str!("../../../schemas/artifact-ref.schema.json"),
    );
    write(
        fixture.root.join("schemas/capability-request.schema.json"),
        include_str!("../../../schemas/capability-request.schema.json"),
    );
    write(
        fixture.root.join("schemas/cycle.schema.json"),
        include_str!("../../../schemas/cycle.schema.json"),
    );
    write(
        fixture.root.join("schemas/phase-result.schema.json"),
        include_str!("../../../schemas/phase-result.schema.json"),
    );
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
    ];
    let adopted = fixture.run_adopt(
        "apply",
        &[
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
        ],
    );
    assert!(adopted.status.success());

    let valid_file = fixture.root.join("valid-result.json");
    fs::write(
        &valid_file,
        r#"{"schema_version":1,"agent":"explorer","cycle_id":"cycle-1","phase":"explore","verdict":"completed","summary":"ok"}"#,
    )
    .unwrap();
    let valid = run_with_root(
        &fixture,
        &[
            "validate",
            "schema",
            "--schema",
            "agent-result",
            "--file",
            valid_file.to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert!(valid.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&valid.stdout).unwrap()["valid"],
        true
    );

    let invalid_file = fixture.root.join("invalid-result.json");
    fs::write(
        &invalid_file,
        r#"{"schema_version":1,"agent":"explorer","cycle_id":"cycle-1","phase":"explore","verdict":"maybe","summary":"ok"}"#,
    )
    .unwrap();
    let invalid = run_with_root(
        &fixture,
        &[
            "validate",
            "schema",
            "--schema",
            "agent-result",
            "--file",
            invalid_file.to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert_eq!(invalid.status.code(), Some(1));
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&invalid.stdout).unwrap()["valid"],
        false
    );

    let cycle_file = fixture.root.join("valid-cycle.json");
    fs::write(
        &cycle_file,
        r#"{"schema_version":1,"project_id":"p-1234","workspace_id":"w-1234","cycle_id":"cycle-1","display_name":"x","status":"OPEN","phase":"explore","path":"a-full","branch":"feat/x","base":"abc","head":null,"artifacts":{},"release":null,"remediation_round":0,"remote_url":null,"scope":null}"#,
    )
    .unwrap();
    let cycle_ok = run_with_root(
        &fixture,
        &[
            "validate",
            "schema",
            "--schema",
            "cycle",
            "--file",
            cycle_file.to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        cycle_ok.status.success(),
        "{}",
        String::from_utf8_lossy(&cycle_ok.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&cycle_ok.stdout).unwrap()["valid"],
        true
    );

    let phase_file = fixture.root.join("invalid-phase.json");
    fs::write(
        &phase_file,
        r#"{"schema_version":1,"cycle_id":"cycle-1","phase":"explore","success":true,"summary":"","timestamp":"2026-08-04T10:00:00Z"}"#,
    )
    .unwrap();
    let phase_bad = run_with_root(
        &fixture,
        &[
            "validate",
            "schema",
            "--schema",
            "phase-result",
            "--file",
            phase_file.to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert_eq!(phase_bad.status.code(), Some(1));

    let converted = run_with_root(
        &fixture,
        &[
            "agent-result",
            "convert",
            "--text",
            "Legacy summary",
            "--agent",
            "explorer",
            "--cycle",
            "cycle-1",
            "--phase",
            "explore",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(converted.status.success());
    let converted_json: serde_json::Value = serde_json::from_slice(&converted.stdout).unwrap();
    assert_eq!(converted_json["result"]["summary"], "Legacy summary");
    assert_eq!(converted_json["schema_errors"].as_array().unwrap().len(), 0);
    assert!(!converted_json["warnings"].as_array().unwrap().is_empty());

    let legacy_file = fixture.root.join("legacy.json");
    fs::write(
        &legacy_file,
        r#"{"status":"success","message":"done","artifacts":["a.md"]}"#,
    )
    .unwrap();
    let mapped = run_with_root(
        &fixture,
        &[
            "agent-result",
            "convert",
            "--file",
            legacy_file.to_str().unwrap(),
            "--agent",
            "explorer",
            "--cycle",
            "cycle-1",
            "--phase",
            "build",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(mapped.status.success());
    let mapped_json: serde_json::Value = serde_json::from_slice(&mapped.stdout).unwrap();
    assert_eq!(mapped_json["result"]["verdict"], "completed");
    assert_eq!(
        mapped_json["result"]["artifacts"].as_array().unwrap().len(),
        1
    );
}

#[test]
fn cli_permission_policy_enforces_default_deny() {
    let fixture = CliFixture::new("permissions");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    write(
        fixture.root.join("permissions.yaml"),
        r#"
agents:
  sdd-kernel-apply:
    phases: [build, verify]
    capabilities: [git.inspect, git.commit]
"#,
    );
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
    ];
    let adopted = fixture.run_adopt(
        "apply",
        &[
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
        ],
    );
    assert!(adopted.status.success());

    let allowed = run_with_root(
        &fixture,
        &[
            "permission",
            "check",
            "--agent",
            "sdd-kernel-apply",
            "--phase",
            "build",
            "--capability",
            "git.commit",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(allowed.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&allowed.stdout).unwrap()["allowed"],
        true
    );

    let denied = run_with_root(
        &fixture,
        &[
            "permission",
            "check",
            "--agent",
            "mystery-agent",
            "--phase",
            "build",
            "--capability",
            "git.commit",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(denied.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&denied.stdout).unwrap()["allowed"],
        false
    );

    let gated = run_with_root(
        &fixture,
        &[
            "capability",
            "apply",
            "--capability",
            "git.create_branch",
            "--program",
            "echo",
            "--agent",
            "sdd-kernel-apply",
            "--phase",
            "build",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ],
        &common,
    );
    assert_eq!(gated.status.code(), Some(1));
    assert!(
        String::from_utf8_lossy(&gated.stderr).contains("not allowed capability"),
        "{}",
        String::from_utf8_lossy(&gated.stderr)
    );

    let permitted = run_with_root(
        &fixture,
        &[
            "capability",
            "apply",
            "--capability",
            "git.commit",
            "--program",
            "echo",
            "--arg",
            "ok",
            "--approve",
            "--agent",
            "sdd-kernel-apply",
            "--phase",
            "build",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "cli-test",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        permitted.status.success(),
        "{}",
        String::from_utf8_lossy(&permitted.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&permitted.stdout).unwrap()["status"],
        "succeeded"
    );
}

#[test]
fn cli_release_plan_reports_canonical_sequence() {
    let fixture = CliFixture::new("release-plan");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
    ];
    let adopted = fixture.run_adopt(
        "apply",
        &[
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
        ],
    );
    assert!(adopted.status.success());

    let plan = run_with_root(
        &fixture,
        &[
            "release",
            "plan",
            "--repo",
            "acme/repo",
            "--branch",
            "feat/release",
            "--base",
            "main",
            "--title",
            "Release",
            "--tag",
            "v1.0.0",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        plan.status.success(),
        "{}",
        String::from_utf8_lossy(&plan.stderr)
    );
    let plan_json: serde_json::Value = serde_json::from_slice(&plan.stdout).unwrap();
    assert_eq!(plan_json["branch"], "feat/release");
    assert_eq!(plan_json["base"], "main");
    assert_eq!(plan_json["tag"], "v1.0.0");
    let steps = plan_json["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| step.as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(steps, vec!["create_pr", "merge_pr", "create_release"]);
}

#[test]
fn cli_vault_index_validate_search_and_export() {
    let fixture = CliFixture::new("vault");
    // Write canonical workflow so vault capability checks can load the policy.
    fs::create_dir_all(fixture.root.join("workflow")).unwrap();
    fs::write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    )
    .unwrap();
    let vault = fixture.root.join("vault");
    fs::create_dir_all(vault.join("terms")).unwrap();
    fs::create_dir_all(vault.join("adrs")).unwrap();
    fs::write(
        vault.join("terms/TERM-Auth.md"),
        "---\nid: TERM-Auth\ntype: term\nstatus: active\n---\n# Auth\n\nOAuth token exchange [[ADR-Auth]]\n",
    )
    .unwrap();
    fs::write(
        vault.join("adrs/ADR-Auth.md"),
        "---\nid: ADR-Auth\ntype: adr\n---\n# Auth Decision\n\nSee [[TERM-Auth]]\n",
    )
    .unwrap();
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--fallback-seed",
        "00000000-0000-0000-0000-000000000001",
    ];

    let indexed = run_with_root(
        &fixture,
        &[
            "vault",
            "index",
            "--vault",
            vault.to_str().unwrap(),
            "--db",
            fixture.root.join("index.sqlite").to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        indexed.status.success(),
        "{}",
        String::from_utf8_lossy(&indexed.stderr)
    );
    let indexed_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&indexed.stdout)).unwrap();
    assert_eq!(indexed_json["nodes"], 2);
    assert_eq!(indexed_json["errors"], 0);
    assert_eq!(indexed_json["backlinks"], 2);
    assert_eq!(indexed_json["inserted"], 2);
    assert_eq!(indexed_json["updated"], 0);

    let reindexed = run_with_root(
        &fixture,
        &[
            "vault",
            "index",
            "--vault",
            vault.to_str().unwrap(),
            "--db",
            fixture.root.join("index.sqlite").to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        reindexed.status.success(),
        "{}",
        String::from_utf8_lossy(&reindexed.stderr)
    );
    let reindexed_json: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&reindexed.stdout)).unwrap();
    assert_eq!(reindexed_json["inserted"], 0);
    assert_eq!(reindexed_json["updated"], 0);
    assert_eq!(reindexed_json["deleted"], 0);

    let searched = run_with_root(
        &fixture,
        &[
            "vault",
            "search",
            "--db",
            fixture.root.join("index.sqlite").to_str().unwrap(),
            "--query",
            "token",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(searched.status.success());
    let hits: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&searched.stdout)).unwrap();
    assert_eq!(hits.as_array().unwrap().len(), 1);
    assert_eq!(hits[0]["id"], "TERM-Auth");

    let graphed = run_with_root(
        &fixture,
        &[
            "vault",
            "graph",
            "--vault",
            vault.to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert!(graphed.status.success());
    let graph: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&graphed.stdout)).unwrap();
    assert_eq!(graph["node_count"], 2);
    assert_eq!(graph["edge_count"], 2);
    assert_eq!(graph["cyclic"], true);
    assert!(graph["sample_cycle"].is_array());

    let exported = run_with_root(
        &fixture,
        &[
            "vault",
            "export",
            "--vault",
            vault.to_str().unwrap(),
            "--output",
            fixture.root.join("inspector.html").to_str().unwrap(),
        ],
        &common,
    );
    assert!(exported.status.success());
    let html = fs::read_to_string(fixture.root.join("inspector.html")).unwrap();
    assert!(html.contains("SDDK Vault Inspector"));
    assert!(html.contains("TERM-Auth"));

    let broken = fixture.root.join("broken-vault");
    fs::create_dir_all(broken.join("terms")).unwrap();
    fs::write(
        broken.join("terms/TERM-X.md"),
        "---\nid: TERM-X\ntype: term\n---\n# X\n\n[[Ghost]]\n",
    )
    .unwrap();
    let validated = run_with_root(
        &fixture,
        &[
            "vault",
            "validate",
            "--vault",
            broken.to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert_eq!(validated.status.code(), Some(1));
    let validation: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&validated.stdout)).unwrap();
    assert_eq!(validation["errors"], 1);
    assert_eq!(validation["diagnostics"][0]["code"], "VAULT003");
}

#[test]
fn cli_dev_install_verify_uninstall_are_atomic() {
    let fixture = CliFixture::new("dev-install");
    let prefix = fixture.root.join("prefix");

    let installed = run_from([
        "sddk",
        "dev",
        "install",
        "--prefix",
        prefix.to_str().unwrap(),
        "--channel",
        "dev",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--format",
        "json",
    ]);
    assert_eq!(installed.status, 0, "{}", installed.stderr);
    let installed_json: serde_json::Value = serde_json::from_str(&installed.stdout).unwrap();
    assert_eq!(installed_json["channel"], "dev");
    assert!(prefix.join("bin/sddk").exists());
    assert!(prefix.join("sddk-install.json").exists());

    let verified = run_from([
        "sddk",
        "dev",
        "verify",
        "--prefix",
        prefix.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(verified.status, 0, "{}", verified.stderr);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&verified.stdout).unwrap()["valid"],
        true
    );

    let binary_path = prefix.join("bin/sddk");
    let mut bytes = fs::read(&binary_path).unwrap();
    bytes[0] ^= 0xFF;
    fs::write(&binary_path, &bytes).unwrap();
    let tampered = run_from([
        "sddk",
        "dev",
        "verify",
        "--prefix",
        prefix.to_str().unwrap(),
    ]);
    assert_eq!(tampered.status, 1);

    let refused = run_from([
        "sddk",
        "dev",
        "uninstall",
        "--prefix",
        prefix.to_str().unwrap(),
    ]);
    assert_eq!(refused.status, 1);

    let reinstalled = run_from([
        "sddk",
        "dev",
        "install",
        "--prefix",
        prefix.to_str().unwrap(),
        "--channel",
        "dev",
        "--timestamp",
        "2026-08-04T10:00:01Z",
    ]);
    assert_eq!(reinstalled.status, 0, "{}", reinstalled.stderr);
    let uninstalled = run_from([
        "sddk",
        "dev",
        "uninstall",
        "--prefix",
        prefix.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(uninstalled.status, 0, "{}", uninstalled.stderr);
    assert!(!binary_path.exists());
    assert!(!prefix.join("sddk-install.json").exists());
    assert_eq!(
        run_from([
            "sddk",
            "dev",
            "verify",
            "--prefix",
            prefix.to_str().unwrap()
        ])
        .status,
        1
    );
}

#[test]
fn cli_release_dist_and_verify_checksums_and_sbom() {
    let fixture = CliFixture::new("release-dist");
    let prefix = fixture.root.join("dist-prefix");

    let dist = run_from([
        "sddk",
        "release",
        "dist",
        "--prefix",
        prefix.to_str().unwrap(),
        "--channel",
        "release",
        "--timestamp",
        "2026-08-04T10:00:00Z",
        "--format",
        "json",
    ]);
    assert_eq!(dist.status, 0, "{}", dist.stderr);
    let dist_dir = prefix.join("dist");
    assert!(dist_dir.join("sddk").exists());
    assert!(dist_dir.join("checksums.txt").exists());
    assert!(dist_dir.join("sbom.json").exists());
    assert!(dist_dir.join("attestation.json").exists());
    let sbom: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(dist_dir.join("sbom.json")).unwrap()).unwrap();
    assert_eq!(sbom["tool"], "sddk");

    let verified = run_from([
        "sddk",
        "release",
        "verify",
        "--prefix",
        prefix.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(verified.status, 0, "{}", verified.stderr);
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&verified.stdout).unwrap()["valid"],
        true
    );

    fs::write(dist_dir.join("checksums.txt"), "tampered\n").unwrap();
    let broken = run_from([
        "sddk",
        "release",
        "verify",
        "--prefix",
        prefix.to_str().unwrap(),
    ]);
    assert_eq!(broken.status, 1);
}

#[test]
fn cli_dev_link_doctor_and_framework_checks() {
    let fixture = CliFixture::new("dev-link");
    let root = fixture.root.clone();
    // Minimal framework layout in the fixture repo.
    write(
        root.join("agents/orchestrator.md"),
        "---\nname: orchestrator\ndescription: Test orchestrator\ndescription: test\ndescription: x\n---\n# Orchestrator\n",
    );
    // Wait — fix the frontmatter to a single description.
    fs::write(
        root.join("agents/orchestrator.md"),
        "---\nname: orchestrator\ndescription: Test orchestrator agent\nmodel: minimax-coding-plan/MiniMax-M3\n---\n# Orchestrator\n",
    )
    .unwrap();
    write(
        root.join("permissions.yaml"),
        "agents:\n  orchestrator:\n    phases: []\n    capabilities: []\n",
    );
    write(root.join("skills/demo/SKILL.md"), "# Demo Skill\n");
    write(
        root.join("prompts/sdd-kernel/workflows/sddk-a-lite.yaml"),
        "name: a-lite\nversion: 0.1.0\n",
    );
    write(
        root.join("prompts/sdd-kernel/phases/apply.md"),
        "# Apply Phase\n",
    );

    let opencode_dir = fixture.root.join("opencode");
    let zcode_dir = fixture.root.join("zcode");
    fs::create_dir_all(opencode_dir.join("agents")).unwrap();
    // A stale copy of an agent that exists in the repo.
    fs::write(opencode_dir.join("agents/orchestrator.md"), "stale content").unwrap();
    // A local-only agent (no repo counterpart) must be preserved.
    fs::write(opencode_dir.join("agents/local-only.md"), "local agent").unwrap();
    // opencode.json with a local entry only.
    write(
        opencode_dir.join("opencode.json"),
        r#"{
  "agent": {
    "local-only": {"mode": "subagent", "prompt": "{file:/tmp/local.md}", "hidden": true}
  },
  "mcp": {}
}"#,
    );

    // U2: link into both editors.
    let linked = fixture.run(&[
        "dev",
        "link",
        "--root",
        root.to_str().unwrap(),
        "--editor",
        "all",
        "--opencode-dir",
        opencode_dir.to_str().unwrap(),
        "--zcode-dir",
        zcode_dir.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );
    let link_json: serde_json::Value = serde_json::from_slice(&linked.stdout).unwrap();
    let reports = link_json.as_array().unwrap();
    assert_eq!(reports.len(), 2, "one report per editor");
    assert_eq!(reports[0]["agents_linked"], 1);
    assert_eq!(reports[0]["workflows_linked"], 1);
    assert_eq!(
        reports[0]["stale_replaced"], 1,
        "stale orchestrator replaced"
    );

    // The local-only agent must still be a regular file (not touched).
    let local_only = fs::symlink_metadata(opencode_dir.join("agents/local-only.md")).unwrap();
    assert!(local_only.file_type().is_file());

    // The orchestrator agent is now a symlink to the repo.
    let orchestrator = fs::symlink_metadata(opencode_dir.join("agents/orchestrator.md")).unwrap();
    assert!(orchestrator.file_type().is_symlink());
    // Stale backup exists.
    assert!(opencode_dir.join("agents/orchestrator.sddk-stale").exists());

    // U1: opencode.json now registers the framework agent pointing at the repo.
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(opencode_dir.join("opencode.json")).unwrap())
            .unwrap();
    let registered = &config["agent"]["orchestrator"];
    assert_eq!(registered["mode"], "primary");
    assert!(
        registered.get("hidden").is_none(),
        "primary agents are selectable"
    );
    assert_eq!(
        registered["prompt"],
        format!("{{file:{}}}", root.join("agents/orchestrator.md").display())
    );
    assert_eq!(registered["description"], "Test orchestrator agent");
    // Local entry untouched.
    assert!(config["agent"]["local-only"].is_object());

    // U2: uninstall removes the framework entry + symlink, keeps local.
    let uninstalled = fixture.run(&[
        "dev",
        "uninstall",
        "--editor",
        "opencode",
        "--root",
        root.to_str().unwrap(),
        "--opencode-dir",
        opencode_dir.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(
        uninstalled.status.success(),
        "{}",
        String::from_utf8_lossy(&uninstalled.stderr)
    );
    // Uninstall renders text output; verify the entry removal happened on disk.
    assert!(String::from_utf8_lossy(&uninstalled.stdout).contains("1 entries"));
    let after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(opencode_dir.join("opencode.json")).unwrap())
            .unwrap();
    assert!(after["agent"]["orchestrator"].is_null());
    assert!(after["agent"]["local-only"].is_object());
    assert!(
        !opencode_dir.join("agents/orchestrator.md").exists(),
        "framework symlink removed"
    );
    assert!(opencode_dir.join("agents/local-only.md").exists());
}

#[test]
fn cli_dev_link_creates_opencode_json_and_links_markdown_skills() {
    let fixture = CliFixture::new("dev-link-fresh");
    let root = fixture.root.clone();
    write(
        root.join("agents/orchestrator.md"),
        "---\nname: orchestrator\ndescription: Test orchestrator agent\n---\n# Orchestrator\n",
    );
    write(
        root.join("permissions.yaml"),
        "agents:\n  orchestrator:\n    phases: []\n    capabilities: []\n",
    );
    write(root.join("skills/demo/SKILL.md"), "# Demo\n");
    write(root.join("skills/BOOK-WORKFLOW.md"), "# Book workflow\n");
    // Fresh editor install: config dir exists but has NO opencode.json.
    let opencode_dir = fixture.root.join("opencode");
    fs::create_dir_all(&opencode_dir).unwrap();

    let linked = fixture.run(&[
        "dev",
        "link",
        "--root",
        root.to_str().unwrap(),
        "--editor",
        "opencode",
        "--opencode-dir",
        opencode_dir.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(
        linked.status.success(),
        "{}",
        String::from_utf8_lossy(&linked.stderr)
    );

    // G5: opencode.json is created and the framework agent is registered.
    let config: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(opencode_dir.join("opencode.json")).unwrap())
            .unwrap();
    assert!(config["agent"]["orchestrator"].is_object());
    assert_eq!(config["agent"]["orchestrator"]["mode"], "primary");
    assert_eq!(
        config["agent"]["orchestrator"]["prompt"],
        format!("{{file:{}}}", root.join("agents/orchestrator.md").display())
    );
    // Agent + skill directory + top-level markdown skill are all symlinked.
    assert!(
        fs::symlink_metadata(opencode_dir.join("agents/orchestrator.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert!(
        fs::symlink_metadata(opencode_dir.join("skills/demo"))
            .unwrap()
            .file_type()
            .is_symlink()
    );
    // G6: top-level markdown skills (BOOK-*.md) are linked too.
    assert!(
        fs::symlink_metadata(opencode_dir.join("skills/BOOK-WORKFLOW.md"))
            .unwrap()
            .file_type()
            .is_symlink()
    );

    // Uninstall removes the created registration and links, keeps the file.
    let uninstalled = fixture.run(&[
        "dev",
        "uninstall",
        "--editor",
        "opencode",
        "--root",
        root.to_str().unwrap(),
        "--opencode-dir",
        opencode_dir.to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert!(
        uninstalled.status.success(),
        "{}",
        String::from_utf8_lossy(&uninstalled.stderr)
    );
    let after: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(opencode_dir.join("opencode.json")).unwrap())
            .unwrap();
    assert!(after["agent"]["orchestrator"].is_null());
    assert!(!opencode_dir.join("agents/orchestrator.md").exists());
    assert!(!opencode_dir.join("skills/BOOK-WORKFLOW.md").exists());
}

#[test]
fn cli_full_runtime_pipeline_dogfood() {
    let fixture = CliFixture::new("dogfood");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    write(
        fixture.root.join("permissions.yaml"),
        r#"
agents:
  sdd-kernel-apply:
    phases: [build, verify]
    capabilities: [git.inspect, git.commit]
"#,
    );
    write(
        fixture.root.join("schemas/agent-result.schema.json"),
        include_str!("../../../schemas/agent-result.schema.json"),
    );
    write(
        fixture.root.join("schemas/artifact-ref.schema.json"),
        include_str!("../../../schemas/artifact-ref.schema.json"),
    );
    write(
        fixture.root.join("schemas/capability-request.schema.json"),
        include_str!("../../../schemas/capability-request.schema.json"),
    );
    std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(&fixture.root)
        .output()
        .unwrap();
    for (key, value) in [("user.name", "SDDK Test"), ("user.email", "test@sddk.dev")] {
        std::process::Command::new("git")
            .args(["config", key, value])
            .current_dir(&fixture.root)
            .output()
            .unwrap();
    }
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
    ];

    let adopted = fixture.run_adopt(
        "apply",
        &[
            "--root",
            fixture.root.to_str().unwrap(),
            "--scope",
            ".",
            "--remote",
            "https://example.com/acme/repo.git",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "dogfood",
            "--format",
            "json",
        ],
    );
    assert!(adopted.status.success());

    let vault = fixture.root.join("vault");
    fs::create_dir_all(vault.join("terms")).unwrap();
    fs::write(
        vault.join("terms/TERM-Auth.md"),
        "---\nid: TERM-Auth\ntype: term\n---\n# Auth\n\nToken [[TERM-JWT]]\n",
    )
    .unwrap();
    fs::write(
        vault.join("terms/TERM-JWT.md"),
        "---\nid: TERM-JWT\ntype: term\n---\n# JWT\n",
    )
    .unwrap();
    let indexed = run_with_root(
        &fixture,
        &[
            "vault",
            "index",
            "--vault",
            vault.to_str().unwrap(),
            "--db",
            fixture.root.join("index.sqlite").to_str().unwrap(),
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        indexed.status.success(),
        "{}",
        String::from_utf8_lossy(&indexed.stderr)
    );
    let indexed_json: serde_json::Value = serde_json::from_slice(&indexed.stdout).unwrap();
    assert_eq!(indexed_json["errors"], 0);
    assert_eq!(indexed_json["nodes"], 2);

    let started = run_with_root(
        &fixture,
        &[
            "cycle",
            "start",
            "--name",
            "dogfood",
            "--path",
            "a-full",
            "--timestamp",
            "2026-08-04T10:00:00Z",
            "--actor",
            "dogfood",
            "--lease-owner",
            "agent-a",
            "--lease-ms",
            "3600000",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        started.status.success(),
        "{}",
        String::from_utf8_lossy(&started.stderr)
    );
    let started_json: serde_json::Value = serde_json::from_slice(&started.stdout).unwrap();
    let cycle_id = started_json["cycle_id"].as_str().unwrap().to_owned();
    assert_eq!(started_json["phase"], "explore");

    let evaluated = run_with_root(
        &fixture,
        &[
            "cycle",
            "evaluate-gate",
            "--cycle",
            &cycle_id,
            "--transition",
            "phase.explore.complete",
            "--gate",
            "exploration-sufficient",
            "--timestamp",
            "2026-08-04T10:00:01Z",
            "--actor",
            "dogfood",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        evaluated.status.success(),
        "{}",
        String::from_utf8_lossy(&evaluated.stderr)
    );
    let gate_receipt =
        serde_json::from_slice::<serde_json::Value>(&evaluated.stdout).unwrap()["receipt_id"]
            .as_str()
            .unwrap()
            .to_owned();

    let transitioned = run_with_root(
        &fixture,
        &[
            "cycle",
            "transition",
            "--cycle",
            &cycle_id,
            "--transition",
            "phase.explore.complete",
            "--artifact",
            "exploration-report=artifacts/exploration.md",
            "--gate-receipt",
            &gate_receipt,
            "--lease-owner",
            "agent-a",
            "--fencing-token",
            "1",
            "--timestamp",
            "2026-08-04T10:00:02Z",
            "--actor",
            "dogfood",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        transitioned.status.success(),
        "{}",
        String::from_utf8_lossy(&transitioned.stderr)
    );
    let transition_json: serde_json::Value = serde_json::from_slice(&transitioned.stdout).unwrap();
    assert_eq!(transition_json["phase"], "specify");

    let capability = run_with_root(
        &fixture,
        &[
            "capability",
            "apply",
            "--capability",
            "git.inspect",
            "--program",
            "echo",
            "--arg",
            "ok",
            "--agent",
            "sdd-kernel-apply",
            "--phase",
            "build",
            "--timestamp",
            "2026-08-04T10:00:03Z",
            "--actor",
            "dogfood",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        capability.status.success(),
        "{}",
        String::from_utf8_lossy(&capability.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&capability.stdout).unwrap()["status"],
        "succeeded"
    );

    let branch = run_with_root(
        &fixture,
        &[
            "git",
            "create-branch",
            "--name",
            "feat/dogfood",
            "--timestamp",
            "2026-08-04T10:00:04Z",
            "--actor",
            "dogfood",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        branch.status.success(),
        "{}",
        String::from_utf8_lossy(&branch.stderr)
    );
    let commit = run_with_root(
        &fixture,
        &[
            "git",
            "commit",
            "--message",
            "dogfood",
            "--approve",
            "--timestamp",
            "2026-08-04T10:00:05Z",
            "--actor",
            "dogfood",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        commit.status.success(),
        "{}",
        String::from_utf8_lossy(&commit.stderr)
    );
    let tag = run_with_root(
        &fixture,
        &[
            "git",
            "tag",
            "--name",
            "v9.9.9",
            "--timestamp",
            "2026-08-04T10:00:06Z",
            "--actor",
            "dogfood",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        tag.status.success(),
        "{}",
        String::from_utf8_lossy(&tag.stderr)
    );

    let source = fixture.root.join("report.md");
    fs::write(&source, "dogfood artifact\n").unwrap();
    let stored = run_with_root(
        &fixture,
        &[
            "artifact",
            "store",
            "--file",
            source.to_str().unwrap(),
            "--kind",
            "report",
            "--timestamp",
            "2026-08-04T10:00:07Z",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        stored.status.success(),
        "{}",
        String::from_utf8_lossy(&stored.stderr)
    );
    let digest = serde_json::from_slice::<serde_json::Value>(&stored.stdout).unwrap()["sha256"]
        .as_str()
        .unwrap()
        .to_owned();
    let destination = fixture.root.join("restored.md");
    let fetched = run_with_root(
        &fixture,
        &[
            "artifact",
            "get",
            "--digest",
            &digest,
            "--output",
            destination.to_str().unwrap(),
        ],
        &common,
    );
    assert!(
        fetched.status.success(),
        "{}",
        String::from_utf8_lossy(&fetched.stderr)
    );
    assert_eq!(
        fs::read_to_string(&destination).unwrap(),
        "dogfood artifact\n"
    );

    let verified = run_with_root(&fixture, &["ledger", "verify", "--format", "json"], &common);
    assert!(verified.status.success());
    let ledger_json: serde_json::Value = serde_json::from_slice(&verified.stdout).unwrap();
    assert!(ledger_json["event_count"].as_i64().unwrap() >= 2);

    let release_plan = run_with_root(
        &fixture,
        &[
            "release",
            "plan",
            "--repo",
            "acme/repo",
            "--branch",
            "feat/dogfood",
            "--base",
            "main",
            "--title",
            "Dogfood",
            "--tag",
            "v9.9.9",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(
        release_plan.status.success(),
        "{}",
        String::from_utf8_lossy(&release_plan.stderr)
    );

    let permission_ok = run_with_root(
        &fixture,
        &[
            "permission",
            "check",
            "--agent",
            "sdd-kernel-apply",
            "--phase",
            "build",
            "--capability",
            "git.inspect",
            "--format",
            "json",
        ],
        &common,
    );
    assert!(permission_ok.status.success());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&permission_ok.stdout).unwrap()["allowed"],
        true
    );

    let prefix = fixture.root.join("prefix");
    let installed = run_from([
        "sddk",
        "dev",
        "install",
        "--prefix",
        prefix.to_str().unwrap(),
        "--channel",
        "dogfood",
        "--timestamp",
        "2026-08-04T10:00:08Z",
    ]);
    assert_eq!(installed.status, 0, "{}", installed.stderr);
    assert!(prefix.join("bin/sddk").exists());
    assert!(prefix.join("sddk-install.json").exists());
    let uninstalled = run_from([
        "sddk",
        "dev",
        "uninstall",
        "--prefix",
        prefix.to_str().unwrap(),
    ]);
    assert_eq!(uninstalled.status, 0, "{}", uninstalled.stderr);
}

#[test]
fn cli_dev_doctor_reports_environment() {
    let doctor = run_from(["sddk", "dev", "doctor", "--format", "json"]);
    // Status reflects environment completeness (all_present); the runner
    // image may lack optional tools like gh, so accept both outcomes.
    assert!(
        doctor.status == 0 || doctor.status == 1,
        "{}",
        doctor.stderr
    );
    let output: serde_json::Value = serde_json::from_str(&doctor.stdout).unwrap();
    // all_present depends on the runner environment (e.g. gh availability),
    // so assert structural validity and the stable core tools instead.
    assert!(output["all_present"].is_boolean());
    let tools = output["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|check| check["tool"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(tools.contains(&"cargo"));
    assert!(tools.contains(&"git"));
}

fn run_with_root(fixture: &CliFixture, args: &[&str], common: &[&str]) -> std::process::Output {
    fixture.run(
        &args
            .iter()
            .chain(common.iter())
            .copied()
            .collect::<Vec<_>>(),
    )
}

fn repository_fixture() -> TestRepository {
    let repository = TestRepository::new().unwrap();
    repository
        .write("workflow/workflow.yaml", WORKFLOW)
        .unwrap();
    repository
        .write("schemas/workflow.schema.json", WORKFLOW_SCHEMA)
        .unwrap();
    repository
        .write("permissions.yaml", "agents: {}\n")
        .unwrap();
    repository
        .write(
            "manifest.toml",
            "[pack]\nid = \"fixture\"\nversion = \"0.1.0\"\nschema_version = 1\ncompatibility = \">=1.85\"\nrisk = \"low\"\nconsequence = \"creates\"\n\n[[commands]]\nname = \"a\"\nsurface = [\"a\"]\n\n[fixtures]\npaths = [\"tests/a.sh\"]\n",
        )
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

#[test]
fn cli_pack_validate_and_lint_enforce_manifest() {
    let fixture = CliFixture::new("pack-validate");
    let valid_manifest = r#"
[pack]
id = "fixture-pack"
version = "0.2.0"
schema_version = 1
compatibility = ">=1.85"
risk = "low"
consequence = "creates"

[[commands]]
name = "check"
surface = ["check"]

[fixtures]
paths = ["tests/a.sh"]
"#;
    write(fixture.root.join("manifest.toml"), valid_manifest);

    let validated = run_from([
        "sddk",
        "pack",
        "validate",
        "--manifest",
        fixture.root.join("manifest.toml").to_str().unwrap(),
        "--format",
        "json",
    ]);
    assert_eq!(validated.status, 0, "{}", validated.stderr);
    let output: serde_json::Value = serde_json::from_str(&validated.stdout).unwrap();
    assert_eq!(output["id"], "fixture-pack");
    assert_eq!(output["valid"], true);

    write(fixture.root.join("manifest.toml"), "[pack]\nid = \"\"\n");
    let broken = run_from([
        "sddk",
        "pack",
        "validate",
        "--manifest",
        fixture.root.join("manifest.toml").to_str().unwrap(),
    ]);
    assert_eq!(broken.status, 1);

    let repository = repository_fixture();
    generate_workflow_docs(repository.path(), false).unwrap();
    repository
        .write("manifest.toml", "[pack]\nid = \"\"\n")
        .unwrap();
    let report = lint_repository(repository.path()).unwrap();
    assert!(
        report
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "SDDK014")
    );
}

#[test]
fn cli_runtime_errors_include_stable_code_and_recovery() {
    let fixture = CliFixture::new("error-envelope");
    write(
        fixture.root.join("workflow/workflow.yaml"),
        CANONICAL_WORKFLOW,
    );
    let common = [
        "--root",
        fixture.root.to_str().unwrap(),
        "--scope",
        ".",
        "--remote",
        "https://example.com/acme/repo.git",
    ];
    let adopted = fixture.run_adopt(
        "apply",
        &[
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
        ],
    );
    assert!(adopted.status.success());

    let missing = run_with_root(
        &fixture,
        &[
            "cycle",
            "status",
            "--cycle",
            "cycle-missing",
            "--format",
            "json",
        ],
        &common,
    );
    assert_eq!(missing.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&missing.stderr);
    assert!(stderr.contains("error[STORAGE_NOT_FOUND]"), "{}", stderr);
    assert!(stderr.contains("recovery:"), "{}", stderr);

    let bad_transition = run_with_root(
        &fixture,
        &[
            "cycle",
            "transition",
            "--cycle",
            "cycle-missing",
            "--transition",
            "phase.explore.complete",
            "--format",
            "json",
        ],
        &common,
    );
    assert_eq!(bad_transition.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&bad_transition.stderr);
    assert!(stderr.contains("error[ENGINE_STORAGE]"), "{}", stderr);
    assert!(stderr.contains("cause:"), "{}", stderr);
    assert!(stderr.contains("recovery:"), "{}", stderr);
}

#[test]
fn skills_and_agents_reference_only_real_sddk_commands() {
    // Drift gate: every `sddk <cmd>` / `sddk <cmd> <sub>` token found in the
    // framework's skills and agents must exist in the real CLI. Keeps the
    // agent ecosystem aligned with the shipped binary (skills adapted for the
    // sddk CLI must never reference a command that does not exist).
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let mut documents = Vec::new();
    for entry in walkdir::WalkDir::new(root.join("skills"))
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
    {
        if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
            documents.push(entry.into_path());
        }
    }
    for entry in std::fs::read_dir(root.join("agents")).unwrap().flatten() {
        if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
            documents.push(entry.path());
        }
    }
    assert!(
        documents.len() > 80,
        "expected the full skills+agents corpus, found {}",
        documents.len()
    );

    // Extract command tokens only from code blocks and inline backtick
    // commands, so prose mentions and skill triggers create no false
    // positives.
    let mut references: Vec<(String, Option<String>)> = Vec::new();
    let mut in_block = false;
    for document in &documents {
        let content = std::fs::read_to_string(document).unwrap();
        for line in content.lines() {
            if line.trim_start().starts_with("```") {
                in_block = !in_block;
                continue;
            }
            let mut candidates: Vec<&str> = Vec::new();
            if line.trim_start().starts_with("sddk ") {
                candidates.push(line.trim_start());
            }
            for span in line.split('`') {
                if span.starts_with("sddk ") {
                    candidates.push(span);
                }
            }
            for candidate in candidates {
                let tokens: Vec<&str> = candidate.split_whitespace().take(3).collect();
                let command = tokens.get(1).copied().unwrap_or("");
                if command.is_empty() {
                    continue;
                }
                let subcommand = tokens
                    .get(2)
                    .copied()
                    .filter(|token| token.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
                references.push((command.to_owned(), subcommand.map(str::to_owned)));
            }
        }
    }
    assert!(
        references.len() > 30,
        "expected a substantial CLI reference corpus, found {}",
        references.len()
    );

    let help = |args: &[&str]| -> String {
        let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
            .args(args)
            .arg("--help")
            .output()
            .unwrap();
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        format!("{stdout}\n{stderr}")
    };
    let top_level = help(&[]);
    let mut help_cache: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    let mut broken: Vec<String> = Vec::new();
    for (command, subcommand) in references {
        if subcommand.is_none() {
            if !top_level.contains(&command) {
                broken.push(format!("sddk {command}"));
            }
            continue;
        }
        let sub = subcommand.as_deref().unwrap();
        let page = help_cache
            .entry(command.clone())
            .or_insert_with(|| help(&[&command]));
        if !page.contains(sub) {
            broken.push(format!("sddk {command} {sub}"));
        }
    }
    assert!(
        broken.is_empty(),
        "skills/agents reference CLI commands that do not exist:\n  {}",
        broken.join("\n  ")
    );
}

#[test]
fn lint_passes_without_workflow_file_in_repo() {
    // Non-intrusive policy (ADR-0011): a project without workflow/workflow.yaml
    // must still lint cleanly because the canonical manifest is embedded.
    let fixture = CliFixture::new("lint-no-workflow");
    let report = lint_repository(&fixture.root).unwrap();
    let workflow_errors = report
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.file == "workflow/workflow.yaml")
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    assert_eq!(
        workflow_errors, 0,
        "lint must fall back to the embedded canonical workflow (ADR-0011)"
    );
    assert!(!fixture.root.join("workflow/workflow.yaml").exists());
}
