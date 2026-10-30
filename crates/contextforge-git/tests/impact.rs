#![allow(clippy::unwrap_used, clippy::expect_used)]
use contextforge_core::Repository;
use contextforge_git::{Mode, analyze};
use git2::{Repository as GitRepository, Signature};
use std::fs;

fn setup() -> (tempfile::TempDir, GitRepository, Repository) {
    let temp = tempfile::tempdir().unwrap();
    let git = GitRepository::init(temp.path()).unwrap();
    git.set_head("refs/heads/main").unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    (temp, git, repo)
}
fn stage(git: &GitRepository, paths: &[&str]) {
    let mut index = git.index().unwrap();
    for path in paths {
        index.add_path(std::path::Path::new(path)).unwrap();
    }
    index.write().unwrap();
}
fn commit(git: &GitRepository) -> git2::Oid {
    let tree_id = git.index().unwrap().write_tree().unwrap();
    let tree = git.find_tree(tree_id).unwrap();
    let author = Signature::now("Fixture", "fixture@example.invalid").unwrap();
    let parent = git.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<_> = parent.iter().collect();
    git.commit(Some("HEAD"), &author, &author, "fixture", &tree, &parents)
        .unwrap()
}
fn write_fixture(temp: &tempfile::TempDir, git: &GitRepository) {
    fs::write(
        temp.path().join("central.rs"),
        "pub fn central() -> u8 { 1 }\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("caller.rs"),
        "use crate::central::central;\npub fn caller() { central(); }\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("test.rs"),
        "fn test_flow() { caller(); }\n",
    )
    .unwrap();
    stage(git, &["central.rs", "caller.rs", "test.rs"]);
    commit(git);
}

#[test]
fn central_edit_maps_both_symbols_and_direct_indirect_test_dependents_without_writes() {
    let (temp, git, repo) = setup();
    write_fixture(&temp, &git);
    fs::write(
        temp.path().join("central.rs"),
        "pub fn central() -> u8 { 2 }\n",
    )
    .unwrap();
    let index_before = fs::read(git.path().join("index")).unwrap();
    let head_before = git.head().unwrap().target();
    let report = analyze(&repo, &Mode::Unstaged, 4, 100).unwrap();
    assert_eq!(report.direct_changes.len(), 1);
    let change = &report.direct_changes[0];
    assert_eq!(change.hunks[0].new_start, 1);
    assert!(change.current_symbols.iter().any(|s| s.name == "central"));
    assert!(change.previous_symbols.iter().any(|s| s.name == "central"));
    assert!(
        report
            .heuristic_dependents
            .iter()
            .any(|s| s.node.path == "caller.rs" && s.depth == 1)
    );
    assert!(
        report
            .heuristic_dependents
            .iter()
            .any(|s| s.node.path == "test.rs" && s.depth == 2)
    );
    assert_eq!(report.potentially_affected_tests.len(), 1);
    assert!(report.resolved_dependents.is_empty());
    assert_eq!(fs::read(git.path().join("index")).unwrap(), index_before);
    assert_eq!(git.head().unwrap().target(), head_before);
    assert!(!temp.path().join(".contextforge").exists());
    assert_eq!(
        fs::read_to_string(temp.path().join("central.rs")).unwrap(),
        "pub fn central() -> u8 { 2 }\n"
    );
}

#[test]
fn staged_and_unstaged_views_are_separate_and_deleted_symbols_remain_evidence() {
    let (temp, git, repo) = setup();
    write_fixture(&temp, &git);
    fs::write(
        temp.path().join("central.rs"),
        "pub fn renamed() -> u8 { 2 }\n",
    )
    .unwrap();
    stage(&git, &["central.rs"]);
    fs::write(
        temp.path().join("central.rs"),
        "pub fn working_only() -> u8 { 3 }\n",
    )
    .unwrap();
    let staged = analyze(&repo, &Mode::Staged, 3, 100).unwrap();
    assert!(
        staged.direct_changes[0]
            .current_symbols
            .iter()
            .any(|s| s.name == "renamed")
    );
    assert!(
        !staged.direct_changes[0]
            .current_symbols
            .iter()
            .any(|s| s.name == "working_only")
    );
    assert!(
        staged
            .heuristic_dependents
            .iter()
            .any(|s| s.node.name == "caller" && s.view == "previous")
    );
    let unstaged = analyze(&repo, &Mode::Unstaged, 3, 100).unwrap();
    assert!(
        unstaged.direct_changes[0]
            .previous_symbols
            .iter()
            .any(|s| s.name == "renamed")
    );
    assert!(
        unstaged.direct_changes[0]
            .current_symbols
            .iter()
            .any(|s| s.name == "working_only")
    );
    fs::remove_file(temp.path().join("central.rs")).unwrap();
    let deletion = analyze(&repo, &Mode::Unstaged, 3, 100).unwrap();
    assert_eq!(deletion.direct_changes[0].kind, "deleted");
    assert!(!deletion.direct_changes[0].previous_symbols.is_empty());
}

