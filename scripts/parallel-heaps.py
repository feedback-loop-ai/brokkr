#!/usr/bin/env python3
"""Run only the three heap-budget binaries together, joining every child.

Cargo resolves the executables with the suite's complete feature selection.
Coverage uses the gate's current target and profile names; no parallel Cargo
commands, cleans, or reports can race with the three instrumented processes.
"""

import argparse
import json
import os
from pathlib import Path
import platform
import re
import shlex
import signal
import subprocess
import sys
import tempfile
import time

NAMES = ("heap_claude", "heap_codex", "heap_dsh")
KINDS = dict(zip(NAMES, ("claude-session", "codex-thread", "dsh-session")))


def prepare(nightly):
    env = os.environ.copy()
    toolchain = ["+" + nightly] if nightly else []
    if nightly:
        if not env.get("CARGO_LLVM_COV_TARGET_DIR") or not env.get("LLVM_PROFILE_FILE_NAME"):
            raise RuntimeError("coverage requires the exact gate's target and profile name")
        output = subprocess.check_output(
            ["cargo", *toolchain, "llvm-cov", "show-env", "--sh", "--branch"],
            env=env, text=True,
        )
        for line in output.splitlines():
            words = shlex.split(line)
            if len(words) != 2 or words[0] != "export" or "=" not in words[1]:
                raise RuntimeError("unreadable cargo-llvm-cov environment")
            key, value = words[1].split("=", 1)
            env[key] = value
        profile = env.get("LLVM_PROFILE_FILE", "")
        expected = Path(env["CARGO_LLVM_COV_TARGET_DIR"]) / env["LLVM_PROFILE_FILE_NAME"]
        if profile != str(expected) or "%p" not in profile or "%m" not in profile:
            raise RuntimeError("coverage must preserve the gate's process-specific profile path")
        env["CARGO_TARGET_DIR"] = env["CARGO_LLVM_COV_TARGET_DIR"]
    command = ["cargo", *toolchain, "test", "--workspace", "--all-features", "--locked",
               "--no-fail-fast", "--no-run", "--message-format=json"]
    for name in NAMES:
        command += ["--test", name]
    output = subprocess.check_output(command, env=env, text=True)
    binaries = {}
    for line in output.splitlines():
        entry = json.loads(line)
        if (entry.get("reason") != "compiler-artifact" or not entry.get("executable")
                or not entry["profile"]["test"]):
            continue
        name = entry["target"]["name"]
        if name not in NAMES or name in binaries:
            raise RuntimeError(f"unexpected or duplicate heap test executable: {name}")
        binaries[name] = entry["executable"]
    if set(binaries) != set(NAMES):
        raise RuntimeError("Cargo did not report all three heap test executables")
    sysroot = subprocess.check_output(["rustc", *toolchain, "--print", "sysroot"], text=True).strip()
    version = subprocess.check_output(["rustc", *toolchain, "-vV"], text=True)
    host = re.search(r"^host: (.+)$", version, re.MULTILINE)
    if host is None:
        raise RuntimeError("rustc did not report its host")
    # Match Cargo's dynamic-library search path when executing the binaries
    # directly, including the instrumented target's deps and toolchain std.
    libpath = "DYLD_FALLBACK_LIBRARY_PATH" if platform.system() == "Darwin" else "LD_LIBRARY_PATH"
    paths = list(dict.fromkeys(str(Path(binary).parent) for binary in binaries.values()))
    paths.append(str(Path(sysroot) / "lib/rustlib" / host[1] / "lib"))
    if env.get(libpath):
        paths.append(env[libpath])
    env[libpath] = os.pathsep.join(paths)
    return binaries, env


def valid_output(name, output):
    summaries = [line for line in output.splitlines() if line.startswith("test result:")]
    heaps = re.findall(r"(?:^|\.\.\. )heap\t([^\t\n]+)\t([0-9]+)$", output, re.MULTILINE)
    return (len(summaries) == 1 and re.fullmatch(
        r"test result: ok\. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in [0-9.]+s",
        summaries[0]) is not None and len(heaps) == 1 and heaps[0][0] == KINDS[name])


def stop(processes):
    # Send to every group before waiting, then escalate together. A failed
    # spawn, timeout, SIGINT or SIGTERM cannot leave a heap running beside
    # the next shard. The heap tests do not spawn children themselves.
    handlers = {sig: signal.signal(sig, signal.SIG_IGN)
                for sig in (signal.SIGINT, signal.SIGTERM)}
    try:
        for process in processes:
            if process.poll() is None:
                try:
                    os.killpg(process.pid, signal.SIGTERM)
                except ProcessLookupError:
                    pass  # The child exited between poll and killpg.
        deadline = time.monotonic() + 5
        for process in processes:
            try:
                process.wait(timeout=max(0, deadline - time.monotonic()))
            except subprocess.TimeoutExpired:
                try:
                    os.killpg(process.pid, signal.SIGKILL)
                except ProcessLookupError:
                    pass
        for process in processes:
            process.wait()
    finally:
        for sig, handler in handlers.items():
            signal.signal(sig, handler)


def run(binaries, env, timeout=900):
    processes = []
    started = time.monotonic()
    with tempfile.TemporaryDirectory(prefix="brokkr-parallel-heaps-") as directory:
        try:
            for name in NAMES:
                with open(Path(directory) / name, "wb") as log:
                    processes.append(subprocess.Popen(
                        [binaries[name], "--nocapture", "--test-threads=1"], env=env,
                        stdout=log, stderr=subprocess.STDOUT, start_new_session=True,
                    ))
            for process in processes:
                process.wait(timeout=max(0, timeout - (time.monotonic() - started)))
        finally:
            stop(processes)
            valid = len(processes) == len(NAMES)
            for name, process in zip(NAMES, processes):
                output = (Path(directory) / name).read_text(errors="replace")
                print(f"==> {name}: exit {process.returncode}\n{output}", flush=True)
                if process.returncode != 0 or not valid_output(name, output):
                    print(f"parallel heaps: {name} did not pass exactly one heap measurement", file=sys.stderr)
                    valid = False
    print(f"parallel heaps: joined all three in {time.monotonic() - started:.2f}s", flush=True)
    return 0 if valid else 1


class Interrupted(Exception):
    def __init__(self, signum):
        self.signum = signum


def interrupted(signum, _frame):
    # Repeated cancellation must not interrupt the join in the finally block.
    signal.signal(signal.SIGINT, signal.SIG_IGN)
    signal.signal(signal.SIGTERM, signal.SIG_IGN)
    raise Interrupted(signum)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--nightly", help="coverage toolchain; only inside coverage-exact.sh")
    args = parser.parse_args()
    signal.signal(signal.SIGINT, interrupted)
    signal.signal(signal.SIGTERM, interrupted)
    try:
        binaries, env = prepare(args.nightly)
        return run(binaries, env)
    except Interrupted as error:
        return 128 + error.signum
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(f"parallel heaps: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
