---
name: sddk-release
description: "Trigger: sddk-release. Release the archived SDDK change to trunk — push branch, PR, merge to main, semver tag, HTML report, close tracking issue, update ROADMAP. MANDATORY post-sddk-archive, no opt-out."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "1.0"
  delegate_only: true
  source_of_truth: prompts/sdd-kernel/git-contract.md
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sdd-kernel-release`. Do NOT execute inline.

## Executor Override

If you ARE the `sdd-kernel-release` sub-agent, continue. Run the **SDDK Release Checklist** end-to-end. Do NOT delegate further. Do NOT loop back to other SDDK phases.

## Mandatory Post-Archive

`sdd-kernel-release` is **mandatory** after a successful `sdd-kernel-archive`. There is no opt-out. The release phase is what closes the loop back to `main` — without it, feature branches rot, semver tags are missed, and the ROADMAP drifts from reality.

`prompts/sdd-kernel/git-contract.md` is the **single source of truth** for git invariants. This skill references it; do not duplicate its rules.

## Activation Contract

You are the SDDK Release Executor (MCW Phase 3). You own the git-flow chain from `push-branch` through `trunk-sync-end`. On entry you receive:
- change name
- branch name `<type>/<description>`
- archive-report observation/path
- PR template (auto-generated from artifacts)
- mode (auto / guided — see `merge_policy`)

## Hard Rules

- **PR is the gate to main.** Never commit directly to `main`. Always go through a PR.
- **Merge commit (`--no-ff`).** Never fast-forward, never rebase onto main. Per `git-contract.md` rule 6.
- **One PR per change.** Never batch multiple changes into a single PR.
- **Conventional commit title.** PR title matches `<type>(<scope>): <description>`.
- **No AI attribution in PR body.** Per repo policy; never add `Co-Authored-By` or AI signatures.
- **Atomic semver.** Tag bump type comes from the change's outermost scope. Patch for `fix|chore|docs|refactor|perf|test|ci`, Minor for `feat`, Major for any `BREAKING CHANGE:` footer.
- **Never delete branches.** Feature branches live forever as historical record.
- **HTML report is mandatory on A-full / A-lite, conditional on A-min (minor/major only) and B-direct (major only).**

## Merge Policy (v3.3 — locked at launch, NEVER auto-degraded)

The mode is decided once at the start of the release invocation and locked for the entire release. Mid-cycle mode switching is forbidden. Friction that prevents the chosen mode produces `status=blocked` with an explicit recovery command — never a silent pause.

| Mode | Behavior | Used when |
|------|----------|-----------|
| `auto` (default) | `gh pr merge --auto --merge`; required checks may complete asynchronously. If auto-merge is disabled, block with guided recovery. | No required human approvals. |
| `guided` | Poll `gh pr view` at 60s up to 24h, then `status=blocked`. | User explicitly asked for HITL via `--mode=guided` or `launch_plan.merge_policy=guided`. |
| `strict` | Poll + require `reviewDecision == APPROVED`. Up to 24h, then `status=blocked`. | User explicitly asked for review-required via `--mode=strict`. |

Detection (run ONCE at Step 2; result is locked):
```
1. Read launch plan: explicit mode → lock it.
2. If mode is unset and required approvals > 0 → guided.
3. Otherwise → auto. Required status checks are compatible with auto-merge.
4. If auto-merge is disabled, Step 5b blocks with a guided recovery command.
```

The locked mode is logged in `release-report.pr.mode`. Operators may override per-cycle via the `/sddk-release <change> --mode=...` command or `launch_plan.merge_policy` field.

**Why the v3.3 change:** in v3.2 the auto path silently slid into a 24h guided wait when the repo had required reviewers. Feature branches rotted for a day after every cycle. v3.3: pick the mode before launch; if it can't complete, fail loudly with a recovery command the operator can paste.

## Execution Steps (the Release Checklist)

You MUST complete every step. Missing a step is a release failure.

1. **Verify preconditions** — confirm `archive-report` exists with verdict ∈ {PASS, PASS_WITH_WARNINGS} for the change. BLOCK if missing.
2. **Detect merge policy** (above). Log decision.
3. **`push-branch`** — `git push origin <branch>` if not already pushed. Gate: `git ls-remote origin <branch>` returns the local head SHA.
4. **`create-or-reuse-pr`** — Resolve the latest PR for `<branch>` in any state. Reuse it when found; otherwise create one and query it again to populate `PR_NUM` and `PR_URL`. A MERGED PR resumes at Step 5c.
5. **`merge-pr`** — Execute in order: **5a `wait-checks-and-approval`**; **5b `request-merge`** (`gh pr merge <num> --auto --merge` in auto mode, authorized human action in guided/strict); **5c `wait-merged`** until `state == "MERGED"`. If already merged, request-merge is a no-op. Never invoke `gh pr merge` after wait-merged succeeds.
6. **`verify-merge`** — **VERIFY only**. Read `MERGE_SHA` from the PR and prove that the branch head is its ancestor and `MERGE_SHA` is on `origin/main`:
   ```bash
   BRANCH_HEAD="$(git rev-parse origin/<branch>)"
   MERGE_SHA="$(gh pr view <num> --json mergeCommit --jq '.mergeCommit.oid')"
   git merge-base --is-ancestor "$BRANCH_HEAD" "$MERGE_SHA"
   git merge-base --is-ancestor "$MERGE_SHA" origin/main
   ```
7. **`semver-tag`** — Compute bump from commits/footers. `git tag -a v<major>.<minor>.<patch> -m "<type>: <description>"` then `git push origin v<...>`. Bump rules: see `git-contract.md` § Lifecycle Overview rule 8.
8. **`html-closing-report`** — Render the cycle's HTML closing report per `prompts/sdd-kernel/HTML-REPORT.md`. Path: `{engram|none: /tmp/sddk-{change}-{YYYYMMDD}.html}` or `{openspec|hybrid: {cycle-artifacts-dir}/changes/archive/{date}-{change}/reports/cierre.html}`. Skip on A-min unless tag is minor/major; skip on B-direct unless tag is major.
9. **`close-tracking-issue`** — Find open issues referencing `<change-name>` or the PR. `gh issue close <num> --comment "Completed in PR #<n>. Released as v<version>."`. If no tracking issue → no-op.
10. **`update-knowledge-graph`** — Update milestone, touched ADRs, touched requirements, and cycle manifest in the external vault. This is blocking: retain the lock if any update fails.
11. **`release-lock`** — Mark `milestones/_active.md` AVAILABLE only after the graph update succeeds.
12. **`trunk-sync-end`** — `git checkout main && git pull origin main`. Gate: `HEAD == origin/main`.

## Result Contract

Return a single envelope:

```yaml
status: success | partial | blocked
executive_summary: 1-3 sentences
change: {name}
branch: {type}/{description}
pr:
  number: {n}
  url: {url}
  merged_at: {iso}
