# Changelog

Changes are recorded here before each release. The first desktop developer preview
was published on 31 October 2026 as
[`v0.1.0-desktop-preview.1`](https://github.com/MohamedRF/ContextForge/releases/tag/v0.1.0-desktop-preview.1).

## 0.1.0 — published as v0.1.0-desktop-preview.1

### Added

- Preview-only `setup` for Claude Code, Codex, Cursor and VS Code with documented formats, absolute launch paths, JSON output and real MCP descriptor tests.
- Isolated installed-Codex configuration check; deterministic malformed Unicode parser and bounded cyclic graph invariant tests.
- Repeatable release CLI benchmarks over four-language 100/1000-file fixtures, raw metrics and scoped retrieval quality measurements.
- Native development ZIP/checksum packaging with extracted-binary smoke checks, manual artifact workflow, known-limits document and release checklist.

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
- No published crates or actual harness-session verification. Published binaries are an unsigned developer preview; setup is preview-only and requires manual merge.
- Ordinary status/search/graph/context use labeled captured snapshots; memory status validates observed source hashes. Hostile filesystem/cache, Windows permissions/hard links and process quotas remain open; local cargo-audit reports no known advisories at the recorded database revision.


## Desktop preview (0.1.0)

- Add a Tauri 2 / React / TypeScript desktop adapter using shared Rust services.
- Persist project registrations and themes; add CLI `projects` registry commands.
- Add native folder selection, bounded indexing progress, cooperative cancellation
  and OS file locks coordinating writers across processes.
- Expose cited search, bounded source outlines, dependencies, Git impact and
  context packing without duplicating retrieval or ranking.
- Add MCP configuration review, native confirmation, backup, atomic merge/removal
  and syntax/entry validation. Actual harness connectivity remains unverified.
- Add Windows/macOS artifact workflows, checksums, architecture and user guides.
- This is an unsigned developer preview. Signing, notarization and full production
  acceptance are not claimed; see release notes for actual validation evidence.
- Respect Git's CRLF/text-attribute conversion during unstaged impact analysis
  while retaining raw source fingerprints and detecting raced snapshots.
- Avoid Tailwind utility-name collisions in sidebar and source-outline styles.
