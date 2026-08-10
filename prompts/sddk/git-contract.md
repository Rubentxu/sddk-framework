# SDDK Git Contract v3

This contract defines Git authority for the SDDK lifecycle. It is the source
of truth for release operations across prompts, agents, skills, and the CLI.

## Lifecycle Overview

```
PRE-FLIGHT: verify local trunk and prior cycle state
    ↓
PLAN: explore -> propose -> spec/design -> tasks
    ↓
BUILD: apply -> verify -> debt-verify -> archive
    ↓
CONSOLIDATE: local verify -> push main -> verify SHA -> annotated tag -> receipts
    ↓
RESET: HTML, knowledge graph, lock release, trunk sync
```

## Release Authority

The local Git route is authoritative. A release succeeds only when:

1. Required local verification and configured human gates passed.
2. The full local `HEAD` SHA is the full SHA on `origin/main` after the direct
   push.
3. Exactly one selected annotated semver tag peels to that same SHA on the
   remote.
4. Local `merge-receipt` and `release-receipt` preserve those postconditions.

SDDK never depends on a pull request, forge, CI/CD system, required check,
GitHub Action, hosted release, asset upload, signature, or distribution job to
reach release success. A forge integration is optional post-tag work. Its
state must not gate, delay, or reopen an SDDK cycle.

## Local Release Procedure

```bash
git fetch origin main --tags
git checkout main
git pull --ff-only origin main
test -z "$(git status --porcelain)"

# Required local publication
git push origin main
SHA="$(git rev-parse HEAD)"
git fetch origin main
test "$SHA" = "$(git rev-parse origin/main)"

# Annotated tag, safe to resume after an interruption
TAG="v<major>.<minor>.<patch>"
if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
  test "$(git cat-file -t "refs/tags/$TAG")" = tag
  test "$(git rev-parse "refs/tags/$TAG^{}")" = "$SHA"
else
  git tag -a "$TAG" "$SHA" -m "<type>: <description>"
fi
git push origin "refs/tags/$TAG"
test "$(git ls-remote origin "refs/tags/$TAG^{}" | awk '{print $1}')" = "$SHA"
```

The typed equivalent is:

```bash
sddk release apply --route local --branch main --base main --cycle "<cycle-id>" \
  --tag "v<major>.<minor>.<patch>" --title "<type>: <description>" --approve
```

The local route reads the cycle's `verification-report`, `tests-pass`,
`policy-compliant`, and `release-uat-approved` evidence before Git effects.
`--route forge --repo owner/repo` remains available only for optional external
integration. It must not read provider checks or become the authority for the
main SHA or tag.

## Invariants

### Rule 0 - Trunk is the release source of truth

`main` is the release branch. Before and after release, fetch it and require:

```bash
test "$(git rev-parse HEAD)" = "$(git rev-parse origin/main)"
```

Repositories may use local branches for development, but a local integration
to `main` is sufficient. SDDK does not require a hosted pull request or any
provider-mediated merge.

### Rule 1 - Local verification precedes publication

`sddk-verify`, any required debt verification, and the configured UAT gate are
hard gates. These are local evidence gates, not remote CI checks. Never push
or tag a failed or dirty checkout.

### Rule 2 - Direct main push is the publication effect

The required remote effect is `git push origin main`. Its capability receipt
plus `HEAD == origin/main` constitutes `merge-receipt`. Do not wait for a
provider to report a merge, build, or status check.

### Rule 3 - Annotated tags mark releases

Every completed cycle creates or verifies one annotated `vX.Y.Z` tag at the
verified main SHA. A lightweight tag, a tag missing remotely, or a tag peeling
to another commit blocks the release. Re-entry never creates a second version.

Version bump rules:

| Bump | When |
| --- | --- |
| `major` | Breaking public API or contract |
| `minor` | New non-breaking feature |
| `patch` | Fix, chore, docs, refactor, perf, or test |

### Rule 4 - Receipts and no-pending-effects

- `merge-receipt`: full SHA confirmed at `HEAD` and `origin/main`, with the
  `git.push` capability receipt.
- `release-receipt`: annotated remote tag and its peeled SHA, with the
  `git.tag` capability receipt.
- `no-pending-effects`: no required local Git effect is pending. It explicitly
  excludes CI/CD, GitHub Actions, external releases, assets, signing, and
  distribution triggered by the tag.

### Rule 5 - Optional post-tag distribution

Tag-triggered workflows, release hosting, asset generation, signing, and
publication can consume a tag after SDDK release success. They are external
integration status, not cycle status. Never enable, cancel, rerun, or wait for
them as part of this contract.

### Rule 6 - Recovery

On interruption, inspect only local Git postconditions:

1. If `HEAD == origin/main`, direct trunk publication is complete.
2. If the remote annotated tag peels to `HEAD`, release tagging is complete.
3. If either postcondition differs, block with the observed SHA/tag evidence.
4. Resume the first missing local step. Do not query or wait for CI/CD.

## Phase Responsibilities

| Phase | Git responsibility |
| --- | --- |
| `sddk-apply` | Produce atomic conventional commits and preserve local verification evidence. |
| `sddk-verify` | Prove required tests and policies before local publication. |
| `sddk-debt-verify` | Supply mandatory A-* debt evidence before release. |
| `sddk-archive` | Hand off archive evidence and candidate release metadata. |
| `sddk-release` | Execute local verify, direct main push, SHA verification, annotated tag, receipts, and cycle bookkeeping. |

## Forbidden Dependencies

- Treating a PR as the only route to `main`.
- Reading or waiting on provider checks as a release gate.
- Waiting for Actions, assets, hosted releases, signing, or distribution.
- Declaring local release failed because an optional external integration fails.
- Retagging another SHA or creating a replacement version during retry.
