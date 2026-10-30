# Development

Prerequisites: Git, Rust 1.99.1 with rustfmt/Clippy, and a native C compiler/linker for bundled SQLite. The root `rust-toolchain.toml` pins the compiler and components.

- Windows: use Visual Studio C++ Build Tools with the MSVC Rust target, or a complete compatible MinGW toolchain with the GNU target.
- Linux: install your distribution's C compiler and linker/build tools.
- macOS: install the Xcode command-line build tools.

The baseline host was Windows x86_64 with a complete MinGW GCC 16.2 POSIX/SEH/MSVCRT toolchain. CI is configured for Windows, Linux and macOS; check the actual workflow results before claiming platform support. Machine-specific bootstrap scripts and downloaded toolchains are not distributed in this repository.

Fetching/building dependencies may require network access; the application runtime does not. Keep `Cargo.lock` committed for reproducible application builds. Never commit build artifacts, local caches, or credentials.

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --release --locked
```

Tests construct independent temporary repositories, including Unicode paths, invalid config, ignored/binary/oversized input, Windows junctions and Unix symlinks. CLI tests run the actual binary. MCP tests use the official SDK client over a child-process stdio transport with bounded test timeouts. No private source repositories or user harness configurations are needed.

Run `cargo audit` / `cargo deny check` when installed; do not claim they ran if unavailable. Build/test each OS before adding platform claims. Follow AGENTS.md for reviews, focused changes and source-trust policy.

## Installing a local build

```sh
cargo install --path crates/contextforge-cli --locked
contextforge --help
```

For an isolated installation, add `--root /path/to/install-directory` and invoke its `bin/contextforge` executable. This does not publish a crate or create a GitHub release.
