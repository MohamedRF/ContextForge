# Roadmap

This document describes planned release scope. See the [README](../README.md) and [changelog](../CHANGELOG.md) for implemented behavior and the [security model](security.md) for unresolved hardening work.

- v0.1: CLI/config/root policy; discovery and transactional incremental SQLite storage; Rust/Python/TypeScript/JavaScript Tree-sitter definitions; cited FTS5 search and symbol lookup; real stdio MCP calls; verified Claude Code and Codex manual registration documentation.
- v0.2: typed evidence-qualified dependency graph, Git changed ranges and impact, token-budgeted context, persistent freshness and invalidation.
- v0.3: opt-in setup helpers, Cursor/VS Code verification, measured performance, packaging and release automation.
- v1.0: path/resource/security hardening, migrations, cross-platform regression tests, reproducible benchmarks, stable integration contracts and release review.

Optional local embeddings, more grammars, native adapters and authenticated HTTP follow measured needs. None blocks an offline MVP. Each slice requires tests, format, Clippy and release build before completion. External publication and editor settings changes require authorization.
