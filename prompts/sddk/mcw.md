# Mandatory Complete Workflow (MCW)

Source of truth for end-to-end SDDK execution. Every cycle MUST follow this numbered sequence. Skipping any step = broken pipeline.

The MCW runs in **5 phases**, each with numbered steps. Hard gates only where stated.

For the triage gate that selects which path (B-direct / A-min / A-lite / A-full) executes this MCW, see `prompts/sddk/orchestrator.md` § Triage.

---

## Phase 0 — Cycle Pre-flight

### Step 0.1 — Trunk Sync (MANDATORY)

```
git fetch origin main
git checkout main && git pull origin main
```

Hard gate: `git rev-parse HEAD == origin/main`. If `main` cannot be pulled → BLOCK.

### Step 0.2 — Previous Cycle Consolidation Check

Verify previous cycle is fully closed. **Do NOT use remote branch existence as the primary signal** — stale branches linger. Instead, consult the knowledge graph vault first, then use Git as secondary verification.

**Primary check — consult lock/vault/cycle state:**
```bash
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
# Vault resolved by stable project identity, never from a basename (ADR-0011)
VAULT="$(sddk knowledge path --root "$PROJECT_ROOT" --scope .)"

# Check serialization lock
if [ -f "$VAULT/milestones/_active.md" ]; then
  if grep -q "LOCKED" "$VAULT/milestones/_active.md" 2>/dev/null; then
    # Cycle in progress — extract milestone name
    MILESTONE=$(grep "Milestone:" "$VAULT/milestones/_active.md" | head -1)
    BLOCK "Cycle in progress — $MILESTONE"
  fi
fi

# Tags are historical evidence only. They never override an active lock.
```

**Secondary check — Git verification only (not primary):**
```bash
# Detect default branch and primary remote dynamically
PRIMARY_REMOTE=$(git remote | head -1)
[ -n "$PRIMARY_REMOTE" ] || PRIMARY_REMOTE="origin"
DEFAULT_BRANCH=$(git symbolic-ref "refs/remotes/$PRIMARY_REMOTE/HEAD" 2>/dev/null | sed "s|refs/remotes/$PRIMARY_REMOTE/||")
[ -n "$DEFAULT_BRANCH" ] || DEFAULT_BRANCH="main"

# Normalize branch output before filtering; git indents remote branch names.
UNMERGED=$(git branch -r --no-merged "$PRIMARY_REMOTE/$DEFAULT_BRANCH" \
  | sed 's/^[*[:space:]]*//' \
  | grep -E "^${PRIMARY_REMOTE}/(feat|fix|refactor|chore|perf|test|docs)/" \
  || true)
[ -z "$UNMERGED" ] || BLOCK "Unmerged cycle branches: $UNMERGED"

# Check for open PRs from this actor
OPEN_PRS=$(gh pr list --state open --author @me --base "$DEFAULT_BRANCH" --json number,title,headRefName)
[ "$OPEN_PRS" = "[]" ] || BLOCK "Open PRs from a previous cycle: $OPEN_PRS"
```

Hard gate: the serialization lock is AVAILABLE **AND** there are no matching unmerged cycle branches **AND** no open PR from a prior cycle. A historical tag never overrides an active lock.

If unmerged branches exist: resume that cycle, do not start new one.


### Step 0.3 — Knowledge Coverage Check (A-full only)

Resolve milestone, work items, ADRs, architecture context, ownership, and learnings from the external knowledge vault.

```bash
cat "$VAULT/milestones/_active.md" 2>/dev/null
ls "$VAULT"/milestones/M-*.md "$VAULT"/adrs/ADR-*.md 2>/dev/null
cat "$VAULT/_index.md" 2>/dev/null
```

Hard gates:
- Vault index or milestone state missing → BLOCK and repair adoption/knowledge state.
- Cycle's milestone missing from the vault → add it before proceeding.
- ADR with `superseded by ADR-NNN` where NNN doesn't exist → block.

For A-lite/A-min/B-direct: skip this step.

### Step 0.4 — Triage

Run the triage gate (C0-C3 + jurisprudence + path selection). Output: selected path, lenses, F3 tuning. Inject into next phase launch plan. See `prompts/sddk/orchestrator.md` § Triage and `prompts/sddk/decision-model.md` § Path Selection.

---

## Phase 1 — Plan

### A-full: explore → propose → spec+design → tasks

**Step 1.1 — Explore** (A-full only)

