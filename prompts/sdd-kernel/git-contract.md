# SDD Kernel Git Contract v2

This contract defines how kernel SDD phases integrate with git operations and the full change lifecycle. It is technology-agnostic: the rules apply regardless of language, framework, or toolchain.

This contract is the **single source of truth** for git operations across the SDDK pipeline. The orchestrator prompt references this file but does not duplicate its rules.

## Lifecycle Overview

```
PRE-FLIGHT: trunk sync + consolidation check
    ↓
PLAN: explore → propose → spec || design → tasks → branch
    ↓
BUILD: apply → verify → debt-verify (pre-PR) → archive
    ↓
CONSOLIDATE: push → PR → wait approval → merge → tag → HTML report → close issue → roadmap
    ↓
RESET: trunk sync + cycle marker
```

## Quick Path

1. **Trunk base always.** Every cycle starts on a fresh `main` and ends by syncing back to it.
2. **Branch per change.** Every SDDK change lives on its own feature branch. No two changes share a branch.
3. **Conventional commits only.** `<type>(<scope>): <description>` enforced by `git-boundary` plugin lint.
4. **One commit = one logical unit.** Atomic, never broken.
5. **PR is the gate to main.** Never commit directly to main. Always go through a PR.
6. **Merge commit (`--no-ff`).** Never fast-forward, never rebase onto main.
7. **Never delete branches.** Feature branches live forever as historical record.
8. **Semver tag at milestones.** Every completed cycle → `v<major>.<minor>.<patch>` tag pushed.
9. **HTML report at end.** Every cycle ends with a self-contained HTML closing report.
10. **No two cycles open.** Previous cycle must be 100% consolidated before starting a new one.
11. **ZERO knowledge documents in the project repo (v3.5).** ALL project knowledge — ROADMAP, ADRs, specs, requirements, cycle manifests, incidences, terms — lives in the **knowledge graph vault** at `~/.sddk-knowledge/{project}/`, NOT in the project repo. The project repo contains ONLY product code. The `docs/` directory is never created by SDDK agents. Working artifacts (`sddk/`, `openspec/changes/`, `sddk-config.json`, `**/apply-checkpoint.json`) remain local-only (gitignored). See `skills/knowledge-graph/SKILL.md` for the vault protocol.

## Local-Only Artifact Policy (v3.5)

**v3.5 change: knowledge documents moved to the knowledge graph vault.** Previously, ROADMAP, ADRs, and specs lived in `docs/` inside the project repo (gitignored). As of v3.5, they live **outside** the project repo entirely, in `~/.sddk-knowledge/{project}/`. The project repo has ZERO documentation files — only product code.

**Two artifact categories:**

| Category | Where | Examples | In project repo? |
|----------|-------|----------|------------------|
| **Working state** (per-cycle, ephemeral) | `sddk/{change}/`, `openspec/changes/` | proposal, spec delta, design, tasks, verify-report, debt-report, apply-progress | Gitignored (local disk, readable by agents) |
| **Knowledge graph** (persistent, project-wide) | `~/.sddk-knowledge/{project}/` | milestones, ADRs, requirements, cycle manifests, incidences, terms | **NOT in repo at all** — separate vault |

**The project repo contains ONLY product code.** No `docs/ROADMAP.md`, no `docs/adr/`, no `openspec/specs/`. All of that is in the vault.

**Git half:** the project's `.gitignore` lists SDDK working paths (`sddk/`, `openspec/changes/`) so `git status` / `git add` skip them. The `docs/` directory is never created.

**Local-readability half:** SDDK working artifacts (`sddk/`, `openspec/changes/`) need a `.ignore` override for ripgrep visibility (same as v3.3). The knowledge vault at `~/.sddk-knowledge/{project}/` is fully readable by all tools (it's outside the repo, so `.gitignore` doesn't apply).

See `skills/knowledge-graph/SKILL.md` for the vault protocol (how to read, write, and query nodes).

- They are large, generated, and ephemeral-per-cycle.
- They duplicate information already mirrored in Engram / Logseq (the durable cross-machine record).
- They would bloat git history with non-deterministic local snapshots.

**The contract has two halves and BOTH must hold:**

1. **Git half:** the project's `.gitignore` lists every SDDK-generated path so `git status` / `git add` skip them. Templates are at `~/.config/opencode/prompts/sdd-kernel/templates/sddk.gitignore.template`. `sdd-kernel-init` plants these once per project at Step 5 of init.

