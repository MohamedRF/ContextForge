#![allow(clippy::unwrap_used, clippy::expect_used)]
use std::{
    fs,
    process::{Command, Output},
};
use tempfile::tempdir;

fn run(root: &std::path::Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_contextforge"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn help_version_and_invalid_arguments() {
    let temp = tempdir().unwrap();
    for argument in ["--help", "--version"] {
        let output = run(temp.path(), &[argument]);
        assert!(output.status.success());
        assert!(String::from_utf8_lossy(&output.stdout).contains("contextforge"));
        if argument == "--version" {
            assert_eq!(
                String::from_utf8_lossy(&output.stdout).trim(),
                "contextforge 0.1.0"
            );
        }
    }
    for args in [
        vec!["unknown"],
        vec!["project-map", "--limit", "0"],
        vec!["project-map", "--limit", "1001"],
    ] {
        let output = run(temp.path(), &args);
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
}

#[test]
fn missing_repo_and_invalid_config_are_safe_errors() {
    let temp = tempdir().unwrap();
    let output = run(temp.path(), &["status"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--root"));
    fs::create_dir(temp.path().join(".contextforge")).unwrap();
    fs::write(
        temp.path().join(".contextforge/config.toml"),
        "SECRET_DO_NOT_LEAK = [",
    )
    .unwrap();
    let output = run(temp.path(), &["status"]);
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!String::from_utf8_lossy(&output.stderr).contains("SECRET_DO_NOT_LEAK"));
}

#[test]
fn unicode_init_status_doctor_and_map() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("源 project");
    fs::create_dir(&root).unwrap();
    fs::write(root.join("example.rs"), "fn example() {}\n").unwrap();
    for command in ["init", "index", "status", "doctor", "project-map"] {
        let output = run(&root, &["--root", ".", "--json", command]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert!(value.is_object());
        if command == "project-map" {
            assert_eq!(value["files"][0]["path"], "example.rs");
        }
    }
    assert_eq!(
        fs::read_to_string(root.join("example.rs")).unwrap(),
        "fn example() {}\n"
    );
    assert!(!root.join(".gitignore").exists());
}

#[test]
fn analyze_current_source_json_human_and_invalid_paths() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("源.rs"), "pub fn café() {}\n").unwrap();
    for json in [false, true] {
        let mut args = vec!["--root", "."];
        if json {
            args.push("--json");
        }
        args.extend(["analyze", "源.rs"]);
        let output = run(temp.path(), &args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        if json {
            let outline: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(outline["symbols"][0]["name"], "café");
            assert_eq!(outline["symbols"][0]["span"]["start_line"], 1);
        } else {
            assert!(String::from_utf8_lossy(&output.stdout).contains("café"));
        }
    }
    for path in ["../outside.rs", "missing.rs"] {
        let output = run(temp.path(), &["--root", ".", "--json", "analyze", path]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains(temp.path().to_str().unwrap()));
    }
    assert!(!temp.path().join(".contextforge").exists());
}

#[test]
fn indexed_search_requires_explicit_index_and_returns_cited_json() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("a.rs"),
        "fn water() { let text = \"\u{1b}[31m\"; }\n",
    )
    .unwrap();
    let missing = run(temp.path(), &["--root", ".", "search", "water"]);
    assert!(!missing.status.success());
    assert!(missing.stdout.is_empty());
    assert!(!temp.path().join(".contextforge").exists());
    for command in ["init", "index"] {
        let output = run(temp.path(), &["--root", ".", command]);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let output = run(
        temp.path(),
        &["--root", ".", "--json", "search", "water", "--limit", "1"],
    );
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let result: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(result["results"][0]["path"], "a.rs");
    assert_eq!(result["results"][0]["symbol"], "water");
    assert_eq!(result["results"][0]["span"]["start_line"], 1);
    let output = run(temp.path(), &["--root", ".", "search", "water"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("a.rs:1-1"));
    assert!(!output.stdout.contains(&0x1b));
    assert!(String::from_utf8_lossy(&output.stderr).contains("freshness not checked"));
}
