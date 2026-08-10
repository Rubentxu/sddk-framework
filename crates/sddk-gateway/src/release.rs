//! Release planning, idempotent application, and effect reconciliation.

use serde::Serialize;
use serde_json::json;
use thiserror::Error;

use crate::forge::{Forge, ForgeError, PrRequest, ReleaseRequest};
use crate::gateway::{CapabilityGateway, CapabilityPlanInput, GatewayError};
use crate::git::{GitError, GitExecutor};
use sddk_storage::{CapabilityReceipt, CapabilityStatus};

/// Inputs for one release across a forge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ReleasePlanInput {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle identifier.
    pub cycle_id: Option<String>,
    /// Source branch to release.
    pub branch: String,
    /// Target branch for the pull request.
    pub base_branch: String,
    /// Pull request title.
    pub pr_title: String,
    /// Pull request body.
    pub pr_body: String,
    /// Release tag.
    pub tag: String,
    /// Release title.
    pub release_title: String,
    /// Release notes.
    pub release_notes: String,
    /// Explicit approval for R3/R4 forge steps.
    pub approve: bool,
    /// Caller-supplied deterministic timestamp.
    pub timestamp: String,
    /// Actor responsible for the release.
    pub actor: String,
}

/// One executable release phase of the canonical sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ReleaseStep {
    /// Open the pull request when none is open.
    CreatePr,
    /// Merge the open pull request.
    MergePr,
    /// Publish the release when missing.
    CreateRelease,
}

/// Deterministic release plan over the current forge state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleasePlan {
    /// Original inputs.
    pub input: ReleasePlanInput,
    /// Ordered steps required to converge.
    pub steps: Vec<ReleaseStep>,
}

/// Outcome of applying a release plan.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct ReleaseOutcome {
    /// Executed steps with their receipts.
    pub applied: Vec<StepOutcome>,
    /// Steps skipped because the provider already held the effect.
    pub skipped: Vec<String>,
    /// Whether the release converged to the target state.
    pub converged: bool,
}

/// Receipt of one executed release step.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct StepOutcome {
    /// Step label.
    pub step: String,
    /// Persisted capability receipt id.
    pub receipt_id: String,
    /// Provider result summary.
    pub result: serde_json::Value,
}

