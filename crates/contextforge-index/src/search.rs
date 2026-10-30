//! Deterministic cited retrieval from an explicitly captured index snapshot.
use crate::{Error, connection, identity, verify_schema};
use contextforge_core::Repository;
use contextforge_parser::Span;
use rusqlite::params;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

/// Search hit. Snippets are clipped evidence from the stored source fingerprint.
#[derive(Debug, Serialize)]
pub struct SearchHit {
    pub path: String,
    pub symbol: Option<String>,
    pub kind: Option<String>,
    pub span: Span,
    pub snippet: String,
    pub snippet_truncated: bool,
    pub method: &'static str,
    pub rank_tier: u8,
    pub fingerprint: String,
    pub contains_errors: Option<bool>,
    #[serde(skip)]
    matched_excerpt: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SearchResults {
    pub query: String,
    pub results: Vec<SearchHit>,
    pub offset: usize,
    pub next_offset: Option<usize>,
    pub truncated: bool,
    pub freshness: &'static str,
    pub symbol_coverage: &'static str,
}

/// Literal exact/prefix symbol, exact path/basename and FTS5 phrase search.
/// Tier order is exact symbol/qualified name, filename, prefix symbol, then BM25.
pub fn search(
    repository: &Repository,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<SearchResults, Error> {
    if query.chars().any(char::is_control) {
        return Err(Error::InvalidQuery);
    }
    let query = query.trim();
    if query.is_empty() || query.len() > 256 || !(1..=100).contains(&limit) || offset > 10_000 {
        return Err(Error::InvalidQuery);
    }
    let mut connection = connection(repository, false)?.ok_or(Error::MissingIndex)?;
    // One read transaction keeps candidates and evidence on the same captured snapshot.
    let transaction = connection.transaction()?;
    verify_schema(&transaction, &identity(repository))?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version != 2 {
        return Err(Error::MissingIndex);
    }
    search_snapshot(&transaction, query, limit, offset)
}

/// Shared retrieval inside a caller-owned read transaction.
pub(crate) fn search_snapshot(
    transaction: &rusqlite::Connection,
    query: &str,
    limit: usize,
    offset: usize,
) -> Result<SearchResults, Error> {
    let cap = offset + limit + 1;
    let mut hits = symbol_hits(transaction, query, cap)?;
    hits.extend(filename_hits(transaction, query, cap)?);
    hits.extend(fts_hits(transaction, query, cap)?);
    // Stable sort retains BM25 order inside tier 3; other tiers already have path/byte order.
    hits.sort_by_key(|hit| hit.rank_tier);
    let mut seen = BTreeSet::new();
    hits.retain(|hit| seen.insert((hit.path.clone(), hit.symbol.clone(), hit.span.start_byte)));
    let truncated = hits.len() > offset + limit;
    let mut results: Vec<_> = hits.into_iter().skip(offset).take(limit).collect();
    attach_evidence(transaction, &mut results)?;
    Ok(SearchResults {
        query: query.to_owned(),
        results,
        offset,
        next_offset: (truncated && offset + limit <= 10_000).then_some(offset + limit),
        truncated,
        freshness: "snapshot_not_checked",
        symbol_coverage: "bounded_syntax_per_file",
    })
}

fn file_hit(path: String, fingerprint: String, method: &'static str, rank_tier: u8) -> SearchHit {
    SearchHit {
        path,
        fingerprint,
        method,
        rank_tier,
        symbol: None,
        kind: None,
        span: Span {
            start_line: 1,
            end_line: 1,
            start_byte: 0,
            end_byte: 0,
        },
        snippet: String::new(),
        snippet_truncated: false,
        contains_errors: None,
        matched_excerpt: None,
    }
}

fn symbol_hits(
    transaction: &rusqlite::Connection,
    query: &str,
    cap: usize,
) -> Result<Vec<SearchHit>, Error> {
    let mut hits = Vec::new();
    let mut statement = transaction.prepare("SELECT s.path,s.name,s.kind,s.start_line,s.end_line,s.start_byte,s.end_byte,s.contains_errors,f.fingerprint,CASE WHEN s.name = ?1 OR s.qualified_name = ?1 THEN 0 ELSE 2 END AS tier FROM symbols s JOIN files f ON f.path=s.path WHERE s.name = ?1 OR s.qualified_name = ?1 OR substr(s.name,1,length(?1)) = ?1 ORDER BY tier,s.path,s.start_byte,s.id LIMIT ?2")?;
    let rows = statement.query_map(params![query, cap as i64], |row| {
        Ok(SearchHit {
            path: row.get(0)?,
            symbol: Some(row.get(1)?),
            kind: Some(row.get(2)?),
            span: Span {
                start_line: row.get::<_, u32>(3)? as usize,
                end_line: row.get::<_, u32>(4)? as usize,
                start_byte: row.get::<_, u32>(5)? as usize,
                end_byte: row.get::<_, u32>(6)? as usize,
            },
            contains_errors: Some(row.get(7)?),
            fingerprint: row.get(8)?,
            snippet: String::new(),
            snippet_truncated: false,
            matched_excerpt: None,
            rank_tier: row.get(9)?,
            method: if row.get::<_, u8>(9)? == 0 {
                "exact_symbol"
            } else {
                "prefix_symbol"
            },
        })
    })?;
    hits.extend(rows.collect::<Result<Vec<_>, _>>()?);
    Ok(hits)
}

fn filename_hits(
    transaction: &rusqlite::Connection,
    query: &str,
    cap: usize,
) -> Result<Vec<SearchHit>, Error> {
    let mut hits = Vec::new();
    let mut statement = transaction.prepare("SELECT path,fingerprint FROM files WHERE path = ?1 OR substr(path,-length(?1)-1) = '/' || ?1 ORDER BY path LIMIT ?2")?;
    let rows = statement.query_map(params![query, cap as i64], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;
    for row in rows {
        let (path, fingerprint) = row?;
        hits.push(file_hit(path, fingerprint, "exact_filename", 1));
    }
    Ok(hits)
}

fn fts_hits(
    transaction: &rusqlite::Connection,
    query: &str,
    cap: usize,
) -> Result<Vec<SearchHit>, Error> {
    let mut hits = Vec::new();
    // Quoted literal phrase: operators, quotes and punctuation cannot become FTS syntax.
    let literal = format!("\"{}\"", query.replace('"', "\"\""));
    let mut statement = transaction.prepare("SELECT source_fts.path,f.fingerprint,snippet(source_fts,1,'','','',32) FROM source_fts JOIN files f ON f.path=source_fts.path WHERE source_fts MATCH ?1 ORDER BY bm25(source_fts),source_fts.path LIMIT ?2")?;
    let rows = statement.query_map(params![literal, cap as i64], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (path, fingerprint, excerpt) = row?;
        let mut hit = file_hit(path, fingerprint, "fts5", 3);
        hit.matched_excerpt = Some(excerpt);
        hits.push(hit);
    }

    Ok(hits)
}

fn attach_evidence(
    transaction: &rusqlite::Connection,
    results: &mut [SearchHit],
) -> Result<(), Error> {
    // Fetch only evidence needed for this page. Source bytes are bounded at index capture.
    let paths: Vec<_> = results.iter().map(|hit| &hit.path).collect();
    let mut source_query = transaction.prepare("SELECT path,content FROM files WHERE path IN (SELECT value FROM json_each(?1)) ORDER BY path")?;
    let sources = source_query
        .query_map([serde_json::to_string(&paths)?], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })?
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    for hit in results {
        let content = sources.get(&hit.path).ok_or(Error::Schema)?;
        let evidence = if hit.symbol.is_some() {
            content
                .get(hit.span.start_byte..hit.span.end_byte)
                .ok_or(Error::Schema)?
        } else {
            let excerpt = hit
                .matched_excerpt
                .as_deref()
                .unwrap_or_else(|| content.split_inclusive('\n').next().unwrap_or(""));
            let byte = content.find(excerpt).ok_or(Error::Schema)?;
            let end_byte = byte + excerpt.len();
            let line_at =
                |position| content[..position].bytes().filter(|b| *b == b'\n').count() + 1;
            hit.span = Span {
                start_line: line_at(byte),
                end_line: if end_byte > byte {
                    line_at(end_byte - 1)
                } else {
                    line_at(byte)
                },
                start_byte: byte,
                end_byte,
            };
            &content[byte..end_byte]
        };
        hit.snippet = evidence.chars().take(512).collect();
        hit.snippet_truncated = hit.snippet.len() < evidence.len();
    }
    Ok(())
}
