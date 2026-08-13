//! `sddk rules check` — Architecture-rule registry observational check (SDDK2-003 rev-2).

use std::path::PathBuf;
use clap::{Args, Subcommand};
use sddk_domain::{RuleRegistry, ARCHITECTURE_RULES_SCHEMA_VERSION};
use sddk_engine::rules::{evaluate_all, BaselineConsumer, EVALUATOR_VERSION};
use serde::Serialize;
use time::OffsetDateTime;
use crate::{failure, CommandOutput};

const CATALOG_DEFAULT: &str = "docs/sddk-2.0-architecture-consolidation/data/architecture-rules.yaml";
const BASELINE_DEFAULT: &str = "data/projects/p-52b95ef55999f9de/cycle-artifacts/p-52b95ef55999f9de/sddk-2-0-phase0-baseline/baseline-dependency-entropy.json";

#[derive(Debug, Subcommand)]
pub(crate) enum RulesCommand {
    /// Evaluate every registered rule against the baseline JSON (SDDK2-003 rev-2).
    Check(RulesCheckArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct RulesCheckArgs {
    #[arg(long, default_value = CATALOG_DEFAULT)]
    pub(crate) catalog: PathBuf,
    #[arg(long, default_value = BASELINE_DEFAULT)]
    pub(crate) baseline: PathBuf,
    #[arg(long)]
    pub(crate) out: Option<PathBuf>,
}

pub(crate) fn run_rules(command: RulesCommand) -> CommandOutput {
    match command {
        RulesCommand::Check(args) => run_rules_check(args),
    }
}

fn now_rfc3339() -> String {
    OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_owned())
}

#[derive(Serialize)]
struct EvaluationOutput<'a> {
    schema_version: &'static str,
    evaluator_version: &'static str,
    baseline_schema_version: String,
    baseline_sha256: String,
    evaluated_at: String,
    evaluations: Vec<&'a sddk_domain::RuleEvaluation>,
}

fn run_rules_check(args: RulesCheckArgs) -> CommandOutput {
    let yaml = match std::fs::read_to_string(&args.catalog) {
        Ok(y) => y,
        Err(e) => return failure(format!("failed to read {}: {e}", args.catalog.display())),
    };
    let registry = match RuleRegistry::from_yaml_str(&yaml) {
        Ok(r) => r,
        Err(e) => return failure(e.to_string()),
    };
    let consumer = match BaselineConsumer::new(&args.baseline, &["1.0.0"]) {
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
        baseline_schema_version: baseline.ref_.schema_version.clone(),
        baseline_sha256: baseline.ref_.sha256.clone(),
        evaluated_at,
        evaluations: evaluations.iter().collect(),
    };
    let body = match serde_json::to_string_pretty(&output) {
        Ok(s) => format!("{s}\n"),
        Err(e) => return failure(format!("failed to serialize evaluations: {e}")),
    };
    let mut cmd = CommandOutput { status: 0, stdout: body.clone(), stderr: String::new() };
    if let Some(out_path) = args.out {
        if let Err(e) = std::fs::write(&out_path, &body) {
            return failure(format!("failed to write {}: {e}", out_path.display()));
        }
        cmd.stdout = format!("wrote {} evaluations to {}\n", evaluations.len(), out_path.display());
    }
    cmd
}
