#!/usr/bin/env python3
"""Isolated same-binary ABBA evaluation; never selected by normal CI."""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import resource
import shlex
import shutil
import signal
import subprocess
import sys
import time

NAMES = ("heap_claude", "heap_codex", "heap_dsh")
KINDS = ("claude-session", "codex-thread", "dsh-session")
HERE = Path(__file__).resolve()
ROOT = HERE.parents[2]
OUT = ROOT / "target/heap-evaluation"


def say(message):
    print(time.strftime("%H:%M:%S UTC", time.gmtime()), message, flush=True)


def save(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(value, indent=2) + "\n")


def toolchain(mode):
    if mode == "coverage":
        return (ROOT / "rust-nightly-version.txt").read_text().strip()
    return re.search(r'channel\s*=\s*"([^"]+)"', (ROOT / "rust-toolchain.toml").read_text())[1]


def coverage_environment():
    env = os.environ.copy()
    if not env.get("CARGO_LLVM_COV_TARGET_DIR"):
        cache = Path(env.get("XDG_CACHE_HOME", str(Path.home() / ".cache"))) / "brokkr-coverage-cache"
        targets = list(cache.glob(toolchain("coverage") + "-*/target"))
        if len(targets) != 1:
            raise RuntimeError(f"expected exactly one coverage cache after the full gate: {targets}")
        env["CARGO_LLVM_COV_TARGET_DIR"] = str(targets[0])
    output = subprocess.check_output(
        ["cargo", "+" + toolchain("coverage"), "llvm-cov", "show-env", "--sh", "--branch"],
        env=env, text=True,
    )
    for line in output.splitlines():
        words = shlex.split(line)
        if len(words) != 2 or words[0] != "export" or "=" not in words[1]:
            raise RuntimeError(f"unreadable coverage environment: {line}")
        key, value = words[1].split("=", 1)
        env[key] = value
    env["CARGO_TARGET_DIR"] = env["CARGO_LLVM_COV_TARGET_DIR"]
    return env


def prepare(mode):
    env = coverage_environment() if mode == "coverage" else os.environ.copy()
    command = ["cargo", "+" + toolchain(mode), "test", "--workspace", "--all-features",
               "--locked", "--no-fail-fast", "--no-run", "--message-format=json"]
    for name in NAMES:
        command += ["--test", name]
    result = subprocess.run(command, env=env, text=True, stdout=subprocess.PIPE, check=True)
    binaries = {}
    for line in result.stdout.splitlines():
        entry = json.loads(line)
        if (entry.get("reason") != "compiler-artifact" or not entry.get("executable")
                or not entry["profile"]["test"]):
            continue
        name = entry["target"]["name"]
        if name not in NAMES or name in binaries or not entry["profile"]["test"]:
            raise RuntimeError(f"unexpected or duplicate test executable: {entry}")
        path = Path(entry["executable"])
        binaries[name] = {"path": str(path), "sha256": hashlib.sha256(path.read_bytes()).hexdigest()}
    if set(binaries) != set(NAMES):
        raise RuntimeError(f"expected all three heap test binaries: {binaries}")
    sysroot = subprocess.check_output(["rustc", "+" + toolchain(mode), "--print", "sysroot"], text=True).strip()
    host = re.search(r"^host: (.+)$", subprocess.check_output(
        ["rustc", "+" + toolchain(mode), "-vV"], text=True), re.MULTILINE)[1]
    target = Path(env.get("CARGO_TARGET_DIR", str(ROOT / "target")))
    libpath = "DYLD_FALLBACK_LIBRARY_PATH" if platform.system() == "Darwin" else "LD_LIBRARY_PATH"
    env[libpath] = os.pathsep.join([str(target / "debug/deps"),
                                  str(Path(sysroot) / "lib/rustlib" / host / "lib"),
                                  env.get(libpath, "")])
    return binaries, env


def worker(args):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.monotonic()
    with Path(args.log).open("w") as log:
        result = subprocess.run([args.binary, "--nocapture", "--test-threads=1"],
                                stdout=log, stderr=subprocess.STDOUT, timeout=900)
    elapsed = time.monotonic() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    output = Path(args.log).read_text(errors="replace")
    heaps = re.findall(r"(?:^|\.\.\. )heap\t([^\t\n]+)\t([0-9]+)$", output, re.MULTILINE)
    summaries = re.findall(r"^test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;", output, re.MULTILINE)
    expected = KINDS[NAMES.index(args.name)]
    valid = result.returncode == 0 and len(summaries) == 1 and len(heaps) == 1 and heaps[0][0] == expected
    value = {"name": args.name, "returncode": result.returncode, "valid": valid,
             "elapsed_seconds": elapsed, "cpu_seconds": after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime,
             "max_rss_bytes": after.ru_maxrss * (1 if platform.system() == "Darwin" else 1024),
             "heap_bytes": int(heaps[0][1]) if len(heaps) == 1 else None}
    save(Path(args.record), value)
    return 0 if valid else 1


