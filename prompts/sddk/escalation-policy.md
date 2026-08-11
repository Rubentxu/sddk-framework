# Escalation Policy

Use `grill-with-docs`, `auto-grill`, or `auto-grill-loop` only when launch plan justifies it.

Escalate for:
- Ambiguous domain language
- Code/docs/user claim contradiction
- Hard-to-reverse decision with real trade-off
- Critical connascence or poor design-quality score
- Context stuck at C0/C1

## Specialized Agent Delegation (within SDDK cycle)

| Agent | Trigger | Purpose |
|-------|---------|---------|
| `auto-grill-loop-orchestrator` | Proposal/design needs validation | Multi-pass adversarial |
| `jd-judge-a` + `jd-judge-b` | Pre-merge blind review (judgment-day) | Dual adversarial review |
| `sddk-coherence` | Between phase transitions (A-full/A-lite only) | Coherence score |
| **`impeccable-primary`** | Frontend design request (any UI/UX/craft work) | Primary design agent — declares register, routes to 23 impeccable commands, integrates with SDDK via Path D |
| **`sddk-debt-verify`** | After sddk-verify PASS/PW (MCW Step 2.4) — **MANDATORY on A-*; not invoked on B-direct** | Post-verify debt audit phase orchestrator |
| **`debt-architecture-cluster`** | Debt-verify phase on **A-full** (depth=deep) | Connascence, DQS, SOLID entropy, depth/seam/leverage, Matsumoto + Khononov critiques (5 skills) |
| **`debt-smells-cluster`** | Debt-verify phase on **A-lite / A-full** (depth=standard/deep) | Fowler smells, SOLID mapping, refactor backlog (6 skills) |
| **`debt-duplication-cluster`** | Debt-verify phase on **A-lite / A-full** (depth=standard/deep) | Structural/logical/semantic duplication + dead code (2 skills) |
| **`debt-coupling-cluster`** | Debt-verify phase on **A-min / A-lite / A-full** | Hidden dependencies, global state, brittle coupling (3 skills) |
| **`debt-overeng-cluster`** | Debt-verify phase on **A-min / A-lite / A-full** | Over-engineering audit + ponytail: comment debt ledger (2 skills) |

## Debt-Verify Policy (v3.6 + reversibility depth adjustment)

Once `sddk-verify` returns `PASS` or `PASS_WITH_WARNINGS`, `sddk-debt-verify` runs with depth **derived from path**, not selected by the user. There is no prompt for "debt-verify or direct to archive".

The `reversibility` axis assessed at triage step 4.5 may adjust depth but never remove the mandatory A-* gate:

| Reversibility | Override effect |
|---|---|
| **HIGH** | Use `smoke` as the minimum mandatory depth on A-* paths. |
| **MEDIUM** | No override — use path-derived depth. |
| **LOW** | Force depth=`deep` (5 clusters) + invoke `jd-judge-a` + `jd-judge-b` (judgment-day) as additional verify lenses, even if path is A-min. Irreversible changes get maximum scrutiny. |

**Base depth by path (when reversibility = MEDIUM or unset):**

| Path | Depth | Clusters (all run in parallel) |
|------|-------|-------------------------------|
| A-full | `deep` | architecture + smells + duplication + coupling + overeng (5) |
| A-lite | `standard` | smells + duplication + coupling + overeng (4) |
| A-min | `smoke` | coupling + overeng (2) |
| B-direct | n/a — debt-verify is NOT invoked (hotfix) | 0 |

**No mid-cycle prompt about debt-verify.** The reversibility override is decided at triage, not mid-cycle.

## SDDK Artifacts Live in User Space (ADR-0011)

SDDK **never writes inside a project repo** (zero intrusion, ADR-0011). All working paths created during a cycle live in XDG user directories:

- Cycle artifacts: `$SDDK_DATA_DIR/projects/<project_id>/cycle-artifacts/{cycle_id}/`
- Generated docs: `$SDDK_DATA_DIR/projects/<project_id>/generated/`
- Knowledge vault: `<vault>` from `sddk knowledge path`

