# U8d — Fold settled calls through confirmed append

Tasks 41.1 and 41.2; requirements CC2, CR3 and CR4, with MB3's and MB4's
failure causes. Branch `s2/U8d`, cut from main at d7907e93. This is the
unit's third run, `0065-slice-two-unit-u8d-see-the--f1457508`, a repair. The
first run (31eff75f) reported oversized, and the operator's ruling "A" of
2026-10-20 admitted `engine.rs` (`run_driver` only) and `artifacts.rs`
(visibility only). The second run (`0065-slice-two-unit-u8d-see-the--c8d0c134`)
built the fold and was held by its review council (REVIEW-SECURITY-HOLD);
its staged work is this run's starting point. The section "Run 3: the
review's findings" names each finding, its repair and its removal control.

## What was built

A single seat's driver report passes a settlement barrier before
`conclude_single` admits it. `run_driver` takes the journal's head before
the driver spawns (`Engine::begun`, `engine/broker/fold.rs`), for a site
sealed with at least one broker only, and hands the report, its ids, its
sealed spawn and that head to `Engine::folded`. `folded` first witnesses the
driver's cleanup on the lease, exactly as `run_driver` did before (#403's
`settlement::witnessed`), then folds the site's broker calls. The site's
brokers are the `SealedServers::Brokered` list in `spawn.serving`'s
isolation (U7d2). A site sealed with no broker takes no head and folds
nothing, and an attempt whose cleanup is not `Settled` passes unchanged: a
live broker's partial write is never read.

Route coverage, exactly: a driver run through `run_driver`, which ordinary
dispatch (`dispatch.rs`) and sequence steps (`sequence.rs`) call, reaches the
barrier once its driver returns a report, whatever that report's outcome,
failed, cancelled and timed-out reports included. Evidence is read only
where the engine holds the attempt's lease and the cleanup is
`Cleanup::Settled`. A spawn that failed, an error returned before the
report, a cleanup not proven settled, a panel member's run
(`engine/panel.rs`) and the completion of an engine restart do not reach it
from this call site; covering those routes is U8e's.

The fold first publishes the attempt's cited responses through U8c's
`artifacts::published`, inside `settlement::evidenced` (the lease's
evidence located from the coordination root's handle). A publication
refusal appends nothing and fails the attempt with U8c's cause. It then
reads the inventory (`artifacts::provisioned`), refuses a `*.ledger` no pin
names, and folds each sealed broker in turn: its plan pinned at the
broker's very locator and digest and serving its capability
(`artifacts::pinned`), its ledger read whole by `artifacts::record` under
`LEDGER_BYTES_MAX`, verified only by `brokkr_protocol::broker::ledger::recover`
and read only through `Lifecycle::calls`, `closed` and `cited`. Each broker's
rows are appended before the next broker's ledger is read. No second
ledger, inventory or plan reader and no protocol or store API was added.

Each accepted call of the verified prefix becomes exactly one settled
seat-record v6 checkpoint, the typed `capability_calls::BrokerCheckpoint`:
step `capability-call-settled` (a one-variant enum), the ledger's tool, the
plan's capability and dialect, call id `m-` plus the SHA-256 of the
canonical tuple {attempt, site, instance, server, call} from the sealed
plan (D7), and call state succeeded, failed, refused or interrupted
(`Settled`). An interrupted call is a verified Started with no Terminal and
carries no digest; `response_sha256` appears only where the verified
Terminal cites one. The rows never pass through `Checkpoints::offer`'s hold,
and harness MCP telemetry still yields no attributed call (`Calls::consume`,
unchanged).

`checkpoints::confirmed` appends one broker's rows under the existing
`SETTLING_PATIENCES` through `within_patiences`. Each row becomes the typed
`capability_calls::SettledEvent` {effect_id, attempt_id, checkpoint}; it is
written to JSON only at the store's edge (`SettledEvent::payload`, inside the
`Rows` payload closure, which runs only once the transaction holds the
lock). Before every try it reads what the journal committed since the
attempt began (`Store::load_after` from the head `begun` took, which
verifies the suffix chains onto that head), never the run whole, and
`capability_calls::unjournaled` decides what is pending. A derived row the
journal holds whole, once, under its call id is already committed and
skipped. A second derived row for one call id, a committed row under a
derived id that does not decode as a `SettledEvent` or is not that row
whole, and a second committed row for one, are each CR3's conflicting
duplicate, refused before anything is appended. The result is typed:
`Confirmed::{Journaled, Conflict, Refused(fence), Stranded(n)}`. A storage
failure that is not contention is handed up. No cursor is kept.

