#!/usr/bin/env bash
# bootstrap.sh — Install the SDDK framework into detected AI coding editors.
#
# Usage:
#   ./bootstrap.sh                    # auto-detect editors, create symlinks
#   ./bootstrap.sh --zcode            # only ZCode
#   ./bootstrap.sh --opencode         # only OpenCode
#   ./bootstrap.sh --all              # all detected + force re-link
#
# This script symlinks agents/skills/prompts from the framework root
# (default: the dir containing this script = the CWD repo) into each
# editor's expected directory (~/.config/opencode, ~/.zcode, ...).

set -euo pipefail

SDDK_FRAMEWORK_ROOT="${SDDK_FRAMEWORK_ROOT:-$(cd "$(dirname "$0")" && pwd)}"
ZCODE_DIR="${ZCODE_DIR:-$HOME/.zcode}"
OPENCODE_DIR="${OPENCODE_DIR:-$HOME/.config/opencode}"

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

info()  { echo -e "${GREEN}✅ $1${NC}"; }
warn()  { echo -e "${YELLOW}⚠️  $1${NC}"; }
error() { echo -e "${RED}❌ $1${NC}"; }

# --- Detect editors ---

detect_editors() {
    local editors=()
    [ -d "$ZCODE_DIR/agents" ] && editors+=("zcode")
    [ -d "$OPENCODE_DIR" ] && editors+=("opencode")
    echo "${editors[@]}"
}

# --- ZCode linking ---

link_zcode() {
    info "Linking ZCode agents..."
    mkdir -p "$ZCODE_DIR/agents"
    for f in "$SDDK_FRAMEWORK_ROOT"/agents/*.md; do
        name=$(basename "$f")
        target="$ZCODE_DIR/agents/$name"
        ln -sf "$f" "$target"
    done
    info "Linked $(ls "$ZCODE_DIR/agents"/*.md | wc -l) agents"

    info "Linking ZCode skills..."
    mkdir -p "$ZCODE_DIR/skills"
    for d in "$SDDK_FRAMEWORK_ROOT"/skills/*/; do
        name=$(basename "$d")
        target="$ZCODE_DIR/skills/$name"
        ln -sfn "$d" "$target"
    done
    info "Linked $(ls -d "$ZCODE_DIR/skills"/*/ | wc -l) skills"
}

# --- OpenCode linking ---

link_opencode() {
    info "Linking OpenCode skills..."
    mkdir -p "$OPENCODE_DIR/skills"
    for d in "$SDDK_FRAMEWORK_ROOT"/skills/*/; do
        name=$(basename "$d")
        target="$OPENCODE_DIR/skills/$name"
        ln -sfn "$d" "$target"
    done
    info "Linked $(ls -d "$OPENCODE_DIR/skills"/*/ 2>/dev/null | wc -l) skills"

    # Link BOOK-*.md top-level (where consumers expect them)
    for f in "$SDDK_FRAMEWORK_ROOT"/skills/BOOK-*.md; do
        [ -f "$f" ] || continue
        name=$(basename "$f")
        target="$OPENCODE_DIR/skills/$name"
        ln -sf "$f" "$target"
    done

    info "Linking OpenCode agents..."
    mkdir -p "$OPENCODE_DIR/agents"
    for f in "$SDDK_FRAMEWORK_ROOT"/agents/*.md; do
        name=$(basename "$f")
        target="$OPENCODE_DIR/agents/$name"
        ln -sf "$f" "$target"
    done
    info "Linked $(ls "$OPENCODE_DIR/agents"/*.md 2>/dev/null | wc -l) agents"

    info "Linking OpenCode prompts (sddk)..."
    mkdir -p "$OPENCODE_DIR/prompts/sddk"
    # Link phase specs and docs
    for f in "$SDDK_FRAMEWORK_ROOT"/prompts/sddk/*.md; do
        name=$(basename "$f")
        target="$OPENCODE_DIR/prompts/sddk/$name"
        ln -sf "$f" "$target"
    done
    # Link phase specs subdirectory
    mkdir -p "$OPENCODE_DIR/prompts/sddk/phases"
    for f in "$SDDK_FRAMEWORK_ROOT"/prompts/sddk/phases/*.md; do
        name=$(basename "$f")
        target="$OPENCODE_DIR/prompts/sddk/phases/$name"
        ln -sf "$f" "$target"
    done
    # Link templates subdirectory
    if [ -d "$SDDK_FRAMEWORK_ROOT/prompts/sddk/templates" ]; then
        mkdir -p "$OPENCODE_DIR/prompts/sddk/templates"
        for f in "$SDDK_FRAMEWORK_ROOT"/prompts/sddk/templates/*; do
            name=$(basename "$f")
            target="$OPENCODE_DIR/prompts/sddk/templates/$name"
            ln -sf "$f" "$target"
        done
    fi
    info "Linked sddk prompts"

    info "OpenCode agents linked to: $OPENCODE_DIR/agents/"
    info "Register agents in opencode.json with: {file: \"$SDDK_FRAMEWORK_ROOT/agents/<name>.md\"}"
}

# --- Knowledge vault setup ---

setup_knowledge_base() {
    info "Knowledge graph template is at: $SDDK_FRAMEWORK_ROOT/knowledge-template/"
    info "Per-project vaults will be created at: \$HOME/.sddk-knowledge/{project}/ (in user home, outside repo)"
    info "  (auto-created on first SDDK cycle per project)"
}

# --- Main ---

main() {
    echo "🔍 SDDK Framework Bootstrap"
    echo "   Framework root: $SDDK_FRAMEWORK_ROOT"
    echo ""

    local editors
    if [ "${1:-}" = "--all" ]; then
        editors="zcode opencode"
    elif [ "${1:-}" = "--zcode" ]; then
        editors="zcode"
    elif [ "${1:-}" = "--opencode" ]; then
        editors="opencode"
    else
        editors=$(detect_editors)
    fi

    if [ -z "$editors" ]; then
        error "No editors detected. Install ZCode (~/.zcode/) or OpenCode (~/.config/opencode/) first."
        exit 1
    fi

    info "Detected editors: $editors"
    echo ""

    for editor in $editors; do
        case "$editor" in
            zcode)    link_zcode ;;
            opencode) link_opencode ;;
        esac
        echo ""
    done

    setup_knowledge_base
    echo ""

    info "Bootstrap complete!"
    echo ""
    echo "Next steps:"
    echo "  1. Adopt a project: /sddk-adopt in your project directory"
    echo "  2. Initialize: /sddk-init (after adoption)"
    echo "  3. Start a cycle: /sddk-new <change-name>"
    echo ""
    echo "To verify symlinks:"
    echo "  ls -la ~/.zcode/agents/"
    echo "  ls -la ~/.config/opencode/agents/"
    echo "  ls -la ~/.config/opencode/skills/knowledge-graph/"
}

main "$@"
