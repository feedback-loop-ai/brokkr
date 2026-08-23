"""Property, behavior, and lint tests for the pure phase-machine core.

The imported table under policy/phase-machine.json is the real production
table from the the origin workspace workspace — these tests are the beginning of the
parity harness. The loader-rejection tests double as the first slice of the
constitutional policy lint (docs/extension-model.md): a malformed table
refuses to LOAD; it never degrades into rules that silently stop matching.
"""

import itertools
import json
from pathlib import Path

import pytest

from forge.machine import (
    BOOLEAN_INPUTS,
    Machine,
    NoRule,
    PolicyError,
    RULING_SEVERITIES,
    Ruling,
    SEVERITY_ORDER,
)

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


def test_info_severity_ranks_below_medium(machine: Machine) -> None:
    """'info' is a legitimate production severity (forge-control.py
    SEVERITY_ORDER, the phase-advance CLI accepts it). The extraction
    dropped it, which would have misread valid results as unknown."""
    ruling = machine.evaluate("review", "residual", {"max_residual_severity": "info"})
    assert isinstance(ruling, Ruling)
    assert ruling.rule_id == "REVIEW-RESIDUAL-OK"
    assert ruling.next_phase == "regression"


def test_unknown_severity_parks_not_guesses(machine: Machine) -> None:
    """An unreadable input is invalid control-plane data. The referee raised
    (forge-control.py severity_above); the pure core parks. It is never
    ranked above known severities, coerced, or guessed at (decision 0004)."""
    ruling = machine.evaluate("review", "residual", {"max_residual_severity": "banana"})
    assert isinstance(ruling, NoRule)
    assert "banana" in (ruling.problem or "")


def test_unmatched_pair_parks_not_guesses(machine: Machine) -> None:
    ruling = machine.evaluate("review", "totally-novel-result", {})
    assert isinstance(ruling, NoRule)
    assert ruling.problem is None  # no-match park, not an unreadable input


def test_unknown_phase_parks(machine: Machine) -> None:
    ruling = machine.evaluate("nonexistent", "complete", {})
    assert isinstance(ruling, NoRule)
    assert ruling.problem is not None


def test_ruling_severity_uses_journal_vocabulary(machine: Machine) -> None:
    """A rule that names no severity rules at 'normal' — the token the
    forge.phase-event/v1 schema enumerates and production journals record.
    ('none' belongs to the residual axis and is not a ruling severity.)"""
    ruling = machine.evaluate("intake", "resolved", {})
    assert isinstance(ruling, Ruling)
    assert ruling.severity == "normal"
    assert ruling.severity in RULING_SEVERITIES


# ---------------------------------------------------------------------------
# Absent- and mistyped-input semantics (decision 0004)
# ---------------------------------------------------------------------------


def test_absent_fixes_applied_never_skips_regression(machine: Machine) -> None:
    """`when {fixes_applied: false}` is the skip-V' shortcut. Absence is not
    a claim that no fixes were applied: the shortcut requires the explicit
    false; absence takes the regression path. (Deliberately stricter than
    the referee, whose bool() coercion let omission skip regression.)"""
    absent = machine.evaluate("review", "clean", {})
    explicit = machine.evaluate("review", "clean", {"fixes_applied": False})
    applied = machine.evaluate("review", "clean", {"fixes_applied": True})
    assert isinstance(absent, Ruling) and absent.rule_id == "REVIEW-CLEAN"
    assert absent.next_phase == "regression"
    assert isinstance(explicit, Ruling) and explicit.rule_id == "REVIEW-CLEAN-NO-FIXES"
    assert explicit.next_phase == "ship"
    assert isinstance(applied, Ruling) and applied.rule_id == "REVIEW-CLEAN"


def test_absent_flags_fail_safe(machine: Machine) -> None:
    # No skip_verify flag means it was not given: env-degraded is a hard stop.
    degraded = machine.evaluate("verify", "env-degraded", {})
    assert isinstance(degraded, Ruling) and degraded.rule_id == "VERIFY-ENV-DEGRADED"
    assert degraded.next_phase == "stop"
    # No failure counter yet satisfies no _gte threshold: first failure retries.
    first = machine.evaluate("architecture", "failed", {})
    assert isinstance(first, Ruling) and first.rule_id == "ARCH-RETRY"