The session is judged apart from its calls. The fold's refusals, the first
one wins, are the private `fold::Unfolded`:

| Refusal | Text |
| --- | --- |
| `Missing` | broker ledger is missing for this attempt |
| `Unexpected` | broker ledger is not expected for this attempt |
| `Ungranted` | broker ledger claims an ungranted tool |
| `Invalid(Invalid)` | protocol's own text (lifecycle, owner, conflict, partial, malformed, gap) |
| `Failed(Failure)` | the latched cause, in its shared words |
| `Unpublished(Unpublished)` | U8c's text, e.g. broker evidence is not bound to this attempt |
| `Fence(SeatRecordError)` | the seat-record fence's refusal |

A verified Closed of `Failed(cause)` fails the harness's success with that
exact cause, whatever the calls, none included, and no checkpoint stands
for it. `checkpoints::carry_settled` carries the outcome on the existing
`AttemptReport::refused`, through the existing `refused_outcome`; a driver
that failed on its own keeps its error beside the cause. Stranded rows make
the attempt indeterminate with the shared `outlasted` text ("N checkpoint(s)
were not journaled: a peer still held the journal's write lock 3 patiences
after the seat stopped"), and once one broker's rows are stranded the later
brokers' rows are counted with them without another try. The ledgers stay
on disk. Every box, source, store, readiness, handoff and exec cause rides
`Failure::Refused(Refusal)` transparently, so the existing closed projection
covers each; a test enumerates all of them.

## Run 3: the review's findings

| Finding | Repair in this run | Removal control (observed failing, then restored) |
| --- | --- | --- |
| H1 HIGH: committed payloads indexed by call id, the earlier silently replaced | `unjournaled` now indexes only the engine's own derived rows, refusing a second one for an id before either is indexed over the other, and compares every committed row under a derived id, as it is read, to that row whole; a second committed row for one id is refused too. Nothing is collected last-wins or first-wins. | R1 (run 2's last-wins index restored): the failed-then-succeeded case passes as journaled. R2 (first-wins): the succeeded-then-failed case passes. R3 (second committed row allowed): the identical-twice case passes. R4 (state compared, not the whole payload): the differing-digest case passes. R5 (`deny_unknown_fields` dropped from `BrokerCheckpoint`): the case with a v6-valid `turn` passes. R6 (derived duplicate allowed): `two_derived_rows_of_one_call_are_a_conflict` gets `Ok([both rows])`. |
| M1 MEDIUM: bounded memory | Per operator ruling 2: each ledger is still read whole under its 64 MiB / 4,096-call cap through protocol's existing `recover(&[u8], …)`; brokers are read and appended one at a time; the journal side reads only `load_after` from the head taken before the driver spawned. No streaming recover, no protocol or store API. | R7 (`load_after` replaced by `load`): `the_journal_is_read_only_since_the_attempt_began` gets the conflicting duplicate. R8 (every broker's ledger read before any append): `each_broker_is_appended_before_the_next_ledger_is_read` gets "broker ledger is missing for this attempt". R9 (stranded short-circuit removed): `a_lock_stranding_one_broker_…` sees 6 tries, not 3. |
| M2 MEDIUM: the refusal cause | Not fixed, by operator ruling 3. A settled refused checkpoint carries `call_state: "refused"` and no cause: seat-record v6 is frozen (`additionalProperties: false`, no cause field), U8d adds no public journal field, and nothing is written to `result` or any other field. The typed `Denial` stays in the private ledger. This is an operator-ruled gap in CC2's refused-checkpoint scenario, owed to a later seat-record version (v7). The view still tells refused apart: the fold journals `"refused"` exactly (`a_clean_session_…`, three refused rows, one per denial), and brokkr-view's one derivation reads it as `CallState::Refused` (`every_recorded_call_state_reaches_its_checkpoint_row_exactly`, run here and passing). No second derivation was added. | None: no behaviour was added. |
| L1 LOW: `Value` past the edge | `settled` returns the typed `BrokerCheckpoint`; the fold, `Folded`, `confirmed` and `unjournaled` carry `BrokerCheckpoint`/`SettledEvent`, and JSON is written only in the store's `Rows` payload closure. A committed payload is decoded once at the journal's edge: a probe for its call id selects it, and the comparison is `SettledEvent == SettledEvent`. | R4 and R5 above bind the whole typed comparison. |
| L2 LOW: artifact durability checked only after the fold returned | `a_retained_response_is_published_before_the_checkpoint_citing_it` reads the artifact's bytes inside the append seam (`between`, run before every try) and asserts exactly `[Some(bytes)]`. | R10 (publication moved after the per-broker appends): that test gets `[None]` ("published when tried"); `an_unpublished_response_appends_no_call` also gets a journaled row. |
| L3 LOW: `.ledger` spelled again | Carried. `artifacts.rs` spells it inline inside `cited` (no shared constant exists to make visible), and ruling 1 limits that file to visibility, so `fold.rs` keeps `LEDGER` with a comment naming that reason. | None. |
| Route-comment correction | `fold.rs`'s module comment and this file now state the coverage paragraph above: every report `run_driver` returns reaches the fold, and the uncovered routes are named. | R11 (`run_driver` passes no head): `a_dispatched_driver_…` gets `EffectSucceeded` where `EffectFailed` is required. |

