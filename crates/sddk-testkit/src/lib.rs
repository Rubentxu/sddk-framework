//! Shared testing utilities for SDDK crates.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use tempfile::TempDir;

/// RAII guard that kills and reaps a child process on drop.
///
/// Covers every exit path — normal return, `?`-propagation, and panic unwind.
/// Idempotent via `Option::take()`. Use [`ChildGuard::take`] to transfer
/// ownership out of the guard (avoids double-kill).
#[derive(Debug)]
pub struct ChildGuard(Option<std::process::Child>);

impl ChildGuard {
    /// Wraps a spawned child; it will be killed and reaped when dropped.
    pub fn new(child: std::process::Child) -> Self {
        ChildGuard(Some(child))
    }

    /// Transfers ownership of the child out of the guard (avoids double-kill).
    pub fn take(&mut self) -> Option<std::process::Child> {
        self.0.take()
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        if let Some(mut child) = self.0.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Isolated temporary repository for integration and contract tests.
#[derive(Debug)]
pub struct TestRepository {
    directory: TempDir,
}

impl TestRepository {
    /// Creates an empty repository root that is deleted when the fixture is dropped.
    pub fn new() -> io::Result<Self> {
        Ok(Self {
            directory: tempfile::tempdir()?,
        })
    }

    /// Returns the repository root.
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// Writes UTF-8 content to a repository-relative path, creating parent directories.
    pub fn write(&self, relative: impl AsRef<Path>, content: &str) -> io::Result<PathBuf> {
        let relative = relative.as_ref();
        if relative.is_absolute()
            || relative.components().any(|component| {
                matches!(
                    component,
                    Component::ParentDir | Component::RootDir | Component::Prefix(_)
                )
            })
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("fixture path must stay inside the repository: {relative:?}"),
            ));
        }

        let destination = self.path().join(relative);
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&destination, content)?;
        Ok(destination)
    }
}

#[cfg(test)]
mod tests {
    use super::TestRepository;

    #[test]
    fn writes_nested_files_inside_repository() {
        let repository = TestRepository::new().unwrap();

        let path = repository.write("nested/file.txt", "fixture\n").unwrap();

        assert_eq!(std::fs::read_to_string(path).unwrap(), "fixture\n");
    }

    #[test]
    fn rejects_paths_outside_repository() {
        let repository = TestRepository::new().unwrap();

        let error = repository.write("../outside.txt", "nope").unwrap_err();

        assert_eq!(error.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn child_guard_take_then_drop_is_idempotent() {
        let child = std::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("POSIX sleep available in test env");
        let mut guard = super::ChildGuard::new(child);
        // First take() returns Some.
        let taken = guard.take();
        assert!(taken.is_some());
        // Second take() returns None — guard is now empty.
        assert!(guard.take().is_none());
        // Dropping the empty guard must not panic.
        drop(guard);
        // Drop the child we took out so it doesn't leak.
        if let Some(mut c) = taken {
            let _ = c.kill();
            let _ = c.wait();
        }
    }

    #[test]
    #[cfg(unix)]
    fn child_guard_kills_on_drop() {
        let child = std::process::Command::new("sleep")
            .arg("60")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("POSIX sleep available in test env");
        let pid = child.id();
        let guard = super::ChildGuard::new(child);
        drop(guard);
        // Poll /proc/{pid} until it disappears (child was killed+reaped).
        let proc_path = std::path::PathBuf::from(format!("/proc/{pid}"));
        let mut attempts = 0;
        while proc_path.exists() && attempts < 20 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            attempts += 1;
        }
        assert!(
            !proc_path.exists(),
            "child process {pid} still present in /proc after Drop"
        );
    }
}
