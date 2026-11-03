# U6d evidence: durable ledger records before calls

U6d is delivered by run `0065-slice-two-unit-u6d-see-the--4d01a3f3` on
branch `s2/U6d`, cut from main at `e9d012f6`, as one signed commit. It
closes tasks 29.1 and 29.2. Everything below was observed in that run's
seat on Linux; what was not observed is marked pending at the end.

## What changed

The shared closed ledger lives in the new protocol module
`crates/brokkr-protocol/src/broker/ledger.rs`, declared `pub mod ledger`
from `crates/brokkr-protocol/src/broker.rs`. One ledger is one JSON object
per line, at most 4 KiB with its newline, numbered by a contiguous record
sequence: exactly one Opened, naming the plan's `Owner` and digest; per
accepted call one Started with the next call sequence and its tool, then at
most one Terminal for that call and tool with its outcome (succeeded,
failed, or refused for an ungranted tool, an active call or a spent
budget); and one Closed, last, carrying `Disposition::Clean` or
`Disposition::Failed(Failure)`. `Failure` is the latched first fatal cause:
`Refused(Refusal)`, which reuses U6c's closed catalogue for every
admission, readiness, handoff and exec cause, or the server's version, its
protocol, or the ledger's own limit. No argument, request identity,
response body or child output is recorded.

`Lifecycle` is the one statement of D7's table. The writer admits every
record through it before writing it, and `recover` reads a settled ledger
through it, keeping the verified prefix, each call's outcome (none for an
interrupted call), the closure where one verifies, and the first `Invalid`
cause where verification stops; a ledger that ends without its closure is
`Lifecycle`. The writer (`Ledger`) appends each record whole and syncs it
through one append closure (`Ledger::open_with`, the house's closure seam);
`Ledger::open` creates the file beside its plan, named `<plan>.ledger`, with
`O_CREAT | O_EXCL | O_NOFOLLOW`, mode 0600, syncs the directory, then
records the Opened. A ledger already there is `Unrecorded::Reopened`
("broker plan has already been opened"); any other failure to create or
record is `Unrecorded::Record` ("broker call could not be recorded"), after
which the writer records nothing more. `Ledger::room` is asked before each
request is read: at 4,096 accepted calls it latches `Failure::Limit` and
gives no room. `Room::begin` reserves the call's longest Terminal against
the record bound before it writes the Started, so an accepted call always
has room for its Terminal; the closure's room follows from the call bound
and the record bound, which a compile-time assertion holds under 64 MiB.

The CLI consumes it in `crates/brokkr-cli/src/broker/session.rs` and its new
private child `crates/brokkr-cli/src/broker/session/lifetime.rs` (added
under the operator's rule of 2026-10-09, extended 2026-10-11:
`session.rs` would otherwise pass 800 lines; it is now 798). `bound` now
also checks the plan's closed shape (`well_formed`, moved from `screened`
with its order unchanged) and returns the held attempt directory. `admit`
opens the plan's ledger right after binding, before the binding names, the
fixed keys, the observer's box preparation, readiness, lookup or exec, and
closes it once admission answers: Failed with admission's refusal, or with
SD3's `ServingIncomplete` where it admitted the plan, which `serve` then
refuses with. So no session closes Clean yet and none has a call. A
closure that cannot be recorded leaves the first cause as the broker's
exit; recovery then finds only the verified prefix. An unbound plan opens
no ledger.

The carried U6c LOW (SC-1, rulings 3 and 6) is fixed in `broker.rs`: the
public records are declared through one private `records!` macro whose only
`Deserialize` reads the same fields through a struct no caller can name,
offered a map and nothing else (`Object`); `Tree` does the same with a
private tagged enum. There is no longer a derived `remote = "Self"`
decoder, which was a public inherent `deserialize` beside the trait.
`Owner` gains `Clone`, `PartialEq`, `Eq` and `Serialize`, for the Opened
record; `Refusal` gains `Clone`, `Serialize` and `Deserialize`, by its
kebab-case name, for `Failure::Refused`.

The carried U6c4 LOW is already answered on this base, as triage found:
`Refusal::Establishment` exists and this unit reuses it.

