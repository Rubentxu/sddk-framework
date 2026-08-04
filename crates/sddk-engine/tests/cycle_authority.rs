//! End-to-end cycle authority tests: lease fencing, rebuild, and frames.

use std::collections::{BTreeSet, HashMap};

use sddk_domain::{ArtifactRef, CycleManifest, CyclePath, CycleStatus, Phase};
use sddk_engine::{CycleStartInput, Engine, EventContext, GateOutcome, TransitionEvidence};
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
    evidence.gates.insert(gate.into(), GateOutcome::Passed);
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
