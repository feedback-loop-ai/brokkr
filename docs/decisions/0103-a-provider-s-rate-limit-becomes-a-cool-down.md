# 0103 — A provider's pre-session rate limit becomes a cool-down, not a park

Status: proposed
Date: 2026-10-30

Built: unbuilt (#430)

## Context

This is #430's fourth slice, under decision 0068 ruling 4: a provider's
pre-session rate limit is a cool-down recorded for that provider and route,
new admissions are held to it, a one-turn probe answers at its end under
decision 0006's attempt rules, and "the run's journal shows the wait as a
wait, with the provider's own words, not as a park for a person. A run parks
for the operator only when the provider's refusal is not a rate limit or the
cool-down exceeds a declared maximum." Slice 3 is built (decision 0095:
#865, #876, #884, #886, #889); #430's slices 4 to 6 are open, and slice 6's
standing dispatcher is not built.

The evidence is the controller's, from the house journal (not opened here).
Since 2026-09-23 there are 5 `run/parked` events with `provider refused
before the first turn: rate_limit`, in 5 runs, 2026-10-21 11:42Z to
2026-10-26 17:42Z. All were claude seats on the account's usage limit; each
sat until a person retried it, and the feeder launched the next story into
the freed lane, which parked the same way. The dsh seats are unclassified: a
DashScope token-plan 429 (from 2026-10-29) and DeepSeek's `STREAM_CLOSED:
DeepSeek Messages stream ended before message_stop` (2026-10-30 08:47Z,
three seats in the same two seconds) both read as `agent CLI exited 1`;
decision 0053 records that dsh classifies no pre-session refusal.

This decision is docs only. It chooses what slice 4 builds so the operator
can rule before code exists. Decision 0082 makes a new contract version
mintable only under an accepted decision; the versions named below are
minted when this one is accepted.

## What main holds at HEAD

Measured at 0675e03e8.

**The adapters.** `claude_refusal` (`crates/brokkr-protocol/src/adapters.rs:1475`)
returns a string for three shapes — the `assistant` record with
`isApiErrorMessage` or an `error` token, the error `result`, and the
`rate_limit_event` (always the token `rate_limit`) — and reads the HTTP
status from `apiErrorStatus`. `codex_refusal` (`:1530`) reads the harness's
`error` event and a `turn.failed` before `turn.started`, always as
`api_error` with no status. Both write through `refusal_reason` (`:1437`),
which clamps the token to 80 and the excerpt to 160 (`:1422`, `:1425`) and
writes `provider refused before the first turn: <token>`, appends ` (HTTP
<status>)` when the event gave a status and `: <excerpt>` when it gave
prose. The provider's words exist only in that string. The
classification is before the first counted turn only (`fold_stream_event`,
`:1562`, call `:1575`; `fold_codex_event`, `:1886`, call `:1898`), which is
decision 0053's boundary. The reason travels as `Invocation.refusal:
Option<String>` (`:504`); `lost::refusal_to_start`
(`crates/brokkr-protocol/src/adapters/lost.rs:86`) admits it only when no
work was checkpointed and no result was delivered, and `adapters.rs:6635`
sends the driver's `Result` as `failed` with it.

dsh classifies nothing: `finish_dsh` (`:5367`) builds `refusal: None` at
`:5400`, and the comment at `:5380` says so; the launch that reaches it is
`crates/brokkr-protocol/src/adapters/dsh_launch.rs:12`. The qualified route
reads stream-json (`invoke_dsh_stream_json`, `:4828`; the flag is appended by
`dsh_command`, `:4546`, at `:4556`) but has no refusal arm, so a provider
error still reaches the driver as stderr prose plus a non-zero exit.
Decision 0053's table
(`docs/decisions/0053-a-pre-session-refusal-is-a-failure-to-start.md:129`)
records the same. The 429 and the stream drop are **unmeasured** here:
`dsh --version` printed `0.2.1-rc.2`, and `dsh headless --help` stopped at
`EROFS: read-only file system` on its profile file before any provider call.

**The attempt and the park.** The failed `Result` is read at
`crates/brokkr-protocol/src/process.rs:655`, becomes
`AttemptOutcome::Failed` at `:746`, and arrives with `accepted: false` and
no checkpoint (`crates/brokkr-protocol/src/lib.rs:145`, `:167`).
`failed_to_start` (`crates/brokkr-runtime/src/engine.rs:1971`) is that shape
minus a deadline kill; `conclude_single` then writes `effect/failed` with
`start_failure: true` (`:1422`, `:1428-1432`; `start_failure_fields` at
`:1981`). The fold's `fail` (`crates/brokkr-core/src/fold.rs:601`) returns
the open effect to `ExecuteEffect` with `failed_attempts + 1` (`:613`). The
engine retries under `max_attempts` or parks with `effect <id> failed <n> of
<max> attempt(s); last error: <error>`
(`crates/brokkr-runtime/src/engine/cursor.rs:47-80`, text `:61-76`; last
error `:127`). The provider's words reach the park through that error. No
`retry-after` is kept: a search of `crates/` finds no reader, and the only
occurrence is an evidence file under `docs/evidence/`. Decision 0006's
matrix puts this in
the "Pre-session refusal" row
(`docs/decisions/0006-bounded-attempts-and-deadlines.md:269`): the executor
never began, so it retries within `max_attempts`; the 2026-10-11 addendum
(accepted 2026-10-12, #434) keeps the effect's identity and treats a rate
limit as determinate, not a partial side effect.

**Admission and the host.** `crates/brokkr-runtime/src/admission.rs:46`
says "Cool-downs are not judged yet." `pass` (`:471`) reads and writes
nothing; `pass_within` (`:481`) and `judge_within` (`:490`) add capacity
through `capacity::sized`
(`crates/brokkr-runtime/src/admission/capacity.rs:262`). The checks run in a
fixed order and the first unmet is the reason: host declared and readable;
seats measurable and every running run's seats readable; every provider and
route declared; cloud and provider ceilings; shared-local ceilings; floors;
boxed builds (`capacity.rs:26-34`; `room` `:322`, `declared` `:341`,
`ceilings` `:354`). `Capacity` is `:69`; `kind` is `:178`; `holds` is `:173`
(only an unreadable host and an unmeasurable entry are operator holds).
`brokkr queue list` prints the kind and text
(`crates/brokkr-cli/src/verbs/queue.rs:126`, `:239-256`). The host file is
`forge.host/v1` (`contracts/host.v1.schema.json:7`), whose own description
says "Cool-down bounds are not part of v1" (`:5`). The reader's types are
`HostConfig` (`crates/brokkr-runtime/src/admission/host.rs:38`), `Provider`
(`:57`), `Route` (`:66`) and `RouteClass` (`:77`), read once at `:292` from
`host_file` (`:268`). A `cooldowns` key is refused today:
`admission/host/tests.rs:114` plants one, and
`admission/capacity/tests.rs:446`/`:461` pins the refusal, which names
`schema`, `providers`, `floors` and `boxed_builds` as the expected keys.

**The queue and the dispatcher.** Queue storage is version 3
(`crates/brokkr-store/src/queue/storage.rs:58`). Version 3 (`:199`) adds
`queue_claims`, `queue_refusals` (`:221`), `dispatch_holders` (`:232`) and
`dispatch_settings` (`:244`), append-only by trigger; the table list is
`:290`. The stored state is `EntryState` (`crates/brokkr-store/src/queue.rs:194`),
`Launching` at `:202`; the holder and bound are read and written in
`crates/brokkr-store/src/queue/holder.rs:65`, `:71`, `:89`, `:102`. There
is no `dispatch` verb: `crates/brokkr-cli/src/lib.rs`'s `Cmd` carries
`Queue` and no `Dispatch`, so nothing standing runs a dispatcher and
admission is read by `brokkr queue list` and `brokkr queue judge`.

**What a run's journal can say while it waits.** The event vocabulary is
closed (`crates/brokkr-core/src/envelope.rs:16-47`;
`contracts/event-envelope.v1.schema.json:21-38`), and the payloads are
listed in `contracts/README.md:175-193`. A payload field not listed is
forbidden at v1 (`contracts/README.md:200-207`), which permits an optional,
absent-by-default field under a numbered extension schema provided `fold`
never reads it. Today a run's only wait is a park: `run/parked` makes the
fold `awaiting_operator` (`contracts/README.md:303`; `fold.rs:692`), with
the reason `cursor.rs:61-76` builds. `brokkr runs` prints the standing and
that reason (`crates/brokkr-view/src/fleet.rs:577-586`; "needs you" at
`:435`, `Need::Parked` at `:309`), `brokkr inspect` shows the events and the
park (`crates/brokkr-cli/src/verbs/readouts.rs:127`), and the TUI paints the
same derivation. A running run has no wait event; only the 3-hour staleness
bound calls it dead (`fleet.rs:164`, `:174`). So today the rate limit is a
park in "needs you", which is the fact 0068 ruling 4 says must change.

## Rulings

Each ruling gives the options, what each costs and refuses, how it meets or
misses 0068 ruling 4, and the recommended option. The operator may accept
all or pick.

### 1. The refusal's type

**Recommended: (a) one typed refusal at the existing classification edge.**

- **(a) `PreSessionRefusal` where the text is built.** A closed class
  (`rate_limit` or `other`), provider, route, the provider's bounded words,
  an optional HTTP status and an optional `retry_after_seconds`.
  `claude_refusal` and `codex_refusal` produce it; provider and route are
  filled from the adapter and launch facts the driver already resolved,
  never by parsing prose. It crosses the driver boundary as an optional
  `refusal` object on the v1 `result`, published as a numbered extension
  schema: the v1 result schema admits an additional property and the Rust
  reader already ignores an unknown field, so a driver that sends none
  reads as no typed refusal and the engine keeps today's path. The
  protocol word stays `forge-driver/v1` and `DRIVER_PROTOCOL` stays 1
  (`crates/brokkr-runtime/src/bundle.rs:88`), so no bundle manifest moves.
  An unresolved route is a typed absence, and the run parks naming it
  rather than recording a provider-wide cool-down that would hold
  unrelated routes.
- **(b) Keep the text and parse it in the engine.** Refused: decision 0001
  forbids a control-plane decision from prose, 0053 made the classifier the
  driver's, and a string cannot carry a typed route or retry-after without
  becoming a string vocabulary (0071 ruling 3).
- **(c) Put a JSON document in `error`.** Refused: a hidden type in a field
  the journal renders as the provider's words.
- **(d) Re-classify downstream from the message.** Refused: (b) with a
  smaller vocabulary.

Cost: one extension schema and the optional field on every in-process
`AttemptReport`. It refuses dsh: dsh stays `None` and follows today's path
until a slice measures its 429 and stream drop. That gap is deliberate.

### 2. Where the cool-down lives

**Recommended: (a) a typed wait on `effect/failed`, plus a store table.**

- **The journal.** An optional `wait` object on `effect/failed`, under a new
  numbered extension schema: `provider`, `route`, the provider's bounded
  words, `retry_after_seconds` when given, the computed `until` and
  `rounds`. It is legal at `event_schema: 1` under
  `contracts/README.md:204-207` for the reason
  `contracts/effect-provenance.v1.schema.json:5` gives for its own fields:
  optional, absent by default, published as an extension schema, never read
  by `fold`. The engine reads it by scanning the effect's own events, as it
  already scans them for `start_failure`; `RunState` gains nothing. The run
  stays `running`, so the wait is not a park and never enters "needs you";
  its end is the retry's `effect/started`, and the readouts paint it from
  the event. A wait object the engine cannot read is refused by name, never
  read as no wait: the attempt's failure then follows 0006 as today.
- **The table.** `provider_cooldowns`, beside 0095's tables, in queue
  storage v4: one append-only row per wait, keyed by the `effect/failed`
  that recorded it. Columns: rising `seq`, `run_id`, `event_seq`,
  `provider`, `route`, `until_at`, `reason` (the provider's words,
  bounded), nullable `retry_after_seconds`, `rounds`, `recorded_at`.
  `CHECK`s: whole `rounds >= 1`; a nullable retry-after is a whole number
  above 0; `until_at > recorded_at`; non-empty provider, route and reason.
  Triggers refuse update and delete, as 0095's tables do. The latest row per
  provider and route is in force while `until_at > now`; an expired row is
  history and is never deleted. The engine that observed the refusal writes
  the row and the `effect/failed` in one `Immediate` transaction, so the two
  cannot disagree and admission never misses a recorded refusal; a crash
  before it writes neither, and the attempt is settled on resume by 0006 as
  an in-flight attempt is. The event is the one home and the row is
  admission's index, as the claim is the pass's.
- **(b) A new `run/waited` event type.** The most literal reading of "the
  journal shows the wait", but it costs `event-envelope` v2 and
  `EVENT_SCHEMA_VERSION = 2`, a constant embedded in every bundle manifest
  (`crates/brokkr-runtime/src/bundle.rs:4258`;
  `contracts/README.md:208-214`), so every manifest, witness and compose
  digest moves, plus a new fold transition. A large price for a fact the
  extension carries.
- **(c) The table only.** Refused: the run's journal would not show the
  wait, and a readout would have to join the table to a run.

Cost: one extension schema and queue storage v4. It refuses to put the wait
in the fold, so a fold-only reader sees a running run; the readout reads the
event, the division `start_failure` already uses.

### 3. Length and maximum

**Recommended: (a) the provider's retry-after, else a declared default,
bounded by a declared maximum; a repeat lengthens; a wait or an accumulated
episode past the maximum parks.**

`forge.host/v2` adds an optional `cooldowns` block: `default_seconds` and
`max_seconds`, required when the block is present; optional
`providers.<name>.{default_seconds,max_seconds}` and
`providers.<name>.routes.<route>.{default_seconds,max_seconds}`. Every named
provider or route must already be declared under `providers` in the capacity
block, so one file names one vocabulary. Every object stays closed, so an
unknown key refuses as it does in v1. Every value is positive;
`default_seconds <= max_seconds`; `max_seconds <= 7200`. The 2-hour cap is
the shipped seats' longest attempt deadline (`fleet.rs:164`), so an
in-process wait cannot outlive the fleet's 3-hour staleness bound
(`fleet.rs:174`) and be mistaken for a dead engine. A v1 file read by the v2
loader has no cool-downs declared, so a rate-limit refusal parks as today
with a reason naming the absent declaration. That is fail-closed, and it is
the price of the default being the operator's data rather than a constant.

One wait is the provider's `retry_after_seconds` when given, else the
route's `default_seconds`, doubled for each earlier wait of the same effect
(`rounds`). A single wait longer than `max_seconds`, or the sum of one
effect's waits passing it, parks. A constant provider retry-after therefore
cannot repeat forever: the accumulated episode crosses the maximum. A
retry-after is the provider's own number and is not lengthened; a longer one
parks. Whether claude or codex gives one is unmeasured; where no field is
measured the type carries `None` and the default applies.

(b) Constants in code: refused, because 0068 says "declared" and the
operator's 2026-10-10 ruling puts machine facts in the host file. (c) One
global maximum only: refused, no per-provider or per-route number. (d) No
maximum: an unbounded wait, refused.

### 4. The probe

**Recommended: (b) the retry's first turn is the probe.** At the cool-down's
end the engine retries the refused effect — the seat's own adapter, model
pin, route, boundary and credentials; the first turn is what "answers".
The boundary is 0053's: a first counted turn or a delivered result, not an
HTTP 200. A second typed rate-limit refusal lengthens the wait; any other
pre-session refusal, timeout, lost result or indeterminate end parks. It
costs no separate call and no extra boundary or credentials; a rate limit
before inference bills no tokens, and when it answers the call is the work
the run wanted. Its risk is that the first turn answers and a later turn
hits the limit, which is 0016's mid-session boundary and does not move.

(a) A dedicated one-turn probe on the same adapter, model pin, route,
boundary and credentials, with a fixed one-word prompt, journaled on the
wait, and the full retry only if the probe's first turn opens: it matches
0068 literally, but costs a second paid call per round and a second
boundary and credential surface, and still cannot prove the next full
attempt. A probe that gets a non-rate-limit refusal must park. (c) The time
alone: cheapest, but it retries on time rather than evidence.

`brokkr probe harness` cannot serve: decision 0075 ruling 3's conformance
probe measures an adapter's declared facts against a scratch repository with
live credentials and writes a report — a design-time operator step for
onboarding, naming neither the refused effect's model pin nor its route
(`docs/decisions/0075-harness-onboarding-in-a-day.md:116-130`). It is the
pattern the adapter tests follow, not the probe.

### 5. Who waits, before slice 6

**Recommended: (a) the engine waits in-process.** At the refusal it writes
`effect/failed` with the wait and the row, waits until `until` holding the
run's lock and lane, then retries. A resume reads the same wait and waits
the remainder, or parks past the maximum. It works with nothing else built,
keeps the effect identity, and needs no standing process. Its cost is the
held lane: the run counts against its provider and route for the wait, which
admission would otherwise release. Its bound is the 2-hour maximum.

(b) The engine ends its drive and the dispatcher lifts the run: closest to a
standing dispatcher, but nothing drives it before slice 6, and a running run
nothing drives is a stale run, not a wait; unavailable now. (c) A minimal
`brokkr dispatch --once` pass: it would need slice 6's resume-and-spawn path
and would make slice 4 depend on a verb slice 6 is chartered to build; the
right end state, not the first step. `brokkr dispatch` and its loop remain
slice 6, which reads the table for admission and may replace the in-process
sleep with a dispatcher-paced resume. This decision reads 0068 ruling 4's
"dispatcher" as the engine's dispatch-and-admission mechanism; 0068 ruling
7's standing loop starts runs and does not drive one run's effects. If the
operator reads the word as the loop only, (b) or (c) is the path and slice 6
must precede the retry.

### 6. Admission

**Recommended: (a) admission reads the table after the declaration checks
and before the ceilings, and writes nothing.** `Measure::take`
(`capacity.rs:262`) already reads the host and the journal once; it gains
one read of the latest `provider_cooldowns` row per provider and route.
`Measure::room` (`:322`) checks the entry's lanes after every provider and
route is declared and before the ceilings, and the first cooling lane is
`Capacity::CoolingDown { provider, route, until, words }`: kind
`cooling_down`, text naming the provider, route, until and the provider's
words, `holds` false (it clears by itself and never latches), and no model
asked. A row it cannot read refuses the pass, as an unreadable host does; it
is never read as no cool-down. Once slice 6 claims an entry, its fence (0095
ruling 7) re-reads the table inside the claim's transaction, so a cool-down
recorded between pass and claim refuses the start; that is slice 6's file,
named so it is not missed.

(b) Derive from each run's journal every pass: refused, a second fold beside
the census, and cross-run admission has no run to fold. (c) Treat cooling as
`ProviderFull`: refused, it loses the until and the provider's words. (d)
Clear expired rows in the pass: refused, a read-only pass writes nothing.

### 7. Attempts

**Recommended: (c) the refused attempt is counted as 0006 counts it, but a
typed rate-limit wait in force is handled before the exhaustion check, so
the count cannot park a run within its cool-down maximum.** The attempt is
one terminal `effect/failed` with a fresh `attempt_id`; the effect's
`effect_id` and idempotency key are unchanged across the wait and the retry,
as 0006's same-open-effect rule and the addendum's operator-retry sentence
require. The count increments as today (`fold.rs:613`), so the journal says
how many attempts failed. `execute_effect` (`cursor.rs:47`) reads a wait in
force before the exhaustion check, so the wait path retries and the maximum
(choice 3) parks; every other failure keeps 0006's count and park.

(a) Do not count it: needs the fold to read the class (a v2 event, with the
digest cost choice 2 avoids) or a second counter (0071 ruling 5), and makes
`failed_attempts` mean something other than 0006 says. (b) Count and park at
`max_attempts`: today's behaviour, refused because it parks a rate-limited
run for the reason 0068 says must not park it.

A run whose cool-down expires while nobody drives it stays `running` with
the wait in its last `effect/failed`. A resume reads it and retries; if the
maximum passed while it waited, the resume parks. With no driver it is not
parked and not lost: the 3-hour stale bound names it and the operator
resumes it. Admission admits new entries as soon as `until_at` passes.

The versions this decision's acceptance mints, under 0082:

```text
contracts/driver-refusal.v1.schema.json
contracts/provider-wait.v1.schema.json
contracts/host.v2.schema.json
```

## What stays out

- Decision 0071's rules are kept. The type is a closed enum and data, not
  a trait; the wire is parsed once into a type; no `serde_json::Value` goes
  past the edge into state; `fold` gains no clock, and the engine passes
  `now`; new functions stay within ruling 4's ceilings; the extension and
  the table have one home each (ruling 5); new public items each have a
  consumer (ruling 6); the readouts derive the wait once in `brokkr-view`
  (ruling 7); errors are typed (ruling 8); every new test is shown to bind
  (ruling 9).
- No model is asked during admission (0068 ruling 3): the table is read and
  no process is spawned. The retry's first turn is a model call, but it is
  the effect the run already asked for, not an admission decision.
- No non-rate-limit refusal becomes a retry. 0053's boundary, 0006's
  timeout, lost-result, transport-limit, partial-side-effect and
  cleanup-failure rows, and 0016's mid-session rule are unchanged.
- A run still parks for a person exactly when it does today, except that a
  rate-limit refusal within its maximum waits instead.
- No Windows code, test or evidence (decision 0063).
- No contract is edited by this story; the acceptance of this decision
  mints the named versions, and the build slices mint them in order.

## The slices

Smallest first; each lands alone, tested, through the merge queue. Only 4a
edits `crates/brokkr-protocol/src/adapters.rs`; the adapter slot is held for
it, and a later dsh classification would be its own slice and its own turn.

1. **4a — the typed refusal crosses the adapter boundary.** Files:
   `crates/brokkr-protocol/src/adapters.rs` (the only slice that edits it),
   `crates/brokkr-protocol/src/lib.rs`, `crates/brokkr-protocol/src/process.rs`,
   `crates/brokkr-protocol/src/adapters/lost.rs`, the new driver-refusal
   extension schema, `crates/brokkr-runtime/tests/frozen_contracts.rs`,
   `quality/public-api/brokkr-protocol.txt`, and the protocol, conformance
   and runtime tests that build an `AttemptReport`. About 350 production and
   150 test lines, plus the schema. Contract: the driver-refusal extension
   schema at the v1 protocol word (the message family gains an optional
   field; an old reader ignores it). Test: a fake adapter refuses with a
   `rate_limit` record and answers on the next attempt; the report carries
   the class, provider, route and words, `failed_to_start` still holds, and
   the second attempt proceeds.
2. **4b — the host's cool-down bounds.** Files:
   `crates/brokkr-runtime/src/admission/host.rs`,
   `crates/brokkr-runtime/src/admission/host/tests.rs`, the new
   `forge.host/v2` schema. About 200 production and 120 test lines.
   Contract: `forge.host/v2`. Tests: a v1 file reads as no cool-downs; a v2
   block reads and inherits; a maximum past 2 hours, a default above its
   maximum, a zero, an unknown key, and a provider or route not declared
   under `providers` each refuse with its own line.
3. **4c — the wait in the journal and the engine's wait.** Files:
   `crates/brokkr-core/src/payload.rs` (read the extension beside
   `start_failure`), `crates/brokkr-runtime/src/engine.rs`,
   `crates/brokkr-runtime/src/engine/cursor.rs`, the new provider-wait
   extension schema, `crates/brokkr-runtime/tests/frozen_contracts.rs`, and
   the engine tests. About 220 production and 180 test lines. Contract: the
   `effect/failed.wait` extension schema. Tests: a fake adapter refuses with
   a rate limit and then answers (the wait is written, the run stays
   `running`, the retry starts); a cool-down whose computed end passes the
   declared maximum parks with the provider's words and the maximum named;
   a resume waits the remainder.
4. **4d — the table and its index.** Files:
   `crates/brokkr-store/src/queue/storage.rs` (queue storage v4),
   `crates/brokkr-store/src/queue/cooldown.rs` (new),
   `crates/brokkr-store/src/queue.rs`, `crates/brokkr-runtime/src/engine.rs`
   (the one transaction with the `effect/failed`), and the store and engine
   tests. About 260 production and 150 test lines. Contract: queue storage
   v4, a store version rather than a wire contract. Tests: a v3 journal
   migrates to v4 and reads its queue, claims, refusals, holders and settings
   unchanged; the row and the `effect/failed` land together or not at all;
   the `CHECK`s refuse a non-integer round, a non-positive retry-after, an
   until not after its record and an empty provider; the latest row per
   provider and route is the one in force, and an expired row never is.
5. **4e — admission's cool-down leg.** Files:
   `crates/brokkr-runtime/src/admission.rs`,
   `crates/brokkr-runtime/src/admission/capacity.rs` and
   `crates/brokkr-runtime/src/admission/tests.rs`. About 120 production and
   100 test lines. Contract: none. Test: an entry whose provider and route
   stand in a cool-down gets `Capacity::CoolingDown`, kind `cooling_down`,
   the until and the provider's words in its text, and is not held; an entry
   on a route with no row is admitted.
6. **4f — the readouts paint the wait.** Files:
   `crates/brokkr-view/src/fleet.rs`, `crates/brokkr-view/src/lib.rs`
   (`VIEW_VERSION` 27 to 28), `crates/brokkr-view/src/tests.rs`, and the
   CLI and TUI rows if the view's typed field is not enough. About 120
   production and 80 test lines. Contract: none under `contracts/`; the view
   model gains a typed wait. Test: a run whose last event carries a wait
   reads `running` with the wait and the provider's words, is not in "needs
   you", and the same derivation feeds `brokkr runs`, `inspect` and the TUI.
7. **Then #430's slices 5 and 6**, as 0095's plan has them: `shared-local`
   capacity and the scratch sweep, then `brokkr dispatch` and its standing
   loop. Slice 6 reads `provider_cooldowns` in admission and in the claim's
   fence, and may replace 4c's in-process wait with a dispatcher-paced
   resume; that is slice 6's decision.

## Consequences

- A rate-limited run shows a wait with the provider's words, not a park for
  a person; the wait is bounded by the declared maximum, and a wait past it
  parks.
- Before slice 6 the price is the held lane: an in-process wait holds the
  run's lock and counts against its provider and route until the retry. The
  operator can shorten it with a lower maximum.
- A v1 host file keeps today's park: the feature is opt-in through
  `forge.host/v2`. An operator who wants the cool-down writes the block;
  one who does not keeps the park and its evidence.
- Dsh seats keep today's behaviour: a DashScope 429 and a stream drop still
  read as an exit code and follow 0006 unchanged. A later slice measures and
  classifies them.
- No manifest digest, witness or compose digest is predicted to move: the
  wait is an extension at `event_schema: 1`, not a new event type, and the
  refusal is an extension at the v1 driver protocol, not a version bump. If
  the operator chooses 2(b), every one of those moves; a driver-protocol
  version bump would move them too, through `DRIVER_PROTOCOL`
  (`crates/brokkr-runtime/src/bundle.rs:88`), which this decision avoids.
- The new contracts are minted only by the operator's acceptance of this
  decision, under 0082; this story mints nothing. No remote CI, publication
  or live-profile check is claimed; this slice's validation is the local
  doc, ledger, typo and ratchet gate.

## Evidence

Read at 0675e03e8. The journal counts in the Context are the controller's
and were not re-read here. Each code claim names its file and line; the dsh
check is this session's: `dsh --version` printed `0.2.1-rc.2`, and `dsh
headless --help` stopped at `EROFS: read-only file system` on its profile
file, so the 429 and the stream drop are unmeasured.
