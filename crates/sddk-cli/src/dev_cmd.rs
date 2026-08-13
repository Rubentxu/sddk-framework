//! Developer tooling: environment doctor, gates, and atomic install/verify.

use std::path::{Path, PathBuf};

use clap::{Args, Subcommand, ValueEnum};
use sddk_gateway::{PermissionPolicy, RunSpec, run};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{CliEnvironment, CommandOutput, OutputFormat, render_result};

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

/// Framework surfaces covered by the manifest (agents, skills, prompts,
/// workflows, assets). Relative to the framework root.
const MANIFEST_SURFACES: [&str; 5] = [
    "agents",
    "skills",
    "prompts/sddk",
    "prompts/sddk/workflows",
    "assets",
];

/// Manifest file name, written at the framework root (and shipped in the
/// release bundle).
pub(crate) const MANIFEST_FILE: &str = "MANIFEST.sha256";

#[derive(Debug, Clone, Args)]
pub(crate) struct ManifestArgs {
    /// Framework root to scan (default: current directory).
    #[arg(long)]
    pub(crate) root: Option<PathBuf>,
    /// Verify an existing manifest instead of generating one.
    #[arg(long)]
    pub(crate) verify: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct UseArgs {
    /// Version to activate (installed bundle) or `path:<dir>` for dogfooding.
    #[arg(long, required_unless_present = "show")]
    pub(crate) version: Option<String>,
    /// Show the active version without changing it.
    #[arg(long)]
    pub(crate) show: bool,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

#[derive(Debug, Clone, Args)]
pub(crate) struct DoctorArgs {
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
    /// Strict mode: exit 1 when any surface brevity check reports a file over threshold.
    #[arg(long)]
    pub(crate) strict: bool,
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

pub(crate) fn run_dev(command: DevCommand, environment: &CliEnvironment) -> CommandOutput {
    match command {
        DevCommand::Doctor(args) => run_dev_doctor(args, environment),
        DevCommand::Check(args) => run_dev_check(args),
        DevCommand::Install(args) => run_dev_install(args),
        DevCommand::Verify(args) => run_dev_verify(args),
        DevCommand::Uninstall(args) => run_dev_uninstall(args),
        DevCommand::Link(args) => run_dev_link(args, environment),
        DevCommand::Use(args) => run_dev_use(args, environment),
        DevCommand::Update(args) => run_dev_update(args, environment),
        DevCommand::Manifest(args) => run_dev_manifest(args),
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

fn run_dev_doctor(args: DoctorArgs, environment: &CliEnvironment) -> CommandOutput {
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
    // Runtime assets integrity: the CLI resolves dashboard kit + UAT drivers
    // from the active framework bundle (ADR-013). A dev update without asset
    // sync leaves stale/missing assets that break `uat dashboard` and
    // `uat run --executor playwright|computer_use` at runtime.
    if let Ok(framework_root) = resolve_active_framework_root(environment) {
        let assets = framework_root.join("assets");
        let driver_ok = assets.join("uat-driver/driver.mjs").is_file()
            && assets.join("uat-driver/computer_use.mjs").is_file()
            && assets.join("uat-driver/assess.mjs").is_file();
        let kit_ok = assets.join("uat-dashboard/kit/components.js").is_file()
            && assets.join("uat-dashboard/views/guided.html").is_file();
        checks.push(DoctorCheck {
            tool: "assets.uat-driver".into(),
            present: driver_ok,
        });
        checks.push(DoctorCheck {
            tool: "assets.uat-dashboard-kit".into(),
            present: kit_ok,
        });
        if !driver_ok || !kit_ok {
            framework_warnings += 1;
        }
        // Content integrity: verify the active framework root against its
        // MANIFEST.sha256 (per-file hashes of agents/skills/prompts/
        // workflows/assets — the same manifest shipped with the release).
        // A missing manifest is informational (pre-manifest bundles), not a
        // failure; a present-but-mismatched manifest is a real problem.
        let manifest_status = verify_manifest(&framework_root);
        let (manifest_present, manifest_ok) = match &manifest_status {
            Ok(mismatches) => (true, mismatches.is_empty()),
            Err(_) => (false, true),
        };
        checks.push(DoctorCheck {
            tool: "content.manifest".into(),
            present: manifest_ok,
        });
        if manifest_present && !manifest_ok {
            framework_warnings += 1;
        }
    }

    // Surface brevity checks (ADR-016): agent ≤ 300, skill ≤ 150, prompt ≤ 200.
    let root = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut brevity_violations = 0usize;

    // Agents: agents/*.md
    if let Ok(entries) = std::fs::read_dir(root.join("agents")) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("md")
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let line_count = content.lines().count();
                let rel = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let present = line_count <= 300;
                if !present {
                    brevity_violations += 1;
                }
                checks.push(DoctorCheck {
                    tool: format!("surface.briefness.{rel}"),
                    present,
                });
            }
        }
    }

    // Skills: skills/*/SKILL.md
    if let Ok(entries) = std::fs::read_dir(root.join("skills")) {
        for entry in entries.flatten() {
            let skill_dir = entry.path();
            if skill_dir.is_dir() {
                let skill_name = skill_dir
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");
                let skl_path = skill_dir.join("SKILL.md");
                if skl_path.is_file()
                    && let Ok(content) = std::fs::read_to_string(&skl_path)
                {
                    let line_count = content.lines().count();
                    let present = line_count <= 150;
                    if !present {
                        brevity_violations += 1;
                    }
                    checks.push(DoctorCheck {
                        tool: format!("surface.briefness.{skill_name}/SKILL.md"),
                        present,
                    });
                }
            }
        }
    }

    // Prompts: prompts/sddk/*.md
    let prompts_dir = root.join("prompts/sddk");
    if prompts_dir.is_dir()
        && let Ok(entries) = std::fs::read_dir(&prompts_dir)
    {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file()
                && path.extension().and_then(|e| e.to_str()) == Some("md")
                && let Ok(content) = std::fs::read_to_string(&path)
            {
                let line_count = content.lines().count();
                let rel = path
                    .strip_prefix(&prompts_dir)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .replace('\\', "/");
                let present = line_count <= 200;
                if !present {
                    brevity_violations += 1;
                }
                checks.push(DoctorCheck {
                    tool: format!("surface.briefness.{rel}"),
                    present,
                });
            }
        }
    }

    // Surface empty-dirs check (ADR-016): no empty subdirectories in surfaces.
    for surface_dir in ["agents", "skills", "prompts/sddk"] {
        let dir_path = root.join(surface_dir);
        if dir_path.is_dir()
            && let Ok(entries) = std::fs::read_dir(&dir_path)
        {
            for entry in entries.flatten() {
                let path = entry.path();
                // Only check directories (not files).
                if path.is_dir() {
                    let rel = path
                        .strip_prefix(&root)
                        .unwrap_or(&path)
                        .to_string_lossy()
                        .replace('\\', "/");
                    // Empty if no files (recursive check).
                    let is_empty = path
                        .read_dir()
                        .map(|mut i| i.next().is_none())
                        .unwrap_or(false);
                    let present = !is_empty;
                    checks.push(DoctorCheck {
                        tool: format!("surface.empty_dirs.{rel}"),
                        present,
                    });
                }
            }
        }
    }

    // `all_present` reflects only non-brevity checks (framework layout).
    // Brevity violations are tracked separately via `brevity_violations` and
    // only affect the exit code in strict mode (ADR-016 §4).
    let all_present = framework_warnings == 0;
    let result = Ok::<_, anyhow::Error>(DoctorOutput {
        all_present,
        checks,
    });
    match result {
        Ok(output) => {
            let cloned = DoctorOutput {
                all_present: output.all_present,
                checks: output.checks.clone(),
            };
            let mut command = render_result(Ok(cloned), format, doctor_text);
            // Strict mode: only brevity violations trigger non-zero exit (ADR-016 §4).
            // surface.empty_dirs is detect-only advisory — never promoted by --strict.
            if args.strict && brevity_violations > 0 {
                command.status = 1;
            } else if !output.all_present {
                // Advisory: non-brevity layout issues are fatal.
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
        // Routing: if the prefix already terminates in `/bin`, install the
        // binary directly under the prefix (no extra `bin/` segment); this
        // matches the GNU autoconf/CMake convention of `--prefix=/opt/sdk/bin`
        // meaning "the binary directory". Otherwise nest under `bin/`.
        let ends_with_bin = args
            .prefix
            .file_name()
            .and_then(|name| name.to_str())
            == Some("bin");
        let target_dir = if ends_with_bin {
            args.prefix.clone()
        } else {
            args.prefix.join("bin")
        };
        std::fs::create_dir_all(&target_dir)?;
        let destination = target_dir.join("sddk");
        // Mode 0o755 BEFORE rename so the binary is born executable — fixes
        // the chmod-less atomic write that left ELF files at 0644.
        atomic_write(&destination, &bytes, Some(0o755))?;
        let binary_path = if ends_with_bin {
            "sddk".to_owned()
        } else {
            "bin/sddk".to_owned()
        };

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
            binary_path,
        };
        let receipt_path = args.prefix.join(RECEIPT_FILE);
        atomic_write(
            &receipt_path,
            serde_json::to_string_pretty(&receipt)?.as_bytes(),
            None,
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

fn atomic_write(destination: &Path, bytes: &[u8], mode: Option<u32>) -> anyhow::Result<()> {
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
                let result = (|| -> std::io::Result<()> {
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    drop(file);
                    // chmod BEFORE rename so the destination is born with
                    // the requested mode (no 0644 window). Unix-only.
                    #[cfg(unix)]
                    {
                        if let Some(bits) = mode {
                            use std::os::unix::fs::PermissionsExt;
                            std::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(bits))?;
                        }
                    }
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
    /// Entries removed from the editor because they no longer exist in the
    /// framework source (deprecated/renamed surfaces).
    pruned: usize,
    errors: Vec<String>,
}

fn link_text(report: &LinkReport) -> String {
    format!(
        "editor: {}\nagents: {}\nskills: {}\nprompts: {}\nworkflows: {}\nstale_replaced: {}\npruned: {}\nerrors: {}\n",
        report.editor,
        report.agents_linked,
        report.skills_linked,
        report.prompts_linked,
        report.workflows_linked,
        report.stale_replaced,
        report.pruned,
        report.errors.len()
    )
}

/// Replace a regular-file copy with a symlink, backing up the stale copy.
fn link_file(source: &Path, target: &Path, stale_replaced: &mut usize) -> std::io::Result<()> {
    if let Ok(metadata) = std::fs::symlink_metadata(target) {
        if metadata.file_type().is_symlink() {
            // Already a link: only recreate when it points somewhere else
            // (hash-free check: compare canonical target).
            if let Ok(current) = std::fs::read_link(target) {
                let source_abs =
                    std::fs::canonicalize(source).unwrap_or_else(|_| source.to_path_buf());
                let current_abs = if current.is_absolute() {
                    current
                } else {
                    target
                        .parent()
                        .unwrap_or_else(|| Path::new("/"))
                        .join(current)
                };
                let current_abs = std::fs::canonicalize(&current_abs).unwrap_or(current_abs);
                if current_abs == source_abs {
                    // Correct target already: leave it (idempotent).
                    return Ok(());
                }
            }
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

/// Prune editor entries that the framework used to manage but no longer
/// ships: broken symlinks, `.sddk-stale` backups left by previous links, and
/// namespaced entries (sddk-*/sdd-*/gentle-*) absent from the source tree.
/// Entries NOT namespaced by the framework (e.g. arch-stack skills) are
/// never touched — they belong to other systems.
fn prune_editor(root: &Path, editor_dir: &Path) -> usize {
    let framework_namespace = |name: &std::ffi::OsStr| {
        let name = name.to_string_lossy();
        name.starts_with("sddk-")
            || name.starts_with("sdd-")
            || name.starts_with("gentle-")
            || name == "orchestrator.md"
    };
    let mut pruned = 0usize;
    let mut prune_entry = |path: &Path, source: &Path, name: &std::ffi::OsStr| -> bool {
        let is_framework_entry = framework_namespace(name);
        let exists_in_source = source.join(name).exists();
        let is_broken_link = path
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink() && !path.exists())
            .unwrap_or(false);
        let is_stale_backup = name.to_string_lossy().ends_with(".sddk-stale");
        let should_remove =
            is_broken_link || is_stale_backup || (is_framework_entry && !exists_in_source);
        if should_remove {
            let _ = std::fs::remove_file(path);
            let _ = std::fs::remove_dir_all(path);
            pruned += 1;
        }
        should_remove
    };

    // Agents: symlinks or markdown files in <editor>/agents.
    let source_agents = root.join("agents");
    let target_agents = editor_dir.join("agents");
    if let Ok(entries) = std::fs::read_dir(&target_agents) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let is_md = entry.path().extension().and_then(|e| e.to_str()) == Some("md");
            if is_md {
                prune_entry(&entry.path(), &source_agents, &name);
            }
        }
    }
    // Skills: directories or top-level markdown in <editor>/skills.
    let source_skills = root.join("skills");
    let target_skills = editor_dir.join("skills");
    if let Ok(entries) = std::fs::read_dir(&target_skills) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let path = entry.path();
            let is_skill = entry.file_type().map(|t| t.is_dir()).unwrap_or(false)
                || path.extension().and_then(|e| e.to_str()) == Some("md");
            if is_skill {
                prune_entry(&path, &source_skills, &name);
            }
        }
    }
    // Prompts: canonical SDDK tree mirrors source.
    let source_prompts = root.join("prompts/sddk");
    let target_prompts = editor_dir.join("prompts/sddk");
    if let Ok(entries) = std::fs::read_dir(&target_prompts) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let is_md = entry.path().extension().and_then(|e| e.to_str()) == Some("md");
            if is_md {
                prune_entry(&entry.path(), &source_prompts, &name);
            }
        }
    }
    // Workflows: yaml files in <editor>/workflows.
    let source_workflows = root.join("prompts/sddk/workflows");
    let target_workflows = editor_dir.join("workflows");
    if let Ok(entries) = std::fs::read_dir(&target_workflows) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let path = entry.path();
            let ext = path.extension().and_then(|e| e.to_str()).map(str::to_owned);
            let is_workflow_entry = matches!(ext.as_deref(), Some("yaml") | Some("yml"));
            let is_stale_backup = name.to_string_lossy().ends_with(".sddk-stale");
            if is_workflow_entry || is_stale_backup {
                prune_entry(&path, &source_workflows, &name);
            }
        }
    }
    pruned
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
        pruned: 0,
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

