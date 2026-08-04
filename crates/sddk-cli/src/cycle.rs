//! Cycle and lease commands exposing the local workflow authority.

use std::path::PathBuf;

use clap::{Args, Subcommand, ValueEnum};
use sddk_domain::{ArtifactRef, CycleId, CycleManifest, CyclePath, normalize_scope};
use sddk_engine::{CycleStartInput, Engine, EventContext, GateOutcome, TransitionEvidence};
use sddk_storage::Storage;
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;

use crate::{CliEnvironment, CommandOutput, OutputFormat, render_result};

const CYCLE_START_REQUIREMENTS: [&str; 4] = [
    "project.adopted",
    "project.initialized",
    "worktree.clean",
    "cycle.no_active_conflict",
];

/// Shared runtime resolution inputs for cycle and ledger commands.
#[derive(Debug, Clone, Args)]
pub(crate) struct RuntimeArgs {
    /// Checkout or worktree root.
    #[arg(long)]
    pub(crate) root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    pub(crate) scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    pub(crate) remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    pub(crate) fallback_seed: Option<String>,
}
/// Resolved identity, storage, and engine for one runtime invocation.
pub(crate) struct RuntimeContext {
    pub(crate) root: PathBuf,
    pub(crate) identity: sddk_domain::ResolvedProjectIdentity,
    pub(crate) workspace_id: String,
    pub(crate) engine: Engine,
    pub(crate) storage: Storage,
    pub(crate) artifacts_path: PathBuf,
}

