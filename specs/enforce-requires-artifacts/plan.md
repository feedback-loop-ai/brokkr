# Implementation Plan: Enforce `requires_artifacts` at decide time

**Feature slug**: `enforce-requires-artifacts`
**Spec**: [spec.md](spec.md)

## Position reconciliation (how this plan was synthesized)

The design panel's two positions agreed on the load-bearing rulings —
strictly static (option 1), `rule_id` retained on the blocked decision,
symlinks followed, regular-file-and-non-empty predicate, zero fold
changes, no `inputs` injection, no corpus changes. The chief's rulings
on the four genuine divergences:

1. **Where the gate lives** — *reconciled*. Simplicity wanted ~20
   inline lines in the `Outcome::Ruling` match arm; robustness wanted a
   pure verdict function plus problem-string renderer as forge-core
   public API, for parity mirroring. Adopted: **forge-core's diff is
   empty** (simplicity's strongest form of evaluator purity — there is
   no second caller, and a new public API is a new parity surface that
   nothing currently mirrors), **but** the check and the evidence
   formatting are **private pure free functions in `engine.rs`**, not
   code interleaved into the match arm (robustness's real requirement:
   the journal-borne problem string must be produced by one pure,
   unit-testable function, never ad-hoc `format!` at the call site).
   The canonical string is pinned character-exact in the spec and by
   machine proof — that, not a crate boundary, is what makes it a
   recorded contract the Python parity core can mirror if it ever
   needs to.
2. **`severity` on the blocked decision** — *robustness adopted,
   simplicity rejected*. Nulled. Severity is a property of a taken
   transition; the schema-violation park already nulls it
   (`engine.rs:1030`), and a retained `hard` on a non-transition is
   bait for severity-scanning consumers. Simplicity's "retain both"
   traded a real consumer hazard for a line of code.
3. **Load-time lint** — *simplicity adopted, robustness's mechanism
   rejected, its goal kept*. Robustness wanted lexical validation "at
   lint and check time", but `policy_lint.rs` is a test of the loader
   (`crates/forge-core/tests/policy_lint.rs`) — rejecting entries at
   load means changing forge-core's parser, contradicting the empty
   forge-core diff both positions valued. Adopted: **one enforcement
   point, decide time**, failing closed as class `invalid`. The
   reserved-character fence (robustness's goal) survives intact at
   that single point: an entry containing `{ } $ < >` can never
   advance, so no bundle can come to depend on those characters'
   literal meaning — the forward-compatibility door is closed without
   a loader change. A load-time lint remains an additive follow-up if
   authoring feedback ever hurts in practice.
4. **Terminal-destination rules** — *robustness's lint rejected, its
   observation kept*. No engine special-case and no lint; the gate is
   uniform over destinations, and the spec documents
   requires-artifacts-on-a-terminal-rule as an authoring smell for
   review. No in-tree rule does it; machinery for a hypothetical is
   the same speculative generality the panel rejected for option 2.

## Approach

All behavior lands in `crates/forge-runtime/src/engine.rs`, in the
decide step:

1. Two private pure free functions (no `std::fs` in the second):
   - `fn artifact_failures(workdir: &Path, required: &[String]) -> Vec<(String, &'static str)>`
     — for each entry, in table order: the lexical predicate
     (`invalid`), then one `std::fs::metadata` probe classified as
     `missing` / `not-a-file` / `empty` per the spec's predicate.
     Returns the complete failure list, empty when the gate passes.
     (This function is the one place I/O happens; it takes the workdir
     as a value so tests exercise it against temp dirs directly.)
   - `fn artifact_problem(rule_id: &str, failures: &[(String, &'static str)]) -> String`
     — renders the canonical string
     `requires_artifacts unmet for rule <id>: <class>: <entry>; ...`.
     Pure, unit-testable, the single producer of the journal-borne
     format.
2. The `Outcome::Ruling` arm (`engine.rs:1079`) stops discarding
   `requires_artifacts` with `..`; binds it; when the failure list is
   empty it builds exactly today's advancing payload, otherwise the
   blocked payload: `rule_id` retained, `next: null`,
   `severity: null`, `inputs` unchanged, `problem` from
   `artifact_problem`.
3. Nothing else changes. The `next: null` + `problem` payload rides
   the existing fold arm (`fold.rs:296-330`) into
   `awaiting_operator`, exactly like NoRule and schema violations
   today.

## Files touched

- `crates/forge-runtime/src/engine.rs` — the gate (two pure helpers +
  the match-arm change). The only production code change.
- `crates/forge-cli/tests/machine_proof.rs` — the nine acceptance
  criteria from spec.md, using the existing `Workspace`
  self-authored-table scaffolding.
- `specs/enforce-requires-artifacts/*`,
  `openspec/changes/enforce-requires-artifacts/proposal.md` — this
  design.

Explicitly untouched: `crates/forge-core/*` (empty diff),
`crates/forge-core/src/fold.rs`, `policy/phase-machine.json`,
`reference/`, `fixtures/`, `contracts/`, `recipes/sdd/*`.

## Risks and mitigations

- **The problem string becomes frozen contract by accident of
  wording.** It is frozen contract *on purpose*: specified
  character-exact in spec.md, produced by one pure function, pinned by
  machine proof. Changing it later is a contract change and will read
  as one.
- **`rule_id` retained on a blocked decision could be misread as "the
  rule succeeded."** Accepted: `next: null` plus a `problem` prefixed
  `requires_artifacts unmet` is unambiguous, and the fold renders it
  as a park reason. The alternative — nulling it — makes gate blocks
  permanently indistinguishable from NoRule in the journal.
- **The park reason's `no ruling for (from, result):` prefix is
  misleading for a gate block.** Accepted; fixing it touches the
  replay path for a cosmetic gain. The evidence lives in `problem`.
- **Malformed entries (`../x`, absolute, reserved chars) park at
  decide time instead of failing at load.** Accepted: tables are
  reviewed text, the park names the offending entry verbatim with
  class `invalid`, and a load-time lint is an additive follow-up that
  would otherwise cost the empty forge-core diff.
- **Symlinks to non-empty regular files pass.** Accepted: the workdir
  is operator-controlled; the gate defends against absent work, not
  adversarial layout beyond the lexical predicate.
- **`consecutive_failures` resets on a gate park.** Intended semantics
  (the seat succeeded; the gate blocked), stated in the spec and
  pinned by proof so a future "fix" can't silently change replayed
  journals.
- **sdd-style recipes cannot use `requires_artifacts` for per-feature
  paths.** By design: speckit-check exists, works, and checks more
  than existence. The spec's division-of-labor paragraph prevents
  rediscovering this tension.
