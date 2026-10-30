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

**Stopped after Phase 2 at the user's request (2026-10-30).** Phases 0–2 and the minimal MCP `project_map` slice are verified on Windows. All 22 behavior tests, format check, Clippy and release build pass. The next development task is Phase 3 Tree-sitter parsing, only when further work is requested. Search/symbol/graph/Git/context tools have not been started.

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
- [x] Create SQLite schema for repository, file, symbol, relation, index version and metadata records as needed. Schema 1 contains repository identity and file text/language/hash/bytes; symbol/relation tables await their engines rather than adding placeholders.
- [x] Implement versioned migrations and transactional writes. Empty schema 0 → 1 atomically; unknown/future schemas and corruption are preserved and rejected.
- [x] Implement file fingerprints/hashes and deterministic index ordering. SHA-256 content/root identity and ordered discovery/SQL reads.
- [x] Implement initial indexing and user-facing progress/errors via stderr. CLI index has JSON/human summaries; errors are on stderr; no source-body diagnostics.
- [x] Implement incremental reindex of added/modified/deleted/renamed files. All eligible bytes are read/hashed; only changed database rows are written. This does not claim incremental source reading/parsing.
- [x] Invalidate dependent/stale records on changes. Changed file rows replaced; missing/renamed/newly ignored/binary paths removed transactionally. Derived symbol/relation records do not exist yet.
- [x] Test unchanged reindex, single-file changes, deletes, renames, ignored files, interrupted updates, database upgrades. Includes concurrent writers, injected partial failure, abrupt child exit with uncommitted writes, schema rejection and corruption preservation.
- [x] Measure initial index baseline on a public fixture repo (record environment, no invented numbers). Reproducible generated 100-file source fixture: debug cold 115 ms, unchanged 103 ms, SQLite 28672 bytes; see docs/benchmarking.md. No private repository or downloaded corpus used.

**Gate:** Reindex only processes required changes, old records disappear, and repeated indexes yield consistent results.

## Phase 3 — Tree-sitter Parsing and Symbols

**Goal:** Correct source-backed structural outlines for the first four languages.

- [ ] Define normalized symbol model, source spans, visibility, parent relationships, and language adapters.
- [ ] Add and verify Rust Tree-sitter grammar and tests.
- [ ] Add and verify Python grammar and tests.
- [ ] Add and verify TypeScript grammar and tests.
- [ ] Add and verify JavaScript grammar and tests.
- [ ] Extract functions, methods, types, modules, imports/exports and calls where syntactically available.
- [ ] Store line/byte offsets with documented indexing convention.
- [ ] Handle syntax errors and partial parse results safely.
- [ ] Make stable symbol identity/update semantics explicit.
- [ ] Record limitations of syntax-only reference resolution.
- [ ] Add fixture-based tests for nested constructs, Unicode, malformed syntax and changed files.

**Gate:** `analyze <file>` returns a reproducible outline with valid spans for each supported language.

## Phase 4 — Indexed Search and Ranking

**Goal:** Useful results without embeddings or a model API.

- [ ] Implement exact filename/symbol/qualified-name lookup.
- [ ] Implement SQLite FTS5 indexing and query escaping/validation.
- [ ] Return repo-relative path, symbol, snippet, range, retrieval method and relevance signal.
- [ ] Implement deterministic, explainable result ranking and result caps.
- [ ] Implement search CLI and JSON output.
- [ ] Make pagination/size limits and empty-result handling explicit.
- [ ] Add search correctness tests, ranking fixtures and query edge cases.
- [ ] Compare against basic keyword search on a reproducible small corpus.

**Gate:** `contextforge search "<query>"` returns relevant, cited results from an indexed repository entirely offline.

## Phase 5 — Typed Dependency Graph

**Goal:** Evidence-backed relationships, not fake compiler-level certainty.