Delegate to `sddk-explore`. Output: `explore-report.md` with context quality (C0-C3) and taxonomy.

Hard gate: artifact approved.

**Step 1.2 — Propose**

Delegate to `sddk-propose`. Output: `proposal.md`.

**Step 1.3 — Coherence Check (propose → spec)** (A-full only)

Score ≥ 60 to proceed.

**Step 1.4 — Spec + Design (PARALLEL)** (A-full only)

Delegate spec + design concurrently. Both required before tasks.

**ADR Creation**: if spec/design contains architectural decisions → write ADR before tasks.

Hard gate: both spec and design approved.

**Step 1.5 — Coherence Check (spec+design → tasks)** (A-full only)

Score ≥ 60.

**Step 1.6 — Tasks**

Delegate to `sddk-tasks`. Output: `tasks.md` with file lists, commit messages, scope.

**Step 1.7 — Review Budget Guard**

Inspect `tasks.md` for forecast.

| Forecast | Action |
|----------|--------|
| ≤ 400 LOC | Proceed normally |
| > 400 LOC + `single-pr-default` | STOP, require user exception |
| > 400 LOC + `auto-forecast` | Load `skill(name="chained-pr")` |
| > 400 LOC + `force-chained` | Proceed with chained PRs |
| > 400 LOC + `ask-always` | STOP, ask user |

**Step 1.8 — Branch Creation** (after tasks)

```
git checkout -b <type>/<description>
git push -u origin <type>/<description>
```

Hard gate: branch matches `^[a-z]+/[a-z0-9-]+$`, type in `feat|fix|chore|docs|refactor|perf|test|ci|revert`.

For A-lite/A-min/B-direct: branch creation happens before apply (inline step), same rule.

### A-lite

Phases: `propose → spec → apply → verify`. Coherence: 1 (apply→verify). Skip explore, design, tasks, coherence (propose→spec and spec+design→tasks).

### A-min

Phases: `spec → apply → verify`. Coherence: 0 unless spec complexity high.

### B-direct

Load skill → execute → light verify. No SDDK phases.

---

## Phase 2 — Build

### Step 2.1 — Apply

Delegate to `sddk-apply`. Output: atomic conventional commits on branch.

Hard gate: every commit passes the declarative git checklist (type/scope/imperative/72-char/no AI attribution).

The apply phase follows `prompts/sddk/phases/apply.md`, which loads `phases/apply-strict-tdd.md` conditionally when `strict_tdd_mode: true` in the launch plan. The orchestrator sets this from project testing-capabilities (cached during sddk-init).

Within apply, the per-task inner loop (Loop Engineering L3) runs Razonar→Actuar→Observar→Evaluar with:
- `per_task_max_attempts` hard brake (default 5)
- No-progress streak detection (default 3 same signatures → BLOCK)
- Strict TDD discipline when active (RED→GREEN→TRIANGULATE→REFACTOR)
- Safety Net (pre-existing failure detection)
- Merge Protocol (no overwrite of prior apply-progress)

### Step 2.2 — Coherence Check (apply → verify) (A-full, A-lite)

Score ≥ 60.

### Step 2.3 — Verify

Delegate to `sddk-verify`. Output: `verify-report.md` with test pyramid, lens verdicts, verdict (PASS / PW / FAIL).

Hard gate: PASS or PW. If FAIL → return to Step 2.1 (correction cycle).

### Step 2.4 — Debt-Verify (v3.3 — MANDATORY on A-*, n/a on B-direct)

> **Mandatory step on A-*.** Triggers unconditionally after `sddk-verify` returns PASS or PW. Depth is **derived from path** — the orchestrator NEVER asks the user, and there is no skip option inside A-* paths. B-direct (hotfix) does NOT invoke debt-verify.

**Path-derived depth (locked, no user choice):**

| Path | Depth | Clusters |
|------|-------|----------|
| A-full | **deep** | architecture + smells + duplication + coupling + overeng (5 clusters) |
| A-lite | **standard** | smells + duplication + coupling + overeng (4) |
| A-min | **smoke** | coupling + overeng (2) |
| B-direct | n/a — debt-verify not invoked | — |

Delegate to `sddk-debt-verify`. Output: `debt-report.md` with cluster verdicts, severity aggregation, pre-existing-main-debt detection, verdict (PASS / PW / FAIL), and `re_iterate_from` recommendation.

The phase orchestrator launches cluster agents in parallel based on the chosen depth:

| Depth | Clusters launched (parallel) |
|---|---|
| smoke | 2: `debt-overeng-cluster` + `debt-coupling-cluster` |
| standard | 4: + `debt-smells-cluster` + `debt-duplication-cluster` |
| deep | **5: ALL clusters in parallel** (`debt-architecture-cluster`, `debt-smells-cluster`, `debt-duplication-cluster`, `debt-coupling-cluster`, `debt-overeng-cluster`) |

Each cluster agent emits its dimension verdict. The phase orchestrator merges them and applies Decision Gates:

| Condition | Verdict |
|-----------|---------|
| Any CRITICAL from any cluster, OR ≥3 HIGH, OR ≥3 SOLID CRIT, OR DQS < 0.3, OR connascence > 5 bits, OR cycles, OR god-class CRIT, OR ≥10 ponytail | **FAIL** |
| 1–2 HIGH, no CRITICAL, OR ≥3 SOLID MEDIUM, OR deepening candidates | **PASS_WITH_WARNINGS** |
| All clean | **PASS** |

**Re-iteration decision matrix** (drives next action):

| Severity | re_iterate_from | Action |
|----------|-----------------|--------|
| DQS < 0.3 OR connascence > 5 bits OR new cycles OR god-class CRIT OR ≥3 SOLID CRIT | `beginning` | Re-iterate from Step 0.4 (triage → re-explore → re-propose) — problem framing is wrong |
| Multiple HIGH OR ≥1 accidental-bloat OR ≥10 ponytail | `apply` | **Remediate on SAME feature branch** — increment `remediation_round`, apply fixes, re-verify, re-debt-verify (max 3 rounds) |
| 1–2 HIGH, mostly LOW/MEDIUM | `none` | Proceed to Step 2.5 (archive) with debt report attached to PR |
| All clean | `none` | Proceed to Step 2.5 (archive) |

**Remediation discipline (trunk-based — same branch, same cycle_id)**:
- Remediation happens on the **SAME feature branch** — increment `remediation_round` (starts at 1, max 3).
- Orchestrator applies fixes via `sddk-apply` on the same branch.
- Re-run `sddk-verify` then `sddk-debt-verify` with incremented `remediation_round`.
- **NO auxiliary branch, NO separate PR, NO separate release**.
- **Max 3 remediation rounds**. After round 3 fails → escalate to user with full debt report and STOP. No auto-merge.

**Pre-existing main debt detection**: if any CRITICAL finding traces to a commit on `main` BEFORE the feature branch was created, flag `pre_existing_main_debt: true` and create a follow-up incidence. It does not create a second cycle inside the active cycle.

**Hard gate**: PASS or PW. FAIL → enter remediation on the same branch and `cycle_id`. There is no skip path; depth is path-derived.

### Step 2.5 — Coherence Check (verify → archive) (A-full only)

Score ≥ 60. Runs AFTER debt-verify so the coherence score reflects both functional and debt dimensions.

### Step 2.6 — Archive

Delegate to `sddk-archive`. Output: `archive-report.md` with knowledge impact, entropy trend, and debt-report attachment.

The archive report embeds the debt summary so it travels with the PR description.

---

---

## Phase 3 — Consolidate (after archive)

### Step 3.1 — Push Branch (MANDATORY)

```
git push origin <branch>
```

Hard gate: `git ls-remote origin <branch>` returns latest local commit SHA.

### Step 3.2 — Create or Reuse Pull Request (MANDATORY)

```
gh pr list --head <branch> --state all --json number,url,state
# If no PR exists in any state:
gh pr create --base main --head <branch> --title "<type>(<scope>): <description>" --body "..."
# Query again to populate PR number and URL after creation.
```

PR body includes: summary, test plan, artifacts, tracking issue.

Hard gate: `gh pr view --json number,url` returns valid PR.

### Step 3.3 — Checks, Approval, Merge Request, and MERGED Gate

```
gh pr checks <pr-number> --watch
# auto mode, after required checks/approval:
gh pr merge <pr-number> --auto --merge
# guided/strict: authorized human performs the merge action.
# Then poll:
gh pr view <pr-number> --json state
```

Required order: wait checks and required approval → request merge or enable auto-merge → wait until `state == MERGED`. Timeout (default 24h, configurable) → BLOCK and notify the user. If the PR is already merged, the merge request is an idempotent no-op.

### Step 3.4 — Verify Merge SHA

The PR is already MERGED. Verify only; do not invoke `gh pr merge` in this step:

```bash
git fetch origin main <branch>
BRANCH_HEAD="$(git rev-parse origin/<branch>)"
MERGE_SHA="$(gh pr view <pr-number> --json mergeCommit --jq '.mergeCommit.oid')"
git merge-base --is-ancestor "$BRANCH_HEAD" "$MERGE_SHA"
git merge-base --is-ancestor "$MERGE_SHA" origin/main
```

Hard gate: branch head is an ancestor of the PR merge commit, and the merge commit is an ancestor of the remote default branch.

### Step 3.5 — Create Semver Tag (MANDATORY)

Bump derives from the PR commit headlines and bodies:

| Change type | Bump |
|-------------|------|
| Breaking public API/contract | `major` |
| New feature (non-breaking) | `minor` |
| Bug fix, chore, docs, refactor | `patch` |

Use the dependency-free, idempotent algorithm in `prompts/sddk/phases/release.md` Step 7. If a semver tag already points to `MERGE_SHA`, reuse it. Otherwise compute the next version once, tag `MERGE_SHA`, and push. Never bump again merely because release resumed.

Hard gate: exactly one selected semver tag points to `MERGE_SHA` and exists on the remote.

### Step 3.6 — HTML Closing Report (CONDITIONAL)

- A-full: always.
- A-lite: always.
- A-min: only if tag is `minor` or `major`.
- B-direct: only if tag is `major`.

Generate via `sddk-archive` agent (uses `prompts/sddk/HTML-REPORT.md`):

```
xdg-open <report-path>
```

Path routing:
- `/tmp/sddk-<change>-<YYYYMMDD>.html` + `$SDDK_DATA_DIR/projects/{project_id}/changes/<change>/reports/cierre.html`

Hard gate (when required): HTML exists and non-empty.

### Step 3.7 — Close Tracking Issue (if any)

```
gh issue close <issue-number> --comment "Completed in PR #<pr-number>. Released as v<version>."
```

### Step 3.8 — Update Knowledge Graph (MANDATORY)

Update the external vault milestone, touched ADRs, touched requirements, and cycle manifest with PR, tag, verdicts, completion time, and report path. Append every mutation to `_log.md`.

Hard gate: milestone and cycle manifest are `completed`; touched ADRs and requirements reference this cycle and tag. Failure BLOCKS release and retains the serialization lock.

### Step 3.9 — Release Serialization Lock (MANDATORY)

Mark `milestones/_active.md` AVAILABLE only after Step 3.8 succeeds. Failure BLOCKS release; never announce next-cycle readiness with a stale lock.

---

## Phase 4 — Trunk Sync + F3 + Reset

### Step 4.1 — Sync Local Main

```
git checkout main && git pull origin main
```

Hard gate: HEAD == origin/main.

### Step 4.2 — F3 Self-Tuning

1. Read `metrics/aggregate` from Engram.
2. Apply self-tuning signals table (see `prompts/sddk/lateral-thinking.md`).
3. Write tuning block to `sddk/next-tuning.md`.
4. Append cycle metrics to `$SDDK_DATA_DIR/projects/{project_id}/metrics/{cycle_id}.jsonl`.
5. Mirror as Engram observation with `topic_key: cycle-metrics/{cycle_id}`.
6. Update `metrics/aggregate` rolling 7d/30d.

This replaces the old `.sddk-last-cycle-complete` marker file. Tag presence on main commit = cycle closed; F3 metrics aggregation = jurisprudence updated.

### Step 4.3 — Save Jurisprudence (conditional)

If cycle had `verify_verdict=PASS` + `first_pass_success=true` + reusable decision (ADR, lens, atajo):

```
mem_save(
  topic_key: jurisprudence/{category},
  title: "{goal_pattern} — {path_that_worked}",
  type: jurisprudence,
  content: {jurisprudence schema per decision-model.md}
)
```

### Step 4.4 — Print Result Contract + Next-Cycle Ready

```
✓ Cycle {goal_pattern} closed
  Path: {path} (C{x}, jurisprudence: {n} hits)
  Verdict: {verdict} {first_pass_badge}
  Lead time: {h}h  |  Cost: ${usd}  |  Tokens: {n}
  Spec coverage: {passing}/{total} scenarios ({pct}%)
  PR #{n} → main @ {tag}
  Bottleneck: {phase} ({reason})
  Saved as jurisprudence: {topic_key} {if reusable}

  vs rolling {window}:
    - first_pass_success_rate: {value} ({delta})
    - median_lead_time: {value}h ({delta})
    - top_bottleneck_phase: {phase} ({you_too|new})

Ready for next cycle.
```

