//! Persistence records used by the SQLite storage boundary.

// `missing_docs` is allowed across this file because several public items
// were introduced by earlier cycles (Ledger port, UatResultRow move,
// ControlPlane split) before the workspace-wide `#![warn(missing_docs)]`
// activation. A future docs-pass cycle should restore the per-field
// `///` doc comments and remove this crate-level allow.
#![allow(missing_docs)]

use crate::cycle::CycleManifest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

// Storage error exposed via the Ledger trait. The concrete SQLite
// implementation in `sddk_storage` wraps this via a From impl.
//
// `missing_documentation` is allowed on pre-existing variants
// (`NotFound { entity, id }`, `LeaseConflict { cycle_id, owner }`) that
// pre-date the workspace-wide `missing_docs` lint activation. The
// variants that already carry `///` docs keep them; a future docs-pass
// cycle should restore per-field doc comments and remove these allows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StorageError {
    /// A requested record does not exist.
    NotFound { entity: &'static str, id: String },
    /// A database-level error (constraint violation, I/O failure, etc.).
    Database(String),
    /// A lease conflict: the resource is already locked by another owner.
    LeaseConflict { cycle_id: String, owner: String },
    /// A storage operation failed; see the inner error for details.
    Other(String),
}

impl std::fmt::Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound { entity, id } => write!(f, "{entity} not found: {id}"),
            Self::Database(msg) => write!(f, "database error: {msg}"),
            Self::LeaseConflict { cycle_id, owner } => {
                write!(f, "lease conflict on {cycle_id} held by {owner}")
            }
            Self::Other(msg) => write!(f, "storage error: {msg}"),
        }
    }
}

impl std::error::Error for StorageError {}

impl crate::SddkErrorCode for StorageError {
    fn code(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "STORAGE_NOT_FOUND",
            Self::Database(_) => "STORAGE_DATABASE_ERROR",
            Self::LeaseConflict { .. } => "STORAGE_LEASE_CONFLICT",
            Self::Other(_) => "STORAGE_ERROR",
        }
    }
    fn recovery(&self) -> &'static str {
        match self {
            Self::NotFound { .. } => "ensure the record exists before operating on it",
            Self::Database(_) => "check the database is accessible and not corrupted",
            Self::LeaseConflict { .. } => "release the existing lease before acquiring a new one",
            Self::Other(_) => "retry the operation; if the problem persists, check the logs",
        }
    }
}

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
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Hash of the agent binary that executed this capability (optional for backward compat).
    pub agent_version_hash: Option<String>,
    /// Hash of the behavior/workflow that authorized this capability (optional for backward compat).
    pub behavior_version_hash: Option<String>,
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
    /// Hash of the agent binary that executed this capability (optional for backward compat).
    pub agent_version_hash: Option<String>,
    /// Hash of the behavior/workflow that authorized this capability (optional for backward compat).
    pub behavior_version_hash: Option<String>,
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
    /// The gate was explicitly waived by an authorized evaluator (it does not
    /// apply in this context). Satisfies cycle-phase transitions (the engine
    /// treats any non-Failed receipt as satisfied) but does NOT satisfy
    /// release-authority gates, which require [`GateOutcomeStatus::Passed`].
    Waived,
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

/// Data required to persist one authorized gate receipt, with atomic seq allocation.
///
/// The `seq` number is computed inside the method and the `receipt_id` is
/// built from it — both are **absent** from the input. This prevents split
/// read-modify-write races between `allocate_gate_receipt_seq` and `insert_gate_receipt`.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceiptNextSeqInput {
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

impl LedgerEvent {
    /// Converts this immutable ledger event into the input form used when
    /// appending events to the ledger.
    pub fn as_input(&self) -> LedgerEventInput {
        LedgerEventInput {
            event_id: self.event_id.clone(),
            project_id: self.project_id.clone(),
            cycle_id: self.cycle_id.clone(),
            frame_id: self.frame_id.clone(),
            command_id: self.command_id.clone(),
            actor: self.actor.clone(),
            event_type: self.event_type.clone(),
            occurred_at: self.occurred_at.clone(),
            state_before: self.state_before.clone(),
            state_after: self.state_after.clone(),
            payload: self.payload.clone(),
        }
    }
}