## Tests and their removals

The owning suite is `crates/brokkr-cli/tests/capability_broker.rs`, with one
new child, `crates/brokkr-cli/tests/capability_broker/ledger.rs`, declared
there; no test moved. The fixture's `Sealed::serve_in` and
`observer::serve_args` now remove the plan's ledger first, a fresh lifetime
as the engine provisions each, and `Sealed::serve_again` serves without
doing so.

Every removal was run as `cargo test --locked -p brokkr-cli --test it --
capability_broker::ledger::` (the decoder rows with
`capability_broker::no_public capability_broker::every_record`) on the
candidate tree committed as `0455cdcb`, one compiling mutation at a time,
each restored with `git checkout` before the next.

| Test | Removal | Observed failure |
| --- | --- | --- |
| `the_first_fatal_cause_survives_later_causes_and_the_end_of_input` | `fail` overwrites the latched cause | closure carried the later cause (assertion at :186) |
| `at_the_call_bound_the_next_frame_stays_unread_and_closure_says_why` | `room` admits at 4,096 (`<=`) | the 4,097th frame was read and its Started refused (:251) |
| `a_ledger_of_more_than_4096_calls_is_out_of_order` | the lifecycle's call bound removed | a crafted 4,097th Started verified (:597) |
| `a_records_terminal_is_reserved_before_it_is_accepted` | the Terminal reservation in `begin` removed | the exact-fit call's Terminal could not be written (:281) |
| same test | the writer's 4 KiB line bound removed | the over-long call was accepted (:281) |
| `a_record_not_made_whole_is_never_acted_on_and_only_its_prefix_verifies` | the writer no longer latched broken after a failed append | a Terminal-failure session closed `Ok` (:403) |
| same test, and `a_ledger_out_of_sequence_of_another_owner_or_malformed_verifies_only_its_prefix` | the reader's partial-record check removed | torn records verified (:403, :584) |
| `a_ledger_out_of_sequence_…` | the reader's 4 KiB line bound removed | "a record over 4 KiB" verified a call |
| same test | the Opened owner and plan comparison removed | "another owner" verified |
| same test | the record-sequence check weakened to `>` | "a record sequence gap" verified a call |
| same test | the call-sequence check weakened to `>` | "a call sequence gap" verified a call |
| `a_ledger_out_of_d7s_order_verifies_only_its_prefix` | a second Opened admitted | the case loop failed (:523), whose only case it reaches is "a second Opened" |
| same test | a second Terminal admitted | "a second Terminal": `Failed`, `Lifecycle` instead of `Conflict` |
| same test | another tool's Terminal admitted | "another tool's Terminal": `Failed`, `Lifecycle` |
| same test, and `closure_is_refused_while_a_call_is_unresolved` | Closed admitted with a call unresolved | "Closed before a Terminal" closed Clean; `close` returned `Ok(Clean)` |
| same tests, and `a_record_not_made_whole_…` | a ledger without closure no longer `Lifecycle` | "no Closed", "opened" and the unresolved case gave `None` |
| `a_ledger_out_of_d7s_order_…` | records admitted before an Opened | "no Opened" verified a begun call |
| same test | records admitted after Closed | "a record after Closed" verified a second call |
| `a_plans_ledger_is_created_once_beside_it_and_this_users_alone`, `a_second_lifetime_of_a_plan_is_refused_before_anything_starts` | `O_EXCL` removed | the second open succeeded (`None`); the second `serve` ran admission |
| `a_plans_ledger_…` | every create failure mapped to `Reopened` | "missing/plan.json" gave `Some(Reopened)` |
| same test | mode widened with group read | mode 0o640 (416) |
| `each_ledger_cause_reads_in_cr3s_words` | `Invalid::Gap`'s text changed | the pinned text differed (:155) |
| `a_healthy_session_with_no_call_is_opened_then_closed_clean`, `tool_errors_and_local_denials_are_calls_and_leave_the_session_clean` | closure without a latched cause written Failed | both, and four others pinning Clean, failed |
| `a_bound_plan_refused_closes_failed_with_its_cause_and_no_call`, `a_box_that_cannot_stand_closes_failed_with_no_call` | the session's cause not latched in `lifetime.rs` | both closures were `Clean` |
| `a_bound_plan_refused_…` | an admitted plan's closure cause changed to `Exec` | closure `Failed(Refused(Exec))` |
| same test | the broker creates `.forge` where it runs | the no-journal assertion gave `[true, false]` |
| `a_second_lifetime_…` | the ledger opened after admission | the second lifetime read the store once (`lookups() == 1`, :753) |
| `an_unbound_plan_opens_no_ledger` | the shape check moved after the ledger opened | a misnamed bound plan created a ledger (:669) |
| `no_public_decoder_reads_a_record_by_position` | `Restrictions` restored to the pre-fix derived `remote = "Self"` decoder | `Restrictions::deserialize(json!([]))` returned `Ok(())`, while `every_record_is_an_object_never_a_positional_array` still passed |
| same test, and `every_record_is_an_object_…` | `Tree` read without `Object` | the positional tree decoded; the plan with a positional tree was admitted |
| `no_public_decoder_…` | `deny_unknown_fields` removed from the records' read struct | an unknown field on `Owner` decoded |

