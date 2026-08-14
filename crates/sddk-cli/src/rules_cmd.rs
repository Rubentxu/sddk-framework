//! `sddk rules check` — Architecture-rule registry observational check (SDDK2-003 rev-2).

use crate::{CliEnvironment, CommandOutput, failure};
use clap::{Args, Subcommand};
use sddk_domain::{ARCHITECTURE_RULES_SCHEMA_VERSION, RuleEvaluation, RuleRegistry};
use sddk_engine::rules::{BaselineConsumer, EVALUATOR_VERSION, evaluate_all};
use serde::Serialize;
use std::path::PathBuf;
use time::OffsetDateTime;

#[derive(Debug, Subcommand)]
pub(crate) enum RulesCommand {
    /// Evaluate every registered rule against the baseline JSON (SDDK2-003 rev-2).
    Check(RulesCheckArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct RulesCheckArgs {
    /// Checkout or worktree root used for project capability resolution.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Monorepo scope used by adoption identity resolution.
    #[arg(long, default_value = ".")]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

pub(crate) fn run_rules(command: RulesCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        RulesCommand::Check(args) => run_rules_check(args, environment),
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_owned())
}

#[derive(Serialize)]
struct EvaluationOutput {
    schema_version: &'static str,
    evaluator_version: &'static str,
    applicable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capability_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    capability_authority: Option<crate::knowledge_ingest::Authority>,
    capability_status: String,
    receipt_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_schema_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    baseline_sha256: Option<String>,
    evaluated_at: String,
    evaluations: Vec<RuleEvaluation>,
}

fn run_rules_check(args: RulesCheckArgs, environment: &CliEnvironment) -> CommandOutput {
    let context = match crate::knowledge_cmd::resolve_managed_knowledge(
        &args.root,
        &args.scope,
        args.remote.clone(),
        args.fallback_seed.clone(),
        environment,
    ) {
        Ok(context) => context,
        Err(error) => {
            return write_output(
                &args,
                EvaluationOutput {
                    schema_version: ARCHITECTURE_RULES_SCHEMA_VERSION,
                    evaluator_version: EVALUATOR_VERSION,
                    applicable: false,
                    reason: Some(error.to_string()),
                    capability_id: None,
                    capability_authority: None,
                    capability_status: "not_applicable".to_owned(),
                    receipt_id: "kr-unadopted".to_owned(),
                    baseline_schema_version: None,
                    baseline_sha256: None,
                    evaluated_at: now_rfc3339(),
                    evaluations: Vec::new(),
                },
            );
        }
    };
    let capability = match crate::knowledge_ingest::resolve_architecture_capability(&context) {
        Ok(capability) => capability,
        Err(error) => return failure(error.to_string()),
    };
    let (catalog, baseline_path) = match (&capability.catalog, &capability.baseline) {
        (Some(catalog), Some(baseline)) => (catalog.clone(), baseline.clone()),
        _ => {
            return write_output(
                &args,
                EvaluationOutput {
                    schema_version: ARCHITECTURE_RULES_SCHEMA_VERSION,
                    evaluator_version: EVALUATOR_VERSION,
                    applicable: false,
                    reason: Some(capability.reason),
                    capability_id: capability.capability_id,
                    capability_authority: capability.authority,
                    capability_status: capability.status,
                    receipt_id: capability.receipt_id,
                    baseline_schema_version: None,
                    baseline_sha256: None,
                    evaluated_at: now_rfc3339(),
                    evaluations: Vec::new(),
                },
            );
        }
    };
    let yaml = match std::fs::read_to_string(&catalog) {
        Ok(y) => y,
        Err(e) => return failure(format!("failed to read {}: {e}", catalog.display())),
    };
    let registry = match RuleRegistry::from_yaml_str(&yaml) {
        Ok(r) => r,
        Err(e) => return failure(e.to_string()),
    };
    let consumer = match BaselineConsumer::new(&baseline_path, &["1.0.0"]) {
        Ok(c) => c,
        Err(e) => return failure(e.to_string()),
    };
    let baseline = match consumer.load() {
        Ok(b) => b,
        Err(e) => return failure(e.to_string()),
    };
    let evaluated_at = now_rfc3339();
    let evaluations = evaluate_all(&registry, &baseline, &evaluated_at);
    let output = EvaluationOutput {
        schema_version: ARCHITECTURE_RULES_SCHEMA_VERSION,
        evaluator_version: EVALUATOR_VERSION,
        applicable: true,
        reason: None,
        capability_id: capability.capability_id,
        capability_authority: capability.authority,
        capability_status: capability.status,
        receipt_id: capability.receipt_id,
        baseline_schema_version: Some(baseline.ref_.schema_version.clone()),
        baseline_sha256: Some(baseline.ref_.sha256.clone()),
        evaluated_at,
        evaluations,
    };
    write_output(&args, output)
}

fn write_output(args: &RulesCheckArgs, output: EvaluationOutput) -> CommandOutput {
    let body = match serde_json::to_string_pretty(&output) {
        Ok(s) => format!("{s}\n"),
        Err(e) => return failure(format!("failed to serialize evaluations: {e}")),
    };
    let mut cmd = CommandOutput {
        status: 0,
        stdout: body.clone(),
        stderr: String::new(),
    };
    if let Some(out_path) = &args.out {
        if let Err(e) = std::fs::write(out_path, &body) {
            return failure(format!("failed to write {}: {e}", out_path.display()));
        }
        cmd.stdout = format!(
            "wrote {} evaluations to {}\n",
            output.evaluations.len(),
            out_path.display()
        );
    }
    cmd
}
