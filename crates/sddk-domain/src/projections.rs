//! Read-model projections over [`EventEnvelopeV1`].
//!
//! Projections are deterministic, stateless read-models that derive from the
//! append-only event ledger. Each projection implements [`Projection::apply`] to
//! consume events and [`Projection::checkpoint`] to produce a durable progress
//! marker. The [`rebuild`] algorithm in `sddk_storage` uses these to reconstruct
//! a projection from the ledger.
//!
//! [`rebuild`]: sddk_storage::rebuild

use serde::{Deserialize, Serialize};

/// Schema version for a projection — bumped when [`apply`](Projection::apply) semantics change.
pub type ProjectionVersion = u32;

/// Persistent checkpoint for a projection, persisted to the
/// `projection_checkpoints_v1` table.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Checkpoint {
    /// Canonical projection name used in the checkpoint table primary key.
    pub projection_name: String,
    /// Schema version of the projection that wrote this checkpoint.
    pub version: ProjectionVersion,
    /// Monotonic sequence number of the last event applied.
    pub last_event_sequence: u64,
    /// SHA-256 content hash of the last event applied, in `sha256:<64-hex>` format.
    pub last_event_hash: String,
    /// Wall-clock time when this checkpoint was written (RFC 3339).
    pub updated_at: String,
}

/// Errors that may arise when applying events or rebuilding a projection.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProjectionError {
    /// The named projection is not registered.
    #[error("projection '{0}' is not registered")]
    UnknownProjection(String),

    /// An event payload could not be interpreted.
    #[error("invalid event payload for '{event_type}': {detail}")]
    InvalidPayload {
        /// The event type that failed parsing.
        event_type: String,
        /// Why parsing failed.
        detail: String,
    },

    /// The event store's content-hash chain is broken at the given sequence.
    /// The rebuild algorithm fails closed: no checkpoint is persisted when
    /// chain integrity is lost.
    #[error("event chain integrity broken for stream '{stream_id}' at sequence {sequence}")]
    ChainIntegrityBroken {
        /// Stream where the break was detected.
        stream_id: String,
        /// Sequence at which verification failed.
        sequence: u64,
    },

    /// Underlying storage failure.
    #[error("storage: {0}")]
    Storage(String),
}

/// A read-model projection over [`EventEnvelopeV1`]. The projection must
/// be deterministic for a fixed input stream: calling [`apply`](Projection::apply)
/// with the same ordered events always produces the same checkpoint.
///
/// Implementations are expected to be idempotent for a given `(event_id, event_hash)`.
///
/// [`EventEnvelopeV1`]: super::EventEnvelopeV1
pub trait Projection {
    /// The serialized state produced by this projection.
    type State: Serialize + for<'de> Deserialize<'de> + Default + Clone;

    /// Canonical name used as the primary key in the checkpoint table.
    fn name(&self) -> &str;

    /// Schema version. Increase when [`apply`](Projection::apply) semantics change.
    fn version(&self) -> ProjectionVersion;

    /// Apply one event to the projection's state.
    ///
    /// Implementations must update monotone fields (`last_event_sequence`,
    /// `last_event_hash`) on every call regardless of event type, so that a
    /// restarted rebuild can pick up where it left off.
    fn apply(&mut self, event: &super::EventEnvelopeV1) -> Result<(), ProjectionError>;

    /// Build the current checkpoint from in-memory state.
    fn checkpoint(&self) -> Checkpoint;

    /// Borrow the current state for serialization.
    fn state_ref(&self) -> &Self::State;
}

/// Tracks the current phase of a single cycle's workflow.
///
/// The projection's `apply` method handles `workflow.phase.entered` and
/// `workflow.phase.exited` event types and ignores others.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CycleState {
    /// Current phase label, or `"unknown"` before any `phase.entered` event.
    pub phase: String,
    /// Monotonic sequence number of the last event applied to this projection.
    pub last_event_sequence: u64,
    /// Hash of the last event applied (for [`Checkpoint.last_event_hash`]).
    pub last_event_hash: String,
    /// RFC 3339 wall-clock time of the most recent `workflow.phase.entered` event.
    pub entered_at: Option<String>,
}

