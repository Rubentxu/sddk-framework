//! Metrics capture, aggregation, F3 tuning, and analytics commands.

use std::collections::HashMap;
use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::{F3Tuning, MetricsAggregate, MetricsRecord};
use time::OffsetDateTime;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

/// Window selector for metrics aggregation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum MetricsWindow {
    #[value(name = "7d")]
    SevenDays,
    #[value(name = "30d")]
    ThirtyDays,
}

impl MetricsWindow {
    pub(crate) fn days(self) -> u16 {
        match self {
            Self::SevenDays => 7,
            Self::ThirtyDays => 30,
        }
    }
}

#[derive(Debug, Subcommand)]
pub(crate) enum MetricsCommand {
    /// Record metrics for a closed cycle (Levels A-E + L1-L6 costs).
    Record(MetricsRecordArgs),
    /// Compute rolling aggregates over a window.
    Aggregate(MetricsAggregateArgs),
    /// Emit the F3 self-tuning recommendation block.
    Tuning(MetricsTuningArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MetricsRecordArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier to record metrics for.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Verification verdict: PASS | PW | FAIL.
    #[arg(long, default_value = "PASS")]
    pub(crate) verdict: String,
    /// Whether the change was merged to main.
    #[arg(long)]
    pub(crate) merged: bool,
    /// First verification attempt passed.
    #[arg(long)]
    pub(crate) first_pass: bool,
    /// Correction cycles count.
    #[arg(long, default_value_t = 0)]
    pub(crate) corrections: u8,
    /// Context quality at triage (C0..C3).
    #[arg(long, default_value = "C2")]
    pub(crate) context_quality: String,
    /// Workflow path taken (b-direct | a-min | a-lite | a-full).
    #[arg(long)]
    pub(crate) path: Option<String>,
    /// Semantic version tag when released.
    #[arg(long)]
    pub(crate) tag: Option<String>,
    /// Estimated cost in USD.
    #[arg(long)]
    pub(crate) cost: Option<f64>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MetricsAggregateArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Aggregation window.
    #[arg(long, value_enum, default_value_t = MetricsWindow::SevenDays)]
    pub(crate) window: MetricsWindow,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct MetricsTuningArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Aggregation window for the tuning signals.
    #[arg(long, value_enum, default_value_t = MetricsWindow::SevenDays)]
    pub(crate) window: MetricsWindow,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Metrics store file names.
const METRICS_JSONL: &str = "metrics.jsonl";
const AGGREGATE_JSON: &str = "aggregate.json";

/// Resolve the project metrics directory from a runtime context.
///
/// The metrics directory lives next to the artifacts directory under the
/// project data root: `<data>/sddk/projects/<project_id>/metrics`.
fn metrics_dir(context: &RuntimeContext) -> anyhow::Result<PathBuf> {
    let artifacts = &context.artifacts_path;
    let project_data = artifacts
        .parent()
        .ok_or_else(|| anyhow::anyhow!("artifacts path has no parent"))?;
    Ok(project_data.join("metrics"))
}

/// Append one metrics record to `metrics.jsonl`.
pub(crate) fn append_record(
    context: &RuntimeContext,
    record: &MetricsRecord,
) -> anyhow::Result<PathBuf> {
    let dir = metrics_dir(context)?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(METRICS_JSONL);
    let mut line = serde_json::to_string(record)?;
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    use std::io::Write;
    file.write_all(line.as_bytes())?;
    Ok(path)
}

/// Read all records from `metrics.jsonl`, skipping corrupt lines.
pub(crate) fn read_records(context: &RuntimeContext) -> anyhow::Result<Vec<MetricsRecord>> {
    let dir = metrics_dir(context)?;
    let path = dir.join(METRICS_JSONL);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = std::fs::read_to_string(&path)?;
    let mut records = Vec::new();
    for (index, line) in content.lines().enumerate() {
        match serde_json::from_str::<MetricsRecord>(line) {
            Ok(record) => records.push(record),
            Err(_) => eprintln!("warning: skipping corrupt metrics line {}", index + 1),
        }
    }
    Ok(records)
}

/// Filter records to a window (by recorded_at).
pub(crate) fn window_records(records: Vec<MetricsRecord>, window_days: u16) -> Vec<MetricsRecord> {
    let cutoff = OffsetDateTime::now_utc() - time::Duration::days(window_days as i64);
    records
        .into_iter()
        .filter(|record| {
            OffsetDateTime::parse(
                &record.recorded_at,
                &time::format_description::well_known::Rfc3339,
            )
            .map(|when| when >= cutoff)
            .unwrap_or(true)
        })
        .collect()
}

fn median(values: &mut [f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let middle = values.len() / 2;
    if values.len().is_multiple_of(2) {
        Some((values[middle - 1] + values[middle]) / 2.0)
    } else {
        Some(values[middle])
    }
}

/// Compute the rolling aggregate for a set of records.
pub(crate) fn compute_aggregate(records: &[MetricsRecord], window_days: u16) -> MetricsAggregate {
    let mut aggregate = MetricsAggregate::empty(window_days);
    aggregate.sample_size = records.len() as u32;
    if records.is_empty() {
        return aggregate;
    }
    let passes = records
        .iter()
        .filter(|record| record.first_pass_success)
        .count();
    aggregate.first_pass_success_rate = passes as f64 / records.len() as f64;

    let mut lead_times: Vec<f64> = records
        .iter()
        .filter_map(|record| record.lead_time_hours)
        .collect();
    aggregate.median_lead_time_hours = median(&mut lead_times);

    let mut costs: Vec<f64> = records
        .iter()
        .filter(|record| record.cost_estimate_usd > 0.0)
        .map(|record| record.cost_estimate_usd)
        .collect();
    aggregate.median_cost_usd = median(&mut costs);

    let mut phase_totals: HashMap<String, (u64, u32)> = HashMap::new();
    for record in records {
        for (phase, seconds) in &record.phase_durations_sec {
            let entry = phase_totals.entry(phase.clone()).or_insert((0, 0));
            entry.0 += seconds;
            entry.1 += 1;
        }
        *aggregate
            .path_distribution
            .entry(record.path.clone())
            .or_insert(0) += 1;
        *aggregate
            .verdict_distribution
            .entry(record.verify_verdict.clone())
            .or_insert(0) += 1;
    }
    aggregate.top_bottleneck_phase = phase_totals
        .into_iter()
        .filter(|(_, (_, count))| *count > 0)
        .max_by(|a, b| (a.1.0 as f64 / a.1.1 as f64).total_cmp(&(b.1.0 as f64 / b.1.1 as f64)))
        .map(|(phase, _)| phase);
    aggregate
}

/// Map an aggregate to F3 tuning recommendations (advisory).
pub(crate) fn tuning_from_aggregate(aggregate: &MetricsAggregate) -> F3Tuning {
    let mut tuning = F3Tuning::default();
    if aggregate.sample_size >= 3 {
        if aggregate.first_pass_success_rate > 0.85 {
            tuning.path_bias = Some("A-min".to_owned());
        } else if aggregate.first_pass_success_rate < 0.6 {
            tuning.recommended_deepen.push("spec".to_owned());
            tuning.recommended_deepen.push("verify".to_owned());
        }
    }
    if aggregate.top_bottleneck_phase.as_deref() == Some("apply") {
        tuning.recommended_lens.push("test-quality".to_owned());
    }
    tuning
}

/// Write the aggregate to `aggregate.json`.
fn write_aggregate(
    context: &RuntimeContext,
    aggregate: &MetricsAggregate,
) -> anyhow::Result<PathBuf> {
    let dir = metrics_dir(context)?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(AGGREGATE_JSON);
    let content = serde_json::to_string_pretty(aggregate)?;
    std::fs::write(&path, content)?;
    Ok(path)
}

/// Read the persisted aggregate if present.
fn read_aggregate(context: &RuntimeContext) -> anyhow::Result<Option<MetricsAggregate>> {
    let dir = metrics_dir(context)?;
    let path = dir.join(AGGREGATE_JSON);
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&path)?;
    Ok(serde_json::from_str(&content).ok())
}

pub(crate) fn run_metrics(command: MetricsCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        MetricsCommand::Record(args) => run_metrics_record(args, environment),
        MetricsCommand::Aggregate(args) => run_metrics_aggregate(args, environment),
        MetricsCommand::Tuning(args) => run_metrics_tuning(args, environment),
    }
}

fn run_metrics_record(args: MetricsRecordArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<MetricsRecord> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let now = OffsetDateTime::now_utc();
        let recorded_at = now.format(&time::format_description::well_known::Rfc3339)?;
        let record = MetricsRecord {
            cycle_id: args.cycle.clone(),
            path: args.path.clone().unwrap_or_else(|| "unknown".to_owned()),
            context_quality: args.context_quality.clone(),
            phase_durations_sec: HashMap::new(),
            coherence_scores: Vec::new(),
            correction_cycles: args.corrections,
            tokens_used: 0,
            cost_estimate_usd: args.cost.unwrap_or(0.0),
            first_pass_success: args.first_pass,
            verify_verdict: args.verdict.clone(),
            merged_to_main: args.merged,
            tag_version: args.tag.clone(),
            lead_time_hours: None,
            teleological_coherence_pct: None,
            costs: HashMap::new(),
            recorded_at,
        };
        let path = append_record(&context, &record)?;
        eprintln!("metrics appended: {}", path.display());
        Ok(record)
    })();
    render_result(result, format, metrics_record_text)
}

