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
