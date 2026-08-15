//! Persistence records used by the SQLite storage boundary.

use sddk_domain::CycleManifest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A logical SDDK project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectRecord {
    /// Stable logical project identifier.
    pub project_id: String,
    /// Human-readable project name.
    pub display_name: String,
    /// Canonical remote URL, when the project has one.
    pub remote_url: Option<String>,
    /// Identity scope used with the remote URL.
    pub scope: String,
    /// Caller-supplied creation timestamp.
    pub created_at: String,
}

/// A checkout or worktree belonging to a project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceRecord {
    /// Stable workspace identifier.
    pub workspace_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Canonical checkout path.
    pub canonical_path: String,
    /// Caller-supplied creation timestamp.
    pub created_at: String,
}

/// A persisted cycle manifest and its storage timestamps.
#[derive(Debug, Clone, PartialEq)]
pub struct CycleRecord {
    /// Canonical cycle manifest.
    pub manifest: CycleManifest,
    /// Caller-supplied creation timestamp.
    pub created_at: String,
    /// Caller-supplied last-update timestamp.
    pub updated_at: String,
}

/// Data required to append one ledger event.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerEventInput {
    /// Stable event identifier supplied by the runtime.
    pub event_id: String,
    /// Project to which the event belongs.
    pub project_id: String,
    /// Related cycle, if the event is cycle-scoped.
    pub cycle_id: Option<String>,
    /// Frame shared by all events from one command.
    pub frame_id: String,
    /// Command invocation identifier.
    pub command_id: String,
    /// Actor responsible for the event.
    pub actor: String,
    /// Stable event type.
    pub event_type: String,
    /// Caller-supplied event timestamp.
    pub occurred_at: String,
    /// State before the event, when applicable.
    pub state_before: Option<Value>,
    /// State after the event, when applicable.
    pub state_after: Option<Value>,
    /// Event-specific structured data.
    pub payload: Value,
}

/// An immutable hash-linked ledger event.
#[derive(Debug, Clone, PartialEq)]
pub struct LedgerEvent {
    /// Monotonically increasing database sequence.
    pub sequence: i64,
    /// Stable event identifier.
    pub event_id: String,
    /// Project to which the event belongs.
    pub project_id: String,
    /// Related cycle, if any.
    pub cycle_id: Option<String>,
    /// Command frame identifier.
    pub frame_id: String,
    /// Command invocation identifier.
    pub command_id: String,
    /// Actor responsible for the event.
    pub actor: String,
    /// Stable event type.
    pub event_type: String,
    /// Caller-supplied event timestamp.
    pub occurred_at: String,
    /// State before the event.
    pub state_before: Option<Value>,
    /// State after the event.
    pub state_after: Option<Value>,
    /// Event-specific structured data.
    pub payload: Value,
    /// Hash of the preceding event, or `None` for the first event.
    pub previous_hash: Option<String>,
    /// SHA-256 hash of this event and its predecessor link.
    pub event_hash: String,
}

/// Result of verifying the complete ledger chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerVerification {
    /// Number of verified events.
    pub event_count: usize,
    /// Hash at the head of the verified chain.
    pub last_hash: Option<String>,
}

/// Metadata for an artifact stored outside SQLite.
#[derive(Debug, Clone, PartialEq)]
pub struct ArtifactRecord {
    /// Stable artifact identifier.
    pub artifact_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Producing cycle, if applicable.
    pub cycle_id: Option<String>,
    /// Artifact kind from the workflow contract.
    pub kind: String,
    /// Artifact store path or logical reference.
    pub path: String,
    /// Content hash, normally prefixed with `sha256:`.
    pub sha256: Option<String>,
    /// Producer identifier, when known.
    pub producer: Option<String>,
    /// Caller-supplied creation timestamp.
    pub created_at: String,
    /// Additional structured metadata.
    pub metadata: Value,
}

/// Lifecycle state of a capability execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    /// The external effect has been registered but not reconciled.
    Started,
    /// The effect completed and its postcondition was verified.
    Succeeded,
    /// The effect failed with a known result.
    Failed,
    /// The effect outcome is unknown and requires reconciliation.
    Unknown,
}

