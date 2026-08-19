//! Workflow IR types — compile-time validated, content-addressed executable plans.
//!
//! All collection fields use `BTreeMap`/`BTreeSet` for deterministic serialization
//! and hash stability. HashMap is explicitly forbidden in this module.

use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Maximum inline context capsule size in bytes (4096).
pub const INLINE_CAPSULE_MAX_BYTES: usize = 4096;

// ── Newtypes ─────────────────────────────────────────────────────────────────

/// Content hash in `sha256:<64-hex-lowercase>` format.
pub type ContentHash = String;

/// IR identifier (ULID, assigned post-hoc).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IrId(pub String);

/// Revision identifier (ULID).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevisionId(pub String);

/// Run identifier (UUID v7).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunId(pub String);

/// Node identifier (stable within an IR).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct NodeId(pub String);

/// Operator identifier (stable within an IR).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct OperatorId(pub String);

/// Edge identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct EdgeId(pub String);

/// Event identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct EventId(pub String);

/// Capability identifier (e.g. `git.commit`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct CapabilityId(pub String);

/// Schema version constant for IR types.
pub const SCHEMA_VERSION: u32 = 1;

// ── ExpansionPermission ───────────────────────────────────────────────────────

/// Expansion permission set — which runtime expansions a node is allowed to perform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpansionPermission {
    /// Node may expand a Map operator.
    Map,
    /// Node may expand a Discover operator.
    Discover,
    /// Node may expand a Replan operator.
    Replan,
}

impl ExpansionPermission {
    /// Checks if this permission is allowed by the given allowlist.
    pub fn is_allowed(&self, _allowlist: &BTreeSet<ExpansionPermission>) -> bool {
        // v1 closed set: only Map, Discover, Replan exist
        matches!(
            self,
            ExpansionPermission::Map | ExpansionPermission::Discover | ExpansionPermission::Replan
        )
    }
}

// ── Budgets ─────────────────────────────────────────────────────────────────

/// Execution budgets for a workflow run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Budgets {
    /// Maximum wall-clock time in milliseconds.
    pub max_wall_ms: u64,
    /// Maximum input tokens.
    pub max_tokens: u64,
    /// Maximum output tokens.
    pub max_cost_micros: u64,
    /// Maximum call depth.
    pub max_depth: u64,
    /// Maximum nodes in the execution graph.
    pub max_nodes: u64,
    /// Remaining tokens (decremented at runtime).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub remaining_tokens: Option<u64>,
}

impl Default for Budgets {
    fn default() -> Self {
        Self {
            max_wall_ms: u64::MAX,
            max_tokens: u64::MAX,
            max_cost_micros: u64::MAX,
            max_depth: u64::MAX,
            max_nodes: u64::MAX,
            remaining_tokens: None,
        }
    }
}

impl Budgets {
    /// Validates that this budget fits within the template-level budget.
    pub fn fits_within(&self, _template_budget: &Budgets) -> Result<(), CompileError> {
        // For v1.29.0: budget comparison is stub-declared; full semantics in cycle 2
        Ok(())
    }
}

// ── Invariant ────────────────────────────────────────────────────────────────

/// Invariant that the workflow IR must satisfy.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Invariant {
    /// DAG must have no cycles (default).
    #[default]
    ConvergenceBounded,
    /// All operators must have arity > 0.
    ArityPositive,
}

// ── Policy ──────────────────────────────────────────────────────────────────

/// Pack-specific policy profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// Policy name.
    pub name: String,
    /// Policy JSON blob.
    pub config: BTreeMap<String, serde_json::Value>,
}

// ── ConvergenceSpec ─────────────────────────────────────────────────────────

/// Convergence criteria for loop/expansion termination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConvergenceSpec {
    /// Maximum iterations before forced convergence.
    pub max_iterations: u32,
    /// Signature that indicates no progress (stable output).
    pub no_progress_signature: Option<String>,
}

// ── Provenance ─────────────────────────────────────────────────────────────

/// Provenance metadata for a compiled IR.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    /// Tool that generated this IR.
    pub generated_by: String,
    /// Hash of the prompt that produced this IR.
    pub prompt_hash: String,
    /// Hash of the model that produced this IR.
    pub model_hash: String,
    /// Hash of the policy applied.
    pub policy_hash: String,
}

// ── GuardExpr ──────────────────────────────────────────────────────────────

/// Runtime guard expression evaluated before operator execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardExpr {
    /// Expression text.
    pub expr: String,
}

// ── Operator enum ──────────────────────────────────────────────────────────