- [ ] Define node/edge types and statuses (`resolved`, `heuristic`, `unknown`).
- [ ] Extract imports and syntactic call/reference candidates.
- [ ] Implement qualified symbol lookup and reference retrieval with confidence/evidence.
- [ ] Implement bounded, cycle-safe graph traversal and deterministic ordering.
- [ ] Support direct/indirect dependents with configurable depth and node limits.
- [ ] Include evidence spans and uncertainty in graph outputs.
- [ ] Test known graphs and false-positive cases; calculate precision/recall when fixture truth exists.

**Gate:** Graph responses distinguish trustworthy edges from heuristic guesses and stop within limits.

## Phase 6 — Git-Diff-Aware Impact Analysis (v0.2 priority)

**Goal:** Identify likely ripple effects of specific code changes.

- [ ] Support unstaged, staged, branch-base and explicit-file changes.
- [ ] Parse changed ranges and map them to current/previous symbols where possible.
- [ ] Identify direct changed symbols plus bounded dependent code.
- [ ] Suggest related tests only with explicit evidence or clearly labeled heuristics.
- [ ] Return clear categories: direct, resolved dependent, heuristic dependent, unresolved.
- [ ] Handle detached HEAD, missing refs, binary files, renames and deleted symbols.
- [ ] Test change-impact outputs against fixture repos with known dependency graphs.
- [ ] Implement documented `impact` CLI options and machine-readable response.

**Gate:** A known central-function modification finds expected dependent nodes; uncertainty is explicit.

## Phase 7 — Token-Budgeted Context Packing (v0.2 priority)

**Goal:** Provide agents compact evidence-rich context within a defined budget.

- [ ] Define query/selection/result contract and source citation format.
- [ ] Combine retrieval signals (symbol, FTS, graph, Git proximity).
- [ ] Deduplicate and diversify relevant snippets.
- [ ] Respect code boundaries and attach reason/confidence/evidence.
- [ ] Implement explicit token estimator interface and budget-overhead handling.
- [ ] Enforce bounded, deterministic output and truncation metadata.
- [ ] Add CLI `context <query> --budget <tokens>` and tests.
- [ ] Create evaluation fixtures for recall and retrieved-token measurements.

**Gate:** No output exceeds the stated estimator's configured budget; repeated queries produce stable results.

## Phase 8 — Persistent Structural Knowledge

**Goal:** Reuse indexed project intelligence safely across runs.

- [ ] Persist module map, indexed symbols and source-linked derived summaries/notes where appropriate.
- [ ] Track versions, fingerprints, provenance and invalidation dependencies.
- [ ] Isolate repositories; support status/clear/rebuild commands.
- [ ] Never store agent chats or secrets by default.
- [ ] Handle schema migrations and corruption/recovery paths.
- [ ] Test persistent reuse, stale invalidation and repository separation.

**Gate:** A new process can reuse validated data; source changes prevent stale outputs.

## Phase 9 — MCP Server and Tools

**Goal:** A working, compatible MCP server using official `rmcp` APIs.

- [x] Implement stdio server startup/initialize/list tools/call tools/shutdown. Minimal early slice exposes only the implemented inventory tool; remaining tool engines stay pending.
- [x] Ensure stdout contains only MCP protocol messages; stderr diagnostics only.
- [ ] Implement `search_context` with validated arguments, compact results and limits.
- [ ] Implement `symbol_lookup` with valid locations.
- [ ] Implement `find_references` with certainty markers.
- [ ] Implement `dependency_graph` with depth/result limits.
- [ ] Implement `analyze_change` with evidence.
- [ ] Implement `pack_context` with token-budget metadata.
- [x] Implement `project_map` with source-backed overview. Bounded filesystem inventory only; no unimplemented syntax/dependency claims.
- [ ] Add structured errors, timeouts and request cancellation handling where supported.
- [x] Add child-process protocol tests for initialize, list, call, errors and shutdown. Includes repeated calls, malformed/oversized messages, root-argument rejection and clean EOF exit.
- [x] Verify at least one real MCP client can use the tools. Official rmcp 3.8.0 SDK child-process client; actual harness interoperability remains unverified.

