#![allow(clippy::unwrap_used, clippy::expect_used)]
use contextforge_graph::{Direction, Graph};
use contextforge_parser::parse_source;
use std::collections::BTreeSet;

#[test]
fn generated_rings_terminate_deterministically_within_every_traversal_bound() {
    for size in 1..=16 {
        let outlines: Vec<_> = (0..size)
            .map(|i| {
                parse_source(
                    "ring",
                    &format!("file_{i}.rs"),
                    &format!("fn node_{i}() {{ node_{}(); }}", (i + 1) % size),
                    10,
                )
                .unwrap()
            })
            .collect();
        let graph = Graph::from_outlines(&outlines).unwrap();
        for direction in [Direction::Incoming, Direction::Outgoing] {
            for depth in [0, 1, 3, 8] {
                for node_limit in [1, 4, 20] {
                    let result = graph
                        .traverse("node_0", direction, depth, node_limit, 5)
                        .unwrap();
                    assert!(result.nodes.len() <= node_limit);
                    assert!(result.edges.len() <= 5);
                    assert!(result.nodes.iter().all(|visit| visit.depth <= depth));
                    let ids: BTreeSet<_> =
                        result.nodes.iter().map(|visit| &visit.node.id).collect();
                    assert_eq!(ids.len(), result.nodes.len());
                    assert_eq!(
                        serde_json::to_value(&result).unwrap(),
                        serde_json::to_value(
                            graph
                                .traverse("node_0", direction, depth, node_limit, 5)
                                .unwrap()
                        )
                        .unwrap()
                    );
                }
            }
        }
    }
}
