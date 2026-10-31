//! Reviewed configuration changes. Full client contents never cross the IPC boundary.
use super::{Harness, Preview, preview};
use crate::{ServiceError, core::reject_link};
use serde_json::{Value, json};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    time::{Duration, Instant},
};

pub struct ChangePlan {
    pub preview: Preview,
    destination: PathBuf,
    before: Option<Vec<u8>>,
    after: String,
}

fn destination(root: &Path, harness: Harness) -> Result<PathBuf, ServiceError> {
    Ok(match harness {
        Harness::Claude => root.join(".mcp.json"),
        Harness::Cursor => root.join(".cursor/mcp.json"),
        Harness::Vscode => root.join(".vscode/mcp.json"),
        Harness::Codex => {
            let directory = match std::env::var_os("CODEX_HOME") {
                Some(value) => PathBuf::from(value),
                None => directories::BaseDirs::new()
                    .ok_or("home directory unavailable")?
                    .home_dir()
                    .join(".codex"),
            };
            if !directory.is_absolute() {
                return Err("CODEX_HOME must be absolute".into());
            }
            directory.join("config.toml")
        }
    })
}

fn validate_parents(path: &Path) -> Result<(), ServiceError> {
    let mut current = PathBuf::new();
    for component in path
        .parent()
        .ok_or("invalid configuration destination")?
        .components()
    {
        current.push(component);
        if !matches!(component, Component::Prefix(_)) {
            reject_link(&current, true)?;
        }
    }
    Ok(())
}

fn read(path: &Path) -> Result<Option<Vec<u8>>, ServiceError> {
    validate_parents(path)?;
    if !reject_link(path, false)? {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    fs::File::open(path)?
        .take(1_048_577)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 1_048_576 {
        return Err("client configuration exceeds 1 MiB".into());
    }
    Ok(Some(bytes))
}

fn text(bytes: Option<&[u8]>) -> Result<&str, ServiceError> {
    Ok(std::str::from_utf8(bytes.unwrap_or_default())?)
}

/// Only the ContextForge entry is included in the returned preview.
pub fn plan(
    root: &Path,
    harness: Harness,
    executable: Option<&Path>,
) -> Result<ChangePlan, ServiceError> {
    let path = destination(root, harness)?;
    let before = read(&path)?;
    let removing = executable.is_none();
    if removing
        && !configuration_status(root, harness)?["contextforge_entry_present"]
            .as_bool()
            .unwrap_or(false)
    {
        return Err("no ContextForge entry exists to remove".into());
    }
    let mut description = preview(harness, root, executable.unwrap_or(root))?;
    let after = if matches!(harness, Harness::Codex) {
        let mut document: toml::Table = if before.is_some() {
            toml::from_str(text(before.as_deref())?)?
        } else {
            toml::Table::new()
        };
        if !document.contains_key("mcp_servers") {
            document.insert("mcp_servers".into(), toml::Value::Table(toml::Table::new()));
        }
        let servers = document
            .get_mut("mcp_servers")
            .and_then(toml::Value::as_table_mut)
            .ok_or("mcp_servers must be a table")?;
        if removing {
            servers.remove("contextforge");
        } else {
            let generated: toml::Table = toml::from_str(&description.configuration)?;
            servers.insert(
                "contextforge".into(),
                generated["mcp_servers"]["contextforge"].clone(),
            );
        }
        toml::to_string_pretty(&document)?
    } else {
        let mut document: Value = if before.is_some() {
            serde_json::from_str(text(before.as_deref())?)?
        } else {
            json!({})
        };
        let key = if matches!(harness, Harness::Vscode) {
            "servers"
        } else {
            "mcpServers"
        };
        let object = document
            .as_object_mut()
            .ok_or("client configuration must be an object")?;
        if !object.contains_key(key) {
            object.insert(key.into(), json!({}));
        }
        let servers = object
            .get_mut(key)
            .and_then(Value::as_object_mut)
            .ok_or("server entries must be an object")?;
        if removing {
            servers.remove("contextforge");
        } else {
            let generated: Value = serde_json::from_str(&description.configuration)?;
            servers.insert(
                "contextforge".into(),
                generated[key]["contextforge"].clone(),
            );
        }
        serde_json::to_string_pretty(&document)?
    };
    description.destination = match harness {
        Harness::Claude => "<repository>/.mcp.json",
        Harness::Cursor => "<repository>/.cursor/mcp.json",
        Harness::Vscode => "<repository>/.vscode/mcp.json",
        Harness::Codex => "$CODEX_HOME/config.toml (default ~/.codex/config.toml)",
    };
    if removing {
        description.configuration =
            "Remove only the ContextForge server entry; preserve other client settings.".into();
    }
    description.instructions = "Review this entry, then Apply. A native confirmation precedes writing. Existing files are backed up. Unrelated values are preserved; JSON/TOML formatting and comments may be normalized. Concurrent modifications cause rejection. JSONC configurations are not supported.";
    description.verification = "Configuration syntax validated. Actual MCP connectivity has NOT been tested. Restart the harness, review its trust prompt and call project_map then search_context.";
    Ok(ChangePlan {
        preview: description,
        destination: path,
        before,
        after,
    })
}

impl ChangePlan {
    /// Caller must obtain explicit confirmation after displaying `preview`.
    pub fn apply(self) -> Result<Option<PathBuf>, ServiceError> {
        validate_parents(&self.destination)?;
        let parent = self.destination.parent().ok_or("invalid destination")?;
        fs::create_dir_all(parent)?;
        validate_parents(&self.destination)?;
        let lock_path = parent.join(".contextforge-config.lock");
        reject_link(&lock_path, false)?;
        let lock = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        let start = Instant::now();
        loop {
            match lock.try_lock() {
                Ok(()) => break,
                Err(fs::TryLockError::WouldBlock) if start.elapsed() < Duration::from_secs(3) => {
                    std::thread::sleep(Duration::from_millis(20))
                }
                Err(_) => return Err("client configuration is busy; preview again".into()),
            }
        }
        if read(&self.destination)? != self.before {
            return Err("client configuration changed; preview again".into());
        }
        let backup = if let Some(before) = &self.before {
            let mut backup = tempfile::Builder::new()
                .prefix("contextforge-backup-")
                .suffix(".bak")
                .tempfile_in(parent)?;
            backup.write_all(before)?;
            backup.as_file().sync_all()?;
            Some(backup.keep()?.1)
        } else {
            None
        };
        let mut replacement = tempfile::NamedTempFile::new_in(parent)?;
        replacement.write_all(self.after.as_bytes())?;
        replacement.as_file().sync_all()?;
        replacement.persist(&self.destination)?;
        Ok(backup)
    }
}

pub fn configuration_status(root: &Path, harness: Harness) -> Result<Value, ServiceError> {
    let path = destination(root, harness)?;
    let bytes = read(&path)?;
    let configured = if let Some(bytes) = bytes.as_deref() {
        if matches!(harness, Harness::Codex) {
            let value: toml::Value = toml::from_str(text(Some(bytes))?)?;
            value
                .get("mcp_servers")
                .and_then(|servers| servers.get("contextforge"))
                .is_some()
        } else {
            let value: Value = serde_json::from_slice(bytes)?;
            let key = if matches!(harness, Harness::Vscode) {
                "servers"
            } else {
                "mcpServers"
            };
            value
                .get(key)
                .and_then(|servers| servers.get("contextforge"))
                .is_some()
        }
    } else {
        false
    };
    Ok(
        json!({"configuration_exists": bytes.is_some(), "contextforge_entry_present": configured, "connectivity": "not_tested"}),
    )
}
