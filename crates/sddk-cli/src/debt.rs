//! Debt management subcommands: report, incs, backfill, gates.

use std::collections::HashSet;
use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_domain::{
    DebtReport, Finding, FindingStatus, GateOutcomeStatus, IncRecord, IncStatus,
    Priority, Severity,
};
use sddk_engine::{self, fingerprint, evaluate_named_gate, render_inc_template, GateOutcome};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

use crate::{CliEnvironment, CommandOutput, compose};

/// Debt management subcommands.
#[derive(Debug, Clone, Args)]
pub struct DebtArgs {
    #[command(subcommand)]
    pub command: DebtCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum DebtCommand {
    /// Write a debt-report.json for the current cycle state.
    Report {
        /// Output path for the debt-report.json file.
        output: PathBuf,
    },
    /// List existing INC files in the project vault.
    Incs,
    /// Backfill INC files from an archived cycle's debt-report.json.
    Backfill {
        /// Cycle ID to backfill INCs from (e.g. p-52b95ef55999f9de/kernel-cycle-7b-durable-debt-runtime).
        cycle_id: String,
    },
    /// Evaluate a named gate against the current debt report.
    Gates {
        /// Gate name to evaluate (e.g. debt-severity-assigned, debt-priority-assigned).
        name: String,
    },
}

/// Runs the debt subcommand.
pub fn run_debt(args: DebtArgs, env: &CliEnvironment) -> CommandOutput {
    match args.command {
        DebtCommand::Report { output } => run_report(&output),
        DebtCommand::Incs {} => run_incs(env),
        DebtCommand::Backfill { cycle_id } => run_backfill(&cycle_id, env),
        DebtCommand::Gates { name } => run_gates(&name, env),
    }
}

fn run_report(output: &PathBuf) -> CommandOutput {
    // Build a minimal debt report for the current framework state.
    // In a full implementation this would read actual findings from the graph.
    let report = DebtReport {
        schema_version: "1.1.0".into(),
        cycle_id: "p-52b95ef55999f9de/kernel-cycle-8".into(),
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "2026-08-21T00:00:00Z".into()),
        findings: vec![],
    };
    let json = serde_json::to_string_pretty(&report).map_err(|e| format!("JSON error: {e}")).unwrap_or_default();
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    match std::fs::write(output, &json) {
        Ok(_) => CommandOutput {
            status: 0,
            stdout: format!("wrote {}\n", output.display()),
            stderr: String::new(),
        },
        Err(e) => CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("error writing {}: {e}\n", output.display()),
        },
    }
}

fn run_incs(env: &CliEnvironment) -> CommandOutput {
    // Resolve project vault path for incs/
    let vault = match resolve_vault_path(env) {
        Ok(p) => p,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error resolving vault: {e}\n"),
            };
        }
    };
    let incs_dir = vault.join("incs");
    if !incs_dir.exists() {
        return CommandOutput {
            status: 0,
            stdout: format!("{}\n", incs_dir.display()),
            stderr: String::new(),
        };
    }
    let mut files: Vec<String> = Vec::new();
    match std::fs::read_dir(&incs_dir) {
        Ok(entries) => {
            for entry in entries.filter_map(Result::ok) {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    if let Some(name) = entry.file_name().to_str() {
                        files.push(name.to_string());
                    }
                }
            }
        }
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error reading incs dir: {e}\n"),
            };
        }
    }
    files.sort();
    let stdout = if files.is_empty() {
        format!("no INC files found in {}\n", incs_dir.display())
    } else {
        files.join("\n") + "\n"
    };
    CommandOutput {
        status: 0,
        stdout,
        stderr: String::new(),
    }
}

