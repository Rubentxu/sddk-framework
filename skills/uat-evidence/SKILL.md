---
name: uat-evidence
description: "Trigger: uat-evidence, captura evidencia, screenshot UAT. Capture and hash UAT evidence in the browser (clipboard API, MediaRecorder) and reference it by SHA-256 in uat-session.yaml."
disable-model-invocation: true
user-invocable: false
license: MIT
metadata:
  author: sddk-framework
  version: "1.0"
  delegate_only: true
---

> **ORCHESTRATOR GATE**: If you loaded this skill, STOP. Delegate to `uat-runner`.

## Purpose

Evidence is what makes a PASS/FAIL defensible. Every verdict should be backed by a screenshot, log snippet, or note — referenced by hash so the chain ties to the ledger (ADR-003).

## Capture patterns

| Kind | How | Notes |
|------|-----|-------|
| `screenshot` | Ctrl+V paste in the guided wizard (clipboard API) | The wizard stores a data-URL preview and records `sha256:<hash>` |
| `log` | Copy a console/terminal snippet into the comment box | Reference `sha256:<hash>` of the snippet |
| `note` | Free text | For observations that are not visual |

## Integrity rules

1. Every evidence entry has `kind` + `ref` in `sha256:<hash>` form.
2. The payload itself is stored in XDG artifacts (`~/.local/share/sddk/projects/<id>/uat/`), never in the project repo (ADR-0011).
3. The session file only carries the reference — it stays small and diffable.

## Browser capabilities used

- Clipboard API (`navigator.clipboard.read` / paste event) for screenshots — no external tools.
- MediaRecorder (future): video evidence for flaky scenarios (ADR-012 reevaluation trigger).
- localStorage: session progress survives reloads.

## References

- `agents/uat-runner.md` — pre-flight execution with evidence
- ADR-003 (ledger hash chain), ADR-012 §4 (evidence) in the knowledge vault
