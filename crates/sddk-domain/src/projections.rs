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
    #[error(
        "event chain integrity broken for stream '{stream_id}' at sequence {sequence}"
    )]
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

/// Returns the current wall-clock time as an RFC 3339 string with second precision.
fn now_rfc3339() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let dur = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
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
    use serde_json::json;
    use crate::event_envelope::{ActorKind, ActorRef, EventEnvelopeV1};

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
        proj.apply(&make_event(
            "cycle-1",
            "uat.scenario.started",
            1,
            json!({}),
        ))
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
}
