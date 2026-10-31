# Architecture

ContextForge is an offline, local-first code intelligence engine. The MVP is safe discovery → incremental SQLite index → Tree-sitter definitions → cited search → stdio MCP. No LLM, telemetry, source uploads, GUI, or embedding service is required. No runtime component executes project scripts or shell commands.

## Initial boundaries and flow

`contextforge-cli` owns Clap arguments, stderr tracing, output and the `rmcp` adapter. `contextforge-core` owns authorized roots, config, discovery/source reads and historical-path ignore policy. `contextforge-index` owns SQLite persistence/search and bounded read-only syntax export. `contextforge-parser` owns Tree-sitter outlines; `contextforge-graph` is a pure, I/O-free candidate graph engine; `contextforge-git` captures read-only Git/source snapshots, computes buffer diffs and orchestrates graph impact. Core does not depend on transports, SQLite or Git. Graph depends on parser evidence types; Git depends on core/parser/graph, never on CLI/MCP.

Current flow: explicit `--root` or nearest ancestor with `.git`/`.contextforge` → canonical root → validated config → status or bounded file inventory → CLI output / MCP JSON. Explicit roots can be non-Git directories. `.git` detection is a marker check, not validation of Git object databases; worktree `.git` files are recognized.

`init` exclusively creates `.contextforge/config.toml` using a synchronized temporary file and atomic no-clobber publication. It preserves existing valid configuration and never edits `.gitignore`. Only explicit CLI `index` or memory clear/rebuild mutate database records; MCP, status and doctor remain read-only. `index_root` defaults to `index` relative to `.contextforge`; absolute external locations explicitly configure external storage. Relative traversal, unsupported versions, unknown fields, oversized configuration and invalid ignore patterns fail closed. Config v1 has no migration yet; incompatible versions are rejected. See [storage](storage.md) for database migrations and privacy.

## Verified technology choices (2026-10-30)

