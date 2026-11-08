# 0106 — A park names its cause, a long reset is a held wait an operator can lift, and the dispatcher adopts an orphan within a declared budget

Status: proposed
Date: 2026-11-08

Built: unbuilt (#1035)

This decision acts on 0068's rulings 4 and 7 and on 0103's rulings 2, 3 and 5. Because this
story may not edit 0068 or 0103, the header pointer and the two back-pointers are absent until
acceptance; the exact replacement sentences below are the amendment. They stand in [What 0068 and 0103 say after acceptance](#what-0068-and-0103-say-after-acceptance);
the older files are not edited here, and the amendments take effect when this decision is
accepted.

## Context

Decision 0068 ruling 4 promised a provider's refusal would be a cool-down with the provider's
own words, not a park for a person. 0103 built the cool-down half: a typed pre-session refusal,
a per-site wait, a one-turn probe, and 0006's attempt cap kept exactly. On 2026-11-08 two
provider failures showed what it left open.

The evidence is the controller's, from the house journal (not opened here). At about 07:34Z
DeepSeek's HTTP 402 "Insufficient Balance" (code `QUOTA`) failed every dsh seat as `agent CLI
exited 1`; three runs parked after 0006's two attempts, both spent within seconds. At about
10:30Z Codex's usage limit (429, then "You've hit your usage limit … try again at Nov 11th,
2026 10:38 AM") failed every review chief and correctness seat the same way; three more runs
parked. The operator topped up and reset by hand, and the controller ran `brokkr operator
retry` on each run and resumed it. A quota and an outage are not the operator's decision, and
Codex's reset lay days past 0103's two-hour maximum.

The controller's lane script then chose which parks to retry by matching
`indeterminate|engine restarted|contended` in the park's reason text; the delivery driver in
#1035 matched a wider pattern and retried a ruled park whose findings mentioned one of the
words. Both are string matches over operator-facing prose. 0001 forbids a control-plane
decision from prose and 0071 ruling 3 calls a string vocabulary a missing enum.

The operator accepted three things on 2026-11-08 (#1035): a park does not mean the person
decided; a provider-stated reset past the declared maximum is a held wait, not a park; and a
human may force the retry with a journaled command. This decision turns them into one design,
before any code. It is docs only. 0082 makes a contract version mintable only under an accepted
decision, so the versions named here are minted when this one is accepted, by the slices that
need them.

0064's ending is not a park's. A park is a live run waiting on the operator; it gains an
`ending` only when it concludes, and a park the operator closes takes the cause the operator
names. This decision types the park's cause on `run/parked`, not the ending: an answered park
leaves an ending as 0064 says, and an open park has none.

## What main holds at HEAD

Measured at `b56e6909e`, the branch's base; every line below was re-read there.

### Every `run/parked` write, and why it writes

| # | Where | The reason it writes | What it carries today |
|---|---|---|---|
| 1 | `crates/brokkr-runtime/src/engine.rs:519` | A strategy-selecting seat has no triage result and no default (`SELECT-NO-DEFAULT`, `:522`). | `reason`, empty `evidence`. |
| 2 | `crates/brokkr-runtime/src/engine/cursor.rs:280` | `Cursor::Park`: a policy rule (`crates/brokkr-core/src/fold.rs:696`), unmatched result (`:698`), schema-invalid result (`crates/brokkr-runtime/src/engine/decide.rs:35`), indeterminate close (`crates/brokkr-core/src/fold.rs:652`), or moved gate head. | `reason`; moved-head evidence alone (`crates/brokkr-runtime/src/engine/cursor.rs:275`). |
| 3 | `crates/brokkr-runtime/src/engine/cooldown.rs:725` | A typed rate-limit refusal that parks: unresolved route, unreadable host or earlier wait, computed wait past the maximum, or an unspellable end; text at `:179`. | `reason`; the failed event before it carries `waits` and `refused`. |
| 4 | `crates/brokkr-runtime/src/engine/cursor.rs:207` | 0006's cap (`:166`): `effect <id> failed <n> of <max> attempt(s); last error: …`, plus the last refusal's and wait's facts. | `reason`, empty `evidence`. |
| 5 | `crates/brokkr-runtime/src/engine/cursor.rs:79` | A probe's cleanup could not be proven, so nothing may wait, probe or retry (`:152`). | `reason`, empty `evidence`. |
| 6 | `crates/brokkr-runtime/src/engine/cursor.rs:88` | A recorded wait that parks: a resume past the maximum (`crates/brokkr-runtime/src/engine/cooldown.rs:591`), or a probe that refuses, hangs or answers nothing (`crates/brokkr-runtime/src/engine/cooldown/probe.rs:146`). | `reason`, empty `evidence`. |
| 7 | `crates/brokkr-runtime/src/engine/cursor.rs:124` | A crash between a `refused`-carrying `effect/failed` and its park; the record is the only evidence. | `reason`, empty `evidence`. |
| 8 | `crates/brokkr-runtime/src/engine/cursor.rs:130`, `:141` | A present `refused` or `waits` that does not read: refused by name, never as absent. | `reason`, empty `evidence`. |
| 9 | `crates/brokkr-runtime/src/engine.rs:815` | Journal contention, after the open attempt is settled indeterminate; text at `:788`. | `reason`, empty `evidence`. |

Three facts arrive through `effect/indeterminate`, and the park itself is write 2. A restarted
engine writes it at `crates/brokkr-runtime/src/engine/broker/fold.rs:348` with `engine restarted
while the attempt was in flight; completion cannot be established` (`:355`); a deadline, lost result or
transport limit writes it from `crates/brokkr-runtime/src/engine.rs:1450`; a gate that moved the
head writes `GATE-MOVED-HEAD` at `:887`. The fold reaches `Cursor::Park`
(`crates/brokkr-core/src/fold.rs:652`) and write 2 carries the text. The moved-head park is the
one indeterminate close that is not infrastructure: the machine refusing to call a gate that
moved the tree. No write carries a typed cause; the reason is the only thing a script can read.

### What `transition/decided` already types

`Decision` is the closed seven-field record (`crates/brokkr-core/src/decision.rs:84`): `from`,
`inputs`, `next`, `problem`, `result`, `rule_id`, `severity`, read once at the contract's edge
(`crates/brokkr-core/src/payload.rs:335`) and refused as `BadDecision` for any field v1 does not
list (0084). For a park the engine writes:

- `rule_id`: present when a policy rule matched and no transition was taken, null when no rule
  matched (`crates/brokkr-runtime/src/engine/decide.rs:209`);
- `result`: the seat result, or `__schema-invalid__` when the schema refused it (`:35`);
- `severity`: null, because severity is a property of a taken transition (`:209`);
- `problem`: the rule's reason, or the parked seat's non-empty notes under 0092 (`:224`).

The view reads that shape: `Facts::ruling_parked` is true when the last ruling routed nowhere
(`crates/brokkr-view/src/fleet.rs:555`), and a parked row shows its last rule (`:579`). A rule
park's rule is not lost; it is in the decision beside the park. The park does not say whether it
was the policy's, the cap's, an indeterminate end's or a provider's.

### What the fold and the readouts show

The fold has one parked state, `Run::AwaitingOperator { reason }`
(`crates/brokkr-core/src/fold/run.rs:14`); `park_reason` is that reason (`:42`); `ParkReason` is
only `GateMovedHead` or `Stated(String)` (`:63`). A `run/parked` payload is read for `reason`
alone (`crates/brokkr-core/src/fold.rs:713`).

`brokkr runs` derives one row in `brokkr-view`: `Facts::of` reads status, phase, last ruling and
park reason (`crates/brokkr-view/src/fleet.rs:517`); `Standing::Parked` comes from
`awaiting_operator` (`:538`); the row prints `parked` (`:230`); its rule is shown only when the
last ruling routed nowhere (`:579`); its reason is copied onto the row (`:607`); `Need::Parked`
(`crates/brokkr-view/src/fleet/need.rs:151`) puts it under `needs you`
(`crates/brokkr-view/src/fleet.rs:391`). `brokkr inspect` prints `park <reason>`
(`crates/brokkr-view/src/run_summary.rs:33`, `crates/brokkr-cli/src/render/inspect.rs:32`), and
the TUI paints the same row (`crates/brokkr-cli/src/tui/panes.rs:352`). Every surface shows the
prose, and only the prose, for every cause but a rule park.

### A pre-session refusal, and a seat that exits before its first event

`claude_refusal` (`crates/brokkr-protocol/src/adapters/refusal.rs:99`), `codex_refusal` (`:154`)
and their lanetally twin build a typed `PreSessionRefusal` at the adapter's edge. The class is
closed to `rate_limit` and `other` (`crates/brokkr-protocol/src/lib.rs:42`); `refusal_reason`
sets `rate_limit` only for that exact token and `other` otherwise (`:65`); the provider's words
are bounded once (`:52`). Dsh classifies nothing: `finish_dsh` writes `refusal: None`
(`crates/brokkr-protocol/src/adapters.rs:5329`), as does the dsh launch
(`crates/brokkr-protocol/src/adapters/dsh_launch.rs:471`); 0053 records the gap.

When a seat exits before its first event, the failed report takes `failed_to_start`: a `Failed`
outcome, no `accepted`, no checkpoint, not a deadline kill
(`crates/brokkr-runtime/src/engine.rs:1973`). The terminal `effect/failed` carries
`start_failure: true` and the failing sites (`:1995`). A typed rate-limit refusal takes
`rate_limit_terminal` (`:1440`); every other pre-session refusal keeps 0006's path, retries
within the limit, and at the cap parks with the 0006 text plus `last error`. For the dsh 402
that text was `agent CLI exited 1`; the provider's actual words were only in the stderr tail. A
dsh 402 is therefore not a `provider-limit` today: it is an attempt-cap park whose last failure
is a start failure, and this decision does not guess otherwise.

### The dispatcher and `brokkr operator retry`

The dispatcher is built as far as 0095's lock and reconciliation: `dispatch.rs` says its pass
and the `brokkr dispatch` verb are #430's later slices
(`crates/brokkr-runtime/src/dispatch.rs:4`); `take_dispatch` takes the one journal lock
(`crates/brokkr-runtime/src/dispatch/lock.rs:86`); `reconcile` settles an entry left launching
(`crates/brokkr-runtime/src/dispatch/reconcile.rs:63`). Reconcile never reads a parked run. A
run it linked with a `spawn` row is left alone, "its run reading `Running` for the operator to
resume should its driver have died" (`crates/brokkr-runtime/src/dispatch/reconcile.rs:13`).
That sentence is the orphan this decision is about.

`brokkr operator retry` writes `operator/commanded` and `operator/accepted`
(`crates/brokkr-runtime/src/engine/operator.rs:277`); the fold accepts a retry only when the run
is `awaiting_operator` (`crates/brokkr-core/src/fold.rs:391`) and moves it to
`Cursor::RequestEffect` (`crates/brokkr-core/src/fold/operator.rs:41`). The engine then requests
a new effect with a new id (`crates/brokkr-runtime/src/engine.rs:899`). On a waiting run — still
`running` with a recorded wait — the same retry is refused `run_not_awaiting_operator`
(`crates/brokkr-core/src/fold.rs:357`); only `stop` is accepted, and the wait's one-second
slices see it and probe nothing (`crates/brokkr-runtime/src/engine/cooldown.rs:636`). The
operator can stop a wait, not lift one.

## Rulings

Each ruling gives the options, what each costs and refuses, and the recommended option. The
operator may accept all or pick.

### 1. The park's cause

**Recommended: (a) a closed four-word cause as an optional `cause` object on `run/parked`, read
beside the reason and never guessed from it.**

| Cause | What it names | Typed fields | Dispatcher retry? |
|---|---|---|---|
| `ruled` | Policy or engine refused: a rule park, unmatched result, schema refusal, missing default, moved gate head. | `rule_id` where a rule matched; `detail`. | Never. |
| `attempt-cap` | 0006's cap was reached. | `failure_class`: `provider-limit`, `provider-refusal`, `start-failure` or `determinate`; `detail`. | Never. |
| `infrastructure` | An indeterminate end, engine restart in flight, or contention. | `retryable` (the attempt evidence proves no effect landed); `detail`. | Only when `retryable` and budget remains. |
| `provider-limit` | A typed rate-limit refusal parked below the cap. | `provider`, `route`, `until`, `max_seconds`, `detail`. | Never. |

`unknown` is a fifth word only when reading: an absent `cause` reads `unknown`, and every script
treats it as not retryable, like `ruled`. A present `cause` that does not read is refused by
name, never as absent.

**Its home.** Recommended: an optional `cause` object on `run/parked`, published as
`contracts/park-cause.v1.schema.json` under the amended extension rule at `event_schema: 1`
(0016's amendment, as 0103 used it). `reason` and `evidence` stay byte for byte; an old reader
ignores the field; the fold does not read it, so `RunState` gains nothing and an old journal
folds exactly as it does now. The summary and view read it once into a typed `ParkCause`, and
the readouts paint that one derivation.

- **(a) The optional `cause` object at the current event schema.** Recommended. Cost: one
  extension schema, one typed core reader, one summary field, the view's paint. It refuses to put
  the cause in the fold, so a fold-only reader still sees only the reason.
- **(b) A new `run/park-caused` event, or `event-envelope` v2.** The literal home, but it moves
  `EVENT_SCHEMA_VERSION`, every bundle manifest, witness and compose digest, and every fold arm.
  Refused for a fact an extension can carry.
- **(c) A second `transition/decided` after the park.** Refused: a park is not a ruling,
  `Decision` is closed, and every view would read the second decision as the run's last ruling.
- **(d) Keep the reason and match it.** Refused by 0001 and 0071 ruling 3; it is the lane
  script's failure.

**Older journals.** Recommended: a park with no `cause` reads `unknown`, never `ruled` inferred
from the preceding decision and never guessed from the reason. A preceding decision may not be
the park's — an operator command, a crash or a later decision can stand between them — and a
script that inferred `ruled` would retry a park the journal never typed. The rule id stays
visible in the decision and the row; the cause stays `unknown` until a writer records one. Cost:
one contract version minted on acceptance; an old park needs the operator. It refuses to repair
history from prose.

### 2. The held wait

**Recommended: (a) a provider-stated reset past the declared maximum is held — the run stays
`running`, the wait is recorded, no lane is held, and the readouts say `waiting`.**

0103 computes a wait's length: the provider's `retry_after_seconds` when given, else the route's
declared default doubled by round, bounded by the declared maximum. This decision keeps the
engine-computed wait capped: a computed wait or an accumulated episode past the maximum still
parks with cause `provider-limit`. It changes only the provider's own word: when the provider
states a reset, and that reset alone puts the wait past the maximum, the wait is **held**, not
parked. The provider has said when it will serve again; the engine declines to spend days
holding a lane, and the operator is not asked to rule on a fact the provider stated. A held
`until` may lie past the maximum; the maximum bounds the engine's own computation, not the
provider's word.

The cap is untouched. A held wait at 0006's cap is still a park with cause `attempt-cap` and
`failure_class: provider-limit`: the cap is the operator's, and 0103 ruling 7(c) keeps it. A held
wait below the cap spends no attempt.

**Its home.** Recommended: the run stays `Run::Running(Cursor::ExecuteEffect)` with its effect
open; the terminal `effect/failed` carries the existing `waits` and `refused` (0103's
`provider-wait/v1`), and no `run/parked` is written. The held state is derived from that event
and the fold's status: a `past-maximum` refusal with a provider `retry_after_seconds` and a
`running` run is held; the same record with `awaiting_operator` is a park. The engine ends its
drive, so the lane and process are released; the effect id and idempotency key stay, and the
retry is the same effect.

Admission records the provider and route in 0103's `provider_cooldowns`, one row per refused site
and round, with `until_at` the held end. Every unexpired row for the provider and route holds new
admissions until the latest `until_at`; a later, shorter refusal never shortens it. This reuses
0103's aggregate and builds no second index. A row whose end the provider stated may lie more
than `max_seconds` after `recorded_at`; that is the one construction sentence 0103 loses.

- **(a) The held wait above, no new event and no new wait field.** Recommended. Cost: the
  cooldown path branches on the provider-stated flag, the summary and view learn `waiting`,
  admission reads a row that may outlive the maximum. It refuses to sleep: no multi-day lane.
- **(b) Hold every wait past the maximum.** Refused: a repeated default or an exhausted episode
  is the engine's own backoff, not the provider's word; holding it forever hides a route that is
  not recovering. That case keeps 0103's park.
- **(c) Keep 0103's park for every past-maximum wait.** Refused: it contradicts the 2026-11-08
  ruling and leaves the long reset in `needs you`.
- **(d) A new `run/waiting` event or a `waits.held` field.** Both cost a contract or event
  version for a state the existing records express. Refused; a later field lands as
  `provider-wait/v2`, never as an edit to v1.

**Who wakes it at `until`.** Recommended: the dispatcher's pass once #430 slice 6 builds it;
before then `brokkr resume` at or after `until` takes the wait without sleeping, and `brokkr
operator retry` lifts it early. `resume` before `until` refuses by name and names the lift; it
does not sleep the remainder in process. The slice order is record and paint, then the lift, then
the dispatcher's wake; the pass reads the same `until` and rows, not a second mechanism. Options:
(a) the dispatcher's pass, with the resume hand before it — recommended; (b) the engine sleeps in
process — refused: 0103's held lane and process for days; (c) a new timer process — refused: a
second standing process for a fact the dispatcher owns.

**How the readouts show it.** Recommended: a new `Standing::Waiting` in `brokkr-view`, word
`waiting`, listed under `running` (not `needs you`), with provider, route, `until` and "operator
retry lifts" in the detail. It is not stale: `Need` returns nothing and the stale bound does not
apply. `inspect` prints a `wait` line; the TUI paints the same row; `VIEW_VERSION` moves once.
Options: (b) keep `Standing::Running` and put the wait in `detail` — cheapest, but a narrow
`runs` row drops it and the stale rule needs an exception; (c) a `Need::Waiting` under `needs
you` — visible, but says the operator must act when they need not. Refused. Cost: one view
version and the exhaustive matches a `Standing` variant forces.

### 3. The operator lift

**Recommended: (a) `brokkr operator retry` on a waiting run lifts it; a separate provider-wide
lift names the provider and route; and only a probe's answer clears rows, through the sequence
they were recorded at.**

**Per run.** `brokkr operator retry --run <waiting>` ends the wait and lets the engine probe the
same effect now; the next turn keeps the same effect id and idempotency key, with no remaining
sleep. It differs from retry on a parked run in what moves: a parked retry is accepted because
the run is `awaiting_operator` and moves to `Cursor::RequestEffect`, so the engine requests a
**new** effect with a new id (`crates/brokkr-core/src/fold/operator.rs:41`,
`crates/brokkr-runtime/src/engine.rs:899`); a waiting lift moves nothing, because the run is
already at its open effect, and clears only the remaining wait. The journal distinguishes them:
the lift is a new operator-command class `lift`, written under the `retry` surface; the parked
command stays `retry`. A `ruled`, `attempt-cap` or `provider-limit` park is still refused a lift;
only a held wait can be lifted. The journal records the operator, reason, provider, route and
`until`.

Options: (a) the `lift` class under the `retry` surface — recommended; (b) reuse `retry` and
broaden its acceptance to a running run — refused: it weakens "retry needs a park", makes one
word mean two transitions, and lets a retry land on a normal running effect; (c) a new top-level
`brokkr lift` verb — a second surface for the same command, when the operator asked for `retry`.

**Provider-wide.** Recommended: both forms. `brokkr operator lift --provider <p> --route <r>
--reason <why>` journals a provider-wide lift and wakes every waiting run on that provider and
route to probe now. It does not clear a row itself: an operator's word is not evidence the route
serves. The first probe's answer writes the clearing fact; a refusal writes a new row with the
next round and resumes the wait at its new `until`. A provider-wide lift with no waiting run on
that route refuses by name: it has no pin and no site to ask, and a clear without a probe would
violate 0103's condition.

**The clearing rule.** Recommended: `provider_lifts`, an append-only table beside 0103's
`provider_cooldowns` (queue storage v6, after 0104's v5, itself after 0103's v4). At a probe's start the engine reads the
latest `provider_cooldowns.seq` for that provider and route as `cleared_through_seq`. On an
answer it appends one lift row with that snapshot, provider, route, run, event, site,
`answered_at`, operator and reason; the aggregate takes the highest `cleared_through_seq` for
that provider and route and ignores every row at or below it. A row recorded during the probe has a higher `seq`, so it survives and holds the
route again; a row after the lift survives too. A refusal writes no lift row, writes the next
cooldown row, and the wait resumes at its new `until`. Only an answer writes a lift; a pass, a
clock and a manual clear write none. This is 0103's condition made concrete: the answer is the
evidence, and the sequence snapshot proves no row recorded after the probe is removed.

Options: (a) the sequence snapshot and an append-only lift table — recommended; (b) delete the
cleared rows — refused: the table is append-only and a delete can race a row recorded after the
probe; (c) clear by `answered_at` time — refused: equal or moved clocks cannot order a row at the
same instant, while the sequence can; (d) clear all rows on an operator's word — refused: no
evidence, and it removes rows recorded after the probe.

**What Skírnir may grant.** Recommended: a 0025 grant may enumerate `lift` and `lift-provider`,
naming the providers and routes it covers. It may not grant a retry of a `ruled`, `attempt-cap`
or `provider-limit` park; it may not grant a clear without a probe; and the dispatcher's
infrastructure budget is not a grant class, because the budget is the queue entry's declared
data. A grant naming a class or word outside this vocabulary refuses at load.

### 4. Orphan adoption

**Recommended: (a) the dispatcher adopts a queue-started `running` run whose driver is gone, and
never a run another process drives.**

An orphan is a run whose fold is `Running` and whose driver is gone: the engine died between
effects, or died mid-attempt and the next process found the attempt in flight. Reconcile already
names the gap: a run linked to an entry with a `spawn` row is left for the operator
(`crates/brokkr-runtime/src/dispatch/reconcile.rs:13`). Adoption is the dispatcher's `resume` of
that run under its pinned bundle, not a new run and not a retry of a parked one. At
`ExecuteEffect` it drives the effect; at `EffectInFlight` it takes the existing `restarted` path
(`crates/brokkr-runtime/src/engine/cursor.rs:45`), which settles the attempt first, so it never
starts a second driver beside an unsettled one. It adopts a queue entry's run only: a direct
`brokkr run` is the operator's, and the dispatcher's authority is the entries it owns.

Options: (a) queue-started runs only — recommended; (b) every `running` run — broader recovery,
but the dispatcher would drive work it did not start and could surprise an operator's own resume;
the fence keeps it safe, but the authority is wider than 0068's; (c) no adoption — refused:
reconcile already leaves the run for a person, which is the orphan the issue names.

**How "no live driver" is proven.** Recommended: layered, never one signal alone.

- **The writer lease.** The protected worktree's reservation names every attempt the run
  admitted, standing or settled (`crates/brokkr-runtime/src/engine/broker/admission.rs:1`). A
  standing attempt means a writer may still own the tree; adoption refuses and `release` or the
  settlement facts decide. A reservation with no standing attempt and every attempt settled is
  the first proof.
- **The attempt's settlement.** For `EffectInFlight`, `restarted` reads the reservation and the
  driver tallies and writes the attempt's terminal fact before any drive
  (`crates/brokkr-runtime/src/engine/broker/fold.rs:315`). Adoption does not invent a second
  settlement and does not re-run an unsettled attempt.
- **The dispatcher's lock.** `take_dispatch` excludes a second dispatcher
  (`crates/brokkr-runtime/src/dispatch/lock.rs:86`), and nothing more: it does not exclude a
  direct engine, so it is not the proof.
- **0029's fence.** The adoption appends on the head its fold read
  (`crates/brokkr-store/src/lib.rs:639`); a live driver that appends first moves the head, and the
  adoption writes nothing. The fence is the final proof and the one that holds when liveness was
  misjudged.

**What it must never do.** Never adopt a run another process drives; the fence refuses the write,
and the dispatcher does not retry it into a second driver. Never adopt a parked or terminal run.
Never adopt a run with a standing writer reservation. Never adopt at `EffectInFlight` without
settling the attempt first. Never adopt a direct run.

### 5. The infrastructure retry budget

**Recommended: (a) an operator-declared budget on the queue entry, default 3, spent only by an
`infrastructure` park whose attempt evidence proves no effect landed; when spent, the park is the
operator's.**

**Its home.** `queue_retry_budgets` beside 0106c's queue storage v6: one row per entry, the budget
and the operator and reason that set it, absent meaning 3. It is written by `queue add
--retry-budget <n>` or `queue set-retry-budget`, and the spends are append-only `queue_retries`
rows, one per dispatcher retry, naming the run, the park's seq and the cause. The budget belongs
to the entry, not the host: two entries may differ, and a direct run has none. It rides queue
storage v5, after 0103's v4.

Options: (a) the entry's table — recommended; (b) a field in the launch payload — refused: the
payload is the claim's digest and the store keeps it verbatim, so a budget would move every
manifest that carries it; (c) a host constant — refused: per-entry operator data, not a machine
fact; (d) a global default only — refused: the operator must be able to spend more or less on one
commission.

**What spends it.** Only an `infrastructure` park with `retryable` true, computed when the park is
written from 0006's attempt evidence: no checkpoint, no result, no broker call, no standing
reservation. A cleanup refusal, partial side effect, delivered transport frame, moved-head park
or standing reservation is `retryable: false` and spends nothing; 0006's safety rules are not
weakened by a budget. A `ruled`, `attempt-cap` or `provider-limit` park spends nothing whatever
its text. The dispatcher writes the retry as `operator/commanded` and `operator/accepted` with
command `retry`, operator `dispatcher`, and a reason naming the park's seq and "infrastructure
retry n of N"; the fold's retry moves the run to a new effect, exactly as an operator's retry
does. It writes under the same fence as any other dispatcher step and asks no model.

**When it is spent.** The park stays: `awaiting_operator`, cause `infrastructure`, `retryable`
true, and the readouts say "budget spent (N of N); needs you". A direct run's infrastructure park
has no budget and is always the operator's.

**Ruling 7's text.** Decision 0068's ruling 7 is replaced in the sentence quoted below, so the
loop may spend the declared budget and nothing else:

> It never retries a run the policy parked, except a queue-started run parked with the typed
> cause `infrastructure`, with `retryable` true, while its entry's declared retry budget remains;
> it never retries a `ruled`, `attempt-cap` or `provider-limit` park, never concludes or stops a
> run, and never changes an entry's order.

## What 0068 and 0103 say after acceptance

These are the replacement sentences, kept here so the older files are not edited. Each takes
effect when this decision is accepted; the header pointers and back-pointers are added then, not
in this story.

**0068 ruling 4.** "A run parks for the operator only when the provider's refusal is not a rate
limit or the cool-down exceeds a declared maximum."
(`docs/decisions/0068-the-dispatcher.md:85`) becomes:

> A run parks for the operator when the provider's refusal is not a rate limit, or when the
> engine's own computed wait or accumulated episode exceeds a declared maximum. A
> provider-stated reset past the declared maximum is a held wait: the run stays running, the wait
> is recorded with the provider's words and its end, admission holds the provider and route until
> that end, and an operator command can lift it now.

**0068 ruling 7.** "It never retries a run the policy parked, never concludes or stops a run"
(`docs/decisions/0068-the-dispatcher.md:103`) becomes the ruling 5 text above.

**0103 ruling 2.** "No row is cleared by a pass or by a probe: only `until_at` expiry retires a
row from the aggregate, and an expired row is history and is never deleted." and "This decision
proposes no clearing rule."
(`docs/decisions/0103-a-provider-s-rate-limit-becomes-a-cool-down.md:277`) become:

> A row is retired from the aggregate by its `until_at` expiry, and a probe's answer clears the
> rows for its provider and route recorded before that probe, through the sequence snapshot taken
> at its start. It never clears a row recorded after the probe: that row has a higher sequence and
> holds the route again. A refusal writes a new row and resumes the wait at its new `until`. This
> is the clearing rule 0103 asked for: the answer is the evidence, and the snapshot is the fence.

**0103 ruling 3.** "A single wait longer than `max_seconds`, or the sum of one effect's waits
across its sites passing it, parks."
(`docs/decisions/0103-a-provider-s-rate-limit-becomes-a-cool-down.md:340`) becomes:

> A wait the engine computed longer than `max_seconds`, or the sum of one effect's waits across
> its sites passing it, parks with cause `provider-limit`. A wait whose end the provider stated,
> past `max_seconds`, is held, not parked, and is woken at its end or lifted.

"By construction no row's `until_at` is more than `max_seconds` after its `recorded_at`"
(`docs/decisions/0103-a-provider-s-rate-limit-becomes-a-cool-down.md:356`) becomes:

> A row whose end the provider stated may lie past `max_seconds`; the maximum bounds the engine's
> own computed wait and episode, not the provider's word.

**0103 ruling 5.** "A run whose cool-down expires while nobody drives it stays `running` with the
waits in its last terminal event. A resume reads them and retries if attempts remain; at the cap,
or if the declared maximum passed while it waited, the resume parks naming the cool-down."
(`docs/decisions/0103-a-provider-s-rate-limit-becomes-a-cool-down.md:585`) becomes:

> A run whose cool-down expires while nobody drives it stays `running` with the waits in its last
> terminal event. A resume before a held wait's `until` refuses by name and names the lift; a
> resume at or after it probes and retries if attempts remain; a resume at the cap parks naming
> the cool-down.

## What stays out

- 0071's rules are kept: a closed enum and data, not a trait; typed at the edge; no
  `serde_json::Value` past the edge; the engine passes `now`; ruling 4's ceilings; one home each
  for the cause, the held wait and the lift; one derivation in `brokkr-view`; typed errors; every
  new test shown to bind.
- No model decides admission or a retry. The cause is computed from the journal and the typed
  refusal; the budget is data; the probe asks the route the run already used and its answer is
  evidence, not a verdict.
- No `ruled` park is retried without the operator: residual findings, a security hold, an
  oversized unit, a policy rule, a schema refusal, a moved gate head and a missing seat default
  all stay `ruled`. No `attempt-cap`, `provider-limit` or `unknown` park is retried by the
  dispatcher, and a `ruled` park is never lifted.
- No non-limit refusal becomes a silent retry. 0053's boundary, 0006's timeout, lost-result,
  transport-limit, partial-side-effect and cleanup-failure rows, and 0016's mid-session rule are
  unchanged. A probe that ends without an answer is indeterminate and parks.
- No trust, egress, boundary or schema refusal is weakened. The held wait records only the
  provider and route the typed refusal resolved; a null route parks and holds no provider-wide
  lock. The lift reads no new credential and resolves no new boundary.
- No Windows code, test or evidence (0063). No `contracts/`, `fixtures/`, `policy/` or
  `reference/` edit. Acceptance mints `park-cause.v1`; the held wait reuses `provider-wait/v1`.
  No old journal is backfilled: a park before this decision reads `unknown`.

## The slices

Smallest first; each lands alone, tested, through the merge queue. No slice edits
`crates/brokkr-protocol/src/adapters.rs` except the last, which needs it; #1037 holds that file
now. The first slice is the typed cause, which the lane script and any driver can read at once.

1. **0106a — the typed cause on `run/parked`.** Files: `crates/brokkr-core/src/park.rs` (new
   `ParkCause`), `crates/brokkr-core/src/payload.rs`, `crates/brokkr-core/src/summary.rs`,
   `crates/brokkr-core/src/summary/park.rs`, `crates/brokkr-runtime/src/engine/cursor.rs`,
   `crates/brokkr-runtime/src/engine/cooldown.rs`, `crates/brokkr-view/src/fleet.rs`,
   `crates/brokkr-view/src/run_summary.rs`, `crates/brokkr-cli/src/render/inspect.rs`, the new
   `contracts/park-cause.v1.schema.json`, `crates/brokkr-runtime/tests/frozen_contracts.rs`, and
   core, view, render and runtime tests. About 220 production and 180 test lines. Contract:
   `park-cause/v1`, optional at `event_schema: 1`, minted on acceptance. Tests: each word
   round-trips with its fields; a rule park carries `rule_id`; a cap carries `failure_class`; an
   indeterminate or contention park carries `retryable`; a provider-limit park carries provider,
   route and `until`; `an_old_park_reads_unknown_and_is_not_retryable`;
   `a_malformed_cause_refuses_by_name`; the view, `runs`, `inspect` and the TUI paint the same
   cause.
2. **0106b — the held wait, its readouts and its resume.** Depends on 0103's 4d and 4e; it does
   not edit `adapters.rs`. Files: `crates/brokkr-runtime/src/engine/cooldown.rs`,
   `crates/brokkr-runtime/src/engine/cursor.rs`, `crates/brokkr-runtime/src/admission.rs`,
   `crates/brokkr-runtime/src/admission/capacity.rs`, `crates/brokkr-view/src/fleet.rs`,
   `crates/brokkr-view/src/fleet/need.rs`, `crates/brokkr-view/src/run_summary.rs`,
   `crates/brokkr-cli/src/render/inspect.rs`, `crates/brokkr-cli/src/tui/panes.rs`, and runtime,
   view and render tests. About 260 production and 220 test lines. Contract: none; it reuses
   `provider-wait/v1`. Tests: `a_provider_stated_reset_past_the_maximum_is_held_not_parked` writes
   no `run/parked`, leaves the run `running` and paints `waiting`; a computed or accumulated
   past-maximum wait still parks `provider-limit`; a held wait at the cap parks `attempt-cap`;
   `a_resume_before_the_held_until_refuses_and_names_the_lift`; a resume at or after `until`
   probes; admission holds until the latest `until`, and a later shorter row does not shorten it.
3. **0106c — the operator lift and its clearing rule.** Depends on 0106b and 0103's 4d. Files:
   `crates/brokkr-core/src/fold.rs`, `crates/brokkr-core/src/fold/operator.rs`,
   `crates/brokkr-runtime/src/engine/operator.rs`,
   `crates/brokkr-runtime/src/engine/operator/retry.rs`, `crates/brokkr-store/src/queue/storage.rs`
   (queue storage v6, `provider_lifts`), `crates/brokkr-store/src/queue.rs`,
   `crates/brokkr-cli/src/verbs/delivery.rs`, `crates/brokkr-cli/src/cli_args.rs`,
   `crates/brokkr-view/src/commands.rs`, and core, store, runtime and CLI tests. About 300
   production and 240 test lines. Contract: queue storage v6, a store version. Tests:
   `retry_on_a_waiting_run_lifts_it_and_retry_on_a_parked_run_moves_it` pins the two transitions
   and the same effect id on the lift; `a_ruled_park_is_never_lifted`;
   `a_provider_wide_lift_refuses_with_no_waiting_run`;
   `a_probe_answer_clears_only_rows_recorded_before_it` (a row planted during the probe
   survives); `a_refusal_after_a_lift_writes_a_new_round` and resumes at its new `until`; the
   lift and clear are journaled with the operator's reason.
4. **0106d — orphan adoption.** Depends on #430 slice 6's pass, or builds its first `dispatch
   --once` step. Files: `crates/brokkr-runtime/src/dispatch.rs`,
   `crates/brokkr-runtime/src/dispatch/reconcile.rs`, `crates/brokkr-runtime/src/engine/resume.rs`,
   `crates/brokkr-cli/src/lib.rs`, `crates/brokkr-cli/src/verbs/delivery.rs`, the store's run
   reads, and runtime and CLI tests. About 240 production and 200 test lines. Contract: none.
   Tests: `the_dispatcher_adopts_a_queue_started_running_run_with_no_driver`;
   `a_standing_writer_reservation_blocks_adoption`;
   `a_live_append_moves_the_head_and_the_adoption_writes_nothing`;
   `an_in_flight_attempt_is_settled_before_the_drive`;
   `the_dispatcher_never_adopts_a_parked_or_terminal_run`; a direct run is left to `resume`.
5. **0106e — the infrastructure budget.** Depends on 0106a, 0106d and queue storage v6. Files:
   `crates/brokkr-store/src/queue/storage.rs` (`queue_retry_budgets`, `queue_retries`),
   `crates/brokkr-store/src/queue.rs`, `crates/brokkr-runtime/src/dispatch.rs`,
   `crates/brokkr-cli/src/cli_args/queue.rs`, `crates/brokkr-view/src/fleet.rs`, and store,
   runtime, view and CLI tests. About 220 production and 180 test lines. Contract: queue storage
   v5. Tests: `the_default_budget_is_three`; `only_an_infrastructure_park_spends_the_budget` (a
   `ruled`, an `attempt-cap` and a `provider-limit` park each spend nothing);
   `a_non_retryable_infrastructure_park_spends_nothing`;
   `the_spent_budget_leaves_the_park_to_the_operator`; `a_direct_run_has_no_budget`.
6. **0106f — dsh classifies its 402 and 429 (optional, last).** This is the one slice that edits
   `crates/brokkr-protocol/src/adapters.rs`; it waits for #1037 and a measured dsh 402 and 429.
   Files: `crates/brokkr-protocol/src/adapters.rs`,
   `crates/brokkr-protocol/src/adapters/refusal.rs`, the driver-refusal tests and runtime tests.
   About 160 production and 140 test lines. Contract: `driver-refusal/v2` only if the class
   vocabulary must grow beyond `rate_limit` and `other`. Tests: a measured 402 "Insufficient
   Balance" and a 429 with `retry_after_seconds` read as typed refusals; a dsh exit with no
   measured shape stays unclassified and parks with `attempt-cap` and `failure_class:
   start-failure`. It is optional because 0106a to 0106e change no adapter; it is last because it
   needs the adapter slot.

## Consequences

- A park says what parked it, and a script no longer matches `indeterminate|engine
  restarted|contended` in prose. The lane script reads `cause.kind`; `unknown` and `ruled` are
  never retried.
- A provider-stated long reset is `waiting`, not `needs you`: the run keeps its effect, the lane
  is released, and admission holds the provider and route until the stated end. The operator can
  lift it now, per run or provider-wide, and the probe's answer is the clearing evidence.
- The clearing rule is bounded by 0103's condition: a row recorded before the probe is cleared
  through the sequence snapshot, and a row recorded after it is never removed. A refusal extends
  the wait; it never becomes a silent retry.
- The dispatcher adopts a queue-started orphan and spends a declared budget on `infrastructure`
  parks whose evidence proves no effect landed. It never adopts a run another process drives,
  never adopts a parked run, and never retries a `ruled`, `attempt-cap` or `provider-limit` park.
- The price is one extension schema, one view version, queue storage v6 and v7 (after 0104's v5) and a
  dispatcher pass not built yet. Until it is, a held wait is resumed by the operator at its end
  or lifted early; nothing is lost and nothing is called stale.
- Dsh's 402 and 429 remain `attempt-cap` + `start-failure` until 0106f. The design does not guess
  them from prose, and the operator still sees the real failure class.
- No manifest, witness or compose digest is predicted to move for 0106a: the cause is an
  extension at `event_schema: 1`, not a new event, and the held wait reuses `provider-wait/v1`.
  A move would mean the extension rule was used for a field the fold reads, which this decision
  forbids.

## Evidence

Read at `b56e6909e`. Every file:line above was re-read at that commit with `git grep` and `sed`;
the park writes were enumerated from `EventType::RunParked` in `crates/brokkr-runtime/src`, and
the `effect/indeterminate` writers from the same tree. The journal counts and the controller's
lane-script and delivery-driver behaviour in the Context are the framing's evidence, not reopened
here. The 402, 429 and reset words are the operator's, quoted in #1035; they are not re-measured
by this story. The dsh classifier's absence is the tree's, at the lines above. No contract,
fixture, policy table or reference file was edited; the only change is this file and its index
row.

This decision is proposed. It claims no acceptance, mints no contract and builds no slice. The
operator's "accept all" or a pick per ruling is the next step; the header pointers, the
back-pointers and the contract version are added when that acceptance is recorded.
