use rusqlite::Connection;
use sddk_domain::{CycleId, CycleManifest, CycleStatus};
use sddk_storage::{
    ArtifactRecord, CapabilityReceiptInput, CapabilityStatus, CycleRecord, LedgerEventInput,
    ProjectRecord, Storage, StorageError, WorkspaceRecord,
};
use serde_json::json;
use tempfile::tempdir;

const CREATED_AT: &str = "2026-08-03T12:00:00Z";

#[test]
fn persists_canonical_records_across_reopen() {
    let directory = tempdir().unwrap();
    let database_path = directory.path().join("state/ledger.sqlite");
    let cycle = cycle_record();
    let artifact = ArtifactRecord {
        artifact_id: "artifact-1".into(),
        project_id: "project-1".into(),
        cycle_id: Some(cycle.manifest.cycle_id.clone()),
        kind: "specification".into(),
        path: "sha256/ab/spec.md".into(),
        sha256: Some(format!("sha256:{}", "a".repeat(64))),
        producer: Some("sddk-spec".into()),
        created_at: CREATED_AT.into(),
        metadata: json!({"media_type": "text/markdown"}),
    };

    {
        let storage = Storage::open(&database_path).unwrap();
        assert_eq!(storage.schema_version().unwrap(), 2);
        storage.insert_project(&project_record()).unwrap();
        storage.insert_workspace(&workspace_record()).unwrap();
        storage.insert_cycle(&cycle).unwrap();
        storage.insert_artifact(&artifact).unwrap();
    }

    let storage = Storage::open(&database_path).unwrap();
    assert_eq!(storage.get_project("project-1").unwrap(), project_record());
    assert_eq!(
        storage.get_workspace("workspace-1").unwrap(),
        workspace_record()
    );
    assert_eq!(storage.get_cycle(&cycle.manifest.cycle_id).unwrap(), cycle);
    assert_eq!(storage.get_artifact("artifact-1").unwrap(), artifact);
}

#[test]
fn adoption_registration_is_transactional_idempotent_and_conflict_safe() {
    let mut storage = Storage::open_in_memory().unwrap();
    let project = project_record();
    let workspace = workspace_record();

    storage
        .register_project_workspace(&project, &workspace)
        .unwrap();
    storage
        .register_project_workspace(
            &ProjectRecord {
                display_name: "Another checkout label".into(),
                created_at: "2026-08-04T00:00:00Z".into(),
                ..project.clone()
            },
            &WorkspaceRecord {
                created_at: "2026-08-04T00:00:00Z".into(),
                ..workspace.clone()
            },
        )
        .unwrap();

    let conflicting = ProjectRecord {
        remote_url: Some("https://example.com/other/project".into()),
        ..project
    };
    assert!(matches!(
        storage.register_project_workspace(&conflicting, &workspace),
        Err(StorageError::RegistrationConflict {
            entity: "project",
            ..
        })
    ));
    assert_eq!(storage.get_project("project-1").unwrap(), project_record());
    assert_eq!(
        storage.get_workspace("workspace-1").unwrap(),
        workspace_record()
    );
}

#[test]
fn ledger_is_hash_linked_ordered_and_append_only() {
    let directory = tempdir().unwrap();
    let database_path = directory.path().join("ledger.sqlite");
    let mut storage = Storage::open(&database_path).unwrap();
    storage.insert_project(&project_record()).unwrap();

    let first = storage.append_event(&event("event-1", None)).unwrap();
    let second = storage.append_event(&event("event-2", None)).unwrap();

    assert_eq!(first.sequence, 1);
    assert_eq!(second.sequence, 2);
    assert_eq!(
        second.previous_hash.as_deref(),
        Some(first.event_hash.as_str())
    );
    let events = storage.list_events().unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        [1, 2]
    );
    let verification = storage.verify_ledger().unwrap();
    assert_eq!(verification.event_count, 2);
    assert_eq!(verification.last_hash, Some(second.event_hash));

    drop(storage);
    let connection = Connection::open(&database_path).unwrap();
    assert!(
        connection
            .execute(
                "UPDATE ledger_events SET actor = 'tampered' WHERE sequence = 1",
                []
            )
            .is_err()
    );
    assert!(
        connection
            .execute("DELETE FROM ledger_events WHERE sequence = 1", [])
            .is_err()
    );
}

#[test]
fn cycle_event_listing_is_scoped_and_ordered() {
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    let cycle_id = cycle.manifest.cycle_id.clone();
    storage.insert_cycle(&cycle).unwrap();
    storage.append_event(&event("event-project", None)).unwrap();
    storage
        .append_event(&event("event-cycle-1", Some(&cycle_id)))
        .unwrap();
    storage
        .append_event(&event("event-cycle-2", Some(&cycle_id)))
        .unwrap();

    let events = storage.list_cycle_events(&cycle_id).unwrap();
    assert_eq!(
        events
            .iter()
            .map(|event| event.event_id.as_str())
            .collect::<Vec<_>>(),
        ["event-cycle-1", "event-cycle-2"]
    );
    assert!(
        storage
            .list_cycle_events("missing-cycle")
            .unwrap()
            .is_empty()
    );
}