/// One step in a workflow DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operator {
    /// A leaf task that calls a capability.
    Task {
        /// Capability required for this task.
        capability: CapabilityId,
        /// Inputs to the capability (deterministic map).
        inputs: BTreeMap<String, serde_json::Value>,
    },
    /// Execute operators in sequence.
    Sequence {
        /// Ordered list of operator IDs.
        body: Vec<OperatorId>,
    },
    /// Execute branches in parallel, limited by max_concurrency.
    Parallel {
        /// Branch operator IDs.
        branches: Vec<OperatorId>,
        /// Maximum concurrent branches.
        max_concurrency: u32,
    },
    /// Map over a collection (stub in v1.29.0 — full semantics in cycle 3).
    Map {
        /// Source operator ID.
        source: OperatorId,
        /// Body operator ID.
        body: OperatorId,
        /// Maximum concurrent mappings.
        max_concurrency: u32,
    },
    /// Wait for all branches then continue (stub in v1.29.0).
    Join {
        /// Join policy name.
        policy: String,
        /// Branch operator IDs.
        branches: Vec<OperatorId>,
    },
    /// Race: first branch to complete wins (stub in v1.29.0).
    Race {
        /// Branch operator IDs.
        branches: Vec<OperatorId>,
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },
    /// Conditional branch (stub in v1.29.0).
    Choice {
        /// Branch map: condition string -> operator ID.
        branches: BTreeMap<String, OperatorId>,
    },
    /// Iterative loop (stub in v1.29.0).
    Loop {
        /// Maximum iterations.
        max_iterations: u32,
        /// Guard expression.
        until: GuardExpr,
        /// Body operator ID.
        body: OperatorId,
    },
    /// Conditional execution gate.
    Gate {
        /// Guard expression.
        condition: GuardExpr,
        /// Body operator ID.
        body: OperatorId,
    },
    /// Wait for an external event.
    Wait {
        /// Event type to wait for.
        event_type: String,
        /// Timeout in milliseconds.
        timeout_ms: u64,
    },
    /// Invoke a sub-workflow.
    SubWorkflow {
        /// Reference to the sub-workflow run.
        run_ref: String,
    },
    /// Compensation for a failed operator (stub in v1.29.0).
    Compensate {
        /// Operator ID to compensate.
        of: OperatorId,
    },
}

impl Operator {
    /// Returns all operator IDs referenced by this operator (for cycle detection).
    pub fn referenced_ids(&self) -> Vec<OperatorId> {
        match self {
            Operator::Task { .. } => vec![],
            Operator::Sequence { body } => body.clone(),
            Operator::Parallel { branches, .. } => branches.clone(),
            Operator::Map { source, body, .. } => vec![source.clone(), body.clone()],
            Operator::Join { branches, .. } => branches.clone(),
            Operator::Race { branches, .. } => branches.clone(),
            Operator::Choice { branches } => branches.values().cloned().collect(),
            Operator::Loop { body, .. } => vec![body.clone()],
            Operator::Gate { body, .. } => vec![body.clone()],
            Operator::Wait { .. } => vec![],
            Operator::SubWorkflow { .. } => vec![],
            Operator::Compensate { of } => vec![of.clone()],
        }
    }
}

// ── TemplateRef ────────────────────────────────────────────────────────────

/// Reference to a workflow template.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TemplateRef {
    /// Template identifier (reverse-DNS).
    pub id: String,
    /// Template version.
    pub version: String,
}

// ── WorkflowTemplate ───────────────────────────────────────────────────────

/// Human-authored intent declaration — source of truth for compilation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorkflowTemplate {
    /// Template identifier (reverse-DNS, e.g. `sddk.adaptive.discovery`).
    pub template_id: String,
    /// Human-readable name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Free-text intent description.
    pub intent: String,
    /// Authored capability allowlist (no wildcards).
    pub capability_allowlist: BTreeSet<CapabilityId>,
    /// Expansion permissions granted by this template.
    pub expansion_permissions: BTreeSet<ExpansionPermission>,
    /// Invariants this template guarantees.
    pub invariants: BTreeSet<Invariant>,
    /// Execution budgets.
    pub budgets: Budgets,
    /// Pack-specific policies.
    pub policies: BTreeMap<String, Policy>,
    /// Convergence criteria.
    pub convergence: ConvergenceSpec,
    /// Schema version (must be 1 for v1.29.0 readers).
    pub schema_version: u32,
}

