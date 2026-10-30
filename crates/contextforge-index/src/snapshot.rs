//! Read-only, bounded export of one consistent captured source/syntax snapshot.
use crate::{Error, connection, identity, verify_schema};
use contextforge_core::Repository;

/// Cached syntax with its source identity. Consumers must not assume current freshness.
pub struct SyntaxFile {
    pub path: String,
    pub fingerprint: String,
    pub payload: String,
}

/// Full captured evidence used by context packing. Never interpreted as instructions.
pub struct SourceFile {
    pub syntax: SyntaxFile,
    pub content: String,
}

/// Source text, outlines and retrieval hits from exactly one read transaction.
pub struct Retrieval {
    pub files: Vec<SourceFile>,
    pub searches: Vec<crate::search::SearchResults>,
}

/// Bounded multi-query retrieval with consistent source/syntax evidence.
pub fn retrieve(repository: &Repository, queries: &[String]) -> Result<Retrieval, Error> {
    if queries.is_empty()
        || queries.len() > 9
        || queries
            .iter()
            .any(|q| q.trim().is_empty() || q.len() > 256 || q.chars().any(char::is_control))
    {
        return Err(Error::InvalidQuery);
    }
    let mut connection = connection(repository, false)?.ok_or(Error::MissingIndex)?;
    let transaction = connection.transaction()?;
    verify_schema(&transaction, &identity(repository))?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version != 2 {
        return Err(Error::MissingIndex);
    }
    let mut statement = transaction.prepare("SELECT f.path,f.fingerprint,o.payload,f.content FROM files f JOIN outlines o ON o.path=f.path ORDER BY f.path LIMIT 1001")?;
    let rows = statement.query_map([], |row| {
        Ok(SourceFile {
            syntax: SyntaxFile {
                path: row.get(0)?,
                fingerprint: row.get(1)?,
                payload: row.get(2)?,
            },
            content: row.get(3)?,
        })
    })?;
    let mut files = Vec::new();
    let mut bytes = 0usize;
    for row in rows {
        let file = row?;
        bytes += file.content.len() + file.syntax.payload.len();
        if files.len() == 1000 || bytes > 128 * 1024 * 1024 {
            return Err(Error::SnapshotTooLarge);
        }
        files.push(file);
    }
    let searches = queries
        .iter()
        .map(|query| crate::search::search_snapshot(&transaction, query, 50, 0))
        .collect::<Result<_, _>>()?;
    Ok(Retrieval { files, searches })
}

/// Read all bounded outlines in one transaction, without creating or refreshing metadata.
pub fn syntax(repository: &Repository) -> Result<Vec<SyntaxFile>, Error> {
    let mut connection = connection(repository, false)?.ok_or(Error::MissingIndex)?;
    let transaction = connection.transaction()?;
    verify_schema(&transaction, &identity(repository))?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version != 2 {
        return Err(Error::MissingIndex);
    }
    let mut statement = transaction.prepare("SELECT f.path,f.fingerprint,o.payload FROM files f JOIN outlines o ON o.path=f.path ORDER BY f.path LIMIT 1001")?;
    let rows = statement.query_map([], |row| {
        Ok(SyntaxFile {
            path: row.get(0)?,
            fingerprint: row.get(1)?,
            payload: row.get(2)?,
        })
    })?;
    let mut files = Vec::new();
    let mut bytes = 0usize;
    for row in rows {
        let file = row?;
        bytes += file.payload.len();
        if files.len() == 1000 || bytes > 64 * 1024 * 1024 {
            return Err(Error::SnapshotTooLarge);
        }
        files.push(file);
    }
    Ok(files)
}
