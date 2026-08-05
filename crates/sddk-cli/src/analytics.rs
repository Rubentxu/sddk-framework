//! Analytics commands: report, trends, and bottleneck from aggregate metrics.

use clap::{Args, Subcommand};
use sddk_domain::MetricsAggregate;
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

use crate::metrics::{MetricsWindow, compute_aggregate, read_records, window_records};

#[derive(Debug, Subcommand)]
pub(crate) enum AnalyticsCommand {
    /// Show the current rolling aggregate report.
    Report(AnalyticsWindowArgs),
    /// Show per-window trends (7d vs 30d).
    Trends(AnalyticsWindowArgs),
    /// Show the top bottleneck phase and its cost impact.
    Bottleneck(AnalyticsWindowArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct AnalyticsWindowArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Aggregation window.
    #[arg(long, value_enum, default_value_t = MetricsWindow::SevenDays)]
    pub(crate) window: MetricsWindow,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct TrendsOutput {
    window_7d: MetricsAggregate,
    window_30d: MetricsAggregate,
}

pub(crate) fn run_analytics(
    command: AnalyticsCommand,
    environment: &CliEnvironment,
) -> CommandOutput {
    match command {
        AnalyticsCommand::Report(args) => run_analytics_report(args, environment),
        AnalyticsCommand::Trends(args) => run_analytics_trends(args, environment),
        AnalyticsCommand::Bottleneck(args) => run_analytics_bottleneck(args, environment),
    }
}

fn aggregate_for(
    context: &RuntimeContext,
    window: MetricsWindow,
) -> anyhow::Result<MetricsAggregate> {
    let records = read_records(context)?;
    let records = window_records(records, window.days());
    Ok(compute_aggregate(&records, window.days()))
}

fn run_analytics_report(args: AnalyticsWindowArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<MetricsAggregate> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        aggregate_for(&context, args.window)
    })();
    render_result(result, format, crate::metrics::aggregate_text)
}

fn run_analytics_trends(args: AnalyticsWindowArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<TrendsOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let window_7d = aggregate_for(&context, MetricsWindow::SevenDays)?;
        let window_30d = aggregate_for(&context, MetricsWindow::ThirtyDays)?;
        Ok(TrendsOutput {
            window_7d,
            window_30d,
        })
    })();
    render_result(result, format, trends_text)
}

fn run_analytics_bottleneck(
    args: AnalyticsWindowArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<MetricsAggregate> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        aggregate_for(&context, args.window)
    })();
    render_result(result, format, bottleneck_text)
}

fn trends_text(output: &TrendsOutput) -> String {
    format!(
        "trends 7d -> 30d:\n  first_pass_rate: {:.2} -> {:.2}\n  median_lead_time_hours: {} -> {}\n  sample: {} -> {}\n",
        output.window_7d.first_pass_success_rate,
        output.window_30d.first_pass_success_rate,
        output
            .window_7d
            .median_lead_time_hours
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_owned()),
        output
            .window_30d
            .median_lead_time_hours
            .map(|value| format!("{value:.2}"))
            .unwrap_or_else(|| "n/a".to_owned()),
        output.window_7d.sample_size,
        output.window_30d.sample_size,
    )
}

fn bottleneck_text(output: &MetricsAggregate) -> String {
    let mut text = format!("sample: {}\n", output.sample_size);
    if let Some(bottleneck) = &output.top_bottleneck_phase {
        text.push_str(&format!("top_bottleneck_phase: {bottleneck}\n"));
    } else {
        text.push_str("top_bottleneck_phase: n/a (no phase durations recorded)\n");
    }
    text
}
