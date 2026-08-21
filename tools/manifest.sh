#!/usr/bin/env bash
# Regenerates MANIFEST.sha256 from current prompts/ + skills/ + agents/ + docs/
# Usage: tools/manifest.sh
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

# Files that go into the runtime bundle
BUNDLE_FILES=(
  "prompts/sddk"
  "skills"
  "agents"
  "assets"
)

# Hash each file with sha256, exclude generated files
find "${BUNDLE_FILES[@]}" -type f \
  ! -name "*.sha256" \
  ! -path "*/target/*" \
  ! -path "*/.git/*" \
  -print0 \
| sort -z \
| xargs -0 sha256sum \
> MANIFEST.sha256

echo "MANIFEST.sha256 regenerated: $(wc -l < MANIFEST.sha256) entries"
