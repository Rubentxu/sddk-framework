//! Golden vector integration tests for EventEnvelopeV1 against uat-acceptance.jsonl.
//!
//! The fixture `docs/sddk-2.0-architecture-consolidation/examples/events/uat-acceptance.jsonl`
//! carries placeholder `content_hash` values. Run the `regenerate_uat_acceptance_jsonl`
//! ignored test to regenerate it with real SHA-256 hashes:
//!
//! ```
//! cargo test -p sddk-domain --test event_envelope_golden -- --ignored --nocapture
//! ```

use sddk_domain::{
    schema, ActorKind, ActorRef, EntityRef, EntityRefVersion, EventEnvelopeV1,
};
use serde_json::json;
use std::fs;
use std::path::PathBuf;

const JSONL_FIXTURE: &str =
    include_str!("../../../docs/sddk-2.0-architecture-consolidation/examples/events/uat-acceptance.jsonl");
const SCHEMA_JSON: &str =
    include_str!("../../../docs/sddk-2.0-architecture-consolidation/schemas/event-envelope.schema.json");

/// Path relative to this crate's manifest dir (for the regenerate helper).
const FIXTURE_PATH: &str =
    "../../docs/sddk-2.0-architecture-consolidation/examples/events/uat-acceptance.jsonl";

/// Regenerates `uat-acceptance.jsonl` with real SHA-256 content_hash values.
///
/// Build the 3 events from the fixture data, compute their canonical hashes,
/// then write back with the correct `content_hash` field.
#[test]
#[ignore = "regenerates uat-acceptance.jsonl with real sha256 content_hash values; run manually"]
fn regenerate_uat_acceptance_jsonl() {
    let mut e1 = build_event_1();
    let mut e2 = build_event_2();
    let mut e3 = build_event_3();

    // Clear the placeholder content_hash, then compute the real hash.
    // Since content_hash is a required field in the struct, we must set it
    // to a stable placeholder before computing so the hash is deterministic.
    e1.content_hash.clear();
    e1.content_hash = e1.compute_content_hash();

    e2.content_hash.clear();
    e2.content_hash = e2.compute_content_hash();

    e3.content_hash.clear();
    e3.content_hash = e3.compute_content_hash();

    let mut out = String::new();
    out.push_str(&serde_json::to_string(&e1).unwrap());
    out.push('\n');
    out.push_str(&serde_json::to_string(&e2).unwrap());
    out.push('\n');
    out.push_str(&serde_json::to_string(&e3).unwrap());
    out.push('\n');

    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FIXTURE_PATH);
    println!("Path: {:?}", path);
    fs::write(path, &out).expect("write fixture");
    println!("Regenerated fixture with\ne1: {}\ne2: {}\ne3: {}",
             e1.content_hash, e2.content_hash, e3.content_hash);
}

// ---------------------------------------------------------------------------
// Event builders — mirror the data in uat-acceptance.jsonl
// ---------------------------------------------------------------------------

fn build_event_1() -> EventEnvelopeV1 {
    // evt-100: uat.scenario.started
    // subjects: [{type: "uat_scenario", id: "UAT-17", version: 4, content_hash: "sha256:aaa..."}]
    // correlation_id: "uat-run-9", cycle_id: "C42", frame_id: "uat-frame-1"
    // metadata: {}
    EventEnvelopeV1 {
        event_id: "evt-100".into(),
        event_type: "uat.scenario.started".into(),
        schema_version: 1,
        stream_id: "cycle-C42".into(),
        sequence: 100,
        project_id: "p-demo".into(),
        occurred_at: "2026-08-11T20:00:00Z".into(),
        recorded_at: "2026-08-11T20:00:00Z".into(),
        actor: ActorRef {
            kind: ActorKind::Human,
            id: "user-1".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "uat_scenario".into(),
            id: "UAT-17".into(),
            version: Some(EntityRefVersion::Integer(4)),
            content_hash: Some("sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
        }],
        payload: json!({"mode": "runner"}),
        evidence_refs: vec![],
        content_hash: "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
        metadata: Some(json!({})),
        causation_id: None,
        correlation_id: Some("uat-run-9".into()),
        cycle_id: Some("C42".into()),
        frame_id: Some("uat-frame-1".into()),
        fork_id: None,
    }
}

