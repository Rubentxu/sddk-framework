//! End-to-end cycle authority tests: lease fencing, rebuild, and frames.

use std::collections::{BTreeSet, HashMap};

use sddk_domain::{ArtifactRef, CycleManifest, CyclePath, CycleStatus, Phase};
use sddk_engine::{
    CycleStartInput, Engine, EventContext, GateEvaluationInput, GateReceiptRef, TransitionEvidence,
};
use sddk_storage::{ProjectRecord, Storage, StorageError, WorkspaceRecord};

const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");
const TIMESTAMP: &str = "2026-08-04T10:00:00Z";

fn engine_with_storage(storage: Storage) -> Engine {
    Engine::new(
        sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap(),
        storage,
    )
    .unwrap()
}

fn setup() -> (Storage, Engine) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("ledger.sqlite");
    let storage = Storage::open(&path).unwrap();
    storage
        .insert_project(&ProjectRecord {
            project_id: "project-1".into(),
            display_name: "project".into(),
            remote_url: Some("https://example.com/owner/project".into()),
            scope: "owner".into(),
            created_at: TIMESTAMP.into(),
        })
        .unwrap();
    storage
        .insert_workspace(&WorkspaceRecord {
            workspace_id: "workspace-1".into(),
            project_id: "project-1".into(),
            canonical_path: "/work/project".into(),
            created_at: TIMESTAMP.into(),
        })
        .unwrap();
    let engine = engine_with_storage(Storage::open(&path).unwrap());
    std::mem::forget(directory);
    (storage, engine)
}

fn start_cycle(engine: &mut Engine, event_id: &str) -> CycleManifest {
    let input = CycleStartInput {
        manifest: manifest_for_path(CyclePath::AFull),
        requirements: cycle_start_requirements(),
    };
    let plan = engine.plan_cycle_start(input).unwrap();
    engine
        .apply_cycle_start(&plan, &context(event_id, "command-a"))
        .unwrap()
        .manifest
}

fn transition_explore(engine: &mut Engine, event_id: &str, command_id: &str) {
    advance(
        engine,
        "phase.explore.complete",
        "exploration-report",
        "exploration-sufficient",
        event_id,
        command_id,
    );
}

fn transition_specify(engine: &mut Engine, event_id: &str, command_id: &str) {
    advance(
        engine,
        "phase.specify.complete",
        "specification",
        "requirements-testable",
        event_id,
        command_id,
    );
}

fn advance(
    engine: &mut Engine,
    transition_id: &str,
    artifact_kind: &str,
    gate: &str,
    event_id: &str,
    command_id: &str,
) {
    let mut evidence = TransitionEvidence::default();
    evidence.artifacts.insert(
        artifact_kind.into(),
        ArtifactRef::new(artifact_kind, "artifacts/out.md"),
    );
    evidence.gates.insert(
        gate.into(),
        GateReceiptRef {
            receipt_id: engine
                .evaluate_gate(&GateEvaluationInput {
                    cycle_id: "cycle-1".into(),
                    transition_id: transition_id.into(),
                    gate: gate.into(),
                    evaluator: sddk_engine::DEFAULT_EVALUATOR.into(),
                    evidence: serde_json::json!({"verified": true}),
                    outcome: sddk_storage::GateOutcomeStatus::Passed,
                    evaluated_at: TIMESTAMP.into(),
                    actor: "test-runtime".into(),
                    command_id: format!("gate-{gate}"),
                })
                .unwrap()
                .receipt_id,
        },
    );
    let plan = engine
        .plan_transition("cycle-1", transition_id, evidence)
        .unwrap();
    engine
        .apply_transition(&plan, &context(event_id, command_id))
        .unwrap();
}

fn manifest_for_path(path: CyclePath) -> CycleManifest {
    CycleManifest {
        schema_version: 1,
        project_id: "project-1".into(),
        workspace_id: "workspace-1".into(),
        cycle_id: "cycle-1".into(),
        display_name: "Authority work".into(),
        status: CycleStatus::Open,
        phase: Phase::Explore,
        path,
        branch: "feat/authority".into(),
        base: "abc123".into(),
        head: None,
        artifacts: HashMap::new(),
        release: None,
        remediation_round: 0,
        remote_url: Some("https://example.com/owner/project".into()),
        scope: Some("owner".into()),
    }
}

