use crate::{Change, ChangedRange, Error, Mode, snapshots::Snapshot};
use contextforge_graph::{Graph, Node};
use git2::{DiffOptions, Patch};
use std::collections::BTreeSet;

pub(crate) fn compare(
    old: &Snapshot,
    new: &Snapshot,
    mode: &Mode,
    old_graph: &Graph,
    new_graph: &Graph,
) -> Result<Vec<Change>, Error> {
    let paths: BTreeSet<_> = old.keys().chain(new.keys()).collect();
    let mut changes = Vec::new();
    for path in paths {
        if let Mode::File(selected) = mode
            && std::path::Path::new(selected) != std::path::Path::new(path)
        {
            continue;
        }
        let before = old.get(path);
        let after = new.get(path);
        if !matches!(mode, Mode::File(_))
            && before.and_then(|file| file.text.as_ref())
                == after.and_then(|file| file.text.as_ref())
            && before.and_then(|file| file.reason) == after.and_then(|file| file.reason)
            && (before.and_then(|file| file.reason).is_none()
                || before.and_then(|file| file.revision) == after.and_then(|file| file.revision))
        {
            continue;
        }
        if changes.len() == 100 {
            return Err(Error::Limit);
        }
        let skipped = before
            .and_then(|file| file.reason)
            .or_else(|| after.and_then(|file| file.reason));
        let old_text = before.and_then(|file| file.text.as_deref()).unwrap_or("");
        let new_text = after.and_then(|file| file.text.as_deref()).unwrap_or("");
        let mut hunks = Vec::new();
        if skipped.is_none() {
            if matches!(mode, Mode::File(_)) {
                hunks.push(ChangedRange {
                    old_start: 1,
                    old_lines: old_text.lines().count(),
                    new_start: 1,
                    new_lines: new_text.lines().count(),
                });
            } else {
                let mut options = DiffOptions::new();
                options.context_lines(0).interhunk_lines(0);
                let patch = Patch::from_buffers(
                    old_text.as_bytes(),
                    None,
                    new_text.as_bytes(),
                    None,
                    Some(&mut options),
                )?;
                for index in 0..patch.num_hunks() {
                    if hunks.len() == 1000 {
                        return Err(Error::Limit);
                    }
                    let (hunk, _) = patch.hunk(index)?;
                    hunks.push(ChangedRange {
                        old_start: hunk.old_start() as usize,
                        old_lines: hunk.old_lines() as usize,
                        new_start: hunk.new_start() as usize,
                        new_lines: hunk.new_lines() as usize,
                    });
                }
            }
        }
        let previous_symbols = if skipped.is_some() {
            known_symbols(old_graph, path)
        } else {
            affected(old_graph, path, &hunks, true)
        };
        let current_symbols = if skipped.is_some() {
            known_symbols(new_graph, path)
        } else {
            affected(new_graph, path, &hunks, false)
        };
        changes.push(Change {
            old_path: before.map(|_| path.clone()),
            new_path: after.map(|_| path.clone()),
            kind: if before.is_none() {
                "added"
            } else if after.is_none() {
                "deleted"
            } else {
                "modified"
            },
            hunks,
            previous_symbols,
            current_symbols,
            skipped,
        });
    }
    if matches!(mode, Mode::File(_)) && changes.is_empty() {
        return Err(Error::InvalidQuery);
    }
    let total_hunks: usize = changes.iter().map(|change| change.hunks.len()).sum();
    let total_symbols: usize = changes
        .iter()
        .map(|change| change.current_symbols.len() + change.previous_symbols.len())
        .sum();
    if total_hunks > 1000 || total_symbols > 1000 {
        return Err(Error::Limit);
    }
    Ok(changes)
}

fn known_symbols(graph: &Graph, path: &str) -> Vec<Node> {
    graph
        .nodes
        .values()
        .filter(|node| node.path == path && node.span.is_some())
        .cloned()
        .collect()
}

fn affected(graph: &Graph, path: &str, hunks: &[ChangedRange], previous: bool) -> Vec<Node> {
    graph
        .nodes
        .values()
        .filter(|node| {
            node.path == path
                && node.span.as_ref().is_some_and(|span| {
                    hunks.iter().any(|hunk| {
                        let (start, lines) = if previous {
                            (hunk.old_start, hunk.old_lines)
                        } else {
                            (hunk.new_start, hunk.new_lines)
                        };
                        // Empty sides denote an insertion/deletion boundary, not a changed line.
                        lines > 0
                            && start <= span.end_line
                            && start.saturating_add(lines - 1) >= span.start_line
                    })
                })
        })
        .take(1000)
        .cloned()
        .collect()
}