    // Prompts (canonical SDDK tree).
    let prompts_source = root.join("prompts/sddk");
    let prompts_target = editor_dir.join("prompts/sddk");
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
    let workflows_source = root.join("prompts/sddk/workflows");
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
    // Reconcile: remove editor entries that the framework no longer ships
    // (deprecated surfaces, broken links, stale backups) — while never
    // touching entries namespaced by other systems (arch-stack, books...).
    report.pruned = prune_editor(root, editor_dir);
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

/// Copy the framework assets tree (kit, drivers, themes) from `source` into
/// `target`, preserving relative paths. Returns the number of files copied.
fn sync_assets(source: &Path, target: &Path) -> anyhow::Result<usize> {
    let mut copied = 0usize;
    std::fs::create_dir_all(target)?;
    for entry in walk_dir(source) {
        let relative = entry
            .strip_prefix(source)
            .unwrap_or(entry.as_path())
            .to_path_buf();
        let destination = target.join(&relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        // Only overwrite when content differs (idempotent, avoids touching
        // mtimes on every dev update).
        let needs_copy = match (std::fs::read(&entry), std::fs::read(&destination)) {
            (Ok(src), Ok(dst)) => src != dst,
            _ => true,
        };
        if needs_copy {
            std::fs::copy(&entry, &destination)?;
        }
        copied += 1;
    }
    Ok(copied)
}

/// Collect every managed file of the framework (surfaces in
/// `MANIFEST_SURFACES`) as `(relative_path, sha256_hex)`.
fn manifest_entries(root: &Path) -> anyhow::Result<Vec<(String, String)>> {
    let mut entries = Vec::new();
    for surface in MANIFEST_SURFACES {
        let dir = root.join(surface);
        if !dir.is_dir() {
            continue;
        }
        for file in walk_dir(&dir) {
            if !file.is_file() {
                continue;
            }
            let relative = file
                .strip_prefix(root)
                .unwrap_or(file.as_path())
                .to_string_lossy()
                .replace('\\', "/");
            let digest = sha256_hex(&file)?;
            entries.push((relative, digest));
        }
    }
    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

/// Serialize manifest entries as `sha256  relative-path` lines (sha256sum
/// compatible, one entry per line).
fn manifest_lines(entries: &[(String, String)]) -> String {
    entries
        .iter()
        .map(|(path, digest)| format!("{digest}  {path}"))
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

/// Generate MANIFEST.sha256 at the framework root. Returns the number of
/// hashed files.
fn write_manifest(root: &Path) -> anyhow::Result<usize> {
    let entries = manifest_entries(root)?;
    let content = manifest_lines(&entries);
    let target = root.join(MANIFEST_FILE);
    atomic_write(&target, content.as_bytes(), None)?;
    Ok(entries.len())
}

/// Verify a framework root against its MANIFEST.sha256. Returns the list of
/// mismatches (empty = intact). A missing manifest is reported as a single
/// entry.
pub(crate) fn verify_manifest(root: &Path) -> anyhow::Result<Vec<String>> {
    let manifest_path = root.join(MANIFEST_FILE);
    let raw = std::fs::read_to_string(&manifest_path)?;
    let mut mismatches = Vec::new();
    for line in raw.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let (expected, relative) = line
            .split_once("  ")
            .ok_or_else(|| anyhow::anyhow!("malformed manifest line: {line}"))?;
        let file = root.join(relative);
        if !file.is_file() {
            mismatches.push(format!("{relative}: missing"));
            continue;
        }
        let actual = sha256_hex(&file)?;
        if actual != expected {
            mismatches.push(format!("{relative}: hash mismatch"));
        }
    }
    Ok(mismatches)
}

/// Run the `dev manifest` subcommand.
fn run_dev_manifest(args: ManifestArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let root = args
            .root
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
        if args.verify {
            let mismatches = verify_manifest(&root)?;
            if mismatches.is_empty() {
                return Ok(format!(
                    "manifest OK: {} verified against {}",
                    root.display(),
                    root.join(MANIFEST_FILE).display()
                ));
            }
            anyhow::bail!(
                "manifest verification FAILED ({}):\n  {}",
                mismatches.len(),
                mismatches.join("\n  ")
            );
        }
        let count = write_manifest(&root)?;
        Ok(format!(
            "manifest written: {} ({} files hashed)",
            root.join(MANIFEST_FILE).display(),
            count
        ))
    })();
    render_result(result, format, |t| t.clone())
}

fn run_dev_link(args: LinkArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<Vec<LinkReport>> {
        // When no explicit root is given, link from the active framework
        // bundle (`$SDDK_DATA_DIR/framework/current`, asdf-style). An explicit
        // `--root` still links from a repo/checkout (dogfooding) and syncs
        // its assets into the active bundle so the CLI resolves them.
        let root = if args.root.as_os_str() == "." {
            resolve_active_framework_root(environment)?
        } else {
            std::fs::canonicalize(&args.root)?
        };
        // Dogfooding from an explicit source root: sync its assets into the
        // active bundle so `uat dashboard`/`uat run` resolve the latest kit
        // and drivers, then regenerate the runtime manifest. This never
        // touches git — the source is whatever the user points at.
        if args.root.as_os_str() != "."
            && let Ok(framework_root) = resolve_active_framework_root(environment)
            && root.join("assets").is_dir()
        {
            let _ = sync_assets(&root.join("assets"), &framework_root.join("assets"));
            let _ = write_manifest(&framework_root);
        }
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

/// Data root of the framework: `SDDK_DATA_DIR` override, else XDG data dir
/// (`~/.local/share/sddk` on Linux, platform dir on macOS/Windows).
fn sddk_data_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    if let Some(dir) = &environment.sddk_data_dir {
        return Ok(dir.clone());
    }
    let data_home = match (&environment.data_home, &environment.home) {
        (Some(data), _) => data.clone(),
        (None, Some(home)) => home.join(".local/share"),
        (None, None) => dirs::data_dir().ok_or_else(|| {
            anyhow::anyhow!("no data root: set HOME, XDG_DATA_HOME or SDDK_DATA_DIR")
        })?,
    };
    Ok(data_home.join("sddk"))
}

/// The `framework/` dir inside the data root (bundles per version + `current`).
fn framework_dir(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    Ok(sddk_data_dir(environment)?.join("framework"))
}

/// Resolve the active framework root: `current` symlink target, else the
/// latest installed version, else the data dir (empty).
fn resolve_active_framework_root(environment: &CliEnvironment) -> anyhow::Result<PathBuf> {
    let dir = framework_dir(environment)?;
    let current = dir.join("current");
    if let Ok(target) = std::fs::read_link(&current) {
        if target.is_absolute() {
            return Ok(target);
        }
        return Ok(dir.join(target));
    }
    // Fall back to the highest installed version.
    let mut versions: Vec<String> = std::fs::read_dir(&dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter_map(|entry| entry.file_name().into_string().ok())
                .filter(|name| name != "current")
                .collect()
        })
        .unwrap_or_default();
    versions.sort();
    versions
        .last()
        .map(|version| dir.join(version))
        .ok_or_else(|| {
            anyhow::anyhow!("no framework bundle installed; run `sddk dev update --root <dir>`")
        })
}

/// Resolve the static `assets/` directory of the active framework root
/// (ADR-0013: dashboard kit shipped in the bundle). Returns `None` when the
/// bundle has no assets (pre-1.5.0 bundles are still supported).
pub(crate) fn resolve_assets_dir(environment: &CliEnvironment) -> anyhow::Result<Option<PathBuf>> {
    let root = resolve_active_framework_root(environment)?;
    let assets = root.join("assets");
    if assets.is_dir() {
        return Ok(Some(assets));
    }
    // Dogfooding fallback: when running from the framework development repo
    // (which carries `manifest.toml` and an `assets/` tree), resolve there.
    let cwd = std::env::current_dir().unwrap_or_default();
    if cwd.join("manifest.toml").is_file() {
        let repo_assets = cwd.join("assets");
        if repo_assets.is_dir() {
            return Ok(Some(repo_assets));
        }
    }
    Ok(None)
}

#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
struct UseOutput {
    version: String,
    current: String,
}

fn run_dev_use(args: UseArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<UseOutput> {
        let dir = framework_dir(environment)?;
        std::fs::create_dir_all(&dir)?;
        let current = dir.join("current");
        if args.show {
            let active = match std::fs::read_link(&current) {
                Ok(target) => target
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| target.to_string_lossy().into_owned()),
                Err(_) => "none".to_owned(),
            };
            return Ok(UseOutput {
                version: active.clone(),
                current: active,
            });
        }
        // Resolve the target: `path:<dir>` points at a working tree
        // (dogfooding); otherwise a bundle version under framework/<version>/.
        let target = if let Some(path) = args
            .version
            .as_deref()
            .and_then(|version| version.strip_prefix("path:"))
        {
            std::fs::canonicalize(path)?
        } else {
            let version = args
                .version
                .clone()
                .ok_or_else(|| anyhow::anyhow!("--version is required unless --show"))?;
            let version_dir = dir.join(&version);
            if !version_dir.is_dir() {
                anyhow::bail!(
                    "bundle version {version} not installed; run `sddk dev update --root <dir> --version {version}`",
                    version = version
                );
            }
            version_dir
        };
        // Atomically swap the `current` symlink.
        let tmp = dir.join("current.tmp");
        let _ = std::fs::remove_file(&tmp);
        std::os::unix::fs::symlink(&target, &tmp)?;
        std::fs::rename(&tmp, &current)?;
        Ok(UseOutput {
            version: args.version.unwrap_or_else(|| "current".to_owned()),
            current: target.to_string_lossy().into_owned(),
        })
    })();
    render_result(result, format, use_text)
}