The durable sync itself (`sync_data`, and the directory's `fsync`) was not
bound on this visit; the second return from review, below, binds both.

## Gates on the restored candidate

All were run in this seat after the last removal was restored.
`cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets
--all-features --locked -- -D warnings` were clean. The owning filter
passed with the code-health tests (`capability_broker:: layering::
suppressions:: ratchets::`: 118 passed, 1 ignored). Crate by crate, under
`timeout 590`: `brokkr-protocol` 877 passed (3 ignored) plus its three
small binaries; `brokkr-cli` lib 679, `it` 571 (2 ignored),
`driver_conformance` 27, `transcript_surfaces` 1, the three heap tests;
`brokkr-runtime` lib 930, `capability_launch` 75, `it` 131 (its
`witness_digests::` included, unchanged, so no pin moves), `operated_repo`
1, `queued_launch` 3; core, store, view and bridge all green; doc tests
green. `cargo run --locked -p brokkr-cli -- compile --bundle recipes/self`
compiled (policy sweep: 47 valuations, 0 unruled). `openspec validate --all
--strict`: 19 passed. `typos --hidden` and `git diff --check` were clean.
`quality/ratchet.sh files`, `clones` and `api` hold with the updated
baselines.

`quality/file-lines.txt` carries the new rows for `broker/ledger.rs` (471),
`session/lifetime.rs` (60) and `capability_broker/ledger.rs` (760), and the
re-measured `broker.rs` (313), `session.rs` (798), `capability_broker.rs`
(1,820) and `capability_broker/observer.rs` (975). No function is over a
ceiling and no suppression is added. `quality/public-api/brokkr-protocol.txt`
is re-measured on the pinned nightly: the only difference is the new
`broker::ledger` items. `quality/ratchet.sh baselines e9d012f6` names that
raise, 955 to 999 public items, which needs the operator's ruling at PR
time.

A scoped `cargo llvm-cov` run of the new tests showed `broker/ledger.rs` at
200 of 200 lines and 31 of 31 functions, `broker.rs`'s changed decoders and
`session/lifetime.rs` fully covered; it measured no branches.

## Return from review (run 0065-slice-two-unit-u6d-see-the--4d01a3f3)

The engine council returned four MEDIUM findings on 3f99826f; this visit
answers each, inside the row's files and their test suites.

M1, the overlap rule. D7's table lets a second Started stand only for an
already decoded bounded local refusal. `Lifecycle` now counts the
unresolved calls and marks each call begun while another is unresolved;
such a call's Terminal is admitted only refused, else `Invalid::Lifecycle`.
The closure's check reads the same count. Writer and recovery share the
rule, since both admit through `Lifecycle`. The new test
`a_call_begun_while_another_is_unresolved_ends_only_refused` holds the
positive control (an active call, a second refused as `Active`, the first
answered, closed Clean, recovered whole), the writer's refusal (the second
ended `Succeeded` gives `Err(Unrecorded::Record)`, nothing appended, and
closure then refuses), and recovery of the reviewer's sequence both ways
round, each stopping at the second call's Terminal with `Lifecycle`.

