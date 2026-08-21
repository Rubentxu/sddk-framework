//! Gate evaluation receipts and status.
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Outcome recorded by an authorized gate evaluation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GateOutcomeStatus {
    Passed,
    Failed,
    Waived,
}

/// Data required to persist one authorized gate receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceiptInput {
    pub receipt_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub gate: String,
    pub evaluator: String,
    pub transition_id: String,
    pub plan_hash: String,
    pub outcome: GateOutcomeStatus,
    pub evidence: Value,
    pub actor: String,
    pub command_id: String,
    pub frame_id: String,
    pub evaluated_at: String,
    pub seq: i64,
}

/// Data required to persist one authorized gate receipt, with atomic seq allocation.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceiptNextSeqInput {
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub gate: String,
    pub evaluator: String,
    pub transition_id: String,
    pub plan_hash: String,
    pub outcome: GateOutcomeStatus,
    pub evidence: Value,
    pub actor: String,
    pub command_id: String,
    pub frame_id: String,
    pub evaluated_at: String,
}

/// An authorized, persisted gate evaluation receipt.
#[derive(Debug, Clone, PartialEq)]
pub struct GateReceipt {
    pub receipt_id: String,
    pub project_id: String,
    pub cycle_id: Option<String>,
    pub gate: String,
    pub evaluator: String,
    pub transition_id: String,
    pub plan_hash: String,
    pub outcome: GateOutcomeStatus,
    pub evidence: Value,
    pub actor: String,
    pub command_id: String,
    pub frame_id: String,
    pub evaluated_at: String,
    pub seq: i64,
}
