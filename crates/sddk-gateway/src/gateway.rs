//! Capability gateway orchestrating policy, execution, and receipts.

use std::collections::BTreeMap;

use sddk_storage::{CapabilityReceipt, CapabilityReceiptInput, CapabilityStatus, Storage};
use serde_json::{Value, json};
use thiserror::Error;

use crate::policy::{CapabilityPolicy, PolicyDecision};
use crate::redact;
use crate::runner::{RunSpec, run};

/// Caller input used to plan one capability execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityPlanInput {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle, when applicable.
    pub cycle_id: Option<String>,
    /// Declared capability identifier.
    pub capability: String,
    /// Human-readable justification.
    pub reason: String,
    /// Executable invoked by the typed runner.
    pub program: String,
    /// Positional arguments passed without a shell.
    pub args: Vec<String>,
    /// Environment allowlist.
    pub env: BTreeMap<String, String>,
    /// Runner timeout in milliseconds.
    pub timeout_ms: u64,
    /// Runner output limit in bytes per stream.
    pub output_max_bytes: usize,
    /// Whether the caller supplies explicit human approval.
    pub approve: bool,
    /// Caller-supplied deterministic timestamps and actor.
    pub timestamp: String,
    /// Actor responsible for the request.
    pub actor: String,
}

/// A policy-validated plan ready to execute.
#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityPlan {
    /// Policy outcome for the capability.
    pub decision: PolicyDecision,
    /// Original request.
    pub input: CapabilityPlanInput,
    /// Derived runner specification.
    pub run_spec: RunSpec,
    /// Idempotency key binding retries to one receipt.
    pub idempotency_key: String,
    /// Deterministic receipt identifier.
    pub receipt_id: String,
}

