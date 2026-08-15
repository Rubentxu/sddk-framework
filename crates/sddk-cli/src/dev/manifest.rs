//! `dev manifest` — generate or verify MANIFEST.sha256.

use crate::CommandOutput;

pub(super) fn run_dev_manifest(args: super::ManifestArgs) -> CommandOutput {
    let _ = args;
    CommandOutput::default()
}
