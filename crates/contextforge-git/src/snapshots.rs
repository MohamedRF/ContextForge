use crate::{ChangedRange, Error, Mode};
use contextforge_core::Repository;
use git2::{ObjectType, Oid, Repository as GitRepository};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    time::Instant,
};

/// Source selected from Git or the bounded working tree. Unsupported content stays explicit.
pub(crate) struct Source {
    pub text: Option<String>,
    pub reason: Option<&'static str>,
    pub revision: Option<Oid>,
}
pub(crate) type Snapshot = BTreeMap<String, Source>;

/// Let libgit2 apply its own index/worktree conversion policy in one bounded diff.
/// Never normalize captured source or invoke external diff/filter commands.
pub(crate) fn working_hunks(
    repository: &Repository,
    git: &GitRepository,
    old: &Snapshot,
    new: &Snapshot,
    deadline: Instant,
) -> Result<BTreeMap<String, Vec<ChangedRange>>, Error> {
    let paths: BTreeSet<_> = old
        .keys()
        .chain(new.keys())
        .filter(|path| {
            let before = old.get(*path);
            let after = new.get(*path);
            before.and_then(|file| file.reason).is_none()
                && after.and_then(|file| file.reason).is_none()
                && before.and_then(|file| file.text.as_ref())
                    != after.and_then(|file| file.text.as_ref())
        })
        .cloned()
        .collect();
    let mut result: BTreeMap<_, Vec<ChangedRange>> = paths
        .iter()
        .cloned()
        .map(|path| (path, Vec::new()))
        .collect();
    if paths.is_empty() {
        return Ok(result);
    }
    let mut index = git.index()?;
    index.read(true)?;
    if index.has_conflicts() {
        return Err(Error::Conflict);
    }
    if index.len() > 10_000 {
        return Err(Error::Limit);
    }
    let mut options = git2::DiffOptions::new();
    options
        .context_lines(0)
        .interhunk_lines(0)
        .disable_pathspec_match(true)
        .include_untracked(true)
        .recurse_untracked_dirs(true)
        .show_untracked_content(true)
        .force_text(true)
        .ignore_submodules(true)
        .max_size(repository.config.max_file_bytes as i64);
    for path in &paths {
        if index.get_path(Path::new(path), 0).map(|entry| entry.id)
            != old.get(path).and_then(|file| file.revision)
        {
            return Err(Error::SnapshotChanged);
        }
        options.pathspec(path);
    }
    let diff = git.diff_index_to_workdir(Some(&index), Some(&mut options))?;
    for (position, delta) in diff.deltas().enumerate() {
        if Instant::now() > deadline {
            return Err(Error::TimeBudget);
        }
        let path = delta
            .new_file()
            .path()
            .or_else(|| delta.old_file().path())
            .and_then(Path::to_str)
            .ok_or(Error::NonUtf8Path)?
            .to_owned();
        let hunks = result.get_mut(&path).ok_or(Error::SnapshotChanged)?;
        if let Some(patch) = git2::Patch::from_diff(&diff, position)? {
            if patch.num_hunks() > 1000 {
                return Err(Error::Limit);
            }
            for hunk_index in 0..patch.num_hunks() {
                let (hunk, _) = patch.hunk(hunk_index)?;
                hunks.push(ChangedRange {
                    old_start: hunk.old_start() as usize,
                    old_lines: hunk.old_lines() as usize,
                    new_start: hunk.new_start() as usize,
                    new_lines: hunk.new_lines() as usize,
                });
            }
        }
    }
    // A second authorized read binds the Git ranges to the captured source. Fail
    // explicitly if a concurrent edit or link replacement invalidated that evidence.
    for path in paths {
        if Instant::now() > deadline {
            return Err(Error::TimeBudget);
        }
        if let Some(expected) = new.get(&path).and_then(|file| file.text.as_deref()) {
            if repository.read_source(Path::new(&path))? != expected {
                return Err(Error::SnapshotChanged);
            }
        } else {
            match std::fs::symlink_metadata(repository.root().join(path)) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                _ => return Err(Error::SnapshotChanged),
            }
        }
    }
    Ok(result)
}

pub(crate) fn capture(
    repository: &Repository,
    git: &GitRepository,
    mode: &Mode,
) -> Result<(Snapshot, Snapshot), Error> {
    match mode {
        Mode::Unstaged | Mode::File(_) => {
            let old = index_snapshot(repository, git)?;
            let current = worktree(repository, &old)?;
            Ok((old, current))
        }
        Mode::Staged => {
            let old = match git.head() {
                Ok(head) => tree_snapshot(repository, git, &head.peel_to_tree()?)?,
                Err(error)
                    if matches!(
                        error.code(),
                        git2::ErrorCode::UnbornBranch | git2::ErrorCode::NotFound
                    ) =>
                {
                    BTreeMap::new()
                }
                Err(error) => return Err(error.into()),
            };
            Ok((old, index_snapshot(repository, git)?))
        }
        Mode::Base { base, head } => {
            let base_commit = git.revparse_single(base)?.peel_to_commit()?;
            let head_commit = git.revparse_single(head)?.peel_to_commit()?;
            let ancestor = git.merge_base(base_commit.id(), head_commit.id())?;
            Ok((
                tree_snapshot(repository, git, &git.find_commit(ancestor)?.tree()?)?,
                tree_snapshot(repository, git, &head_commit.tree()?)?,
            ))
        }
    }
}

