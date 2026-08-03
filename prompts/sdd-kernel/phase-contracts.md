# SDD Kernel Phase Contracts

These contracts describe the `sdd-kernel-*` phase agents. They are not used by the traditional `sdd-*` agents.

## Router Context Contract

Every phase consumes the `SDD Kernel Launch Plan` from the orchestrator. Do not rediscover this context unless a required field is missing or contradicted by code/docs.

Every phase also follows `prompts/sdd-kernel/decision-model.md` (Knowledge Layers, Source Hierarchy, Knowledge States, Jurisprudence sections) for retrieval order, authority, knowledge states, promotion, and verify feedback.
Git operations follow `prompts/sdd-kernel/git-contract.md` — phases are interleaved with git, not separate from it.

Required router fields:
- Knowledge Coverage: roadmap/work items/architecture/ownership/learnings presence.
- Context Quality: C0/C1/C2/C3.
- Problem Taxonomy: dominant axes and evidence.
- Domain Language: resolved terms and unresolved ambiguities.
- Invariants: known rules or explicit unknowns.
- Recommended Effort: skip, verify, deepen, or recommend-lenses.
- Git Checkpoints: branch status, push status, merge target, semver tag plan.

Required knowledge behavior:
- Preserve provenance for important claims.
- Record knowledge gaps as first-class artifacts when they block progress.
- Keep workflow state, durable knowledge, and Engram memory separate.

### Git Phase Interleaving

The orchestrator owns git operations, but phases must respect the interleaving:

| Phase | Git State | Phase Responsibility |
|-------|-----------|---------------------|
| `sddk-tasks` | Branch NOT yet created | Produce tasks. Orchestrator creates branch after this phase. |
| `sddk-apply` | Branch exists, pushed to remote | Produce atomic conventional commits per task slice. Never commit broken code. |
| `sddk-verify` | Commits exist on branch | Fix commits follow conventional format. |
| **`sddk-debt-verify`** (v3.3 — MANDATORY on A-*, n/a on B-direct) | Commits exist on feature branch, pre-PR — runs unconditionally after verify PASS/PW on A-* paths | **Read-only audit.** Launches cluster orchestrators in parallel with depth derived from path. Emits `debt-report.md` and verdict. On FAIL, launches fix cycle on `refactor/debt-<change>-<round>` (max 3 rounds). Never commits; never pushes. |
| `sddk-archive` | All commits pushed + debt-report PASS/PW (mandatory on A-*) | Orchestrator hands off to `sdd-kernel-release` (Phase 3) which owns PR + merge + tag + html + roadmap (local-only) + trunk-sync. **No commits to gitignored paths** (Local-Only Artifact Policy v3.3). |
| **`sdd-kernel-release`** (NEW v3.3 — MANDATORY post-archive) | All commits pushed + archive-report success | Single owner of Phase 3 end-to-end. See `prompts/sdd-kernel/phases/release.md` and `skills/sddk-release/SKILL.md`. ROADMAP update is local-only + Engram; no `git add docs/`. |

Phases must NOT perform git operations directly. The orchestrator owns branch creation. From Phase 3 onward, `sdd-kernel-release` owns pushing, PR creation, merging, tagging, and HTML report (writing to `/tmp/` and gitignored `docs/reports/`), and ROADMAP update (local-only — see git-contract.md § Local-Only Artifact Policy v3.3).

The ROADMAP, ADRs, archive folders, and HTML reports are **gitignored** and **locally readable** (paired `.ignore` overrides). They are committed NEVER.

The `sddk-debt-verify` phase agent (mandatory on A-*) is the only phase that runs BEFORE PR creation. It runs on the same feature branch as `sddk-apply` and `sddk-verify`. The orchestrator verifies the debt-report verdict is PASS or PW before handing off to `sdd-kernel-release`.

## Explore

Explore produces evidence for routing and proposal work.

Required additions beyond traditional explore:
- Knowledge Coverage: present/missing/stale classes and why they matter.
- Context Quality: C0/C1/C2/C3.
- Problem Taxonomy: dominant axes and evidence.
- Domain Language: resolved terms and unresolved ambiguities.
- Invariants: known rules or explicit unknowns.
- Recommended Effort: skip, verify, deepen, or recommend-lenses.
- Knowledge Gaps: explicit gaps that should be persisted for later phases.

## Propose

Propose converts evidence into WHAT and WHY without guessing.