**Gate:** Tool schemas match behavior; real MCP client can complete initialize → list → call without stdout corruption.

**Scheduling note:** Build a **minimal MCP vertical slice** (e.g., `search_context` and `symbol_lookup`) during MVP, rather than waiting until all Phase 5–8 features exist. Complete remaining MCP tools as their engines become available.

## Phase 10 — Security and Hardening

**Goal:** No unexpected code execution, leaks or unauthorized file access.

- [ ] Threat-model repository content, local cache, MCP inputs and harness configurations.
- [ ] Enforce authorized-root access; block traversal, symlink/junction escapes and unauthorized absolute paths.
- [ ] Confirm indexing never invokes target repository scripts/build/test commands.
- [ ] Apply file, request, graph, time and output limits.
- [ ] Test prompt-injection-looking source comments as **data**, not instructions.
- [ ] Redact secrets and source content in logs by default.
- [ ] Validate DB query safety and handle malformed syntax/inputs.
- [ ] Define safe database and cache file permissions for supported systems.
- [ ] If HTTP added: loopback default, auth, Origin/host validation, request limits and clear deployment warnings.
- [ ] Run dependency audits and document any accepted findings.
- [ ] Write `SECURITY.md` and `docs/security.md` with reporting and limitations.

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

- [ ] CI: format, Clippy, tests, build on supported platforms.
- [ ] Add database migration and index lifecycle regression tests.
- [ ] Add MCP contract tests and failure-mode tests.
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

- [ ] README: clear problem/solution, demo, installation, supported platforms/languages, limitations.
- [ ] README: architecture, feature matrix, CLI and MCP tool examples.
- [ ] README: Claude Code, Codex, Cursor, VS Code integration instructions (verified status shown).
- [ ] Include sample repositories/test fixtures and safe demo workflows.
- [ ] Include real benchmark results with reproduction steps and no invented savings.
- [ ] `CONTRIBUTING.md`, `SECURITY.md`, `LICENSE`, `CHANGELOG.md`, issue/PR templates.
- [ ] Document high-value beginner and advanced contribution opportunities.
- [ ] Review all docs for unsupported feature promises and stale command examples.
- [ ] Prepare first public release checklist and obtain approval before publishing.

**Gate:** A new developer can build, install, configure, exercise an MCP tool and understand limitations from the README alone.

---

## Cross-Cutting Acceptance Matrix

| Capability | Required verification | State |
|---|---|---|
| Offline operation | Index and search with network disabled | Not tested |
| Read-only repository | Snapshot/diff confirms sources unchanged | PASS: fixture contents/hash unchanged; init/index write only managed metadata |
| Incremental index | Fixture edits, deletions, renames, reindex | PASS: source snapshot lifecycle and rollback/restart tests |
| Syntax outlines | Rust, Python, TypeScript, JS fixture assertions | Not tested |
| Search quality | Ground-truth source hits and result spans | Not tested |
| Reference accuracy | Confidence labels and known graph fixtures | Not tested |
| Change impact | Test known dependencies against Git changes | Not tested |
| Token budget | Estimator plus serialization overhead tests | Not tested |
| MCP compatibility | Actual client initialize → list → call | PASS: official SDK client; harness-specific installs NOT RUN |
| Prompt-injection resilience | Malicious comments treated as data | Not tested |
| Cross-platform support | Build + smoke tests per advertised OS | Windows x86_64 PASS; Linux/macOS NOT RUN |
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

## Decisions Log