- Rust stable 1.99.1, edition 2024; pinned toolchain, initial MSRV 1.99 (only 1.99.1 tested). Raise the floor deliberately when needed; no older-compiler compatibility claim.
- [Official rmcp SDK](https://docs.rs/rmcp/3.8.0/rmcp/) 3.8, minimal stdio/server/macros/schema features. The lockfile records actual versions. [SDK examples](https://github.com/modelcontextprotocol/rust-sdk) define the tool macros and child-process client used here.
- [Clap](https://docs.rs/clap/latest/clap/) derive for typed arguments; [Tokio](https://docs.rs/tokio/latest/tokio/) for protocol I/O, timeouts and process tests. Blocking discovery uses a bounded permit and blocking worker.
- [Tree-sitter Parser](https://docs.rs/tree-sitter/latest/tree_sitter/struct.Parser.html) supports `set_language` and parsing with a previous tree; incremental parsing requires applying edits to that tree first. Future grammar versions must be checked together for ABI compatibility. Syntax structure does not prove semantic binding.
- [SQLite FTS5](https://www.sqlite.org/fts5.html) provides literal MATCH phrase retrieval, BM25 and source-backed snippets. Current rusqlite 0.41.1 uses bundled SQLite, parameter binding and immediate transactions to update file snapshots. [SQLite transactions](https://www.sqlite.org/lang_transaction.html) informed the atomic update design; SHA-256 fingerprints detect same-length source edits. Search queries are bound and quoted as literal phrases; operators cannot become FTS syntax. See [search](search.md).
- [git2](https://docs.rs/git2/0.21.0/git2/) 0.21.0/libgit2 1.9.7 reads trees/blobs/indexes and computes `Patch::from_buffers` without repository attributes or external commands. Default network/SSH/HTTPS features are disabled. Installed sources verified the actual APIs. See [impact](impact.md) for snapshot semantics, trust boundaries and limits.
- [rust-analyzer](https://rust-analyzer.github.io/) illustrates compiler-aware analysis; [Tree-sitter](https://tree-sitter.github.io/tree-sitter/) supplies cross-language syntax structure. ContextForge starts with syntax and explicit uncertainty rather than promising compiler-grade cross-language references.

## Discovery and trust boundaries

Only the selected canonical root is inventoried. Ignore handling uses `ignore` with repository `.gitignore`, `.contextforgeignore`, and config `ignore_patterns` in gitignore syntax. User-global Git ignores, parent ignore lookup, `.ignore` files and Git exclude files are disabled for reproducibility and scope. Ignore files are checked before directory descent: links, unexpected types and files over 64 KiB fail closed. Built-in exclusions cannot be overridden: `.git`, `.contextforge`, `.tools`, `target`, `node_modules`, `dist`, `build`, `__pycache__`, `.venv`, and the configured index location.

Inventory supports `.rs`, `.py`, `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`. These languages also support current-source Tree-sitter outlines; see [parsing](parsing.md). Content and paths must be UTF-8; binary NUL data is skipped. Default file size is 1 MiB, configurable up to 16 MiB. Walk order is deterministic, depth capped at 64, entries capped at 10000, output capped at 1000 files. Map returns relative slash-separated paths, language, byte length and truncation metadata; never source bodies.

Symlinks are not followed; Windows reparse points (including junctions) are skipped. Metadata links are rejected. Parent-directory ignore lookup is disabled. Canonical containment is checked before reading a file. These checks do not eliminate TOCTOU races against an actively hostile process replacing directory entries; handle-relative/no-follow I/O is required before claiming hardened containment. Do not operate on a repository being maliciously modified concurrently. Reaching a directory at the depth limit conservatively marks output truncated. Cancellable traversal is still tracked work.

## MCP lifecycle

`project_map`, `symbol_lookup`, `search_context`, `find_references`, `dependency_graph`, `analyze_change` and `pack_context` are exposed. Repository operations share one worker permit. Inventory/search/symbol/graph requests have ten-second timeouts; Git impact/context have thirty seconds, and Git additionally has a twenty-second cooperative work budget. The parser checks a two-second parse budget per file. See [parsing](parsing.md), [graph](graph.md), [context](context.md) and [impact](impact.md) for evidence and limits. Initialization/routing use `rmcp`; unsupported tools are not advertised. stdout is protocol-only; diagnostics use stderr and application-owned tracing targets to avoid logging peer-controlled SDK fields. Startup times out after thirty seconds. Timed-out/cancelled workers retain their permits until they finish; cancellation does not forcibly interrupt native/OS calls. EOF/transport closure and Ctrl+C terminate the server. Incoming protocol lines are capped at 64 KiB; malformed/oversized lines are discarded by the SDK and subsequent valid messages remain usable. Request-rate/task limits need further hardening; no remote transport is exposed.

## Storage and evidence roadmap

Index records are scoped to canonical repository identity, with schema version 2 and content fingerprints. A transaction replaces changed file text and removes obsolete paths. Definitions, bounded outlines and FTS rows join the same transaction; schema 1 is upgraded additively during explicit indexing. Spans are one-based inclusive lines, zero-based half-open bytes. Definitions, imports, calls and reference candidates remain separate models. Relationships carry resolved/heuristic/unknown status; names alone cannot prove an edge.

`contextforge-context` depends on index/graph/parser/core and owns bounded query decomposition, consistent snapshot retrieval, graph expansion, diversification, deduplication and serialized token estimation. CLI/MCP orchestrate opt-in Git proximity without coupling this library to transport/Git. [Context](context.md) documents the estimator, complete-boundary policy and limits. `memory status/clear/rebuild` reuse index storage; no second knowledge database or stored chat summaries exist. [Knowledge lifecycle](memory.md) distinguishes hash validation from captured-snapshot queries.

All seven MCP tools now exist. SDK request-context cancellation races the bounded worker wait; detached native workers retain their semaphore permit. SQLite read progress, row/statement/database limits and Unix cache hard-link rejection supplement existing path checks. See [security](security.md) for precise guarantees, permissions and unresolved platform/race/process-quota gaps.

## Open decisions

Harness configuration generation stays in the CLI adapter as a read-only preview.
It emits absolute executable/root launch arguments in each documented client
format, without reading or editing harness settings. JSON serializers encode path
strings; Codex TOML uses the compatible basic-string escape subset, verified by
the TOML parser and installed Codex CLI. Control characters and client variable
interpolation in paths fail closed. See [integrations](integrations.md).
Benchmarking and packaging are development-only Python standard-library tools,
separate from runtime intelligence and requiring no production dependencies.

The owner selected the MIT license for public distribution; see the root LICENSE. Cross-platform support is a design goal; assess actual CI results before adding support claims. Automatic harness config updates, actual harness sessions, future persistent migrations, automatic snapshot refresh, peer/process quotas, path races, Windows cache permissions and broad benchmark design remain tracked work.


## Desktop adapter and shared services

`contextforge-service` owns shared orchestration, project registrations and MCP
configuration plans, and re-exports the existing engines. CLI and MCP retain
protocol/argument handling; Desktop uses typed Tauri commands. No engine depends
on Tauri. Native blocking workers and SQLite transactions keep UI work separate
from repository operations. Index writers also use bounded-wait OS file locks,
including cancellation before commit. See [desktop architecture](desktop-architecture.md).
