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
decision 0053's boundary. The adapters count turns but impose no one-turn
invocation: `fold_stream_event` counts an `assistant` record (`:1600`) and
`fold_codex_event` counts `turn.started` (`:1922`), and an invocation runs
to its result or deadline. A probe therefore needs a bound of its own;
today there is none. The reason travels as `Invocation.refusal:
Option<String>` (`:504`); a panel drives each member as its own invocation,
so each member's report carries one refusal and the terminal event is where
a per-site collection is assembled. `lost::refusal_to_start`
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
`retry-after` is kept: a search of `crates/` finds no reader; outside this
decision the token appears at
`docs/evidence/adapters/slice-two-mcp-isolation.md:785`,
`docs/evidence/adapters/slice-two-mcp-observations.json:2516` (also
`:2578`, `:2640`, `:2763`), and
`docs/research/github-peers/strongdm--attractor.md:37`. That no crate reads
it stands. Decision 0006's matrix puts this in
the "Pre-session refusal" row
(`docs/decisions/0006-bounded-attempts-and-deadlines.md:269`): the executor
never began, so it retries within `max_attempts`; the 2026-10-11 addendum
(accepted 2026-10-12, #434) keeps the effect's identity and treats a rate
limit as determinate, not a partial side effect.

A pre-session refusal already advances the chain. `chain_index`
(`crates/brokkr-runtime/src/engine.rs:1729-1744`) counts the effect's
`effect/failed` events whose `start_failure_sites` name the site, and
`select_candidates` (`:1757-1769`) reads it to pick the next candidate;
`plan_attempt` calls that before anything spawns
(`crates/brokkr-runtime/src/engine/dispatch.rs:222`).
`a_pre_session_refusal_advances_the_chain_and_keeps_its_reason`
(`crates/brokkr-runtime/src/engine/agent_tests.rs:393-452`) pins a
two-candidate chain moving from the first link to the second.

**Admission and the host.** `crates/brokkr-runtime/src/admission.rs:46`
says "Cool-downs are not judged yet." `pass` (`:471-472`) reads the queue
and writes nothing; `read` (`:626-628`) is its read. `pass_within` (`:481`)
and `judge_within` (`:490`) add capacity through `capacity::sized`
(`crates/brokkr-runtime/src/admission/capacity.rs:262`). The checks run in a
fixed order and the first unmet is the reason: host declared and readable;
seats measurable and every running run's seats readable; every provider and
route declared; cloud and provider ceilings; shared-local ceilings; floors;
boxed builds (`capacity.rs:26-34`; `room` `:322`, `declared` `:341`,
`ceilings` `:354`). `Capacity` is `:69`; `kind` is `:178`; `holds` is `:173`
(only an unreadable host and an unmeasurable entry are operator holds).
`brokkr queue list` prints the kind and text
(`crates/brokkr-cli/src/verbs/queue.rs:126`, `:239-256`). The host file is
`forge.host/v1` (`contracts/host.v1.schema.json:10`), whose own description
says "Cool-down bounds are not part of v1" (`:5`). The reader's types are
`HostConfig` (`crates/brokkr-runtime/src/admission/host.rs:38`), `Provider`
(`:57`), `Route` (`:66`) and `RouteClass` (`:77`), read once at `:292` from
`host_file` (`:268`). A `cooldowns` key is refused today:
`admission/host/tests.rs:114` plants one, and
`admission/capacity/tests.rs:446`/`:461` pins the refusal, which names
`schema`, `providers`, `floors` and `boxed_builds` as the expected keys.

**The queue and the dispatcher.** Queue storage is version 3
(`crates/brokkr-store/src/queue/storage.rs:58`). Version 3 (`:199`) adds
`queue_claims` (`:200`), `queue_refusals` (`:221`), `dispatch_holders`
(`:232`) and `dispatch_settings` (`:244`); all are append-only by trigger
except `queue_claims.run_id`, which may be set once and never moved
(`:210-213`), so a claim's run link is written once. The table list is
`:289`. The stored state is `EntryState` (`crates/brokkr-store/src/queue.rs:194`),
`Launching` at `:202`; the holder and bound are read and written in
`crates/brokkr-store/src/queue/holder.rs:65`, `:71`, `:89`, `:102`. There
is no `dispatch` verb: `crates/brokkr-cli/src/lib.rs`'s `Cmd` carries
`Queue` and no `Dispatch`, so nothing standing runs a dispatcher and
admission is read by `brokkr queue list` and `brokkr queue judge`.

**Panels and sequences.** A panel or sequence writes one terminal
`effect/failed`. `execute_panel` (`crates/brokkr-runtime/src/engine/panel.rs:64-137`)
writes it at `:130`; `run_body`
(`crates/brokkr-runtime/src/engine/dispatch.rs:285-330`) routes panels and
sequences around `conclude_single` for singles; a sequence's panel step
joins its members at `crates/brokkr-runtime/src/engine/sequence.rs:333-374`,
and a step's failure reaches `end_failed` (`:675`). A panel whose members
refused on claude/A and codex/B therefore journals one terminal event with
two `start_failure_sites`; today it has no wait, and the whole attempt
retries or parks under 0006.

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

**Recommended: (a) one typed refusal per invocation, assembled per site at
the terminal event.**

- **(a) `PreSessionRefusal` where the text is built.** A closed class
  (`rate_limit` or `other`), provider, route, the provider's bounded words,
  an optional HTTP status and an optional `retry_after_seconds`.
  `claude_refusal` and `codex_refusal` produce it; provider and route are
  filled from the adapter and launch facts the driver already resolved,
  never by parsing prose. It crosses the driver boundary as an optional
  `refusal` object on the v1 `result`, published as a numbered extension
  schema: the v1 result schema admits an additional property and the Rust
  reader already ignores an unknown field, so a driver that sends none
  reads as no typed refusal and the engine keeps today's path. A panel or
  sequence drives each member or step as its own invocation, so each
  report carries one refusal and the engine assembles the per-site
  collection at the terminal event; a single seat carries at most one. A
  present object that cannot be parsed — an unknown class, a missing
  provider or route, a non-positive retry-after — refuses by name and is
  never read as absent. The protocol word stays `forge-driver/v1` and
  `DRIVER_PROTOCOL` stays 1
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

Cost: one extension schema, the optional field on every in-process
`AttemptReport`, and the per-site assembly in panels and sequences. It
refuses dsh: dsh stays `None` and follows today's path until a slice
measures its 429 and stream drop. That gap is deliberate.

### 2. Where the cool-down lives

**Recommended: (a) a typed per-site wait on the terminal event, plus a
store table whose rows are per site.**

- **The journal.** An optional `waits` array on the terminal `effect/failed`
  (and on `effect/indeterminate` when a member refused while another is
  indeterminate), under a new numbered extension schema. Each entry is one
  refused site: `site` (the journal tag, absent for an inline seat),
  `provider`, `route`, the provider's bounded words, `retry_after_seconds`
  when given, the computed `until` and `rounds`. A single seat has one entry
  or none; a panel or sequence has one per refused site, beside the
  `start_failure_sites` the event already carries. It is legal at
  `event_schema: 1` under `contracts/README.md:204-207` for the reason
  `contracts/effect-provenance.v1.schema.json:5` gives for its own fields:
  optional, absent by default, published as an extension schema, never read
  by `fold`. The engine reads it by scanning the effect's own events, as it
  already scans them for `start_failure`; `RunState` gains nothing. The run
  stays `running`, so the wait is not a park and never enters "needs you";
  its end is the retry's `effect/started`, and the readouts paint it from
  the event. A `waits` entry the engine cannot read is refused by name,
  never read as no wait: the attempt's failure then follows 0006 as today.
- **The table.** `provider_cooldowns`, beside 0095's tables, in queue
  storage v4: one append-only row per refused site and round of the episode,
  not one per event. Columns: rising `seq` (the row's key), `run_id`,
  `event_seq` (the terminal event that began the episode), `site` (the
  journal tag, `''` for an inline seat), `provider`, `route`, `until_at`,
  `reason` (the provider's words, bounded), nullable `retry_after_seconds`,
  `rounds`, `recorded_at`. A unique index on `(event_seq, site, rounds)`
  holds one row per site and round of an episode. `CHECK`s: whole
  `rounds >= 1`; a nullable retry-after is a whole number above 0;
  `until_at > recorded_at`; non-empty provider, route and reason. Triggers
  refuse update and delete, as 0095's tables do.
- **The aggregate.** Admission is keyed by provider and route, not by the
  row or the event: every unexpired row for a provider and route is in
  force, and the wait ends at the latest `until_at` across them — a
  monotonic `MAX` aggregate. A later, shorter refusal never shortens a
  wait. No row is cleared by a pass or by a probe: only `until_at` expiry
  retires a row from the aggregate, and an expired row is history and is
  never deleted. A probe that answered is evidence the cooled route served
  at that moment; it is not evidence another run's unexpired refusal has
  ended, so deleting earlier rows would let one run's answer release a
  route another run's row still refuses. This decision proposes no clearing
  rule. The aggregate is re-read after the probe and before the retry, so a
  row recorded while the probe ran holds the route again; if a clearing
  rule is proposed later, it must carry evidence the route is no longer
  refused and must not remove a row recorded after the probe.
- **The episode's record.** The attempt's waits and their terminal event
  land together; the event is the attempt's home and the rows are
  admission's index, as the claim is the pass's. A probe's later refusal is
  not a second attempt and has no event of its own — a new event type is
  option (b), refused below — so it appends a row with the same `event_seq`
  and the next `rounds` for that site: the table is that fact's one home.
  The table lives in the journal's own database, so the probe is journaled
  there even though the event stream carries no row for it. The journal's
  `waits` array stays the attempt's record; the aggregate, not the event's
  first `until`, is the end admission and the readout use.
  A crash before that single row is written loses the extension and the
  resume probes again; no attempt is spent. The attempt's first rows and
  its terminal event are one `Immediate` transaction, so admission never
  misses that refusal; a crash before it writes neither, and the attempt is
  settled on resume by 0006 as an in-flight attempt is.
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
and site (`rounds`). A single wait longer than `max_seconds`, or the sum of
one effect's waits across its sites passing it, parks. A constant provider
retry-after therefore cannot repeat forever: the accumulated episode
crosses the maximum. A retry-after is the provider's own number and is not
lengthened; a longer one parks. Whether claude or codex gives one is
unmeasured; where no field is measured the type carries `None` and the
default applies.

The aggregate changes what one effect waits. Its own refusal computes one
candidate `until`, but the time it may retry is the latest `until_at`
across every unexpired row for its provider and route, including other
runs'. It waits that remainder; a later, shorter refusal cannot shorten it.
The sum of the effect's actual remainders is its episode, and when the next
remainder would pass `max_seconds` it parks. An overlapping row from
another run can therefore make an effect's remainder longer than its own
candidate; that longer remainder counts against its episode. By
construction no row's `until_at` is more than `max_seconds` after its
`recorded_at`, so the aggregate remainder is never longer than the newest
row's maximum.

(b) Constants in code: refused, because 0068 says "declared" and the
operator's 2026-10-10 ruling puts machine facts in the host file. (c) One
global maximum only: refused, no per-provider or per-route number. (d) No
maximum: an unbounded wait, refused.

### 4. The probe

**Recommended: (a) a dedicated one-turn probe before the retry.**

A probe is a dedicated driver invocation of the cooled site's own adapter and
launch path — the same `run_seat`/`run_attempt_resuming` path the refused
attempt used — not the retried effect, and not `brokkr probe harness`. It
asks the route the row names: the cooled provider and route, with that
site's model pin and effort. It uses the same boundary and credential facts
the refused attempt used; it resolves no new credential or boundary, and a
fact it cannot resolve parks rather than falling back to another route. The
driver advertises `probe` in the `capabilities` it already sends
(`crates/brokkr-protocol/src/process.rs:565-585`), and `Start` carries a
typed `probe: true` under a numbered extension schema; a driver that does
not advertise it is never probed, and the run parks naming the unsupported
probe. The adapter ends the session at the first counted turn and admits no
second: the first counted turn is the answer, decision 0053's boundary (a
turn or a delivered result), not an HTTP 200 and not a spawn. The probe's
own arm reports `Succeeded` with that first turn's checkpoint, whichever way
the harness ends after it is stopped; a report with no `accepted` and no
checkpoint is not an answer.

The answer is journaled on the retry's `effect/started` as an optional
extension under the same `provider-wait/v1` schema — the site, provider,
route and answered-at — so the wait's end has a record; a probe's refusal
that extends the wait is the table row ruling 2 describes. A driver's
reported usage rides that extension where the driver gives it, but the
probe's token cost is unmeasured today.

What a probe's other ends do:

- A typed `rate_limit` refusal records a new row for the same site and
  extends the cool-down. It spends no effect attempt — the probe's failure
  is not a `failed_attempts` — and the engine waits the new aggregate and
  probes again. That loop is bounded by the effect's declared episode
  maximum (choice 3), which parks it, not by the attempt cap.
- Any other typed refusal (auth, an unknown model, a non-rate-limit
  provider refusal) parks, naming the provider's words. It does not fall
  back and does not retry the effect.
- A timeout, a lost result, an unreadable or malformed present refusal, and
  an indeterminate end park under 0006's rows; none is a silent retry.
- An answer releases the cooled route for admission, and the effect retries
  only after the answer.

The probe's cost is one extra driver invocation per cooled site per round.
Whether it bills tokens is **unmeasured**, as is whether a pre-inference
rate limit bills none. The narrower benefit of option (b) — no separate
paid call — does not apply to (a).

**(b) The retry's first turn is the probe.** The former recommendation. It
costs no separate call, but it inverts 0068 ruling 4's order: the retry is
the effect's attempt, so a refusal at its first turn has already spent an
attempt and, at 0006's cap, parks before a clean probe can run. It also has
no one-turn bound today: the adapters count turns (`adapters.rs:1600`,
`:1922`) but the invocation runs past the first, so "the first turn" is not
a stop. Its risk is that the first turn answers and a later turn hits the
limit, which is 0016's mid-session boundary and does not move. Refused.

**(c) The time alone.** Cheapest, but it retries on time rather than
evidence. Refused.

`brokkr probe harness` cannot serve: decision 0075 ruling 3's conformance
probe measures an adapter's declared facts against a scratch repository with
live credentials and writes a report — a design-time operator step for
onboarding, naming neither the refused effect's model pin nor its route
(`docs/decisions/0075-harness-onboarding-in-a-day.md:116-130`). It is the
pattern the adapter tests follow, not the probe.

### 5. Who waits, before slice 6

**Recommended: (a) the engine waits in-process.** At the refusal it writes
the terminal event with its waits and the rows, waits until the aggregate
`until` holding the run's lock and lane, probes each cooled site, re-reads
the aggregate, then retries. The wait is taken in bounded slices and
re-reads the run's journal, so an operator `stop` during the wait is
honoured: the stop ends the run and no probe or retry follows. A resume
reads the same waits and waits the remainder, or parks past the maximum. It
works with nothing else built, keeps the effect identity, and needs no
standing process. Its cost is the held lane: the run counts against its
provider and route for the wait, which admission would otherwise release.
Its bound is the 2-hour maximum, and its extra cost is one probe per cooled
site per round.

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

**Recommended: (a) admission reads the aggregate after the declaration
checks and before the ceilings, and writes nothing.** `Measure::take`
(`capacity.rs:304`) already reads the host and the journal once; it takes
`now` as an argument (0071 ruling 1) and gains one read of every unexpired
`provider_cooldowns` row, aggregated to the latest `until_at` and the words
of the latest row at that maximum, per provider and route.
`Measure::room` (`:322`) checks the entry's lanes after every provider and
route is declared and before the ceilings, and the first cooling lane is
`Capacity::CoolingDown { provider, route, until, words }`: kind
`cooling_down`, text naming the provider, route, until and the provider's
words, `holds` false (it clears by itself and never latches), and no model
asked. Every unexpired row holds the lane, not the latest row only: a later,
shorter refusal never shortens the aggregate. A row it cannot read refuses
the pass, as an unreadable host does; it is never read as no cool-down.

The claim's fence re-reads the same aggregate inside the claim's `Immediate`
transaction (0095 ruling 7). The store hands the claim's judge the
unexpired rows it reads there, and `Room::recheck` refuses a claim when any
lane is cooling now, even if the pass saw none. That is slice 4d's store
read and slice 4e's recheck, so a cool-down recorded between pass and claim
refuses the start; slice 6's dispatcher uses the same fence and does not
build a second one.

(b) Derive from each run's journal every pass: refused, a second fold beside
the census, and cross-run admission has no run to fold. (c) Treat cooling as
`ProviderFull`: refused, it loses the until and the provider's words. (d)
Clear expired rows in the pass: refused, a read-only pass writes nothing.
(e) The latest row per provider and route only: refused, the overlap case;
a later, shorter row would release the route while an earlier row's wait
still runs.

### 7. Attempts

**Recommended: (c) 0006's cap is kept exactly; a wait and its probe are
taken only while attempts remain.** The refused attempt is one terminal
`effect/failed` with a fresh `attempt_id`; the effect's `effect_id` and
idempotency key are unchanged across the wait and the retry, as 0006's
same-open-effect rule and the addendum's operator-retry sentence require.
The count increments as today (`fold.rs:613`), so the journal says how many
attempts failed. `execute_effect` (`cursor.rs:47-80`) checks
`failed_attempts >= max_attempts` first, exactly as it does now (`:61-76`):
at the cap the run parks, and when the last terminal event carries a wait
the park's reason names the provider, route, `until` and the provider's
words beside 0006's `effect <id> failed <n> of <max>` text. No probe and no
retry starts at the cap.

Below the cap, a typed rate-limit wait in force is taken before the next
attempt: the engine waits the aggregate `until`, probes each cooled site,
re-reads the aggregate after the answers, and only then spends the next
attempt. The retry is an attempt within 0006's limit, so with
`max_attempts = 2` two rate-limit refusals park the run at the second; there
is no third attempt.

This decision proposes **no amendment to 0006**. The operator ruled on
2026-10-30 that the cap is kept (#430 comment 6533439724). A change to the
cap for rate limits would be a separate, named amendment for the operator,
with its own reason and tests; it is not this decision's default, and
0006's file is not edited here.

(a) Do not count it: needs the fold to read the class (a v2 event, with the
digest cost choice 2 avoids) or a second counter (0071 ruling 5), and makes
`failed_attempts` mean something other than 0006 says. (b) Keep today's
order exactly: count, and at the cap park before any wait. That keeps the
cap but loses the wait whenever the rate limit lands on the last permitted
attempt; refused because 0068 says a rate-limited run within its maximum
waits instead of parking.

The retry's candidate is selected by the existing chain rule, not by the
cool-down. A rate-limit refusal is still a pre-session failure to start
under 0053 ruling 1, and `chain_index`/`select_candidates`
(`engine.rs:1729-1769`) has already counted it, so for
`[claude/first, codex/second]` the retried effect takes codex/second while
the probe asks the cooled route claude/first. The probe's answer releases
the cooled route for admission; it does not reset the chain and does not
re-select the retried effect onto the cooled link. If the retried
candidate's route is itself in an unexpired cool-down, the engine waits that
aggregate too before the attempt. On a one-candidate chain, or once the
chain clamps to its last link, the retried effect takes the cooled route,
and the probe's answer is its own evidence. A panel advances each member's
chain independently, as `start_failure_sites` already does; each member
takes its own next link.

The panel and sequence join is 0006's, unchanged. `outcome_over`
(`crates/brokkr-runtime/src/engine/panel.rs:540-567`) makes any
indeterminate member an `effect/indeterminate` that parks, and otherwise any
failed member an `effect/failed` that may retry. A panel with a succeeded
member and a rate-limit refusal is therefore `effect/failed`: it records a
wait only for the refused site, probes that route, and retries the whole
panel after the answer, so the succeeded member re-runs; 0093's member carry
is the operator's retry and is not used here. A panel with any indeterminate
member records the refused sites' rows on its `effect/indeterminate` — so
admission holds those routes — and retries nothing. A sequence step is one
such attempt: a failed step ends the sequence's `effect/failed` with its
step-tagged sites, and the retry restarts from step 1; an indeterminate step
parks the sequence.

A run whose cool-down expires while nobody drives it stays `running` with
the waits in its last terminal event. A resume reads them and retries if
attempts remain; at the cap, or if the declared maximum passed while it
waited, the resume parks naming the cool-down. With no driver it is not
parked and not lost: the 3-hour stale bound names it and the operator
resumes it. Admission admits new entries as soon as the aggregate
`until_at` passes.

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
  no process is spawned. The probe is a model call, but it asks the cooled
  route the run already used; admission itself asks no model.
- No non-rate-limit refusal becomes a retry. 0053's boundary, 0006's
  timeout, lost-result, transport-limit, partial-side-effect and
  cleanup-failure rows, and 0016's mid-session rule are unchanged.
- A run still parks for a person exactly when it does today, plus the cap:
  a rate-limit refusal with attempts remaining waits and probes instead of
  parking, and a rate-limit refusal at 0006's cap parks naming the
  cool-down. A refusal whose route cannot be resolved, whose declaration is
  missing, or whose typed object cannot be read also parks, as ruling 1
  and ruling 3 say; a provider-wide hold would hold unrelated routes, so
  the run waits for the operator instead.
- A panel or sequence with any indeterminate member is indeterminate and
  parks under 0006; the refused sites' rows are still recorded on that
  terminal event, so admission holds the routes the panel did observe even
  though the effect does not retry.
- No Windows code, test or evidence (decision 0063).
- No contract is edited by this story; the acceptance of this decision
  mints the named versions, and the build slices mint them in order.

## The slices

Smallest first; each lands alone, tested, through the merge queue. Only 4a
edits `crates/brokkr-protocol/src/adapters.rs`; the adapter slot is held for
it, and a later dsh classification would be its own slice and its own turn.
4c, 4d and 4e do not touch it, because the typed refusal and the probe's
one-turn bound are 4a's. 4c waits the attempt's own event and has no table
yet; 4d adds the table and extends the engine's wait, retry and transaction
to read its aggregate; 4e adds admission. The overlapping-refusal aggregate
tests therefore land in 4d and 4e, not 4c.

1. **4a — the typed refusal and the probe's adapter bound.** Files:
   `crates/brokkr-protocol/src/adapters.rs` (the only slice that edits it),
   `crates/brokkr-protocol/src/lib.rs`, `crates/brokkr-protocol/src/process.rs`
   (the `probe` capability and the `Start` field),
   `crates/brokkr-protocol/src/adapters/lost.rs`, the new driver-refusal
   extension schema, `crates/brokkr-runtime/tests/frozen_contracts.rs`,
   `quality/public-api/brokkr-protocol.txt`, and the protocol, conformance
   and runtime tests that build an `AttemptReport`. About 400 production and
   180 test lines, plus the schema. Contract: the driver-refusal extension
   schema at the v1 protocol word (the message family gains an optional
   field; an old reader ignores it) and the optional `probe` field on
   `Start`. Tests: a fake adapter refuses with a `rate_limit` record and
   answers on the next attempt; the report carries the class, provider,
   route and words, `failed_to_start` still holds, and the second attempt
   proceeds; **a present but malformed refusal refuses by name, never as
   absent**; a panel's members each carry their own refusal; a probe
   invocation ends at the first counted turn and admits no second; a driver
   that does not advertise `probe` is refused a probe.
2. **4b — the host's cool-down bounds.** Files:
   `crates/brokkr-runtime/src/admission/host.rs`,
   `crates/brokkr-runtime/src/admission/host/tests.rs`, the new
   `forge.host/v2` schema. About 200 production and 120 test lines.
   Contract: `forge.host/v2`. Tests: a v1 file reads as no cool-downs; a v2
   block reads and inherits; a maximum past 2 hours, a default above its
   maximum, a zero, an unknown key, and a provider or route not declared
   under `providers` each refuse with its own line.
3. **4c — the per-site wait in the journal, the engine's own-event wait
   and probe, selection, panels and sequences.** Files:
   `crates/brokkr-core/src/payload.rs` (read
   the extension beside `start_failure`),
   `crates/brokkr-runtime/src/engine.rs` (`chain_index`,
   `select_candidates`, `conclude_single`),
   `crates/brokkr-runtime/src/engine/cursor.rs` (`execute_effect`),
   `crates/brokkr-runtime/src/engine/dispatch.rs` (`plan_attempt`),
   `crates/brokkr-runtime/src/engine/panel.rs`,
   `crates/brokkr-runtime/src/engine/sequence.rs`, the new provider-wait
   extension schema, `crates/brokkr-runtime/tests/frozen_contracts.rs`, and
   the engine, panel and sequence tests. About 320 production and 260 test
   lines. Contract: the `effect/failed.waits` extension schema (also on
   `effect/indeterminate`). Tests:
   - **the cap:** `max_attempts = 2`; two rate-limit refusals, the second at
     the cap; the run parks, names the cool-down, and no third
     `effect/started` exists.
   - **the probe comes first:** no `effect/started` for the effect before
     the probe's answer; a probe refused again extends the cool-down and
     leaves `failed_attempts` unchanged.
   - **selection:** a two-candidate chain `[claude/first, codex/second]`;
     the first refuses, the probe asks claude/first, and after it answers
     the retried effect starts codex/second; a one-candidate chain retries
     the cooled route.
   - **a two-route panel:** claude/A and codex/B refuse in one attempt; one
     terminal `effect/failed` carries two waits; the effect retries only
     after both probes answer; a non-rate-limit member parks; an
     indeterminate member parks, records the refused site's row, and does
     not retry.
   - a sequence step's refusal carries its step-tagged site, and the retry
     restarts from step 1.
   - a present but malformed `waits` entry refuses by name.
   - **an operator `stop` during the wait is honoured:** no probe or retry
     follows it.
   - a cool-down whose computed end passes the declared maximum, or whose
     accumulated episode would, parks with the provider's words and the
     maximum named; a resume waits the remainder.
4. **4d — the table, its per-site rows, and the claim's transaction read.**
   Files: `crates/brokkr-store/src/queue/storage.rs` (queue storage v4),
   `crates/brokkr-store/src/queue/cooldown.rs` (new),
   `crates/brokkr-store/src/queue.rs`, `crates/brokkr-store/src/queue/claim.rs`
   (the claim re-reads the aggregate inside its transaction),
   `crates/brokkr-runtime/src/engine.rs` (the one transaction with the
   terminal event), `quality/public-api/brokkr-store.txt` (the claim's judge
   gains the cooldown rows), and the store and engine tests. About 300
   production and 180 test lines. Contract: queue storage v4, a store version rather than a
   wire contract. Tests: a v3 journal migrates to v4 and reads its queue,
   claims, refusals, holders and settings unchanged; every row and the
   terminal event land together or not at all; the `CHECK`s refuse a
   non-integer round, a non-positive retry-after, an until not after its
   record and an empty provider; one event with two refused sites writes two
   rows; a probe's second refusal for the same site writes a second row with
   the next round; **two overlapping refusals from two runs aggregate, and
   admission holds until the longer one ends**; **the claim transaction
   refuses a claim taken between the pass's read and the claim**.
5. **4e — admission's cool-down leg.** Files:
   `crates/brokkr-runtime/src/admission.rs`,
   `crates/brokkr-runtime/src/admission/capacity.rs`,
   `crates/brokkr-runtime/src/admission/tests.rs`, and
   `quality/public-api/brokkr-runtime.txt` (the new `Capacity` variant).
   About 150 production and 130 test lines. Contract: none. Tests: an entry whose provider and route
   stand in a cool-down gets `Capacity::CoolingDown`, kind `cooling_down`,
   the until and the provider's words in its text, and is not held; an entry
   on a route with no row is admitted; **two overlapping refusals, with the
   later one shorter, hold until the longer ends**; an expired row never
   holds; an unreadable row refuses; the claim's recheck refuses a row
   recorded after the pass read.
6. **4f — the readouts paint the waits.** Files:
   `crates/brokkr-view/src/fleet.rs`, `crates/brokkr-view/src/lib.rs`
   (`VIEW_VERSION` 27 to 28), `crates/brokkr-view/src/tests.rs`, and the
   CLI and TUI rows if the view's typed field is not enough. About 120
   production and 80 test lines. Contract: none under `contracts/`; the view
   model gains a typed collection of waits whose end is the aggregate the
   engine reads, so a probe's extension is painted too. Test: a run whose
   last event carries waits reads `running` with every wait and the
   provider's words, a probe-extended wait shows the later end, the run is
   not in "needs you", and the same derivation feeds `brokkr runs`,
   `inspect` and the TUI.
7. **Then #430's slices 5 and 6**, as 0095's plan has them: `shared-local`
   capacity and the scratch sweep, then `brokkr dispatch` and its standing
   loop. Slice 6 uses the 4d/4e aggregate read and claim fence, and may
   replace 4c's in-process wait with a dispatcher-paced resume; it builds no
   second cooldown index.

## Consequences

- A rate-limited run shows every refused site's wait with the provider's
  words, not a park for a person; every wait is bounded by the declared
  maximum, and a wait past it or a refusal at 0006's cap parks. The retry is
  an attempt inside 0006's limit; the probe is not, and no amendment to 0006
  is proposed.
- Before slice 6 the price is the held lane: an in-process wait holds the
  run's lock and counts against its provider and route until the retry, and
  each round costs one probe per cooled site, whose token cost is unmeasured.
  The operator can shorten it with a lower maximum.
- A later, shorter refusal never releases a route an earlier row still
  holds; only expiry retires a row, so admission waits for the latest
  `until_at` across unexpired rows.
- A panel's retry re-runs the whole panel, including a member that settled;
  0093's member carry is the operator's retry and is unchanged. That is the
  cost of keeping 0006's panel semantics.
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

Read at 0675e03e8 and re-read at e2a2b1e6f for this revision. The journal
counts in the Context are the controller's and were not re-read here. Each
code claim names its file and line; this revision ran `git grep`/`sed` on
each one and the commands are recorded in the fix round's notes. The dsh
check is the drafting session's, not re-run here: `dsh --version` printed
`0.2.1-rc.2`, and `dsh headless --help` stopped at `EROFS: read-only file
system` on its profile file, so the 429 and the stream drop remain
unmeasured. The probe's token cost and whether a pre-inference rate limit
bills none are unmeasured too.
