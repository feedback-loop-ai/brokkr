# 0064 — A run says how it ended: `stopped` keeps its meaning, and every terminal run carries a typed ending

Status: accepted (operator ruled in chat, 2026-09-21)
Date: 2026-09-21

Built: built — E1 #988, E2 #1002, E3 #1006, E4 #1014, E5 #1015, E6 #1016
Amends: 0026, 0047

## Context

A run ends in one of two statuses, `completed` or `stopped`, and waits in
a third, `awaiting_operator`. Those three words are the fold's, they are
in the frozen event envelope (`run/completed`, `run/stopped`,
`run/parked`), and they are correct as far as they go: the run is over,
or it is not.

What they do not say is how. The operator asked on 2026-09-21 whether
`stopped` paints the right picture, since almost nothing that reads
`stopped` was stopped by an operator. The journal was counted the same
day (the-forge's hearth, 282 runs, 162k events). Of the **149 runs whose
last word is `stopped`**:

| what actually happened | runs |
|---|---|
| the policy table ruled a hard stop; no operator was involved | 62 |
| the run parked, and an operator closed the park later | 75 |
| an operator stopped a run that was still working | 12 |

The 62 policy stops are verdicts: `IMPL-BLOCKED` 19,
`REVIEW-RESIDUAL-SECURITY` 13, `VERIFY-FAIL` 9, `REVIEW-SECURITY-HOLD` 8,
`IMPL-BROKEN-TWICE` 5, `REVIEW-RESIDUAL-ABOVE-MEDIUM` 4, and four more.
A security refusal and a smith that hit a permission wall read the same
word as each other, and the same word as a run somebody pulled.

The 87 operator closures were bucketed by their free-text reason:
infrastructure (a flake, an outage, a full `/tmp`, a ghost driver) 21;
superseded by a re-commissioned run 18; an exhausted reforge the operator
ruled on 18; delivered by hand or landed elsewhere 9; a wrong commission
6; and **15 that no reading of the prose could place**. That last row is
the defect in one number: `stop` and `conclude` take a sentence, so the
cause of an ending is recoverable only by a person reading it, and
sometimes not by them.

Three things break on this.

- **The fleet view cannot answer the operator's real question**, which
  is never "is it over" but "is anything left for me to do". A security
  refusal wants a ruling. A superseded run wants nothing. An
  infrastructure death may want a retry. They are one row colour.
- **The journal cannot be read as a ledger.** Issue #271 proposes the
  journal as an evaluation corpus. A model comparison drawn from it on
  2026-09-21 could not tell a run that failed from a run a better run
  replaced, so a smith's superseded runs counted against it exactly as
  its security refusals did.
- **`completed` has the mirror defect.** It covers a clean review and a
  review that passed flagged residuals through (`REVIEW-RESIDUAL-OK`).

Decision 0047 met the neighbouring problem — a stopped run's residual
finding closed by another run — and its shape is the precedent here: an
operator command that cites, a payload schema over `args`, and a view
that reads what the fold does not. It also drew the line this decision
keeps: the annotation "says nothing about the run's status" (0047
ruling 6). Supersession under 0047 closes *findings* on a finished run;
nothing today says that a *run* was superseded, so everyone writes it
into `conclude`'s free text.

## Rulings

1. **The status vocabulary does not change.** `running`,
   `awaiting_operator`, `completed` and `stopped` stay the fold's four
   words. `run/stopped` keeps meaning "this run is over and did not
   complete", which is true of all 149. No event type is added to the
   envelope, no status is renamed, and the fold stays byte-identical:
   a fold test folds a terminal run with and without every annotation
   this decision adds and asserts the states are equal. The
   byte-identity witnesses do not move.

2. **Every terminal run has an `ending`, and it is a view, never a
   status.** The ending is one word from a closed vocabulary, derived
   from the journal by the read side exactly as decision 0047 ruling 3
   derives its marks. The vocabulary:

   | ending | meaning | what it asks of the operator |
   |---|---|---|
   | `shipped` | completed, review clean | nothing |
   | `shipped-flagged` | completed, residuals passed through flagged | read the residuals |
   | `refused` | a review rule ruled a hard stop on the work | a ruling on the finding |
   | `failed` | verification or a twice-broken smith ended it | a new commission, or none |
   | `blocked` | the smith reported it could not proceed | remove the block |
   | `exhausted` | a bounded loop ran out and the ladder's terminal rung ruled | a debt ruling (0022) |
   | `superseded` | another run replaced this one | nothing; follow the citation |
   | `infrastructure` | the host, a provider or the engine failed; the work was not judged | retry when the cause is gone |
   | `delivered-elsewhere` | the work landed by hand or by another road | nothing |
   | `withdrawn` | the commission was wrong or no longer wanted | nothing |
   | `operator-stopped` | an operator stopped a run that was still working, and named no other cause | nothing |
   | `unclassified` | the journal does not say | classify it (ruling 5) |

   The vocabulary is closed and lives in one place in `brokkr-core`. A
   new ending is a new decision.

3. **A policy ending is derived from the ruling that ended the run, not
   from its rule id.** The last `transition/decided` of a policy-stopped
   run names `from` and `result`, and the result vocabulary is already
   closed. The derivation is a total function over that pair and the
   counters the ruling carried: a review ruling that stops is `refused`;
   a `fail` from verify, a `broken` that exhausts, and any `…-FAIL-TWICE`
   is `failed`; `blocked` is `blocked`; a rule whose `when` cites an
   exhausted counter is `exhausted`. `completed` splits on the severity
   of the last review ruling: `flagged` is `shipped-flagged`, otherwise
   `shipped`. Recipes invent rule ids freely; they cannot invent results,
   so no recipe can produce an ending the view cannot name. **No phase
   machine, policy table or schema changes**, and
   `policy/phase-machine.json` does not move.

   **Enforcement binding:** a sweep test derives the ending of every
   stopping and completing rule in every shipped recipe and bundle, and
   fails on a rule that lands in `unclassified`.

4. **An operator who ends a run names the cause, by type, and cites
   where there is something to cite.** `brokkr operator stop` and
   `brokkr conclude` gain `--cause`, taking exactly the operator-owned
   endings: `superseded`, `infrastructure`, `delivered-elsewhere`,
   `withdrawn`, `exhausted`, `operator-stopped`. `--reason` stays, and
   stays a sentence; the cause is what the machine reads.
   - `superseded` requires `--by-run` and takes `--by-realm`, keyed as
     decision 0026 ruling 3 keys every fleet fact. The cited run must
     exist in the journal it names; it need not be finished, because a
     re-commission is usually fired in the same minute.
   - `delivered-elsewhere` takes `--by-ref`, a commit or a pull request,
     recorded as written and not resolved.
   - `exhausted` is the operator closing a park that an `…-EXHAUSTED`
     rule opened. It accepts no risk by itself: a debt ruling is
     decision 0022's and a closed finding is decision 0047's, and
     neither is implied by this word.
   The cause rides in `args` of the `operator/commanded` event that
   already records the stop, under a payload schema
   `operator-ending.v1.schema.json` over `args` and nothing else, as
   0047 ruling 1 did. **A `--cause` is required** on both verbs from the
   release that carries this decision; omitting it is a usage error that
   names the vocabulary. The envelope is unchanged.

5. **The past is classified by name or not at all.** No ending is ever
   inferred from a reason's prose: the census in this decision's context
   did that once, by hand, to measure the problem, and its 15
   unplaceable rows are why it is not a mechanism. A terminal run whose
   operator closure carries no typed cause reads `unclassified`. The
   operator may classify it afterwards with `brokkr operator classify
   --run <run> --cause <ending> [--by-run …] --reason <text>`, which
   appends one `operator/commanded` event with `command: "classify"` to
   the finished run, takes no `operator/accepted`, and is read by the
   view and not by the fold — decision 0047's shape, for the same reason.
   A later `classify` on the same run supersedes an earlier one, and the
   view says so. Policy endings need no classification: ruling 3 derives
   them for every run already in the journal.

6. **Every read surface shows the ending, and colour says what a row is
   waiting on, never whether it was good.** An ending implies its status
   (`shipped` and `shipped-flagged` are `completed`; every other ending is
   `stopped`), so `brokkr runs` prints the ending in the status column
   for a terminal run and the status for a live one. The table gains no
   column and the feature text keeps its width; `--status` and the JSON
   keep the fold's word.

   A red row says "something went wrong", and of the endings that is
   true of exactly one. A refusal is the review gate doing its job, an
   exhausted loop ended where decision 0022 says it should, and a blocked
   smith was never judged. So tone is keyed to the third column of
   ruling 2's table:

   | tone | endings | what it says |
   |---|---|---|
   | green | `shipped`, `shipped-flagged` | done |
   | yellow | `refused`, `exhausted`, and a parked `awaiting_operator` | waiting on a ruling from the operator |
   | cyan | `blocked`, `infrastructure` | waiting on an action; the work was not judged |
   | red | `failed` | judged, and did not pass |
   | dim | `superseded`, `delivered-elsewhere`, `withdrawn`, `operator-stopped`, `unclassified` | settled, or nothing known |
   | bold | `running` | live, as today |

   `shipped-flagged` stays green because it shipped; `unclassified` is
   dim because it is an absence of information, not a warning.

   A one-character gutter carries the same distinction without colour,
   so the table reads under `NO_COLOR`: `!` a ruling is wanted, `>` an
   action is wanted, `~` shipped with residuals to read, `?`
   unclassified, a space otherwise. The header counts the two that
   matter, separately: `282 runs · 2 want a ruling · 1 wants an action`.
   The existing dim detail line under a row names the rule that ruled a
   policy ending, and the citation for `superseded` and
   `delivered-elsewhere`. A closing line tallies the listing by ending,
   which keeps the unclassified past visible until it is named.

   ```
   brokkr · 282 runs · 2 want a ruling · 1 wants an action · ./.forge/forge.db
     dsh-launch-planner-…-ed4ff1bc  running          implement  seq 1113  1h32m  …
   > issue-307-astra-as-…-8cb205dc  blocked          implement  seq 1233  1h32m  …
       IMPL-BLOCKED
   ! dsh-composite-identity-…7331e  refused          review     seq 902     2d   …
       REVIEW-SECURITY-HOLD
   ~ dsh-launch-planner-…-10abd37c  shipped-flagged  done       seq 424   3h04m  …
     dsh-launch-planner-…-3d08ce19  superseded       review     seq 1337  5h10m  …
       by dsh-launch-planner-…-10abd37c
   ```

   `--ending <word>` filters on it; `--needs-operator` lists exactly the
   `!` and `>` rows. No listing hides settled runs by default: a table
   that silently omits rows is a table nobody trusts; `--hide-settled`
   is the explicit form. `brokkr inspect` and `watch` have the room and
   show both words — `status stopped · ending superseded by <run>` —
   with the ruling's reason sentence for a policy ending. The TUI and
   the console paint the same word as a chip from the same six tone
   classes, added to the console's fixed class allowlist, and the
   console groups the fleet as *wants you*, *live*, *settled*. Muninn
   queues by the ending: a `superseded`, `withdrawn` or
   `delivered-elsewhere` run is never offered as work, and an
   `infrastructure` or `blocked` run is offered as a retry or an unblock,
   never as a failure of the seat that was in it. JSON output carries
   `ending`, `ending_cited` and `wants` (`ruling`, `action` or null)
   under a bumped `view_version`.

   **Enforcement binding:** golden tests render a listing holding every
   ending, once with colour and once under `NO_COLOR`, and a test asserts
   that tone and gutter are total functions of the ending, so a new
   ending cannot ship unpainted. Every ending, rule id and citation
   reaches the terminal through `Safe`, as every journal string does.

7. **The ledger reads endings, and says which it excluded.** Any
   comparison of seats, models or recipes drawn from the journal —
   `brokkr compare`, Muninn's advice, issue #271's corpus — counts
   `shipped`, `shipped-flagged`, `refused`, `failed` and `exhausted` as
   outcomes of the work. It excludes `blocked`, `infrastructure`,
   `superseded`, `withdrawn`, `delivered-elsewhere` and
   `operator-stopped` from any success rate, because in none of them was
   the work judged to its end, and it prints the count it excluded. An
   `unclassified` run is excluded and counted as such. A `refused` run is
   read both ways: against the seat whose work was refused, and for the
   gate that raised the finding — a security hold is the judge's result
   as much as the smith's.

## What this decision does not do

It does not rename `stopped`, add a status, or add an event type. It
does not make an ending a ground for any rule in the phase machine: a
policy reads results and counters, never the view. It does not close a
finding (0047) or accept a risk (0022). It does not classify the
existing journal; ruling 5 gives the operator the verb and leaves the
87 to them.

## Consequences

The fleet view answers the question the operator actually asks. Of the
149 stopped runs counted here, 62 gain an exact ending the day this
ships, with no operator action, because the journal already holds the
ruling that ended them. The other 87 read `unclassified` until named,
which is honest: the journal does not say.

`conclude` and `stop` get slightly harder to type and much easier to
read back. The cost is one required flag; the alternative measured here
is a sixth of operator closures that nobody, including their author,
can place a month later.

The journal becomes usable as the corpus issue #271 wants. A smith is
no longer charged for a run that was replaced, and a recipe is no longer
credited for one that shipped its residuals.

Scratch and experimental recipes cost nothing: they cannot invent a
result, so they cannot produce an ending the view lacks a word for.

Amends the read surfaces of decisions 0026 and 0047 in part: both gain
a column. Extends decision 0047's operator-annotation shape to a second
command. Decisions 0022 and 0029 are unchanged; `conclude` stays fenced.

## Evidence

The census is reproducible from the journal alone: for every run whose
last terminal event is `run/stopped`, read its `reason` (`hard stop
ruled by <RULE>` names a policy stop), whether an `operator/commanded`
stop precedes it, and whether a `run/parked` precedes that. The
controller's scripts that produced the counts above, and the model and
cost comparison that first tripped on the ambiguity, are kept on the
operator's host under the-forge's `.forge/tools/`
(`journal-model-ledger.py`, `journal-economics.py`); the counts are
dated 2026-09-21 and will drift as the journal grows.

## Addendum — 2026-11-03, the endings against what was built since, and the slices that build them (#317), ruled

Addendum status: accepted (operator, 2026-11-03: "accept all", https://github.com/feedback-loop-ai/brokkr/pull/981#issuecomment-6597953409)

Rulings 1 to 7 above were accepted on 2026-09-21 and have not been built.
Since then, decisions 0089, 0092, 0094, 0102 and 0103 and 0006's
2026-10-11 addendum have landed or been ruled, and some of them touch the
same rows, verbs and words. This addendum measures HEAD (`1f135c25f`)
against the seven rulings. Where a choice is open, it states the options
and recommends one, and it lists the slices. It does not re-decide
rulings 1 to 7. It adds no status, event type or ending, and nothing in
it reads a reason's prose. Every claim below cites a file and line at
that commit, or the command that printed it.

### What HEAD holds

**Two homes already tell a policy stop from an operator's.** Both read
"the fold is `Stopped` and its phase is `stop`" as a policy stop:
`Facts::standing` (`crates/brokkr-view/src/fleet.rs:479-488`, giving
`Standing::Stopped` or `OperatorStopped`) and the aggregate's `ending`
(`crates/brokkr-core/src/summary/aggregate.rs:178-189`). That aggregate
already defines a type named `Ending` with the variants `Shipped`,
`ByRule(Option<String>)` and `ByOperator`
(`aggregate.rs:24-34`). `run_summary` stores it as an `ending` column with
the words `shipped`, `by_rule` and `by_operator`, beside an `ending_rule`
column (`crates/brokkr-store/src/summary/aggregate.rs:26-38`, `:89-96`).
Analytics and the recipe graph count from it
(`crates/brokkr-view/src/analytics/figures.rs:28-38`;
`crates/brokkr-view/src/graph/figures.rs:421-423`).

**How a run comes to be stopped.** Three writers append `run/stopped`:
- the engine's policy stop, with `hard stop ruled by <rule>`
  (`crates/brokkr-runtime/src/engine.rs:893-901`);
- the engine at `Cursor::Stop` (`engine/cursor.rs:115-122`);
- `conclude` (`engine/operator.rs:430-442`).

Both operator paths write the reason
`OPERATOR-STOP: operator '…' commanded …: …`
(`engine/operator/conclusion.rs:51-55`).

`operator stop` journals `operator/commanded` with `args: {}`
(`engine/operator.rs:337`, `engine/operator/disposition.rs:46`). The
operator's reason rides on `operator/accepted` (`disposition.rs:73`).
`conclude` on a run with no stop in force commands one, also with
`args: {}` (`operator.rs:449-469`). On a run whose stop is already in
force it commands nothing, and its own `--reason` is not journaled at all
(`operator.rs:407-424`). The fold reads only `command_id` and `command`
from `operator/commanded` (`crates/brokkr-core/src/fold/operator.rs:11-16`).
`args` is kept verbatim (`crates/brokkr-core/src/payload.rs:194-199`). A
cause in `args` therefore moves no fold byte, as ruling 1 requires.

**The ruling event.** `transition/decided` carries `from`, `result`,
`rule_id`, `next`, `severity`, `inputs` and `problem`, and refuses any
other field (`crates/brokkr-core/src/decision.rs:84-99`). It does not
record the rule's `when`. The run manifest pins file digests, not the
composed table (`contracts/run-manifest.v14.schema.json` properties,
printed with `jq -r '.properties|keys'`). The rule's `result` is an open
string: `contracts/phase-machine.v2.schema.json:50` reads
`"result": { "type": "string" }`. Each seat declares its own results
(`recipes/fast/bundle.json:9`). Ruling 3's sentence "they cannot invent
results" is therefore untrue at HEAD. A recipe can declare any result
word, and ruling 3's sweep is what keeps the shipped ones named. The
severities are `normal`, `flagged` and `hard`
(`crates/brokkr-core/src/policy.rs:57`).

**Every stopping and completing pair that ships.** `jq` over every
`recipes/*/policy.json` and `policy/phase-machine.json` printed the rules
whose `next` is `stop` or `done`. Five recipe policies hold such rules:
`fast`, `preflight`, `verify`, `research` and `triage`. The other
recipes extend one of them.

| pair (`from`, `result`) | rules | ruling 3 as written |
|---|---|---|
| `implement`, `broken` | `IMPL-BROKEN-TWICE` | `failed` |
| `implement`, `blocked` | `IMPL-BLOCKED` | `blocked` |
| `verify`, `fail` | `VERIFY-FAIL`; `VERIFY-FAIL-EXHAUSTED` (fast, `when` cites `visits_implement_gte`) | `failed`, and also `exhausted` for fast's: two clauses |
| `review`, `security-hold` / `residual` | `REVIEW-SECURITY-HOLD`, `REVIEW-RESIDUAL-ABOVE-MEDIUM`, `REVIEW-RESIDUAL-SECURITY`; `REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM` (fast, `visits_implement_gte`) | `refused`, and also `exhausted` for fast's: two clauses |
| `specify` / `design` / `tasks`, `fail` | `…-FAIL-TWICE` (triage) | `failed`, but only through the rule id |
| `ship`, `ready` / `shipped` | `SHIP-DIRTY`, `SHIPPED-DIRTY` (fast, so every recipe that extends it) | **no clause: `unclassified`** |
| `review`, `clean` / `residual` → `done`; `ship`, `shipped` → `done`; `verify`, `pass` → `done` | `REVIEW-CLEAN`, `REVIEW-RESIDUAL-OK` (`flagged`), `SHIP-COMPLETE`, `VERIFY-PASS` | `shipped` / `shipped-flagged` |

`policy/phase-machine.json` also holds `architecture`/`failed`,
`regression`/`fail`, `review`/`clean-unverified`, `verify`/`stack-conflict`
and `verify`/`env-degraded`. No shipped recipe composes that table. Only
tests read it (`grep -rln phase-machine.json crates/` printed four test
files). The sweep as written would fail at HEAD on fast's two `ship`
stops. Separately, `flagged` also marks rulings outside `review`, such as
fast's `SHIP-DRIFT` and the v1 table's `VERIFY-RISK-UNMITIGATED`.

**The parks added since.**
- 0006's addendum: a deadline kill, a protocol violation after `start`
  and a lost result each end the attempt `effect/indeterminate` and park
  the run (`0006-bounded-attempts-and-deadlines.md:267-276`). Its
  `Built:` line says slice 1 built those rows.
- 0092 (`Status: proposed`, `Built: built`): a rule-driven park,
  `IMPL-BLOCKED-ENVIRONMENT`, in `recipes/fast/policy.json`.
- 0103 (accepted, unbuilt, #430): a cool-down keeps the run `running` and
  out of "needs you"
  (`0103-a-provider-s-rate-limit-becomes-a-cool-down.md:252`). It parks
  past its maximum, or at 0006's cap.

Every park is `awaiting_operator`, not terminal. So none has an ending
until an operator closes it, and closing one always takes an operator
stop: either `operator stop` and then the driver's `run/stopped`, or
`conclude`.

**The verbs added since.**
- `operator acknowledge` appends nothing to the run
  (`crates/brokkr-cli/src/verbs/acknowledge.rs:26-61`).
- `operator release` writes `args: {"reason": …}` and moves nothing
  (`engine/operator/release.rs:77-79`; `fold.rs:285-289`).
- `operator retry --member` writes `args: {"members": […]}` under
  `panel-carry.v1`, which closes `args`
  (`contracts/panel-carry.v1.schema.json:71-83`). Retry is admitted only
  on a parked run.
- `run --enter … --continues` requires every continued run to be
  concluded already, and writes nothing to it
  (`crates/brokkr-runtime/src/launch.rs:436-475`).

None of the four ends a run, and none takes a cause. `stop` and
`conclude` are still the only verbs that end a run by hand.
`operator classify` does not exist (`grep -rn classify` over the
operator modules, the CLI args and the console found none). `--by-run`
and `--by-realm` already exist on `OperatorArgs` for `supersede`
(`crates/brokkr-cli/src/cli_args.rs:315-324`). Its other flags are refused
off `supersede` (`verbs/delivery.rs:230-235`).

**Every in-repo caller a required `--cause` breaks.**
- The dsh console builds `[operator stop | conclude, --run, --db,
  --reason]` (`extensions/dsh/brokkr-console/lib/host/brokkr.js:74-78`),
  with the commands listed in `lib/host/commands.js:12`. Its tests press
  them (`tests/commands.test.js`, `tests/fence.test.js`,
  `tests/client.test.js`).
- In the guides, `docs/guides/quickstart.md:769` and `:853` are fenced
  commands that `every_fenced_brokkr_command_in_a_living_doc_parses`
  parses with clap (`crates/brokkr-cli/src/cli_reference_tests.rs:1091`).
  `quickstart.md:775` says `--reason` is required.
  `docs/reference/cli.md:480-500` is rendered from clap.
- Way-out texts that the TUI tests parse:
  - `crates/brokkr-view/src/fleet/need.rs:129`, with its goldens in
    `fleet/tests.rs`, `crates/brokkr-cli/src/tui/fleet_tests.rs` and three
    TUI snapshots;
  - `crates/brokkr-cli/src/run_view.rs:202` and its test;
  - `verbs/acknowledge.rs:41`.
- CLI argv in tests:
  - `crates/brokkr-cli/tests/realms.rs:489`, `:499`;
  - `brokkr_folder.rs:1115`, `:1117`;
  - `machine_proof.rs:1527-1535`;
  - `machine_proof/release.rs:91-98`.
- Args built in process: `crates/brokkr-cli/src/tests.rs:398-463`,
  `:3711-3762`; `verbs/tests.rs:109`, `:148`;
  `verbs/delivery/member_tests.rs:55-61`.

No script under `scripts/`, `recipes/` or `agents/` calls either verb.
`grep -rn -e "brokkr conclude" -e "operator stop" -e '"conclude"'` over
the three printed nothing. The word "conclude" appears there only as
prose, in three `shipper.md` charters and `recipes/fast/policy.json`. The operator's own
scripts outside the repository call `--reason` alone. That is recorded
here as a fact, not as a design input.

**The read surfaces.**
- `VIEW_VERSION` is 28 (`crates/brokkr-view/src/lib.rs:163`). 0103's
  slice 4f plans 29, "as measured when built"
  (`0103-a-provider-s-rate-limit-becomes-a-cool-down.md:788`).
- `brokkr runs` builds its six cells from `status_word()`
  (`crates/brokkr-cli/src/render.rs:223-246`) and tints only the status
  cell, by `tone`: `stopped` is red, `stopping` falls to dim
  (`render.rs:154-170`).
- The listing has no `--status` flag (its args are
  `cli_args.rs:522-540`). A single-journal listing prints no header; the
  `realm · N runs · journal` line is a many-hearth listing's only
  (`render.rs:326-356`). There is no closing tally.
- `status_word` is 0102's one painted word, and only `Stopping` changes it
  (`fleet.rs:232-242`). The stopping detail reads "stop accepted at seq N;
  no ending written. …" (`fleet.rs:249-254`).
- `inspect` and `watch` print `status_word · phase · seq` and a `ruling`
  line (`crates/brokkr-cli/src/render/inspect.rs:22-57`).
- `/api/runs` and `/api/view` serialize `RunRow`/`RunsView` and `RunView`
  (`crates/brokkr-cli/src/ui.rs:78-98`, `:180-190`).
- The console's allowlist is `BADGE_CLASS` (`crates/brokkr-cli/src/ui.html:244-245`),
  which holds the four fold words and no `stopping`. The TUI tones by the
  same `render::tone` (`crates/brokkr-cli/src/tui/style.rs:57-63`).
- `compare` prints the raw fold status
  (`crates/brokkr-cli/src/compare.rs:250`).
- Muninn's report validator checks each work-queue entry's run citation
  (`crates/brokkr-cli/src/muninn.rs:645-658`). It reads no standing or
  ending today, and it is the only Muninn code that takes a run as work.

**Projections.** `run_summary` already keeps `status`, `phase`,
`rule_id`, `rule_from`, `residual_open`, `stop_accepted_seq`, `ending` and
`ending_rule` (`crates/brokkr-store/src/summary.rs:154-174`). It keeps
neither the last ruling's `result` nor its severity, nor a cause.
`DATABASE_SCHEMA` is 1 (`crates/brokkr-store/src/schema.rs:16`). 0102
added two nullable columns and a stamp trigger without moving it
(`crates/brokkr-store/src/summary/stop.rs:12-45`).

### The choices

**Choice 1: ruling 3's derivation, restated over the measured pairs.**
Ruling 3's clauses overlap on fast's two `…-EXHAUSTED` stops. Its
`…-FAIL-TWICE` clause reads a rule id. It also names no pair for fast's
`ship` stops. The journal cannot show which `when` matched.
- **(a) A table over (`from`, `result`), in order, with no `exhausted` on
  a policy stop.** *Recommended.*
  1. `blocked` from any phase is `blocked`.
  2. `broken`, `fail` or `failed` from any phase is `failed`.
  3. Any other result from `review` is `refused`.
  4. `ship` with `ready` or `shipped` is `blocked`: the worktree is dirty,
     and the operator's action is to clean it.
  5. `verify` with `stack-conflict` is `blocked`, and `verify` with
     `env-degraded` is `infrastructure`. These two rows exist for journals
     ruled by the v1 table.
  6. Anything else is `unclassified`.

  A completed run is `shipped-flagged` when its last ruling from `review`
  carried `flagged`, and `shipped` otherwise. That is ruling 3's own
  sentence. The other flagged rulings, which mark a route rather than
  residuals, are not read.

  Fast's `VERIFY-FAIL-EXHAUSTED` reads `failed`, because verification
  judged the work and it did not pass. Its
  `REVIEW-REFORGE-EXHAUSTED-ABOVE-MEDIUM` reads `refused`, because it
  wants a ruling on above-medium residuals. `exhausted` stays the
  operator's word for closing an `…-EXHAUSTED` park, the row where the
  2026-09-21 census counted 18 operator closures. An open result
  vocabulary lands in
  `unclassified`, never in a guess, and the sweep keeps shipped recipes
  out of it.

  Cost: no policy stop reads `exhausted`. Refuses: nothing ruling 2
  names. Keeps rulings 1 and 3 and "no phase machine, policy table or
  schema changes".
- **(b) Look up the matched rule's `when` in the run's pinned table.**
  The table is not in the journal, only its file digests. Reading the
  bundle from disk at read time is I/O outside the journal, and a bundle
  edited since would change a finished run's ending. Refused.
- **(c) Journal the exhaustion on `transition/decided`.** This needs an
  extension schema over a payload that refuses unknown fields. It is
  exactly the schema change that ruling 3 rules out. It would take a
  decision of its own.

**Choice 2: 0102's `stopping` and the word "ending".** 0102 uses
"ending" for the terminal event, and this decision uses it for the view's
word. Read against 0102's rulings, the two never disagree. A run has an
ending exactly when its terminal event is written, and a stopping run has
neither.
- **(a) One rule in prose, and `status_word` carries the ending.**
  *Recommended.*
  - **In prose:** new text says "terminal event" for the event and
    "ending" for the word. 0102's detail text stays byte-identical,
    because "no ending written" is true under both readings.
  - **In `brokkr runs`:** the status column prints the ending for a
    terminal run, `stopping` for a stopping run and the fold's word
    otherwise, all through `status_word`, 0102's one painted word.
    `status` keeps the fold's word (ruling 6).
  - **In the JSON:** a stopping run has `ending: null` and
    `wants: "action"`, matching 0102 ruling 2's "needs you". It is painted
    cyan with `>`.
  - **Other standings with no row in ruling 6's table:** a quarantined row
    not yet acknowledged wants an action too (acknowledge), cyan with `>`.
    A cool-down under 0103 is `running`, bold, with no gutter mark.

  Cost: `status_word`'s value set grows, which is a `VIEW_VERSION` bump.
  Refuses: nothing.
- **(b) Rename 0102's detail text** to "no terminal event written". This
  moves a pinned text and needs a ruling on 0102's own words, for no
  change of meaning. Not recommended.
- **(c) A stopping run shows its stop's cause as a pending ending.** This
  adds a derived fact that rulings 2 and 6 do not have, and it would read
  as terminal on a live run. Refused here. If wanted, it is a later
  decision.

**Choice 3: what a concluded park reads.** Every park, new or old, ends
only through an operator stop. So its ending is the cause the operator
names, and nothing is derived from the park's reason (ruling 5).
- **(a) Name the cause, with a short guard on `exhausted`.**
  *Recommended.* The expected causes:
  - 0006's indeterminate park: `infrastructure` when the host or provider
    failed, otherwise `withdrawn` or `superseded`.
  - 0092's `IMPL-BLOCKED-ENVIRONMENT`: `infrastructure`. "Remove the
    block" is that word's "retry when the cause is gone", and ruling 4's
    list does not hold `blocked`.
  - 0103's park past the maximum: `infrastructure`.
  - An `…-EXHAUSTED` park: `exhausted`.

  `--cause exhausted` is refused unless the run is parked by a rule (the
  fold's last decision names a rule and takes no transition). The rule
  id's spelling is never read.

  Cost: one refusal text. Refuses: `exhausted` on an engine park or a
  live run.
- **(b) A default cause read from the typed park reason**, for example
  0006's indeterminate park as `infrastructure`. Refused: ruling 4 makes
  these causes the operator's, and a deadline kill is as often the seat's
  hang as the host's.
- **(c) No guard on `exhausted`.** Simpler, but it admits a realistic
  slip, calling an outage "exhausted", which a debt reading would then
  count.

**Choice 4: making `--cause` required.**
- **(a) Required at once, in the slice that adds it, with every in-repo
  caller listed above moved in that same slice.** *Recommended.*
  - The console gains a cause picker, and `superseded` asks for the run.
  - The guides and the way-out texts name `--cause`.
  - The CLI reference is regenerated.

  `conclude` always takes `--cause`:
  - When it commands the stop itself, the cause rides in that stop's
    `args`.
  - When a stop is in force carrying the same cause, nothing more is
    written.
  - When the stop carries another cause or none (an older journal's),
    `conclude` appends one `classify` annotation after `run/stopped`.
    That is ruling 5's event, and by ruling 5 it stands over the stop's.

  Cost: one release in which the operator's own scripts fail with a usage
  error naming the vocabulary, as ruling 4 says.
- **(b) One release where `--cause` is optional and its absence warns,
  then required.** The cost is a release of new `unclassified` closures,
  which ruling 4 was written to stop, and a deprecation flag to remove
  (decision 0071 ruling 6).
- **(c) Optional for good.** This is the free text ruling 4 rules out.
  Refused.

**Choice 5: where the ending is computed for a page.**
- **(a) In the projection, written from the writer's own fold.**
  *Recommended.* One pure function in `brokkr-core` derives the ending
  from the fold and a small reading of events: the last `review`
  ruling's severity, the cause in the accepted stop's `args`, and the
  latest `classify`. The reading is noted per event, as the aggregate's
  `Reading::note` already does (`summary/aggregate.rs:193-199`), and it
  is carried on the summary's `Account`, which already carries 0047's
  marks (`crates/brokkr-core/src/summary.rs:1-10`).
  - The `ending` column takes the twelve words, in place of its three,
    and its `Ending` type is the one in `brokkr-core`.
  - `ending_rule` stays.
  - New nullable columns hold the citation (realm, run or ref), whether a
    later `classify` stood over an earlier one, and a stamp
    `ending_folded_seq`. A trigger sends a row behind when an older
    writer folded it, as 0102's `stop_folded_seq` does.
  - `DATABASE_SCHEMA` stays 1. A read-only reader falls back to #744's
    kept read.

  A whole read and a paged read call the same function, and `doctor`'s
  equality check covers it. Cost: one backfill refold of every run at the
  next writable open. Its time at 0089's 1.1 GB journal is unmeasured.
- **(b) Derived per row at read time.** Each terminal row would need two
  or three keyed reads through `event_index` for the last review ruling,
  the stop's command and any `classify`. Its cost against 0089's 50 ms
  first-page budget at 1.1 GB is unmeasured. 0102 ruling 3 put its
  standing on the row precisely so that a paged read prints what a whole
  read prints without reading events. (b) would reach the same output by
  reading events for every row.

**Choice 6: what the header's "want a ruling" counts (named, not
made).** Ruling 6 keys tone and gutter to the ending alone, so every
`refused` run wants a ruling for as long as it stays in the journal. The
2026-09-21 census already held at least 25 such stops (13 + 8 + 4), so
the header's count would grow and never fall. Two readings:
- **(a) As written.** *Recommended*, because ruling 6's enforcement
  binding makes tone and gutter total functions of the ending.
- **(b) A refused run stops wanting a ruling once 0047 has closed its
  residual findings** (`residual_open` is already on the row). That makes
  the gutter a function of more than the ending. It would take a new
  decision, so it is only named here.

**Choice 7: what the build mints.** Under 0082 ruling 1, the accepted
0064 already authorizes `operator-ending.v1.schema.json`.
- **(a) Exactly that, with one bump.** *Recommended.* The schema is over
  `args` of `stop` and `classify`:
  - `cause` is required, from the six operator-owned endings;
  - `by` (`realm`, `run_id`) goes with `superseded`;
  - `ref` goes with `delivered-elsewhere`;
  - `reason` is written by `classify`, which has no acceptance to carry
    it, as 0047's `supersede` does.

  The build adds its lifecycle row in `docs/guides/versioning.md` and its
  pin in `crates/brokkr-runtime/tests/frozen_contracts.rs`. `VIEW_VERSION`
  moves once, to the next number free at merge: 29 before 0103's slice
  4f, 30 after it. No envelope, run-manifest, phase-machine, policy or
  `DATABASE_SCHEMA` change is made.
- **(b) Two schemas, one per command.** This mints a second version that
  no ruling names. Refused under 0082.

**Where ruling 6's and ruling 7's sentences land.**

| sentence | surface and file |
|---|---|
| ending in the status column; `--status` and JSON keep the fold's word | `status_word` (`crates/brokkr-view/src/fleet.rs:232-242`); `RunRow` (`fleet.rs:21-76`); no `--status` flag exists, so the JSON alone keeps it |
| tone and gutter | `crates/brokkr-cli/src/render.rs:154-170`, `:262-293`; TUI `tui/style.rs:57-63`, `tui/panes.rs:346-391` |
| header counts | a new single-journal head and the many-hearth head at `render.rs:326-356` |
| detail line, closing tally | `under_row` (`render.rs:299-318`); a tally line at the end of the listing (`crates/brokkr-cli/src/fleet/listing.rs`) |
| `--ending`, `--needs-operator`, `--hide-settled` | `RunsArgs` (`cli_args.rs:522-540`), read as a filter over the page |
| `inspect` and `watch` show both words | `render/inspect.rs:22-37` |
| TUI and console chips, the console's groups | `tui/panes.rs`; `ui.html:244-245` gains the six tone classes |
| Muninn queues by the ending | `queued_run` (`muninn.rs:645-658`) refuses a work-queue entry citing a `superseded`, `withdrawn` or `delivered-elsewhere` run; no queueing Muninn exists beyond that |
| JSON `ending`, `ending_cited`, `wants` | `RunRow`, `Summary` (`crates/brokkr-view/src/run_summary.rs`), `/api/runs`, `/api/view` |
| ledger counts and exclusions (ruling 7) | `analytics/figures.rs:28-38`, `analytics/recipes.rs:79`, `graph/figures.rs:421-423`, `compare.rs:250`; issue #271's corpus has no code yet |

### Rulings

The operator accepted all seven on 2026-11-03, each as recommended, and
amended ruling 3 on 2026-11-04.

1. **The derivation is choice 1 (a).** It is an ordered table over
   (`from`, `result`), with `ship`'s dirty stops as `blocked`, no
   `exhausted` on a policy stop and an unknown pair `unclassified`. The
   sweep runs every composed `recipes/*` bundle and reads
   `policy/phase-machine.json` without editing it. Its expected
   `unclassified` set is empty.
2. **0102 reads with this as choice 2 (a).**
   - `status_word` paints the ending for a terminal run.
   - A stopping run has no ending and wants an action.
   - 0102's detail text does not change.
3. **A concluded park takes the cause the operator names, as choice 3
   (c).** Amended by the operator on 2026-11-04 ("go with recommended"),
   from choice 3 (a) as accepted on 2026-11-03. `exhausted` is named as
   the other five causes are, and nothing refuses it.
   - Four reviews of E2 (#317) found that the fold cannot prove which
     park a stop closed. A park's reason is untyped text
     (`ParkReason::Stated`), and the fold's last decision survives a
     retry. So the guard could only guess, and ruling 5 forbids an
     inferred ending.
   - Choice 3 (c)'s cost stands: an outage named `exhausted` counts as
     one, and the operator's word is the record.
   - A typed record of what opened each park would bring the guard
     back. That is a decision of its own.
4. **`--cause` is required at once, as choice 4 (a).** Every in-repo
   caller moves in the same slice, and `conclude` always takes a cause.
5. **The ending lives in `run_summary`, as choice 5 (a).** There is one
   function in `brokkr-core`, and `DATABASE_SCHEMA` does not move.
6. **The header counts as ruling 6 wrote it, as choice 6 (a).** Reading
   (b) is left to a decision of its own.
7. **The build mints `operator-ending.v1` and moves `VIEW_VERSION` once,
   as choice 7 (a).**

### The slices

Each slice is one pull request. The order it needs is stated after the
list.

1. **E1, the vocabulary and the derivation.** Files: a new
   `crates/brokkr-core/src/ending.rs`, `crates/brokkr-core/src/lib.rs`,
   and a sweep test beside `every_shipped_bundle_addresses_its_sites_unambiguously`
   (`crates/brokkr-runtime/src/bundle/tests.rs:1831`), in a test file of
   its own. About 200 production and 250 test lines.
   - It adds the closed `Ending` enum, its ruling 2 tone class, gutter
     mark and `wants`, the table of ruling 1 above, and the event reading
     of choice 5.
   - Contract: none. View: none.
   - Tests:
     - the sweep;
     - each table row by exact variant;
     - the totality test that tone and gutter are functions of the ending.
2. **E2, `operator classify` and the schema.** Files:
   - a new `engine/operator/classify.rs`, shaped like `supersede.rs`
     (155 lines);
   - the CLI verb in `cli_args.rs` (714 lines, +15) and
     `verbs/delivery.rs`;
   - the new `contracts/operator-ending.v1.schema.json` and
     `contracts/README.md`;
   - its row in `docs/guides/versioning.md` and its pin in
     `frozen_contracts.rs`.

   About 150 production and 200 test lines. Nothing existing breaks,
   since the verb is new.
   - Contract: `operator-ending.v1`. View: none.
   - Tests:
     - ruling 1's fold-identity test in
       `crates/brokkr-core/src/fold/tests.rs`, which folds a finished run
       with and without a `classify`;
     - a classify on a live run refused;
     - a second classify standing over the first;
     - the schema checked against what the verb writes.
3. **E3, the required cause on `stop` and `conclude`.** Files:
   - `cli_args.rs` (+20) and `verbs/delivery.rs`;
   - `engine/operator.rs` (717 lines, so the cause goes to a new
     `engine/operator/ending.rs`), and `conclude`'s path that writes
     E2's `classify` when a stop in force names another cause or none;
   - every caller listed under the inventory's broken-caller heading:
     the dsh console and its tests, `quickstart.md`, the regenerated
     `docs/reference/cli.md`, `need.rs`, `run_view.rs`, `acknowledge.rs`,
     their goldens and the three TUI snapshots, and the CLI and
     in-process tests.

   About 200 production lines, 300 test lines and the caller edits.
   - Contract: none new. View: none.
   - Tests:
     - the fold-identity test extended with a cause in a stop's `args`;
     - a refusal test per missing or misplaced flag, with its text pinned
       once;
     - `conclude` on a stop in force, once with the stop's cause and
       once with another;
     - `witness_journal.rs` unmoved.
4. **E4, the projection and the wire.** Files:
   - `crates/brokkr-store/src/summary/aggregate.rs` and a new
     `summary/ending.rs` with the columns, stamp and trigger;
   - `crates/brokkr-core/src/summary.rs`, whose `Account` carries the
     reading;
   - the aggregate's old `Ending`, removed;
   - `crates/brokkr-view/src/fleet.rs` (701 lines) and `fleet/page.rs`;
   - `run_summary.rs`;
   - `lib.rs` for `VIEW_VERSION` only, since the file is over 800 lines and
     may not grow;
   - the analytics and graph readers, moved mechanically.

   About 300 production and 300 test lines.
   - View: one bump. `DATABASE_SCHEMA`: unmoved.
   - Tests:
     - a paged row equal to a whole read for every ending;
     - an old-writer row sent behind and refolded;
     - `doctor`'s equality.
   - Shares `fleet.rs` and `VIEW_VERSION` with 0103's slice 4f (#430).
     Whichever merges second rebases and takes the next number.
5. **E5, the painting.** Files: `render.rs`, `render/inspect.rs`,
   `fleet/listing.rs`, `cli_args.rs` (the three filters), `tui/style.rs`,
   `tui/panes.rs`, `ui.html`, and the doc samples whose `brokkr runs`
   output gains the header. About 300 production and 350 test lines.
   - Tests:
     - ruling 6's golden listings of every ending, with colour and under
       `NO_COLOR`;
     - the filters;
     - the console allowlist test;
     - every string through `Safe`.
6. **E6, the ledger.** Files: `analytics/figures.rs`,
   `analytics/recipes.rs`, `graph/figures.rs`, `compare.rs` and
   `muninn.rs`. About 150 production and 200 test lines.
   - Tests:
     - exclusion counts printed with exact values;
     - a `refused` run read both ways;
     - a Muninn entry citing a settled run refused.
   - This slice moves `Built:` to `built`.

E2 needs E1. E3 needs E2, whose schema and `classify` it writes. E4
needs E3, because the projection reads both the cause and `classify`. E5
and E6 need E4 and may land in either order. Only E3 breaks callers, and
it moves all of them.

### What was not measured

- The backfill's time at 1.1 GB.
- How many refused runs the operator's journals hold today.
- Whether any journal still ruled by `policy/phase-machine.json` exists
  outside the tests.
- How often a park closes as each cause.

The house journal was not opened for this addendum.
