//! Offline context packages with explicit serialized-output token estimates.
use contextforge_core::Repository;
use contextforge_graph::{Direction, Edge, Graph, Node, Status};
use contextforge_index::{search::SearchHit, snapshot::SourceFile};
use contextforge_parser::Span;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("query must be 1–256 non-control bytes; budget must be 256–32000 estimated tokens")]
    InvalidQuery,
    #[error("unable to retrieve captured context; run index explicitly and check limits")]
    Index(#[from] contextforge_index::Error),
    #[error("captured syntax is invalid or exceeds graph limits")]
    Graph(#[from] contextforge_graph::Error),
    #[error("unable to serialize context evidence")]
    Serialization(#[from] serde_json::Error),
    #[error("context metadata exceeds budget; increase the budget")]
    Budget,
}

/// Estimators count the compact JSON package, including metadata and escaped source.
/// Implementations must be deterministic and monotonic as serialized bytes are added.
pub trait TokenEstimator {
    fn name(&self) -> &'static str;
    fn estimate(&self, text: &str) -> usize;
}

/// Approximation only: ceil(UTF-8 bytes / 4), not a model-specific tokenizer.
pub struct ByteEstimator;
impl TokenEstimator for ByteEstimator {
    fn name(&self) -> &'static str {
        "utf8_bytes_div_4_ceil_v1"
    }
    fn estimate(&self, text: &str) -> usize {
        text.len().div_ceil(4)
    }
}

#[derive(Debug, Serialize)]
pub struct Snippet {
    pub path: String,
    pub symbol: Option<String>,
    pub span: Span,
    pub fingerprint: String,
    pub snippet: String,
    pub reason: &'static str,
    pub confidence: &'static str,
    pub contains_errors: bool,
    pub git_proximity: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<Edge>,
}

#[derive(Debug, Serialize)]
pub struct Package {
    pub query: String,
    pub budget_tokens: usize,
    pub estimated_tokens: usize,
    pub estimator: &'static str,
    pub freshness: &'static str,
    pub results: Vec<Snippet>,
    pub truncated: bool,
    pub partial_syntax: bool,
}

struct Candidate {
    node_id: Option<String>,
    tier: u8,
    order: usize,
    snippet: Snippet,
}

/// Validate before accessing the filesystem or database.
pub fn validate(query: &str, budget: usize) -> Result<(), Error> {
    if query.trim().is_empty()
        || query.len() > 256
        || query.chars().any(char::is_control)
        || !(256..=32_000).contains(&budget)
    {
        return Err(Error::InvalidQuery);
    }
    Ok(())
}

fn queries(query: &str) -> (Vec<String>, bool) {
    let mut queries = vec![query.trim().to_owned()];
    let mut seen = BTreeSet::new();
    let mut truncated = false;
    for word in query.split(|c: char| !c.is_alphanumeric() && c != '_') {
        let lower = word.to_lowercase();
        if word.len() < 2
            || [
                "the", "a", "an", "and", "or", "to", "of", "in", "for", "explain", "modify", "how",
                "does", "flow",
            ]
            .contains(&lower.as_str())
            || !seen.insert(lower)
        {
            continue;
        }
        if queries.iter().any(|q| q == word) {
            continue;
        }
        if queries.len() == 9 {
            truncated = true;
            break;
        }
        queries.push(word.to_owned());
    }
    (queries, truncated)
}

/// Pack a captured index. Optional Git paths influence ranking only, never citation freshness.
/// Git collection is deliberately caller-controlled; this engine does no hidden Git work.
pub fn pack(
    repository: &Repository,
    query: &str,
    budget: usize,
    git_paths: &BTreeSet<String>,
) -> Result<Package, Error> {
    pack_with(repository, query, budget, git_paths, &ByteEstimator)
}

pub fn pack_with(
    repository: &Repository,
    query: &str,
    budget: usize,
    git_paths: &BTreeSet<String>,
    estimator: &impl TokenEstimator,
) -> Result<Package, Error> {
    validate(query, budget)?;
    let (queries, mut truncated) = queries(query);
    let retrieval = contextforge_index::snapshot::retrieve(repository, &queries)?;
    for file in &retrieval.files {
        let value: serde_json::Value = serde_json::from_str(&file.syntax.payload)?;
        if value["path"] != file.syntax.path || value["fingerprint"] != file.syntax.fingerprint {
            return Err(contextforge_graph::Error::InvalidSyntax.into());
        }
    }
    let graph = Graph::from_json(retrieval.files.iter().map(|f| f.syntax.payload.as_str()))?;
    let files: BTreeMap<_, _> = retrieval
        .files
        .iter()
        .map(|f| (f.syntax.path.as_str(), f))
        .collect();
    let mut candidates = Vec::new();
    for (order, search) in retrieval.searches.into_iter().enumerate() {
        truncated |= search.truncated;
        for hit in search.results {
            let file = files
                .get(hit.path.as_str())
                .ok_or(contextforge_graph::Error::InvalidSyntax)?;
            match from_hit(&hit, file, &graph, git_paths, order)? {
                Some(candidate) => candidates.push(candidate),
                None => truncated = true,
            }
        }
    }
    candidates.sort_by(|a, b| {
        (
            a.tier,
            !a.snippet.git_proximity,
            a.order,
            &a.snippet.path,
            a.snippet.span.start_byte,
        )
            .cmp(&(
                b.tier,
                !b.snippet.git_proximity,
                b.order,
                &b.snippet.path,
                b.snippet.span.start_byte,
            ))
    });
    let mut seen_seeds = BTreeSet::new();
    let seeds: Vec<_> = candidates
        .iter()
        .filter_map(|c| c.node_id.clone())
        .filter(|id| seen_seeds.insert(id.clone()))
        .take(8)
        .collect();
    for direction in [Direction::Outgoing, Direction::Incoming] {
        let expansion = graph.traverse_ids(&seeds, direction, 1, 32, 64)?;
        truncated |= expansion.truncated;
        for visit in expansion
            .nodes
            .into_iter()
            .filter(|v| v.node.span.is_some())
        {
            let relation = expansion
                .edges
                .iter()
                .find(|e| {
                    e.kind != contextforge_graph::EdgeKind::Contains
                        && (e.to.as_deref() == Some(&visit.node.id) || e.from == visit.node.id)
                })
                .cloned();
            let Some(relation) = relation else {
                continue;
            };
            let file = files
                .get(visit.node.path.as_str())
                .ok_or(contextforge_graph::Error::InvalidSyntax)?;
            if let Some(mut snippet) = from_node(&visit.node, file, git_paths)? {
                snippet.reason = "syntax_graph_neighbor";
                snippet.confidence = if relation.status == Status::Resolved {
                    "syntax_only"
                } else {
                    "heuristic"
                };
                snippet.relation = Some(relation);
                if let Some(existing) = candidates
                    .iter_mut()
                    .find(|c| c.snippet.path == snippet.path && c.snippet.span == snippet.span)
                {
                    existing.snippet.relation = snippet.relation;
                    existing.snippet.confidence = snippet.confidence;
                } else {
                    candidates.push(Candidate {
                        node_id: Some(visit.node.id),
                        tier: 4,
                        order: 0,
                        snippet,
                    });
                }
            } else {
                truncated = true;
            }
        }
    }
    // Prefer coverage: first visit the highest ranked candidate from each file,
    // then consume remaining candidates in their retrieval order.
    let mut seen_files = BTreeSet::new();
    let mut first = Vec::new();
    let mut rest = Vec::new();
    for candidate in candidates {
        if seen_files.insert(candidate.snippet.path.clone()) {
            first.push(candidate);
        } else {
            rest.push(candidate);
        }
    }
    let mut package = Package {
        query: query.trim().into(),
        budget_tokens: budget,
        estimated_tokens: budget,
        estimator: estimator.name(),
        freshness: "snapshot_not_checked",
        results: Vec::new(),
        truncated,
        partial_syntax: graph.partial_syntax,
    };
    // Reserve metadata using the largest possible count and boolean spelling.
    if estimate(&package, estimator)? > budget {
        return Err(Error::Budget);
    }
    for candidate in first.into_iter().chain(rest) {
        let snippet = candidate.snippet;
        if package.results.iter().any(|old| {
            old.path == snippet.path
                && old.span.start_byte < snippet.span.end_byte
                && snippet.span.start_byte < old.span.end_byte
        }) {
            continue;
        }
        if package.results.len() == 32 {
            package.truncated = true;
            break;
        }
        package.results.push(snippet);
        if estimate(&package, estimator)? > budget {
            package.results.pop();
            package.truncated = true;
        }
    }
    // Count the final count field itself. Its digit width can only decrease from budget.
    for _ in 0..16 {
        let count = estimator.estimate(&serde_json::to_string(&package)?);
        if count == package.estimated_tokens {
            return Ok(package);
        }
        package.estimated_tokens = count;
    }
    Err(Error::Budget)
}

fn estimate(package: &Package, estimator: &impl TokenEstimator) -> Result<usize, Error> {
    // `false` is longer than `true`; reserve it even if later candidates are omitted.
    let mut value = serde_json::to_value(package)?;
    value["truncated"] = serde_json::Value::Bool(false);
    Ok(estimator.estimate(&serde_json::to_string(&value)?))
}

fn from_hit(
    hit: &SearchHit,
    file: &SourceFile,
    graph: &Graph,
    git_paths: &BTreeSet<String>,
    order: usize,
) -> Result<Option<Candidate>, Error> {
    let node = graph
        .nodes
        .values()
        .filter(|node| {
            node.path == hit.path
                && node.span.as_ref().is_some_and(|s| {
                    s.start_byte <= hit.span.start_byte && s.end_byte >= hit.span.end_byte
                })
        })
        .min_by_key(|node| node.span.as_ref().map(|s| s.end_byte - s.start_byte));
    let snippet = if let Some(node) = node {
        from_node(node, file, git_paths)?
    } else {
        // Text-only evidence uses complete lines around the matched passage.
        let start = file
            .content
            .get(..hit.span.start_byte)
            .ok_or(contextforge_graph::Error::InvalidSyntax)?
            .rfind('\n')
            .map_or(0, |b| b + 1);
        let end = file
            .content
            .get(hit.span.end_byte..)
            .ok_or(contextforge_graph::Error::InvalidSyntax)?
            .find('\n')
            .map_or(file.content.len(), |b| hit.span.end_byte + b + 1);
        make_snippet(
            file,
            None,
            start,
            end,
            hit.contains_errors.unwrap_or(false),
            git_paths,
        )?
    };
    Ok(snippet.map(|mut snippet| {
        snippet.reason = hit.method;
        Candidate {
            node_id: node.map(|n| n.id.clone()),
            tier: hit.rank_tier,
            order,
            snippet,
        }
    }))
}

fn from_node(
    node: &Node,
    file: &SourceFile,
    git_paths: &BTreeSet<String>,
) -> Result<Option<Snippet>, Error> {
    let Some(span) = &node.span else {
        return Ok(None);
    };
    make_snippet(
        file,
        Some(node.name.clone()),
        span.start_byte,
        span.end_byte,
        node.contains_errors,
        git_paths,
    )
}

fn make_snippet(
    file: &SourceFile,
    symbol: Option<String>,
    start: usize,
    end: usize,
    contains_errors: bool,
    git_paths: &BTreeSet<String>,
) -> Result<Option<Snippet>, Error> {
    let text = file
        .content
        .get(start..end)
        .ok_or(contextforge_graph::Error::InvalidSyntax)?;
    if text.is_empty() || text.len() > 16_384 {
        return Ok(None);
    }
    let line = |offset| {
        file.content.as_bytes()[..offset]
            .iter()
            .filter(|b| **b == b'\n')
            .count()
            + 1
    };
    Ok(Some(Snippet {
        path: file.syntax.path.clone(),
        symbol,
        span: Span {
            start_line: line(start),
            end_line: line(end - 1),
            start_byte: start,
            end_byte: end,
        },
        fingerprint: file.syntax.fingerprint.clone(),
        snippet: text.into(),
        reason: "source_match",
        confidence: "syntax_only",
        contains_errors,
        git_proximity: git_paths.contains(&file.syntax.path),
        relation: None,
    }))
}