2. **Local-readability half:** opencode's `grep` / `glob` / `Read` tools use ripgrep under the hood, which respects `.gitignore` by default. Without an override, agents could not read `sddk/{change}/verify-report.md`, `docs/ROADMAP.md`, or any other SDDK artifact — the very files each phase needs. The project's `.ignore` (a separate ripgrep input file) re-includes those paths with `!`-prefixed patterns. Templates are at `~/.config/opencode/prompts/sdd-kernel/templates/sddk.dotignore.template`. Same `sdd-kernel-init` step plants it.

**What this is NOT:**

- It is **not** "stop using these files." They are the SDDK's primary work surface. Phase agents read and write them constantly.
- It is **not** "hide them from opencode." The `.ignore` file actively makes them readable.
- It is **not** "delete them at the end of a cycle." The audit trail is `sddk/{change}/archive-report.md` plus the durable Engram observation IDs — both must persist.

**Verification at init (must appear in init envelope):**
```
git check-ignore -v sddk/ openspec/changes/ docs/ROADMAP.md   # exit 0, paths ignored
rg --files --hidden sddk/                                       # paths listed
```

**Failure modes:**
- Init cannot write `.gitignore` (read-only filesystem, non-git project) → log `sddk-local-only-policy-degraded` and proceed with Engram-only persistence.
- Init cannot write `.ignore` (read-only filesystem) → log `sddk-local-only-policy-local-read-degraded` and warn that phase agents will not find SDDK artifacts via `grep`/`glob` (they must use `Read` directly on known paths).
- Drift later: a user manually edits `.gitignore` and removes the SDDK section → next `sdd-kernel-explore` re-emits a warning via `git check-ignore`.

**Interactions with git operations in this contract:**
- Phase 3.8 ("Update Roadmap") v3.3+ does **NOT** `git add` / `git commit` / `git push` the ROADMAP. It writes `docs/ROADMAP.md` locally and persists its full rendered content to Engram under topic `sddk/{change}/roadmap`.
- Tags (`v<major>.<minor>.<patch>`) and merge commits ARE pushed — those are product milestones, not working artifacts. Quick-Path rules 7–9 are unaffected.
- HTML closing reports under `docs/reports/` are gitignored; the orchestrator optionally mirrors them to `/tmp/` for one-off sharing. They never enter git.

**Reference:** opencode docs on ignore patterns — `https://github.com/anomalyco/opencode/blob/dev/packages/web/src/content/docs/tools.mdx#internals--ignore-patterns`.

## Git-SDDK Phase Interleaving (Detailed)

```
sddk-init
    ↓
sddk-explore → explore-report.md
    ↓
sddk-propose → proposal.md
    ↓ (coherence check)
[sddk-spec || sddk-design] (PARALLEL) → spec.md, design.md
    ↓ (coherence check)
sddk-tasks → tasks.md
    ↓
git checkout -b <type>/<description>
git push -u origin <type>/<description>
    ↓
sddk-apply → atomic conventional commits on branch
    ↓ (coherence check)
sddk-verify → verify-report.md
    ↓ (correction cycle on FAIL)
sddk-debt-verify → debt-report.md (NEW v3.1 — on feature branch, pre-PR)
    ↓ (debt-fix cycle on FAIL — max 3 rounds on `refactor/debt-<change>-<round>`)
sddk-archive → archive-report.md
    ↓
git push origin <type>/<description>
    ↓
gh pr create --base main --head <type>/<description> \
  --title "<type>(<scope>): <description>" \
  --body-file <generated from artifacts>
    ↓
[wait for review approval]
    ↓ (after merge)
git checkout main && git pull origin main
    ↓
git tag -a v<major>.<minor>.<patch> -m "<type>: milestone — <description>"
git push origin v<major>.<minor>.<patch>
    ↓
[generate HTML closing report]
    ↓
[close tracking issue if any]
    ↓
[update ROADMAP]
    ↓
[commit cycle marker to main]
    ↓
NEXT CYCLE (must repeat Phase 0 pre-flight)
```

## Invariant Rules

### Rule 0 — Trunk is the only source of truth (HARD GATE)

`main` is the trunk. All other branches are ephemeral work areas. **No commit may reach `main` except via an approved PR that has passed `sddk-verify` (PASS or PW). When the user opts into `sddk-debt-verify`, it must also have passed (PASS or PW) before PR creation.**

