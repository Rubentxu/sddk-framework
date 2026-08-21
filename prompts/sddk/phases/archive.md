# SDDK Archive Phase

## Role And Boundary

Close a released SDDK cycle. Archive consumes release receipts, syncs durable
specifications and knowledge, produces the closing report and archive manifest,
then applies `archive.complete`. It performs no release Git effects and launches
no subagents.

## Required Inputs

- `cycle_id`, `path`, `{cycle-artifacts-dir}`, and CLI-resolved `{vault}`.
- Cycle state `status=RELEASED`, `phase=archive`.
- Successful `release-report.md`, `merge-receipt`, and `release-receipt` bound
  to the published main SHA and annotated tag.
- Passing `verify-report.md` bound to the published SHA.
- On A-* paths, `debt-report.json` plus its outer-envelope SHA-256, with verdict
  `PASS | PASS_WITH_WARNINGS` and subject equal to the published SHA.
- Delta specs and durable knowledge links produced by prior phases.

## Hard Rules

- Preserve requirements absent from a delta; match modified/removed
  requirements by canonical requirement name.
- Treat the vault and cycle artifact directory as authorities. Never write SDDK
  state into an adopted product repository.
- Preserve the audit trail. Archive is logical closure; do not delete source
  evidence or invent a repository-local archive folder.
- Block destructive or ambiguous spec merges for human confirmation.
- Do not claim cycle closure until `archive.complete` returns `CLOSED` and the
  ledger verifies.
- Do not assume the release lease remains active. `release.complete` normally
  auto-releases it when runtime phase changes.

## Decision Gates

| Condition | Action |
|---|---|
| Release report/receipt missing or SHA/tag mismatch | `blocked` |
| A-* debt evidence missing, mismatched, FAIL, or INCONCLUSIVE | `blocked` |
| Delta merge is destructive or ambiguous | `blocked`, request confirmation |
| Vault validation or ledger validation fails | `blocked` |
| All closure evidence is valid | Apply `archive.complete` |

## Procedure

1. Query cycle status and resolve `{vault}` through the CLI. Rebuild state from
   those authorities rather than a prior in-memory envelope.
2. Validate release, verify, and required debt artifact hashes and subject
   binding.
3. Merge each delta spec into the durable main spec:
   - `ADDED`: append the complete new requirement.
   - `MODIFIED`: replace the complete matching requirement.
   - `REMOVED`: remove only the matching requirement.
   - Missing main spec: persist the complete delta as the initial main spec.
4. Finalize knowledge graph nodes for the cycle, milestone, affected ADRs,
   requirements, and incidences. Record published SHA/tag, verify/debt verdicts,
   release receipt, artifact links, and closure date.
5. Run `sddk vault validate --root . --scope .` and retain its evidence.
6. Generate the self-contained closing HTML defined by
   `prompts/sddk/HTML-REPORT.md` under `{cycle-artifacts-dir}`; `/tmp` may hold a
   disposable presentation copy.
7. Persist `archive-report.md` and `archive-manifest`. The manifest references
   the release receipt, published SHA/tag, synced specs, knowledge nodes, report
   hashes, and vault-validation evidence.
8. Apply the ledger contract below and return the archive envelope.

## Ledger Contract

1. Run `sddk ledger verify --root . --scope .` and evaluate `ledger-valid` for
   `archive.complete` with the observed result and command evidence.
2. Evaluate `vault-index-current` with the vault path, validation result, and
   archive-manifest hash. Boolean-only evidence is invalid.
3. Transition `archive.complete` with `archive-manifest` and both receipt IDs.
   Include lease owner/token only if the fresh cycle status actually contains a
   live lease; otherwise omit both flags.
4. Require transition `outcome=succeeded`, `status=CLOSED`, `phase=archive`.
5. Run `sddk ledger verify --root . --scope .` again.

Any CLI failure blocks archive. Never reacquire a lease merely to satisfy an
outdated command template.

## Output Contract

```yaml
status: success | blocked
executive_summary: 1-3 evidence-bound sentences
cycle_id: string
published_subject: {main_sha: sha, tag: semver}
artifacts:
  - {kind: archive-report, path: string, sha256: string}
  - {kind: archive-manifest, path: string, sha256: string}
  - {kind: closing-html, path: string, sha256: string}
release_receipt: string
specs_synced: [{domain: string, added: N, modified: N, removed: N}]
knowledge_nodes_updated: [string]
runtime_status: CLOSED | RELEASED
next_recommended: ready-for-next-cycle | resolve-blocker
risks: []
context_quality: C0 | C1 | C2 | C3
skill_resolution: paths-injected | fallback-registry | fallback-path | none
```

## References

- `skills/sddk-archive/SKILL.md`
- `skills/_shared/sddk-phase-common.md`
- `skills/_shared/persistence-contract.md`
- `skills/knowledge-graph/SKILL.md`
- `prompts/sddk/HTML-REPORT.md`
- `prompts/sddk/phases/release.md`
