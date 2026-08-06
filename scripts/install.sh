#!/usr/bin/env bash
# install.sh — Install the sddk binary and framework from GitHub Releases.
#
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Rubentxu/sddk-framework/main/scripts/install.sh | bash
#   bash install.sh                          # interactive: asks which editor to configure
#   bash install.sh --editor opencode       # non-interactive: configure OpenCode only
#   bash install.sh --editor zcode          # non-interactive: configure ZCode only
#   bash install.sh --editor all            # non-interactive: configure both
#   bash install.sh --editor none           # binary only, skip framework
#   bash install.sh --version v1.0.0        # pinned release
#   bash install.sh --prefix /usr/local/bin  # custom prefix
#
# The binary AND the framework bundle are verified against their published
# sha256 before installation. When `cosign` is available, signatures are
# verified keyless (sigstore) as an additional authenticity check.
# No git required.
#
# Environment overrides:
#   SDDK_REPO, SDDK_VERSION, SDDK_PREFIX, SDDK_FRAMEWORK_DIR, SDDK_EDITOR,
#   SDDK_ASSET, SDDK_BASE_URL (testing).

set -euo pipefail

REPO="${SDDK_REPO:-Rubentxu/sddk-framework}"
VERSION="${SDDK_VERSION:-latest}"
PREFIX="${SDDK_PREFIX:-$HOME/.local/bin}"
FRAMEWORK_DIR="${SDDK_FRAMEWORK_DIR:-$HOME/.sddk-shared/framework}"
EDITOR="${SDDK_EDITOR:-}"
BASE_URL="${SDDK_BASE_URL:-https://github.com/$REPO/releases}"

# Backwards compatibility: --framework used to mean "also clone and link".
# Framework setup is now the default interactive path; accept the flag as a no-op.
while [ $# -gt 0 ]; do
    case "$1" in
        --version) VERSION="$2"; shift 2 ;;
        --prefix) PREFIX="$2"; shift 2 ;;
        --editor) EDITOR="$2"; shift 2 ;;
        --framework) shift ;;
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
    # musl is the portable STATIC build: it runs on ANY Linux distribution
    # regardless of glibc version (the glibc build compiled on ubuntu-24.04
    # requires GLIBC >= 2.39, which excludes Debian 12, Ubuntu <= 23.10,
    # CentOS 9, etc.). ALL Linux targets are standalone: x86_64 AND aarch64.
    # macOS keeps the native build (libSystem is mandatory there — even
    # Go/Zig binaries cannot be fully static on macOS).
    if [ "$os" = "linux" ]; then
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

release_url() {
    local name="$1"
    if [ "$VERSION" = "latest" ]; then
        echo "$BASE_URL/latest/download/$name"
    else
        echo "$BASE_URL/download/$VERSION/$name"
    fi
}

# Verify a downloaded file against its published sha256.
verify_sha256() {
    local file="$1" sum="$2"
    local expected actual
    expected="$(awk '{print $1}' "$sum")"
    actual="$(sha256sum "$file" | awk '{print $1}')"
    if [ "$expected" != "$actual" ]; then
        echo "error: sha256 mismatch" >&2
        echo "  expected: $expected" >&2
        echo "  actual:   $actual" >&2
        exit 1
    fi
    echo "  sha256 verified: $actual"
}

# Verify a downloaded file keyless via cosign when available (best effort).
verify_signature() {
    local file="$1" sig="$2" pem="$3"
    if ! command -v cosign >/dev/null 2>&1; then
        echo "  (cosign not installed: skipping signature check)"
        return 0
    fi
    if [ ! -f "$sig" ] || [ ! -f "$pem" ]; then
        echo "  (signature assets not published: skipping signature check)"
        return 0
    fi
    if cosign verify-blob \
        --certificate-identity-regexp "https://github.com/$REPO/.github/workflows/release.yml@refs/tags/v.*" \
        --certificate-oidc-issuer "https://token.actions.githubusercontent.com" \
        --signature "$sig" --certificate "$pem" "$file" >/dev/null 2>&1; then
        echo "  signature verified (cosign keyless)"
    else
        echo "  warning: cosign signature verification failed (sha256 already verified)"
    fi
}

# --- 1. Binary ---

