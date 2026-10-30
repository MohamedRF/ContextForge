#![allow(clippy::unwrap_used, clippy::expect_used)]
use super::*;

fn fixture() -> (tempfile::TempDir, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    repo.init().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    index(&repo).unwrap();
    (temp, repo)
}

#[test]
fn sqlite_rejects_oversized_rows_statements_attachments_and_excessive_vm_work() {
    let (_temp, repo) = fixture();
    let conn = connection(&repo, false).unwrap().unwrap();
    assert!(
        conn.prepare(&format!("SELECT 1 /*{}*/", "x".repeat(65_536)))
            .is_err()
    );
    assert!(
        conn.query_row::<Vec<u8>, _, _>("SELECT zeroblob(33554433)", [], |row| row.get(0))
            .is_err()
    );
    assert!(
        conn.execute("ATTACH DATABASE ':memory:' AS other", [])
            .is_err()
    );
    let error = conn.query_row::<i64, _, _>("WITH RECURSIVE counter(x) AS (VALUES(1) UNION ALL SELECT x+1 FROM counter WHERE x<1000000000) SELECT sum(x) FROM counter", [], |row| row.get(0)).unwrap_err();
    assert_eq!(
        error.sqlite_error_code(),
        Some(rusqlite::ErrorCode::OperationInterrupted)
    );
}

#[test]
fn oversized_database_is_rejected_before_sqlite_reads_it() {
    let (_temp, repo) = fixture();
    let path = cache_directory(&repo).join("index.sqlite");
    let file = fs::OpenOptions::new().write(true).open(&path).unwrap();
    file.set_len(512 * 1024 * 1024 + 1).unwrap();
    drop(file);
    assert!(matches!(status(&repo), Err(Error::ResourceLimit)));
}

#[cfg(unix)]
#[test]
fn cache_directory_is_private_and_hardlinked_database_is_rejected() {
    use std::os::unix::fs::PermissionsExt;
    let (temp, repo) = fixture();
    let directory = cache_directory(&repo);
    assert_eq!(
        fs::metadata(&directory).unwrap().permissions().mode() & 0o777,
        0o700
    );
    fs::hard_link(
        directory.join("index.sqlite"),
        temp.path().join("linked.sqlite"),
    )
    .unwrap();
    assert!(status(&repo).is_err());
}
