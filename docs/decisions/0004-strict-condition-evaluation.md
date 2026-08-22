# 0004 — Strict condition evaluation in the pure core

**Status**: accepted (operator ruling, 2026-08-23)

## Ruling

The pure evaluator (`src/forge/machine.py`) closes four gaps found during an
architecture review, three of which were extraction regressions against the
referee-era control plane (`reference/forge-control.py`):

1. **Ruling severity vocabulary is `normal | flagged | hard`.** A rule that
   names no severity rules at `normal` — the token the
   `forge.phase-event/v1` schema enumerates and production journals record
   (`forge-control.py:3294`). The previous default `"none"` collided with the
   residual-severity axis and could not be recorded schema-valid.
2. **The residual-severity axis regains `info`.** The referee's
   `SEVERITY_ORDER` is `none, info, low, medium, high, critical`
   (`forge-control.py:1249`) and the `phase-advance` CLI accepts `info`. The
   extraction dropped it, so a valid production value would have been
   misread as unknown and ranked above `critical` — a wrong-but-closed stop.
3. **The condition vocabulary is closed and enforced at load.** Every `when`
   key must name a declared input (counter, severity axis, or boolean) and
   carry a threshold of the right type, or `Machine.from_table` raises
   `PolicyError`. This restores the referee's `KNOWN_CONDITION_KEYS` guard
   (`forge-control.py:1252-1265`, added after a typo'd key "silently
   disabled the hard stop it guarded, with no error and no test failure"),
   upgraded from evaluation-time error to load-time refusal: a dead deny
   rule cannot even be installed. Extending the vocabulary is deliberately a
   two-file diff — the table must use the new input AND the evaluator's
   registry must declare it — the same property the constitutional lint
   gives protected rules.
4. **Input semantics are uniform and never guessed:**
   - An **absent (or null) input never satisfies a condition**, whatever its
     polarity. In particular, `when {fixes_applied: false}` — the skip-V′
     shortcut — no longer matches when the input is merely missing; absence
     routes through regression.
   - A **present input the vocabulary cannot read** (non-boolean flag,
     non-numeric counter, severity outside the axis) **parks the
     evaluation**: `evaluate` returns `NoRule` with a `problem` string, and
     the engine maps that to `awaiting-operator` per decision 0001. It is
     never coerced (`bool()`), never ranked, never defaulted.

The loader additionally rejects rules that leave a terminal phase, rules
shadowed by a preceding unconditional rule for the same `(phase, result)`
(dead policy under first-match-wins), unknown ruling severities, and
malformed `requires_artifacts`.

## Deliberate divergences from the referee

The referee is the behavioral baseline; where the pure core differs, the
difference is toward decision 0001, and parity fixtures must encode these as
expected outcomes, not defects:

| Case | Referee (`forge-control.py`) | Pure core |
|---|---|---|
| Unknown condition key | `ForgeControlError` at evaluation | `PolicyError` at load |
| Unknown severity value | raises, no transition recorded | parks (`NoRule` + `problem`) |
| Non-boolean flag value | `bool()` coercion | parks (`NoRule` + `problem`) |
| Non-numeric counter | uncaught `TypeError` | parks (`NoRule` + `problem`) |
| Absent flag on a negative-polarity condition | satisfies (`bool(None) is False`) | never satisfies |

The last row is the only behavior change reachable from well-formed
production data: a review `clean` result that omits `fixes_applied` now takes
the regression path instead of the ship shortcut. Omission is not a claim.

## What stays

- First-match-wins over the ordered rule list; deny-before-allow remains a
  property of table authoring.
- An unmatched `(phase, result)` pair returns `NoRule` (`problem: None`) and
  parks — decision 0001 unchanged.
- The evaluator cannot enforce *presence*. When a deny rule is conditional
  and the permissive fallback is unconditional (the review severity and
  security denies), an absent input still reaches the fallback — exactly as
  in production. Presence enforcement is the result schemas' job; the
  evaluator guarantees only that absence never *satisfies* a condition.

## Consequences

- `tests/test_machine.py` grows loader-rejection tests (the first executable
  slice of the constitutional policy lint), absent/mistyped-input semantics
  tests, table-wide lint (hard ⇒ stop; flagged ⇒ non-terminal;
  `security-hold` ⇒ hard stop wherever it appears), graph tests (all phases
  reachable; every phase reaches a terminal; removing `review` makes `ship`
  and `done` unreachable), and a full-grid sweep across every non-terminal
  phase.
- `policy/phase-machine.json`, `policy/schemas/phase-state.schema.json`, and
  everything under `reference/` are untouched: the machine was aligned to
  the recorded contracts, not the contracts to the machine.
- The Rust port inherits these semantics through the parity suite instead of
  rediscovering them.

## Noted for the port, not remediated here

- The ship result taxonomy admits two paths into `done` (`ready` → done with
  push instructed, and `shipped` → done); the port may tighten this to
  `ready` → push/PR effect → `shipped` as the only entry.
- The table's self-description still cites referee-era paths
  (`.scripts/forge-control.py`, SKILL.md); the bundle compiler will own that
  prose.
- `forge compile` should add the provenance lint this decision defers: any
  input referenced by a hard conditional rule must be engine-computed or
  schema-required, so the absent-input fallback documented above becomes
  unreachable for deny rules.
