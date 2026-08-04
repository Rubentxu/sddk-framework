//! Shared testing utilities for SDDK crates.

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(missing_docs)]

use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use tempfile::TempDir;

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
}