Required additions beyond traditional propose:
- Knowledge Alignment: which durable artifacts define scope, ownership, and acceptance.
- Context Gate: quality, taxonomy, and effort decision.
- Invariants: rules that must survive the change, with verification target.
- Capabilities named in domain language.
- Recommended lenses only when context quality and risk justify them.
- Promotion Notes: what stays memory-only vs what should become durable knowledge.

If C0/C1 gaps affect scope, ownership, or capabilities, propose returns partial/blocked.

## Design

Design converts proposal/specs into HOW without repeating exploration.

Required additions beyond traditional design:
- Knowledge Reuse Check: roadmap/work item/ADR/ownership/learnings reused vs missing.
- Context Reuse Check: artifacts reused, gaps, code verification, quality level.
- Applied Lenses: only lenses that affected the design.
- Invariants and Constraints: enforcement point and verification.
- Entropy envelope: interface/coupling risk at the depth selected by the kernel.
- Knowledge Impact: what design choices may supersede or stale earlier knowledge.

If C0/C1 gaps affect boundaries, invariants, or contracts, design returns partial/blocked.

## Verify

Verify remains multi-lens in the kernel flow.

Required additions beyond traditional verify:
- Knowledge traceability lens.
- Architecture/connascence lens.
- Test quality lens.
- Design coherence lens.
- Two adversarial judge lenses when risk is high enough.
- Synthesis agent merges findings into one verdict.
- Knowledge Impact: confirmed claims, contradicted claims, stale artifacts, promotion candidates.

The kernel decides lens count based on context quality and risk; it must not spawn lenses just because they exist.

## Apply

Apply implements approved kernel tasks safely, preserving progress and verifying each slice.

Required additions beyond traditional apply:
- Follow `prompts/sdd-kernel/git-contract.md` for commit format and atomicity.
- One atomic conventional commit per completed task slice.
- Report git checkpoints in apply-progress: branch, push status, merge target.
- Never commit broken code. Every commit must build and pass tests.
- If unexpected blast radius appears, stop and report partial.

## Archive

Archive closes a completed kernel change. Syncs delta specs and updates durable knowledge.

Required additions beyond traditional archive:
- Verify all commits are pushed to the remote feature branch.
- Confirm merge target is main via merge commit (--no-ff).
- **If user opted into debt-verify (NEW v3.1)**: confirm `debt-report.md` exists with verdict PASS or PW. Block archive if debt-report is missing or FAIL.
- **If user skipped debt-verify (NEW v3.1)**: proceed normally without debt-report.
- If debt-report exists, embed its summary in PR body so debt travels with the merge.
- Orchestrator creates semver tag after this phase completes.
- Never delete the feature branch after merge.
- Generate self-contained HTML closing report using `prompts/sdd-kernel/HTML-REPORT.md`.
- Open the HTML report in the browser automatically.
- Report path: `/tmp/sddk-{change-name}-{YYYYMMDD}.html` or `openspec/changes/{change-name}/reports/cierre.html`.

## Debt-Verify (v3.3 — MANDATORY on A-*, n/a on B-direct)

Debt-verify is a **mandatory** post-verify phase on A-* paths that runs on the feature branch BEFORE PR creation. It is the gate that prevents CRITICAL technical debt from reaching `main`. The user is NEVER asked and NEVER allowed to skip — the only legitimate way to avoid it is to triage into B-direct (hotfix).

Required additions beyond traditional verify:
- **Trigger**: unconditional after `sddk-verify` returns PASS or PW on A-full / A-lite / A-min.
- **Depth**: derived from path and locked. `smoke | standard | deep` is selected automatically: A-full=deep, A-lite=standard, A-min=smoke. The user is not prompted.
- **Read-only on the codebase.** Cluster agents audit and emit findings; never modify code, never commit.
- **Cluster fan-out**: A-full=5 clusters, A-lite=4 clusters, A-min=2 clusters, B-direct=0.
- **Trunk-based discipline**: runs on the feature branch, NOT on main. Branch must be pushed. Working tree clean.
- **Pre-existing main debt detection**: for each CRITICAL finding, `git blame` and flag if last touched on main BEFORE the feature branch was created.
- **Re-iteration decision**: emit `re_iterate_from: beginning | apply | none` per the Re-Iteration Decision Matrix.
- **Fix cycle discipline**: on FAIL with `re_iterate_from: apply`, launch a fix cycle on `refactor/debt-<change>-<round>` (max 3 rounds, path forced to A-min).
- Persist `debt-report` per artifact store mode.
- Return envelope with verdict, re_iterate_from, clusters_run, depth, findings_by_severity, pre_existing_main_debt, next_recommended.

When skipped, `sddk-archive` proceeds normally and no debt-report is required.
