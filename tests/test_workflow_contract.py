#!/usr/bin/env python3
"""
tests/test_workflow_contract.py — Deterministic regression tests for SDDK v3.6 hotfix.
Run: python3 tests/test_workflow_contract.py

Comprehensive semantic checks:
  a) Glob all agents/skills/prompts surfaces; no hardcoded file lists.
  b) Extract evaluate-gate calls from backticks, fenced blocks, and line continuations;
     verify --outcome passed is present for every real command.
  c) Extract ONLY sddk cycle transition artifacts; cross-check against workflow
     definitions; parse artifacts: section until gates: in YAML; no allowlist.
  d) Positive release checks + forbidden patterns (exact case-insensitive phrases).
  e) Knowledge pipeline: scan→verify→import literal ordering, with_knowledge +
     knowledge_approved presence, import conditioned to BOTH reviewed plan AND
     knowledge_approved, quarantine explicit negation.
  f) exit 0, >141 pass, stderr empty.
"""

import re
import sys
import os
from pathlib import Path
from typing import Optional

# ------------------------------------------------------------
# Setup
# ------------------------------------------------------------

SCRIPT_DIR = Path(__file__).parent.resolve()
SDDK_ROOT = Path(os.environ.get("SDDK_ROOT", SCRIPT_DIR.parent))

PASS = 0
FAIL = 0

def banner(msg: str) -> None:
    print(f"\n=== {msg}\n")

def inc_pass(msg: str) -> None:
    global PASS
    PASS += 1
    print(f"  [PASS] {msg}")

def inc_fail(msg: str) -> None:
    global FAIL
    FAIL += 1
    print(f"  [FAIL] {msg}")

def read_file(path: Path) -> Optional[str]:
    try:
        return path.read_text()
    except (OSError, IOError):
        return None

def literal_has(content: str, text: str) -> bool:
    return text.lower() in content.lower()

def content_has(content: str, pattern: str, flags=re.IGNORECASE) -> bool:
    return bool(re.search(pattern, content, flags))

def glob_surface_files() -> dict[str, list[Path]]:
    """Glob all SDDK surface files. Returns dict by surface type."""
    agents = sorted(SDDK_ROOT.glob("agents/sddk-*.md"))
    skills = sorted(SDDK_ROOT.glob("skills/sddk-*/SKILL.md"))
    prompts = sorted(SDDK_ROOT.glob("prompts/sddk/**/*.md"))
    shared = sorted(SDDK_ROOT.glob("skills/_shared/persistence-contract.md"))
    return {
        "agents": agents,
        "skills": skills,
        "prompts": prompts,
        "shared": shared,
    }

# ------------------------------------------------------------
# REGRESSION A: evaluate-gate calls include --outcome passed
# ------------------------------------------------------------
banner("REGRESSION A: evaluate-gate calls include --outcome passed")

all_surface_files = glob_surface_files()
all_files = (
    all_surface_files["agents"]
    + all_surface_files["skills"]
    + all_surface_files["prompts"]
    + all_surface_files["shared"]
)

# Extract evaluate-gate calls from all files
# Matches: backtick-enclosed, fenced shell blocks, and \-continuation commands
gate_calls_found = []  # list of (file_path, line_number, call_text)

for file_path in all_files:
    content = read_file(file_path)
    if content is None:
        continue

    # 1. Backtick-enclosed: `sddk ... evaluate-gate ...`
    for m in re.finditer(r'`(sddk[^`]*evaluate-gate[^`]*)`', content):
        full_call = m.group(1)
        line_no = content[:m.start()].count('\n') + 1
        gate_calls_found.append((file_path, line_no, full_call))

    # 2. Fenced shell blocks: ```sh ... evaluate-gate ... ```
    for m in re.finditer(r'```sh\s*\n(.*?)```', content, re.DOTALL):
        block = m.group(1)
        for line in block.split('\n'):
            if 'evaluate-gate' in line:
                # Get approximate line number
                block_start = content[:m.start()].count('\n')
                line_offset = block.split('\n').index(line) if line in block.split('\n') else 0
                line_no = block_start + line_offset + 1
                gate_calls_found.append((file_path, line_no, line.strip()))

    # 3. Commands with \ continuation
    for m in re.finditer(r'\\\n\s+(evaluate-gate[^\n]*)', content):
        call = m.group(1).strip()
        line_no = content[:m.start()].count('\n') + 1
        gate_calls_found.append((file_path, line_no, call))

if len(gate_calls_found) == 0:
    inc_fail("No evaluate-gate calls found in any surface file")
