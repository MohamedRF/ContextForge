"""Check generated TOML with an installed Codex CLI in an isolated CODEX_HOME.

Does not use credentials, launch an agent, or modify the user's configuration.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--codex", default="codex")
    args = parser.parse_args()
    binary = args.binary.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix="contextforge-codex-") as directory:
        root = Path(directory)
        plan = json.loads(subprocess.check_output([
            str(binary), "--root", str(root), "--json", "setup", "codex"], timeout=30))
        config = plan["configuration"] + '\n[mcp_servers.preserved]\ncommand = "untouched"\n'
        (root / "config.toml").write_text(config, encoding="utf-8")
        env = dict(os.environ, CODEX_HOME=str(root))
        result = subprocess.run([args.codex, "mcp", "get", "contextforge", "--json"],
                                env=env, capture_output=True, text=True, timeout=30, check=False)
        if result.returncode:
            raise RuntimeError(f"isolated Codex config check failed: exit {result.returncode}")
        transport = json.loads(result.stdout)["transport"]
        assert transport["type"] == "stdio"
        assert Path(transport["command"]).samefile(binary)
        launch_args = transport["args"]
        assert launch_args[0] == "--root" and launch_args[2:] == ["mcp"]
        assert Path(launch_args[1]).samefile(root)
        assert (root / "config.toml").read_text(encoding="utf-8") == config
        version = subprocess.check_output([args.codex, "--version"], timeout=30, text=True).strip()
        print(json.dumps({"codex_version": version, "configuration_parse": "PASS",
                          "config_preserved": True, "agent_tool_call": "NOT RUN"}))


if __name__ == "__main__":
    main()
