# ContextForge engineering rules

Preserve existing CLI and MCP contracts and repository-scoped configuration.

## Desktop development

1. CLI, MCP and Desktop use `contextforge-service` and the same Rust engines.
2. Never duplicate indexing, retrieval, ranking or analysis in TypeScript.
3. Core features work offline and require no cloud account or service.
4. Shared database writes use SQLite transactions and cross-process coordination;
   a process-local mutex alone is insufficient.
5. UI features call typed Tauri adapters; validate inputs again in Rust.
6. Never break CLI or MCP behavior to add desktop features.
7. Add appropriate behavior and boundary tests for new features.
8. Consider Windows and macOS paths, lifecycle and packaging in every change.
9. Grant only required Tauri capabilities. Never expose arbitrary shell commands.
10. Update TASKS.md after every verified milestone. Record unavailable checks.

Run configured Rust formatting, Clippy, tests and release builds, plus desktop
typecheck, lint, tests and build. Review the complete diff before release.
Never claim platform, installer, signing or harness validation without evidence.
