//! Pack manifest validation command.

use std::path::PathBuf;

use clap::{Args, Subcommand};
use sddk_domain::{PackDiagnostic, load_pack_manifest, validate_pack_manifest};
use serde::Serialize;

use crate::{CommandOutput, OutputFormat, render_result};

#[derive(Debug, Subcommand)]
pub(crate) enum PackCommand {
    /// Validate a pack manifest against the pack model.
    Validate(PackValidateArgs),
}

#[derive(Debug, Clone, Args)]
pub(crate) struct PackValidateArgs {
    /// Manifest path.
    #[arg(long, default_value = "manifest.toml")]
    pub(crate) manifest: PathBuf,
    /// Output format.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub(crate) format: OutputFormat,
}

pub(crate) fn run_pack(command: PackCommand) -> CommandOutput {
    match command {
        PackCommand::Validate(args) => run_pack_validate(args),
    }
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "snake_case")]
struct PackValidationOutput {
    id: String,
    version: String,
    valid: bool,
    diagnostics: Vec<PackDiagnostic>,
}

fn run_pack_validate(args: PackValidateArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<PackValidationOutput> {
        let manifest = load_pack_manifest(&args.manifest)?;
        let diagnostics = validate_pack_manifest(&manifest);
        Ok(PackValidationOutput {
            id: manifest.pack.id.clone(),
            version: manifest.pack.version.clone(),
            valid: diagnostics.is_empty(),
            diagnostics,
        })
    })();
    match result {
        Ok(output) => {
            let mut command = render_result(Ok(output.clone()), format, pack_validation_text);
            if !output.valid {
                command.status = 1;
            }
            command
        }
        Err(error) => crate::failure(error.to_string()),
    }
}

fn pack_validation_text(output: &PackValidationOutput) -> String {
    let mut text = format!(
        "id: {}\nversion: {}\nvalid: {}\n",
        output.id, output.version, output.valid
    );
    for diagnostic in &output.diagnostics {
        text.push_str(&format!(
            "error[{}]: {}\n  help: {}\n",
            diagnostic.code, diagnostic.message, diagnostic.hint
        ));
    }
    text
}
