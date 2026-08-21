# SDDK Debt-Verify Gate Contract

This document is the single declarative authority for the post-verify technical
debt gate. Agents and skills point here instead of copying its decision tables
or report schemas.

Debt-verify is a workflow **capability/gate** between functional verify and
release. It is not a new value in the legacy runtime `Phase` enum. Runtime and
CLI enforcement are intentionally outside this specification change.

## Activation

Run after `sddk-verify` returns `PASS` or `PASS_WITH_WARNINGS`:

| Path | Policy | Depth | Required clusters |
|---|---|---|---|
| A-min | mandatory | smoke | coupling, overeng |
| A-lite | mandatory | standard | coupling, overeng, smells, duplication |
| A-full | mandatory | deep | architecture, coupling, overeng, smells, duplication |
| B-direct | disabled | n/a | none |

Depth is path-derived and locked. Reversibility may influence triage into a
different path; it does not skip or deepen debt-verify after the cycle starts.

## Policy Trade-offs

| Choice | Benefit | Cost / Risk | Mitigation |
|---|---|---|---|
| Path-derived depth | Predictable cost and no mid-cycle negotiation | Smoke and standard may miss dimensions outside their cluster set | Triage irreversible or architectural work into A-full before the cycle starts |
| Fail closed on incomplete coverage | Prevents a partial audit from becoming a false PASS | Analyzer outages can delay release without proving debt | Retry transient failures up to the bounded limit, then require human review |
| Block only introduced/updated debt | Makes adoption viable in repositories with legacy debt | Pre-existing debt can remain indefinitely | Keep it visible and create owned, prioritized follow-up incidences |
| Reproducible evidence and stable fingerprints | Enables deduplication, comparison, and audit | Adds hashing, normalization, and provenance overhead | Scale cluster count by path and reject unsupported numeric precision |
| JSON authority plus Markdown projection | Gives machines deterministic input and humans a readable report | Two artifacts can drift | Generate Markdown from persisted JSON; bind both hashes in the outer envelope |
| Specification-only runtime handoff | Avoids claiming CLI enforcement that does not exist | The declarative gate can be bypassed by current runtime integrations | Track typed runtime enforcement as deferred roadmap work |

## Required Input

The orchestrator supplies one immutable audit packet:

```yaml
contract_version: debt-gate/v1
cycle:
  cycle_id: {cycle-id}
  change_name: {change-name}
  path: A-min | A-lite | A-full
  remediation_round: 0..3
subject:
  branch: {feature-branch}
  base_commit: {full SHA}
  head_commit: {full SHA}
  diff_digest: {sha256 of normalized base...head diff}
scope:
  effective_depth: smoke | standard | deep
  changed_paths: [repo-relative paths]
  one_hop_dependencies: [repo-relative paths]
verify_evidence:
  path: {cycle-artifacts-dir}/verify-report.md
  sha256: {64 lowercase hex}
  subject_sha: {same head_commit}
  verdict: PASS | PASS_WITH_WARNINGS
router:
  context_quality: C0 | C1 | C2 | C3
  strict_tdd: true | false
  engram_memory: true | false
```

## Preflight

Block before launching clusters when any condition fails:

1. Verify evidence is missing, malformed, not PASS/PW, or bound to another SHA.
2. `base_commit` or `head_commit` is unavailable.
3. The worktree contains changes not represented by `head_commit`.
4. Path and depth do not match the Activation table.
5. `remediation_round > 3`.

Use `base_commit...head_commit` for scope. Never assume the default branch is
named `main`. Remote push status is release evidence, not a debt-analysis
precondition.

## Cluster Run Contract

Every required cluster returns:

```yaml
cluster_run:
  cluster: debt-{dimension}-cluster
  status: completed | failed | timed_out
  attempts: 1..3
  analyzer:
    name: {agent or tool}
    version: {model, skill hash, or tool version}
  subject_sha: {head_commit}
  started_at: {RFC3339}
  finished_at: {RFC3339}
  findings: [Common Finding]
  errors: [{code, message}]
```

A required run that is not `completed`, or whose subject differs, makes the
global verdict `INCONCLUSIVE`.

## Common Finding Contract

