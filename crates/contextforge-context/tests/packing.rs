#![allow(clippy::unwrap_used, clippy::expect_used)]
use contextforge_context::{ByteEstimator, TokenEstimator, pack, pack_with};
use contextforge_core::Repository;
use std::{collections::BTreeSet, fs};

fn fixture() -> (tempfile::TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    fs::write(
        temp.path().join("payment.rs"),
        "pub fn validate_payment() -> bool { true }\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("checkout.rs"),
        "fn checkout() -> bool { validate_payment() }\n",
    )
    .unwrap();
    fs::write(temp.path().join("unrelated.rs"), "fn unrelated() {}\n").unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    repo.init().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    contextforge_index::index(&repo).unwrap();
    (temp, repo)
}

#[test]
fn retrieval_graph_citations_diversity_and_repeatability() {
    let (temp, repo) = fixture();
    let result = pack(
        &repo,
        "explain validate_payment flow",
        4000,
        &BTreeSet::new(),
    )
    .unwrap();
    let paths: BTreeSet<_> = result.results.iter().map(|s| s.path.as_str()).collect();
    // Ground truth for this synthetic task: implementation and caller, 2/2 recall.
    assert_eq!(paths, BTreeSet::from(["payment.rs", "checkout.rs"]));
    assert!(
        result
            .results
            .iter()
            .any(|s| s.relation.is_some() && s.confidence == "heuristic")
    );
    for snippet in &result.results {
        let source = fs::read_to_string(temp.path().join(&snippet.path)).unwrap();
        assert_eq!(
            snippet.snippet,
            source[snippet.span.start_byte..snippet.span.end_byte]
        );
        assert!(!snippet.fingerprint.is_empty());
    }
    for (i, a) in result.results.iter().enumerate() {
        assert!(!result.results[i + 1..].iter().any(|b| a.path == b.path
            && a.span.start_byte < b.span.end_byte
            && b.span.start_byte < a.span.end_byte));
    }
    let json = serde_json::to_string(&result).unwrap();
    assert_eq!(result.estimated_tokens, ByteEstimator.estimate(&json));
    assert_eq!(
        json,
        serde_json::to_string(
            &pack(
                &repo,
                "explain validate_payment flow",
                4000,
                &BTreeSet::new()
            )
            .unwrap()
        )
        .unwrap()
    );
    assert!(!paths.contains("unrelated.rs"));
}

#[test]
fn all_budgets_include_json_escapes_unicode_and_metadata() {
    let (temp, repo) = fixture();
    fs::write(
        temp.path().join("unicode.rs"),
        "// 雪\nfn unicode() { let s = \"雪\\\\\\\"\"; }\n",
    )
    .unwrap();
    contextforge_index::index(&repo).unwrap();
    for budget in [256, 257, 300, 512, 1000, 4000, 32000] {
        let result = pack(&repo, "unicode", budget, &BTreeSet::new()).unwrap();
        let json = serde_json::to_string(&result).unwrap();
        assert_eq!(result.estimated_tokens, ByteEstimator.estimate(&json));
        assert!(result.estimated_tokens <= budget);
    }
    for budget in 256..600 {
        let result = pack(&repo, "validate_payment", budget, &BTreeSet::new()).unwrap();
        assert!(ByteEstimator.estimate(&serde_json::to_string(&result).unwrap()) <= budget);
    }
}

struct OneByte;
impl TokenEstimator for OneByte {
    fn name(&self) -> &'static str {
        "test_one_byte"
    }
    fn estimate(&self, text: &str) -> usize {
        text.len()
    }
}

#[test]
fn estimator_interface_and_invalid_or_empty_queries() {
    let (_temp, repo) = fixture();
    let result = pack_with(&repo, "validate_payment", 4000, &BTreeSet::new(), &OneByte).unwrap();
    assert_eq!(
        result.estimated_tokens,
        serde_json::to_string(&result).unwrap().len()
    );
    for query in ["", "   ", "bad\ninput", &"x".repeat(257)] {
        assert!(pack(&repo, query, 4000, &BTreeSet::new()).is_err());
    }
    for budget in [0, 255, 32001, usize::MAX] {
        assert!(pack(&repo, "validate_payment", budget, &BTreeSet::new()).is_err());
    }
    let empty = pack(&repo, "nothingshouldmatch", 256, &BTreeSet::new()).unwrap();
    assert!(empty.results.is_empty());
    assert!(!empty.truncated);
}

#[test]
fn large_boundaries_are_omitted_and_read_only_snapshot_is_explicit() {
    let (temp, repo) = fixture();
    fs::write(
        temp.path().join("huge.rs"),
        format!("fn huge() {{ /*{}*/ }}", "x".repeat(20_000)),
    )
    .unwrap();
    contextforge_index::index(&repo).unwrap();
    let result = pack(&repo, "huge", 32000, &BTreeSet::new()).unwrap();
    assert!(result.truncated);
    assert!(result.results.is_empty());
    fs::write(temp.path().join("payment.rs"), "fn replaced() {}\n").unwrap();
    let result = pack(&repo, "validate_payment", 4000, &BTreeSet::new()).unwrap();
    assert_eq!(result.freshness, "snapshot_not_checked");
    assert!(
        result
            .results
            .iter()
            .any(|s| s.snippet.contains("pub fn validate_payment"))
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("payment.rs")).unwrap(),
        "fn replaced() {}\n"
    );
}

#[test]
fn optional_git_path_signal_breaks_retrieval_ties() {
    let (temp, repo) = fixture();
    fs::write(temp.path().join("a.rs"), "fn same() {}\n").unwrap();
    fs::write(temp.path().join("z.rs"), "fn same() {}\n").unwrap();
    contextforge_index::index(&repo).unwrap();
    let result = pack(&repo, "same", 4000, &BTreeSet::from(["z.rs".into()])).unwrap();
    assert_eq!(result.results[0].path, "z.rs");
    assert!(result.results[0].git_proximity);
}