Two further arms the repair made reachable or left untested are now bound:
a broker its inventory does not pin (R12: refusal replaced by `continue`,
`a_broker_its_inventory_does_not_pin_is_unbound` gets success) and a head the
journal does not hold (R13: a non-contention failure reported as stranded,
`a_head_the_journal_does_not_hold_…` gets `Ok` with an indeterminate
report). Publication now precedes every ledger read, so a ledger or
inventory publication could not read would already have refused. The
fold's own reads of them keep failing closed, written as `?`, `ok_or` and
`and_then` so no dead arm is left: a missing run directory reaches
`provisioned`'s `None` (R14: `ledgers`' refusal of an unread inventory
replaced by an empty tally, the "gone" case passes; run before and again
after that rewrite) and a run path that is a file fails `evidenced` (R15: refusal
replaced by an empty tally, the "file" case passes), both in
`evidence_without_its_run_directory_is_unbound`. The disposition judgment
was removed again on this tree (R16: `ledger()` ignores a `Failed`
closure): `version_protocol_…`, `a_failed_zero_call_…` and
`a_dispatched_driver_…` fail. R16 and R11 are CR4's two independent
removals; a cleanup removal is a different property and stays U8e's.
R17 (fence refusal read as journaled) makes
`a_checkpoint_the_fence_refuses_…` succeed.

Each R mutation was a compiling edit, run with `cargo test -p brokkr-runtime
--lib broker::fold`, and restored from a scratch copy under `.forge/mut/`;
`cmp` confirmed the restored files byte-identical before the next one. R1–R11
and R15–R17 ran while the suite held 22 tests, before the two newest tests
and the `?`/`ok_or` rewrite, which touched none of their lines; R12–R14 ran
after. The final restored suite passed 24 of 24.

## Run 4: the second review's residual

Run 3's review (REVIEW-REFORGE, residual medium) confirmed H1, M1, L1 and L2
repaired and left two gate findings for this visit; the rest it carried.
Only `broker/fold/tests.rs` and its `quality/file-lines.txt` row moved: no
production file changed in this visit, and every assertion keeps its exact
value.

