#!/usr/bin/env python3
"""
tests/test_workflow_contract.py — Deterministic regression tests for SDDK v3.6 hotfix.
Run: python3 tests/test_workflow_contract.py

Comprehensive semantic checks:
  a) Glob all agents/skills/prompts surfaces; no hardcoded file lists.
  b) Extract evaluate-gate calls from backticks, fenced blocks (bash/sh), and line continuations;
     verify an explicit --outcome is present for every real command.
  c) Extract ONLY sddk cycle transition artifacts; cross-check against workflow
     definitions; parse artifacts: section with indent-2 keys until gates: in YAML; no allowlist.
  d) Positive release checks + forbidden patterns + archive report precondition check.
  e) Knowledge pipeline: scan→verify→import literal ordering for core files only.
  f) Propose/debt: require sddk artifact store and absence of sddk cycle transition.
  g) exit 0, >141 pass, stderr empty, 13 workflow definitions + >=15 transition artifact refs.
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
# REGRESSION A: evaluate-gate calls include an explicit --outcome
# ------------------------------------------------------------
banner("REGRESSION A: evaluate-gate calls include an explicit --outcome")

all_surface_files = glob_surface_files()
all_files = (
    all_surface_files["agents"]
    + all_surface_files["skills"]
    + all_surface_files["prompts"]
    + all_surface_files["shared"]
)

# Extract evaluate-gate calls from all files
# Matches: backtick-enclosed, fenced bash/sh blocks, and \-continuation commands
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

    # 2. Fenced shell blocks: ```sh or ```bash ... evaluate-gate ... ```
    for m in re.finditer(r'```(?:sh|bash)\s*\n(.*?)```', content, re.DOTALL):
        block = m.group(1)
        block_start = content[:m.start()].count('\n')
        # Join continued lines (lines ending with \)
        continued_lines = []
        for i, line in enumerate(block.split('\n')):
            if line.rstrip().endswith('\\'):
                continued_lines.append(line.rstrip()[:-1] + ' ')
            else:
                continued_lines.append(line.rstrip())
                # Process the completed logical line
                full_line = ''.join(continued_lines)
                if 'evaluate-gate' in full_line:
                    line_no = block_start + i + 1
                    gate_calls_found.append((file_path, line_no, full_line))
                continued_lines = []
        # Handle case where block ends with a continued line
        if continued_lines:
            full_line = ''.join(continued_lines)
            if 'evaluate-gate' in full_line:
                line_no = block_start + len(block.split('\n')) + 1
                gate_calls_found.append((file_path, line_no, full_line))

    # 3. Commands with \ continuation
    for m in re.finditer(r'\\\n\s+(evaluate-gate[^\n]*)', content):
        call = m.group(1).strip()
        line_no = content[:m.start()].count('\n') + 1
        gate_calls_found.append((file_path, line_no, call))

if len(gate_calls_found) == 0:
    inc_fail("No evaluate-gate calls found in any surface file")
else:
    # For EACH real command containing sddk cycle evaluate-gate, require fail-closed intent.
    missing_outcome = 0
    for file_path, line_no, call_text in gate_calls_found:
        fname = file_path.name
        rel_path = str(file_path.relative_to(SDDK_ROOT)) if file_path.is_relative_to(SDDK_ROOT) else str(file_path)
        # Only check real sddk cycle evaluate-gate commands
        if 'sddk cycle evaluate-gate' in call_text:
            if not re.search(
                r"--outcome\s+(?:passed|failed|waived|\{(?:[a-z_]+|passed\|failed)\})",
                call_text,
            ):
                inc_fail(f"{rel_path}:{line_no}: evaluate-gate call missing explicit --outcome")
                missing_outcome += 1
            else:
                inc_pass(f"{rel_path}:{line_no}: evaluate-gate with explicit --outcome")

# ------------------------------------------------------------
# REGRESSION B: cycle transition artifact names match workflow definitions
# ------------------------------------------------------------
banner("REGRESSION B: cycle transition artifacts match workflow definitions")

# Extract ONLY sddk cycle transition commands; parse --artifact <name>=<path>
# Artifact names support hyphens: [a-z0-9][a-z0-9-]*
transition_artifacts = []  # list of (artifact_name, file_path, line_no)
workflow_artifacts = set()  # artifact names declared in workflow yaml

# Collect transition artifacts ONLY from sddk cycle transition commands
for file_path in all_files:
    content = read_file(file_path)
    if content is None:
        continue

    # Find ONLY sddk cycle transition commands (not artifact store, not other uses)
    # Handle inline backticks
    for m in re.finditer(r'`(sddk\s+cycle\s+transition[^`]*)`', content):
        full_call = m.group(1)
        line_no = content[:m.start()].count('\n') + 1
        # Extract all --artifact name=value pairs from this command
        # Support hyphens in artifact names
        for am in re.finditer(r"--artifact\s+([a-z0-9][a-z0-9-]*)=", full_call):
            artifact_name = am.group(1)
            transition_artifacts.append((artifact_name, file_path, line_no))

    # Fenced shell blocks with sddk cycle transition (bash/sh)
    for m in re.finditer(r'```(?:sh|bash)\s*\n(.*?)```', content, re.DOTALL):
        block = m.group(1)
        block_start = content[:m.start()].count('\n')
        for i, line in enumerate(block.split('\n')):
            if 'sddk cycle transition' in line:
                line_no = block_start + i + 1
                for am in re.finditer(r"--artifact\s+([a-z0-9][a-z0-9-]*)=", line):
                    transition_artifacts.append((am.group(1), file_path, line_no))

# Collect workflow artifacts from workflow yaml files
# Parse ONLY top-level keys under `artifacts:` (indent-2) until `gates:`
workflow_files = list(SDDK_ROOT.glob("prompts/sddk/workflows/*.yaml"))
workflow_files += list(SDDK_ROOT.glob("workflow/workflow.yaml"))

for wf_path in workflow_files:
    content = read_file(wf_path)
    if content is None:
        continue

    # Find artifacts: section and extract names (indent-2 keys) until gates:
    artifacts_section = re.search(r'^artifacts:\s*$', content, re.MULTILINE)
    if artifacts_section:
        start = artifacts_section.end()
        gates_match = re.search(r'^gates:\s*$', content[start:], re.MULTILINE)
        end = start + gates_match.start() if gates_match else len(content)
        artifacts_content = content[start:end]
        # Match artifact name entries at exactly 2 spaces indentation under artifacts:
        # Format: "  artifact-name:\n"
        for m in re.finditer(r'^  ([a-z0-9][a-z0-9-]*):\s*$', artifacts_content, re.MULTILINE):
            workflow_artifacts.add(m.group(1))

if len(transition_artifacts) == 0:
    inc_fail("No sddk cycle transition --artifact calls found in any surface file")
else:
    inc_pass(f"Found {len(transition_artifacts)} sddk cycle transition artifact references")

if len(workflow_artifacts) == 0:
    inc_fail("No workflow artifacts found in any workflow file")
else:
    inc_pass(f"Found {len(workflow_artifacts)} workflow artifact definitions")
    # Require at least 13 workflow definitions
    if len(workflow_artifacts) < 13:
        inc_fail(f"Expected >= 13 workflow definitions, found {len(workflow_artifacts)}")

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

# Require >= 15 transition artifact refs
if len(transition_artifacts) < 15:
    inc_fail(f"Expected >= 15 transition artifact refs, found {len(transition_artifacts)}")

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
    (r"closes an archived cycle", "closes an archived cycle"),
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

    # FAILED if release precondition contains "archive report"
    if re.search(r"archive\s+report", content, re.IGNORECASE):
        inc_fail(f"{fname}: release precondition contains 'archive report'")
    else:
        inc_pass(f"{fname}: release precondition has no archive report")

    # Require positive after review (A-full) - only if file mentions A-full path or ordering context
    mentions_review = bool(re.search(r"A-full\b", content, re.IGNORECASE))
    has_after_review = bool(re.search(
        r"after\s+review|or\s+review\s*\(|review\s+phase",
        content, re.IGNORECASE
    ))
    if mentions_review:
        # File mentions A-full/review - require positive after review
        if has_after_review:
            inc_pass(f"{fname}: has positive after review")
        else:
            inc_fail(f"{fname}: missing positive after review")
    else:
        # File doesn't mention review/A-full - skip check
        inc_pass(f"{fname}: no review phase required (A-min/lite path)")

    # Require after verify (A-min/lite) - flexible matching
    has_after_verify = bool(re.search(
        r"after\s+(successful\s+)?verify|verify\s+phase",
        content, re.IGNORECASE
    ))
    if has_after_verify:
        inc_pass(f"{fname}: has positive after verify")
    else:
        inc_fail(f"{fname}: missing positive after verify")

    # Require BEFORE archive - flexible matching
    has_before_archive = bool(re.search(
        r"before\s+(the\s+)?archive|prior\s+to\s+archive",
        content, re.IGNORECASE
    ))
    if has_before_archive:
        inc_pass(f"{fname}: has positive before archive")
    else:
        inc_fail(f"{fname}: missing positive before archive")

# ------------------------------------------------------------
# REGRESSION D: Knowledge pipeline checks (CORE FILES ONLY)
# ------------------------------------------------------------
banner("REGRESSION D: Knowledge pipeline — scan→verify→import literal ordering")

# Canonical knowledge ordering patterns
KNOWLEDGE_ORDER_PATTERNS = [
    r"scan\s*[→\->]+\s*verify\s*[→\->]+\s*import",  # scan → verify → import
    r"run\s+scan\s+then\s+verify[;\s].*import",      # run scan then verify; import...
]

# ONLY these 3 core files require explicit knowledge ordering
CORE_KNOWLEDGE_FILES = {
    "orchestrator.md", "dynamic-workflow.md", "launch-plan-helper.md"
}

# Additional flexible ordering patterns (run scan then verify without import on same line)
# Handle backticks around scan/verify in markdown
FLEXIBLE_ORDER_PATTERNS = [
    r"run\s+\`?\s*scan\s*\`?\s+then\s+\`?\s*verify",  # run `scan` then `verify`
]

for file_path in all_surface_files["prompts"]:
    content = read_file(file_path)
    if content is None:
        continue
    fname = file_path.name

    # Check for correct ordering pattern (only for core files)
    has_correct_order = any(
        re.search(p, content, re.IGNORECASE) for p in KNOWLEDGE_ORDER_PATTERNS
    )
    # Also check flexible patterns
    has_flexible_order = any(
        re.search(p, content, re.IGNORECASE) for p in FLEXIBLE_ORDER_PATTERNS
    )

    # Check for wrong ordering (verify → scan)
    has_wrong_order = bool(re.search(r"verify\s*[→\->]+\s*scan", content, re.IGNORECASE))

    if fname in CORE_KNOWLEDGE_FILES:
        # Core files: FAIL if missing correct ordering
        if (has_correct_order or has_flexible_order) and not has_wrong_order:
            inc_pass(f"{fname}: scan→verify→import ordering correct")
        elif has_wrong_order:
            inc_fail(f"{fname}: verify→scan wrong ordering found")
        else:
            inc_fail(f"{fname}: missing explicit scan→verify→import ordering")
    else:
        # Non-core files: skip ordering check entirely
        pass

    # Only check with_knowledge/knowledge_approved for files that have with_knowledge
    has_with_knowledge = literal_has(content, "with_knowledge")
    has_knowledge_approved = literal_has(content, "knowledge_approved")
    # "reviewed plan" or "plan is reviewed" or "plan was reviewed"
    has_reviewed_plan = bool(re.search(
        r"reviewed\s+plan|plan\s+is\s+reviewed|plan\s+was\s+reviewed",
        content, re.IGNORECASE
    ))

    if has_with_knowledge:
        inc_pass(f"{fname}: contains with_knowledge")
        if has_knowledge_approved:
            inc_pass(f"{fname}: contains knowledge_approved")
        else:
            inc_fail(f"{fname}: missing knowledge_approved")

        if has_reviewed_plan:
            inc_pass(f"{fname}: contains reviewed plan")
            # Import conditioned to BOTH reviewed plan AND knowledge_approved
            # Flexible matching: "reviewed plan" OR "plan is reviewed"
            has_knowledge_approved_cond = bool(re.search(
                r"import.*(?:reviewed\s+plan|plan\s+is\s+reviewed).*knowledge_approved|"
                r"knowledge_approved.*import.*(?:reviewed\s+plan|plan\s+is\s+reviewed)|"
                r"import\s+only\s+when.*knowledge_approved",
                content, re.IGNORECASE
            ))
            if has_knowledge_approved_cond:
                inc_pass(f"{fname}: import conditioned to both reviewed plan and knowledge_approved")
            else:
                inc_fail(f"{fname}: import not properly conditioned")
        else:
            inc_fail(f"{fname}: missing reviewed plan")
    # Files without with_knowledge: skip all knowledge checks

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
# REGRESSION I: Propose/debt require sddk artifact store
# ------------------------------------------------------------
banner("REGRESSION I: Propose/debt require sddk artifact store, no sddk cycle transition")

# Only check agent files (sddk-propose.md, sddk-debt-verify.md), not phase prompt files
PROPOSE_DEBT_FILES = [
    f for f in all_files
    if f.name in ["sddk-propose.md", "sddk-debt-verify.md"]
]

for file_path in PROPOSE_DEBT_FILES:
    content = read_file(file_path)
    if content is None:
        continue
    fname = file_path.name

    has_artifact_store = bool(re.search(r"sddk\s+artifact\s+store", content, re.IGNORECASE))
    has_cycle_transition = bool(re.search(r"sddk\s+cycle\s+transition", content, re.IGNORECASE))

    if has_artifact_store:
        inc_pass(f"{fname}: contains sddk artifact store")
    else:
        inc_fail(f"{fname}: missing sddk artifact store")

    if has_cycle_transition:
        inc_fail(f"{fname}: contains sddk cycle transition (prohibited)")
    else:
        inc_pass(f"{fname}: no sddk cycle transition")

# ------------------------------------------------------------
# REGRESSION J: verify CLI contract matches path-scoped workflow
# ------------------------------------------------------------
banner("REGRESSION J: verify CLI contract matches path-scoped workflow")

verify_skill_path = SDDK_ROOT / "skills/sddk-verify/SKILL.md"
verify_skill = read_file(verify_skill_path) or ""
verify_requirements = {
    "status includes cycle": "sddk cycle status --root . --scope . --cycle {cycle_id}" in verify_skill,
    "A-full transition": "phase.verify.complete`" in verify_skill,
    "A-min transition": "phase.verify.complete.a-min" in verify_skill,
    "A-lite transition": "phase.verify.complete.a-lite" in verify_skill,
    "B-direct transition": "phase.verify.complete.b-direct" in verify_skill,
    "failed gate outcome": "otherwise use `failed`" in verify_skill,
    "failed transition state": "status=REMEDIATING" in verify_skill,
    "conditional lease flags": "when `lease` is null, omit both flags" in verify_skill,
}

for description, present in verify_requirements.items():
    if present:
        inc_pass(f"sddk-verify: {description}")
    else:
        inc_fail(f"sddk-verify: missing {description}")

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
