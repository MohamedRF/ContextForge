#![allow(clippy::unwrap_used)]
use contextforge_service::integrations::{Harness, configuration_status, plan};
use std::fs;

#[test]
fn reviewed_merge_preserves_entries_backs_up_and_removes_only_contextforge() {
    let temp = tempfile::tempdir().unwrap();
    // macOS temp directories may use /var -> /private/var; production callers use Repository::root().
    let root = temp.path().canonicalize().unwrap();
    let path = root.as_path().join(".mcp.json");
    let original = r#"{"mcpServers":{"other":{"command":"keep"}},"unrelated":{"enabled":true}}"#;
    fs::write(&path, original).unwrap();
    let executable = root.as_path().join("contextforge.exe");
    let change = plan(root.as_path(), Harness::Claude, Some(&executable)).unwrap();
    assert!(!change.preview.configuration.contains("unrelated"));
    let backup = change.apply().unwrap().unwrap();
    assert_eq!(fs::read_to_string(backup).unwrap(), original);
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(value["mcpServers"]["other"]["command"], "keep");
    assert_eq!(value["unrelated"]["enabled"], true);
    assert_eq!(
        configuration_status(root.as_path(), Harness::Claude).unwrap()["contextforge_entry_present"],
        true
    );
    plan(root.as_path(), Harness::Claude, None)
        .unwrap()
        .apply()
        .unwrap();
    let value: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    assert!(value["mcpServers"].get("contextforge").is_none());
    assert_eq!(value["mcpServers"]["other"]["command"], "keep");
}

#[test]
fn rejects_changed_invalid_and_oversized_client_configuration() {
    let temp = tempfile::tempdir().unwrap();
    // macOS temp directories may use /var -> /private/var; production callers use Repository::root().
    let root = temp.path().canonicalize().unwrap();
    let path = root.as_path().join(".mcp.json");
    let executable = root.as_path().join("contextforge");
    let change = plan(root.as_path(), Harness::Claude, Some(&executable)).unwrap();
    fs::write(&path, "{\"changed\":true}").unwrap();
    assert!(change.apply().is_err());
    assert_eq!(fs::read_to_string(&path).unwrap(), "{\"changed\":true}");
    for invalid in [
        "not JSON".to_string(),
        "[]".into(),
        "{\"mcpServers\":[]}".into(),
        "x".repeat(1_048_577),
    ] {
        fs::write(&path, invalid).unwrap();
        assert!(plan(root.as_path(), Harness::Claude, Some(&executable)).is_err());
    }
}

#[test]
fn cursor_and_vscode_use_their_own_server_keys() {
    let temp = tempfile::tempdir().unwrap();
    // macOS temp directories may use /var -> /private/var; production callers use Repository::root().
    let root = temp.path().canonicalize().unwrap();
    let executable = root.as_path().join("contextforge");
    for (harness, path, key) in [
        (Harness::Cursor, ".cursor/mcp.json", "mcpServers"),
        (Harness::Vscode, ".vscode/mcp.json", "servers"),
    ] {
        plan(root.as_path(), harness, Some(&executable))
            .unwrap()
            .apply()
            .unwrap();
        let value: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(root.as_path().join(path)).unwrap()).unwrap();
        assert!(value[key].get("contextforge").is_some());
    }
}