/// Errors emitted by the capability gateway.
#[derive(Debug, Error)]
pub enum GatewayError {
    /// The policy denies the capability.
    #[error("capability {capability} is denied by policy")]
    Denied {
        /// Denied capability identifier.
        capability: String,
    },
    /// The capability requires approval that was not supplied.
    #[error("capability {capability} requires approval")]
    ApprovalRequired {
        /// Capability awaiting approval.
        capability: String,
    },
    /// The stored request disagrees with the supplied idempotency key.
    #[error("gateway idempotency error: {0}")]
    Idempotency(#[from] sddk_storage::StorageError),
    /// The runner failed to execute the plan.
    #[error("gateway runner error: {0}")]
    Runner(#[from] crate::runner::RunnerError),
    /// A structured payload could not be encoded.
    #[error("payload serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
}

/// Default-deny gateway combining policy, execution, and receipt persistence.
pub struct CapabilityGateway {
    pub(crate) policy: CapabilityPolicy,
    pub(crate) storage: Storage,
}

impl CapabilityGateway {
    /// Constructs a gateway with a policy and the project ledger.
    pub fn new(policy: CapabilityPolicy, storage: Storage) -> Self {
        Self { policy, storage }
    }

    /// Evaluates policy and builds an executable plan.
    pub fn plan(&self, input: CapabilityPlanInput) -> Result<CapabilityPlan, GatewayError> {
        let decision = self.policy.authorize(&input.capability, input.approve);
        if !decision.allowed {
            if decision.requires_approval {
                return Err(GatewayError::ApprovalRequired {
                    capability: input.capability.clone(),
                });
            }
            return Err(GatewayError::Denied {
                capability: input.capability.clone(),
            });
        }
        let run_spec = RunSpec {
            program: input.program.clone(),
            args: input.args.clone(),
            env: input.env.clone(),
            timeout_ms: input.timeout_ms,
            output_max_bytes: input.output_max_bytes,
        };
        let request_key = crate::stable_request_key(
            &input.project_id,
            &input.cycle_id,
            &input.capability,
            &input.args,
            &input.reason,
        );
        let idempotency_key = format!("{}-{}", input.capability, &request_key[..16]);
        let receipt_id = format!(
            "cap-{}-{}",
            input.capability.replace('.', "-"),
            &request_key[..12]
        );
        Ok(CapabilityPlan {
            decision,
            input,
            run_spec,
            idempotency_key,
            receipt_id,
        })
    }

    /// Executes a plan with begin -> run -> finalize receipt lifecycle.
    ///
    /// The request and result are redacted before persistence. A failed or
    /// timed-out run finalizes the receipt as `Failed`.
    pub fn apply(&mut self, plan: &CapabilityPlan) -> Result<CapabilityReceipt, GatewayError> {
        let begin = self.begin_effect(&plan.input)?;
        if begin.status != CapabilityStatus::Started {
            return Ok(begin);
        }

        let outcome = run(&plan.run_spec)?;
        let (status, result) = if outcome.timed_out {
            (
                CapabilityStatus::Failed,
                json!({"error": "timed out", "stderr": outcome.stderr}),
            )
        } else if outcome.exit_status == Some(0) {
            (
                CapabilityStatus::Succeeded,
                json!({"stdout": outcome.stdout}),
            )
        } else {
            (
                CapabilityStatus::Failed,
                json!({"exit_status": outcome.exit_status, "stderr": outcome.stderr}),
            )
        };

        self.finish_effect(&begin.receipt_id, status, result, &plan.input.timestamp)
    }

    /// Starts a capability effect under policy and persists a started receipt.
    ///
    /// The request is redacted and the idempotency key is derived
    /// deterministically from the request; replaying the same request returns
    /// the original receipt.
    pub fn begin_effect(
        &mut self,
        input: &CapabilityPlanInput,
    ) -> Result<CapabilityReceipt, GatewayError> {
        let decision = self.policy.authorize(&input.capability, input.approve);
        if !decision.allowed {
            if decision.requires_approval {
                return Err(GatewayError::ApprovalRequired {
                    capability: input.capability.clone(),
                });
            }
            return Err(GatewayError::Denied {
                capability: input.capability.clone(),
            });
        }
        let request_key = crate::stable_request_key(
            &input.project_id,
            &input.cycle_id,
            &input.capability,
            &input.args,
            &input.reason,
        );
        let request = json!({
            "capability": input.capability,
            "arguments": input.args,
            "reason": input.reason,
        });
        Ok(self
            .storage
            .begin_capability_receipt(&CapabilityReceiptInput {
                receipt_id: format!(
                    "cap-{}-{}",
                    input.capability.replace('.', "-"),
                    &request_key[..12]
                ),
                project_id: input.project_id.clone(),
                cycle_id: input.cycle_id.clone(),
                capability: input.capability.clone(),
                idempotency_key: format!("{}-{}", input.capability, &request_key[..16]),
                request: redact(request),
                status: CapabilityStatus::Started,
                result: None,
                started_at: input.timestamp.clone(),
                completed_at: None,
            })?)
    }

    /// Finalizes a started effect receipt with a redacted result.
    pub fn finish_effect(
        &mut self,
        receipt_id: &str,
        status: CapabilityStatus,
        result: Value,
        completed_at: &str,
    ) -> Result<CapabilityReceipt, GatewayError> {
        Ok(self.storage.finalize_capability_receipt(
            receipt_id,
            status,
            Some(redact(result)),
            completed_at,
        )?)
    }

    /// Lists persisted receipts for a project.
    pub fn receipts(&self, project_id: &str) -> Result<Vec<CapabilityReceipt>, GatewayError> {
        Ok(self.storage.list_capability_receipts(project_id)?)
    }
}

#[cfg(test)]
mod tests {
    use sddk_domain::{CapabilityDef, ForgeDef};
    use sddk_storage::{ProjectRecord, Storage};

    use super::{CapabilityGateway, CapabilityPlanInput};

    const WORKFLOW_YAML: &str = include_str!("../../../workflow/workflow.yaml");

    fn gateway() -> (Storage, CapabilityGateway) {
        let mut workflow = sddk_engine::load_workflow_str(WORKFLOW_YAML).unwrap();
        workflow.forge = Some(ForgeDef {
            provider: "auto".into(),
            capabilities: Some(
                [
                    ("echo.test", Some("low"), Some("creates")),
                    ("git.delete_branch", Some("medium"), Some("irreversible")),
                ]
                .into_iter()
                .map(|(name, risk, consequence)| {
                    (
                        name.to_owned(),
                        CapabilityDef {
                            risk: risk.map(str::to_owned),
                            consequence: consequence.map(str::to_owned),
                        },
                    )
                })
                .collect(),
            ),
        });
        let policy = crate::CapabilityPolicy::from_workflow(&workflow);
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("ledger.sqlite");
        let storage = Storage::open(&path).unwrap();
        storage
            .insert_project(&ProjectRecord {
                project_id: "project-1".into(),
                display_name: "project".into(),
                remote_url: Some("https://example.com/owner/project".into()),
                scope: "owner".into(),
                created_at: "2026-08-04T10:00:00Z".into(),
            })
            .unwrap();
        let gateway_storage = Storage::open(&path).unwrap();
        std::mem::forget(directory);
        let gateway = CapabilityGateway::new(policy, gateway_storage);
        (storage, gateway)
    }

    fn input(capability: &str, program: &str, approve: bool) -> CapabilityPlanInput {
        CapabilityPlanInput {
            project_id: "project-1".into(),
            cycle_id: None,
            capability: capability.into(),
            reason: "test".into(),
            program: program.into(),
            args: vec!["hello".into()],
            env: Default::default(),
            timeout_ms: 5_000,
            output_max_bytes: 1_024,
            approve,
            timestamp: "2026-08-04T10:00:00Z".into(),
            actor: "gateway-test".into(),
        }
    }

    #[test]
    fn unknown_capability_is_denied() {
        let (_storage, gateway) = gateway();
        let plan = gateway.plan(input("git.push", "echo", false));
        assert!(matches!(
            plan,
            Err(crate::GatewayError::Denied { capability }) if capability == "git.push"
        ));
    }

    #[test]
    fn irreversible_capability_requires_approval() {
        let (_storage, gateway) = gateway();
        let denied = gateway.plan(input("git.delete_branch", "echo", false));
        assert!(matches!(
            denied,
            Err(crate::GatewayError::ApprovalRequired { capability }) if capability == "git.delete_branch"
        ));
        let plan = gateway
            .plan(input("git.delete_branch", "echo", true))
            .unwrap();
        assert!(plan.decision.allowed);
    }

    #[test]
    fn apply_runs_typed_program_and_persists_redacted_receipt() {
        let (storage, mut gateway) = gateway();
        let plan = gateway.plan(input("echo.test", "echo", false)).unwrap();
        let receipt = gateway.apply(&plan).unwrap();
        assert_eq!(receipt.status, sddk_storage::CapabilityStatus::Succeeded);
        assert!(receipt.result.unwrap().to_string().contains("hello"));

        let listed = gateway.receipts("project-1").unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].status, sddk_storage::CapabilityStatus::Succeeded);
        assert!(storage.get_capability_receipt(&receipt.receipt_id).is_ok());
    }

    #[test]
    fn apply_reuses_receipt_for_the_same_request() {
        let (_storage, mut gateway) = gateway();
        let plan = gateway.plan(input("echo.test", "echo", false)).unwrap();
        let first = gateway.apply(&plan).unwrap();
        let second = gateway.apply(&plan).unwrap();
        assert_eq!(first.receipt_id, second.receipt_id);
    }
}
