# 0118 — A CI failure GitHub proves was not the change's is re-run once, under Brokkr's own credential

Status: accepted (operator, 2026-11-23, "accept 0118 with the alignment in mind")
Date: 2026-11-22, revised 2026-11-23 (finished by hand; a green re-run is a pass, on 0117's
amended tested-commit rule)

Built: unbuilt (#1093)

## Context

Issue #1093 asks Brokkr to re-run a pull request's CI once, rather than park, when the runner
failed and the change did not. Decision 0117, the CI observer, is on the branch
`story-1093-ci-rerun-design`. It was proposed as 0111 and renumbered 0117, because main's 0111
is #1136's decision. The operator accepted it on 2026-11-23 (#1093). "0117" below means that
branch's text, cited by its rules and never by line. 0117
rules what CI says on a head and when a run may proceed. It parks on every true row of the park
table in its ruling 3, and its ruling 1 says Brokkr makes no forge write. Its slice 0117a holds
real payloads from this repository's CI, under crates/brokkr-core/src/ci/evidence/ on that
branch, with a README that lists what they settle.

On 2026-11-22 the operator replaced 0117 ruling 1 (a)'s "a person re-runs" with one forge write:
re-run a run once, when the failure is proven not to be the code's, under a credential of
Brokkr's own (#1093, issuecomment-7076284656). The controller ruled D1 to D9 "recommended
everywhere". Round one's review held, and the operator's "go with all recommended" took the
controller's rulings V1 to V7 (#1093, issuecomment-7080810656). Round two's review held too. On
2026-11-23 the operator's "go" had the controller finish this text by hand, with no further
reviewed round: the three HIGHs are designed out below, and the MEDIUMs are named acceptance
tests of their slices. This decision was proposed as 0116 and renumbered 0118 (0116 is #1273's).

This decision does not restate 0117, and says where it changes it. It lifts 0117 ruling 1 for
this one write, and narrows `other-run-not-success`, a row of 0117 ruling 3's park table, for the
cancelled-twin class alone (ruling 7). It carries no header pointer to 0117, which is on another
branch; the controller restacks 0117, and that is when the pointer pair belongs.

**What 0118 adds (V3, as the operator amended it on 2026-11-23).** 0118 adds the re-run, and **a
green re-run is a pass.** The request itself needs no green observation and grants nothing by
itself. The re-run's result goes back to 0117 and is judged fresh, and a green result is a pass
there, as any green run is. So a proven flake, or a cancelled twin, whose re-run comes back green
proceeds, and no person is asked. This rests on 0117's tested-commit rule as the operator amended
it the same day ("a re-run should count as a pass if the failure is proven flake"):
- a green `pull_request` run of the bound workflow whose `head_sha` equals the gated head proves
  the change's own revision;
- main's merge queue (ruleset 24071236, #451) proves the merge, because it re-runs CI on the
  merge group before anything lands.
So `revision-unproven` is true only when the current execution's `head_sha` is unread, its run is
not of the bound key, or the base branch's rules hold no `merge_queue` rule (or that read is
unread or incomplete). A `head_sha` that reads and is not the gated head is `revision-disagrees`.

Today the controller runs an operator-side watcher that re-runs cancelled twins under the
operator's own login, so the audit log names the operator for a write no person made. The
watcher stays until slice 0118a has landed and requests the re-runs the watcher makes today.

## The rule

After 0117 judges a `ci` observation not green, the engine draws a **re-run plan** from that
observation before it parks. The plan is a pure core function (0071 ruling 1). It reads the
sealed observation, the pinned check list, the sealed flake list, the queue and run facts the
fix-round check reads (ruling 3), passed in, and the run's own journal. It returns either one
request or one typed refusal. On a request, the engine reserves the workflow run and journals
the request (ruling 4), sends it (ruling 5), and enters `ci` again through one named transition,
`CI-RERUN-SENT`, which 0117's phase machine gains under this decision, from the request's
`effect/succeeded` only. The observer waits for the
target's next attempt, and 0117 judges it fresh. On a refusal, the run parks exactly as 0117
parks it, on the same rows, the park's `detail` adding the refusal's word; a refusal never
weakens a 0117 row and never grants `green`.

**Every row of 0117's park table falls in exactly one closed set.**

| Set | Rows | What the plan does |
|---|---|---|
| answered | `not-success`, for an entry of the current execution; `other-run-not-success`, for another run | a class must answer every true one |
| pass-only | `revision-unproven` | nothing: the row says the change's revision is unproven, which blocks a pass but not a re-run |
| refused | every other row: the launch, binding, `-incomplete`, currentness and association rows, and `carried-job`, `snapshot-inconsistent` and `revision-disagrees` | any true one refuses `not-rerunnable` |

`revision-unproven` is the one pass-only row; adding a row to the set is a ruling. With no
answered row true, no plan is drawn. The answered rows must all name one workflow run, the
**target**; two refuse `two-targets`. The plan reads its refusals in this order, the first that
holds being the refusal: `not-rerunnable`, `two-targets`, the scope's (ruling 2), `no-class` (an
answered row neither class answers), the class's (rulings 1 and 3), the guards' (ruling 4).