impl RuntimeContext {
    /// Resolves identity and opens the project ledger and workflow engine.
    ///
    /// `generate_seed` permits state-changing commands to mint a fallback UUID
    /// when the repository has no remote and no persisted adoption receipt.
    pub(crate) fn open(
        args: &RuntimeArgs,
        environment: &CliEnvironment,
        generate_seed: bool,
    ) -> anyhow::Result<Self> {
        let root = crate::canonical_root(&args.root)?;
        let remote = crate::resolve_remote(&root, args.remote.clone())?;
        let mut fallback_seed = args.fallback_seed.clone();
        if remote.is_none() && fallback_seed.is_none() {
            fallback_seed = crate::find_persisted_fallback_seed(environment, &root, &args.scope)?;
        }
        if remote.is_none() && fallback_seed.is_none() && generate_seed {
            fallback_seed = Some(Uuid::new_v4().hyphenated().to_string());
        }
        let identity = sddk_domain::resolve_project_identity(
            remote.as_deref(),
            &args.scope,
            fallback_seed.as_deref(),
        )?;
        let canonical_workspace_path = crate::path_string(&root)?;
        let workspace_id =
            crate::stable_workspace_id(&identity.project_id, &canonical_workspace_path);
        let paths = sddk_engine::resolve_xdg_paths(
            &environment.xdg(),
            identity.project_id.as_str(),
            &workspace_id,
        )?;
        let storage = Storage::open(&paths.ledger)?;
        let workflow = sddk_engine::load_workflow_path(root.join(crate::WORKFLOW_MANIFEST))?;
        let engine = Engine::new(workflow, Storage::open(&paths.ledger)?)?;
        Ok(Self {
            root,
            identity,
            workspace_id,
            engine,
            storage,
            artifacts_path: paths.artifacts,
        })
    }
}
#[derive(Debug, Subcommand)]
pub(crate) enum CycleCommand {
    /// Create a cycle through the declared `cycle.start` transition.
    Start(CycleStartArgs),
    /// Show the current cycle snapshot and lease.
    Status(CycleStatusArgs),
    /// Apply one declared transition with caller evidence.
    Transition(CycleTransitionArgs),
    /// Restore a missing cycle snapshot from its ledger events.
    Rebuild(CycleRebuildArgs),
    /// Acquire, release, or inspect the exclusive cycle lease.
    #[command(subcommand)]
    Lock(CycleLockCommand),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleStartArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Display name used to derive the stable cycle identifier.
    #[arg(long)]
    pub(crate) name: String,
    /// Workflow path applied to the cycle.
    #[arg(long, value_enum, default_value_t = CyclePathArg::AFull)]
    pub(crate) path: CyclePathArg,
    /// Git branch associated with the cycle.
    #[arg(long)]
    pub(crate) branch: Option<String>,
    /// Base commit SHA.
    #[arg(long)]
    pub(crate) base: Option<String>,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    pub(crate) actor: Option<String>,
    /// Acquire the exclusive cycle lease for this owner after creation.
    #[arg(long)]
    pub(crate) lease_owner: Option<String>,
    /// Lease duration in milliseconds when acquiring a lease.
    #[arg(long, default_value_t = 3_600_000)]
    pub(crate) lease_ms: i64,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleStatusArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleTransitionArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Declared transition identifier, for example `phase.build.complete`.
    #[arg(long)]
    pub(crate) transition: String,
    /// Satisfied non-artifact requirement.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) requirement: Vec<String>,
    /// Passed gate outcome.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) gate_pass: Vec<String>,
    /// Failed gate outcome.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) gate_fail: Vec<String>,
    /// Produced artifact as `kind=path`.
    #[arg(long, action = clap::ArgAction::Append)]
    pub(crate) artifact: Vec<String>,
    /// Lease owner required by the fencing check.
    #[arg(long)]
    pub(crate) lease_owner: Option<String>,
    /// Fencing token required by the fencing check.
    #[arg(long)]
    pub(crate) fencing_token: Option<i64>,
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

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleRebuildArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Subcommand)]
pub(crate) enum CycleLockCommand {
    /// Acquire an absent or expired lease, bumping the fencing token.
    Acquire(CycleLockAcquireArgs),
    /// Release the lease only when owner and fencing token match.
    Release(CycleLockReleaseArgs),
    /// Show the current lease.
    Status(CycleLockStatusArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleLockAcquireArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Lease owner.
    #[arg(long)]
    pub(crate) owner: String,
    /// Lease duration in milliseconds.
    #[arg(long, default_value_t = 3_600_000)]
    pub(crate) lease_ms: i64,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    pub(crate) timestamp: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleLockReleaseArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Lease owner.
    #[arg(long)]
    pub(crate) owner: String,
    /// Fencing token issued at acquisition.
    #[arg(long)]
    pub(crate) fencing_token: i64,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CycleLockStatusArgs {
    #[command(flatten)]
    pub(crate) runtime: RuntimeArgs,
    /// Cycle identifier.
    #[arg(long)]
    pub(crate) cycle: String,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum CyclePathArg {
    AMin,
    ALite,
    AFull,
    BDirect,
}

impl From<CyclePathArg> for CyclePath {
    fn from(value: CyclePathArg) -> Self {
        match value {
            CyclePathArg::AMin => CyclePath::AMin,
            CyclePathArg::ALite => CyclePath::ALite,
            CyclePathArg::AFull => CyclePath::AFull,
            CyclePathArg::BDirect => CyclePath::BDirect,
        }
    }
}

pub(crate) fn run_cycle(command: CycleCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        CycleCommand::Start(args) => run_cycle_start(args, environment),
        CycleCommand::Status(args) => run_cycle_status(args, environment),
        CycleCommand::Transition(args) => run_cycle_transition(args, environment),
        CycleCommand::Rebuild(args) => run_cycle_rebuild(args, environment),
        CycleCommand::Lock(command) => run_cycle_lock(command, environment),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleStartOutput {
    cycle_id: String,
    status: String,
    phase: String,
    path: String,
    sequence: i64,
    event_id: String,
    event_hash: String,
    lease: Option<LeaseOutput>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct LeaseOutput {
    owner: String,
    acquired_at_ms: i64,
    expires_at_ms: i64,
    fencing_token: i64,
}

fn run_cycle_start(args: CycleStartArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleStartOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, true)?;
        let scope = normalize_scope(&args.runtime.scope)?;
        let cycle_id = CycleId::from_parts(&context.identity.project_id, &args.name)?;
        let mut manifest = CycleManifest::new(
            context.identity.project_id.to_string(),
            context.workspace_id.clone(),
            cycle_id,
            args.name.clone(),
            args.branch
                .clone()
                .unwrap_or_else(|| format!("feat/{}", args.name)),
            args.base.clone().unwrap_or_else(|| "HEAD".to_owned()),
        );
        manifest.path = args.path.into();
        manifest.remote_url = context.identity.remote_url.clone();
        manifest.scope = Some(scope);
        let input = CycleStartInput {
            manifest,
            requirements: CYCLE_START_REQUIREMENTS
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
        };
        let plan = context.engine.plan_cycle_start(input)?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let command_id = format!("cycle.start-{}", Uuid::new_v4().hyphenated());
        let started = context.engine.apply_cycle_start(
            &plan,
            &event_context(
                &command_id,
                &format!("evt-{}", Uuid::new_v4().hyphenated()),
                &args.actor,
                environment,
                &timestamp,
            ),
        )?;
        let lease = match &args.lease_owner {
            Some(owner) => {
                let now_ms = timestamp_ms(args.timestamp.as_deref())?;
                Some(context.storage.acquire_cycle_lease(
                    &started.manifest.cycle_id,
                    owner,
                    now_ms,
                    now_ms + args.lease_ms,
                )?)
            }
            None => None,
        };
        Ok(CycleStartOutput {
            cycle_id: started.manifest.cycle_id,
            status: wire(&started.manifest.status),
            phase: wire(&started.manifest.phase),
            path: cycle_path_text(&started.manifest.path),
            sequence: started.event.sequence,
            event_id: started.event.event_id,
            event_hash: started.event.event_hash,
            lease: lease.map(Into::into),
        })
    })();
    render_result(result, format, cycle_start_text)
}

fn run_cycle_status(args: CycleStatusArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleStatusOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let record = context.storage.get_cycle(&args.cycle)?;
        let lease = context.storage.get_cycle_lease(&args.cycle).ok();
        Ok(CycleStatusOutput {
            cycle_id: record.manifest.cycle_id,
            status: wire(&record.manifest.status),
            phase: wire(&record.manifest.phase),
            path: cycle_path_text(&record.manifest.path),
            updated_at: record.updated_at,
            artifacts: record.manifest.artifacts.len(),
            lease: lease.map(Into::into),
        })
    })();
    render_result(result, format, cycle_status_text)
}

