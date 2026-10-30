# Syntax outlines

`contextforge --root <directory> --json analyze <relative-file> --limit 200`
reads current eligible source without creating metadata. MCP `symbol_lookup`
uses the same engine with `path` and optional `limit`. Discovery/root/link/ignore
and file-size policies apply. A file outside the first 1000 discovered files
cannot be analyzed in this initial implementation; choose a narrower authorized
root when appropriate. There is no cached-source freshness claim.

Tree-sitter 0.27.1 loads Rust 0.24.2, Python 0.25.0, TypeScript/TSX 0.23.2
and JavaScript/JSX 0.25.0 grammars. Installed grammar sources, node fields,
tag queries and Rust bindings were inspected and ABI compatibility is tested.
See the [official parser API](https://docs.rs/tree-sitter/0.27.1/tree_sitter/struct.Parser.html).

The result contains repository identity, relative path, language, SHA-256 source
fingerprint, `method: tree_sitter_syntax`, definitions (`symbols`), separate
`occurrences`, `has_errors` and `truncated`. Definitions include name, kind,
signature, declared visibility, parent ID, span and local `contains_errors`.
Lines are one-based inclusive; bytes are zero-based half-open UTF-8 offsets.
End lines exclude the following line when the node ends at column zero.

IDs hash repository identity, path, parent identity, definition kind, full name
and same-parent duplicate ordinal. They survive unrelated line shifts and body
edits; rename/move/reparent/kind changes create a new ID. Reordering duplicate
names can change which declaration receives an ordinal. IDs are syntax identity,
not compiler binding or cross-repository identifiers. Display names/signatures
and occurrence text are clipped to 256 Unicode scalar values; spans retain the
full evidence. Identity always uses the full name.

Supported definitions include named functions/methods, Rust modules, structs,
enums, traits, implementations, constants/static items and type aliases; Python
classes and nested/decorated functions; JS/TS classes, named functions and
variable-bound arrow/function expressions; TS interfaces, enums, types and
namespaces. Other anonymous callbacks, destructured bindings and dynamically
assigned members are omitted. Rust visibility distinguishes public/restricted/
private; TS accessibility modifiers are reported when declared; other visibility
is unspecified. Export syntax is separate evidence, not inferred visibility.

Calls, imports and exports are separate source occurrences with `status: unknown`.
No target is resolved, no dependency edge is asserted, and macro expansions,
conditional compilation and type checking are not performed. Syntax errors set
`has_errors`; recoverable nodes are returned with valid spans and individual
error flags. This is not a complete compiler diagnostic list.

The combined definition/occurrence limit is 1–1000 (default 200). Traversal stops
after 100000 nodes or depth 256 and reports truncation. Parsing uses the SDK
progress callback with a two-second budget; callbacks are cooperative, not a
hard process CPU/memory quota. Input is capped at 16 MiB and the configured
file limit. MCP additionally applies a ten-second operation timeout and a single
blocking-operation permit retained until the worker finishes. Cancellation of
the client request does not forcibly stop filesystem I/O.

CLI analysis and MCP symbol outlines read current source. Explicit `index`
additionally persists bounded outlines and definitions in schema 2; indexed
search reads this captured snapshot. See [storage](storage.md) and [search](search.md).