**A full re-run (V4).** The request re-runs every job of the target, never its failed jobs
alone: a failed-jobs re-run carries the passed jobs, and 0117's `carried-job` would park the next
observation (0117 ruling 3, carried jobs), while a full re-run re-creates every job, as 0117a
measured in older-run-full-rerun-complete. It costs one run's CI time, every job's.

## 1. The classes

The list is closed. It is an enum, matched without a wildcard (0071 ruling 2).

| Class | Proven by | Answers | Built by |
|---|---|---|---|
| `cancelled-twin` | forge facts alone | `other-run-not-success` naming the target | 0118a |
| `listed-flake` | the base branch's flake list, each failed test's own failure in its job's log, and a bound fix round | the current execution's `not-success` | 0118b |

**`runner-loss` has no predicate until it is measured (V7).** 0117a holds no runner loss and read
neither annotations nor logs, so the conclusion and annotation GitHub writes for a lost runner,
and how a runner-owned step is told from one the workflow names, are **unmeasured**. Until a
capture settles them (0118c, by 0117a's method, the check run's annotations added),
`runner-loss` is no variant of v1's enum and no refusal. A lost runner today answers to no class:
its job's log names no failed test, so its `not-success` refuses `unlisted-failure`.

The code under test writes its job log and its step names, so neither proves `cancelled-twin`.
`listed-flake` reads the log by the operator's ruling, and ruling 6 bounds what a forged log buys.

**`cancelled-twin`.** The target is a run at attempt 1 whose `conclusion` is `cancelled`. Beside
it stands a sibling of the same `path`, `event` and `head_sha`, with a higher `run_number`: the
current execution, its run `completed`, so a still-running sibling is 0117's
`execution-unsettled` and no request. The plan reads the target's `conclusion`, `path`, `event`,
`head_sha`, `run_number`, `run_attempt`, `head_repository` and `repository`. Measured in
cancelled-twin/runs.page1.json: run 3560 is `cancelled` and its sibling 3561 is `success`, both
`pull_request` runs of `.github/workflows/ci.yml` on 669303526, one second apart. The cause is
`.github/workflows/ci.yml:25-27`: a `pull_request` run's concurrency group is its ref, and it
cancels in progress, so a `labeled` event on an unchanged head cancels the run before it.

The class reads the run's conclusion, not its jobs. In
cancelled-twin/run.44913617230.jobs.all.page1.json, 30 of the cancelled run's 33 jobs are
`cancelled`, but three aggregate jobs (`exact coverage gate`, `test (ubuntu-latest)` and
`test (macos-latest)`) concluded `failure` after their steps ran. No annotation is used.

**`listed-flake`, matched per test (V1).** The target is the current execution. The plan reads the
target attempt's **complete failure inventory**: every job of that attempt (`filter=all`, every
page) that did not conclude `success` or `skipped`, whether or not a required check names it,
and each such job's failed steps. Every entry of that inventory must be explained, or the plan
refuses `unlisted-failure`. A required aggregate job is explained only by the shards it `needs`
(the shard rule, below). Every other failed job must pass three checks:

1. Its log names a **failed-test set** read whole: the log read to its end, and the harness's own
   count of failed tests equal to the set's size.
2. Every test in the set is a listed entry's `test`.
3. That entry's `signature` is on a line of that test's own failure output, the segment the
   harness prints under the test's name. A match elsewhere in the log is no match.

