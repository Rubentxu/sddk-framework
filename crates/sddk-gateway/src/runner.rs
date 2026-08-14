//! Typed process runner without shell interpretation.

use std::collections::BTreeMap;
use std::process::{Command, Stdio};
use std::time::Duration;

use thiserror::Error;

/// Execution limits and environment allowlist for one capability run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSpec {
    /// Executable path or name resolved by the operating system.
    pub program: String,
    /// Positional arguments passed directly, never through a shell.
    pub args: Vec<String>,
    /// Complete environment allowlist; inherited variables are dropped.
    pub env: BTreeMap<String, String>,
    /// Maximum wall-clock time in milliseconds before the process is killed.
    pub timeout_ms: u64,
    /// Maximum captured bytes for stdout and stderr combined per stream.
    pub output_max_bytes: usize,
}

impl RunSpec {
    /// Creates a minimal spec with no environment and default limits.
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: BTreeMap::new(),
            timeout_ms: 30_000,
            output_max_bytes: 1_048_576,
        }
    }
}

/// Captured result of one typed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunOutcome {
    /// Process exit status, or `None` when the process was killed.
    pub exit_status: Option<i32>,
    /// Captured standard output, truncated to the output limit.
    pub stdout: String,
    /// Captured standard error, truncated to the output limit.
    pub stderr: String,
    /// Whether the process was killed by the timeout.
    pub timed_out: bool,
}

/// Raw process output retained for typed binary consumers inside the gateway.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawRunOutcome {
    pub(crate) exit_status: Option<i32>,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
    pub(crate) timed_out: bool,
}

impl RawRunOutcome {
    pub(crate) fn into_lossy(self, output_max_bytes: usize) -> RunOutcome {
        RunOutcome {
            exit_status: self.exit_status,
            stdout: truncate(
                String::from_utf8_lossy(&self.stdout).into_owned(),
                output_max_bytes,
            ),
            stderr: truncate(
                String::from_utf8_lossy(&self.stderr).into_owned(),
                output_max_bytes,
            ),
            timed_out: self.timed_out,
        }
    }
}

/// Failures while spawning or polling a typed run.
#[derive(Debug, Error)]
pub enum RunnerError {
    /// The executable could not be spawned.
    #[error("failed to spawn {program}: {source}")]
    Spawn {
        /// Requested executable.
        program: String,
        /// Underlying spawn failure.
        source: std::io::Error,
    },
    /// Reading captured output failed.
    #[error("failed to read output of {program}: {source}")]
    Read {
        /// Executable whose output could not be read.
        program: String,
        /// Underlying read failure.
        source: std::io::Error,
    },
}

/// Runs a spec with separated argv, allowlisted environment, limits, and timeout.
pub fn run(spec: &RunSpec) -> Result<RunOutcome, RunnerError> {
    Ok(run_raw(spec)?.into_lossy(spec.output_max_bytes))
}

/// Runs a spec while preserving raw output bytes for typed internal consumers.
pub(crate) fn run_raw(spec: &RunSpec) -> Result<RawRunOutcome, RunnerError> {
    use std::io::Read;

    let mut command = Command::new(&spec.program);
    command
        .args(&spec.args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in &spec.env {
        command.env(key, value);
    }
    let mut child = command.spawn().map_err(|source| RunnerError::Spawn {
        program: spec.program.clone(),
        source,
    })?;
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();

    let deadline = std::time::Instant::now() + Duration::from_millis(spec.timeout_ms);
    let mut timed_out = false;
    let exit_status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status.code(),
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    timed_out = true;
                    break None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(error) => {
                return Err(RunnerError::Read {
                    program: spec.program.clone(),
                    source: error,
                });
            }
        }
    };

    let read_output = |stream: &mut Option<std::process::ChildStdout>| {
        let mut buffer = Vec::new();
        if let Some(stream) = stream.as_mut() {
            stream.read_to_end(&mut buffer)?;
        }
        Ok::<_, std::io::Error>(buffer)
    };
    let read_error = |stream: &mut Option<std::process::ChildStderr>| {
        let mut buffer = Vec::new();
        if let Some(stream) = stream.as_mut() {
            stream.read_to_end(&mut buffer)?;
        }
        Ok::<_, std::io::Error>(buffer)
    };

    let stdout_bytes = read_output(&mut stdout).map_err(|source| RunnerError::Read {
        program: spec.program.clone(),
        source,
    })?;
    let stderr_bytes = read_error(&mut stderr).map_err(|source| RunnerError::Read {
        program: spec.program.clone(),
        source,
    })?;

    Ok(RawRunOutcome {
        exit_status,
        stdout: stdout_bytes,
        stderr: stderr_bytes,
        timed_out,
    })
}

fn truncate(value: String, max_bytes: usize) -> String {
    if value.len() <= max_bytes {
        return value;
    }
    let mut end = max_bytes;
    while !value.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}…<truncated {} bytes>", &value[..end], value.len() - end)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{RunSpec, run};

    fn echo_spec(env: &[(&str, &str)]) -> RunSpec {
        let mut allowlist = BTreeMap::new();
        for (key, value) in env {
            allowlist.insert((*key).to_owned(), (*value).to_owned());
        }
        RunSpec {
            program: "echo".into(),
            args: vec!["hello".into()],
            env: allowlist,
            timeout_ms: 5_000,
            output_max_bytes: 1_024,
        }
    }

    #[test]
    fn runs_typed_argv_without_shell() {
        let outcome = run(&echo_spec(&[])).unwrap();
        assert_eq!(outcome.exit_status, Some(0));
        assert_eq!(outcome.stdout.trim(), "hello");
        assert!(!outcome.timed_out);
    }

    #[test]
    fn spawn_failure_is_reported() {
        let spec = RunSpec::new("sddk-no-such-binary-xyz");
        let error = run(&spec).unwrap_err();
        assert!(error.to_string().contains("sddk-no-such-binary-xyz"));
    }

    #[test]
    fn timeout_kills_the_process() {
        let spec = RunSpec {
            program: "sleep".into(),
            args: vec!["5".into()],
            env: BTreeMap::new(),
            timeout_ms: 50,
            output_max_bytes: 1_024,
        };
        let outcome = run(&spec).unwrap();
        assert!(outcome.timed_out);
        assert_eq!(outcome.exit_status, None);
    }

    #[test]
    fn output_is_truncated_to_the_limit() {
        let spec = RunSpec {
            program: "echo".into(),
            args: vec!["0123456789".into()],
            env: BTreeMap::new(),
            timeout_ms: 5_000,
            output_max_bytes: 4,
        };
        let outcome = run(&spec).unwrap();
        assert!(outcome.stdout.len() < 30);
        assert!(outcome.stdout.contains("truncated"));
    }
}
