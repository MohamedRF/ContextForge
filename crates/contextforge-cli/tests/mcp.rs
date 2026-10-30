#![allow(clippy::unwrap_used, clippy::expect_used)]
use rmcp::{ServiceExt, model::CallToolRequestParams, transport::TokioChildProcess};
use serde_json::{Value, json};
use std::{fs, process::Stdio, time::Duration};
use tempfile::tempdir;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::Command,
};

#[tokio::test]
async fn official_client_initializes_lists_and_calls_real_tool() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("hello.rs"), "fn hello() {}\n").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_contextforge"));
        command.arg("--root").arg(temp.path()).arg("mcp");
        let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "project_map");
        let schema = serde_json::to_value(&tools[0].input_schema).unwrap();
        assert!(schema["properties"]["limit"].is_object());
        assert_eq!(schema["additionalProperties"], false);
        for _ in 0..2 {
            let result = client
                .call_tool(
                    CallToolRequestParams::new("project_map")
                        .with_arguments(json!({"limit":10}).as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            assert_ne!(result.is_error, Some(true));
            let value = result.structured_content.unwrap();
            assert_eq!(value["files"][0]["path"], "hello.rs");
            assert_eq!(value["method"], "filesystem_source_inventory");
        }
        for args in [json!({"limit":0}), json!({"limit":1001})] {
            assert!(
                client
                    .call_tool(
                        CallToolRequestParams::new("project_map")
                            .with_arguments(args.as_object().unwrap().clone())
                    )
                    .await
                    .is_err()
            );
        }
        // rmcp routes parameter-deserialization failures as tool-level errors;
        // explicit range validation above remains a JSON-RPC invalid-params error.
        for args in [json!({"limit":"bad"}), json!({"root":"../escape"})] {
            let result = client
                .call_tool(
                    CallToolRequestParams::new("project_map")
                        .with_arguments(args.as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            assert_eq!(result.is_error, Some(true));
            assert!(result.structured_content.is_none());
            assert!(!result.content.is_empty());
        }
        assert!(
            client
                .call_tool(CallToolRequestParams::new("not_a_tool"))
                .await
                .is_err()
        );
        client.cancel().await.unwrap();
        assert!(!temp.path().join(".contextforge").exists());
        assert_eq!(
            fs::read_to_string(temp.path().join("hello.rs")).unwrap(),
            "fn hello() {}\n"
        );
    })
    .await
    .expect("MCP SDK test timed out");
}

#[tokio::test]
async fn stdout_is_protocol_only_and_eof_exits_cleanly() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let temp = tempdir().unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_contextforge"))
            .arg("--root").arg(temp.path()).arg("--verbose").arg("mcp")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped())
            .kill_on_drop(true).spawn().unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut output = BufReader::new(child.stdout.take().unwrap()).lines();
        let initialize = json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-11-25","capabilities":{},"clientInfo":{"name":"contextforge-test","version":"1"}}});
        input.write_all(format!("{initialize}\n").as_bytes()).await.unwrap();
        let first: Value = serde_json::from_str(&output.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(first["id"], 1);
        assert!(first["result"]["capabilities"]["tools"].is_object());
        input.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n").await.unwrap();
        // Oversized valid JSON and malformed JSON must not poison subsequent requests.
        let oversized = json!({"jsonrpc":"2.0","id":9,"method":"ping","params":{"padding":"x".repeat(70_000)}});
        input.write_all(format!("{oversized}\nnot json\n").as_bytes()).await.unwrap();
        input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"tools/list\"}\n").await.unwrap();
        loop {
            let message: Value = serde_json::from_str(&output.next_line().await.unwrap().unwrap()).unwrap();
            assert_eq!(message["jsonrpc"], "2.0");
            assert_ne!(message["id"], 9);
            if message["id"] == 2 {
                assert_eq!(message["result"]["tools"][0]["name"], "project_map");
                break;
            }
        }
        drop(input);
        while let Some(line) = output.next_line().await.unwrap() {
            let message: Value = serde_json::from_str(&line).unwrap();
            assert_eq!(message["jsonrpc"], "2.0");
        }
        assert!(child.wait().await.unwrap().success());
        assert!(!temp.path().join(".contextforge").exists());
    }).await.expect("raw protocol test timed out");
}
