# 0106 — A park names its cause, a long reset is a held wait an operator can lift, and the dispatcher spends a declared retry budget

Status: proposed
Date: 2026-11-08

Built: unbuilt (#1035)

This decision acts on 0068's rulings 4 and 7 and on 0103's rulings 2, 3, 4, 5 and 7. Because this
story may not edit 0068 or 0103, the header pointer and the two back-pointers are absent until
acceptance; the exact replacement sentences below are the amendment. They stand in [What 0068 and 0103 say after acceptance](#what-0068-and-0103-say-after-acceptance);
the older files are not edited here, and the amendments take effect when this decision is
accepted.

This revision (fix round 1036c, 2026-11-09) answers the chief's second security hold (#1036,
c-6730756079) on `25fa2d590`. Orphan adoption leaves this decision: both holds came from it, and
the last showed its per-run engine lock is not proof, because a lock file can be replaced under a
quiet holder. It is carried to a follow-up decision named in [The follow-up this decision
leaves](#the-follow-up-this-decision-leaves), with the hold's counterexample and the requirement it
sets. The lift is consumed by the one probe it authorizes, keyed by probe, site and round (ruling
3); a direct run's lift is driven by `brokkr resume` (ruling 3); and every 0103 passage the second
hold named has its replacement in [What 0068 and 0103 say after
acceptance](#what-0068-and-0103-say-after-acceptance).

The hold's findings, and where each is answered: the HIGH finding is answered by removing orphan
adoption and naming the follow-up that owns stable-identity engine ownership. The three MEDIUMs
are answered in ruling 3 (a lift is consumed by its probe; a refused probe's new wait needs a
fresh lift) and in the amendment section (every 0103 passage replaced or scoped). The five LOWs
are answered as follows: (a) the readouts name the whole read's wait source, its `Account`
propagation, `RunEntry`'s input and its two producers in ruling 2, and the `VIEW_VERSION` moves
are assigned to 0106a and 0106b in the slices; (b) `a_lift_with_no_live_engine_is_taken_by_resume_once`
is in the lift's slice, 0106c; (c) a lift's refusal word is `run_not_waiting`, never a retry's
`run_not_awaiting_operator`, in ruling 3; (d) the provider-wide lift's fan-out record is the set of
`provider_lifts` grant rows sharing its command id, tested by
`a_provider_wide_lift_reaches_two_waiting_runs_once_each`; (e) the dispatcher citation is
`crates/brokkr-runtime/src/dispatch.rs:6`.

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

Measured at `b56e6909e`, the branch's base; every line below was re-read there. The code is
unchanged at `25fa2d590`: that commit changed only this decision and its index row.

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

`Decision` is the closed seven-field record (`crates/brokkr-core/src/decision.rs:86`): `from`,
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
(`crates/brokkr-core/src/fold/run.rs:16`); `park_reason` is that reason (`:42`); `ParkReason` is
only `GateMovedHead` or `Stated(String)` (`:63`). A `run/parked` payload is read for `reason`
alone (`crates/brokkr-core/src/fold.rs:713`).

`brokkr runs` derives one row in `brokkr-view`: `Facts::of` reads status, phase, last ruling and
park reason (`crates/brokkr-view/src/fleet.rs:520`); `Standing::Parked` comes from
`awaiting_operator` (`:538`); the row prints `parked` (`:230`); its rule is shown only when the
last ruling routed nowhere (`:579`); its reason is copied onto the row (`:607`); `Need::Parked`
(`crates/brokkr-view/src/fleet/need.rs:151`) puts it under `needs you`
(`crates/brokkr-view/src/fleet.rs:391`). `brokkr inspect` prints `park <reason>`
(`crates/brokkr-cli/src/render/inspect.rs:33`) from the row's `park_reason`
(`crates/brokkr-view/src/run_summary.rs:32`), and the TUI paints the same row
(`crates/brokkr-cli/src/tui/panes.rs:352`). Every surface shows the
prose, and only the prose, for every cause but a rule park.

The summary row the paged `runs` listing reads (`crates/brokkr-view/src/fleet/page.rs:93-103`) is
written from `Summary` (`crates/brokkr-core/src/summary.rs:442`), whose `run_summary` columns
(`crates/brokkr-store/src/summary.rs:153-166`) carry `status`, `park_reason` and the rule but no
wait and no cause. A paged listing and a whole read agree today only because both lack the fact;
the slices add it to the row once and paint it on both, with a paged-versus-whole test.

### A pre-session refusal, and a seat that exits before its first event

`claude_refusal` (`crates/brokkr-protocol/src/adapters/refusal.rs:99`), `codex_refusal` (`:154`)
and their lanetally twin build a typed `PreSessionRefusal` at the adapter's edge. The class is
closed to `rate_limit` and `other` (`crates/brokkr-protocol/src/lib.rs:42`); `refusal_reason`
(`crates/brokkr-protocol/src/adapters/refusal.rs:52`) sets `rate_limit` only for that exact token
and `other` otherwise (`:65`); the provider's words are bounded once (`:52`). Dsh classifies
nothing: `finish_dsh` writes `refusal: None` (`crates/brokkr-protocol/src/adapters.rs:5329`);
the `refusal: None` at `crates/brokkr-protocol/src/adapters/dsh_launch.rs:471` is a test
fixture's launch, not a provider classifier. 0053 records the gap.

When a seat exits before its first event, the failed report takes `failed_to_start`: a `Failed`
outcome, no `accepted`, no checkpoint, not a deadline kill
(`crates/brokkr-runtime/src/engine.rs:1973`). The terminal `effect/failed` carries
`start_failure: true` and the failing sites (`:1995`). A typed rate-limit refusal takes
`rate_limit_terminal` (`:1440`); every other pre-session refusal keeps 0006's path, retries
within the limit, and at the cap parks with the 0006 text plus `last error`. For the dsh 402
that text was `agent CLI exited 1`; the provider's actual words were only in the stderr tail. A
dsh 402 is therefore not a `provider-limit` today: it is an attempt-cap park whose last failure
is a start failure, and this decision does not guess otherwise.

### The queue's storage versions

The tree records queue storage version 3 (`crates/brokkr-store/src/queue/storage.rs:59`).
Accepted 0103 reserves version 4 for `provider_cooldowns` and accepted 0104 reserves version 5
for `seat_trees` (`docs/decisions/0104-shared-local-hardware-sets-a-deadline-floor-and-the-host-sweeps-its-seat-trees.md:162`),
both unbuilt. This decision numbers its four tables after those, once each: `provider_lifts` and
`provider_lift_probes` are version 6 (slice 0106c), and `queue_retry_budgets` with
`queue_retries` is version 7 (slice 0106d). `provider-wait/v2` and `park-cause/v1` are payload
extension versions, not queue storage.

### The dispatcher and `brokkr operator retry`

The dispatcher is built as far as 0095's lock and reconciliation: `dispatch.rs` says its pass
and the `brokkr dispatch` verb are #430's later slices
(`crates/brokkr-runtime/src/dispatch.rs:6`); `take_dispatch` takes the one journal lock
(`crates/brokkr-runtime/src/dispatch/lock.rs:86`); `reconcile` settles an entry left launching
(`crates/brokkr-runtime/src/dispatch/reconcile.rs:63`). Reconcile never reads a parked run. A
run it linked with a `spawn` row is left alone, "its run reading `Running` for the operator to
resume should its driver have died" (`crates/brokkr-runtime/src/dispatch/reconcile.rs:13`).
That sentence is the orphan the follow-up names; this decision adopts nothing.

`brokkr operator retry` writes `operator/commanded` and `operator/accepted`
(`crates/brokkr-runtime/src/engine/operator.rs:277`); the fold accepts a retry only when the run
is `awaiting_operator` (`crates/brokkr-core/src/fold.rs:391`) and moves it to
`Cursor::RequestEffect` (`crates/brokkr-core/src/fold/operator.rs:41`). The engine then requests
a new effect with a new id (`crates/brokkr-runtime/src/engine.rs:899`). On a waiting run — still
`running` with a recorded wait — the same retry is refused `run_not_awaiting_operator`
(`crates/brokkr-core/src/fold.rs:357`); `stop` and `release` are accepted
(`crates/brokkr-core/src/fold.rs:395`), neither lifts a wait, and the wait's one-second
slices see it and probe nothing (`crates/brokkr-runtime/src/engine/cooldown.rs:636`). The
operator can stop a wait, not lift one. This decision adds `lift` to that surface (ruling 3).

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
| `infrastructure` | An indeterminate end, engine restart in flight, or contention. | `retryable` (positive non-execution evidence, ruling 4); `detail`. | Only when `retryable` and budget remains. |
| `provider-limit` | A typed rate-limit refusal parked below the cap. | `provider`, `detail`, and one `refusal` variant with its conditional fields below. | Never. |

`unknown` is a fifth word only when reading: an absent `cause` reads `unknown`, and every script
treats it as not retryable, like `ruled`. A present `cause` that does not read is refused by
name, never as absent.

`provider-limit` is one cause with five representable refusal variants, one per cause 0103
records (`crates/brokkr-core/src/payload/provider_wait.rs:81-93`). No variant requires a fact
the refusal does not have:

| `refusal` | `provider` | `route` | `until` | `seconds` | `max_seconds` |
|---|---|---|---|---|---|
| `route-unresolved` | required | absent | absent | absent | absent |
| `host-unreadable` | required | required | absent | absent | absent |
| `episode-unreadable` | required | required | absent | absent | absent |
| `past-maximum` | required | required | required | required | required |
| `no-end` | required | required | absent | required | required |

The field rules are enforced at the edge: a present field the variant forbids refuses by name; a
required field the variant lacks refuses by name. The provider's words stay in the park's
`reason` and in the typed `refused` record; the cause does not copy them. `route-unresolved`
has no route and so cannot hold a provider-wide lock; `host-unreadable` and `episode-unreadable` never reached a computed
bound; `no-end` has a computed length but no spellable `until`; `past-maximum` reached both. A
cap park is `attempt-cap` with `failure_class: provider-limit`, and it carries the refusal's
facts in `detail`; the cap's cause is the cap's, not a second `provider-limit`.

**Its home.** Recommended: an optional `cause` object on `run/parked`, published as
`contracts/park-cause.v1.schema.json` under the amended extension rule at `event_schema: 1`
(0016's amendment, as 0103 used it). `reason` and `evidence` stay byte for byte; an old reader
ignores the field; the fold does not read it, so `RunState` gains nothing and an old journal
folds exactly as it does now. The summary and view read it once into a typed `ParkCause`, and
the readouts paint that one derivation. Every `run/parked` writer passes the cause, including
the two in `crates/brokkr-runtime/src/engine.rs` (`:519` for the selector's missing default and
`:815` for journal contention); a writer that cannot name a cause writes none, which reads
`unknown`, never a guessed word.

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
`running`, the wait is recorded under `provider-wait/v2`, no lane is held, and the readouts say
`waiting`.**

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

**Its home.** Recommended: `provider-wait/v2`, an additive version of 0103's extension, never an
edit to v1. The terminal `effect/failed` keeps its type and event schema; each `waits` entry may
carry `held: true` and `max_seconds` — the declared maximum the provider's stated end exceeded.
A held wait's `until` may lie past that maximum. `refused` is not written for a held wait: v1
defines it as the refusal that ends in a park (`contracts/provider-wait.v1.schema.json:5`, `:16`,
`:96`), and `cursor.rs` recovers that park from it
(`crates/brokkr-runtime/src/engine/cursor.rs:117-125`). `refused` keeps that frozen meaning.
The held state is read from `held`, never inferred from `refused` plus the run's status. The run
stays `Run::Running(Cursor::ExecuteEffect)` with its effect open; the engine ends its drive, so
the lane and process are released; the effect id and idempotency key stay, and the retry is the
same effect.

**Compatibility.** A v1 journal reads exactly as today: no entry carries `held`, and a wait past
the maximum still parks. A v1-only reader that meets a v2 `held` entry refuses the entry by name
— v1's `waits` entry is closed (`additionalProperties: false`) and its reader's key list is
closed (`crates/brokkr-core/src/payload/provider_wait.rs:145-154`) — so it parks; it never reads
the entry as no wait and never retries. A v2 reader reads a v1 entry as not held. An entry that
carries both `held` and `refused`, or a held entry with no `max_seconds`, refuses by name. v1
stays frozen; v2 is minted on acceptance by 0106b. Tests:
`a_v1_reader_refuses_a_v2_held_wait_by_name`, `a_v2_reader_reads_a_v1_wait_as_not_held`,
`a_held_wait_carries_no_refused`, and `a_held_entry_with_no_maximum_refuses_by_name`.

Admission records the provider and route in 0103's `provider_cooldowns`, one row per refused site
and round, with `until_at` the held end. Every unexpired row for the provider and route holds new
admissions until the latest `until_at`; a later, shorter refusal never shortens it. This reuses
0103's aggregate and builds no second index. A row whose end the provider stated may lie more
than `max_seconds` after `recorded_at`; that is the one construction sentence 0103 loses.

- **(a) The held wait under `provider-wait/v2`, no new event.** Recommended. Cost: a new
  extension version, the cooldown path branches on the provider-stated flag, the summary and view
  learn `waiting`, admission reads a row that may outlive the maximum. It refuses to sleep: no
  multi-day lane. It also refuses to repurpose `refused`: a held wait is not a park.
- **(b) Hold every wait past the maximum.** Refused: a repeated default or an exhausted episode
  is the engine's own backoff, not the provider's word; holding it forever hides a route that is
  not recovering. That case keeps 0103's park.
- **(c) Keep 0103's park for every past-maximum wait.** Refused: it contradicts the 2026-11-08
  ruling and leaves the long reset in `needs you`.
- **(d) A new `run/waiting` event.** Refused: it moves `EVENT_SCHEMA_VERSION`, every bundle
  manifest, witness and compose digest. The field under an additive extension version carries
  the same fact without moving the event schema.
- **(e) Edit `provider-wait/v1` with a `held` field.** Refused: v1 is frozen, and a v1 reader
  that ignored the field would read a held wait as a park or as no wait. A new version is how
  the frozen bytes keep their meaning.

**Who wakes it at `until`.** Recommended: the operator's `brokkr resume`, because a held wait has
no live engine. The engine ended its drive when it wrote the held entry, so the lane and process
are released and there is no process to wake; `brokkr resume` at or after `until` takes the wait
without sleeping, and before `until` it refuses by name and names the lift. A lift before
`until` is taken by the same `resume` (ruling 3). The ordinary wait below the maximum is
different: the engine stays in its sliced wait and probes at `until` or on a lift. This decision
builds no dispatcher pass and no adoption: a held run with no operator stays `running`, and the
3-hour stale bound names it, as 0103's ruling 7 says. Options: (a) the operator's resume —
recommended; (b) the engine sleeps in process — refused for a held wait: 0103's held lane and
process for days; (c) a new timer process — refused: a second standing process for a fact the
operator's resume already carries. A dispatcher pass may later drive it; that is the follow-up's,
not this decision's.

**How the readouts show it.** Recommended: a new `Standing::Waiting` in `brokkr-view`, word
`waiting`, listed under `running` (not `needs you`), with provider, route, `until` and "operator
retry lifts" in the detail. It is not stale: `Need` returns nothing and the stale bound does not
apply. The wait is derived once in `brokkr-view`; the whole and paged reads carry the same typed
fact:

- **The whole read's source.** `Account::onto` (`crates/brokkr-core/src/summary.rs:269`) reads
  the last terminal `effect/failed` or `effect/indeterminate` event's `waits` into a new `wait`
  reading beside `Reading` (`crates/brokkr-core/src/summary/aggregate.rs:116`); `Summary::of`
  (`crates/brokkr-core/src/summary.rs:475`) copies it to `Summary`'s nullable `wait` field.
  `RunEntry` (`crates/brokkr-view/src/lib.rs:200`) gains one new field, `wait:
  Option<&HeldWait>`, and its two producers set it from the events they already hold:
  `ListedRun::entry` (`crates/brokkr-cli/src/fleet.rs:66`) and `KeptFleet::entries`
  (`crates/brokkr-cli/src/fleet/kept.rs:149`). `Facts::of` (`crates/brokkr-view/src/fleet.rs:520`)
  takes that field: a `Status::Running` run with a held wait is `Standing::Waiting`. `run_row`
  (`:652`), `inspect` (`crates/brokkr-cli/src/render/inspect.rs:33`) and the TUI
  (`crates/brokkr-cli/src/tui/panes.rs:352`) paint the one derivation.
- **The paged read's source.** `run_summary` gains nullable `wait_provider`, `wait_route` and
  `wait_until` in `crates/brokkr-store/src/summary.rs:149-170`, written from `Summary::of`
  (`:298-341`); `crates/brokkr-core/src/page.rs:91` `Row` gains them, and
  `crates/brokkr-view/src/fleet/page.rs:93` `summary_row` builds the same `Facts` from them. A
  page and a whole read then paint the same `waiting` row. `inspect` prints a `wait` line; the
  TUI paints the same row.

`VIEW_VERSION` (`crates/brokkr-view/src/lib.rs:164`) is 29 at HEAD. 0106a takes it to 30 for the
cause, and 0106b takes it to 31 for the wait; each landing re-measures it because another branch
may have moved it. Tests: `the_held_wait_is_read_from_the_terminal_event_into_the_account`,
`a_paged_row_and_a_whole_read_paint_the_same_wait`, and
`a_waiting_run_is_not_listed_under_needs_you`. Options: (b) keep `Standing::Running` and put the
wait in `detail` — cheapest, but a narrow `runs` row drops it and the stale rule needs an
exception; (c) a `Need::Waiting` under `needs you` — visible, but says the operator must act when
they need not. Refused. Cost: one view version per readout slice and the exhaustive matches a
`Standing` variant forces.

### 3. The operator lift

**Recommended: (a) `brokkr operator retry` on a waiting run journals a `lift`; a separate
provider-wide lift names the provider and route; a lift is consumed by the one probe it
authorizes, keyed by probe, site and round; and only a probe's answer clears rows, through the
sequence snapshot.**

**Per run.** `brokkr operator retry --run <waiting>` journals a new operator-command class
`lift`. It records the run, effect, provider, route, until, and one `probe` token for each
`(site, round)` of the recorded wait it authorizes; the acceptance is written before any probe
starts. It ends the remaining sleep and lets the run's one driver probe the same effect now; the
next turn keeps the same effect id and idempotency key, and the lift spends no attempt. The
ordinary wait and the held wait are one eligibility, not two: an ordinary wait's live engine
reads the grant in `take_cooldown`'s one-second slices
(`crates/brokkr-runtime/src/engine/cooldown.rs:583-598`, `:636-658`); a held wait's driver is
`brokkr resume`, which reads the same grant and probes once (below). Every wait is liftable;
"held" is not a second eligibility.

It differs from retry on a parked run in what moves: a parked retry is accepted because the run is
`awaiting_operator` and moves to `Cursor::RequestEffect`, so the engine requests a **new** effect
with a new id (`crates/brokkr-core/src/fold/operator.rs:41`,
`crates/brokkr-runtime/src/engine.rs:899`); a waiting lift moves nothing, because the run is
already at its open effect, and clears only the remaining wait. The journal distinguishes them:
`lift` under the `retry` surface, `retry` for the parked command. A parked run of any cause —
`ruled`, `attempt-cap`, `infrastructure`, `provider-limit` or `unknown` — is refused a lift with
`run_not_waiting`; a terminal run keeps `after_terminal`; a lift never retries a park. The
acceptance is admitted only while the run is `Run::Running(Cursor::ExecuteEffect)` with a
recorded wait; a parked or running-without-wait run refuses `run_not_waiting`, never a retry's
`run_not_awaiting_operator` (`crates/brokkr-core/src/fold.rs:357`; `AwaitingOperator` is not
waiting: `crates/brokkr-core/src/fold/run.rs:16`, `:28`). The journal records the operator,
reason, provider, route and `until`.

**The acceptance precedes the probe, and the probe consumes it.** A per-run `operator/commanded`
and `operator/accepted` land before any probe starts; a provider-wide lift has no run of its own,
so its grant rows are the record, written with the operator, reason and command id. The typed
lift is the `provider_lifts` grant rows written in the same transaction as that record: one row
per authorized `(run, effect, site, round, probe)`, with the provider, route, until, operator,
reason and command id. That set of rows is the lift's whole authority; it names no round that is
not in the recorded wait. At the probe's start the engine writes one `provider_lift_probes`
`start` row in one `Immediate` transaction, keyed uniquely by `(probe, site, round)`, with
`cleared_through_seq` read from the latest `provider_cooldowns.seq`. The unique key is the once:
only the transaction that writes it probes, and a competing driver's insert is refused, re-reads
and finds the key consumed. **A lift is consumed by the one probe it authorizes.** A lift that
names several sites has one token per site, and each token is consumed by its own probe; no token
authorizes a second probe. A lift with no live engine is therefore still exactly one probe per
token: the driver that writes the start row owns it.

**A refused probe needs a fresh lift.** A rate-limit refusal writes the next cooldown row with
`rounds + 1` (0103:287-299) and resumes the wait at its new `until`; it writes no answer and no
new grant. The next round is a key no earlier lift names, so a crash and a resume before the new
`until` probe nothing: the consumed key is the old round, and the new round is unauthorized. A
fresh lift names it, or the run waits to its own `until`. This is the hold's second MEDIUM
answered: the old `RunState.lifted` cleared only at a terminal event, and a refusal writes none;
there is no `RunState.lifted` here, and consumption is the table row, not an event.

**How a waiting engine sees it.** The ordinary wait loop samples the journal's high-water mark and
re-reads on every slice (`crates/brokkr-runtime/src/engine/cooldown.rs:636-658`); today it only
asks whether the run left `ExecuteEffect`, which a `stop` does (`:680-687`). The slice adds one
read: the unconsumed `provider_lifts` grant for this run's effect and current `(site, round)`. It
probes on the next slice, after writing the consumption row. A held wait's grant is read by
`brokkr resume` before this loop: the engine ended its drive (ruling 2), and resume is its
driver.

**Who drives it.** An ordinary wait's live engine sees the lift and probes. A held wait has no
live engine, so the operator's `brokkr resume` is its driver: it reads the grant and probes once
on the same effect, and `resume` before the held `until` refuses by name and names the lift. A
direct run's lift is therefore driven by `resume`; a queue-started held run's is too. For an
ordinary wait, a live engine and a competing `resume` race on the one consumption row, and the
unique key admits one. This is the lift's once, not a general proof that a quiet engine is dead:
the follow-up owns that proof. Exactly one driver per lift is tested by
`a_lift_reaches_exactly_one_driver` (an ordinary wait's live engine and a competing `resume`; the
consumption row refuses the second) and `a_lift_with_no_live_engine_is_taken_by_resume_once`.

**Provider-wide.** Recommended: both forms. `brokkr operator lift --provider <p> --route <r>
--reason <why>` resolves every waiting run on that provider and route — ordinary or held — and
writes one `provider_lifts` grant row per authorized `(run, effect, site, round, probe)`. That
set of rows, sharing the command id, is the provider-wide lift's fan-out record: it names every
run and probe the one command authorizes, and nothing else reads as it. It does not clear a row
itself: an operator's word is not evidence the route serves. Each run's one driver consumes its
own grant and probes once; the first probe's answer writes the answer row and clears through its
snapshot; a refusal writes a new round and waits for a fresh lift or its own `until`. A
provider-wide lift with no waiting run on that route refuses by name: it has no target and no
site to ask, and a clear without a probe would violate 0103's condition.
`a_provider_wide_lift_reaches_two_waiting_runs_once_each` starts two waiting runs on one route,
lifts once, and asserts each reaches one driver and probes once.

**The clearing rule.** Recommended: `provider_lifts` and `provider_lift_probes`, append-only
tables beside 0103's `provider_cooldowns` (queue storage v6, after 0104's v5, itself after 0103's
v4). A `provider_lifts` row is a grant: provider, route, run, effect, site, rounds, probe, until,
operator, reason and the command id; a provider-wide command has one row per target. A
`provider_lift_probes` row is either the probe's start — its unique key is `(probe, site, round)`,
and it carries the `cleared_through_seq` snapshot — or its answer, naming the start row and
`answered_at`. The aggregate takes the highest `cleared_through_seq` among the start rows of
answered probes for that provider and route
and ignores every cooldown row at or below it. A row recorded during the probe has a higher
`seq`, so it survives and holds the route again; a row after the lift survives too. A refusal
writes no answer, writes the next cooldown row, and the wait resumes at its new `until`. Only
an answer clears a row; a pass, a clock and a manual clear write none. This is 0103's condition
made concrete: the answer is the evidence, and the sequence snapshot proves no row recorded
during or after the probe is removed.

Options: (a) the sequence snapshot and an append-only lift table — recommended; (b) delete the
cleared rows — refused: the tables are append-only and a delete can race a row recorded after the
probe; (c) clear by `answered_at` time — refused: equal or moved clocks cannot order a row at the
same instant, while the sequence can; (d) clear all rows on an operator's word — refused: no
evidence, and it removes rows recorded after the probe.

**What Skírnir may grant.** Recommended: a 0025 grant may enumerate `lift` and `lift-provider`,
naming the providers and routes it covers. It may not grant a retry or a lift of a `ruled`,
`attempt-cap`, `infrastructure` or `provider-limit` park; it may not grant a clear without a
probe; and the dispatcher's infrastructure budget is not a grant class, because the budget is
the queue entry's declared data. A grant naming a class or word outside this vocabulary refuses
at load.

### 4. The infrastructure retry budget

**Recommended: (a) an operator-declared budget on the queue entry, default 3, spent only by an
`infrastructure` park with positive non-execution evidence; when spent, the park is the
operator's.**

**Its home.** `queue_retry_budgets` is queue storage version 7, after 0106c's `provider_lifts`
and `provider_lift_probes` (version 6) and 0104's `seat_trees` (version 5): one row per entry,
the budget and the operator and reason that set it, absent meaning 3. It is written by `queue add
--retry-budget <n>` or `queue set-retry-budget`, and the spends are append-only `queue_retries`
rows, one per dispatcher retry, naming the run, the park's seq and the cause. The budget belongs
to the entry, not the host: two entries may differ, and a direct run has none.

Options: (a) the entry's table — recommended; (b) a field in the launch payload — refused: the
payload is the claim's digest and the store keeps it verbatim, so a budget would move every
manifest that carries it; (c) a host constant — refused: per-entry operator data, not a machine
fact; (d) a global default only — refused: the operator must be able to spend more or less on one
commission.

**What spends it: positive non-execution evidence, never an absent result.** 0006's addendum
keeps four facts apart and says a failure the ruling may retry needs receipt or a refusal before
execution; termination alone never makes one
(`docs/decisions/0006-bounded-attempts-and-deadlines.md:231-250`). The budget reads that rule
literally. An `infrastructure` park is `retryable: true` only when the last attempt is proven
never to have executed:

- its failed report is `failed_to_start` under 0053's predicate — no `accepted`, no checkpoint,
  no deadline kill (`crates/brokkr-runtime/src/engine.rs:1973`), and the terminal event carries
  `start_failure: true` (`:1995`); or the engine refused before spawning an attempt at all; and
- every attempt is settled and there is no standing writer reservation.

The absence of a checkpoint, a result, a broker call or a transport frame is corroboration, not
the proof: 0006 says that absence is not receipt. A park whose last attempt may have executed is
never retryable, whatever its cause field says. In particular:

- a cleanly settled probe with no answer is never budget-retryable
  (`crates/brokkr-runtime/src/engine/cooldown/probe.rs:146-187`;
  `crates/brokkr-runtime/src/engine/cooldown/tests.rs:514-535`): the probe spawned a process and
  may have run, and a budget retry as a new effect would evade the original wait and probe;
- an `indeterminate` attempt is never budget-retryable, whatever its reason;
- a cleanup refusal, partial side effect, delivered transport frame, moved-head park or standing
  reservation is `retryable: false` and spends nothing;
- a `ruled`, `attempt-cap` or `provider-limit` park spends nothing whatever its text. The cap is
  the operator's and 0006's safety rules are not weakened by a budget.

`retryable` is computed when the park is written, but the dispatcher re-reads the attempt
evidence at spend time and refuses a `retryable: true` the journal's last attempt does not
support; a stored boolean is not trusted over the events. The dispatcher writes the retry as
`operator/commanded` and `operator/accepted` with command `retry`, operator `dispatcher`, and a
reason naming the park's seq and "infrastructure retry n of N"; the fold's retry moves the run to
a new effect, exactly as an operator's retry does. It writes under the same fence as any other
dispatcher step and asks no model.

**When it is spent.** The park stays: `awaiting_operator`, cause `infrastructure`, `retryable`
true, and the readouts say "budget spent (N of N); needs you". A direct run's infrastructure park
has no budget and is always the operator's.

**The tests in the slice.** `the_budget_does_not_retry_a_cleanly_settled_unanswered_probe` builds
the parked probe and asserts no `queue_retries` row and no new effect;
`an_indeterminate_attempt_spends_nothing`;
`an_infrastructure_park_after_a_failed_to_start_attempt_spends_the_budget` (the qualifying
positive case, with no accepted, checkpoint or deadline kill);
`a_start_failure_with_a_checkpoint_spends_nothing` (the predicate fails, so absence does not
rescue it); `only_an_infrastructure_park_spends_the_budget` (a `ruled`, an `attempt-cap` and a
`provider-limit` park each spend nothing); `a_non_retryable_infrastructure_park_spends_nothing`;
`the_spent_budget_leaves_the_park_to_the_operator`; `a_direct_run_has_no_budget`.

**Decision 0068 ruling 7's text.** Its replacement is the full sentence in [What 0068 and 0103
say after acceptance](#what-0068-and-0103-say-after-acceptance); it authorizes the declared
budget and nothing else. It no longer authorizes an adoption.

## What 0068 and 0103 say after acceptance

These are the replacement sentences, kept here so the older files are not edited. Each takes
effect when this decision is accepted; the header pointers and back-pointers are added then, not
in this story.

The 0103 passages this decision touches are its ruling 2's no-clearing paragraph (`:277-286`),
ruling 3's recommendation (`:319-321`), its computed-wait and episode paragraphs (`:340-344`,
`:352-358`), ruling 4's no-answer and answer bullets (`:419-425`, `:426-428`), ruling 5's
in-process wait (`:456-480`), ruling 7's answer clause and expired-wait paragraph (`:561-563`,
`:585-591`), its `What stays out` bullet (`:644-651`), its slice 4c test (`:745-747`) and its
Consequences (`:806-815`). Every one is quoted and replaced below. No other 0103 passage is
touched; in particular its engine-computed wait and episode bounds stay, and a row recorded during
or after a probe is never removed.

**0068 ruling 4.** "A run parks for the operator only when the provider's refusal is not a rate
limit or the cool-down exceeds a declared maximum."
(`docs/decisions/0068-the-dispatcher.md:85`) becomes:

> A run parks for the operator **only** when the provider's refusal is not a rate limit, or when
> the engine's own computed wait or accumulated episode exceeds a declared maximum. A
> provider-stated reset past the declared maximum is a held wait: the run stays running, the wait
> is recorded with the provider's words and its end under `provider-wait/v2`, admission holds the
> provider and route until that end, and an operator command can lift it now.

**0068 ruling 7.** "`brokkr dispatch` runs as one process the operator starts, or one pass on
demand; it starts runs, records cool-downs and sweeps, and does nothing else. It never retries a
run the policy parked, never concludes or stops a run, and never changes an entry's order — those
remain operator commands" (`docs/decisions/0068-the-dispatcher.md:100-106`) becomes:

> `brokkr dispatch` runs as one process the operator starts, or one pass on demand; it starts
> runs, records cool-downs and sweeps, and does nothing else. It never retries a run the policy
> parked, except a queue-started run parked with the typed cause `infrastructure`, with
> `retryable` true on positive non-execution evidence and while its entry's declared retry budget
> remains; it never retries a `ruled`, `attempt-cap` or `provider-limit` park, never concludes or
> stops a run, and never changes an entry's order — those remain operator commands.

**0103 ruling 2, the no-clearing paragraph (`:277-286`).** Each sentence is answered:

"No row is cleared by a pass or by a probe: only `until_at` expiry retires a row from the
aggregate, and an expired row is history and is never deleted." becomes:

> A row is retired from the aggregate by its `until_at` expiry; an expired row is history and is
> never deleted. A probe's answer also clears the rows recorded before that probe, through the
> sequence snapshot taken at its start; it clears no row recorded during or after the probe.

"A probe that answered is evidence the cooled route served at that moment; it is not evidence
another run's unexpired refusal has ended, so deleting earlier rows would let one run's answer
release a route another run's row still refuses." becomes:

> A probe's answer is evidence the route served at that moment. It is not evidence that a refusal
> recorded during or after the probe has ended: that row has a higher sequence than the snapshot
> and survives, so one run's answer never releases a route a later refusal still holds.

"This decision proposes no clearing rule." becomes:

> This is the clearing rule: the answer is the evidence, and the sequence snapshot is the fence.

"The aggregate is re-read after the probe and before the retry, so a row recorded while the probe
ran holds the route again; if a clearing rule is proposed later, it must carry evidence the route
is no longer refused and must not remove a row recorded after the probe." becomes:

> The aggregate is re-read after the probe and before the retry, so a row recorded while the
> probe ran survives the snapshot and holds the route again.

**0103 ruling 3's recommendation (`:319-321`).** "a wait or an accumulated episode past the
maximum parks" becomes:

> A wait the engine computed, or an accumulated episode, past the maximum parks with cause
> `provider-limit`; a wait whose end the provider stated, past the maximum, is held, not parked.

**0103 ruling 3 (`:340-344`).** "A single wait longer than `max_seconds`, or the sum of one
effect's waits across its sites passing it, parks." becomes:

> A wait the engine computed longer than `max_seconds`, or the sum of one effect's waits across
> its sites passing it, parks with cause `provider-limit`. A wait whose end the provider stated,
> past `max_seconds`, is held, not parked.

"A retry-after is the provider's own number and is not lengthened; a longer one parks." becomes:

> A provider's retry-after is its own number and is not lengthened; when it alone puts the wait
> past `max_seconds`, the wait is held and admission holds until that end. An engine-computed
> default or accumulated episode past the maximum parks.

**0103 ruling 3's remainder and episode paragraph (`:352-358`).** "The sum of the effect's actual
remainders is its episode, and when the next remainder would pass `max_seconds` it parks. An
overlapping row from another run can therefore make an effect's remainder longer than its own
candidate; that longer remainder counts against its episode. By construction no row's `until_at`
is more than `max_seconds` after its `recorded_at`, so the aggregate remainder is never longer
than the newest row's maximum." becomes:

> The sum of the effect's actual remainders is its episode, and when the next remainder would
> pass `max_seconds` it parks with cause `provider-limit`. An overlapping row from another run
> can make an effect's remainder longer than its own candidate; that longer remainder counts
> against its episode. A row whose end the provider stated may lie more than `max_seconds` after
> its `recorded_at`: the maximum bounds the engine's own computed wait and the episode the
> effect accumulates, not the length of the provider's stated row, so an effect whose
> accumulated episode passes the maximum still parks while the row itself stays held for
> admission.

**0103 ruling 4's no-answer bullet (`:419-425`).** "A probe that ends with no answer is an
**indeterminate** end under 0006: … all park and spend no effect attempt. The row is neither
ended nor extended — only its own `until_at` expiry retires it — and no effect retries. …"
becomes:

> A probe that ends with no answer is an **indeterminate** end under 0006: a hang, its deadline, a
> session start with no model output, a lost result, a clean exit before an answer, or an error
> after `turn.started` (which 0053 does not classify) all park and spend no effect attempt. The
> row is neither ended nor extended by that probe, and no effect and no budget retries it: a
> cleanly settled probe with no answer is not positive non-execution evidence, and a budget retry
> as a new effect would evade the original wait and the probe. It parks; only the row's own
> `until_at` expiry or a later answer's snapshot retires it. A driver `failed` report with no
> answer is indeterminate here even where the same report would retry an effect.

**0103 ruling 4's answer bullet (`:426-428`).** "An answer ends this run's wait and permits its
retry. It clears no row: ruling 2 keeps every unexpired row in admission's aggregate until its own
`until_at`, so an answer is not evidence another run's refusal has ended." becomes:

> An answer ends this run's wait and permits its retry. It clears the rows recorded before that
> probe, through the sequence snapshot taken at its start; a row recorded during or after the
> probe has a higher sequence and holds the route again. A refusal writes a new row and resumes
> the wait at its new `until`.

**0103 ruling 5 (`:456-480`).** Its recommended (a) is scoped: an ordinary wait below the
maximum is still taken in process in bounded slices, so an operator `stop` during it is honoured
and the retry keeps the effect. The paragraph that says a resume "reads the same waits and waits
the remainder, or parks past the maximum" becomes:

> An ordinary wait below the declared maximum is taken in process in bounded slices, and a stop
> during it ends the run with no probe or retry. A wait whose end the provider stated past the
> maximum is held: the engine ends its drive and releases the lane and process, the run stays
> `running` with the effect open, and no process sleeps the remainder. The held wait has no live
> engine: `brokkr resume` at or after `until` takes the wait and probes, and before `until` it
> refuses by name and names the lift. An ordinary wait's live engine wakes at its `until` and
> probes. No dispatcher pass is proposed by this decision.

**0103 ruling 7 (`:561-563`), the answer's clearing clause.** "The probe's answer ends this
run's wait; it clears no row and does not reset the chain or re-select the retried effect onto the
cooled link." becomes:

> The probe's answer ends this run's wait; it clears the rows recorded before that probe, through
> the sequence snapshot taken at its start, and does not reset the chain or re-select the retried
> effect onto the cooled link. A row recorded during or after the probe has a higher sequence and
> holds the route again.

**0103 ruling 7 (`:585-591`), the expired-wait paragraph.** "A run whose cool-down expires while
nobody drives it stays `running` with the waits in its last terminal event. A resume reads them
and retries if attempts remain; at the cap, or if the declared maximum passed while it waited,
the resume parks naming the cool-down." becomes:

> A run whose cool-down expires while nobody drives it stays `running` with the waits in its last
> terminal event. A resume before a held wait's `until` refuses by name and names the lift; a
> resume at or after it probes and retries if attempts remain; at the cap it parks naming the
> cool-down. A held wait is not parked because the maximum passed: it is held, and only its own
> `until`, a lift or the operator's resume wakes it. With no driver it is not parked and not
> lost: the 3-hour stale bound names it and the operator resumes or lifts it. Admission admits
> new entries as soon as the aggregate `until_at` passes.

**0103 `What stays out` (`:644-651`).** "A run still parks for a person exactly when it does
today, plus the cap: a rate-limit refusal with attempts remaining waits and probes instead of
parking, and a rate-limit refusal at 0006's cap parks naming the cool-down. … A probe with no
answer parks too and ends no cool-down (ruling 4)." becomes:

> A run still parks for a person when a policy or engine rule refuses, 0006's cap is reached, an
> indeterminate or contention end is not budget-retryable, or a typed provider refusal is not a
> rate limit. A provider-stated rate-limit reset past the declared maximum is a held wait, not a
> park; an engine-computed wait or an accumulated episode past it parks `provider-limit`. A
> probe that refuses again writes the next round and resumes the wait at its new `until`; the
> consumed lift does not authorize that probe, so it needs a fresh lift or its own expiry. A
> probe that ends with no answer still parks and is never budget-retryable. A run's lift is
> taken by its live engine or by `brokkr resume`, never by the dispatcher in this decision.

**0103 slice 4c's test (`:745-747`).** "a cool-down whose computed end passes the declared
maximum, or whose accumulated episode would, parks with the provider's words and the maximum
named; a resume waits the remainder." becomes:

> a cool-down whose engine-computed end passes the declared maximum, or whose accumulated
> episode would, parks with the provider's words and the maximum named; a provider-stated end
> past the maximum is held, and a resume before its `until` refuses by name while a resume at or
> after it waits the remainder.

**0103 Consequences (`:806-815`).** "A rate-limited run shows every refused site's wait with the
provider's words, not a park for a person; every wait is bounded by the declared maximum, and a
wait past it or a refusal at 0006's cap parks. … Before slice 6 the price is the held lane …"
becomes:

> A rate-limited run shows every refused site's wait with the provider's words, not a park for a
> person; an engine-computed wait and an effect's accumulated episode are bounded by the declared
> maximum, and either past it parks, while a provider-stated end past the maximum is held. A
> probe ends the wait only with a provider's answer, never with a session start, a hang, a
> deadline or an error. The retry is an attempt inside 0006's limit; the probe is not, and no
> amendment to 0006 is proposed.
>
> Before any dispatcher pass, the price of a held wait is the operator's hand: the run keeps its
> effect and releases its lane, the held wait has no engine, and `brokkr resume` is its driver;
> an ordinary wait's engine probes at its `until`. The operator can lift early, and each round
> costs one probe per cooled site, whose token cost is unmeasured.

## What stays out

- 0071's rules are kept: a closed enum and data, not a trait; typed at the edge; no
  `serde_json::Value` past the edge; the engine passes `now`; ruling 4's ceilings; one home each
  for the cause, the held wait, the lift and its consumption; one derivation in `brokkr-view`;
  typed errors; every new test shown to bind.
- No model decides admission or a retry. The cause is computed from the journal and the typed
  refusal; the budget is data; the probe asks the route the run already used and its answer is
  evidence, not a verdict.
- No `ruled` park is retried without the operator: residual findings, a security hold, an
  oversized unit, a policy rule, a schema refusal, a moved gate head and a missing seat default
  all stay `ruled`. No `attempt-cap`, `provider-limit` or `unknown` park is retried by the
  dispatcher, and a `ruled` park is never lifted.
- No non-limit refusal becomes a silent retry. 0053's boundary, 0006's timeout, lost-result,
  transport-limit, partial-side-effect and cleanup-failure rows, and 0016's mid-session rule are
  unchanged. A probe that ends without an answer is indeterminate, parks, and is never
  budget-retryable.
- No trust, egress, boundary or schema refusal is weakened. The held wait records only the
  provider and route the typed refusal resolved; a null route parks and holds no provider-wide
  lock. The lift reads no new credential and resolves no new boundary. The lift's grant and
  consumption are one run's effect and site, never the journal's or a provider's whole.
- No orphan adoption and no engine lock. The per-run engine lock at `25fa2d590` is not proposed:
  the follow-up owns stable-identity engine ownership. A run whose driver is gone stays
  `running` for the operator's `resume`; the dispatcher adopts nothing and this decision edits no
  dispatch or launch adoption path.
- No Windows code, test or evidence (0063). No `contracts/`, `fixtures/`, `policy/` or
  `reference/` edit. Acceptance mints `park-cause.v1` and `provider-wait/v2` and numbers queue
  storage v6 and v7. No old journal is backfilled: a park before this decision reads `unknown`,
  and a wait written under v1 reads as not held. The Codex classifier is #1037's, not this
  decision's: no admission is derived from Codex prose.

## The follow-up this decision leaves

Orphan adoption leaves 0106 because its proof did not hold. The first hold asked adoption to
prove a quiet live driver dead; the second showed the proposed per-run lock is not that proof.
`crates/brokkr-runtime/src/lock_file.rs:18-34` checks only the file it opened — this user's
regular file of one link — and cannot tell that the pathname now names another file;
`crates/brokkr-runtime/src/dispatch/lock.rs:9-13` already records that a lock file removed under
a live dispatcher lets a second dispatcher take a lock of its own. The chief reproduced the
counterexample: an adopter acquired `LOCK_EX|LOCK_NB` on a recreated pathname while the quiet
original holder lived. The adopted run would then have two drivers.

The follow-up decision must bind engine ownership to a stable file identity journaled at the
run's start (for example the lock file's device and inode), refuse a replaced lock file, cover
every driving path, and carry a quiet-live-holder replacement test. It must also decide orphan
adoption itself, with the writer lease, settlement and 0029's fence as gates, not proof. Until
it lands, nothing adopts: `reconcile` leaves a linked run with a `spawn` row to the operator
(`crates/brokkr-runtime/src/dispatch/reconcile.rs:13`), a direct run is the operator's, and
`brokkr resume` is the hand. This decision names the follow-up; it does not build it.