/// A row of the control-plane `uat_results` table (SDDK2-103).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UatResultRow {
    /// Owning project identifier.
    pub project_id: String,
    /// Semantic version tag.
    pub tag_version: String,
    /// Readiness verdict: READY | READY_WITH_RISKS | NOT_READY.
    pub verdict: String,
    /// Test coverage percentage.
    pub coverage_pct: f64,
    /// Number of defects detected.
    pub defects: i64,
    /// Number of UAT sessions executed.
    pub session_count: i64,
    /// Total UAT duration in minutes.
    pub uat_duration_minutes: i64,
    /// RFC 3339 timestamp when the row was written.
    pub recorded_at: String,
}

/// Outcome of a human approval decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Human granted the approval.
    Granted,
    /// Human denied the approval.
    Denied,
}

/// Input data for recording an approval decision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ApprovalReceiptInput {
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle identifier.
    pub cycle_id: String,
    /// Capability identifier.
    pub capability: String,
    /// SHA-256 hash of the structured request.
    pub request_hash: String,
    /// Decision made by the human.
    pub decision: ApprovalDecision,
    /// Human operator identifier.
    pub actor: String,
    /// Mandatory justification for the decision.
    pub reason: String,
    /// RFC 3339 timestamp when approval was requested.
    pub requested_at: String,
    /// RFC 3339 timestamp when decision was made.
    pub decided_at: String,
}

// Debt domain types ─────────────────────────────────────────────────────────────────

/// Severity level for a debt finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

/// Priority level for a debt finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Priority {
    P0,
    P1,
    P2,
    P3,
}

/// Lifecycle status of a debt finding.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FindingStatus {
    Open,
    InProgress,
    Deferred,
    Resolved,
    Superseded,
}

/// Status of an INC record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IncStatus {
    Open,
    AcceptedRisk,
    Resolved,
}

/// A single debt finding within a [`DebtReport`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub priority: Priority,
    pub status: FindingStatus,
    pub fingerprint: String,
    #[serde(default)]
    pub fingerprint_aliases: Vec<String>,
    pub cluster_id: String,
    pub category: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation_cycle: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remediation_pr: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence_refs: Option<Vec<serde_json::Value>>,
}

/// Per-cycle debt report emitted by sddk-debt-verify.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebtReport {
    pub schema_version: String,
    pub cycle_id: String,
    pub generated_at: String,
    pub findings: Vec<Finding>,
}

/// Durable cross-cycle incidence record (INC).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IncRecord {
    pub inc_id: String,
    pub finding_id: String,
    pub cycle_id: String,
    pub status: IncStatus,
    pub severity: Severity,
    pub priority: Priority,
    pub fingerprint: String,
    pub fingerprint_aliases: Vec<String>,
    pub cluster_id: String,
    pub created_at: String,
    pub created_by: String,
    pub owner: String,
    pub inc_path: String,
    #[serde(default)]
    pub lifecycle_events: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

