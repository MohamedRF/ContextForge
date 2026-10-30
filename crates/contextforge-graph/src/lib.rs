//! Bounded syntax dependency candidates. No compiler-level binding is claimed.
mod candidates;
mod traversal;

use contextforge_parser::{Outline, Span};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use traversal::{Direction, GraphResult, Visit};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("invalid graph query or limits (depth 0–8, nodes 1–500, edges 1–1000)")]
    InvalidQuery,
    #[error("graph exceeds the bounded input policy")]
    TooLarge,
    #[error("cached syntax is invalid; reindex explicitly")]
    InvalidSyntax,
    #[error("unable to decode captured syntax")]
    Decode(#[from] serde_json::Error),
}

/// Source-linked definition or file node. Qualified names express lexical nesting.
#[derive(Debug, Clone, Serialize)]
pub struct Node {
    pub id: String,
    pub path: String,
    pub name: String,
    pub qualified_name: String,
    pub kind: String,
    pub language: String,
    pub span: Option<Span>,
    pub fingerprint: String,
    pub contains_errors: bool,
}

/// Certainty of a relation, independent of whether a syntactic occurrence exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Status {
    Resolved,
    Heuristic,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    Contains,
    Call,
    Import,
    Export,
}

/// `to: None` is unresolved evidence, never a fabricated graph target.
#[derive(Debug, Clone, Serialize)]
pub struct Edge {
    pub from: String,
    pub to: Option<String>,
    pub kind: EdgeKind,
    pub status: Status,
    pub path: String,
    pub span: Option<Span>,
    pub text: String,
    pub method: &'static str,
    pub fingerprint: String,
    pub view: &'static str,
}

#[derive(Deserialize)]
struct StoredSymbol {
    id: String,
    name: String,
    kind: String,
    parent_symbol: Option<String>,
    span: Span,
    contains_errors: bool,
}
#[derive(Deserialize)]
struct StoredOccurrence {
    kind: String,
    text: String,
    span: Span,
    parent_symbol: Option<String>,
    contains_errors: bool,
}
#[derive(Deserialize)]
struct StoredOutline {
    path: String,
    language: String,
    fingerprint: String,
    symbols: Vec<StoredSymbol>,
    occurrences: Vec<StoredOccurrence>,
    has_errors: bool,
    truncated: bool,
}

/// An in-memory graph derived from a bounded set of source-fingerprinted outlines.
pub struct Graph {
    pub nodes: BTreeMap<String, Node>,
    pub edges: Vec<Edge>,
    pub partial_syntax: bool,
    outgoing: BTreeMap<String, Vec<usize>>,
    incoming: BTreeMap<String, Vec<usize>>,
}

impl Graph {
    /// Build directly from current/historical parser results without persistence or I/O.
    pub fn from_outlines(outlines: &[Outline]) -> Result<Self, Error> {
        let payloads = outlines
            .iter()
            .map(serde_json::to_string)
            .collect::<Result<Vec<_>, _>>()?;
        Self::from_json(payloads.iter().map(String::as_str))
    }

