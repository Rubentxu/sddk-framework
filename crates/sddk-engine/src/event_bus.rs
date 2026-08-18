//! Bridges cycle phase transitions to the `events_v1` ledger substrate.

use std::path::PathBuf;

use sddk_domain::{
    ActorKind, ActorRef, EntityRef, EventAppended, EventEnvelopeV1, EventStore, StorageError,
};
use serde_json::json;

use crate::TransitionOutcome;

/// Returns the canonical XDG storage dir for a project:
/// `$XDG_STATE_HOME/sddk/projects/<id>/`.
pub fn project_storage_dir(project_id: &str) -> Result<PathBuf, StorageError> {
    let base = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local").join("state")))
        .ok_or_else(|| StorageError::Other("cannot resolve XDG state dir".into()))?;
    Ok(base.join("sddk").join("projects").join(project_id))
}

/// Internal logic for `project_storage_dir`, parameterised so tests can
/// supply fake env values without unsafe env manipulation.
#[cfg(test)]
fn project_storage_dir_with(
    xdg_state_home: Option<&str>,
    home: Option<&str>,
    project_id: &str,
) -> Result<PathBuf, StorageError> {
    let base = xdg_state_home
        .map(PathBuf::from)
        .or_else(|| home.map(|h| PathBuf::from(h).join(".local").join("state")))
        .ok_or_else(|| StorageError::Other("cannot resolve XDG state dir".into()))?;
    Ok(base.join("sddk").join("projects").join(project_id))
}

/// Input for phase-transition event emission.
#[derive(Debug, Clone)]
pub struct PhaseEventInput {
    /// Project that owns the cycle.
    pub project_id: String,
    /// Cycle being transitioned.
    pub cycle_id: String,
    /// Phase being exited.
    pub from_phase: String,
    /// Phase being entered.
    pub to_phase: String,
    /// Wall-clock time of the transition (RFC 3339).
    pub transition_at: String,
    /// Actor identifier.
    pub actor_id: String,
    /// Actor kind.
    pub actor_kind: ActorKind,
    /// Prefix for deterministic event_id generation.
    pub event_id_prefix: String,
}

/// Input for transition-outcome event emission.
#[derive(Debug, Clone)]
pub struct OutcomeEventInput {
    /// Project that owns the cycle.
    pub project_id: String,
    /// Cycle being transitioned.
    pub cycle_id: String,
    /// Transition identifier.
    pub transition_id: String,
    /// Phase being exited (None if transition failed before planning).
    pub from_phase: Option<String>,
    /// Phase being entered (None if transition failed before reaching target).
    pub to_phase: Option<String>,
    /// Wall-clock time of the transition (RFC 3339).
    pub transition_at: String,
    /// Actor identifier.
    pub actor_id: String,
    /// Actor kind.
    pub actor_kind: ActorKind,
    /// Prefix for deterministic event_id generation.
    pub event_id_prefix: String,
    /// Names of gates that failed (empty for succeeded transitions).
    pub failed_gates: Vec<String>,
}

/// Appends two events to events_v1:
///
///   - `workflow.phase.exited` (for `from_phase`)
///   - `workflow.phase.entered` (for `to_phase`)
///
/// Both share `stream_id = cycle_id`. Idempotency comes from the unique
/// `event_id` built deterministically from `(event_id_prefix, cycle_id, phase_label)`.
///
/// Returns the stored `from_phase` and `to_phase` `EventAppended` references.
pub fn emit_phase_event<S: EventStore>(
    store: &mut S,
    input: &PhaseEventInput,
) -> Result<(EventAppended, EventAppended), StorageError> {
    let exited_id = format!("{}-exited-{}", input.event_id_prefix, input.cycle_id);
    let exited_env = build_event_envelope(
        &exited_id,
        "workflow.phase.exited",
        &input.from_phase,
        input,
    );
    let from_result = store.append(&exited_env)?;

    let entered_id = format!("{}-entered-{}", input.event_id_prefix, input.cycle_id);
    let entered_env = build_event_envelope(
        &entered_id,
        "workflow.phase.entered",
        &input.to_phase,
        input,
    );
    let to_result = store.append(&entered_env)?;

    Ok((from_result, to_result))
}

/// Emits a `workflow.transition.succeeded` or `workflow.transition.failed` event
/// to events_v1.
///
/// Idempotent: re-appending the same event_id returns the stored result.
///
/// Returns the stored `EventAppended` reference.
pub fn emit_outcome_event<S: EventStore>(
    store: &mut S,
    input: &OutcomeEventInput,
    outcome: TransitionOutcome,
) -> Result<EventAppended, StorageError> {
    let event_type = match outcome {
        TransitionOutcome::Succeeded => "workflow.transition.succeeded",
        TransitionOutcome::Failed => "workflow.transition.failed",
    };
    let event_id = format!("{}-outcome-{}", input.event_id_prefix, input.cycle_id);
    let env = build_outcome_envelope(event_id, event_type, input);
    store.append(&env)
}

