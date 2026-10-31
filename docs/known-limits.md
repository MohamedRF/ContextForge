# Known implementation limits

- Syntax is Tree-sitter evidence, not compiler binding. Call targets are heuristic
  or unknown; impact results cannot establish that a change is safe.
- Indexing accepts at most 1000 source files, 10000 walk entries, depth 64 and
  64 MiB source snapshots. A larger/truncated inventory fails atomically; it does
  not silently delete previously indexed records. Large 10000-file performance
  is not supported or claimed.
- Search/graph/context use captured snapshots. Run `index` explicitly to refresh;
  `memory status` compares observed source fingerprints.
- Context budgets use a documented UTF-8 byte estimate over compact package JSON,
  not a model tokenizer. MCP/client envelope overhead is excluded.
- Source snapshots may retain embedded secrets. Keep caches and backups private;
  no source-secret exclusion guarantee is made.
- Path containment does not eliminate hostile concurrent replacement races.
  Windows cache ACL/hard-link checks and strict peer/process resource quotas are
  incomplete. See [the threat model](security.md).
- Setup is preview-only. No automatic harness configuration merge, backup or
  modification is implemented. Generated formats/launch descriptors are tested;
  actual agent-session calls are unverified. See [integrations](integrations.md).
- Current-source CI passed on Windows, Linux and macOS for the desktop preview
  commit. Local Windows GUI/package evidence does not establish macOS GUI,
  clean-machine or installer-lifecycle behavior.
- Benchmarks are tiny synthetic cyclic name tasks, three samples per size on one
  host. They demonstrate measurements and regression behavior, not general code
  understanding quality, token savings or downstream task success.
- Published binaries are an unsigned desktop developer preview. No crates.io
  packages, clean-machine installs, HTTP transport, embeddings, native harness
  plugins or autonomous agent memory are provided.

See [TASKS.md](../TASKS.md) for acceptance status and [release gates](release.md)
for remaining publication work.
