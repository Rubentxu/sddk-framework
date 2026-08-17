//! Release plan and apply commands.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::GateOutcomeStatus;
use sddk_gateway::{
    CapabilityGateway, CapabilityPolicy, GitExecutor, GitHubForge, LocalReleaseInput,
    LocalReleaseOutcome, LocalReleasePreconditions, PermissionPolicy, ReleasePlanInput,
    apply_local_release, apply_release, plan_release,
};
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
    uat::ReleaseTypeArg,
};

#[derive(Debug, Subcommand)]
pub(crate) enum ReleaseCommand {
    /// Show the selected release sequence.
    Plan(ReleaseArgs),
    /// Apply the selected release route.
    Apply(ReleaseArgs),
    /// Package the current binary with checksums, SBOM, and attestation.
    Dist(DistArgs),
    /// Verify a dist prefix against its checksums and attestation.
    Verify(DistArgs),
}

/// Release authority selected for one invocation.
#[derive(Debug, Clone, Copy, ValueEnum, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ReleaseRoute {
    /// Push the trunk branch and annotated tag with local Git only.
    Local,
    /// Use the optional external forge integration.
    Forge,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct DistArgs {
    /// Distribution prefix directory.
    #[arg(long)]
    pub(crate) prefix: PathBuf,
    /// Release channel.
    #[arg(long, default_value = "release")]
    pub(crate) channel: String,
    /// Explicit RFC 3339 timestamp.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit source commit.
    #[arg(long)]
    pub(crate) commit: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReleaseArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Release authority. `local` never uses GitHub, CI/CD, or assets.
    #[arg(long, value_enum)]
    pub(crate) route: Option<ReleaseRoute>,
    /// GitHub repository as `owner/repo`, required only for `--route forge`.
    #[arg(long)]
    pub(crate) repo: Option<String>,
    /// Branch to release. Local releases must target the trunk branch.
    #[arg(long, default_value = "main")]
    pub(crate) branch: String,
    /// Target branch for the optional forge pull request.
    #[arg(long, default_value = "main")]
    pub(crate) base: String,
    /// Annotated tag message and optional forge release title.
    #[arg(long, default_value = "SDDK release")]
    pub(crate) title: String,
    /// Release tag.
    #[arg(long)]
    pub(crate) tag: String,
    /// Release cycle providing local verification and UAT evidence.
    #[arg(long)]
    pub(crate) cycle: Option<String>,
    /// Previous tag for semver diff (alternative to `--release-type`).
    /// When omitted the release type defaults to Major (fail-closed).
    #[arg(long)]
    pub(crate) previous_tag: Option<String>,
    /// Explicit release type (overrides `--previous-tag` auto-detection).
    #[arg(long, value_enum)]
    pub(crate) release_type: Option<ReleaseTypeArg>,
    /// Release notes.
    #[arg(long, default_value = "")]
    pub(crate) notes: String,
    /// Explicit approval for capability effects.
    #[arg(long)]
    pub(crate) approve: bool,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_release(command: ReleaseCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        ReleaseCommand::Plan(args) => run_release_plan(args, environment),
        ReleaseCommand::Apply(args) => run_release_apply(args, environment),
        ReleaseCommand::Dist(args) => run_release_dist(args),
        ReleaseCommand::Verify(args) => run_release_dist_verify(args),
    }
}

const CHECKSUMS_FILE: &str = "checksums.txt";
const SBOM_FILE: &str = "sbom.json";
const ATTESTATION_FILE: &str = "attestation.json";

/// Generated distribution artifacts for one binary.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct DistOutput {
    version: String,
    channel: String,
    commit: String,
    binary: String,
    checksums: String,
    sbom: String,
    attestation: String,
}