M2, capacity. A call whose longest Terminal would not fit the record bound
is no longer a persistence failure: `Room::begin` latches `Failure::Limit`
and answers the new `Unrecorded::Limit`, whose text is `Failure::Limit`'s,
written once. `a_records_terminal_is_reserved_before_it_is_accepted` now
asserts `Unrecorded::Limit`, no append, closure `Failed(Limit)`, and the
call accepted before it recovered with that closure.

M3, the box's refusals. `Held::ended` in `capability_broker/readiness.rs`
now also returns what the plan's ledger proves, recovered for the owner
and digest of the plan as sealed (the new test helper
`ledger::sealed_ledger`). The absolute-deadline test asserts
`Failed(Refused(Establishment))` with zero calls; in
`capability_broker/handoff.rs` each post-lookup case asserts its own
`Failed(Refused(Name(..)))` or `Failed(Refused(Handoff))`, and each exec
case `Failed(Refused(Exec))`, all with zero calls.

M4, typed answers. `faulted` collects `Result<(), Unrecorded>` and the
fault table compares `Err(Unrecorded::Record)` and `Ok(())` as values; the
operator text stays in `each_ledger_cause_reads_in_cr3s_words`, which now
also pins `Unrecorded::Limit`.

Each removal below compiled, failed as recorded, and was restored. Line
numbers are the panics' own, observed before `cargo fmt` re-wrapped two of
the test files.

| Test | Removal | Observed failure |
| --- | --- | --- |
| `a_call_begun_while_another_is_unresolved_ends_only_refused` | the refused-only check in `Lifecycle::ended` dropped | the writer gave `Ok(())` for `Err(Record)` (:454) |
| same test, its writer block removed for the run | the same check dropped | recovery of "the second first" verified both calls and closed Clean (:487) |
| same test | the overlap never marked (`unresolved > usize::MAX`) | the writer gave `Ok(())` for `Err(Record)` (:454) |
| `a_records_terminal_is_reserved_before_it_is_accepted` | `Room::begin` no longer latches the limit | closure `Ok(Clean)` for `Ok(Failed(Limit))` (:286) |
| `a_box_held_at_its_entry_refuses_at_the_absolute_deadline` | `lifetime.rs` closes every session with `ServingIncomplete` | ledger `Failed(Refused(ServingIncomplete))` for `Establishment` (:261) |
| `a_handoff_that_fails_after_the_lookup_starts_no_server` | the same | case "missing": `ServingIncomplete` for the store's `Name` cause (:246) |
| same test | only `Handoff` closed as `ServingIncomplete` | case "NUL": `ServingIncomplete` for `Handoff` (:246) |
| `the_kernel_alone_runs_a_script_in_the_box_and_nothing_retries_one_it_refuses` | every session closed with `ServingIncomplete` | case "absent": `ServingIncomplete` for `Exec` (:317) |

The three boxed tests ran on this host, serially, rather than returning
early: their removals failed at the ledger assertion. One parallel run of
the handoff test before any edit of this visit saw case "not UTF-8" answer
with an empty stderr; it passed alone and in every later run, the full
`it` binary included, so it is recorded as a flake under load, not a
finding of this unit.

Gates on the restored candidate, run in this seat: format and clippy
clean; `brokkr-protocol` 877 passed (3 ignored) with its small binaries and
doc tests; `brokkr-cli` lib 679 (1 ignored), `it` 572 (2 ignored),
`driver_conformance` 27, `transcript_surfaces` 1, the three heap tests;
`recipes/self` compiled (47 valuations, 0 unruled); `openspec validate
--all --strict` 19 passed; `typos --hidden` and `git diff --check` clean;
`quality/ratchet.sh files`, `clones` and `api` hold. Rows re-measured:
`broker/ledger.rs` 498, `capability_broker/ledger.rs` 836,
`capability_broker/handoff.rs` 839, `capability_broker/readiness.rs` 299.
The public API grows by `Unrecorded::Limit`, now 955 to 1,000 items.
`brokkr-runtime` is untouched by this visit and was not re-run.

