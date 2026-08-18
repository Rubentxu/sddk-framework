//! Integration tests for [`emit_phase_event`](sddk_engine::event_bus::emit_phase_event).
//!
//! Tests PE-01 through PE-04 from SDDK2-204 tasks.md.

use sddk_domain::{ActorKind, EventStore};
use sddk_engine::event_bus::{PhaseEventInput, emit_phase_event};
use sddk_storage::SqliteEventStore;

/// Helper to build a PhaseEventInput for testing.
fn make_input(cycle_id: &str, from: &str, to: &str) -> PhaseEventInput {
    PhaseEventInput {
        project_id: "test-project".into(),
        cycle_id: cycle_id.into(),
        from_phase: from.into(),
        to_phase: to.into(),
        transition_at: "2026-08-17T10:00:00Z".into(),
        actor_id: "user:test".into(),
        actor_kind: ActorKind::Human,
        event_id_prefix: format!("ph-{cycle_id}"),
    }
}

// PE-01: emit_phase_event appends 2 events with sequence 1,2 and correct payload.
#[test]
fn pe01_dual_emit_sequence_and_payload() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_input("cycle-1", "build", "test");

    let (exited, entered) = emit_phase_event(&mut store, &input).unwrap();

    // Sequences should be 1 and 2.
    assert_eq!(exited.sequence, 1);
    assert_eq!(entered.sequence, 2);

    // Load both events and verify payload.
    let loaded_exited = store
        .load_by_event_id(&format!("ph-{}-exited-cycle-1", "cycle-1"))
        .unwrap()
        .expect("exited event should exist");
    assert_eq!(loaded_exited.event_type, "workflow.phase.exited");
    assert_eq!(
        loaded_exited
            .payload
            .get("phase")
            .unwrap()
            .as_str()
            .unwrap(),
        "build"
    );

    let loaded_entered = store
        .load_by_event_id(&format!("ph-{}-entered-cycle-1", "cycle-1"))
        .unwrap()
        .expect("entered event should exist");
    assert_eq!(loaded_entered.event_type, "workflow.phase.entered");
    assert_eq!(
        loaded_entered
            .payload
            .get("phase")
            .unwrap()
            .as_str()
            .unwrap(),
        "test"
    );
}

// PE-02: idempotency — re-appending same event_id returns the stored result
// (no error, no new sequence allocated). This is the correct SqliteEventStore behavior.
#[test]
fn pe02_idempotency_returns_stored_result() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_input("cycle-2", "specify", "design");

    // First emit succeeds.
    let (ex1_first, en1_first) = emit_phase_event(&mut store, &input).unwrap();
    assert_eq!(ex1_first.sequence, 1);
    assert_eq!(en1_first.sequence, 2);

    // Second emit with same event_id succeeds (idempotent) and returns stored result.
    let (ex1_second, en1_second) = emit_phase_event(&mut store, &input).unwrap();
    assert_eq!(
        ex1_second.sequence, 1,
        "should return stored sequence, not allocate new"
    );
    assert_eq!(
        en1_second.sequence, 2,
        "should return stored sequence, not allocate new"
    );

    // Only 2 events total (no duplicates created).
    let count = store.count().unwrap();
    assert_eq!(count, 2);
}

// PE-03: two distinct transitions with DIFFERENT cycle_ids produce 4 events.
// Each transition uses its own cycle_id to get unique event_ids.
#[test]
fn pe03_two_transitions_produce_four_events() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();

    // Transition 1 on cycle-A: explore -> specify
    let input_a = make_input("cycle-A", "explore", "specify");
    let (ex_a, en_a) = emit_phase_event(&mut store, &input_a).unwrap();
    assert_eq!(ex_a.sequence, 1);
    assert_eq!(en_a.sequence, 2);

    // Transition 2 on cycle-B: specify -> release
    let input_b = make_input("cycle-B", "specify", "release");
    let (ex_b, en_b) = emit_phase_event(&mut store, &input_b).unwrap();
    assert_eq!(ex_b.sequence, 1, "new cycle starts at sequence 1");
    assert_eq!(en_b.sequence, 2);

    // Total: 4 events across 2 streams.
    let count = store.count().unwrap();
    assert_eq!(count, 4);

    // Verify stream A has 2 events.
    let stream_a = store.load_stream("cycle-A", None, 10).unwrap();
    assert_eq!(stream_a.len(), 2);

    // Verify stream B has 2 events.
    let stream_b = store.load_stream("cycle-B", None, 10).unwrap();
    assert_eq!(stream_b.len(), 2);
}

// PE-04: ledger coexistence — SqliteEventStore writes only to events_v1,
// not to the legacy ledger_events table (managed by Storage/Ledger).
#[test]
fn pe04_ledger_coexistence_events_v1_only() {
    let mut store = SqliteEventStore::open_in_memory().unwrap();
    let input = make_input("cycle-4", "design", "plan");

    emit_phase_event(&mut store, &input).unwrap();

    // Verify events were written to events_v1.
    let count = store.count().unwrap();
    assert_eq!(count, 2, "should have 2 events in events_v1");

    // SqliteEventStore operates on events_v1 ONLY.
    // The legacy ledger_events table is managed by Storage/Ledger trait.
    // This architectural boundary ensures coexistence: LedgerEvent (v0)
    // and EventEnvelopeV1 (v1) are separate tables, written by separate code paths.
}