fn cycle_start_requirements() -> BTreeSet<String> {
    [
        "project.adopted",
        "project.initialized",
        "worktree.clean",
        "cycle.no_active_conflict",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn context(event_id: &str, command_id: &str) -> EventContext {
    EventContext {
        command_id: command_id.into(),
        frame_id: format!("frame:{command_id}"),
        event_id: event_id.into(),
        actor: "test-runtime".into(),
        occurred_at: TIMESTAMP.into(),
    }
}

#[test]
fn lease_fencing_blocks_stale_holders_and_expired_reacquire_bumps_token() {
    let (mut storage, mut engine) = setup();

    start_cycle(&mut engine, "evt-1");

    let lease = storage
        .acquire_cycle_lease("cycle-1", "agent-a", 1_000, 2_000)
        .unwrap();
    assert_eq!(lease.fencing_token, 1);

    let fenced = engine.require_lease_fence("cycle-1", "agent-a", 1).unwrap();
    assert_eq!(fenced.owner, "agent-a");

    let stale_holder = engine.require_lease_fence("cycle-1", "agent-b", 1);
    assert!(matches!(
        stale_holder,
        Err(sddk_engine::EngineError::Storage(StorageError::LeaseConflict {
            owner,
            ..
        })) if owner == "agent-a"
    ));

    let reacquired = storage
        .acquire_cycle_lease("cycle-1", "agent-b", 3_000, 4_000)
        .unwrap();
    assert_eq!(reacquired.fencing_token, 2);

    let stale_token = engine.require_lease_fence("cycle-1", "agent-b", 1);
    assert!(matches!(
        stale_token,
        Err(sddk_engine::EngineError::Storage(StorageError::LeaseConflict {
            owner,
            ..
        })) if owner == "agent-b"
    ));

    let valid = engine.require_lease_fence("cycle-1", "agent-b", 2).unwrap();
    assert_eq!(valid.fencing_token, 2);
}

#[test]
fn renew_cycle_lease_keeps_token_valid_for_require_lease_fence() {
    let (mut storage, mut engine) = setup();

    start_cycle(&mut engine, "evt-renew-1");

    let lease = storage
        .acquire_cycle_lease("cycle-1", "agent-a", 1_000, 2_000)
        .unwrap();
    assert_eq!(lease.fencing_token, 1);

    let renewed = storage
        .renew_cycle_lease("cycle-1", "agent-a", 1, 1_500, 5_000)
        .unwrap();
    assert_eq!(renewed.fencing_token, 1);
    assert_eq!(renewed.expires_at_ms, 5_000);

    let fenced = engine.require_lease_fence("cycle-1", "agent-a", 1).unwrap();
    assert_eq!(fenced.fencing_token, 1);
    assert_eq!(fenced.expires_at_ms, 5_000);
}

#[test]
fn rebuild_restores_missing_snapshot_without_appending_events() {
    let (storage, mut engine) = setup();

    start_cycle(&mut engine, "evt-1");
    transition_explore(&mut engine, "evt-2", "command-b");
    assert_eq!(storage.list_events().unwrap().len(), 2);

    storage.delete_cycle_snapshot("cycle-1").unwrap();
    assert!(matches!(
        storage.get_cycle("cycle-1"),
        Err(StorageError::NotFound {
            entity: "cycle",
            ..
        })
    ));

    let rebuilt = engine.rebuild_cycle("cycle-1").unwrap();
    assert!(rebuilt.restored);
    assert_eq!(rebuilt.manifest.phase, Phase::Specify);
    assert_eq!(rebuilt.sequence, 2);

    engine.verify_cycle_snapshot("cycle-1").unwrap();
    assert_eq!(storage.list_events().unwrap().len(), 2);
    assert_eq!(storage.verify_ledger().unwrap().event_count, 2);

    let again = engine.rebuild_cycle("cycle-1").unwrap();
    assert!(!again.restored);
}

#[test]
fn frame_events_are_grouped_by_command() {
    let (storage, mut engine) = setup();

    start_cycle(&mut engine, "evt-1");
    transition_explore(&mut engine, "evt-2", "command-b");
    transition_specify(&mut engine, "evt-3", "command-c");

    let frame_a = storage.list_frame_events("frame:command-a").unwrap();
    let frame_b = storage.list_frame_events("frame:command-b").unwrap();
    let frame_c = storage.list_frame_events("frame:command-c").unwrap();

    assert_eq!(frame_a.len(), 1);
    assert_eq!(frame_b.len(), 1);
    assert_eq!(frame_c.len(), 1);
    assert_eq!(frame_a[0].event_id, "evt-1");
    assert_eq!(frame_b[0].event_id, "evt-2");
    assert_eq!(frame_c[0].event_id, "evt-3");
    assert!(frame_a[0].frame_id.ends_with("command-a"));
}

#[test]
fn rebuild_refuses_to_overwrite_divergent_snapshot() {
    let (storage, mut engine) = setup();

    start_cycle(&mut engine, "evt-1");
    transition_explore(&mut engine, "evt-2", "command-b");

    let tampered = engine.replay_cycle("cycle-1").unwrap().manifest.clone();
    let mut corrupt = tampered.clone();
    corrupt.status = CycleStatus::Abandoned;
    storage.delete_cycle_snapshot("cycle-1").unwrap();
    let record = sddk_storage::CycleRecord {
        manifest: corrupt,
        created_at: TIMESTAMP.into(),
        updated_at: TIMESTAMP.into(),
    };
    storage.insert_cycle(&record).unwrap();

    assert!(matches!(
        engine.rebuild_cycle("cycle-1"),
        Err(sddk_engine::EngineError::SnapshotMismatch { cycle_id }) if cycle_id == "cycle-1"
    ));
    assert_eq!(storage.list_events().unwrap().len(), 2);
}

#[test]
fn gate_receipt_requires_registered_evaluator_and_matches_plan_state() {
    let (_storage, mut engine) = setup();
    start_cycle(&mut engine, "evt-1");

    let unregistered = engine.evaluate_gate(&GateEvaluationInput {
        cycle_id: "cycle-1".into(),
        transition_id: "phase.explore.complete".into(),
        gate: "exploration-sufficient".into(),
        evaluator: "untrusted.script".into(),
        evidence: serde_json::json!({}),
        outcome: sddk_storage::GateOutcomeStatus::Passed,
        evaluated_at: TIMESTAMP.into(),
        actor: "test-runtime".into(),
        command_id: "gate-1".into(),
    });
    assert!(matches!(
        unregistered,
        Err(sddk_engine::EngineError::UnregisteredEvaluator { gate, evaluator })
            if gate == "exploration-sufficient" && evaluator == "untrusted.script"
    ));

    engine.register_evaluator("exploration-sufficient", "tests.pytest");
    assert!(engine.evaluator_registered("exploration-sufficient", "tests.pytest"));
    assert!(engine.evaluator_registered("exploration-sufficient", sddk_engine::DEFAULT_EVALUATOR));
    assert!(!engine.evaluator_registered("exploration-sufficient", "untrusted.script"));

    transition_explore(&mut engine, "evt-2", "command-b");
    let specify_receipt = engine
        .evaluate_gate(&GateEvaluationInput {
            cycle_id: "cycle-1".into(),
            transition_id: "phase.specify.complete".into(),
            gate: "requirements-testable".into(),
            evaluator: sddk_engine::DEFAULT_EVALUATOR.into(),
            evidence: serde_json::json!({}),
            outcome: sddk_storage::GateOutcomeStatus::Passed,
            evaluated_at: TIMESTAMP.into(),
            actor: "test-runtime".into(),
            command_id: "gate-3".into(),
        })
        .unwrap();
    let mut apply_evidence = TransitionEvidence::default();
    apply_evidence.artifacts.insert(
        "specification".into(),
        sddk_domain::ArtifactRef::new("specification", "artifacts/spec.md"),
    );
    apply_evidence.gates.insert(
        "requirements-testable".into(),
        GateReceiptRef {
            receipt_id: specify_receipt.receipt_id.clone(),
        },
    );
    let apply_plan = engine
        .plan_transition("cycle-1", "phase.specify.complete", apply_evidence)
        .unwrap();
    engine
        .apply_transition(&apply_plan, &context("evt-3", "command-c"))
        .unwrap();

    let current = engine.storage().get_cycle("cycle-1").unwrap().manifest;
    let new_hash = engine.plan_hash("cycle-1", "phase.specify.complete", &current);
    assert_ne!(new_hash, specify_receipt.plan_hash);
    assert_eq!(
        engine.plan_hash("cycle-1", "phase.specify.complete", &current),
        new_hash
    );
}

#[test]
fn transition_rejects_mismatched_or_foreign_gate_receipts() {
    let (_storage, mut engine) = setup();
    start_cycle(&mut engine, "evt-1");
    transition_explore(&mut engine, "evt-2", "command-b");

    let mismatched = engine
        .evaluate_gate(&GateEvaluationInput {
            cycle_id: "cycle-1".into(),
            transition_id: "phase.specify.complete.a-min".into(),
            gate: "requirements-testable".into(),
            evaluator: sddk_engine::DEFAULT_EVALUATOR.into(),
            evidence: serde_json::json!({}),
            outcome: sddk_storage::GateOutcomeStatus::Passed,
            evaluated_at: TIMESTAMP.into(),
            actor: "test-runtime".into(),
            command_id: "gate-1".into(),
        })
        .unwrap();
    let mut evidence = TransitionEvidence::default();
    evidence.artifacts.insert(
        "specification".into(),
        sddk_domain::ArtifactRef::new("specification", "artifacts/spec.md"),
    );
    evidence.gates.insert(
        "requirements-testable".into(),
        GateReceiptRef {
            receipt_id: mismatched.receipt_id.clone(),
        },
    );
    let rejected = engine.plan_transition("cycle-1", "phase.specify.complete", evidence);
    assert!(matches!(
        rejected,
        Err(sddk_engine::EngineError::GateReceiptMismatch { receipt_id, .. })
            if receipt_id == mismatched.receipt_id
    ));
}
