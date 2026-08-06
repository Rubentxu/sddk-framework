//! Developer tooling: environment doctor, gates, and atomic install/verify.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum};
use sddk_gateway::{PermissionPolicy, RunSpec, run};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CommandOutput, OutputFormat, render_result};

const RECEIPT_FILE: &str = "sddk-install.json";

#[derive(Debug, Subcommand)]
pub(crate) enum DevCommand {
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
    /// Update the framework: pull, re-link, rebuild, verify.
    Update(UpdateArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct DoctorArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct CheckArgs {
    /// Repository root.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct InstallArgs {
    /// Installation prefix directory.
    #[arg(long)]
    pub(crate) prefix: PathBuf,
    /// Release channel.
    #[arg(long, default_value = "dev")]
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
pub(crate) struct VerifyArgs {
    /// Installation prefix directory.
    #[arg(long)]
    pub(crate) prefix: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UninstallArgs {
    /// Installation prefix directory (optional when removing editor assets only).
    #[arg(long)]
    pub(crate) prefix: Option<PathBuf>,
    /// Also remove framework assets from an editor (opencode|zcode|all).
    #[arg(long, value_enum)]
    pub(crate) editor: Option<LinkEditor>,
    /// Repository root (required with --editor).
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Override the OpenCode config dir.
    #[arg(long)]
    pub(crate) opencode_dir: Option<PathBuf>,
    /// Override the ZCode dir.
    #[arg(long)]
    pub(crate) zcode_dir: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Target editor for framework linking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub(crate) enum LinkEditor {
    #[value(name = "opencode")]
    OpenCode,
    #[value(name = "zcode")]
    ZCode,
    #[value(name = "all")]
    All,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct LinkArgs {
    /// Repository root containing agents/skills/prompts.
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Target editor(s).
    #[arg(long, value_enum, default_value_t = LinkEditor::All)]
    pub(crate) editor: LinkEditor,
    /// Override the OpenCode config dir.
    #[arg(long)]
    pub(crate) opencode_dir: Option<PathBuf>,
    /// Override the ZCode dir.
    #[arg(long)]
    pub(crate) zcode_dir: Option<PathBuf>,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UpdateArgs {
    /// Framework root containing agents/skills/prompts (bundle install)
    /// or a git checkout (developer install).
    #[arg(long, default_value = ".")]
    pub(crate) root: PathBuf,
    /// Release version to fetch when the root is not a git checkout.
    #[arg(long)]
    pub(crate) version: Option<String>,
    /// GitHub repository (owner/name) providing release assets.
    #[arg(long, default_value = "Rubentxu/sddk-framework")]
    pub(crate) repo: String,
    /// Release base URL override (testing with file://).
    #[arg(long)]
    pub(crate) base_url: Option<String>,
    /// Target editor(s) to re-link after the update.
    #[arg(long, value_enum, default_value_t = LinkEditor::All)]
    pub(crate) editor: LinkEditor,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

/// Persisted installation receipt for side-by-side prefixes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) struct InstallReceipt {
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
}

pub(crate) fn run_dev(command: DevCommand) -> CommandOutput {
    match command {
        DevCommand::Doctor(args) => run_dev_doctor(args),
        DevCommand::Check(args) => run_dev_check(args),
        DevCommand::Install(args) => run_dev_install(args),
        DevCommand::Verify(args) => run_dev_verify(args),
        DevCommand::Uninstall(args) => run_dev_uninstall(args),
        DevCommand::Link(args) => run_dev_link(args),
        DevCommand::Update(args) => run_dev_update(args),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct DoctorOutput {
    checks: Vec<DoctorCheck>,
    all_present: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct DoctorCheck {
    tool: String,
    present: bool,
}

fn run_dev_doctor(args: DoctorArgs) -> CommandOutput {
    let format = args.format;
    let mut checks = Vec::new();
    for tool in ["cargo", "rustc", "git", "gh"] {
        let present = tool_version(tool).is_ok();
        checks.push(DoctorCheck {
            tool: tool.to_owned(),
            present,
        });
    }
    // Framework asset integrity checks for detected editors.
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let opencode_dir = home.join(".config/opencode");
    let zcode_dir = home.join(".zcode");
    let mut framework_warnings = 0usize;
    for (label, editor_dir) in [("opencode", opencode_dir), ("zcode", zcode_dir)] {
        if !editor_dir.is_dir() {
            continue;
        }
        for check in check_framework(&root, &editor_dir) {
            if check.status != "PASS" {
                framework_warnings += 1;
            }
            checks.push(DoctorCheck {
                tool: format!("{label}.{}", check.name),
                present: check.status == "PASS",
            });
        }
    }
    let result = Ok::<_, anyhow::Error>(DoctorOutput {
        all_present: checks.iter().all(|check| check.present) && framework_warnings == 0,
        checks,
    });
    match result {
        Ok(output) => {
            let cloned = DoctorOutput {
                all_present: output.all_present,
                checks: output.checks.clone(),
            };
            let mut command = render_result(Ok(cloned), format, doctor_text);
            if !output.all_present {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure_envelope(&error),
    }
}

fn run_dev_check(args: CheckArgs) -> CommandOutput {
    let steps = [
        ("fmt", vec!["fmt", "--all", "--", "--check"]),
        (
            "clippy",
            vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("test", vec!["test", "--workspace", "--locked"]),
    ];
    let mut text = String::new();
    let mut failed = false;
    for (name, args) in steps {
        let spec = RunSpec {
            program: "cargo".into(),
            args: args.into_iter().map(str::to_owned).collect(),
            env: Default::default(),
            timeout_ms: 600_000,
            output_max_bytes: 1_048_576,
        };
        let outcome = match run(&spec) {
            Ok(outcome) => outcome,
            Err(error) => {
                failed = true;
                text.push_str(&format!("{name}: FAILED ({error})\n"));
                continue;
            }
        };
        let passed = outcome.exit_status == Some(0) && !outcome.timed_out;
        if !passed {
            failed = true;
        }
        text.push_str(&format!(
            "{name}: {}\n",
            if passed { "PASS" } else { "FAIL" }
        ));
    }
    let mut output = CommandOutput {
        status: i32::from(failed),
        stdout: text,
        stderr: String::new(),
    };
    if let OutputFormat::Json = args.format {
        output.stdout = format!("{}\n", serde_json::json!({"passed": !failed}));
    }
    output
}

fn run_dev_install(args: InstallArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<InstallReceipt> {
        let binary = std::env::current_exe()?;
        let bytes = std::fs::read(&binary)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        let bin_dir = args.prefix.join("bin");
        std::fs::create_dir_all(&bin_dir)?;
        let destination = bin_dir.join("sddk");
        atomic_write(&destination, &bytes)?;

        let receipt = InstallReceipt {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            commit: args
                .commit
                .or_else(|| std::env::var("GITHUB_SHA").ok())
                .unwrap_or_else(|| "unknown".to_owned()),
            binary_sha256: digest,
            channel: args.channel.clone(),
            installed_at: args
                .timestamp
                .unwrap_or_else(crate::git_cmd::default_timestamp),
            binary_path: "bin/sddk".to_owned(),
        };
        let receipt_path = args.prefix.join(RECEIPT_FILE);
        atomic_write(
            &receipt_path,
            serde_json::to_string_pretty(&receipt)?.as_bytes(),
        )?;
        Ok(receipt)
    })();
    render_result(result, format, receipt_text)
}

fn run_dev_verify(args: VerifyArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<InstallReceipt> {
        let receipt = read_receipt(&args.prefix)?;
        let binary_path = args.prefix.join(&receipt.binary_path);
        let bytes = std::fs::read(&binary_path)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        if digest != receipt.binary_sha256 {
            anyhow::bail!(
                "binary digest mismatch: receipt {}, found {}",
                receipt.binary_sha256,
                digest
            );
        }
        Ok(receipt)
    })();
    match result {
        Ok(receipt) => match format {
            OutputFormat::Json => {
                let mut value = serde_json::to_value(&receipt).unwrap_or(serde_json::Value::Null);
                if let serde_json::Value::Object(map) = &mut value {
                    map.insert("valid".into(), serde_json::Value::Bool(true));
                }
                CommandOutput {
                    stdout: format!(
                        "{}\n",
                        serde_json::to_string_pretty(&value).unwrap_or_default()
                    ),
                    ..CommandOutput::default()
                }
            }
            OutputFormat::Text => CommandOutput {
                stdout: format!("valid: true\n{}", receipt_text(&receipt)),
                ..CommandOutput::default()
            },
        },
        Err(error) => failure_status(error.to_string()),
    }
}

fn run_dev_uninstall(args: UninstallArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let mut output = String::new();

        // Binary prefix removal (existing behavior) — optional when --editor is used.
        if let Some(prefix) = &args.prefix {
            let receipt = read_receipt(prefix)?;
            let binary_path = prefix.join(&receipt.binary_path);
            let bytes = std::fs::read(&binary_path)?;
            let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
            if digest != receipt.binary_sha256 {
                anyhow::bail!("refusing to uninstall: binary does not match the receipt");
            }
            std::fs::remove_file(&binary_path)?;
            std::fs::remove_file(prefix.join(RECEIPT_FILE))?;
            output.push_str("binary: removed\n");
        }

        // Editor framework removal (optional).
        if let Some(editor) = args.editor {
            let root = std::fs::canonicalize(&args.root)?;
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp"));
            let opencode_dir = args
                .opencode_dir
                .clone()
                .unwrap_or_else(|| home.join(".config/opencode"));
            let zcode_dir = args
                .zcode_dir
                .clone()
                .unwrap_or_else(|| home.join(".zcode"));
            if matches!(editor, LinkEditor::OpenCode | LinkEditor::All) {
                let report = uninstall_editor(&root, &opencode_dir)?;
                output.push_str(&format!(
                    "opencode: {} entries, {} symlinks removed, {} kept\n",
                    report.entries_removed, report.symlinks_removed, report.files_kept
                ));
            }
            if matches!(editor, LinkEditor::ZCode | LinkEditor::All) {
                let report = uninstall_editor(&root, &zcode_dir)?;
                output.push_str(&format!(
                    "zcode: {} entries, {} symlinks removed, {} kept\n",
                    report.entries_removed, report.symlinks_removed, report.files_kept
                ));
            }
        }
        Ok(output)
    })();
    render_result(result, format, |output: &String| output.clone())
}

fn read_receipt(prefix: &Path) -> anyhow::Result<InstallReceipt> {
    let path = prefix.join(RECEIPT_FILE);
    if !path.exists() {
        anyhow::bail!("no installation receipt at {path:?}");
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

fn tool_version(tool: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new(tool).arg("--version").output()?;
    if !output.status.success() {
        anyhow::bail!("{tool} exited {}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn atomic_write(destination: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::io::Write;
    let parent = destination.parent().expect("destination has a parent");
    std::fs::create_dir_all(parent)?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let mut last_error = None;
    for attempt in 0..100 {
        let temporary = parent.join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                let result = (|| {
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    drop(file);
                    std::fs::rename(&temporary, destination)
                })();
                if let Err(source) = result {
                    let _ = std::fs::remove_file(&temporary);
                    return Err(source.into());
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = Some(error);
            }
            Err(source) => return Err(source.into()),
        }
    }
    Err(last_error
        .unwrap_or_else(|| std::io::Error::other("no temporary path available"))
        .into())
}

fn doctor_text(output: &DoctorOutput) -> String {
    let mut text = String::new();
    for check in &output.checks {
        text.push_str(&format!(
            "{}: {}\n",
            check.tool,
            if check.present { "present" } else { "missing" }
        ));
    }
    text.push_str(&format!("all_present: {}\n", output.all_present));
    text
}

fn receipt_text(receipt: &InstallReceipt) -> String {
    format!(
        "version: {}\ncommit: {}\nbinary_sha256: {}\nchannel: {}\ninstalled_at: {}\nbinary_path: {}\n",
        receipt.version,
        receipt.commit,
        receipt.binary_sha256,
        receipt.channel,
        receipt.installed_at,
        receipt.binary_path
    )
}

fn failure_status(message: String) -> CommandOutput {
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
    }
}

// --- Framework linking (agents/skills/prompts/workflows) ---

/// Report from a framework link operation.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct LinkReport {
    editor: String,
    agents_linked: usize,
    skills_linked: usize,
    prompts_linked: usize,
    workflows_linked: usize,
    stale_replaced: usize,
    errors: Vec<String>,
}

fn link_text(report: &LinkReport) -> String {
    format!(
        "editor: {}\nagents: {}\nskills: {}\nprompts: {}\nworkflows: {}\nstale_replaced: {}\nerrors: {}\n",
        report.editor,
        report.agents_linked,
        report.skills_linked,
        report.prompts_linked,
        report.workflows_linked,
        report.stale_replaced,
        report.errors.len()
    )
}

/// Replace a regular-file copy with a symlink, backing up the stale copy.
fn link_file(source: &Path, target: &Path, stale_replaced: &mut usize) -> std::io::Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(target) {
        if metadata.file_type().is_symlink() {
            // Already a link: refresh the target.
            std::fs::remove_file(target)?;
        } else {
            // Stale copy: back it up.
            let backup = target.with_extension("sddk-stale");
            if !backup.exists() {
                std::fs::rename(target, &backup)?;
            } else {
                std::fs::remove_file(target)?;
            }
            *stale_replaced += 1;
        }
    }
    std::fs::create_dir_all(target.parent().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "target has no parent")
    })?)?;
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(source, target)?;
    }
    #[cfg(not(unix))]
    {
        std::fs::copy(source, target)?;
    }
    Ok(())
}

/// Link one editor directory: agents, skills, prompts, workflows.
fn link_editor(root: &Path, editor_dir: &Path) -> LinkReport {
    let mut report = LinkReport {
        editor: editor_dir.to_string_lossy().into_owned(),
        agents_linked: 0,
        skills_linked: 0,
        prompts_linked: 0,
        workflows_linked: 0,
        stale_replaced: 0,
        errors: Vec::new(),
    };
    let mut stale = 0usize;

    // Agents.
    let agents_source = root.join("agents");
    if let Ok(entries) = std::fs::read_dir(&agents_source) {
        let agents_target = editor_dir.join("agents");
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("md") {
                let name = entry.file_name();
                if let Err(error) = link_file(&entry.path(), &agents_target.join(&name), &mut stale)
                {
                    report.errors.push(format!("agents/{name:?}: {error}"));
                } else {
                    report.agents_linked += 1;
                }
            }
        }
    }

    // Skills (directories + top-level markdown like BOOK-*.md).
    let skills_source = root.join("skills");
    if let Ok(entries) = std::fs::read_dir(&skills_source) {
        let skills_target = editor_dir.join("skills");
        for entry in entries.flatten() {
            let path = entry.path();
            let is_skill_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);
            let is_markdown = path.extension().and_then(|e| e.to_str()) == Some("md");
            if is_skill_dir || is_markdown {
                let name = entry.file_name();
                if let Err(error) = link_file(&path, &skills_target.join(&name), &mut stale) {
                    report.errors.push(format!("skills/{name:?}: {error}"));
                } else {
                    report.skills_linked += 1;
                }
            }
        }
    }

    // Prompts (sdd-kernel tree).
    let prompts_source = root.join("prompts/sdd-kernel");
    let prompts_target = editor_dir.join("prompts/sdd-kernel");
    if prompts_source.is_dir() {
        for entry in walk_dir(&prompts_source) {
            if entry.is_file() {
                let relative = entry
                    .strip_prefix(&prompts_source)
                    .unwrap_or(entry.as_path());
                let target = prompts_target.join(relative);
                if let Err(error) = link_file(&entry, &target, &mut stale) {
                    report.errors.push(format!("prompts/{relative:?}: {error}"));
                } else {
                    report.prompts_linked += 1;
                }
            }
        }
    }

    // Workflows (canonical path in repo).
    let workflows_source = root.join("prompts/sdd-kernel/workflows");
    let workflows_target = editor_dir.join("workflows");
    if workflows_source.is_dir() {
        for entry in walk_dir(&workflows_source) {
            if entry.is_file()
                && let Some(name) = entry
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(str::to_owned)
            {
                if let Err(error) = link_file(&entry, &workflows_target.join(&name), &mut stale) {
                    report.errors.push(format!("workflows/{name:?}: {error}"));
                } else {
                    report.workflows_linked += 1;
                }
            }
        }
    }

    report.stale_replaced = stale;
    report
}

/// Recursively list files under a directory.
fn walk_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_dir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

fn run_dev_link(args: LinkArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<LinkReport>> {
        let root = std::fs::canonicalize(&args.root)?;
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let opencode_dir = args
            .opencode_dir
            .clone()
            .unwrap_or_else(|| home.join(".config/opencode"));
        let zcode_dir = args
            .zcode_dir
            .clone()
            .unwrap_or_else(|| home.join(".zcode"));
        let mut reports = Vec::new();
        if matches!(args.editor, LinkEditor::OpenCode | LinkEditor::All) {
            reports.push(link_editor(&root, &opencode_dir));
            // Register framework agents in opencode.json (created when absent,
            // so a fresh editor install still gets its agents registered).
            let opencode_json = opencode_dir.join("opencode.json");
            match register_opencode_agents(&root, &opencode_json) {
                Ok(registered) => {
                    eprintln!("opencode: registered {registered} framework agents in opencode.json")
                }
                Err(error) => eprintln!("warning: opencode.json registration failed: {error}"),
            }
        }
        if matches!(args.editor, LinkEditor::ZCode | LinkEditor::All) {
            reports.push(link_editor(&root, &zcode_dir));
        }
        Ok(reports)
    })();
    render_result(result, format, |reports: &Vec<LinkReport>| {
        let mut text = String::new();
        for report in reports {
            text.push_str(&link_text(report));
        }
        text
    })
}

// --- Framework doctor checks ---

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct FrameworkCheck {
    name: String,
    status: String,
    detail: String,
}

fn check_framework(root: &Path, editor_dir: &Path) -> Vec<FrameworkCheck> {
    let mut checks = Vec::new();

    // Broken symlinks in editor agents.
    let agents_dir = editor_dir.join("agents");
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        let broken: Vec<String> = entries
            .flatten()
            .filter(|entry| {
                entry
                    .path()
                    .symlink_metadata()
                    .map(|m| m.file_type().is_symlink())
                    .unwrap_or(false)
                    && !entry.path().exists()
            })
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        checks.push(FrameworkCheck {
            name: "broken_agent_links".into(),
            status: if broken.is_empty() { "PASS" } else { "WARN" }.into(),
            detail: if broken.is_empty() {
                "no broken agent symlinks".into()
            } else {
                format!("broken: {}", broken.join(", "))
            },
        });
    }

    // Stale copies: regular files where a symlink is expected AND the repo has
    // a matching asset (local-only agents are legitimate, not stale).
    let mut stale: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(metadata) = std::fs::symlink_metadata(&path)
                && metadata.file_type().is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("md")
                && root.join("agents").join(entry.file_name()).exists()
            {
                stale.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    checks.push(FrameworkCheck {
        name: "stale_agent_copies".into(),
        status: if stale.is_empty() { "PASS" } else { "WARN" }.into(),
        detail: if stale.is_empty() {
            "all agents are symlinks".into()
        } else {
            format!("stale copies (run dev link): {}", stale.join(", "))
        },
    });

    // Workflow origin: editor workflows must be symlinks to repo.
    let workflows_dir = editor_dir.join("workflows");
    let mut orphan_workflows = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&workflows_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| e == "yaml")
                && path
                    .symlink_metadata()
                    .map(|m| m.file_type().is_file())
                    .unwrap_or(false)
            {
                orphan_workflows.push(entry.file_name().to_string_lossy().into_owned());
            }
        }
    }
    checks.push(FrameworkCheck {
        name: "workflow_origin".into(),
        status: if orphan_workflows.is_empty() {
            "PASS"
        } else {
            "WARN"
        }
        .into(),
        detail: if orphan_workflows.is_empty() {
            "workflows are linked from repo".into()
        } else {
            format!(
                "orphan copies (run dev link): {}",
                orphan_workflows.join(", ")
            )
        },
    });

    let _ = root;
    checks
}

