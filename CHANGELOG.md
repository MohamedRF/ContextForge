# Changelog

Changes will be recorded here before each release. There are no published releases yet.

## Unreleased

### Added

- Rust workspace with a `contextforge` CLI and versioned project configuration.
- Root-scoped source discovery for Rust, Python, TypeScript and JavaScript files.
- Incremental SQLite source snapshots with SHA-256 fingerprints, repository isolation, transactional updates and interrupted-writer recovery.
- CLI initialization, indexing, status, doctor, source inventory and JSON output.
- Official `rmcp` stdio server exposing the read-only `project_map` tool.
- Regression tests for filesystem boundaries, Unicode, index lifecycle and MCP protocol handling.
- MIT license, contributor documentation, issue/PR templates and a CI quality workflow.

### Limitations

- No Tree-sitter parsing, symbol lookup, search, dependency graph, Git impact or context packing yet.
- No published crates, prebuilt binaries or verified harness-specific setup helpers.
- Source freshness is not checked by status; additional filesystem/cache hardening remains planned.
