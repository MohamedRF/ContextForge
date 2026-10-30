//! Bounded syntax outlines. Tree-sitter does not resolve semantic references.

mod adapters;

use contextforge_core::Repository;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    path::Path,
    time::{Duration, Instant},
};
use tree_sitter::{Node, ParseOptions, Parser};

/// User-safe parser failures never contain source text or absolute paths.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("source is not an eligible file under the authorized root")]
    Ineligible,
    #[error("source access failed")]
    Source(#[from] contextforge_core::Error),
    #[error("source language is unsupported")]
    Unsupported,
    #[error("limit must be between 1 and 1000")]
    Limit,
    #[error("source exceeds the parser input limit or contains binary data")]
    Input,
    #[error("parser grammar is incompatible")]
    Grammar,
    #[error("source parsing exceeded its time limit")]
    Timeout,
}

/// One-based inclusive lines and zero-based half-open UTF-8 byte offsets.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, serde::Deserialize)]
pub struct Span {
    pub start_line: usize,
    pub end_line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

impl Span {
    fn of(node: Node<'_>) -> Self {
        let end = node.end_position();
        Self {
            start_line: node.start_position().row + 1,
            end_line: if end.column == 0 && node.end_byte() > node.start_byte() {
                end.row
            } else {
                end.row + 1
            },
            start_byte: node.start_byte(),
            end_byte: node.end_byte(),
        }
    }
}

/// Syntactic definition, with identity stable across unrelated byte/line shifts.
#[derive(Debug, Serialize)]
pub struct Symbol {
    pub id: String,
    pub name: String,
    pub kind: &'static str,
    pub parent_symbol: Option<String>,
    pub visibility: &'static str,
    pub span: Span,
    pub signature: String,
    pub contains_errors: bool,
}

/// Source-backed syntax occurrence; no target binding is inferred.
#[derive(Debug, Serialize)]
pub struct Occurrence {
    pub kind: &'static str,
    pub text: String,
    pub span: Span,
    pub parent_symbol: Option<String>,
    pub status: &'static str,
    pub contains_errors: bool,
}

/// A current-source outline; source text is evidence, never operational instructions.
#[derive(Debug, Serialize)]
pub struct Outline {
    pub repository_id: String,
    pub path: String,
    pub language: &'static str,
    pub fingerprint: String,
    pub method: &'static str,
    pub symbols: Vec<Symbol>,
    pub occurrences: Vec<Occurrence>,
    pub has_errors: bool,
    pub truncated: bool,
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Analyze an eligible current file. Ignore, size, containment and link policy match discovery.
/// Fails closed if bounded discovery cannot reach the requested path.
pub fn analyze(repository: &Repository, path: &Path, limit: usize) -> Result<Outline, Error> {
    if !(1..=1000).contains(&limit) {
        return Err(Error::Limit);
    }
    if path.as_os_str().is_empty()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(Error::Ineligible);
    }
    let relative = path.to_str().ok_or(Error::Ineligible)?;
    #[cfg(windows)]
    let normalized = relative.replace('\\', "/");
    #[cfg(windows)]
    let relative = normalized.as_str();
    let map = repository.project_map(1000)?;
    if !map.files.iter().any(|file| file.path == relative) {
        return Err(Error::Ineligible);
    }
    let source = repository.read_source(path)?;
    let repository_id = digest(repository.root().as_os_str().as_encoded_bytes());
    parse_source(&repository_id, relative, &source, limit)
}

/// Parse supplied UTF-8 evidence with a stable repository ID and relative file path.
/// At most `limit` combined definitions/occurrences are returned, with bounded traversal.
pub fn parse_source(
    repository_id: &str,
    path: &str,
    source: &str,
    limit: usize,
) -> Result<Outline, Error> {
    if !(1..=1000).contains(&limit) {
        return Err(Error::Limit);
    }
    if source.len() > 16 * 1024 * 1024 || source.contains('\0') {
        return Err(Error::Input);
    }
    let (language, grammar) = adapters::language(Path::new(path))?;
    let mut parser = Parser::new();
    parser.set_language(&grammar).map_err(|_| Error::Grammar)?;
    let started = Instant::now();
    let mut cancelled = |_: &tree_sitter::ParseState| {
        if started.elapsed() > Duration::from_secs(2) {
            std::ops::ControlFlow::Break(())
        } else {
            std::ops::ControlFlow::Continue(())
        }
    };
    let options = ParseOptions::new().progress_callback(&mut cancelled);
    let bytes = source.as_bytes();
    let tree = parser
        .parse_with_options(
            &mut |offset, _| bytes.get(offset..).unwrap_or_default(),
            None,
            Some(options),
        )
        .ok_or(Error::Timeout)?;
    let mut outline = Outline {
        repository_id: repository_id.to_owned(),
        path: path.to_owned(),
        language,
        fingerprint: digest(bytes),
        method: "tree_sitter_syntax",
        symbols: Vec::new(),
        occurrences: Vec::new(),
        has_errors: tree.root_node().has_error(),
        truncated: false,
    };
    let mut cursor = tree.walk();
    let mut parent = None;
    let mut ancestry = Vec::new();
    let mut duplicates = BTreeMap::new();
    let mut visited = 0;
    loop {
        visited += 1;
        if visited > 100_000 || ancestry.len() > 256 {
            outline.truncated = true;
            break;
        }
        let node = cursor.node();
        let mut child_parent = parent;
        let definition = adapters::definition(node, source);
        let occurrence = adapters::occurrence(node);
        if definition.is_some() || occurrence.is_some() {
            if outline.symbols.len() + outline.occurrences.len() == limit {
                outline.truncated = true;
                break;
            }
            let parent_id = parent.map(|index: usize| outline.symbols[index].id.clone());
            if let Some((name, kind)) = definition {
                // Length-delimited tuple avoids ambiguous concatenation. Ordinals distinguish
                // duplicate definitions in one lexical parent, without using mutable byte offsets.
                let key = (parent_id.clone(), kind, name.to_owned());
                let ordinal = duplicates.entry(key).or_insert(0usize);
                let identity = format!(
                    "{}:{repository_id}{}:{path}{parent_id:?}{kind}{}:{name}:{ordinal}",
                    repository_id.len(),
                    path.len(),
                    name.len()
                );
                *ordinal += 1;
                let signature_end = node
                    .child_by_field_name("body")
                    .map_or(node.end_byte(), |body| body.start_byte());
                let signature = bounded_text(&source[node.start_byte()..signature_end]);
                child_parent = Some(outline.symbols.len());
                outline.symbols.push(Symbol {
                    id: digest(identity.as_bytes()),
                    name: bounded_text(name),
                    kind,
                    parent_symbol: parent_id,
                    visibility: adapters::visibility(node, language, source),
                    span: Span::of(node),
                    signature,
                    contains_errors: node.has_error(),
                });
            } else if let Some(kind) = occurrence {
                let evidence = if kind == "call" {
                    node.child_by_field_name("function")
                        .or_else(|| node.child_by_field_name("constructor"))
                        .unwrap_or(node)
                } else {
                    node
                };
                outline.occurrences.push(Occurrence {
                    kind,
                    text: bounded_text(&source[evidence.byte_range()]),
                    span: Span::of(node),
                    parent_symbol: parent_id,
                    status: "unknown",
                    contains_errors: node.has_error(),
                });
            }
        }
        if cursor.goto_first_child() {
            ancestry.push(parent);
            parent = child_parent;
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                return Ok(outline);
            }
            parent = ancestry.pop().flatten();
        }
    }
    Ok(outline)
}

// Bound every source-controlled string in the result; spans retain the full evidence range.
fn bounded_text(text: &str) -> String {
    text.trim().chars().take(256).collect()
}
