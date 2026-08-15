//! Shared I/O helpers and constants used across dev subcommands.

use std::path::{Path, PathBuf};

use crate::CommandOutput;

pub(super) const RECEIPT_FILE: &str = "sddk-install.json";

pub(super) const MANIFEST_SURFACES: [&str; 4] = ["agents", "skills", "prompts/sddk", "assets"];

pub(super) fn read_receipt(prefix: &Path) -> anyhow::Result<super::InstallReceipt> {
    let path = prefix.join(RECEIPT_FILE);
    if !path.exists() {
        anyhow::bail!("no installation receipt at {path:?}");
    }
    Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
}

pub(super) fn tool_version(tool: &str) -> anyhow::Result<String> {
    let output = std::process::Command::new(tool).arg("--version").output()?;
    if !output.status.success() {
        anyhow::bail!("{tool} exited {}", output.status);
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub(super) fn atomic_write(
    destination: &Path,
    bytes: &[u8],
    mode: Option<u32>,
) -> anyhow::Result<()> {
    use std::io::Write;
    let parent = destination.parent().expect("destination has a parent");
    std::fs::create_dir_all(parent)?;
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("artifact");
    let mut last_error = None;
    for attempt in 0..100 {
        let temporary = parent.join(format!(".{file_name}.tmp-{}-{attempt}", std::process::id()));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(mut file) => {
                let result = (|| -> std::io::Result<()> {
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    drop(file);
                    // chmod BEFORE rename so the destination is born with
                    // the requested mode (no 0644 window). Unix-only.
                    #[cfg(unix)]
                    {
                        if let Some(bits) = mode {
                            use std::os::unix::fs::PermissionsExt;
                            std::fs::set_permissions(
                                &temporary,
                                std::fs::Permissions::from_mode(bits),
                            )?;
                        }
                    }
                    std::fs::rename(&temporary, destination)
                })();
                if let Err(source) = result {
                    let _ = std::fs::remove_file(&temporary);
                    return Err(source.into());
                }
                return Ok(());
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                last_error = Some(error);
            }
            Err(source) => return Err(source.into()),
        }
    }
    Err(last_error
        .unwrap_or_else(|| std::io::Error::other("no temporary path available"))
        .into())
}

pub(super) fn failure_status(message: String) -> CommandOutput {
    CommandOutput {
        status: 1,
        stdout: String::new(),
        stderr: format!("error: {message}\n"),
    }
}

/// Format an `InstallReceipt` as human-readable text.
pub(super) fn receipt_text(receipt: &super::InstallReceipt) -> String {
    format!(
        "version: {}\ncommit: {}\nbinary_sha256: {}\nchannel: {}\ninstalled_at: {}\nbinary_path: {}\nbundle: {}\n",
        receipt.version,
        receipt.commit,
        receipt.binary_sha256,
        receipt.channel,
        receipt.installed_at,
        receipt.binary_path,
        receipt.bundle
    )
}

pub(super) fn walk_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                files.extend(walk_dir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}

/// Compute the plain lowercase hex SHA-256 of a file.
pub(super) fn sha256_hex(path: &Path) -> anyhow::Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path)?;
    Ok(format!("{:x}", Sha256::digest(&bytes)))
}

/// Download a URL to a destination via curl/wget, or copy from file://.
pub(super) fn download_to(url: &str, destination: &Path) -> anyhow::Result<()> {
    if let Some(source) = url.strip_prefix("file://") {
        std::fs::copy(source, destination)?;
        return Ok(());
    }
    let status = std::process::Command::new("curl")
        .args(["-fsSL", "--retry", "3", "-o"])
        .arg(destination)
        .arg(url)
        .status();
    match status {
        Ok(status) if status.success() => Ok(()),
        Ok(status) => anyhow::bail!("curl exited {status} for {url}"),
        Err(_) => {
            let status = std::process::Command::new("wget")
                .args(["-qO"])
                .arg(destination)
                .arg(url)
                .status()?;
            if status.success() {
                Ok(())
            } else {
                anyhow::bail!("wget exited {status} for {url}")
            }
        }
    }
}

pub(super) fn copy_bundle_tree(source: &Path, target: &Path) -> anyhow::Result<()> {
    for file in walk_dir(source) {
        if !file.is_file() {
            continue;
        }
        let relative = file.strip_prefix(source)?;
        let destination = target.join(relative);
        if let Some(parent) = destination.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(file, destination)?;
    }
    Ok(())
}

/// Count entries in a root's MANIFEST.sha256 (0 when absent).
pub(super) fn count_manifest_entries(root: &Path) -> anyhow::Result<usize> {
    let raw = std::fs::read_to_string(root.join(super::manifest::MANIFEST_FILE))?;
    Ok(raw.lines().filter(|l| !l.trim().is_empty()).count())
}
