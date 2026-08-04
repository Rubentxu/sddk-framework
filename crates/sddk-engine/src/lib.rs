//! Deterministic workflow planning, application, and replay for SDDK.
//!
//! The engine owns workflow interpretation and delegates atomic persistence to
//! `sddk-storage`. Callers supply every identifier and timestamp.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

mod adoption;
mod paths;

pub use adoption::*;
pub use paths::*;

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use sddk_domain::{
    ArtifactRef, CycleManifest, CyclePath, CycleStatus, Phase, Requirement, StateRef, Transition,
    WORKFLOW_SCHEMA_VERSION, WorkflowManifest,
};
use sddk_storage::{CycleRecord, LedgerEvent, LedgerEventInput, Storage, StorageError};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use thiserror::Error;

/// Loads and validates a workflow manifest from a YAML string.
pub fn load_workflow_str(yaml: &str) -> Result<WorkflowManifest, WorkflowLoadError> {
    let manifest = serde_saphyr::from_str(yaml).map_err(WorkflowLoadError::Parse)?;
    validate_workflow(&manifest)?;
    Ok(manifest)
}

/// Loads and validates a workflow manifest from a YAML file.
pub fn load_workflow_path(path: impl AsRef<Path>) -> Result<WorkflowManifest, WorkflowLoadError> {
    let path = path.as_ref();
    let yaml = std::fs::read_to_string(path).map_err(|source| WorkflowLoadError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    load_workflow_str(&yaml)
}

/// Performs semantic validation that is required before workflow execution.
pub fn validate_workflow(manifest: &WorkflowManifest) -> Result<(), WorkflowValidationError> {
    if manifest.schema_version != WORKFLOW_SCHEMA_VERSION {
        return Err(WorkflowValidationError::UnsupportedSchemaVersion {
            actual: manifest.schema_version,
            supported: WORKFLOW_SCHEMA_VERSION,
        });
    }

    ensure_unique(manifest.statuses.iter().copied(), |status| {
        WorkflowValidationError::DuplicateStatus { status }
    })?;
    ensure_unique(manifest.phases.iter().copied(), |phase| {
        WorkflowValidationError::DuplicatePhase { phase }
    })?;

    let mut transition_ids = HashSet::new();
    let mut cycle_starts = Vec::new();
    for transition in &manifest.transitions {
        if transition.id.is_empty() {
            return Err(WorkflowValidationError::EmptyTransitionId);
        }
        if !transition_ids.insert(transition.id.as_str()) {
            return Err(WorkflowValidationError::DuplicateTransitionId {
                transition_id: transition.id.clone(),
            });
        }
        let is_cycle_start =
            transition.id == "cycle.start" || transition.id.starts_with("cycle.start.");
        if is_cycle_start {
            if transition.from.is_some() {
                return Err(WorkflowValidationError::CycleStartHasSource);
            }
            cycle_starts.push(transition);
        } else if transition.from.is_none() {
            return Err(WorkflowValidationError::CreationSourceOnTransition {
                transition_id: transition.id.clone(),
            });
        }
        for path in &transition.paths {
            if !manifest.paths.contains_key(path) {
                return Err(WorkflowValidationError::UnknownTransitionPath {
                    transition_id: transition.id.clone(),
                    path: path.clone(),
                });
            }
        }

        if let Some(from) = &transition.from {
            validate_state_ref(manifest, &transition.id, "from", from)?;
        }
        validate_state_ref(manifest, &transition.id, "to", &transition.to)?;
        if let Some(on_failure) = &transition.on_failure {
            validate_state_ref(manifest, &transition.id, "on_failure", on_failure)?;
        }
        validate_requirements(manifest, transition)?;
    }

    if !cycle_starts
        .iter()
        .any(|transition| transition.id == "cycle.start")
    {
        return Err(WorkflowValidationError::MissingCycleStart);
    }
    if cycle_starts
        .iter()
        .any(|transition| transition.to.phase.is_none())
    {
        return Err(WorkflowValidationError::CycleStartMissingPhase);
    }

    for (path_name, path) in &manifest.paths {
        match path.debt_verification.as_str() {
            "mandatory" | "disabled" => {}
            policy => {
                return Err(WorkflowValidationError::InvalidDebtVerificationPolicy {
                    path: path_name.clone(),
                    policy: policy.to_owned(),
                });
            }
        }
        for phase in &path.phases {
            let parsed =
                parse_phase(phase).ok_or_else(|| WorkflowValidationError::UnknownPathPhase {
                    path: path_name.clone(),
                    phase: phase.clone(),
                })?;
            if !manifest.phases.contains(&parsed) {
                return Err(WorkflowValidationError::UnknownPathPhase {
                    path: path_name.clone(),
                    phase: phase.clone(),
                });
            }
        }
        match cycle_starts
            .iter()
            .filter(|transition| transition_applies_to_path(transition, path_name))
            .count()
        {
            0 => {
                return Err(WorkflowValidationError::MissingPathCycleStart {
                    path: path_name.clone(),
                });
            }
            1 => {}
            _ => {
                return Err(WorkflowValidationError::AmbiguousPathCycleStart {
                    path: path_name.clone(),
                });
            }
        }
    }
    Ok(())
}

fn ensure_unique<T, E>(
    values: impl IntoIterator<Item = T>,
    error: E,
) -> Result<(), WorkflowValidationError>
where
    T: Copy + Eq + std::hash::Hash,
    E: Fn(T) -> WorkflowValidationError,
{
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(error(value));
        }
    }
    Ok(())
}