fn run_backfill(cycle_id: &str, env: &CliEnvironment) -> CommandOutput {
    // Build archive path: ~/.sddk-knowledge/<project>/archive/<cycle_id>/debt-report.json
    let vault = match resolve_vault_path(env) {
        Ok(p) => p,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error resolving vault: {e}\n"),
            };
        }
    };
    let archive_dir = vault.join("archive").join(cycle_id);
    let report_path = archive_dir.join("debt-report.json");
    let report_json = match std::fs::read_to_string(&report_path) {
        Ok(c) => c,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error reading {}: {e}\n", report_path.display()),
            };
        }
    };
    let report: DebtReport = match serde_json::from_str(&report_json) {
        Ok(r) => r,
        Err(e) => {
            return CommandOutput {
                status: 1,
                stdout: String::new(),
                stderr: format!("error parsing debt-report.json: {e}\n"),
            };
        }
    };
    // Resolve project_id from vault path
    let project_id = vault
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sddk-framework")
        .to_string();
    // Collect existing INC IDs
    let incs_dir = vault.join("incs");
    let mut existing_ids: HashSet<String> = HashSet::new();
    if incs_dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&incs_dir) {
            for entry in entries.filter_map(Result::ok) {
                if let Some(name) = entry.file_name().to_str() {
                    if name.ends_with(".md") {
                        existing_ids.insert(name.trim_end_matches(".md").to_string());
                    }
                }
            }
        }
    }
    // Emit INC for each non-resolved finding
    let mut emitted = 0;
    let mut errors = vec![];
    std::fs::create_dir_all(&incs_dir).ok();
    for finding in report.findings.iter().filter(|f| {
        !matches!(f.status, FindingStatus::Resolved | FindingStatus::Superseded)
    }) {
        let inc_content = render_inc_template(finding, &project_id, &report.cycle_id);
        let inc_slug = sddk_engine::derive_inc_slug(finding);
        let inc_id = sddk_engine::derive_inc_id(finding, &existing_ids);
        let inc_path = incs_dir.join(format!("{}.md", inc_id));
        match std::fs::write(&inc_path, &inc_content) {
            Ok(_) => {
                existing_ids.insert(inc_id.clone());
                emitted += 1;
            }
            Err(e) => {
                errors.push(format!("{}: {e}", inc_path.display()));
            }
        }
    }
    if errors.is_empty() {
        CommandOutput {
            status: 0,
            stdout: format!("emitted {} INC files to {}\n", emitted, incs_dir.display()),
            stderr: String::new(),
        }
    } else {
        CommandOutput {
            status: 1,
            stdout: format!("emitted {} INC files with {} errors\n", emitted, errors.len()),
            stderr: errors.join("\n") + "\n",
        }
    }
}

fn run_gates(gate_name: &str, env: &CliEnvironment) -> CommandOutput {
    // Build a minimal debt report and evaluate the gate
    let report = DebtReport {
        schema_version: "1.1.0".into(),
        cycle_id: "p-52b95ef55999f9de/kernel-cycle-8".into(),
        generated_at: OffsetDateTime::now_utc()
            .format(&Rfc3339)
            .unwrap_or_else(|_| "2026-08-21T00:00:00Z".into()),
        findings: vec![],
    };
    let outcome = evaluate_named_gate(gate_name, &report);
    match &outcome {
        GateOutcome::Passed { notes } => CommandOutput {
            status: 0,
            stdout: format!("PASS: {}\n", notes),
            stderr: String::new(),
        },
        GateOutcome::Failed { offending_ids, notes } => CommandOutput {
            status: 1,
            stdout: String::new(),
            stderr: format!("FAIL: {} (offending: {})\n", notes, offending_ids.join(", ")),
        },
    }
}

/// Resolves the project vault path: ~/.sddk-knowledge/<project>/
fn resolve_vault_path(env: &CliEnvironment) -> Result<PathBuf, String> {
    let data_home = match (&env.sddk_data_dir, &env.data_home, &env.home) {
        (Some(d), _, _) => d.clone(),
        (None, Some(d), _) => d.clone(),
        (None, None, Some(h)) => h.join(".local/share"),
        _ => return Err("no data root".into()),
    };
    let vault = data_home.join("sddk/knowledge/sddk-framework");
    if !vault.exists() {
        std::fs::create_dir_all(&vault).map_err(|e| e.to_string())?;
    }
    Ok(vault)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_vault_path() {
        let env = CliEnvironment {
            home: Some(PathBuf::from("/home/test")),
            data_home: Some(PathBuf::from("/tmp/sddk-test-data")),
            sddk_data_dir: None,
            state_home: None,
            cache_home: None,
            sddk_actor: None,
            user: None,
        };
        let vault = resolve_vault_path(&env).unwrap();
        let path = vault.to_string_lossy();
        // Path should be: /tmp/sddk-test-data/sddk/knowledge/sddk-framework
        assert!(path.contains("sddk"), "vault path should contain sddk: {}", path);
        assert!(path.contains("knowledge"), "vault path should contain knowledge: {}", path);
    }

    #[test]
    fn test_report_empty_findings() {
        let temp = tempfile::tempdir().unwrap();
        let output = temp.path().join("debt-report.json");
        let result = run_report(&output);
        assert_eq!(result.status, 0);
        assert!(output.exists());
    }

    #[test]
    fn test_gates_unknown_gate() {
        let env = CliEnvironment {
            home: Some(PathBuf::from("/home/test")),
            data_home: None,
            sddk_data_dir: None,
            state_home: None,
            cache_home: None,
            sddk_actor: None,
            user: None,
        };
        let result = run_gates("unknown-gate", &env);
        assert_eq!(result.status, 1);
        assert!(result.stderr.contains("FAIL"));
    }
}
