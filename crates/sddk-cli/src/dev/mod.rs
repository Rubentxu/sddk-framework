//! Developer tooling: environment doctor, gates, and atomic install/verify.

use clap::{Args, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

mod check;
mod common;
mod doctor;
mod framework_check;
mod install;
mod link;
mod manifest;
pub(crate) mod paths;
mod registry;
mod uninstall;
mod update;
mod use_cmd;
mod verify;

use crate::{CliEnvironment, CommandOutput, OutputFormat};

/// Manifest file name, written at the framework root (and shipped in the
/// release bundle).
pub(super) const MANIFEST_FILE: &str = "MANIFEST.sha256";

/// Persisted installation receipt for side-by-side prefixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) struct InstallReceipt {
    /// Installed version.
    pub version: String,
    /// Source commit.
    pub commit: String,
    /// SHA-256 of the installed binary.
    pub binary_sha256: String,
    /// Release channel.
    pub channel: String,
    /// Installation timestamp.
    pub installed_at: String,
    /// Binary path relative to the prefix.
    pub binary_path: String,
    /// Whether this install included the full bundle (agents/skills/prompts/assets).
    /// When true, `dev verify` checks installed surfaces against the manifest.
    #[serde(default = "default_bundle_true")]
    pub bundle: bool,
}

fn default_bundle_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(super) enum LinkEditor {
    #[value(name = "opencode")]
    OpenCode,
    #[value(name = "zcode")]
    ZCode,
    #[value(name = "all")]
    All,
}

#[derive(Debug, Subcommand)]
pub(super) enum DevCommand {
    /// Check the toolchain and environment prerequisites.
    Doctor(DoctorArgs),
    /// Run repository quality gates (fmt, clippy, tests).
    Check(CheckArgs),
    /// Install this binary atomically into a prefix with a receipt.
    Install(InstallArgs),
    /// Verify an installed prefix against its receipt.
    Verify(VerifyArgs),
    /// Remove an installed prefix only when it matches its receipt.
    Uninstall(UninstallArgs),
    /// Symlink the framework assets (agents/skills/prompts/workflows) into an editor.
    Link(LinkArgs),
    /// Select the active framework bundle version (asdf-style `use`).
    Use(UseArgs),
    /// Generate or verify MANIFEST.sha256 — per-file content hashes of the
    /// framework surfaces (agents, skills, prompts, workflows, assets).
    Manifest(ManifestArgs),
    /// Install a framework release bundle (download, verify checksum +
    /// internal MANIFEST.sha256, extract). Never touches git — source
    /// checkouts are managed by the developer (`git pull` + `dev link`).
    Update(UpdateArgs),
}

