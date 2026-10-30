//! Deterministic bounded source-file map. No source code is executed.
use crate::{Error, Repository};
use ignore::{WalkBuilder, gitignore::GitignoreBuilder};
use serde::Serialize;
use std::{
    io::Read,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

fn validate_ignore_files(directory: &Path) -> Result<(), Error> {
    for name in [".gitignore", ".contextforgeignore"] {
        let path = directory.join(name);
        if crate::reject_link(&path, false)? {
            let metadata = std::fs::metadata(path).map_err(|_| Error::Discovery)?;
            if metadata.len() > 65_536 {
                return Err(Error::Discovery);
            }
        }
    }
    Ok(())
}

pub(crate) fn validate_patterns(root: &Path, patterns: &[String]) -> Result<(), Error> {
    pattern_matcher(root, patterns).map(|_| ())
}

fn pattern_matcher(
    root: &Path,
    patterns: &[String],
) -> Result<ignore::gitignore::Gitignore, Error> {
    let mut builder = GitignoreBuilder::new(root);
    for pattern in patterns {
        builder
            .add_line(None, pattern)
            .map_err(|_| Error::InvalidConfig)?;
    }
    builder.build().map_err(|_| Error::InvalidConfig)
}

#[derive(Debug, Serialize)]
pub struct SourceFile {
    pub path: String,
    pub language: &'static str,
    pub bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ProjectMap {
    pub files: Vec<SourceFile>,
    pub truncated: bool,
    pub inspected_entries: usize,
    pub method: &'static str,
}

fn language(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()? {
        "rs" => Some("rust"),
        "py" => Some("python"),
        "ts" | "tsx" => Some("typescript"),
        "js" | "jsx" | "mjs" | "cjs" => Some("javascript"),
        _ => None,
    }
}

impl Repository {
    /// Apply current source/ignore policy to a relative path, including absent historical files.
    /// Does not read source contents or follow links; ancestor ignore files remain scoped.
    pub fn source_path_allowed(&self, relative: &Path) -> Result<bool, Error> {
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(Error::InvalidRoot);
        }
        if language(relative).is_none() {
            return Ok(false);
        }
        let path = self.root().join(relative);
        let index_root = if self.config.index_root.is_absolute() {
            self.config.index_root.clone()
        } else {
            self.root()
                .join(".contextforge")
                .join(&self.config.index_root)
        };
        if path.starts_with(index_root)
            || relative
                .components()
                .any(|part| excluded_name(part.as_os_str().to_str().unwrap_or("")))
        {
            return Ok(false);
        }
        let configured = pattern_matcher(self.root(), &self.config.ignore_patterns)?;
        if configured
            .matched_path_or_any_parents(&path, false)
            .is_ignore()
        {
            return Ok(false);
        }
        let mut builder = GitignoreBuilder::new(self.root());
        let mut directory = self.root().to_path_buf();
        let parents: Vec<_> = relative
            .parent()
            .unwrap_or(Path::new(""))
            .components()
            .collect();
        if parents.len() > 63 {
            return Err(Error::Discovery);
        }
        for position in 0..=parents.len() {
            if position > 0 {
                directory.push(parents[position - 1].as_os_str());
                // A whitelisted leaf cannot resurrect a directory pruned by discovery.
                if configured
                    .matched_path_or_any_parents(&directory, true)
                    .is_ignore()
                    || builder
                        .build()
                        .map_err(|_| Error::Discovery)?
                        .matched_path_or_any_parents(&directory, true)
                        .is_ignore()
                {
                    return Ok(false);
                }
            }
            if !crate::reject_link(&directory, true)? {
                break;
            }
            validate_ignore_files(&directory)?;
            for name in [".gitignore", ".contextforgeignore"] {
                let ignore = directory.join(name);
                if ignore.exists()
                    && let Some(_error) = builder.add(ignore)
                {
                    return Err(Error::Discovery);
                }
            }
        }
        Ok(!builder
            .build()
            .map_err(|_| Error::Discovery)?
            .matched_path_or_any_parents(&path, false)
            .is_ignore())
    }

    /// Maps at most 1000 UTF-8 source files, inspecting at most 10000 entries.
    /// Skips links/reparse points, binary data, non-UTF-8 paths/content and oversized files.
    pub fn project_map(&self, limit: usize) -> Result<ProjectMap, Error> {
        if !(1..=1000).contains(&limit) {
            return Err(Error::InvalidLimit);
        }
        validate_ignore_files(self.root())?;
        let matcher = pattern_matcher(self.root(), &self.config.ignore_patterns)?;
        let index_root = if self.config.index_root.is_absolute() {
            self.config.index_root.clone()
        } else {
            self.root()
                .join(".contextforge")
                .join(&self.config.index_root)
        };
        let mut builder = WalkBuilder::new(self.root());
        let index_root = std::fs::canonicalize(&index_root).unwrap_or(index_root);
        builder
            .hidden(false)
            .parents(false)
            .ignore(false)
            .follow_links(false)
            .require_git(false)
            .git_global(false)
            .git_exclude(false)
            .add_custom_ignore_filename(".contextforgeignore")
            .max_depth(Some(64))
            .sort_by_file_path(|a, b| a.cmp(b));
        let invalid_ignore = Arc::new(AtomicBool::new(false));
        let invalid_ignore_filter = invalid_ignore.clone();
        builder.filter_entry(move |entry| {
            let name = entry.file_name().to_str().unwrap_or("");
            if excluded_name(name) {
                return false;
            }
            if entry.path() == index_root {
                return false;
            }
            #[cfg(windows)]
            if let Ok(meta) = std::fs::symlink_metadata(entry.path()) {
                use std::os::windows::fs::MetadataExt;
                if meta.file_attributes() & 0x400 != 0 {
                    return false;
                }
            }
            if entry.file_type().is_some_and(|t| t.is_dir())
                && validate_ignore_files(entry.path()).is_err()
            {
                invalid_ignore_filter.store(true, Ordering::Relaxed);
                return false;
            }
            !matcher
                .matched_path_or_any_parents(
                    entry.path(),
                    entry.file_type().is_some_and(|t| t.is_dir()),
                )
                .is_ignore()
        });
        let mut result = ProjectMap {
            files: Vec::new(),
            truncated: false,
            inspected_entries: 0,
            method: "filesystem_source_inventory",
        };
        for entry in builder.build() {
            if result.inspected_entries == 10_000 {
                result.truncated = true;
                break;
            }
            result.inspected_entries += 1;
            let entry = entry.map_err(|_| Error::Discovery)?;
            if entry.error().is_some() {
                return Err(Error::Discovery);
            }
            if entry.depth() == 64 && entry.file_type().is_some_and(|t| t.is_dir()) {
                result.truncated = true;
            }
            if !entry.file_type().is_some_and(|t| t.is_file()) {
                continue;
            }
            let Some(language) = language(entry.path()) else {
                continue;
            };
            let canonical = std::fs::canonicalize(entry.path()).map_err(|_| Error::Discovery)?;
            if !canonical.starts_with(self.root()) {
                return Err(Error::Discovery);
            }
            let meta = entry.metadata().map_err(|_| Error::Discovery)?;
            if meta.len() > self.config.max_file_bytes {
                continue;
            }
            let mut bytes = Vec::new();
            std::fs::File::open(entry.path())
                .map_err(|_| Error::Discovery)?
                .take(self.config.max_file_bytes + 1)
                .read_to_end(&mut bytes)
                .map_err(|_| Error::Discovery)?;
            if bytes.len() as u64 > self.config.max_file_bytes
                || bytes.contains(&0)
                || std::str::from_utf8(&bytes).is_err()
            {
                continue;
            }
            let Some(path) = entry
                .path()
                .strip_prefix(self.root())
                .ok()
                .and_then(|p| p.to_str())
            else {
                continue;
            };
            if result.files.len() == limit {
                result.truncated = true;
                break;
            }
            #[cfg(windows)]
            let path = path.replace('\\', "/");
            #[cfg(not(windows))]
            let path = path.to_owned();
            result.files.push(SourceFile {
                path,
                language,
                bytes: bytes.len() as u64,
            });
        }
        if invalid_ignore.load(Ordering::Relaxed) {
            return Err(Error::Discovery);
        }
        Ok(result)
    }
}

fn excluded_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".contextforge"
            | ".tools"
            | "node_modules"
            | "target"
            | "dist"
            | "build"
            | "__pycache__"
            | ".venv"
    )
}
