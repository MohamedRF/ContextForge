# Architecture

ContextForge is an offline, local-first code intelligence engine. The MVP is safe discovery → incremental SQLite index → Tree-sitter definitions → cited search → stdio MCP. No LLM, telemetry, source uploads, GUI, or embedding service is required. No runtime component executes project scripts or shell commands.

## Initial boundaries and flow

`contextforge-cli` owns Clap arguments, stderr tracing, command output and the `rmcp` adapter. `contextforge-core` owns the authorized root, versioned configuration and bounded discovery/source reads. `contextforge-index` owns SQLite schema, isolated cache paths and transactional incremental source snapshots. The core does not depend on Tokio, Clap, SQLite or MCP. Extract parser/search crates when those engines exist; do not create empty crates in advance.

Current flow: explicit `--root` or nearest ancestor with `.git`/`.contextforge` → canonical root → validated config → status or bounded file inventory → CLI output / MCP JSON. Explicit roots can be non-Git directories. `.git` detection is a marker check, not validation of Git object databases; worktree `.git` files are recognized.

`init` exclusively creates `.contextforge/config.toml` using a synchronized temporary file and atomic no-clobber publication. It preserves existing valid configuration and never edits `.gitignore`. Only explicit CLI `index` creates/updates database metadata; MCP, status and doctor remain read-only. `index_root` defaults to `index` relative to `.contextforge`; absolute external locations explicitly configure external storage. Relative traversal, unsupported versions, unknown fields, oversized configuration and invalid ignore patterns fail closed. Config v1 has no migration yet; incompatible versions are rejected. See [storage](storage.md) for database migrations and privacy.

## Verified technology choices (2026-10-30)

- Rust stable 1.99.1, edition 2024; pinned toolchain, initial MSRV 1.99 (only 1.99.1 tested). Raise the floor deliberately when needed; no older-compiler compatibility claim.
- [Official rmcp SDK](https://docs.rs/rmcp/3.8.0/rmcp/) 3.8, minimal stdio/server/macros/schema features. The lockfile records actual versions. [SDK examples](https://github.com/modelcontextprotocol/rust-sdk) define the tool macros and child-process client used here.
- [Clap](https://docs.rs/clap/latest/clap/) derive for typed arguments; [Tokio](https://docs.rs/tokio/latest/tokio/) for protocol I/O, timeouts and process tests. Blocking discovery uses a bounded permit and blocking worker.
- [Tree-sitter Parser](https://docs.rs/tree-sitter/latest/tree_sitter/struct.Parser.html) supports `set_language` and parsing with a previous tree; incremental parsing requires applying edits to that tree first. Future grammar versions must be checked together for ABI compatibility. Syntax structure does not prove semantic binding.
- [SQLite FTS5](https://www.sqlite.org/fts5.html) offers MATCH, BM25 and snippets for future search. Current rusqlite 0.41.1 uses bundled SQLite, parameter binding and immediate transactions to update file snapshots. [SQLite transactions](https://www.sqlite.org/lang_transaction.html) informed the atomic update design; SHA-256 fingerprints detect same-length source edits. FTS remains unimplemented; future search queries must be bound and escaped rather than treated as raw FTS expressions.
- [git2](https://docs.rs/git2/latest/git2/) provides native diff/repository APIs; defer adding it until impact work. Prefer its read-only diff APIs to spawning Git with external diff/textconv configuration. Git CLI remains a candidate if measured portability benefits justify it.
- [rust-analyzer](https://rust-analyzer.github.io/) illustrates compiler-aware analysis; [Tree-sitter](https://tree-sitter.github.io/tree-sitter/) supplies cross-language syntax structure. ContextForge starts with syntax and explicit uncertainty rather than promising compiler-grade cross-language references.

## Discovery and trust boundaries

Only the selected canonical root is inventoried. Ignore handling uses `ignore` with repository `.gitignore`, `.contextforgeignore`, and config `ignore_patterns` in gitignore syntax. User-global Git ignores, parent ignore lookup, `.ignore` files and Git exclude files are disabled for reproducibility and scope. Ignore files are checked before directory descent: links, unexpected types and files over 64 KiB fail closed. Built-in exclusions cannot be overridden: `.git`, `.contextforge`, `.tools`, `target`, `node_modules`, `dist`, `build`, `__pycache__`, `.venv`, and the configured index location.

Inventory supports `.rs`, `.py`, `.ts`, `.tsx`, `.js`, `.jsx`, `.mjs`, `.cjs`. This is file classification only, not implemented parsing. Content and paths must be UTF-8; binary NUL data is skipped. Default file size is 1 MiB, configurable up to 16 MiB. Walk order is deterministic, depth capped at 64, entries capped at 10000, output capped at 1000 files. Map returns relative slash-separated paths, language, byte length and truncation metadata; never source bodies.

Symlinks are not followed; Windows reparse points (including junctions) are skipped. Metadata links are rejected. Parent-directory ignore lookup is disabled. Canonical containment is checked before reading a file. These checks do not eliminate TOCTOU races against an actively hostile process replacing directory entries; handle-relative/no-follow I/O is required before claiming hardened containment. Do not operate on a repository being maliciously modified concurrently. Reaching a directory at the depth limit conservatively marks output truncated. Cancellable traversal is still tracked work.

## MCP lifecycle

Only `project_map` is exposed. Initialization and routing are handled by `rmcp`. No unsupported tool names/capabilities are advertised. stdout is protocol-only; tracing and CLI errors use stderr. Only application-owned tracing targets are enabled, preventing SDK diagnostics from logging peer-controlled fields. Session errors expose a safe message while preserving their cause internally. Startup times out after 30 seconds; discovery after 10 seconds. One blocking discovery may run at a time. A timed-out task retains its semaphore permit until it finishes, avoiding accumulating workers. Cancellation does not forcibly stop an active blocking filesystem call. EOF/transport closure and Ctrl+C terminate the server. Incoming protocol lines are capped at 64 KiB; malformed syntax and oversized lines are discarded by the SDK, which can then process the next valid message. Request-rate/task limits need further hardening; do not expose remote transports.

## Storage and evidence roadmap

Index records are scoped to canonical repository identity, with schema version 1 and content fingerprints. A transaction replaces changed file text and removes obsolete paths. Future definitions and related evidence must join the same transaction. Search spans will be one-based inclusive lines, zero-based half-open bytes. Definitions, imports, calls and reference candidates remain separate models. Relationships must carry resolved/heuristic/unknown status; names alone cannot prove an edge. Token packing must budget serialized output using an explicit estimator.

## Open decisions

The owner selected the MIT license for public distribution; see the root LICENSE. Cross-platform support is a design goal; assess actual CI results before adding support claims. Harness configs, persistent migrations, source freshness, request-frame limits, path races, private cache permissions and benchmark design remain tracked work.
