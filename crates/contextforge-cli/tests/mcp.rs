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
async fn cancelled_database_request_is_suppressed_and_protocol_remains_usable() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("hello.rs"), "fn hello() {}\n").unwrap();
        for op in ["init", "index"] {
            assert!(Command::new(env!("CARGO_BIN_EXE_contextforge")).arg("--root").arg(temp.path()).arg(op).output().await.unwrap().status.success());
        }
        let database = fs::read_dir(temp.path().join(".contextforge/index")).unwrap().next().unwrap().unwrap().path().join("index.sqlite");
        let lock = rusqlite::Connection::open(&database).unwrap();
        lock.execute_batch("BEGIN EXCLUSIVE").unwrap();
        let mut child = Command::new(env!("CARGO_BIN_EXE_contextforge"))
            .arg("--root").arg(temp.path()).arg("--verbose").arg("mcp")
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).kill_on_drop(true).spawn().unwrap();
        let mut input = child.stdin.take().unwrap();
        let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
        input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"2025-11-25\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1\"}}}\n").await.unwrap();
        assert_eq!(serde_json::from_str::<Value>(&lines.next_line().await.unwrap().unwrap()).unwrap()["id"], 1);
        input.write_all(b"{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"tools/call\",\"params\":{\"name\":\"search_context\",\"arguments\":{\"query\":\"PRIVATE_SENTINEL_42\"}}}\n{\"jsonrpc\":\"2.0\",\"method\":\"notifications/cancelled\",\"params\":{\"requestId\":3,\"reason\":\"PRIVATE_SENTINEL_42\"}}\n").await.unwrap();
        // rmcp removes the cancelled request from its response pool and suppresses
        // the handler's cancellation error. It must still answer subsequent pings.
        input.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"ping\"}\n").await.unwrap();
        let pong: Value = serde_json::from_str(&lines.next_line().await.unwrap().unwrap()).unwrap();
        assert_eq!(pong["id"], 4);
        assert!(pong["result"].is_object());
        lock.execute_batch("ROLLBACK").unwrap();
        drop(lock);
        drop(input);
        let result = child.wait_with_output().await.unwrap();
        assert!(result.status.success());
        assert!(!String::from_utf8_lossy(&result.stderr).contains("PRIVATE_SENTINEL_42"));
    }).await.expect("cancellation protocol test timed out");
}

#[tokio::test]
async fn official_client_initializes_lists_and_calls_real_tool() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("hello.rs"), "fn hello() {}\n").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_contextforge"));
        command.arg("--root").arg(temp.path()).arg("mcp");
        let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();
        assert_eq!(tools.len(), 7);
        let map_tool = tools
            .iter()
            .find(|tool| tool.name == "project_map")
            .unwrap();
        let schema = serde_json::to_value(&map_tool.input_schema).unwrap();
        assert!(schema["properties"]["limit"].is_object());
        assert_eq!(schema["additionalProperties"], false);
        assert!(
            client
                .call_tool(
                    CallToolRequestParams::new("search_context")
                        .with_arguments(json!({"query":"hello"}).as_object().unwrap().clone())
                )
                .await
                .is_err()
        );
        let symbols = tools
            .iter()
            .find(|tool| tool.name == "symbol_lookup")
            .unwrap();
        let schema = serde_json::to_value(&symbols.input_schema).unwrap();
        assert!(schema["properties"]["path"].is_object());
        assert_eq!(schema["additionalProperties"], false);
        for _ in 0..2 {
            let result = client
                .call_tool(
                    CallToolRequestParams::new("symbol_lookup")
                        .with_arguments(json!({"path":"hello.rs"}).as_object().unwrap().clone()),
                )
                .await
                .unwrap();
            assert_ne!(result.is_error, Some(true));
            let outline = result.structured_content.unwrap();
            assert_eq!(outline["symbols"][0]["name"], "hello");
            assert_eq!(outline["symbols"][0]["span"]["start_line"], 1);
            assert_eq!(outline["path"], "hello.rs");
        }
        for args in [
            json!({"path":"../hello.rs"}),
            json!({"path":"missing.rs"}),
            json!({"path":"hello.rs","limit":0}),
        ] {
            assert!(
                client
                    .call_tool(
                        CallToolRequestParams::new("symbol_lookup")
                            .with_arguments(args.as_object().unwrap().clone())
                    )
                    .await
                    .is_err()
            );
        }
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
async fn official_client_searches_persisted_snapshot_without_writes() {
    tokio::time::timeout(Duration::from_secs(20), async {
        let temp = tempdir().unwrap();
        fs::write(temp.path().join("hello.rs"), "fn hello() {}\n").unwrap();
        for operation in ["init", "index"] {
            let output = Command::new(env!("CARGO_BIN_EXE_contextforge"))
                .arg("--root")
                .arg(temp.path())
                .arg(operation)
                .output()
                .await
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
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
        fs::write(temp.path().join("hello.rs"), "fn changed() {}\n").unwrap();
        let mut command = Command::new(env!("CARGO_BIN_EXE_contextforge"));
        command.arg("--root").arg(temp.path()).arg("mcp");
        let client = ().serve(TokioChildProcess::new(command).unwrap()).await.unwrap();
        let tools = client.list_all_tools().await.unwrap();
        let tool = tools
            .iter()
            .find(|tool| tool.name == "search_context")
            .unwrap();
        assert_eq!(tool.input_schema["additionalProperties"], false);
        for _ in 0..2 {
            let result = client
                .call_tool(
                    CallToolRequestParams::new("search_context").with_arguments(
                        json!({"query":"hello","limit":1})
                            .as_object()
                            .unwrap()
                            .clone(),
                    ),
                )
                .await
                .unwrap();
            assert_ne!(result.is_error, Some(true));
            let result = result.structured_content.unwrap();
            assert_eq!(result["results"][0]["symbol"], "hello");
            assert_eq!(result["freshness"], "snapshot_not_checked");
        }
        for args in [
            json!({"query":""}),
            json!({"query":"hello","limit":101}),
            json!({"query":"hello","offset":10001}),
        ] {
            assert!(
                client
                    .call_tool(
                        CallToolRequestParams::new("search_context")
                            .with_arguments(args.as_object().unwrap().clone())
                    )
                    .await
                    .is_err()
            );
        }
        let malformed = client
            .call_tool(
                CallToolRequestParams::new("search_context").with_arguments(
                    json!({"query":"hello","root":"../other"})
                        .as_object()
                        .unwrap()
                        .clone(),
                ),
            )
            .await
            .unwrap();
        assert_eq!(malformed.is_error, Some(true));
        client.cancel().await.unwrap();
        assert_eq!(fs::read(&database).unwrap(), before);
        assert_eq!(
            fs::read_to_string(temp.path().join("hello.rs")).unwrap(),
            "fn changed() {}\n"
        );
    })
    .await
    .expect("search MCP test timed out");
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
                assert!(message["result"]["tools"].as_array().unwrap().iter().any(|tool| tool["name"] == "project_map"));
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
