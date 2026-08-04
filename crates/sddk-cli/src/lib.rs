//! Testable command surface for the SDDK CLI.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

mod artifact;
mod capability;
mod cycle;
mod dev_cmd;
mod docs;
mod git_cmd;
mod inventory;
mod ledger;
mod lint;
mod pack_cmd;
mod permission;
mod release_cmd;
mod result_cmd;
mod vault_cmd;

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;

use artifact::ArtifactCommand;
use capability::CapabilityCommand;
use clap::{Args, Parser, Subcommand, ValueEnum};
pub(crate) use cycle::{CycleCommand, RuntimeArgs, RuntimeContext};
use dev_cmd::DevCommand;
use git_cmd::GitCommand;
use pack_cmd::PackCommand;
use permission::PermissionCommand;
use release_cmd::ReleaseCommand;
use result_cmd::{AgentResultCommand, ValidateCommand};
use sddk_domain::{IdentitySource, normalize_scope, resolve_project_identity, stable_workspace_id};
use sddk_engine::{
    AdoptionPlan, AdoptionPlanInput, AdoptionStatus, AdoptionStatusKind, XdgEnvironment,
    adoption_status, apply_adoption, plan_adoption, read_adoption_receipt, repair_adoption,
};
use serde::Serialize;
use time::{OffsetDateTime, format_description::well_known::Rfc3339};
use uuid::Uuid;
use vault_cmd::VaultCommand;
use walkdir::WalkDir;

pub use docs::{GENERATED_WORKFLOW_DOC, GenerationStatus, generate_workflow_docs};
pub use inventory::{GENERATED_INVENTORY_DOC, generate_inventory};
pub use lint::{Diagnostic, LintReport, Severity, lint_repository};

/// Canonical workflow manifest path, relative to the repository root.
pub(crate) const WORKFLOW_MANIFEST: &str = "workflow/workflow.yaml";

/// Parsed SDDK command line.
#[derive(Debug, Parser)]
#[command(name = "sddk", version, about = "Deterministic SDDK workflow tooling")]
pub struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Resolve deterministic project and workspace identity.
    Project {
        #[command(subcommand)]
        command: ProjectCommand,
    },
    /// Plan, apply, inspect, or repair project adoption.
    Adopt {
        #[command(subcommand)]
        command: AdoptCommand,
    },
    /// Validate repository contracts and generated workflow documentation.
    Lint {
        /// Repository root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Diagnostic output format.
        #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
        format: OutputFormat,
    },
    /// Generate deterministic repository documentation.
    Generate {
        #[command(subcommand)]
        command: GenerateCommand,
    },
    /// Plan and apply workflow cycles under the local authority.
    Cycle {
        #[command(subcommand)]
        command: CycleCommand,
    },
    /// Verify the causal ledger and list its events.
    Ledger {
        #[command(subcommand)]
        command: ledger::LedgerCommand,
    },
    /// Plan and execute typed capabilities under the default-deny policy.
    Capability {
        #[command(subcommand)]
        command: CapabilityCommand,
    },
    /// Run typed local Git operations with verified postconditions.
    Git {
        #[command(subcommand)]
        command: GitCommand,
    },
    /// Store and verify content-addressed artifacts.
    Artifact {
        #[command(subcommand)]
        command: ArtifactCommand,
    },
    /// Check agent phase and capability permissions.
    Permission {
        #[command(subcommand)]
        command: PermissionCommand,
    },
    /// Validate structured results against canonical schemas.
    Validate {
        #[command(subcommand)]
        command: ValidateCommand,
    },
    /// Convert legacy agent output into structured results.
    AgentResult {
        #[command(subcommand)]
        command: AgentResultCommand,
    },
    /// Plan and apply releases through the forge adapter.
    Release {
        #[command(subcommand)]
        command: ReleaseCommand,
    },
    /// Index, validate, search, and export knowledge vaults.
    Vault {
        #[command(subcommand)]
        command: VaultCommand,
    },
    /// Developer tooling: doctor, gates, and atomic install/verify.
    Dev {
        #[command(subcommand)]
        command: DevCommand,
    },
    /// Validate declarative pack manifests.
    Pack {
        #[command(subcommand)]
        command: PackCommand,
    },
}

#[derive(Debug, Subcommand)]
enum ProjectCommand {
    /// Resolve identity without writing project or SDDK state.
    Resolve(ProjectResolveArgs),
}