fn use_text(output: &UseOutput) -> String {
    format!("version: {}\ncurrent: {}\n", output.version, output.current)
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
    let agents_dir = root.join("agents");
    // Prefer the permission policy when present; fall back to the actual
    // agent files (release bundles may omit permissions.yaml).
    if let Ok(policy) = PermissionPolicy::from_file(root.join("permissions.yaml")) {
        let mut names: Vec<String> = policy
            .agents()
            .filter(|name| agents_dir.join(format!("{name}.md")).exists())
            .map(str::to_owned)
            .collect();
        names.sort();
        return names;
    }
    let mut names: Vec<String> = std::fs::read_dir(&agents_dir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|entry| entry.path().extension().and_then(|e| e.to_str()) == Some("md"))
                .filter_map(|entry| {
                    entry
                        .path()
                        .file_stem()
                        .map(|stem| stem.to_string_lossy().into_owned())
                })
                .collect()
        })
        .unwrap_or_default();
    names.sort();
    names
}

/// Orchestrator agents registered as primary (user-selectable) agents in
/// opencode; every other framework agent stays a hidden subagent.
const PRIMARY_AGENTS: [&str; 2] = ["orchestrator", "book-orchestrator"];

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
    // Prune orphaned registrations: framework-namespaced agents that no
    // longer exist in the source tree (renamed/removed surfaces). Entries
    // from other systems are left untouched.
    let source_agent_names: std::collections::HashSet<String> =
        framework_agent_names(root).into_iter().collect();
    let orphans: Vec<String> = agents
        .keys()
        .filter(|name| {
            let name = name.as_str();
            (name.starts_with("sddk-") || name.starts_with("sdd-") || name.starts_with("gentle-"))
                && !source_agent_names.contains(name)
        })
        .cloned()
        .collect();
    for orphan in orphans {
        agents.remove(&orphan);
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

fn run_dev_update(args: UpdateArgs, environment: &CliEnvironment) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<String> {
        let mut output = String::new();

        // The framework distributes RELEASE BUNDLES (agents/skills/prompts/
        // workflows/assets + MANIFEST.sha256), never repository clones. Git
        // operations are the developer's responsibility: if the target root
        // is a checkout, the user updates it with `git pull` themselves and
        // re-links with `sddk dev link --root <checkout>` (dogfooding).
        if args.root.join(".git").is_dir() {
            anyhow::bail!(
                "`dev update` installs release bundles and never touches git. \
                 You passed a repository checkout ({}). \
                 To update a checkout, run `git pull` yourself, then \
                 `sddk dev link --root {}` to re-link the editors.",
                args.root.display(),
                args.root.display()
            );
        }

        // Bundle install: download the framework release bundle, verify, and
        // extract into `$SDDK_DATA_DIR/framework/<version>/` (asdf-style
        // installs/<tool>/<version>). The bundle root defaults to the data
        // root when --root is not an explicit existing dir.
        let bundle_root = if args.root.as_os_str() == "." {
            framework_dir(environment)?
        } else {
            std::fs::canonicalize(&args.root).unwrap_or(args.root.clone())
        };
        output.push_str(&update_bundle(&bundle_root, &args)?);

        // The extracted bundle lands in a version dir; update_bundle extracts
        // directly into bundle_root, so if the user passed the framework root
        // we additionally fix the `current` symlink to point at it.
        if args.root.as_os_str() == "." {
            let current = bundle_root.join("current");
            let tmp = bundle_root.join("current.tmp");
            let _ = std::fs::remove_file(&tmp);
            if let Ok(target) = std::fs::read_link(&current) {
                let _ = target;
            }
            let _ = std::fs::remove_file(&current);
            std::os::unix::fs::symlink(&bundle_root, &tmp)?;
            std::fs::rename(&tmp, &current)?;
            output.push_str("framework: current -> bundle root (dev link resolves it)\n");
        }
        Ok(output)
    })();
    render_result(result, format, |output: &String| output.clone())
}

