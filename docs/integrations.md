# Harness configuration

Use the same installed ContextForge executable for every harness. First run
`contextforge --root <project> init` and `contextforge --root <project> index`.
Then generate a configuration fragment:

```text
contextforge --root <project> setup claude --dry-run
contextforge --root <project> setup codex --dry-run
contextforge --root <project> setup cursor --dry-run
contextforge --root <project> setup vscode --dry-run
```

`--dry-run` is optional: setup always previews and never reads or writes harness
configuration. `--json` returns the destination description, format, configuration
string, instructions, verification status and `writes_files: false`. The generated
command is the running executable's absolute path; arguments authorize one fixed
canonical repository root. Generate the preview from your installed binary rather
than a temporary build that you intend to delete. Paths must be UTF-8 without
control characters or `${...}` client interpolation.

Before applying manually, back up the destination. Merge only the `contextforge`
entry, preserve other servers/settings and review a conflicting existing entry.
For Codex, add the table once; duplicate TOML tables are invalid. For JSON, merge
into the existing server object, rather than appending a second JSON document.
Restore the backup to roll back, or remove only the added entry. Automatic merge,
backup and modification are not implemented.

## Formats and verification

Official documentation checked on 2026-10-30:

| Harness | Manual destination | Server entry | Evidence |
|---|---|---|---|
| Claude Code | Project `.mcp.json` | `mcpServers.contextforge` with `type: "stdio"`, `command`, `args` | [Official MCP documentation](https://code.claude.com/docs/en/mcp); JSON parsed and descriptor launched with official rmcp client; Claude installation unavailable |
| Codex | `$CODEX_HOME/config.toml`, default `~/.codex/config.toml` | `[mcp_servers.contextforge]`, `command`, `args` | [Official MCP documentation](https://developers.openai.com/codex/mcp); generated TOML parsed by Codex CLI 0.166.0-alpha.5.1 on Windows in isolated CODEX_HOME; descriptor launched with official rmcp client |
| Cursor | Project `.cursor/mcp.json` | `mcpServers.contextforge`, `command`, `args` | [Official MCP documentation](https://cursor.com/docs/mcp); JSON parsed and descriptor launched with official rmcp client; Cursor installation unavailable |
| VS Code / Copilot | Workspace `.vscode/mcp.json` | `servers.contextforge` with `type: "stdio"`, `command`, `args` | [Official VS Code format reference](https://code.visualstudio.com/docs/agents/reference/mcp-configuration); JSON parsed and descriptor launched with official rmcp client; VS Code 1.144.0 present, interactive Copilot call not run |

The VS Code preview targets its workspace format. Copilot's portable user format
and other VS Code providers can have different destinations and schemas; this
preview does not target them. Remote workspaces require the binary and repository
to exist in the execution environment.

These checks verify syntax and launch descriptors, not actual agent-session
interoperability. **Actual harness tool calls remain NOT RUN for all four.**
No user-level settings were changed and no agent credentials were used.

Reproduce the isolated Codex configuration check with Python 3.11+:

```text
python scripts/check-codex-setup.py --binary target/release/contextforge.exe
```

Use `target/release/contextforge` on Unix. `--codex <executable>` selects a Codex
CLI installation. The script creates a temporary CODEX_HOME and checks the real
CLI's `mcp get contextforge --json` result without modifying your configuration.

## Manual connection acceptance

After merging, reload the client and review its MCP trust/approval prompt.
List the seven tools and invoke `project_map` with `{"limit":10}`. Confirm a
relative path from the authorized repository. Invoke `search_context` with a
known indexed symbol and confirm its path, source range and captured fingerprint.
Tools do not refresh the index; rerun CLI `index` after source changes.

For Codex, inspect `codex mcp get contextforge --json`; for Claude Code, inspect
`claude mcp get contextforge` and the interactive MCP status. In Cursor inspect
the MCP settings; in VS Code use the MCP server controls and output channel.
Record client version, OS, config destination, tool request/result and restart
behavior before claiming harness interoperability. A listed server alone is not
proof of a successful tool call.
