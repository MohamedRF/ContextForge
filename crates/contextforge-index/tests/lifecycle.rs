#![allow(clippy::unwrap_used, clippy::expect_used)]
use contextforge_core::Repository;
use contextforge_index::{index, status};
use rusqlite::Connection;
use std::{
    fs,
    path::{Path, PathBuf},
    time::Instant,
};
use tempfile::TempDir;

fn setup() -> (TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let repo = Repository::open(Some(root), root).unwrap();
    repo.init().unwrap();
    let repo = Repository::open(Some(root), root).unwrap();
    (temp, repo)
}

fn database(root: &Path) -> PathBuf {
    let base = root.join(".contextforge/index");
    fs::read_dir(base)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("index.sqlite")
}

#[test]
fn incremental_add_edit_delete_rename_and_persistent_status() {
    let (temp, repo) = setup();
    fs::write(temp.path().join("a.rs"), "fn one() {}\n").unwrap();
    fs::write(temp.path().join("b.py"), "def two(): pass\n").unwrap();
    let report = index(&repo).unwrap();
    assert_eq!(
        (
            report.added,
            report.modified,
            report.unchanged,
            report.deleted
        ),
        (2, 0, 0, 0)
    );
    let report = index(&repo).unwrap();
    assert_eq!(
        (
            report.added,
            report.modified,
            report.unchanged,
            report.deleted
        ),
        (0, 0, 2, 0)
    );
    fs::write(temp.path().join("a.rs"), "fn new() {}\n").unwrap();
    let report = index(&repo).unwrap();
    assert_eq!((report.modified, report.unchanged), (1, 1));
    fs::rename(temp.path().join("b.py"), temp.path().join("renamed.py")).unwrap();
    let report = index(&repo).unwrap();
    assert_eq!((report.added, report.deleted, report.unchanged), (1, 1, 1));
    fs::remove_file(temp.path().join("a.rs")).unwrap();
    assert_eq!(index(&repo).unwrap().deleted, 1);
    let reopened = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert_eq!(status(&reopened).unwrap().files, 1);
    assert_eq!(status(&reopened).unwrap().freshness, "not_checked");
    let connection = Connection::open(database(temp.path())).unwrap();
    let path: String = connection
        .query_row("SELECT path FROM files", [], |row| row.get(0))
        .unwrap();
    assert_eq!(path, "renamed.py");
}