#[derive(Debug, Clone, Args)]
pub(super) struct ManifestArgs {
    /// Framework root to scan (default: current directory).
    #[arg(long)]
    pub(super) root: Option<std::path::PathBuf>,
    /// Verify an existing manifest instead of generating one.
    #[arg(long)]
    pub(super) verify: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct UseArgs {
    /// Version to activate (installed bundle) or `path:<dir>` for dogfooding.
    #[arg(long, required_unless_present = "show")]
    pub(super) version: Option<String>,
    /// Show the active version without changing it.
    #[arg(long)]
    pub(super) show: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct DoctorArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
    /// Strict mode: exit 1 when any surface brevity check reports a file over threshold.
    #[arg(long)]
    pub(super) strict: bool,
}

#[derive(Debug, Clone, Args)]
pub(super) struct CheckArgs {
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct InstallArgs {
    /// Installation prefix directory.
    #[arg(long)]
    pub(super) prefix: std::path::PathBuf,
    /// Release channel.
    #[arg(long, default_value = "dev")]
    pub(super) channel: String,
    /// Explicit RFC 3339 timestamp.
    #[arg(long)]
    pub(super) timestamp: Option<String>,
    /// Explicit source commit.
    #[arg(long)]
    pub(super) commit: Option<String>,
    /// Source checkout or bundle root containing agents/skills/prompts/workflows/assets
    /// and MANIFEST.sha256.
    #[arg(long)]
    pub(super) source: Option<std::path::PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct VerifyArgs {
    /// Installation prefix directory.
    #[arg(long)]
    pub(super) prefix: std::path::PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct UninstallArgs {
    /// Installation prefix directory (optional when removing editor assets only).
    #[arg(long)]
    pub(super) prefix: Option<std::path::PathBuf>,
    /// Also remove framework assets from an editor (opencode|zcode|all).
    #[arg(long, value_enum)]
    pub(super) editor: Option<LinkEditor>,
    /// Repository root (required with --editor).
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Override the OpenCode config dir.
    #[arg(long)]
    pub(super) opencode_dir: Option<std::path::PathBuf>,
    /// Override the ZCode dir.
    #[arg(long)]
    pub(super) zcode_dir: Option<std::path::PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct LinkArgs {
    /// Repository root containing agents/skills/prompts.
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Target editor(s).
    #[arg(long, value_enum, default_value_t = LinkEditor::All)]
    pub(super) editor: LinkEditor,
    /// Override the OpenCode config dir.
    #[arg(long)]
    pub(super) opencode_dir: Option<std::path::PathBuf>,
    /// Override the ZCode dir.
    #[arg(long)]
    pub(super) zcode_dir: Option<std::path::PathBuf>,
    /// Write an idempotent, deduplicated skill registry to
    /// `$SDDK_DATA_DIR/projects/<project_id>/skill-registry.md`.
    #[arg(long)]
    pub(super) write_registry: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(super) struct UpdateArgs {
    /// Framework root containing agents/skills/prompts (bundle install)
    /// or a git checkout (developer install).
    #[arg(long, default_value = ".")]
    pub(super) root: std::path::PathBuf,
    /// Release version to fetch when the root is not a git checkout.
    #[arg(long)]
    pub(super) version: Option<String>,
    /// GitHub repository (owner/name) providing release assets.
    #[arg(long, default_value = "Rubentxu/sddk-framework")]
    pub(super) repo: String,
    /// Release base URL override (testing with file://).
    #[arg(long)]
    pub(super) base_url: Option<String>,
    /// Target editor(s) to re-link after the update.
    #[arg(long, value_enum, default_value_t = LinkEditor::All)]
    pub(super) editor: LinkEditor,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(super) format: OutputFormat,
}

pub(super) fn run_dev(command: DevCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        DevCommand::Doctor(args) => self::doctor::run_dev_doctor(args, environment),
        DevCommand::Check(args) => self::check::run_dev_check(args),
        DevCommand::Install(args) => self::install::run_dev_install(args),
        DevCommand::Verify(args) => self::verify::run_dev_verify(args),
        DevCommand::Uninstall(args) => self::uninstall::run_dev_uninstall(args),
        DevCommand::Link(args) => self::link::run_dev_link(args, environment),
        DevCommand::Use(args) => self::use_cmd::run_dev_use(args, environment),
        DevCommand::Update(args) => self::update::run_dev_update(args, environment),
        DevCommand::Manifest(args) => self::manifest::run_dev_manifest(args),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Smoke tests — verify subcommand entry points do not panic
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod smoke_tests {
    use super::*;

    fn env() -> CliEnvironment {
        CliEnvironment::default()
    }

    #[test]
    fn dev_doctor_does_not_panic() {
        let args = super::DoctorArgs {
            format: OutputFormat::Text,
            strict: false,
        };
        let _ = self::doctor::run_dev_doctor(args, &env());
    }

    #[test]
    fn dev_check_does_not_panic() {
        let args = super::CheckArgs {
            root: std::path::PathBuf::from("."),
            format: OutputFormat::Text,
        };
        let _ = self::check::run_dev_check(args);
    }

    #[test]
    fn dev_use_show_does_not_panic() {
        let args = super::UseArgs {
            version: None,
            show: true,
            format: OutputFormat::Text,
        };
        let _ = self::use_cmd::run_dev_use(args, &env());
    }

    #[test]
    fn dev_manifest_write_does_not_panic() {
        let tmp = tempfile::tempdir().unwrap();
        let args = super::ManifestArgs {
            root: Some(tmp.path().to_path_buf()),
            verify: false,
            format: OutputFormat::Text,
        };
        let _ = self::manifest::run_dev_manifest(args);
    }
}
