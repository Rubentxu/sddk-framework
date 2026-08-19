# Migration Plan — Current SDDK → Decision Kernel with Dynamic Workflows

## Principle
Strangle the legacy architecture; do not rewrite all packs/agents at once.

## Step 1 — Architectural seam
Remove concrete storage creation from core, introduce focused ports and contract tests. Preserve current CLI/agent behavior.

## Step 2 — Event compatibility
Map current cycle/phase/agent results into canonical events without changing user-visible SDD behavior.

## Step 3 — Workflow v2 compatibility compiler
Translate current `CyclePath/Phase` paths into `WorkflowTemplate/WorkflowIR` while keeping existing agents/artifacts.

```text
AFull legacy manifest → LegacySddCompiler → WorkflowIR
```

## Step 4 — Introduce dynamic operators without changing SDD default
Implement Map/Join/Loop/ExecutionGraphRevision and prove them with synthetic workflows/research/incident use cases.

## Step 5 — OpenCode + resilience
Add AgentHost adapter and provider failover on the new Attempt model.

## Step 6 — Introduce ChangeContract alongside current artifacts
During current A-full, populate ChangeContract from explore/propose/spec/design/tasks. This proves schema completeness before removing phase boundaries.

## Step 7 — Add `sdd-adaptive` as experimental
Run SHAPE/BUILD/CONVERGE/INTEGRATE. Generate legacy Markdown artifacts as projections where useful.

## Step 8 — Workflow Laboratory
Compare A-full and adaptive. Run ablation tests merging/removing phase boundaries. Measure quality first, efficiency second.

## Step 9 — Promote cautiously
If evidence supports it:
- adaptive becomes default for eligible changes;
- A-full remains an explicit high-ceremony/reference preset;
- `CyclePath` compatibility remains at boundary until deprecation window ends.

## Step 10 — Remove kernel SDD coupling
Only after packs and migration adapters cover persisted legacy state, remove `Phase/CyclePath` from generic domain runtime.

## No-delete rule in this refinement
No existing pack/spec needs deletion now. The new architecture is additive and migration-oriented.
