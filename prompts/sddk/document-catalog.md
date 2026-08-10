# SDDK Document Catalog

This catalog defines the authoritative location and ownership of SDDK output.
Resolve paths before work with `sddk knowledge status` and `sddk knowledge
path`; never infer `project_id` from the checkout name.

## Authority Matrix

| Class | Location | Content | Authority |
|---|---|---|---|
| Durable knowledge | `{vault}` | Milestones, ADRs, requirements, cycle nodes, incidences, terms | Canonical project knowledge |
| Cycle artifacts | `{cycle-artifacts-dir}` | Explore, proposal, spec, design, tasks, progress, verification, debt, archive, release, reports | Canonical operational record for one cycle |
| Generated docs | `$SDDK_DATA_DIR/projects/{project_id}/generated/` | Inventory and workflow renderings | Generated output |
| CAS and receipts | `$SDDK_DATA_DIR/projects/{project_id}/` | Artifact blobs, adoption receipt, workspace records | Operational state |
| Engram | Profile-controlled | Optional `sddk/...` mirrors and episodic memory | Never authoritative alone |
| `/tmp` | Temporary | Optional presentation copies | Disposable, never authoritative |

## Cycle Artifacts

Every phase writes beneath the CLI-resolved `{cycle-artifacts-dir}`:

| Artifact | Producer | Consumers |
|---|---|---|
| `explore-report.md` | `sddk-explore` | propose, coherence |
| `proposal.md` | `sddk-propose` | spec, design |
| `spec.md` or `specs/` | `sddk-spec` | design, tasks, verify, archive |
| `design.md` | `sddk-design` | tasks, apply, verify |
| `tasks.md` | `sddk-tasks` | apply, verify |
| `apply-progress.yaml` | `sddk-apply` | verify, orchestrator |
| `verify-report.md` | `sddk-verify` | debt-verify, archive |
| `debt-report.md` | `sddk-debt-verify` | archive, release |
| `archive-report.md` | `sddk-archive` | release, HTML report |
| `release-report.md` | `sddk-release` | cycle audit |
| `reports/cierre.html` | `sddk-release` | human reviewer |

Optional Engram mirrors use `sddk/{change-name}/{artifact-type}` only when
`sddk knowledge status` reports `engram_enabled: true`.

## Durable Nodes

| Node | Vault location | Owner |
|---|---|---|
| Milestone and serialization lock | `{vault}/milestones/` | orchestrator, release |
| ADR | `{vault}/adrs/` | spec/design proposal; orchestrator-approved write |
| Requirement | `{vault}/specs/{domain}/` | spec, archive |
| Cycle | `{vault}/cycles/` | archive, release |
| Incidence | `{vault}/incidences/` | verify, debt-verify, release |
| Term | `{vault}/terms/` | explore, spec |

All vault writes follow `skills/knowledge-graph/SKILL.md`, preserve provenance,
and append to `{vault}/_log.md`.

## Adopted Workspace

The workspace contains product code and product-owned files. SDDK may read
existing `README`, architecture documents, ADRs, roadmap, `CONTEXT.md`, or
other product documentation as **read-only evidence**. Those files are not
SDDK authority and SDDK never creates, updates, migrates, or indexes them in
place.

SDDK never writes workspace-local `docs/`, ROADMAP, ADRs, specs, context files,
`sddk/`, `.sddk/`, `.atl/`, workflow manifests, ignore files, checkpoints, or
reports. Missing SDDK knowledge is repaired in `{vault}`; missing operational
state is repaired under `$SDDK_DATA_DIR/projects/{project_id}/`.

## Cross-References

- Cycle artifacts reference neighbouring artifacts by paths under
  `{cycle-artifacts-dir}`.
- Durable nodes use vault wikilinks.
- The cycle node links the PR/tag, verdicts, relevant durable nodes, and
  operational artifact paths.
- A temporary HTML copy may point back to
  `{cycle-artifacts-dir}/reports/cierre.html`; it is not retained as authority.

## Dogfooding Exception

`sddk generate docs|inventory --in-repo` is allowed only when explicitly run
against the `sddk-framework` development repository. It is never used for an
adopted product workspace.
