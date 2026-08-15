//! `dev update` — download and install a framework release bundle.

use crate::{CliEnvironment, CommandOutput};

pub(super) fn run_dev_update(args: super::UpdateArgs, environment: &CliEnvironment) -> CommandOutput {
    let _ = (args, environment);
    CommandOutput::default()
}