## The slices

Smallest first; each lands alone, tested, through the merge queue. No slice edits
`crates/brokkr-protocol/src/adapters.rs` except 0106e, and #1037 holds that file now. The first
slice is the typed cause, which the lane script and any driver can read at once. The lock and
adoption are not here; the lift (0106c) precedes the budget (0106d), and storage versions follow
that order.

**Persistence and backfill.** Every fact here is additive and nothing is backfilled. A park
written before `park-cause/v1` has no `cause` and reads `unknown`, never `ruled`. A wait written
before `provider-wait/v2` has no `held` and reads as not held; a past-maximum wait from that
journal still parks under 0103's rule. The summary columns the cause and the wait add are
nullable, and a row written before them keeps null, which is the same `unknown`/not-held read; no
history row is rewritten. `provider_lifts`, `provider_lift_probes`, `queue_retry_budgets` and
`queue_retries` are new and empty, so an old journal has no lift and every entry reads the
default budget of 3; a direct run always has none.

1. **0106a — the typed cause on `run/parked`.** Files: `crates/brokkr-core/src/park.rs` (new
   `ParkCause`), `crates/brokkr-core/src/payload.rs`, `crates/brokkr-core/src/summary.rs`,
   `crates/brokkr-core/src/summary/park.rs`, `crates/brokkr-store/src/summary.rs`,
   `crates/brokkr-runtime/src/engine.rs` (the two park writers at `:519` and `:815`),
   `crates/brokkr-runtime/src/engine/cursor.rs`, `crates/brokkr-runtime/src/engine/cooldown.rs`,
   `crates/brokkr-view/src/fleet.rs`, `crates/brokkr-view/src/fleet/page.rs`,
   `crates/brokkr-view/src/run_summary.rs`, `crates/brokkr-view/src/lib.rs` (`VIEW_VERSION` 29
   to 30), `crates/brokkr-cli/src/render/inspect.rs`, the new
   `contracts/park-cause.v1.schema.json`, `crates/brokkr-runtime/tests/frozen_contracts.rs`, and
   core, store, view, render and runtime tests. About 260 production and 220 test lines.
   Contract: `park-cause/v1`, optional at `event_schema: 1`, minted on acceptance. Tests: each
   word round-trips with its fields; a rule park carries `rule_id`; a cap carries
   `failure_class`; an indeterminate or contention park carries `retryable`; each
   `provider-limit` variant round-trips, including `route-unresolved` with no route,
   `host-unreadable` and `episode-unreadable` with no bounds, `no-end` with no `until`, and
   `past-maximum` with both; a variant with a forbidden field and a variant missing a required
   one each refuse by name; `an_old_park_reads_unknown_and_is_not_retryable`;
   `a_malformed_cause_refuses_by_name`; both `engine.rs` writers carry the cause;
   `a_paged_row_and_a_whole_read_paint_the_same_cause`; the view, `runs`, `inspect` and the TUI
   paint the same cause.