// --- Agent registration (opencode.json) ---

/// Names of framework agents: declared in permissions.yaml AND present in agents/*.md.
fn framework_agent_names(root: &Path) -> Vec<String> {
    let policy = match PermissionPolicy::from_file(root.join("permissions.yaml")) {
        Ok(policy) => policy,
        Err(_) => return Vec::new(),
    };
    let agents_dir = root.join("agents");
    policy
        .agents()
        .filter(|name| agents_dir.join(format!("{name}.md")).exists())
        .map(str::to_owned)
        .collect()
}

/// Orchestrator agents registered as primary (user-selectable) agents in
/// opencode; every other framework agent stays a hidden subagent.
const PRIMARY_AGENTS: [&str; 3] = ["orchestrator", "gentle-orchestrator", "book-orchestrator"];

/// Minimal frontmatter extraction (description/model) from an agent .md.
struct AgentFrontmatter {
    description: String,
    model: Option<String>,
}

fn parse_frontmatter(path: &Path) -> Option<AgentFrontmatter> {
    let content = std::fs::read_to_string(path).ok()?;
    let block = content.strip_prefix("---")?.split_once("---")?.0;
    let mut description = String::new();
    let mut model = None;
    for line in block.lines() {
        if let Some(value) = line.strip_prefix("description:") {
            description = value.trim().trim_matches('"').to_owned();
        } else if let Some(value) = line.strip_prefix("model:") {
            model = Some(value.trim().trim_matches('"').to_owned());
        }
    }
    if description.is_empty() {
        return None;
    }
    Some(AgentFrontmatter { description, model })
}

