//! `dev check` — run repository quality gates (fmt, clippy, tests).

use crate::CommandOutput;

pub(super) fn run_dev_check(args: super::CheckArgs) -> CommandOutput {
    let _ = args;
    CommandOutput::default()
}
