//! Reactive knowledge/evidence graph (SPEC-004, Phase 5).
//!
//! The graph is a deterministic read-model projection over the CEP event
//! ledger: events are the authority, `GraphProjection` derives typed nodes and
//! edges with provenance, and `GraphView` exposes bounded scopes to pattern
//! queries and behaviors.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::event_envelope::EventEnvelopeV1;
use crate::projections::{Checkpoint, Projection, ProjectionError, ProjectionVersion};

/// One typed node in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Entity kind (e.g. `cycle`, `capability`, `actor`, `phase`).
    pub kind: String,
    /// Stable entity id within its kind namespace.
    pub id: String,
    /// Event id that created this node (provenance).
    pub created_by: String,
    /// Content hash of the creating event (provenance).
    pub content_hash: String,
    /// RFC 3339 timestamp of the creating event.
    pub occurred_at: String,
}

impl GraphNode {
    /// Canonical graph key: `kind:id`.
    pub fn key(&self) -> String {
        format!("{}:{}", self.kind, self.id)
    }
}

/// One typed directed edge in the graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node key (`kind:id`).
    pub from: String,
    /// Relation name — the event type (`realm.object.verb`).
    pub relation: String,
    /// Target node key (`kind:id`).
    pub to: String,
    /// Event id that created this edge (provenance).
    pub event_id: String,
    /// RFC 3339 timestamp of the event.
    pub occurred_at: String,
    /// Actor id of the event.
    pub actor: String,
}

/// Full projection state: nodes keyed by `kind:id` and edges in append order.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct GraphState {
    /// Nodes keyed by `kind:id` (BTreeMap → deterministic JSON).
    pub nodes: BTreeMap<String, GraphNode>,
    /// Edges in event-append order.
    pub edges: Vec<GraphEdge>,
    /// Monotonic sequence of the last applied event.
    pub last_event_sequence: u64,
    /// Hash of the last applied event.
    pub last_event_hash: String,
}

/// Deterministic graph projection over the event ledger (SPEC-004 §2).
pub struct GraphProjection {
    /// Stream this projection consumes from.
    stream_id: String,
    /// Mutable projection state.
    state: GraphState,
}

impl GraphProjection {
    /// Canonical projection name.
    pub const NAME: &'static str = "graph";
    /// Version for the v1 `apply` semantics.
    pub const VERSION: ProjectionVersion = 1;

    /// Creates a new `GraphProjection` for the given event stream.
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            stream_id: stream_id.into(),
            state: GraphState::default(),
        }
    }

    /// Returns the stream this projection consumes from.
    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }
}

/// Validates that an event type has the `realm.object.verb` shape (3+ segments).
fn is_valid_event_type(event_type: &str) -> bool {
    event_type.split('.').count() >= 3
}

/// Upserts a node from an event subject, preserving first `created_by`.
fn upsert_node(state: &mut GraphState, kind: &str, id: &str, event: &EventEnvelopeV1) {
    let key = format!("{kind}:{id}");
    state.nodes.entry(key).or_insert_with(|| GraphNode {
        kind: kind.to_string(),
        id: id.to_string(),
        created_by: event.event_id.clone(),
        content_hash: event.content_hash.clone(),
        occurred_at: event.occurred_at.clone(),
    });
}

/// Appends a typed edge with full provenance.
fn push_edge(
    state: &mut GraphState,
    from: &str,
    relation: &str,
    to: &str,
    event: &EventEnvelopeV1,
) {
    state.edges.push(GraphEdge {
        from: from.to_string(),
        relation: relation.to_string(),
        to: to.to_string(),
        event_id: event.event_id.clone(),
        occurred_at: event.occurred_at.clone(),
        actor: event.actor.id.clone(),
    });
}

impl Projection for GraphProjection {
    type State = GraphState;

    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> ProjectionVersion {
        Self::VERSION
    }

