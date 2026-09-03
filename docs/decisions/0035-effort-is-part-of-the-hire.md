# 0035 — Effort is part of the hire: every model pin carries one, and the record reports it

Status: proposed
Date: 2026-09-03

## Context

Decision 0031 ruled that the served model is evidence and that every
model seat is pinned. It closed the gap between the plan ("which model
did we ask for") and the evidence ("which model answered"). It left a
second gap open, and decision 0034 froze the seat record without
closing it: the model name alone does not identify the worker.

A reasoning model is not one worker. `gpt-5.6-sol` at minimal effort
and `gpt-5.6-sol` at xhigh differ in latency, in price, and in what
they can finish — as different from each other as two models are. A
record that reports the name and withholds the effort reports half a
hire, and the half it withholds is the half that moves the bill.

This is not hypothetical, and the first instance is already in the
journal. Run `the-seat-record-is-a-contract-th-fab25c33` — the run that
built decision 0034's own contract — was served by `gpt-5.6-sol` at
**xhigh**. Its thread says so on every turn. Its seat record cannot say
so at all, because there is no field for it to say it in. The wager
compares seats by cost and outcome; two wager runs at different efforts
are not comparable, and nothing in the journal would tell a reader
which one they were reading.

What the harnesses report, measured against codex-cli 0.153.0 on
2026-09-03 rather than assumed:

| Fact | Where codex puts it |
|---|---|
| `input_tokens`, `cached_input_tokens`, `output_tokens` | `turn.completed.usage`, read by the fold today |
| `cache_write_input_tokens` | `turn.completed.usage`, **dropped** — though 0034 defines `cache_write_tokens` |
| `reasoning_output_tokens` | `turn.completed.usage`, **dropped** — 0034 has no field for it |
| reasoning effort | **not on the `--json` stream at all**; in the thread record as `turn_context.effort` and `thread_settings_applied.reasoning_effort`, written per turn |

Two consequences follow from that table. Codex already reports two
accounting facts the record either defines and ignores or does not
define. And effort is reported by the harness, but in the thread, not
the stream — which decision 0032 already makes addressable, since the
adapter holds the transcript locator.

Three alternatives were weighed and rejected. Filling `effort` from the
pin repeats exactly the move decision 0031 refused for the model:
internally tidy, evidentially false, and silent when a thread changes
effort mid-run. Waiting for codex to put effort on the `--json` stream
leaves the fact unrecorded for as long as that takes, and the journal is
append-only — the window cannot be repaired afterwards. Amending v1 in
place is not available at all: contracts are frozen by construction,
which is why ruling 6 adds a file rather than a field.

The operator ruled in chat on 2026-09-03: "the information is
meaningless without effort", "if we have a model, we need effort as
well", and "we do version / SHA pinning, explicit model, effort,
reasoning — that is core to our ledger and modus operandi."

## Rulings

1. **Explicit over implicit is the ledger's rule, stated once.**
   Brokkr pins what it depends on and records what it was served:
   release versions and their digests, bundle and recipe digests, the
   concrete model, and — by this decision — the effort and the
   reasoning the model spent. A fact that decides an outcome or a price
   is named in the plan and reported in the evidence. Nothing that
   moves a bill is left to a default.

   **Enforcement binding:** this decision is the general rule the
   specific bindings below implement; it supersedes nothing and is
   cited where an implicit default would otherwise be argued for.

2. **The record carries the served effort.** A new
   `contracts/seat-record.v2.schema.json` adds `effort` to the per-turn
   checkpoint, the finishing checkpoint and the successful result. The
   value is the harness's own report, never the pin: codex reads it
   from its thread record via the decision 0032 locator, where it is
   written per turn and may change mid-thread. A harness with an effort
   control that does not report one reports `not reported`; a harness
   with no such control, and exec, report `not applicable` — the two
   sentinels of decision 0031, reused rather than reinvented.

   **Enforcement binding:** the built-in folds in
   `brokkr-protocol::adapters`, driver conformance, the `brokkr-store`
   validator, and the shared `brokkr-view` derivation.

3. **The record carries the reasoning it paid for.** v2 adds
   `reasoning_output_tokens`, a reported subset of `output_tokens` in
   the way `cache_read_tokens` is a subset of `input_tokens`, and is
   never added to a total a second time. The codex fold additionally
   maps the `cache_write_input_tokens` it already receives onto the
   `cache_write_tokens` v1 defines and does not fill.

   **Enforcement binding:** the codex fold and its conformance shim;
   the view sums exactly as documented in
   `docs/guides/driver-authoring.md`.

4. **Every model pin carries an effort pin.** Where decision 0031
   ruling 2 requires a concrete `--model`, an effort-bearing driver
   also requires a concrete effort. In the agent library (decision
   0016) an agent's `models` chain gains a companion effort per
   candidate, so a charter names the effort it hires exactly as it
   names the model. Exec and effortless harnesses need no pin.

   **Enforcement binding:** bundle compilation scans the fully composed
   seat tree, reports every unpinned seat/member/step in one refusal
   alongside the existing model refusal, and gives the repair.
   `brokkr doctor --bundle` exposes the same refusal.

5. **Configured and served effort remain separate facts,** exactly as
   decision 0031 ruling 3 separates the selected model from the served
   one. The pin is the plan; `effort` is evidence. Existing journal
   rows are not rewritten and their absence stays visibly absent.

   **Enforcement binding:** the view carries the pin and the served
   effort as separate cells and derives old-journal absence without
   fallback.

6. **v1 is not edited.** Contracts are frozen: v2 is a new file beside
   v1, and v1 remains the contract for every record already written
   under it. Records are validated against the version their run's
   engine wrote.

   **Enforcement binding:** `crates/brokkr-runtime/tests/frozen_contracts.rs`
   and the `contracts/README.md` table.

## Consequences

A reader of any seat record can tell who worked: the model, the effort
it worked at, the reasoning it spent, and what that cost. The wager
becomes a comparison between stated hires rather than between names
that may hide a tenfold difference in effort.

Bundle and recipe digests move where effort pins are added, and
descendants whose base changed move with them. Frozen evaluator
fixtures and historical journals do not.

The wire protocol does not change: `effort` and
`reasoning_output_tokens` are additive payload facts under
`forge-driver/v1`, like `model` before them.

The cost of this ruling arriving late is one version. Decision 0034's
v1 was frozen on 2026-09-03 knowing effort was missing, because the
operator ruled that the seat's work should land as it was built rather
than be amended in flight. Every record written between that merge and
v2's landing names a model without its effort, and cannot be repaired
afterwards — the journal is append-only. That window is the reason this
decision is worth its own number rather than a footnote.
