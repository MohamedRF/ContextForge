//! Transactional, repository-isolated source snapshots. No project code is executed.
use contextforge_core::{Repository, reject_link};
use rusqlite::{Connection, OpenFlags, TransactionBehavior, params};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use thiserror::Error;

mod derived;
pub mod memory;
pub mod search;
#[cfg(test)]
mod security_tests;
pub mod snapshot;

#[derive(Debug, Error)]
pub enum Error {
    #[error("source parsing failed; index was not changed")]
    Parse(#[from] contextforge_parser::Error),
    #[error("unable to serialize syntax evidence")]
    Serialization(#[from] serde_json::Error),
    #[error("index changed concurrently; retry indexing")]
    Concurrent,
    #[error("search requires a current index; run contextforge index first")]
    MissingIndex,
    #[error("query must contain 1–256 non-control bytes, limit 1–100, offset 0–10000")]
    InvalidQuery,
    #[error("repository discovery or source validation failed")]
    Repository(#[from] contextforge_core::Error),
    #[error("index filesystem operation failed")]
    Filesystem(#[from] std::io::Error),
    #[error("index database operation failed; preserve metadata and check access or corruption")]
    Database(#[from] rusqlite::Error),
    #[error("index requires initialization; run contextforge init first")]
    NotInitialized,
    #[error("inventory is truncated; index was not changed")]
    Truncated,
    #[error("source snapshot exceeds the 64 MiB total input limit; index was not changed")]
    SnapshotTooLarge,
    #[error("unsupported or invalid index schema; index was not changed")]
    Schema,
    #[error("index belongs to a different repository")]
    Identity,
    #[error("index database or query exceeds resource limits")]
    ResourceLimit,
}

#[derive(Debug, Serialize)]
pub struct IndexReport {
    pub schema_version: u32,
    pub added: usize,
    pub modified: usize,
    pub unchanged: usize,
    pub deleted: usize,
    pub total_files: usize,
    /// Actual parser invocations, including speculative work before concurrent writers finish.
    pub parsed_files: usize,
    /// Files whose stored syntax contains errors or reaches an extraction limit.
    pub partial_files: u32,
}

#[derive(Debug, Serialize)]
pub struct IndexStatus {
    pub state: &'static str,
    pub schema_version: Option<u32>,
    pub files: u32,
    pub freshness: &'static str,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn identity(repository: &Repository) -> String {
    // Encoded native bytes preserve non-UTF-8 root identity without lossy conversion.
    digest(repository.root().as_os_str().as_encoded_bytes())
}

fn cache_directory(repository: &Repository) -> PathBuf {
    let configured = &repository.config.index_root;
    let base = if configured.is_absolute() {
        configured.clone()
    } else {
        repository.root().join(".contextforge").join(configured)
    };
    base.join(identity(repository))
}

fn validate_directory(path: &Path, create: bool) -> Result<bool, Error> {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        // Prefixes such as C: are resolved by the following root component.
        if matches!(component, std::path::Component::Prefix(_)) {
            continue;
        }
        if !reject_link(&current, true)? {
            if !create {
                return Ok(false);
            }
            let builder = fs::DirBuilder::new();
            #[cfg(unix)]
            let builder = {
                use std::os::unix::fs::DirBuilderExt;
                let mut builder = builder;
                builder.mode(0o700);
                builder
            };
            match builder.create(&current) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                    reject_link(&current, true)?;
                }
                Err(error) => return Err(Error::Filesystem(error)),
            }
        }
    }
    Ok(true)
}

fn connection(repository: &Repository, write: bool) -> Result<Option<Connection>, Error> {
    let directory = cache_directory(repository);
    if !validate_directory(&directory, write)? {
        return Ok(None);
    }
    let path = directory.join("index.sqlite");
    let exists = reject_link(&path, false)?;
    if exists && fs::metadata(&path)?.len() > 512 * 1024 * 1024 {
        return Err(Error::ResourceLimit);
    }
    #[cfg(unix)]
    if exists {
        use std::os::unix::fs::MetadataExt;
        if fs::metadata(&path)?.nlink() != 1 {
            return Err(contextforge_core::Error::UnsafeMetadata.into());
        }
    }
    // SQLite rollback/journal sidecars must not redirect writes outside managed metadata.
    for name in [
        "index.sqlite-journal",
        "index.sqlite-wal",
        "index.sqlite-shm",
    ] {
        let sidecar = directory.join(name);
        if reject_link(&sidecar, false)? {
            let metadata = fs::metadata(&sidecar)?;
            if metadata.len() > 512 * 1024 * 1024 {
                return Err(Error::ResourceLimit);
            }
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if metadata.nlink() != 1 {
                    return Err(contextforge_core::Error::UnsafeMetadata.into());
                }
            }
        }
    }
    if !exists && !write {
        return Ok(None);
    }
    let flags = if write {
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
    } else {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    };
    let mut connection =
        Connection::open_with_flags(path, flags | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    connection.busy_timeout(Duration::from_secs(2))?;
    // Bound SQL VM work in read-only requests. The budget belongs to the whole
    // connection, so issuing many individually cheap queries cannot reset it.
    if !write {
        let started = Instant::now();
        let mut callbacks = 0usize;
        connection.progress_handler(
            10_000,
            Some(move || {
                callbacks += 1;
                callbacks >= 10_000 || started.elapsed() >= Duration::from_secs(5)
            }),
        )?;
    }
    use rusqlite::limits::Limit;
    connection.set_limit(Limit::SQLITE_LIMIT_LENGTH, 32 * 1024 * 1024)?;
    connection.set_limit(Limit::SQLITE_LIMIT_SQL_LENGTH, 64 * 1024)?;
    connection.set_limit(Limit::SQLITE_LIMIT_ATTACHED, 0)?;
    connection.pragma_update(None, "trusted_schema", false)?;
    if write {
        let page_size: i64 = connection.pragma_query_value(None, "page_size", |row| row.get(0))?;
        if page_size <= 0 {
            return Err(Error::Schema);
        }
        connection.pragma_update(None, "max_page_count", 512 * 1024 * 1024 / page_size)?;
    }
    Ok(Some(connection))
}

fn verify_schema(connection: &Connection, expected_identity: &str) -> Result<(), Error> {
    let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if !matches!(version, 1 | 2) {
        return Err(Error::Schema);
    }
    let stored: String = connection.query_row(
        "SELECT identity FROM repository WHERE singleton = 1",
        [],
        |row| row.get(0),
    )?;
    if stored != expected_identity {
        return Err(Error::Identity);
    }
    Ok(())
}

/// Read index metadata without creating directories, databases, or claiming source freshness.
pub fn status(repository: &Repository) -> Result<IndexStatus, Error> {
    let Some(connection) = connection(repository, false)? else {
        return Ok(IndexStatus {
            state: "missing",
            schema_version: None,
            files: 0,
            freshness: "not_checked",
        });
    };
    verify_schema(&connection, &identity(repository))?;
    let files = connection.query_row("SELECT count(path) FROM files", [], |row| row.get(0))?;
    Ok(IndexStatus {
        state: "ready",
        schema_version: Some(
            connection.pragma_query_value(None, "user_version", |row| row.get(0))?,
        ),
        files,
        freshness: "not_checked",
    })
}

/// Capture eligible source bytes, then atomically upsert changed files and remove obsolete ones.
/// All files are read/hashed to detect same-size and same-timestamp edits; only changes are written.
pub fn index(repository: &Repository) -> Result<IndexReport, Error> {
    index_with_rebuild(repository, false)
}

fn index_with_rebuild(repository: &Repository, rebuild: bool) -> Result<IndexReport, Error> {
    if !repository.initialized {
        return Err(Error::NotInitialized);
    }
    let map = repository.project_map(1000)?;
    if map.truncated {
        return Err(Error::Truncated);
    }
    let mut documents = Vec::new();
    let mut total_bytes = 0;
    for file in map.files {
        let content = repository.read_source(Path::new(&file.path))?;
        total_bytes += content.len();
        if total_bytes > 64 * 1024 * 1024 {
            return Err(Error::SnapshotTooLarge);
        }
        documents.push((
            file.path,
            file.language,
            digest(content.as_bytes()),
            content,
        ));
    }
    let repository_identity = identity(repository);
    // Prepare syntax before taking the write lock. Unchanged schema-2 files are not reparsed.
    // A write-capable connection is necessary for SQLite hot-journal recovery after
    // an interrupted writer. This remains an explicitly requested index operation.
    let (anticipated, old_version) = if let Some(connection) = connection(repository, true)? {
        let version =
            connection.pragma_query_value(None, "user_version", |row| row.get::<_, u32>(0))?;
        if version == 0 {
            (BTreeMap::new(), 0)
        } else {
            verify_schema(&connection, &repository_identity)?;
            (fingerprints(&connection)?, version)
        }
    } else {
        (BTreeMap::new(), 0)
    };
    let mut prepared = BTreeMap::new();
    for (path, _, fingerprint, content) in &documents {
        if rebuild || old_version != 2 || anticipated.get(path) != Some(fingerprint) {
            prepared.insert(
                path.clone(),
                contextforge_parser::parse_source(&repository_identity, path, content, 1000)?,
            );
        }
    }
    let mut connection = connection(repository, true)?.ok_or(Error::NotInitialized)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let version: u32 = transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if version == 0 {
        let tables: i64 = transaction.query_row("SELECT count(name) FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0))?;
        if tables != 0 {
            return Err(Error::Schema);
        }
        transaction.execute_batch("CREATE TABLE repository (singleton INTEGER PRIMARY KEY CHECK (singleton = 1), identity TEXT NOT NULL); CREATE TABLE files (path TEXT PRIMARY KEY, language TEXT NOT NULL, fingerprint TEXT NOT NULL, content TEXT NOT NULL, bytes INTEGER NOT NULL CHECK (bytes >= 0)); PRAGMA user_version = 1;")?;
        transaction.execute(
            "INSERT INTO repository (singleton, identity) VALUES (1, ?1)",
            [&repository_identity],
        )?;
    }
    verify_schema(&transaction, &repository_identity)?;
    let needs_migration: bool = transaction.pragma_query_value(None, "user_version", |row| {
        row.get::<_, u32>(0).map(|v| v == 1)
    })?;
    if needs_migration {
        derived::migrate(&transaction)?;
    }
    let mut previous = fingerprints(&transaction)?;
    let mut report = IndexReport {
        schema_version: 2,
        added: 0,
        modified: 0,
        unchanged: 0,
        deleted: 0,
        total_files: documents.len(),
        parsed_files: prepared.len(),
        partial_files: 0,
    };
    {
        let mut upsert = transaction.prepare("INSERT INTO files (path, language, fingerprint, content, bytes) VALUES (?1, ?2, ?3, ?4, ?5) ON CONFLICT(path) DO UPDATE SET language = excluded.language, fingerprint = excluded.fingerprint, content = excluded.content, bytes = excluded.bytes")?;
        for (path, language, fingerprint, content) in documents {
            match previous.remove(&path) {
                Some(old) if old == fingerprint => {
                    report.unchanged += 1;
                    if needs_migration || rebuild {
                        let outline = prepared.remove(&path).ok_or(Error::Concurrent)?;
                        derived::replace(&transaction, &path, &content, &outline)?;
                    }
                    continue;
                }
                Some(_) => report.modified += 1,
                None => report.added += 1,
            }
            // Source reads are capped at 16 MiB, safely below SQLite's signed integer range.
            upsert.execute(params![
                path,
                language,
                fingerprint,
                content,
                content.len() as i64
            ])?;
            let outline = prepared.remove(&path).ok_or(Error::Concurrent)?;
            derived::replace(&transaction, &path, &content, &outline)?;
        }
        let mut delete = transaction.prepare("DELETE FROM files WHERE path = ?1")?;
        for path in previous.keys() {
            derived::delete(&transaction, path)?;
            delete.execute([path])?;
            report.deleted += 1;
        }
    }
    report.partial_files = transaction.query_row(
        "SELECT count(path) FROM outlines WHERE partial = 1",
        [],
        |row| row.get(0),
    )?;
    transaction.commit()?;
    Ok(report)
}

fn fingerprints(connection: &Connection) -> Result<BTreeMap<String, String>, Error> {
    let mut statement = connection.prepare("SELECT path, fingerprint FROM files ORDER BY path")?;
    let records = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(records.collect::<Result<BTreeMap<_, _>, _>>()?)
}