/// A persisted human approval receipt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ApprovalReceipt {
    /// Stable receipt identifier.
    pub receipt_id: String,
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle identifier.
    pub cycle_id: String,
    /// Capability identifier.
    pub capability: String,
    /// SHA-256 hash of the structured request.
    pub request_hash: String,
    /// Decision made by the human.
    pub decision: ApprovalDecision,
    /// Human operator identifier.
    pub actor: String,
    /// Mandatory justification for the decision.
    pub reason: String,
    /// RFC 3339 timestamp when approval was requested.
    pub requested_at: String,
    /// RFC 3339 timestamp when decision was made.
    pub decided_at: String,
    /// Event identifier of the `approval.capability.requested` event.
    pub requested_event_id: String,
    /// Event identifier of the `approval.capability.granted` or `denied` event.
    pub decision_event_id: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_approval_decision_granted_roundtrip() {
        let decision = ApprovalDecision::Granted;
        let json = serde_json::to_string(&decision).unwrap();
        assert_eq!(json, "\"granted\"");
        let roundtrip: ApprovalDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, ApprovalDecision::Granted);
    }

    #[test]
    fn test_approval_decision_denied_roundtrip() {
        let decision = ApprovalDecision::Denied;
        let json = serde_json::to_string(&decision).unwrap();
        assert_eq!(json, "\"denied\"");
        let roundtrip: ApprovalDecision = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, ApprovalDecision::Denied);
    }

    #[test]
    fn test_approval_receipt_input_roundtrip() {
        let input = ApprovalReceiptInput {
            receipt_id: "ar-1".into(),
            project_id: "p-1".into(),
            cycle_id: "c-1".into(),
            capability: "git.delete_branch".into(),
            request_hash: "sha256:abcd1234".into(),
            decision: ApprovalDecision::Granted,
            actor: "alice".into(),
            reason: "ok, reversible via reflog".into(),
            requested_at: "2026-08-18T10:00:00Z".into(),
            decided_at: "2026-08-18T10:05:00Z".into(),
        };
        let json = serde_json::to_string(&input).unwrap();
        let roundtrip: ApprovalReceiptInput = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, input);
    }

    #[test]
    fn test_approval_receipt_roundtrip() {
        let receipt = ApprovalReceipt {
            receipt_id: "ar-1".into(),
            project_id: "p-1".into(),
            cycle_id: "c-1".into(),
            capability: "git.delete_branch".into(),
            request_hash: "sha256:abcd1234".into(),
            decision: ApprovalDecision::Granted,
            actor: "alice".into(),
            reason: "ok, reversible via reflog".into(),
            requested_at: "2026-08-18T10:00:00Z".into(),
            decided_at: "2026-08-18T10:05:00Z".into(),
            requested_event_id: "approval-cap-git-delete_branch-abcd1234-requested".into(),
            decision_event_id: "approval-cap-git-delete_branch-abcd1234-granted".into(),
        };
        let json = serde_json::to_string(&receipt).unwrap();
        let roundtrip: ApprovalReceipt = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, receipt);
    }

    #[test]
    fn test_debt_report_roundtrip_with_optional_fields() {
        let report = DebtReport {
            schema_version: "1.1.0".into(),
            cycle_id: "p-test/kernel-cycle-8".into(),
            generated_at: "2026-08-21T00:00:00Z".into(),
            findings: vec![Finding {
                id: "FIND-0001".into(),
                title: "Test finding".into(),
                severity: Severity::Medium,
                priority: Priority::P2,
                status: FindingStatus::Open,
                fingerprint: "3ef321c4efe1d87e".into(),
                fingerprint_aliases: vec!["alias1".into()],
                cluster_id: "CL-01".into(),
                category: "architecture".into(),
                description: "Test".into(),
                remediation_cycle: Some("p-next".into()),
                remediation_pr: Some("https://github.com/org/repo/pull/123".into()),
                evidence_refs: Some(vec![serde_json::json!({"kind": "commit", "ref": "abc123"})]),
            }],
        };
        let json = serde_json::to_string(&report).unwrap();
        let roundtrip: DebtReport = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, report);
        // v1.0 compat: optional fields absent
        let report_v1: DebtReport = serde_json::from_str(r#"{"schema_version":"1.0.0","cycle_id":"p-test/kernel-cycle-7b","generated_at":"2026-08-21T00:00:00Z","findings":[]}"#).unwrap();
        assert_eq!(report_v1.schema_version, "1.0.0");
        assert!(report_v1.findings.is_empty());
    }

    #[test]
    fn test_inc_record_roundtrip() {
        let inc = IncRecord {
            inc_id: "INC-001-3ef321c4".into(),
            finding_id: "FIND-0001".into(),
            cycle_id: "p-test/kernel-cycle-8".into(),
            status: IncStatus::Open,
            severity: Severity::Medium,
            priority: Priority::P2,
            fingerprint: "3ef321c4efe1d87e".into(),
            fingerprint_aliases: vec![],
            cluster_id: "CL-01".into(),
            created_at: "2026-08-21T00:00:00Z".into(),
            created_by: "sddk".into(),
            owner: "team".into(),
            inc_path: "~/.sddk-knowledge/sddk-framework/incs/INC-001-3ef321c4.md".into(),
            lifecycle_events: vec!["created:2026-08-21T00:00:00Z".into()],
            evidence_refs: vec![],
        };
        let json = serde_json::to_string(&inc).unwrap();
        let roundtrip: IncRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(roundtrip, inc);
    }
}
