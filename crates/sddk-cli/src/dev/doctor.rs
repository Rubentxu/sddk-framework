//! `dev doctor` — toolchain and environment prerequisite checker.

use crate::{CliEnvironment, CommandOutput, OutputFormat};

pub(super) fn run_dev_doctor(args: super::DoctorArgs, environment: &CliEnvironment) -> CommandOutput {
    let _ = (args, environment);
    CommandOutput::default()
}