/// Re-link the requested editors from a given framework root.
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
    // Post-extract integrity: verify every file of the extracted bundle
    // against the manifest that SHIPPED INSIDE the tarball. The tarball
    // checksum proves transport integrity; the internal manifest proves
    // content integrity of each framework surface (agents, skills, prompts,
    // workflows, assets) — no repository clone involved, only the release
    // surfaces (ADR-011).
    let manifest_path = root.join(MANIFEST_FILE);
    if manifest_path.is_file() {
        match verify_manifest(root) {
            Ok(mismatches) if mismatches.is_empty() => {}
            Ok(mismatches) => {
                let _ = std::fs::remove_dir_all(&tmp);
                anyhow::bail!(
                    "bundle content verification FAILED ({} mismatch(es)):\n  {}",
                    mismatches.len(),
                    mismatches.join("\n  ")
                );
            }
            Err(e) => {
                let _ = std::fs::remove_dir_all(&tmp);
                anyhow::bail!("bundle manifest unreadable: {e}");
            }
        }
    }
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(format!(
        "framework: {version} ({asset}) sha256 verified: {actual}; {} files content-verified via {MANIFEST_FILE}\n",
        count_manifest_entries(root).unwrap_or(0)
    ))
}

/// Count entries in a root's MANIFEST.sha256 (0 when absent).
fn count_manifest_entries(root: &Path) -> anyhow::Result<usize> {
    let raw = std::fs::read_to_string(root.join(MANIFEST_FILE))?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).count())
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