**Pre-flight check (MCW Step 0.1)** — HARD GATE:
```bash
git fetch origin main
git checkout main
[ "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)" ] || BLOCK
```

**Post-flight check (MCW Step 4.1)** — HARD GATE: same check, must pass before next cycle.

**Trunk sync is enforced by git-boundary plugin** on every `git push` attempt that targets `main` (blocked) and every `git checkout main && git pull` (allowed, but result must match `origin/main`).

If the cycle ended with commits not on `main` (e.g., on a feature branch or fix branch), the cycle is **NOT closed**. Step 0.2 gate will BLOCK the next cycle.

**Debt-verify gate is conditional on user opt-in** — see "Pre-PR Debt Gate (NEW v3.1, OPTIONAL)" section below.

### Rule 1 — Branch per SDDK change

Every SDDK change lives in its own feature branch. Two distinct SDDK changes never share a branch.

**Branch naming**: `<type>/<description>`

| Type | When |
|------|------|
| `feat` | New user-visible or API functionality |
| `fix` | Bug fix |
| `docs` | Documentation changes |
| `chore` | Maintenance, dependencies, tooling, configuration |
| `refactor` | Code change without behavior change (also used for `refactor/debt-<change>-<round>` debt-fix cycles launched by `sddk-debt-verify` failures) |
| `perf` | Performance improvement |
| `test` | Tests only |
| `ci` | CI/CD changes |
| `revert` | Reversion of a previous commit |

Description: kebab-case, max 72 chars, descriptive.

**Validation**: enforced by `git-boundary` plugin on every `git checkout -b <name>`. Invalid names block the call.

### Rule 2 — Branch creation and push

- **When**: Immediately after `sddk-tasks` completes and before `sddk-apply` starts.
- **Push**: Push to remote immediately after creation. Code must never live uncommitted between SDDK iterations.

### Rule 3 — Conventional Commits

**Format**:
```
<type>(<scope>): <short description>

[optional body]

[optional footer with references]
```

**Examples**:
```
feat(auth): add OAuth2 login flow
fix(api): handle null user in session lookup
docs(readme): update install instructions
chore(deps): bump axios to 1.6.0
```

**Validation**: enforced by `git-boundary` plugin on every `git commit -m "<msg>"`. Bad messages block the call.

### Rule 4 — Atomic commits

One commit = one logical unit of work. Do not bundle unrelated changes.

Every commit must:
- Build cleanly
- Pass all tests
- Be self-contained
- Reference an issue if applicable (via `Closes #N` in the footer)

Never commit broken code. If a partial change is unavoidable, use `chore(WIP):` commits sparingly.

### Rule 5 — PR is the only path to main

Main is protected:
- **No direct commits to main.** Always go through a PR.
- **No force pushes to main.** Ever.
- **No merge commits bypassing review.** PR must be approved before merge.
- **No fast-forward from feature branch.** Always use merge commit (`--no-ff`).

PR creation is part of the orchestrator workflow (see MCW Step 3.2 in orchestrator.md).

### Rule 6 — Merge via approved PR

```
gh pr create --base main --head <branch> --title "..." --body "..."
# wait for CI + review approval
gh pr merge <pr-number> --squash  # or --merge for --no-ff
```

The orchestrator must wait for the PR to reach `MERGED` state before proceeding to tag.

### Rule 7 — Never delete branches

Feature branches live in the remote as permanent historical record. GitHub preserves the PR↔branch link even after merge.

Do NOT run `git branch -d` or `git push origin --delete` on a feature branch.

### Rule 8 — Trunk sync at start AND end of every cycle

**At start (Phase 0 Step 0.1)**:
```
git checkout main && git pull origin main
```

**At end (Phase 4 Step 4.1)**:
```
git checkout main && git pull origin main
```

Never start a new change from a stale main. Never leave the workspace on a feature branch when the cycle is done.

### Rule 9 — Close before open (no overlapping cycles)

Do not start a new SDDK change until the previous one is:
- Merged to main
- Tagged with semver
- Reported in HTML
- Marked with semver tag pointing to a main commit (the tag itself is the cycle marker)

If blocked, document the blocker in the project roadmap and resolve before starting the new cycle.

### Rule 10 — Semantic versioning at milestones

After each completed cycle (PR merged to main), create an annotated tag:

```
git tag -a v<major>.<minor>.<patch> \
  -m "<type>: milestone — <description>

Release: <change-name>
- <feature bullet>
- <fix bullet>

See sddk/<change>/archive-report.md for details."
git push origin v<major>.<minor>.<patch>
```

**Version bump rules**:

| Bump | When | Example |
|------|------|---------|
| `major` | Breaking change to public API or contract | `v1.0.0 → v2.0.0` |
| `minor` | New feature, non-breaking | `v1.0.0 → v1.1.0` |
| `patch` | Bug fix, chore, docs, refactor | `v1.0.0 → v1.0.1` |

First release of a new project starts at `v0.1.0`. Pre-1.0 versions follow the same rules (0.1.0 → 0.2.0 → 1.0.0).

### Rule 11 — HTML closing report at end of every cycle

After tag push, generate the HTML closing report (delegated to `sdd-kernel-archive` agent using `prompts/sdd-kernel/HTML-REPORT.md`):

```
xdg-open <report-path>
```

Path routing:
- `engram` / `none` → `/tmp/sddk-<change>-<YYYYMMDD>.html`
- `openspec` → `openspec/changes/<change>/reports/cierre.html` + `/tmp/` copy

The orchestrator must verify the HTML file exists and is non-empty.

### Rule 12 — Issue closure

If the cycle was tracked by a GitHub issue:

```
gh issue close <issue-number> --comment "Completed in PR #<pr-number>. Released as v<version>."
```

### Rule 13 — Large PRs (>400 LOC changed)

Load `skill(name="chained-pr")` to split into reviewable stacked PRs. Each slice must be independently buildable and testable.

This rule is enforced by the Review Budget Guard (MCW Step 1.7) before launching apply.

### Rule 14 — Cycle completion: tag + F3 metrics

The semver tag (Rule 10) IS the cycle marker. No separate marker file.

Additionally, MCW Step 4.2 writes cycle metrics to:
- `~/.local/share/opencode/sddk/metrics/{cycle_id}.jsonl` (local, machine-readable)
- Engram observation `topic_key: cycle-metrics/{cycle_id}` (durable, cross-session)
- `metrics/aggregate` rolling 7d/30d (F3 tuner consumes this)

If cycle had `verify_verdict=PASS` + `first_pass_success=true` + reusable decision, also save jurisprudence observation (topic_key: `jurisprudence/{category}`). See `prompts/sdd-kernel/decision-model.md` § Jurisprudence Schema.

The presence of a semver tag on a main commit = cycle closed. MCW Step 0.2 checks this to enforce "no overlapping cycles" via `git tag --points-at main`.

## Phase Agent Responsibilities

| Phase | Git responsibility |
|-------|--------------------|
| `sdd-kernel-tasks` | After producing tasks, the orchestrator creates the feature branch and pushes to remote before launching apply. |
| `sdd-kernel-apply` | Every completed task slice gets its own atomic conventional commit. Use the type table from Rule 3. Never commit broken code. |
| `sdd-kernel-verify` | Fix commits follow the same conventional format (`fix(<scope>): ...`). No skipping verify. |
| **`sddk-debt-verify`** (NEW v3.1) | **Read-only** on the feature branch. Runs BEFORE PR creation. Emits `debt-report.md` and verdict. On FAIL, launches a fix cycle on `refactor/debt-<change>-<round>` (max 3 rounds). Cluster agents never commit; only the fix-cycle apply phase produces commits. |
| `sdd-kernel-archive` | After archiving, the orchestrator: pushes branch, creates PR, waits for merge, creates tag, generates HTML report, closes issue, updates roadmap. |

## Debt-Fix Cycle Branch Naming (NEW v3.1)

When `sddk-debt-verify` returns FAIL with `re_iterate_from: apply`, the orchestrator creates a new branch:

```
git checkout -b refactor/debt-<change-name>-<round>
git push -u origin refactor/debt-<change-name>-<round>
```

Where `<round>` starts at `1` and increments on each subsequent fix attempt. Max 3 rounds.

The fix cycle is a complete SDDK cycle but path is **forced to A-min** (`spec → apply → verify → debt-verify → archive`). The fix branch merges back into the original feature branch (NOT directly to main) via a PR after the fix cycle's archive completes. The original feature branch then re-enters debt-verify with `debt_fix_round` incremented.

After round 3 fails, escalate to user with full debt report. Do NOT auto-merge.