fn validate_state_ref(
    manifest: &WorkflowManifest,
    transition_id: &str,
    field: &'static str,
    state: &StateRef,
) -> Result<(), WorkflowValidationError> {
    if !manifest.statuses.contains(&state.status) {
        return Err(WorkflowValidationError::UnknownTransitionStatus {
            transition_id: transition_id.to_owned(),
            field,
            status: state.status,
        });
    }
    if let Some(phase) = state.phase
        && !manifest.phases.contains(&phase)
    {
        return Err(WorkflowValidationError::UnknownTransitionPhase {
            transition_id: transition_id.to_owned(),
            field,
            phase,
        });
    }
    Ok(())
}

fn validate_requirements(
    manifest: &WorkflowManifest,
    transition: &Transition,
) -> Result<(), WorkflowValidationError> {
    for requirement in &transition.requires {
        let Requirement::Structured { kind, name } = requirement else {
            continue;
        };
        match kind.as_str() {
            "artifact" if !manifest.artifacts.contains_key(name) => {
                return Err(WorkflowValidationError::UnknownArtifactRequirement {
                    transition_id: transition.id.clone(),
                    artifact: name.clone(),
                });
            }
            "gate" if !manifest.gates.contains_key(name) => {
                return Err(WorkflowValidationError::UnknownGateRequirement {
                    transition_id: transition.id.clone(),
                    gate: name.clone(),
                });
            }
            "artifact" | "gate" => {}
            _ => {
                return Err(WorkflowValidationError::UnknownRequirementKind {
                    transition_id: transition.id.clone(),
                    kind: kind.clone(),
                });
            }
        }
    }
    Ok(())
}

fn parse_phase(value: &str) -> Option<Phase> {
    serde_json::from_value(Value::String(value.to_owned())).ok()
}