2. **0106b — the held wait, its readouts and its resume.** Depends on 0103's 4d and 4e; it does
   not edit `adapters.rs`. Files: `crates/brokkr-runtime/src/engine/cooldown.rs`,
   `crates/brokkr-runtime/src/engine/cursor.rs`, `crates/brokkr-runtime/src/admission.rs`,
   `crates/brokkr-runtime/src/admission/capacity.rs`, `crates/brokkr-view/src/fleet.rs`,
   `crates/brokkr-view/src/fleet/need.rs`, `crates/brokkr-view/src/fleet/page.rs`,
   `crates/brokkr-view/src/run_summary.rs`, `crates/brokkr-view/src/lib.rs` (`VIEW_VERSION` 30
   to 31), `crates/brokkr-core/src/summary.rs`,
   `crates/brokkr-core/src/summary/aggregate.rs`, `crates/brokkr-core/src/page.rs`,
   `crates/brokkr-store/src/summary.rs`, `crates/brokkr-cli/src/render/inspect.rs`,
   `crates/brokkr-cli/src/tui/panes.rs`, `crates/brokkr-cli/src/fleet.rs`,
   `crates/brokkr-cli/src/fleet/kept.rs`, the new
   `contracts/provider-wait.v2.schema.json`, `crates/brokkr-runtime/tests/frozen_contracts.rs`,
   and runtime, view, render and store tests. About 300 production and 260 test lines. Contract:
   `provider-wait/v2`, an additive extension at `event_schema: 1`, minted on acceptance. Tests:
   `a_provider_stated_reset_past_the_maximum_is_held_not_parked` writes no `run/parked`, carries
   `held: true` and `max_seconds` in its `waits` entry, writes no `refused`, leaves the run
   `running` and paints `waiting`; a computed or accumulated past-maximum wait still parks
   `provider-limit`; a held wait at the cap parks `attempt-cap`;
   `a_v1_reader_refuses_a_v2_held_wait_by_name`;
   `a_v2_reader_reads_a_v1_wait_as_not_held`; `a_held_wait_carries_no_refused`;
   `a_held_entry_with_no_maximum_refuses_by_name`;
   `a_resume_before_the_held_until_refuses_and_names_the_lift`; a resume at or after `until`
   probes; admission holds until the latest `until`, and a later shorter row does not shorten it;
   `the_held_wait_is_read_from_the_terminal_event_into_the_account`;
   `a_paged_row_and_a_whole_read_paint_the_same_wait`;
   `a_waiting_run_is_not_listed_under_needs_you`.