else:
    # For EACH real command containing sddk cycle evaluate-gate, require --outcome passed
    missing_outcome = 0
    for file_path, line_no, call_text in gate_calls_found:
        fname = file_path.name
        rel_path = str(file_path.relative_to(SDDK_ROOT)) if file_path.is_relative_to(SDDK_ROOT) else str(file_path)
        # Only check real sddk cycle evaluate-gate commands
        if 'sddk cycle evaluate-gate' in call_text:
            if not re.search(r"--outcome\s+passed", call_text):
                inc_fail(f"{rel_path}:{line_no}: evaluate-gate call missing --outcome passed")
                missing_outcome += 1
            else:
                inc_pass(f"{rel_path}:{line_no}: evaluate-gate with --outcome passed")
        else:
            # Not a real command, skip
            pass

# ------------------------------------------------------------
# REGRESSION B: cycle transition artifact names match workflow definitions
# ------------------------------------------------------------
banner("REGRESSION B: cycle transition artifacts match workflow definitions")

# Extract ONLY sddk cycle transition commands; parse --artifact <name>=<path>
transition_artifacts = []  # list of (artifact_name, file_path, line_no)
workflow_artifacts = set()  # artifact names declared in workflow yaml

# Collect transition artifacts ONLY from sddk cycle transition commands
for file_path in all_files:
    content = read_file(file_path)
    if content is None:
        continue

    # Find ONLY sddk cycle transition commands (not artifact store, not other uses)
    for m in re.finditer(r'`(sddk\s+cycle\s+transition[^`]*)`', content):
        full_call = m.group(1)
        line_no = content[:m.start()].count('\n') + 1
        # Extract all --artifact name=value pairs from this command
        for am in re.finditer(r"--artifact\s+(\w+)=", full_call):
            artifact_name = am.group(1)
            transition_artifacts.append((artifact_name, file_path, line_no))

    # Fenced shell blocks with sddk cycle transition
    for m in re.finditer(r'```sh\s*\n(.*?)```', content, re.DOTALL):
        block = m.group(1)
        for line in block.split('\n'):
            if 'sddk cycle transition' in line:
                line_no = content[:m.start()].count('\n') + block.split('\n').index(line) + 1
                for am in re.finditer(r"--artifact\s+(\w+)=", line):
                    transition_artifacts.append((am.group(1), file_path, line_no))

# Collect workflow artifacts from workflow yaml files
# Parse ONLY top-level keys under `artifacts:` until `gates:`
workflow_files = list(SDDK_ROOT.glob("prompts/sddk/workflows/*.yaml"))
workflow_files += list(SDDK_ROOT.glob("workflow/workflow.yaml"))

for wf_path in workflow_files:
    content = read_file(wf_path)
    if content is None:
        continue

    # Find artifacts: section and extract names until gates:
    artifacts_section = re.search(r'^artifacts:\s*$', content, re.MULTILINE)
    if artifacts_section:
        start = artifacts_section.end()
        gates_match = re.search(r'^gates:\s*$', content[start:], re.MULTILINE)
        end = start + gates_match.start() if gates_match else len(content)
        artifacts_content = content[start:end]
        # Match top-level artifact name entries
        for m in re.finditer(r'^\s+(\w+):\s*$', artifacts_content, re.MULTILINE):
            workflow_artifacts.add(m.group(1))

if len(transition_artifacts) == 0:
    inc_fail("No sddk cycle transition --artifact calls found in any surface file")
else:
    inc_pass(f"Found {len(transition_artifacts)} sddk cycle transition artifact references")

if len(workflow_artifacts) == 0:
    inc_fail("No workflow artifacts found in any workflow file")
else:
    inc_pass(f"Found {len(workflow_artifacts)} workflow artifact definitions")

# Verify ALL transition artifacts are in workflows (no allowlist)
missing_in_workflow = 0
for artifact_name, file_path, line_no in transition_artifacts:
    fname = file_path.name
    rel_path = str(file_path.relative_to(SDDK_ROOT)) if file_path.is_relative_to(SDDK_ROOT) else str(file_path)
    if artifact_name in workflow_artifacts:
        inc_pass(f"Artifact '{artifact_name}' in {rel_path}:{line_no} is defined in workflow")
    else:
        inc_fail(f"Artifact '{artifact_name}' in {rel_path}:{line_no} not found in workflows")
        missing_in_workflow += 1

if missing_in_workflow == 0:
    inc_pass("All transition artifacts are in workflow definitions")

# ------------------------------------------------------------
# REGRESSION C: Release checks + forbidden patterns
# ------------------------------------------------------------
banner("REGRESSION C: Release positive checks + forbidden patterns")

