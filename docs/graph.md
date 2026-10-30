# Syntax dependency graph

`graph <selector> --direction outgoing|incoming --depth 2 --limit 100
--edge-limit 200` and `references <selector> --limit 100` read the captured index.
MCP tools are `dependency_graph` and `find_references`, with matching arguments
(`selector`, optional `direction`, `depth`, `limit`, `edge_limit` for graph;
`selector` and optional `limit` for references). Select by exact ID, lexical
qualified name, simple name or relative file path. Multiple matches are retained.

Nodes represent files and definitions (including modules). Directed edges carry
typed containment, call, import or export evidence with source path/span and
fingerprint-linked nodes. Only lexical containment is `resolved`. A unique same-
language function name or exact lexical qualified method is a `heuristic` call
candidate. Ambiguous, dynamic, malformed, clipped and unmatched targets remain
`unknown` with `to: null`. Receiver calls never match solely by final method name.
Name matching cannot prove binding: parameters/local variables, imports, visibility,
conditional compilation, dynamic dispatch and runtime lookup can invalidate a
candidate. This engine does not claim complete references or compiler semantics.

Conservative import candidates recognize relative JS/TS module literals, Python
absolute `from` modules and Rust `use crate::` module paths when exactly one
same-language file matches. Aliases, symbol imports that require semantic binding,
package resolution and Python relative imports may remain unknown. Export
statements retain unresolved evidence. Source text is data, never instructions.

Outgoing traversal includes containment/dependencies; incoming traversal follows
candidate dependency edges and excludes containment. References also show matching
unresolved bare/qualified call text, marked unknown. Unknown targets cannot be
traversed. Breadth-first traversal is deterministic and cycle safe. Depth is 0–8,
node limit 1–500, evidence limit 1–1000 (references CLI/MCP 1–500).
Seed nodes are known syntactic definitions/files (`resolved` at depth zero), not
proof of a semantic reference. Edges include their source fingerprint and a view
label (`syntax_snapshot` for graph queries; previous/current in Git reports).
Depth/node/edge bounds set `truncated`; empty selectors are invalid and unmatched
valid selectors return empty results. Path certainty propagates the weakest edge.

Construction caps are 1000 outlines, 64 MiB JSON, 20000 nodes and 40000 edges;
exceeding them returns an error rather than silently dropping graph input.
Per-file parser limits/errors set `partial_syntax`. `freshness` is always
`snapshot_not_checked` for cached queries. Reindex explicitly after changes.
Read-only snapshot export uses one SQLite transaction and never migrates data.
No new persistent format is introduced: graph candidates derive from schema-2
outlines and are recomputed per request. The pure graph crate performs no I/O.

The known three-call cycle fixture asserts all three intended candidate edges
(precision 3/3, recall 3/3 for that fixture only), with zero compiler-resolved
calls. Separate cases assert ambiguous names/dynamic receivers stay unknown,
qualified matches, import evidence, determinism, partial syntax and bounds.
CLI/official MCP client tests verify graph/reference results and unchanged source
and SQLite bytes. These fixtures do not establish general semantic accuracy.
