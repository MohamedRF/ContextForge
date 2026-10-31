# Development progress

The former internal tracker was removed in commit 595235b. Existing engine work
and validation history remain in Git history and docs/development.md; no previous
phase numbering is reconstructed or replaced here.

## Desktop Development

- [x] Milestone A: shared facade, Tauri 2 window, typed IPC, real project/status UI.
- [ ] Milestone B: persistent registrations, native folder selection, indexing,
  progress, search and source outlines.
- [ ] Milestone C: dependencies, Git impact and context builder using existing engines.
- [ ] Milestone D: executable discovery, MCP configuration preview and validation.
- [ ] Milestone E: platform builds, security review, integration tests, documentation
  and GitHub prerelease. Signing requires distribution credentials.

Acceptance and validation evidence will be recorded below as work proceeds.

### Milestone A — verified on Windows, 31 October 2026

- Added `contextforge-service`, shared orchestration/registry/integration services,
  and the `apps/desktop` native adapter and React interface.
- Actual optimized Windows window launched. Native folder selection registered
  a Unicode-path repository; project/status results appeared in React.
- CLI indexed that same registered repository (3 source files) independently.
- Rust formatting, warning-free workspace Clippy and workspace tests passed.
- Frontend formatter, typecheck, lint, five tests and production build passed;
  npm audit reported zero vulnerabilities.
- Separate-process indexing-lock and cancellation-rollback tests passed.
- Windows GNU CLI ZIP was extracted and smoke-tested by `scripts/package.py`.
- After unlocking Windows, the NSIS installer completed with exit code 0. Its
  installed application restored the registered repository and displayed the
  CLI-created index (3 files). Native keyboard search, cited results, source and
  symbol navigation, and dependency traversal were exercised successfully.
- Git impact UI testing found a Windows CRLF false-positive case. The shared Git
  engine now uses libgit2's text-conversion policy, preserves raw source evidence
  and rejects changed snapshots. Both CRLF and explicit `-text` regressions pass.
- Actual Windows screenshots are stored under `docs/images`. Broader UI workflows,
  clean-machine installs, signing, notarization and production acceptance remain
  outstanding; the task tracker deliberately does not mark those milestones done.

### Desktop preview publication — verified 31 October 2026

- Published [`v0.1.0-desktop-preview.1`](https://github.com/MohamedRF/ContextForge/releases/tag/v0.1.0-desktop-preview.1)
  from `cdc1ad8` as an unsigned GitHub prerelease with 16 assets: Windows x64
  NSIS installer and CLI ZIP, plus macOS ARM64/Intel app ZIP, DMG and CLI ZIP,
  each with a SHA-256 sidecar.
- Native artifact workflow 41174192293 (attempt 2, after an `hdiutil` "Resource
  busy" macOS ARM failure) and CI workflow 41174192276 completed successfully for
  the release commit. All three archives were downloaded from that run and passed
  checksum, PE/Mach-O architecture, bundle metadata and internal build-fingerprint
  validation before upload.
- The public release endpoint and remote tag were verified after publication.
  Signing, notarization, clean-machine installation, macOS GUI launch and real
  Claude/Codex sessions remain outstanding, so Milestone E is still open.

The next acceptance checks cover Milestones B–D in the native window, installer
lifecycle, and the corrected macOS/Windows artifact workflow. Advanced search
filters, full textual diff rendering, GUI editing of all repository settings,
automatic stale-index detection and richer operation history remain tracked work;
the first desktop release is explicitly a developer preview.
