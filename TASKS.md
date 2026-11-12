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

### Improvement batch — verified on Windows, 5 November 2026 (Milestones still open)

- `doctor` now includes a read-only freshness probe (`memory status` hashes)
  under `knowledge` in JSON output and a validated/stale summary in text
  output. `status` output is unchanged. A new CLI regression test verifies
  validated → stale → read-only behavior.
- `benchmarks/run.py` measures peak RSS and CPU time on Unix via
  `resource.getrusage(RUSAGE_CHILDREN)` (Linux KiB → bytes, macOS bytes),
  replacing null fields. `docs/benchmarking.md` updated. Windows 4-file
  smoke run passes; Unix execution not run on this host.
- Crate metadata completed for crates.io readiness (descriptions, keywords,
  categories, `authors`/`repository`/`homepage`) in all `crates/*/Cargo.toml`.
- Docs clarified: CLI `setup` is preview-only, desktop Integrations can back up
  and atomically apply/remove entries. `README.md` capability table, doctor
  description, freshness note, `docs/integrations.md`, `docs/known-limits.md`,
  and `CHANGELOG.md` updated; README now embeds `docs/images/` screenshots.
- Desktop `api.ts` mirrors Rust bounds client-side (`clampBudget`,
  `clampDepth`, `clampOffset`, `normalizeQuery`/`normalizeSelector`/
  `normalizePath`) and hardens `errorMessage()` for Tauri errors. New
  `api.test.ts` adds validation tests.

Checks performed on Windows x86_64 GNU (Rust 1.99.1, Node 24): `cargo fmt
--all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D
warnings`, `cargo test --workspace --locked` (84 tests), `cargo build
--workspace --release --locked`, `npm run typecheck`, `npm run lint`,
`npm run test` (9 tests), `npm run build`, `python -m py_compile`
benchmark/package scripts, and a 4-file 1-repeat benchmark smoke (exit 0,
usage fields present). Unavailable on this host: Unix/macOS benchmark
execution, macOS GUI launch, clean-machine installer test, signing/
notarization, and real Claude/Codex/Cursor/VS Code sessions. No milestone
checkbox is marked done; Milestones B–E remain open.

### MCP annotation fix — verified on Windows, 12 November 2026 (Milestones still open)

- All seven MCP tools now emit an explicit `idempotentHint` annotation
  (`true`) alongside the existing `readOnlyHint`/`destructiveHint`/
  `openWorldHint`. Handler tracing showed every tool is read-only with no
  side effects on repeat calls: context packing, Git impact, graph
  traversal, reference search, index search, symbol outlines and the
  project map all run on read-only SQLite connections, in-memory graph
  construction, libgit2 read-only access and read-only filesystem walks.
  A changing captured snapshot does not make a read non-idempotent.
- The stdio `tools/list` integration test
  (`stdout_is_protocol_only_and_eof_exits_cleanly`) now asserts explicit
  boolean `readOnlyHint`, `destructiveHint`, `idempotentHint` and
  `openWorldHint` values for all seven tools. Tool names, descriptions,
  schemas, handler behavior and other annotations are unchanged.

Checks performed on Windows x86_64 GNU (Rust 1.99.1, Node 24): `cargo fmt
--all -- --check`, `cargo clippy --workspace --all-targets --locked -- -D
warnings`, `cargo test --workspace --locked` (all pass, 4 MCP stdio tests),
`cargo build --workspace --release --locked`, `npm run typecheck`,
`npm run lint`, `npm run test` (9 tests) and `npm run build`. Not run:
macOS/Linux validation and the other M8ven findings (out of scope).