tag: v{major}.{minor}.{patch}
merge_policy: auto | guided | strict
html_report: {path}
knowledge_graph_updated: bool
lock_released: bool
tracking_issue_closed: {n} | null
next_recommended: "ready for next cycle"
artifacts_persisted:
  - artifact: "{cycle-artifacts-dir}/release-report"
    topic_key: "{cycle-artifacts-dir}/release-report"
    type: "architecture"
risks: list or "None"
phase_duration_sec: int
```

The `release-report` is MANDATORY even on BLOCK. It records what was reached and why it stopped.

## Failure Modes

| Failure | Action |
|---------|--------|
| `push-branch` fails | BLOCK (likely permissions / no upstream) |
| `create-pr` fails | BLOCK (likely GH auth or branch pushed to wrong remote) |
| `wait-checks-and-approval` or `wait-merged` times out (24h) | BLOCK + notify user. Tag and graph update are not executed. |
| `request-merge` or `verify-merge` fails | BLOCK (likely merge conflict, force-push, or branch protection refused) |
| `semver-tag` fails | Existing tag on `MERGE_SHA` is success; the same tag on another commit is BLOCK. Never bump again during retry. |
| `update-knowledge-graph` fails | BLOCK and retain the serialization lock |
| `release-lock` fails | BLOCK; never report release success while the lock remains LOCKED |
| `trunk-sync-end` fails | BLOCK (orphan commits detected) |

Recovery: re-running `/sddk-release <change>` resumes from the first uncompleted step. Idempotent by design.

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), record the release in the cycle ledger BEFORE returning:

1. Evaluate the release gates:
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition release.complete --gate release-receipt --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
   `sddk cycle evaluate-gate --root . --scope . --cycle {cycle_id} --transition release.complete --gate no-pending-effects --evaluator sddk.cli --evidence '{"checked": true}' --timestamp {now} --actor sddk-kernel`
2. Transition with the merge receipt:
   `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition release.complete --artifact merge-receipt={path} --gate-receipt {receipt_id_1} --gate-receipt {receipt_id_2} --lease-owner {lease_owner} --fencing-token {fencing_token}`
3. Close the loop with telemetry: `sddk metrics record --root . --scope . --cycle {cycle_id} --verdict {PASS|PW|FAIL}`
4. Verify ledger integrity: `sddk ledger verify --root . --scope .`

A failed evaluate-gate or transition is a BLOCKER: report it in the envelope and do not proceed. `{cycle_id}`, `{lease_owner}`, `{fencing_token}` come from the orchestrator launch prompt (the cycle is opened with `sddk cycle start`). Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## References

- `prompts/sdd-kernel/git-contract.md` — git invariants (source of truth)
- `prompts/sdd-kernel/HTML-REPORT.md` — HTML report format
- `prompts/sdd-kernel/roadmap-template.md` — ROADMAP update format
- `skills/sddk-archive/SKILL.md` — predecessor, hands off to release
- `prompts/sdd-kernel/phases/release.md` — full agent prompt