| ID | Date | Decision | Rationale | Status |
|---|---|---|---|---|
| D-001 | Initial | Rust + Cargo workspace, stdio MCP first | Single deployable core with broad harness compatibility | Adopted; CLI and SDK-client slice tested |
| D-002 | Initial | SQLite + FTS5 first, optional embeddings later | Small offline baseline without model dependency | Adopted; SQLite snapshots tested; FTS pending Phase 4 |
| D-003 | Initial | Tree-sitter with explicit confidence levels | Syntax extraction must not imply full semantic resolution | API research complete; implementation pending Phase 3 |
| D-004 | Initial | Source repository read-only, local index writable | Safe default and auditable trust boundary | Implemented/tested for current slices; hardening caveats documented |
| D-005 | Initial | No UI required for MVP | Focus on useful harness integration and tests | Adopted |
| D-006 | 2026-10-30 | Three useful crates: core, index, CLI (MCP adapter inside CLI) | Enforces actual domain boundaries without placeholder crates | Implemented/tested |
| D-007 | 2026-10-30 | Rust 1.99.1 pinned; application lockfile retained | Reproducible initial host; older MSRV/cross-platform compatibility unclaimed | Implemented/tested on Windows |
| D-008 | 2026-10-30 | SHA-256 source snapshots, SQLite schema 1, immediate transactions | Detect same-size edits; atomically remove stale paths; refuse truncated snapshots | Implemented/tested |
| D-009 | 2026-10-30 | MIT selected by owner for public GitHub repository | Explicit owner choice; root LICENSE and package metadata agree | Adopted; crate publishing remains disabled |

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

- **Current milestone:** Phases 0–2 complete; feature development remains stopped at Phase 2. Owner subsequently authorized professional repository presentation and public GitHub publication under MohamedRF with MIT licensing.
- **Completed:** Root/config/CLI, bounded ignore-aware discovery, official stdio MCP inventory, transactional versioned incremental SQLite source index, lifecycle/security regression tests and measured fixture baseline.
- **Files changed:** Cargo workspace/lock/toolchain, crates/contextforge-{core,index,cli}, docs, README/contribution/security/changelog/LICENSE documents, GitHub workflow/templates, .gitignore/.gitattributes and this tracker. Original AGENTS.md and local PROMPT.md preserved; local prompt/bootstrap tools/obsolete license notes excluded from publication.
- **Validation:** Format, Clippy, 22 tests, release build, official SDK MCP test, abrupt-writer recovery test and release CLI smoke PASS; optional audits/other OS/harness installations NOT RUN as detailed above.
- **Open issues/security:** Cached freshness is explicitly unchecked; no parser/search yet; hostile concurrent filesystem replacement, hard-linked/hostile caches, query/entry resource hardening and harness validation remain documented limitations. No packaged release claim.
- **Next task:** Phase 3 normalized Tree-sitter symbol model and first language adapter, only on a new instruction to continue. Do not start it in this session.
- **Safety reminder:** Preserve user changes and require approval for destructive or externally visible actions.

## Repository Presentation and Publication — 2026-10-30

This is a separately authorized repository-maintenance task; it does not start Phase 3 feature development.

- [x] Rewrite README around actual product behavior, quick start, CLI/MCP contracts, limits, measured evidence and roadmap.
- [x] Apply owner-selected MIT license and package author/repository metadata.
- [x] Add contribution/security guidance, changelog, GitHub issue/PR templates, LF normalization and a minimal-permission CI workflow pinned to the verified checkout action commit.
- [x] Ignore downloaded tools, builds, source caches, local credentials/editor files, original local prompt, obsolete license notes and machine-specific bootstrap script without deleting them.
- [x] Verify local format, Clippy, 22 tests and release build with locked dependencies; validate workflow with actionlint 1.7.12.
- [x] Verify the documented source-install command using an isolated local install. `cargo install --path crates/contextforge-cli --root .tools/product-install --locked` passed; installed binary reports `contextforge 0.1.0`.
- [ ] Review staged file list for local artifacts/secrets and commit as MohamedRF <soft-arch@ots.annauniv.edu>.
- [ ] Create public `MohamedRF/ContextForge`, push `main`, and verify the remote commit.
- [ ] Inspect actual GitHub CI results; fix confirmed failures or record external blockers without claiming unrun checks passed.

Publication preparation evidence: all four local Cargo quality gates passed with `--locked` where applicable; actionlint 1.7.12 accepted the workflow; `git diff --cached --check`, staged credential-pattern scan and artifact-exclusion checks passed. Git author configured locally as requested; global Git settings untouched.
