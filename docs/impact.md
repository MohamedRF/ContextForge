# Read-only Git change impact

Commands: `impact` (index to working tree), `impact --staged` (HEAD to index),
`impact --base <ref>` (merge-base(ref, HEAD) to HEAD),
`impact --base <ref> --head <ref>` (merge-base(base, head) to head), and
`impact --file <relative-path>` (whole selected file, even unchanged).
`--depth` is 0–8 (default 3); `--limit` is 1–500 graph nodes (default 100).
Modes are mutually exclusive. `--head` requires `--base`. Refs accept names/OIDs
and bounded ancestry syntax using alphanumerics, `/_.-~^`; revision searches,
path selectors and arbitrary revparse expressions are rejected.

MCP `analyze_change` accepts optional `staged`, `base`, `head`, `file`, `depth`
and `limit` with the same semantics; `{}` is unstaged. Unknown fields and mode
conflicts are rejected. The authorized root must be the Git worktree root;
ancestor repository discovery is disabled. Explicit file mode also requires Git.
Unborn HEAD is supported for staged additions; detached HEAD works. Missing refs,
conflicted indexes, inaccessible content and resource limits return safe errors.

`git2` 0.21/libgit2 1.9.7 runs locally with default network/SSH/HTTPS features
disabled. Runtime code opens repositories, reads trees/blobs/indexes and computes
buffer patches; it never invokes a shell/Git executable, checks out files, stages,
writes the index, executes hooks, runs tests or refreshes ContextForge metadata.
Diffs use `Patch::from_buffers` without repository attributes, so external diff,
textconv and project filter commands are not executed. Git metadata (including
worktree common stores/alternates/configuration) must be trusted; this is not a
Git object/config sandbox. Source read containment retains the existing documented
hostile-filesystem race limitations.

Current root-scoped ignores, built-in exclusions, file size and link policies
apply to historical paths too. Normal source blobs only are parsed. Untracked
eligible text is included in working-tree mode; new untracked binary/oversized
files are excluded by discovery. Changed tracked binary/non-UTF-8/oversized or
non-regular entries are explicit skipped evidence; known syntax from the other
side is retained. `incomplete_content` marks such changes. Renames are deletion
plus addition, including edited renames, without claiming a guessed rename match.
Line-ending changes are compared as captured bytes; no checkout filters run.

Zero-context changed hunks have separate old/new start lines and line counts;
zero counts mean an empty side, not a real line zero. Overlapping definitions
(including containing modules/types) appear in `previous_symbols` and
`current_symbols`. Fingerprints and graph IDs use canonical native-root identity,
matching normal parser/index IDs. Views and fingerprints distinguish previous
and current dependent/evidence records. Git snapshots are captured sequentially,
not atomically across concurrent edits. `freshness` is `captured_git_snapshots`;
cached SQLite evidence is not used for impact.

Both old and new graphs seed changed definitions and file nodes; incoming
traversal finds likely direct/indirect dependents. `resolved_dependents` is kept
separate and is normally empty with the current syntax-only resolver.
`heuristic_dependents`, `dependency_evidence` and `unresolved` retain uncertainty.
Test suggestions are a subset of reached dependents selected by path/name
(`tests`, `test_*`, `.test.`, `.spec.`); this is a heuristic, not proof of coverage.
Depth, node and evidence bounds set `truncated`. Partial/error syntax is explicit.
No absence of dependencies proves a change safe.

Limits: each view has at most 1000 source files, 10000 Git entries, tree depth 64
and 64 MiB of text, with the configured per-file size cap. Graph limits also apply.
At most 100 changed files, 1000 total hunks and 1000 directly affected definitions
across both views are accepted; exceeding these aborts rather than returning a
silently incomplete direct-change set. Dependents are capped by `limit`, evidence
and unknown records at 1000 each. A twenty-second cooperative budget is checked
between source parses and after diff work; a single native/OS call is not forcibly
interrupted. MCP adds a thirty-second timeout, retaining the shared permit until
the worker finishes. Strict process memory/CPU quotas remain future hardening.

Tests create isolated local Git fixture commits with a synthetic author. They
verify central edit → caller → test, staged/unstaged separation, removed symbols,
merge bases, detached/unborn HEAD, missing refs, binary input, ignored history,
rename-as-delete/add, bounds, CLI/MCP calls, unchanged Git index/HEAD/source and no
ContextForge metadata creation. They do not execute fixture code or project hooks.
