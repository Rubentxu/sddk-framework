//! `dev install` — atomic binary prefix installation with receipt.

use crate::CommandOutput;

pub(super) fn run_dev_install(args: super::InstallArgs) -> CommandOutput {
    let _ = args;
    CommandOutput::default()
}