#[test]
fn capability_receipts_begin_once_and_finalize_only_from_started() {
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    let input = capability_receipt("receipt-1", json!({"branch": "feature"}));

    let inserted = storage.begin_capability_receipt(&input).unwrap();
    assert_eq!(inserted.status, CapabilityStatus::Started);
    assert_eq!(inserted.completed_at, None);

    let replay_input = CapabilityReceiptInput {
        receipt_id: "receipt-2".into(),
        ..input.clone()
    };
    let replayed = storage.begin_capability_receipt(&replay_input).unwrap();
    assert_eq!(replayed, inserted);
    assert!(matches!(
        storage.get_capability_receipt("receipt-2"),
        Err(StorageError::NotFound { .. })
    ));

    let conflicting = CapabilityReceiptInput {
        receipt_id: "receipt-3".into(),
        capability: "git.delete_branch".into(),
        ..input
    };
    assert!(matches!(
        storage.begin_capability_receipt(&conflicting),
        Err(StorageError::IdempotencyConflict { .. })
    ));

    let finalized = storage
        .finalize_capability_receipt(
            "receipt-1",
            CapabilityStatus::Succeeded,
            Some(json!({"merged": true})),
            "2026-08-04T10:00:01Z",
        )
        .unwrap();
    assert_eq!(finalized.status, CapabilityStatus::Succeeded);
    assert_eq!(
        finalized.completed_at.as_deref(),
        Some("2026-08-04T10:00:01Z")
    );

    assert!(matches!(
        storage.finalize_capability_receipt(
            "receipt-1",
            CapabilityStatus::Failed,
            None,
            "2026-08-04T10:00:02Z"
        ),
        Err(StorageError::TerminalReceipt { .. })
    ));

    let listed = storage.list_capability_receipts("project-1").unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].status, CapabilityStatus::Succeeded);
}

#[test]
fn capability_receipts_reject_terminal_begins() {
    let mut storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    let input = CapabilityReceiptInput {
        status: CapabilityStatus::Succeeded,
        ..capability_receipt("receipt-1", json!({}))
    };
    assert!(matches!(
        storage.begin_capability_receipt(&input),
        Err(StorageError::InvalidReceiptBegin)
    ));
}

#[test]
fn uniqueness_and_lease_conflicts_are_enforced() {
    let (mut storage, cycle) = storage_with_cycle();
    let duplicate_identity = ProjectRecord {
        project_id: "project-2".into(),
        display_name: "Duplicate".into(),
        ..project_record()
    };
    assert!(matches!(
        storage.insert_project(&duplicate_identity),
        Err(StorageError::Database(_))
    ));

    let first = storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();
    assert_eq!(first.fencing_token, 1);
    assert!(matches!(
        storage.acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-b", 1_500, 2_500),
        Err(StorageError::LeaseConflict { .. })
    ));

    let recovered = storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-b", 2_000, 3_000)
        .unwrap();
    assert_eq!(recovered.fencing_token, 2);
    assert_eq!(
        storage.get_cycle_lease(&cycle.manifest.cycle_id).unwrap(),
        recovered
    );
    assert!(
        !storage
            .release_cycle_lease(
                "project-1",
                &cycle.manifest.cycle_id,
                "runtime-a",
                1,
                "tester",
                "command-1",
                "2026-08-13T15:00:00Z",
            )
            .unwrap()
    );
    assert!(
        storage
            .release_cycle_lease(
                "project-1",
                &cycle.manifest.cycle_id,
                "runtime-b",
                2,
                "tester",
                "command-2",
                "2026-08-13T15:00:01Z",
            )
            .unwrap()
    );
}

#[test]
fn renew_cycle_lease_extends_expiry_preserving_token() {
    let (mut storage, cycle) = storage_with_cycle();
    let first = storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();
    assert_eq!(first.fencing_token, 1);

    let renewed = storage
        .renew_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1, 1_500, 5_000)
        .unwrap();
    assert_eq!(renewed.fencing_token, 1);
    assert_eq!(renewed.expires_at_ms, 5_000);
    assert_eq!(renewed.acquired_at_ms, 1_000);

    let verified = storage
        .verify_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1)
        .unwrap();
    assert_eq!(verified.expires_at_ms, 5_000);
}

#[test]
fn renew_cycle_lease_fails_with_wrong_owner() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();

    let error = storage
        .renew_cycle_lease(&cycle.manifest.cycle_id, "runtime-b", 1, 1_500, 5_000)
        .unwrap_err();
    assert!(matches!(
        error,
        StorageError::LeaseNotRenewable { ref current_owner, .. } if current_owner == "runtime-a"
    ));

    let verified = storage
        .verify_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1)
        .unwrap();
    assert_eq!(verified.expires_at_ms, 2_000);
}

