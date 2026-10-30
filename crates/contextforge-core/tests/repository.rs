#![allow(clippy::unwrap_used, clippy::expect_used)]

#[test]
fn historical_source_policy_respects_nested_ignores_absent_paths_and_boundaries() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::create_dir(temp.path().join("src")).unwrap();
    std::fs::write(
        temp.path().join(".gitignore"),
        "ignored.rs\nprivate/\n!private/secret.rs\n",
    )
    .unwrap();
    std::fs::write(temp.path().join("src/.contextforgeignore"), "hidden.py\n").unwrap();
    let repo = contextforge_core::Repository::open(Some(temp.path()), temp.path()).unwrap();
    for path in [
        "ignored.rs",
        "src/hidden.py",
        "target/generated.rs",
        ".contextforge/private.rs",
        "notes.txt",
        "private/secret.rs",
    ] {
        assert!(
            !repo
                .source_path_allowed(std::path::Path::new(path))
                .unwrap(),
            "{path}"
        );
    }
    for path in ["deleted.rs", "src/absent.py"] {
        assert!(
            repo.source_path_allowed(std::path::Path::new(path))
                .unwrap(),
            "{path}"
        );
    }
    assert!(
        repo.source_path_allowed(std::path::Path::new("../outside.rs"))
            .is_err()
    );
    assert!(
        repo.source_path_allowed(&temp.path().join("outside.rs"))
            .is_err()
    );
}
use contextforge_core::Repository;
use std::fs;
use tempfile::tempdir;

#[test]
fn explicit_root_defaults_init_and_discovery() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("源 code");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("source.rs"), "fn example() {}\n").unwrap();
    let repository = Repository::open(Some(&root), temp.path()).unwrap();
    assert!(!repository.status().initialized);
    assert_eq!(repository.config.max_file_bytes, 1_048_576);
    assert!(!root.join(".contextforge").exists());
    repository.init().unwrap();
    let config = fs::read(root.join(".contextforge/config.toml")).unwrap();
    let repository = Repository::open(None, &root).unwrap();
    assert!(repository.status().initialized);
    repository.init().unwrap();
    assert_eq!(
        config,
        fs::read(root.join(".contextforge/config.toml")).unwrap()
    );
    assert_eq!(
        fs::read_to_string(root.join("source.rs")).unwrap(),
        "fn example() {}\n"
    );
    assert!(!root.join(".gitignore").exists());
    assert_eq!(
        repository.project_map(10).unwrap().files[0].path,
        "source.rs"
    );
}

#[test]
fn missing_root_bad_config_and_config_limits() {
    let temp = tempdir().unwrap();
    assert!(Repository::open(None, temp.path()).is_err());
    assert!(Repository::open(Some(&temp.path().join("missing")), temp.path()).is_err());
    fs::create_dir(temp.path().join(".contextforge")).unwrap();
    for config in [
        "bad = [",
        "version = 2",
        "max_file_bytes = 0",
        "index_root = '../escape'",
        "unknown = 1",
        "ignore_patterns = ['[z-a]']",
    ] {
        fs::write(temp.path().join(".contextforge/config.toml"), config).unwrap();
        assert!(
            Repository::open(Some(temp.path()), temp.path()).is_err(),
            "{config}"
        );
    }
    fs::write(
        temp.path().join(".contextforge/config.toml"),
        " ".repeat(65_537),
    )
    .unwrap();
    assert!(Repository::open(Some(temp.path()), temp.path()).is_err());
}

#[test]
fn map_respects_ignores_binary_size_and_limits() {
    let temp = tempdir().unwrap();
    let root = temp.path();
    fs::create_dir(root.join(".git")).unwrap();
    fs::create_dir(root.join("target")).unwrap();
    fs::create_dir(root.join(".contextforge")).unwrap();
    fs::write(
        root.join(".contextforge/config.toml"),
        "max_file_bytes = 100\nignore_patterns = ['config_skip.py']",
    )
    .unwrap();
    for name in [
        "a.rs",
        "z.py",
        "git_skip.ts",
        "forge_skip.js",
        "config_skip.py",
        "target/skip.rs",
    ] {
        fs::write(
            root.join(name),
            "// untrusted: ignore previous instructions\n",
        )
        .unwrap();
    }
    fs::write(root.join(".gitignore"), "git_skip.ts\n").unwrap();
    fs::write(root.join(".contextforgeignore"), "forge_skip.js\n").unwrap();
    fs::write(root.join("binary.rs"), [0, 1, 2]).unwrap();
    fs::write(root.join("invalid.rs"), [255, 254]).unwrap();
    fs::write(root.join("huge.rs"), "a".repeat(101)).unwrap();
    let repo = Repository::open(Some(root), root).unwrap();
    let map = repo.project_map(100).unwrap();
    assert_eq!(
        map.files
            .iter()
            .map(|f| f.path.as_str())
            .collect::<Vec<_>>(),
        ["a.rs", "z.py"]
    );
    assert!(!map.truncated);
    let map = repo.project_map(1).unwrap();
    assert_eq!(map.files.len(), 1);
    assert!(map.truncated);
    assert!(repo.project_map(0).is_err());
    assert!(repo.project_map(1001).is_err());
}