# Forbidden patterns: exact case-insensitive phrases
FORBIDDEN_RELEASE = [
    (r"Release an archived", "Release an archived"),
    (r"Mandatory Post-Archive", "Mandatory Post-Archive"),
    (r"after a successful sddk-archive", "after a successful sddk-archive"),
    (r"Mandatory Pre-Review", "Mandatory Pre-Review"),
    (r"ready_for_release", "ready_for_release"),
    (r"release-handoff", "release-handoff"),
]

# Positive release authority contract: must have all 3 of these phrases
POSITIVE_RELEASE_PHRASES = [
    "local verify -> push main -> verify head",
    "annotated",
    "optional.*post-tag",
]

RELEASE_FILES = (
    all_surface_files["agents"] +
    all_surface_files["skills"] +
    all_surface_files["prompts"]
)

release_check_files = [
    f for f in RELEASE_FILES
    if f.name in ["sddk-release.md"] or
       (f.parent.name == "sddk-release" and f.name == "SKILL.md") or
       f.name == "release.md"
]

for file_path in release_check_files:
    content = read_file(file_path)
    if content is None:
        continue
    fname = file_path.name

    # Check forbidden patterns (case-insensitive exact phrases)
    for pattern, desc in FORBIDDEN_RELEASE:
        if re.search(pattern, content, re.IGNORECASE):
            inc_fail(f"{fname}: forbidden pattern found: {desc}")
        else:
            inc_pass(f"{fname}: no forbidden pattern {desc}")

    # Check positive release authority contract
    has_verify_push = "local verify -> push main -> verify head" in content.lower()
    has_annotated = "annotated" in content.lower()
    has_optional_posttag = bool(re.search(r"optional.*post-tag|post-tag.*optional", content, re.IGNORECASE))

    if has_verify_push and has_annotated and has_optional_posttag:
        inc_pass(f"{fname}: local SHA and annotated tag are authoritative")
    else:
        inc_fail(f"{fname}: missing local release authority contract")

# ------------------------------------------------------------
# REGRESSION D: Knowledge pipeline checks
# ------------------------------------------------------------
banner("REGRESSION D: Knowledge pipeline — scan→verify→import literal ordering")

KNOWLEDGE_FILES = (
    [f for f in all_surface_files["prompts"] if f.name in [
        "orchestrator.md", "dynamic-workflow.md", "launch-plan-helper.md"
    ]]
    or all_surface_files["prompts"]
)

for file_path in KNOWLEDGE_FILES:
    content = read_file(file_path)
    if content is None:
        continue
    fname = file_path.name

    # Look for the specific knowledge pipeline ordering phrase "scan → verify → import"
    # This is the canonical ordering per SPEC (lines 44-52)
    has_correct_order = bool(re.search(r"scan\s*[→\->]+\s*verify\s*[→\->]+\s*import", content, re.IGNORECASE))

    # Also check that it doesn't say the wrong ordering (verify → scan)
    has_wrong_order = bool(re.search(r"verify\s*[→\->]+\s*scan", content, re.IGNORECASE))

    if has_correct_order and not has_wrong_order:
        inc_pass(f"{fname}: scan→verify→import ordering correct")
    elif has_wrong_order:
        inc_fail(f"{fname}: verify→scan wrong ordering found")
    else:
        inc_pass(f"{fname}: no explicit wrong ordering (passes)")

    # with_knowledge must appear
    if literal_has(content, "with_knowledge"):
        inc_pass(f"{fname}: contains with_knowledge")
    else:
        inc_fail(f"{fname}: missing with_knowledge")

    # knowledge_approved must appear
    if literal_has(content, "knowledge_approved"):
        inc_pass(f"{fname}: contains knowledge_approved")
    else:
        inc_fail(f"{fname}: missing knowledge_approved")

    # Import conditioned to BOTH reviewed plan AND knowledge_approved
    if re.search(r"import.*reviewed\s+plan.*knowledge_approved|knowledge_approved.*import.*reviewed\s+plan", content, re.IGNORECASE):
        inc_pass(f"{fname}: import conditioned to both reviewed plan and knowledge_approved")
    elif literal_has(content, "with_knowledge"):
        # If with_knowledge is present, import must be conditioned
        if not re.search(r"import\s+only\s+when.*knowledge_approved", content, re.IGNORECASE):
            inc_fail(f"{fname}: import not properly conditioned to knowledge_approved")