fn source(bytes: &[u8], max: u64) -> Source {
    if bytes.len() as u64 > max {
        return Source {
            text: None,
            reason: Some("oversized"),
            revision: None,
        };
    }
    if bytes.contains(&0) {
        return Source {
            text: None,
            reason: Some("binary"),
            revision: None,
        };
    }
    match std::str::from_utf8(bytes) {
        Ok(text) => Source {
            text: Some(text.to_owned()),
            reason: None,
            revision: None,
        },
        Err(_) => Source {
            text: None,
            reason: Some("non_utf8"),
            revision: None,
        },
    }
}

fn insert_blob(
    repository: &Repository,
    git: &GitRepository,
    files: &mut Snapshot,
    path: String,
    id: Oid,
    mode: u32,
) -> Result<(), Error> {
    if !repository.source_path_allowed(Path::new(&path))? {
        return Ok(());
    }
    if files.len() >= 1000 {
        return Err(Error::Limit);
    }
    if mode & 0o170000 != 0o100000 {
        files.insert(
            path,
            Source {
                text: None,
                reason: Some("non_regular_git_entry"),
                revision: Some(id),
            },
        );
        return Ok(());
    }
    // Check the object header before materializing the complete blob in memory.
    let (size, kind) = git.odb()?.read_header(id)?;
    if kind != ObjectType::Blob {
        return Err(Error::Repository);
    }
    let mut value = if size as u64 > repository.config.max_file_bytes {
        Source {
            text: None,
            reason: Some("oversized"),
            revision: Some(id),
        }
    } else {
        let blob = git.find_blob(id)?;
        source(blob.content(), repository.config.max_file_bytes)
    };
    value.revision = Some(id);
    files.insert(path, value);
    check_size(files)
}

fn index_snapshot(repository: &Repository, git: &GitRepository) -> Result<Snapshot, Error> {
    let index = git.index()?;
    if index.has_conflicts() {
        return Err(Error::Conflict);
    }
    if index.len() > 10_000 {
        return Err(Error::Limit);
    }
    let mut files = BTreeMap::new();
    for entry in index.iter() {
        let path = std::str::from_utf8(&entry.path)
            .map_err(|_| Error::NonUtf8Path)?
            .to_owned();
        insert_blob(repository, git, &mut files, path, entry.id, entry.mode)?;
    }
    Ok(files)
}

fn tree_snapshot(
    repository: &Repository,
    git: &GitRepository,
    tree: &git2::Tree<'_>,
) -> Result<Snapshot, Error> {
    let mut files = BTreeMap::new();
    let mut pending = vec![(String::new(), tree.id(), 0)];
    let mut inspected = 0;
    while let Some((prefix, id, depth)) = pending.pop() {
        if depth > 64 {
            return Err(Error::Limit);
        }
        let tree = git.find_tree(id)?;
        for entry in tree.iter() {
            inspected += 1;
            if inspected > 10_000 {
                return Err(Error::Limit);
            }
            let name = entry.name().map_err(|_| Error::NonUtf8Path)?;
            let path = if prefix.is_empty() {
                name.to_owned()
            } else {
                format!("{prefix}/{name}")
            };
            if entry.kind() == Some(ObjectType::Tree) {
                pending.push((path, entry.id(), depth + 1));
            } else {
                insert_blob(
                    repository,
                    git,
                    &mut files,
                    path,
                    entry.id(),
                    entry.filemode() as u32,
                )?;
            }
        }
    }
    Ok(files)
}

fn worktree(repository: &Repository, tracked: &Snapshot) -> Result<Snapshot, Error> {
    let map = repository.project_map(1000)?;
    if map.truncated {
        return Err(Error::Limit);
    }
    let mut files = BTreeMap::new();
    for file in map.files {
        files.insert(
            file.path.clone(),
            Source {
                text: Some(repository.read_source(Path::new(&file.path))?),
                reason: None,
                revision: None,
            },
        );
    }
    for path in tracked.keys() {
        if files.contains_key(path) || !repository.source_path_allowed(Path::new(path))? {
            continue;
        }
        match std::fs::symlink_metadata(repository.root().join(path)) {
            Ok(_) => {
                files.insert(
                    path.clone(),
                    Source {
                        text: None,
                        reason: Some("excluded_or_unreadable_current_content"),
                        revision: None,
                    },
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(Error::Repository),
        }
    }
    check_size(&files)?;
    Ok(files)
}

fn check_size(files: &Snapshot) -> Result<(), Error> {
    let bytes: usize = files
        .values()
        .filter_map(|file| file.text.as_ref())
        .map(String::len)
        .sum();
    if bytes > 64 * 1024 * 1024 {
        Err(Error::Limit)
    } else {
        Ok(())
    }
}