#[derive(Debug, Subcommand)]
enum AdoptCommand {
    /// Preview identity, paths, and receipt without writing them.
    Plan(AdoptionArgs),
    /// Converge absent or matching partial adoption state.
    Apply(AdoptionArgs),
    /// Classify current receipt and SQLite registration state.
    Status(AdoptionArgs),
    /// Complete matching partial state without overwriting conflicts.
    Repair(AdoptionArgs),
}

#[derive(Debug, Clone, Args)]
struct ProjectResolveArgs {
    /// Checkout or worktree root.
    #[arg(long)]
    root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    fallback_seed: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
struct AdoptionArgs {
    /// Checkout or worktree root.
    #[arg(long)]
    root: PathBuf,
    /// Required monorepo scope, using `.` for the repository root.
    #[arg(long)]
    scope: String,
    /// Explicit remote URL instead of read-only Git discovery.
    #[arg(long)]
    remote: Option<String>,
    /// Stable UUID for a repository without a remote.
    #[arg(long)]
    fallback_seed: Option<String>,
    /// Explicit RFC 3339 timestamp for deterministic execution.
    #[arg(long)]
    timestamp: Option<String>,
    /// Explicit actor for deterministic execution.
    #[arg(long)]
    actor: Option<String>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
}

#[derive(Debug, Subcommand)]
enum GenerateCommand {
    /// Render workflow metadata, tables, and Mermaid state diagram.
    Docs {
        /// Repository root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Check generated output without writing it.
        #[arg(long)]
        check: bool,
    },
    /// Render a deterministic inventory of repository agents and skills.
    Inventory {
        /// Repository root.
        #[arg(long, default_value = ".")]
        root: PathBuf,
        /// Check generated output without writing it.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

/// Captured process output and exit status.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CommandOutput {
    /// Process-style exit status.
    pub status: i32,
    /// Captured standard output.
    pub stdout: String,
    /// Captured standard error.
    pub stderr: String,
}

/// Process environment values used by CLI XDG and actor defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliEnvironment {
    /// `HOME`, when set and non-empty.
    pub home: Option<PathBuf>,
    /// `XDG_DATA_HOME`, when set and non-empty.
    pub data_home: Option<PathBuf>,
    /// `XDG_STATE_HOME`, when set and non-empty.
    pub state_home: Option<PathBuf>,
    /// `XDG_CACHE_HOME`, when set and non-empty.
    pub cache_home: Option<PathBuf>,
    /// `SDDK_ACTOR`, when set and non-empty.
    pub sddk_actor: Option<String>,
    /// `USER`, when set and non-empty.
    pub user: Option<String>,
}

impl CliEnvironment {
    fn current() -> Self {
        Self {
            home: nonempty_env_path("HOME"),
            data_home: nonempty_env_path("XDG_DATA_HOME"),
            state_home: nonempty_env_path("XDG_STATE_HOME"),
            cache_home: nonempty_env_path("XDG_CACHE_HOME"),
            sddk_actor: nonempty_env_string("SDDK_ACTOR"),
            user: nonempty_env_string("USER"),
        }
    }

