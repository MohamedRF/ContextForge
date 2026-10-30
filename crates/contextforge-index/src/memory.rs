//! Structural knowledge lifecycle. Reuses versioned source/outline storage, never agent chats.
use crate::{Error, IndexReport, connection, digest, fingerprints, identity, verify_schema};
use contextforge_core::Repository;
use rusqlite::TransactionBehavior;
use serde::Serialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Debug, Serialize)]
pub struct KnowledgeStatus {
    pub state: &'static str,
    pub schema_version: Option<u32>,
    pub freshness: &'static str,
    pub files: Vec<KnowledgeFile>,
    pub added: usize,
    pub changed_or_removed: usize,
    pub provenance: &'static str,
}

#[derive(Debug, Serialize)]
pub struct KnowledgeFile {
    pub path: String,
    pub language: String,
    pub fingerprint: String,
    pub valid: bool,
    /// Stale definitions are never exposed as current structural knowledge.
    pub symbols: Option<u32>,
}

/// Compare every current eligible source hash with one consistent stored snapshot.
/// This observes sources; it does not update any metadata or promise a filesystem-wide lock.
pub fn status(repository: &Repository) -> Result<KnowledgeStatus, Error> {
    let mut result = KnowledgeStatus {
        state: "missing",
        schema_version: None,
        freshness: "not_applicable",
        files: Vec::new(),
        added: 0,
        changed_or_removed: 0,
        provenance: "tree_sitter_syntax_and_sha256_source; no_agent_chats_or_notes",
    };
    let Some(mut connection) = connection(repository, false)? else {
        return Ok(result);
    };
    let transaction = connection.transaction()?;
    verify_schema(&transaction, &identity(repository))?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    result.schema_version = Some(version);
    let map = repository.project_map(1000)?;
    if map.truncated {
        return Err(Error::Truncated);
    }
    let mut current = BTreeMap::new();
    let mut bytes = 0;
    for file in map.files {
        let content = repository.read_source(Path::new(&file.path))?;
        bytes += content.len();
        if bytes > 64 * 1024 * 1024 {
            return Err(Error::SnapshotTooLarge);
        }
        current.insert(file.path, digest(content.as_bytes()));
    }
    let stored = fingerprints(&transaction)?;
    if stored.len() > 1000 {
        return Err(Error::SnapshotTooLarge);
    }
    let counts: BTreeMap<String, u32> = if version == 2 {
        let mut statement = transaction
            .prepare("SELECT path,count(id) FROM symbols GROUP BY path ORDER BY path")?;
        statement
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<_, _>>()?
    } else {
        BTreeMap::new()
    };
    let mut statement = transaction
        .prepare("SELECT path,language,fingerprint FROM files ORDER BY path LIMIT 1001")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    for row in rows {
        let (path, language, fingerprint) = row?;
        let valid = version == 2 && current.get(&path) == Some(&fingerprint);
        result.changed_or_removed += usize::from(!valid);
        result.files.push(KnowledgeFile {
            symbols: valid.then(|| counts.get(&path).copied().unwrap_or(0)),
            path,
            language,
            fingerprint,
            valid,
        });
    }
    result.added = current
        .keys()
        .filter(|path| !stored.contains_key(*path))
        .count();
    result.state = "ready";
    result.freshness = if version == 2 && result.added == 0 && result.changed_or_removed == 0 {
        "validated_at_read"
    } else {
        "stale"
    };
    Ok(result)
}

/// Explicitly clear only this repository's indexed source and derived rows atomically.
/// Keeps configuration, schema and root identity; never recursively removes directories.
pub fn clear(repository: &Repository) -> Result<usize, Error> {
    if !repository.initialized {
        return Err(Error::NotInitialized);
    }
    // Probe read-only first: clearing a missing cache creates nothing.
    if connection(repository, false)?.is_none() {
        return Ok(0);
    }
    let mut connection = connection(repository, true)?.ok_or(Error::MissingIndex)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    verify_schema(&transaction, &identity(repository))?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == 2 {
        transaction.execute("DELETE FROM symbols", [])?;
        transaction.execute("DELETE FROM outlines", [])?;
        transaction.execute("DELETE FROM source_fts", [])?;
    }
    let cleared = transaction.execute("DELETE FROM files", [])?;
    transaction.commit()?;
    Ok(cleared)
}

/// Reparse and replace all structural data atomically, preserving the old cache on failure.
pub fn rebuild(repository: &Repository) -> Result<IndexReport, Error> {
    crate::index_with_rebuild(repository, true)
}
