---
name: sddk-verify
description: "Trigger: sddk-verify, verify change. Gate spec compliance and production-ready implementation quality with executable evidence."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "2.1"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you are not the `sddk-verify` executor, delegate to it and stop.

## Activation Contract

Act as the read-only quality gate after implementation. Verify the exact change subject against proposal, spec, design, invariants, tasks, project standards, and fresh runtime evidence.

## Hard Rules

- Treat specs first, design second, tasks last; completed tasks never prove correctness.
- Inspect changed production code and runtime wiring. Green tests alone never prove a real implementation.
- Execute fresh relevant tests plus the repository's required regression/build checks.
- Fail required behavior implemented as a stub, placeholder, constant test satisfier, unreachable path, or production-wired fake.
- Apply production-readiness and evidence-based SOLID checks on every path. Use entropy metrics only as supporting evidence.
- A scenario is compliant only when a covering test passed and exercised its production path.
- Keep verify read-only. `sddk-debt-verify` remains the separate later debt audit.
- If Strict TDD is active, load `prompts/sddk/phases/strict-tdd-verify.md`; never downgrade silently.
- Obey `verify_role`: only the coordinator persists and touches the ledger; a lens evaluates exactly one `lens_id` and returns.

## Decision Gates

| Condition | Result |
|---|---|
| Required scenario, invariant, build, regression, or production gate fails | `FAIL` |
| Stub/placeholder/non-production implementation reaches the changed runtime path | `FAIL` |
| Required evidence cannot be obtained or the authoritative artifacts contradict | `blocked` envelope with verdict `FAIL` |
| Only optional, explicitly deferred improvement remains | `PASS_WITH_WARNINGS` |
| Every mandatory gate has reproducible passing evidence | `PASS` |

Warnings never compensate for a failed or unevaluated mandatory gate.

## Execution Steps

1. Read `skills/_shared/sddk-phase-common.md` and `prompts/sddk/phases/verify.md`.
2. Identify the exact base/head or dirty diff and load proposal, spec, design, tasks, and apply evidence.
3. Build requirement-to-test and production-readiness matrices.
4. Inspect the changed implementation, execution paths, and composition wiring for non-production code.
5. Run fresh deterministic checks; assess test strength and concrete SOLID effects.
6. As coordinator, dispatch only the path-configured lenses and synthesize without overriding deterministic failures.
7. Coordinator only: persist `{cycle-artifacts-dir}/verify-report.md` and return the standard envelope. A lens returns only the lens envelope from the phase prompt.

## Output Contract

Coordinator output MUST follow the report and standard envelope in `prompts/sddk/phases/verify.md`. Include the verified subject, exact commands and exit codes, behavioral matrix, production-readiness matrix, concrete SOLID findings, unresolved evidence, and `PASS | PASS_WITH_WARNINGS | FAIL`. Lens output MUST use only the lens envelope defined there.

## CLI Contract (coordinator only)

Before returning, inspect the authoritative cycle state:

`sddk cycle status --root . --scope . --cycle {cycle_id} --format json`

Require the returned cycle ID and path to match the launch input and require `status=OPEN`, `phase=verify`. Select one transition ID from the returned path:

| Path | Transition |
|---|---|
| `A-full` | `phase.verify.complete` |
| `A-min` | `phase.verify.complete.a-min` |
| `A-lite` | `phase.verify.complete.a-lite` |
| `B-direct` | `phase.verify.complete.b-direct` |

Then:

1. Verify locally that `git rev-parse HEAD` equals `{head_commit}`, the report and command logs exist, and their freshly computed SHA-256 values equal the evidence recorded in the report. A mismatch changes the affected gate outcome to `failed`; it never creates a passing receipt.
2. Resolve `tests_outcome` to `passed` only when every required test/build command passed against the verified subject; otherwise use `failed`. Resolve `policy_outcome` to `passed` only when every other mandatory gate passed; otherwise use `failed`. `PASS_WITH_WARNINGS` still requires both outcomes to be `passed`.
3. Evaluate both gates against the selected `{verify_transition}`. Evidence must identify the subject and result; boolean evidence such as `{"checked":true}` is invalid:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition {verify_transition} --gate tests-pass --outcome {tests_outcome} --evaluator sddk.cli --evidence '{"subject_sha":"{head_commit}","result":"{tests_outcome}","commands":[{command_evidence}],"report_path":"{report_path}","report_sha256":"{report_sha256}"}' --timestamp {now} --actor sddk-kernel --format json`
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition {verify_transition} --gate policy-compliant --outcome {policy_outcome} --evaluator sddk.cli --evidence '{"subject_sha":"{head_commit}","result":"{policy_outcome}","mandatory_gates":{mandatory_gate_results},"report_path":"{report_path}","report_sha256":"{report_sha256}"}' --timestamp {now} --actor sddk-kernel --format json`
4. Transition with the exact report and both returned receipt IDs. If cycle status contains a lease, append its matching `--lease-owner {lease_owner} --fencing-token {fencing_token}`; when `lease` is null, omit both flags:
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition {verify_transition} --artifact verification-report={report_path} --gate-receipt {tests_receipt} --gate-receipt {policy_receipt} {lease_flags} --timestamp {now} --actor sddk-kernel --format json`
5. Require transition output `outcome=succeeded` for `PASS`/`PASS_WITH_WARNINGS`. Require `outcome=failed`, `status=REMEDIATING`, `phase=verify` for `FAIL` or blocked verification. Any other state is blocked.
6. Run `sddk ledger verify --root . --scope .`.

Gate evaluation, transition, and ledger verification are mandatory even for a verification failure: failed receipts plus the transition are what persist the fail-closed remediation state. A CLI command failure blocks the phase. Renew an expiring lease with `sddk cycle lock renew` before evaluating gates so receipts do not become stale.

## References

- `prompts/sddk/phases/verify.md` - full phase procedure and report schema
- `prompts/sddk/phases/strict-tdd-verify.md` - conditional TDD evidence checks
- `skills/_shared/sddk-phase-common.md` - artifact and envelope protocol
- `skills/_shared/persistence-contract.md` - ledger commands
