# TASKS.md — ContextForge Development Tracker

> Implementation plan and evidence ledger for the ContextForge Rust + MCP code intelligence project.
> **Initial state:** All implementation tasks are unchecked because no project code or tests have been verified as part of this tracker.
> Update this file after every substantive change. Never mark a task complete based on intent, generated scaffolding, or unrun tests.

## Status and Working Rules

- `[ ]` Pending / not verified complete.
- `[x]` Complete **only after** the listed acceptance criteria are tested.
- For work in progress, add `IN PROGRESS` and a short note beside the item; keep checkbox unchecked.
- For blockers, add `BLOCKED — reason`, responsible action, and workaround if possible.
- Record exact commands and outcomes in the [Validation Log](#validation-log).
- Respect existing files, preserve user changes, and obtain approval before external writes, publishing, or destructive actions.
- Complete a **small end-to-end slice** first; do not generate all modules as empty placeholders.

## Project Outcomes

- **v0.1 (MVP):** Offline indexing → symbol search → stdio MCP tool call demonstrated in two harnesses or with verified integration plus clearly documented remaining verification.
- **v0.2:** Git change-impact, bounded dependency graph, token-budgeted context, persistent correctness.
- **v0.3:** Polished setup/integrations, release packaging, measurable performance, expansion if warranted.
- **v1.0:** Security hardening, migrations, cross-platform verification, reproducible benchmarks, stable documented behavior.

### Immediate next action

**Phases 7/9 and Phase 8 structural lifecycle verified locally on Windows (2026-10-30).** CLI context and MCP pack_context share bounded consistent-snapshot retrieval, explicit serialized token estimates, deduplication and graph evidence. Memory status/clear/rebuild work transactionally. All 65 tests, format, Clippy, release build and cargo-audit pass (175 dependencies, zero known vulnerabilities/warnings); release smoke verifies budgets, persistence and source/cache integrity. Phase 10 is IN PROGRESS: SQL resource limits, cancellable MCP waits and threat-model documentation implemented; hostile filesystem races, Windows cache hard links/permissions, strict peer/process quotas and cross-platform verification remain open. Literal Phase 8 secret-retention gate remains open because necessary full-source snapshots can contain embedded secrets. All pre-existing changes preserved.

---

## Phase 0 — Discovery and Design

**Goal:** Confirm architecture and scope without blocking delivery on speculative features.

- [x] Inspect existing files, `git status`, user changes, and toolchain availability.
- [x] Confirm product scope, core use cases, and MVP non-goals.
- [x] Verify current `rmcp`, Tokio, Clap, Tree-sitter, SQLite, and Git integration options from authoritative sources or installed docs.
- [x] Choose initial Rust toolchain/MSRV policy; record rationale.
- [x] Write `docs/architecture.md`: data flow, crate boundaries, trust boundaries, storage, CLI/MCP lifecycle.
- [x] Write `docs/roadmap.md`: MVP → v0.2 → v0.3 → v1.0.
- [x] Write `docs/development.md`: setup, build, test, lint, repository fixtures.
- [x] Create root license strategy (`MIT` or `Apache-2.0`, subject to owner choice), contribution/security document plan.
- [x] Record unresolved risks and decisions instead of silently guessing.

**Gate:** Architecture supports a working offline CLI + stdio MCP slice; dependency choices are verified; no critical unknown blocks Phase 1.

## Phase 1 — Workspace and CLI Foundation

**Goal:** A genuinely compiling executable with reliable commands and configuration handling.

- [x] Create Cargo workspace and minimal useful crate layout; commit `Cargo.lock` for the application. Lockfile retained for the application and included in the authorized repository publication scope.
- [x] Configure Rust toolchain, workspace lints, formatting, tracing, and error handling.
- [x] Implement `contextforge --help` and `--version`.
- [x] Implement `init`, `status`, `doctor`, and placeholder-free `mcp` entry point (a real minimal server, if included).
- [x] Implement repository/root detection and config loading with safe defaults.
- [x] Support configurable index root, file size limit, and ignore patterns. External-cache isolation and bounded inventory are tested.
- [x] Support diagnostics on stderr and structured machine output where appropriate.
- [x] Add CLI unit/integration tests (invalid args, missing repo, bad config, Unicode paths).
- [x] Run quality gates and fix failures. Final format check, Clippy, 22 behavior tests and release build pass; see Validation Log.

**Gate:** Commands documented as supported actually run; `cargo test --workspace` and release build succeed.

## Phase 2 — Discovery, Storage, and Incremental Index

**Goal:** Reliably discover repository files and persist a correct index without executing project code.

- [x] Implement root-scoped recursive file discovery and path safety policy. Canonical containment, traversal rejection, junction/link policy and scoped ignore loading; remaining hostile filesystem races documented.
- [x] Respect `.gitignore`, `.contextforgeignore`, excluded directories, and max-size restrictions. Verified in bounded inventory and index regression tests, including removal of newly ignored files.
- [x] Detect binary/non-UTF-8 input; define a documented handling policy. Inventory skips NUL and invalid UTF-8 content and non-UTF-8 paths; see architecture.
- [x] Create SQLite schema for repository, file, symbol, relation, index version and metadata records as needed. Initial schema 1 stored source snapshots; Phase 4 schema 2 adds actual symbols, outlines and FTS. Relations await their engine rather than adding placeholders.
- [x] Implement versioned migrations and transactional writes. Empty schema 0 → 1 → 2 and additive 1 → 2 backfill are atomic; unknown/future schemas and corruption are preserved and rejected.
- [x] Implement file fingerprints/hashes and deterministic index ordering. SHA-256 content/root identity and ordered discovery/SQL reads.
- [x] Implement initial indexing and user-facing progress/errors via stderr. CLI index has JSON/human summaries; errors are on stderr; no source-body diagnostics.
- [x] Implement incremental reindex of added/modified/deleted/renamed files. All eligible bytes are read/hashed; only changed database rows are written and changed files reparsed (schema upgrades backfill). No incremental source-reading or Tree-sitter tree-editing claim.
- [x] Invalidate dependent/stale records on changes. Changed/missing/renamed/newly ignored/binary file snapshots and their outlines, symbols and FTS records update/remove transactionally. Phase 5 derives relations from current captured outlines per request; no separate persistent relation cache exists.
- [x] Test unchanged reindex, single-file changes, deletes, renames, ignored files, interrupted updates, database upgrades. Includes concurrent writers, injected partial failure, abrupt child exit with uncommitted writes, schema rejection and corruption preservation.
- [x] Measure initial index baseline on a public fixture repo (record environment, no invented numbers). Reproducible generated 100-file source fixture: debug cold 115 ms, unchanged 103 ms, SQLite 28672 bytes; see docs/benchmarking.md. No private repository or downloaded corpus used.

**Gate:** Reindex only processes required changes, old records disappear, and repeated indexes yield consistent results.

## Phase 3 — Tree-sitter Parsing and Symbols

**Goal:** Correct source-backed structural outlines for the first four languages.

- [x] Define normalized symbol model, source spans, visibility, parent relationships, and language adapters.
- [x] Add and verify Rust Tree-sitter grammar and tests.
- [x] Add and verify Python grammar and tests.
- [x] Add and verify TypeScript grammar and tests.
- [x] Add and verify JavaScript grammar and tests.
- [x] Extract functions, methods, types, modules, imports/exports and calls where syntactically available.
- [x] Store line/byte offsets with documented indexing convention.
- [x] Handle syntax errors and partial parse results safely.
- [x] Make stable symbol identity/update semantics explicit.
- [x] Record limitations of syntax-only reference resolution.
- [x] Add fixture-based tests for nested constructs, Unicode, malformed syntax and changed files.

**Gate:** `analyze <file>` returns a reproducible outline with valid spans for each supported language.

## Phase 4 — Indexed Search and Ranking

**Goal:** Useful results without embeddings or a model API.

- [x] Implement exact filename/symbol/qualified-name lookup.
- [x] Implement SQLite FTS5 indexing and query escaping/validation.
- [x] Return repo-relative path, symbol, snippet, range, retrieval method and relevance signal.
- [x] Implement deterministic, explainable result ranking and result caps.
- [x] Implement search CLI and JSON output.
- [x] Make pagination/size limits and empty-result handling explicit.
- [x] Add search correctness tests, ranking fixtures and query edge cases.
- [x] Compare against basic keyword search on a reproducible small corpus. Two-file definition/comment regression; exact measured scope and reproduction in docs/search.md, no general quality claim.

**Gate:** `contextforge search "<query>"` returns relevant, cited results from an indexed repository entirely offline.

## Phase 5 — Typed Dependency Graph

**Goal:** Evidence-backed relationships, not fake compiler-level certainty.

- [x] Define node/edge types and statuses (`resolved`, `heuristic`, `unknown`).
- [x] Extract imports and syntactic call/reference candidates.
- [x] Implement qualified symbol lookup and reference retrieval with confidence/evidence.
- [x] Implement bounded, cycle-safe graph traversal and deterministic ordering.
- [x] Support direct/indirect dependents with configurable depth and node limits.
- [x] Include evidence spans and uncertainty in graph outputs.
- [x] Test known graphs and false-positive cases; calculate precision/recall when fixture truth exists.

**Gate:** Graph responses distinguish trustworthy edges from heuristic guesses and stop within limits.

## Phase 6 — Git-Diff-Aware Impact Analysis (v0.2 priority)

**Goal:** Identify likely ripple effects of specific code changes.

- [x] Support unstaged, staged, branch-base and explicit-file changes. Base/head comparison uses merge-base(base, head) → head; modes remain distinct.
- [x] Parse changed ranges and map them to current/previous symbols where possible. Zero-context old/new hunks, source fingerprints and previous/current view labels.
- [x] Identify direct changed symbols plus bounded dependent code. Both source graphs retain deleted/renamed-definition evidence; depth/node/evidence caps apply.
- [x] Suggest related tests only with explicit evidence or clearly labeled heuristics. Only reached dependent paths/names matching documented test patterns are suggested.
- [x] Return clear categories: direct, resolved dependent, heuristic dependent, unresolved. Current resolver does not claim compiler-resolved calls; resolved dependents normally empty.
- [x] Handle detached HEAD, missing refs, binary files, renames and deleted symbols. Renames deliberately appear as delete/add; skipped content is explicit, with known opposite-side syntax retained.
- [x] Test change-impact outputs against fixture repos with known dependency graphs. Central edit → caller → test, staged/unstaged views, removed definitions, merge base, detached/unborn HEAD, ignored history and oversized/binary content.
- [x] Implement documented `impact` CLI options and machine-readable response. Release binary and official MCP client verified; no hidden indexing or command execution.

**Gate:** A known central-function modification finds expected dependent nodes; uncertainty is explicit.

## Phase 7 — Token-Budgeted Context Packing (v0.2 priority)

**Goal:** Provide agents compact evidence-rich context within a defined budget.

- [x] Define query/selection/result contract and source citation format. See docs/context.md; precise byte/line spans and source fingerprints.
- [x] Combine retrieval signals (symbol, FTS, graph, Git proximity). Literal request plus bounded terms; Git path proximity explicitly opt-in, no implicit refresh.
- [x] Deduplicate and diversify relevant snippets. Overlap suppression, graph-evidence merge and first candidate per file before extra snippets.
- [x] Respect code boundaries and attach reason/confidence/evidence. Whole symbols/lines; omit boundaries above 16 KiB with truncation, never claim semantic binding.
- [x] Implement explicit token estimator interface and budget-overhead handling. Named UTF-8 bytes/4 estimate, serialized JSON metadata/escaping included, self-count fixed point; MCP envelope/client overhead excluded explicitly.
- [x] Enforce bounded, deterministic output and truncation metadata. Query/search/source/graph/result caps; repeated fixture output identical.
- [x] Add CLI `context <query> --budget <tokens>` and tests. Shared CLI/MCP implementation; invalid budget/query, Unicode/escaping, optional real Git proximity and read-only checks.
- [x] Create evaluation fixtures for recall and retrieved-token measurements. Three synthetic files, 2/2 expected paths and no unrelated path; release package 1340 bytes/335 estimated tokens. Tiny regression corpus only, no general savings claim.

**Gate:** No output exceeds the stated estimator's configured budget; repeated queries produce stable results.

## Phase 8 — Persistent Structural Knowledge

**Goal:** Reuse indexed project intelligence safely across runs.

- [x] Persist module map, indexed symbols and source-linked derived summaries/notes where appropriate. Reuses schema-2 file/outline/symbol inventory; graph derived from persisted occurrences. Prose summaries/notes deliberately not introduced.
- [x] Track versions, fingerprints, provenance and invalidation dependencies. Memory status compares current source SHA-256, reports additions/staleness, omits stale symbol counts; snapshot APIs preserve documented captured-evidence contracts.
- [x] Isolate repositories; support status/clear/rebuild commands. Root-scoped managed rows only; transactional clear and atomic forced reparse, no directory deletion.
- [ ] Never store agent chats or secrets by default. IN PROGRESS — no chats/notes are retained, but full-source index can contain source-embedded secrets. Literal secret-exclusion gate is not claimed; private-cache requirement documented.
- [x] Handle schema migrations and corruption/recovery paths. Reuses tested schema 1→2 migration/recovery; unknown/corrupt caches preserved; injected clear/rebuild failures roll back.
- [x] Test persistent reuse, stale invalidation and repository separation. Separate CLI processes, changed/added/removed sources, forced unchanged reparsing and existing external-cache identity tests.

**Gate:** A new process can reuse validated data; source changes prevent stale outputs.

## Phase 9 — MCP Server and Tools

**Goal:** A working, compatible MCP server using official `rmcp` APIs.

- [x] Implement stdio server startup/initialize/list tools/call tools/shutdown. Official SDK lifecycle verified; all seven real tools implemented.
- [x] Ensure stdout contains only MCP protocol messages; stderr diagnostics only.
- [x] Implement `search_context` with validated arguments, compact results and limits. Official SDK client verifies repeated snapshot queries, schema, errors, explicit refresh requirement and no index/source writes.
- [x] Implement `symbol_lookup` with valid locations. Current-file outline mode takes a relative path and shares the CLI parser. Indexed name/qualified lookup is available through `search_context` and CLI search.
- [x] Implement `find_references` with certainty markers. Heuristic incoming calls and matching unresolved evidence; official MCP client tested.
- [x] Implement `dependency_graph` with depth/result limits. Typed bounded traversal, cycle and partial-syntax tests, official MCP client tested.
- [x] Implement `analyze_change` with evidence. Real Git snapshots and bounded graph traversal, safe errors, structured old/new evidence and shared worker/time limits; official child-process client tested.
- [x] Implement `pack_context` with token-budget metadata. Official client compares exact shared CLI package, schema/range/root rejection and repeated calls; no source/cache writes.
- [x] Implement `project_map` with source-backed overview. Bounded filesystem inventory only; no unimplemented syntax/dependency claims.
- [x] Add structured errors, timeouts and request cancellation handling where supported. All seven workers observe SDK RequestContext cancellation and timeout waits. Unit tests verify retained busy permits/timeouts; raw protocol cancellation suppresses cancelled replies, answers next ping and excludes query/cancellation reason from diagnostics. Native work is not forcibly killed.
- [x] Add child-process protocol tests for initialize, list, call, errors and shutdown. Includes repeated calls, malformed/oversized messages, root-argument rejection and clean EOF exit.
- [x] Verify at least one real MCP client can use the tools. Official rmcp 3.8.0 SDK child-process client; actual harness interoperability remains unverified.

**Gate:** Tool schemas match behavior; real MCP client can complete initialize → list → call without stdout corruption.

**Scheduling note:** Build a **minimal MCP vertical slice** (e.g., `search_context` and `symbol_lookup`) during MVP, rather than waiting until all Phase 5–8 features exist. Complete remaining MCP tools as their engines become available.

## Phase 10 — Security and Hardening

**Goal:** No unexpected code execution, leaks or unauthorized file access.

- [x] Threat-model repository content, local cache, MCP inputs and harness configurations. docs/security.md records boundaries, quotas, trust assumptions and unresolved gaps.
- [ ] Enforce authorized-root access; block traversal, symlink/junction escapes and unauthorized absolute paths. IN PROGRESS — existing static-path/junction tests pass; concurrent hostile replacement and Windows hard links remain unresolved.
- [x] Confirm indexing never invokes target repository scripts/build/test commands. Runtime review: only filesystem/parser/SQLite and native Git APIs; fixture source/cache integrity tests pass. Development tests explicitly authorized separately.
- [ ] Apply file, request, graph, time and output limits. IN PROGRESS — existing caps plus serialized context limits, SQLite 512 MiB main/sidecar, 32 MiB rows, 64 KiB SQL, no attachments and 5-second/100-million-VM read work cap verified. Strict peer task/rate/process RSS quotas remain open.
- [x] Test prompt-injection-looking source comments as **data**, not instructions. Existing parser regressions and new source-integrity context/client fixture pass; no command interpretation exists.
- [x] Redact secrets and source content in logs by default. Application-owned tracing only; raw verbose protocol test excludes synthetic private query/cancellation sentinel. Intentional returned source evidence is not secret-redacted.
- [x] Validate DB query safety and handle malformed syntax/inputs. Parameter binding/literal FTS, malformed protocol/schema/range cases, SQL row/statement/attachment/work limits and corrupt-cache preservation tests pass.
- [ ] Define safe database and cache file permissions for supported systems. IN PROGRESS — Unix 0700 new directories and hard-link rejection implemented; Unix-only test NOT RUN here. Windows inherits parent ACLs; active ACL/hard-link validation remains open.
- [x] If HTTP added: loopback default, auth, Origin/host validation, request limits and clear deployment warnings. Not applicable: no HTTP transport or remote listener implemented/advertised.
- [x] Run dependency audits and document any accepted findings. cargo-audit 0.22.2 installed only in ignored .tools/security-tools; 175 dependencies checked against RustSec commit 2295466c4e9df26647fd67d8fa1c705ac2fc7f81 (1395 advisories), zero known vulnerabilities and no warnings. No ignored/accepted findings. cargo-deny remains unavailable/unconfigured.
- [x] Write `SECURITY.md` and `docs/security.md` with reporting and limitations. Existing reporting guidance linked to explicit threat model and gaps.

**Gate:** Security regression suite passes; risks and remaining limitations are documented.

## Phase 11 — Harness Integration and Installation

**Goal:** Make the same binary easy to use from multiple coding assistants.

- [ ] Verify current Claude Code MCP registration format and create tested manual instructions.
- [ ] Verify current Codex MCP registration format and create tested manual instructions.
- [ ] Verify Cursor MCP configuration and provide tested instructions.
- [ ] Verify VS Code/Copilot MCP configuration and provide tested instructions.
- [ ] Implement `setup <harness> --dry-run` with safe preview.
- [ ] Implement opt-in config update with backup, merge, syntactic validation and rollback guidance.
- [ ] Provide `doctor` checks for binary, repository, index and MCP connectivity.
- [ ] Add optional native skills/commands only where actual harness specifications permit.
- [ ] Run per-harness manual smoke tests and record versions, operating system, evidence and limitations.

**Gate:** At least Claude Code and Codex integration paths have been verified (actual harness tests where accessible; otherwise explicitly mark unverified).

## Phase 12 — Performance and Retrieval Benchmarks

**Goal:** Publish repeatable evidence of usefulness, not marketing claims.

- [ ] Create representative fixture repositories (small ~100 files, medium ~1k, large ~10k; adjust to available resources).
- [ ] Benchmark cold and incremental indexing, search, graph traversal, and context packing.
- [ ] Measure peak RAM, CPU, disk/index size, latency, and OS/hardware/compiler details.
- [ ] Compare retrieval strategies against basic grep/keyword baselines on comparable tasks.
- [ ] Measure relevant-file recall, precision, estimated tokens and task success when feasible.
- [ ] Document reproducibility instructions and raw results.
- [ ] Remove any claimed performance improvement not supported by a recorded benchmark.

**Gate:** Another developer can repeat representative measurements from documented inputs and scripts.

## Phase 13 — Reliability, CI and Regression Coverage

**Goal:** Ensure changes remain safe and testable.

- [x] CI: format, Clippy, tests, build on supported platforms. GitHub Actions run 41014286004 passed on ubuntu-latest, windows-latest and macos-latest for source revision 8acb134933c3d977338c6907f32bf9ca98640bdf.
- [x] Add database migration and index lifecycle regression tests. Includes schema 1→2 backfill/rollback, atomic source/syntax/FTS invalidation, unchanged parse counts and interrupted-writer recovery.
- [x] Add MCP contract tests and failure-mode tests. Official child client exercises seven tools; raw cancellation, malformed/oversized frames, errors and EOF tests pass locally.
- [ ] Add property/fuzz tests around risky parsers, paths, graphs and JSON boundaries when practical.
- [ ] Test cancellation, process restart, interrupted indexing and large-input safeguards.
- [ ] Add dependency/license review automation if appropriate.
- [ ] Publish a known-limits document and maintain a changelog.

**Gate:** CI consistently passes and significant bug classes have regression tests.

## Phase 14 — Packaging and Distribution

**Goal:** Cross-platform binaries and verifiable installation.

- [ ] Check `contextforge` name availability on crates.io before publishing; resolve naming conflicts.
- [ ] Configure package metadata, licenses and release versioning.
- [ ] Build/test Windows x86_64 binary.
- [ ] Build/test Linux x86_64 binary.
- [ ] Build/test Linux ARM64 binary if runner/toolchain available.
- [ ] Build/test macOS ARM64 binary.
- [ ] Build/test macOS Intel binary if feasible.
- [ ] Publish release workflow with checksums and clearly supported targets.
- [ ] Test installation from packaged artifact on clean environment.
- [ ] Verify documentation commands before advertising them as working.
- [ ] Prepare release notes and rollback/upgrade guidance.
- [ ] Obtain owner approval before any public publish, push, tag or release.

**Gate:** Each claimed platform has a tested artifact and reliable installation instructions.

## Phase 15 — GitHub/Open-Source Readiness

**Goal:** A useful and credible repository that invites adoption and contributions.

- [x] README: clear problem/solution, demo, installation, supported platforms/languages, limitations. Current capabilities and unreleased status stated explicitly; source install smoke-tested locally.
- [x] README: architecture, feature matrix, CLI and MCP tool examples.
- [ ] README: Claude Code, Codex, Cursor, VS Code integration instructions (verified status shown).
- [x] Include sample repositories/test fixtures and safe demo workflows. Synthetic temporary fixtures and the documented init/index/reindex workflow.
- [x] Include real benchmark results with reproduction steps and no invented savings. Single-sample limits and debug environment documented.
- [x] `CONTRIBUTING.md`, `SECURITY.md`, `LICENSE`, `CHANGELOG.md`, issue/PR templates.
- [x] Document high-value beginner and advanced contribution opportunities.
- [x] Review all docs for unsupported feature promises and stale command examples. Current capabilities reflect verified Phases 0–6; later roadmap work remains pending. Local documentation links verified; original source-install evidence is historical.
- [ ] Prepare first public release checklist and obtain approval before publishing.

**Gate:** A new developer can build, install, configure, exercise an MCP tool and understand limitations from the README alone.

---

## Cross-Cutting Acceptance Matrix

| Capability | Required verification | State |
|---|---|---|
| Offline operation | Index and search with network disabled | Not tested |
| Read-only repository | Snapshot/diff confirms sources unchanged | PASS: fixture contents/hash unchanged; init/index write only managed metadata |
| Incremental index | Fixture edits, deletions, renames, reindex | PASS: source snapshot lifecycle and rollback/restart tests |
| Syntax outlines | Rust, Python, TypeScript, JS fixture assertions | PASS: four language fixtures, TSX/JSX, Unicode/CRLF, malformed syntax, stable identity and changed-source reads |
| Search quality | Ground-truth source hits and result spans | PASS: cited exact/prefix/qualified/filename/FTS fixtures; tiny two-file keyword comparison, not broad retrieval evaluation |
| Reference accuracy | Confidence labels and known graph fixtures | PASS: three expected candidate calls (3/3 fixture precision/recall), ambiguous/dynamic calls unknown; no general semantic accuracy claim |
| Change impact | Test known dependencies against Git changes | PASS: central edit finds caller/test; staged/unstaged/base/file, old/deleted syntax and failure cases; native Git index/source unchanged |
| Token budget | Estimator plus serialization overhead tests | PASS: self-inclusive compact JSON estimate, Unicode/escaping, 256–599 sweep, whole-boundary omission; release 335/4000 and 252/256 estimated tokens |
| MCP compatibility | Actual client initialize → list → call | PASS: official SDK client; harness-specific installs NOT RUN |
| Prompt-injection resilience | Malicious comments treated as data | PASS: parser ignores instruction-looking comments; no code execution |
| Cross-platform support | Build + smoke tests per advertised OS | Phase 2 CI PASS on Windows/Linux/macOS; Phases 3–6 verified locally on Windows only. New-source other-OS checks and packaged installs NOT RUN |
| Release claims | Reproducible benchmark and install evidence | Not tested |

## Validation Log

> Add an entry after every meaningful implementation session. Do not write `PASS` without executing the command.

| Date (UTC) | Phase/task | Command or test | Result (PASS/FAIL/NOT RUN) | Environment / evidence |
|---|---|---|---|---|
| — | Initial tracker creation | No implementation validation performed | NOT RUN | Template created for new project |
| 2026-10-30 | Discovery | `git status --short`; full instruction/spec/tracker reads; official crate docs and installed source inspection | PASS | Initially only untracked AGENTS.md/PROMPT.md/TASKS.md; AGENTS.md and PROMPT.md preserved |
| 2026-10-30 | Toolchain bootstrap | Initial Cargo test attempts with incomplete/mixed Windows GNU tools | FAIL, resolved | Missing dlltool, import-library generation failure and pre-test access violation; installed complete workspace-local MinGW GCC 16.2 POSIX/SEH/MSVCRT bundle, verified release asset SHA-256; no system PATH changes |
| 2026-10-30 | Early test/compile corrections | Initial SDK argument-error assertion, invalid-glob fixture and SQLite numeric mappings | FAIL, resolved | Verified installed SDK tool-error semantics; used an actually invalid reversed range; mapped SQLite values to supported signed/count types; final assertions remain strict |
| 2026-10-30 | Final quality | `cargo fmt --all -- --check` | PASS | Invoked through `pwsh -NoProfile -File scripts/cargo-local.ps1 ...`; Rust 1.99.1 Windows GNU |
| 2026-10-30 | Final quality | `cargo clippy --workspace --all-targets -- -D warnings` | PASS | All three workspace crates and test targets |
| 2026-10-30 | Final quality | `cargo test --workspace` | PASS | 22 tests: 3 CLI, 2 MCP, 8 root/discovery, 9 index lifecycle; Unix-only tests not run on this Windows host |
| 2026-10-30 | Final quality | `cargo build --workspace --release` | PASS | Working `target/release/contextforge.exe`; Windows system DLL imports only |
| 2026-10-30 | Phase 2 baseline | `cargo test -p contextforge-index fixture_baseline_and_source_integrity -- --nocapture` | PASS | 100 generated Rust files, i7-7700 @ 3.60 GHz, Windows x86_64, debug: cold 115 ms, unchanged 103 ms, SQLite 28672 bytes; one sample |
| 2026-10-30 | Binary smoke | Release binary `--version`, fixture `init`, two `index` runs, `status`, `doctor`; source SHA-256 comparison | PASS | First index added 1; second unchanged 1; ready schema 1; source unchanged; fixtures stored under ignored .tools/ |
| 2026-10-30 | Optional checks | `cargo audit`, `cargo deny check`, Linux/macOS builds, real harness installations, network-disabled execution | NOT RUN | Audit/deny absent from `cargo --list`; no other-platform runners or harness installation performed; network disabling not attempted |
| 2026-10-30 | Final review | Full new source/manifest/docs review; `git diff --check`; instruction file hashes | PASS | No commits, pushes, publications or external editor settings changes; generated artifacts remain ignored |
| 2026-10-30 | Repository publication | Locked Cargo format/Clippy/tests/release; isolated source install; actionlint; staged-file review and local link checks | PASS | 22 Windows behavior tests; installed binary reports contextforge 0.1.0; 23 local documentation links valid |
| 2026-10-30 | GitHub publication | Create public MohamedRF/ContextForge, push main, verify remote SHA/visibility/license | PASS | Initial commit fc789ca6b3b37f80e97fe5d598cde651d6d2afbb authored as requested; MIT detected by GitHub |
| 2026-10-30 | Initial CI | GitHub Actions run 41013732878 | FAIL, resolved | Linux/Windows passed; macOS external-cache fixture used a linked /var ancestor. Canonicalized the fixture and added Unix linked-cache rejection coverage, preserving the production safety policy |
| 2026-10-30 | Cross-platform CI | GitHub Actions run 41014286004 | PASS | Windows, Linux, macOS: format, Clippy with warnings denied, tests, release builds all passed at source revision 8acb134933c3d977338c6907f32bf9ca98640bdf |
| 2026-10-30 | Phase 3 | Format, Clippy, workspace tests, release build via existing Windows Cargo helper | PASS | 29 tests at Phase 3 gate; four-language release outlines and official SDK symbol calls verified |
| 2026-10-30 | Phase 4 final gates | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo build --workspace --release --locked` | PASS | Rust 1.99.1 Windows GNU; 38 tests: 5 CLI, 3 MCP, 8 core, 9 lifecycle, 7 search, 6 parser |
| 2026-10-30 | Phase 4 recovery correction | Interrupted child-writer regression after new preparation read | FAIL, resolved | Read-only preparation could not recover a hot journal; write-capable preparation during explicit index restores SQLite recovery; strict original test passes |
| 2026-10-30 | Phase 4 smoke | Release binary fixture init/index/reindex/analyze/search/status; source SHA-256 comparison | PASS | Four source languages; first index parsed 4, unchanged run parsed 0; cited exact/qualified hits; no source changes |
| 2026-10-30 | Phase 4 review | Full source/manifest/docs diff review; `git diff --check`; local doc links | PASS | 31 local links checked; terminal-control escaping regression; AGENTS.md/PROMPT.md preserved; no external writes/publication |
| 2026-10-30 | Phase 3/4 optional checks | `cargo audit`; `cargo deny check`; new-source Linux/macOS builds; actual harness installations; network-disabled execution | NOT RUN | Audit/deny absent from `cargo --list`; only local Windows environment used; no harness settings changed or network isolation attempted |
| 2026-10-30 | Phase 5 | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo build --workspace --release --locked` | PASS | 43 Windows behavior tests; pure graph fixture and real CLI/MCP child-client checks |
| 2026-10-30 | Phase 6 final | Same four Cargo gates via existing scripts/cargo-local.ps1 | PASS | 50 Windows tests: 5 CLI, 1 graph adapter, 1 impact adapter, 3 MCP, 9 core, 5 Git, 4 graph, 9 lifecycle, 7 search, 6 parser; release build 2m37s |
| 2026-10-30 | Phase 6 corrections | git2 API/error conversion/Clippy and overlapping Windows executable locks | FAIL, resolved | Corrected Result-returning TreeEntry API, boxed CLI error conversion and nested conditionals; overlapping reruns caused linker/process locks, then fully sequential final checks passed |
| 2026-10-30 | Phase 6 release smoke | Release impact/default/staged/file, init/index, graph/references; source/Git index hashes and ID comparison | PASS | Isolated uncommitted Git fixture: 1 changed file, 2 likely dependents, 3 graph nodes, 1 reference; cached and Git IDs agree; no source/Git index mutation |
| 2026-10-30 | Phase 5/6 review | Full affected-source review, `git diff --check`, local links, instruction hashes | PASS | 40 local documentation links; AGENTS.md/PROMPT.md hashes unchanged; no project commits/pushes/harness changes |
| 2026-10-30 | Phase 5/6 optional | `cargo audit`; `cargo deny check`; new-source Linux/macOS builds; actual harness install; network-disabled runtime | NOT RUN | Audit/deny absent from cargo --list; local Windows environment only; no external settings or CI push authorized |
| 2026-10-30 | Phase 7/8/9/10 baseline | `cargo test --workspace --locked` via scripts/cargo-local.ps1 | PASS | All 50 pre-existing Windows tests; initial user changes preserved |
| 2026-10-30 | Phase 7/8/9/10 final gates | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked`; `cargo build --workspace --release --locked` via local Cargo helper | PASS | Rust 1.99.1 Windows GNU; 65 tests including official seven-tool client, serialized budgets, persistent lifecycle, SQL limits, cancellation/timeout/permit behavior; release build 29.90s |
| 2026-10-30 | Implementation corrections | Context graph-evidence merge/seed certainty; SQLite page-size signed conversion; SDK cancellation response semantics; Git depth-zero truncation | FAIL, resolved | Regression tests caught dropped relation evidence; installed SDK suppresses cancelled responses; corrected strict protocol assertions; real Git proximity regression passes |
| 2026-10-30 | Phase 7/8 release smoke | Release --version/init/index/context twice, 256 budget, memory status/rebuild/clear/rebuild and source/cache SHA-256 comparisons | PASS | Synthetic three-file fixture; 2 expected paths, 1340 compact JSON bytes = 335 estimated tokens/4000; small 252/256 truncated; deterministic; sources/read-only cache unchanged; force parsed and cleared 3 files |
| 2026-10-30 | Review | Affected full source/manifest/docs review; `git diff --check`; local doc links | PASS | 55 local documentation links; no project commits/pushes, user data deletion, external settings or new third-party production dependencies. Existing rusqlite hooks/limits enabled; dev test reuses rusqlite |
| 2026-10-30 | Optional platform/integration checks | Unix-only security test; new-source Linux/macOS builds; actual harness install; network-disabled runtime; cargo deny check | NOT RUN | Windows-only host; no harness settings or network isolation changed; deny absent from cargo --list |
| 2026-10-30 | Local security tooling | `cargo install cargo-audit --root .tools/security-tools --locked` via local Cargo helper | PASS | cargo-audit 0.22.2 compiled and installed locally; no global software/PATH change, no project dependency update |
| 2026-10-30 | Dependency advisories | `.tools/security-tools/bin/cargo-audit.exe audit --db .tools/rustsec-advisory-db --file Cargo.lock --json` | PASS | Exit 0; 175 dependencies, 1395 advisories, zero known vulnerabilities, no warnings; RustSec commit 2295466c4e9df26647fd67d8fa1c705ac2fc7f81. Raw report .tools/phase7-10-audit.json; no source uploaded |

## Decisions Log

| ID | Date | Decision | Rationale | Status |
|---|---|---|---|---|
| D-001 | Initial | Rust + Cargo workspace, stdio MCP first | Single deployable core with broad harness compatibility | Adopted; CLI and SDK-client slice tested |
| D-002 | Initial | SQLite + FTS5 first, optional embeddings later | Small offline baseline without model dependency | Adopted; SQLite/FTS implemented and tested in Phases 2/4 |
| D-003 | Initial | Tree-sitter with explicit confidence levels | Syntax extraction must not imply full semantic resolution | Four-language implementation tested; occurrences have unknown targets |
| D-004 | Initial | Source repository read-only, local index writable | Safe default and auditable trust boundary | Implemented/tested for current slices; hardening caveats documented |
| D-005 | Initial | No UI required for MVP | Focus on useful harness integration and tests | Adopted |
| D-006 | 2026-10-30 | Three useful crates: core, index, CLI (MCP adapter inside CLI) | Enforces actual domain boundaries without placeholder crates | Implemented/tested |
| D-007 | 2026-10-30 | Rust 1.99.1 pinned; application lockfile retained | Reproducible initial host; older MSRV/cross-platform compatibility unclaimed | Implemented/tested on Windows |
| D-008 | 2026-10-30 | SHA-256 source snapshots, SQLite schema 1, immediate transactions | Detect same-size edits; atomically remove stale paths; refuse truncated snapshots | Implemented/tested |
| D-009 | 2026-10-30 | MIT selected by owner for public GitHub repository | Explicit owner choice; root LICENSE and package metadata agree | Adopted; crate publishing remains disabled |
| D-010 | 2026-10-30 | Pure contextforge-graph; candidate edges derived per request from schema-2 outlines | No new persistent format, no transport/storage coupling, explicit syntactic certainty | Implemented/tested locally on Windows |
| D-011 | 2026-10-30 | git2 0.21.0 without default network features; buffer diffs and current historical-path policy | No shell/external filters, no Git writes, old/new evidence with bounded likely impact | Implemented/tested locally on Windows |
| D-012 | 2026-10-30 | contextforge-context reuses one SQLite read snapshot; compact JSON uses named injectable token estimator | Consistent citations, explicit approximate budget, no transport coupling or new third-party production dependency | Implemented/tested locally on Windows |
| D-013 | 2026-10-30 | Reuse schema-2 structural storage for memory lifecycle | Avoid duplicate prose cache and stale invalidation paths; atomic clear/forced reparse, no stored chats | Lifecycle verified; literal embedded-secret exclusion remains open |

## Blockers and Risks

| Risk | Mitigation | Status |
|---|---|---|
| MCP SDK/API churn | Pin verified versions, test protocol with real client | Open |
| Tree-sitter syntax vs semantic resolution | Separate resolved/heuristic/unknown, fixture tests | Open |
| Cross-platform path and symlink safety | OS-aware path tests, allowed-root policy | Open |
| Token estimates vary by model | Explicit tokenizer interface, label estimates, enforce overhead | Open |
| Harness setup formats change | Verify official docs, dry-run and non-destructive config merge | Open |
| Name/package availability | Check crates.io/GitHub before publication | Open |

## Current Session Handoff

- **Current milestone:** Phase 7 and remaining Phase 9 tools/cancellation verified; Phase 8 structural lifecycle verified with embedded-secret privacy gate open; Phase 10 IN PROGRESS. All pre-existing uncommitted changes preserved.
- **Completed:** Consistent-snapshot bounded context packing, CLI context, seventh MCP tool, injectable serialized token estimates, graph evidence merge, source-boundary/dedup/diversity policy, optional Git path ranking; memory status/clear/rebuild; SDK cancellation waits, SQL limits and security threat model.
- **Files changed this session:** Added contextforge-context and tests; index search/snapshot/memory/security helpers and tests; CLI/MCP orchestration and context/memory/cancellation tests; manifests/lock; context/memory/security/architecture/MCP/storage docs, README/SECURITY/changelog/TASKS. AGENTS.md/PROMPT.md unchanged.
- **Validation:** Windows GNU Rust 1.99.1 format/Clippy/65 tests/release/audit PASS. Release three-file fixture: two expected paths, 1340 serialized bytes/335 estimated tokens within 4000, small package 252/256, deterministic output, unchanged source/read-only cache hashes, forced parse/clear/rebuild of three files. All seven MCP tools exercised with official client; raw protocol cancellation/privacy/EOF verified. Local cargo-audit 0.22.2: 175 dependencies, zero known vulnerabilities/warnings. Other-platform, actual harness, cargo-deny and network-disabled runtime checks NOT RUN.
- **Open issues/security:** Captured search/graph/context freshness stays explicit; memory status validates observed hashes. No exact tokenizer, semantic proof, guaranteed secret-free source cache, hostile-filesystem race protection, Windows hard-link/ACL validation, strict peer/process quota or forced native cancellation guarantee. No packaged release claim.
- **Next task:** Complete Phase 10 permissions/race/peer-quota work and verify new-source Unix tests; establish explicit source-secret exclusion/retention policy for the literal Phase 8 gate. Manual Claude Code/Codex registration remains an independent MVP task; no external settings writes without authorization.
- **Safety reminder:** Preserve user changes and require approval for destructive or externally visible actions.

## Repository Presentation and Publication — 2026-10-30

This is a separately authorized repository-maintenance task; it does not start Phase 3 feature development.

- [x] Rewrite README around actual product behavior, quick start, CLI/MCP contracts, limits, measured evidence and roadmap.
- [x] Apply owner-selected MIT license and package author/repository metadata.
- [x] Add contribution/security guidance, changelog, GitHub issue/PR templates, LF normalization and a minimal-permission CI workflow pinned to the verified checkout action commit.
- [x] Ignore downloaded tools, builds, source caches, local credentials/editor files, original local prompt, obsolete license notes and machine-specific bootstrap script without deleting them.
- [x] Verify local format, Clippy, 22 tests and release build with locked dependencies; validate workflow with actionlint 1.7.12.
- [x] Verify the documented source-install command using an isolated local install. `cargo install --path crates/contextforge-cli --root .tools/product-install --locked` passed; installed binary reports `contextforge 0.1.0`.
- [x] Review staged file list for local artifacts/secrets and commit as MohamedRF <soft-arch@ots.annauniv.edu>. Initial commit `fc789ca6b3b37f80e97fe5d598cde651d6d2afbb`; 35 source/documentation/config files; local notes and generated artifacts excluded.
- [x] Create public `MohamedRF/ContextForge`, push `main`, and verify the remote commit. https://github.com/MohamedRF/ContextForge; public visibility, default branch main and MIT detection verified through GitHub API; origin/main matches the initial commit.
- [x] Inspect actual GitHub CI results; fix confirmed failures or record external blockers without claiming unrun checks passed. [Run 41014286004](https://github.com/MohamedRF/ContextForge/actions/runs/41014286004) passed on all three platforms at source revision 8acb134933c3d977338c6907f32bf9ca98640bdf.

Publication preparation evidence: all four local Cargo quality gates passed with `--locked` where applicable; actionlint 1.7.12 accepted the workflow; `git diff --cached --check`, staged credential-pattern scan and artifact-exclusion checks passed. Git author configured locally as requested; global Git settings untouched.

Initial GitHub CI run `41013732878`: Linux and Windows passed; macOS correctly rejected a linked ancestor in the external-cache test fixture (`/var` alias). Fixed the fixture to select the canonical authorized cache, preserving all isolation assertions, and added a Unix-only linked-cache rejection regression. No production safety policy was weakened. Local Windows format/tests (22)/Clippy/release gates pass after the correction; cross-platform rerun `41014286004` passed on all three platforms. The final publication-record commit updates documentation only and skips redundant CI; it does not change the verified source or workflow.

## Phase 3 Verification — 2026-10-30

User explicitly resumed development. Initial worktree was clean; AGENTS.md, TASKS.md and PROMPT.md read completely, architecture and current callers/tests inspected. Context7 tools were unavailable; official Tree-sitter docs and installed grammar/binding sources verified instead.

- Implemented `contextforge-parser`, language adapters, source-backed spans, stable IDs, declared visibility, parent relationships, bounded signatures and separate unknown-target imports/exports/calls.
- `analyze <relative-file> --limit` and MCP `symbol_lookup` read current eligible source without metadata writes. Coverage and identity/partial-parse/limit semantics are in docs/parsing.md. Schema 1 remains unchanged; persistence belongs to the next indexed-search slice.
- PASS: `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets --locked -- -D warnings`; `cargo test --workspace --locked` (29 behavior tests); `cargo build --workspace --release --locked`. All invoked through the existing local Windows Cargo helper.
- PASS: release binary analyzed all four committed fixtures, reporting 9/4/9/4 symbols respectively, valid JSON and no syntax errors/truncation. Official rmcp child-process tests exercise both tools and rejection paths.
- Initial compilation failed on the Tree-sitter 0.27 cancellation callback return type; fixed to the installed `ControlFlow` API. Final gates passed without weakening tests.
- NOT RUN: dependency audits (tools not installed), new-source Linux/macOS builds and real harness installations. No publishing, commits or external settings changes.
- Next smallest task: schema migration for persisted definitions/FTS and bounded offline CLI/MCP search. Existing hostile filesystem/cache and cooperative-cancellation limitations remain.

## Phase 4 Verification — 2026-10-30

- Added schema 1→2 migration, persisted bounded outlines/definitions and Unicode61 FTS5 with parameter-bound literal phrases. Original source snapshots remain available; all updates, backfill and obsolete record invalidations share one immediate transaction.
- Syntax preparation happens before the write lock. Unchanged schema-2 files are not reparsed; parse counts and partial syntax are explicit in reports. Hot-journal recovery remains available during explicit indexing. A conflicting intervening writer can cause an explicit retry error rather than mismatched source/syntax.
- Added deterministic exact/qualified symbol, filename, prefix and BM25 tiers, page/result/query limits, source fingerprints, byte/line citations, matched passages and clipped snippets. Current-source outlines and captured-source search have distinct documented freshness semantics. Unknown occurrence targets are never presented as proven references.
- CLI `search` and MCP `search_context` are real and read-only against the captured index. Human output escapes terminal controls. MCP preserves protocol-only stdout, shared worker limits and safe errors. Search uses one read transaction and batch evidence loading.
- All four final Cargo gates PASS on Windows (38 behavior tests). The official rmcp client initializes/lists/calls all three implemented tools. Final release fixture init/index/reindex/analyze/search/status and source-hash integrity PASS. Tiny corpus comparison assertions PASS; see docs/search.md for scope.
- Final review corrected a documentation-edit script error in draft parsing/MCP docs and verified the corrected content/links. No user changes existed initially; no AGENTS/PROMPT modifications, commits, pushes, production deployment, global installs or external harness writes occurred.
- Limitations: no semantic reference resolution, graph/Git/context engines, automatic freshness validation, tunable ranking, hardened SQL/worker cancellation, actual harness interoperability or new-source cross-platform validation. Optional audit tools unavailable. Broader roadmap tasks remain unchecked.
- Next smallest task: Phase 5 typed graph with candidate uncertainty, bounded traversal and fixture precision/false-positive checks. Harness registration documentation/verification remains an independent MVP task.

## Phase 5 Verification — 2026-10-30

Preserved all pre-existing uncommitted Phase 3/4 changes. Read AGENTS.md, TASKS.md, PROMPT.md completely and inspected architecture/parser/index/CLI/MCP callers before edits. Added pure contextforge-graph, read-only bounded index syntax export, CLI graph/references and MCP dependency_graph/find_references. Schema remains 2; no hidden refresh or source execution. See docs/graph.md for candidate policy, uncertainty, limits and fixture precision/recall.

PASS on local Windows GNU Rust 1.99.1 via scripts/cargo-local.ps1: cargo fmt --all -- --check; cargo clippy --workspace --all-targets --locked -- -D warnings; cargo test --workspace --locked (43 behavior tests); cargo build --workspace --release --locked; git diff --check. Test suite initially needed a CLI boxed-error type correction; Clippy required a collapsed conditional; corrected without weakening checks. Pure graph fixtures prove a three-edge cycle, ambiguous/dynamic-call unknowns, import/qualified evidence and bounds. Official MCP child client calls both tools and confirms source/cache bytes unchanged.

Graph-related Phase 9 items verified complete; later tools remain pending. Graph calls/imports are heuristic or unknown; only lexical containment is resolved. No semantic-completeness claim. Next: Phase 6 read-only Git snapshot/diff/change-impact engine. Dependency audit, actual harness and new-source other-platform verification remain NOT RUN.

## Phase 6 Verification — 2026-10-30

- Added contextforge-git using verified git2 0.21/libgit2 1.9.7 APIs, with default network features disabled. Runtime reads index/tree/blob snapshots and uses buffer patches, never shells, Git writes, external diffs/textconv, project tests or implicit ContextForge indexing. Fixture commits are isolated synthetic test data; the project repository was not committed.
- Implemented unstaged, staged (including unborn HEAD), merge-base-to-selected-head and explicit-file modes; old/new zero-context ranges and symbols, source fingerprints/view labels, deleted-symbol dependents, skipped content and rename-as-delete/add semantics. Refs, roots, paths, conflicts, file/input/output/time bounds are explicit. Blob headers are size-checked before loading content.
- Added current-policy historical-path checks with scoped ignore files and pruned-parent protection. A whitelisted leaf cannot bypass an ignored directory. Existing discovery exclusions and read-safety behavior remain intact.
- CLI impact and MCP analyze_change return direct changes, heuristic/resolved dependent categories, test candidates, unresolved evidence, partial/incomplete flags and an explicit no-safety-verdict uncertainty statement. Graph candidates are not semantic proof; old/current graphs preserve removed-definition evidence.
- Final format, Clippy with denied warnings, 50 workspace tests and release build PASS locally on Windows GNU Rust 1.99.1. Central-function fixture finds caller and indirect test; stage/working separation, detached/unborn HEAD, base/missing refs, binary/oversized/ignored/rename/deletion cases pass. Official rmcp client exercises all six tools, including graph/impact errors, and verifies read-only state.
- Release smoke confirms one changed file → two heuristic dependents, matching graph/reference results and root-scoped cached/Git IDs; source and Git index hashes unchanged. Final diff/link review PASS (40 local links); instruction hashes match the prior session. No project commits/pushes/publication, global installs, source execution or external harness writes.
- NOT RUN: unavailable audit/deny tools, actual harness installations, new-source other-platform builds and network-disabled runtime. Caches/Git metadata must remain trusted; filesystem races and strict native-call/process quotas are open hardening work. Snapshot freshness and rename/candidate limitations are documented in docs/graph.md and docs/impact.md.
- Next smallest action: Phase 7 bounded context packing with deduplication and explicit token-estimate/serialization overhead enforcement. Harness verification remains an independent MVP task.
