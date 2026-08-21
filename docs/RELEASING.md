# Release & Distribution

From v1.28.0 SDDK distributes **pre-compiled binaries** via GitHub Releases,
no cloning or compilation required. Users install with a one-liner
(`rustup` / `mise` model):

```bash
curl -fsSL https://raw.githubusercontent.com/Rubentxu/sddk-framework/main/scripts/install.sh | bash
```

The `scripts/install.sh` script (244 lines):
- Detects platform (`uname -s/m`) → asset `sddk-linux-{x86_64,aarch64}-musl`
  (Linux: **musl static**, runs on any distro regardless of glibc)
- Downloads binary + `sha256` from GitHub Releases
- Verifies SHA256 before installing (fails if mismatch)
- If `cosign` is available, verifies keyless signature (optional)
- Prompts which editor to configure (opencode/zcode/claude/codex or all)
- Downloads `sddk-framework.tar.gz` (bundle: `agents/`, `skills/`,
  `prompts/sddk/`, `assets/`, `MANIFEST.sha256`) and extracts it to
  `$SDDK_DATA_DIR/framework/<v>/`
- Runs `sddk dev link --editor <X>` (symlinks bundle to editor dir)
- Prints `sddk dev doctor` (final verification)

**Supported platforms in v1.28.0:**
- ✅ Linux x86_64 (musl static)
- ✅ Linux aarch64 (musl static)
- ⏳ macOS x86_64 + arm64 (pending: `cargo-zigbuild` toolchain already installed;
  need to generate binaries and upload to release)
- ⏳ Windows x86_64 (pending: requires `#[cfg(unix)]` carve-out in code using
  `std::os::unix::*` — see `crates/sddk-cli/src/dev_cmd.rs`)

**Local-first release (manual):** tag is pushed first (`git tag vX.Y.Z &&
git push origin vX.Y.Z`), then the binary is uploaded to GitHub Releases.
Workflow `.github/workflows/release.yml` is in `workflow_dispatch` manual mode
since 2026-08-10 (CI exhausted); today's operational path is:

```bash
# 1. Tag + push (local)
cargo build --release --target x86_64-unknown-linux-musl -p sddk-cli --locked
git tag vX.Y.Z && git push origin vX.Y.Z

# 2. Stage assets (Linux x86_64 + aarch64)
./target/x86_64-unknown-linux-musl/release/sddk release dist \
  --prefix dist-amd64 --channel release --commit "$(git rev-parse HEAD)"
cp dist-amd64/dist/sddk sddk-linux-x86_64-musl
cp dist-amd64/dist/{checksums.txt,sbom.json,attestation.json} sddk-linux-x86_64-musl.{CHECKSUMS,sbom.json,attestation.json}
sha256sum sddk-linux-x86_64-musl > sddk-linux-x86_64-musl.sha256
# (repeat for aarch64)

# 3. Framework bundle
tar czf sddk-framework.tar.gz agents skills prompts/sddk assets MANIFEST.sha256
sha256sum sddk-framework.tar.gz > sddk-framework.tar.gz.sha256

# 4. gh release create
gh release create vX.Y.Z --repo Rubentxu/sddk-framework \
  --target <commit> --title "vX.Y.Z" --notes "..." \
  sddk-linux-x86_64-musl sddk-linux-x86_64-musl.{sha256,CHECKSUMS,sbom.json,attestation.json} \
  sddk-linux-aarch64-musl sddk-linux-aarch64-musl.{sha256,CHECKSUMS,sbom.json,attestation.json} \
  sddk-framework.tar.gz sddk-framework.tar.gz.sha256
```

The E2E smoke test lives in `.github/workflows/release.yml:170-217` and runs
automatically when CI is available.
