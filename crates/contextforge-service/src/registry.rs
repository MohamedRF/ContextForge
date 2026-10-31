//! Shared project registrations; unregistering never deletes source or index data.
use crate::{ServiceError, core::Repository};
use rusqlite::{Connection, TransactionBehavior, params};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Debug, Serialize)]
pub struct Project {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub available: bool,
}

#[derive(Clone)]
pub struct Registry {
    path: PathBuf,
}

impl Registry {
    pub fn user() -> Result<Self, ServiceError> {
        let directory = directories::ProjectDirs::from("dev", "ContextForge", "ContextForge")
            .ok_or("unable to locate user application data")?;
        Self::open(directory.data_local_dir())
    }

    pub fn open(directory: &Path) -> Result<Self, ServiceError> {
        std::fs::create_dir_all(directory)?;
        crate::core::reject_link(directory, true)?;
        let registry = Self {
            path: directory.join("projects.sqlite"),
        };
        let mut connection = registry.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: u32 =
            transaction.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 1 {
            return Err(
                "project registry is newer than this application; upgrade ContextForge".into(),
            );
        }
        if version == 0 {
            transaction.execute_batch("CREATE TABLE projects (id INTEGER PRIMARY KEY, path TEXT NOT NULL UNIQUE, name TEXT NOT NULL); CREATE TABLE preferences (key TEXT PRIMARY KEY, value TEXT NOT NULL); PRAGMA user_version = 1;")?;
        }
        transaction.commit()?;
        Ok(registry)
    }

    fn connection(&self) -> Result<Connection, ServiceError> {
        crate::core::reject_link(&self.path, false)?;
        if std::fs::metadata(&self.path).is_ok_and(|metadata| metadata.len() > 2 * 1024 * 1024) {
            return Err("project registry exceeds its size limit".into());
        }
        for suffix in ["-journal", "-wal", "-shm"] {
            crate::core::reject_link(
                &self.path.with_file_name(format!("projects.sqlite{suffix}")),
                false,
            )?;
        }
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "trusted_schema", false)?;
        let version: u32 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version > 1 {
            return Err(
                "project registry is newer than this application; upgrade ContextForge".into(),
            );
        }
        Ok(connection)
    }

    pub fn add(&self, path: &Path) -> Result<i64, ServiceError> {
        let repository = Repository::open(Some(path), path)?;
        let path = repository.root();
        let text = path.to_str().ok_or("project path must be valid Unicode")?;
        if text.len() > 32_768 {
            return Err("project path exceeds its size limit".into());
        }
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(text);
        let mut connection = self.connection()?;
        let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let count: u32 =
            transaction.query_row("SELECT count(id) FROM projects", [], |row| row.get(0))?;
        if count >= 256 {
            return Err("project registry limit reached (256)".into());
        }
        transaction.execute(
            "INSERT INTO projects(path,name) VALUES (?1,?2) ON CONFLICT(path) DO NOTHING",
            params![text, name],
        )?;
        let id = transaction.query_row("SELECT id FROM projects WHERE path=?1", [text], |row| {
            row.get(0)
        })?;
        transaction.commit()?;
        Ok(id)
    }

    pub fn list(&self) -> Result<Vec<Project>, ServiceError> {
        let connection = self.connection()?;
        let mut statement =
            connection.prepare("SELECT id,name,path FROM projects ORDER BY name,path LIMIT 256")?;
        let rows = statement.query_map([], |row| {
            let path: String = row.get(2)?;
            Ok(Project {
                id: row.get(0)?,
                name: row.get(1)?,
                available: Path::new(&path).is_dir(),
                path,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    pub fn repository(&self, id: i64) -> Result<Repository, ServiceError> {
        let connection = self.connection()?;
        let path: String =
            connection.query_row("SELECT path FROM projects WHERE id=?1", [id], |row| {
                row.get(0)
            })?;
        Ok(Repository::open(Some(Path::new(&path)), Path::new(&path))?)
    }

    pub fn remove(&self, id: i64) -> Result<(), ServiceError> {
        self.connection()?
            .execute("DELETE FROM projects WHERE id=?1", [id])?;
        Ok(())
    }

    pub fn theme(&self) -> Result<String, ServiceError> {
        use rusqlite::OptionalExtension;
        Ok(self
            .connection()?
            .query_row(
                "SELECT value FROM preferences WHERE key='theme'",
                [],
                |row| row.get(0),
            )
            .optional()?
            .unwrap_or_else(|| "system".into()))
    }

    pub fn set_theme(&self, theme: &str) -> Result<(), ServiceError> {
        if !["system", "dark", "light"].contains(&theme) {
            return Err("theme must be system, dark or light".into());
        }
        self.connection()?.execute("INSERT INTO preferences(key,value) VALUES ('theme',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [theme])?;
        Ok(())
    }

    pub fn selected_project(&self) -> Result<Option<i64>, ServiceError> {
        use rusqlite::OptionalExtension;
        let value: Option<String> = self
            .connection()?
            .query_row(
                "SELECT value FROM preferences WHERE key='selected_project'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.and_then(|value| value.parse().ok()))
    }

    pub fn select_project(&self, id: i64) -> Result<(), ServiceError> {
        self.repository(id)?;
        self.connection()?.execute("INSERT INTO preferences(key,value) VALUES ('selected_project',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [id.to_string()])?;
        Ok(())
    }
}
