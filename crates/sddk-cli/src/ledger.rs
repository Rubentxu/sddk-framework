//! Ledger verification and event inspection commands.

use clap::{Args, Subcommand};
use serde::Serialize;

use crate::{
    CliEnvironment, CommandOutput, OutputFormat, RuntimeArgs, RuntimeContext, render_result,
};

#[derive(Debug, Subcommand)]
pub(crate) enum LedgerCommand {
    /// Verify sequence continuity, predecessor links, and event hashes.
    Verify(LedgerVerifyArgs),
    /// List ledger events, optionally scoped to one command frame.
    Events(LedgerEventsArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct LedgerVerifyArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct LedgerEventsArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Restrict to events sharing one command frame.
    #[arg(long)]
    pub(crate) frame: Option<String>,
    /// Maximum events to list.
    #[arg(long, default_value_t = 50)]
    pub(crate) limit: usize,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_ledger(command: LedgerCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        LedgerCommand::Verify(args) => run_ledger_verify(args, environment),
        LedgerCommand::Events(args) => run_ledger_events(args, environment),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct LedgerVerifyOutput {
    event_count: usize,
    last_hash: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct LedgerEventOutput {
    sequence: i64,
    event_id: String,
    frame_id: String,
    command_id: String,
    event_type: String,
    cycle_id: Option<String>,
    actor: String,
    occurred_at: String,
}

fn run_ledger_verify(args: LedgerVerifyArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<LedgerVerifyOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let verified = context.storage.verify_ledger()?;
        Ok(LedgerVerifyOutput {
            event_count: verified.event_count,
            last_hash: verified.last_hash,
        })
    })();
    render_result(result, format, ledger_verify_text)
}

fn run_ledger_events(args: LedgerEventsArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<LedgerEventOutput>> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let events = match &args.frame {
            Some(frame) => context.storage.list_frame_events(frame)?,
            None => context.storage.list_events()?,
        };
        Ok(events
            .into_iter()
            .rev()
            .take(args.limit)
            .rev()
            .map(|event| LedgerEventOutput {
                sequence: event.sequence,
                event_id: event.event_id,
                frame_id: event.frame_id,
                command_id: event.command_id,
                event_type: event.event_type,
                cycle_id: event.cycle_id,
                actor: event.actor,
                occurred_at: event.occurred_at,
            })
            .collect())
    })();
    render_result(result, format, ledger_events_text)
}

fn ledger_verify_text(output: &LedgerVerifyOutput) -> String {
    format!(
        "event_count: {}\nlast_hash: {}\n",
        output.event_count,
        output.last_hash.as_deref().unwrap_or("null")
    )
}

fn ledger_events_text(events: &Vec<LedgerEventOutput>) -> String {
    if events.is_empty() {
        return "no events\n".to_owned();
    }
    let mut output = String::new();
    for event in events {
        output.push_str(&format!(
            "{} {} {} {} {}\n",
            event.sequence,
            event.event_type,
            event.event_id,
            event.frame_id,
            event.cycle_id.as_deref().unwrap_or("-")
        ));
    }
    output
}