/// A capability receipt before its deterministic request hash is assigned.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityReceiptInput {
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, if any.
    pub cycle_id: Option<String>,
    /// Typed capability identifier.
    pub capability: String,
    /// Caller-provided idempotency key.
    pub idempotency_key: String,
    /// Canonical structured capability request.
    pub request: Value,
    /// Current execution state.
    pub status: CapabilityStatus,
    /// Structured sanitized result, when available.
    pub result: Option<Value>,
    /// Caller-supplied start timestamp.
    pub started_at: String,
    /// Caller-supplied completion timestamp, when complete.
    pub completed_at: Option<String>,
}

/// A persisted capability execution receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityReceipt {
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, if any.
    pub cycle_id: Option<String>,
    /// Typed capability identifier.
    pub capability: String,
    /// SHA-256 hash of the structured request.
    pub request_hash: String,
    /// Canonical structured capability request.
    pub request: Value,
    /// Current execution state.
    pub status: CapabilityStatus,
    /// Structured sanitized result, when available.
    pub result: Option<Value>,
    /// Caller-supplied start timestamp.
    pub started_at: String,
    /// Caller-supplied completion timestamp, when complete.
    pub completed_at: Option<String>,
}

/// Outcome of an idempotent capability receipt write.
#[derive(Debug, Clone, PartialEq)]
pub enum IdempotencyOutcome {
    /// A new receipt and idempotency record were inserted.
    Inserted(CapabilityReceipt),
    /// The existing receipt was returned without a duplicate write.
    Replayed(CapabilityReceipt),
}

/// An exclusive cycle lease with a monotonic fencing token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CycleLease {
    /// Leased cycle identifier.
    pub cycle_id: String,
    /// Runtime owner identifier.
    pub owner: String,
    /// Caller-supplied acquisition time in Unix milliseconds.
    pub acquired_at_ms: i64,
    /// Caller-supplied expiry time in Unix milliseconds.
    pub expires_at_ms: i64,
    /// Monotonic token invalidating previous lease holders.
    pub fencing_token: i64,
}

/// Outcome recorded by an authorized gate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateOutcomeStatus {
    /// The gate passed.
    Passed,
    /// The gate failed.
    Failed,
}

/// Data required to persist one authorized gate receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceiptInput {
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, when applicable.
    pub cycle_id: Option<String>,
    /// Evaluated gate name.
    pub gate: String,
    /// Registered evaluator identifier that issued the receipt.
    pub evaluator: String,
    /// Transition the gate belongs to.
    pub transition_id: String,
    /// Deterministic plan hash the receipt attests.
    pub plan_hash: String,
    /// Evaluation outcome.
    pub outcome: GateOutcomeStatus,
    /// Sanitized evaluation evidence.
    pub evidence: Value,
    /// Actor responsible for the evaluation.
    pub actor: String,
    /// Command invocation identifier.
    pub command_id: String,
    /// Frame shared by the command's events.
    pub frame_id: String,
    /// Caller-supplied evaluation timestamp.
    pub evaluated_at: String,
    /// Sequence number within the (gate, plan_hash) group.
    pub seq: i64,
}

/// An authorized, persisted gate evaluation receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceipt {
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, when applicable.
    pub cycle_id: Option<String>,
    /// Evaluated gate name.
    pub gate: String,
    /// Registered evaluator identifier that issued the receipt.
    pub evaluator: String,
    /// Transition the gate belongs to.
    pub transition_id: String,
    /// Deterministic plan hash the receipt attests.
    pub plan_hash: String,
    /// Evaluation outcome.
    pub outcome: GateOutcomeStatus,
    /// Sanitized evaluation evidence.
    pub evidence: Value,
    /// Actor responsible for the evaluation.
    pub actor: String,
    /// Command invocation identifier.
    pub command_id: String,
    /// Frame shared by the command's events.
    pub frame_id: String,
    /// Caller-supplied evaluation timestamp.
    pub evaluated_at: String,
    /// Sequence number within the (gate, plan_hash) group.
    pub seq: i64,
}
