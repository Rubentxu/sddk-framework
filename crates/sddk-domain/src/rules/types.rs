//! Domain types for the architecture-rule registry.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleSeverity {
    Error,
    Warning,
    WarningThenRatchet,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleTarget {
    DependencyGraph,
    SourceImportsAndCalls,
    PackManifest,
    CapabilityImports,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuleStatus {
    Pass,
    Fail,
    Waived,
    NotApplicable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluatorKind {
    Heuristic,
    Ast,
    Schema,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureRule {
    pub id: String,
    pub severity: RuleSeverity,
    pub rule: String,
    pub target: RuleTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub desired_state: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waiver {
    pub id: String,
    pub rule_id: String,
    pub reason: String,
    pub granted_until_sha: String,
    pub granted_by: String,
    pub granted_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BaselineRef {
    pub schema_version: String,
    pub head_anchor: String,
    pub sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cycle_id: Option<String>,
    pub captured_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RuleEvaluation {
    pub rule_id: String,
    pub status: RuleStatus,
    pub observed: serde_json::Value,
    pub baseline_sha256: String,
    pub evaluated_at: String,
    pub evaluated_by: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub waiver_id: Option<String>,
    pub evaluator_kind: EvaluatorKind,
    pub evaluator_version: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provenance: Option<String>,
}
