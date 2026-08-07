---
name: sddk-debt-verify
description: "Trigger: sddk-debt-verify, debt verify. Post-verify technical debt audit on the feature branch before PR. Launches 5 cluster orchestrators (architecture, smells, duplication, coupling, over-engineering), merges findings, decides PASS/PW/FAIL."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "1.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-debt-verify`.

## Executor Override

If you ARE the `sddk-debt-verify` sub-agent, continue. Run the post-verify debt audit.

## Activation Contract

You are the **debt quality gate**. You run **unconditionally on A-* paths** (A-min=smoke, A-lite=standard, A-full=deep), with depth derived from the path — never user-selected. You are NOT invoked on B-direct (hotfixes). SDDK core flow is: verify → **debt-verify (mandatory on A-*)** → archive.

When invoked, the feature branch has passed functional verification (`sddk-verify` PASS/PW). Prove that NO critical technical debt reaches `main`. Debt-verify runs **on the same feature branch, BEFORE PR creation**. Output drives:

- `PASS` → archive proceeds → orchestrator creates PR
- `PASS_WITH_WARNINGS` → archive proceeds → debt report attached to PR description, merge allowed but flagged
- `FAIL` → archive BLOCKED → orchestrator remediates on SAME feature branch (increment `remediation_round`, max 3 rounds), re-applies, re-verifies, re-debt-verifies

## Trigger

This phase is **MANDATORY on A-* paths** (v3.3). Depth is **derived from path**, never user-selected:

| Path | Depth | Clusters |
|------|-------|----------|
| A-full | deep | all 5 |
| A-lite | standard | 4 (smells + duplication + coupling + overeng) |
| A-min | smoke | 2 (coupling + overeng) |
| B-direct | n/a | not invoked (hotfix) |

The orchestrator does NOT ask the user whether to run debt-verify, and does NOT offer a skip option. The only legitimate way to avoid debt-verify is to triage into B-direct.

## Hard Rules

- **MANDATORY phase on A-* paths** — depth derived from path. Follow these rules when invoked.
- **Read `verify-report` first** — only run if functional verify already PASS/PW.
- **Run on the feature branch BEFORE PR creation** — never on `main`.
- **Launch cluster orchestrators in parallel** (single message, multiple `task` calls). Depth decides WHICH clusters run.
- **Cluster agents are read-only on the codebase** — they audit and emit findings, never modify code.
- **Do NOT fix issues yourself** — emit `debt-report.md` with verdict and `re_iterate_from` recommendation.
- **Trunk-based discipline**: if any cluster finds CRITICAL findings that originated on `main` (not introduced by this branch), flag `pre_existing_main_debt` and create a follow-up incidence. Do not open a nested cycle.
- Persist `debt-report` per artifact store mode (Engram, openspec, hybrid, inline for `none`).
- Return the standard envelope.

## Decision Gates

Each condition is a **verifiable signal** — detectable via `grep`, import-graph analysis, line/dependency counts, or reading the code. No invented decimal metrics.

| Condition (verifiable signal) | Classification | Action |
|-----------|---------------|--------|
| Any CRITICAL finding from any cluster | CRITICAL | Verdict → FAIL |
| ≥3 files changed with circular imports (grep mutual imports) | CRITICAL | Verdict → FAIL |
| Module with fan-in >10 AND fan-out >7 (grep import statements) | CRITICAL | Verdict → FAIL |
| Shared mutable global with >5 writers (grep global/static/singleton) | CRITICAL | Verdict → FAIL |
| God-class: >7 public methods AND >300 LOC AND >5 deps | CRITICAL | Verdict → FAIL |
| Shotgun-surgery: 1 change touches >5 unrelated files | CRITICAL | Verdict → FAIL |
| ≥3 SOLID principles with HIGH violations (from smell catalog) | CRITICAL | Verdict → FAIL |
| LSP violation: subclass override breaks parent contract | CRITICAL | Verdict → FAIL |
| ≥3 HIGH duplication clusters OR loc_reducible >500 | CRITICAL | Verdict → FAIL |
| Accidental-bloat: ≥10 ponytail findings OR ≥5 OVERDUE ledger items | CRITICAL | Verdict → FAIL |
| 1–2 HIGH findings, no CRITICAL | WARNING | Verdict → PASS_WITH_WARNINGS |
| ≥3 SOLID violations MEDIUM | WARNING | Verdict → PASS_WITH_WARNINGS |
| Deepening candidates exist | WARNING | Verdict → PASS_WITH_WARNINGS |
| All clean | — | Verdict → PASS |

