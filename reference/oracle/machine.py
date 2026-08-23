"""Pure evaluation of the Forge phase machine.

This module is the deterministic heart of the engine. It holds NO side
effects: no filesystem, no subprocesses, no clock, no randomness. Given the
same transition table and the same inputs it always returns the same ruling.

Design law (decision 0001): an executor result that does not validate, or a
(phase, result) pair no rule matches, is NEVER repaired, guessed at, or
handed to a model to "fix". It parks the run in awaiting-operator with the
raw evidence attached. Fail closed into human judgment.

Strictness laws (decision 0004):

- The condition vocabulary is CLOSED and checked when the table loads. A
  typo'd condition key is a malformed table (PolicyError), never a rule
  that silently stops matching.
- An absent (or null) input never satisfies a condition, whatever its
  polarity.
- A present input the vocabulary cannot read (wrong type, unknown
  severity) parks the evaluation (NoRule with `problem`) — the control
  plane never coerces and never guesses.
"""

from __future__ import annotations

from collections.abc import Mapping as MappingABC
from dataclasses import dataclass
from typing import Any, Mapping

#: Residual-finding severity axis, lowest to highest — the value vocabulary
#: of `max_residual_severity`. Distinct from the RULING severity axis below.
#: Mirrors reference/forge-control.py SEVERITY_ORDER, "info" included.
SEVERITY_ORDER = ("none", "info", "low", "medium", "high", "critical")

#: Ruling severity axis — the vocabulary the forge.phase-event/v1 journal
#: schema records. A rule that names no severity rules at "normal".
RULING_SEVERITIES = ("normal", "flagged", "hard")

#: The closed condition vocabulary (restores forge-control.py
#: KNOWN_CONDITION_KEYS, lost in extraction). Admitting a new input is
#: deliberately a two-file diff — the table must use it AND this registry
#: must declare it — the same property the constitutional lint gives rules.
COUNTER_INPUTS = frozenset({"consecutive_failures"})
SEVERITY_INPUTS = frozenset({"max_residual_severity"})
BOOLEAN_INPUTS = frozenset(
    {
        "skip_verify",
        "fixes_applied",
        "has_security_residual",
        "high_risk_uncovered",
        "drift_detected",
        "dirty_worktrees",
    }
)


class PolicyError(ValueError):
    """The transition table itself is malformed. Refuse to run at all."""


@dataclass(frozen=True)
class Ruling:
    """The outcome of evaluating one (phase, result, inputs) against the table."""

    rule_id: str
    next_phase: str
    severity: str  # "normal" | "flagged" | "hard" (forge.phase-event/v1)
    reason: str
    requires_artifacts: tuple[str, ...] = ()


@dataclass(frozen=True)
class NoRule:
    """No ruling is possible. The engine MUST park in awaiting-operator (law 0001).

    `problem` records WHY: None means no rule matched the (phase, result)
    pair; a string means the machine refused to rule — an unknown phase, or
    a present condition input it cannot read. Either way: park, never guess.
    """

    phase: str
    result: str
    inputs: Mapping[str, Any]
    problem: str | None = None


class _UnreadableInput(Exception):
    """Internal: a condition referenced a present input outside its vocabulary."""

    def __init__(self, problem: str) -> None:
        super().__init__(problem)
        self.problem = problem