Carried, not fixed here: the 4,096-call test's unread-frame assertion
still drives a loop the test writes, to be rebound against the production
serving loop in U6e (council L2); the call-writing and recovery API's only
consumers are tests until U6e/U8b/U8d (L1); `session.rs`'s `Cause` table
beside the serializable `Refusal` catalogue is owed to its consolidating
unit (L3).

## Second return from review: the persistence boundary

The council's chief returned one MEDIUM on c092072c (M1, from
spec-compliance SC-1; ruling 9, CR3): the fault table replaced the whole
append with a memory closure, so neither the directory's `fsync` nor a
record's `write_all` and `sync_data` was bound by a failure.

`Ledger::open` now hands the file it created to a private
`Ledger::durable`, which builds the one append the broker uses: write the
line whole, then sync its data. Behaviour does not move; the split lets a
test reach the real append without a public item (ruling 6). The new
Linux-only unit module `crates/brokkr-protocol/src/broker/ledger/tests.rs`,
declared `#[cfg(all(test, target_os = "linux"))]` from `ledger.rs`, opens a
ledger over a memfd and, at one chosen record, puts a medium that refuses in
place of the ledger's own descriptor with `dup2`, so the failure is the
kernel's: `/dev/full` refuses the write with ENOSPC, and a pipe takes the
write and refuses the sync with EINVAL. For the Opened, the Started, the
Terminal and the Closed, under each medium,
`each_record_is_written_whole_then_synced_before_the_writer_answers` asserts
the writer's exact answers (`Err(Unrecorded::Record)` at the faulted record,
and at closure after it), that the file kept exactly the records before it,
that the pipe took exactly that record whole and nothing after it while the
full device took nothing, and that recovery proves only the verified prefix
with `Invalid::Lifecycle` and never a closure. Its control, with no fault,
answers `Ok` twice and recovers one succeeded call closed Clean. The module
sits in the protocol crate, beside the private constructor, because the
owning CLI suite reaches only public items.

In the owning suite, `a_ledger_whose_directory_cannot_be_synced_records_nothing`
(Linux) opens the plan's ledger through `Ledger::open` with a path-only
(`O_PATH`) handle on the attempt directory, which `openat` accepts and
`fsync` refuses with EBADF. It asserts `Some(Unrecorded::Record)`, a ledger
file created but empty (no Opened), recovery `([], None, Lifecycle)`, and
that a second open of the plan is still `Reopened`.

Each removal compiled, failed as below, and was restored by hand before the
next. The protocol rows ran `cargo test --locked -p brokkr-protocol --lib
broker::ledger`, the CLI rows `cargo test --locked -p brokkr-cli --test it
capability_broker::ledger::a_ledger_whose_directory`.

| Test | Removal | Observed failure |
| --- | --- | --- |
| `each_record_is_written_whole_then_synced_before_the_writer_answers` | the append's `sync_data` dropped | case (1, Pipe): answers `[Ok(()), Ok(())]` for `[Err(Record)]`, the pipe holding all four records (tests.rs:128) |
| same test | `sync_data` before `write_all` | case (1, Pipe): the pipe took `""` for the whole Opened (:128) |
| same test | the writer no longer latched broken before its append | case (3, Pipe): the pipe took the Terminal and then a Closed (:128) |
| `a_ledger_whose_directory_cannot_be_synced_records_nothing` | the directory's `fsync` dropped | `(None, <the Opened>)` for `(Some(Record), "")` (ledger.rs:713) |
| same test | the directory synced after the Opened instead of before | `(Some(Record), <the Opened>)` for `(Some(Record), "")` (:713) |

