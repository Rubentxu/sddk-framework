#!/usr/bin/env bash
# bootstrap.sh — Install the SDDK framework into detected AI coding editors.
#
# Usage:
#   ./bootstrap.sh                    # auto-detect editors, create symlinks
#   ./bootstrap.sh --zcode            # only ZCode
#   ./bootstrap.sh --opencode         # only OpenCode
#   ./bootstrap.sh --all              # all detected + force re-link
#
# This script makes ~/.sddk-shared/ the single source of truth and symlinks
# agents/skills/prompts into each editor's expected directory.

set -euo pipefail

SHARED_DIR="${SDDK_SHARED_DIR:-$(cd "$(dirname "$0")" && pwd)}"
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
    for f in "$SHARED_DIR"/agents/*.md; do
        name=$(basename "$f")
        target="$ZCODE_DIR/agents/$name"
        ln -sf "$f" "$target"
    done
    info "Linked $(ls "$ZCODE_DIR/agents"/*.md | wc -l) agents"

    info "Linking ZCode skills..."
    mkdir -p "$ZCODE_DIR/skills"
    for d in "$SHARED_DIR"/skills/*/; do
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
    for d in "$SHARED_DIR"/skills/*/; do
        name=$(basename "$d")
        target="$OPENCODE_DIR/skills/$name"
        ln -sfn "$d" "$target"
    done
    info "Linked $(ls -d "$OPENCODE_DIR/skills"/*/ 2>/dev/null | wc -l) skills"

    info "Linking OpenCode prompts (sdd-kernel)..."
    mkdir -p "$OPENCODE_DIR/prompts/sdd-kernel"
    # Link phase specs and docs
    for f in "$SHARED_DIR"/prompts/sdd-kernel/*.md; do
        name=$(basename "$f")
        target="$OPENCODE_DIR/prompts/sdd-kernel/$name"
        ln -sf "$f" "$target"
    done
    # Link phase specs subdirectory
    mkdir -p "$OPENCODE_DIR/prompts/sdd-kernel/phases"
    for f in "$SHARED_DIR"/prompts/sdd-kernel/phases/*.md; do
        name=$(basename "$f")
        target="$OPENCODE_DIR/prompts/sdd-kernel/phases/$name"
        ln -sf "$f" "$target"
    done
    # Link templates subdirectory
    if [ -d "$SHARED_DIR/prompts/sdd-kernel/templates" ]; then
        mkdir -p "$OPENCODE_DIR/prompts/sdd-kernel/templates"
        for f in "$SHARED_DIR"/prompts/sdd-kernel/templates/*; do
            name=$(basename "$f")
            target="$OPENCODE_DIR/prompts/sdd-kernel/templates/$name"
            ln -sf "$f" "$target"
        done
    fi
    info "Linked sdd-kernel prompts"

    warn "OpenCode agents are registered in opencode.json with {file:...} paths."
    warn "If opencode.json references old paths, update them to point to $SHARED_DIR/agents/"
    warn "Run: grep -r 'prompts/debt-verify\|prompts/sdd-kernel/phases' $OPENCODE_DIR/opencode.json"
}

# --- Knowledge vault setup ---

setup_knowledge_base() {
    info "Knowledge graph template is at: $SHARED_DIR/knowledge-template/"
    info "Per-project vaults will be created at: {project}/~/.sddk-knowledge/{project}/ (inside repo)"
    info "  (auto-created on first SDDK cycle per project)"
}

# --- Main ---

main() {
    echo "🔍 SDDK Framework Bootstrap"
    echo "   Shared dir: $SHARED_DIR"
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
    echo "  1. For OpenCode: verify opencode.json agent paths point to $SHARED_DIR/agents/"
    echo "  2. Start an SDDK cycle: /sddk-new <change-name> in your project"
    echo "  3. The knowledge vault will auto-initialize on first cycle"
    echo ""
    echo "To verify symlinks:"
    echo "  ls -la ~/.zcode/agents/orchestrator.md"
    echo "  ls -la ~/.config/opencode/skills/knowledge-graph/"
}

main "$@"
