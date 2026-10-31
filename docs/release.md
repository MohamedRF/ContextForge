# Development packaging and release checklist

There is no published ContextForge release. Crates.io publication is disabled;
package name availability and a multi-crate publishing plan remain unverified.
Use `cargo install --path crates/contextforge-cli --locked` for a source install.

## Local package preparation

Build and test first, then use Python 3.11+ to package a native binary:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
python scripts/package.py --binary target/release/contextforge.exe --target x86_64-pc-windows-gnu --output target/packages
```

Use your actual native target (`rustc -vV` reports the host), and the executable
without `.exe` on Unix. Cross-compiled artifacts require a matching native runner
for smoke testing. Output directories must not already contain the named artifact.
The script creates a versioned ZIP, LICENSE, changelog, installation note,
BUILD.json with binary fingerprint, and a SHA-256 sidecar. ZIP entries have fixed
timestamps and permissions; this does not claim reproducible compiler output.
It verifies the extracted copy's version, init/index/doctor/search/setup and
source integrity before writing the archive. It never publishes or changes PATH.

Verify the SHA-256 before installing. Extract into a private directory, restore
Unix execute permission if your unzip tool omits it, then run by full path.
Generate setup previews from this stable location. Native libraries/glibc and OS
compatibility still require testing; neither static linking nor portable Linux
compatibility is claimed. A temporary-directory smoke on the build host is not
a clean-machine install.

`.github/workflows/package.yml` is a manually dispatched preparation workflow
with read-only repository permissions. It runs quality gates, builds/tests each
runner's native package and uploads development artifacts with checksums. It has
no release/tag/publish step. The new workflow has been linted locally but has
**not run on GitHub**. Windows GNU packaging was exercised locally; Linux,
Linux ARM64, macOS ARM64 and Intel packaging remain unverified. The initial
workflow covers the three default hosted OS runners; ARM Linux and Intel macOS
need dedicated tested runner entries before claiming those targets.

## Release gates

- [ ] Resolve remaining Phase 8/10 privacy, containment, cache and process-quota limits.
- [ ] Run current-source quality gates on every claimed OS/architecture.
- [ ] Verify real tool calls in at least Claude Code and Codex; record versions.
- [ ] Run the package workflow and validate native architecture/ABI and checksums.
- [ ] Install each artifact on a clean matching machine and invoke an MCP tool.
- [ ] Review current dependency advisories and licenses; record accepted findings.
- [ ] Decide whether to publish crates; check registry names and path dependency versions.
- [ ] Review exact version, changelog, migration notes, benchmark scope and security docs.
- [ ] Obtain owner authorization for push/tag/publication and publish only reviewed artifacts.

## Upgrade and rollback

Keep the previous binary and harness configuration backup. Stop MCP processes
before replacing the executable, especially on Windows. Back up the private
`.contextforge` cache before a new version upgrades it; do not share that backup.
Schema 1→2 is additive during explicit indexing; older binaries can reject a newer
schema. For rollback, restore the compatible private cache backup or configure a
fresh cache and reindex sources with the older binary. Do not downgrade schema
metadata manually. Restart the harness and verify a cited search result.

The unreleased changelog is the development release note. No stable-release,
upgrade-compatibility or signed-binary guarantee is currently made.