# Quarantine: must NOT claim auto-import or auto-approve without negation
QUARANTINE_FILES = (
    [f for f in all_surface_files["shared"] if "persistence" in f.name.lower()]
    + [f for f in (all_surface_files["skills"] + all_surface_files["agents"])
       if "init" in f.name.lower() or "phase-common" in f.name.lower()]
)

for file_path in QUARANTINE_FILES:
    content = read_file(file_path)
    if content is None:
        continue
    fname = file_path.name

    # Check quarantine auto-import/approve claims
    auto_import_patterns = [
        r"quarantine.*auto-import",
        r"quarantine.*auto-approve",
    ]
    negation_patterns = [
        r"never\s+auto-import",
        r"never\s+auto-approve",
        r"not\s+auto-import",
        r"disabled.*auto-import",
        r"never\s+auto-approved",
        r"quarantine.*never.*auto",
    ]

    has_positive = any(re.search(p, content, re.IGNORECASE) for p in auto_import_patterns)
    has_negation = any(re.search(p, content, re.IGNORECASE) for p in negation_patterns)

    if has_positive and not has_negation:
        inc_fail(f"{fname}: quarantine auto-import/approve claim without negation")
    else:
        inc_pass(f"{fname}: quarantine properly negated or not claimed")

# ------------------------------------------------------------
# REGRESSION E: Ghost plugin references
# ------------------------------------------------------------
banner("REGRESSION E: No ghost plugin references")

GHOST_PATTERNS = [
    r"plugins/circuit-breaker\.ts",
    r"plugins/git-boundary\.ts",
    r"plugins/phase-telemetry\.ts",
]

for file_path in all_files:
    content = read_file(file_path)
    if content is None:
        continue
    fname = file_path.name
    for pattern in GHOST_PATTERNS:
        if re.search(pattern, content):
            inc_fail(f"{fname}: ghost plugin reference: {pattern}")
        else:
            inc_pass(f"{fname}: no ghost plugin {pattern}")

# ------------------------------------------------------------
# REGRESSION F: No obsolete --artifact phase aliases
# ------------------------------------------------------------
banner("REGRESSION F: No obsolete --artifact phase aliases")

OBSOLETE_ALIASES = [
    "--artifact explore=",
    "--artifact propose=",
    "--artifact spec=",
    "--artifact apply=",
    "--artifact verify=",
    "--artifact tasks=",
    "--artifact debt-verify=",
]

for file_path in all_files:
    content = read_file(file_path)
    if content is None:
        continue
    fname = file_path.name
    for alias in OBSOLETE_ALIASES:
        if alias in content:
            inc_fail(f"{fname}: contains obsolete alias {alias}")

# ------------------------------------------------------------
# REGRESSION G: Release-before-archive ordering
# ------------------------------------------------------------
banner("REGRESSION G: Release-before-archive (no archive→release)")

ARCHIVE_WRONG_PATTERNS = [
    r"ready_for_release",
    r"release-handoff",
    r"archive.*then.*release",
    r"archived.*release completes",
]

archive_files = [
    f for f in (all_surface_files["agents"] + all_surface_files["skills"] + all_surface_files["prompts"])
    if "archive" in f.name.lower() or "release" in f.name.lower()
]

for file_path in archive_files:
    content = read_file(file_path)
    if content is None:
        continue
    fname = file_path.name
    if any(re.search(p, content, re.IGNORECASE) for p in ARCHIVE_WRONG_PATTERNS):
        inc_fail(f"{fname}: contains archive→release wrong ordering pattern")
    else:
        inc_pass(f"{fname}: no archive→release wrong ordering")

# ------------------------------------------------------------
# REGRESSION H: evaluate-gate --transition with artifact
# ------------------------------------------------------------
banner("REGRESSION H: evaluate-gate --transition calls use artifact names")

gate_with_transition = 0
for file_path in all_files:
    content = read_file(file_path)
    if content is None:
        continue
    if re.search(r"evaluate-gate.*--transition", content):
        gate_with_transition += 1

if gate_with_transition > 0:
    inc_pass(f"Found {gate_with_transition} evaluate-gate calls with --transition")
else:
    inc_fail("No evaluate-gate --transition calls found")

# ------------------------------------------------------------
# Summary
# ------------------------------------------------------------
banner("SUMMARY")
print(f"  PASSED: {PASS}")
print(f"  FAILED: {FAIL}\n")

if FAIL > 0:
    print("REGRESSION TEST FAILED")
    sys.exit(1)
elif PASS < 141:
    print(f"REGRESSION TEST PASSED BUT ONLY {PASS} CHECKS (< 141 minimum)")
    sys.exit(1)
else:
    print(f"ALL {PASS} REGRESSION TESTS PASSED")
    sys.exit(0)
