//! Typed local Git executor with postcondition verification.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::Serialize;
use thiserror::Error;

use crate::runner::{RunOutcome, RunSpec, run};

/// Errors emitted by typed Git operations.
#[derive(Debug, Error)]
pub enum GitError {
    /// The typed git run failed to spawn or execute.
    #[error("git runner error: {0}")]
    Runner(#[from] crate::runner::RunnerError),
    /// The git command exited with a non-zero status.
    #[error("git {command} failed with exit status {status}: {stderr}")]
    CommandFailed {
        /// Executed git subcommand.
        command: String,
        /// Non-zero exit status.
        status: i32,
        /// Captured standard error.
        stderr: String,
    },
    /// The verified postcondition did not hold after the command.
    #[error("git {command} postcondition failed: expected {expected}, found {actual}")]
    Postcondition {
        /// Executed git subcommand.
        command: String,
        /// Expected observable state.
        expected: String,
        /// Observed state.
        actual: String,
    },
}

/// Read-only snapshot of repository state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GitInspect {
    /// Current HEAD short SHA, when the repository has commits.
    pub head: Option<String>,
    /// Current branch name, when detached or unborn.
    pub branch: Option<String>,
    /// Whether the worktree has uncommitted changes.
    pub dirty: bool,
}

/// Result of creating a branch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GitBranch {
    /// Created branch name.
    pub branch: String,
}

/// Result of creating a commit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GitCommit {
    /// Short SHA of the new HEAD.
    pub sha: String,
}

/// Result of creating a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct GitTag {
    /// Created tag name.
    pub tag: String,
}

/// Typed Git boundary executing commands without a shell.
#[derive(Debug, Clone)]
pub struct GitExecutor {
    root: PathBuf,
    /// Environment allowlist applied to every invocation.
    env: BTreeMap<String, String>,
    timeout_ms: u64,
    output_max_bytes: usize,
}

impl GitExecutor {
    /// Creates an executor over one repository root.
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            env: BTreeMap::new(),
            timeout_ms: 30_000,
            output_max_bytes: 1_048_576,
        }
    }

    /// Returns the repository root.
    pub fn root(&self) -> &std::path::Path {
        &self.root
    }

    /// Overrides the environment allowlist (for example Git identity).
    pub fn with_env(mut self, env: BTreeMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Reads the repository head, branch, and dirty state.
    pub fn inspect(&self) -> Result<GitInspect, GitError> {
        let head = self
            .run_ok("rev-parse", &["--short", "HEAD"])
            .ok()
            .map(|outcome| outcome.stdout.trim().to_owned());
        let branch = self
            .run_ok("branch", &["--show-current"])
            .ok()
            .map(|outcome| outcome.stdout.trim().to_owned())
            .filter(|name| !name.is_empty());
        let dirty = self
            .run_ok("status", &["--porcelain"])
            .map(|outcome| !outcome.stdout.trim().is_empty())
            .unwrap_or(false);
        Ok(GitInspect {
            head,
            branch,
            dirty,
        })
    }

    /// Creates a branch and verifies it is the current branch afterwards.
    pub fn create_branch(&self, name: &str) -> Result<GitBranch, GitError> {
        self.run_ok("checkout", &["-b", name])?;
        let current = self
            .run_ok("symbolic-ref", &["--short", "HEAD"])?
            .stdout
            .trim()
            .to_owned();
        if current != name {
            return Err(GitError::Postcondition {
                command: "checkout -b".into(),
                expected: name.into(),
                actual: current,
            });
        }
        Ok(GitBranch {
            branch: name.to_owned(),
        })
    }

    /// Creates an empty commit and verifies HEAD matches the reported SHA.
    pub fn commit(&self, message: &str) -> Result<GitCommit, GitError> {
        self.run_ok("commit", &["--allow-empty", "-m", message])?;
        let head = self
            .run_ok("rev-parse", &["--short", "HEAD"])?
            .stdout
            .trim()
            .to_owned();
        if head.is_empty() {
            return Err(GitError::Postcondition {
                command: "commit".into(),
                expected: "a non-empty HEAD".into(),
                actual: "empty".into(),
            });
        }
        Ok(GitCommit { sha: head })
    }

    /// Creates a tag and verifies it is listed afterwards.
    pub fn tag(&self, name: &str) -> Result<GitTag, GitError> {
        self.run_ok("tag", &[name])?;
        let listed = self
            .run_ok("tag", &["--list", name])?
            .stdout
            .trim()
            .to_owned();
        if listed != name {
            return Err(GitError::Postcondition {
                command: "tag".into(),
                expected: name.into(),
                actual: listed,
            });
        }
        Ok(GitTag {
            tag: name.to_owned(),
        })
    }

    fn run_ok(&self, command: &str, args: &[&str]) -> Result<RunOutcome, GitError> {
        let mut spec = RunSpec {
            program: "git".into(),
            args: vec![
                "-C".into(),
                self.root.to_string_lossy().into_owned(),
                command.into(),
            ],
            env: self.env.clone(),
            timeout_ms: self.timeout_ms,
            output_max_bytes: self.output_max_bytes,
        };
        spec.args.extend(args.iter().map(|arg| (*arg).to_owned()));
        let outcome = run(&spec)?;
        if outcome.timed_out {
            return Err(GitError::CommandFailed {
                command: command.to_owned(),
                status: -1,
                stderr: "timed out".into(),
            });
        }
        if let Some(status) = outcome.exit_status
            && status != 0
        {
            return Err(GitError::CommandFailed {
                command: command.to_owned(),
                status,
                stderr: outcome.stderr,
            });
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::fs;

    use super::GitExecutor;

    fn git_repo() -> (tempfile::TempDir, GitExecutor) {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().to_path_buf();
        std::process::Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .output()
            .unwrap();
        let mut env = BTreeMap::new();
        env.insert("GIT_AUTHOR_NAME".into(), "SDDK Test".into());
        env.insert("GIT_AUTHOR_EMAIL".into(), "test@sddk.dev".into());
        env.insert("GIT_COMMITTER_NAME".into(), "SDDK Test".into());
        env.insert("GIT_COMMITTER_EMAIL".into(), "test@sddk.dev".into());
        (directory, GitExecutor::new(root).with_env(env))
    }

    #[test]
    fn inspect_reports_head_branch_and_dirty_state() {
        let (_directory, git) = git_repo();
        let before = git.inspect().unwrap();
        assert!(before.head.is_none());
        assert!(before.branch.is_some());

        git.commit("initial").unwrap();
        fs::write(git.root().join("file.txt"), "change").unwrap();
        let after = git.inspect().unwrap();
        assert!(after.head.is_some());
        assert!(after.dirty);
    }

    #[test]
    fn create_branch_verifies_postcondition() {
        let (_directory, git) = git_repo();
        let branch = git.create_branch("feat/cas").unwrap();
        assert_eq!(branch.branch, "feat/cas");
        assert_eq!(git.inspect().unwrap().branch.as_deref(), Some("feat/cas"));
    }

    #[test]
    fn commit_reports_new_head() {
        let (_directory, git) = git_repo();
        let commit = git.commit("first commit").unwrap();
        assert!((7..=12).contains(&commit.sha.len()));
        assert_eq!(
            git.inspect().unwrap().head.as_deref(),
            Some(commit.sha.as_str())
        );
    }

    #[test]
    fn tag_verifies_postcondition() {
        let (_directory, git) = git_repo();
        git.commit("initial").unwrap();
        let tag = git.tag("v0.1.0").unwrap();
        assert_eq!(tag.tag, "v0.1.0");
    }
}