fn build_event_2() -> EventEnvelopeV1 {
    // evt-101: uat.check.passed
    // causation_id: "evt-100", correlation_id: "uat-run-9"
    // cycle_id: "C42", frame_id: "uat-frame-1"
    // subjects: [{type: "uat_check", id: "CHK-17-3", version: 1, content_hash: null}]
    // evidence_refs: ["E-991"]
    // metadata: {}
    EventEnvelopeV1 {
        event_id: "evt-101".into(),
        event_type: "uat.check.passed".into(),
        schema_version: 1,
        stream_id: "cycle-C42".into(),
        sequence: 101,
        project_id: "p-demo".into(),
        occurred_at: "2026-08-11T20:03:00Z".into(),
        recorded_at: "2026-08-11T20:03:00Z".into(),
        actor: ActorRef {
            kind: ActorKind::Human,
            id: "user-1".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "uat_check".into(),
            id: "CHK-17-3".into(),
            version: Some(EntityRefVersion::Integer(1)),
            content_hash: None,
        }],
        payload: json!({"verdict": "pass"}),
        evidence_refs: vec!["E-991".into()],
        content_hash: "sha256:cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".into(),
        metadata: Some(json!({})),
        causation_id: Some("evt-100".into()),
        correlation_id: Some("uat-run-9".into()),
        cycle_id: Some("C42".into()),
        frame_id: Some("uat-frame-1".into()),
        fork_id: None,
    }
}

fn build_event_3() -> EventEnvelopeV1 {
    // evt-109: uat.acceptance.granted
    // causation_id: "evt-108", correlation_id: "uat-run-9"
    // cycle_id: "C42", frame_id: "uat-frame-1"
    // subjects: [{type: "release_candidate", id: "rc-2.0.0", version: "git:abc123", content_hash: null}]
    // evidence_refs: ["E-991", "E-992"]
    // metadata: {}
    // payload has nested sha256 value
    EventEnvelopeV1 {
        event_id: "evt-109".into(),
        event_type: "uat.acceptance.granted".into(),
        schema_version: 1,
        stream_id: "cycle-C42".into(),
        sequence: 109,
        project_id: "p-demo".into(),
        occurred_at: "2026-08-11T20:12:00Z".into(),
        recorded_at: "2026-08-11T20:12:00Z".into(),
        actor: ActorRef {
            kind: ActorKind::Human,
            id: "user-1".into(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "release_candidate".into(),
            id: "rc-2.0.0".into(),
            version: Some(EntityRefVersion::String("git:abc123".into())),
            content_hash: None,
        }],
        payload: json!({"acceptance_record_hash": "sha256:dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd"}),
        evidence_refs: vec!["E-991".into(), "E-992".into()],
        content_hash: "sha256:eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".into(),
        metadata: Some(json!({})),
        causation_id: Some("evt-108".into()),
        correlation_id: Some("uat-run-9".into()),
        cycle_id: Some("C42".into()),
        frame_id: Some("uat-frame-1".into()),
        fork_id: None,
    }
}

// ---------------------------------------------------------------------------
// Golden vector integration tests
// ---------------------------------------------------------------------------

/// Verifies that each event in the fixture has a self-consistent content_hash:
/// re-computing `compute_content_hash()` yields the declared value.
#[test]
fn golden_vectors_match_content_hash() {
    for (i, line) in JSONL_FIXTURE.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let mut env: EventEnvelopeV1 = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("event {i} parse error: {e}\nline: {line}"));
        // The content_hash field is part of the canonical bytes. To make it
        // self-consistent we temporarily blank it before computing, then compare
        // the result with the declared value.
        let declared_hash = env.content_hash.clone();
        env.content_hash.clear();
        let computed = env.compute_content_hash();
        assert_eq!(
            declared_hash, computed,
            "event {i} content_hash mismatch; expected {declared_hash}, got {computed}"
        );
    }
}

/// Verifies that each event in the fixture passes JSON Schema validation.
#[test]
fn golden_vectors_pass_schema_validation() {
    for (i, line) in JSONL_FIXTURE.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let v: serde_json::Value = serde_json::from_str(line)
            .unwrap_or_else(|e| panic!("event {i} parse: {e}"));
        let result = schema::validate_against_schema_str(&v, SCHEMA_JSON);
        assert!(
            result.is_ok(),
            "event {i} failed schema validation: {:?}",
            result.err()
        );
    }
}

/// Verifies that each event in the fixture parses correctly as EventEnvelopeV1
/// with the expected schema_version and a valid event_type.
#[test]
fn golden_vectors_parse_as_event_envelope() {
    let mut count = 0;
    for line in JSONL_FIXTURE.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let env: EventEnvelopeV1 = serde_json::from_str(line)
            .expect("event should deserialize to EventEnvelopeV1");
        assert_eq!(env.schema_version, 1, "event {count}: schema_version should be 1");
        assert!(
            EventEnvelopeV1::validate_event_type(&env.event_type).is_ok(),
            "event {count}: event_type {:?} should be valid",
            env.event_type
        );
        count += 1;
    }
    assert_eq!(count, 3, "fixture should have exactly 3 events");
}
