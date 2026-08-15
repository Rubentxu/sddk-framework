//! `dev check` — run repository quality gates (fmt, clippy, tests).

use crate::{CommandOutput, OutputFormat, render_result};
use sddk_gateway::{RunSpec, run};

pub(super) fn run_dev_check(args: super::CheckArgs) -> CommandOutput {
    let steps = [
        ("fmt", vec!["fmt", "--all", "--", "--check"]),
        (
            "clippy",
            vec![
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ],
        ),
        ("test", vec!["test", "--workspace", "--locked"]),
    ];
    let mut text = String::new();
    let mut failed = false;
    for (name, args) in steps {
        let spec = RunSpec {
            program: "cargo".into(),
            args: args.into_iter().map(str::to_owned).collect(),
            env: Default::default(),
            timeout_ms: 600_000,
            output_max_bytes: 1_048_576,
        };
        let outcome = match run(&spec) {
            Ok(outcome) => outcome,
            Err(error) => {
                failed = true;
                text.push_str(&format!("{name}: FAILED ({error})\n"));
                continue;
            }
        };
        let passed = outcome.exit_status == Some(0) && !outcome.timed_out;
        if !passed {
            failed = true;
        }
        text.push_str(&format!(
            "{name}: {}\n",
            if passed { "PASS" } else { "FAIL" }
        ));
    }
    let mut output = CommandOutput {
        status: i32::from(failed),
        stdout: text,
        stderr: String::new(),
    };
    if let OutputFormat::Json = args.format {
        output.stdout = format!("{}\n", serde_json::json!({"passed": !failed}));
    }
    output
}
