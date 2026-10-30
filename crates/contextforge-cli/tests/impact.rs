#![allow(clippy::unwrap_used, clippy::expect_used)]
use git2::{Repository, Signature};
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::json;
use std::{fs, path::Path, process::Command, time::Duration};

#[tokio::test]
async fn real_cli_and_mcp_git_impact_are_read_only_and_cited() {
    tokio::time::timeout(Duration::from_secs(30), async {
        let temp = tempfile::tempdir().unwrap();
        let git = Repository::init(temp.path()).unwrap();
        fs::write(
            temp.path().join("core.rs"),
            "fn central() -> u8 { 1 }\nfn caller() { central(); }\n",
        )
        .unwrap();
        let mut index = git.index().unwrap();
        index.add_path(Path::new("core.rs")).unwrap();
        index.write().unwrap();
        let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
        let author = Signature::now("Fixture", "fixture@example.invalid").unwrap();
        git.commit(Some("HEAD"), &author, &author, "fixture", &tree, &[])
            .unwrap();
        fs::write(
            temp.path().join("core.rs"),
            "fn central() -> u8 { 2 }\nfn caller() { central(); }\n",
        )
        .unwrap();
        let index_before = fs::read(git.path().join("index")).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_contextforge"))
            .arg("--root")
            .arg(temp.path())
            .arg("--json")
            .arg("impact")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(
            value["direct_changes"][0]["current_symbols"][0]["name"],
            "central"
        );
        assert_eq!(value["heuristic_dependents"][0]["node"]["name"], "caller");
        let invalid = Command::new(env!("CARGO_BIN_EXE_contextforge"))
            .arg("--root")
            .arg(temp.path())
            .args(["impact", "--staged", "--base", "HEAD"])
            .output()
            .unwrap();
        assert_eq!(invalid.status.code(), Some(2));
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_contextforge"));
        command.arg("--root").arg(temp.path()).arg("mcp");
        let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
        let result = client
            .call_tool(
                CallToolRequestParams::new("analyze_change")
                    .with_arguments(json!({}).as_object().unwrap().clone()),
            )
            .await
            .unwrap();
        assert_ne!(result.is_error, Some(true));
        let value = result.structured_content.unwrap();
        assert_eq!(value["direct_changes"][0]["new_path"], "core.rs");
        assert_eq!(value["heuristic_dependents"][0]["status"], "heuristic");
        assert!(
            client
                .call_tool(
                    CallToolRequestParams::new("analyze_change").with_arguments(
                        json!({"file":"../escape.rs"}).as_object().unwrap().clone()
                    )
                )
                .await
                .is_err()
        );
        client.cancel().await.unwrap();
        assert_eq!(fs::read(git.path().join("index")).unwrap(), index_before);
        assert!(!temp.path().join(".contextforge").exists());
    })
    .await
    .expect("impact client timed out");
}
