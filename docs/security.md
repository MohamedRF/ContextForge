# Security model and known limits

This development binary serves local stdio only. It has no HTTP listener, authentication service, telemetry, network retrieval, source upload or runtime shell execution. Native git2 reads local objects/index and computes buffer diffs without project hooks, textconv or external diff commands. Build/test commands used during development are not runtime tools. See [reporting](../SECURITY.md).

## Trust boundaries

| Input or resource | Policy |
|---|---|
| Source files, comments, names, AGENTS.md text in indexed projects | Untrusted quoted evidence; no interpretation as commands or privileged instructions |
| Authorized repository root | Fixed at CLI/server startup; tool arguments cannot replace it |
| Filesystem paths | Relative normal components for source requests; canonical containment, scoped ignore checks and per-component link/reparse rejection |
| Managed cache and Git metadata | Must be owned/trusted by the local user; schema/root identity checked, unsupported/corrupt stores preserved |
| MCP peer | Local client with access to selected source; strict argument shapes, explicit ranges and bounded frames/results |
| Harness settings | No configuration writes implemented; future changes require explicit approval |

No repository script, comment, test command or executable is run while indexing/retrieving. Source data is returned intentionally to the authorized client; this is not secret detection or source redaction. Application diagnostics exclude source bodies, query values, peer cancellation reasons and absolute roots. Third-party tracing targets are filtered to avoid SDK dumps of peer-controlled fields. CLI human source output escapes controls; context JSON escapes source content.

## Resource and storage policy

Discovery: depth 64, 10000 entries, 1000 eligible files; per-file 1 MiB by default, configurable to 16 MiB; total indexed source 64 MiB. Truncated/failed capture cannot remove old rows. Parser time/occurrence bounds, graph depth/node/edge limits and Git work deadlines are documented in their feature docs. Context caps combined captured source/outlines at 128 MiB, queries at nine, snippets at 32 and whole evidence boundaries at 16 KiB, with an explicit [serialization estimator](context.md).

SQLite database and known journal/WAL/shared-memory sidecar files are rejected above 512 MiB before opening. Write connections cap the main database at 512 MiB using actual page size. Rows/strings are limited to 32 MiB, SQL statements to 64 KiB, and attached databases are disabled. Read connections interrupt SQL every 10000 VM instructions after 100 million instructions or five elapsed seconds for that connection, whichever occurs first. This is cooperative SQL work accounting, not an OS memory/CPU sandbox; blocking OS calls and some native operations are outside it. SQLite lock waits have a two-second timeout. Write transactions retain existing bounded-input behavior; they are not aborted by the read query budget.

MCP frames are capped at 64 KiB. One blocking repository worker owns a semaphore permit; expensive concurrent requests fail with a retry message. Worker waits time out at ten seconds (30 for Git/context). MCP cancellation stops waiting with a safe internal error (the SDK suppresses responses for cancelled requests); native work retains the permit until finished, preventing overlapping orphan workers. Cancellation does not forcibly stop SQLite/filesystem/libgit2 operations. SDK protocol tasks/rate and total process RSS still lack strict quotas; hostile peer flooding remains outside the hardened guarantee. No remote endpoint is offered.

New Unix cache directories use 0700. Existing permissions are preserved; keep the directory and its parents private. Unix cache databases and known sidecars with multiple hard links are rejected. Windows inherits parent ACLs; choose a private local directory. Windows hard-link detection, active ACL validation and other-platform verification of the new changes remain open. Logical cache clearing does not guarantee physical erasure.

## Remaining limits

Path checks do not eliminate TOCTOU races against an actively hostile process swapping entries after validation. Handle-relative/no-follow APIs are needed for that guarantee. Hard-linked source files and Windows cache hard links cannot be conclusively confined by canonical-path checks. Avoid actively hostile/concurrently replaced directories and attacker-controlled cache/Git databases. The schema is validated for compatibility/identity, not authenticated against tampering. Cached snapshots can be stale by design; `memory status` validates observed hashes, while search/graph/context explicitly label unchecked freshness. Cancellation/timeouts are cooperative, and shutdown may wait for a blocking worker to return.

Tests cover traversal/absolute paths, Windows junctions, Unix symlinks, malformed frames/arguments, literal SQL/FTS queries, injection-looking comments as data, Unicode/escaping budgets, oversized databases, SQLite work/row/statement limits, transactional failure/restart, source/cache integrity, cancellation and timeouts. Unix-only tests require Unix runners. `cargo audit` and `cargo deny check` must be recorded separately; unavailable tools are never treated as a clean audit. This document defines current protections and gaps, not a completed security audit or release approval.

On 2026-10-30, locally installed cargo-audit 0.22.2 checked 175 locked dependencies against 1395 RustSec advisories (database commit `2295466c4e9df26647fd67d8fa1c705ac2fc7f81`): zero known vulnerabilities and no warnings. No advisory suppressions were used. This is a point-in-time dependency advisory check, not proof that the application or bundled native code has no vulnerabilities. `cargo deny check` and new-source Unix/other-platform validation were not run; see [the validation ledger](../TASKS.md).
