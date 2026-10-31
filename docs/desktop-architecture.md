# Desktop architecture

## Existing implementation

The Rust 2024 workspace contains core (root authorization/configuration/discovery),
parser (bounded Tree-sitter syntax), index (SQLite source snapshots and FTS5),
graph (bounded evidence-qualified traversal), Git impact and context packing.
The CLI embeds the rmcp stdio server. There is no separate MCP executable: the
`contextforge mcp` entry point serves independently of the GUI.

Configuration and indexes currently live under each authorized repository's
`.contextforge` directory, with optional configured absolute cache roots. Keep
this canonical format for backward compatibility. Moving all existing caches
into a per-user directory would require an explicit migration design.

Index writes already use IMMEDIATE SQLite transactions, a busy timeout and
fingerprint comparison against the committed snapshot. Schema migration occurs
inside the same writer transaction. Readers use snapshot transactions. Do not
enable WAL casually: existing sidecar validation and recovery must remain valid.

## Implementation plan

1. Introduce a framework-independent service facade over existing engines and
   extract shared orchestration from the CLI. Route all adapters through it.
2. Store project registrations and desktop preferences in a versioned per-user
   SQLite registry. Use unique canonical paths and transactional registration.
   Removing a registration never removes indexes or source files.
3. Add Tauri 2 + React/TypeScript/Vite/Tailwind, TanStack Query and typed IPC.
   Native folder selection happens in Rust; webview callers use registered IDs.
4. Run bounded expensive work on blocking workers. Emit correlated, throttled
   indexing events. Cancellation must roll back writes rather than stop a thread.
5. Expose existing search, outlines, graph, impact and context algorithms, keeping
   snapshot freshness, heuristic relationships and truncation visible.
6. Package companion CLI and GUI from one workspace commit. Gate release claims
   on actual build and runtime checks; unsigned previews are prereleases.

## Boundaries and risks

Tauri owns native dialogs, events and windows only; Rust engines contain no Tauri
dependencies. The webview receives no filesystem/shell plugin. Source reads pass
through existing root, ignore, size and symlink checks. Repository content is
untrusted evidence, rendered as escaped text. No HTTP server or updater endpoint
is needed. Existing documented hostile concurrent filesystem replacement and
cache-permission limitations remain relevant (see security.md).

The registry is shared persistent state, while `.contextforge` remains the sole
index/configuration format. CLI and MCP do not need the desktop process running.
Native platform tests, real harness connectivity, signing and clean-machine
installer tests must be recorded separately from browser and unit tests.
