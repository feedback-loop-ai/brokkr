# Change: an environment block parks for the operator instead of stopping the run (#789)

Status: proposed. The law is decision 0091, proposed; only the operator accepts it.
Change: `2026-10-20-789-environment-blocks-park`.
Parent story: [#789](https://github.com/feedback-loop-ai/brokkr/issues/789), part of epic [#790](https://github.com/feedback-loop-ai/brokkr/issues/790) (cheaper recovery).

## Why

A work seat that returns `blocked` routes to `IMPL-BLOCKED`, a hard stop, in `recipes/fast/policy.json` and in every recipe that extends it. A seat its environment blocked (a signing key, a tool off its `PATH`, a capability its box refused) is fixed by the operator in minutes, but the stop is terminal, so finished work needs a fresh run that repeats every earlier phase. Six swarmail runs on 2026-10-18 and 19 ended so, with their implementation complete and every gate green.

## What Changes

- **A typed blocker.** `blocked_by`, a seat-owned choice input over `environment` | `work`, read by the `blocked_by_in` condition: one row of the input table, checked at load, unreadable values parking, absence satisfying nothing.
- **The park above the stop.** `recipes/fast/policy.json` gains `IMPL-BLOCKED-ENVIRONMENT`, a park on `blocked_by_in: ["environment"]`, above the unchanged `IMPL-BLOCKED`. A work block and an unnamed one still stop hard. Every recipe that extends `fast` inherits both.
- **A park carries the seat's notes.** Every rule-driven park journals the parked seat's non-empty notes as its problem, else the rule's reason; triage's `escalate` was the one case that did so before.
- **The charters.** The six charters that teach `blocked`, and the two recipe copies pinned to the intakeless one, tell the seat to set `inputs.blocked_by` and name both words.
- **The loader's unknown-condition refusal** lists every choice condition from the one table.

## Impact

- `crates/brokkr-core`: `policy/inputs.rs` (`BLOCKERS`, the `blocked_by` row, `choice_conditions`), `policy/errors.rs` (the refusal's text); tests in `policy/tests.rs` and `policy/audit/tests.rs`.
- `crates/brokkr-runtime`: `engine/decide.rs` (`park_problem`); tests in `engine/tests.rs`, `tests/table_lints.rs`, `tests/library_data.rs`; `tests/witnesses.json` re-blessed.
- `crates/brokkr-cli`: `tests/machine_proof/environment_block.rs`, the acceptance proof.
- Data: `recipes/fast/policy.json`; `agents/charters/` (six); `recipes/standby/roles/implementer.md` and `recipes/wager-harness/roles/implementer.md`.
- Docs: decision 0091 and its index row, 0022's back-pointer, `docs/guides/recipe-authoring.md`, `docs/guides/quickstart.md`, `ARCHITECTURE.md`, `recipes/fast/README.md`, `recipes/night-shift/README.md`.
- No contract, `policy/phase-machine.json`, `reference/` or `fixtures/` edit; no public-API item; no operator verb.
- Specs: `blocked-seat-routing`, added.
