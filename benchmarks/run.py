"""Deterministic, local CLI benchmarks; Python 3.11+, no third-party packages.

Each sample is a fresh process and includes startup/configuration/output costs.
Never reads a user repository. All generated sources live in temporary directories.
"""
import argparse
import ctypes
import hashlib
import json
import os
from pathlib import Path
import platform
import subprocess
import tempfile
import time


def windows_usage(process):
    from ctypes import wintypes

    class Memory(ctypes.Structure):
        _fields_ = [("cb", wintypes.DWORD), ("faults", wintypes.DWORD)] + [
            (name, ctypes.c_size_t) for name in (
                "peak_rss", "rss", "peak_paged", "paged", "peak_nonpaged",
                "nonpaged", "pagefile", "peak_pagefile")]

    memory = Memory()
    memory.cb = ctypes.sizeof(memory)
    handle = wintypes.HANDLE(int(process._handle))
    if not ctypes.windll.psapi.GetProcessMemoryInfo(handle, ctypes.byref(memory), memory.cb):
        raise ctypes.WinError()
    times = [wintypes.FILETIME() for _ in range(4)]
    if not ctypes.windll.kernel32.GetProcessTimes(handle, *(ctypes.byref(t) for t in times)):
        raise ctypes.WinError()
    ticks = lambda t: (t.dwHighDateTime << 32) | t.dwLowDateTime
    return {"peak_rss_bytes": memory.peak_rss,
            "cpu_seconds": (ticks(times[2]) + ticks(times[3])) / 10_000_000}


def invoke(binary, root, args):
    started = time.perf_counter()
    # File-backed pipes avoid output deadlocks while keeping process memory separate.
    with tempfile.TemporaryFile() as output, tempfile.TemporaryFile() as errors:
        process = subprocess.Popen([str(binary), "--root", str(root), "--json", *args],
                                   stdout=output, stderr=errors)
        try:
            process.wait(timeout=120)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
            raise RuntimeError("benchmark command exceeded 120 seconds") from None
        elapsed = time.perf_counter() - started
        usage = windows_usage(process) if os.name == "nt" else {
            "peak_rss_bytes": None, "cpu_seconds": None}
        usage["cpu_percent_one_core"] = (
            usage["cpu_seconds"] / elapsed * 100 if usage["cpu_seconds"] is not None else None)
        output.seek(0)
        payload = output.read()
        if process.returncode:
            # Do not print arbitrary diagnostic/source payloads.
            raise RuntimeError(f"benchmark command {args[0]} failed: exit {process.returncode}")
    value = json.loads(payload)
    return value, {"wall_seconds": elapsed, **usage, "stdout_bytes": len(payload)}


def source(number, count):
    target = (number + 1) % count
    ext = ["rs", "py", "ts", "js"][number % 4]
    body = {
        "rs": f"pub fn task_{number}() {{ task_{target}(); }}\n",
        "py": f"def task_{number}():\n    task_{target}()\n",
        "ts": f"export function task_{number}(): void {{ task_{target}(); }}\n",
        "js": f"export function task_{number}() {{ task_{target}(); }}\n",
    }[ext]
    return f"file_{number:04}.{ext}", body


def measure(binary, count, repeats):
    records = []
    for trial in range(repeats):
        with tempfile.TemporaryDirectory(prefix="contextforge-bench-") as directory:
            root = Path(directory)
            files = dict(source(i, count) for i in range(count))
            digest = hashlib.sha256()
            for name, text in files.items():
                (root / name).write_text(text, encoding="utf-8", newline="")
                digest.update(name.encode() + b"\0" + text.encode())
            invoke(binary, root, ["init"])
            result = {"files": count, "trial": trial, "fixture_sha256": digest.hexdigest(),
                      "source_bytes": sum(len(t.encode()) for t in files.values()), "operations": {}}
            def run(name, args):
                value, metrics = invoke(binary, root, args)
                result["operations"][name] = metrics
                return value
            cold = run("cold_index", ["index"])
            assert cold["added"] == count and cold["parsed_files"] == count
            unchanged = run("unchanged_index", ["index"])
            assert unchanged["unchanged"] == count and unchanged["parsed_files"] == 0
            first = next(iter(files))
            (root / first).write_text(files[first] + "\n// incremental edit\n", encoding="utf-8", newline="")
            changed = run("single_edit_index", ["index"])
            assert changed["modified"] == 1 and changed["parsed_files"] == 1
            search = run("search", ["search", "task_0", "--limit", "20"])
            assert any(hit["path"] == first for hit in search["results"])
            outline = run("symbol_outline", ["analyze", first])
            assert outline["symbols"][0]["name"] == "task_0"
            graph = run("graph", ["graph", "task_0", "--depth", "3", "--limit", "20"])
            assert graph["nodes"]
            context = run("context", ["context", "task_0", "--budget", "4000"])
            assert context["estimated_tokens"] <= 4000
            result["context_estimated_tokens"] = context["estimated_tokens"]
            result["context_truncated"] = context["truncated"]
            # Ground truth: definition plus its sole incoming caller. This is a
            # tiny synthetic name task, not semantic task success or token savings.
            expected = {first, next(reversed(files))}
            started = time.perf_counter()
            keyword = {name for name in files if "task_0" in (root / name).read_text(encoding="utf-8")}
            keyword_seconds = time.perf_counter() - started
            def quality(paths):
                hits = len(paths & expected)
                return {"precision": hits / len(paths) if paths else 0,
                        "recall": hits / len(expected), "retrieved_files": len(paths)}
            result["quality"] = {
                "expected_files": sorted(expected),
                "keyword": {**quality(keyword), "wall_seconds": keyword_seconds},
                "search": quality({hit["path"] for hit in search["results"]}),
                "context": quality({hit["path"] for hit in context["results"]})}
            result["index_bytes"] = sum(p.stat().st_size for p in (root / ".contextforge").rglob("*.sqlite*"))
            records.append(result)
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--files", nargs="+", type=int, default=[100, 1000])
    parser.add_argument("--repeats", type=int, default=3)
    args = parser.parse_args()
    if not 1 <= args.repeats <= 20 or any(n < 4 or n > 1000 for n in args.files):
        parser.error("repeats must be 1..20 and file counts 4..1000 (current index cap)")
    binary = args.binary.resolve(strict=True)
    records = [record for count in args.files for record in measure(binary, count, args.repeats)]
    print(json.dumps({"schema_version": 1, "platform": platform.platform(),
                      "machine": platform.machine(), "processor": platform.processor(),
                      "python": platform.python_version(), "logical_cpus": os.cpu_count(),
                      "binary_sha256": hashlib.sha256(binary.read_bytes()).hexdigest(),
                      "binary_version": subprocess.check_output([str(binary), "--version"], text=True).strip(),
                      "measurement": "fresh CLI process wall time including startup; Windows process peak working set and kernel+user CPU time; resource metrics null on other OSes",
                      "records": records}, indent=2))


if __name__ == "__main__":
    main()
