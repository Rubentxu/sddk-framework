---
name: sddk-cycle-resume
description: "Trigger: orchestrator rebuilds state after session compaction, restart, or explicit /sddk-continue. Pull-based state reconstruction from authoritative CLI queries (sddk cycle lock status, sddk cycle status, sddk vault validate) — never from in-memory phase envelopes. Returns a validated state_token the orchestrator uses for Gate 0 pre-flight checks."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: gentleman-programming
  version: "1.0"
  delegate_only: true
  trigger_after_compaction: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Run this skill
> **inline** (do NOT delegate) — it is the orchestrator's own state
> reconstruction, not a phase agent's work.

## Why this skill exists

Between phases (and especially after a session compaction or restart), the
orchestrator must rebuild its working `state_token` from the **authoritative
CLI**, not from in-memory phase envelopes. Phase envelopes are push-based and
can be lost; the CLI is the source of truth (XDG-backed, ADR-0011).

## When to load

- Session start, BEFORE the first triage call.
- After any compaction event (per AGENTS.md "AFTER COMPACTION").
- Before each phase delegation in interactive mode (`Pre-flight Gate 0`).
- When `/sddk-continue` is invoked (mid-cycle resumption).

## CLI reconstruction steps

Run these in order. Every command MUST exit non-zero-tolerant — capture
output and continue.

```bash
# 1. Resolve canonical project identity (stable UUID)
PROJECT_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
PROJECT_ID="$(sddk project --root "$PROJECT_ROOT" --scope . 2>/dev/null | jq -r .id // unknown)"

# 2. Vault path (XDG, never inside the repo)
VAULT_PATH="$(sddk knowledge path --root "$PROJECT_ROOT" --scope . 2>/dev/null || echo "$HOME/.sddk-knowledge/$PROJECT_ID")"

# 3. Active lease — the gatekeeper for "is a cycle in progress?"
LEASE_JSON="$(sddk cycle lock status --root "$PROJECT_ROOT" --scope . --format json 2>/dev/null || echo '{"fencing_token": null}')"
ACTIVE="$(echo "$LEASE_JSON" | jq -r '.fencing_token // empty' 2>/dev/null)"

# 4. Cycle snapshot — only when there is an active lease
if [ -n "$ACTIVE" ]; then
  CYCLE_ID="$(echo "$LEASE_JSON" | jq -r .cycle_id)"
  CYCLE_JSON="$(sddk cycle status --root "$PROJECT_ROOT" --scope . --cycle "$CYCLE_ID" --format json 2>/dev/null)"
else
  CYCLE_JSON='{"phase": "absent"}'
fi

# 5. Last 10 ledger events (causal chain reconstruction)
LEDGER_JSON="$(sddk ledger events --root "$PROJECT_ROOT" --scope . --limit 10 --format json 2>/dev/null || echo '[]')"

# 6. Vault index coherence (advisory — non-blocking)
VAULT_DRIFT="$(sddk vault validate --root "$PROJECT_ROOT" --scope . --vault-path "$VAULT_PATH" --format json 2>/dev/null | jq -r '.drift_count // 0' || echo 'unknown')"

# 7. HEAD vs cycle head_sha (catch "another terminal pulled main" races)
HEAD_SHA="$(git rev-parse HEAD)"
CYCLE_HEAD_SHA="$(echo "$CYCLE_JSON" | jq -r '.head_sha // empty')"
HEAD_DRIFT="false"
if [ -n "$CYCLE_HEAD_SHA" ] && [ "$HEAD_SHA" != "$CYCLE_HEAD_SHA" ]; then
  HEAD_DRIFT="true"
fi
```

## state_token envelope

```json
{
  "project_id": "<stable-uuid>",
  "vault_path": "<xd-vault-path>",
  "cycle": {
    "id": "<cycle_id or absent>",
    "phase": "<phase name or absent>",
    "branch": "<branch name or main>",
    "head_sha": "<sha>",
    "fencing_token": "<token>"
  },
  "ledger_events": [/* last 10 */],
  "vault_drift_count": <int or "unknown">,
  "head_drift": <bool>,
  "rebuilt_at": "<RFC3339>"
}
```

## Hard rules

| Condition | Action |
|-----------|--------|
| `vault_drift_count > 0` | Log `vault-drift-detected`, run `sddk vault index --rebuild` once, continue |
| `head_drift == true` and cycle phase ≠ `release.complete` / `archive.complete` | Log `head-drift-detected`, refresh from `git fetch && git reset --hard origin/<branch>`, BLOCK the next delegation until stable |
| Cycle phase in `{archive.complete, release.complete}` but in-memory state shows mid-delegation | BLOCK with `next_recommended=/sddk-cycle-cancel` — another orchestrator won |
| `fencing_token` differs from in-memory `state_token` | BLOCK with `next_recommended=/sddk-cycle-cancel` — orchestrator desynced from lease |
| `sddk cycle lock status` exits non-zero | Treat as `fencing_token: null` (no active cycle) and proceed to triage |

## Return Format

- status: success | partial | blocked
- executive_summary: one sentence describing the rebuilt state
- state_token: the JSON envelope above
- next_recommended: phase to dispatch (or "ready for triage")
- risks: "vault-drift" / "head-drift" / "lease-desync" or "None"
- rebuild_source: ["cli:cycle-lock-status", "cli:cycle-status", "cli:ledger-list", "cli:vault-validate"]

## Difference from existing patterns

- **`mem_session_summary`** (Engram) — persists narrative across sessions.
- **`sddk-continue-options`** (skill) — presents tablet-friendly options.
- **`sddk-cycle-resume`** (this skill) — rebuilds **authoritative state** from the CLI. Other patterns consume its `state_token` output.

## Reference

- `prompts/sddk/orchestrator.md` § Pre-flight Gates (Gate 0)
- `prompts/sddk/mcw.md` § Phase 0 Step 0.2 (lock check)
- `prompts/sddk/status-query.md` (manual status queries)
- ADR-0011 — XDG-backed persistence, never repo-local SDDK state