//! Release plan and apply commands.

use clap::{Args, Subcommand};
use sddk_gateway::{
    CapabilityGateway, CapabilityPolicy, GitHubForge, ReleasePlanInput, apply_release, plan_release,
};
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

#[derive(Debug, Subcommand)]
pub(crate) enum ReleaseCommand {
    /// Show the canonical release sequence for a branch.
    Plan(ReleaseArgs),
    /// Apply the release through the GitHub adapter.
    Apply(ReleaseArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct ReleaseArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// GitHub repository as `owner/repo`.
    #[arg(long)]
    pub(crate) repo: String,
    /// Source branch to release.
    #[arg(long)]
    pub(crate) branch: String,
    /// Target branch for the pull request.
    #[arg(long, default_value = "main")]
    pub(crate) base: String,
    /// Pull request and release title.
    #[arg(long)]
    pub(crate) title: String,
    /// Release tag.
    #[arg(long)]
    pub(crate) tag: String,
    /// Release notes.
    #[arg(long, default_value = "")]
    pub(crate) notes: String,
    /// Explicit approval for R3/R4 forge steps.
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
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ReleasePlanOutput {
    branch: String,
    base: String,
    tag: String,
    head: Option<String>,
    steps: Vec<&'static str>,
}

fn run_release_plan(args: ReleaseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<ReleasePlanOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let head = sddk_gateway::GitExecutor::new(context.root.clone())
            .inspect()?
            .head;
        Ok(ReleasePlanOutput {
            branch: args.branch.clone(),
            base: args.base.clone(),
            tag: args.tag.clone(),
            head,
            steps: vec!["create_pr", "merge_pr", "create_release"],
        })
    })();
    render_result(result, format, release_plan_text)
}

fn run_release_apply(args: ReleaseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<sddk_gateway::ReleaseOutcome> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let policy = CapabilityPolicy::from_workflow(context.engine.workflow());
        let mut gateway = CapabilityGateway::new(policy, context.storage);
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
        let input = ReleasePlanInput {
            project_id: context.identity.project_id.to_string(),
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
        let mut forge = GitHubForge::new(&args.repo);
        let plan = plan_release(input, &forge)?;
        Ok(apply_release(&mut gateway, &plan, &mut forge)?)
    })();
    render_result(result, format, release_outcome_text)
}

fn release_plan_text(output: &ReleasePlanOutput) -> String {
    format!(
        "branch: {}\nbase: {}\ntag: {}\nhead: {}\nsteps:\n- create_pr\n- merge_pr\n- create_release\n",
        output.branch,
        output.base,
        output.tag,
        output.head.as_deref().unwrap_or("null")
    )
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
