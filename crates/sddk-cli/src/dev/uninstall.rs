//! `dev uninstall` — remove an installed prefix or editor assets.

use crate::{CliEnvironment, CommandOutput};

pub(super) fn run_dev_uninstall(args: super::UninstallArgs) -> CommandOutput {
    let _ = args;
    CommandOutput::default()
}
