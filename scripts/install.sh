#!/usr/bin/env bash
# install.sh — Install the sddk binary from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Rubentxu/sddk-framework/main/scripts/install.sh | bash
#   bash install.sh                          # latest, ~/.local/bin
#   bash install.sh --version v1.0.0        # pinned release
#   bash install.sh --prefix /usr/local/bin  # custom prefix
#   bash install.sh --framework              # also clone the framework repo and
#                                            # link agents/skills/prompts into
#                                            # opencode (and zcode if present)
#
# The binary is verified against its published sha256 before installation.
# Environment overrides: SDDK_REPO, SDDK_VERSION, SDDK_PREFIX,
# SDDK_WITH_FRAMEWORK, SDDK_ASSET, SDDK_BASE_URL (testing).

set -euo pipefail

REPO="${SDDK_REPO:-Rubentxu/sddk-framework}"
VERSION="${SDDK_VERSION:-latest}"
PREFIX="${SDDK_PREFIX:-$HOME/.local/bin}"
WITH_FRAMEWORK="${SDDK_WITH_FRAMEWORK:-0}"
BASE_URL="${SDDK_BASE_URL:-https://github.com/$REPO/releases}"
FRAMEWORK_DIR="${SDDK_SHARED_DIR:-$HOME/.sddk-shared}"

while [ $# -gt 0 ]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --prefix) PREFIX="$2"; shift 2 ;;
        --framework) WITH_FRAMEWORK=1; shift ;;
        *) echo "unknown option: $1" >&2; exit 2 ;;
    esac
done

detect_asset() {
    local os arch
    case "$(uname -s)" in
        Linux*) os=linux ;;
        Darwin*) os=darwin ;;
        *) echo "unsupported OS: $(uname -s)" >&2; exit 1 ;;
    esac
    case "$(uname -m)" in
        x86_64|amd64) arch=x86_64 ;;
        arm64|aarch64) arch=aarch64 ;;
        *) echo "unsupported architecture: $(uname -m)" >&2; exit 1 ;;
    esac
    # musl Linux is the portable static build; glibc otherwise.
    if [ "$os" = "linux" ] && ldd --version 2>/dev/null | grep -qi musl; then
        echo "sddk-${os}-${arch}-musl"
    else
        echo "sddk-${os}-${arch}"
    fi
}

ASSET="${SDDK_ASSET:-$(detect_asset)}"
echo "sddk installer"
echo "  repo:      $REPO"
echo "  version:   $VERSION"
echo "  asset:     $ASSET"
echo "  prefix:    $PREFIX"

TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

download() {
    local url="$1" out="$2"
    echo "  downloading: $url"
    case "$url" in
        file://*)
            cp "${url#file://}" "$out"
            ;;
        *)
            if command -v curl >/dev/null 2>&1; then
                curl -fsSL --retry 3 -o "$out" "$url"
            elif command -v wget >/dev/null 2>&1; then
                wget -qO "$out" "$url"
            else
                echo "error: need curl or wget" >&2
                exit 1
            fi
            ;;
    esac
}

if [ "$VERSION" = "latest" ]; then
    BIN_URL="$BASE_URL/latest/download/$ASSET"
    SUM_URL="$BASE_URL/latest/download/$ASSET.sha256"
else
    BIN_URL="$BASE_URL/download/$VERSION/$ASSET"
    SUM_URL="$BASE_URL/download/$VERSION/$ASSET.sha256"
fi

download "$BIN_URL" "$TMP_DIR/sddk"
download "$SUM_URL" "$TMP_DIR/sddk.sha256"

EXPECTED="$(awk '{print $1}' "$TMP_DIR/sddk.sha256")"
ACTUAL="$(sha256sum "$TMP_DIR/sddk" | awk '{print $1}')"
if [ "$EXPECTED" != "$ACTUAL" ]; then
    echo "error: sha256 mismatch" >&2
    echo "  expected: $EXPECTED" >&2
    echo "  actual:   $ACTUAL" >&2
    exit 1
fi
echo "  sha256 verified: $ACTUAL"

mkdir -p "$PREFIX"
install -m 0755 "$TMP_DIR/sddk" "$PREFIX/sddk"
echo "  installed: $PREFIX/sddk"
"$PREFIX/sddk" --version

if [ "$WITH_FRAMEWORK" = "1" ]; then
    echo "framework: cloning $REPO into $FRAMEWORK_DIR"
    if [ -d "$FRAMEWORK_DIR/.git" ]; then
        git -C "$FRAMEWORK_DIR" fetch --tags --quiet
        if [ "$VERSION" != "latest" ]; then
            git -C "$FRAMEWORK_DIR" checkout "$VERSION" --quiet
        else
            git -C "$FRAMEWORK_DIR" pull --ff-only --quiet
        fi
    else
        git clone --quiet --depth 1 "$(printf 'https://github.com/%s.git' "$REPO")" "$FRAMEWORK_DIR"
        if [ "$VERSION" != "latest" ] && [ "$VERSION" != "main" ]; then
            git -C "$FRAMEWORK_DIR" fetch --quiet --depth 1 origin "refs/tags/$VERSION:refs/tags/$VERSION"
            git -C "$FRAMEWORK_DIR" checkout --quiet "$VERSION"
        fi
    fi
    echo "framework: linking agents/skills/prompts into editors"
    "$PREFIX/sddk" dev link --root "$FRAMEWORK_DIR" --editor opencode
fi

echo
echo "Done. Run 'sddk --help' to get started."
