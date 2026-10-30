#![allow(clippy::unwrap_used, clippy::expect_used)]
use contextforge_graph::{Direction, EdgeKind, Graph, Status};
use contextforge_parser::parse_source;

fn graph(files: &[(&str, &str)]) -> Graph {
    let outlines: Vec<_> = files
        .iter()
        .map(|(path, source)| parse_source("fixture", path, source, 1000).unwrap())
        .collect();
    Graph::from_outlines(&outlines).unwrap()
}

#[test]
fn known_cycle_has_exact_candidate_edges_and_bounded_dependents() {
    let graph = graph(&[
        ("core.rs", "fn central() { caller(); }"),
        ("caller.rs", "fn caller() { central(); }"),
        ("test.rs", "fn test_flow() { caller(); }"),
    ]);
    let calls: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.kind == EdgeKind::Call)
        .collect();
    assert_eq!(calls.len(), 3);
    for edge in &calls {
        assert_eq!(edge.status, Status::Heuristic);
        assert!(edge.to.is_some());
        assert_eq!(edge.span.as_ref().unwrap().start_line, 1);
    }
    let result = graph
        .traverse("central", Direction::Incoming, 4, 20, 20)
        .unwrap();
    assert_eq!(result.nodes.len(), 3);
    assert!(
        result
            .nodes
            .iter()
            .any(|v| v.node.name == "test_flow" && v.depth == 2 && v.status == Status::Heuristic)
    );
    let small = graph
        .traverse("central", Direction::Incoming, 4, 1, 20)
        .unwrap();
    assert_eq!(small.nodes.len(), 1);
    assert!(small.truncated);
    let depth = graph
        .traverse("central", Direction::Incoming, 1, 20, 20)
        .unwrap();
    assert_eq!(depth.nodes.len(), 2);
    assert!(depth.truncated);
    for (depth, nodes, edges) in [(9, 1, 1), (1, 0, 1), (1, 501, 1), (1, 1, 0), (1, 1, 1001)] {
        assert!(
            graph
                .traverse("central", Direction::Incoming, depth, nodes, edges)
                .is_err()
        );
    }
    assert!(
        graph
            .traverse("absent", Direction::Incoming, 2, 20, 20)
            .unwrap()
            .nodes
            .is_empty()
    );
    println!(
        "Known three-call cycle fixture: candidate precision=3/3, recall=3/3; all target edges heuristic, zero compiler-resolved calls."
    );
}

#[test]
fn ambiguous_dynamic_and_external_calls_remain_unknown() {
    let graph = graph(&[
        (
            "a.py",
            "def duplicate(): pass\ndef caller():\n    duplicate()\n    obj.duplicate()\n    external()\n",
        ),
        ("b.py", "def duplicate(): pass\n"),
        ("other.rs", "fn external() {}"),
    ]);
    let calls: Vec<_> = graph
        .edges
        .iter()
        .filter(|edge| edge.kind == EdgeKind::Call)
        .collect();
    assert_eq!(calls.len(), 3);
    assert!(
        calls
            .iter()
            .all(|edge| edge.to.is_none() && edge.status == Status::Unknown)
    );
    let references = graph.find_references("duplicate", 20).unwrap();
    assert_eq!(references.edges.len(), 1);
    assert_eq!(references.edges[0].text, "duplicate");
    assert_eq!(references.edges[0].status, Status::Unknown);
}

#[test]
fn qualified_methods_imports_and_containment_have_distinct_certainty() {
    let graph = graph(&[
        (
            "src/view.ts",
            "import { Thing } from './model'; export class View { run() { Model.build(); obj.build(); } }",
        ),
        ("src/model.ts", "export class Model { static build() {} }"),
    ]);
    let calls: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.kind == EdgeKind::Call)
        .collect();
    assert_eq!(calls.len(), 2);
    assert!(
        calls
            .iter()
            .any(|edge| edge.text == "Model.build" && edge.status == Status::Heuristic)
    );
    assert!(
        calls
            .iter()
            .any(|edge| edge.text == "obj.build" && edge.status == Status::Unknown)
    );
    let import = graph
        .edges
        .iter()
        .find(|e| e.kind == EdgeKind::Import)
        .unwrap();
    assert_eq!(import.to.as_deref(), Some("file:src/model.ts"));
    assert_eq!(import.status, Status::Heuristic);
    assert!(
        graph
            .edges
            .iter()
            .filter(|e| e.kind == EdgeKind::Contains)
            .all(|e| e.status == Status::Resolved)
    );
    assert_eq!(graph.find_symbol("Model::build").unwrap().len(), 1);
    let nodes = graph
        .traverse("src/view.ts", Direction::Outgoing, 4, 30, 30)
        .unwrap();
    assert!(nodes.nodes.iter().any(|v| v.node.path == "src/model.ts"));
    let limited = graph
        .traverse("src/view.ts", Direction::Outgoing, 4, 30, 1)
        .unwrap();
    assert_eq!(limited.edges.len(), 1);
    assert!(limited.truncated);
}

#[test]
fn stable_repeated_queries_and_partial_syntax_are_explicit() {
    let graph = graph(&[("a.rs", "fn a() { missing(); } fn broken(")]);
    assert!(graph.partial_syntax);
    let first = graph.traverse("a", Direction::Outgoing, 2, 20, 20).unwrap();
    let second = graph.traverse("a", Direction::Outgoing, 2, 20, 20).unwrap();
    assert_eq!(
        serde_json::to_value(first).unwrap(),
        serde_json::to_value(second).unwrap()
    );
    assert!(graph.find_symbol("").is_err());
    assert!(Graph::from_json(["{}"]).is_err());
}
