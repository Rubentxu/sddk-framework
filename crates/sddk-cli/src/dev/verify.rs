//! `dev verify` — verify an installed prefix against its receipt.

use crate::CommandOutput;

pub(super) fn run_dev_verify(args: super::VerifyArgs) -> CommandOutput {
    let _ = args;
    CommandOutput::default()
}