3. **0106c — the operator lift, its consumption and its clearing rule.** Depends on 0106b. Files:
   `crates/brokkr-core/src/fold.rs`, `crates/brokkr-core/src/fold/operator.rs`,
   `crates/brokkr-runtime/src/engine/operator.rs`,
   `crates/brokkr-runtime/src/engine/operator/retry.rs`,
   `crates/brokkr-runtime/src/engine/cooldown.rs`, `crates/brokkr-runtime/src/engine.rs`,
   `crates/brokkr-runtime/src/launch.rs`,
   `crates/brokkr-store/src/queue/storage.rs` (queue storage v6, `provider_lifts` and
   `provider_lift_probes`), `crates/brokkr-store/src/queue.rs`, the new
   `crates/brokkr-store/src/queue/lift.rs`, `crates/brokkr-cli/src/verbs/delivery.rs`,
   `crates/brokkr-cli/src/cli_args.rs`, `crates/brokkr-view/src/commands.rs`, and core, store,
   runtime and CLI tests. About 340 production and 280 test lines. Contract: queue storage v6, a
   store version. Tests:
   `retry_on_a_waiting_run_lifts_it_and_retry_on_a_parked_run_refuses_run_not_waiting` pins the
   two transitions and the same effect id on the lift; `an_ordinary_wait_is_lifted_like_a_held_one`;
   `a_lift_is_consumed_by_its_probe_once`;
   `a_lift_then_a_rate_limit_refusal_then_a_crash_and_resume_before_the_new_until_makes_no_second_probe`;
   `a_direct_runs_lift_is_driven_by_resume_and_probes_once_on_the_same_effect`;
   `a_lift_with_no_live_engine_is_taken_by_resume_once`;
   `a_lift_reaches_exactly_one_driver` (a live engine and a competing `resume`; the consumption
   row refuses the second); `a_provider_wide_lift_reaches_two_waiting_runs_once_each`;
   `a_provider_wide_lift_refuses_with_no_waiting_run`; `a_ruled_park_is_never_lifted`;
   `a_lift_with_no_waiting_effect_refuses_run_not_waiting`; `a_stop_after_a_lift_wins`;
   `a_probe_answer_clears_only_rows_recorded_before_it` (a row planted during the probe
   survives); `a_refusal_after_a_lift_writes_a_new_round_and_waits_for_a_fresh_lift`; the lift
   and clear are journaled with the operator's reason.
