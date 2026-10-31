//! Read-only Git snapshot comparison and evidence-qualified change impact.
mod changes;
mod snapshots;

use contextforge_core::Repository;
use contextforge_graph::{Direction, Edge, Graph, Node, Status, Visit};
use contextforge_parser::{Outline, parse_source};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeSet,
    path::Path,
    time::{Duration, Instant},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Git index or source changed during analysis; retry to capture consistent evidence")]
    SnapshotChanged,
    #[error("Git repository is missing or does not match the authorized root")]
    Repository,
    #[error("unable to inspect Git snapshots; verify refs and repository state")]
    Git(#[from] git2::Error),
    #[error("source access or ignore policy validation failed")]
    Source(#[from] contextforge_core::Error),
    #[error("source parsing failed")]
    Parse(#[from] contextforge_parser::Error),
    #[error("dependency graph construction failed")]
    Graph(#[from] contextforge_graph::Error),
    #[error("impact exceeds snapshot/change/graph resource limits")]
    Limit,
    #[error("Git index has unresolved conflicts; resolve them before analysis")]
    Conflict,
    #[error("Git snapshot contains a non-UTF-8 path; analysis is incomplete and was aborted")]
    NonUtf8Path,
    #[error("invalid impact mode, relative path or limits")]
    InvalidQuery,
    #[error("Git impact cooperative time budget exhausted")]
    TimeBudget,
}

#[derive(Debug, Clone)]
pub enum Mode {
    Unstaged,
    Staged,
    Base { base: String, head: String },
    File(String),
}

/// A direct source change with separate zero-context old/new line ranges.
#[derive(Debug, Serialize)]
pub struct Change {
    pub old_path: Option<String>,
    pub new_path: Option<String>,
    pub kind: &'static str,
    pub hunks: Vec<ChangedRange>,
    pub previous_symbols: Vec<Node>,
    pub current_symbols: Vec<Node>,
    pub skipped: Option<&'static str>,
}
#[derive(Debug, Clone, Serialize)]
pub struct ChangedRange {
    pub old_start: usize,
    pub old_lines: usize,
    pub new_start: usize,
    pub new_lines: usize,
}

/// Impact is a bounded hypothesis with direct source/Git evidence, never a safety verdict.
#[derive(Debug, Serialize)]
pub struct ImpactReport {
    pub mode: &'static str,
    pub direct_changes: Vec<Change>,
    pub resolved_dependents: Vec<Visit>,
    pub heuristic_dependents: Vec<Visit>,
    pub potentially_affected_tests: Vec<Visit>,
    pub unresolved: Vec<Edge>,
    pub dependency_evidence: Vec<Edge>,
    pub truncated: bool,
    pub partial_syntax: bool,
    pub incomplete_content: bool,
    pub freshness: &'static str,
    pub uncertainty: &'static str,
}

/// Inspect selected snapshots without writing the index, Git state, source or external config.
pub fn analyze(
    repository: &Repository,
    mode: &Mode,
    depth: usize,
    limit: usize,
) -> Result<ImpactReport, Error> {
    validate_mode(mode, depth, limit)?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let git = git2::Repository::open_ext(
        repository.root(),
        git2::RepositoryOpenFlags::NO_SEARCH,
        std::iter::empty::<&Path>(),
    )
    .map_err(|_| Error::Repository)?;
    let workdir = git
        .workdir()
        .and_then(|path| std::fs::canonicalize(path).ok())
        .ok_or(Error::Repository)?;
    if workdir != repository.root() {
        return Err(Error::Repository);
    }
    let (old, new) = snapshots::capture(repository, &git, mode)?;
    // Match the canonical native-root identity used by the parser/index so IDs
    // remain repository-scoped across cached and Git-selected evidence.
    let repository_id = format!(
        "{:x}",
        Sha256::digest(repository.root().as_os_str().as_encoded_bytes())
    );
    let old_outlines = parse_snapshot(&old, &repository_id, deadline)?;
    let new_outlines = parse_snapshot(&new, &repository_id, deadline)?;
    let old_graph = Graph::from_outlines(&old_outlines)?;
    let new_graph = Graph::from_outlines(&new_outlines)?;
    let working_hunks = if matches!(mode, Mode::Unstaged) {
        Some(snapshots::working_hunks(
            repository, &git, &old, &new, deadline,
        )?)
    } else {
        None
    };
    let direct_changes = changes::compare(
        &old,
        &new,
        mode,
        &old_graph,
        &new_graph,
        working_hunks.as_ref(),
    )?;
    if Instant::now() > deadline {
        return Err(Error::TimeBudget);
    }
    build_report(mode, direct_changes, &old_graph, &new_graph, depth, limit)
}

fn validate_mode(mode: &Mode, depth: usize, limit: usize) -> Result<(), Error> {
    if depth > 8 || !(1..=500).contains(&limit) {
        return Err(Error::InvalidQuery);
    }
    match mode {
        Mode::File(path)
            if path.is_empty()
                || path.len() > 1024
                || path.chars().any(char::is_control)
                || Path::new(path)
                    .components()
                    .any(|part| !matches!(part, std::path::Component::Normal(_))) =>
        {
            Err(Error::InvalidQuery)
        }
        Mode::Base { base, head }
            if [base, head].iter().any(|reference| {
                reference.is_empty()
                    || reference.len() > 256
                    || !reference.chars().all(|c| {
                        c.is_alphanumeric() || matches!(c, '/' | '_' | '-' | '.' | '~' | '^')
                    })
            }) =>
        {
            Err(Error::InvalidQuery)
        }
        _ => Ok(()),
    }
}

fn parse_snapshot(
    snapshot: &snapshots::Snapshot,
    repository_id: &str,
    deadline: Instant,
) -> Result<Vec<Outline>, Error> {
    let mut outlines = Vec::new();
    for (path, file) in snapshot {
        if Instant::now() > deadline {
            return Err(Error::TimeBudget);
        }
        if let Some(text) = &file.text {
            outlines.push(parse_source(repository_id, path, text, 1000)?);
        }
    }
    Ok(outlines)
}

fn build_report(
    mode: &Mode,
    direct_changes: Vec<Change>,
    old: &Graph,
    new: &Graph,
    depth: usize,
    limit: usize,
) -> Result<ImpactReport, Error> {
    let mut report = ImpactReport {
        mode: match mode {
            Mode::Unstaged => "unstaged",
            Mode::Staged => "staged",
            Mode::Base { .. } => "merge_base_to_head",
            Mode::File(_) => "explicit_file",
        },
        incomplete_content: direct_changes.iter().any(|change| change.skipped.is_some()),
        direct_changes,
        resolved_dependents: Vec::new(),
        heuristic_dependents: Vec::new(),
        potentially_affected_tests: Vec::new(),
        unresolved: Vec::new(),
        dependency_evidence: Vec::new(),
        truncated: false,
        partial_syntax: old.partial_syntax || new.partial_syntax,
        freshness: "captured_git_snapshots",
        uncertainty: "Syntactic name/import candidates are heuristic. Missing edges, skipped content and incomplete syntax do not prove a change safe. Snapshots are captured sequentially and may race concurrent edits.",
    };
    let mut visited = BTreeSet::new();
    let mut evidence = BTreeSet::new();
    for (graph, previous) in [(new, false), (old, true)] {
        let mut seeds = Vec::new();
        for change in &report.direct_changes {
            let nodes = if previous {
                &change.previous_symbols
            } else {
                &change.current_symbols
            };
            seeds.extend(nodes.iter().map(|node| node.id.clone()));
            let path = if previous {
                &change.old_path
            } else {
                &change.new_path
            };
            if let Some(path) = path {
                let id = format!("file:{path}");
                if graph.nodes.contains_key(&id) {
                    seeds.push(id);
                }
            }
        }
        seeds.sort();
        seeds.dedup();
        let result = graph.traverse_ids(&seeds, Direction::Incoming, depth, limit, 1000)?;
        report.truncated |= result.truncated;
        for mut visit in result.nodes.into_iter().filter(|visit| visit.depth > 0) {
            visit.view = if previous { "previous" } else { "current" };
            let key = (visit.node.path.clone(), visit.node.qualified_name.clone());
            if !visited.insert(key) {
                continue;
            }
            if report.resolved_dependents.len() + report.heuristic_dependents.len() == limit {
                report.truncated = true;
                break;
            }
            if is_test(&visit.node) {
                report.potentially_affected_tests.push(visit.clone());
            }
            if visit.status == Status::Resolved {
                report.resolved_dependents.push(visit);
            } else {
                report.heuristic_dependents.push(visit);
            }
        }
        for mut edge in result.edges {
            edge.view = if previous { "previous" } else { "current" };
            if report.dependency_evidence.len() == 1000 {
                report.truncated = true;
                break;
            }
            let key = (
                edge.from.clone(),
                edge.to.clone(),
                edge.path.clone(),
                edge.span.as_ref().map(|span| span.start_byte),
            );
            if evidence.insert(key) {
                report.dependency_evidence.push(edge);
            }
        }
        // Unknown evidence in changed code or candidate dependents remains explicit.
        let related: BTreeSet<_> = seeds
            .into_iter()
            .chain(
                report
                    .heuristic_dependents
                    .iter()
                    .map(|visit| visit.node.id.clone()),
            )
            .collect();
        for edge in graph
            .edges
            .iter()
            .filter(|edge| edge.status == Status::Unknown && related.contains(&edge.from))
        {
            if report.unresolved.len() == 1000 {
                report.truncated = true;
                break;
            }
            let mut evidence = edge.clone();
            evidence.view = if previous { "previous" } else { "current" };
            report.unresolved.push(evidence);
        }
    }
    Ok(report)
}

fn is_test(node: &Node) -> bool {
    node.path
        .split('/')
        .any(|part| matches!(part, "test" | "tests"))
        || node.name.starts_with("test_")
        || node.path.contains(".test.")
        || node.path.contains(".spec.")
}
