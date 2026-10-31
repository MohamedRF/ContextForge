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

## Release CLI measurements (2026-10-30)

[Raw samples](../benchmarks/results/windows-2026-10-30.json) were collected with
the standard-library [Python runner](../benchmarks/run.py). Reproduce from the
repository root with Python 3.11+ and a built release executable:

```text
cargo build --workspace --release --locked
python benchmarks/run.py --binary target/release/contextforge.exe --files 100 1000 --repeats 3
```

On Unix use `target/release/contextforge`. The runner prints JSON to stdout,
creates isolated temporary fixtures and removes only those fixtures. It never
uses a user repository. Counts are restricted to 4–1000; 10000-file indexing is
outside the current 1000-file cap and was not benchmarked. No limits were weakened.

Environment: Windows build 26200 x86_64 (Python identifies it as
Windows-10-10.0.26200-SP0), Intel i7-7700 @ 3.60 GHz, 42818826240 bytes RAM,
8 logical CPUs, Rust 1.99.1 Windows GNU, MinGW GCC 16.2, release profile.
Executable SHA-256 is recorded in the raw report. No development build or test
ran concurrently with these samples; background OS activity, antivirus and warm
filesystem caches were not controlled. This is three samples on one host.

Dataset: deterministically generated equal Rust/Python/TypeScript/JavaScript
files. Each defines one unique numbered function calling the next in a cycle.
These cross-file/cross-language name matches deliberately remain syntax
heuristics, not executable projects or semantic ground truth. The report stores
a SHA-256 over filenames and source bytes. File generation and init are outside
the measured operations. Each operation uses a fresh CLI process; timings include
startup, config, retrieval and JSON output. Unchanged indexing still reads/hashes
all eligible files. The single-edit sample appends a comment to the first file.

Median wall time in milliseconds:

| Files | Cold index | Unchanged | Single edit | Search | File outline | Graph | Context |
|---|---:|---:|---:|---:|---:|---:|---:|
| 100 | 256.06 | 128.94 | 143.08 | 33.64 | 83.15 | 35.03 | 36.24 |
| 1000 | 3297.53 | 965.87 | 998.49 | 35.04 | 493.83 | 54.35 | 55.97 |

Windows resource metrics use native GetProcessMemoryInfo/GetProcessTimes after
each child exits: peak working set and kernel+user CPU time. CPU percent in the
raw report is CPU seconds / wall seconds × 100, relative to one logical core;
it is not total-machine CPU utilization. Non-Windows resource fields are `null`
until an equivalent measured backend is added.

| Files | Maximum cold-index peak working set | Median cold CPU seconds | First-trial index bytes |
|---|---:|---:|---:|
| 100 | 11694080 | 0.140625 | 217088 |
| 1000 | 14528512 | 1.234375 | 1658880 |

SQLite size can vary by inserted fingerprints/root identity; per-trial values
are retained. The context package is 304 estimated tokens in all six trials.

The quality task is literal `task_0`; expected files contain its definition and
sole incoming caller. Basic substring scanning, indexed search and context all
retrieve exactly those two files: precision 1.0, recall 1.0 on this **single
synthetic name task**. No semantic strategy exists. The Python keyword scan
times exclude Python startup; CLI times include process startup, so they do not
support a direct speedup claim. Corpus and expected files are shared for quality
comparison only. Downstream developer task success, broad retrieval quality and
token savings were not measured. More realistic labeled task corpora remain
future work.
