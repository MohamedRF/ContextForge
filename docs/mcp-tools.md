# Implemented MCP interface

Start `contextforge --root <authorized-directory> mcp`. The root is selected once at startup; clients cannot select other roots through tool arguments. No initialization or source writes occur.

`project_map`: input `{ "limit": 100 }` (optional limit, integer 1–1000). Unknown properties are rejected. Output has `files` (`path`, `language`, `bytes`), `truncated`, `inspected_entries`, `method: "filesystem_source_inventory"`. Paths are repo-relative. This is a bounded source inventory, not a symbol or dependency graph. JSON structured content and SDK-generated schemas are provided.

Empty inventories are successful. Explicit out-of-range limits are JSON-RPC invalid-parameter errors; rmcp reports parameter-deserialization failures (wrong types or unknown properties) as tool errors with `isError: true`. I/O failures return safe errors without source bodies or absolute paths. Only one discovery operation may run at once; busy requests fail with a retry-later message. See architecture for limits and cancellation caveats.

Search, symbols, graph, Git analysis and context packing are not yet implemented or advertised. Harness-specific setup remains unverified; no external settings are modified.