fn run_metrics_aggregate(
    args: MetricsAggregateArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<MetricsAggregate> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let records = read_records(&context)?;
        let records = window_records(records, args.window.days());
        let aggregate = compute_aggregate(&records, args.window.days());
        let path = write_aggregate(&context, &aggregate)?;
        eprintln!("aggregate written: {}", path.display());
        Ok(aggregate)
    })();
    render_result(result, format, aggregate_text)
}

fn run_metrics_tuning(args: MetricsTuningArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<F3Tuning> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let aggregate = match read_aggregate(&context)? {
            Some(aggregate) => aggregate,
            None => {
                let records = read_records(&context)?;
                let records = window_records(records, args.window.days());
                compute_aggregate(&records, args.window.days())
            }
        };
        Ok(tuning_from_aggregate(&aggregate))
    })();
    render_result(result, format, tuning_text)
}

fn metrics_record_text(record: &MetricsRecord) -> String {
    format!(
        "cycle: {}\nverdict: {}\nfirst_pass: {}\nmerged: {}\ncorrections: {}\ncost: {}\n",
        record.cycle_id,
        record.verify_verdict,
        record.first_pass_success,
        record.merged_to_main,
        record.correction_cycles,
        record.cost_estimate_usd
    )
}

pub(crate) fn aggregate_text(aggregate: &MetricsAggregate) -> String {
    format!(
        "window: {}d\nsample: {}\nfirst_pass_rate: {:.2}\nmedian_lead_time_hours: {}\nmedian_cost_usd: {}\ntop_bottleneck_phase: {}\npaths: {:?}\nverdicts: {:?}\n",
        aggregate.window_days,
        aggregate.sample_size,
        aggregate.first_pass_success_rate,
        aggregate
            .median_lead_time_hours
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_owned()),
        aggregate
            .median_cost_usd
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_owned()),
        aggregate.top_bottleneck_phase.as_deref().unwrap_or("n/a"),
        aggregate.path_distribution,
        aggregate.verdict_distribution
    )
}

fn tuning_text(tuning: &F3Tuning) -> String {
    let mut text = String::new();
    if let Some(path_bias) = &tuning.path_bias {
        text.push_str(&format!("path_bias: {path_bias}\n"));
    }
    if let Some(threshold) = tuning.circuit_threshold {
        text.push_str(&format!("circuit_threshold: {threshold}\n"));
    }
    if let Some(attempts) = tuning.per_task_max_attempts {
        text.push_str(&format!("per_task_max_attempts: {attempts}\n"));
    }
    for phase in &tuning.recommended_skip {
        text.push_str(&format!("recommend_skip: {phase}\n"));
    }
    for phase in &tuning.recommended_deepen {
        text.push_str(&format!("recommend_deepen: {phase}\n"));
    }
    for lens in &tuning.recommended_lens {
        text.push_str(&format!("recommend_lens: {lens}\n"));
    }
    if text.is_empty() {
        text.push_str("no tuning recommendations (sample too small or steady state)\n");
    }
    text
}
