#!/usr/bin/env python3
"""Derive the evaluator behavior corpus from the production table + oracle.

For every (phase, result) rule group, enumerate the full domain of every
input the group's conditions reference. Unreferenced inputs cannot affect
the outcome (evaluate only reads inputs through conditions), so coverage
over referenced-input assignments is exhaustive over behavior classes.
Adds novel-result, unknown-phase, and mistyped-input park cases.

Deterministic: same table + same oracle -> byte-identical corpus.
Regenerate with:  python3 tools/generate_evaluator_corpus.py
"""

from __future__ import annotations

import itertools
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))

from forge.machine import Machine, NoRule, Ruling, SEVERITY_ORDER  # noqa: E402

ABSENT = object()

COUNTER_DOMAIN = [ABSENT, 1, 2]
SEVERITY_DOMAIN = [ABSENT, *SEVERITY_ORDER]
BOOLEAN_DOMAIN = [ABSENT, False, True]

MISTYPED_CASES = [
    ("review", "clean", {"fixes_applied": "no"}),
    ("review", "residual", {"has_security_residual": "yes"}),
    ("review", "residual", {"max_residual_severity": "banana"}),
    ("review", "residual", {"max_residual_severity": 3}),
    ("architecture", "failed", {"consecutive_failures": "2"}),
    ("architecture", "failed", {"consecutive_failures": True}),
]


def referenced_inputs(rules: list[dict]) -> list[str]:
    names: set[str] = set()
    for rule in rules:
        for key in rule.get("when", {}):
            if key.endswith("_gte"):
                names.add(key[: -len("_gte")])
            elif key.endswith("_above"):
                names.add(key[: -len("_above")])
            else:
                names.add(key)
    return sorted(names)


def domain(name: str):
    if name == "consecutive_failures":
        return COUNTER_DOMAIN
    if name == "max_residual_severity":
        return SEVERITY_DOMAIN
    return BOOLEAN_DOMAIN


def expectation(outcome) -> dict:
    if isinstance(outcome, Ruling):
        return {
            "kind": "ruling",
            "rule_id": outcome.rule_id,
            "next": outcome.next_phase,
            "severity": outcome.severity,
        }
    assert isinstance(outcome, NoRule)
    return {"kind": "no_rule", "problem": outcome.problem}


def main() -> None:
    table = json.loads((ROOT / "policy" / "phase-machine.json").read_text())
    machine = Machine.from_table(table)

    groups: dict[tuple[str, str], list[dict]] = {}
    for rule in table["rules"]:
        groups.setdefault((rule["from"], rule["result"]), []).append(rule)

    cases: list[dict] = []

    for (phase, result), rules in sorted(groups.items()):
        names = referenced_inputs(rules)
        for values in itertools.product(*(domain(n) for n in names)):
            inputs = {n: v for n, v in zip(names, values) if v is not ABSENT}
            cases.append({"phase": phase, "result": result, "inputs": inputs})

    for phase in table["phases"]:
        if phase in table["terminal"]:
            continue
        cases.append({"phase": phase, "result": "totally-novel-result", "inputs": {}})

    cases.append({"phase": "nonexistent", "result": "complete", "inputs": {}})

    for phase, result, inputs in MISTYPED_CASES:
        cases.append({"phase": phase, "result": result, "inputs": inputs})

    out = ROOT / "fixtures" / "evaluator" / "corpus.ndjson"
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w") as fh:
        for case in cases:
            case["expect"] = expectation(
                machine.evaluate(case["phase"], case["result"], case["inputs"])
            )
            fh.write(json.dumps(case, sort_keys=True, separators=(",", ":")) + "\n")
    print(f"wrote {len(cases)} cases to {out.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