/// Errors emitted by release planning, application, and reconciliation.
#[derive(Debug, Error)]
pub enum ReleaseError {
    /// The forge rejected an operation.
    #[error("release forge error: {0}")]
    Forge(#[from] ForgeError),
    /// A receipt could not be started or finalized.
    #[error("release gateway error: {0}")]
    Gateway(#[from] GatewayError),
    /// Structured data could not be encoded.
    #[error("release serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    /// Persistence rejected the operation.
    #[error("release storage error: {0}")]
    Storage(#[from] sddk_storage::StorageError),
    /// A local Git postcondition did not hold.
    #[error("release git error: {0}")]
    Git(#[from] GitError),
}

/// Inputs for a local trunk-based release without a forge dependency.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LocalReleaseInput {
    /// Owning project identifier.
    pub project_id: String,
    /// Related cycle identifier.
    pub cycle_id: Option<String>,
    /// Trunk branch pushed directly by the release.
    pub branch: String,
    /// Release tag.
    pub tag: String,
    /// Annotated tag message.
    pub tag_message: String,
    /// Explicit approval for capability effects.
    pub approve: bool,
    /// Caller-supplied deterministic timestamp.
    pub timestamp: String,
    /// Actor responsible for the release.
    pub actor: String,
}

/// Outcome of a local trunk-based release.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct LocalReleaseOutcome {
    /// SHA verified on both local HEAD and the remote trunk branch.
    pub sha: String,
    /// Verified annotated tag on the remote.
    pub tag: String,
    /// Executed capability effects with their receipts.
    pub applied: Vec<StepOutcome>,
    /// Effects skipped because their postconditions already held.
    pub skipped: Vec<String>,
    /// Whether the local release converged to trunk SHA plus remote tag.
    pub converged: bool,
}

/// Computes the ordered steps needed to converge a release.
pub fn plan_release(
    input: ReleasePlanInput,
    forge: &dyn Forge,
) -> Result<ReleasePlan, ReleaseError> {
    let mut steps = Vec::new();
    let open_pr = forge.find_open_pr(&input.branch, &input.base_branch)?;
    if open_pr.is_none() {
        steps.push(ReleaseStep::CreatePr);
    }
    steps.push(ReleaseStep::MergePr);
    if !forge
        .release_state(&input.tag)?
        .is_some_and(|state| state.published)
    {
        steps.push(ReleaseStep::CreateRelease);
    }
    Ok(ReleasePlan { input, steps })
}

/// Applies the local release contract: push trunk, verify its SHA, and create
/// or verify one annotated remote tag. It never reads provider checks or waits
/// for CI/CD or external distribution.
pub fn apply_local_release(
    gateway: &mut CapabilityGateway,
    input: &LocalReleaseInput,
    git: &GitExecutor,
) -> Result<LocalReleaseOutcome, ReleaseError> {
    let mut applied = Vec::new();
    let mut skipped = Vec::new();
    let head = git.head_sha()?;

    if git.remote_branch_sha(&input.branch)?.as_deref() == Some(head.as_str()) {
        skipped.push(format!(
            "push-main (origin/{} already at {head})",
            input.branch
        ));
    } else {
        let branch_args = vec![input.branch.clone(), head.clone()];
        let receipt = run_local_step(
            gateway,
            input,
            "git.push",
            &branch_args,
            "push direct trunk branch",
            |git| {
                Ok(
                    json!({"branch": input.branch, "sha": git.push_and_verify_branch(&input.branch)?}),
                )
            },
            git,
        )?;
        applied.push(receipt);
    }

    let sha = git.verify_head_matches_remote_branch(&input.branch)?;
    match git.remote_annotated_tag_target(&input.tag)? {
        Some(existing) if existing == sha => {
            skipped.push(format!("tag ({} already points to {sha})", input.tag));
        }
        Some(existing) => {
            return Err(ReleaseError::Git(GitError::Postcondition {
                command: format!("verify remote tag {}", input.tag),
                expected: sha,
                actual: existing,
            }));
        }
        None => {
            let tag_args = vec![input.tag.clone(), sha.clone()];
            let receipt = run_local_step(
                gateway,
                input,
                "git.tag",
                &tag_args,
                "create and push annotated release tag",
                |git| {
                    git.create_annotated_tag(&input.tag, &sha, &input.tag_message)?;
                    git.push_and_verify_annotated_tag(&input.tag, &sha)?;
                    Ok(json!({"tag": input.tag, "sha": sha, "annotated": true}))
                },
                git,
            )?;
            applied.push(receipt);
        }
    }

    let converged = git.verify_head_matches_remote_branch(&input.branch)? == sha
        && git.remote_annotated_tag_target(&input.tag)?.as_deref() == Some(sha.as_str());
    Ok(LocalReleaseOutcome {
        sha,
        tag: input.tag.clone(),
        applied,
        skipped,
        converged,
    })
}

/// Applies a release plan idempotently against the forge.
///
/// Every step records a capability receipt. Interrupted runs converge: an
/// already-merged PR or already-published release is skipped without duplicating
/// effects, and the provider state is re-checked before each step.
pub fn apply_release(
    gateway: &mut CapabilityGateway,
    plan: &ReleasePlan,
    forge: &mut dyn Forge,
) -> Result<ReleaseOutcome, ReleaseError> {
    let mut applied = Vec::new();
    let mut skipped = Vec::new();

    for step in &plan.steps {
        match step {
            ReleaseStep::CreatePr => {
                if forge
                    .find_open_pr(&plan.input.branch, &plan.input.base_branch)?
                    .is_some()
                {
                    skipped.push("create-pr (open PR exists)".into());
                    continue;
                }
                let branch_args = vec![plan.input.branch.clone()];
                let receipt = run_step(
                    gateway,
                    &plan.input,
                    "pr.create",
                    &branch_args,
                    "open pull request",
                    |forge: &mut dyn Forge, _: &str| {
                        let pr = forge.create_pr(&PrRequest {
                            title: plan.input.pr_title.clone(),
                            body: plan.input.pr_body.clone(),
                            head: plan.input.branch.clone(),
                            base: plan.input.base_branch.clone(),
                        })?;
                        Ok(json!({"pr_number": pr.pr_number, "url": pr.url}))
                    },
                    forge,
                )?;
                applied.push(receipt);
            }
            ReleaseStep::MergePr => {
                let Some(number) =
                    forge.find_open_pr(&plan.input.branch, &plan.input.base_branch)?
                else {
                    skipped.push("merge-pr (no open PR)".into());
                    continue;
                };
                let number_args = vec![number.to_string()];
                let receipt = run_step(
                    gateway,
                    &plan.input,
                    "pr.merge",
                    &number_args,
                    "merge pull request",
                    |forge: &mut dyn Forge, _: &str| {
                        let merged = forge.merge_pr(number)?;
                        Ok(json!({"merged": merged.merged, "merge_sha": merged.merge_sha}))
                    },
                    forge,
                )?;
                applied.push(receipt);
            }
            ReleaseStep::CreateRelease => {
                if forge
                    .release_state(&plan.input.tag)?
                    .is_some_and(|state| state.published)
                {
                    skipped.push(format!(
                        "create-release ({} already published)",
                        plan.input.tag
                    ));
                    continue;
                }
                let tag_args = vec![plan.input.tag.clone()];
                let receipt = run_step(
                    gateway,
                    &plan.input,
                    "release.create",
                    &tag_args,
                    "publish release",
                    |forge: &mut dyn Forge, _: &str| {
                        let release = forge.create_release(&ReleaseRequest {
                            tag: plan.input.tag.clone(),
                            title: plan.input.release_title.clone(),
                            notes: plan.input.release_notes.clone(),
                            target_commitish: plan.input.base_branch.clone(),
                        })?;
                        Ok(json!({"tag": release.tag, "url": release.url}))
                    },
                    forge,
                )?;
                applied.push(receipt);
            }
        }
    }

    let converged = forge
        .find_open_pr(&plan.input.branch, &plan.input.base_branch)?
        .is_none()
        && forge
            .release_state(&plan.input.tag)?
            .is_some_and(|state| state.published);
    Ok(ReleaseOutcome {
        applied,
        skipped,
        converged,
    })
}

fn run_step(
    gateway: &mut CapabilityGateway,
    input: &ReleasePlanInput,
    capability: &str,
    args: &[String],
    reason: &str,
    effect: impl FnOnce(&mut dyn Forge, &str) -> Result<serde_json::Value, ForgeError>,
    forge: &mut dyn Forge,
) -> Result<StepOutcome, ReleaseError> {
    let plan_input = CapabilityPlanInput {
        project_id: input.project_id.clone(),
        cycle_id: input.cycle_id.clone(),
        capability: capability.to_owned(),
        reason: reason.to_owned(),
        program: "forge".into(),
        args: args.to_vec(),
        env: Default::default(),
        timeout_ms: 60_000,
        output_max_bytes: 1_048_576,
        approve: input.approve,
        timestamp: input.timestamp.clone(),
        actor: input.actor.clone(),
    };
    let begin = gateway.begin_effect(&plan_input)?;
    if begin.status != CapabilityStatus::Started {
        return Ok(StepOutcome {
            step: capability.to_owned(),
            receipt_id: begin.receipt_id,
            result: begin.result.unwrap_or(serde_json::Value::Null),
        });
    }
    let argument = args.first().cloned().unwrap_or_default();
    let result = effect(forge, &argument)?;
    let receipt = gateway.finish_effect(
        &begin.receipt_id,
        CapabilityStatus::Succeeded,
        result.clone(),
        &input.timestamp,
    )?;
    Ok(StepOutcome {
        step: capability.to_owned(),
        receipt_id: receipt.receipt_id,
        result,
    })
}

fn run_local_step(
    gateway: &mut CapabilityGateway,
    input: &LocalReleaseInput,
    capability: &str,
    args: &[String],
    reason: &str,
    effect: impl FnOnce(&GitExecutor) -> Result<serde_json::Value, GitError>,
    git: &GitExecutor,
) -> Result<StepOutcome, ReleaseError> {
    let plan_input = CapabilityPlanInput {
        project_id: input.project_id.clone(),
        cycle_id: input.cycle_id.clone(),
        capability: capability.to_owned(),
        reason: reason.to_owned(),
        program: "git".into(),
        args: args.to_vec(),
        env: Default::default(),
        timeout_ms: 60_000,
        output_max_bytes: 1_048_576,
        approve: input.approve,
        timestamp: input.timestamp.clone(),
        actor: input.actor.clone(),
    };
    let begin = gateway.begin_effect(&plan_input)?;
    if begin.status != CapabilityStatus::Started {
        return Ok(StepOutcome {
            step: capability.to_owned(),
            receipt_id: begin.receipt_id,
            result: begin.result.unwrap_or(serde_json::Value::Null),
        });
    }
    let result = effect(git)?;
    let receipt = gateway.finish_effect(
        &begin.receipt_id,
        CapabilityStatus::Succeeded,
        result.clone(),
        &input.timestamp,
    )?;
    Ok(StepOutcome {
        step: capability.to_owned(),
        receipt_id: receipt.receipt_id,
        result,
    })
}

/// Reconciles started receipts against provider reality.
///
/// Receipts left in the started state by an interrupted run are finalized by
/// querying the forge: a present effect finalizes as succeeded, an absent one
/// as failed.
pub fn reconcile_pending(
    gateway: &mut CapabilityGateway,
    forge: &dyn Forge,
) -> Result<Vec<CapabilityReceipt>, ReleaseError> {
    let mut reconciled = Vec::new();
    for receipt in gateway.storage.list_all_capability_receipts()? {
        if receipt.status != CapabilityStatus::Started {
            continue;
        }
        let argument = receipt
            .request
            .get("arguments")
            .and_then(|args| args.as_array())
            .and_then(|args| args.first())
            .and_then(|value| value.as_str())
            .unwrap_or_default();
        let (status, result) = match receipt.capability.as_str() {
            "pr.create" => match forge.find_open_pr(argument, "")? {
                Some(_) => (CapabilityStatus::Succeeded, json!({"present": true})),
                None => (CapabilityStatus::Failed, json!({"present": false})),
            },
            "release.create" | "release.publish" => {
                let published = forge
                    .release_state(argument)?
                    .is_some_and(|state| state.published);
                if published {
                    (CapabilityStatus::Succeeded, json!({"present": true}))
                } else {
                    (CapabilityStatus::Failed, json!({"present": false}))
                }
            }
            _ => continue,
        };
        let finalized = gateway.finish_effect(
            &receipt.receipt_id,
            status,
            result,
            receipt.started_at.as_str(),
        )?;
        reconciled.push(finalized);
    }
    Ok(reconciled)
}
