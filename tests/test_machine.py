"""Property and behavior tests for the pure phase-machine core.

The imported table under policy/phase-machine.json is the real production
table from the the origin workspace workspace — these tests are the beginning of the
parity harness.
"""

import itertools
import json
from pathlib import Path

import pytest

from forge.machine import Machine, NoRule, Ruling

TABLE = json.loads(
    (Path(__file__).resolve().parents[1] / "policy" / "phase-machine.json").read_text()
)


@pytest.fixture(scope="module")
def machine() -> Machine:
    return Machine.from_table(TABLE)


def test_table_loads_and_validates(machine: Machine) -> None:
    assert machine.initial == "intake"
    assert set(machine.terminal) == {"done", "stop"}


def test_security_hold_never_ships(machine: Machine) -> None:
    ruling = machine.evaluate("review", "security-hold", {})
    assert isinstance(ruling, Ruling)
    assert ruling.next_phase == "stop"
    assert ruling.severity == "hard"


def test_first_match_wins_deny_before_allow(machine: Machine) -> None:
    # Two consecutive architect failures must hit the hard stop, not the retry.
    hard = machine.evaluate("architecture", "failed", {"consecutive_failures": 2})
    retry = machine.evaluate("architecture", "failed", {"consecutive_failures": 1})
    assert isinstance(hard, Ruling) and hard.rule_id == "ARCH-FAIL-TWICE"
    assert isinstance(retry, Ruling) and retry.rule_id == "ARCH-RETRY"


def test_security_residual_never_takes_debt_path(machine: Machine) -> None:
    ruling = machine.evaluate(
        "review", "residual", {"has_security_residual": True, "max_residual_severity": "low"}
    )
    assert isinstance(ruling, Ruling)
    assert ruling.next_phase == "stop"


def test_residual_above_medium_stops(machine: Machine) -> None:
    ruling = machine.evaluate("review", "residual", {"max_residual_severity": "high"})
    assert isinstance(ruling, Ruling)
    assert ruling.next_phase == "stop"


def test_unknown_severity_fails_closed(machine: Machine) -> None:
    ruling = machine.evaluate("review", "residual", {"max_residual_severity": "banana"})
    assert isinstance(ruling, Ruling)
    assert ruling.next_phase == "stop"


def test_unmatched_pair_parks_not_guesses(machine: Machine) -> None:
    ruling = machine.evaluate("review", "totally-novel-result", {})
    assert isinstance(ruling, NoRule)


def test_unknown_phase_parks(machine: Machine) -> None:
    assert isinstance(machine.evaluate("nonexistent", "complete", {}), NoRule)


KNOWN_RESULTS = sorted({r["result"] for r in TABLE["rules"]})
BOOL_INPUTS = [
    "skip_verify",
    "high_risk_uncovered",
    "has_security_residual",
    "fixes_applied",
    "drift_detected",
    "dirty_worktrees",
]


def _consistent(result: str, inputs: dict) -> bool:
    """The result taxonomy carries residual information: a review that found
    residuals MUST report 'residual' (or 'clean-unverified'), never 'clean'.
    An executor emitting 'clean' alongside residual evidence violates its
    result schema and is parked by decision 0001 before the table is ever
    consulted. The first run of this sweep without this filter flagged
    exactly those impossible combinations — documenting that shippability
    safety rests jointly on the table AND on result-schema validation."""
    if result == "clean":
        return (
            not inputs["has_security_residual"]
            and inputs["max_residual_severity"] == "none"
        )
    return True


def test_no_path_to_ship_without_shippable_review(machine: Machine) -> None:
    """Exhaustive sweep over schema-consistent inputs: from 'review', only
    outcomes below the severity bar may move forward, and never with a
    security residual."""
    for result in KNOWN_RESULTS:
        for sev in ["none", "low", "medium", "high", "critical"]:
            for flags in itertools.product([False, True], repeat=len(BOOL_INPUTS)):
                inputs = dict(zip(BOOL_INPUTS, flags))
                inputs["max_residual_severity"] = sev
                if not _consistent(result, inputs):
                    continue
                ruling = machine.evaluate("review", result, inputs)
                if isinstance(ruling, NoRule):
                    continue
                if ruling.next_phase in ("regression", "ship"):
                    assert not inputs["has_security_residual"], ruling.rule_id
                    assert sev in ("none", "low", "medium"), ruling.rule_id


def test_every_nonterminal_phase_has_at_least_one_rule(machine: Machine) -> None:
    covered = {r["from"] for r in TABLE["rules"]}
    for phase in machine.phases:
        if phase in machine.terminal:
            continue
        assert phase in covered, f"phase {phase} has no outgoing rules"
