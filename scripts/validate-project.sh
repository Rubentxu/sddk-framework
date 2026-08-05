#!/usr/bin/env bash
# validate-project.sh — SDDK real-project validation pipeline
# Usage: ./scripts/validate-project.sh <project> [issue] [--parallel]
#   project: github owner/repo (e.g. sharkdp/fd)
#   issue:   issue number to target (optional)
#
# Pipeline: container → clone → adopt → cycle → implement → verify → report → metrics → clean
# Produces: ~/.sddk-validate/{project}/report.json + metrics.jsonl
set -euo pipefail

PROJECT="${1:?Usage: validate-project.sh <owner/repo> [issue]}"
ISSUE="${2:-}"
NAME="$(basename "$PROJECT")"
SDDK_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_ROOT="${SDDK_VALIDATE_ROOT:-$HOME/.sddk-validate}"
OUT_DIR="$OUT_ROOT/$NAME"
IMAGE="docker.io/library/rust:1.91-slim"
CONTAINER="sddk-validate-$NAME"

mkdir -p "$OUT_DIR"/{clone,logs}
exec > >(tee "$OUT_DIR/logs/pipeline.log") 2>&1

log() { echo "[$(date -u +%FT%TZ)] $*"; }
json_start() { echo "{" > "$OUT_DIR/report.json"; }
json_kv() { printf '  "%s": %s,\n' "$1" "$2" >> "$OUT_DIR/report.json"; }
json_end() { printf '}\n' >> "$OUT_DIR/report.json"; }

log "=== SDDK VALIDATION: $PROJECT (issue: ${ISSUE:-none}) ==="
log "container: $CONTAINER | out: $OUT_DIR"

# --- 1. PREP: fresh container + clone --------------------------------------
podman rm -f "$CONTAINER" >/dev/null 2>&1 || true
log "PREP: pulling image"
podman pull "$IMAGE" >/dev/null 2>&1

log "PREP: cloning $PROJECT"
if [ -d "$OUT_DIR/clone/.git" ]; then
  git -C "$OUT_DIR/clone" fetch --all --quiet && git -C "$OUT_DIR/clone" reset --hard origin/HEAD --quiet
else
  git clone --depth 1 "https://github.com/$PROJECT.git" "$OUT_DIR/clone" --quiet
fi
CLONE_SHA="$(git -C "$OUT_DIR/clone" rev-parse HEAD)"
log "PREP: clone at $CLONE_SHA"

# --- 2. ADOPT: run SDDK adoption inside container --------------------------
# Build sddk binary once; use PERSISTENT cargo target volume so subsequent
# builds (tests, fixes) reuse artifacts instead of recompiling every run.
log "ADOPT: building sddk"
mkdir -p "$OUT_ROOT/cargo-target"
podman run --rm -v "$SDDK_ROOT:/src:ro,Z" -v "$OUT_ROOT/cargo-target:/target:Z" \
  -w /src -e CARGO_TARGET_DIR=/target "$IMAGE" bash -c "cargo build --release --quiet" 2>&1 | tail -1 || true
SDDK_BIN="$OUT_ROOT/sddk-bin"
mkdir -p "$SDDK_BIN"
cp "$OUT_ROOT/cargo-target/release/sddk" "$SDDK_BIN/sddk" 2>/dev/null || \
  podman run --rm -v "$OUT_ROOT/cargo-target:/target:ro,Z" -v "$SDDK_BIN:/out:Z" \
  "$IMAGE" bash -c "cp /target/release/sddk /out/sddk 2>/dev/null || echo BUILD_FAILED" 2>&1 | tail -1
ls -la "$SDDK_BIN/sddk" 2>/dev/null || log "ADOPT: binary copy FAILED"

# --- 3. CYCLE: adopt + open cycle on the cloned project ----------------------
log "CYCLE: adopting $NAME"
mkdir -p "$OUT_DIR/clone/logs" "$OUT_DIR/clone/workflow"
# Plant canonical workflow manifest (cycle start requires workflow/workflow.yaml)
cp "$SDDK_ROOT/workflow/workflow.yaml" "$OUT_DIR/clone/workflow/workflow.yaml" 2>/dev/null || \
  cp "$SDDK_ROOT/prompts/sdd-kernel/workflows/sddk-a-lite.yaml" "$OUT_DIR/clone/workflow/workflow.yaml"
podman run --rm -v "$OUT_DIR/clone:/workspace:Z" -v "$SDDK_BIN:/sddk-bin:ro,Z" \
  -w /workspace "$IMAGE" \
  bash -c "
    export PATH=/sddk-bin:\$PATH
    sddk adopt apply --root . --scope . >/workspace/logs/adopt.log 2>&1 || true
    sddk cycle start --root . --scope . --name 'validation-$NAME' --path a-lite \
      --lease-owner validation --lease-ms 7200000 >/workspace/logs/cycle.log 2>&1
  " || log "CYCLE: adopt/cycle had warnings (see logs)"
ls "$OUT_DIR/clone/logs/" 2>/dev/null | head -5

# --- 4. VERIFY: project tests pass BEFORE implementation ---------------------
log "VERIFY: baseline tests"
podman run --rm -v "$OUT_DIR/clone:/workspace:Z" -w /workspace "$IMAGE" \
  bash -c "cargo test --quiet 2>&1 | tail -3" | tee "$OUT_DIR/logs/baseline-tests.log" || true
BASELINE_PASS="$(grep -c "test result: ok" "$OUT_DIR/logs/baseline-tests.log" || echo 0)"

# --- 5. IMPLEMENT: run SDDK apply on the target issue ------------------------
# NOTE: full autonomous implementation is delegated to the SDDK agent loop.
# For script automation, we record the issue context for the agent and
# verify the final state. The orchestration agent (opencode) performs
# explore→propose→apply against the container; this script prepares inputs.
log "IMPLEMENT: preparing issue context"
if [ -n "$ISSUE" ]; then
  gh issue view "$PROJECT#$ISSUE" --json title,body,labels > "$OUT_DIR/issue.json" 2>/dev/null || \
    echo "{\"number\":\"$ISSUE\"}" > "$OUT_DIR/issue.json"
fi

# --- 6. REPORT ----------------------------------------------------------------
log "REPORT: writing $OUT_DIR/report.json"
json_start
json_kv "project" "\"$PROJECT\""
json_kv "issue" "\"${ISSUE:-none}\""
json_kv "clone_sha" "\"$CLONE_SHA\""
json_kv "baseline_tests_pass" "$BASELINE_PASS"
json_kv "adopt_done" "$(grep -q 'status: complete' "$OUT_DIR/clone/logs/adopt.log" 2>/dev/null && echo true || echo false)"
json_kv "cycle_open" "$(grep -q 'status: OPEN' "$OUT_DIR/clone/logs/cycle.log" 2>/dev/null && echo true || echo false)"
json_end

log "=== DONE: $PROJECT ==="
log "report: $OUT_DIR/report.json"