@pytest.mark.parametrize(
    ("phase", "result", "inputs"),
    [
        ("review", "clean", {"fixes_applied": "no"}),
        ("review", "residual", {"has_security_residual": "yes"}),
        ("architecture", "failed", {"consecutive_failures": "2"}),
        ("architecture", "failed", {"consecutive_failures": True}),
        ("review", "residual", {"max_residual_severity": 3}),
    ],
)
def test_mistyped_condition_inputs_park(
    machine: Machine, phase: str, result: str, inputs: dict
) -> None:
    """A present input the vocabulary cannot read is never coerced (the
    referee's bool()) and never ranked: the evaluation parks with evidence."""
    ruling = machine.evaluate(phase, result, inputs)
    assert isinstance(ruling, NoRule)
    assert ruling.problem


# ---------------------------------------------------------------------------
# Loader rejection: the closed condition vocabulary (decision 0004)
# ---------------------------------------------------------------------------


def _rule(**overrides):
    rule = {"id": "R", "from": "a", "result": "ok", "next": "b", "reason": "r"}
    rule.update(overrides)
    return rule


def _minimal_table(rules):
    return {
        "phases": ["a", "b", "stop"],
        "initial": "a",
        "terminal": ["stop"],
        "rules": rules,
    }


@pytest.mark.parametrize(
    "when",
    [
        {"has_security_residualz": True},  # the recorded typo incident
        {"skip_verify": "yes"},  # boolean condition, non-bool threshold
        {"consecutive_failures_gte": "2"},  # counter threshold not numeric
        {"consecutive_failures_gte": True},  # bool is not a count
        {"retries_gte": 2},  # undeclared counter
        {"max_residual_severity_above": "banana"},  # unknown severity threshold
        {"severity_above": "medium"},  # undeclared severity axis
    ],
)
def test_loader_rejects_malformed_conditions(when: dict) -> None:
    """forge-control.py learned this in production: 'a typo'd key silently
    disabled the hard stop it guarded, with no error and no test failure'.
    The pure core refuses one step earlier than the referee — at load, not
    at evaluation — so a dead deny rule cannot even be installed."""
    with pytest.raises(PolicyError):
        Machine.from_table(_minimal_table([_rule(when=when)]))


def test_loader_rejects_unknown_ruling_severity() -> None:
    with pytest.raises(PolicyError):
        Machine.from_table(_minimal_table([_rule(severity="critical")]))


def test_loader_rejects_rule_shadowed_by_unconditional() -> None:
    rules = [
        _rule(id="R1"),
        _rule(id="R2", when={"skip_verify": True}),
    ]
    with pytest.raises(PolicyError):
        Machine.from_table(_minimal_table(rules))


def test_loader_rejects_rules_leaving_terminal() -> None:
    with pytest.raises(PolicyError):
        Machine.from_table(_minimal_table([_rule(**{"from": "stop"})]))


def test_loader_rejects_malformed_requires_artifacts() -> None:
    # A bare string is iterable-of-strings and would slip a naive check.
    with pytest.raises(PolicyError):
        Machine.from_table(_minimal_table([_rule(requires_artifacts="spec.md")]))


# ---------------------------------------------------------------------------
# Table-wide lint: the production table's own invariants
# ---------------------------------------------------------------------------


def test_hard_rulings_always_stop() -> None:
    for rule in TABLE["rules"]:
        if rule.get("severity") == "hard":
            assert rule["next"] == "stop", rule["id"]


def test_flagged_rulings_never_terminal() -> None:
    terminal = set(TABLE["terminal"])
    for rule in TABLE["rules"]:
        if rule.get("severity") == "flagged":
            assert rule["next"] not in terminal, rule["id"]


def test_security_hold_is_a_hard_stop_wherever_it_appears() -> None:
    holds = [r for r in TABLE["rules"] if r["result"] == "security-hold"]
    assert holds, "the table must rule on security-hold"
    for rule in holds:
        assert rule["next"] == "stop", rule["id"]
        assert rule.get("severity") == "hard", rule["id"]