fn run_cycle_transition(args: CycleTransitionArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleTransitionOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        match context.storage.get_cycle_lease(&args.cycle) {
            Ok(_) => {
                let owner = args.lease_owner.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("cycle {} is leased; --lease-owner is required", args.cycle)
                })?;
                let token = args.fencing_token.ok_or_else(|| {
                    anyhow::anyhow!(
                        "cycle {} is leased; --fencing-token is required",
                        args.cycle
                    )
                })?;
                context
                    .engine
                    .require_lease_fence(&args.cycle, owner, token)?;
            }
            Err(sddk_storage::StorageError::NotFound { .. }) => {
                if args.lease_owner.is_some() || args.fencing_token.is_some() {
                    anyhow::bail!(
                        "cycle {} has no lease; fencing arguments are not applicable",
                        args.cycle
                    );
                }
            }
            Err(error) => return Err(error.into()),
        }
        let mut evidence = TransitionEvidence {
            requirements: args
                .requirement
                .iter()
                .map(|value| value.to_owned())
                .collect(),
            ..TransitionEvidence::default()
        };
        for artifact in &args.artifact {
            let (kind, path) = split_artifact(artifact)?;
            evidence
                .artifacts
                .insert(kind.clone(), ArtifactRef::new(kind, path));
        }
        for gate in &args.gate_pass {
            evidence.gates.insert(gate.clone(), GateOutcome::Passed);
        }
        for gate in &args.gate_fail {
            evidence
                .gates
                .insert(gate.clone(), GateOutcome::Failed { reason: None });
        }
        let plan = context
            .engine
            .plan_transition(&args.cycle, &args.transition, evidence)?;
        let timestamp = args.timestamp.clone().unwrap_or_else(default_timestamp);
        let command_id = format!("cycle.transition-{}", Uuid::new_v4().hyphenated());
        let applied = context.engine.apply_transition(
            &plan,
            &event_context(
                &command_id,
                &format!("evt-{}", Uuid::new_v4().hyphenated()),
                &args.actor,
                environment,
                &timestamp,
            ),
        )?;
        Ok(CycleTransitionOutput {
            cycle_id: applied.manifest.cycle_id,
            transition_id: applied.transition_id,
            outcome: transition_outcome_text(applied.outcome),
            status: wire(&applied.manifest.status),
            phase: wire(&applied.manifest.phase),
            sequence: applied.event.sequence,
            event_id: applied.event.event_id,
            event_hash: applied.event.event_hash,
        })
    })();
    render_result(result, format, cycle_transition_text)
}

