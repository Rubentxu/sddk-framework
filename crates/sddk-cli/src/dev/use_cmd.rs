//! `dev use` — asdf-style framework bundle version selector.

use crate::{CliEnvironment, CommandOutput};

pub(super) fn run_dev_use(args: super::UseArgs, environment: &CliEnvironment) -> CommandOutput {
    let _ = (args, environment);
    CommandOutput::default()
}
