"""Protocol smoke tests for the headless Claude Code driver.

A shim stands in for the claude CLI (FORGE_CLAUDE_BIN): it consumes the
prompt, optionally honors the result contract by writing the result
file, and prints the session JSON. This proves the driver's protocol
behavior without spending a model session.
"""

import json
import os
import stat
import subprocess
import sys
from pathlib import Path

import pytest

DRIVER = Path(__file__).resolve().parents[1] / "bundles/self/drivers/claude_driver.py"

OBEDIENT_SHIM = """#!/usr/bin/env python3
import json, re, sys
prompt = sys.stdin.read()
match = re.search(r"^    (\\S+\\.json)$", prompt, re.MULTILINE)
if match:
    with open(match.group(1), "w") as fh:
        json.dump({"result": "resolved", "notes": "shim did the work"}, fh)
print(json.dumps({"type": "result", "session_id": "s1", "num_turns": 1,
                  "total_cost_usd": 0.0}))
"""

SILENT_SHIM = """#!/usr/bin/env python3
import sys
sys.stdin.read()
print("{}")
"""


def make_shim(tmp_path: Path, body: str) -> Path:
    shim = tmp_path / "claude-shim"
    shim.write_text(body)
    shim.chmod(shim.stat().st_mode | stat.S_IEXEC)
    return shim


def drive(tmp_path: Path, shim_body: str) -> list[dict]:
    shim = make_shim(tmp_path, shim_body)
    result_path = tmp_path / "results" / "fx.json"
    start_input = {
        "feature": "smoke",
        "phase": "intake",
        "seat": "intake",
        "role_path": str(tmp_path / "missing-role.md"),
        "workdir": str(tmp_path),
        "result_path": str(result_path),
        "allowed_results": ["resolved"],
        "context": {},
    }
    messages = [
        {"proto": "forge-driver/v1", "msg_id": "m1", "type": "hello",
         "engine_version": "test"},
        {"proto": "forge-driver/v1", "msg_id": "m2", "type": "start",
         "effect_id": "fx", "attempt_id": "a1", "seat": "intake",
         "input": start_input},
        {"proto": "forge-driver/v1", "msg_id": "m3", "type": "shutdown"},
    ]
    proc = subprocess.run(
        [sys.executable, str(DRIVER)],
        input="".join(json.dumps(m) + "\n" for m in messages),
        capture_output=True,
        text=True,
        env={**os.environ, "FORGE_CLAUDE_BIN": str(shim)},
        timeout=60,
    )
    assert proc.returncode == 0, proc.stderr
    return [json.loads(line) for line in proc.stdout.strip().splitlines()]


def test_obedient_seat_result_reaches_the_engine(tmp_path: Path) -> None:
    out = drive(tmp_path, OBEDIENT_SHIM)
    kinds = [m["type"] for m in out]
    assert kinds == ["capabilities", "accepted", "checkpoint", "result"]
    result = out[-1]
    assert result["status"] == "succeeded"
    assert result["result"] == {"result": "resolved", "notes": "shim did the work"}
    assert out[2]["data"]["session_id"] == "s1"


def test_missing_result_file_fails_the_attempt(tmp_path: Path) -> None:
    out = drive(tmp_path, SILENT_SHIM)
    result = out[-1]
    assert result["type"] == "result"
    assert result["status"] == "failed"
    assert "no result file" in result["error"]


@pytest.mark.parametrize("body", [OBEDIENT_SHIM])
def test_every_message_carries_the_protocol_header(tmp_path: Path, body: str) -> None:
    for message in drive(tmp_path, body):
        assert message["proto"] == "forge-driver/v1"
        assert message["msg_id"]