    /// Decode the same versioned outline shape used by schema 2. Input is capped.
    pub fn from_json<'a>(payloads: impl IntoIterator<Item = &'a str>) -> Result<Self, Error> {
        let mut stored = Vec::new();
        let mut bytes = 0;
        for payload in payloads {
            bytes += payload.len();
            if stored.len() == 1000 || bytes > 64 * 1024 * 1024 {
                return Err(Error::TooLarge);
            }
            let outline: StoredOutline = serde_json::from_str(payload)?;
            if outline.symbols.len() + outline.occurrences.len() > 1000 {
                return Err(Error::InvalidSyntax);
            }
            stored.push(outline);
        }
        let mut graph = Self {
            nodes: BTreeMap::new(),
            edges: Vec::new(),
            partial_syntax: false,
            outgoing: BTreeMap::new(),
            incoming: BTreeMap::new(),
        };
        for outline in &stored {
            graph.add_nodes(outline)?;
        }
        let names = candidates::NameIndex::new(&graph.nodes);
        for outline in &stored {
            graph.add_occurrences(outline, &names)?;
        }
        graph.edges.sort_by(|a, b| {
            (
                &a.from,
                &a.to,
                a.kind,
                a.span.as_ref().map(|s| s.start_byte),
            )
                .cmp(&(
                    &b.from,
                    &b.to,
                    b.kind,
                    b.span.as_ref().map(|s| s.start_byte),
                ))
        });
        for (index, edge) in graph.edges.iter().enumerate() {
            graph
                .outgoing
                .entry(edge.from.clone())
                .or_default()
                .push(index);
            if let Some(target) = &edge.to {
                graph
                    .incoming
                    .entry(target.clone())
                    .or_default()
                    .push(index);
            }
        }
        Ok(graph)
    }

    fn add_nodes(&mut self, outline: &StoredOutline) -> Result<(), Error> {
        self.partial_syntax |= outline.has_errors || outline.truncated;
        let file_id = format!("file:{}", outline.path);
        if outline.path.len() > 1024
            || std::path::Path::new(&outline.path)
                .components()
                .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return Err(Error::InvalidSyntax);
        }
        self.insert_node(Node {
            id: file_id.clone(),
            path: outline.path.clone(),
            name: outline.path.clone(),
            qualified_name: outline.path.clone(),
            kind: "file".into(),
            language: outline.language.clone(),
            span: None,
            fingerprint: outline.fingerprint.clone(),
            contains_errors: outline.has_errors,
        })?;
        for symbol in &outline.symbols {
            if symbol.span.end_byte < symbol.span.start_byte
                || symbol.span.start_line == 0
                || symbol.span.end_line < symbol.span.start_line
            {
                return Err(Error::InvalidSyntax);
            }
            let parent = symbol.parent_symbol.as_ref().unwrap_or(&file_id);
            let parent_node = self.nodes.get(parent).ok_or(Error::InvalidSyntax)?;
            if parent_node.path != outline.path {
                return Err(Error::InvalidSyntax);
            }
            let qualified = if parent_node.kind == "file" {
                symbol.name.clone()
            } else {
                format!("{}::{}", parent_node.qualified_name, symbol.name)
            };
            self.edges.push(Edge {
                from: parent.clone(),
                to: Some(symbol.id.clone()),
                kind: EdgeKind::Contains,
                status: Status::Resolved,
                path: outline.path.clone(),
                span: Some(symbol.span.clone()),
                text: symbol.name.clone(),
                method: "syntax_containment",
                fingerprint: outline.fingerprint.clone(),
                view: "syntax_snapshot",
            });
            self.insert_node(Node {
                id: symbol.id.clone(),
                path: outline.path.clone(),
                name: symbol.name.clone(),
                qualified_name: qualified,
                kind: symbol.kind.clone(),
                language: outline.language.clone(),
                span: Some(symbol.span.clone()),
                fingerprint: outline.fingerprint.clone(),
                contains_errors: symbol.contains_errors,
            })?;
        }
        Ok(())
    }

    fn insert_node(&mut self, node: Node) -> Result<(), Error> {
        if self.nodes.len() >= 20_000 {
            return Err(Error::TooLarge);
        }
        if self.nodes.insert(node.id.clone(), node).is_some() {
            return Err(Error::InvalidSyntax);
        }
        Ok(())
    }

    fn add_occurrences(
        &mut self,
        outline: &StoredOutline,
        names: &candidates::NameIndex,
    ) -> Result<(), Error> {
        for occurrence in &outline.occurrences {
            if self.edges.len() >= 40_000 {
                return Err(Error::TooLarge);
            }
            let kind = match occurrence.kind.as_str() {
                "call" => EdgeKind::Call,
                "import" => EdgeKind::Import,
                "export" => EdgeKind::Export,
                _ => continue,
            };
            let from = occurrence
                .parent_symbol
                .clone()
                .unwrap_or_else(|| format!("file:{}", outline.path));
            if !self.nodes.contains_key(&from) {
                return Err(Error::InvalidSyntax);
            }
            let target = if occurrence.contains_errors || occurrence.text.chars().count() >= 256 {
                None
            } else {
                match kind {
                    EdgeKind::Call => names.call_target(&outline.language, &occurrence.text),
                    EdgeKind::Import => candidates::import_target(
                        &outline.path,
                        &outline.language,
                        &occurrence.text,
                        &self.nodes,
                    ),
                    _ => None,
                }
            };
            let status = if target.is_some() {
                Status::Heuristic
            } else {
                Status::Unknown
            };
            self.edges.push(Edge {
                from,
                to: target,
                kind,
                status,
                path: outline.path.clone(),
                span: Some(occurrence.span.clone()),
                text: occurrence.text.clone(),
                method: if status == Status::Heuristic {
                    "unique_syntax_candidate"
                } else {
                    "unresolved_syntax"
                },
                fingerprint: outline.fingerprint.clone(),
                view: "syntax_snapshot",
            });
        }
        Ok(())
    }

    /// Exact ID, lexical qualified name, simple symbol name, or file path selection.
    pub fn find_symbol(&self, selector: &str) -> Result<Vec<String>, Error> {
        if selector.is_empty() || selector.len() > 1024 || selector.chars().any(char::is_control) {
            return Err(Error::InvalidQuery);
        }
        if self.nodes.contains_key(selector) {
            return Ok(vec![selector.to_owned()]);
        }
        Ok(self
            .nodes
            .values()
            .filter(|n| n.qualified_name == selector || n.name == selector)
            .map(|n| n.id.clone())
            .collect())
    }

    /// Incoming candidate calls/imports plus matching unresolved occurrences. Never proven references.
    pub fn find_references(&self, selector: &str, limit: usize) -> Result<GraphResult, Error> {
        let mut result = self.traverse(selector, Direction::Incoming, 1, limit.min(500), limit)?;
        let seeds = self.find_symbol(selector)?;
        let names: Vec<_> = seeds
            .iter()
            .filter_map(|id| self.nodes.get(id))
            .map(|node| (&node.name, &node.qualified_name, &node.language))
            .collect();
        for edge in self
            .edges
            .iter()
            .filter(|edge| edge.to.is_none() && edge.kind == EdgeKind::Call)
        {
            let owner = self.nodes.get(&edge.from).ok_or(Error::InvalidSyntax)?;
            if !names.iter().any(|(name, qualified, language)| {
                *language == &owner.language && (edge.text == **name || edge.text == **qualified)
            }) {
                continue;
            }
            if result.edges.len() == limit {
                result.truncated = true;
                break;
            }
            if !result.nodes.iter().any(|visit| visit.node.id == edge.from) {
                if result.nodes.len() == limit.min(500) {
                    result.truncated = true;
                    break;
                }
                result.nodes.push(Visit {
                    node: owner.clone(),
                    depth: 1,
                    status: Status::Unknown,
                    view: "syntax_snapshot",
                });
            }
            result.edges.push(edge.clone());
        }
        Ok(result)
    }
}