#[test]
fn renew_cycle_lease_fails_with_stale_fencing_token() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();

    let error = storage
        .renew_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 99, 1_500, 5_000)
        .unwrap_err();
    assert!(matches!(
        error,
        StorageError::LeaseNotRenewable { current_fencing_token: 1, .. }
    ));
}

#[test]
fn release_cycle_lease_writes_lease_released_event() {
    let (mut storage, cycle) = storage_with_cycle();
    storage
        .acquire_cycle_lease(&cycle.manifest.cycle_id, "runtime-a", 1_000, 2_000)
        .unwrap();

    let released = storage
        .release_cycle_lease(
            "project-1",
            &cycle.manifest.cycle_id,
            "runtime-a",
            1,
            "tester-1",
            "cycle.lock.release-1",
            "2026-08-13T15:00:00Z",
        )
        .unwrap();
    assert!(released);

    let events = storage.list_events().unwrap();
    let event = events
        .iter()
        .find(|event| event.event_type == "lease.released")
        .expect("lease.released event must be appended");
    assert_eq!(event.actor, "tester-1");
    assert_eq!(event.command_id, "cycle.lock.release-1");
    assert_eq!(event.frame_id, "frame:cycle.lock.release-1");
    assert_eq!(
        event.payload,
        json!({
            "cycle_id": cycle.manifest.cycle_id.as_str(),
            "owner": "runtime-a",
            "fencing_token": 1,
            "actor": "tester-1",
        })
    );

    let miss = storage
        .release_cycle_lease(
            "project-1",
            &cycle.manifest.cycle_id,
            "runtime-a",
            1,
            "tester-1",
            "cycle.lock.release-2",
            "2026-08-13T15:00:01Z",
        )
        .unwrap();
    assert!(!miss);
}

#[test]
fn failed_event_append_rolls_back_cycle_state_update() {
    let (mut storage, cycle) = storage_with_cycle();
    let initial_event = event("event-1", Some(&cycle.manifest.cycle_id));
    storage.append_event(&initial_event).unwrap();

    let mut blocked = cycle.manifest.clone();
    blocked.status = CycleStatus::Blocked;
    let duplicate_event = LedgerEventInput {
        state_before: Some(json!({"status": "OPEN"})),
        state_after: Some(json!({"status": "BLOCKED"})),
        ..initial_event
    };

    assert!(matches!(
        storage.update_cycle_with_event(&blocked, "2026-08-03T12:01:00Z", &duplicate_event),
        Err(StorageError::Database(_))
    ));
    assert_eq!(
        storage
            .get_cycle(&cycle.manifest.cycle_id)
            .unwrap()
            .manifest
            .status,
        CycleStatus::Open
    );
    assert_eq!(storage.list_events().unwrap().len(), 1);
}

fn storage_with_cycle() -> (Storage, CycleRecord) {
    let storage = Storage::open_in_memory().unwrap();
    storage.insert_project(&project_record()).unwrap();
    storage.insert_workspace(&workspace_record()).unwrap();
    let cycle = cycle_record();
    storage.insert_cycle(&cycle).unwrap();
    (storage, cycle)
}

fn project_record() -> ProjectRecord {
    ProjectRecord {
        project_id: "project-1".into(),
        display_name: "Project One".into(),
        remote_url: Some("https://example.com/owner/project".into()),
        scope: "owner".into(),
        created_at: CREATED_AT.into(),
    }
}

fn workspace_record() -> WorkspaceRecord {
    WorkspaceRecord {
        workspace_id: "workspace-1".into(),
        project_id: "project-1".into(),
        canonical_path: "/work/project".into(),
        created_at: CREATED_AT.into(),
    }
}

fn cycle_record() -> CycleRecord {
    let manifest = CycleManifest::new(
        "project-1".into(),
        "workspace-1".into(),
        CycleId::new("project-1/change").unwrap(),
        "Change".into(),
        "sddk/change".into(),
        "abc123".into(),
    );
    CycleRecord {
        manifest,
        created_at: CREATED_AT.into(),
        updated_at: CREATED_AT.into(),
    }
}

fn event(event_id: &str, cycle_id: Option<&str>) -> LedgerEventInput {
    LedgerEventInput {
        event_id: event_id.into(),
        project_id: "project-1".into(),
        cycle_id: cycle_id.map(str::to_owned),
        frame_id: "frame-1".into(),
        command_id: "command-1".into(),
        actor: "runtime".into(),
        event_type: "cycle.state_changed".into(),
        occurred_at: CREATED_AT.into(),
        state_before: None,
        state_after: None,
        payload: json!({"event": event_id}),
    }
}

fn capability_receipt(receipt_id: &str, request: serde_json::Value) -> CapabilityReceiptInput {
    CapabilityReceiptInput {
        receipt_id: receipt_id.into(),
        project_id: "project-1".into(),
        cycle_id: None,
        capability: "git.create_branch".into(),
        idempotency_key: "create-feature-branch".into(),
        request,
        status: CapabilityStatus::Started,
        result: None,
        started_at: CREATED_AT.into(),
        completed_at: None,
    }
}
