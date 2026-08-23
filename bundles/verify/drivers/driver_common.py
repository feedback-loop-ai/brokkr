#!/usr/bin/env python3
"""Shared forge-driver/v1 plumbing for the Python driver adapters.

Stdlib-only. An adapter supplies one `invoke(prompt, input_)` callable
that runs its agent CLI to completion and returns
`(exit_code, session_meta_dict, stderr_text)`; everything else — the
NDJSON protocol loop, prompt composition, the result-file contract, the
checkpoint — is identical across adapters and lives here.

The result-file contract (decision 0001 discipline): the seat writes its
typed result to `input.result_path`; that file is the only result
channel. A missing file fails the attempt; an unparseable file is
forwarded as-is so the ENGINE parks with the raw evidence. Drivers never
repair anything.
"""

from __future__ import annotations

import json
import os
import sys
import uuid

PROTO = "forge-driver/v1"


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


def run_seat(start: dict, invoke, checkpoint_step: str) -> None:
    input_ = start.get("input") or {}
    effect_id = start["effect_id"]
    attempt_id = start["attempt_id"]
    send({"type": "accepted", "effect_id": effect_id, "attempt_id": attempt_id,
          "session_ref": None})

    result_path = input_.get("result_path") or ""
    os.makedirs(os.path.dirname(result_path) or ".", exist_ok=True)

    try:
        exit_code, session_meta, stderr_text = invoke(compose_prompt(input_), input_)
    except OSError as exc:
        send({"type": "result", "effect_id": effect_id, "attempt_id": attempt_id,
              "status": "failed", "result": None,
              "error": f"could not invoke the agent CLI: {exc}"})
        return

    sys.stderr.write(stderr_text[-4000:])
    checkpoint = {"step": checkpoint_step, "exit_code": exit_code, **session_meta}
    send({"type": "checkpoint", "effect_id": effect_id, "attempt_id": attempt_id,
          "data": checkpoint})

    if exit_code != 0:
        send({"type": "result", "effect_id": effect_id, "attempt_id": attempt_id,
              "status": "failed", "result": None,
              "error": f"agent CLI exited {exit_code}"})
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
        # Typed-invalid on purpose: the engine parks with raw evidence
        # (decision 0001); the driver repairs nothing.
        seat_result = {"__unparseable_result_file__": str(exc)}
    send({"type": "result", "effect_id": effect_id, "attempt_id": attempt_id,
          "status": "succeeded", "result": seat_result, "error": None})


def serve(driver_name: str, driver_version: str, invoke,
          supports: list[str] | None = None) -> None:
    """The adapter main loop: hello/capabilities, start->run_seat,
    cancel/shutdown. `invoke(prompt, input_)` runs the agent CLI."""
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
            send({"type": "capabilities", "driver": driver_name,
                  "version": driver_version, "supports": supports or []})
        elif kind == "start":
            run_seat(message, invoke, f"{driver_name}-session-finished")
        elif kind == "cancel":
            send({"type": "cancelled", "effect_id": message.get("effect_id", "")})
            return
        elif kind == "shutdown":
            return
