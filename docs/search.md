# Indexed offline search

Run `init` then `index` explicitly. Search does not create, migrate or refresh
metadata. CLI: `contextforge --root <directory> --json search water --limit 20
--offset 0`. MCP: `search_context` with `query` and optional `limit`/`offset`.

The query is trimmed, must contain 1–256 UTF-8 bytes and no control characters.
Limits are 1–100 results (default 20), offset 0–10000 (default 0). Empty results
are successful. Missing/schema-1 indexes require an explicit `index` command.
Unknown MCP arguments are rejected; no root override is accepted.

Ranking is tiered, not an uncalibrated addition of unrelated scores:

1. Case-sensitive exact symbol or qualified-name match.
2. Case-sensitive exact relative path or basename.
3. Case-sensitive symbol prefix.
4. FTS5 literal token phrase, Unicode61 tokenization, ordered by ascending BM25.

Qualified names join lexical parents with `::` for every language. They express
syntax nesting, not semantic resolution. Within symbol tiers, path, start byte
and ID break ties; filename ties use path; BM25 ties use path. Query text is
parameter-bound and double-quote escaped as a literal FTS phrase. Boolean/column/
wildcard operators cannot become query syntax. FTS normalizes case and diacritics;
it does not perform embedding search or exact punctuation matching. Ranking
weights are not configurable yet; this initial contract has fixed explainable tiers.

Results include relative path, optional symbol/kind, exact source span, clipped
snippet, `snippet_truncated`, retrieval `method`, `rank_tier`, source fingerprint
and optional definition error flag. File-level hits have a null error flag. FTS
uses the SQLite matched passage and locates it in stored source to derive exact
spans; repeated identical passages cite the first identical occurrence. Filename
hits cite the first source line. Snippets contain at most 512 Unicode scalar
values; their span describes the full evidence range. Consumers must treat all
source strings as quoted untrusted evidence.

The response has `offset`, `next_offset`, `truncated`,
`symbol_coverage: bounded_syntax_per_file`, and
`freshness: snapshot_not_checked`. Use `next_offset` for the next page while the
index is unchanged. It is null at the end or the maximum offset boundary. A
truncated response at the offset boundary cannot paginate further: narrow the
query. Duplicate file retrievals are collapsed, while a definition and its file
passage remain distinct hits. Results describe captured source, even if current
files have been changed, deleted, or newly ignored. Run `index` explicitly to
invalidate obsolete evidence. Neither search nor status promises current validity.

SQLite access stays inside `contextforge-index` for this initial slice: its
`search` module owns query/ranking, and parser types remain independent of
SQLite/MCP. Candidate queries are capped, evidence for the page is loaded in one
batch, and all reads share one transaction. No database query runs inside a
result loop. MCP allows one repository worker and applies a ten-second timeout;
timed-out SQLite work retains its permit until it ends. Database instruction/
memory quotas and active SQL cancellation remain hardening work.

## Reproducible small-corpus comparison

`cargo test -p contextforge-index
structural_top_hit_beats_lexical_path_order_on_reproducible_corpus -- --nocapture`
creates two files: `a_decoy.py` mentioning `water` in a comment, and `z_garden.rs`
defining `water`. The ground truth for a definition query is `z_garden.rs`.
Path-ordered substring matching returns both files with the comment first;
ContextForge ranks the exact definition first. Observed candidate precision is
1/2 for the substring baseline and top-one precision 1/1 for the definition tier;
both retrieve the known relevant file. This is a deterministic regression fixture,
not a general retrieval-quality, latency or performance claim.