So a listed flake beside an unlisted failure in one job refuses, and so does a job whose log
names no failed test: an aggregate job, a build or lint failure, a lost runner. Each matched
entry must then have a bound fix round (ruling 3), or the plan refuses `flake-unfixed`. The log's
shape, how a test's segment is bounded and where the count is printed are **unmeasured** until
0118b's capture; a segment it cannot bound refuses `unlisted-failure`, failing closed.

**A failed step outside the tests.** A failed job whose failed steps include any step other than
the one that runs its tests (a build, a lint or a setup step) is unexplained, whatever its log
says.

**The shard rule.** ci.yml's test jobs run as shards, and their required aggregates (`test
(ubuntu-latest)`, `test (macos-latest)`, `exact coverage gate`) fail when a shard fails. Their
own logs name no test. An aggregate is explained when every shard it `needs` that did not succeed
is explained. The needs graph is read from ci.yml at the base revision and sealed with the
observation.

**The log reader (0118b).** 0117 reads no job log (its ruling 3), and 0118b adds one read. For
each failed job the inventory holds, it makes one GET of that job's log through the read
credential, capped at 8 MiB; a log past the cap refuses `unlisted-failure`. The log's digest and
the failed-test set read from it are sealed into the observation at a new observation version,
so a replay reads the same set and never the forge.

## 2. Scope

The plan refuses outside this scope, each case by its own word. Every fact comes from the target
run as sealed.

| Refusal | True when |
|---|---|
| `fork` | the run's `head_repository.id` differs from its `repository.id`, or either is absent or `unread` |
| `wrong-workflow` | the run's `path` is not `.github/workflows/ci.yml` |
| `wrong-event` | the run's `event` is not `pull_request`; `pull_request_target`, `workflow_run`, `workflow_dispatch`, `merge_group`, `push` and release among them |

A workflow that holds secrets is out by both words: release runs on `push`, and
`pull_request_target` and `workflow_run` are refused by `event`. A fork's pull request is
**unmeasured** in 0117a, so `fork` fails closed on any field the observation lacks.

## 3. The flake list

**Its home (D2, recommended).** It is one tracked file, quality/ci-flakes.json, under the schema
word `brokkr.ci-flakes.v1`, parsed with `deny_unknown_fields`. The observer reads it from the
forge at the base revision the observation names, through the contents endpoint with its read
credential. It seals the file's bytes and digest into the observation, so a replay reads the same
list. The file in the pull request's tree is never read. A list that is absent at the base reads
as empty. A list that does not parse refuses `flake-list-unreadable`.

**An entry.**

| Field | What it holds |
|---|---|
| `test` | the test's identity as the harness prints it, its test binary included, e.g. `it::doc_truth::every_decision_called_proposed_is_proposed` under `brokkr-cli` |
| `signature` | a literal string, matched as a substring of one line of that test's own failure output, never a pattern |
| `issue` | the tracking issue's number |
| `fix` | `{"queue": <entry id>}`: the fix round's entry in 0110's queue |

**A bound fix round, checked at read, never trusted (V2).** The plan follows `fix` through 0110's
queue to the entry, and from a claimed entry to its run. The round is live only when the entry's
commission, and for a claimed entry also its run's own commission as that run's `run/started`
journaled it, is the template's (below), naming exactly this entry's `test`, `signature` and
`issue`, and the entry is `Queued` or `Launching`, or `Claimed { run }` with that run not ended
in the journal. 0110's queue keeps a started entry `Claimed { run }` after its run ends, so a
claim alone is never read as live. An entry that is unknown, `Held`, `Dropped`, claimed by an
ended run, or commissioned for
another test refuses `flake-unfixed`, and the flake is not re-run. The park's detail reads, in
fixed words: `flake <test> is listed without a live fix round (<fix>); it is not re-run`.

**Listing commissions the fix, tied to the base branch's listing (V2).** A new verb,
`brokkr queue flake <test> --signature <text> --issue <n>`, enqueues one commission from a fixed
template, a constant in its handler pinned by a test, naming the test, the signature, the issue
and the fix's acceptance: remove this entry from the list. It enqueues it `Held`, under the hold
`awaiting-listing`, so no dispatcher pass can claim it. It then writes the list entry, its `fix`
naming that queue entry, into the working tree's list, and the listing lands as any change does.

The trigger is idempotent: a second call with the same three values returns the same queue entry
and writes the same listing, and a call for a test whose round names other values refuses
`flake-already-commissioned`. The listing condition is enforced at **claim**, on every release
path. 0110's claim refuses an entry held `awaiting-listing`, or released from it, until the base
branch's list, read as above, holds an entry whose `fix` names it with the same three values.
The dispatcher's release and an operator's `QueueCommand::Release`
(`crates/brokkr-store/src/queue.rs:353`), which sets `Held` to `Queued` directly, both reach that
check, so the fix cannot be claimed before its listing lands. An abandoned or failed listing
leaves only a held entry, which no pass starts; `brokkr queue` names it `awaiting-listing`, and a
person drops it. Between the landing and the release the flake refuses `flake-unfixed`.

**How an entry leaves.** The fix's merge removes it, as its commission requires. A round that
ends unmerged leaves the entry `flake-unfixed` until it is listed again. A `brokkr queue flake`
call for a test whose round is `Dropped`, or claimed by an ended run, replaces that round with a
new queue entry and rewrites the list's `fix`. A call while the round is live stays idempotent.

## 4. Once per workflow run, reserved and journaled first

**The event (D3, recommended).** The request is an engine effect. Its `effect/requested` carries
a `ci-rerun/v1` extension at `event_schema: 1`, which the fold never reads, as `park-cause/v1`
rides `run/parked` (decision 0106). It binds the repository id, the target's run id and the
attempt observed, the gated head, the class, the observation's digest and the rows and job ids
the class answered; for `cancelled-twin`, the sibling's run id, which ruling 7 reads; and for a
flake, each matched entry's `test` and `fix`, and the flake list's digest.

**The reservation (V5).** The serialization key is the workflow run: the repository id and the
target's run id. The reservation lives in the journal's store, in a table beside 0110's queue
tables with two unique keys, the workflow run and the Brokkr run. The engine inserts it and
appends `effect/requested` in one transaction, synced before the call. A second reservation of
either key fails that transaction, and the plan refuses `second-request`, naming the holder. So
two Brokkr runs on one journal cannot both send for one workflow run, and one Brokkr run sends at
most once, whatever the first outcome, so a second failure parks for a person. The store is
recommended over the coordination root: the queue's claim already fences in that store's
transaction, and the coordination root holds no journal (0105). Two journals on one host share
the coordination root, so the engine also takes a host-wide lock there, keyed by the workflow
run, for the reservation's transaction, and a second holder refuses `second-request`.

**The forge's guard.** A target whose observed `run_attempt` is not 1 refuses `already-re-run`.
**Before the call** the engine reads the pull request's `head.sha` and the target's `run_attempt`
again: a moved head refuses `head-moved`, a moved attempt `already-re-run`. Nothing is sent, the
refusal is journaled as the effect's failure, and the reservation stands.

**What remains.** GitHub's re-run takes no attempt precondition (0117 ruling 1 (b)). A person's
concurrent re-run between the pre-call read and the call is outside both guards, and so is a
Brokkr on another host: two re-runs of one run, and no wrong `green`, since 0117 judges what
the forge shows.

**Its endings, never retried in a loop.**

| Ending | Recorded as | Then |
|---|---|---|
| the forge answers 201 | `effect/succeeded` with the status | `ci` again; the observer waits for the target's attempt to pass the one observed, inside 0117's polling bound |
| the forge answers any other status | `effect/failed`, `rerun-refused`, with the status and the forge's message, masked | parks |
| no answer (timeout, transport error, the engine killed mid-call) | `effect/failed` `rerun-unconfirmed`, or, on resume, a request with no ending | parks; a resume never sends it again, since the reservation stands |
| the next attempt never appears inside the bound | the second observation's own rows, and `rerun-unseen` in the detail | parks |

Every park here is 0106's `ruled` cause, its rule `CI-PARK`, with the refusal's word in `detail`.

## 5. The credential

**What it is (D4, recommended).** A GitHub App, created by the operator and installed on
`feedback-loop-ai/brokkr` alone, with repository permissions **Actions: read and write** and
**Metadata: read** (which GitHub grants every App), no other permission and no webhook. Its
installation tokens are short-lived: the engine mints each one for the single call, asks for
`actions: write` on this repository alone, and drops it after. The audit log names the App's bot
as the actor, never the operator.

**Held by name (decision 0012).** Three names sit in the operator's store:
`BROKKR_CI_RERUN_APP_ID`, `BROKKR_CI_RERUN_INSTALLATION_ID` and `BROKKR_CI_RERUN_PRIVATE_KEY`.
The key and each minted token are 0012's `Secret`. The token appears in the engine's memory, from
mint to the call's end, and in the `Authorization` header of one request over TLS to
`api.github.com`, and nowhere else; the App's signed JWT only in the mint request's header.
Neither is ever journaled, on argv, in a child's environment, in a seat, its bundle or its input,
in a log, or in an error's text: an error carries the status and the forge's message, through
0012's masker with both bound.

**The typed call (0071 rulings 2 and 8).** One closed enum, `ForgeWrite::RerunRun { repo, run_id }`,
sends `POST /repos/{owner}/{repo}/actions/runs/{run_id}/rerun`, which re-runs every job (V4).
Its errors are a `thiserror` enum. Tests reach it through a `*_with` closure seam, never a trait.
It lives in a runtime crate, never in core.

**0012's layer 4 gains one named site, if the operator accepts it.** 0012 lets a `Secret`'s
plaintext out at exactly one call site, the spawn injector. 0118 adds the forge-write client as
the second, and the only other. It is engine-side because putting the token on a child's argv or
environment is what D4 forbids. Signing the JWT (RS256) needs a crate the workspace does not
hold, and 0118a names it and justifies it.

**What the operator does once.** Create the App under the organization, with the permissions
above, the webhook off, installable only on this account; generate a private key; install it on
this repository only; store the three values with `brokkr system secrets set NAME`, read from
stdin (whether the env-format store holds a PEM's line breaks is 0118a's to measure, and if not,
the value is the PEM's base64); and revoke the watcher's use of their login once 0118a lands.

## 6. The threat model

| Case | Answer |
|---|---|
| A pull request fakes a listed flake's signature | The log is the code's: the code under test prints the test names, the failure segments and the count the rule reads, so a pull request can make any failure read as a listed flake's. Against such code CI was never the defense. Code that can forge its log can as well make its tests pass, and review stands against it, not CI. What the class bounds is honest, buggy code: an unlisted failure, a failed step outside the tests, or a failed test outside the list refuses. A listed test buys one full re-run, judged fresh, and a deterministic failure fails again and parks on `second-request`. The list is read from the base, so a pull request cannot list its own failure. |
| An intermittent real regression in a listed test | One re-run may pass it, so the regression can land. This is the residual the operator accepted by listing the flake. It is bounded: the entry names one test and one literal signature, and its fix round is bound to that test and live, so a person is already working on it. A second failure parks. |
| A re-run racing a person's | Ruling 4's "What remains": two re-runs of one run, and no wrong `green`. |
| A head that moves between read and request | The pre-call read refuses `head-moved`. A head that moves after the call leaves a re-run on the old head, which is harmless. 0117's P1 rows (`unrelated`, `head-unpublished`) judge the new head, so the old head's re-run grants nothing. |
| A token leak | Exposure lasts at most a token's lifetime, on this repository's Actions only. The holder could re-run, cancel or dispatch workflows and delete runs, logs and artifacts. It cannot read the code or the secrets, or write the code, its contents or a pull request. Dispatching a secret-holding scheduled workflow, renovate's for one, runs that workflow with its own secrets and shows the holder none of them. A leaked private key is the larger loss: the operator revokes it in the App's settings, and the App, installed on one repository, bounds what it reached. |

Every class buys one fresh observation. A green re-run passes, as any green run does, and no
class or forged fact grants `green` by itself: the re-run must come back green on the gated head.

## 7. The twin's sibling: an amendment of 0117's `other-run-not-success`

**The defect.** Under 0117 ruling 3's currentness rule, `other-run-not-success` reads every
other run's checks, its after-ship checks among them (0117 ruling 6), and before ship
`delivered by brokkr` fails on every run without the by-hand label. Re-running a cancelled twin
R0 makes R0/2 the current execution and its sibling R1 "the other run", whose red
`delivered by brokkr` keeps the park standing, so no re-run of a twin clears it before ship.

**The ruling (recommended), for the cancelled-twin class alone (V7).** The narrowed reading
applies to one run only: the sibling named by a `cancelled-twin` `ci-rerun/v1` request in this
Brokkr run's journal **that was sent and succeeded**. Its effect must have ended `effect/succeeded`,
the forge having answered 201. A request that is only `effect/requested`, was refused before the
call, or is unconfirmed moves nothing. And the current execution must be an attempt of that
record's target whose `run_started_at` is after the success was recorded, so a person's earlier
re-run of the target is read as 0117 reads it. 0117's judge reads
that one pair from the journal, beside the observation and the pinned list, and nothing else of
the journal; a replay reads the same record. That sibling's own run-level state is read first: a
latest attempt queued, in progress or waiting, or completed with any conclusion but `success` or
`failure` (`cancelled` among them), keeps the row true. A latest attempt completed `success` or
`failure` is then read through the entry set, exactly as the current execution is, so its
after-ship checks are not read before ship, and a red entry-set check still makes the row true.

Every other run is read as 0117 reads it: a twin a person re-ran, which no record names, and a
run of another event, a `workflow_dispatch` among them. No other row moves. The amendment lands
in 0118a, with the class it serves.

## 8. The test plan

Every test runs through recorded payloads, 0117a's by case directory under
crates/brokkr-core/src/ci/evidence/ on 0117's branch and the new captures below; a **derived**
fixture is one with named fields changed by the shared fixture builder. Each test asserts the
exact refusal variant or request, each refusal's text pinned once (0071 rulings 8 and 9).

**Each test names its baseline (V6).** The recorded cases park on rows the plan refuses
(cancelled-twin's #1222 is closed, so its `pulls-disagree` is true), so positives are derived:

- **B0** is cancelled-twin with derived association facts (the open-pull-request list holds the
  bound pull request, and each current execution's `pull_requests` names it with the gated head
  and base), its runs' `head_sha` the gated head, and the base branch's rules holding the
  `merge_queue` rule as 0117a measured them, so under 0117's amended rule `revision-unproven` and
  `revision-disagrees` are false. It also drops the mutants runs the recording holds (1841
  cancelled, 1842 failed), so its one true row is `other-run-not-success`, naming the target (run
  id 44913617230, number 3560). Each refusal test derives from a reachable baseline whose only
  change is the fact its guard reads, so it asserts one refusal row, not several.
- **F0** is the flake capture, derived as B0 is: its one failed job's set is one listed test,
  whose queue entry a live run claims under the template's commission for it.

Each positive test asserts its baseline's true rows first. Each refusal test changes the one fact
its predicate reads, so a compiling mutation that makes the predicate false turns the refusal
into a request and fails the test; each slice records each mutation with the test it failed.

| Test | Proves | Baseline, and what changes |
|---|---|---|
| `a_cancelled_twin_on_its_own_branch_requests_one_full_rerun_of_the_cancelled_run` | `cancelled-twin`, `RerunRun` on 3560 | B0 |
| `a_listed_flake_with_a_bound_fix_round_requests_one_full_rerun` | `listed-flake` | F0 |
| `a_listed_flake_beside_an_unlisted_failure_in_one_job_parks_and_sends_nothing` | V1, a mixed failure | F0, the job's set gaining an unlisted test |
| `a_failure_no_listed_entry_names_parks_as_unlisted_failure` | `unlisted-failure` | F0, the list empty |
| `a_job_whose_log_names_no_failed_test_parks_as_unlisted_failure` | a job that is not an entry's | F0, the failed job an aggregate one whose log names no test |
| `a_signature_outside_its_tests_own_failure_parks_as_unlisted_failure` | the signature per test | F0, the signature moved to another test's segment |
| `a_set_whose_count_disagrees_parks_as_unlisted_failure` | the set read whole | F0, the harness's count one more than the set |
| `a_listed_flake_whose_linked_run_ended_refuses_as_flake_unfixed` | V2, an ended linked run; its fixed words | F0, the claiming run ended |
| `a_listed_flake_linked_to_an_unrelated_live_entry_refuses_as_flake_unfixed` | V2, an unrelated live entry | F0, `fix` naming a live entry commissioned for another test |
| `a_fix_round_cannot_start_before_its_listing_lands_on_the_base_branch` | V2, the trigger | a temporary journal: the verb enqueues `Held`; a dispatcher pass over a base list without the entry claims nothing; over one with it, releases and claims |
| `listing_one_flake_twice_returns_one_fix_round` | the trigger is idempotent | the same journal; and other values for the test refuse `flake-already-commissioned` |
| `a_flake_list_only_in_the_pull_requests_tree_is_never_read` | the list read from base | F0, the base list empty |
| `an_unparseable_flake_list_refuses_as_flake_list_unreadable` | `flake-list-unreadable` | F0, the base list's bytes |
| `a_run_from_a_fork_refuses_as_fork_and_sends_nothing` | `fork` | B0, 3560's `head_repository.id`; and its `unread` |
| `a_cancelled_mutants_run_refuses_as_wrong_workflow` | `wrong-workflow` | B0, 3560's `path` that of run 1841 |
| `a_cancelled_dispatch_run_refuses_as_wrong_event` | `wrong-event` | B0, 3560's `event` `workflow_dispatch` |
| `a_park_with_a_refused_row_refuses_as_not_rerunnable` | `not-rerunnable` | B0, `pulls-disagree` made true alone |
| `two_cancelled_runs_on_one_head_refuse_as_two_targets` | `two-targets` | two-runs-both-rerun, both at attempt 1, its association derived as B0's |
| `a_run_a_person_already_re_ran_refuses_as_already_re_run` | `already-re-run` | B0, the pre-call read finding 3560 at attempt 2 |
| `a_second_failure_in_one_run_parks_as_second_request` | `second-request` | B0, the store holding this run's reservation |
| `two_callers_reserving_one_workflow_run_send_one_request` | V5 | B0, two plans racing on one store: one reservation, one seam call, the other `second-request` |
| `a_head_that_moved_before_the_call_refuses_as_head_moved_and_sends_nothing` | `head-moved` | B0, the pre-call read a new head |
| `the_rerun_request_is_reserved_journaled_and_synced_before_the_call` | ruling 4's order | B0, the seam asserting the event and reservation at call time |
| `a_forge_refusal_of_the_rerun_parks_as_rerun_refused_and_is_sent_once` | `rerun-refused` | B0, the seam answering 403 |
| `a_request_with_no_ending_parks_on_resume_and_is_never_sent_again` | `rerun-unconfirmed` | B0, the engine killed in the call |
| `the_minted_token_reaches_no_journal_argv_environment_or_seat` | ruling 5, 0012 layer 6's byte scan | B0 |
| `the_observation_after_a_full_rerun_parks_on_no_carried_job` | V4 | older-run-full-rerun-complete's attempt 2, derived as B0: no `carried-job`, and it passes |
| `a_green_rerun_of_a_listed_flake_passes` | the amended rule, a flake | F0, the target's re-run green on the gated head: no true row, and the run proceeds |
| `a_green_rerun_of_a_cancelled_twin_passes` | the amended rule, a twin | cancelled-twin-rerun, derived as B0, the journal holding B0's sent request: no true row, and the run proceeds |
| `a_rerun_whose_head_sha_is_not_the_gated_head_does_not_pass` | the amended rule's bound | F0's re-run, its `head_sha` another commit: `revision-disagrees`, and it parks |
| `a_green_rerun_on_a_base_with_no_merge_queue_rule_does_not_pass` | the merge-queue dependency | B0's green re-run, the base branch's rules holding no `merge_queue` rule: `revision-unproven`, and it parks |
| `a_rerun_twin_beside_a_sibling_whose_only_red_is_after_ship_is_not_other_run_not_success` | ruling 7 | cancelled-twin-rerun (new), derived as B0, the journal holding B0's sent request: no true row |
| `a_same_event_sibling_with_a_red_entry_check_still_parks` | ruling 7 | the same, one entry-set check of 3561 `failure` |
| `a_queued_in_progress_or_cancelled_sibling_keeps_other_run_not_success` | ruling 7, run-level state | the same, 3561's latest attempt in each state |
| `a_twin_with_no_rerun_record_is_read_as_0111_reads_it` | ruling 7's scope | the same, the journal holding no request |
| `a_recorded_but_unsent_request_exempts_no_sibling` | ruling 7, bound to a sent re-run | cancelled-twin-rerun, the journal holding B0's request refused before the call (`head-moved`), and again unconfirmed |
| `a_failed_job_no_required_check_names_parks_as_unlisted_failure` | V1, the complete inventory | F0, a failed job outside every required check's match |
| `a_failed_build_step_beside_a_listed_test_parks_as_unlisted_failure` | V1, failed steps | F0, the job's build step failed too |
| `a_listed_shard_failure_explains_its_required_aggregate` | the shard rule | F0, the failed test in a shard and its aggregate failed |
| `a_manual_release_cannot_claim_a_fix_before_its_listing_lands` | V2, every release path | the commission test's journal: `QueueCommand::Release`, then a claim |
| `a_live_linked_run_with_another_commission_refuses_as_flake_unfixed` | V2, the run's own commission | F0, the claiming run's `run/started` commission for another test |
| `a_job_log_past_its_bound_refuses_as_unlisted_failure` | the log reader | F0, the log one byte over the bound |
| `the_failed_test_set_replays_from_the_sealed_log` | the log reader's seal | F0, replayed with the forge unreachable |
| `relisting_after_a_dead_fix_commissions_a_new_round` | relisting | the commission test's journal, its first round ended unmerged |
| `two_journals_on_one_host_send_one_request` | the host-wide lock | B0, two journals on one coordination root |
| `an_other_run_failure_no_class_answers_refuses_as_no_class` | `no-class` | B0, the sibling `failure`, not `cancelled` |
| `a_dispatch_sibling_is_fully_read_and_the_entry_still_parks` | ruling 7 | dispatch-and-pr-both-green, derived as B0 (the dispatch's `delivered by brokkr` is `skipped`) |
| `a_runner_loss_on_a_runner_owned_step_requests_one_full_rerun` | `runner-loss`, 0118c | runner-loss (new) |

The new captures use 0117a's method. cancelled-twin-rerun is a twin pair after R0 is fully
re-run, taken on a head the watcher re-runs before it retires, with R1's only failure its
after-ship check. flake is a failed job's log with the test's failure segment. runner-loss is a
lost runner, with its check run's annotations.

## 9. The slices

1. **0118a: the request path, the cancelled twin, and ruling 7.** Needs 0117d, the gate live on
   `landing`. It captures cancelled-twin-rerun first, then builds the plan, the row sets, the
   scope refusals, the reservation, `ci-rerun/v1`, the typed call, the App credential and ruling
   7's amendment. Its tests: every row above whose baseline is B0, cancelled-twin,
   two-runs-both-rerun, older-run-full-rerun-complete, cancelled-twin-rerun or
   dispatch-and-pr-both-green. When it lands and requests the cancelled twins' re-runs, the
   controller's watcher retires.
2. **0118b: the flake list and its commission.** Needs 0118a and 0110's dispatcher. It captures
   flake first, then builds the list's contract, its base read and seal, the log reader, the
   complete failure inventory and the shard rule, the per-test match, the bound fix-round check,
   `brokkr queue flake`, the template and the claim-time listing check. Its tests: every row
   whose baseline is F0, and every test on the commission test's journal.
3. **0118c: runner loss.** Needs 0118a and a capture of a real runner loss; until that capture
   exists it is not commissioned. Its test:
   `a_runner_loss_on_a_runner_owned_step_requests_one_full_rerun`.

## Evidence

- 0117 on `story-1093-ci-rerun-design`, as the operator accepted it (#1093): ruling 1, ruling
  3's park table, its currentness rule, carried jobs and tested commit as measured, and ruling
  6's entry set; 0117a's README and payloads there, named where used.
- `crates/brokkr-store/src/queue/entry.rs`: `EntryState`'s `Held`, and `Claimed { run }`, which
  stays after its run ends.
- `.github/workflows/ci.yml:25-27`, the concurrency that cancels a twin, and the workflows'
  triggers, which put release on `push` alone.
- Unmeasured, each named where used: runner loss's shape; a job log's shape, a test's failure
  segment and the harness's count; a fork's payloads; how soon a re-run's attempt appears;
  whether the secrets store holds a PEM.