impl WorkflowTemplate {
    /// Validates this template for compilation.
    pub fn validate(&self) -> Result<(), CompileError> {
        // Empty allowlist rejected
        if self.capability_allowlist.is_empty() {
            return Err(CompileError::EmptyCapabilityAllowlist);
        }
        // Schema version must be 1
        if self.schema_version != SCHEMA_VERSION {
            return Err(CompileError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        // Check that all expansion permissions are in the closed set
        for perm in &self.expansion_permissions {
            if !perm.is_allowed(&self.expansion_permissions) {
                return Err(CompileError::ExpansionNotAllowed);
            }
        }
        // Budget must fit within hard limits
        self.budgets.fits_within(&Budgets::default())?;
        Ok(())
    }
}

// ── WorkflowIR ──────────────────────────────────────────────────────────────

/// Validated, content-addressed executable plan produced by the compiler.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "snake_case")]
pub struct WorkflowIR {
    /// IR identifier (assigned post-compilation).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ir_id: Option<IrId>,
    /// Schema version (must be 1).
    pub schema_version: u32,
    /// Template this IR was compiled from.
    pub template_ref: TemplateRef,
    /// Operators keyed by ID (order-independent BTreeMap).
    pub operators: BTreeMap<OperatorId, Operator>,
    /// Guards keyed by operator ID.
    pub guards: BTreeMap<OperatorId, GuardExpr>,
    /// Effective expansion permissions (may be subset of template).
    pub expansion_permissions: BTreeSet<ExpansionPermission>,
    /// Effective budgets (may be tighter than template).
    pub budgets: Budgets,
    /// Required invariants (must be subset of template invariants).
    pub required_invariants: BTreeSet<Invariant>,
    /// Provenance metadata.
    pub provenance: Provenance,
}

impl WorkflowIR {
    /// Computes the content hash of this IR (mirrors EventEnvelopeV1::compute_content_hash).
    ///
    /// Excludes `ir_id` (assigned post-hoc) and `schema_version` (metadata).
    /// Stable across BTreeMap key ordering because serde_json uses BTreeMap by default.
    pub fn compute_content_hash(&self) -> ContentHash {
        // Canonical form: zero out fields excluded from hash
        let mut canonical = self.clone();
        canonical.ir_id = None;
        canonical.schema_version = 0;

        let bytes = serde_json::to_vec(&canonical).expect("WorkflowIR is always serializable");
        let digest = Sha256::digest(&bytes);
        let hex = format!("{:064x}", digest);
        format!("sha256:{}", hex)
    }

    /// Validates this IR after compilation.
    pub fn validate(&self) -> Result<(), ValidateError> {
        // Schema version check
        if self.schema_version != SCHEMA_VERSION {
            return Err(ValidateError::UnsupportedSchemaVersion {
                got: self.schema_version,
                want: SCHEMA_VERSION,
            });
        }
        // Required invariants must be subset of expansion permissions
        // (stub for v1.29.0)
        Ok(())
    }
}

// ── Errors ─────────────────────────────────────────────────────────────────

/// Compile-time errors for WorkflowTemplate validation.
#[derive(Debug, Error)]
pub enum CompileError {
    /// Capability allowlist is empty.
    #[error("capability allowlist is empty")]
    EmptyCapabilityAllowlist,

    /// Expansion permission not in the closed set.
    #[error("expansion permission not in closed set")]
    ExpansionNotAllowed,

    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// The version that was found.
        got: u32,
        /// The version that was expected.
        want: u32,
    },

    /// Budget exceeds template limit.
    #[error("budget exceeds template limit")]
    BudgetExceedsLimit,

    /// YAML serialization error.
    #[error("YAML serialization error: {0}")]
    YamlSerde(String),

    /// Operator not in allowlist.
    #[error("operator not in allowlist: {0:?}")]
    OperatorNotAllowed(CapabilityId),

    /// Capability not in allowlist.
    #[error("capability not in allowlist: {0:?}")]
    CapabilityNotInAllowlist(CapabilityId),

    /// Cycle detected in operator graph.
    #[error("cycle detected in operator graph")]
    CycleDetected,

    /// Hash collision detected.
    #[error("hash collision detected")]
    HashCollision,

    /// Invariant subsumed by template.
    #[error("invariant subsumed by template")]
    InvariantSubsumed,
}

/// Runtime validation errors for WorkflowIR.
#[derive(Debug, Error)]
pub enum ValidateError {
    /// Unsupported schema version.
    #[error("unsupported schema version: got {got}, want {want}")]
    UnsupportedSchemaVersion {
        /// The version that was found.
        got: u32,
        /// The version that was expected.
        want: u32,
    },

    /// Operator not found.
    #[error("operator not found: {0:?}")]
    OperatorNotFound(OperatorId),

    /// Cycle detected in operator graph.
    #[error("cycle detected in operator graph")]
    CycleDetected,

    /// Guard expression failed.
    #[error("guard expression failed: {0}")]
    GuardFailed(String),
}
