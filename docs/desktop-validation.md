# Desktop preview validation — 31 October 2026

This is a developer prerelease. The complete production acceptance checklist in
TASKS.md remains open; successful compilation is not a substitute for native GUI,
clean-machine installation, signing or real coding-harness session validation.

## Checks performed locally

- Rust formatting, workspace Clippy with warnings denied, and the full workspace
  test suite passed after the final Git changes. These include real CLI/MCP client
  integration, cross-process indexing locks, cancellation rollback, registry
  persistence, configuration merge/backup and schema compatibility tests.
- Seven Git impact tests passed, including new CRLF checkout and explicit `-text`
  regressions. Unstaged ranges now use libgit2's conversion policy without changing
  captured source bytes; raced index/source snapshots are rejected.
- Frontend formatting, type checking, lint, all five tests and production build
  passed. `npm audit` reported zero vulnerabilities.
- Workflow syntax (`actionlint`), Python packaging syntax and local documentation
  links passed. macOS app ZIP packaging is validated on macOS runners using `ditto`.
- The Rust advisory snapshot dated 30 October 2026 reported no vulnerability
  entries and five transitive warning entries: proc-macro-error, zbus_names,
  zvariant, zvariant_derive and glib. This is not a claim of an advisory-free tree.

## Native Windows development checks

The optimized GUI launched from both the build output and a per-user NSIS
installation. Installation returned exit code 0. The native folder dialog
registered a disposable repository whose path contains Unicode. The standalone
CLI indexed the same repository; the installed desktop restored its registration
and displayed the actual three-file index and version.

Ctrl+K search returned cited exact-symbol and full-text results from that index.
Opening a result displayed current source, line numbers and symbol outlines.
Dependency traversal returned four real fixture nodes and two heuristic relation
records with source evidence. Screenshots in `docs/images` are captures of those
running native views from the development build, before the final styling fix.

Git impact UI testing exposed CRLF-only false positives. The engine correction
and its regressions were verified through Rust tests; a post-fix GUI retest was
not performed. Sidebar/style corrections passed frontend checks. Computer Use
was stopped by the user; no additional GUI workflow is claimed as verified.

The extracted Windows GNU CLI ZIP also passed the packaging script's independent
version/init/index/doctor/search/setup smoke tests and source-integrity checks.
Release CLI ZIPs undergo the same smoke checks on their own native CI runners.

## Release and remaining checks

The release links its exact commit and native artifact workflow. That workflow
runs Rust/frontend checks and builds Windows x64 NSIS, macOS ARM64 app/DMG and
macOS Intel app/DMG, plus independently usable CLI/MCP ZIPs and SHA-256 sidecars.
Only successful native artifact jobs are eligible for publication.

No Authenticode/Developer ID credentials, notarization or updater endpoint are
configured. macOS GUI launch and installation were not tested. Clean-machine
Windows installation, uninstallation/data preservation, remaining GUI workflows
(context building, integration apply/remove and cancellation), and real Claude
Code/Codex sessions remain unverified. Reproduce them using the workflows in
[the desktop guide](desktop-user-guide.md), retaining configuration/cache backups.
