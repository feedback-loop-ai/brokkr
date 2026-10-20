# Change: a retry by member re-runs only the named panel members and carries the others' settled verdicts (#787)

Status: proposed. The law is decision 0092, proposed by this change; only the operator accepts it.
Change: `2026-10-20-787-retry-one-member`.
Parent story: [#787](https://github.com/feedback-loop-ai/brokkr/issues/787), part of epic [#790](https://github.com/feedback-loop-ai/brokkr/issues/790) (cheaper recovery).

## Why

`panel_outcome` (`crates/brokkr-runtime/src/engine/panel.rs`) parks the whole attempt when any member of a panel is indeterminate, and `brokkr operator retry` re-runs the phase as a new attempt with every member. The verdicts that settled are discarded although nothing they judged has changed: the reviewed head, the pinned bundle and each member's inputs are the same bytes. On 2026-10-19 one reviewer of a four-member panel ran 141 turns and wrote no verdict, and recovering it re-ran all four, about forty minutes for one stuck seat.

The pieces already exist. The journal records the head the protected phase was entered at (`phase/entered.head`, decision 0039) and the digest of every effect's requested input (`effect/requested.input_digest`), which the engine rebuilds and compares before it executes. `operator/commanded` carries an `args` object the fold never reads, which decision 0047 already uses for structured verb arguments. `effect/started` carries per-member entries keyed by the member's tag (`provenance`, decision 0016; `boundary`, decision 0046 ruling 3), and `effect/indeterminate` carries per-member evidence of what is not proven over (`unresolved_members`, #403). The aggregate is a pure function of the members' result objects (`Aggregate::aggregate_results`). What is missing is a verb that names the members to run, a record of the verdicts that settled in an attempt that ended short of success, a rule that says when such a verdict may be carried, and a readout that names it.

## What Changes

- **A retry names the members that run** (decision 0092 ruling 1). `brokkr operator retry --member <name>`, repeatable, journals the names on its `operator/commanded` as `args.members`. The fold does not read them; the engine reads them for the one effect the acceptance requests, and for every bounded attempt of that effect. A plain retry journals `args: {}` and re-runs every member, byte for byte as today. `--member` belongs to `retry` alone and names a member as the panel declares it.
- **A settled verdict is carried only when nothing it judged has changed** (ruling 2). Three facts must be byte-identical: the reviewed head, read as the parked phase's latest `phase/entered.head` against the repository's head now; the pinned bundle, held by the manifest gate `resume` applies before it builds any input; and the member's inputs, the seat input rebuilt for the source attempt's effect digesting to the `input_digest` that effect recorded. The verb checks the head and the journal's facts when the command is given and refuses with the reason, writing nothing. The engine checks all three before anything of the attempt spawns; a carry that no longer holds there starts the attempt on no driver and ends it `effect/indeterminate` with the marker `PANEL-CARRY-REFUSED` and its evidence, so the run parks for the operator and a plain retry re-runs the whole panel.
- **What is not carried refuses by name** (ruling 2). A member that never settled, indeterminate or failed, is not carried: left unnamed, the command refuses naming it. A name the panel does not have refuses naming the panel's members. A parked phase whose latest attempt ran no panel, whose latest attempt succeeded, or whose latest entry recorded no head refuses by that fact. A journal written before this change records no settled verdict and refuses the same way.
- **The carrying attempt aggregates as a fresh panel** (ruling 3). Only the named members run; every other member's verdict is carried from the attempt it settled in. The declared aggregate joins carried and fresh verdicts exactly as it joins a fresh panel's, so the result, its inputs and its notes are the bytes a full re-run produces on the same verdicts, and policy routes them unchanged: a residual or a finding from the retried member goes where the table sends it today. A fresh member that is indeterminate parks the attempt; one that fails fails it, and decision 0006's bounded retry of that effect runs the named members again with the same carry.
- **The journal names every carried verdict** (ruling 4). `effect/started` carries `carried`: one entry per carried member with its tag and the attempt its verdict settled in. Every panel attempt that ends short of success records each settled member's result under `settled_members` on its `effect/indeterminate` or `effect/failed`, beside `unresolved_members`. Both fields and `args.members` are optional, absent-by-default extension fields at `event_schema: 1`, published as `contracts/panel-carry.v1.schema.json` under the amended rule in `contracts/README.md`: no new event type, no seat-record version, nothing the fold reads, and an attempt that carries nothing journals byte-identical payloads. The member marker `panel-member-finished` is written only for members that ran.
- **Every readout names a carried verdict as carried** (ruling 5). `brokkr-view` derives it once from `carried`: the carried member's participant stands at status `carried` and names the attempt its verdict came from; `brokkr runs`, `inspect`, `seats`, the TUI, `ui.html` and the console paint it and derive nothing of their own.

## Capabilities

### New Capabilities

- `panel-member-retry`: the verb and the carry rule: what a retry names, when a verdict is carried, what refuses by name, and how the carrying attempt runs and aggregates.
- `panel-attempt-record`: what the journal records of a panel attempt that ends short of success and of an attempt that carries, and how every readout names a carried verdict.

### Modified Capabilities

- None. `operator-console-commands` keeps offering the plain retry; the console's Retry sends no members (see Not changed).

## Not changed

- The plain `brokkr operator retry`, the Looper bridge's fenced `retry` and the console's Retry button: each journals `args: {}` and re-runs every member; their bytes, their fence and their refusals are as today.
- The fold (decision 0050 ruling 6): no new event type, no new cursor, no arm reads a new field. A carry refused at drive time parks by the `close` arm that parks every indeterminate.
- `panel_outcome`'s rule: any indeterminate member parks, any failed member fails, else the aggregate; `Aggregate::aggregate_results` and both aggregates' vocabularies.
- `effect/requested`, its `input_digest` and `seat_input`: a carrying effect requests the same input a full panel does, so recovery rebuilds and compares it as it always has.
- The seat-record contract (`seat-record.v6`), the frozen contracts, `policy/phase-machine.json`, `reference/`, `fixtures/`, every recipe and every charter.
- Decision 0006's bounded attempts and its #403 addendum, 0039's recorded head and 0047's `args` precedent stand; 0092 amends none of them.

## Impact

- `crates/brokkr-runtime`: the carry rule and its typed refusal (`thiserror`, every text pinned once) in a module of the engine, called by the verb and by dispatch; `engine/operator.rs` stands at its 800-line ceiling, so the verb's member form lands in a submodule beside `conclusion`, `receipt` and `supersede`; `engine/dispatch.rs` (the drive-time check before `effect/started`, the `carried` entries beside `provenance` and `boundary`), `engine/panel.rs` (`member_runs` over the fresh members, `panel_outcome` over carried and fresh verdicts, markers for the fresh, `settled_members` on both terminal payloads), `engine/sequence.rs` (the panel step and its tag). `engine.rs` is over its ceiling and may not grow: the settled members ride a type in a module, not a new field written there.
- `crates/brokkr-cli`: `--member` (repeatable) and `--repo` (retry only, default the current directory as `resume` reads it) on the operator verb; `cli_args.rs` stands seven lines under its ceiling, so the arguments land in a module; `verbs/delivery.rs` passes them. The acceptance proofs land in a `tests/machine_proof/` file over a git-initialised workspace (today's workspace records no head), because `machine_proof.rs` is over the test ceiling.
- `crates/brokkr-view`: the scan reads `carried` as it reads `provenance` and `boundary`; `Participant` gains `carried_from` and the status word `carried`; `VIEW_VERSION` moves; `lib.rs` is over its ceiling, so the reader lands in a module. `crates/brokkr-cli/src/render.rs` tones `carried` as good.
- Contracts: `contracts/panel-carry.v1.schema.json`, added; `contracts/README.md` gains its paragraph under the amended additive rule. No frozen byte moves.
- Docs: decision 0092 (`Status: proposed`) and its index row; `docs/guides/quickstart.md`'s operator commands; `ARCHITECTURE.md` where it describes the panel and the operator's verbs.
- Public API: `brokkr-view`'s participant and `brokkr-runtime`'s verb move the snapshots under `quality/public-api`; the raised baseline is the operator's ruling, cited in the pull request.
- Beside this change: PR #802 (#789, decision 0091) touches the policy inputs, `engine/decide.rs`, the fast recipe and the decisions index. Nothing above overlaps it but the index, where both add a row after 0090's.
- Specs: `panel-member-retry` and `panel-attempt-record`, added.

## Decisions

### S1 — One rule, two observers

The carry rule is one pure function over typed facts: the parked attempt's members and each one's outcome, its settled verdicts and its carried entries, the phase's recorded head, the head observed, and the names given. The verb observes the journal and the repository and decides; the engine observes the same, and the rebuilt input digests besides, and decides with the same function (decision 0071 rulings 5 and 10: observing is split from deciding, and the rule has one home). The verb alone was refused because a head moved between the acceptance and the drive would then be carried, a fail-open. The engine alone was refused because the issue asks the command to refuse, and the operator would otherwise learn of the refusal two commands later. The inputs condition is the engine's alone: the verb compiles no bundle, the manifest gate stands before any input is built, and that condition can only differ at drive time.

### S2 — A carry refused at the command writes nothing

Decision 0047 ruling 2 is the precedent: a verb that refuses before it commands leaves the journal as it was, and the refusal is the operator's to read at the terminal. The fold's own refusals (not parked, no phase to retry, a lost fence) keep their journaled `operator/rejected`, because those dispose of a command already written. Journaling the refused member retry was weighed and set aside: `operator/rejected.reason` is a closed vocabulary every readout maps to one sentence, and a refusal that names a member or two heads does not fit a word. A later ruling can add the record if the console needs it.

### S3 — A carry refused at drive time parks through an indeterminate

The attempt starts on no driver (`driver: "none"`, as `refuse_changed_input` already does when the rebuilt input moved) and ends `effect/indeterminate` with a marked reason, as `GATE-MOVED-HEAD` marks a gate that moved the head, so the fold parks by its existing arm and nothing is auto-retried: decision 0006 keeps an indeterminate for the operator. Running the whole panel instead was refused as silent: it spends exactly what the operator asked not to spend. A `run/parked` written at `ExecuteEffect` with no attempt was refused: the relation admits it, but it leaves a requested effect with no terminal fact, which every reader of effects would have to learn.

### S4 — A carried verdict is named on `effect/started`, not on the member marker

The marker `panel-member-finished` is a closed seat record (`contracts/seat-record.v6.schema.json`, `additionalProperties: false`); naming the source there needs a seventh seat-record version for one field. `effect/started` already carries per-site entries keyed by the same tag, `provenance` and `boundary`, and the view reads them per member; `carried` is a third list of that shape. A carried member writes no marker in the carrying attempt: nothing of it ran, and a marker saying `succeeded` with no model, no session and no checkpoints would be a half-truth the readouts would then have to unlearn. Its evidence stands in its source attempt, which the entry names.

### S5 — The source is read from the parked attempt alone

A member's verdict is carried from the parked attempt when it settled there, or from the attempt the parked attempt's own `carried` entry names, so "the attempt it settled in" is exact and no walk over the phase's attempts is needed. A member that settled in an earlier attempt, was named again and did not settle is not carried: the operator named it to judge again, and refusing by name is the safer reading. The parked attempt is the parked phase's latest attempt in its current visit; an earlier visit judged an earlier head and is never a source.

### S6 — `settled_members` rides both terminal facts short of success

A park reached by exhausted failed attempts (decision 0006) has a failed attempt as its latest, and its settled verdicts are as sound as an indeterminate attempt's; the record is one type written on both. `effect/succeeded` records nothing new: a panel that succeeded has no park to be retried from, and a rule-driven park after a success re-runs the whole panel (S8).

### S7 — The reviewed head is the phase's entry head

Decision 0039 records the protected phase's head at entry. A retry re-enters no phase, so the latest entry is the one every attempt of the visit judged, and the gate-moved-head check proves no gate moved it. The recorded head is checked against the contract's commit-id shape before git sees it, as `fixes_docs_only` checks it. A phase that recorded no head, a work panel's or a run with no repository, refuses the carry; recording a head for every phase entry is 0039's field and a ruling of its own.

### S8 — Left out, named

- A member retry through the Looper bridge and the console's Retry button: both stay the plain retry.
- Carrying a sequence's single step: the chief of a review sequence is a step, not a member, and naming it is refused; a sequence parked at the chief carries its panel step's settled verdicts when a member of that step is named, and the chief runs again.
- A park after a succeeded panel attempt (a rule-driven park such as `REVIEW-RESIDUAL-NONE`): nothing is recorded to carry, and the plain retry stands.
- A work panel in a phase that records no head: refused by name (S7).
- Carrying a member's model, session and checkpoints into the carrying attempt's row: the readouts name the source attempt instead.
- Carrying across phase visits.

### S9 — Names

`--member` takes the member's name as the panel declares it. In a sequence the journal's tag is `<step>:<member>`, derived from the parked attempt, and the tag spelled whole is accepted too; a bare name two panel steps of one attempt share is refused naming both tags.