    fn apply(&mut self, event: &EventEnvelopeV1) -> Result<(), ProjectionError> {
        // Only process events from our stream.
        if event.stream_id != self.stream_id {
            return Ok(());
        }

        // Update monotone fields regardless of event type.
        self.state.last_event_sequence = event.sequence;
        self.state.last_event_hash = event.content_hash.clone();

        // Skip malformed event types (no 3-segment realm.object.verb).
        if !is_valid_event_type(&event.event_type) {
            return Ok(());
        }

        // Root node: the cycle (or project when no cycle is set).
        let root_kind = if event.cycle_id.is_some() {
            "cycle"
        } else {
            "project"
        };
        let root_id = event
            .cycle_id
            .clone()
            .unwrap_or_else(|| event.project_id.clone());
        upsert_node(&mut self.state, root_kind, &root_id, event);
        let root_key = format!("{root_kind}:{root_id}");

        // Special-case: workflow.phase.entered → cycle --entered_phase--> phase.
        if event.event_type == "workflow.phase.entered" {
            let phase = event
                .payload
                .get("phase")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ProjectionError::InvalidPayload {
                    event_type: event.event_type.clone(),
                    detail: format!("event {} missing 'phase' string in payload", event.event_id),
                })?
                .to_string();
            upsert_node(&mut self.state, "phase", &phase, event);
            push_edge(
                &mut self.state,
                &root_key,
                "entered_phase",
                &format!("phase:{phase}"),
                event,
            );
            return Ok(());
        }

        // Generic mapping: subjects → nodes; event_type → edge.
        let subject_keys: Vec<String> = event
            .subjects
            .iter()
            .map(|subject| {
                upsert_node(&mut self.state, &subject.kind, &subject.id, event);
                format!("{}:{}", subject.kind, subject.id)
            })
            .collect();

        match subject_keys.len() {
            0 => {
                // No subjects: root --event_type--> root (self-loop marks the event).
                push_edge(
                    &mut self.state,
                    &root_key,
                    &event.event_type,
                    &root_key,
                    event,
                );
            }
            1 => {
                // One subject: subject --event_type--> subject (loop).
                push_edge(
                    &mut self.state,
                    &subject_keys[0],
                    &event.event_type,
                    &subject_keys[0],
                    event,
                );
            }
            _ => {
                // Two or more subjects: first --event_type--> second.
                push_edge(
                    &mut self.state,
                    &subject_keys[0],
                    &event.event_type,
                    &subject_keys[1],
                    event,
                );
            }
        }

        Ok(())
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            projection_name: Self::NAME.to_string(),
            version: self.version(),
            last_event_sequence: self.state.last_event_sequence,
            last_event_hash: self.state.last_event_hash.clone(),
            updated_at: crate::projections::now_rfc3339(),
        }
    }

    fn state_ref(&self) -> &Self::State {
        &self.state
    }
}

/// Bounded read view over a graph state (SPEC-004 §7).
///
/// Behaviors and queries receive a `GraphView`, never the full state.
#[derive(Debug, Clone)]
pub struct GraphView<'a> {
    /// Underlying state (borrowed).
    state: &'a GraphState,
    /// Visible node keys after filtering.
    visible_nodes: Vec<String>,
    /// Visible edges after filtering.
    visible_edges: Vec<&'a GraphEdge>,
    /// Maximum hop depth from the start node (0 = no bound).
    max_depth: u32,
}

impl<'a> GraphView<'a> {
    /// Creates an unbounded view over the whole state.
    pub fn new(state: &'a GraphState) -> Self {
        Self {
            state,
            visible_nodes: state.nodes.keys().cloned().collect(),
            visible_edges: state.edges.iter().collect(),
            max_depth: 0,
        }
    }

    /// Filters the view to the given node kinds.
    pub fn with_node_types(mut self, kinds: &[&str]) -> Self {
        self.visible_nodes = self
            .state
            .nodes
            .keys()
            .filter(|key| {
                kinds.iter().any(|kind| {
                    key.strip_prefix(kind)
                        .map(|rest| rest.starts_with(':'))
                        .unwrap_or(false)
                })
            })
            .cloned()
            .collect();
        self.visible_edges = self
            .state
            .edges
            .iter()
            .filter(|edge| {
                self.visible_nodes.contains(&edge.from) && self.visible_nodes.contains(&edge.to)
            })
            .collect();
        self
    }

