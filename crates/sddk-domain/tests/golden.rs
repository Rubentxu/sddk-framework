//! Golden dataset runner (SPEC-014, Phase 9).
//!
//! Loads `fixtures/golden/*.yaml`, applies each case's events to a fresh
//! `GraphProjection`, and asserts the resulting node/edge counts match the
//! expectation. This is the deterministic regression ratchet for the graph.

use sddk_domain::{ActorKind, ActorRef, EntityRef, EventEnvelopeV1, GraphProjection, Projection};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct GoldenCase {
    name: String,
    #[serde(default)]
    events: Vec<GoldenEvent>,
    expect: GoldenExpect,
}

#[derive(Debug, Deserialize)]
struct GoldenEvent {
    event_type: String,
    #[serde(default)]
    subjects: Vec<Vec<String>>,
    #[serde(default)]
    cycle_id: Option<String>,
    #[serde(default)]
    payload: Value,
}

#[derive(Debug, Deserialize)]
struct GoldenExpect {
    nodes: usize,
    edges: usize,
}

fn golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/golden")
}

fn load_cases() -> Vec<GoldenCase> {
    let mut cases = Vec::new();
    let dir = golden_dir();
    let mut entries: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("golden dir {dir:?}: {e}"))
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map(|e| e == "yaml")
                .unwrap_or(false)
        })
        .collect();
    entries.sort_by_key(|entry| entry.file_name());
    for entry in entries {
        let content = std::fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("read {}: {e}", entry.path().display()));
        let case: GoldenCase = serde_saphyr::from_str(&content)
            .unwrap_or_else(|e| panic!("parse {}: {e}", entry.path().display()));
        cases.push(case);
    }
    cases
}

fn apply_case(case: &GoldenCase) -> (usize, usize) {
    let mut projection = GraphProjection::new("project:golden");
    for (i, event) in case.events.iter().enumerate() {
        let envelope = EventEnvelopeV1 {
            event_id: format!("golden-{}-{i}", case.name),
            event_type: event.event_type.clone(),
            schema_version: 1,
            stream_id: "project:golden".into(),
            sequence: (i + 1) as u64,
            project_id: "project:golden".into(),
            occurred_at: format!("2026-08-18T10:00:{i:02}Z"),
            recorded_at: format!("2026-08-18T10:00:{i:02}Z"),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "golden".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects: event
                .subjects
                .iter()
                .map(|s| EntityRef {
                    kind: s[0].clone(),
                    id: s[1].clone(),
                    version: None,
                    content_hash: None,
                })
                .collect(),
            payload: if event.payload.is_null() {
                Value::Null
            } else {
                event.payload.clone()
            },
            evidence_refs: vec![],
            content_hash: format!("sha256:{i:064x}"),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: event.cycle_id.clone(),
            frame_id: None,
            fork_id: None,
        };
        projection.apply(&envelope).unwrap();
    }
    let state = projection.state_ref();
    (state.nodes.len(), state.edges.len())
}

#[test]
fn golden_dataset_matches_expectations() {
    let cases = load_cases();
    assert!(
        cases.len() >= 10,
        "golden dataset must have at least 10 cases, got {}",
        cases.len()
    );
    for case in &cases {
        let (nodes, edges) = apply_case(case);
        assert_eq!(
            nodes, case.expect.nodes,
            "case '{}': expected {} nodes, got {}",
            case.name, case.expect.nodes, nodes
        );
        assert_eq!(
            edges, case.expect.edges,
            "case '{}': expected {} edges, got {}",
            case.name, case.expect.edges, edges
        );
    }
}

#[test]
fn golden_dataset_is_deterministic() {
    let cases = load_cases();
    for case in &cases {
        let (a_nodes, a_edges) = apply_case(case);
        let (b_nodes, b_edges) = apply_case(case);
        assert_eq!(
            (a_nodes, a_edges),
            (b_nodes, b_edges),
            "case '{}' nondeterministic",
            case.name
        );
    }
}