**Rules:**
- Never commit a repo-local SDDK working path or copy vault knowledge into `docs/`.
- Never create `.gitignore`, `.ignore`, `.atl/`, `sddk/`, or checkpoint files inside a project repo to hold SDDK state (ADR-0011).
- Never refuse to read an SDDK path because it lives outside the repo.
- `sddk-init` never plants ignore files or repo-local state. Persistence is Engram-memory + XDG + vault only.

## Release → Archive: Release Is Mandatory Before Archive (v3.7, no opt-out)

The workflow enforces `release.complete` BEFORE `archive.complete`. After verify (or, on A-full, after review) the cycle transitions to `RELEASE_PENDING`/`release` and the orchestrator **MUST** invoke `sddk-release` on the next tick — no opt-in, no user prompt, no skip.

**Mandatory transition sequence:**
```
verify (or review) → RELEASE_PENDING/release
    ↓  (next tick, no questions, no opt-in)
sddk-release(route=local)        → produces merge-receipt + release-receipt
    ↓  (only on status=success)
sddk-archive                     → produces archive-manifest linked to release-receipt
    ↓
trunk-sync-end (Phase 4.1)
```

**Recovery on blocker:** if `sddk-release` returns `status=blocked`, the orchestrator surfaces the blockers[] and instructs the user to re-run `/sddk-release <change>` (idempotent resume).

**Skill gate:** when `sddk-release/SKILL.md` is loaded, it is **delegate-only** — re-delegate to the executor agent.

## Auto Mode = Complete Cycle, No Mid-Cycle Pauses (v3.3)

When launched with `mode=auto` (the default), the orchestrator must run **the entire MCW from explore (Phase 1) through trunk-sync-end (Phase 4.1)** without pausing.

Forbidden mid-cycle pauses:
- "Do you want me to run debt-verify?" — debt-verify is mandatory
- "Do you want me to archive?" — archive is mandatory after verify PASS
- "Do you want me to release?" — release is mandatory before archive
- "Should I continue?" — never asked in auto mode

The **only** legitimate mid-cycle user interaction is `escalation_needed=true` from a phase agent.

## Interactive Mode — Phase Checkpoints, NOT Between Archive and Release (v3.4)

In interactive mode, the orchestrator pauses after each phase to ask "¿Continuamos?" — **except** between `release` and `archive`. Those two phases are **fused into an atomic unit**.

| After phase | Interactive pause? | Rationale |
|---|---|---|
| explore, propose, spec, design, tasks | YES | Planning is reversible |
| apply (Phase 2.1 start) | **YES — last checkpoint** | Once commits exist, cycle must close |
| verify, debt-verify | NO | Automatic gate |
| release | **NO — atomic handoff to archive** | Local effects are settled |
| trunk-sync-end | NO | Final gate, automatic |

## Skill Loading (within SDDK cycle)

| Skill | Trigger | Purpose |
|-------|---------|---------|
| `grill-with-docs` | Ambiguous language, glossary conflicts | Resolve terminology |
| `improve-codebase-architecture` | Post-implementation, debt signals | Refactor |
| `design-an-interface` | New API design | Explore interface options |
| `chained-pr` | PR > 400 LOC | Split into reviewable PRs |
| `work-unit-commits` | Preparing large PRs | Plan commits as review units |
| `branch-pr` | Creating PRs | Issue-first PR creation |
| `issue-creation` | Creating issues | Validation before creation |
| `judgment-day` | Pre-merge blind review | Dual judges |
| `test-pyramid` | Test strategy | Coverage + integration + E2E |
| `cognicode-sdd` | CogniCode MCP available + architectural work | Code intelligence |
| `chronos-sdd` | Chronos MCP available + runtime bug | Time-travel debugging |
| **`impeccable`** | Always available (installed). Use when request is design/UI craft. Routes 23 commands: craft, audit, critique, polish, bolder, quieter, distill, harden, animate, colorize, typeset, layout, delight, overdrive, clarify, adapt, optimize, live, extract, document, init, shape, onboard. | Frontend design craft + 46-rule anti-pattern detector |
| `entropy-sdd` | Quality lens needed | Connascence + SOLID entropy |

Do not pull in agents/skills because they exist. Use only when launch plan signals trigger.