/// Errors produced while loading a workflow manifest.
#[derive(Debug, Error)]
pub enum WorkflowLoadError {
    /// The manifest file could not be read.
    #[error("failed to read workflow manifest {path:?}: {source}")]
    Io {
        /// Requested manifest path.
        path: PathBuf,
        /// Underlying filesystem failure.
        source: std::io::Error,
    },
    /// YAML could not be deserialized into the workflow domain model.
    #[error("invalid workflow YAML: {0}")]
    Parse(serde_saphyr::Error),
    /// The parsed manifest violates an executable workflow invariant.
    #[error(transparent)]
    Validation(#[from] WorkflowValidationError),
}

/// Semantic workflow validation errors.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum WorkflowValidationError {
    /// The manifest uses a schema version this runtime does not support.
    #[error("unsupported workflow schema version {actual}; supported version is {supported}")]
    UnsupportedSchemaVersion {
        /// Version found in the manifest.
        actual: i32,
        /// Version supported by this runtime.
        supported: i32,
    },
    /// A status occurs more than once in the declaration.
    #[error("duplicate workflow status: {status:?}")]
    DuplicateStatus {
        /// Repeated status.
        status: CycleStatus,
    },
    /// A phase occurs more than once in the declaration.
    #[error("duplicate workflow phase: {phase:?}")]
    DuplicatePhase {
        /// Repeated phase.
        phase: Phase,
    },
    /// A transition identifier is empty.
    #[error("workflow transition id cannot be empty")]
    EmptyTransitionId,
    /// A transition identifier occurs more than once.
    #[error("duplicate workflow transition id: {transition_id}")]
    DuplicateTransitionId {
        /// Repeated transition identifier.
        transition_id: String,
    },
    /// The required creation transition is absent.
    #[error("workflow does not declare cycle.start")]
    MissingCycleStart,
    /// The creation transition incorrectly declares a source state.
    #[error("cycle.start must declare from: null")]
    CycleStartHasSource,
    /// A non-creation transition incorrectly declares `from: null`.
    #[error("transition {transition_id} declares from: null; only cycle.start may create cycles")]
    CreationSourceOnTransition {
        /// Invalid transition identifier.
        transition_id: String,
    },
    /// The creation target omits the initial phase.
    #[error("cycle.start must declare a target phase")]
    CycleStartMissingPhase,
    /// A transition references a status omitted from the manifest declaration.
    #[error("transition {transition_id} {field} references undeclared status {status:?}")]
    UnknownTransitionStatus {
        /// Transition containing the reference.
        transition_id: String,
        /// State field containing the reference.
        field: &'static str,
        /// Undeclared status.
        status: CycleStatus,
    },
    /// A transition references a phase omitted from the manifest declaration.
    #[error("transition {transition_id} {field} references undeclared phase {phase:?}")]
    UnknownTransitionPhase {
        /// Transition containing the reference.
        transition_id: String,
        /// State field containing the reference.
        field: &'static str,
        /// Undeclared phase.
        phase: Phase,
    },
    /// A structured requirement uses an unsupported kind.
    #[error("transition {transition_id} uses unknown requirement kind {kind:?}")]
    UnknownRequirementKind {
        /// Transition containing the requirement.
        transition_id: String,
        /// Unsupported requirement kind.
        kind: String,
    },
    /// A transition requires an undeclared artifact.
    #[error("transition {transition_id} requires undeclared artifact {artifact:?}")]
    UnknownArtifactRequirement {
        /// Transition containing the requirement.
        transition_id: String,
        /// Missing artifact declaration.
        artifact: String,
    },
    /// A transition requires an undeclared gate.
    #[error("transition {transition_id} requires undeclared gate {gate:?}")]
    UnknownGateRequirement {
        /// Transition containing the requirement.
        transition_id: String,
        /// Missing gate declaration.
        gate: String,
    },
    /// A path names a phase absent from the workflow.
    #[error("path {path} references unknown phase {phase:?}")]
    UnknownPathPhase {
        /// Path containing the phase.
        path: String,
        /// Unknown phase name.
        phase: String,
    },
    /// A path uses an unsupported debt-verification policy.
    #[error("path {path} uses invalid debt verification policy {policy:?}")]
    InvalidDebtVerificationPolicy {
        /// Path containing the policy.
        path: String,
        /// Unsupported policy value.
        policy: String,
    },
    /// A transition is restricted to a path absent from the manifest.
    #[error("transition {transition_id} references unknown workflow path {path:?}")]
    UnknownTransitionPath {
        /// Transition containing the restriction.
        transition_id: String,
        /// Unknown path name.
        path: String,
    },
    /// A workflow path has no applicable cycle creation transition.
    #[error("workflow path {path} has no applicable cycle.start transition")]
    MissingPathCycleStart {
        /// Path without a creation transition.
        path: String,
    },
    /// A workflow path has more than one applicable cycle creation transition.
    #[error("workflow path {path} has multiple applicable cycle.start transitions")]
    AmbiguousPathCycleStart {
        /// Path with ambiguous creation transitions.
        path: String,
    },
}

/// Caller-supplied causal context for one state-changing command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EventContext {
    /// Stable command invocation identifier.
    pub command_id: String,
    /// Frame grouping events produced by the command.
    pub frame_id: String,
    /// Stable event identifier.
    pub event_id: String,
    /// Actor responsible for the command.
    pub actor: String,
    /// Explicit event timestamp.
    pub occurred_at: String,
}

