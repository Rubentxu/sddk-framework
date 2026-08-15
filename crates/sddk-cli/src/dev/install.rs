//! `dev install` — atomic binary prefix installation with receipt.

use crate::dev::common::{MANIFEST_SURFACES, RECEIPT_FILE, atomic_write, receipt_text, walk_dir};
use crate::dev::manifest::{MANIFEST_FILE, verify_manifest};
use crate::git_cmd::default_timestamp;
use crate::{CommandOutput, OutputFormat, render_result};
use sha2::Digest;

pub(super) fn run_dev_install(args: super::InstallArgs) -> CommandOutput {
    let format = args.format;
    let result = (|| -> anyhow::Result<super::InstallReceipt> {
        // FAIL-CLOSED: when --source is provided, verify the source MANIFEST
        // BEFORE any writes to the prefix. A tampered source cannot corrupt an
        // existing installation.
        if let Some(source) = &args.source {
            let source = std::fs::canonicalize(source)?;
            let mismatches = verify_manifest(&source)?;
            if !mismatches.is_empty() {
                anyhow::bail!(
                    "manifest verification FAILED ({} mismatch(es)):\n  {}",
                    mismatches.len(),
                    mismatches.join("\n  ")
                );
            }
        }

        // NOW safe to write: compute binary digest after manifest verified.
        let binary = std::env::current_exe()?;
        let bytes = std::fs::read(&binary)?;
        let digest = format!("sha256:{:x}", sha2::Sha256::digest(&bytes));

        // Routing: if the prefix already terminates in `/bin`, install the
        // binary directly under the prefix (no extra `bin/` segment); this
        // matches the GNU autoconf/CMake convention of `--prefix=/opt/sdk/bin`
        // meaning "the binary directory". Otherwise nest under `bin/`.
        let ends_with_bin = args.prefix.file_name().and_then(|name| name.to_str()) == Some("bin");
        let target_dir = if ends_with_bin {
            args.prefix.clone()
        } else {
            args.prefix.join("bin")
        };
        std::fs::create_dir_all(&target_dir)?;
        let destination = target_dir.join("sddk");
        // Mode 0o755 BEFORE rename so the binary is born executable — fixes
        // the chmod-less atomic write that left ELF files at 0644.
        atomic_write(&destination, &bytes, Some(0o755))?;
        let binary_path = if ends_with_bin {
            "sddk".to_owned()
        } else {
            "bin/sddk".to_owned()
        };

        // Bundle surface copy: when --source is provided, copy surfaces AFTER
        // manifest verified. Binary-only (no --source) skips this block.
        if let Some(source) = &args.source {
            let source = std::fs::canonicalize(source)?;
            for surface in MANIFEST_SURFACES {
                let src_dir = source.join(surface);
                if !src_dir.is_dir() {
                    continue;
                }
                for file in walk_dir(&src_dir) {
                    if !file.is_file() {
                        continue;
                    }
                    let relative = file
                        .strip_prefix(&source)
                        .unwrap_or(file.as_path())
                        .to_path_buf();
                    let dest = args.prefix.join(&relative);
                    if let Some(parent) = dest.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    // Only copy when content differs (idempotent).
                    let needs_copy = match (std::fs::read(&file), std::fs::read(&dest)) {
                        (Ok(src), Ok(dst)) => src != dst,
                        _ => true,
                    };
                    if needs_copy {
                        std::fs::copy(&file, &dest)?;
                    }
                }
            }
            // Also copy the MANIFEST.sha256 itself to the prefix so `dev verify`
            // can re-check installed surfaces against it.
            let manifest_src = source.join(MANIFEST_FILE);
            if manifest_src.is_file() {
                let manifest_dest = args.prefix.join(MANIFEST_FILE);
                std::fs::copy(&manifest_src, &manifest_dest)?;
            }
        }

        let receipt = super::InstallReceipt {
            version: env!("CARGO_PKG_VERSION").to_owned(),
            commit: args
                .commit
                .or_else(|| std::env::var("GITHUB_SHA").ok())
                .unwrap_or_else(default_timestamp),
            binary_sha256: digest,
            channel: args.channel.clone(),
            installed_at: args.timestamp.unwrap_or_else(default_timestamp),
            binary_path,
            bundle: args.source.is_some(),
        };
        let receipt_path = args.prefix.join(RECEIPT_FILE);
        atomic_write(
            &receipt_path,
            serde_json::to_string_pretty(&receipt)?.as_bytes(),
            None,
        )?;
        Ok(receipt)
    })();
    render_result(result, format, receipt_text)
}
