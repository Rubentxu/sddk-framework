#!/usr/bin/env bash
# tests/test_workflow_contract.sh — wrapper that delegates to Python implementation
set -euo pipefail
exec python3 "$(dirname "${BASH_SOURCE[0]}")/test_workflow_contract.py" "$@"