impl Default for CycleState {
    fn default() -> Self {
        Self {
            phase: "unknown".into(),
            last_event_sequence: 0,
            last_event_hash: String::new(),
            entered_at: None,
        }
    }
}

/// Concrete projection for the `cycle_state` read-model.
///
/// Listens for `workflow.phase.entered` and `workflow.phase.exited` events
/// on the cycle's stream and updates [`CycleState::phase`] accordingly.
pub struct CycleStateProjection {
    /// Stream this projection is subscribed to.
    cycle_id: String,
    /// Mutable projection state.
    state: CycleState,
}

impl CycleStateProjection {
    /// Canonical name for the `cycle_state` projection.
    pub const NAME: &'static str = "cycle_state";
    /// Version for the v1 `apply` semantics.
    pub const VERSION: ProjectionVersion = 1;

    /// Creates a new `CycleStateProjection` for the given cycle stream.
    pub fn new(stream_id: impl Into<String>) -> Self {
        Self {
            cycle_id: stream_id.into(),
            state: CycleState::default(),
        }
    }

    /// Returns the cycle stream ID this projection consumes from.
    pub fn cycle_id(&self) -> &str {
        &self.cycle_id
    }
}

impl Projection for CycleStateProjection {
    type State = CycleState;

    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> ProjectionVersion {
        Self::VERSION
    }

    fn apply(&mut self, event: &super::EventEnvelopeV1) -> Result<(), ProjectionError> {
        // Only process events from our stream.
        if event.stream_id != self.cycle_id {
            return Ok(());
        }

        // Update monotone fields regardless of event type.
        self.state.last_event_sequence = event.sequence;
        self.state.last_event_hash = event.content_hash.clone();

        match event.event_type.as_str() {
            "workflow.phase.entered" => {
                let phase = event
                    .payload
                    .get("phase")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| ProjectionError::InvalidPayload {
                        event_type: event.event_type.clone(),
                        detail: format!(
                            "event {} missing 'phase' string in payload",
                            event.event_id
                        ),
                    })?
                    .to_string();
                self.state.phase = phase;
                self.state.entered_at = Some(event.occurred_at.clone());
                Ok(())
            }
            "workflow.phase.exited" => {
                self.state.phase = "exited".into();
                Ok(())
            }
            _ => Ok(()), // Ignore other event types per spec.
        }
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            projection_name: Self::NAME.to_string(),
            version: self.version(),
            last_event_sequence: self.state.last_event_sequence,
            last_event_hash: self.state.last_event_hash.clone(),
            updated_at: now_rfc3339(),
        }
    }

    fn state_ref(&self) -> &Self::State {
        &self.state
    }
}

/// Approval state for one `(cycle_id, capability)` pair.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ApprovalState {
    /// SHA-256 hash of the structured request.
    pub request_hash: String,
    /// RFC 3339 timestamp of the latest event.
    pub last_event_at: String,
    /// Event identifier of the latest event.
    pub last_event_id: String,
    /// Decision outcome, if resolved.
    pub decision: Option<crate::models::ApprovalDecision>,
    /// Human operator who made the decision, if resolved.
    pub actor: Option<String>,
    /// Justification, if resolved.
    pub reason: Option<String>,
}

/// Concrete projection for the `approval` read-model.
///
/// Listens for `approval.capability.requested`, `approval.capability.granted`,
/// and `approval.capability.denied` events and tracks the latest decision
/// per `(cycle_id, capability)` pair.
pub struct ApprovalProjection {
    /// Stream this projection is subscribed to (used as the cycle context).
    cycle_stream: String,
    /// Monotonic sequence of the last event applied (global, not per-capability).
    last_event_sequence: u64,
    /// Content hash of the last event applied.
    last_event_hash: String,
    /// Mutable projection state keyed by `(cycle_id, capability)`.
    state: std::collections::HashMap<(String, String), ApprovalState>,
}