All clusters normalize findings to this shape. Cluster-specific payloads may be
added under `details`.

```yaml
finding:
  finding_id: {cluster-local stable id}
  fingerprint: {sha256 of normalized canonical rule_id + path + symbol/context}
  rule_id: {canonical rule identifier shared across clusters}
  cluster: architecture | smells | duplication | coupling | overeng
  category: {stable category}
  severity: CRITICAL | HIGH | MEDIUM | LOW
  confidence: HIGH | MEDIUM | LOW
  baseline_state: new | updated | unchanged | unknown
  attribution: introduced | pre_existing | unknown
  locations:
    - path: {repo-relative path}
      start_line: {positive integer}
      end_line: {positive integer}
      symbol: {optional symbol}
  evidence:
    - kind: command | source | graph | test | analyzer
      observation: {what was observed, not an inference}
      command: {optional exact argv/string}
      tool: {tool name}
      tool_version: {version or unknown}
      exit_code: {integer or null}
      output_digest: {sha256 or null}
  impact: {concrete failure/change cost}
  remediation:
    target: apply | replan | backlog
    action: {specific next action}
  details: {}
```

Rules:

- `severity` measures impact; `confidence` measures evidentiary certainty.
- Corroboration by multiple clusters raises confidence only.
- `rule_id` identifies the issue independently of the analyzer. Clusters that
  observe the same issue use the same canonical rule id, so `fingerprint` does
  not include cluster identity. `finding_id` remains cluster-local.
- A finding without a repository-relative location and observable evidence is
  invalid unless its category is repository-wide; repository-wide findings
  must cite the analyzed scope and command/tool evidence.
- Numeric estimates require a reproducible method. Otherwise emit qualitative
  bands and raw counts.
- `baseline_state` is computed against the supplied base/head pair. `git blame`
  may add provenance but does not decide attribution by itself.
- Normalize paths, ordering, and fingerprints before hashing so identical input
  produces identical output.

## Baseline And Suppressions

The gate follows a new-code policy:

- Findings with `attribution: introduced` and `baseline_state: new | updated`
  participate in blocking decisions.
- `pre_existing` findings remain visible and create follow-up debt, but do not
  become newly introduced merely because a cluster rediscovered them.
- `unknown` attribution on a would-be blocker yields `INCONCLUSIVE`.
- A suppression requires a finding fingerprint, human owner, justification,
  creation date, and expiry. Agent-authored or expired suppressions do not waive
  a finding.
- A valid suppression removes a finding from blocking counts but not from the
  report. The decision reasons must name every applied suppression.

## Deterministic Aggregation

1. Sort cluster runs by cluster name.
2. Reject malformed or wrong-subject results.
3. Normalize and sort findings by fingerprint.
4. Merge identical fingerprints; retain all evidence and source clusters.
5. Raise confidence when independent evidence corroborates a finding.
6. Count findings once after deduplication.
7. Apply the Decision Contract in table order.

## Decision Contract

| First matching condition | Verdict | Remediation |
|---|---|---|
| Required cluster missing/failed/timed out; invalid subject; malformed evidence; unknown attribution or LOW confidence on a potential blocker | `INCONCLUSIVE` | retry gate or human review |
| Any unsuppressed introduced CRITICAL finding in baseline state new/updated with confidence HIGH or MEDIUM | `FAIL` | `apply` unless structural signals require `replan` |
| Circular dependency, unencapsulated shared mutable state, or contract-breaking LSP violation introduced by the change | `FAIL` | `replan` for boundary/design failure; otherwise `apply` |
| Three or more unsuppressed introduced HIGH findings in baseline state new/updated with confidence HIGH/MEDIUM | `FAIL` | `apply` |
| One or two unsuppressed introduced HIGH findings, or three or more introduced MEDIUM findings, with no blocker | `PASS_WITH_WARNINGS` | `none`; attach backlog |
| Only pre-existing HIGH/CRITICAL findings, with complete evidence and no introduced blocker | `PASS_WITH_WARNINGS` | `none`; create follow-up incidence |
| No warning or blocking condition | `PASS` | `none` |

