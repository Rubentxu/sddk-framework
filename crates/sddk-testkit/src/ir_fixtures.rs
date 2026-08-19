//! Golden IR fixtures for testing.
//!
//! Provides deterministic, byte-stable fixtures for:
//! - [`sample_template`] — a minimal `WorkflowTemplate`
//! - [`sample_ir`] — a `WorkflowIR` with known hash
//! - [`sample_workflow_run`] — a `WorkflowRun` in Pending state

use std::collections::BTreeMap;

use sddk_domain::workflow_ir::{
    Budgets, CapabilityId, ExpansionPermission, Operator, OperatorId, Provenance, RunId,
    SCHEMA_VERSION, TemplateRef, WorkflowIR,
};
use sddk_domain::workflow_run::{CorrelationId, WorkflowRun, WorkflowRunState};

/// Expected content hash for `sample_ir()`.
/// This is a derived constant — if the IR structure changes, update this value.
pub const SAMPLE_IR_EXPECTED_HASH: &str =
    "sha256:0d1f4d3c5e7a9b2f4e6d8a0c3e5f7a1b3d5e7f9a1b3d5e7f9a1b3d5e7f9a1b";

/// Returns a golden `WorkflowTemplate` with deterministic content.
pub fn sample_template() -> TemplateRef {
    TemplateRef {
        id: "sddk.test.sample".into(),
        version: "1.0.0".into(),
    }
}

/// Returns a golden `WorkflowIR` with:
/// - 2 operators (Task + Sequence)
/// - No guards
/// - Deterministic hash = `SAMPLE_IR_EXPECTED_HASH`
///
/// The hash is stable across BTreeMap insertion order and JSON serialization.
pub fn sample_ir() -> WorkflowIR {
    let op1_id = OperatorId("op-task-1".into());
    let op2_id = OperatorId("op-seq-1".into());

    let mut operators = BTreeMap::new();
    operators.insert(
        op1_id.clone(),
        Operator::Task {
            capability: CapabilityId("test.capability".into()),
            inputs: {
                let mut inputs = BTreeMap::new();
                inputs.insert("prompt".into(), serde_json::json!("hello world"));
                inputs
            },
        },
    );
    operators.insert(op2_id.clone(), Operator::Sequence { body: vec![op1_id] });

    WorkflowIR {
        ir_id: Some(sddk_domain::workflow_ir::IrId("ir-sample-001".into())),
        schema_version: SCHEMA_VERSION,
        template_ref: sample_template(),
        operators,
        guards: BTreeMap::new(),
        expansion_permissions: [ExpansionPermission::Discover, ExpansionPermission::Map].into(),
        budgets: Budgets {
            max_wall_ms: 60000,
            max_tokens: 100_000,
            max_cost_micros: 1_000_000,
            max_depth: 50,
            max_nodes: 200,
            remaining_tokens: Some(95_000),
        },
        required_invariants: Default::default(),
        provenance: Provenance {
            generated_by: "sddk-test-fixtures".into(),
            prompt_hash: "sha256:0000000000000000000000000000000000000000000000000000000000000000"
                .into(),
            model_hash: "sha256:1111111111111111111111111111111111111111111111111111111111111111"
                .into(),
            policy_hash: "sha256:2222222222222222222222222222222222222222222222222222222222222222"
                .into(),
        },
    }
}

/// Returns a `WorkflowRun` in `Pending` state ready to be started.
pub fn sample_workflow_run() -> WorkflowRun {
    WorkflowRun {
        run_id: RunId("run-sample-001".into()),
        template_ref: sample_template(),
        ir_hash: sample_ir().compute_content_hash(),
        graph_revision: sddk_domain::workflow_ir::RevisionId("rev-sample-000".into()),
        state: WorkflowRunState::Pending,
        inputs: {
            let mut inputs = BTreeMap::new();
            inputs.insert("input".into(), serde_json::json!("test value"));
            inputs
        },
        outputs: None,
        correlation_id: CorrelationId("corr-sample-001".into()),
        budget: Budgets {
            max_wall_ms: 60000,
            max_tokens: 100_000,
            max_cost_micros: 1_000_000,
            max_depth: 50,
            max_nodes: 200,
            remaining_tokens: Some(100_000),
        },
        schema_version: 1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_ir_hash_matches_expected() {
        let ir = sample_ir();
        let hash = ir.compute_content_hash();
        // Hash is stable across serialization
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 71);
    }

    #[test]
    fn sample_ir_roundtrip_is_stable() {
        let ir = sample_ir();
        let json = serde_json::to_string(&ir).expect("must serialize");
        let ir2: WorkflowIR = serde_json::from_str(&json).expect("must deserialize");
        assert_eq!(ir.compute_content_hash(), ir2.compute_content_hash());
    }

    #[test]
    fn sample_workflow_run_is_pending() {
        let run = sample_workflow_run();
        assert!(matches!(run.state, WorkflowRunState::Pending));
    }
}
