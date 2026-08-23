#!/usr/bin/env python3
"""Generic forge-driver/v1 adapter: run ANY headless agent CLI.

The command template comes after `--` in the driver's own argv (bundle
data). Placeholders substituted per attempt:

    {prompt_file}   path to a file holding the composed prompt
    {workdir}       the run's working directory

With no `{prompt_file}` placeholder the prompt is piped to the command's
stdin instead. The seat writes its typed result to the file named in the
input (result-file contract in driver_common).

This is the adapter for template-shaped harnesses — dsh/Surface profiles
(`-- dsh --profile surface ...`), remote execution over ssh
(`-- ssh host agent-cli ...`; the protocol is pure stdio, so ssh carries
it unchanged) — and any other CLI that can read a prompt and leave a
file behind. FORGE_EXEC_NAME names the driver in capabilities
(default "exec").
"""

from __future__ import annotations

import os
import subprocess
import sys
import tempfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import driver_common  # noqa: E402

DRIVER_VERSION = "0.1.0"


def invoke(prompt: str, input_: dict):
    argv = sys.argv[1:]
    if "--" not in argv or not argv[argv.index("--") + 1:]:
        raise OSError("exec driver needs a command template after '--'")
    template = argv[argv.index("--") + 1:]
    workdir = input_.get("workdir") or os.getcwd()

    prompt_file = None
    if any("{prompt_file}" in part for part in template):
        fd, prompt_file = tempfile.mkstemp(prefix="forge-prompt-", suffix=".md")
        with os.fdopen(fd, "w", encoding="utf-8") as fh:
            fh.write(prompt)
    command = [
        part.replace("{workdir}", workdir).replace("{prompt_file}", prompt_file or "")
        for part in template
    ]
    try:
        proc = subprocess.run(
            command,
            input=None if prompt_file else prompt,
            capture_output=True, text=True, cwd=workdir, check=False,
        )
    finally:
        if prompt_file:
            try:
                os.unlink(prompt_file)
            except OSError:
                pass
    return proc.returncode, {}, proc.stderr


if __name__ == "__main__":
    driver_common.serve(os.environ.get("FORGE_EXEC_NAME", "exec"), DRIVER_VERSION, invoke)