/// Builds an `EventEnvelopeV1` for a transition-outcome event.
fn build_outcome_envelope(
    event_id: String,
    event_type: &str,
    input: &OutcomeEventInput,
) -> EventEnvelopeV1 {
    let payload = json!({
        "transition_id": input.transition_id,
        "outcome": serde_json::to_value(outcome_from_enum(event_type)).unwrap(),
        "from_phase": input.from_phase,
        "to_phase": input.to_phase,
        "failed_gates": input.failed_gates,
    });
    let mut env = EventEnvelopeV1 {
        event_id,
        event_type: event_type.to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.transition_at.clone(),
        recorded_at: input.transition_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "cycle".into(),
            id: input.cycle_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some(input.cycle_id.clone()),
        frame_id: None,
        fork_id: None,
    };
    env.content_hash = env.compute_content_hash();
    env
}

/// Converts an event_type string to a TransitionOutcome enum value for the payload.
/// `workflow.transition.succeeded` → Succeeded, `workflow.transition.failed` → Failed.
fn outcome_from_enum(event_type: &str) -> TransitionOutcome {
    match event_type {
        "workflow.transition.succeeded" => TransitionOutcome::Succeeded,
        "workflow.transition.failed" => TransitionOutcome::Failed,
        _ => TransitionOutcome::Failed,
    }
}

/// Builds an `EventEnvelopeV1` for a phase transition event.
fn build_event_envelope(
    event_id: &str,
    event_type: &str,
    phase_label: &str,
    input: &PhaseEventInput,
) -> EventEnvelopeV1 {
    let payload = json!({ "phase": phase_label });
    let mut env = EventEnvelopeV1 {
        event_id: event_id.to_string(),
        event_type: event_type.to_string(),
        schema_version: 1,
        stream_id: input.cycle_id.clone(),
        sequence: 0,
        project_id: input.project_id.clone(),
        occurred_at: input.transition_at.clone(),
        recorded_at: input.transition_at.clone(),
        actor: ActorRef {
            kind: input.actor_kind,
            id: input.actor_id.clone(),
            definition_hash: None,
            policy_hash: None,
            model: None,
        },
        subjects: vec![EntityRef {
            kind: "cycle".into(),
            id: input.cycle_id.clone(),
            version: None,
            content_hash: None,
        }],
        payload,
        evidence_refs: vec![],
        content_hash: String::new(),
        metadata: None,
        causation_id: None,
        correlation_id: None,
        cycle_id: Some(input.cycle_id.clone()),
        frame_id: None,
        fork_id: None,
    };
    env.content_hash = env.compute_content_hash();
    env
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_storage_dir_uses_xdg_state_home() {
        let dir = project_storage_dir_with(Some("/custom/xdg/state"), Some("/home/test"), "proj-1")
            .unwrap();
        assert_eq!(
            dir.to_str().unwrap(),
            "/custom/xdg/state/sddk/projects/proj-1"
        );
    }

    #[test]
    fn project_storage_dir_falls_back_to_home() {
        let dir = project_storage_dir_with(None, Some("/home/test"), "proj-2").unwrap();
        assert_eq!(
            dir.to_str().unwrap(),
            "/home/test/.local/state/sddk/projects/proj-2"
        );
    }

    #[test]
    fn build_event_envelope_produces_valid_envelope() {
        let input = PhaseEventInput {
            project_id: "p-1".into(),
            cycle_id: "c-1".into(),
            from_phase: "build".into(),
            to_phase: "test".into(),
            transition_at: "2026-08-17T10:00:00Z".into(),
            actor_id: "user:test".into(),
            actor_kind: ActorKind::Human,
            event_id_prefix: "e-c-1".into(),
        };
        let env = build_event_envelope(
            "e-c-1-entered-c-1",
            "workflow.phase.entered",
            "test",
            &input,
        );

        assert_eq!(env.event_type, "workflow.phase.entered");
        assert_eq!(env.stream_id, "c-1");
        assert_eq!(env.payload.get("phase").unwrap().as_str().unwrap(), "test");
        assert_eq!(env.actor.id, "user:test");
        assert!(!env.content_hash.is_empty());
        assert!(env.content_hash.starts_with("sha256:"));
        assert_eq!(env.subjects.len(), 1);
        assert_eq!(env.subjects[0].kind, "cycle");
        assert_eq!(env.subjects[0].id, "c-1");
    }
}