/// Caller evidence used to evaluate a transition.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionEvidence {
    /// Named non-artifact preconditions, including cycle-start requirements.
    #[serde(default)]
    pub requirements: BTreeSet<String>,
    /// Artifact results offered to the transition.
    #[serde(default)]
    pub artifacts: BTreeMap<String, ArtifactRef>,
    /// Explicit outcomes for required gates.
    #[serde(default)]
    pub gates: BTreeMap<String, GateOutcome>,
}

/// Explicit gate evaluation supplied by the caller.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum GateOutcome {
    /// The gate passed.
    Passed,
    /// The gate failed with optional structured context.
    Failed {
        /// Stable human-readable failure reason.
        #[serde(default)]
        reason: Option<String>,
    },
}

/// Logical outcome selected while planning a transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransitionOutcome {
    /// All required gates passed and the normal target applies.
    Succeeded,
    /// At least one gate failed and the declared failure target applies.
    Failed,
}

/// Deterministic plan for a declared non-creation transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionPlan {
    transition_id: String,
    outcome: TransitionOutcome,
    failed_gates: Vec<String>,
    evidence: TransitionEvidence,
    state_before: CycleManifest,
    state_after: CycleManifest,
}

impl TransitionPlan {
    /// Declared transition identifier.
    pub fn transition_id(&self) -> &str {
        &self.transition_id
    }

    /// Planned success or failure path.
    pub fn outcome(&self) -> TransitionOutcome {
        self.outcome
    }

    /// Gates that selected the failure target, in declaration order.
    pub fn failed_gates(&self) -> &[String] {
        &self.failed_gates
    }

    /// Snapshot before the transition.
    pub fn state_before(&self) -> &CycleManifest {
        &self.state_before
    }

    /// Snapshot after the transition.
    pub fn state_after(&self) -> &CycleManifest {
        &self.state_after
    }
}

/// Explicit input used to plan cycle creation through `cycle.start`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CycleStartInput {
    /// Caller-constructed cycle manifest containing identity and repository data.
    pub manifest: CycleManifest,
    /// Explicitly satisfied initial workflow requirements.
    pub requirements: BTreeSet<String>,
}

/// Deterministic plan for creating a cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CycleStartPlan {
    input: CycleStartInput,
    state_after: CycleManifest,
}

impl CycleStartPlan {
    /// Initial canonical cycle snapshot.
    pub fn state_after(&self) -> &CycleManifest {
        &self.state_after
    }
}

/// Minimal immutable receipt for an applied ledger event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EventReceipt {
    /// Stable caller-supplied event identifier.
    pub event_id: String,
    /// Monotonic ledger sequence assigned by storage.
    pub sequence: i64,
    /// Hash of the persisted event and its predecessor link.
    pub event_hash: String,
}

impl From<&LedgerEvent> for EventReceipt {
    fn from(event: &LedgerEvent) -> Self {
        Self {
            event_id: event.event_id.clone(),
            sequence: event.sequence,
            event_hash: event.event_hash.clone(),
        }
    }
}

/// Result of atomically applying a transition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct TransitionResult {
    /// Applied transition identifier.
    pub transition_id: String,
    /// Applied logical outcome.
    pub outcome: TransitionOutcome,
    /// Persisted cycle snapshot.
    pub manifest: CycleManifest,
    /// Causal ledger receipt.
    pub event: EventReceipt,
}

/// Result of atomically creating a cycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CycleStartResult {
    /// Persisted initial cycle snapshot.
    pub manifest: CycleManifest,
    /// Causal ledger receipt.
    pub event: EventReceipt,
}

/// Debt verification behavior declared for a workflow path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DebtVerificationPolicy {
    /// Debt verification is required for this path.
    Mandatory,
    /// Debt verification is disabled for this path.
    Disabled,
}

/// Verified relationship between the replayed and stored cycle states.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ReplayVerification {
    /// Reconstructed logical cycle snapshot.
    pub manifest: CycleManifest,
    /// Sequence of the state event used for reconstruction.
    pub sequence: i64,
}

