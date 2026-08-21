"""Pure evaluation of the Forge phase machine.

This module is the deterministic heart of the engine. It holds NO side
effects: no filesystem, no subprocesses, no clock, no randomness. Given the
same transition table and the same inputs it always returns the same ruling.

Design law (decision 0001): an executor result that does not validate, or a
(phase, result) pair no rule matches, is NEVER repaired, guessed at, or
handed to a model to "fix". It parks the run in awaiting-operator with the
raw evidence attached. Fail closed into human judgment.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any, Mapping, Sequence

SEVERITY_ORDER = ["none", "low", "medium", "high", "critical"]


class PolicyError(ValueError):
    """The transition table itself is malformed. Refuse to run at all."""


@dataclass(frozen=True)
class Ruling:
    """The outcome of evaluating one (phase, result, inputs) against the table."""

    rule_id: str
    next_phase: str
    severity: str  # "none" | "flagged" | "hard"
    reason: str
    requires_artifacts: tuple[str, ...] = ()


@dataclass(frozen=True)
class NoRule:
    """No rule matched. The engine MUST park in awaiting-operator (law 0001)."""

    phase: str
    result: str
    inputs: Mapping[str, Any]


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
        for t in table["terminal"]:
            if t not in phases:
                raise PolicyError(f"terminal phase '{t}' not in phases")
        seen_ids: set[str] = set()
        for rule in table["rules"]:
            for key in ("id", "from", "result", "next", "reason"):
                if key not in rule:
                    raise PolicyError(f"rule {rule.get('id', '?')} missing '{key}'")
            if rule["id"] in seen_ids:
                raise PolicyError(f"duplicate rule id {rule['id']}")
            seen_ids.add(rule["id"])
            if rule["from"] not in phases or rule["next"] not in phases:
                raise PolicyError(f"rule {rule['id']} references unknown phase")
        return cls(
            phases=phases,
            initial=table["initial"],
            terminal=tuple(table["terminal"]),
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
            return NoRule(phase=phase, result=result, inputs=inputs)
        for rule in self.rules:
            if rule["from"] != phase or rule["result"] != result:
                continue
            if not _conditions_met(rule.get("when", {}), inputs):
                continue
            return Ruling(
                rule_id=rule["id"],
                next_phase=rule["next"],
                severity=rule.get("severity", "none"),
                reason=rule["reason"],
                requires_artifacts=tuple(rule.get("requires_artifacts", ())),
            )
        return NoRule(phase=phase, result=result, inputs=inputs)


def _conditions_met(when: Mapping[str, Any], inputs: Mapping[str, Any]) -> bool:
    for key, expected in when.items():
        if key.endswith("_gte"):
            actual = inputs.get(key[: -len("_gte")], 0)
            if not (isinstance(actual, (int, float)) and actual >= expected):
                return False
        elif key == "max_residual_severity_above":
            actual = inputs.get("max_residual_severity", "none")
            if _severity_rank(actual) <= _severity_rank(expected):
                return False
        else:
            if bool(inputs.get(key, False)) is not bool(expected):
                return False
    return True


def _severity_rank(name: str) -> int:
    try:
        return SEVERITY_ORDER.index(name)
    except ValueError:
        # Unknown severity is treated as ABOVE every known one: fail closed.
        return len(SEVERITY_ORDER)
