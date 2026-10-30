# Initial source-index baseline

This is one local diagnostic sample, not a speedup claim or release benchmark.

Environment: Windows x86_64, Intel Core i7-7700 @ 3.60 GHz, Rust 1.99.1 GNU, MinGW GCC 16.2, Cargo debug/test profile, bundled SQLite via rusqlite 0.41.1. Date: 2026-10-30. Background host workload/filesystem caching were not controlled; CPU/RAM utilization and peak memory were not measured.

Fixture: `fixture_baseline_and_source_integrity` in `crates/contextforge-index/tests/lifecycle.rs` creates 100 deterministic Rust files, each containing a single numbered public function, in a temporary directory. It creates config, indexes once, then reindexes unchanged source. Setup/file generation are outside the timed index calls. All content is synthetic; no private source or remote corpus is used.

Reproduce:

```text
cargo test -p contextforge-index fixture_baseline_and_source_integrity -- --nocapture
```

Use your configured Rust toolchain. On Windows, ensure the C compiler and linker match the installed Rust target.

Recorded output:

```text
fixture=100-generated-rust-files cold_ms=115 unchanged_ms=103 sqlite_bytes=28672
```

The assertions verify 100 initially added files, 100 unchanged files on reindex, and unchanged source text. Unchanged indexing still reads/hashes eligible files; the improvement being tested is avoidance of unnecessary database writes. No percentage savings or general throughput claim is inferred. Future Phase 12 work must add repeated samples, release profiling, larger corpora, memory/CPU measurements and comparable retrieval baselines.
