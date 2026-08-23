"""Protocol conformance for every Python driver adapter.

One suite, parameterized over the claude, codex, and exec adapters, each
standing in front of a shim CLI. Conformance means: capabilities on
hello; accepted, checkpoint, then exactly one result per start; the
result-file contract honored (missing file fails the attempt, the file's
content forwarded verbatim otherwise); the protocol header on every
message. What holds for one adapter must hold for all — they share
driver_common by construction, and this suite keeps it that way.
"""

import json
import os
import stat
import subprocess
import sys
from pathlib import Path

import pytest

DRIVERS_DIR = Path(__file__).resolve().parents[1] / "bundles/self/drivers"

# A shim that honors the result contract: finds the result path in the
# prompt (stdin or a file argument) and writes a typed result there.
OBEDIENT_SHIM = """#!/usr/bin/env python3
import json, re, sys
if len(sys.argv) > 1 and sys.argv[1] != "-p" and not sys.argv[1].startswith("-"):
    prompt = open(sys.argv[-1]).read() if sys.argv[-1].endswith(".md") else sys.stdin.read()
else:
    prompt = sys.stdin.read()
match = re.search(r"^    (\\S+\\.json)$", prompt, re.MULTILINE)
if match:
    with open(match.group(1), "w") as fh:
        json.dump({"result": "resolved", "notes": "shim did the work"}, fh)
print(json.dumps({"type": "result", "session_id": "s1", "num_turns": 1,
                  "total_cost_usd": 0.0}))
print("session id: deadbeef-1234")
"""

SILENT_SHIM = """#!/usr/bin/env python3
import sys
try:
    sys.stdin.read()
except Exception:
    pass
print("did nothing")
"""


def make_shim(tmp_path: Path, body: str, name: str) -> Path:
    shim = tmp_path / name
    shim.write_text(body)
    shim.chmod(shim.stat().st_mode | stat.S_IEXEC)
    return shim


def driver_invocations(tmp_path: Path, shim: Path):
    """(label, argv, env) for each adapter fronting the given shim."""
    env = os.environ.copy()
    env["FORGE_CLAUDE_BIN"] = str(shim)
    env["FORGE_CODEX_BIN"] = str(shim)
    return [
        ("claude", [sys.executable, str(DRIVERS_DIR / "claude_driver.py")], env),
        ("codex", [sys.executable, str(DRIVERS_DIR / "codex_driver.py")], env),
        (
            "exec-stdin",
            [sys.executable, str(DRIVERS_DIR / "exec_driver.py"), "--", str(shim)],
            env,
        ),
        (
            "exec-promptfile",
            [
                sys.executable,
                str(DRIVERS_DIR / "exec_driver.py"),
                "--",
                str(shim),
                "{prompt_file}",
            ],
            env,
        ),
    ]


def drive(argv, env, tmp_path: Path) -> list[dict]:
    result_path = tmp_path / "results" / "fx.json"
    start_input = {
        "feature": "conformance",
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
        argv,
        input="".join(json.dumps(m) + "\n" for m in messages),
        capture_output=True, text=True, env=env, timeout=60,
    )
    assert proc.returncode == 0, proc.stderr
    out = [json.loads(line) for line in proc.stdout.strip().splitlines()]
    # Clean the result file between parameterized cases.
    if result_path.exists():
        result_path.unlink()
    return out


@pytest.mark.parametrize("case", ["obedient", "silent"])
def test_conformance_across_all_adapters(tmp_path: Path, case: str) -> None:
    shim = make_shim(
        tmp_path, OBEDIENT_SHIM if case == "obedient" else SILENT_SHIM, f"shim-{case}"
    )
    for label, argv, env in driver_invocations(tmp_path, shim):
        out = drive(argv, env, tmp_path)
        kinds = [m["type"] for m in out]
        assert kinds == ["capabilities", "accepted", "checkpoint", "result"], (
            f"{label}: {kinds}"
        )
        for message in out:
            assert message["proto"] == "forge-driver/v1", label
            assert message["msg_id"], label
        result = out[-1]
        if case == "obedient":
            assert result["status"] == "succeeded", f"{label}: {result}"
            assert result["result"] == {
                "result": "resolved",
                "notes": "shim did the work",
            }, label
        else:
            assert result["status"] == "failed", f"{label}: {result}"
            assert "no result file" in result["error"], label


def test_capabilities_name_each_driver(tmp_path: Path) -> None:
    shim = make_shim(tmp_path, OBEDIENT_SHIM, "shim")
    expected = {
        "claude": "claude-code",
        "codex": "codex",
        "exec-stdin": "exec",
        "exec-promptfile": "exec",
    }
    for label, argv, env in driver_invocations(tmp_path, shim):
        out = drive(argv, env, tmp_path)
        assert out[0]["driver"] == expected[label], label


def test_exec_driver_requires_a_template(tmp_path: Path) -> None:
    argv = [sys.executable, str(DRIVERS_DIR / "exec_driver.py")]
    out = drive(argv, os.environ.copy(), tmp_path)
    result = out[-1]
    assert result["status"] == "failed"
    assert "command template" in result["error"]


def test_driver_files_identical_across_bundles() -> None:
    """The verify bundle ships the same drivers as the self bundle; the
    forge-verify review flagged that this parity was untested and could
    drift silently. (The `forge init` embeds are compile-time
    include_str! of the self-bundle sources, so they cannot drift.)"""
    verify_drivers = DRIVERS_DIR.parents[1] / "verify/drivers"
    for name in ["driver_common.py", "claude_driver.py", "codex_driver.py",
                 "exec_driver.py"]:
        ours = (DRIVERS_DIR / name).read_bytes()
        theirs = (verify_drivers / name).read_bytes()
        assert ours == theirs, f"{name} drifted between bundles/self and bundles/verify"
