# 0001 — Schema mismatches are never repaired by a model

**Status**: accepted (operator ruling, 2026-08-21)

## Ruling

When a phase executor returns a result that fails schema validation, or the
machine finds no rule matching a `(phase, result)` pair, the engine does
**not** ask any model to fix, reinterpret, coerce, or retry-with-guidance the
malformed output at the control-plane level. The run transitions to
`awaiting-operator` with the raw evidence attached (the invalid payload, the
schema errors, the phase context), and only an operator event can move it.

## Why

- The entire value of the outermost state machine is that transitions are
  computed by pure code. An "LLM fixer" for control-plane data reintroduces
  nondeterminism at exactly the layer we built to exclude it — the repaired
  value would decide a transition, so agent output would decide a transition.
- A malformed result is *signal*, not noise: the executor's inner loop
  already had bounded retries with the schema in-prompt. Malformed output
  surviving that means something structural is wrong (prompt drift, provider
  degradation, a schema/prompt version skew). Papering over it hides the
  defect and lets a degraded seat keep steering a delivery.
- Fail-closed-into-human-judgment is the same posture the ship gate and
  security-hold already take. Unknown severity ranks above every known
  severity; unknown results rank as "park", never "guess".

## What IS allowed

- *Inside* an executor, the normal structured-output retry loop (validation
  error fed back to the same seat, bounded attempts) — that is the seat
  producing its own result, not the engine repairing one.
- Purely mechanical, lossless normalization defined in code (e.g. JSON
  parsing, trailing-whitespace) — deterministic, reviewable, no model.

## Consequences

- `Machine.evaluate` returns `NoRule` rather than raising or defaulting;
  the engine maps `NoRule` and schema failures to `awaiting-operator`.
- Table totality is a *test* concern (sweep tests), not a runtime hope.
- Operators see rare, high-signal parks instead of silent self-healing.