/// Upsert framework agent entries into opencode.json, creating the file when
/// absent so a fresh editor install still registers its agents.
fn register_opencode_agents(root: &Path, opencode_json: &Path) -> anyhow::Result<usize> {
    let mut config: serde_json::Value = if opencode_json.exists() {
        serde_json::from_str(&std::fs::read_to_string(opencode_json)?)
            .map_err(|e| anyhow::anyhow!("opencode.json invalid: {e}"))?
    } else {
        serde_json::json!({
            "$schema": "https://opencode.ai/config.json",
            "agent": {},
            "mcp": {}
        })
    };
    let agents = config
        .get_mut("agent")
        .and_then(|value| value.as_object_mut())
        .ok_or_else(|| anyhow::anyhow!("opencode.json has no agent map"))?;
    let mut registered = 0usize;
    for name in framework_agent_names(root) {
        let md_path = root.join("agents").join(format!("{name}.md"));
        let Some(frontmatter) = parse_frontmatter(&md_path) else {
            continue;
        };
        let model = frontmatter
            .model
            .clone()
            .unwrap_or_else(|| "minimax-coding-plan/MiniMax-M3".to_owned());
        let primary = PRIMARY_AGENTS.contains(&name.as_str());
        let mut entry = serde_json::json!({
            "description": frontmatter.description,
            "mode": if primary { "primary" } else { "subagent" },
            "model": model,
            "prompt": format!("{{file:{}}}", md_path.to_string_lossy()),
        });
        if !primary {
            entry["hidden"] = serde_json::Value::Bool(true);
        }
        agents.insert(name, entry);
        registered += 1;
    }
    let serialized = serde_json::to_string_pretty(&config)?;
    std::fs::write(opencode_json, serialized)?;
    Ok(registered)
}

