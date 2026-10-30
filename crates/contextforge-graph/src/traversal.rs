use crate::{Edge, EdgeKind, Error, Graph, Node, Status};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, VecDeque};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    #[default]
    Outgoing,
    Incoming,
}

#[derive(Debug, Clone, Serialize)]
pub struct Visit {
    pub node: Node,
    pub depth: usize,
    pub status: Status,
    pub view: &'static str,
}
#[derive(Debug, Serialize)]
pub struct GraphResult {
    pub nodes: Vec<Visit>,
    pub edges: Vec<Edge>,
    pub truncated: bool,
    pub partial_syntax: bool,
    pub freshness: &'static str,
}

impl Graph {
    /// Cycle-safe breadth-first dependencies/dependents; limits bound both nodes and evidence.
    pub fn traverse(
        &self,
        selector: &str,
        direction: Direction,
        depth: usize,
        max_nodes: usize,
        max_edges: usize,
    ) -> Result<GraphResult, Error> {
        let seeds = self.find_symbol(selector)?;
        self.traverse_ids(&seeds, direction, depth, max_nodes, max_edges)
    }

    /// Shared traversal for impact analysis, which may start from multiple changed symbols.
    pub fn traverse_ids(
        &self,
        seeds: &[String],
        direction: Direction,
        depth: usize,
        max_nodes: usize,
        max_edges: usize,
    ) -> Result<GraphResult, Error> {
        if depth > 8 || !(1..=500).contains(&max_nodes) || !(1..=1000).contains(&max_edges) {
            return Err(Error::InvalidQuery);
        }
        let mut result = GraphResult {
            nodes: Vec::new(),
            edges: Vec::new(),
            truncated: false,
            partial_syntax: self.partial_syntax,
            freshness: "snapshot_not_checked",
        };
        let mut seen = BTreeSet::new();
        let mut edge_seen = BTreeSet::new();
        let mut queue = VecDeque::new();
        for id in seeds {
            if !self.nodes.contains_key(id) {
                return Err(Error::InvalidQuery);
            }
            if !seen.insert(id.clone()) {
                continue;
            }
            if result.nodes.len() == max_nodes {
                result.truncated = true;
                break;
            }
            result.nodes.push(Visit {
                node: self.nodes[id].clone(),
                depth: 0,
                status: Status::Resolved,
                view: "syntax_snapshot",
            });
            queue.push_back((id.clone(), 0, Status::Resolved));
        }
        while let Some((id, distance, certainty)) = queue.pop_front() {
            let adjacency = if direction == Direction::Outgoing {
                &self.outgoing
            } else {
                &self.incoming
            };
            let Some(indices) = adjacency.get(&id) else {
                continue;
            };
            if distance == depth {
                if indices.iter().any(|index| {
                    direction == Direction::Outgoing
                        || self.edges[*index].kind != EdgeKind::Contains
                }) {
                    result.truncated = true;
                }
                continue;
            }
            for &index in indices {
                let edge = &self.edges[index];
                // References/dependents follow dependency relations, not a child's lexical parent.
                if direction == Direction::Incoming && edge.kind == EdgeKind::Contains {
                    continue;
                }
                if edge_seen.contains(&index) {
                    continue;
                }
                if result.edges.len() == max_edges {
                    result.truncated = true;
                    break;
                }
                let next = if direction == Direction::Outgoing {
                    edge.to.as_ref()
                } else {
                    Some(&edge.from)
                };
                if let Some(next) = next
                    && !seen.contains(next)
                {
                    if result.nodes.len() == max_nodes {
                        result.truncated = true;
                        continue;
                    }
                    seen.insert(next.clone());
                    let status = certainty.max(edge.status);
                    result.nodes.push(Visit {
                        node: self.nodes[next].clone(),
                        depth: distance + 1,
                        status,
                        view: "syntax_snapshot",
                    });
                    queue.push_back((next.clone(), distance + 1, status));
                }
                edge_seen.insert(index);
                result.edges.push(edge.clone());
            }
        }
        Ok(result)
    }
}