`re_iterate_from: replan` blocks automatic progression and recommends new
exploration/proposal work while preserving the current cycle branch and
evidence. It does not claim the current CLI can rewind phase state. The
orchestrator must surface the blocker and obtain an explicit recovery/new-cycle
decision before dispatching planning work.

## Authoritative Report

Persist `debt-report.json` as the machine authority. Render
`debt-report.md` from the same data for humans; Markdown never overrides JSON.

```yaml
contract_version: debt-gate/v1
report_id: {stable id}
generated_at: {RFC3339}
cycle: {cycle_id, change_name, path, remediation_round}
subject: {branch, base_commit, head_commit, diff_digest}
verify_evidence: {path, sha256, subject_sha, verdict}
coverage:
  required_clusters: [names]
  completed_clusters: [names]
  failed_clusters: [{name, status, attempts, errors}]
findings: [Common Finding, deduplicated, with source_clusters]
summary:
  total: {n}
  by_severity: {critical: n, high: n, medium: n, low: n}
  by_confidence: {high: n, medium: n, low: n}
  by_attribution: {introduced: n, pre_existing: n, unknown: n}
  by_cluster: {cluster: n}
decision:
  verdict: PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE
  re_iterate_from: replan | apply | none
  reasons: [{rule, finding_fingerprints, explanation}]
  fail_closed: true
waivers: [{fingerprint, owner, justification, created_at, expires_at}]
follow_up: [{finding_fingerprint, action, owner, priority}]
artifact_paths:
  json: {path}
  markdown: {path}
runtime_handoff:
  status: specification_only
  desired_artifact_kind: debt-report
  desired_gate: debt-approved
  note: "No debt-specific CLI transition is declared by this documentation change."
```

The JSON report cannot contain its own digest or the Markdown digest without a
hash cycle. Persist and hash JSON first, render Markdown with
`source_json_sha256`, hash Markdown, then place both digests only in the outer
orchestrator envelope.

## Markdown Projection

The human report leads with:

1. Verdict, subject SHAs, and coverage completeness.
2. Counts by severity, confidence, attribution, and cluster.
3. Blocking/warning findings with `path:line`, evidence, and remediation.
4. Pre-existing findings and follow-up ownership.
5. Cluster failures or uncertainty.
6. Runtime handoff status and source JSON hash. The Markdown digest exists only
   in the outer envelope because an artifact cannot contain its own stable hash.

## Orchestrator Envelope

```yaml
contract_version: debt-gate/v1
status: success | partial | blocked
executive_summary: {1-3 evidence-bound sentences}
artifacts:
  - {kind: debt-report-json, path: ..., sha256: ...}
  - {kind: debt-report-markdown, path: ..., sha256: ...}
subject: {cycle_id, base_commit, head_commit, diff_digest}
verdict: PASS | PASS_WITH_WARNINGS | FAIL | INCONCLUSIVE
re_iterate_from: replan | apply | none
cluster_coverage: {required: n, completed: n, failed: n}
findings_by_severity: {critical: n, high: n, medium: n, low: n}
findings_by_attribution: {introduced: n, pre_existing: n, unknown: n}
next_recommended: sddk-release | sddk-apply | sddk-explore | retry-debt-verify | human-review
runtime_handoff: specification_only
risks: []
context_quality: C0 | C1 | C2 | C3
```

Mapping:

| Verdict | status | next_recommended |
|---|---|---|
| PASS/PASS_WITH_WARNINGS | success | sddk-release |
| FAIL + apply | blocked | sddk-apply |
| FAIL + replan | blocked | human-review |
| INCONCLUSIVE, retryable | partial | retry-debt-verify |
| INCONCLUSIVE, non-retryable | blocked | human-review |

## References

- `agents/sddk-debt-verify.md`
- `agents/debt-architecture-cluster.md`
- `agents/debt-smells-cluster.md`
- `agents/debt-duplication-cluster.md`
- `agents/debt-coupling-cluster.md`
- `agents/debt-overeng-cluster.md`
- `skills/sddk-debt-verify/SKILL.md`
- `prompts/sddk/orchestrator.md`
- `prompts/sddk/mcw.md`
- `prompts/sddk/git-contract.md`
