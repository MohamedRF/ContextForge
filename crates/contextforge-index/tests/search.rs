#![allow(clippy::unwrap_used, clippy::expect_used)]
use contextforge_core::Repository;
use contextforge_index::{index, search::search};
use rusqlite::Connection;
use std::{fs, path::PathBuf};

fn setup() -> (tempfile::TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    repo.init().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    (temp, repo)
}

fn database(repo: &Repository) -> PathBuf {
    fs::read_dir(repo.root().join(".contextforge/index"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("index.sqlite")
}

#[test]
fn literal_ranked_cited_search_pagination_and_persistent_reuse() {
    let (temp, repo) = setup();
    fs::write(
        temp.path().join("garden.rs"),
        "pub mod garden {\n pub fn water() {}\n pub fn watering() {}\n}\n",
    )
    .unwrap();
    fs::write(temp.path().join("notes.py"), "# water\ndef other(): pass\n").unwrap();
    let report = index(&repo).unwrap();
    assert_eq!(report.schema_version, 2);
    assert_eq!(report.parsed_files, 2);
    assert_eq!(index(&repo).unwrap().parsed_files, 0);
    let result = search(&repo, "water", 20, 0).unwrap();
    assert_eq!(result.results[0].method, "exact_symbol");
    assert_eq!(result.results[0].symbol.as_deref(), Some("water"));
    assert_eq!(result.results[0].span.start_line, 2);
    assert!(result.results[0].snippet.contains("fn water()"));
    assert!(
        result
            .results
            .iter()
            .any(|hit| hit.method == "prefix_symbol")
    );
    assert!(
        result
            .results
            .iter()
            .any(|hit| hit.method == "fts5" && hit.path == "notes.py")
    );
    assert_eq!(result.freshness, "snapshot_not_checked");
    assert_eq!(
        search(&repo, "garden::water", 20, 0).unwrap().results[0]
            .symbol
            .as_deref(),
        Some("water")
    );
    assert_eq!(
        search(&repo, "garden.rs", 20, 0).unwrap().results[0].method,
        "exact_filename"
    );
    let first = search(&repo, "water", 1, 0).unwrap();
    assert_eq!(first.next_offset, Some(1));
    let second = search(&repo, "water", 1, 1).unwrap();
    assert_eq!(second.results[0].symbol, result.results[1].symbol);
    assert_eq!(second.results[0].method, result.results[1].method);
    assert!(search(&repo, "water", 1, 1000).unwrap().results.is_empty());
    let reopened = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert_eq!(
        serde_json::to_value(&result).unwrap(),
        serde_json::to_value(search(&reopened, "water", 20, 0).unwrap()).unwrap()
    );
    for hit in &result.results {
        let source = fs::read_to_string(temp.path().join(&hit.path)).unwrap();
        assert!(source[hit.span.start_byte..hit.span.end_byte].starts_with(&hit.snippet));
    }
}

#[test]
fn invalid_queries_literal_operators_unicode_and_no_hidden_indexing() {
    let (temp, repo) = setup();
    assert!(search(&repo, "water", 20, 0).is_err());
    assert!(!temp.path().join(".contextforge/index").exists());
    fs::write(
        temp.path().join("源.py"),
        "# 🌱 café\ndef café(): pass\n# OR NOT \"quoted\"\n",
    )
    .unwrap();
    index(&repo).unwrap();
    let path = database(&repo);
    let before = fs::read(&path).unwrap();
    for query in ["", " ", "x\ny"] {
        assert!(search(&repo, query, 20, 0).is_err());
    }
    assert!(search(&repo, &"a".repeat(257), 20, 0).is_err());
    assert!(search(&repo, "x", 0, 0).is_err());
    assert!(search(&repo, "x", 101, 0).is_err());
    assert!(search(&repo, "x", 20, 10001).is_err());
    for query in [
        "OR NOT",
        "\"quoted\"",
        "x' OR 1=1 --",
        "*",
        "\"",
        ":",
        "(",
        "%",
        "café",
        "源.py",
    ] {
        let results = search(&repo, query, 20, 0).unwrap();
        assert!(results.results.iter().all(|hit| hit.path == "源.py"));
    }
    assert_eq!(
        search(&repo, "café", 20, 0).unwrap().results[0]
            .symbol
            .as_deref(),
        Some("café")
    );
    assert!(
        search(&repo, "absentWord", 20, 0)
            .unwrap()
            .results
            .is_empty()
    );
    assert_eq!(fs::read(path).unwrap(), before);
}

#[test]
fn incremental_symbols_fts_and_outlines_update_or_rollback_together() {
    let (temp, repo) = setup();
    fs::write(temp.path().join("a.rs"), "fn old() {}\n").unwrap();
    index(&repo).unwrap();
    let connection = Connection::open(database(&repo)).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_outline BEFORE DELETE ON outlines BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert_eq!(index(&repo).unwrap().unchanged, 1); // unchanged files aren't reparsed/replaced
    fs::write(temp.path().join("a.rs"), "fn new() {}\n").unwrap();
    assert!(index(&repo).is_err());
    assert_eq!(
        search(&repo, "old", 20, 0).unwrap().results[0]
            .symbol
            .as_deref(),
        Some("old")
    );
    assert!(search(&repo, "new", 20, 0).unwrap().results.is_empty());
    connection
        .execute_batch("DROP TRIGGER reject_outline;")
        .unwrap();
    assert_eq!(index(&repo).unwrap().modified, 1);
    assert!(search(&repo, "old", 20, 0).unwrap().results.is_empty());
    assert_eq!(
        search(&repo, "new", 20, 0).unwrap().results[0]
            .symbol
            .as_deref(),
        Some("new")
    );
    fs::rename(temp.path().join("a.rs"), temp.path().join("b.rs")).unwrap();
    index(&repo).unwrap();
    assert!(
        search(&repo, "new", 20, 0)
            .unwrap()
            .results
            .iter()
            .all(|hit| hit.path == "b.rs")
    );
    fs::remove_file(temp.path().join("b.rs")).unwrap();
    index(&repo).unwrap();
    assert!(search(&repo, "new", 20, 0).unwrap().results.is_empty());
    let count: u32 = connection
        .query_row("SELECT count(path) FROM outlines", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 0);
}

#[test]
fn schema_one_upgrade_backfills_unchanged_sources_and_rolls_back_on_failure() {
    let (temp, repo) = setup();
    fs::write(temp.path().join("a.rs"), "fn old() {}\n").unwrap();
    index(&repo).unwrap();
    let connection = Connection::open(database(&repo)).unwrap();
    connection.execute_batch("DROP TABLE symbols; DROP TABLE outlines; DROP TABLE source_fts; PRAGMA user_version=1;").unwrap();
    assert!(search(&repo, "old", 20, 0).is_err());
    fs::write(temp.path().join("z.rs"), "fn added() {}\n").unwrap();
    connection.execute_batch("CREATE TRIGGER reject_z BEFORE INSERT ON files WHEN NEW.path='z.rs' BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
    assert!(index(&repo).is_err());
    let version: u32 = connection
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap();
    assert_eq!(version, 1);
    let count: u32 = connection
        .query_row(
            "SELECT count(name) FROM sqlite_schema WHERE name='outlines'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    connection.execute_batch("DROP TRIGGER reject_z;").unwrap();
    let report = index(&repo).unwrap();
    assert_eq!(
        (report.schema_version, report.unchanged, report.added),
        (2, 1, 1)
    );
    assert_eq!(
        search(&repo, "old", 20, 0).unwrap().results[0]
            .symbol
            .as_deref(),
        Some("old")
    );
}

#[test]
fn structural_top_hit_beats_lexical_path_order_on_reproducible_corpus() {
    let (temp, repo) = setup();
    let corpus = [
        ("a_decoy.py", "# water is mentioned here\n"),
        ("z_garden.rs", "pub fn water() {}\n"),
    ];
    for (path, content) in corpus {
        fs::write(temp.path().join(path), content).unwrap();
    }
    index(&repo).unwrap();
    let keyword_paths: Vec<_> = corpus
        .iter()
        .filter(|(_, text)| text.contains("water"))
        .map(|(path, _)| *path)
        .collect();
    assert_eq!(keyword_paths, vec!["a_decoy.py", "z_garden.rs"]);
    let result = search(&repo, "water", 1, 0).unwrap();
    assert_eq!(result.results[0].path, "z_garden.rs");
    assert_eq!(result.results[0].method, "exact_symbol");
    println!(
        "two-file corpus, water definition ground truth z_garden.rs: keyword path-order top1 correct=false, structural top1 correct=true; keyword candidate precision=1/2; definition top1 precision=1/1; both find the relevant file. No general quality/speed claim."
    );
}

#[test]
fn fts_normalized_passages_keep_exact_source_spans_and_clipped_snippets() {
    let (temp, repo) = setup();
    let source = format!(
        "{}\n# café {}\n",
        "# unrelated\n".repeat(50),
        "word ".repeat(600)
    );
    fs::write(temp.path().join("unicode.py"), &source).unwrap();
    index(&repo).unwrap();
    let result = search(&repo, "cafe", 20, 0).unwrap();
    let hit = &result.results[0];
    assert_eq!(hit.method, "fts5");
    assert!(hit.snippet.contains("café"));
    assert!(hit.span.start_line > 1);
    assert!(source[hit.span.start_byte..hit.span.end_byte].starts_with(&hit.snippet));
    assert!(hit.snippet.chars().count() <= 512);
}

#[test]
fn partial_syntax_is_visible_and_full_source_remains_searchable() {
    let (temp, repo) = setup();
    let source = format!(
        "{}\n// tail_evidence\nfn broken(",
        "fn repeated() {}\n".repeat(1001)
    );
    fs::write(temp.path().join("partial.rs"), source).unwrap();
    let report = index(&repo).unwrap();
    assert_eq!((report.parsed_files, report.partial_files), (1, 1));
    let report = index(&repo).unwrap();
    assert_eq!((report.parsed_files, report.partial_files), (0, 1));
    assert!(
        !search(&repo, "tail_evidence", 20, 0)
            .unwrap()
            .results
            .is_empty()
    );
}
