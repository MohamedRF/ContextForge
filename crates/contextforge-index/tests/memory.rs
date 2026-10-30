#![allow(clippy::unwrap_used, clippy::expect_used)]
use contextforge_core::Repository;
use contextforge_index::{index, memory, search};
use std::fs;

#[test]
fn persistent_knowledge_validates_edits_removals_additions_and_rebuilds() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("lib.rs"), "fn first() {}\n").unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert_eq!(memory::status(&repo).unwrap().state, "missing");
    repo.init().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert_eq!(memory::clear(&repo).unwrap(), 0);
    assert!(!temp.path().join(".contextforge/index").exists());
    index(&repo).unwrap();
    let reopened = Repository::open(Some(temp.path()), temp.path()).unwrap();
    let status = memory::status(&reopened).unwrap();
    assert_eq!(status.freshness, "validated_at_read");
    assert_eq!(status.files[0].symbols, Some(1));
    fs::write(temp.path().join("lib.rs"), "fn other() {}\n").unwrap();
    fs::write(temp.path().join("added.rs"), "fn added() {}\n").unwrap();
    let stale = memory::status(&reopened).unwrap();
    assert_eq!(stale.freshness, "stale");
    assert_eq!(stale.added, 1);
    assert_eq!(stale.changed_or_removed, 1);
    assert_eq!(stale.files[0].symbols, None);
    assert!(!stale.files[0].valid);
    let report = memory::rebuild(&reopened).unwrap();
    assert_eq!(report.parsed_files, 2);
    assert!(
        search::search(&reopened, "first", 10, 0)
            .unwrap()
            .results
            .is_empty()
    );
    assert_eq!(
        memory::status(&reopened).unwrap().freshness,
        "validated_at_read"
    );
    // Force reparse even unchanged source to rebuild structural knowledge.
    assert_eq!(memory::rebuild(&reopened).unwrap().parsed_files, 2);
    fs::remove_file(temp.path().join("lib.rs")).unwrap();
    assert_eq!(memory::status(&reopened).unwrap().changed_or_removed, 1);
    memory::rebuild(&reopened).unwrap();
    assert_eq!(memory::status(&reopened).unwrap().files.len(), 1);
    assert_eq!(memory::clear(&reopened).unwrap(), 1);
    assert!(
        search::search(&reopened, "added", 10, 0)
            .unwrap()
            .results
            .is_empty()
    );
    assert_eq!(
        fs::read_to_string(temp.path().join("added.rs")).unwrap(),
        "fn added() {}\n"
    );
    assert!(temp.path().join(".contextforge/config.toml").exists());
    assert_eq!(memory::status(&reopened).unwrap().added, 1);
}

#[test]
fn clear_and_rebuild_are_atomic_and_preserve_unknown_or_corrupt_databases() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("lib.rs"), "fn known() {}\n").unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    repo.init().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    index(&repo).unwrap();
    let database = fs::read_dir(temp.path().join(".contextforge/index"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path()
        .join("index.sqlite");
    let conn = rusqlite::Connection::open(&database).unwrap();
    conn.execute_batch("CREATE TRIGGER fail_clear BEFORE DELETE ON files BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(memory::clear(&repo).is_err());
    assert_eq!(
        search::search(&repo, "known", 10, 0).unwrap().results[0]
            .symbol
            .as_deref(),
        Some("known")
    );
    conn.execute_batch("DROP TRIGGER fail_clear; CREATE TRIGGER fail_rebuild BEFORE INSERT ON symbols BEGIN SELECT RAISE(ABORT,'injected failure'); END;").unwrap();
    assert!(memory::rebuild(&repo).is_err());
    assert!(
        !search::search(&repo, "known", 10, 0)
            .unwrap()
            .results
            .is_empty()
    );
    conn.execute_batch("PRAGMA user_version=999;").unwrap();
    drop(conn);
    let before = fs::read(&database).unwrap();
    assert!(memory::clear(&repo).is_err());
    assert!(memory::rebuild(&repo).is_err());
    assert_eq!(fs::read(&database).unwrap(), before);
    fs::write(&database, b"corrupt cache preserve me").unwrap();
    assert!(memory::clear(&repo).is_err());
    assert_eq!(fs::read(&database).unwrap(), b"corrupt cache preserve me");
}
