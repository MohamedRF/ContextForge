//! Read-only harness configuration previews. No harness files are opened or written.

use serde::Serialize;
use serde_json::{Value, json};
use std::path::Path;

mod configuration;
pub use configuration::{ChangePlan, configuration_status, plan};

#[derive(Clone, Copy, Debug, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Harness {
    Claude,
    Codex,
    Cursor,
    Vscode,
}

#[derive(Serialize)]
pub struct Preview {
    pub harness: &'static str,
    pub destination: &'static str,
    pub format: &'static str,
    pub configuration: String,
    pub writes_files: bool,
    pub instructions: &'static str,
    pub verification: &'static str,
}

// JSON string escapes produced by serde_json are also valid TOML basic-string
// escapes. Paths with control characters or client interpolation are rejected.
fn launch_path(path: &Path) -> Result<String, &'static str> {
    let text = path.to_str().ok_or("setup requires UTF-8 paths")?;
    if text.chars().any(char::is_control) || text.contains("${") {
        return Err("setup paths must not contain control characters or client interpolation");
    }
    #[cfg(windows)]
    let text = text
        .strip_prefix(r"\\?\UNC\")
        .map(|tail| format!(r"\\{tail}"))
        .unwrap_or_else(|| text.strip_prefix(r"\\?\").unwrap_or(text).to_owned());
    Ok(text.to_string())
}

pub fn preview(
    harness: Harness,
    root: &Path,
    executable: &Path,
) -> Result<Preview, Box<dyn std::error::Error + Send + Sync>> {
    if !root.is_absolute() || !executable.is_absolute() {
        return Err("setup requires absolute repository and executable paths".into());
    }
    let command = launch_path(executable)?;
    let args = ["--root".to_owned(), launch_path(root)?, "mcp".to_owned()];
    let mut server = json!({"command":command,"args":args});
    let (name, destination, format, configuration) = match harness {
        Harness::Codex => (
            "codex",
            "$CODEX_HOME/config.toml (default ~/.codex/config.toml)",
            "toml",
            format!(
                "[mcp_servers.contextforge]\ncommand = {}\nargs = {}\n",
                serde_json::to_string(&command)?,
                serde_json::to_string(&args)?
            ),
        ),
        other => {
            let (name, destination, key) = match other {
                Harness::Claude => ("claude", "<repository>/.mcp.json", "mcpServers"),
                Harness::Cursor => ("cursor", "<repository>/.cursor/mcp.json", "mcpServers"),
                Harness::Vscode => ("vscode", "<repository>/.vscode/mcp.json", "servers"),
                Harness::Codex => unreachable!(),
            };
            if matches!(other, Harness::Vscode | Harness::Claude) {
                server["type"] = Value::String("stdio".to_owned());
            }
            (
                name,
                destination,
                "json",
                serde_json::to_string_pretty(&json!({key:{"contextforge":server}}))?,
            )
        }
    };
    Ok(Preview {
        harness: name,
        destination,
        format,
        configuration,
        writes_files: false,
        instructions: "Preview only. Back up the destination, then merge the contextforge entry manually, preserving unrelated settings. Review any existing entry with the same name before replacement. Run init and index explicitly before indexed retrieval. To undo, restore your backup or remove only this entry.",
        verification: "Configuration format checked against official documentation; actual harness connection NOT VERIFIED. Restart/reload the client, review its server trust prompt, list tools and invoke project_map, then search_context on an indexed fixture.",
    })
}
