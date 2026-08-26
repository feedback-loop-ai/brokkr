# Tasks: Enforce `requires_artifacts` at decide time

**Feature slug**: `enforce-requires-artifacts`
**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Ordered; each task names the test that proves it. Acceptance-criteria
numbers (AC-n) refer to spec.md's `## Acceptance Criteria`.

- [x] **T1 — Pure helpers in `engine.rs`.** Add
  `artifact_failures(workdir, required)` (lexical predicate + one
  `fs::metadata` probe per entry, classes
  `missing`/`empty`/`not-a-file`/`invalid`, complete list in table
  order) and `artifact_problem(rule_id, failures)` (the canonical
  string, character-exact per spec).
  *Proven by*: unit tests beside the helpers covering every class,
  the reserved characters `{ } $ < >`, `..`/absolute rejection,
  symlink-followed and dangling-symlink cases, multi-failure ordering,
  and the exact rendered string.
- [x] **T2 — Wire the gate into the `Outcome::Ruling` arm**
  (`engine.rs:1079`): bind `requires_artifacts`; empty failure list →
  today's advancing payload byte-identical; otherwise blocked payload
  (`rule_id` retained, `next: null`, `severity: null`, `inputs`
  unchanged, `problem` from `artifact_problem`).
  *Proven by*: AC-1 machine proof (missing artifact → park with exact
  payload values, no subsequent seat) and AC-5 (artifacts present →
  advance byte-equivalent to an artifact-free rule).
- [x] **T3 — Edge-class machine proofs.** Zero-byte file (`empty`),
  directory (`not-a-file`), and `../escape` / `{slug}` entries
  (`invalid`) each park with the canonical problem string.
  *Proven by*: AC-2 and AC-3 machine proofs.
- [x] **T4 — Multi-failure proof.** One rule, three failing artifacts →
  a single park listing all three in table order.
  *Proven by*: AC-4 machine proof.
- [x] **T5 — Recovery-loop proof.** Park → operator `retry` → seat
  re-runs → artifacts now present → advance; no attempt budget
  consumed by the gate park.
  *Proven by*: AC-6 machine proof.
- [x] **T6 — Replay and counter-semantics proofs.** Replay both the
  parked and advanced runs with the workdir deleted; assert state
  reproduction and export stability
  (`full_delivery_completes_exports_and_replays` pattern); assert the
  blocked decision resets `consecutive_failures` for the phase.
  *Proven by*: AC-7 and AC-8 machine proofs.
- [ ] **T7 — Workspace regression sweep.** `cargo test` across the
  whole workspace: differential corpus untouched and green,
  `policy_lint.rs` unchanged and green, forge-core diff empty
  (`git diff --stat crates/forge-core` shows nothing).
  *Proven by*: AC-9 (full suite) plus the empty-diff check in review.