def _edges(rules):
    edges: dict[str, set[str]] = {}
    for rule in rules:
        edges.setdefault(rule["from"], set()).add(rule["next"])
    return edges


def _reachable(start: str, edges: dict[str, set[str]]) -> set[str]:
    seen, frontier = {start}, [start]
    while frontier:
        node = frontier.pop()
        for nxt in edges.get(node, ()):
            if nxt not in seen:
                seen.add(nxt)
                frontier.append(nxt)
    return seen


def test_every_phase_reachable_from_initial(machine: Machine) -> None:
    assert _reachable(machine.initial, _edges(TABLE["rules"])) == set(machine.phases)


def test_every_phase_reaches_a_terminal(machine: Machine) -> None:
    edges = _edges(TABLE["rules"])
    for phase in machine.phases:
        assert set(machine.terminal) & _reachable(phase, edges), phase


def test_ship_requires_passing_through_review(machine: Machine) -> None:
    """The extension model's protected invariant, asserted over the graph:
    remove the review node and nothing downstream of it stays reachable —
    no table path reaches ship or done without going through review."""
    pruned = _edges(
        [r for r in TABLE["rules"] if "review" not in (r["from"], r["next"])]
    )
    reachable = _reachable(machine.initial, pruned)
    assert "ship" not in reachable
    assert "done" not in reachable


def test_every_nonterminal_phase_has_at_least_one_rule(machine: Machine) -> None:
    covered = {r["from"] for r in TABLE["rules"]}
    for phase in machine.phases:
        if phase in machine.terminal:
            continue
        assert phase in covered, f"phase {phase} has no outgoing rules"


# ---------------------------------------------------------------------------
# Behavioral sweeps over the input grid
# ---------------------------------------------------------------------------

KNOWN_RESULTS = sorted({r["result"] for r in TABLE["rules"]})
BOOL_INPUTS = sorted(BOOLEAN_INPUTS)


def _input_grid():
    for sev in SEVERITY_ORDER:
        for flags in itertools.product([False, True], repeat=len(BOOL_INPUTS)):
            inputs = dict(zip(BOOL_INPUTS, flags))
            inputs["max_residual_severity"] = sev
            yield inputs


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
    outcomes at or below the debt bar may move forward, and never with a
    security residual."""
    for result in KNOWN_RESULTS:
        for inputs in _input_grid():
            if not _consistent(result, inputs):
                continue
            ruling = machine.evaluate("review", result, inputs)
            if isinstance(ruling, NoRule):
                continue
            if ruling.next_phase in ("regression", "ship"):
                assert not inputs["has_security_residual"], ruling.rule_id
                assert inputs["max_residual_severity"] in (
                    "none",
                    "info",
                    "low",
                    "medium",
                ), ruling.rule_id


def test_sweep_all_phases_rulings_are_well_formed(machine: Machine) -> None:
    """Full-grid behavioral sweep across every non-terminal phase: every
    ruling names a real phase and a journal severity, hard always stops,
    flagged never lands terminal, and well-typed inputs never trip an
    unreadable-input park."""
    terminal = set(machine.terminal)
    for phase in machine.phases:
        if phase in terminal:
            continue
        results = {r["result"] for r in TABLE["rules"] if r["from"] == phase}
        results.add("totally-novel-result")
        for result in sorted(results):
            for consecutive in (None, 1, 2):
                for inputs in _input_grid():
                    if consecutive is not None:
                        inputs = {**inputs, "consecutive_failures": consecutive}
                    ruling = machine.evaluate(phase, result, inputs)
                    if isinstance(ruling, NoRule):
                        assert ruling.problem is None, (phase, result)
                        continue
                    assert ruling.next_phase in machine.phases
                    assert ruling.severity in RULING_SEVERITIES
                    if ruling.severity == "hard":
                        assert ruling.next_phase == "stop", ruling.rule_id
                    if ruling.severity == "flagged":
                        assert ruling.next_phase not in terminal, ruling.rule_id
