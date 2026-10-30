//! Root-scoped configuration and bounded, read-only repository discovery.

use serde::Serialize;
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};
use thiserror::Error;

pub mod discovery;

/// User-safe failures; deliberately exclude OS paths and configuration contents.
#[derive(Debug, Error)]
pub enum Error {
    #[error("repository directory is missing or inaccessible")]
    InvalidRoot,
    #[error("no repository found; use --root to select a directory explicitly")]
    MissingRepository,
    #[error("configuration is invalid or unsupported")]
    InvalidConfig,
    #[error("managed metadata contains a link or invalid file type")]
    UnsafeMetadata,
    #[error("unable to read managed metadata")]
    ReadMetadata(#[source] std::io::Error),
    #[error("unable to write managed metadata")]
    WriteMetadata(#[source] std::io::Error),
    #[error("unable to inspect repository files")]
    Discovery,
    #[error("limit must be between 1 and 1000")]
    InvalidLimit,
}

/// Versioned project configuration. Relative index roots must remain below metadata.
#[derive(Debug, Clone, serde::Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub version: u32,
    pub index_root: PathBuf,
    pub max_file_bytes: u64,
    pub ignore_patterns: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: 1,
            index_root: PathBuf::from("index"),
            max_file_bytes: 1_048_576,
            ignore_patterns: Vec::new(),
        }
    }
}

impl Config {
    fn validate(&self) -> Result<(), Error> {
        if self.version != 1
            || self.max_file_bytes == 0
            || self.max_file_bytes > 16_777_216
            || self.ignore_patterns.len() > 256
            || self.ignore_patterns.iter().any(|p| p.len() > 1024)
            || (!self.index_root.is_absolute()
                && (self.index_root.as_os_str().is_empty()
                    || self
                        .index_root
                        .components()
                        .any(|c| !matches!(c, Component::Normal(_)))))
        {
            return Err(Error::InvalidConfig);
        }
        Ok(())
    }
}

/// Canonical authorized root, selected explicitly or discovered from the current directory.
#[derive(Debug, Clone)]
pub struct Repository {
    root: PathBuf,
    pub config: Config,
    pub initialized: bool,
}

/// Paths are intentionally absent from status to avoid disclosing absolute locations.
#[derive(Debug, Serialize)]
pub struct Status {
    pub config_version: u32,
    pub initialized: bool,
    pub git_detected: bool,
    pub max_file_bytes: u64,
}

/// Rejects symlinks, Windows reparse points, and unexpected file types.
pub fn reject_link(path: &Path, directory: bool) -> Result<bool, Error> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            #[cfg(windows)]
            {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return Err(Error::UnsafeMetadata);
                }
            }
            if meta.file_type().is_symlink()
                || (directory && !meta.is_dir())
                || (!directory && !meta.is_file())
            {
                return Err(Error::UnsafeMetadata);
            }
            Ok(true)
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(Error::ReadMetadata(e)),
    }
}

impl Repository {
    /// Read a relative source path without following existing links in any component.
    /// Concurrent hostile replacement still requires platform-specific handle-relative I/O.
    pub fn read_source(&self, relative: &Path) -> Result<String, Error> {
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, Component::Normal(_)))
        {
            return Err(Error::InvalidRoot);
        }
        let mut path = self.root.clone();
        let components: Vec<_> = relative.components().collect();
        for (position, component) in components.iter().enumerate() {
            path.push(component.as_os_str());
            if !reject_link(&path, position + 1 != components.len())? {
                return Err(Error::Discovery);
            }
        }
        if !fs::canonicalize(&path)
            .map_err(|_| Error::Discovery)?
            .starts_with(&self.root)
        {
            return Err(Error::Discovery);
        }
        let mut bytes = Vec::new();
        fs::File::open(path)
            .map_err(|_| Error::Discovery)?
            .take(self.config.max_file_bytes + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| Error::Discovery)?;
        if bytes.len() as u64 > self.config.max_file_bytes || bytes.contains(&0) {
            return Err(Error::Discovery);
        }
        String::from_utf8(bytes).map_err(|_| Error::Discovery)
    }

    /// An explicit root authorizes that directory even without Git or ContextForge metadata.
    pub fn open(explicit: Option<&Path>, current: &Path) -> Result<Self, Error> {
        let start =
            fs::canonicalize(explicit.unwrap_or(current)).map_err(|_| Error::InvalidRoot)?;
        if !start.is_dir() {
            return Err(Error::InvalidRoot);
        }
        let root = if explicit.is_some() {
            start
        } else {
            start
                .ancestors()
                .find(|p| p.join(".git").exists() || p.join(".contextforge").exists())
                .ok_or(Error::MissingRepository)?
                .to_path_buf()
        };
        let metadata = root.join(".contextforge");
        reject_link(&metadata, true)?;
        let path = metadata.join("config.toml");
        let initialized = reject_link(&path, false)?;
        let config = if initialized {
            let mut bytes = Vec::new();
            fs::File::open(path)
                .map_err(Error::ReadMetadata)?
                .take(65_537)
                .read_to_end(&mut bytes)
                .map_err(Error::ReadMetadata)?;
            if bytes.len() > 65_536 {
                return Err(Error::InvalidConfig);
            }
            toml::from_str(std::str::from_utf8(&bytes).map_err(|_| Error::InvalidConfig)?)
                .map_err(|_| Error::InvalidConfig)?
        } else {
            Config::default()
        };
        config.validate()?;
        // Compile patterns at the boundary so status and MCP reject bad configuration too.
        discovery::validate_patterns(&root, &config.ignore_patterns)?;
        Ok(Self {
            root,
            config,
            initialized,
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Creates only the managed config; never overwrites an existing config or edits ignore files.
    pub fn init(&self) -> Result<(), Error> {
        let metadata = self.root.join(".contextforge");
        if !reject_link(&metadata, true)? {
            fs::create_dir(&metadata).map_err(Error::WriteMetadata)?;
        }
        if self.initialized {
            return Ok(());
        }
        let text = toml::to_string_pretty(&self.config).map_err(|_| Error::InvalidConfig)?;
        let mut file = tempfile::NamedTempFile::new_in(&metadata).map_err(Error::WriteMetadata)?;
        file.write_all(text.as_bytes())
            .map_err(Error::WriteMetadata)?;
        file.as_file().sync_all().map_err(Error::WriteMetadata)?;
        file.persist_noclobber(metadata.join("config.toml"))
            .map_err(|error| Error::WriteMetadata(error.error))?;
        Ok(())
    }

    pub fn status(&self) -> Status {
        Status {
            config_version: self.config.version,
            initialized: self.initialized,
            git_detected: self.root.join(".git").exists(),
            max_file_bytes: self.config.max_file_bytes,
        }
    }
}
