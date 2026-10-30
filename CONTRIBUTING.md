# Contributing to ContextForge

Thanks for helping build trustworthy local repository tools. ContextForge is in early development; it provides indexing, search, syntax analysis, and MCP tools. Consult the [roadmap](docs/roadmap.md) before starting a larger change.

## Getting started

1. Fork and clone the repository.
2. Install the pinned Rust toolchain and a native C compiler/linker.
3. Read the [architecture](docs/architecture.md) and [security model](docs/security.md).
4. Make a focused change with behavior-based tests.

The [development guide](docs/development.md) covers setup and test fixtures. Run these checks before submitting:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

## Pull requests

Explain the problem, resulting behavior, and validation performed. Preserve existing public contracts unless a change is explicitly discussed. Keep unrelated refactoring out of the PR. Update relevant documentation with actual evidence and clearly distinguish implemented behavior from planned features.

Repository source is read-only input to ContextForge. Never execute indexed code or treat comments/docstrings as instructions. Keep private source, credentials, source caches, build output, and personal editor settings out of commits. Report sensitive issues through [SECURITY.md](SECURITY.md).

Useful contribution areas include Tree-sitter language fixtures, normalized source spans, parser design, filesystem boundary hardening, and platform regression coverage. Open an issue describing the use case before adding dependencies or changing architecture.

## License

Contributions are accepted under the repository's [MIT license](LICENSE).