---

## Abort Patterns

| Scenario | Action |
|----------|--------|
| spec fails | Block design. Fix spec first. |
| design fails | Block tasks. Fix design first. |
| apply fails | Rollback to last checkpoint. Re-apply from pending. |
| verify fails | Fix in apply, re-verify. Do not skip. |
| coherence < 60 | BLOCK. Resolve contradiction. |
| artifact registry unreachable | Block. Use last-known state, mark `unverified`. |
| PR not merged within timeout | BLOCK. Notify user. |
| Tag push fails | BLOCK. Investigate permissions. |
| HTML report fails (when required) | BLOCK. Re-generate via sddk-archive. |
| Per-task attempts > CIRCUIT_PER_TASK_MAX_ATTEMPTS | BLOCK. Escalate to user (loop engineering freno duro). |

Abort commit format (mid-cycle abandon):
```
chore(abort): abandoning <change> — <reason>

Reason: <what went wrong>
Last checkpoint: <task-id>
```

---

## Anti-Patterns (FORBIDDEN)

| Anti-pattern | Consequence | Enforcement |
|--------------|-------------|-------------|
| Committing directly to main | Bypasses review | Checklist blocks |
| Force-pushing to main | Destroys history | Checklist blocks |
| Rebasing feature branches | Loses review history | Checklist blocks |
| Starting new cycle without closing previous | Two cycles open | Step 0.2 gate |
| Skipping PR review | Merge of unreviewed code | Step 3.3 gate |
| Merging without PR | Bypasses review | Step 3.4 verifies |
| Skipping semver tag | Lost milestone | Step 3.5 gate |
| Skipping trunk sync | Working on stale main | Step 0.1 + 4.1 gates |
| Co-Authored-By in commit | AI attribution leaked | Checklist blocks |
| Running full SDDK for C3 fix | Waste | Use B-direct via triage |
| Coherence check on B-direct | Theater | Skipped by path |
| HTML report for patch tag | Overhead | Skipped by path |

---

## Quick Reference — MCW Step Index

| Phase | Step | Action | Hard gate |
|-------|------|--------|-----------|
| 0 | 0.1 | Trunk sync | HEAD == origin/main |
| 0 | 0.2 | Previous cycle closed | No unmerged branches/PRs |
| 0 | 0.3 | Knowledge coverage (A-full) | No critical gaps |
| 0 | 0.4 | Triage | Path decided |
| 1 | 1.1 | Explore (A-full) | explore-report approved |
| 1 | 1.2 | Propose | proposal approved |
| 1 | 1.3 | Coherence propose→spec (A-full) | ≥ 60 |
| 1 | 1.4 | Spec+Design parallel (A-full) | Both approved |
| 1 | 1.5 | Coherence spec+design→tasks (A-full) | ≥ 60 |
| 1 | 1.6 | Tasks | tasks approved |
| 1 | 1.7 | Review budget | Forecast ≤ budget |
| 1 | 1.8 | Branch creation | Name matches regex |
| 2 | 2.1 | Apply | Commits pass declarative git checklist |
| 2 | 2.2 | Coherence apply→verify (A-full, A-lite) | ≥ 60 |
| 2 | 2.3 | Verify | PASS or PW |
| 2 | 2.4 | **Debt-verify (MANDATORY on A-*; n/a on B-direct; depth derived from path)** | PASS or PW |
| 2 | 2.5 | Coherence verify→archive (A-full) | ≥ 60 |
| 2 | 2.6 | Archive | archive-report registered |
| 3 | 3.1 | Push branch | ls-remote matches |
| 3 | 3.2 | Create PR | gh pr view valid |
| 3 | 3.3 | Checks + merge request + wait | PR MERGED |
| 3 | 3.4 | Verify merge | Branch head and merge SHA are ancestors of main |
| 3 | 3.5 | Semver tag | Tag pushed |
| 3 | 3.6 | HTML report (conditional) | File exists |
| 3 | 3.7 | Close issue | Issue CLOSED |
| 3 | 3.8 | Update knowledge graph | Vault nodes completed |
| 3 | 3.9 | Release lock | Lock AVAILABLE |
| 4 | 4.1 | Sync main | HEAD == origin/main |
| 4 | 4.2 | F3 tuning + metrics | Tuning written |
| 4 | 4.3 | Jurisprudence (conditional) | Observation saved |
| 4 | 4.4 | Result contract | User notified |