| Finding | Repair in this run | Removal control (observed failing, then restored) |
| --- | --- | --- |
| INFO: `quality/ratchet.sh clones` refused two new test clones (the stranded-outcome assertions at fold/tests.rs:643 and :950; the retained-response setup at :784 and :807) | `stranded(count)` builds the one indeterminate outcome both lock tests assert, its text exactly as before; `retaining()` builds the one-call site whose `ANSWER` is retained and cited, shared by the two retained-response tests. The ratchet now reports "duplication holds". | R18 (publication moved after the per-broker appends, as R10): `a_retained_response_…` gets `[None]` where `[Some(bytes)]` is required, and `an_unpublished_response_…` gets one journaled row. R19 (`settle` stops adding the next broker's rows to the stranded count): `a_lock_stranding_one_broker_…` gets "2 checkpoint(s) were not journaled…" where 3 is required. R20 (one extra stranded call reported): `a_lock_outlasting_…` fails, along with every other fold test, since each reads its outcome. |
| INFO: `.forge/artifacts/sha256` spelled twice in the tests | `published(site, sha256)` is the one test spelling; `artifact` reads through it, and the append-seam proof reads the path it returns inside `between`. | R18 above. |
| M2 MEDIUM: the refusal cause | Unchanged: the operator-ruled gap of run 3's table, owed to seat-record v7. | None. |
| L3 LOW and the inherited LOWs | Carried, as run 3 and the commission state. | None. |

R18–R20 were compiling edits to `broker/fold.rs`, run with `cargo test -p
brokkr-runtime --lib broker::fold` and restored from `.forge/mut/`; `git diff
--quiet HEAD` confirmed `fold.rs` unchanged after each. The restored suite
passed 24 of 24.

## Run 5: the third review's residual

"Run 3", "Run 4" and "Run 5" name this run's three implement visits. Run 4's
review (REVIEW-REFORGE, residual medium) confirmed the repairs above, found
no new fail-open path, and asked for no code change. Its medium is M2, which
operator ruling 3 sets as a gap in v6, and no implement visit can close it.
This visit changed only this file. No production or test file moved.

| Finding | Disposition | Removal control |
| --- | --- | --- |
| M2 MEDIUM: the refusal cause | Unchanged: the operator-ruled gap of run 3's table, owed to seat-record v7. It is not fixed and not waived. | None. |
| LOW: a model-supplied tool name outside v6's vocabulary fails the attempt at the fence | Carried as "Notes for review" says, already bound by `a_checkpoint_the_fence_refuses_…` (R17). The only honest v6 row names the tool the model asked for. Any substitute name would invent evidence, and leaving the row out would drop an accepted call (CC2). The broker could refuse such a name before it accepts the call, but that is brokkr-cli's or the protocol's, outside this row. | R17 (run 3). |
| LOW: a journal conflict is reported as `Invalid::Conflict`, whose text names the broker ledger | Carried. CR4 says "conflicts use CR3's conflicting-duplicate cause" (capability-response-retention spec, lines 288–290), and its commit-before-ack scenario requires a different outcome or digest for one call id to refuse "the conflicting-duplicate cause" (line 315). So one cause covers both sources. A runtime-only variant with other text would break that settled contract, so the diagnostic imprecision is left for the design to rule. Fold tests pin the text once (`CONFLICT`, fold/tests.rs:701). | R1–R6 (run 3). |
| L3 LOW and the inherited LOWs | Carried, as run 3 and the commission state. | None. |
| LOW: imperative directions in panel notes | About the review's inputs, not this change. No action. | None. |

## Scope, files and API

| File | Change |
| --- | --- |
| `crates/brokkr-runtime/src/engine.rs` | 3,523 lines before and after; two hunks, both inside `run_driver`. Line 1673: `let workdir = self.workdir();` becomes `let (workdir, begun) = (self.workdir(), self.begun(spawn)?);` (size-neutral, since the file is frozen at its baseline). Line 1746: `broker::settlement::witnessed(&mut self.held, &report.cleanup);` becomes `let report = self.folded([effect_id, attempt_id], spawn, begun, report)?;`. The two `RunStarted` appends (#697) and their surroundings are untouched. |
| `crates/brokkr-runtime/src/engine/broker.rs` (row) | 800 lines before and after: `mod fold;` added, the blank line before the `mod` declarations removed. |
| `crates/brokkr-runtime/src/engine/broker/fold.rs` (row's new private child) | New, 387 lines. |
| `crates/brokkr-runtime/src/engine/broker/settlement.rs` (existing child) | `published` split into `evidenced`, `pub(super)` for the fold, and `published` over it, which stays private. 406 lines. |
| `crates/brokkr-runtime/src/engine/capability_calls.rs` (row) | `Step`, `BrokerCheckpoint`, `SettledEvent`, `settled` and `unjournaled`. 463 lines. |
| `crates/brokkr-runtime/src/engine/checkpoints.rs` (row) | `confirmed`, `Confirmed` and `carry_settled`; `outlasted` shared with `Settled::unjournaled`. 665 lines. |
| `crates/brokkr-runtime/src/engine/artifacts.rs` (ruling A) | Visibility only: `LEDGER_BYTES_MAX`, `provisioned`, `pinned`, `beside`, `record` become `pub(super)`; rustfmt wrapped two signatures. 481 lines. |

`git diff` shows no change under `crates/brokkr-protocol`, `crates/brokkr-store`,
`contracts/`, `policy/`, `reference/`, `fixtures/` or `extensions/`. Every new
runtime item is private, `pub(super)` or `pub(in crate::engine)`.

## Owning tests

While the MCP compile fence stands, no public engine can provision a plan,
so the proofs are the in-crate child module
`crates/brokkr-runtime/src/engine/broker/fold/tests.rs` (1,081 lines), as for
U8a2b and U8c. `crates/brokkr-runtime/tests/capability_ledger.rs` is
unchanged, and no test was moved. Each ledger is written by the protocol's
own `Ledger` in memory and laid beside a plan provisioned by U8c's
`artifacts::tests::Provisioned`, on a leased engine whose coordination root
is a fresh canonical temporary HOME; a second plan, for two-broker proofs,
is written beside the first with server `cap-wiki`. Fixture support:
`admission/tests.rs` gains `lease` and `evidence_leased` (1,721 lines),
`admission/tests/sealing.rs`'s `leasing` reuses `lease` (599 lines), and
`artifacts/tests.rs` makes `Provisioned::owner` `pub(in crate::engine)`.

| Test | Exact assertions |
| --- | --- |
| `a_clean_session_settles_one_checkpoint_per_accepted_call_and_its_result_stands` | Six rows compared whole: succeeded, failed, then refused for an ungranted tool, an active call and a spent budget, one each; success stands. |
| `a_healthy_zero_call_session_settles_nothing_and_succeeds` | (succeeded, no rows) |
| `a_failed_zero_call_session_fails_a_successful_harness_with_its_exact_cause` | All seventeen `Refusal` variants and the seven served causes: `Failed { error: <cause> }`, no row; a failed harness keeps its own error beside the cause. |
| `version_protocol_and_capacity_endings_keep_their_causes_despite_complete_calls` | Exact version, protocol and limit texts with exactly 0, 1 and 4,096 rows; the 4,097th request is refused by `room()` and no 4,097th row exists. |
| `a_ledger_that_does_not_verify_keeps_its_verified_calls_with_its_exact_cause` | Partial tail, malformed record, gap, other owner, Terminal without Started, second Terminal, no closure: each exact cause and exact rows. |
| `a_missing_or_unexpected_ledger_fails_the_attempt_without_inventing_a_call` | Missing: cause, no row. Stray `plan-7.json.ledger`: cause, the plan's call still settled. |
| `a_ledger_claiming_an_ungranted_tool_answered_settles_nothing_from_it` | Its cause, only the call before it. |
| `a_lock_across_repeated_tries_commits_each_call_once_after_it_lets_go` | Three tries, two rows once each, success. |
| `a_lock_outlasting_the_settlement_strands_the_calls_and_keeps_their_evidence` | Three tries, indeterminate with the stranded text, no row, ledger bytes unchanged. |
| `a_commit_whose_acknowledgement_was_lost_is_found_and_never_appended_twice` | The second try finds the rows a lost commit left; a second fold appends nothing and re-runs nothing. |
| `every_journaled_payload_of_a_derived_call_is_compared_whole` | Five journaled states (failed+succeeded, succeeded+failed, succeeded twice, a foreign digest, a `turn`): conflicting duplicate, journal exactly as planted. |
| `two_derived_rows_of_one_call_are_a_conflict` | Two derived rows for one id, differing or identical: `Err(Invalid::Conflict)`. |
| `the_journal_is_read_only_since_the_attempt_began` | A conflicting row planted before the head is not read; rows exactly [planted, call 1, call 2]. |
| `a_retained_response_is_published_before_the_checkpoint_citing_it` | Artifact bytes exactly present at the only try; the row carries the digest. |
| `an_unpublished_response_appends_no_call` | "broker response digest mismatch", no row, no artifact. |
| `evidence_without_its_run_directory_is_unbound` | Run directory gone, or a file: "broker evidence is not bound to this attempt", no row. |
| `a_broker_its_inventory_does_not_pin_is_unbound` | Same text, no row. |
| `a_head_the_journal_does_not_hold_is_handed_up_and_appends_nothing` | `Err(EngineError::Store(StoreError::UnknownHead { seq: 1 }))`, no row. |
| `each_broker_is_appended_before_the_next_ledger_is_read` | The second ledger, laid down only during the first broker's try, is found; rows exactly [docs 1, docs 2, wiki 1]; the journal grew by 2 between the two tries. |
| `a_lock_stranding_one_broker_strands_the_next_without_trying_it` | Three tries, "3 checkpoint(s) were not journaled…", no row. |
| `a_checkpoint_the_fence_refuses_fails_the_attempt_after_the_calls_before_it` | "seat record at journal seq 3 violates contracts/seat-record.v6.schema.json at /", only the call before it. |
| `an_attempt_not_proven_settled_is_not_read` | Unresolved cleanup: `refused` stays `None`, no row. |
| `a_brokered_site_without_its_lease_is_unbound` | Unbound text, no row. |
| `a_dispatched_driver_is_settled_by_its_broker_calls_before_its_result_is_admitted` | Through `run_driver` and `conclude_single`: a zero-call Exec-failed ledger journals `EffectFailed` with the exact cause prefix and no attributed row; a one-call clean ledger journals `EffectSucceeded` with one attributed row beside the telemetry row. |

Run 2 also recorded twenty mutations (its M1–M20) on its tree. Those that
bind code this run kept unchanged (the ledger projection, the settled-cleanup
gate, the lease gate, the patiences) were not re-run here; R11 and R16 re-run
the two CR4 removals on this tree.

## Gates observed in this seat

These are outcomes of commands run in this session on the final tree. The
formatting check is clean. Workspace clippy (all targets, all features,
locked, warnings denied) finished with no diagnostic. brokkr-runtime: lib
1,237 passed; `it` 135 passed (its `witness_digests::` among them, without
blessing, so no pin moved; its budget tests too); `capability_launch` 76,
`operated_repo` 1, `queued_launch` 6. brokkr-cli: lib 944 passed (1
ignored), integration 748 passed (3 ignored), and its other binaries passed.
Every other workspace crate passed. `recipes/self` compiled ("policy sweep
(decision 0050): 47 valuations over 11 groups, 0 unruled"). `openspec
validate --all --strict`: 30 passed, 0 failed. `typos --hidden` and staged
and unstaged `git diff --check`: no findings.

One full runtime lib run failed once in
`engine::broker::admission::tests::every_steps_door_rechecks_the_lease_its_attempt_holds`
(the event-type assertion at its line 1432). It passed alone and in three
further full lib runs. Its spawns are sealed with no broker, so `begun`
returns no head and the fold reads nothing on its route. It is named here as
an intermittent and was not chased.

Run 3's seat did not run `quality/ratchet.sh`; its review then found the
clone ratchet refusing two test clones. Run 4 repaired them (above) and, on
its final tree, observed `quality/ratchet.sh clones` ("duplication holds")
and `quality/ratchet.sh files` ("file size holds"), workspace clippy clean
again, and the brokkr-runtime lib suite at 1,237 passed. Run 4 changed only
that lib's in-crate test module, so the other binaries' run 3 results above
stand and were not re-run. No function grew past clippy's denied
`too_many_lines`, and no `#[expect]` or `#[allow]` was added. Pending, not
observed here: `scripts/measure-budgets.sh` (no budgeted input moved), the
exact-coverage gate (`scripts/coverage-exact.sh`, which needs a host able to
create namespaces), macOS build and test, and remote CI.

## Notes for review

The fold publishes before appending, and `Engine::settle` publishes again at
release through U8c's verified-reuse path, so each ledger is read by
publication and by the fold. Memory is one ledger at a time (64 MiB, 4,096
rows) plus the journal's suffix since the attempt began.

The head `begun` takes is the store's head, not a chain the engine verified
from seq 1; `load_after` verifies that the row at that seq has that hash and
that the suffix chains onto it, which is all deduplication reads. No row of
the attempt can lie before it: it is taken before the driver spawns. A
store failure taking it, on a brokered site only, is an engine error before
spawn.

A refused call whose model-supplied tool falls outside seat-record v6's
tool vocabulary is refused by the append fence and fails the attempt
(`a_checkpoint_the_fence_refuses_…`); its evidence stays in the ledger.
That is fail-closed, but it lets a model's tool name fail its own attempt;
it is named as a follow-up, not changed here.