/// Remove framework agent entries + framework symlinks from one editor.
fn uninstall_editor(root: &Path, editor_dir: &Path) -> anyhow::Result<UninstallReport> {
    let mut report = UninstallReport {
        editor: editor_dir.to_string_lossy().into_owned(),
        entries_removed: 0,
        symlinks_removed: 0,
        files_kept: 0,
        errors: Vec::new(),
    };

    // 1. opencode.json agent entries.
    let opencode_json = editor_dir.join("opencode.json");
    if opencode_json.exists()
        && let Ok(content) = std::fs::read_to_string(&opencode_json)
        && let Ok(mut config) = serde_json::from_str::<serde_json::Value>(&content)
        && let Some(agents) = config.get_mut("agent").and_then(|v| v.as_object_mut())
    {
        let framework = framework_agent_names(root);
        let before = agents.len();
        agents.retain(|name, _| !framework.iter().any(|f| f == name));
        report.entries_removed = before - agents.len();
        if report.entries_removed > 0 {
            let serialized = serde_json::to_string_pretty(&config)?;
            std::fs::write(&opencode_json, serialized)?;
        }
    }

    // 2. Framework symlinks (target points into the repo).
    let root_canon = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    for category in ["agents", "skills", "prompts", "workflows"] {
        let dir = editor_dir.join(category);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(target) = std::fs::read_link(&path) {
                let absolute = if target.is_absolute() {
                    target
                } else {
                    path.parent()
                        .map(|parent| parent.join(&target))
                        .unwrap_or(target)
                };
                if absolute.starts_with(&root_canon) {
                    let _ = std::fs::remove_file(&path);
                    report.symlinks_removed += 1;
                } else {
                    report.files_kept += 1;
                }
            } else {
                // Regular file (not a symlink): preserve local-only assets.
                report.files_kept += 1;
            }
        }
    }
    Ok(report)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct UninstallReport {
    editor: String,
    entries_removed: usize,
    symlinks_removed: usize,
    files_kept: usize,
    errors: Vec<String>,
}

