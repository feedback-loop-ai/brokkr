# 0121 — A review panel's scope and fixes flags are read, never defaulted

Status: proposed
Date: 2026-11-23

Built: built
Amends: 0008

## Context

Two records describe how a `review-panel` aggregate joins its members'
flags. Decision [0008](0008-second-wave-scope.md) ruling 3 named the
join "security/fixes OR-ed", and decision
[0041](0041-one-office-per-seat.md) ruling 5(c) has the aggregate OR a
member's `spec_defect`. The join on main does neither for a value it
cannot read, and does not OR `fixes_applied` at all:

- `fixes_applied` is written `false` on every panel aggregate, whatever
  the members said, on the ground that judges never fix (0041 ruling 4).
  A member that reports `fixes_applied: true` is overwritten. No shipped
  recipe declares the input, so shipped runs never see it; a bundle that
  declares it (the adopter below, the frozen production table's
  `REVIEW-CLEAN-NO-FIXES`) is told no fixes were applied by a panel one
  of whose members said otherwise.
- `spec_defect` is `true` when any member wrote `true`, and `false`
  otherwise, so a member that wrote `"true"`, `1` or `null` reads as no
  spec defect.

Decision [0109](0109-a-gate-s-re-visit-reads-what-changed.md) ruling 2
holds a passing word only to the residual pair and leaves
`spec_defect` and `fixes_applied` to "read as today". So does decision
[0120](0120-a-review-panel-joins-a-residual-members-unreadable-safety-fact-as-unknown.md),
for the safety facts of a residual member.

The adopter `feedback-loop-ai/swarmail` carries a patch for this on
brokkr 0.12.0 (`tooling-patches/0002-preserve-review-scope-and-fixes.patch`,
release `swarmail-engine-a9893b4-p4`): its review table ships a residual
only on `spec_defect: false` and `fixes_applied: false`, routes a panel's
witnessed fixes to re-verification, and stops on anything it cannot
read. It asks for the change upstream so it can pin a stock release
(feedback-loop-ai/swarmail#405). The same patch's other half, a clean
verdict that carries a residual refused at the panel and at the
protected phase, is 0109 ruling 2 on main and is not restated here.

## Rulings

1. **A member's scope and fixes flags are read as written.** The panel
   reads each member's `fixes_applied` and `spec_defect` as a boolean
   when it is one, as `false` when the member did not write it, and as
   unknown when it wrote anything else (`null`, a string, a number, an
   array or an object). The joined flag is `true` when any member
   witnessed `true`, `false` when every member's reads `false`, and
   `null` otherwise, which no `when` matches (decision
   [0004](0004-strict-condition-evaluation.md) ruling 4). A member's
   witnessed `fixes_applied: true` is never overwritten.

   A flag left out reads `false` because that is the charter's shape:
   every member charter writes only the residual pair (0109 ruling 2),
   and the chief or the table owns `spec_defect`. Reading an absent flag
   as unknown, which the adopter's patch does, would turn every shipped
   panel's `spec_defect` to `null` and change what triage's chief is
   handed; the operator may rule that instead (see Alternatives).

   **Enforcement binding:** `scope_flag` and `joined_flag` in
   `crates/brokkr-runtime/src/bundle/aggregate.rs`; tests in
   `crates/brokkr-runtime/src/bundle/aggregate/tests.rs`: every
   combination of eight shapes for each flag, in a `clean` and a
   `residual` member at every position of a panel of one, two and four,
   joins as this ruling reads it and routes through a table that ships
   only explicitly-false flags; a witnessed `true` survives a sibling's
   unknown in every order; and members that write only the residual
   pair, or no readable inputs, leave both flags `false`.

## Consequences

- A table that declares `fixes_applied` now sees a member's witnessed
  `true`, and routes it as the table says (re-verification, under the
  production table's `REVIEW-CLEAN` and the adopter's), where it once
  shipped past it. Judges still never fix (0041 ruling 4), and a gate
  that moves the head still parks (`GATE-MOVED-HEAD`); this ruling only
  stops the aggregate from contradicting its members.
- A malformed flag now reads `null` where it read `false`. Under every
  shipped table that reads `spec_defect` (triage's, for the design and
  engine strategies) the rules that read it ask for `true`, so a `null`
  matches none of them, as `false` did; the review falls to the rules
  after them, as before.
- No shipped recipe declares `fixes_applied`, so no shipped run's
  decision changes from the first consequence.
- No contract, fixture, reference schema or witness digest changes.

## Alternatives

- **Read an absent flag as unknown too** (the adopter's patch as
  written). It would make the join's "unknown never ships" rule complete
  for bundles whose members are told to write both flags, as the
  adopter's are. Not recommended as the default: it changes every
  shipped panel's `spec_defect` to `null`, since the shipped member
  charters do not write it. An opt-in, for instance holding a member to
  the flags its hired agent declares the way 0109 ruling 2 holds it to
  the residual pair, would give the adopter the complete rule without
  that cost; it is left to the operator.
- **Keep `fixes_applied` constant `false`.** Rejected: an aggregate that
  contradicts its members is a record that cannot be trusted, and the
  cost of passing a witnessed `true` through is only a re-verification.
