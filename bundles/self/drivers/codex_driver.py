#!/usr/bin/env python3
"""forge-driver/v1 adapter for headless Codex (`codex exec`).

The prompt goes to codex on stdin; the seat writes its typed result to
the file named in the input (result-file contract in driver_common).
Arguments after `--` are appended to the codex invocation (e.g.
`-- -s workspace-write -m gpt-5.4-codex`). FORGE_CODEX_BIN overrides the
executable (conformance shims use it).
"""

from __future__ import annotations

import os
import re
import subprocess
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import driver_common  # noqa: E402

DRIVER_VERSION = "0.1.0"


def invoke(prompt: str, input_: dict):
    argv = sys.argv[1:]
    extra = argv[argv.index("--") + 1:] if "--" in argv else []
    codex_bin = os.environ.get("FORGE_CODEX_BIN", "codex")
    workdir = input_.get("workdir") or os.getcwd()
    command = [codex_bin, "exec", "-C", workdir, *extra]
    proc = subprocess.run(
        command, input=prompt, capture_output=True, text=True,
        cwd=workdir, check=False,
    )
    session_meta = {}
    match = re.search(
        r"session id:?\s*([0-9a-f-]{8,})", proc.stdout, re.IGNORECASE
    )
    if match:
        session_meta["session_id"] = match.group(1)
    return proc.returncode, session_meta, proc.stderr


if __name__ == "__main__":
    driver_common.serve("codex", DRIVER_VERSION, invoke)