download "$(release_url "$ASSET")" "$TMP_DIR/sddk"
download "$(release_url "$ASSET.sha256")" "$TMP_DIR/sddk.sha256"
verify_sha256 "$TMP_DIR/sddk" "$TMP_DIR/sddk.sha256"
if command -v cosign >/dev/null 2>&1; then
    download "$(release_url "$ASSET.sig")" "$TMP_DIR/sddk.sig" 2>/dev/null || true
    download "$(release_url "$ASSET.pem")" "$TMP_DIR/sddk.pem" 2>/dev/null || true
    verify_signature "$TMP_DIR/sddk" "$TMP_DIR/sddk.sig" "$TMP_DIR/sddk.pem"
fi

mkdir -p "$PREFIX"
install -m 0755 "$TMP_DIR/sddk" "$PREFIX/sddk"
echo "  installed: $PREFIX/sddk"
"$PREFIX/sddk" --version

# --- PATH check ---

case ":$PATH:" in
    *":$PREFIX:"*)
        echo "  PATH: ok ($PREFIX already on PATH)"
        ;;
    *)
        echo "  WARNING: $PREFIX is not on your PATH. Add it with:"
        echo "    export PATH=\"$PREFIX:\$PATH\""
        ;;
esac

# --- 2. Ask which editor to configure ---

if [ -z "$EDITOR" ]; then
    if [ -t 0 ] || [ -e /dev/tty ]; then
        echo
        echo "¿Querés configurar el framework SDDK en un editor de IA?"
        echo "  1) OpenCode"
        echo "  2) ZCode"
        echo "  3) Ambos"
        echo "  4) Ninguno (solo binario)"
        # shellcheck disable=SC2162
        read -rp "Elección [3]: " choice < /dev/tty 2>/dev/null || choice="3"
        case "${choice:-3}" in
            1) EDITOR=opencode ;;
            2) EDITOR=zcode ;;
            3) EDITOR=all ;;
            4) EDITOR=none ;;
            *) echo "opción inválida: $choice" >&2; exit 2 ;;
        esac
    else
        echo "  (no TTY: using --editor all; pass --editor none for binary only)"
        EDITOR=all
    fi
fi

if [ "$EDITOR" = "none" ]; then
    echo
    echo "Framework no configurado. Cuando quieras:"
    echo "  sddk dev link --root <framework-dir> --editor opencode|zcode|all"
    echo "  sddk dev update --root <framework-dir>   # re-descarga el bundle"
    echo "Done. Run 'sddk --help' to get started."
    exit 0
fi

# --- 3. Framework bundle ---

if [ -d "$FRAMEWORK_DIR/.git" ]; then
    echo
    echo "framework: existing git checkout detected at $FRAMEWORK_DIR (using as-is)"
else
    download "$(release_url "sddk-framework.tar.gz")" "$TMP_DIR/sddk-framework.tar.gz"
    download "$(release_url "sddk-framework.tar.gz.sha256")" "$TMP_DIR/sddk-framework.sha256"
    verify_sha256 "$TMP_DIR/sddk-framework.tar.gz" "$TMP_DIR/sddk-framework.sha256"
    if command -v cosign >/dev/null 2>&1; then
        download "$(release_url "sddk-framework.tar.gz.sig")" "$TMP_DIR/sddk-framework.sig" 2>/dev/null || true
        download "$(release_url "sddk-framework.tar.gz.pem")" "$TMP_DIR/sddk-framework.pem" 2>/dev/null || true
        verify_signature "$TMP_DIR/sddk-framework.tar.gz" "$TMP_DIR/sddk-framework.sig" "$TMP_DIR/sddk-framework.pem"
    fi

    mkdir -p "$FRAMEWORK_DIR"
    tar xzf "$TMP_DIR/sddk-framework.tar.gz" -C "$FRAMEWORK_DIR"
    echo "  framework extracted: $FRAMEWORK_DIR"
fi

# --- 4. Link into the chosen editor(s) ---

echo
"$PREFIX/sddk" dev link --root "$FRAMEWORK_DIR" --editor "$EDITOR" --format text

# --- 5. Doctor ---

echo
"$PREFIX/sddk" dev doctor --format text || true

# --- 6. Completions hint ---

echo
echo "Shell completions (optional):"
echo "  bash:    source <(sddk completion bash)"
echo "  zsh:     echo 'source <(sddk completion zsh)' >> ~/.zshrc"
echo "  fish:    sddk completion fish > ~/.config/fish/completions/sddk.fish"
echo
echo "Done. Run 'sddk --help' to get started."