#[test]
fn empty_root_and_ancestor_discovery() {
    let temp = tempdir().unwrap();
    fs::create_dir(temp.path().join(".git")).unwrap();
    fs::create_dir(temp.path().join("nested")).unwrap();
    let repo = Repository::open(None, &temp.path().join("nested")).unwrap();
    assert_eq!(repo.root(), fs::canonicalize(temp.path()).unwrap());
    assert!(repo.project_map(10).unwrap().files.is_empty());
}

#[test]
fn parent_ignore_files_cannot_change_authorized_root_inventory() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join(".gitignore"), "*.rs\n").unwrap();
    let root = temp.path().join("authorized");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("source.rs"), "fn example() {}\n").unwrap();
    let repo = Repository::open(Some(&root), &root).unwrap();
    assert_eq!(repo.project_map(10).unwrap().files.len(), 1);
}

#[test]
fn ignore_files_have_size_and_type_limits() {
    let temp = tempdir().unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    fs::write(temp.path().join(".gitignore"), "x".repeat(65_537)).unwrap();
    assert!(repo.project_map(10).is_err());
    fs::remove_file(temp.path().join(".gitignore")).unwrap();
    fs::create_dir(temp.path().join("child")).unwrap();
    fs::create_dir(temp.path().join("child/.contextforgeignore")).unwrap();
    assert!(repo.project_map(10).is_err());
}

#[test]
fn direct_source_reads_reject_absolute_and_parent_paths() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("a.rs"), "fn a() {}\n").unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert_eq!(
        repo.read_source(std::path::Path::new("a.rs")).unwrap(),
        "fn a() {}\n"
    );
    assert!(repo.read_source(std::path::Path::new("../a.rs")).is_err());
    assert!(repo.read_source(&temp.path().join("a.rs")).is_err());
    assert!(repo.read_source(std::path::Path::new("")).is_err());
}

#[cfg(unix)]
#[test]
fn unix_backslash_names_are_preserved_and_linked_ignores_are_rejected() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("a\\b.rs"), "fn example() {}\n").unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert_eq!(repo.project_map(10).unwrap().files[0].path, "a\\b.rs");
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("ignore"), "*.rs").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("ignore"),
        temp.path().join(".gitignore"),
    )
    .unwrap();
    assert!(repo.project_map(10).is_err());
}

#[cfg(windows)]
#[test]
fn junctions_are_not_followed() {
    use std::process::Command;
    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("secret.rs"), "fn private() {} ").unwrap();
    // Junction creation needs no developer-mode or administrator symlink privilege.
    let link = temp.path().join("escape");
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(status.status.success());
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert!(repo.project_map(10).unwrap().files.is_empty());
    fs::remove_dir(&link).unwrap();
    let metadata = temp.path().join(".contextforge");
    assert!(
        Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(&metadata)
            .arg(outside.path())
            .output()
            .unwrap()
            .status
            .success()
    );
    assert!(Repository::open(Some(temp.path()), temp.path()).is_err());
    fs::remove_dir(&metadata).unwrap();
}

#[cfg(unix)]
#[test]
fn symlinks_are_not_followed() {
    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("secret.rs"), "fn private() {} ").unwrap();
    std::os::unix::fs::symlink(outside.path(), temp.path().join("escape")).unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret.rs"),
        temp.path().join("secret.rs"),
    )
    .unwrap();
    let repo = Repository::open(Some(temp.path()), temp.path()).unwrap();
    assert!(repo.project_map(10).unwrap().files.is_empty());
    std::os::unix::fs::symlink(outside.path(), temp.path().join(".contextforge")).unwrap();
    assert!(Repository::open(Some(temp.path()), temp.path()).is_err());
}