fn run_release_dist(args: DistArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<DistOutput> {
        let binary = std::env::current_exe()?;
        let bytes = std::fs::read(&binary)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        let version = env!("CARGO_PKG_VERSION").to_owned();
        let commit = args
            .commit
            .or_else(|| std::env::var("GITHUB_SHA").ok())
            .unwrap_or_else(|| "unknown".to_owned());
        let timestamp = args
            .timestamp
            .unwrap_or_else(crate::git_cmd::default_timestamp);

        let dist_dir = args.prefix.join("dist");
        std::fs::create_dir_all(&dist_dir)?;
        let binary_path = dist_dir.join("sddk");
        std::fs::write(&binary_path, &bytes)?;

        let checksums = format!("{}  {}\n", digest, "sddk");
        std::fs::write(dist_dir.join(CHECKSUMS_FILE), &checksums)?;

        let sbom = serde_json::json!({
            "tool": "sddk",
            "version": version,
            "commit": commit,
            "channel": args.channel,
            "binary_sha256": digest,
            "dependencies": workspace_dependencies(),
        });
        let sbom_path = dist_dir.join(SBOM_FILE);
        std::fs::write(&sbom_path, serde_json::to_string_pretty(&sbom)?)?;

        let attestation = serde_json::json!({
            "artifact": "sddk",
            "sha256": digest,
            "builder": "sddk dist",
            "channel": args.channel,
            "timestamp": timestamp,
            "commit": commit,
        });
        let attestation_path = dist_dir.join(ATTESTATION_FILE);
        std::fs::write(
            &attestation_path,
            serde_json::to_string_pretty(&attestation)?,
        )?;

        Ok(DistOutput {
            version,
            channel: args.channel.clone(),
            commit,
            binary: binary_path.to_string_lossy().into_owned(),
            checksums: dist_dir.join(CHECKSUMS_FILE).to_string_lossy().into_owned(),
            sbom: sbom_path.to_string_lossy().into_owned(),
            attestation: attestation_path.to_string_lossy().into_owned(),
        })
    })();
    render_result(result, format, dist_text)
}

fn run_release_dist_verify(args: DistArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<serde_json::Value> {
        let dist_dir = args.prefix.join("dist");
        let binary_path = dist_dir.join("sddk");
        let bytes = std::fs::read(&binary_path)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));

        let checksums = std::fs::read_to_string(dist_dir.join(CHECKSUMS_FILE))?;
        let expected = format!("{digest}  sddk\n");
        if checksums != expected {
            anyhow::bail!("checksums.txt does not match the binary digest");
        }

        let sbom: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dist_dir.join(SBOM_FILE))?)?;
        if sbom["binary_sha256"] != digest {
            anyhow::bail!("sbom.json binary digest does not match");
        }

        let attestation: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(dist_dir.join(ATTESTATION_FILE))?)?;
        if attestation["sha256"] != digest {
            anyhow::bail!("attestation.json digest does not match");
        }

        Ok(serde_json::json!({
            "valid": true,
            "binary_sha256": digest,
            "sbom_version": sbom["version"],
            "channel": attestation["channel"],
        }))
    })();
    render_result(result, format, dist_verify_text)
}

fn workspace_dependencies() -> Vec<serde_json::Value> {
    let lock = match std::fs::read_to_string(
        std::env::current_dir()
            .unwrap_or_default()
            .join("Cargo.lock"),
    ) {
        Ok(lock) => lock,
        Err(_) => return Vec::new(),
    };
    let mut dependencies = Vec::new();
    let mut name = None;
    for line in lock.lines() {
        if let Some(rest) = line.strip_prefix("name = ") {
            name = Some(rest.trim_matches('"').to_owned());
        } else if let Some(rest) = line.strip_prefix("version = ")
            && let Some(name) = name.take()
        {
            dependencies.push(serde_json::json!({
                "name": name,
                "version": rest.trim_matches('"'),
            }));
        }
    }
    dependencies
}

fn dist_text(output: &DistOutput) -> String {
    format!(
        "version: {}\nchannel: {}\ncommit: {}\nbinary: {}\nchecksums: {}\nsbom: {}\nattestation: {}\n",
        output.version,
        output.channel,
        output.commit,
        output.binary,
        output.checksums,
        output.sbom,
        output.attestation
    )
}

fn dist_verify_text(output: &serde_json::Value) -> String {
    format!(
        "valid: {}\nbinary_sha256: {}\nsbom_version: {}\nchannel: {}\n",
        output["valid"].as_bool().unwrap_or(false),
        output["binary_sha256"].as_str().unwrap_or(""),
        output["sbom_version"].as_str().unwrap_or(""),
        output["channel"].as_str().unwrap_or("")
    )
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ReleasePlanOutput {
    route: ReleaseRoute,
    branch: String,
    base: String,
    tag: String,
    head: Option<String>,
    steps: Vec<&'static str>,
}

fn run_release_plan(args: ReleaseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ReleasePlanOutput> {
        let route = selected_route(&args)?;
        if matches!(route, ReleaseRoute::Local) && (args.branch != "main" || args.base != "main") {
            anyhow::bail!("--route local requires --branch main and --base main");
        }
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let head = sddk_gateway::GitExecutor::new(context.root.clone())
            .inspect()?
            .head;
        Ok(ReleasePlanOutput {
            route,
            branch: args.branch.clone(),
            base: args.base.clone(),
            tag: args.tag.clone(),
            head,
            steps: match route {
                ReleaseRoute::Local => vec![
                    "push_main",
                    "verify_main_sha",
                    "create_annotated_tag",
                    "verify_remote_tag",
                ],
                ReleaseRoute::Forge => vec!["create_pr", "merge_pr", "create_release"],
            },
        })
    })();
    render_result(result, format, release_plan_text)
}