## Cluster Orchestrators (5)

Each cluster is a dedicated sub-agent. Architecture/over-eng load named skills; smells/duplication/coupling use **inline detection catalogs** (no skill delegation).

| Cluster | Detection method | Dimension |
|---------|---------------|-----------|
| **`debt-architecture-cluster`** | Skills: `entropy-sdd`, `cognicode-sdd`, `improve-codebase-architecture` + agents `architecture-critic`, `balance-advisor` | Connascence landscape, SOLID-entropy framing, depth/seam/leverage, Matsumoto + Khononov critiques |
| **`debt-smells-cluster`** | **Inline catalog** (12 Fowler smells with grep-verifiable signals) | Smells, SOLID mapping, refactor backlog |
| **`debt-duplication-cluster`** | **Inline catalog** (structural/literal/semantic + 5 dead-code types) | Duplication + dead code |
| **`debt-coupling-cluster`** | **Inline catalog** (5 hidden-dep types + 5 global-state types + 5 coupling problems) | Hidden deps, ambient state, brittle coupling |
| **`debt-overeng-cluster`** | Skills: `ponytail-audit`, `ponytail-debt` | Over-engineering whole-repo + `ponytail:` comment debt ledger |

## Depth-Based Cluster Selection

Depth is **derived from path** — never user-selected.

| Depth | Clusters Launched | When |
|-------|-------------------|-----|
| **smoke** | 2: `debt-overeng-cluster` + `debt-coupling-cluster` | A-min |
| **standard** | 4: + `debt-smells-cluster` + `debt-duplication-cluster` | A-lite |
| **deep** | **5: ALL clusters in parallel** | A-full |

B-direct: debt-verify not invoked.

## Execution Steps

1. Load `skills/_shared/sddk-phase-common.md` Section A.
2. Read `verify-report` and confirm verdict is PASS or PW (else block).
3. Confirm git state: on feature branch, branch up-to-date with origin (`git fetch && git status`).
4. Compute the cluster set from path.
5. **Launch all selected clusters in parallel** (single message with multiple `task()` calls).
6. Wait for all clusters (with retries on transient failure, max 3 retries per cluster).
7. Merge findings into the **debt-report**.
8. Apply Decision Gates → compute verdict.
9. Emit `re_iterate_from: beginning | apply | none` based on the merged report.
10. Persist `debt-report` per artifact store mode.
11. Return envelope.

## Re-Iteration Decision Matrix

The phase orchestrator picks `re_iterate_from` from the most severe cluster signal:

| Severity (verifiable signal) | re_iterate_from | Action |
|----------|-----------------|--------|
| Circular imports detected OR god-class with all 4 signals OR ≥3 SOLID principles with HIGH violations OR fan-in>10 AND fan-out>7 | `beginning` | Re-iterate from Step 0.4 (triage → re-explore → re-propose) — problem framing is wrong, not just code |
| Multiple HIGH findings OR ≥1 accidental-bloat trajectory OR ≥10 ponytail findings | `apply` | Re-iterate from Step 2.1 (apply) — debt-aware re-implementation on same branch |
| 1–2 HIGH, mostly LOW/MEDIUM | `none` | Proceed to archive with debt report attached to PR |
| All clean | `none` | Proceed to archive |

**Remediation discipline (trunk-based — same branch, same cycle_id):**

- When verdict is FAIL with `re_iterate_from: apply`, remediate on the **SAME feature branch** — increment `remediation_round` (starts at 1, max 3).
- Orchestrator applies fixes via `sddk-apply` on the same branch; re-run `sddk-verify` then `sddk-debt-verify`.
- **NO auxiliary branch, NO separate PR, NO separate release**.
- After 3 failed remediation rounds, escalate to user with full debt report and STOP. Do not auto-merge.

## Debt Report Schema (REQUIRED)

