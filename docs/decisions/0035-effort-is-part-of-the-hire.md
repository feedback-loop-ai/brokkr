# 0035 — Effort is part of the hire: every model pin carries one, and the record carries it

Status: proposed
Date: 2026-09-03

## Context

Decision 0031 ruled that the served model is evidence and that every
model seat is pinned. It closed the gap between the plan ("which model
did we ask for") and the report ("which model the provider says
answered"). It left two things open. The smaller one is that its word
"evidence" claimed more than a provider's self-report can carry, which
ruling 2 below refines. The larger one is that decision 0034 then froze
the seat record around that word without closing the gap underneath it:
the model name alone does not identify the worker.

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

What the three model harnesses report, measured on 2026-09-03 against
codex-cli 0.153.0, `claude -p --output-format stream-json`, a real dsh
seat session, and live calls to the dsh routes' own providers — rather
than assumed:

| Fact | codex | claude | dsh |
|---|---|---|---|
| input | `input_tokens` | `input_tokens` | `inputTokens` |
| output | `output_tokens` | `output_tokens` | `outputTokens` |
| cache read | `cached_input_tokens` | `cache_read_input_tokens` | `cacheReadTokens` |
| cache write | `cache_write_input_tokens`, **dropped by the fold** | `cache_creation_input_tokens` | **dropped** |
| reasoning | `reasoning_output_tokens`, per turn | `output_tokens_details.thinking_tokens`, **result only** | **dropped** |
| cost | none | `total_cost_usd` | none |
| **effort** | **transcript only** | **transcript only** | **nowhere** |

Read that table as three separate findings, because they need three
different remedies.

Neither codex nor claude puts effort on the stream its adapter folds.
Both record it in the transcript instead — codex as `turn_context.effort`
and `thread_settings_applied.reasoning_effort`, claude as a top-level
`effort` beside each assistant record, both written per turn, so effort
can change inside one seat. Decision 0032 already retains the locator
that reaches either file, so the fact is addressable today without a new
mechanism.

Dsh is different in kind, and the difference is not the harness's models
being simple. Its providers report *more* than codex does: a live call
to `deepseek-v4-flash` returns `completion_tokens_details.reasoning_tokens`,
`prompt_tokens_details.cached_tokens`, and DeepSeek's own
`prompt_cache_hit_tokens`/`prompt_cache_miss_tokens`; `qwen3.8-flash`
through Model Studio returns reasoning and cached counts too. The dsh
session record this seat wrote keeps `inputTokens`, `outputTokens` and
`cacheReadTokens`, and discards the rest before brokkr can see any of
it.

That narrowing is the profile's, not the harness's, and the distinction
decides who fixes it. Dsh is plugin-based: a profile is an ordered stack
of cordis plugin bundles, and the ecosystem already covers this ground —
`@deepseek-ai/dsh-session-telemetry` is a first-party seam for
"session-event capture, projection, redaction, and handoff to a
reporting backend", and community plugins such as
`@laoyuehanni/dsh-token-usage` persist per-request model token usage
from a live hook. The `headless` profile brokkr boots simply loads none
of them. So the thin dsh record is a profile brokkr controls, not a
limit dsh imposes, and widening it is a change to our own profile rather
than a request to somebody else's harness.

Effort on those same dsh lanes is a real control that nothing reports
back. `qwen3.8-flash` honours it exactly — `enable_thinking: false`
returns no reasoning tokens at all, and `thinking_budget: 32` returns
precisely 32. `deepseek-v4-flash` accepts `reasoning_effort` and
consistently acts on it, but not in the direction its name implies:
over four samples of one prompt, `high` spent 24, 14, 14 and 18
reasoning tokens while `low` spent 160, 56, 100 and 167 — a sevenfold
inversion, reproducible, and unexplained here on purpose, because this
decision records measurements rather than theories about them. Neither
provider echoes the effort back in its response, and neither does dsh.

That inversion is the sharpest argument in this document. An operator
reading `--effort high` on a deepseek lane would conclude the seat
thought harder; the meter says it thought roughly a seventh as hard. A
pin is a request, and on at least one live lane the request does not
mean what it says. Only a reported number settles it, which is why
ruling 4 is not a nicety beside ruling 5 but the check that makes ruling
5 auditable.

Three alternatives were weighed and rejected. Filling `effort` from the
pin repeats exactly the move decision 0031 refused for the model:
internally tidy, evidentially false, and silent when a thread changes
effort mid-run. Waiting for the harnesses to put effort on the streams
their adapters fold leaves the fact unrecorded for as long as that
takes, and the journal is append-only — the window cannot be repaired
afterwards. Amending v1 in
place is not available at all: contracts are frozen by construction,
which is why ruling 7 adds a file rather than a field.

The operator ruled in chat on 2026-09-03: "the information is
meaningless without effort", "if we have a model, we need effort as
well", and "we do version / SHA pinning, explicit model, effort,
reasoning — that is core to our ledger and modus operandi."

## Rulings

1. **Explicit over implicit is the ledger's rule, stated once.**
   Brokkr pins what it depends on and records what came back: release
   versions and their digests, bundle and recipe digests, the concrete
   model, and — by this decision — the effort asked of it and the
   reasoning it spent. A fact that decides an outcome or a price is
   named in the plan and carried in the record, whether the record
   holds it as claim or as meter. Nothing that moves a bill is left to
   a default.

   **Enforcement binding:** this decision is the general rule the
   specific bindings below implement; it supersedes nothing and is
   cited where an implicit default would otherwise be argued for.

2. **A served value is the provider's claim, not a proof — decision
   0031's word "evidence" is refined here.** No harness discloses
   quantization, hardware routing, or a substitution made at peak load,
   and the same model string comes back whichever of those happened.
   `model` is the best-attested name available and remains worth more
   than a pin, so 0031's rule stands untouched: a pin, an adapter
   default, or an abstract agent model is never written into it. What
   changes is only what the field may be claimed to be. It is testimony
   from the party being audited, and this ledger does not call testimony
   proof.

   What survives that scrutiny is not a name but a meter. Tokens, money
   and elapsed time are costly to fabricate and are what settle a
   dispute; a label is free to assert. Decision 0034 froze the meters
   the record already had, ruling 4 adds the one it lacked, and ruling 3
   records beside it the configuration that meter audits. That is why
   they are load-bearing here rather than decorative.

   **Enforcement binding:** no code changes for this ruling. It governs
   how the `model` field is described in `docs/guides/driver-authoring.md`
   and in every readout that labels it, and it is the ruling cited when
   a future decision is tempted to treat a provider's self-report as
   settled fact.

3. **The record carries the configured effort, labelled as
   configuration.** A new `contracts/seat-record.v2.schema.json` adds
   `effort` to the per-turn checkpoint, the finishing checkpoint and the
   successful result. There is no served effort anywhere to carry
   instead: what codex writes as `turn_context.effort`, and claude
   beside each assistant record, is each harness echoing its own
   setting, not a measurement of what the model did. The field is
   therefore a configured fact, named as one and never dressed as a
   report — the way decision 0016 labels a model selection a selection.

   It is read from the harness's own echo where one exists, through the
   decision 0032 locator, because that echo is the effective value after
   every profile and plugin layer has applied — strictly better than
   reading back our own bundle, which is the move 0031 refused. Each
   writes it per turn, so either may change mid-thread. Dsh reports
   `not reported`: its lanes carry a real effort control, but neither
   the harness nor the providers behind it echo any value at all, so
   there is nothing to read — an absence the operator ruled on
   2026-09-03 after it was measured, not one assumed from a thin record.
   Exec reports `not applicable`. The two sentinels are decision 0031's,
   reused rather than reinvented, and the distinction between them is
   exactly the one dsh makes visible: a control that exists but goes
   unreported is not a control that does not exist.

   A configured effort is worth recording and is not worth trusting.
   `deepseek-v4-flash` is the lane that proves it, spending roughly a
   seventh as much reasoning at `high` as at `low`: the configuration
   said one thing and the meter said another, and only the meter could
   say so.

   **Enforcement binding:** the built-in folds in
   `brokkr-protocol::adapters`, driver conformance, the `brokkr-store`
   validator, and the shared `brokkr-view` derivation.

4. **The record carries the reasoning it paid for.** v2 adds
   `reasoning_output_tokens`, a reported subset of `output_tokens` in
   the way `cache_read_tokens` is a subset of `input_tokens`, and is
   never added to a total a second time. The three harnesses report it
   at different granularities and the field admits all three without
   inventing the others: codex reports it per turn, claude only in its
   result, and a dsh lane's providers report it per call though the
   profile discards it. Where a harness reports no per-turn figure the
   turn's value is absent, never zero and never back-filled from the
   run total. The codex fold additionally maps the
   `cache_write_input_tokens` it already receives onto the
   `cache_write_tokens` v1 defines and does not fill.

   **Enforcement binding:** the codex fold and its conformance shim;
   the view sums exactly as documented in
   `docs/guides/driver-authoring.md`.

5. **Every model pin carries an effort pin.** Where decision 0031
   ruling 2 requires a concrete `--model`, an effort-bearing driver
   also requires a concrete effort. In the agent library (decision
   0016) an agent's `models` chain gains a companion effort per
   candidate, so a charter names the effort it hires exactly as it
   names the model. Exec and effortless harnesses need no pin.

   **Enforcement binding:** bundle compilation scans the fully composed
   seat tree, reports every unpinned seat/member/step in one refusal
   alongside the existing model refusal, and gives the repair.
   `brokkr doctor --bundle` exposes the same refusal.

6. **The pin and the effective configuration remain separate facts,**
   as decision 0031 ruling 3 separates the selected model from the
   served one. The pin is what a bundle asked for; `effort` is what the
   harness reports it actually applied, after every layer. Both are
   configuration — neither is a measurement of what the model did, and
   ruling 4's reasoning count is the only figure in the record that is.
   Existing journal rows are not rewritten and their absence stays
   visibly absent.

   **Enforcement binding:** the view carries the pin and the applied
   effort as separate cells, labels both as configuration, and derives
   old-journal absence without fallback.

7. **v1 is not edited.** Contracts are frozen: v2 is a new file beside
   v1, and v1 remains the contract for every record already written
   under it. Records are validated against the version their run's
   engine wrote.

   **Enforcement binding:** `crates/brokkr-runtime/tests/frozen_contracts.rs`
   and the `contracts/README.md` table.

## Consequences

A reader of any seat record can tell who was hired and what the work
cost: the model claimed, the effort asked of it, the reasoning it
actually spent, and the money. The first two are claims and the last two
are meters, and the record now says which is which. The wager
becomes a comparison between stated hires rather than between names
that say nothing about the effort behind them.

Bundle and recipe digests move where effort pins are added, and
descendants whose base changed move with them. Frozen evaluator
fixtures and historical journals do not.

The wire protocol does not change: `effort` and
`reasoning_output_tokens` are additive payload facts under
`forge-driver/v1`, like `model` before them.

One finding here is left deliberately unruled. A dsh seat's record will
stay thinner than a codex or claude seat's even after v2, because the
`headless` profile loads no plugin that captures what its providers
return. That is a profile change of ours — dsh already ships the seam
and the ecosystem already ships the plugins — and it is a different
question from what this contract admits. Naming it is not fixing it: it
wants its own ruling, and this decision deliberately declines to make
that ruling here.

The cost of this ruling arriving late is one version. Decision 0034's
v1 was frozen on 2026-09-03 knowing effort was missing, because the
operator ruled that the seat's work should land as it was built rather
than be amended in flight. Every record written between that merge and
v2's landing names a model without its effort, and cannot be repaired
afterwards — the journal is append-only. That window is the reason this
decision is worth its own number rather than a footnote.
