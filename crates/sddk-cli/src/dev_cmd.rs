//! Developer tooling: environment doctor, gates, and atomic install/verify.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand};
use sddk_gateway::{RunSpec, run};
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
    /// Installation prefix directory.
    #[arg(long)]
    pub(crate) prefix: PathBuf,
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
    let result = Ok::<_, anyhow::Error>(DoctorOutput {
        all_present: checks.iter().all(|check| check.present),
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
    let result = (|| -> anyhow::Result<bool> {
        let receipt = read_receipt(&args.prefix)?;
        let binary_path = args.prefix.join(&receipt.binary_path);
        let bytes = std::fs::read(&binary_path)?;
        let digest = format!("sha256:{:x}", Sha256::digest(&bytes));
        if digest != receipt.binary_sha256 {
            anyhow::bail!("refusing to uninstall: binary does not match the receipt");
        }
        std::fs::remove_file(&binary_path)?;
        std::fs::remove_file(args.prefix.join(RECEIPT_FILE))?;
        Ok(true)
    })();
    render_result(result, format, |removed| format!("removed: {removed}\n"))
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