// --- Update ---

fn run_dev_update(args: UpdateArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let root = std::fs::canonicalize(&args.root)?;
        let mut output = String::new();

        if root.join(".git").is_dir() {
            // Developer checkout: pull, re-link, rebuild.
            output.push_str(&update_checkout(&root)?);
        } else {
            // Bundle install: download the framework release bundle, verify, extract.
            output.push_str(&update_bundle(&root, &args)?);
        }

        // Re-link the requested editors.
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));
        let opencode_dir = home.join(".config/opencode");
        let zcode_dir = home.join(".zcode");
        if matches!(args.editor, LinkEditor::OpenCode | LinkEditor::All) && opencode_dir.is_dir() {
            let report = link_editor(&root, &opencode_dir);
            output.push_str(&format!(
                "opencode: {} agents, {} skills, {} stale replaced\n",
                report.agents_linked, report.skills_linked, report.stale_replaced
            ));
        }
        if matches!(args.editor, LinkEditor::ZCode | LinkEditor::All) && zcode_dir.is_dir() {
            let report = link_editor(&root, &zcode_dir);
            output.push_str(&format!(
                "zcode: {} agents, {} skills, {} stale replaced\n",
                report.agents_linked, report.skills_linked, report.stale_replaced
            ));
        }

        Ok(output)
    })();
    render_result(result, format, |output: &String| output.clone())
}