def group(binaries, env, parallel, label, micro=False):
    directory = OUT / label
    directory.mkdir(parents=True, exist_ok=True)
    env = env.copy()
    if micro and "LLVM_PROFILE_FILE" in env:
        env["LLVM_PROFILE_FILE"] = str(directory / "heap-%p-%m.profraw")
    processes = []
    start = time.monotonic()
    say(f"heap group {label}: {'parallel' if parallel else 'sequential'}")
    try:
        for name in NAMES:
            if hashlib.sha256(Path(binaries[name]["path"]).read_bytes()).hexdigest() != binaries[name]["sha256"]:
                raise RuntimeError(f"test executable changed during evaluation: {name}")
            command = [sys.executable, str(HERE), "worker", "--binary", binaries[name]["path"],
                       "--name", name, "--log", str(directory / (name + ".log")),
                       "--record", str(directory / (name + ".json"))]
            process = subprocess.Popen(command, env=env, start_new_session=True)
            processes.append(process)
            if not parallel:
                process.wait(timeout=920)
        for process in processes:
            process.wait(timeout=920)
    finally:
        for process in processes:
            if process.poll() is None:
                os.killpg(process.pid, signal.SIGTERM)
                process.wait(timeout=10)
    elapsed = time.monotonic() - start
    tests = [json.loads((directory / (name + ".json")).read_text()) for name in NAMES]
    valid = all(test["valid"] for test in tests) and all(process.returncode == 0 for process in processes)
    value = {"label": label, "parallel": parallel, "elapsed_seconds": elapsed,
             "cpu_seconds": sum(test["cpu_seconds"] for test in tests), "valid": valid, "tests": tests}
    save(directory / "result.json", value)
    say(f"heap group {label}: {elapsed:.2f}s; valid={valid}")
    return value


def suite(mode, parallel, label):
    command = ["bash", "scripts/coverage-exact.sh"] if mode == "coverage" else ["bash", "scripts/test-shards.sh", "test"]
    if parallel:
        command.append("--parallel-heaps")
    say(f"full suite {label}: starting")
    start = time.monotonic()
    with (OUT / (label + ".log")).open("w") as log:
        run = subprocess.run(command, stdout=log, stderr=subprocess.STDOUT, timeout=2400)
    value = {"label": label, "parallel": parallel, "returncode": run.returncode,
             "elapsed_seconds": time.monotonic() - start}
    if mode == "coverage" and (ROOT / "target/coverage/coverage-exact.json").exists():
        report = json.loads((ROOT / "target/coverage/coverage-exact.json").read_text())
        value["coverage_totals"] = report["data"][0]["totals"]
        save(OUT / (label + "-coverage.json"), report)
    save(OUT / (label + ".json"), value)
    say(f"full suite {label}: {value['elapsed_seconds']:.2f}s; exit={run.returncode}")
    for line in (OUT / (label + ".log")).read_text(errors="replace").splitlines():
        if re.search(r"test shards: .*: exit \d+ after \d+s$|test result: FAILED|panicked at|coverage:.*(100|refus)", line):
            print(line[:500], flush=True)
    return value


def evaluate(mode):
    if OUT.is_symlink():
        raise RuntimeError("the evaluation output directory must not be a symlink")
    if OUT.exists():
        shutil.rmtree(OUT)
    OUT.mkdir(parents=True, exist_ok=True)
    info = {"commit": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
            "mode": mode, "platform": platform.platform(), "toolchain": toolchain(mode),
            "cpu_affinity": sorted(os.sched_getaffinity(0)) if hasattr(os, "sched_getaffinity") else None,
            "test_threads": os.environ.get("RUST_TEST_THREADS"), "runner": os.environ.get("RUNNER_NAME")}
    save(OUT / "environment.json", info)
    if mode == "test":
        say("warming the complete test build before timing either full suite")
        with (OUT / "warmup-build.log").open("w") as log:
            subprocess.run(["cargo", "+" + toolchain(mode), "test", "--workspace", "--all-features",
                            "--locked", "--no-fail-fast", "--no-run"],
                           stdout=log, stderr=subprocess.STDOUT, check=True, timeout=2400)
    # The full serial suite warms all targets before the paired same-binary comparison.
    baseline = suite(mode, False, "full-sequential")
    binaries, env = prepare(mode)
    save(OUT / "binaries.json", binaries)
    samples = []
    for index, parallel in enumerate([False, True, True, False], 1):
        samples.append(group(binaries, env, parallel, f"sample-{index}", micro=True))
    heap_sets = [{test["name"]: test["heap_bytes"] for test in sample["tests"]} for sample in samples]
    heap_equal = all(values == heap_sets[0] for values in heap_sets)
    candidate = suite(mode, True, "full-parallel")
    result = {"environment": info, "baseline": baseline, "candidate": candidate,
              "samples": samples, "identical_heap_bytes": heap_equal}
    result["valid"] = (baseline["returncode"] == candidate["returncode"] == 0
                       and all(sample["valid"] for sample in samples) and heap_equal)
    save(OUT / "result.json", result)
    if (summary := os.environ.get("GITHUB_STEP_SUMMARY")):
        with open(summary, "a") as file:
            file.write(f"Evaluation `{info['commit']}` on {info['runner']}: valid={result['valid']}\n\n")
            for sample in samples:
                file.write(f"- {sample['label']} {'parallel' if sample['parallel'] else 'sequential'}: {sample['elapsed_seconds']:.2f}s\n")
            file.write(f"\nFull suite: {baseline['elapsed_seconds']:.2f}s sequential; {candidate['elapsed_seconds']:.2f}s parallel. Identical heap bytes: {heap_equal}.\n")
    return 0 if result["valid"] else 1


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["evaluate", "stage", "worker"])
    parser.add_argument("--mode", choices=["test", "coverage"])
    for option in ["binary", "name", "log", "record"]:
        parser.add_argument("--" + option)
    args = parser.parse_args()
    os.chdir(ROOT)
    if args.command == "worker":
        return worker(args)
    if args.mode is None:
        parser.error("--mode is required")
    if args.command == "evaluate":
        return evaluate(args.mode)
    binaries, env = prepare(args.mode)
    result = group(binaries, env, True, "full-parallel-heap")
    for name in NAMES:
        print((OUT / "full-parallel-heap" / (name + ".log")).read_text(), flush=True)
    return 0 if result["valid"] else 1


if __name__ == "__main__":
    sys.exit(main())
