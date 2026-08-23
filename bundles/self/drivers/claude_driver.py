#!/usr/bin/env python3
"""forge-driver/v1 adapter for headless Claude Code.

The seat session is ONE `claude -p` invocation in the run's workdir; the
prompt goes on stdin and the seat writes its typed result to the file
named in the input (result-file contract in driver_common — the driver
never repairs anything, decision 0001). Arguments after `--` are
appended to the claude invocation (e.g. `-- --permission-mode
acceptEdits`). FORGE_CLAUDE_BIN overrides the executable (tests use a
shim).
"""

from __future__ import annotations

import json
import os
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import driver_common  # noqa: E402

DRIVER_VERSION = "0.1.0"


def invoke(prompt: str, input_: dict):
    argv = sys.argv[1:]
    extra = argv[argv.index("--") + 1:] if "--" in argv else []
    claude_bin = os.environ.get("FORGE_CLAUDE_BIN", "claude")
    workdir = input_.get("workdir") or os.getcwd()
    command = [claude_bin, "-p", "--output-format", "json", *extra]
    proc = subprocess.run(
        command, input=prompt, capture_output=True, text=True,
        cwd=workdir, check=False,
    )
    session_meta: dict = {}
    for line in reversed(proc.stdout.strip().splitlines() or [""]):
        try:
            parsed = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(parsed, dict):
            session_meta = {
                "session_id": parsed.get("session_id"),
                "num_turns": parsed.get("num_turns"),
                "total_cost_usd": parsed.get("total_cost_usd"),
            }
            break
    return proc.returncode, session_meta, proc.stderr


if __name__ == "__main__":
    driver_common.serve("claude-code", DRIVER_VERSION, invoke)