fn run_cycle_rebuild(args: CycleRebuildArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleRebuildOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        let rebuilt = context.engine.rebuild_cycle(&args.cycle)?;
        Ok(CycleRebuildOutput {
            cycle_id: rebuilt.manifest.cycle_id,
            status: wire(&rebuilt.manifest.status),
            phase: wire(&rebuilt.manifest.phase),
            sequence: rebuilt.sequence,
            restored: rebuilt.restored,
        })
    })();
    render_result(result, format, cycle_rebuild_text)
}

fn run_cycle_lock(command: CycleLockCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        CycleLockCommand::Acquire(args) => run_cycle_lock_acquire(args, environment),
        CycleLockCommand::Release(args) => run_cycle_lock_release(args, environment),
        CycleLockCommand::Status(args) => run_cycle_lock_status(args, environment),
    }
}

fn run_cycle_lock_acquire(
    args: CycleLockAcquireArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<LeaseOutput> {
        let mut context = RuntimeContext::open(&args.runtime, environment, false)?;
        let now_ms = timestamp_ms(args.timestamp.as_deref())?;
        let lease = context.storage.acquire_cycle_lease(
            &args.cycle,
            &args.owner,
            now_ms,
            now_ms + args.lease_ms,
        )?;
        Ok(lease.into())
    })();
    render_result(result, format, lease_text)
}

fn run_cycle_lock_release(
    args: CycleLockReleaseArgs,
    environment: &CliEnvironment,
) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<CycleLockReleaseOutput> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        let released =
            context
                .storage
                .release_cycle_lease(&args.cycle, &args.owner, args.fencing_token)?;
        Ok(CycleLockReleaseOutput { released })
    })();
    render_result(result, format, cycle_lock_release_text)
}

