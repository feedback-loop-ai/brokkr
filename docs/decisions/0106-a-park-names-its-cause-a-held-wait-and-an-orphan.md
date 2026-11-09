# 0106 — A park names its cause, a long reset is a held wait an operator can lift, and the dispatcher adopts an orphan within a declared budget

Status: proposed
Date: 2026-11-08

Built: unbuilt (#1035)

This decision acts on 0068's rulings 4 and 7 and on 0103's rulings 2, 3 and 5. Because this
story may not edit 0068 or 0103, the header pointer and the two back-pointers are absent until
acceptance; the exact replacement sentences below are the amendment. They stand in [What 0068 and 0103 say after acceptance](#what-0068-and-0103-say-after-acceptance);
the older files are not edited here, and the amendments take effect when this decision is
accepted.

This revision (fix round 1036b, 2026-11-09) answers the chief's security hold (#1036,
c-6728143454) on the first revision at `453e26da2`. The hold's two HIGH findings were that
adoption could not prove a quiet live driver dead, and that an unanswered probe could be spent by
the infrastructure budget as a new effect. Its MEDIUM and LOW findings are answered in the ruling
or the slice each names. The answer to the first is a per-run engine lock that adoption must
itself acquire (ruling 4); the answer to the second is positive non-execution evidence, never an
absent result (ruling 6).

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
(`crates/brokkr-view/src/run_summary.rs:33`, `crates/brokkr-cli/src/render/inspect.rs:32`), and
the TUI paints the same row (`crates/brokkr-cli/src/tui/panes.rs:352`). Every surface shows the
prose, and only the prose, for every cause but a rule park.

The summary row the paged `runs` listing reads (`crates/brokkr-view/src/fleet/page.rs:93-103`) is
written from `Summary` (`crates/brokkr-core/src/summary.rs:442-470`), whose `run_summary` columns
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

### A quiet driver is not proven dead

Adoption's only question is whether a process still drives the run, and the store's fence is not
that proof. `append_next_if_head` checks `(expected_seq, expected_hash)` inside the write
transaction (`crates/brokkr-store/src/lib.rs:630-656`); a driver that is alive but has appended
nothing since the adopter folded leaves the head where it was, and the adopter's append lands.
Decision 0029 states the residual in terms: a driver "alive but has not appended since the
concluding process folded still passes the fence"
(`docs/decisions/0029-fenced-append.md:95-111`). The writer lease is no proof either:
`readopted` takes the lease again only where every driver of the attempt is proven settled
(`crates/brokkr-runtime/src/engine/broker/settlement.rs:466-484`); settlement says the attempt is
closed, not that no process drives the run. `take_dispatch` excludes a second dispatcher and no
direct engine (`crates/brokkr-runtime/src/dispatch/lock.rs:86`). So at HEAD a queue-started run
that is alive, quiet, with every earlier attempt settled and no intervening append is
indistinguishable from an orphan. The per-run engine lock (ruling 4) is the missing proof; until
it is built, adoption is ruled out (ruling 5).

### The queue's storage versions

The tree records queue storage version 3 (`crates/brokkr-store/src/queue/storage.rs:59`).
Accepted 0103 reserves version 4 for `provider_cooldowns` and accepted 0104 reserves version 5
for `seat_trees` (`docs/decisions/0104-shared-local-hardware-sets-a-deadline-floor-and-the-host-sweeps-its-seat-trees.md:162`),
both unbuilt. This decision numbers its two tables after those, once each: `provider_lifts` is
version 6 (slice 0106e) and `queue_retry_budgets` with `queue_retries` is version 7 (slice
0106f). `provider-wait/v2` and `park-cause/v1` are payload extension versions, not queue storage.

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
(`crates/brokkr-core/src/fold.rs:357`); `stop` and `release` are accepted
(`crates/brokkr-core/src/fold.rs:395`), neither lifts a wait, and the wait's one-second
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
| `infrastructure` | An indeterminate end, engine restart in flight, or contention. | `retryable` (positive non-execution evidence, ruling 6); `detail`. | Only when `retryable` and budget remains. |
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

**Who wakes it at `until`.** Recommended: the dispatcher's pass (0106d) at or after `until`, or
`brokkr operator retry` as the lift (0106e) before it. Until the pass lands, `brokkr resume` at
or after `until` takes the wait without sleeping. A held wait with no live engine has no driver
to act on a lift; the pass adopts it, and 0106d is ordered before the lift's 0106e. `resume`
before `until` refuses by name and names the lift; it does not sleep the remainder in process. The pass reads the same `until` and rows, not a second mechanism. Options: (a) the
dispatcher's pass, with the resume hand before it — recommended; (b) the engine sleeps in
process — refused: 0103's held lane and process for days; (c) a new timer process — refused: a
second standing process for a fact the dispatcher owns.

**How the readouts show it.** Recommended: a new `Standing::Waiting` in `brokkr-view`, word
`waiting`, listed under `running` (not `needs you`), with provider, route, `until` and "operator
retry lifts" in the detail. It is not stale: `Need` returns nothing and the stale bound does not
apply. The wait is derived once in `brokkr-view` from the v2 held entry and painted by `runs`,
`inspect` and the TUI; the summary row carries `wait_provider`, `wait_route` and `wait_until`,
all nullable, so the paged listing (`crates/brokkr-view/src/fleet/page.rs:93-103`) and the whole
read (`crates/brokkr-core/src/summary.rs:442-470`,
`crates/brokkr-store/src/summary.rs:153-166`) agree. `inspect` prints a `wait` line; the TUI
paints the same row; `VIEW_VERSION` moves once. Tests:
`a_paged_row_and_a_whole_read_paint_the_same_wait` and
`a_waiting_run_is_not_listed_under_needs_you`. Options: (b) keep `Standing::Running` and put the
wait in `detail` — cheapest, but a narrow `runs` row drops it and the stale rule needs an
exception; (c) a `Need::Waiting` under `needs you` — visible, but says the operator must act when
they need not. Refused. Cost: one view version and the exhaustive matches a `Standing` variant
forces.

### 3. The operator lift

**Recommended: (a) `brokkr operator retry` on a waiting run lifts it; a separate provider-wide
lift names the provider and route; and only a probe's answer clears rows, through the sequence
they were recorded at.**

**Per run.** `brokkr operator retry --run <waiting>` journals a new operator-command class
`lift`. It ends the remaining sleep and lets the run's one driver probe the same effect now; the
next turn keeps the same effect id and idempotency key, and the lift spends no attempt. The
ordinary wait (below `max_seconds`) and the held wait are one path: `take_cooldown`
(`crates/brokkr-runtime/src/engine/cooldown.rs:583-598`) sleeps the remainder in one-second
slices (`:636-658`), and the lift is read there. Every wait is liftable; "held" is not a second
eligibility.

It differs from retry on a parked run in what moves: a parked retry is accepted because the run is
`awaiting_operator` and moves to `Cursor::RequestEffect`, so the engine requests a **new** effect
with a new id (`crates/brokkr-core/src/fold/operator.rs:41`,
`crates/brokkr-runtime/src/engine.rs:899`); a waiting lift moves nothing, because the run is
already at its open effect, and clears only the remaining wait. The journal distinguishes them:
`lift` under the `retry` surface, `retry` for the parked command. A parked run of any cause —
`ruled`, `attempt-cap`, `infrastructure`, `provider-limit` or `unknown` — is refused a lift; a
lift never retries a park. The acceptance is admitted
only while the run is `Run::Running(Cursor::ExecuteEffect)` with a recorded wait; a parked run's
lift refuses `run_not_awaiting_operator`, and a running run with no recorded wait refuses a new
`run_not_waiting`, each by name. The journal records the operator, reason, provider, route and
`until`.

**The acceptance precedes the probe.** `operator/commanded` and `operator/accepted` land before
any probe starts. The engine sees the acceptance on its next sliced re-read and probes only then;
if it dies between the acceptance and the probe, a resume reads the acceptance and probes. A
`stop` accepted after a `lift` wins: nothing may probe after a stop. A `lift` is not itself a
driver; it is journaled in the caller's transaction
(`crates/brokkr-cli/src/verbs/delivery.rs:339`), and the run's one driver — its live engine or the
dispatcher — acts on it.

**How a waiting engine sees it.** The wait loop samples the journal's high-water mark and
re-reads on every slice (`crates/brokkr-runtime/src/engine/cooldown.rs:636-658`); today it only
asks whether the run left `ExecuteEffect`, which a `stop` does (`:680-687`). The slice adds the
lift to that read (a `lifted` field on `RunState`, set by the accepted `lift` and cleared by the
effect's next terminal event) and probes on the next slice. It does not sleep the remainder in
process and needs no second mechanism.

**Who is the one driver.** The per-run engine lock (ruling 4) is the exclusion. A live waiting
engine already holds it and sees the lift. A held wait with no live engine has no driver to see
it: the lift is journaled and waits for the dispatcher's adoption (ruling 5), which must acquire
the same lock before it drives. The slices are ordered so 0106d lands before the lift's 0106e;
until that pass, a held wait with no engine is resumed by `brokkr resume` at or after `until`.
Exactly one driver per lift is tested by `each_lift_reaches_exactly_one_driver` (a live engine
and a competing adopter; the lock refuses the second) and by
`a_lift_with_no_live_engine_reaches_the_dispatcher_once`.

Options: (a) the `lift` class under the `retry` surface — recommended; (b) reuse `retry` and
broaden its acceptance to a running run — refused: it weakens "retry needs a park", makes one
word mean two transitions, and lets a retry land on a normal running effect; (c) a new top-level
`brokkr lift` verb — a second surface for the same command, when the operator asked for `retry`;
(d) a lift that deletes the wait record — refused: the wait is the attempt's record and the probe
needs its `until` and route to ask; deleting it would make the probe invent them.

**Provider-wide.** Recommended: both forms. `brokkr operator lift --provider <p> --route <r>
--reason <why>` journals a provider-wide lift and wakes every waiting run on that provider and
route — ordinary or held — to probe now. It does not clear a row itself: an operator's word is
not evidence the route serves. The first probe's answer writes the clearing fact; a refusal
writes a new row with the next round and resumes the wait at its new `until`. A provider-wide
lift with no waiting run on that route refuses by name: it has no pin and no site to ask, and a
clear without a probe would violate 0103's condition. Each run's one driver (ruling 4) acts, so
one provider-wide lift reaches one driver per run, never a second.

**The clearing rule.** Recommended: `provider_lifts`, an append-only table beside 0103's
`provider_cooldowns` (queue storage v6, after 0104's v5, itself after 0103's v4). At a probe's
start the engine reads the latest `provider_cooldowns.seq` for that provider and route as
`cleared_through_seq`. On an answer it appends one lift row with that snapshot, provider, route,
run, event, site, `answered_at`, operator and reason; the aggregate takes the highest
`cleared_through_seq` for that provider and route and ignores every row at or below it. A row
recorded during the probe has a higher `seq`, so it survives and holds the route again; a row
after the lift survives too. A refusal writes no lift row, writes the next cooldown row, and the
wait resumes at its new `until`. Only an answer writes a lift; a pass, a clock and a manual clear
write none. This is 0103's condition made concrete: the answer is the evidence, and the sequence
snapshot proves no row recorded during or after the probe is removed.

Options: (a) the sequence snapshot and an append-only lift table — recommended; (b) delete the
cleared rows — refused: the table is append-only and a delete can race a row recorded after the
probe; (c) clear by `answered_at` time — refused: equal or moved clocks cannot order a row at the
same instant, while the sequence can; (d) clear all rows on an operator's word — refused: no
evidence, and it removes rows recorded after the probe.

**What Skírnir may grant.** Recommended: a 0025 grant may enumerate `lift` and `lift-provider`,
naming the providers and routes it covers. It may not grant a retry or a lift of a `ruled`,
`attempt-cap`, `infrastructure` or `provider-limit` park; it may not grant a clear without a
probe; and the dispatcher's infrastructure budget is not a grant class, because the budget is
the queue entry's declared data. A grant naming a class or word outside this vocabulary refuses
at load.

### 4. The per-run engine lock

**Recommended: (a) a per-run, OS-level advisory lock the driving process holds for its whole
drive and the kernel releases on its death; every driving path takes it.**

**What it is.** A `flock` exclusive, non-blocking lock on one file per run, held by the process
that drives the run. It is not a lease, a heartbeat, a pid record or a clock: the kernel releases
it when the holder dies, which is the positive proof a quiet live process cannot fake. It is the
same primitive and the same lock-file home as 0095's dispatch lock
(`crates/brokkr-runtime/src/dispatch/lock.rs:1-15`, `:86-115`), which already states "Liveness is
the lock, which the kernel lets go when its process dies: there is no lease, no heartbeat and no
clock read for it."

**Its name and home.** `crates/brokkr-runtime/src/run_lock.rs` (new): `RunLock`, taken through
`crate::lock_file::opened` (`crates/brokkr-runtime/src/lock_file.rs:1-31`) so the file is this
user's own single regular file, owner-only, opened without following a link and `CLOEXEC`; a
second acquisition refuses. The file is `<journal>.run-<run_id>.engine.lock` beside the journal;
`<run_id>` must be one safe path component (no `/`, no `.` or `..`), refused otherwise. The lock
is held by `Engine` for its whole life, not by a call, so no drive can outlive it. `Engine`'s
field is not an `Option` on the production path: there is no unlocked `Engine` to drive.

**How every driving path takes it.**
- `brokkr run` and a dispatcher's start: `Engine::start_claimed` takes it in
  `start::journaled` after `door()` mints the run id (`crates/brokkr-runtime/src/engine/start.rs:42-80`)
  and before `store.start_run` (`:108`); `start_with_dispatch` takes it from the envelope's run
  id in `start::dispatched` before its `start_run` (`:115-130`).
- `brokkr resume`, `rerun` and the operator's member retry: `launch::resume` takes it and hands
  it to `Engine::resume` (`crates/brokkr-runtime/src/engine.rs:614-677`), which owns it before it
  reads the pinned manifest; `rerun` starts a new run through the start path. All three go
  through `launch` (`crates/brokkr-runtime/src/launch.rs:313-347`, `:351-383`, `:386-421`), the
  one admission the CLI and the dispatcher share.
- The dispatcher's adoption is `launch::adopt` (ruling 5), a resume under the lock that opens
  the lock file without creating it; it cannot adopt without acquiring it.
- A delivery verb that only journals a command — `operator retry`, `stop`, `supersede`,
  `classify`, `acknowledge` (`crates/brokkr-cli/src/verbs/delivery.rs:332-340`) — drives nothing
  and takes no engine lock; the run's driver holds it.
- `run_view::drive` (`crates/brokkr-cli/src/run_view.rs:134`) can only drive an `Engine`, which
  already holds it.

**A refusal is typed and final.** A second driver gets a typed `RunLocked { run }` naming the run
(`EngineError`) and writes nothing; it never sleeps, retries or waits for the holder. The
test-only constructors take the lock from the test's journal too, so no fixture drives an
unlocked engine.

**Adoption requires the lock.** A run started before this slice has no lock file; the adopter
opens without creating one, so a missing file is `NoLock` and adoption refuses. `brokkr resume`
creates the file and takes the lock, so the next adoption can rely on it. This is the positive
proof the hold asked for: the adopter holds the one lock the driver would have held. Until this
slice lands, the decision rules adoption out and 0106d stays unstarted.

Options: (a) the per-run `flock` — recommended; (b) a pid file or a lease with a heartbeat —
refused: a stale pid and a missed heartbeat are not positive proof, and 0029:95-111 already
documents the quiet holder; (c) the dispatcher's journal lock alone — refused: it excludes a
second dispatcher, not a direct engine (`crates/brokkr-runtime/src/dispatch/lock.rs:86`); (d)
the writer lease or 0029's fence alone — refused: the fence checks the head only
(`crates/brokkr-store/src/lib.rs:630-656`) and a quiet driver appends nothing (0029:95-111); the
lease says a writer may own the tree, not that the driver is dead
(`crates/brokkr-runtime/src/engine/broker/settlement.rs:466-484`).

### 5. Orphan adoption

**Recommended: (a) the dispatcher adopts a queue-started `running` run whose driver is gone, and
never a run another process drives.**

An orphan is a run whose fold is `Running` and whose driver is gone: the engine died between
effects, or died mid-attempt and the next process found the attempt in flight. Reconcile already
names the gap: a run linked to an entry with a `spawn` row is left for the operator
(`crates/brokkr-runtime/src/dispatch/reconcile.rs:13`). Adoption is the dispatcher's
`launch::adopt` of that run under its pinned bundle, not a new run and not a retry of a parked
one. At `ExecuteEffect` it drives the effect; at `EffectInFlight` it takes the existing `restarted` path
(`crates/brokkr-runtime/src/engine/cursor.rs:45`), which settles the attempt first, so it never
starts a second driver beside an unsettled one. It adopts a queue entry's run only: a direct
`brokkr run` is the operator's, and the dispatcher's authority is the entries it owns.

Options: (a) queue-started runs only — recommended; (b) every `running` run — broader recovery,
but the dispatcher would drive work it did not start and could surprise an operator's own resume;
the fence keeps it safe, but the authority is wider than 0068's; (c) no adoption — refused:
reconcile already leaves the run for a person, which is the orphan the issue names.

**How "no live driver" is proven.** The per-run engine lock (ruling 4) is the proof, and it is
positive: `launch::adopt` opens the run's lock file without creating it and takes it
exclusively. If the file is missing (`NoLock`) or the lock is held (`RunLocked`), a driver lives
or the run predates the lock, and adoption refuses. If the adopter takes it, no other process holds it, and
the adopter keeps it for the whole drive. The other facts are gates, not proof:

- **The writer lease.** The protected worktree's reservation names every attempt the run
  admitted, standing or settled (`crates/brokkr-runtime/src/engine/broker/admission.rs:1`). A
  standing attempt means a writer may still own the tree; adoption refuses. A reservation with no
  standing attempt and every attempt settled is required, but it says the attempt is closed, not
  that no process drives the run.
- **The attempt's settlement.** For `EffectInFlight`, `restarted` reads the reservation and the
  driver tallies and writes the attempt's terminal fact before any drive
  (`crates/brokkr-runtime/src/engine/broker/fold.rs:315`). Adoption does not invent a second
  settlement and does not re-run an unsettled attempt.
- **0029's fence.** The adoption appends on the head its fold read
  (`crates/brokkr-store/src/lib.rs:639`); a writer that does not hold the run's lock and appends
  first moves the head, and the adoption writes nothing. With the lock, the quiet driver is
  already excluded; the fence is the final check, not the first proof.
- **The dispatcher's lock.** `take_dispatch` excludes a second dispatcher
  (`crates/brokkr-runtime/src/dispatch/lock.rs:86`), and nothing more: it does not exclude a
  direct engine, so it is not the proof.

**What it must never do.** Never adopt a run without taking its engine lock. Never adopt a run
another process drives; the lock refuses, and the dispatcher does not retry it into a second
driver. Never adopt a parked or terminal run. Never adopt a run with a standing writer
reservation. Never adopt at `EffectInFlight` without settling the attempt first. Never adopt a
direct run. Never create a lock file for a run that has none: that would manufacture the proof.

**The tests in the slice.** `a_quiet_live_engine_is_never_adopted` keeps the engine alive, its
attempts settled and its journal unwritten, holds its lock, and asserts the dispatcher's adoption
refuses `RunLocked` and writes no event. `a_dead_engines_run_is_adopted_exactly_once` drops the
holder, lets one dispatcher take the lock and drive, and asserts a second dispatcher's attempt
refuses `RunLocked` and the journal has one adoption.

### 6. The infrastructure retry budget

**Recommended: (a) an operator-declared budget on the queue entry, default 3, spent only by an
`infrastructure` park with positive non-execution evidence; when spent, the park is the
operator's.**

**Its home.** `queue_retry_budgets` is queue storage version 7, after 0106e's `provider_lifts`
(version 6) and 0104's `seat_trees` (version 5): one row per entry, the budget and the operator
and reason that set it, absent meaning 3. It is written by `queue add --retry-budget <n>` or
`queue set-retry-budget`, and the spends are append-only `queue_retries` rows, one per dispatcher
retry, naming the run, the park's seq and the cause. The budget belongs to the entry, not the
host: two entries may differ, and a direct run has none.

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

**Ruling 7's text.** Decision 0068's ruling 7 is replaced by the full sentence in [What 0068 and
0103 say after acceptance](#what-0068-and-0103-say-after-acceptance); it authorizes the
adoption and the declared budget and nothing else.

## What 0068 and 0103 say after acceptance

These are the replacement sentences, kept here so the older files are not edited. Each takes
effect when this decision is accepted; the header pointers and back-pointers are added then, not
in this story.

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
run the policy parked, never concludes or stops a run, and never changes an entry's order"
(`docs/decisions/0068-the-dispatcher.md:100-106`) becomes:

> `brokkr dispatch` runs as one process the operator starts, or one pass on demand; it starts
> runs, records cool-downs and sweeps, adopts a queue-started run whose per-run engine lock it
> took and whose driver is gone, and does nothing else. It never retries a run the policy parked,
> except a queue-started run parked with the typed cause `infrastructure`, with `retryable` true
> on positive non-execution evidence and while its entry's declared retry budget remains; it
> never retries a `ruled`, `attempt-cap` or `provider-limit` park, never concludes or stops a
> run, never adopts a run it cannot lock, and never changes an entry's order.

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

**0103 ruling 3 (`:340-344`).** "A single wait longer than `max_seconds`, or the sum of one
effect's waits across its sites passing it, parks." becomes:

> A wait the engine computed longer than `max_seconds`, or the sum of one effect's waits across
> its sites passing it, parks with cause `provider-limit`. A wait whose end the provider stated,
> past `max_seconds`, is held, not parked.

"A retry-after is the provider's own number and is not lengthened; a longer one parks." becomes:

> A provider's retry-after is its own number and is not lengthened; when it alone puts the wait
> past `max_seconds`, the wait is held and admission holds until that end. An engine-computed
> default or accumulated episode past the maximum parks.

**0103 ruling 3's construction sentence (`:356`).** "By construction no row's `until_at` is more
than `max_seconds` after its `recorded_at`" becomes:

> A row whose end the provider stated may lie past `max_seconds`; the maximum bounds the engine's
> own computed wait and episode, not the provider's word.

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

**0103 ruling 5 (`:585-588`).** "A run whose cool-down expires while nobody drives it stays
`running` with the waits in its last terminal event. A resume reads them and retries if attempts
remain; at the cap, or if the declared maximum passed while it waited, the resume parks naming the
cool-down." becomes:

> A run whose cool-down expires while nobody drives it stays `running` with the waits in its last
> terminal event. A resume before a held wait's `until` refuses by name and names the lift; a
> resume at or after it probes and retries if attempts remain; a resume at the cap parks naming
> the cool-down. A held wait is not parked because the maximum passed: it is held, and only its
> own `until`, a lift or the dispatcher wakes it.

## What stays out

- 0071's rules are kept: a closed enum and data, not a trait; typed at the edge; no
  `serde_json::Value` past the edge; the engine passes `now`; ruling 4's ceilings; one home each
  for the cause, the held wait, the engine lock and the lift; one derivation in `brokkr-view`;
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
  lock. The lift reads no new credential and resolves no new boundary. The engine lock is the one
  run's, never the journal's or a provider's.
- No adoption without the per-run engine lock: the dispatcher does not create a lock file for a
  run that has none, does not adopt a run whose lock it cannot take, and does not retry a refusal
  into a second driver.
- No Windows code, test or evidence (0063). No `contracts/`, `fixtures/`, `policy/` or
  `reference/` edit. Acceptance mints `park-cause.v1` and `provider-wait/v2` and numbers queue
  storage v6 and v7. No old journal is backfilled: a park before this decision reads `unknown`,
  and a wait written under v1 reads as not held. The Codex classifier is #1037's, not this
  decision's: no admission is derived from Codex prose.

## The slices

Smallest first; each lands alone, tested, through the merge queue. No slice edits
`crates/brokkr-protocol/src/adapters.rs` except 0106g, and #1037 holds that file now. The first
slice is the typed cause, which the lane script and any driver can read at once. The lock
(0106c) precedes adoption (0106d); the lift (0106e) precedes the budget (0106f); storage versions
follow that order.

**Persistence and backfill.** Every fact here is additive and nothing is backfilled. A park
written before `park-cause/v1` has no `cause` and reads `unknown`, never `ruled`. A wait written
before `provider-wait/v2` has no `held` and reads as not held; a past-maximum wait from that
journal still parks under 0103's rule. The summary columns the cause and the wait add are
nullable, and a row written before them keeps null, which is the same `unknown`/not-held read; no
history row is rewritten. `provider_lifts` and the two budget tables are new and empty, so an old
journal has no lift and every entry reads the default budget of 3; a direct run always has none.

1. **0106a — the typed cause on `run/parked`.** Files: `crates/brokkr-core/src/park.rs` (new
   `ParkCause`), `crates/brokkr-core/src/payload.rs`, `crates/brokkr-core/src/summary.rs`,
   `crates/brokkr-core/src/summary/park.rs`, `crates/brokkr-store/src/summary.rs`,
   `crates/brokkr-runtime/src/engine.rs` (the two park writers at `:519` and `:815`),
   `crates/brokkr-runtime/src/engine/cursor.rs`, `crates/brokkr-runtime/src/engine/cooldown.rs`,
   `crates/brokkr-view/src/fleet.rs`, `crates/brokkr-view/src/fleet/page.rs`,
   `crates/brokkr-view/src/run_summary.rs`, `crates/brokkr-cli/src/render/inspect.rs`, the new
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
   `crates/brokkr-view/src/run_summary.rs`, `crates/brokkr-core/src/summary.rs`,
   `crates/brokkr-store/src/summary.rs`, `crates/brokkr-cli/src/render/inspect.rs`,
   `crates/brokkr-cli/src/tui/panes.rs`, the new
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
   `a_paged_row_and_a_whole_read_paint_the_same_wait`;
   `a_waiting_run_is_not_listed_under_needs_you`.
3. **0106c — the per-run engine lock.** The adoption prerequisite; it adds no adoption. Files:
   `crates/brokkr-runtime/src/run_lock.rs` (new `RunLock`),
   `crates/brokkr-runtime/src/lock_file.rs` (a no-create open for adoption),
   `crates/brokkr-runtime/src/engine.rs` (the `Engine` field, `RunLocked` and `NoLock`),
   `crates/brokkr-runtime/src/engine/start.rs`, `crates/brokkr-runtime/src/launch.rs`,
   `crates/brokkr-runtime/src/lib.rs`, `crates/brokkr-cli/src/run_view.rs`, and runtime, launch
   and CLI tests. About 220 production and 180 test lines. Contract: none; it moves no manifest.
   Tests: `a_second_driver_refuses_the_run_lock`; `a_missing_lock_file_is_no_lock` (adoption's
   open does not create one); `the_lock_file_is_owner_only_single_and_not_a_link`; `a_spawned_seat_does_not_inherit_the_lock`;
   `dropping_the_engine_releases_the_lock`; `every_driving_path_holds_the_lock` (`run`, `resume`,
   `rerun`, the operator's member retry); `no_engine_is_constructed_without_a_lock`.
4. **0106d — orphan adoption and the dispatcher's pass.** Depends on 0106c and #430 slice 6's
   pass, or builds its first `dispatch --once` step. Files:
   `crates/brokkr-runtime/src/dispatch.rs`, `crates/brokkr-runtime/src/dispatch/reconcile.rs`,
   `crates/brokkr-runtime/src/launch.rs` (`launch::adopt`), `crates/brokkr-cli/src/lib.rs`,
   `crates/brokkr-cli/src/verbs/delivery.rs`, the store's run reads, and runtime and CLI tests.
   About 260 production and 220 test lines. Contract: none. Tests:
   `a_quiet_live_engine_is_never_adopted` (alive, every attempt settled, no append; the locked
   engine refuses the adopter and no event is written);
   `a_dead_engines_run_is_adopted_exactly_once` (the holder drops, one dispatcher takes the lock
   and drives, a second refuses); `a_run_with_no_lock_file_is_not_adopted`;
   `a_standing_writer_reservation_blocks_adoption`;
   `a_live_append_moves_the_head_and_the_adoption_writes_nothing`;
   `an_in_flight_attempt_is_settled_before_the_drive`;
   `the_dispatcher_never_adopts_a_parked_or_terminal_run`; a direct run is left to `resume`;
   `a_lift_with_no_live_engine_reaches_the_dispatcher_once`.
5. **0106e — the operator lift and its clearing rule.** Depends on 0106b and 0106d. Files:
   `crates/brokkr-core/src/fold.rs`, `crates/brokkr-core/src/fold/operator.rs`,
   `crates/brokkr-runtime/src/engine/operator.rs`,
   `crates/brokkr-runtime/src/engine/operator/retry.rs`,
   `crates/brokkr-runtime/src/engine/cooldown.rs`,
   `crates/brokkr-store/src/queue/storage.rs` (queue storage v6, `provider_lifts`),
   `crates/brokkr-store/src/queue.rs`, `crates/brokkr-cli/src/verbs/delivery.rs`,
   `crates/brokkr-cli/src/cli_args.rs`, `crates/brokkr-view/src/commands.rs`, and core, store,
   runtime and CLI tests. About 320 production and 260 test lines. Contract: queue storage v6, a
   store version. Tests:
   `retry_on_a_waiting_run_lifts_it_and_retry_on_a_parked_run_moves_it` pins the two transitions
   and the same effect id on the lift; `an_ordinary_wait_is_lifted_like_a_held_one`;
   `the_lift_acceptance_is_journaled_before_the_probe`; `a_stop_after_a_lift_wins`;
   `a_ruled_park_is_never_lifted`; `a_lift_with_no_waiting_effect_refuses_run_not_waiting`;
   `a_provider_wide_lift_refuses_with_no_waiting_run`;
   `each_lift_reaches_exactly_one_driver` (a live engine and a competing adopter; the lock
   refuses the second); `a_probe_answer_clears_only_rows_recorded_before_it` (a row planted
   during the probe survives); `a_refusal_after_a_lift_writes_a_new_round` and resumes at its new
   `until`; the lift and clear are journaled with the operator's reason.
6. **0106f — the infrastructure budget.** Depends on 0106a, 0106d and queue storage v6. Files:
   `crates/brokkr-store/src/queue/storage.rs` (`queue_retry_budgets`, `queue_retries`; queue
   storage v7), `crates/brokkr-store/src/queue.rs`, `crates/brokkr-runtime/src/dispatch.rs`,
   `crates/brokkr-cli/src/cli_args/queue.rs`, `crates/brokkr-view/src/fleet.rs`, and store,
   runtime, view and CLI tests. About 240 production and 200 test lines. Contract: queue storage
   v7. Tests: `the_default_budget_is_three`;
   `only_an_infrastructure_park_spends_the_budget` (a `ruled`, an `attempt-cap` and a
   `provider-limit` park each spend nothing);
   `the_budget_does_not_retry_a_cleanly_settled_unanswered_probe`;
   `an_indeterminate_attempt_spends_nothing`;
`an_infrastructure_park_after_a_failed_to_start_attempt_spends_the_budget`;
   `a_start_failure_with_a_checkpoint_spends_nothing`;
   `a_non_retryable_infrastructure_park_spends_nothing`;
   `a_retryable_true_the_journal_does_not_support_refuses_at_spend_time`;
   `the_spent_budget_leaves_the_park_to_the_operator`; `a_direct_run_has_no_budget`.
7. **0106g — dsh classifies its 402 and 429 (optional, last).** This is the one slice that edits
   `crates/brokkr-protocol/src/adapters.rs`; it waits for #1037 and a measured dsh 402 and 429.
   Files: `crates/brokkr-protocol/src/adapters.rs`,
   `crates/brokkr-protocol/src/adapters/refusal.rs`, the driver-refusal tests and runtime tests.
   About 160 production and 140 test lines. Contract: `driver-refusal/v2` only if the class
   vocabulary must grow beyond `rate_limit` and `other`. Tests: a measured 402 "Insufficient
   Balance" and a 429 with `retry_after_seconds` read as typed refusals; a dsh exit with no
   measured shape stays unclassified and parks with `attempt-cap` and `failure_class:
   start-failure`. It is optional because 0106a to 0106f change no adapter; it is last because it
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
  lift it now, per run or provider-wide, and the probe's answer is the clearing evidence. A v1
  reader meeting the v2 held entry parks instead of retrying.
- The clearing rule is bounded by 0103's condition: a row recorded before the probe is cleared
  through the sequence snapshot, and a row recorded during or after it is never removed. A
  refusal extends the wait; it never becomes a silent retry.
- The dispatcher adopts a queue-started orphan only after it takes the per-run engine lock. A
  quiet live engine holds that lock, so it is never adopted; a dead engine's run is adopted
  exactly once. The dispatcher does not create a lock file for a run that has none.
- The dispatcher spends a declared budget on an `infrastructure` park only with positive
  non-execution evidence. A cleanly settled unanswered probe, an indeterminate attempt and any
  last attempt that may have executed spend nothing; a `ruled`, `attempt-cap` or `provider-limit`
  park spends nothing whatever its text.
- The price is two extension schemas (`park-cause/v1`, `provider-wait/v2`), one view version, one
  per-run engine lock, queue storage v6 and v7 (after 0104's v5) and a dispatcher pass not built
  yet. Until 0106d, a held wait is resumed by the operator at its end or lifted early; a lift
  with no live engine waits for that pass; nothing is lost and nothing is called stale.
- Dsh's 402 and 429 remain `attempt-cap` + `start-failure` until 0106g. The design does not guess
  them from prose, and the operator still sees the real failure class. The motivating Codex case
  stays `attempt-cap` + `start-failure` until #1037's classifier lands; this decision derives no
  admission from Codex prose.
- No manifest, witness or compose digest is predicted to move for 0106a or 0106b: the cause and
  the held wait are extensions at `event_schema: 1`, not a new event, and the held wait is an
  additive version of 0103's extension. A move would mean the extension rule was used for a field
  the fold reads, which this decision forbids.

## Evidence

Read at `b56e6909e`. The inventory above was re-read at that commit with `grep -rn` for
`EventType::RunParked`, `EffectIndeterminate`, `RefusalCause` and the lock calls, and with
`sed -n` and `awk` for each cited range; the park writes and the indeterminate writers were
enumerated from that output (the `RunParked` appends at `crates/brokkr-runtime/src/engine.rs:519`
and `:815`, `crates/brokkr-runtime/src/engine/cursor.rs:216` and `:280`, and
`crates/brokkr-runtime/src/engine/cooldown.rs:725`). The security hold's counterexamples were re-read at the lines the hold names:
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
lines above. No contract, fixture, policy table or reference file was edited; the only change is
this file and its index row.

This decision is proposed. It claims no acceptance, mints no contract and builds no slice. The
operator's "accept all" or a pick per ruling is the next step; the header pointers, the
back-pointers and the contract versions are added when that acceptance is recorded.
