#![allow(clippy::unwrap_used, clippy::expect_used)]
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{fs, process::Command, time::Duration};
use tempfile::tempdir;

fn preview(root: &std::path::Path, harness: &str, explicit_dry_run: bool) -> Value {
    let mut command = Command::new(env!("CARGO_BIN_EXE_contextforge"));
    command
        .arg("--root")
        .arg(root)
        .args(["--json", "setup", harness]);
    if explicit_dry_run {
        command.arg("--dry-run");
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[tokio::test]
async fn all_harness_previews_parse_and_launch_the_same_real_server_without_writes() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let temp = tempdir().unwrap();
        let root = temp.path().join("源 project 'quoted'");
        fs::create_dir(&root).unwrap();
        fs::write(root.join("hello.rs"), "fn hello() {}\n").unwrap();
        let existing = b"{\"mcpServers\":{\"other\":{\"command\":\"preserve\"}}}\n";
        fs::write(root.join(".mcp.json"), existing).unwrap();
        for harness in ["claude", "codex", "cursor", "vscode"] {
            let plan = preview(&root, harness, true);
            assert_eq!(plan, preview(&root, harness, false));
            assert_eq!(plan["writes_files"], false);
            assert!(
                plan["verification"]
                    .as_str()
                    .unwrap()
                    .contains("NOT VERIFIED")
            );
            let config = plan["configuration"].as_str().unwrap();
            let server = if harness == "codex" {
                let parsed: toml::Value = toml::from_str(config).unwrap();
                serde_json::to_value(&parsed["mcp_servers"]["contextforge"]).unwrap()
            } else {
                let parsed: Value = serde_json::from_str(config).unwrap();
                let key = if harness == "vscode" {
                    "servers"
                } else {
                    "mcpServers"
                };
                assert_eq!(parsed.as_object().unwrap().len(), 1);
                if harness == "vscode" || harness == "claude" {
                    assert_eq!(parsed[key]["contextforge"]["type"], "stdio");
                }
                parsed[key]["contextforge"].clone()
            };
            let args: Vec<&str> = server["args"]
                .as_array()
                .unwrap()
                .iter()
                .map(|a| a.as_str().unwrap())
                .collect();
            assert_eq!(args.len(), 3);
            assert_eq!(args[0], "--root");
            assert_eq!(args[2], "mcp");
            let mut command = tokio::process::Command::new(server["command"].as_str().unwrap());
            command.args(args);
            let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
            assert_eq!(client.list_all_tools().await.unwrap().len(), 7);
            let result = client
                .call_tool(
                    CallToolRequestParams::new("project_map")
                        .with_arguments(json!({"limit":10}).as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            assert_ne!(result.is_error, Some(true));
            assert_eq!(
                result.structured_content.unwrap()["files"][0]["path"],
                "hello.rs"
            );
            client.cancel().await.unwrap();
        }
        assert_eq!(fs::read(root.join(".mcp.json")).unwrap(), existing);
        assert!(!root.join(".contextforge").exists());
        assert!(!root.join(".cursor").exists());
        assert!(!root.join(".vscode").exists());
    })
    .await
    .expect("setup descriptor test timed out");
}

#[test]
fn setup_rejects_unknown_harness_and_write_flags() {
    let temp = tempdir().unwrap();
    for args in [vec!["setup", "unknown"], vec!["setup", "codex", "--apply"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_contextforge"))
            .arg("--root")
            .arg(temp.path())
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert!(output.stdout.is_empty());
    }
    assert_eq!(fs::read_dir(temp.path()).unwrap().count(), 0);
}

#[test]
fn setup_refuses_client_interpolation_in_paths() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("${INJECTED}");
    fs::create_dir(&root).unwrap();
    for harness in ["claude", "codex", "cursor", "vscode"] {
        let output = Command::new(env!("CARGO_BIN_EXE_contextforge"))
            .arg("--root")
            .arg(&root)
            .args(["setup", harness])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("INJECTED"));
    }
}
