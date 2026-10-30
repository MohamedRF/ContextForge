# Changelog

Changes will be recorded here before each release. There are no published releases yet.

## Unreleased

### Added

- Rust workspace with a `contextforge` CLI and versioned project configuration.
- Root-scoped source discovery for Rust, Python, TypeScript and JavaScript files.
- Incremental SQLite source snapshots with SHA-256 fingerprints, repository isolation, transactional updates and interrupted-writer recovery.
- CLI initialization, indexing, status, doctor, source inventory and JSON output.
- Official `rmcp` stdio server exposing the read-only `project_map` tool.
- Bounded Tree-sitter outlines for Rust, Python, TypeScript/TSX and JavaScript/JSX through `analyze` and MCP `symbol_lookup`, with parent IDs, exact spans and unresolved syntax occurrences.
- Additive SQLite schema 1→2 migration for persisted syntax and FTS5, atomic invalidation, deterministic cited CLI search and MCP `search_context` with bounded pagination.
- Pure bounded syntax graphs with typed containment/call/import/export evidence, heuristic/unknown targets, CLI graph/references and MCP dependency_graph/find_references.
- Read-only Git impact for working/staged/merge-base/explicit-file modes, old/new source spans, likely dependents/test candidates, skipped content, CLI impact and MCP analyze_change.
- Serialized-budget context library, CLI context and MCP pack_context with consistent snapshot retrieval, optional Git proximity, bounded graph evidence, diversity and overlap suppression.
- Persistent structural knowledge status/clear/rebuild, source hash validation and atomic forced reparse/reset using existing schema-2 storage.
- MCP cancellable worker waits, SQLite database/row/statement/read-work limits, Unix cache hard-link rejection and documented threat model.
- Regression tests for filesystem boundaries, Unicode, index lifecycle and MCP protocol handling.
- MIT license, contributor documentation, issue/PR templates and a CI quality workflow.

### Limitations

- Context token counts use an explicit estimate, not a model tokenizer; syntax dependencies do not prove semantic binding or change safety.
- No published crates, prebuilt binaries or verified harness-specific setup helpers.
- Ordinary status/search/graph/context use labeled captured snapshots; memory status validates observed source hashes. Hostile filesystem/cache, Windows permissions/hard links, process quotas and dependency audits remain open.
