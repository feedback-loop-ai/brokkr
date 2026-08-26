# Change Proposal: enforce-requires-artifacts

## Why

Policy rules may declare `requires_artifacts` — the heritage table's
ARCH-OK names `spec.md`, `plan.md`, `repos.yaml` — and the evaluator
returns the field on every ruling, but the engine's decide step
discards it (`crates/forge-runtime/src/engine.rs:1079` binds three
fields and drops the rest with `..`). A declared artifact gate that is
not load-bearing reads as enforced and is not: an advancing ruling
whose named artifacts are missing or empty advances anyway, and the
first honest signal arrives phases later. Mechanical checks must stay
mechanical (decision 0001) and this one must actually run.

## What Changes

At decide time, after `Machine::evaluate` returns a ruling carrying a
non-empty `requires_artifacts`, the engine verifies each entry is a
lexically valid, workdir-relative path naming a non-empty regular file
(symlinks followed; directories, zero-byte files, metadata errors, and
invalid entries — including the reserved characters `{ } $ < >` — all
fail). A pass advances byte-identically to today. Any failure records a
blocked decision through the existing park mechanism: `next: null`,
`rule_id` retained, `severity: null`, and a canonical machine-stable
`problem` string naming every failing artifact with its failure class,
table order. The open static-vs-dynamic question is ruled **strictly
static** (option 1) with a reserved-character fence as the designated
extension point; dynamic per-feature gating remains the province of
deterministic check seats (speckit-check is the precedent). Forge-core,
the fold, the frozen decided-payload field set, the corpus, and the sdd
recipe are all untouched; the only production code change is in
`engine.rs`, covered by nine machine-proof acceptance criteria.

Design artifacts:

- [specs/enforce-requires-artifacts/spec.md](../../../specs/enforce-requires-artifacts/spec.md)
  — what and why, gate semantics, the static-vs-dynamic ruling, and
  the `## Acceptance Criteria`.
- [specs/enforce-requires-artifacts/plan.md](../../../specs/enforce-requires-artifacts/plan.md)
  — how, the panel-position reconciliation, files touched, risks with
  mitigations.
- [specs/enforce-requires-artifacts/tasks.md](../../../specs/enforce-requires-artifacts/tasks.md)
  — ordered tasks, each paired with the proof that closes it.

## Impact

- `crates/forge-runtime/src/engine.rs` — two private pure helpers plus
  the `Outcome::Ruling` match-arm change; the sole production edit.
- `crates/forge-cli/tests/machine_proof.rs` — new proofs for the
  fail-closed park (missing / empty / not-a-file / invalid /
  multi-failure), the artifacts-present advance, the retry recovery
  loop, workdir-deleted replay, and the `consecutive_failures` reset.
- No changes to: `crates/forge-core` (empty diff — `Machine::evaluate`
  byte-identical), `fold.rs`, `policy/phase-machine.json`,
  `reference/`, `fixtures/evaluator/corpus.ndjson`, `contracts/`, the
  condition vocabulary, or the sdd recipe / `speckit_check.sh`.
- Operational: runs whose ruling names absent artifacts now park in
  `awaiting_operator` with the exact missing/empty list in the park
  evidence instead of advancing; recovery is the existing operator
  `retry`. Old journals replay as recorded — the gate exists only at
  decide time.