/// Update a developer checkout (git work tree): pull, re-link, rebuild.
fn update_checkout(root: &Path) -> anyhow::Result<String> {
    let mut output = String::new();
    let pull = std::process::Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(root)
        .output()?;
    output.push_str(&format!(
        "git pull: {} {}\n",
        if pull.status.success() {
            "ok"
        } else {
            "failed"
        },
        String::from_utf8_lossy(&pull.stderr).trim()
    ));

    let build = std::process::Command::new("cargo")
        .args(["build", "--release", "-p", "sddk-cli"])
        .current_dir(root)
        .output()?;
    output.push_str(&format!(
        "build: {} {}\n",
        if build.status.success() {
            "ok"
        } else {
            "failed"
        },
        String::from_utf8_lossy(&build.stderr)
            .lines()
            .last()
            .unwrap_or("")
            .trim()
    ));
    Ok(output)
}

/// Download and extract the framework release bundle into a bundle install root.
fn update_bundle(root: &Path, args: &UpdateArgs) -> anyhow::Result<String> {
    let version = args.version.as_deref().unwrap_or("latest");
    let base_url = match &args.base_url {
        Some(base) => base.clone(),
        None => format!("https://github.com/{}/releases", args.repo),
    };
    let asset = "sddk-framework.tar.gz";
    let url = if version == "latest" {
        format!("{base_url}/latest/download/{asset}")
    } else {
        format!("{base_url}/download/{version}/{asset}")
    };

    let tmp = std::env::temp_dir().join(format!("sddk-update-{}", std::process::id()));
    let tmp_dir = tmp.join("dl");
    let bundle = tmp_dir.join(asset);
    let checksum = tmp_dir.join(format!("{asset}.sha256"));
    std::fs::create_dir_all(&tmp_dir)?;

    download_to(&url, &bundle)?;
    download_to(&format!("{url}.sha256"), &checksum)?;

    let expected = std::fs::read_to_string(&checksum)?
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("empty checksum file: {}", checksum.display()))?
        .to_owned();
    let actual = sha256_hex(&bundle)?;
    if expected != actual {
        anyhow::bail!("framework sha256 mismatch\n  expected: {expected}\n  actual:   {actual}");
    }

    let extract = std::process::Command::new("tar")
        .args([
            "xzf",
            bundle.to_str().unwrap_or_default(),
            "-C",
            root.to_str().unwrap_or_default(),
        ])
        .output()?;
    if !extract.status.success() {
        anyhow::bail!(
            "extract failed: {}",
            String::from_utf8_lossy(&extract.stderr).trim()
        );
    }
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(format!(
        "framework: {version} ({asset}) sha256 verified: {actual}\n"
    ))
}

/// Download a URL to a destination via curl/wget, or copy from file://.
fn download_to(url: &str, destination: &Path) -> anyhow::Result<()> {
    if let Some(source) = url.strip_prefix("file://") {
        std::fs::copy(source, destination)?;
        return Ok(());
    }
    let status = std::process::Command::new("curl")
        .args(["-fsSL", "--retry", "3", "-o"])
        .arg(destination)
        .arg(url)
        .status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => anyhow::bail!("curl exited {status} for {url}"),
        Err(_) => {
            let status = std::process::Command::new("wget")
                .args(["-qO"])
                .arg(destination)
                .arg(url)
                .status()?;
            if status.success() {
                Ok(())
            } else {
                anyhow::bail!("wget exited {status} for {url}")
            }
        }
    }
}

/// Compute the plain lowercase hex SHA-256 of a file.
fn sha256_hex(path: &Path) -> anyhow::Result<String> {
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}