    fn xdg(&self) -> XdgEnvironment {
        XdgEnvironment {
            home: self.home.clone(),
            data_home: self.data_home.clone(),
            state_home: self.state_home.clone(),
            cache_home: self.cache_home.clone(),
        }
    }
}

/// Parses arguments and executes a command without terminating the process.
pub fn run_from<I, T>(args: I) -> CommandOutput
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    match Cli::try_parse_from(args) {
        Ok(cli) => run_with_environment(cli, &CliEnvironment::current()),
        Err(error) => CommandOutput {
            status: error.exit_code(),
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

/// Executes an already parsed command and captures all output.
pub fn run(cli: Cli) -> CommandOutput {
    run_with_environment(cli, &CliEnvironment::current())
}

/// Executes an already parsed command with explicit process environment values.
pub fn run_with_environment(cli: Cli, environment: &CliEnvironment) -> CommandOutput {
    match cli.command {
        Command::Project {
            command: ProjectCommand::Resolve(args),
        } => run_project_resolve(args),
        Command::Adopt { command } => run_adopt(command, environment),
        Command::Lint { root, format } => match lint_repository(&root) {
            Ok(report) => {
                let status = i32::from(report.has_errors());
                let stdout = match format {
                    OutputFormat::Text => report.to_text(),
                    OutputFormat::Json => match serde_json::to_string_pretty(&report) {
                        Ok(json) => format!("{json}\n"),
                        Err(error) => {
                            return failure(format!("failed to serialize diagnostics: {error}"));
                        }
                    },
                };
                CommandOutput {
                    status,
                    stdout,
                    stderr: String::new(),
                }
            }
            Err(error) => failure(error.to_string()),
        },
        Command::Generate {
            command: GenerateCommand::Docs { root, check },
        } => run_generation(
            generate_workflow_docs(&root, check),
            GENERATED_WORKFLOW_DOC,
            "docs",
            &root,
        ),
        Command::Generate {
            command: GenerateCommand::Inventory { root, check },
        } => run_generation(
            generate_inventory(&root, check),
            GENERATED_INVENTORY_DOC,
            "inventory",
            &root,
        ),
        Command::Cycle { command } => cycle::run_cycle(command, environment),
        Command::Ledger { command } => ledger::run_ledger(command, environment),
        Command::Capability { command } => capability::run_capability(command, environment),
        Command::Git { command } => git_cmd::run_git(command, environment),
        Command::Artifact { command } => artifact::run_artifact(command, environment),
        Command::Permission { command } => permission::run_permission(command, environment),
        Command::Validate { command } => result_cmd::run_validate(command, environment),
        Command::AgentResult { command } => result_cmd::run_agent_result(command, environment),
        Command::Release { command } => release_cmd::run_release(command, environment),
        Command::Vault { command } => vault_cmd::run_vault(command),
        Command::Dev { command } => dev_cmd::run_dev(command),
        Command::Pack { command } => pack_cmd::run_pack(command),
    }
}

fn run_generation<E: std::fmt::Display>(
    result: Result<GenerationStatus, E>,
    generated_path: &str,
    command: &str,
    root: &Path,
) -> CommandOutput {
    match result {
        Ok(GenerationStatus::Current) => CommandOutput {
            stdout: format!("{generated_path} is current\n"),
            ..CommandOutput::default()
        },
        Ok(GenerationStatus::Written) => CommandOutput {
            stdout: format!("wrote {generated_path}\n"),
            ..CommandOutput::default()
        },
        Ok(GenerationStatus::Stale) => CommandOutput {
            status: 1,
            stderr: format!(
                "{generated_path} is missing or stale; run `sddk generate {command} --root {}`\n",
                root.display()
            ),
            ..CommandOutput::default()
        },
        Err(error) => failure(error.to_string()),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct ProjectResolution {
    project_id: String,
    workspace_id: String,
    canonical_workspace_path: String,
    identity_source: IdentitySource,
    remote_url: Option<String>,
    scope: String,
    fallback_seed: Option<String>,
}

#[derive(Clone, Copy)]
enum AdoptionOperation {
    Plan,
    Apply,
    Status,
    Repair,
}

fn run_project_resolve(args: ProjectResolveArgs) -> CommandOutput {
    let result = (|| -> anyhow::Result<ProjectResolution> {
        let root = canonical_root(&args.root)?;
        let remote = resolve_remote(&root, args.remote)?;
        let fallback_seed = match (remote.as_ref(), args.fallback_seed) {
            (None, None) => Some(Uuid::new_v4().hyphenated().to_string()),
            (_, seed) => seed,
        };
        let identity =
            resolve_project_identity(remote.as_deref(), &args.scope, fallback_seed.as_deref())?;
        let canonical_workspace_path = path_string(&root)?;
        let workspace_id = stable_workspace_id(&identity.project_id, &canonical_workspace_path);
        Ok(ProjectResolution {
            project_id: identity.project_id.to_string(),
            workspace_id,
            canonical_workspace_path,
            identity_source: identity.identity_source,
            remote_url: identity.remote_url,
            scope: identity.scope,
            fallback_seed: identity.fallback_seed,
        })
    })();
    render_result(result, args.format, project_resolution_text)
}

fn run_adopt(command: AdoptCommand, environment: &CliEnvironment) -> CommandOutput {
    let (operation, args) = match command {
        AdoptCommand::Plan(args) => (AdoptionOperation::Plan, args),
        AdoptCommand::Apply(args) => (AdoptionOperation::Apply, args),
        AdoptCommand::Status(args) => (AdoptionOperation::Status, args),
        AdoptCommand::Repair(args) => (AdoptionOperation::Repair, args),
    };
    let format = args.format;
    let result = (|| -> anyhow::Result<AdoptionCommandResult> {
        let plan = prepare_adoption_plan(args, operation, environment)?;
        Ok(match operation {
            AdoptionOperation::Plan => AdoptionCommandResult::Plan(plan),
            AdoptionOperation::Apply => AdoptionCommandResult::Status(apply_adoption(&plan)?),
            AdoptionOperation::Status => AdoptionCommandResult::Status(adoption_status(&plan)?),
            AdoptionOperation::Repair => AdoptionCommandResult::Status(repair_adoption(&plan)?),
        })
    })();
    match result {
        Ok(result) => {
            let status = match &result {
                AdoptionCommandResult::Plan(_) => 0,
                AdoptionCommandResult::Status(status) => {
                    i32::from(status.status != AdoptionStatusKind::Complete)
                }
            };
            match render(&result, format, adoption_result_text) {
                Ok(stdout) => CommandOutput {
                    status,
                    stdout,
                    stderr: String::new(),
                },
                Err(error) => failure(error.to_string()),
            }
        }
        Err(error) => failure(error.to_string()),
    }
}

#[derive(Serialize)]
#[serde(untagged)]
enum AdoptionCommandResult {
    Plan(AdoptionPlan),
    Status(AdoptionStatus),
}

fn prepare_adoption_plan(
    args: AdoptionArgs,
    operation: AdoptionOperation,
    environment: &CliEnvironment,
) -> anyhow::Result<AdoptionPlan> {
    let root = canonical_root(&args.root)?;
    let remote = resolve_remote(&root, args.remote)?;
    let mut fallback_seed = args.fallback_seed;
    if remote.is_none() && fallback_seed.is_none() {
        fallback_seed = find_persisted_fallback_seed(environment, &root, &args.scope)?;
    }
    if remote.is_none() && fallback_seed.is_none() {
        fallback_seed = match operation {
            AdoptionOperation::Plan | AdoptionOperation::Apply => {
                Some(Uuid::new_v4().hyphenated().to_string())
            }
            AdoptionOperation::Status | AdoptionOperation::Repair => {
                anyhow::bail!(
                    "fallback seed is required because no remote or matching adoption receipt exists"
                )
            }
        };
    }
    let display_name = root
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| anyhow::anyhow!("root has no UTF-8 display name: {root:?}"))?
        .to_owned();
    let timestamp = match args.timestamp {
        Some(timestamp) => timestamp,
        None => OffsetDateTime::now_utc().format(&Rfc3339)?,
    };
    let actor = args
        .actor
        .or_else(|| environment.sddk_actor.clone())
        .or_else(|| environment.user.clone())
        .unwrap_or_else(|| "sddk-cli".into());
    Ok(plan_adoption(AdoptionPlanInput {
        remote_url: remote,
        scope: args.scope,
        fallback_seed,
        canonical_workspace_path: root,
        display_name,
        xdg: environment.xdg(),
        sddk_version: "3.6".into(),
        runtime_version: env!("CARGO_PKG_VERSION").into(),
        timestamp,
        actor,
    })?)
}

fn find_persisted_fallback_seed(
    environment: &CliEnvironment,
    root: &Path,
    scope: &str,
) -> anyhow::Result<Option<String>> {
    let data_home = match (&environment.data_home, &environment.home) {
        (Some(data), _) => data.clone(),
        (None, Some(home)) => home.join(".local/share"),
        (None, None) => return Ok(None),
    };
    if !data_home.is_absolute() {
        anyhow::bail!("XDG_DATA_HOME must be absolute: {data_home:?}");
    }
    let projects = data_home.join("sddk/projects");
    if !projects.exists() {
        return Ok(None);
    }
    let root = path_string(root)?;
    let scope = normalize_scope(scope)?;
    let mut found = None;
    for entry in WalkDir::new(projects)
        .min_depth(4)
        .max_depth(4)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file() && entry.file_name() == "adoption.json")
    {
        let receipt = match read_adoption_receipt(entry.path()) {
            Ok(receipt) => receipt,
            Err(_) => continue,
        };
        if receipt.identity_source == IdentitySource::Fallback
            && receipt.canonical_workspace_path == root
            && receipt.scope == scope
        {
            if found.is_some() {
                anyhow::bail!("multiple fallback adoption receipts match this workspace");
            }
            found = receipt.fallback_seed;
        }
    }
    Ok(found)
}

fn resolve_remote(root: &Path, explicit: Option<String>) -> anyhow::Result<Option<String>> {
    if explicit.is_some() {
        return Ok(explicit);
    }
    let output = match ProcessCommand::new("git")
        .arg("-C")
        .arg(root)
        .args(["config", "--get", "remote.origin.url"])
        .output()
    {
        Ok(output) => output,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if !output.status.success() {
        return Ok(None);
    }
    let remote = String::from_utf8(output.stdout)?.trim().to_owned();
    Ok((!remote.is_empty()).then_some(remote))
}

fn canonical_root(root: &Path) -> anyhow::Result<PathBuf> {
    let root = std::fs::canonicalize(root)?;
    if !root.is_dir() {
        anyhow::bail!("root is not a directory: {root:?}");
    }
    Ok(root)
}

fn project_resolution_text(resolution: &ProjectResolution) -> String {
    format!(
        "project_id: {}\nworkspace_id: {}\ncanonical_workspace_path: {}\nidentity_source: {}\nremote_url: {}\nscope: {}\nfallback_seed: {}\n",
        resolution.project_id,
        resolution.workspace_id,
        resolution.canonical_workspace_path,
        identity_source_text(resolution.identity_source),
        resolution.remote_url.as_deref().unwrap_or("null"),
        resolution.scope,
        resolution.fallback_seed.as_deref().unwrap_or("null")
    )
}

fn adoption_result_text(result: &AdoptionCommandResult) -> String {
    match result {
        AdoptionCommandResult::Plan(plan) => format!(
            "status: planned\nproject_id: {}\nworkspace_id: {}\nconfiguration_hash: {}\nvault: {}\nartifacts: {}\nledger: {}\ncache: {}\nreceipt: {}\n",
            plan.receipt.project_id,
            plan.receipt.workspace_id,
            plan.receipt.configuration_hash,
            plan.paths.vault.display(),
            plan.paths.artifacts.display(),
            plan.paths.ledger.display(),
            plan.paths.cache.display(),
            plan.paths.receipt.display()
        ),
        AdoptionCommandResult::Status(status) => format!(
            "status: {}\nproject_id: {}\nworkspace_id: {}\nreceipt: {}\nledger: {}\n{}",
            adoption_status_text(status.status),
            status.project_id,
            status.workspace_id,
            status.receipt_path.display(),
            status.ledger_path.display(),
            status
                .detail
                .as_ref()
                .map(|detail| format!("detail: {detail}\n"))
                .unwrap_or_default()
        ),
    }
}

fn identity_source_text(source: IdentitySource) -> &'static str {
    match source {
        IdentitySource::Remote => "remote",
        IdentitySource::Fallback => "fallback",
    }
}

fn adoption_status_text(status: AdoptionStatusKind) -> &'static str {
    match status {
        AdoptionStatusKind::Absent => "absent",
        AdoptionStatusKind::Complete => "complete",
        AdoptionStatusKind::ReceiptOnly => "receipt_only",
        AdoptionStatusKind::LedgerOnly => "ledger_only",
        AdoptionStatusKind::Conflict => "conflict",
        AdoptionStatusKind::Corrupt => "corrupt",
    }
}

fn render_result<T: Serialize>(
    result: anyhow::Result<T>,
    format: OutputFormat,
    text: fn(&T) -> String,
) -> CommandOutput {
    match result {
        Ok(value) => match render(&value, format, text) {
            Ok(stdout) => CommandOutput {
                stdout,
                ..CommandOutput::default()
            },
            Err(error) => failure(error.to_string()),
        },
        Err(error) => failure(error.to_string()),
    }
}

fn render<T: Serialize>(
    value: &T,
    format: OutputFormat,
    text: fn(&T) -> String,
) -> anyhow::Result<String> {
    match format {
        OutputFormat::Text => Ok(text(value)),
        OutputFormat::Json => Ok(format!("{}\n", serde_json::to_string_pretty(value)?)),
    }
}

fn path_string(path: &Path) -> anyhow::Result<String> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("path is not valid UTF-8: {path:?}"))
}

fn nonempty_env_path(name: &str) -> Option<PathBuf> {
    std::env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn nonempty_env_string(name: &str) -> Option<String> {
    std::env::var(name).ok().filter(|value| !value.is_empty())
}

fn failure(message: String) -> CommandOutput {
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
    }
}