4. **0106d — the infrastructure budget.** Depends on 0106a, 0106c and #430 slice 6's pass, and
   queue storage v6. Files: `crates/brokkr-store/src/queue/storage.rs` (`queue_retry_budgets`,
   `queue_retries`; queue storage v7), `crates/brokkr-store/src/queue.rs`,
   `crates/brokkr-runtime/src/dispatch.rs`, `crates/brokkr-cli/src/cli_args/queue.rs`,
   `crates/brokkr-view/src/fleet.rs`, and store, runtime, view and CLI tests. About 240 production
   and 200 test lines. Contract: queue storage v7. Tests: `the_default_budget_is_three`;
   `only_an_infrastructure_park_spends_the_budget` (a `ruled`, an `attempt-cap` and a
   `provider-limit` park each spend nothing);
   `the_budget_does_not_retry_a_cleanly_settled_unanswered_probe`;
   `an_indeterminate_attempt_spends_nothing`;
   `an_infrastructure_park_after_a_failed_to_start_attempt_spends_the_budget`;
   `a_start_failure_with_a_checkpoint_spends_nothing`;
   `a_non_retryable_infrastructure_park_spends_nothing`;
   `a_retryable_true_the_journal_does_not_support_refuses_at_spend_time`;
   `the_spent_budget_leaves_the_park_to_the_operator`; `a_direct_run_has_no_budget`.