@dataclass(frozen=True)
class Machine:
    phases: tuple[str, ...]
    initial: str
    terminal: tuple[str, ...]
    shippable_from: tuple[str, ...]
    rules: tuple[Mapping[str, Any], ...]
    computed_inputs: tuple[str, ...] = ()

    @classmethod
    def from_table(cls, table: Mapping[str, Any]) -> "Machine":
        for key in ("phases", "initial", "terminal", "rules"):
            if key not in table:
                raise PolicyError(f"phase machine table missing '{key}'")
        phases = tuple(table["phases"])
        if table["initial"] not in phases:
            raise PolicyError("initial phase not in phases")
        terminal = tuple(table["terminal"])
        for t in terminal:
            if t not in phases:
                raise PolicyError(f"terminal phase '{t}' not in phases")
        seen_ids: set[str] = set()
        ruled_unconditionally: set[tuple[str, str]] = set()
        for rule in table["rules"]:
            for key in ("id", "from", "result", "next", "reason"):
                if key not in rule:
                    raise PolicyError(f"rule {rule.get('id', '?')} missing '{key}'")
            rule_id = rule["id"]
            if rule_id in seen_ids:
                raise PolicyError(f"duplicate rule id {rule_id}")
            seen_ids.add(rule_id)
            if rule["from"] not in phases or rule["next"] not in phases:
                raise PolicyError(f"rule {rule_id} references unknown phase")
            if rule["from"] in terminal:
                raise PolicyError(
                    f"rule {rule_id} leaves terminal phase '{rule['from']}'"
                )
            severity = rule.get("severity", "normal")
            if severity not in RULING_SEVERITIES:
                raise PolicyError(
                    f"rule {rule_id} severity {severity!r} not in {RULING_SEVERITIES}"
                )
            artifacts = rule.get("requires_artifacts", ())
            if not (
                isinstance(artifacts, (list, tuple))
                and all(isinstance(a, str) for a in artifacts)
            ):
                raise PolicyError(
                    f"rule {rule_id} requires_artifacts must be a list of strings"
                )
            when = rule.get("when", {})
            if not isinstance(when, MappingABC):
                raise PolicyError(f"rule {rule_id} 'when' must be an object")
            for cond_key, expected in when.items():
                _validate_condition(rule_id, cond_key, expected)
            # First match wins, so any rule behind an unconditional rule for
            # the same (from, result) can never fire. Dead policy is a defect.
            group = (rule["from"], rule["result"])
            if group in ruled_unconditionally:
                raise PolicyError(
                    f"rule {rule_id} is unreachable: an unconditional rule for "
                    f"{group} precedes it and first match wins"
                )
            if not when:
                ruled_unconditionally.add(group)
        return cls(
            phases=phases,
            initial=table["initial"],
            terminal=terminal,
            shippable_from=tuple(table.get("shippable_from", ())),
            rules=tuple(table["rules"]),
            computed_inputs=tuple(table.get("computed_inputs", {})),
        )

    def evaluate(
        self, phase: str, result: str, inputs: Mapping[str, Any]
    ) -> Ruling | NoRule:
        """First matching rule wins, in table order (deny-before-allow is a
        property of table AUTHORING, preserved here by strict ordering)."""
        if phase not in self.phases:
            return NoRule(
                phase=phase,
                result=result,
                inputs=inputs,
                problem=f"unknown phase '{phase}'",
            )
        for rule in self.rules:
            if rule["from"] != phase or rule["result"] != result:
                continue
            try:
                if not _conditions_met(rule.get("when", {}), inputs):
                    continue
            except _UnreadableInput as exc:
                return NoRule(
                    phase=phase, result=result, inputs=inputs, problem=exc.problem
                )
            return Ruling(
                rule_id=rule["id"],
                next_phase=rule["next"],
                severity=rule.get("severity", "normal"),
                reason=rule["reason"],
                requires_artifacts=tuple(rule.get("requires_artifacts", ())),
            )
        return NoRule(phase=phase, result=result, inputs=inputs)


def _validate_condition(rule_id: str, key: str, expected: Any) -> None:
    """Load-time half of the closed vocabulary: every condition names a
    declared input and carries a threshold of the right type."""
    if key.endswith("_gte"):
        counter = key[: -len("_gte")]
        if counter not in COUNTER_INPUTS:
            raise PolicyError(
                f"rule {rule_id}: unknown counter '{counter}' in condition "
                f"'{key}'; known: {sorted(COUNTER_INPUTS)}"
            )
        if isinstance(expected, bool) or not isinstance(expected, (int, float)):
            raise PolicyError(
                f"rule {rule_id}: condition '{key}' needs a numeric threshold, "
                f"got {expected!r}"
            )
    elif key.endswith("_above"):
        axis = key[: -len("_above")]
        if axis not in SEVERITY_INPUTS:
            raise PolicyError(
                f"rule {rule_id}: unknown severity axis '{axis}' in condition "
                f"'{key}'; known: {sorted(SEVERITY_INPUTS)}"
            )
        if expected not in SEVERITY_ORDER:
            raise PolicyError(
                f"rule {rule_id}: condition '{key}' threshold {expected!r} not "
                f"in {SEVERITY_ORDER}"
            )
    elif key in BOOLEAN_INPUTS:
        if not isinstance(expected, bool):
            raise PolicyError(
                f"rule {rule_id}: condition '{key}' expects true/false, "
                f"got {expected!r}"
            )
    else:
        raise PolicyError(
            f"rule {rule_id}: unknown condition key '{key}'; known: "
            f"{sorted(BOOLEAN_INPUTS)} plus "
            f"{sorted(c + '_gte' for c in COUNTER_INPUTS)} and "
            f"{sorted(s + '_above' for s in SEVERITY_INPUTS)}"
        )


def _conditions_met(when: Mapping[str, Any], inputs: Mapping[str, Any]) -> bool:
    """Runtime half of the vocabulary. An ABSENT (or null) input never
    satisfies a condition, whatever its polarity. A PRESENT input the
    vocabulary cannot read raises so `evaluate` parks instead of coercing
    (law 0001). Presence requirements belong to the result schemas — the
    evaluator only guarantees that absence is never an advantage."""
    for key, expected in when.items():
        if key.endswith("_gte"):
            counter = key[: -len("_gte")]
            actual = inputs.get(counter)
            if actual is None:
                return False
            if isinstance(actual, bool) or not isinstance(actual, (int, float)):
                raise _UnreadableInput(
                    f"{counter} must be a number, got {actual!r}"
                )
            if actual < expected:
                return False
        elif key.endswith("_above"):
            axis = key[: -len("_above")]
            actual = inputs.get(axis)
            if actual is None:
                return False
            if not isinstance(actual, str) or actual not in SEVERITY_ORDER:
                raise _UnreadableInput(
                    f"{axis} severity {actual!r} not in {SEVERITY_ORDER}"
                )
            if SEVERITY_ORDER.index(actual) <= SEVERITY_ORDER.index(expected):
                return False
        else:
            actual = inputs.get(key)
            if actual is None:
                return False
            if not isinstance(actual, bool):
                raise _UnreadableInput(f"{key} must be a boolean, got {actual!r}")
            if actual is not expected:
                return False
    return True
