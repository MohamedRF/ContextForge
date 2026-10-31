#![allow(clippy::unwrap_used)]
use contextforge_service::{ContextForgeService, core::Repository, index, registry::Registry};
use std::fs;

#[test]
fn registration_persists_is_unique_and_removal_preserves_sources_and_index() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source-λ");
    fs::create_dir(&source).unwrap();
    fs::write(source.join("auth.rs"), "pub fn authenticate() {}\n").unwrap();
    let registry_path = temp.path().join("user-data");
    let registry = Registry::open(&registry_path).unwrap();
    let id = registry.add(&source).unwrap();
    assert_eq!(registry.add(&source).unwrap(), id);
    let repo = registry.repository(id).unwrap();
    repo.init().unwrap();
    let repo = registry.repository(id).unwrap();
    index::index(&repo).unwrap();
    assert!(
        index::search::search(&repo, "authenticate", 20, 0)
            .unwrap()
            .results
            .iter()
            .any(|hit| hit.symbol.as_deref() == Some("authenticate"))
    );
    let package = ContextForgeService::pack_context(&repo, "authenticate", 1000, false).unwrap();
    assert!(!package.results.is_empty());
    assert!(ContextForgeService::load_graph(&repo).is_ok());
    registry.set_theme("light").unwrap();
    assert!(registry.set_theme("remote").is_err());
    let reopened = Registry::open(&registry_path).unwrap();
    assert_eq!(reopened.list().unwrap().len(), 1);
    assert_eq!(reopened.theme().unwrap(), "light");
    reopened.remove(id).unwrap();
    assert!(reopened.list().unwrap().is_empty());
    assert_eq!(
        fs::read_to_string(source.join("auth.rs")).unwrap(),
        "pub fn authenticate() {}\n"
    );
    assert_eq!(index::status(&repo).unwrap().files, 1);
    assert!(reopened.repository(id).is_err());
}

#[test]
fn cancellation_rolls_back_source_and_derived_index_together() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("a.rs"), "fn original() {}\n").unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    repo.init().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    index::index(&repo).unwrap();
    fs::write(temp.path().join("a.rs"), "fn replacement() {}\n").unwrap();
    fs::write(temp.path().join("b.rs"), "fn added() {}\n").unwrap();
    let result = index::index_controlled(&repo, false, &mut |phase, position, _| {
        phase != "writing" || position == 0
    });
    assert!(matches!(result, Err(index::Error::Cancelled)));
    assert_eq!(index::status(&repo).unwrap().files, 1);
    assert!(
        !index::search::search(&repo, "original", 10, 0)
            .unwrap()
            .results
            .is_empty()
    );
    assert!(
        index::search::search(&repo, "replacement", 10, 0)
            .unwrap()
            .results
            .is_empty()
    );
    index::index(&repo).unwrap();
    assert_eq!(index::status(&repo).unwrap().files, 2);
}

#[test]
fn registry_handles_missing_roots_and_rejects_future_schema() {
    let temp = tempfile::tempdir().unwrap();
    let registry = Registry::open(&temp.path().join("state")).unwrap();
    assert!(registry.add(&temp.path().join("missing")).is_err());
    let root = temp.path().join("root");
    fs::create_dir(&root).unwrap();
    let id = registry.add(&root).unwrap();
    fs::rename(&root, temp.path().join("moved")).unwrap();
    assert!(!registry.list().unwrap()[0].available);
    assert!(registry.repository(id).is_err());
    let connection = rusqlite::Connection::open(temp.path().join("state/projects.sqlite")).unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    assert!(Registry::open(&temp.path().join("state")).is_err());
}

#[test]
fn simultaneous_registrations_are_unique_across_connections() {
    let temp = tempfile::tempdir().unwrap();
    let state = temp.path().join("state");
    let root = temp.path().join("root");
    fs::create_dir(&root).unwrap();
    Registry::open(&state).unwrap();
    std::thread::scope(|scope| {
        for _ in 0..4 {
            scope.spawn(|| {
                Registry::open(&state).unwrap().add(&root).unwrap();
            });
        }
    });
    assert_eq!(Registry::open(&state).unwrap().list().unwrap().len(), 1);
}