fn run_cycle_lock_status(args: CycleLockStatusArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Option<LeaseOutput>> {
        let context = RuntimeContext::open(&args.runtime, environment, false)?;
        Ok(context
            .storage
            .get_cycle_lease(&args.cycle)
            .ok()
            .map(Into::into))
    })();
    render_result(result, format, lease_option_text)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleStatusOutput {
    cycle_id: String,
    status: String,
    phase: String,
    path: String,
    updated_at: String,
    artifacts: usize,
    lease: Option<LeaseOutput>,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleTransitionOutput {
    cycle_id: String,
    transition_id: String,
    outcome: String,
    status: String,
    phase: String,
    sequence: i64,
    event_id: String,
    event_hash: String,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleRebuildOutput {
    cycle_id: String,
    status: String,
    phase: String,
    sequence: i64,
    restored: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct CycleLockReleaseOutput {
    released: bool,
}

impl From<sddk_storage::CycleLease> for LeaseOutput {
    fn from(value: sddk_storage::CycleLease) -> Self {
        Self {
            owner: value.owner,
            acquired_at_ms: value.acquired_at_ms,
            expires_at_ms: value.expires_at_ms,
            fencing_token: value.fencing_token,
        }
    }
}

fn cycle_start_text(output: &CycleStartOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nphase: {}\npath: {}\nsequence: {}\nevent_id: {}\nevent_hash: {}\n{}",
        output.cycle_id,
        output.status,
        output.phase,
        output.path,
        output.sequence,
        output.event_id,
        output.event_hash,
        output
            .lease
            .as_ref()
            .map(lease_text)
            .unwrap_or_else(|| "lease: none\n".to_owned())
    )
}

fn cycle_status_text(output: &CycleStatusOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nphase: {}\npath: {}\nupdated_at: {}\nartifacts: {}\n{}",
        output.cycle_id,
        output.status,
        output.phase,
        output.path,
        output.updated_at,
        output.artifacts,
        output
            .lease
            .as_ref()
            .map(lease_text)
            .unwrap_or_else(|| "lease: none\n".to_owned())
    )
}

fn cycle_transition_text(output: &CycleTransitionOutput) -> String {
    format!(
        "cycle_id: {}\ntransition_id: {}\noutcome: {}\nstatus: {}\nphase: {}\nsequence: {}\nevent_id: {}\nevent_hash: {}\n",
        output.cycle_id,
        output.transition_id,
        output.outcome,
        output.status,
        output.phase,
        output.sequence,
        output.event_id,
        output.event_hash
    )
}

fn cycle_rebuild_text(output: &CycleRebuildOutput) -> String {
    format!(
        "cycle_id: {}\nstatus: {}\nphase: {}\nsequence: {}\nrestored: {}\n",
        output.cycle_id, output.status, output.phase, output.sequence, output.restored
    )
}

fn cycle_lock_release_text(output: &CycleLockReleaseOutput) -> String {
    format!("released: {}\n", output.released)
}

fn lease_text(lease: &LeaseOutput) -> String {
    format!(
        "lease: owner={} fencing_token={} acquired_at_ms={} expires_at_ms={}\n",
        lease.owner, lease.fencing_token, lease.acquired_at_ms, lease.expires_at_ms
    )
}

fn lease_option_text(lease: &Option<LeaseOutput>) -> String {
    match lease {
        Some(lease) => lease_text(lease),
        None => "lease: none\n".to_owned(),
    }
}

fn cycle_path_text(path: &CyclePath) -> String {
    match path {
        CyclePath::AMin => "A-min",
        CyclePath::ALite => "A-lite",
        CyclePath::AFull => "A-full",
        CyclePath::BDirect => "B-direct",
    }
    .to_owned()
}

fn transition_outcome_text(outcome: sddk_engine::TransitionOutcome) -> String {
    match outcome {
        sddk_engine::TransitionOutcome::Succeeded => "succeeded",
        sddk_engine::TransitionOutcome::Failed => "failed",
    }
    .to_owned()
}

fn split_artifact(value: &str) -> anyhow::Result<(String, String)> {
    let (kind, path) = value
        .split_once('=')
        .ok_or_else(|| anyhow::anyhow!("artifact must use kind=path: {value}"))?;
    if kind.is_empty() || path.is_empty() {
        anyhow::bail!("artifact must use kind=path: {value}");
    }
    Ok((kind.to_owned(), path.to_owned()))
}

fn event_context(
    command_id: &str,
    event_id: &str,
    explicit_actor: &Option<String>,
    environment: &CliEnvironment,
    occurred_at: &str,
) -> EventContext {
    EventContext {
        command_id: command_id.to_owned(),
        frame_id: format!("frame:{command_id}"),
        event_id: event_id.to_owned(),
        actor: explicit_actor
            .clone()
            .or_else(|| environment.sddk_actor.clone())
            .or_else(|| environment.user.clone())
            .unwrap_or_else(|| "sddk-cli".into()),
        occurred_at: occurred_at.to_owned(),
    }
}

fn wire<T: serde::Serialize>(value: &T) -> String {
    serde_json::to_value(value)
        .expect("workflow enums are serializable")
        .as_str()
        .expect("workflow enums serialize as strings")
        .to_owned()
}

fn default_timestamp() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .expect("RFC 3339 formatting cannot fail")
}

fn timestamp_ms(timestamp: Option<&str>) -> anyhow::Result<i64> {
    match timestamp {
        Some(value) => Ok(OffsetDateTime::parse(value, &Rfc3339)?.unix_timestamp() * 1000),
        None => Ok(OffsetDateTime::now_utc().unix_timestamp() * 1000),
    }
}
