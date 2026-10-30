#![allow(clippy::unwrap_used, clippy::expect_used)]
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::json;
use std::{fs, process::Command, time::Duration};

#[tokio::test]
async fn captured_graph_cli_and_official_mcp_client_preserve_sources_and_cache() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let temp = tempfile::tempdir().unwrap();
        fs::write(
            temp.path().join("core.rs"),
            "fn central() {}\nfn caller() { central(); }\n",
        )
        .unwrap();
        for operation in ["init", "index"] {
            assert!(
                Command::new(env!("CARGO_BIN_EXE_contextforge"))
                    .arg("--root")
                    .arg(temp.path())
                    .arg(operation)
                    .output()
                    .unwrap()
                    .status
                    .success()
            );
        }
        let database = fs::read_dir(temp.path().join(".contextforge/index"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
            .join("index.sqlite");
        let before = fs::read(&database).unwrap();
        for operation in ["graph", "references"] {
            let output = Command::new(env!("CARGO_BIN_EXE_contextforge"))
                .arg("--root")
                .arg(temp.path())
                .arg("--json")
                .arg(operation)
                .arg("central")
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert!(!value["nodes"].as_array().unwrap().is_empty());
        }
        let mut command = tokio::process::Command::new(env!("CARGO_BIN_EXE_contextforge"));
        command.arg("--root").arg(temp.path()).arg("mcp");
        let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
        for name in ["find_references", "dependency_graph"] {
            let result = client
                .call_tool(
                    CallToolRequestParams::new(name)
                        .with_arguments(json!({"selector":"central"}).as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            assert_ne!(result.is_error, Some(true));
            let value = result.structured_content.unwrap();
            assert!(!value["nodes"].as_array().unwrap().is_empty());
            if name == "find_references" {
                assert_eq!(value["edges"][0]["status"], "heuristic");
            }
        }
        assert!(
            client
                .call_tool(
                    CallToolRequestParams::new("dependency_graph").with_arguments(
                        json!({"selector":"central","depth":9})
                            .as_object()
                            .unwrap()
                            .clone()
                    )
                )
                .await
                .is_err()
        );
        client.cancel().await.unwrap();
        assert_eq!(fs::read(database).unwrap(), before);
        assert_eq!(
            fs::read_to_string(temp.path().join("core.rs")).unwrap(),
            "fn central() {}\nfn caller() { central(); }\n"
        );
    })
    .await
    .expect("graph client timed out");
}