#[cfg(test)]
mod reconciliation_tests {
    use super::*;

    fn temp_tree(tag: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("sddk-link-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn prune_removes_framework_deprecated_but_keeps_foreign() {
        let root = temp_tree("root");
        let editor = temp_tree("editor");
        // Framework source: one agent + one skill.
        std::fs::create_dir_all(root.join("agents")).unwrap();
        std::fs::write(
            root.join("agents/orchestrator.md"),
            "---\nname: orchestrator\n---\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("skills")).unwrap();
        std::fs::create_dir_all(root.join("skills/sddk-apply")).unwrap();
        std::fs::write(root.join("skills/sddk-apply/SKILL.md"), "# apply\n").unwrap();

        // Editor state: broken framework link, orphan namespaced skill,
        // stale backup, and a foreign (arch-stack) skill that must survive.
        std::fs::create_dir_all(editor.join("agents")).unwrap();
        std::fs::create_dir_all(editor.join("skills")).unwrap();
        std::fs::create_dir_all(editor.join("workflows")).unwrap();
        std::os::unix::fs::symlink(
            "/nonexistent/sddk-deprecated.md",
            editor.join("agents/sddk-deprecated.md"),
        )
        .unwrap();
        std::fs::create_dir_all(editor.join("skills/sddk-continue-options")).unwrap();
        std::fs::write(
            editor.join("skills/sddk-continue-options/SKILL.md"),
            "# orphan\n",
        )
        .unwrap();
        std::fs::write(editor.join("workflows/sddk-a-full.sddk-stale"), "stale\n").unwrap();
        std::fs::create_dir_all(editor.join("skills/architecture-discovery")).unwrap();
        std::fs::write(
            editor.join("skills/architecture-discovery/SKILL.md"),
            "# foreign\n",
        )
        .unwrap();

        let pruned = prune_editor(&root, &editor);
        // 1 broken agent + 1 orphan skill + 1 stale workflow = 3.
        assert_eq!(pruned, 3);
        assert!(!editor.join("agents/sddk-deprecated.md").exists());
        assert!(!editor.join("skills/sddk-continue-options").exists());
        assert!(!editor.join("workflows/sddk-a-full.sddk-stale").exists());
        // Foreign surface untouched.
        assert!(
            editor
                .join("skills/architecture-discovery/SKILL.md")
                .exists()
        );

        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&editor).ok();
    }

    #[test]
    fn link_file_is_idempotent_when_target_matches() {
        let dir = temp_tree("link");
        let source = dir.join("source.md");
        let target = dir.join("target.md");
        std::fs::write(&source, "content").unwrap();
        let mut stale = 0usize;
        link_file(&source, &target, &mut stale).unwrap();
        let mtime1 = std::fs::metadata(&target).unwrap().modified().unwrap();
        link_file(&source, &target, &mut stale).unwrap();
        let mtime2 = std::fs::metadata(&target).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2, "correct symlink must not be recreated");
        assert_eq!(stale, 0);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn link_file_replaces_stale_copy_with_backup() {
        let dir = temp_tree("stale");
        let source = dir.join("source.md");
        let target = dir.join("target.md");
        std::fs::write(&source, "new").unwrap();
        std::fs::write(&target, "old copy").unwrap();
        let mut stale = 0usize;
        link_file(&source, &target, &mut stale).unwrap();
        assert_eq!(stale, 1);
        assert!(dir.join("target.sddk-stale").exists());
        assert!(
            std::fs::symlink_metadata(&target)
                .unwrap()
                .file_type()
                .is_symlink()
        );
        std::fs::remove_dir_all(&dir).ok();
    }
}

#[cfg(test)]
mod manifest_tests {
    use super::*;

