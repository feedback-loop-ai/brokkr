# Design: an environment block parks for the operator

## Context

`blocked` is one word for two causes. The machine already has every part the fix needs: a closed input table where a choice input is one row (`drift_in` is the template), default declaration of the inputs a phase's rules read (decision 0007), the rule-driven park (decision 0022), and `brokkr operator retry` then `brokkr resume`, which runs the parked seat again with the earlier phases' results in its context. Decision 0091 rules how they are joined.

## D1 — A fact in `inputs`, not a result word

The seat says what blocked it in `inputs.blocked_by`, over `environment` | `work`, read by `blocked_by_in`. A second result word would change every seat's `results`, every charter's result list and every table that rules `blocked`; the fact changes one table row and leaves the result vocabulary alone. The vocabulary sits in `inputs.rs` beside its row, `pub(crate)`, since nothing outside the core reads it (decision 0071 ruling 6).

## D2 — The park above the stop, `work` falling to the stop

`IMPL-BLOCKED-ENVIRONMENT` parks on `blocked_by_in: ["environment"]` and precedes `IMPL-BLOCKED`, which stays the unconditional hard stop. `work` earns no rule of its own: it and an absent value take `IMPL-BLOCKED`, so a seat that forgets the fact stops, the fail-closed direction under decision 0004's absence law, and the rule a work block takes keeps its id. The sweep rules both values; a table test names the rule each takes, and absence, in every shipped table that rules a blocked implementer.

## D3 — One law for a park's problem

Today a park's problem is the rule's reason, except at triage's `escalate`, where the seat's non-empty notes are. The new park needs the notes too. The rule grammar is closed (`additionalProperties: false` in `contracts/phase-machine.v2.schema.json`), so a rule cannot declare it without a new contract version, and a second special case would restate escalate's (decision 0071 ruling 5). So every rule-driven park carries the seat's non-empty notes, else the rule's reason (`park_problem` in `engine/decide.rs`). Escalate reads as before. The fold's prefix keeps the rule's id in every park reason. The parks whose reasons already promised the notes (`REVIEW-RESIDUAL-NONE`, `REVIEW-REFORGE-EXHAUSTED-MEDIUM`, `CLARIFY-AMBIGUOUS-EXHAUSTED`) now deliver them.

## D4 — The retry needs nothing new

`blocked` is not in `FAILURE_RESULTS`, so the park spends no attempt and resets the failure count; `retry` moves the run to its seat's effect without entering the phase again, so `visits` does not move and no `returned_from` is added. The acceptance proof runs it through the real engine and fake driver at `max_attempts` 1.

## D5 — The charters

Each charter that teaches `blocked` gets one clause naming `inputs.blocked_by` and both words, with examples of each kind. The recipe copies of the intakeless charter are pinned to it byte for byte, so they move with it. A test holds every charter that teaches `blocked` to the clause.

## D6 — The unknown-condition refusal

The loader's refusal listed `strategy_in` by hand. It now lists every choice condition from `INPUTS`, so `blocked_by_in` appears, and `drift_in`, which the hand-written text left out, comes with it.