impl ApprovalProjection {
    /// Canonical name for the `approval` projection.
    pub const NAME: &'static str = "approval";
    /// Version for the v1 `apply` semantics.
    pub const VERSION: ProjectionVersion = 1;

    /// Creates a new `ApprovalProjection` for the given cycle stream.
    pub fn new(cycle_stream: impl Into<String>) -> Self {
        Self {
            cycle_stream: cycle_stream.into(),
            last_event_sequence: 0,
            last_event_hash: String::new(),
            state: std::collections::HashMap::new(),
        }
    }

    /// Returns the current approval states as a map keyed by `(cycle_id, capability)`.
    pub fn states(&self) -> &std::collections::HashMap<(String, String), ApprovalState> {
        &self.state
    }
}

impl Projection for ApprovalProjection {
    type State = std::collections::HashMap<(String, String), ApprovalState>;

    fn name(&self) -> &str {
        Self::NAME
    }

    fn version(&self) -> ProjectionVersion {
        Self::VERSION
    }

    fn apply(&mut self, event: &super::EventEnvelopeV1) -> Result<(), ProjectionError> {
        // Only process events from our stream.
        if event.stream_id != self.cycle_stream {
            return Ok(());
        }

        // Update monotone fields on every call regardless of event type.
        self.last_event_sequence = event.sequence;
        self.last_event_hash = event.content_hash.clone();

        // Only process approval event types; ignore all others.
        match event.event_type.as_str() {
            "approval.capability.requested"
            | "approval.capability.granted"
            | "approval.capability.denied" => {}
            _ => return Ok(()),
        }

        let cycle_id = event
            .payload
            .get("cycle_id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProjectionError::InvalidPayload {
                event_type: event.event_type.clone(),
                detail: format!(
                    "event {} missing 'cycle_id' string in payload",
                    event.event_id
                ),
            })?
            .to_string();

        let capability = event
            .payload
            .get("capability")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProjectionError::InvalidPayload {
                event_type: event.event_type.clone(),
                detail: format!(
                    "event {} missing 'capability' string in payload",
                    event.event_id
                ),
            })?
            .to_string();

        let request_hash = event
            .payload
            .get("request_hash")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ProjectionError::InvalidPayload {
                event_type: event.event_type.clone(),
                detail: format!(
                    "event {} missing 'request_hash' string in payload",
                    event.event_id
                ),
            })?
            .to_string();

        let key = (cycle_id.clone(), capability.clone());
        let state = self.state.entry(key.clone()).or_default();

        match event.event_type.as_str() {
            "approval.capability.requested" => {
                state.request_hash = request_hash;
                state.last_event_at = event.occurred_at.clone();
                state.last_event_id = event.event_id.clone();
                state.decision = None;
                state.actor = None;
                state.reason = None;
                Ok(())
            }
            "approval.capability.granted" => {
                state.request_hash = request_hash;
                state.last_event_at = event.occurred_at.clone();
                state.last_event_id = event.event_id.clone();
                state.decision = Some(crate::models::ApprovalDecision::Granted);
                state.actor = event
                    .payload
                    .get("actor")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                state.reason = event
                    .payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Ok(())
            }
            "approval.capability.denied" => {
                state.request_hash = request_hash;
                state.last_event_at = event.occurred_at.clone();
                state.last_event_id = event.event_id.clone();
                state.decision = Some(crate::models::ApprovalDecision::Denied);
                state.actor = event
                    .payload
                    .get("actor")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                state.reason = event
                    .payload
                    .get("reason")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                Ok(())
            }
            _ => unreachable!(),
        }
    }

    fn checkpoint(&self) -> Checkpoint {
        Checkpoint {
            projection_name: Self::NAME.to_string(),
            version: self.version(),
            last_event_sequence: self.last_event_sequence,
            last_event_hash: self.last_event_hash.clone(),
            updated_at: now_rfc3339(),
        }
    }

    fn state_ref(&self) -> &Self::State {
        &self.state
    }
}