#[test]
fn failed_write_rolls_back_the_entire_update() {
    let (temp, repo) = setup();
    fs::write(temp.path().join("a.rs"), "fn old() {}\n").unwrap();
    index(&repo).unwrap();
    let connection = Connection::open(database(temp.path())).unwrap();
    connection.execute_batch("CREATE TRIGGER reject_added BEFORE INSERT ON files WHEN NEW.path = 'z.rs' BEGIN SELECT RAISE(ABORT, 'injected failure'); END;").unwrap();
    fs::write(temp.path().join("a.rs"), "fn new() {}\n").unwrap();
    fs::write(temp.path().join("z.rs"), "fn z() {}\n").unwrap();
    assert!(index(&repo).is_err());
    let content: String = connection
        .query_row("SELECT content FROM files WHERE path = 'a.rs'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(content, "fn old() {}\n");
    assert_eq!(status(&repo).unwrap().files, 1);
}

#[test]
fn future_schema_and_corruption_are_not_overwritten() {
    let (temp, repo) = setup();
    index(&repo).unwrap();
    let path = database(temp.path());
    let connection = Connection::open(&path).unwrap();
    connection
        .execute_batch("PRAGMA user_version = 999;")
        .unwrap();
    drop(connection);
    let old = fs::read(&path).unwrap();
    assert!(index(&repo).is_err());
    assert!(status(&repo).is_err());
    assert_eq!(fs::read(&path).unwrap(), old);
    fs::write(&path, "not sqlite").unwrap();
    assert!(index(&repo).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"not sqlite");
}

#[test]
fn truncation_never_deletes_existing_rows() {
    let (temp, repo) = setup();
    fs::write(temp.path().join("initial.rs"), "fn initial() {}\n").unwrap();
    index(&repo).unwrap();
    for number in 0..1000 {
        fs::write(
            temp.path().join(format!("file{number}.rs")),
            "fn example() {}\n",
        )
        .unwrap();
    }
    assert!(index(&repo).is_err());
    assert_eq!(status(&repo).unwrap().files, 1);
}

#[test]
fn external_cache_isolates_repositories_and_status_is_read_only() {
    let cache = tempfile::tempdir().unwrap();
    let (first, _) = setup();
    let (second, _) = setup();
    for temp in [&first, &second] {
        // macOS temporary paths can include /var -> /private/var. Select the
        // canonical authorized cache, retaining the no-linked-metadata policy.
        let canonical_cache = fs::canonicalize(cache.path()).unwrap();
        let path_literal = canonical_cache.to_str().unwrap();
        fs::write(
            temp.path().join(".contextforge/config.toml"),
            format!("index_root = '{path_literal}'\n"),
        )
        .unwrap();
        fs::write(temp.path().join("a.rs"), "fn a() {}\n").unwrap();
        let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
        assert_eq!(status(&repo).unwrap().state, "missing");
        index(&repo).unwrap();
        assert_eq!(status(&repo).unwrap().files, 1);
    }
    assert_eq!(fs::read_dir(cache.path()).unwrap().count(), 2);
    let fresh = tempfile::tempdir().unwrap();
    let repo = Repository::open(Some(fresh.path()), fresh.path()).unwrap();
    assert_eq!(status(&repo).unwrap().state, "missing");
    assert!(index(&repo).is_err());
    assert!(!fresh.path().join(".contextforge").exists());
}

#[test]
fn fixture_baseline_and_source_integrity() {
    let (temp, repo) = setup();
    for number in 0..100 {
        fs::write(
            temp.path().join(format!("file{number}.rs")),
            format!("pub fn example_{number}() {{}}\n"),
        )
        .unwrap();
    }
    let start = Instant::now();
    let cold = index(&repo).unwrap();
    let cold_elapsed = start.elapsed();
    let start = Instant::now();
    let warm = index(&repo).unwrap();
    let warm_elapsed = start.elapsed();
    assert_eq!((cold.added, warm.unchanged), (100, 100));
    assert_eq!(
        fs::read_to_string(temp.path().join("file0.rs")).unwrap(),
        "pub fn example_0() {}\n"
    );
    eprintln!(
        "fixture=100-generated-rust-files cold_ms={} unchanged_ms={} sqlite_bytes={}",
        cold_elapsed.as_millis(),
        warm_elapsed.as_millis(),
        fs::metadata(database(temp.path())).unwrap().len()
    );
}

#[cfg(unix)]
#[test]
fn linked_external_cache_is_rejected_without_writes() {
    let (temp, _) = setup();
    let outside = tempfile::tempdir().unwrap();
    let canonical_root = fs::canonicalize(temp.path()).unwrap();
    let linked_cache = canonical_root.join("linked-cache");
    std::os::unix::fs::symlink(outside.path(), &linked_cache).unwrap();
    fs::write(
        temp.path().join(".contextforge/config.toml"),
        format!("index_root = '{}'\n", linked_cache.to_str().unwrap()),
    )
    .unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert!(status(&repo).is_err());
    assert!(index(&repo).is_err());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn newly_ignored_or_binary_files_remove_stale_rows() {
    let (temp, repo) = setup();
    fs::write(temp.path().join("a.rs"), "fn a() {}\n").unwrap();
    fs::write(temp.path().join("b.rs"), "fn b() {}\n").unwrap();
    assert_eq!(index(&repo).unwrap().added, 2);
    fs::write(temp.path().join(".contextforgeignore"), "a.rs\n").unwrap();
    fs::write(temp.path().join("b.rs"), [0, 1, 2]).unwrap();
    assert_eq!(index(&repo).unwrap().deleted, 2);
    assert_eq!(status(&repo).unwrap().files, 0);
}

#[test]
fn concurrent_indexers_do_not_duplicate_records() {
    let (temp, repo) = setup();
    fs::write(temp.path().join("a.rs"), "fn a() {}\n").unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
    std::thread::scope(|scope| {
        let mut tasks = Vec::new();
        for _ in 0..2 {
            let repo = &repo;
            let barrier = barrier.clone();
            tasks.push(scope.spawn(move || {
                barrier.wait();
                index(repo).unwrap()
            }));
        }
        let reports: Vec<_> = tasks.into_iter().map(|task| task.join().unwrap()).collect();
        assert_eq!(reports.iter().map(|report| report.added).sum::<usize>(), 1);
        assert_eq!(
            reports.iter().map(|report| report.unchanged).sum::<usize>(),
            1
        );
    });
    assert_eq!(status(&repo).unwrap().files, 1);
}

#[test]
fn transaction_survives_process_exit() {
    const CHILD_DATABASE: &str = "CONTEXTFORGE_TEST_CRASH_DATABASE";
    if let Some(path) = std::env::var_os(CHILD_DATABASE) {
        let connection = Connection::open(path).unwrap();
        connection
            .execute_batch("PRAGMA cache_size=1; BEGIN IMMEDIATE;")
            .unwrap();
        connection
            .execute("UPDATE files SET content = ?1", ["x".repeat(1_048_576)])
            .unwrap();
        // Exit without Rust destructors/rollback to model an interrupted writer.
        std::process::exit(23);
    }
    let (temp, repo) = setup();
    fs::write(temp.path().join("a.rs"), "fn committed() {}\n").unwrap();
    index(&repo).unwrap();
    let path = database(temp.path());
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "transaction_survives_process_exit"])
        .env(CHILD_DATABASE, &path)
        .output()
        .unwrap();
    assert_eq!(child.status.code(), Some(23));
    assert_eq!(index(&repo).unwrap().unchanged, 1);
    let connection = Connection::open(path).unwrap();
    let content: String = connection
        .query_row("SELECT content FROM files WHERE path = 'a.rs'", [], |row| {
            row.get(0)
        })
        .unwrap();
    assert_eq!(content, "fn committed() {}\n");
}