Gates on the restored candidate, run in this seat: `cargo fmt` and clippy
(workspace, all targets and features, `-D warnings`) clean;
`brokkr-protocol` lib 878 passed (3 ignored), `hands_exits` 6, `secret_drop`
1, doc tests 1. Its first full run failed one unrelated test,
`hands::tests::server::a_server_box_stands_without_the_seats_or_the_hosts_private_paths`
(server.rs:842); it passed alone and in the complete rerun, so it is
recorded as load-sensitive, not a finding of this unit. `brokkr-cli` lib
679 (1 ignored), `it` 573 (2 ignored), `driver_conformance` 27,
`transcript_surfaces` 13 and the three heap tests 1 each. `recipes/self`
compiled (47 valuations, 0 unruled); `openspec validate --all --strict` 19
passed; `quality/ratchet.sh files`, `clones` and `api` hold, the public API
unchanged at 1,000 items. Rows re-measured: `broker/ledger.rs` 506,
`broker/ledger/tests.rs` 146 (new), `capability_broker/ledger.rs` 859.
`brokkr-runtime` is untouched and was not re-run.

Carried from this review, not fixed here. L1: the call-writing and recovery
API (`room`, `begin`, `end`, `recover`, `Lifecycle`'s readers) has only test
consumers until U6e, U8b and U8d. L2: the 4,096-call test's unread-frame
assertion drives a loop the test writes; U6e rebinds it against the
production serving loop with an independent read-order removal. L3:
`session.rs`'s `Cause` table restates `Refusal`'s vocabulary; its
consolidating unit keeps the observer's restricted subset and exact causes.
L4: `session/lifetime.rs` and the CLI's `broker.rs` each choose
`ServingIncomplete` for an admitted plan; both change together at U6f, and
`broker.rs` is outside this row. L5: `session/lifetime.rs`'s `Ended` has a
hand-written `Display`, `Error` and `From` rather than `thiserror`, which
`brokkr-cli` does not depend on directly; adding it means a `Cargo.toml`
outside this row.

## Second run: the closed call causes (run 0065-slice-two-unit-u6d-see-the--270c615e)

The first run parked on its council's M1 (rulings 3, 8 and 9): the ledger's
`Failure` and the shared `Refusal` lacked three causes CR3, D7, MB3 and MB4
already name. The operator's addendum of 2026-10-11 ruled them into this
unit and approved the public `broker::ledger` module with them. This run
received the first run's whole attempt uncommitted on main at `90556ce4`,
kept all of it, and added only the causes, their tests and this record.

`Refusal` gains `Response` ("MCP server response exceeds the broker
limit"), `Timeout` ("MCP server call timed out") and `Masking` ("MCP
response cannot be safely masked"), after `Exec` and before the temporary
`ServingIncomplete`, so the catalogue's order test still holds. `Failure`
gains the same three as its own variants, in D7's order: after the
server's `Version` and `Protocol`, before the ledger's `Limit`. Each
`Failure` text is its refusal's, written once in `broker.rs`; on the wire
each is its lowercase name (`{"failed":"response"}` and so on). A served
cause is latched as its own variant, never as `Refused(..)`, which stays
the plan-admission causes' home; the `Failure` doc says so. Nothing
latches them yet: U6e and U6f consume them. `session.rs`'s observer
`Cause` table is unchanged, since none of the three is an observer cause.

Separately, `cargo +1.88 check` warned that `LEDGER_MAX` was never used,
since that compiler doesn't count use in a `const _` assertion. The
constant now lives inside the assertion's block, with the same value and
assertion.

The tests are in the owning suite. `each_refusal_reads_in_mb3_and_mb4s_words`
(capability_broker.rs) and `each_ledger_cause_reads_in_cr3s_words`
(capability_broker/ledger.rs) pin the three texts on each type. The new
`each_served_cause_closes_a_durable_ledger_first_and_is_never_replaced`
latches each cause after one failed call through `Ledger::open` on a real
file under the canonicalised fixture root, then a distinct later cause. It
asserts the closure `Ok(Failed(cause))`, the file's last line exactly
`{"record":"closed","seq":4,"disposition":{"failed":<name>}}`, and the
recovery `([resolve: Failed], Some(Failed(cause)), None)`. It then latches a
different cause first and this one after, and asserts that the first one
both closes and recovers.

Each removal compiled, failed as below, and was restored by hand before the
next. The text rows ran `cargo test -p brokkr-cli --test it reads_in` and
the others `cargo test -p brokkr-cli --test it capability_broker::ledger::`.
All ran on the uncommitted candidate over `90556ce4`.

| Test | Removal | Observed failure |
| --- | --- | --- |
| `each_refusal_reads_…`, `each_ledger_cause_reads_…` | `Refusal::Response`'s text cut to "…exceeds the limit" | both: the shortened text for the pinned one (capability_broker.rs:558, ledger.rs:165) |
| same two tests | `Refusal::Timeout`'s text changed to "MCP server call failed" | both, likewise |
| same two tests | `Refusal::Masking`'s text cut to "…cannot be masked" | both, likewise |
| `each_ledger_cause_reads_…` | `Failure::Response` displayed as `Refusal::Timeout` | "MCP server call timed out" for the response limit (:165); the refusal pin passed |
| same test | `Failure::Timeout` displayed as `Refusal::Masking` | the masking text for "MCP server call timed out" (:165) |
| same test | `Failure::Masking` displayed as `Refusal::Response` | the response-limit text for the masking one (:165) |
| `each_served_cause_closes_…` | `Failure::Response` serialized as `"timeout"` | case "response": the file's closure held `{"failed":"timeout"}` (:234) |
| same test | `Failure::Timeout` serialized as `"masking"` | case "timeout": the closure held `{"failed":"masking"}` (:234) |
| same test | `Failure::Masking` serialized as `"response"` | case "timeout": the first cause `Masking` recovered as `Failed(Response)` (:244) |
| same test | `fail` lets `Response` replace a latched cause | case "response": closure `Failed(Response)` for the first `Timeout` (:242); the 18 other ledger tests passed |
| same test | `fail` lets `Timeout` replace a latched cause | case "response": closure `Failed(Timeout)` for `Response` (:230) |
| same test | `fail` lets `Masking` replace a latched cause | case "timeout": closure `Failed(Masking)` for `Timeout` (:230) |

After the last restore, format and clippy were clean in this seat, and the
1.88 check finished with no warning. `brokkr-protocol` passed: lib 878 (3
ignored), `hands_exits` 6, `secret_drop` 1 and doc tests 1. `brokkr-cli`
passed: lib 690 (1 ignored), and `it` in two parts, the broker filter (69
in 149.9 s) and the rest (505, 2 ignored). Its other binaries passed too:
`driver_conformance` 27, `transcript_surfaces` 13 and the three heap tests 1
each. `brokkr-runtime` was all green: lib 930, `capability_launch` 75, `it`
131, `operated_repo` 1, `queued_launch` 3. Core, view, store and bridge
passed in all 15 of their targets. Both bundles compiled: `recipes/self`
with 47 valuations and `recipes/verify` with 16, none unruled. openspec
passed all 19 changes, and typos and the whitespace check were clean. The
api, files and clones ratchets hold. Against `origin/main` the baselines
ratchet names one raise, public items 955 to 1,006. That is the approved
ledger module and these six variants, and the pull request carries it as
its `Ruling:`. `cargo crap` over the protocol's `broker` tree puts every
function at CC 15 or below. The highest is `Lifecycle::admit` at 15, which
this run did not change.

Rows re-measured: `broker.rs` 324, `broker/ledger.rs` 516,
`capability_broker.rs` 1,826 and `capability_broker/ledger.rs` 903. No
production function grew and no suppression was added.

Five findings are carried without action, for the chief's reasons. L1: the
ledger readers have only test consumers until U6e, U8b and U8d. L2: the
capacity test's loop supplies its own read ordering, which U6e binds. L3:
the observer's `Cause` table stays beside the serializable `Refusal`. L4:
U6e/U6f consolidate the two `ServingIncomplete` choices. L5: `Ended` keeps
its hand-written `Display` until `brokkr-cli` depends on `thiserror`.

## Pending

The exact-coverage gate (`scripts/coverage-exact.sh`, branches included)
on a capable host, macOS compilation and tests, and remote CI on the final
head are not observed here. Nor is the markdown link check (`lychee
--offline --include-fragments`), which this seat's sandbox refused to run.
The operator approved the public API growth
(955 to 1,006 items) in the addendum of 2026-10-11. The pull request names
that approval as its `Ruling:`.