/// Returns the current wall-clock time as an RFC 3339 string with second precision.
fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = dur.as_secs();
    let days_since_epoch = total_secs / 86_400;
    let secs_in_day = total_secs % 86_400;
    let hours = secs_in_day / 3600;
    let minutes = (secs_in_day % 3600) / 60;
    let seconds = secs_in_day % 60;
    let (year, month, day) = civil_from_days(days_since_epoch as i64);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Converts a number of days since the Unix epoch (1970-01-01) to a calendar date.
/// Uses Howard Hinnant's civil_from_days algorithm.
fn civil_from_days(z: i64) -> (i32, u32, u32) {
    // Adapted from Howard Hinnant's public-domain C++ algorithm.
    let z = z + 719_468;
    let era = if z >= 0 {
        z / 146_097
    } else {
        (z - 146_096) / 146_097
    };
    let doe = (z - era * 146_097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d: u32 = doy - (153 * mp + 2) / 5 + 1;
    let m: u32 = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_envelope::{ActorKind, ActorRef, EventEnvelopeV1};
    use crate::models::ApprovalDecision;
    use serde_json::json;

    fn make_event(
        stream_id: &str,
        event_type: &str,
        sequence: u64,
        payload: serde_json::Value,
    ) -> EventEnvelopeV1 {
        let mut env = EventEnvelopeV1 {
            event_id: format!("e-{stream_id}-{sequence}"),
            event_type: event_type.into(),
            schema_version: 1,
            stream_id: stream_id.into(),
            sequence,
            project_id: "p-1".into(),
            occurred_at: "2026-08-17T10:00:00Z".into(),
            recorded_at: "2026-08-17T10:00:01Z".into(),
            actor: ActorRef {
                kind: ActorKind::System,
                id: "sddk-cli".into(),
                definition_hash: None,
                policy_hash: None,
                model: None,
            },
            subjects: vec![],
            payload,
            evidence_refs: vec![],
            content_hash: String::new(),
            metadata: None,
            causation_id: None,
            correlation_id: None,
            cycle_id: None,
            frame_id: None,
            fork_id: None,
        };
        env.content_hash = env.compute_content_hash();
        env
    }

    #[test]
    fn apply_workflow_phase_entered_sets_phase() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event(
            "cycle-1",
            "workflow.phase.entered",
            1,
            json!({ "phase": "build" }),
        ))
        .unwrap();
        assert_eq!(proj.state_ref().phase, "build");
        assert_eq!(proj.state_ref().last_event_sequence, 1);
        assert!(!proj.state_ref().last_event_hash.is_empty());
        assert!(proj.state_ref().last_event_hash.starts_with("sha256:"));
    }

    #[test]
    fn apply_workflow_phase_exited_marks_exited() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event(
            "cycle-1",
            "workflow.phase.entered",
            1,
            json!({ "phase": "build" }),
        ))
        .unwrap();
        proj.apply(&make_event(
            "cycle-1",
            "workflow.phase.exited",
            2,
            json!({}),
        ))
        .unwrap();
        assert_eq!(proj.state_ref().phase, "exited");
    }

    #[test]
    fn apply_other_event_types_ignored() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event("cycle-1", "uat.scenario.started", 1, json!({})))
            .unwrap();
        assert_eq!(proj.state_ref().phase, "unknown");
    }

    #[test]
    fn apply_skips_other_streams() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event(
            "cycle-2",
            "workflow.phase.entered",
            1,
            json!({ "phase": "build" }),
        ))
        .unwrap();
        assert_eq!(proj.state_ref().phase, "unknown");
    }

    #[test]
    fn checkpoint_includes_last_event_hash() {
        let mut proj = CycleStateProjection::new("cycle-1");
        proj.apply(&make_event(
            "cycle-1",
            "workflow.phase.entered",
            3,
            json!({ "phase": "test" }),
        ))
        .unwrap();
        let cp = proj.checkpoint();
        assert_eq!(cp.last_event_sequence, 3);
        assert!(cp.last_event_hash.starts_with("sha256:"));
    }

    // ApprovalProjection tests

    #[test]
    fn approval_projection_requested_then_granted_has_decision() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234",
                "expires_at": "2026-08-18T18:00:00Z"
            }),
        ))
        .unwrap();

        // State is pending after requested.
        let key = ("c-1".into(), "git.delete_branch".into());
        assert!(proj.state_ref().contains_key(&key));
        let state = proj.state_ref().get(&key).unwrap();
        assert!(state.decision.is_none());
        assert_eq!(state.request_hash, "sha256:abc1234");

        // Apply granted decision.
        proj.apply(&make_event(
            "c-1",
            "approval.capability.granted",
            2,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234",
                "actor": "alice",
                "reason": "ok, reversible via reflog"
            }),
        ))
        .unwrap();

        let state = proj.state_ref().get(&key).unwrap();
        assert_eq!(state.decision, Some(ApprovalDecision::Granted));
        assert_eq!(state.actor, Some("alice".into()));
        assert_eq!(state.reason, Some("ok, reversible via reflog".into()));
        assert_eq!(proj.checkpoint().last_event_sequence, 2);
    }

    #[test]
    fn approval_projection_denied_has_decision() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:def5678"
            }),
        ))
        .unwrap();
        proj.apply(&make_event(
            "c-1",
            "approval.capability.denied",
            2,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:def5678",
                "actor": "bob",
                "reason": "too risky"
            }),
        ))
        .unwrap();

        let key = ("c-1".into(), "git.delete_branch".into());
        let state = proj.state_ref().get(&key).unwrap();
        assert_eq!(state.decision, Some(ApprovalDecision::Denied));
        assert_eq!(state.actor, Some("bob".into()));
    }

    #[test]
    fn approval_projection_skips_other_streams() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-2",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-2",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234"
            }),
        ))
        .unwrap();
        assert!(proj.state_ref().is_empty());
        assert_eq!(proj.checkpoint().last_event_sequence, 0);
    }

    #[test]
    fn approval_projection_ignores_other_event_types() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "uat.scenario.started",
            1,
            json!({ "cycle_id": "c-1", "capability": "git.delete_branch", "request_hash": "sha256:abc" }),
        ))
        .unwrap();
        assert!(proj.state_ref().is_empty());
    }

    #[test]
    fn approval_projection_multiple_capabilities() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            1,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:aaa"
            }),
        ))
        .unwrap();
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            2,
            json!({
                "cycle_id": "c-1",
                "capability": "git.merge",
                "request_hash": "sha256:bbb"
            }),
        ))
        .unwrap();

        assert_eq!(proj.state_ref().len(), 2);
        assert!(
            proj.state_ref()
                .contains_key(&("c-1".into(), "git.delete_branch".into()))
        );
        assert!(
            proj.state_ref()
                .contains_key(&("c-1".into(), "git.merge".into()))
        );
    }

    #[test]
    fn approval_projection_checkpoint_sequence_tracks_global() {
        let mut proj = ApprovalProjection::new("c-1");
        proj.apply(&make_event(
            "c-1",
            "approval.capability.requested",
            5,
            json!({
                "cycle_id": "c-1",
                "capability": "git.delete_branch",
                "request_hash": "sha256:abc1234"
            }),
        ))
        .unwrap();
        assert_eq!(proj.checkpoint().last_event_sequence, 5);
        assert!(proj.checkpoint().last_event_hash.starts_with("sha256:"));
    }
}
