#!/usr/bin/env bash
# run-golden.sh — Golden dataset runner for SDDK debt-verify meta-validation
#
# Runs the debt-verify clusters against each golden case and compares
# the actual verdict against the expected verdict.
#
# Usage:
#   ./run-golden.sh                          # run all cases
#   ./run-golden.sh cases/06-god-class-fail/ # run one case
#
# Output: results/<case-name>.json with TP/FP/FN/TN classification
#
# PREREQUISITE: This script must be invoked from within an opencode/zcode
# session that has access to the debt-verify agents. It does NOT invoke
# the agents itself — it prepares the invocation prompt and expects the
# caller (the AI agent) to execute it.

set -euo pipefail

DATASET_DIR="${GOLDEN_DATASET_DIR:-$(cd "$(dirname "$0")/.." && pwd)}"
RESULTS_DIR="${DATASET_DIR}/results"
mkdir -p "$RESULTS_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# --- Functions ---

parse_expected() {
  local expected_file="$1"
  # Extract expected verdict using python (yaml not available everywhere)
  python3 -c "
import sys, yaml
with open('$expected_file') as f:
    data = yaml.safe_load(f)
exp = data.get('expected', {})
print(f\"{exp.get('verify', 'UNKNOWN')}\t{exp.get('debt', 'UNKNOWN')}\t{exp.get('primary_cluster', 'unknown')}\t{data.get('bucket', 'unknown')}\")
" 2>/dev/null || echo "UNKNOWN	UNKNOWN	unknown	unknown"
}

validate_case() {
  local case_dir="$1"
  local case_name=$(basename "$case_dir")

  echo ""
  echo "━━━ Case: $case_name ━━━"

  # Check required files exist
  local missing=0
  for required in spec.md expected-verdict.yaml; do
    if [ ! -f "$case_dir/$required" ]; then
      echo -e "${RED}  MISSING: $required${NC}"
      missing=1
    fi
  done
  if [ ! -d "$case_dir/implementation" ]; then
    echo -e "${RED}  MISSING: implementation/ directory${NC}"
    missing=1
  fi
  if [ "$missing" -eq 1 ]; then
    echo -e "${RED}  SKIP: incomplete case${NC}"
    return 1
  fi

  # Parse expected verdict
  local expected_line=$(parse_expected "$case_dir/expected-verdict.yaml")
  local exp_verify=$(echo "$expected_line" | cut -f1)
  local exp_debt=$(echo "$expected_line" | cut -f2)
  local exp_cluster=$(echo "$expected_line" | cut -f3)
  local bucket=$(echo "$expected_line" | cut -f4)

  echo "  Bucket: $bucket"
  echo "  Expected: verify=$exp_verify, debt=$exp_debt, primary=$exp_cluster"
  echo ""
  echo "  Implementation files:"
  find "$case_dir/implementation" -type f -name "*.ts" -o -name "*.js" -o -name "*.py" | sed 's/^/    /'
  echo ""

  # Count implementation metrics for sanity
  local total_loc=$(find "$case_dir/implementation" -type f \( -name "*.ts" -o -name "*.js" -o -name "*.py" \) -exec cat {} + 2>/dev/null | wc -l)
  local file_count=$(find "$case_dir/implementation" -type f \( -name "*.ts" -o -name "*.js" -o -name "*.py" \) | wc -l)
  echo "  Metrics: $file_count files, $total_loc LOC"
  echo ""
  echo -e "${YELLOW}  ⚠ TO RUN: invoke debt-verify against $case_dir/implementation/${NC}"
  echo "  Then compare actual verdict with expected-verdict.yaml"
  echo ""

  # Write result stub (actual verdict filled by the AI agent after running)
  cat > "$RESULTS_DIR/${case_name}.json" << EOF
{
  "case": "$case_name",
  "bucket": "$bucket",
  "expected": {
    "verify": "$exp_verify",
    "debt": "$exp_debt",
    "primary_cluster": "$exp_cluster"
  },
  "actual": {
    "verify": "NOT_RUN",
    "debt": "NOT_RUN",
    "findings": []
  },
  "classification": "PENDING",
  "metrics": {
    "files": $file_count,
    "loc": $total_loc
  }
}
EOF

  echo "  Result stub written to: results/${case_name}.json"
}

generate_summary() {
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  GOLDEN DATASET SUMMARY"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo ""

  local total=0
  local pending=0

  for result_file in "$RESULTS_DIR"/*.json; do
    [ -f "$result_file" ] || continue
    total=$((total + 1))

    local cls=$(python3 -c "import json; print(json.load(open('$result_file'))['classification'])" 2>/dev/null || echo "ERROR")
    local case_name=$(python3 -c "import json; print(json.load(open('$result_file'))['case'])" 2>/dev/null || echo "unknown")

    if [ "$cls" = "PENDING" ]; then
      pending=$((pending + 1))
      echo -e "  ${YELLOW}⏳ $case_name — PENDING (not yet run)${NC}"
    elif [ "$cls" = "TP" ] || [ "$cls" = "TN" ]; then
      echo -e "  ${GREEN}✅ $case_name — $cls (correct)${NC}"
    else
      echo -e "  ${RED}❌ $case_name — $cls (mismatch)${NC}"
    fi
  done

  echo ""
  echo "  Total cases: $total | Pending: $pending"
  echo ""
  echo "  NOTE: This runner prepares cases and validates structure."
  echo "  The actual debt-verify execution must be done by an AI agent"
  echo "  (opencode/zcode) with access to the cluster sub-agents."
  echo "  After running, fill 'actual' and 'classification' in each result JSON."
  echo ""
  echo "  Classification key:"
  echo "    TP = True Positive  (correctly found debt that exists)"
  echo "    TN = True Negative  (correctly found no debt)"
  echo "    FP = False Positive (flagged debt that doesn't exist)"
  echo "    FN = False Negative (missed debt that exists)"
}

# --- Main ---

echo "🔍 SDDK Golden Dataset Runner"
echo "   Dataset: $DATASET_DIR"

if [ $# -gt 0 ]; then
  # Run specific case
  case_path="$1"
  if [[ "$case_path" != /* ]]; then
    case_path="$DATASET_DIR/$case_path"
  fi
  validate_case "$case_path"
else
  # Run all cases
  for case_dir in "$DATASET_DIR"/cases/*/; do
    [ -d "$case_dir" ] && validate_case "$case_dir"
  done
fi

generate_summary
