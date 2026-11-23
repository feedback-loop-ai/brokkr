# 0120 — A review panel joins a residual member's unreadable safety fact as unknown, never as safe

Status: proposed
Date: 2026-11-23

Built: built
Amends: 0008

## Context

A `review-panel` aggregate ranks its members' words worst-first and joins
their `inputs` beside the winning word. Decision
[0008](0008-second-wave-scope.md) ruling 3 named the join "severity max;
security/fixes OR-ed", and the join reads a member's fact the way an OR
over `false` defaults would: a `has_security_residual` that is absent,
`null` or not a boolean counts as `false`, and a `max_residual_severity`
outside the severity vocabulary counts as `none`
(`crates/brokkr-runtime/src/bundle/aggregate.rs`, `ReviewInputs::join`).

Decision [0109](0109-a-gate-s-re-visit-reads-what-changed.md) ruling 2
binds a *passing* word to its residual pair: a review-panel member that
says `clean` over an absent, null or unreadable declared fact is refused
before the join, and the aggregate writes both facts on every result.
It does not, and by its own text should not, read the facts of a
non-passing word: `residual` is admitted whatever its inputs say. So a
`residual` member can still omit its facts or write them mistyped, and
the join then lends the panel the `has_security_residual: false` and the
`max_residual_severity` at or below medium that a table's tracked-debt
route asks for. A table that ships a residual only on explicitly safe
facts, and stops on anything else, is opened by the member that said
least.

This is the first of four engine patches the adopter
`feedback-loop-ai/swarmail` has carried on top of brokkr 0.12.0
(`tooling-patches/0001-preserve-review-residual-facts.patch`, release
`swarmail-engine-a9893b4-p4`), proposed upstream so the adopter can pin a
stock release (feedback-loop-ai/swarmail#405). Pull request #899 first
proposed it, with the adopter's other two review and verify patches,
under decision 0097; this record restates the slice that main still
needs after 0109a2, and nothing else.

## Rulings

1. **A residual member's unreadable safety fact is unknown.** The panel
   reads each member's `has_security_residual` as a boolean and its
   `max_residual_severity` as a word of the severity vocabulary, or not
   at all. A witnessed `true` dominates the joined security flag. Beside
   it, the flag is `false` only when every `residual` member supplied a
   readable `false`; a `residual` member whose flag is absent, `null` or
   mistyped makes it `null`. The joined severity is the highest any
   member names, and `null` when a `residual` member names none
   readably, unless a member witnessed one above `medium`, which a
   table's first-match stop above medium must still see. A member whose
   word is not `residual` is not a residual's witness: its readable
   `true` still dominates, and its silence makes nothing unknown, so an
   all-clean panel writes `none` and `false` as 0109 ruling 2 requires.
   A `null` fact matches no `when` (decision
   [0004](0004-strict-condition-evaluation.md) ruling 4), so the residual
   falls to whatever the table rules for the unknown, which in every
   shipped table that reads the pair is no route at all, and the run
   parks.

   **Enforcement binding:** `ReviewInputs::join` and `joined_flag` in
   `crates/brokkr-runtime/src/bundle/aggregate.rs`; tests in
   `crates/brokkr-runtime/src/bundle/aggregate/tests.rs` drive the panel
   through a table that ships a residual only on
   `has_security_residual: false` and `max_residual_severity_at_most:
   medium`: fourteen absent, null and mistyped shapes stop beside a clean
   member; every readable severity and flag is kept exactly; a safe
   residual member does not lend its facts to a malformed one in either
   order; a witnessed high, critical or security fact survives a
   sibling's unknowns; a clean member's witnessed security flag still
   reaches a residual panel; a hold dominates; and an all-clean panel
   keeps `none` and `false`.

## Consequences

- The amendment to 0008 ruling 3 is narrow: "security OR-ed" stays true
  for every witnessed `true`; what changes is that an unreadable fact on
  a `residual` member no longer reads as `false` or `none`.
- No shipped recipe changes. Under `fast`, `landing`, `verify` and
  `preflight` every residual route is earned by a value, so a `null`
  matches none and parks unmatched under 0004, where it once could ship
  as tracked debt. The frozen production table's `REVIEW-RESIDUAL-OK`
  carries no `when`, so a `residual` with `null` facts still reaches it,
  as it did when the facts read `false`.
- No contract, fixture, reference schema or witness digest changes:
  `transition/decided.inputs` is an open object and the aggregate already
  wrote both facts on every result.

## Alternatives

- **Refuse the residual member** as 0109 ruling 2 refuses a passing word.
  Rejected here: 0109 deliberately leaves a non-passing word's inputs to
  the table, and a residual whose facts are unknown is a fact the table
  can rule on (park, stop or rework) rather than a malformed result.
- **Keep reading unknown as safe** and ask each table to guard with a
  rule on the member evidence. Rejected: the table sees only the joined
  facts, so it cannot tell a witnessed `false` from a defaulted one.
