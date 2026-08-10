---
name: sddk-release
description: "Trigger: sddk-release. Release an archived SDDK change through local Git: verify, push main, verify SHA, tag, and record local receipts. CI/CD distribution is optional post-tag work."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "1.0"
  delegate_only: true
  source_of_truth: prompts/sddk/git-contract.md
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `sddk-release`. Do NOT execute inline.

## Executor Override

If you ARE the `sddk-release` sub-agent, continue. Run the **SDDK Release Checklist** end-to-end. Do NOT delegate further. Do NOT loop back to other SDDK phases.

## Mandatory Post-Archive

`sddk-release` is **mandatory** after a successful `sddk-archive`. There is no opt-out. The release phase is what closes the loop back to `main` — without it, feature branches rot, semver tags are missed, and the ROADMAP drifts from reality.

`prompts/sddk/git-contract.md` is the **single source of truth** for git invariants. This skill references it; do not duplicate its rules.

## Local Release Contract

The mandatory authority is local Git:

```
local verify -> push main -> verify HEAD == origin/main -> annotated tag -> verify remote tag -> receipts
```

Never require a PR, `gh`, CI/CD check, GitHub Action, hosted asset, signature,
or external release to close an SDDK cycle. They are optional post-tag
distribution only.
CI/CD and optional post-tag distribution are explicitly excluded from the
`no-pending-effects` gate.

1. Confirm the archive report, local verification, UAT gate, clean worktree,
   and trunk checkout.
2. Fast-forward `main`, push it directly, and verify full `HEAD == origin/main`.
3. Create or verify an annotated semver tag that peels to that SHA; push it and
   verify the remote peeled SHA.
4. Store `merge-receipt` from the verified `git.push` postcondition and
   `release-receipt` from the verified `git.tag` postcondition.
5. Complete the HTML report, knowledge graph update, serialization lock release,
   and ledger verification.

Use `sddk release apply --route local --branch main --base main --cycle <cycle-id>
--tag <tag> --title <message> --approve` when the typed CLI is available. The
`--cycle <cycle-id>` argument is **mandatory** for the local route: the CLI
links the release to the release-pending cycle, verifies the manifest commit
is an ancestor of HEAD, and requires a clean trunk checkout. A retry is safe:
an existing remote tag succeeds only if it is annotated and points to `HEAD`.

`--route forge --repo owner/repo` is optional integration after local success.
It does not read provider checks and its failure cannot block the cycle.

If `release-lock` fails, BLOCK and retain the lock. Never report success while
the local release bookkeeping remains incomplete.

## Result Contract

```yaml
status: success | blocked
route: local
change: <name>
main_sha: <full-sha>
tag: v<major>.<minor>.<patch>
merge_receipt: <path-or-receipt-id>
release_receipt: <path-or-receipt-id>
archive_manifest: <path-or-receipt-id>   # produced by sddk-archive, references release_receipt
knowledge_graph_updated: bool
lock_released: bool
optional_distribution: not_requested | pending | completed | failed
blockers: []
```

The `release-report` is mandatory even on block. The `archive-manifest` MUST
reference the `release-receipt` so that the cycle closure is traceable back to
the verified trunk SHA + tag.

## CLI Contract (sddk ledger)

When the project is adopted (`sddk cycle status --root . --scope .` exits 0), record the release in the cycle ledger BEFORE returning:

1. Evaluate `release-receipt` with the annotated tag and SHA evidence, and
   `no-pending-effects` with evidence that required local Git effects settled.
   Do not include CI/CD or optional distribution in that evidence.
2. Transition with both local artifacts:
    `sddk cycle transition --root . --scope . --cycle {cycle_id} --transition release.complete --artifact merge-receipt={main-sha-receipt} --artifact release-receipt={tag-receipt} --gate-receipt {receipt_id_1} --gate-receipt {receipt_id_2} --lease-owner {lease_owner} --fencing-token {fencing_token}`
3. Close the loop with telemetry: `sddk metrics record --root . --scope . --cycle {cycle_id} --verdict {PASS|PW|FAIL}`
4. Verify ledger integrity: `sddk ledger verify --root . --scope .`

A failed evaluate-gate or transition is a BLOCKER: report it in the envelope and do not proceed. `{cycle_id}`, `{lease_owner}`, `{fencing_token}` come from the orchestrator launch prompt (the cycle is opened with `sddk cycle start`). Full protocol: `skills/_shared/persistence-contract.md` → CLI Ledger Channel.

## References

- `prompts/sddk/git-contract.md` — git invariants (source of truth)
- `prompts/sddk/HTML-REPORT.md` — HTML report format
- `prompts/sddk/roadmap-template.md` — ROADMAP update format
- `skills/sddk-archive/SKILL.md` — successor, closes the cycle via archive-manifest linked to release-receipt
- `prompts/sddk/phases/release.md` — full agent prompt
