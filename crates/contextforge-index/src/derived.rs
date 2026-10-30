use crate::Error;
use contextforge_parser::Outline;
use rusqlite::{Transaction, params};
use std::collections::BTreeMap;

pub(crate) fn migrate(transaction: &Transaction<'_>) -> Result<(), Error> {
    transaction.execute_batch("CREATE TABLE outlines (path TEXT PRIMARY KEY, payload TEXT NOT NULL, partial INTEGER NOT NULL CHECK (partial IN (0,1)));
        CREATE TABLE symbols (id TEXT PRIMARY KEY, path TEXT NOT NULL, name TEXT NOT NULL, qualified_name TEXT NOT NULL, kind TEXT NOT NULL, start_line INTEGER NOT NULL, end_line INTEGER NOT NULL, start_byte INTEGER NOT NULL, end_byte INTEGER NOT NULL, contains_errors INTEGER NOT NULL);
        CREATE INDEX symbols_name ON symbols(name);
        CREATE INDEX symbols_qualified ON symbols(qualified_name);
        CREATE INDEX symbols_path ON symbols(path);
        CREATE VIRTUAL TABLE source_fts USING fts5(path UNINDEXED, content, tokenize='unicode61');
        PRAGMA user_version = 2;")?;
    Ok(())
}

pub(crate) fn delete(transaction: &Transaction<'_>, path: &str) -> Result<(), Error> {
    transaction.execute("DELETE FROM symbols WHERE path = ?1", [path])?;
    transaction.execute("DELETE FROM outlines WHERE path = ?1", [path])?;
    transaction.execute("DELETE FROM source_fts WHERE path = ?1", [path])?;
    Ok(())
}

pub(crate) fn replace(
    transaction: &Transaction<'_>,
    path: &str,
    content: &str,
    outline: &Outline,
) -> Result<(), Error> {
    delete(transaction, path)?;
    transaction.execute(
        "INSERT INTO outlines(path, payload, partial) VALUES (?1, ?2, ?3)",
        params![
            path,
            serde_json::to_string(outline)?,
            outline.has_errors || outline.truncated
        ],
    )?;
    transaction.execute(
        "INSERT INTO source_fts(path, content) VALUES (?1, ?2)",
        params![path, content],
    )?;
    let mut qualified = BTreeMap::new();
    let mut insert = transaction.prepare("INSERT INTO symbols(id,path,name,qualified_name,kind,start_line,end_line,start_byte,end_byte,contains_errors) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)")?;
    for symbol in &outline.symbols {
        let name = symbol
            .parent_symbol
            .as_ref()
            .and_then(|id| qualified.get(id))
            .map_or_else(
                || symbol.name.clone(),
                |parent| format!("{parent}::{}", symbol.name),
            );
        qualified.insert(symbol.id.clone(), name.clone());
        insert.execute(params![
            symbol.id,
            path,
            symbol.name,
            name,
            symbol.kind,
            symbol.span.start_line as i64,
            symbol.span.end_line as i64,
            symbol.span.start_byte as i64,
            symbol.span.end_byte as i64,
            symbol.contains_errors
        ])?;
    }
    Ok(())
}
