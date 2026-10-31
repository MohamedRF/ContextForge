"""Collect native Tauri bundles with architecture-specific names and SHA-256 files."""
import hashlib
from pathlib import Path
import shutil
import subprocess
import tomllib


def main():
    root = Path(__file__).resolve().parents[1]
    version = tomllib.loads((root / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    details = subprocess.run(["rustc", "-vV"], capture_output=True, text=True,
                             check=True, timeout=30).stdout
    target = next(line[6:] for line in details.splitlines() if line.startswith("host: "))
    bundles = root / "target/release/bundle"
    output = root / "target/packages"
    output.mkdir(parents=True, exist_ok=True)
    artifacts = []
    for path in sorted(bundles.rglob("*")):
        if path.is_file() and (path.suffix == ".dmg" or path.name.endswith("-setup.exe")):
            suffix = "-setup.exe" if path.suffix == ".exe" else ".dmg"
            destination = output / f"ContextForge-{version}-{target}{suffix}"
            if destination.exists():
                raise RuntimeError("refusing to overwrite an existing desktop artifact")
            shutil.copy2(path, destination)
            artifacts.append(destination)
        elif path.is_dir() and path.suffix == ".app":
            destination = output / f"ContextForge-{version}-{target}.app.zip"
            if destination.exists():
                raise RuntimeError("refusing to overwrite an existing app archive")
            # Apple's distribution guidance uses ditto so app-bundle metadata,
            # executable modes and symbolic links survive ZIP packaging.
            subprocess.run(
                ["ditto", "-c", "-k", "--keepParent", str(path), str(destination)],
                check=True, timeout=120,
            )
            artifacts.append(destination)
    if not artifacts:
        raise RuntimeError("no native desktop bundles found")
    for path in artifacts:
        checksum = hashlib.sha256(path.read_bytes()).hexdigest()
        path.with_name(path.name + ".sha256").write_text(f"{checksum}  {path.name}\n")
        print(path.name)


if __name__ == "__main__":
    main()