    fn temp_root(tag: &str) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("sddk-manifest-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn manifest_generates_and_verifies() {
        let root = temp_root("gen");
        std::fs::create_dir_all(root.join("agents")).unwrap();
        std::fs::write(root.join("agents/a.md"), "content-a").unwrap();
        std::fs::create_dir_all(root.join("skills/sddk-x")).unwrap();
        std::fs::write(root.join("skills/sddk-x/SKILL.md"), "content-x").unwrap();

        let count = write_manifest(&root).unwrap();
        assert_eq!(count, 2);
        assert!(root.join(MANIFEST_FILE).is_file());
        let mismatches = verify_manifest(&root).unwrap();
        assert!(
            mismatches.is_empty(),
            "intact tree must verify: {mismatches:?}"
        );
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn manifest_detects_tampering() {
        let root = temp_root("tamper");
        std::fs::create_dir_all(root.join("agents")).unwrap();
        std::fs::write(root.join("agents/a.md"), "content-a").unwrap();
        write_manifest(&root).unwrap();
        // Tamper after manifest generation.
        std::fs::write(root.join("agents/a.md"), "content-TAMPERED").unwrap();
        let mismatches = verify_manifest(&root).unwrap();
        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].contains("agents/a.md"));
        assert!(mismatches[0].contains("hash mismatch"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn manifest_detects_missing_file() {
        let root = temp_root("missing");
        std::fs::create_dir_all(root.join("agents")).unwrap();
        std::fs::write(root.join("agents/a.md"), "content-a").unwrap();
        write_manifest(&root).unwrap();
        std::fs::remove_file(root.join("agents/a.md")).unwrap();
        let mismatches = verify_manifest(&root).unwrap();
        assert_eq!(mismatches.len(), 1);
        assert!(mismatches[0].contains("missing"));
        std::fs::remove_dir_all(&root).ok();
    }
}