5. **0106e — dsh classifies its 402 and 429 (optional, last).** This is the one slice that edits
   `crates/brokkr-protocol/src/adapters.rs`; it waits for #1037 and a measured dsh 402 and 429.
   Files: `crates/brokkr-protocol/src/adapters.rs`,
   `crates/brokkr-protocol/src/adapters/refusal.rs`, the driver-refusal tests and runtime tests.
   About 160 production and 140 test lines. Contract: `driver-refusal/v2` only if the class
   vocabulary must grow beyond `rate_limit` and `other`. Tests: a measured 402 "Insufficient
   Balance" and a 429 with `retry_after_seconds` read as typed refusals; a dsh exit with no
   measured shape stays unclassified and parks with `attempt-cap` and `failure_class:
   start-failure`. It is optional because 0106a to 0106d change no adapter; it is last because it
   needs the adapter slot.

**The Codex case.** The motivating Codex 429 is not a slice here. Its classifier is #1037, a
separate unit in flight; this decision cites it as the prerequisite and derives no admission from
Codex prose. When #1037 lands, its typed refusal reaches 0106a's cause and 0106b's wait with no
adapter edit in this decision. Until then a Codex 429 stays `attempt-cap` with `failure_class:
start-failure`, and no lane script may match its text to retry it.