/// Errors emitted by deterministic engine operations.
#[derive(Debug, Error)]
pub enum EngineError {
    /// The requested transition identifier is not declared.
    #[error("undeclared transition: {transition_id}")]
    UndeclaredTransition {
        /// Unknown transition identifier.
        transition_id: String,
    },
    /// A creation transition was passed to the normal transition API.
    #[error("transition {transition_id} creates a cycle and must use the cycle-start API")]
    CreationTransitionRequiresStartApi {
        /// Creation transition identifier.
        transition_id: String,
    },
    /// The current cycle snapshot does not match the transition source.
    #[error(
        "transition {transition_id} expects {expected_status:?}/{expected_phase:?}, found {actual_status:?}/{actual_phase:?}"
    )]
    SourceStateMismatch {
        /// Requested transition identifier.
        transition_id: String,
        /// Expected source status.
        expected_status: CycleStatus,
        /// Expected source phase, or any phase when absent.
        expected_phase: Option<Phase>,
        /// Actual cycle status.
        actual_status: CycleStatus,
        /// Actual cycle phase.
        actual_phase: Phase,
    },
    /// A required non-artifact precondition was not supplied.
    #[error("transition {transition_id} is missing requirement {requirement:?}")]
    MissingRequirement {
        /// Requested transition identifier.
        transition_id: String,
        /// Missing precondition.
        requirement: String,
    },
    /// A required artifact is absent from both the snapshot and new evidence.
    #[error("transition {transition_id} is missing artifact {artifact:?}")]
    MissingArtifact {
        /// Requested transition identifier.
        transition_id: String,
        /// Missing artifact kind.
        artifact: String,
    },
    /// A required gate outcome was not supplied.
    #[error("transition {transition_id} is missing gate {gate:?}")]
    MissingGate {
        /// Requested transition identifier.
        transition_id: String,
        /// Missing gate name.
        gate: String,
    },
    /// A failed gate has no declared failure target.
    #[error("transition {transition_id} gate {gate:?} failed without an on_failure target")]
    GateFailedWithoutTarget {
        /// Requested transition identifier.
        transition_id: String,
        /// Failed gate name.
        gate: String,
    },
    /// Evidence contains an artifact not declared as an output of this transition.
    #[error("transition {transition_id} does not produce artifact {artifact:?}")]
    UndeclaredProducedArtifact {
        /// Requested transition identifier.
        transition_id: String,
        /// Unexpected artifact name.
        artifact: String,
    },
    /// An artifact map key disagrees with its canonical kind.
    #[error("artifact key {key:?} does not match artifact kind {kind:?}")]
    ArtifactKindMismatch {
        /// Artifact evidence key.
        key: String,
        /// Kind declared by the artifact reference.
        kind: String,
    },
    /// A cycle uses a path absent from the workflow manifest.
    #[error("unknown workflow path: {path}")]
    UnknownPath {
        /// Unknown path name.
        path: String,
    },
    /// The requested transition is not allowed for the cycle's workflow path.
    #[error("transition {transition_id} is not allowed for workflow path {path}")]
    TransitionPathMismatch {
        /// Requested transition identifier.
        transition_id: String,
        /// Current cycle path.
        path: String,
    },
    /// A plan was built from an older cycle snapshot.
    #[error("transition plan is stale for cycle {cycle_id}")]
    StalePlan {
        /// Cycle whose snapshot changed.
        cycle_id: String,
    },
    /// A supplied plan differs from the engine's deterministic recomputation.
    #[error("transition plan failed deterministic revalidation")]
    InvalidPlan,
    /// A workflow state could not be represented as JSON for the ledger.
    #[error("failed to serialize workflow state: {0}")]
    StateSerialization(#[from] serde_json::Error),
    /// The cycle has no workflow state events to replay.
    #[error("cycle {cycle_id} has no replayable state events")]
    MissingReplayState {
        /// Cycle missing state history.
        cycle_id: String,
    },
    /// A workflow event is missing its post-state.
    #[error("cycle state event at sequence {sequence} has no state_after")]
    MissingStateAfter {
        /// Invalid event sequence.
        sequence: i64,
    },
    /// A workflow event stores a non-object post-state.
    #[error("cycle state event at sequence {sequence} has non-object state_after")]
    NonObjectStateAfter {
        /// Invalid event sequence.
        sequence: i64,
    },
    /// A workflow event stores an object that is not a valid cycle manifest.
    #[error("cycle state event at sequence {sequence} has corrupt state_after: {source}")]
    CorruptStateAfter {
        /// Invalid event sequence.
        sequence: i64,
        /// Deserialization failure.
        source: serde_json::Error,
    },
    /// Replayed state and the materialized cycle snapshot differ.
    #[error("replayed cycle state does not match stored snapshot for {cycle_id}")]
    SnapshotMismatch {
        /// Cycle with divergent state.
        cycle_id: String,
    },
    /// Persistence rejected the operation.
    #[error(transparent)]
    Storage(#[from] StorageError),
}

/// Deterministic workflow runtime backed by SQLite storage.
pub struct Engine {
    workflow: WorkflowManifest,
    storage: Storage,
}

impl Engine {
    /// Constructs an engine after validating the supplied workflow manifest.
    pub fn new(
        workflow: WorkflowManifest,
        storage: Storage,
    ) -> Result<Self, WorkflowValidationError> {
        validate_workflow(&workflow)?;
        Ok(Self { workflow, storage })
    }

    /// Returns the validated workflow manifest.
    pub fn workflow(&self) -> &WorkflowManifest {
        &self.workflow
    }

    /// Returns read-only access to the backing storage.
    pub fn storage(&self) -> &Storage {
        &self.storage
    }

    /// Returns the declared debt-verification behavior for a named path.
    pub fn debt_verification_policy(
        &self,
        path: &str,
    ) -> Result<DebtVerificationPolicy, EngineError> {
        let definition = self
            .workflow
            .paths
            .get(path)
            .ok_or_else(|| EngineError::UnknownPath {
                path: path.to_owned(),
            })?;
        match definition.debt_verification.as_str() {
            "mandatory" => Ok(DebtVerificationPolicy::Mandatory),
            "disabled" => Ok(DebtVerificationPolicy::Disabled),
            _ => unreachable!("workflow validation rejects unknown debt policies"),
        }
    }

    /// Plans creation of a cycle through the declared `cycle.start` transition.
    pub fn plan_cycle_start(&self, input: CycleStartInput) -> Result<CycleStartPlan, EngineError> {
        let path = cycle_path_name(&input.manifest.path);
        self.debt_verification_policy(path)?;
        let transition = self
            .cycle_start_transition(path)
            .expect("workflow validation requires one cycle.start transition per path");
        for requirement in &transition.requires {
            match requirement {
                Requirement::Simple(name) if !input.requirements.contains(name) => {
                    return Err(EngineError::MissingRequirement {
                        transition_id: transition.id.clone(),
                        requirement: name.clone(),
                    });
                }
                Requirement::Structured { .. } => {
                    return Err(EngineError::InvalidPlan);
                }
                Requirement::Simple(_) => {}
            }
        }

        let mut state_after = input.manifest.clone();
        state_after.status = transition.to.status;
        state_after.phase = transition
            .to
            .phase
            .expect("validated cycle.start target has a phase");
        Ok(CycleStartPlan { input, state_after })
    }

    /// Atomically persists a planned cycle snapshot and its creation event.
    pub fn apply_cycle_start(
        &mut self,
        plan: &CycleStartPlan,
        context: &EventContext,
    ) -> Result<CycleStartResult, EngineError> {
        if self.plan_cycle_start(plan.input.clone())? != *plan {
            return Err(EngineError::InvalidPlan);
        }
        let transition_id = self
            .cycle_start_transition(cycle_path_name(&plan.input.manifest.path))
            .expect("workflow validation requires one cycle.start transition per path")
            .id
            .clone();
        let manifest = plan.state_after.clone();
        let state_after = serde_json::to_value(&manifest)?;
        let event_input = event_input(
            &manifest,
            context,
            "cycle.created",
            None,
            Some(state_after),
            json!({
                "transition_id": transition_id,
                "outcome": TransitionOutcome::Succeeded,
            }),
        );
        let cycle = CycleRecord {
            manifest: manifest.clone(),
            created_at: context.occurred_at.clone(),
            updated_at: context.occurred_at.clone(),
        };
        let event = self.storage.insert_cycle_with_event(&cycle, &event_input)?;
        Ok(CycleStartResult {
            manifest,
            event: EventReceipt::from(&event),
        })
    }

    /// Plans one declared non-creation transition by identifier.
    pub fn plan_transition(
        &self,
        cycle_id: &str,
        transition_id: &str,
        evidence: TransitionEvidence,
    ) -> Result<TransitionPlan, EngineError> {
        let current = self.storage.get_cycle(cycle_id)?.manifest;
        self.plan_transition_from_state(current, transition_id, evidence)
    }

    /// Atomically applies a plan after revalidating it against current state.
    pub fn apply_transition(
        &mut self,
        plan: &TransitionPlan,
        context: &EventContext,
    ) -> Result<TransitionResult, EngineError> {
        let current = self
            .storage
            .get_cycle(&plan.state_before.cycle_id)?
            .manifest;
        if current != plan.state_before {
            return Err(EngineError::StalePlan {
                cycle_id: plan.state_before.cycle_id.clone(),
            });
        }
        let recomputed =
            self.plan_transition_from_state(current, &plan.transition_id, plan.evidence.clone())?;
        if recomputed != *plan {
            return Err(EngineError::InvalidPlan);
        }

        let state_before = serde_json::to_value(&plan.state_before)?;
        let state_after = serde_json::to_value(&plan.state_after)?;
        let event_input = event_input(
            &plan.state_after,
            context,
            "cycle.transitioned",
            Some(state_before),
            Some(state_after),
            json!({
                "transition_id": plan.transition_id,
                "outcome": plan.outcome,
                "failed_gates": plan.failed_gates,
            }),
        );
        let event = self.storage.update_cycle_with_event(
            &plan.state_after,
            &context.occurred_at,
            &event_input,
        )?;
        Ok(TransitionResult {
            transition_id: plan.transition_id.clone(),
            outcome: plan.outcome,
            manifest: plan.state_after.clone(),
            event: EventReceipt::from(&event),
        })
    }

    /// Reconstructs the latest logical cycle snapshot from state events.
    pub fn replay_cycle(&self, cycle_id: &str) -> Result<ReplayVerification, EngineError> {
        let events = self.storage.list_cycle_events(cycle_id)?;
        let mut latest = None;
        for event in events.iter().filter(|event| is_cycle_state_event(event)) {
            let state = event
                .state_after
                .as_ref()
                .ok_or(EngineError::MissingStateAfter {
                    sequence: event.sequence,
                })?;
            if !state.is_object() {
                return Err(EngineError::NonObjectStateAfter {
                    sequence: event.sequence,
                });
            }
            let manifest = serde_json::from_value(state.clone()).map_err(|source| {
                EngineError::CorruptStateAfter {
                    sequence: event.sequence,
                    source,
                }
            })?;
            latest = Some(ReplayVerification {
                manifest,
                sequence: event.sequence,
            });
        }
        latest.ok_or_else(|| EngineError::MissingReplayState {
            cycle_id: cycle_id.to_owned(),
        })
    }

    /// Replays a cycle and verifies it equals the materialized SQLite snapshot.
    pub fn verify_cycle_snapshot(&self, cycle_id: &str) -> Result<ReplayVerification, EngineError> {
        let replayed = self.replay_cycle(cycle_id)?;
        let stored = self.storage.get_cycle(cycle_id)?.manifest;
        if replayed.manifest != stored {
            return Err(EngineError::SnapshotMismatch {
                cycle_id: cycle_id.to_owned(),
            });
        }
        Ok(replayed)
    }

    fn transition(&self, transition_id: &str) -> Result<&Transition, EngineError> {
        self.workflow
            .transitions
            .iter()
            .find(|transition| transition.id == transition_id)
            .ok_or_else(|| EngineError::UndeclaredTransition {
                transition_id: transition_id.to_owned(),
            })
    }

    fn cycle_start_transition(&self, path: &str) -> Option<&Transition> {
        self.workflow.transitions.iter().find(|transition| {
            transition.from.is_none() && transition_applies_to_path(transition, path)
        })
    }

    fn plan_transition_from_state(
        &self,
        state_before: CycleManifest,
        transition_id: &str,
        evidence: TransitionEvidence,
    ) -> Result<TransitionPlan, EngineError> {
        let transition = self.transition(transition_id)?;
        let path = cycle_path_name(&state_before.path);
        if !transition_applies_to_path(transition, path) {
            return Err(EngineError::TransitionPathMismatch {
                transition_id: transition.id.clone(),
                path: path.to_owned(),
            });
        }
        let source = transition.from.as_ref().ok_or_else(|| {
            EngineError::CreationTransitionRequiresStartApi {
                transition_id: transition.id.clone(),
            }
        })?;
        if source.status != state_before.status
            || source
                .phase
                .is_some_and(|phase| phase != state_before.phase)
        {
            return Err(EngineError::SourceStateMismatch {
                transition_id: transition.id.clone(),
                expected_status: source.status,
                expected_phase: source.phase,
                actual_status: state_before.status,
                actual_phase: state_before.phase,
            });
        }

        for (name, artifact) in &evidence.artifacts {
            if !transition.produces.contains(name) {
                return Err(EngineError::UndeclaredProducedArtifact {
                    transition_id: transition.id.clone(),
                    artifact: name.clone(),
                });
            }
            if artifact.kind != *name {
                return Err(EngineError::ArtifactKindMismatch {
                    key: name.clone(),
                    kind: artifact.kind.clone(),
                });
            }
        }

        for requirement in &transition.requires {
            match requirement {
                Requirement::Simple(name) if !evidence.requirements.contains(name) => {
                    return Err(EngineError::MissingRequirement {
                        transition_id: transition.id.clone(),
                        requirement: name.clone(),
                    });
                }
                Requirement::Structured { kind, name }
                    if kind == "artifact"
                        && !state_before.artifacts.contains_key(name)
                        && !evidence.artifacts.contains_key(name) =>
                {
                    return Err(EngineError::MissingArtifact {
                        transition_id: transition.id.clone(),
                        artifact: name.clone(),
                    });
                }
                Requirement::Structured { kind, name }
                    if kind == "gate" && !evidence.gates.contains_key(name) =>
                {
                    return Err(EngineError::MissingGate {
                        transition_id: transition.id.clone(),
                        gate: name.clone(),
                    });
                }
                _ => {}
            }
        }

        let failed_gates = transition
            .requires
            .iter()
            .filter_map(|requirement| match requirement {
                Requirement::Structured { kind, name }
                    if kind == "gate"
                        && matches!(evidence.gates.get(name), Some(GateOutcome::Failed { .. })) =>
                {
                    Some(name.clone())
                }
                _ => None,
            })
            .collect::<Vec<_>>();
        let (outcome, target) = if failed_gates.is_empty() {
            (TransitionOutcome::Succeeded, &transition.to)
        } else {
            let target = transition.on_failure.as_ref().ok_or_else(|| {
                EngineError::GateFailedWithoutTarget {
                    transition_id: transition.id.clone(),
                    gate: failed_gates[0].clone(),
                }
            })?;
            (TransitionOutcome::Failed, target)
        };

        let mut state_after = state_before.clone();
        state_after.status = target.status;
        state_after.phase = target.phase.unwrap_or(state_before.phase);
        for (name, artifact) in &evidence.artifacts {
            state_after.artifacts.insert(name.clone(), artifact.clone());
        }
        Ok(TransitionPlan {
            transition_id: transition.id.clone(),
            outcome,
            failed_gates,
            evidence,
            state_before,
            state_after,
        })
    }
}

fn event_input(
    manifest: &CycleManifest,
    context: &EventContext,
    event_type: &str,
    state_before: Option<Value>,
    state_after: Option<Value>,
    payload: Value,
) -> LedgerEventInput {
    LedgerEventInput {
        event_id: context.event_id.clone(),
        project_id: manifest.project_id.clone(),
        cycle_id: Some(manifest.cycle_id.clone()),
        frame_id: context.frame_id.clone(),
        command_id: context.command_id.clone(),
        actor: context.actor.clone(),
        event_type: event_type.to_owned(),
        occurred_at: context.occurred_at.clone(),
        state_before,
        state_after,
        payload,
    }
}

fn is_cycle_state_event(event: &LedgerEvent) -> bool {
    matches!(
        event.event_type.as_str(),
        "cycle.created" | "cycle.transitioned"
    )
}

fn cycle_path_name(path: &CyclePath) -> &'static str {
    match path {
        CyclePath::AMin => "A-min",
        CyclePath::ALite => "A-lite",
        CyclePath::AFull => "A-full",
        CyclePath::BDirect => "B-direct",
    }
}

fn transition_applies_to_path(transition: &Transition, path: &str) -> bool {
    transition.paths.is_empty() || transition.paths.iter().any(|candidate| candidate == path)
}
