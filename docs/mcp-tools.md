# Implemented MCP interface

Start `contextforge --root <authorized-directory> mcp`. The root is selected once at startup; clients cannot select other roots through tool arguments. No initialization, source writes or implicit index updates occur.

`project_map`: input `{ "limit": 100 }` (optional integer 1–1000). Unknown properties are rejected. Output has `files` (`path`, `language`, `bytes`), `truncated`, `inspected_entries`, `method: "filesystem_source_inventory"`. Paths are repo-relative. This is a bounded source inventory, not a dependency graph. JSON structured content and SDK-generated schemas are provided.

`symbol_lookup`: input `{ "path": "src/lib.rs", "limit": 200 }`. Path is required and repo-relative; limit is optional, 1–1000 combined definitions/occurrences. Unknown fields, absolute paths, traversal, ignored/missing/binary/oversized files are rejected. Returns a current-source [syntax outline](parsing.md) with exact spans, parent IDs, fingerprint, error/truncation flags and unresolved occurrence evidence. No index or source writes.

`search_context`: input `{ "query": "water", "limit": 20, "offset": 0 }`. Query is required; limit is optional, 1–100; offset is optional, 0–10000. Returns ranked cited snapshot evidence with fingerprints, pagination, bounded syntax coverage and unchecked freshness. Never creates, migrates or refreshes an index; missing/schema-1 indexes require explicit CLI indexing. See [search](search.md).

Empty inventories/search results are successful. Explicit invalid values are JSON-RPC invalid-parameter errors; rmcp reports parameter-deserialization failures (wrong types or unknown properties) as tool errors with `isError: true`. I/O failures return safe errors without source bodies or absolute paths. Only one repository operation may run at once; busy requests fail with a retry-later message. See architecture for limits and cancellation caveats.

`dependency_graph`: required `selector`; optional `direction` (`outgoing` default or `incoming`), `depth` 0–8 (default 2), `limit` 1–500 (default 100), `edge_limit` 1–1000 (default 200). Returns bounded typed graph evidence with uncertainty, truncation, partial syntax and unchecked snapshot freshness.

`find_references`: required `selector`, optional `limit` 1–500 (default 100). Returns candidate incoming relations and matching unknown call evidence. Neither tool claims compiler-proven binding. See [graph](graph.md).

`analyze_change`: optional mutually exclusive `staged: true`, `base` or `file`; `head` requires `base`; optional `depth` 0–8 and `limit` 1–500. Default `{}` compares index to working tree. Returns cited old/new changed symbols, likely dependents, test candidates and unresolved evidence. No shell execution, Git/index/source writes or implicit ContextForge refresh. Git impact has a thirty-second timeout; graph operations have ten seconds, sharing one worker permit. See [impact](impact.md).

`pack_context`: required `query` (1–256 non-control UTF-8 bytes); optional `budget` 256–32000 (default 4000) and `include_git` boolean (default false). Returns a compact package with estimator name, self-inclusive `estimated_tokens`, budget, exact source spans/fingerprints, deduplicated snippets, optional heuristic relation evidence and truncation/partial-syntax flags. Literal request plus bounded term searches, source text and outlines share one read transaction. See [context packing](context.md). Git proximity requires explicit opt-in and valid bounded Git snapshots. This does not refresh sources or assert freshness. The package's compact JSON is budgeted; MCP envelope/text duplication and client prompt overhead are not. Timeout: 30 seconds.

All seven tools observe the SDK request cancellation token. Cancellation stops waiting with a safe internal error; the SDK suppresses responses for cancelled requests, and the native worker keeps its shared permit until actual completion. Ten-second worker waits apply except Git/context (30 seconds). SQL read work also has a cooperative five-second/100-million-VM-instruction cap per connection. Blocking OS/native operations are not forcibly terminated. Raw-protocol cancellation tests and unit permit/timeout tests cover these contracts. See [security limits](security.md).

Client-specific setup previews and launch descriptors are tested; actual
harness-session tool calls remain unverified. No external settings are modified.
See [integration formats and evidence](integrations.md).


Desktop shares these engines through `contextforge-service`; it never runs the
CLI subprocess for retrieval. MCP is still the independent `contextforge mcp`
entry point. The integration manager offers preview/confirm/backup/apply and
syntax validation without claiming harness connectivity. See the [desktop guide](desktop-user-guide.md).