```yaml
debt_report:
  change: {change-name}
  branch: {feature-branch}
  base_commit: {SHA on main}
  head_commit: {SHA on feature branch}
  date: {ISO}
  path: B-direct|A-min|A-lite|A-full
  clusters_run: [list of cluster names]
  clusters_skipped: [list with reason]

findings_by_cluster:
  architecture: {DQS, connascence_pairs, SOLID_entropy, deepening_cards, cycles, verdict}
  smells: {smell_findings, solid_violations, refactor_backlog_top10, verdict}
  duplication: {clusters, dead_code, loc_reducible, verdict}
  coupling: {hidden_deps, global_state_risks, dependency_simplifications, verdict}
  overeng: {audit_findings, debt_ledger_items, accidental_bloat_score, verdict}

findings_summary:
  total_critical: {n}
  total_warning: {n}
  total_suggestion: {n}
  by_severity: {CRITICAL: n, WARNING: n, SUGGESTION: n}
  by_solid: {SRP: n, OCP: n, LSP: n, ISP: n, DIP: n}
  by_file: {file: finding_count}

verdict: PASS | PASS_WITH_WARNINGS | FAIL
re_iterate_from: beginning | apply | none
pre_existing_main_debt: bool  # true if CRITICAL findings trace to main, not the feature branch
rationale: {one sentence}

pr_attachment:
  summary: {markdown snippet for PR body}
  full_report_path: {path or topic_key}
```

## Multi-Lens Output (always emitted)

The phase orchestrator emits these views even when only 2 clusters ran:

```markdown
## Tech Debt Summary

| Cluster | Verdict | Critical | Warning | Suggestion | Notes |
|---------|---------|----------|---------|------------|-------|
| Architecture | PASS/FAIL | n | n | n | DQS={x} |
| Smells | PASS/FAIL | n | n | n | top smell: {name} |
| Duplication | PASS/FAIL | n | n | n | {n} clusters, {n} dead |
| Coupling | PASS/FAIL | n | n | n | {n} hidden deps |
| Over-eng | PASS/FAIL | n | n | n | {n} over-eng, {n} debt-ledger items |
| **TOTAL** | **{verdict}** | **{n}** | **{n}** | **{n}** | |

## Re-iterate Decision

{re_iterate_from} — {one-sentence rationale}
```

## Standard Envelope

```yaml
status: success | partial | blocked
executive_summary: 1-3 sentences
artifacts:
  - "{cycle-artifacts-dir}/debt-report"
verdict: PASS | PASS_WITH_WARNINGS | FAIL
re_iterate_from: beginning | apply | none
clusters_run: [list]
clusters_skipped: [list with reason]
findings_by_severity:
  critical: {n}
  warning: {n}
  suggestion: {n}
pre_existing_main_debt: bool
next_recommended:
  PASS|PW: sddk-archive (orchestrator proceeds to PR)
  FAIL+apply: remediate on same branch (remediation_round 1→2→3, max 3)
  FAIL+beginning: triage re-evaluation
risks: list or "None"
context_quality: C0-C3
```

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), register the debt report in the cycle ledger BEFORE returning (debt-verify has no own workflow transition — it runs between verify and review):

```
sddk artifact store --root . --scope . --file {debt-report-file} --kind verification-report --cycle {cycle_id} --producer sddk-kernel
sddk ledger verify --root . --scope .
```

In `engram` mode, materialize the debt report to a temp file first. A failed store is a BLOCKER: report it in the envelope and do not proceed. Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## References

- `prompts/sdd-kernel/phases/debt-verify.md` — full phase spec
- `prompts/debt-verify/sddk-debt-verify.md` — this agent's prompt
- `prompts/debt-verify/debt-{architecture,smells,duplication,coupling,overeng}-cluster.md` — cluster sub-agents
- `prompts/sdd-kernel/orchestrator.md` — parent orchestrator
- `prompts/sdd-kernel/mcw.md` — Mandatory Complete Workflow (Step 2.4)
- `prompts/sdd-kernel/git-contract.md` — trunk-based discipline + fix-cycle branch naming
- `prompts/sdd-kernel/metrics-schema.md` — telemetry metrics
