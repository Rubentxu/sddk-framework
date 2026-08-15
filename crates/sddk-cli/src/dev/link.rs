//! `dev link` — symlink framework assets into an editor.

use crate::{CliEnvironment, CommandOutput};

pub(super) fn run_dev_link(args: super::LinkArgs, environment: &CliEnvironment) -> CommandOutput {
    let _ = (args, environment);
    CommandOutput::default()
}