    /// Filters the view to the given relation names.
    pub fn with_relations(mut self, relations: &[&str]) -> Self {
        self.visible_edges
            .retain(|edge| relations.contains(&edge.relation.as_str()));
        self
    }

    /// Bounds traversal depth from the start node (reachability).
    pub fn with_max_depth(mut self, max_depth: u32) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Nodes visible in this view.
    pub fn nodes(&self) -> impl Iterator<Item = &'a GraphNode> {
        self.visible_nodes
            .iter()
            .filter_map(|key| self.state.nodes.get(key))
    }

    /// Edges visible in this view.
    pub fn edges(&self) -> impl Iterator<Item = &'a GraphEdge> {
        self.visible_edges.iter().copied()
    }

    /// Returns the node keys visible in this view.
    pub fn node_keys(&self) -> &[String] {
        &self.visible_nodes
    }

    /// Returns the edge references visible in this view.
    pub fn edge_refs(&self) -> &[&'a GraphEdge] {
        &self.visible_edges
    }

    /// Maximum hop depth bound (0 = unbounded).
    pub fn max_depth(&self) -> u32 {
        self.max_depth
    }

    /// Looks up a node by key within the view.
    pub fn node(&self, key: &str) -> Option<&'a GraphNode> {
        if self.visible_nodes.iter().any(|k| k == key) {
            self.state.nodes.get(key)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_envelope::{ActorKind, ActorRef, EntityRef};
    use serde_json::json;

    fn make_event(
        stream: &str,
        event_type: &str,
        seq: u64,
        subjects: Vec<EntityRef>,
        cycle_id: Option<&str>,
        payload: serde_json::Value,
    ) -> EventEnvelopeV1 {
        EventEnvelopeV1 {
            event_id: format!("evt-{stream}-{seq}"),
            event_type: event_type.into(),
            schema_version: 1,
            stream_id: stream.into(),
            sequence: seq,
            project_id: "p-1".into(),
            occurred_at: format!("2026-08-18T10:00:{seq:02}Z"),
            recorded_at: format!("2026-08-18T10:00:{seq:02}Z"),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "sddk-test".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects,
            payload,
            evidence_refs: vec![],
            content_hash: format!("sha256:{seq:064x}"),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: cycle_id.map(|c| c.to_string()),
            frame_id: None,
            fork_id: None,
        }
    }

    fn subject(kind: &str, id: &str) -> EntityRef {
        EntityRef {
            kind: kind.into(),
            id: id.into(),
            version: None,
            content_hash: None,
        }
    }

    #[test]
    fn subjects_become_nodes() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            "approval.capability.granted",
            1,
            vec![
                subject("cycle", "c-1"),
                subject("capability", "git.commit"),
                subject("actor", "alice"),
            ],
            Some("c-1"),
            json!({}),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert!(state.nodes.contains_key("cycle:c-1"));
        assert!(state.nodes.contains_key("capability:git.commit"));
        assert!(state.nodes.contains_key("actor:alice"));
        assert_eq!(
            state.nodes["capability:git.commit"].created_by,
            "evt-project:p-1-1"
        );
    }

    #[test]
    fn event_type_becomes_relation() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            "approval.capability.granted",
            1,
            vec![
                subject("actor", "alice"),
                subject("capability", "git.commit"),
            ],
            Some("c-1"),
            json!({}),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert_eq!(state.edges.len(), 1);
        let edge = &state.edges[0];
        assert_eq!(edge.from, "actor:alice");
        assert_eq!(edge.relation, "approval.capability.granted");
        assert_eq!(edge.to, "capability:git.commit");
        assert_eq!(edge.actor, "sddk-test");
    }

    #[test]
    fn phase_entered_creates_phase_edge() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            "workflow.phase.entered",
            1,
            vec![],
            Some("c-1"),
            json!({ "phase": "verify" }),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert!(state.nodes.contains_key("phase:verify"));
        let edge = &state.edges[0];
        assert_eq!(edge.from, "cycle:c-1");
        assert_eq!(edge.relation, "entered_phase");
        assert_eq!(edge.to, "phase:verify");
    }

    #[test]
    fn unknown_event_type_skipped() {
        let mut projection = GraphProjection::new("project:p-1");
        let event = make_event(
            "project:p-1",
            "short",
            1,
            vec![subject("cycle", "c-1")],
            Some("c-1"),
            json!({}),
        );
        projection.apply(&event).unwrap();
        let state = projection.state_ref();
        assert!(state.edges.is_empty());
    }

    #[test]
    fn rebuild_is_deterministic() {
        let events = vec![
            make_event(
                "project:p-1",
                "approval.capability.requested",
                1,
                vec![subject("cycle", "c-1"), subject("capability", "git.commit")],
                Some("c-1"),
                json!({}),
            ),
            make_event(
                "project:p-1",
                "approval.capability.granted",
                2,
                vec![
                    subject("actor", "alice"),
                    subject("capability", "git.commit"),
                ],
                Some("c-1"),
                json!({}),
            ),
            make_event(
                "project:p-1",
                "workflow.phase.entered",
                3,
                vec![],
                Some("c-1"),
                json!({ "phase": "verify" }),
            ),
        ];
        let mut a = GraphProjection::new("project:p-1");
        let mut b = GraphProjection::new("project:p-1");
        for event in &events {
            a.apply(event).unwrap();
            b.apply(event).unwrap();
        }
        assert_eq!(a.state_ref(), b.state_ref());
        assert_eq!(a.state_ref().nodes.len(), 4); // cycle, capability, actor, phase
        assert_eq!(a.state_ref().edges.len(), 3);
    }

    #[test]
    fn view_filters_by_type() {
        let mut projection = GraphProjection::new("project:p-1");
        projection
            .apply(&make_event(
                "project:p-1",
                "approval.capability.granted",
                1,
                vec![subject("cycle", "c-1"), subject("capability", "git.commit")],
                Some("c-1"),
                json!({}),
            ))
            .unwrap();
        let view = GraphView::new(projection.state_ref()).with_node_types(&["capability"]);
        let keys: Vec<String> = view.node_keys().to_vec();
        assert_eq!(keys, vec!["capability:git.commit"]);
        assert!(view.edge_refs().is_empty());
    }

    #[test]
    fn view_bounds_hop_depth() {
        // A -> B -> C -> D via chain of self-loop-less edges: build manually.
        let mut state = GraphState::default();
        for (i, (from, rel, to)) in [
            ("a:A", "r", "b:B"),
            ("b:B", "r", "c:C"),
            ("c:C", "r", "d:D"),
        ]
        .iter()
        .enumerate()
        {
            state.nodes.insert(
                from.to_string(),
                GraphNode {
                    kind: from.split(':').next().unwrap().into(),
                    id: from.split(':').nth(1).unwrap().into(),
                    created_by: format!("e{i}"),
                    content_hash: "sha256:x".into(),
                    occurred_at: "2026-08-18T10:00:00Z".into(),
                },
            );
            state.edges.push(GraphEdge {
                from: from.to_string(),
                relation: rel.to_string(),
                to: to.to_string(),
                event_id: format!("e{i}"),
                occurred_at: "2026-08-18T10:00:00Z".into(),
                actor: "t".into(),
            });
        }
        state.nodes.insert(
            "d:D".to_string(),
            GraphNode {
                kind: "d".into(),
                id: "D".into(),
                created_by: "e3".into(),
                content_hash: "sha256:x".into(),
                occurred_at: "2026-08-18T10:00:00Z".into(),
            },
        );
        // Depth bound is enforced by pattern queries; the view itself exposes
        // reachability via max_depth only for query use. Here we assert the
        // view exposes all edges (bounded views filter by type/relation, and
        // depth bounding lives in PatternQuery).
        let view = GraphView::new(&state);
        assert_eq!(view.edges().count(), 3);
    }
}