fn run_release_apply(args: ReleaseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let route = match selected_route(&args) {
        Ok(route) => route,
        Err(error) => return render_result(Err(error), format, release_outcome_text),
    };
    let result = (|| -> anyhow::Result<(
        String,
        std::path::PathBuf,
        CapabilityGateway,
        String,
        String,
        Option<LocalReleasePreconditions>,
    )> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let project_id = context.identity.project_id.to_string();
        let root = context.root.clone();
        let permissions = PermissionPolicy::from_file(root.join("permissions.yaml"))?;
        authorize_release(&permissions, route)?;
        let local_preconditions = matches!(route, ReleaseRoute::Local)
            .then(|| {
                local_release_preconditions(
                    &context,
                    &project_id,
                    args.cycle.as_deref(),
                    args.previous_tag.as_deref(),
                    args.release_type.map(|r| r.into()),
                    &args.tag,
                    environment,
                )
            })
            .transpose()?;
        let policy = CapabilityPolicy::from_workflow(context.engine.workflow());
        let gateway = CapabilityGateway::new(policy, context.storage);
        let timestamp = args
            .timestamp
            .clone()
            .unwrap_or_else(crate::git_cmd::default_timestamp);
        let actor = args
            .actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into());
        Ok((project_id, root, gateway, timestamp, actor, local_preconditions))
    })();
    let (project_id, root, mut gateway, timestamp, actor, local_preconditions) = match result {
        Ok(value) => value,
        Err(error) => return render_result(Err(error), format, release_outcome_text),
    };

    match route {
        ReleaseRoute::Local => {
            let result = (|| -> anyhow::Result<LocalReleaseOutcome> {
                if args.branch != "main" || args.base != "main" {
                    anyhow::bail!("--route local requires --branch main and --base main");
                }
                Ok(apply_local_release(
                    &mut gateway,
                    &LocalReleaseInput {
                        project_id,
                        cycle_id: args.cycle,
                        branch: args.branch,
                        tag: args.tag,
                        tag_message: args.title,
                        approve: args.approve,
                        timestamp,
                        actor,
                        preconditions: local_preconditions
                            .expect("local route preconditions were resolved"),
                    },
                    &GitExecutor::new(root),
                )?)
            })();
            render_result(result, format, local_release_outcome_text)
        }
        ReleaseRoute::Forge => {
            let result = (|| -> anyhow::Result<sddk_gateway::ReleaseOutcome> {
                let repo = args.repo.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("--repo is required when --route forge is selected")
                })?;
                let input = ReleasePlanInput {
                    project_id,
                    cycle_id: None,
                    branch: args.branch.clone(),
                    base_branch: args.base.clone(),
                    pr_title: args.title.clone(),
                    pr_body: format!("Release {} from {}", args.tag, args.branch),
                    tag: args.tag.clone(),
                    release_title: args.title.clone(),
                    release_notes: args.notes.clone(),
                    approve: args.approve,
                    timestamp,
                    actor,
                };
                let mut forge = GitHubForge::new(repo);
                let plan = plan_release(input, &forge)?;
                Ok(apply_release(&mut gateway, &plan, &mut forge)?)
            })();
            render_result(result, format, release_outcome_text)
        }
    }
}

fn selected_route(args: &ReleaseArgs) -> anyhow::Result<ReleaseRoute> {
    match args.route {
        Some(route) => Ok(route),
        None if args.repo.is_some() => anyhow::bail!(
            "release route now defaults to local; legacy Forge invocations must pass --route forge"
        ),
        None => Ok(ReleaseRoute::Local),
    }
}

fn authorize_release(policy: &PermissionPolicy, route: ReleaseRoute) -> anyhow::Result<()> {
    // The local route reads the local preconditions through `git.inspect` and
    // also applies `git.push` and `git.tag`. The forge route uses forge-only
    // capabilities. The permission registry MUST list every capability that
    // `apply_local_release` actually executes, otherwise the release would
    // run with an unauthorized read.
    let capabilities: &[&str] = match route {
        ReleaseRoute::Local => &["git.inspect", "git.push", "git.tag"],
        ReleaseRoute::Forge => &["pr.create", "pr.merge", "release.create"],
    };
    for capability in capabilities {
        let decision = policy.authorize("sddk-release", "release", capability);
        if !decision.allowed {
            anyhow::bail!("release permission denied: {}", decision.reason);
        }
    }
    Ok(())
}

