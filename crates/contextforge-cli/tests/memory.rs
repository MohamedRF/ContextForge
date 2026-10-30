#![allow(clippy::unwrap_used, clippy::expect_used)]
use serde_json::Value;
use std::{fs, process::Command};

fn run(path: &std::path::Path, args: &[&str]) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_contextforge"))
        .arg("--root")
        .arg(path)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn separate_cli_processes_reuse_validate_clear_and_rebuild_knowledge() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("lib.rs"), "fn original() {}\n").unwrap();
    run(temp.path(), &["init", "--json"]);
    run(temp.path(), &["memory", "rebuild"]);
    let status = run(temp.path(), &["memory", "status"]);
    assert_eq!(status["freshness"], "validated_at_read");
    assert_eq!(status["files"][0]["symbols"], 1);
    fs::write(temp.path().join("lib.rs"), "fn changed() {}\n").unwrap();
    assert_eq!(
        run(temp.path(), &["memory", "status"])["freshness"],
        "stale"
    );
    run(temp.path(), &["memory", "rebuild"]);
    assert_eq!(
        run(temp.path(), &["memory", "status"])["freshness"],
        "validated_at_read"
    );
    assert_eq!(run(temp.path(), &["memory", "clear"])["cleared_files"], 1);
    assert_eq!(
        fs::read_to_string(temp.path().join("lib.rs")).unwrap(),
        "fn changed() {}\n"
    );
}