## Pre-PR Debt Gate (NEW v3.1, OPTIONAL)

The orchestrator asks the user after `sddk-verify` PASS/PW whether to run `sddk-debt-verify` before archive.

**If the user opted into debt-verify:**
- `gh pr create` MUST be blocked by `git-boundary` plugin if any of the following are true:
  - `verify-report.md` does not exist or verdict is FAIL.
  - `debt-report.md` does not exist.
  - `debt-report.md` verdict is FAIL.
- The orchestrator verifies both reports exist with PASS/PW before invoking `gh pr create` (MCW Step 3.2).

**If the user skipped debt-verify:**
- No debt-report required.
- `gh pr create` proceeds with only `verify-report.md` PASS/PW gate.
- No PR body debt section.
- This is the default behavior for B-direct and is acceptable for A-min.

The orchestrator caches the user's opt-in choice in the session and propagates it to the Launch Plan as `debt_user_opted_in: bool` and `debt_depth: skip|smoke|standard|deep`.

## Enforcement

| Rule | Mechanism |
|------|-----------|
| Branch name format | `git-boundary` plugin blocks `git checkout -b <invalid>` |
| Commit message format | `git-boundary` plugin blocks `git commit -m <bad>` |
| Force-push | `git-boundary` plugin blocks `git push --force` |
| Rebase | `git-boundary` plugin blocks `git rebase` |
| Stash | `git-boundary` plugin blocks `git stash` |
| Reset | `git-boundary` plugin blocks `git reset` |
| Phase agent git writes | `git-boundary` plugin blocks phase agents |
| Commit message secrets | `pre-commit-hooks` plugin blocks commits containing API keys |
| File size > 500KB | `pre-commit-hooks` plugin blocks commit |
| Cycle consolidation | Orchestrator prompt Step 0.2 hard gate |
| Trunk sync | Orchestrator prompt Step 0.1 + Step 4.1 hard gates |
| Verify before PR | `verify-report.md` must exist with PASS/PW (always) |
| Debt-verify before PR (NEW v3.1, OPTIONAL) | `debt-report.md` must exist with PASS/PW ONLY when user opted into debt-verify |
| PR approval | Orchestrator prompt Step 3.3 hard gate |
| Tag push | Orchestrator prompt Step 3.5 hard gate |
| HTML report | Orchestrator prompt Step 3.6 hard gate |

## Anti-Patterns (FORBIDDEN)

| Anti-pattern | Consequence | Enforcement |
|---|---|---|
| Committing directly to main | Bypasses review | git-boundary + branch protection rules |
| Force-pushing to main | Destroys history | git-boundary + GitHub branch protection |
| Rebasing feature branches onto main | Loses review history | git-boundary |
| Deleting branches after merge | Loses PR↔branch link | Manual flag in runbook |
| Starting new cycle without merging previous | Two cycles open, conflicts | MCW Step 0.2 |
| Skipping PR review | Unreviewed merge | MCW Step 3.3 |
| Merging without PR | Bypasses review | MCW Step 3.4 verifies commit came via PR |
| Skipping semver tag | Lost milestone marker | MCW Step 3.5 |
| Skipping HTML report | No audit trail | MCW Step 3.6 |
| Skipping trunk sync | Stale main | MCW Step 0.1 + Step 4.1 |
| Forcing debt-verify when user opts out (NEW v3.1) | Violates opt-in contract | Orchestrator must respect user choice |
| Skipping debt-verify when user opted in (NEW v3.1) | CRITICAL debt reaches main | Pre-PR gate + MCW Step 2.4 |
| Mixing multiple SDDK changes in one branch | Confused scope | Branch naming enforcement |
| Large monolithic commits | Unreviewable | MCW Step 1.7 chained-pr |

## Recovery

If the cycle is interrupted mid-flight, the orchestrator should be able to resume:

1. Check `git tag --points-at main` → if last semver tag exists, cycle was complete; new cycle can start
2. Check the artifact registry for which phase artifact is the latest
3. Resume from the next pending phase
4. If apply was interrupted, use the checkpoint file (`sddk/<change>/apply-checkpoint.json`)
5. If PR was created but not merged, check PR state and resume from there
6. If tag was not pushed, push it after merge
7. If HTML report was not generated, regenerate

The orchestrator prompt's "Dependency Graph" section describes how to resume from any phase.