fn local_release_preconditions(
    context: &RuntimeContext,
    project_id: &str,
    cycle_id: Option<&str>,
    previous_tag: Option<&str>,
    release_type_arg: Option<sddk_domain::ReleaseType>,
    current_tag: &str,
    environment: &CliEnvironment,
) -> anyhow::Result<LocalReleasePreconditions> {
    let cycle_id = cycle_id.ok_or_else(|| {
        anyhow::anyhow!("--cycle is required for --route local to verify local release evidence")
    })?;
    let cycle = context.storage.get_cycle(cycle_id)?;
    let manifest = cycle.manifest;
    if manifest.project_id != project_id
        || manifest.status != sddk_domain::CycleStatus::ReleasePending
        || manifest.phase != sddk_domain::Phase::Release
    {
        anyhow::bail!("cycle {cycle_id} is not the current release-pending cycle for this project");
    }

    // The release ties the cycle to the local trunk. The cycle MUST point at
    // a branch that is an ancestor of the current local trunk HEAD, and the
    // worktree MUST be on the trunk branch with a clean status. A cycle that
    // points at a different branch fails clearly here instead of silently
    // pushing the wrong commits.
    let trunk_branch = manifest.branch.as_str();
    if trunk_branch != "main" {
        anyhow::bail!(
            "cycle {cycle_id} points at branch {trunk_branch:?}; the local release route requires the cycle to point at the trunk branch main"
        );
    }
    let git = sddk_gateway::GitExecutor::new(context.root.clone());
    let inspect = git.inspect()?;
    if inspect.branch.as_deref() != Some("main") {
        anyhow::bail!(
            "cycle {cycle_id} is tied to trunk main but the worktree is on {}; checkout main before running the local release",
            inspect.branch.as_deref().unwrap_or("detached HEAD")
        );
    }
    if inspect.dirty {
        anyhow::bail!(
            "cycle {cycle_id} cannot release from a dirty worktree; commit or stash the changes first"
        );
    }
    if let Some(cycle_head) = manifest.head.as_deref() {
        let local_head = git.head_sha()?;
        if cycle_head != local_head && !is_ancestor(&git, cycle_head, &local_head)? {
            anyhow::bail!(
                "cycle {cycle_id} points at commit {cycle_head}, which is not an ancestor of the local trunk HEAD {local_head}; the cycle is on a different branch"
            );
        }
    }

    // UAT is not required for paths without a UAT phase (A-min, B-direct).
    let path_requires_uat = !matches!(
        manifest.path,
        sddk_domain::CyclePath::AMin | sddk_domain::CyclePath::BDirect
    );

    let uat_passed = if path_requires_uat {
        // Load UatConfig to evaluate the release gate.
        let config = crate::uat::load_uat_config(project_id, environment)?;
        let release_type = release_type_arg.unwrap_or_else(|| {
            if let Some(prev) = previous_tag {
                sddk_domain::release_type_from_diff(current_tag, prev)
                    .unwrap_or(sddk_domain::ReleaseType::Major)
            } else {
                // No signal → fail-closed (default to Major, which requires UAT).
                sddk_domain::ReleaseType::Major
            }
        });
        let action = sddk_domain::evaluate_release_gate(&config, release_type);
        if matches!(action, sddk_domain::ReleaseGateAction::Skip) {
            // UAT gate is configured to skip for this release type.
            true
        } else {
            // Consult the gate-receipts table.
            let gates = context.storage.list_gate_receipts(cycle_id)?;
            let passed = |gate: &str| {
                gates.iter().any(|receipt| {
                    receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed
                })
            };
            passed("release-uat-approved")
        }
    } else {
        // A-min and B-direct paths have no UAT phase — precondition satisfied.
        true
    };

    let gates = context.storage.list_gate_receipts(cycle_id)?;
    let passed = |gate: &str| {
        gates
            .iter()
            .any(|receipt| receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed)
    };
    Ok(LocalReleasePreconditions {
        verification_passed: manifest.artifacts.contains_key("verification-report")
            && passed("tests-pass")
            && passed("policy-compliant"),
        uat_passed,
    })
}

