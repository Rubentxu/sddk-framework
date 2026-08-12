//! CLI Compatibility Snapshot Tests (SDDK2-001)
//!
//! Pins the current CLI command surface as a golden contract. Before any 2.0
//! refactoring touches `crates/*/src/`, these tests prove HEAD behavior is
//! preserved. If Phase 1 breaks a command, a compatibility test fails
//! immediately.
//!
//! ## Regeneration (opt-in, write-and-fail-safe)
//!
//! To update golden fixtures after intentional CLI changes:
//!
//! ```text
//! UPDATE_SNAPSHOTS=1 cargo test --test cli_compatibility
//! ```
//!
//! This writes new bytes to `CARGO_MANIFEST_DIR/tests/fixtures/cli/<name>.txt`.
//! The test then **fails** with `FIXTURE UPDATED — review required` and exits 101.
//! A regression MUST NEVER become silently green: explicit human review is required.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

// =============================================================================
// Golden Fixtures
// =============================================================================

const HELP_TOP_LEVEL: &str = include_str!("fixtures/cli/help-top-level.txt");
const HELP_UAT: &str = include_str!("fixtures/cli/help-uat.txt");

// =============================================================================
// Normalization
// =============================================================================

/// Normalize CLI output for snapshot comparison.
///
/// Applies: CRLF→LF, ANSI escape strip, dynamic segments → sentinels.
/// This ensures cross-platform and version-independent snapshot matching.
fn normalize(output: &[u8]) -> Vec<u8> {
    // Step 1: CRLF → LF
    let s = String::from_utf8_lossy(output);
    let s = s.replace("\r\n", "\n").replace("\r", "\n");

    // Step 2: Strip ANSI escape sequences \x1b[...m
    let s = strip_ansi(&s);

    // Step 3: Replace dynamic segments with sentinels
    let s = mask_dynamic_content(&s);

    s.into_bytes()
}

fn strip_ansi(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip ANSI escape sequence
            while let Some(&next) = chars.peek() {
                chars.next();
                if next == 'm' {
                    break;
                }
            }
        } else {
            result.push(c);
        }
    }
    result
}

fn mask_dynamic_content(s: &str) -> String {
    // Replace common dynamic patterns with sentinels
    let mut result = s.to_string();

    // Mask binary path
    result = result.replace(env!("CARGO_BIN_EXE_sddk"), "<BIN>");

    // Mask HOME paths
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            result = result.replace(&home, "<HOME>");
        }
    }

    // For version patterns like "v1.5.3", we do simple string replacement
    // since regex is not a dev-dependency
    let versions = ["v1.9.1", "v1.5.3", "v1.5.0", "v1.5.2", "v1.5.4"];
    for v in versions {
        result = result.replace(v, "<VERSION>");
    }

    result
}

// =============================================================================
// UPDATE_SNAPSHOTS Gate
// =============================================================================

/// Returns the path to a fixture file within CARGO_MANIFEST_DIR.
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("cli")
        .join(name)
        .with_extension("txt")
}

/// Check if UPDATE_SNAPSHOTS is enabled and perform write-and-fail-safe update.
fn check_update_snapshots(name: &str, new_content: &str) {
    if std::env::var("UPDATE_SNAPSHOTS").as_deref() == Ok("1") {
        let path = fixture_path(name);
        match fs::write(&path, new_content) {
            Ok(_) => {
                eprintln!(
                    "FIXTURE UPDATED — review required; re-run without UPDATE_SNAPSHOTS to verify"
                );
                std::process::exit(101);
            }
            Err(_e) => {
                panic!("cannot write fixture: {} (permission denied)", path.display());
            }
        }
    }
}

// =============================================================================
// Snapshot Tests
// =============================================================================

#[test]
fn top_level_help_matches_snapshot() {
    // Capture current output - clap's --help outputs to stderr
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .arg("--help")
        .output()
        .expect("failed to execute sddk --help");

    let actual = normalize(&output.stderr);

    // Normalize fixture for comparison
    let expected = normalize(HELP_TOP_LEVEL.as_bytes());

    // Check for UPDATE_SNAPSHOTS
    let actual_str = String::from_utf8_lossy(&actual);
    check_update_snapshots("help-top-level", &actual_str);

    // Assert exact match after normalization
    assert_eq!(
        actual, expected,
        "top-level help snapshot mismatch: CLI output differs from fixture.\n\
         If this is an intentional change, run:\n\
         UPDATE_SNAPSHOTS=1 cargo test --test cli_compatibility\n\
         Then review and commit the updated fixture."
    );
}