#[test]
fn merge_base_detached_head_missing_refs_and_explicit_file() {
    let (temp, git, repo) = setup();
    write_fixture(&temp, &git);
    let base = git.head().unwrap().target().unwrap();
    git.reference("refs/heads/base", base, false, "fixture")
        .unwrap();
    fs::write(
        temp.path().join("central.rs"),
        "pub fn central() -> u8 { 2 }\n",
    )
    .unwrap();
    stage(&git, &["central.rs"]);
    let current = commit(&git);
    git.set_head_detached(current).unwrap();
    let branch = analyze(
        &repo,
        &Mode::Base {
            base: "base".into(),
            head: "HEAD".into(),
        },
        3,
        100,
    )
    .unwrap();
    assert_eq!(branch.direct_changes.len(), 1);
    assert!(
        branch
            .heuristic_dependents
            .iter()
            .any(|s| s.node.name == "caller")
    );
    assert!(
        analyze(
            &repo,
            &Mode::Base {
                base: "missing-ref".into(),
                head: "HEAD".into()
            },
            3,
            100
        )
        .is_err()
    );
    let explicit = analyze(&repo, &Mode::File("central.rs".into()), 3, 100).unwrap();
    assert_eq!(explicit.direct_changes.len(), 1);
    assert!(!explicit.direct_changes[0].current_symbols.is_empty());
    assert!(analyze(&repo, &Mode::File("../escape.rs".into()), 3, 100).is_err());
}

#[test]
fn binary_ignored_renamed_and_external_diff_settings_are_safe() {
    let (temp, git, repo) = setup();
    write_fixture(&temp, &git);
    fs::write(
        temp.path().join(".gitattributes"),
        "*.rs diff=must_not_execute\n",
    )
    .unwrap();
    git.config()
        .unwrap()
        .set_str(
            "diff.must_not_execute.command",
            "untrusted-command-that-must-not-run",
        )
        .unwrap();
    fs::write(temp.path().join("central.rs"), [0u8, 1, 2]).unwrap();
    let binary = analyze(&repo, &Mode::Unstaged, 3, 100).unwrap();
    assert!(binary.direct_changes[0].skipped.is_some());
    assert!(binary.direct_changes[0].hunks.is_empty());
    assert!(binary.incomplete_content);
    assert!(
        binary.direct_changes[0]
            .previous_symbols
            .iter()
            .any(|node| node.name == "central")
    );
    assert!(
        binary
            .heuristic_dependents
            .iter()
            .any(|visit| visit.node.name == "caller")
    );
    fs::write(temp.path().join(".contextforgeignore"), "central.rs\n").unwrap();
    assert!(
        analyze(&repo, &Mode::Unstaged, 3, 100)
            .unwrap()
            .direct_changes
            .is_empty()
    );
    fs::remove_file(temp.path().join(".contextforgeignore")).unwrap();
    fs::write(
        temp.path().join("central.rs"),
        "pub fn central() -> u8 { 1 }\n",
    )
    .unwrap();
    fs::rename(temp.path().join("central.rs"), temp.path().join("moved.rs")).unwrap();
    let rename = analyze(&repo, &Mode::Unstaged, 3, 100).unwrap();
    assert!(
        rename
            .direct_changes
            .iter()
            .any(|c| c.kind == "deleted" && c.old_path.as_deref() == Some("central.rs"))
    );
    assert!(
        rename
            .direct_changes
            .iter()
            .any(|c| c.kind == "added" && c.new_path.as_deref() == Some("moved.rs"))
    );
}

#[test]
fn unborn_staged_and_invalid_limits_do_not_panic() {
    let (temp, git, repo) = setup();
    fs::write(temp.path().join("new.py"), "def fresh(): pass\n").unwrap();
    stage(&git, &["new.py"]);
    let report = analyze(&repo, &Mode::Staged, 2, 20).unwrap();
    assert_eq!(report.direct_changes[0].kind, "added");
    fs::write(temp.path().join("large.rs"), "x".repeat(1_048_577)).unwrap();
    stage(&git, &["large.rs"]);
    let oversized = analyze(&repo, &Mode::Staged, 2, 20).unwrap();
    assert!(
        oversized
            .direct_changes
            .iter()
            .any(|change| change.new_path.as_deref() == Some("large.rs")
                && change.skipped == Some("oversized"))
    );
    assert!(analyze(&repo, &Mode::Staged, 9, 20).is_err());
    assert!(analyze(&repo, &Mode::Staged, 2, 0).is_err());
    let empty = tempfile::tempdir().unwrap();
    let repo = Repository::open(Some(empty.path()), empty.path()).unwrap();
    assert!(analyze(&repo, &Mode::Unstaged, 2, 20).is_err());
}
