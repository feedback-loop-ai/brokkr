# 0092 — An environment block parks for the operator, and a work block routes as before

Status: proposed
Date: 2026-10-20

Built: built
Amends: 0022

## Context

Issue #789, part of epic #790 (cheaper recovery). A work seat that returns
`blocked` routes to `IMPL-BLOCKED` in `recipes/fast/policy.json`, a hard
stop, and every shipped recipe that admits `blocked` inherits that rule by
`extends`. Two different causes share the word. A seat its **environment**
blocked (a signing key it cannot reach, a tool off its `PATH`, a capability
its box refused) is the operator's to fix in minutes, after which the same
seat could simply run again. A seat the **work** blocked (a contradictory
framing, a specification problem) needs judgment. Because a stop is
terminal, an environment block today costs a fresh run that repeats every
earlier phase. Six swarmail runs on 2026-10-18 and 19 ended at
`IMPL-BLOCKED` on an unreachable signing key with their implementation
complete and every gate green, and each needed a re-drive.

The pieces already exist:

- The input vocabulary is one table, `INPUTS` in
  `crates/brokkr-core/src/policy/inputs.rs` (#345). `drift_in` is a
  seat-owned choice over a closed vocabulary: every value a table lists is
  checked at load, a present value outside the vocabulary parks the
  evaluation (decision 0004), and absence never satisfies.
- A seat declares by default the seat-owned inputs its phase's rules read
  (decision 0007, `Machine::seat_inputs`), so a rule in `fast` that reads a
  new input is declared for every implement seat with no agent edit.
- A rule may rule a park (decision 0022). `brokkr operator retry` moves a
  parked run back to its seat's effect, and `brokkr resume` runs that seat
  again with the earlier phases' results in its `context.results`.
  `blocked` is not a failure word, so the park spends no attempt and no
  failure count.
- A rule-driven park's problem is the rule's reason, except at triage's
  `escalate`, where the seat's non-empty notes are the reason
  (decision 0041 ruling 6). The fold prefixes the rule's id:
  `<RULE> for (<phase>, <result>): <problem>`.

Weighed and set aside:

- **A second result word** (`blocked-environment`). Every seat's `results`
  list and every charter's result list would change, and every table that
  rules `blocked` would need a rule for the new word. A typed fact inside
  `inputs` leaves the result vocabulary as it is.
- **A rule field that says the park carries notes.** The rule grammar of
  `contracts/phase-machine.v2.schema.json` is closed
  (`additionalProperties: false`), so that is a new contract version. One
  law for every rule-driven park needs none.
- **A second special case beside escalate's** in the engine. Decision 0071
  ruling 5: the one exception becomes the one rule.

No contract moves. `contracts/seat-record.v6.schema.json` keeps
`result.inputs` an open object; the phase-machine contracts bind no park
problem text, and `recipes/triage/policy.json` already ships choice
conditions (`drift_in`, `strategy_in`) under `forge.phase-machine/v2`. No
new contract version file is added, and `policy/phase-machine.json`, the
frozen `v1` table with no park vocabulary, keeps its stop.

## Rulings

1. **A blocked seat names what blocked it.** `blocked_by` is a seat-owned
   choice input over the closed vocabulary `environment` | `work`, read by
   the condition `blocked_by_in`, added as one row of `INPUTS`. A table
   that lists a value outside the vocabulary is refused at load naming the
   rule and the vocabulary; a present value outside it, or a value that is
   not a word, parks the evaluation as unreadable; an absent value
   satisfies no condition. The seat result vocabulary stays `complete`,
   `broken`, `blocked`, `oversized`.

   **Enforcement binding:** `parse_condition` and `conditions_met` in
   `crates/brokkr-core/src/policy.rs` over the `INPUTS` row;
   `crates/brokkr-core/src/policy/tests.rs`
   (`blocked_by_in_has_one_table_arm_for_every_blocker`, the unknown-key
   text pin in `every_malformed_table_reads_as_it_always_has`);
   `crates/brokkr-core/src/policy/inputs/tests.rs` for the row's reader,
   owner and default declaration.

2. **An environment block parks; a work block routes as before.**
   `recipes/fast/policy.json` rules `(implement, blocked)` with
   `IMPL-BLOCKED-ENVIRONMENT`, a park on `blocked_by_in: ["environment"]`,
   placed above `IMPL-BLOCKED`, which stays the unconditional hard stop. A
   `work` value and an absent one fall to the stop, so a seat that omits
   the fact stops, the fail-closed direction. Order holds under decision
   0050 ruling 2 (a conditional rule before an unconditional one is not
   subsumed, and the reverse order deadens the park and is refused at
   load); presence holds under ruling 1 (a park does not advance). Every
   recipe that extends `fast` inherits both rules; none copies them.

   **Enforcement binding:** `crates/brokkr-runtime/tests/table_lints.rs`
   (`a_blocked_implementer_is_ruled_by_what_blocked_it`, over every shipped
   table that rules a blocked implementer, and the per-table sweep pins);
   `crates/brokkr-cli/tests/diagrams.rs` holds `ARCHITECTURE.md`'s phase
   diagram to the table.

3. **A rule-driven park carries the parked seat's notes.** The problem a
   rule-driven park journals is the seat's own non-empty `notes`, and the
   rule's reason when the seat left none. Escalate's park reads as it
   always has; it is now the rule's first case instead of its exception.
   The other shipped parks that promise the seat's notes
   (`REVIEW-RESIDUAL-NONE`, `REVIEW-REFORGE-EXHAUSTED-MEDIUM`,
   `CLARIFY-AMBIGUOUS-EXHAUSTED`) now deliver them. The fold's prefix
   keeps the rule's id in every park reason, and `ParkReason` reads it as
   it reads any stated reason. This amends decision 0022, whose park
   journaled the rule's reason as its problem.

   **Enforcement binding:** `park_problem` in
   `crates/brokkr-runtime/src/engine/decide.rs`;
   `decide_covers_schema_no_rule_review_head_and_ship_drift` in
   `crates/brokkr-runtime/src/engine/tests.rs` (notes carried, empty notes
   keep the reason); `crates/brokkr-cli/tests/machine_proof.rs`
   (`triage_routes_chore_and_escalation_parks_with_its_reasoning`).

4. **The retry re-runs that one seat, its earlier results intact.** No
   operator verb is added: `brokkr operator retry` then `brokkr resume`
   runs the parked seat again, with the earlier phases' results in
   `context.results`, no earlier phase entered again, and no attempt or
   failure count spent. Every shipped charter that teaches `blocked` tells
   its seat to set `inputs.blocked_by` and names both words.

   **Enforcement binding:** `crates/brokkr-cli/tests/machine_proof/environment_block.rs`
   (an environment block parks with the seat's notes and its retry
   completes at `max_attempts` 1 with intake entered once and the re-run
   seat handed intake's result; a work block and an unnamed block stop at
   `IMPL-BLOCKED`); `every_charter_that_teaches_blocked_names_its_kind` in
   `crates/brokkr-runtime/tests/library_data.rs`.

## Consequences

- An operator fixes an environment block and retries one seat instead of
  re-driving a run. A work block, a seat that names no blocker, and every
  table that does not read `blocked_by` behave exactly as before.
- Every table that inherits `fast` sweeps one valuation more: 47 for the
  fast-shaped tables, 1,073 for the triage-shaped ones, 48 for `self` and
  `panel-review`, 49 for `landing`, each with nothing unruled. The pins in
  `table_lints.rs` and the core's audit tests move with it, and the
  manifest and table witnesses of every recipe extending `fast` and the
  six charters' digests are re-blessed.
- The loader's unknown-condition refusal lists every choice condition from
  the one table, so `drift_in` appears in it beside `strategy_in` and
  `blocked_by_in`.
- A seat whose recipe lists its `inputs` itself must list `blocked_by` for
  a `blocked_by_in` rule to read it; compilation refuses the omission
  (decision 0007), as for any declared input.
- `docs/guides/recipe-authoring.md` says how a recipe declares the
  distinction: the input, the park above the stop, and the charter.