#[test]
fn uat_help_matches_snapshot() {
    // Capture current output - clap's --help outputs to stderr
    let output = Command::new(env!("CARGO_BIN_EXE_sddk"))
        .args(["uat", "--help"])
        .output()
        .expect("failed to execute sddk uat --help");

    let actual = normalize(&output.stderr);

    // Normalize fixture for comparison
    let expected = normalize(HELP_UAT.as_bytes());

    // Check for UPDATE_SNAPSHOTS
    let actual_str = String::from_utf8_lossy(&actual);
    check_update_snapshots("help-uat", &actual_str);

    // Assert exact match after normalization
    assert_eq!(
        actual, expected,
        "UAT help snapshot mismatch: CLI output differs from fixture.\n\
         If this is an intentional change, run:\n\
         UPDATE_SNAPSHOTS=1 cargo test --test cli_compatibility\n\
         Then review and commit the updated fixture."
    );
}

// =============================================================================
// Flow Tests
// =============================================================================

/// Run sddk command within an isolated CliFixture environment.
fn run_in_fixture(fixture: &CliFixture, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sddk"));
    cmd.args(args)
        .env("HOME", fixture.home())
        .env("XDG_DATA_HOME", fixture.data())
        .env("XDG_STATE_HOME", fixture.state())
        .env("XDG_CACHE_HOME", fixture.cache());
    cmd.output().expect("failed to execute sddk command")
}

/// Run sddk command in the current repository context (no fixture isolation).
fn run_sddk(args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_sddk"));
    cmd.args(args);
    cmd.output().expect("failed to execute sddk command")
}

#[test]
fn cycle_status_exits_zero_with_canonical_json() {
    // Run cycle status in the current workspace context
    let output = run_sddk(&[
        "cycle", "status",
        "--format", "json",
        "--root", ".",
        "--scope", ".",
        "--cycle", "p-52b95ef55999f9de/sddk-2-0-phase0-guardrails",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "cycle status should exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify JSON is parseable and contains required keys
    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .expect("cycle status JSON should be parseable");

    // Required keys: cycle_id, status, phase, path, lease.owner
    assert!(
        parsed.get("cycle_id").is_some(),
        "JSON should contain cycle_id"
    );
    assert!(
        parsed.get("status").is_some(),
        "JSON should contain status"
    );
    assert!(
        parsed.get("phase").is_some(),
        "JSON should contain phase"
    );
    assert!(
        parsed.get("path").is_some(),
        "JSON should contain path"
    );
    assert!(
        parsed.get("lease").is_some() && parsed["lease"].get("owner").is_some(),
        "JSON should contain lease.owner"
    );

    // Stderr should be empty on success
    assert!(
        output.stderr.is_empty(),
        "cycle status should not write to stderr on success"
    );
}

#[test]
fn ledger_verify_exits_zero_with_clean_json() {
    // Run ledger verify in the current workspace context
    let output = run_sddk(&[
        "ledger", "verify",
        "--format", "json",
        "--root", ".",
        "--scope", ".",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "ledger verify should exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify JSON is parseable with no error key
    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .expect("ledger verify JSON should be parseable");

    // Should not contain an error key (or it should be null/absent)
    let has_error = parsed
        .as_object()
        .map(|obj| obj.contains_key("error"))
        .unwrap_or(false);
    assert!(
        !has_error || parsed["error"].is_null(),
        "ledger verify should not contain an error key"
    );

    // Stderr should be empty on success
    assert!(
        output.stderr.is_empty(),
        "ledger verify should not write to stderr on success"
    );
}

#[test]
fn capability_status_exits_zero_with_empty_array() {
    // Run capability status in the current workspace context
    let output = run_sddk(&[
        "capability", "status",
        "--format", "json",
        "--root", ".",
        "--scope", ".",
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "capability status should exit 0, got {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );

    // Verify JSON is parseable
    let json_str = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&json_str)
        .expect("capability status JSON should be parseable");

    // Should be an empty array in fresh context
    let receipts = parsed.as_array();
    assert!(
        receipts.map(|arr| arr.is_empty()).unwrap_or(false),
        "capability status should return empty array in fresh context"
    );

    // Stderr should be empty on success
    assert!(
        output.stderr.is_empty(),
        "capability status should not write to stderr on success"
    );
}

// =============================================================================
// CliFixture (minimal local implementation for standalone test)
// =============================================================================

use tempfile::TempDir;

struct CliFixture {
    _directory: TempDir,
    root: PathBuf,
    data: PathBuf,
    state: PathBuf,
    cache: PathBuf,
    home: PathBuf,
}

impl CliFixture {
    fn new(name: &str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let root = directory.path().join(name);
        std::fs::create_dir_all(&root).unwrap();
        Self {
            root,
            data: directory.path().join("data"),
            state: directory.path().join("state"),
            cache: directory.path().join("cache"),
            home: directory.path().join("home"),
            _directory: directory,
        }
    }

    fn root(&self) -> &PathBuf {
        &self.root
    }
    fn data(&self) -> &PathBuf {
        &self.data
    }
    fn state(&self) -> &PathBuf {
        &self.state
    }
    fn cache(&self) -> &PathBuf {
        &self.cache
    }
    fn home(&self) -> &PathBuf {
        &self.home
    }
}