## Consequences

- A park says what parked it, and a script no longer matches `indeterminate|engine
  restarted|contended` in prose. The lane script reads `cause.kind`; `unknown` and `ruled` are
  never retried, and a `provider-limit` reads its one `refusal` variant rather than a field it
  does not have.
- A provider-stated long reset is `waiting`, not `needs you`: the run keeps its effect, the lane
  is released, and admission holds the provider and route until the stated end. The operator can
  lift it now, per run or provider-wide, and the probe's answer is the clearing evidence. A
  refused probe's new round is not covered by the lift it consumed, so it needs a fresh lift or
  its own expiry. A v1 reader meeting the v2 held entry parks instead of retrying.
- The clearing rule is bounded by 0103's condition: a row recorded before the probe is cleared
  through the sequence snapshot, and a row recorded during or after it is never removed. A
  refusal extends the wait; it never becomes a silent retry.
- The lift is consumed by the one probe it authorizes, keyed by probe, site and round. A direct
  run's lift is driven by the operator's `brokkr resume`, which probes once on the same effect;
  with no live engine, resume is the hand. The dispatcher adopts nothing in this decision.
- The dispatcher spends a declared budget on an `infrastructure` park only with positive
  non-execution evidence. A cleanly settled unanswered probe, an indeterminate attempt and any
  last attempt that may have executed spend nothing; a `ruled`, `attempt-cap` or `provider-limit`
  park spends nothing whatever its text.