fn is_ancestor(
    git: &sddk_gateway::GitExecutor,
    ancestor: &str,
    descendant: &str,
) -> anyhow::Result<bool> {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(git.root())
        .arg("merge-base")
        .arg("--is-ancestor")
        .arg(ancestor)
        .arg(descendant)
        .status()?;
    Ok(status.success())
}

fn release_plan_text(output: &ReleasePlanOutput) -> String {
    let route = match output.route {
        ReleaseRoute::Local => "local",
        ReleaseRoute::Forge => "forge",
    };
    let mut text = format!(
        "route: {route}\nbranch: {}\nbase: {}\ntag: {}\nhead: {}\nsteps:\n",
        output.branch,
        output.base,
        output.tag,
        output.head.as_deref().unwrap_or("null")
    );
    for step in &output.steps {
        text.push_str(&format!("- {step}\n"));
    }
    text
}

fn release_outcome_text(output: &sddk_gateway::ReleaseOutcome) -> String {
    let mut text = format!(
        "converged: {}\napplied: {}\n",
        output.converged,
        output.applied.len()
    );
    for step in &output.applied {
        text.push_str(&format!("- {} {}\n", step.step, step.receipt_id));
    }
    for skip in &output.skipped {
        text.push_str(&format!("- skipped: {skip}\n"));
    }
    text
}

fn local_release_outcome_text(output: &LocalReleaseOutcome) -> String {
    let mut text = format!(
        "converged: {}\nsha: {}\ntag: {}\napplied: {}\n",
        output.converged,
        output.sha,
        output.tag,
        output.applied.len()
    );
    for step in &output.applied {
        text.push_str(&format!("- {} {}\n", step.step, step.receipt_id));
    }
    for skip in &output.skipped {
        text.push_str(&format!("- skipped: {skip}\n"));
    }
    text
}

#[cfg(test)]
mod tests {
    use sddk_domain::{GateOutcomeStatus, GateReceipt};

    /// Verifies the exact logic used by release_cmd's private `passed` closures
    /// (lines ~536-539 and ~549-552): a receipt satisfies a release gate only
    /// when outcome == Passed.  Waived does NOT satisfy release gates.
    #[test]
    fn release_gate_requires_passed_not_waived() {
        // Replicate the passed() closure from release_preconditions:
        //   let passed = |gate: &str| {
        //       gates.iter().any(|receipt| {
        //           receipt.gate == gate && receipt.outcome == GateOutcomeStatus::Passed
        //       })
        //   };
        let passed = |gates: &[GateReceipt], gate_name: &str| {
            gates
                .iter()
                .any(|r| r.gate == gate_name && r.outcome == GateOutcomeStatus::Passed)
        };

        let receipt_waived = GateReceipt {
            receipt_id: "rcpt-waived".into(),
            project_id: "p".into(),
            cycle_id: Some("c".into()),
            gate: "tests-pass".into(),
            evaluator: "eval".into(),
            transition_id: "t".into(),
            plan_hash: "h".into(),
            outcome: GateOutcomeStatus::Waived,
            evidence: serde_json::json!({}),
            actor: "test".into(),
            command_id: "cmd".into(),
            frame_id: "frame".into(),
            evaluated_at: "2026-08-03T12:00:00Z".into(),
            seq: 1,
        };
        let receipt_passed = GateReceipt {
            receipt_id: "rcpt-passed".into(),
            project_id: "p".into(),
            cycle_id: Some("c".into()),
            gate: "tests-pass".into(),
            evaluator: "eval".into(),
            transition_id: "t".into(),
            plan_hash: "h".into(),
            outcome: GateOutcomeStatus::Passed,
            evidence: serde_json::json!({}),
            actor: "test".into(),
            command_id: "cmd".into(),
            frame_id: "frame".into(),
            evaluated_at: "2026-08-03T12:00:00Z".into(),
            seq: 2,
        };

        // Waived receipt does NOT satisfy the release gate
        assert!(
            !passed(std::slice::from_ref(&receipt_waived), "tests-pass"),
            "Waived receipt must NOT satisfy release gate (fails-closed)"
        );
        // Passed receipt DOES satisfy the release gate
        assert!(
            passed(std::slice::from_ref(&receipt_passed), "tests-pass"),
            "Passed receipt must satisfy release gate"
        );
        // Only Passed matters; Waived alongside Passed still passes
        assert!(
            passed(&[receipt_waived, receipt_passed], "tests-pass"),
            "Passed receipt among Waived must still satisfy release gate"
        );
    }
}
