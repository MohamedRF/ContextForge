"""Package a native release binary, checksum it and smoke-test its extracted copy.

Local preparation only: never uploads artifacts, publishes packages or edits PATH.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import stat
import subprocess
import tempfile
import zipfile


def run(binary, *args):
    result = subprocess.run([str(binary), *map(str, args)], capture_output=True,
                            text=True, encoding="utf-8", timeout=60, check=False)
    if result.returncode:
        raise RuntimeError(f"packaged binary smoke failed: exit {result.returncode}")
    return result.stdout


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", type=Path, required=True)
    parser.add_argument("--target", required=True)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    supported = {
        "x86_64-pc-windows-gnu": ("Windows", "AMD64"),
        "x86_64-pc-windows-msvc": ("Windows", "AMD64"),
        "x86_64-unknown-linux-gnu": ("Linux", "x86_64"),
        "aarch64-unknown-linux-gnu": ("Linux", "aarch64"),
        "aarch64-apple-darwin": ("Darwin", "arm64"),
        "x86_64-apple-darwin": ("Darwin", "x86_64"),
    }
    if supported.get(args.target) != (platform.system(), platform.machine()):
        parser.error("target must match a supported native host; cross-compiled packages need their own runner")
    binary = args.binary.resolve(strict=True)
    repo = Path(__file__).resolve().parents[1]
    version = run(binary, "--version").strip()
    # Obtain the version from the workspace; do not silently package another build.
    import tomllib
    expected = tomllib.loads((repo / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["package"]["version"]
    if version != f"contextforge {expected}":
        raise RuntimeError("binary version differs from workspace version")
    basename = f"contextforge-{expected}-{args.target}"
    executable_name = "contextforge.exe" if os.name == "nt" else "contextforge"
    args.output.mkdir(parents=True, exist_ok=True)
    archive_path = args.output / f"{basename}.zip"
    checksum_path = args.output / f"{basename}.zip.sha256"
    if archive_path.exists() or checksum_path.exists():
        raise RuntimeError("refusing to replace an existing artifact or checksum; choose a fresh output directory")
    with tempfile.TemporaryDirectory(prefix="contextforge-package-") as directory:
        staging = Path(directory)
        # Stage all output and smoke-test before publishing to the output directory.
        archive = staging / archive_path.name
        with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as bundle:
            entries = {executable_name: binary.read_bytes()}
            for name in ["LICENSE", "CHANGELOG.md"]:
                entries[name] = (repo / name).read_bytes()
            entries["README.txt"] = (
                "ContextForge development binary\n\n"
                "Extract this archive into a private directory. On Unix, run chmod +x contextforge.\n"
                "Run the binary by its full path; no global installation is required.\n"
                "Use --version and --help, then --root <project> init and --root <project> index.\n"
                "Preview client configuration with --root <project> setup <claude|codex|cursor|vscode>.\n"
                "The index contains source snapshots; keep .contextforge private.\n"
                "Full documentation and known limitations: https://github.com/MohamedRF/ContextForge\n"
                "This archive is not a stable release. See BUILD.json and CHANGELOG.md.\n"
            ).encode()
            entries["BUILD.json"] = json.dumps({
                "version": expected, "target": args.target,
                "binary_sha256": hashlib.sha256(entries[executable_name]).hexdigest(),
                "status": "unreleased development artifact; platform support requires recorded runner evidence",
            }, indent=2).encode()
            for name, contents in sorted(entries.items()):
                info = zipfile.ZipInfo(f"{basename}/{name}", date_time=(1980, 1, 1, 0, 0, 0))
                mode = 0o755 if name == executable_name else 0o644
                info.create_system = 3
                info.external_attr = (stat.S_IFREG | mode) << 16
                info.compress_type = zipfile.ZIP_DEFLATED
                bundle.writestr(info, contents)
        extracted = staging / "extracted"
        with zipfile.ZipFile(archive) as bundle:
            bundle.extractall(extracted)  # Only the fixed, locally generated names above.
        installed = extracted / basename / executable_name
        installed.chmod(0o755)
        assert run(installed, "--version").strip() == version
        fixture = staging / "fixture"
        fixture.mkdir()
        source = "pub fn packaged_hello() {}\n"
        (fixture / "hello.rs").write_text(source, encoding="utf-8", newline="")
        for operation in ["init", "index", "doctor"]:
            json.loads(run(installed, "--root", fixture, "--json", operation))
        search = json.loads(run(installed, "--root", fixture, "--json", "search", "packaged_hello"))
        assert search["results"][0]["path"] == "hello.rs"
        for harness in ["claude", "codex", "cursor", "vscode"]:
            preview = json.loads(run(installed, "--root", fixture, "--json", "setup", harness, "--dry-run"))
            assert preview["writes_files"] is False
        assert (fixture / "hello.rs").read_text(encoding="utf-8") == source
        checksum = hashlib.sha256(archive.read_bytes()).hexdigest()
        # Exclusive creation also handles another packager racing for the same name.
        own_archive = False
        own_checksum = False
        try:
            with archive_path.open("xb") as destination:
                own_archive = True
                with archive.open("rb") as source_file:
                    shutil.copyfileobj(source_file, destination)
            with checksum_path.open("x", encoding="ascii", newline="\n") as destination:
                own_checksum = True
                destination.write(f"{checksum}  {archive_path.name}\n")
        except OSError:
            # Only remove newly created outputs owned by this invocation.
            if own_checksum:
                checksum_path.unlink()
            if own_archive:
                archive_path.unlink()
            raise
    print(json.dumps({"archive": str(archive_path.resolve()), "sha256": checksum,
                      "target": args.target, "smoke": "PASS", "clean_machine_install": "NOT RUN"}))


if __name__ == "__main__":
    main()
