#!/usr/bin/env python3
"""forge-driver/v1 adapter for headless Claude Code.

Stdlib-only, mirroring the repo's forge-control.py culture. Speaks NDJSON
on stdio: engine->driver on stdin, driver->engine on stdout; everything
diagnostic goes to stderr (the engine captures it as evidence).

The seat session is ONE `claude -p` invocation in the run's workdir. The
seat writes its typed result to the file named in `input.result_path`;
that file is the only result channel. A missing file is a failed attempt
(the engine parks); a schema-invalid file is reported as the attempt's
result and parked by the ENGINE's validation with the raw evidence
attached (decision 0001) — this driver never repairs anything.

Command-line: arguments after `--` are appended to the claude invocation
(e.g. `-- --permission-mode acceptEdits --model opus`).
Environment: FORGE_CLAUDE_BIN overrides the claude executable (tests use
a shim).
"""

from __future__ import annotations

import json
import os
import subprocess
import sys
import uuid

PROTO = "forge-driver/v1"
DRIVER_VERSION = "0.1.0"


def send(body: dict) -> None:
    body = {"proto": PROTO, "msg_id": str(uuid.uuid4()), **body}
    sys.stdout.write(json.dumps(body) + "\n")
    sys.stdout.flush()


def compose_prompt(input_: dict) -> str:
    role = ""
    role_path = input_.get("role_path")
    if role_path and os.path.isfile(role_path):
        with open(role_path, encoding="utf-8") as fh:
            role = fh.read()
    context = json.dumps(input_.get("context") or {}, indent=2, sort_keys=True)
    allowed = ", ".join(input_.get("allowed_results") or [])
    return f"""{role}

---
## Task

Feature: {input_.get('feature')}
Phase: {input_.get('phase')} (you are this phase's only seat)
Working directory: {input_.get('workdir')}

Run context (journal-derived, read-only):
```json
{context}
```

## Result contract — MANDATORY

When your work is finished, write a JSON object to exactly this file:

    {input_.get('result_path')}

with the shape:

    {{"result": "<one of: {allowed}>",
      "inputs": {{ ...optional typed facts for the phase machine... }},
      "notes": "<short human summary of what you did and why>"}}

The file is the ONLY channel the engine reads. Printing the JSON instead
of writing the file counts as producing no result. You never decide the
next phase — the engine's policy table rules on your typed result.
"""


def run_seat(start: dict, claude_args: list[str]) -> None:
    input_ = start.get("input") or {}
    effect_id = start["effect_id"]
    attempt_id = start["attempt_id"]
    send({"type": "accepted", "effect_id": effect_id, "attempt_id": attempt_id,
          "session_ref": None})

    result_path = input_.get("result_path") or ""
    workdir = input_.get("workdir") or os.getcwd()
    os.makedirs(os.path.dirname(result_path) or ".", exist_ok=True)
    claude_bin = os.environ.get("FORGE_CLAUDE_BIN", "claude")
    command = [claude_bin, "-p", "--output-format", "json", *claude_args]

    try:
        proc = subprocess.run(
            command,
            input=compose_prompt(input_),
            capture_output=True,
            text=True,
            cwd=workdir,
            check=False,
        )
    except OSError as exc:
        send({"type": "result", "effect_id": effect_id, "attempt_id": attempt_id,
              "status": "failed", "result": None,
              "error": f"could not invoke {claude_bin}: {exc}"})
        return

    sys.stderr.write(proc.stderr[-4000:])
    session_meta: dict = {}
    for line in reversed(proc.stdout.strip().splitlines() or [""]):
        try:
            parsed = json.loads(line)
            if isinstance(parsed, dict):
                session_meta = parsed
                break
        except json.JSONDecodeError:
            continue
    checkpoint = {
        "step": "claude-session-finished",
        "exit_code": proc.returncode,
        "session_id": session_meta.get("session_id"),
        "num_turns": session_meta.get("num_turns"),
        "total_cost_usd": session_meta.get("total_cost_usd"),
    }
    send({"type": "checkpoint", "effect_id": effect_id, "attempt_id": attempt_id,
          "data": checkpoint})

    if proc.returncode != 0:
        send({"type": "result", "effect_id": effect_id, "attempt_id": attempt_id,
              "status": "failed", "result": None,
              "error": f"claude exited {proc.returncode}"})
        return
    if not os.path.isfile(result_path):
        send({"type": "result", "effect_id": effect_id, "attempt_id": attempt_id,
              "status": "failed", "result": None,
              "error": "seat wrote no result file (the result contract was not met)"})
        return
    try:
        with open(result_path, encoding="utf-8") as fh:
            seat_result = json.load(fh)
    except (OSError, json.JSONDecodeError) as exc:
        # Hand the engine SOMETHING typed-invalid rather than repairing it:
        # the engine parks with the raw evidence (decision 0001).
        seat_result = {"__unparseable_result_file__": str(exc)}
    send({"type": "result", "effect_id": effect_id, "attempt_id": attempt_id,
          "status": "succeeded", "result": seat_result, "error": None})


def main() -> None:
    argv = sys.argv[1:]
    claude_args = argv[argv.index("--") + 1 :] if "--" in argv else []
    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            message = json.loads(line)
        except json.JSONDecodeError:
            continue  # the engine speaks the protocol; ignore noise
        kind = message.get("type")
        if kind == "hello":
            send({"type": "capabilities", "driver": "claude-code",
                  "version": DRIVER_VERSION, "supports": []})
        elif kind == "start":
            run_seat(message, claude_args)
        elif kind == "cancel":
            send({"type": "cancelled", "effect_id": message.get("effect_id", "")})
            return
        elif kind == "shutdown":
            return


if __name__ == "__main__":
    main()