- The price is two extension schemas (`park-cause/v1`, `provider-wait/v2`), two view-version
  moves, queue storage v6 and v7 (after 0104's v5), and no per-run lock or dispatcher adoption.
  Until the follow-up, a run whose driver is gone is resumed by the operator; nothing is lost and
  nothing is called stale. The lift's grant and consumption tables are new and empty on an old
  journal.
- Orphan adoption and stable-identity engine ownership are the follow-up's, with the second
  hold's counterexample and requirement named above. This decision claims no lock, no adoption
  and no proof that a quiet engine is dead.
- Dsh's 402 and 429 remain `attempt-cap` + `start-failure` until 0106e. The design does not guess
  them from prose, and the operator still sees the real failure class. The motivating Codex case
  stays `attempt-cap` + `start-failure` until #1037's classifier lands; this decision derives no
  admission from Codex prose.
- No manifest, witness or compose digest is predicted to move for 0106a or 0106b: the cause and
  the held wait are extensions at `event_schema: 1`, not a new event, and the held wait is an
  additive version of 0103's extension. The lift is queue storage, not a wire contract. A move
  would mean the extension rule was used for a field the fold reads, which this decision forbids.

## Evidence

Read at `b56e6909e`, re-read at `25fa2d590`; the code is unchanged there, and that commit changed
only this decision and its index row. The inventory above was re-read at that commit with
`grep -rn` for `EventType::RunParked`, `EffectIndeterminate`, `RefusalCause` and the lock calls,
and with `sed -n` and `awk` for each cited range; the park writes and the indeterminate writers
were enumerated from that output (the `RunParked` appends at
`crates/brokkr-runtime/src/engine.rs:519` and `:815`,
`crates/brokkr-runtime/src/engine/cursor.rs:216` and `:280`, and
`crates/brokkr-runtime/src/engine/cooldown.rs:725`). The second hold's counterexample was re-read
at the lines it names: the lock-file check (`crates/brokkr-runtime/src/lock_file.rs:18-34`), the
dispatcher lock's own record of replacement
(`crates/brokkr-runtime/src/dispatch/lock.rs:9-13`), and the delivery verb's record-only path
(`crates/brokkr-cli/src/verbs/delivery.rs:332-340`). The lift's refusal word was read at the
existing retry refusal (`crates/brokkr-core/src/fold.rs:357`, `:386`) and the waiting state
(`crates/brokkr-core/src/fold/run.rs:16`, `:28`). The readout path was re-read at
`crates/brokkr-core/src/summary.rs:269` and `:475`,
`crates/brokkr-core/src/summary/aggregate.rs:116`, `crates/brokkr-view/src/lib.rs:200`,
`crates/brokkr-cli/src/fleet.rs:66`, `crates/brokkr-cli/src/fleet/kept.rs:149`,
`crates/brokkr-store/src/summary.rs:149-170` and
`crates/brokkr-view/src/fleet/page.rs:93`. `VIEW_VERSION` is 29 at HEAD
(`crates/brokkr-view/src/lib.rs:164`). The dispatcher citation was corrected to
`crates/brokkr-runtime/src/dispatch.rs:6`.

The other security hold's counterexamples were re-read at the lines the first hold names:
the head-only fence (`crates/brokkr-store/src/lib.rs:630-656`), the settled-attempt adoption
(`crates/brokkr-runtime/src/engine/broker/settlement.rs:466-484`), the cleanly settled probe with
no answer (`crates/brokkr-runtime/src/engine/cooldown/probe.rs:146-187`,
`crates/brokkr-runtime/src/engine/cooldown/tests.rs:514-535`), the ordinary wait loop
(`crates/brokkr-runtime/src/engine/cooldown.rs:583-658`), the closed v1 wait reader
(`crates/brokkr-core/src/payload/provider_wait.rs:145-154`), the queue storage version
(`crates/brokkr-store/src/queue/storage.rs:59`), and the delivery verb's record-only path
(`crates/brokkr-cli/src/verbs/delivery.rs:332-340`). The journal counts and the controller's
lane-script and delivery-driver behaviour in the Context are the framing's evidence, not reopened
here. The 402, 429 and reset words are the operator's, quoted in #1035; they are not re-measured
by this story. The Codex classifier (#1037) and a measured dsh 402/429 shape are unmeasured here
and are named as prerequisites, not observed. The dsh classifier's absence is the tree's, at the
lines above. The hold's reproduced lock replacement was not re-run here; it is the hold's
evidence, not this story's. No contract, fixture, policy table or reference file was edited; the
only change is this file and its index row.

This decision is proposed. It claims no acceptance, mints no contract and builds no slice. The
operator's "accept all" or a pick per ruling is the next step; the header pointers, the
back-pointers and the contract versions are added when that acceptance is recorded.
