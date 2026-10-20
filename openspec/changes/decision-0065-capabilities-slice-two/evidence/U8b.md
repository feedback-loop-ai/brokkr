# U8b evidence: stage retained responses before delivery

U8b was built on branch `s2/U8b`, cut from main at `ebe818f7` (U8a2b, #778),
recipe `claude-flash-dsh-adv`, in run `0065-slice-two-unit-u8b-see-the--4f805c05`.
It is the unit's second run. The first (`0065-slice-two-unit-u8b-see-the--010e5b7b`)
reported oversized before editing anything. The operator then admitted
`rpc.rs`, `rpc/calls.rs` and `crates/brokkr-protocol/src/broker.rs` beyond the
row (2026-10-20, "A"). This run started from main. It closes tasks 39.1 and
39.2 and applies the operator's 2026-10-18 hang-up ruling (U9b1's item 6).

## What changed

The retained path runs through the broker in this order. A tools/call to a
granted tool, with no other call active, is accepted once its Started is
durable. Where its plan retains, it then reserves 8 MiB of the plan's share
(`Ledger::reserve`). With less than that left, it is refused with
`Denial::Budget` and the harness is told "capability evidence budget is
exhausted". The call has its refused Terminal and is forwarded nowhere.

A forwarded call's answer is first prepared at the output edge as one
canonical object. That object holds exactly the result or the error, with no
JSON-RPC id or version. Then the whole frame passes MB4's final guard and
MB3's response bound, whatever the plan retains. Only then does
`Ledger::answered` act. Where the plan retains, it stages those very bytes,
syncs them and charges their actual length. The rest of the reservation is
returned. Then it records the Terminal, citing `{"sha256", "bytes"}`, and
the frame is delivered last. Where the plan retains nothing, it records the
Terminal alone, with no response field.

A stage or a Terminal that cannot be made delivers nothing. The harness is
told "broker call could not be recorded", and the call stays begun for
settlement. Each call holds its own reservation, so refusing a call made
while another is active never takes the active call's room.

The share is the operator's 2026-10-15 function: 256 MiB divided by the
number of retaining plans the attempt's sealed inventory pins.
`session/retaining.rs` counts that number before the ledger opens. It reads
every pin beside the bound plan, as `bound` reads the bound one: in the held
attempt directory, owner-only, of one link, hashing to its pin and parsing
closed. A pin outside the attempt directory, or one that does not verify,
leaves the plan unbound, because its share could not be the engine's. The
same function hands the ledger its own close-on-exec handle of the attempt
directory. Staged files are `<plan>.<call>.response`, created exclusively,
never through a symlink, mode 0600, written whole and synced, then the
directory synced. They sit beside the ledger under the protected root, which
the plan's `excluded.control` keeps out of the box (MB3's identity check, as
before).

SD3's refusal of retained plans is gone (`rpc.rs`'s `initialized`), and so
is `Refusal::RetentionIncomplete`, whose last consumers were tests.

On the hang-up ruling: a harness that ends while a call is active has hung
up on it (`Session::hung_up`, reached when `serving` sees the harness's input
end or its output close). The call fails with its Terminal, the harness is
told so where its output is still open, and the session ends with MB5's
cancellation cause. That cause is the attempt's timeout, `Failure::Timeout`:
the one cause this broker already latches for the attempt's cancellation
(`going`). The ledger so ends with a Terminal and a Failed closure. This is
the seat's reading of "MB5's cancellation cause", since MB5 names no
distinct text.

## Files

| File | Change |
| --- | --- |
| `crates/brokkr-protocol/src/broker/ledger.rs` | Terminal's optional `response`, read strictly; the Lifecycle refuses a response on a refused call; `Call` holds its reservation; `Room::begin` reserves the longest retained Terminal; `retention` registered. 631 → 677 lines. |
| `crates/brokkr-protocol/src/broker/ledger/retention.rs` (new, 179) | Budget, reservation and share; the `Response` record; `Ledger::answered`, `retain`, `retain_with`, `reserve`; the production stage. A new private child, because the ledger would otherwise pass 800 lines. |
| `crates/brokkr-protocol/src/broker.rs` | `Refusal::RetentionIncomplete` removed. 397 → 392. |
| `crates/brokkr-cli/src/broker/rpc.rs` | The SD3 refusal removed; `Refused::Budget`. 787 → 786. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` | Reserve before forwarding; stage through `Ledger::answered` before delivery; `hung_up`. 779 → 693. |
| `crates/brokkr-cli/src/broker/rpc/calls/request.rs` (new, 139) | The harness request decoding (`Asked`, `Greeting`, `Cancelled`, `Called`, `Request`, `decoded`, `called`, `vocabulary`, `TOOL_MAX`), moved out of `calls.rs` unchanged to make room (operator ruling 2026-10-20). |
| `crates/brokkr-cli/src/broker/session.rs` | Registers `retaining`; docs. 775 → 777. |
| `crates/brokkr-cli/src/broker/session/lifetime.rs` (existing child) | `serve` derives the retention; `lived` gives it to the ledger. 307 → 314. |
| `crates/brokkr-cli/src/broker/session/retaining.rs` (new, 59) | The retaining-pin count and the staging handle. |

`output.rs` is unchanged. The carried LOW on duplicate decimal normalization
stays open, for the reason under "Carried and follow-up". Test files touched
or added: `rpc/tests.rs`, `rpc/tests/{delivery,masking}.rs`,
`rpc/tests/retention.rs` (new, 356), `session/lifetime/tests.rs`,
`session/readiness/tests.rs`, `session/retaining/tests.rs` (new, 121),
`brokkr-protocol/src/broker/ledger/tests.rs`, and the CLI suite
`tests/capability_broker.rs` with its children `ledger.rs`, `observer.rs`,
`protocol.rs` and `serving.rs`. One test moved: the "ended harness leaves it
begun" case of `a_server_that_takes_no_call_ends_it` (formerly
`…_and_an_ended_harness_leaves_it_begun`) left `rpc/tests.rs`. Its behaviour
changed under the ruling, and the new case lives in `rpc/tests/retention.rs`.

## Proofs and their removal controls

Every mutation below compiled. Each made exactly the named tests fail with
the shown assertion, and was then restored byte for byte. Restoration was
checked with `cmp` against copies kept under `.forge/`, and the suites were
rerun green. All mutations ran at the working tree on `ebe818f7`.

| Behaviour (requirement) | Test (suite) | Mutation | Observed failure |
| --- | --- | --- | --- |
| Hang-up mid-call fails the call and the session with the cancellation cause (ruling item 6, MB5, CR3) | `retention::a_harness_that_ends_during_a_call_fails_it_and_the_session_with_its_cancellation` (unit, polite and killed); `protocol::a_harness_that_ends_its_input_during_a_call_leaves_its_ledger_finished_and_failed` (real box) | M1: `hung_up` returns `Ok(())` with a call active | unit: `left: None, right: Some({"error":…"MCP server call timed out"})`; box: `left: (Some(0), 2, "", "")` against `(Some(1), 2, <timed-out frame>, "error: MCP server call timed out\n")` |
| Retained bytes staged and cited, exactly the delivered masked object (CR2) | `retention::a_retained_response_is_staged_before_its_terminal_cites_it_and_delivered_as_staged`, `retention::an_error_response_is_retained_as_its_failed_calls_complete_object`, `masking::the_canary_as_text_is_masked_and_an_unrelated_number_is_kept_exactly`, ledger `a_retaining_ledger_stages_each_answer_beside_it_before_the_terminal_citing_it`, `protocol::a_retained_response_is_staged_beside_its_ledger_where_its_boxed_server_cannot_reach` | M2: `answered` treats a retaining ledger as non-retaining | the four unit tests above, plus `a_stage_or_terminal…`, `what_a_call_stages…` and `a_call_refused_while…`, which count stages; the two ledger tests that stage; and the real-box test |
| Exhausted share refuses before external work (CR3, CC2) | `retention::a_share_without_room_for_one_reservation_refuses_before_any_external_work`, `retention::what_a_call_stages_stays_charged_and_only_the_rest_of_its_reservation_returns` | M3: the budget denial is ignored | `left: "broker call could not be recorded"`, `right: "capability evidence budget is exhausted"` |
| Actual bytes charged, not the reservation (CR3) | `what_a_call_stages_…` (16 retaining plans, a third call) | M4: the whole reservation is charged | call 3: `left: budget is exhausted`, `right: its result` |
| No same-digest quota reset (CR3) | `what_a_call_stages_…` (32 plans, an identical second call) | M5: staged bytes are not charged | call 2: `left: its result`, `right: budget is exhausted` |
| Stage and Terminal precede delivery (CR2, CR3) | `retention::a_retained_response_is_staged_…` | M6: frame written before `answered`, with a 50 ms pause so the order is observable | `delivery 0 before its Terminal` |
| A response only on an answered call (CC2) | ledger `a_terminal_cites_a_sound_response_for_an_answered_call_alone` | M7: the Lifecycle check is disabled | `("refused", Some(Lifecycle), 1)` against `… 0` |
| Only a sound response is read (CR3) | the same test | M8: `sound()` is always true | `("upper", Some(Lifecycle), 1)` against `(Some(Malformed), 0)` |
| A retaining call reserves its longest Terminal (CR3) | ledger `a_retaining_call_is_accepted_only_where_its_longest_terminal_fits` | M9: plain reservation for a retaining ledger | `left: (Ok(()), 0)`, `right: (Err(Limit), 0)` |
| Nothing staged past the call's reservation | ledger `a_response_its_call_holds_no_room_for_is_never_staged` | M12: the reservation check is disabled | `left: (Ok(()), 1)`, `right: (Err(Record), 0)` |
| The share counts retaining pins only (operator ruling 2026-10-15) | `retaining::tests::every_retaining_pin_counts_and_the_staging_handle_is_the_brokers_own` | M10: every pin counted | `left: 3`, `right: 2` |
| A pin outside the attempt leaves the plan unbound | `retaining::tests::a_pin_that_does_not_verify_beside_the_plan_leaves_it_unbound` | M11: the parent check is disabled | `("elsewhere", Ok(()))` against `Err(Refused(Unbound))` |
| SD3's refusal is removed; the bound public command retains (SD3) | `delivery::a_retained_plan_is_initialized_and_served_as_any_other`; real-box `protocol::a_retained_response_is_staged_…` | M13: `initialized` refuses retained plans again (with `PROTOCOL`) | unit: `(Err(Failed(Protocol)), [], Some(Failed(Protocol)))`; box: `(Some(1), None, "", "error: MCP server protocol is invalid\n")` |
| A call refused as active takes no part of the active call's reservation | `retention::a_call_refused_while_another_is_active_leaves_the_active_ones_reservation_held` | M14: reserve before the active check | call 2 refused for the budget, not as active |
| Unsafe output and refusals stage nothing and cite no digest, retained or not (MB4, CR2) | `masking::a_digits_only_canary_as_a_bare_number_fails_its_forwarded_call_unmasked`, `masking::a_bound_number_spelled_otherwise_…`, `masking::a_bound_number_a_string_spells_…` (via `masking_failed`), `retention::a_share_without_room_…` | M16: every unstaged ending stages bytes | all four masking tests, the budget test and the other stage-counting tests |
| A failed stage writes no Terminal and delivers nothing; no silent metadata downgrade (CR3) | `retention::a_stage_or_terminal_that_cannot_be_made_delivers_nothing_and_leaves_the_call_begun`; ledger `a_retaining_ledger_stages_…` (taken name) | M17: a failed stage is ignored | unit: the result delivered instead of "broker call could not be recorded"; ledger: `left: Ok(())`, `right: Err(Record)` |

The masking suite's retained cases now retain through `Beside::retain`, so
U6f1a's private-seam proofs run through enabled retention, as task 31.1
promised. For the safe controls (text redaction, unrelated exact numbers),
the test asserts the staged bytes equal `canonical::to_bytes` of the masked
result object. It also asserts the Terminal cites their sha256 and length,
and the delivered line is that object with the harness's id.

The four CR1 outcomes are the engine's to compute, and the plan carries only
the effective result. That table is proved upstream by
`capabilities::tests::manifest::effective_retention_is_the_one_rule_manifest_v12_records`
(runtime, U7c0), and the engine seals `retained: holding.retention.effective()`
(`engine/broker.rs:474`). This unit proves both of the broker's dispositions.
With `retained` false, a Terminal holds no `response` and nothing is staged.
With `retained` true, the exact bytes are staged and cited (the first
retention test runs both).

## The real box

This seat's host stood real boxes. The run printed no skip line, and M2 and
M13 made the real-box test fail. So `protocol::a_retained_response_is_staged_beside_its_ledger_where_its_boxed_server_cannot_reach`
ran its proof. Once called, the boxed server probed the attempt directory,
the plan, its ledger and the stage path, by path and through every
descriptor `/proc/self/fd` lists, and tried to write the stage. It reported
only `probed`. The broker then staged
`{"result":{"content":[{"text":"resolved","type":"text"}]}}` outside the
box, the Terminal cited its sha256 and length, and the harness received the
same object under its id `"c"`. After the box was gone (`boxed(&sealed)`
empty), the stage file still held those bytes. The whole `capability_broker`
suite passed on this host: 108 tests in 275 s.

## Gates run here

These commands ran on this host at the working tree. `cargo test -p
brokkr-protocol --all-features --locked` passed: 1019 unit tests, plus 6, 1
and 1 in its other targets. `cargo test -p brokkr-cli --all-features --locked`
passed in two parts. The first ran `--tests -- --skip capability_broker::`:
938 lib, 27 driver_conformance, the three heap budgets, 632 in `it` and 14
transcript_surfaces. The second ran `--test it capability_broker::`: 108.
`brokkr-runtime` passed: 1188 lib, 135 `it` (`witness_digests::` 6 of them,
unblessed and unmoved), 76 capability_launch, 1 operated_repo and 6
queued_launch. Core, view, store and bridge passed together, and so did
`--workspace --doc`.

Both `recipes/self` and `recipes/verify` compiled (policy sweeps of 47 and 16
valuations, 0 unruled). The format check, the workspace clippy with
`-D warnings`, and `bash quality/ratchet.sh files` and `clones` held. The
#782 flake (`what_waits_for_the_harness_is_bounded_and_taken_whole_in_order`)
passed in every broker unit run here. This change does not move what it
measures, since `hung_up` acts only when a call is active and that test has
none.

`quality/file-lines.txt` was set by hand to `wc -l` of every touched and new
file. This seat cannot run `quality/measure.sh`, which needs the exact
gate's LCOV. `quality/public-api/brokkr-protocol.txt` equals `cargo
+nightly-2026-09-05 public-api -p brokkr-protocol -sss`, header aside. The
regenerated `brokkr-cli` API equals its snapshot. No function entered
`too-many-lines.txt`, and no suppression was added. The budget inputs (prompt
bytes, Cargo.lock, transcript heap) did not move; the heap budget tests
passed above.

## Public API, for the operator's ruling at PR time

`brokkr-protocol` grows by five lines. They are an
`impl brokkr_protocol::broker::ledger::Ledger<'_>` block with `answered`,
`reserve`, `retain` and `retain_with`. The CLI broker consumes all four, and
`retain_with` is also the tests' seam, as `open_with` is. One line goes:
`Refusal::RetentionIncomplete`, which the operator's 2026-10-20 ruling
admits. `Ledger::end` keeps its signature, and `Call` gains a private field.

## Pending

Exact coverage (`scripts/coverage-exact.sh`) is pending on a host or in CI
outside a box. So are macOS (the touched code uses only rustix's portable fs
calls and `std` pipes; the new real-box tests sit in Linux-only modules) and
remote CI on the final head.

## Carried and follow-up

The carried LOW (decision 0071 ruling 5) is that `broker/output.rs`'s
`decimal` and `secret/guard.rs` each normalize digits and an exponent. It
stays open. Sharing the server-number reading needs `secret/guard.rs` to
consume it, and this unit is not admitted to edit `secret.rs` or
`secret/guard.rs`. Decision 0012 keeps the binding comparison there. The
follow-up should move the reading into the secret module's shared
`json_numbers` tokenizer side, with `output.rs` consuming it.

A second home: the 256 MiB budget, the 8 MiB floor and the share's division
now live in `brokkr-protocol`'s `ledger/retention.rs` and also in
`brokkr-runtime`'s `engine/broker.rs` (`BUDGET`, `SHARE_FLOOR`, `share`),
which this unit may not edit. The follow-up is to make the runtime consume
one public protocol function. That is a ruling-6 consumer this unit could
not add.

The share's staged/published bound of 512 MiB holds by construction: shares
sum to at most 256 MiB, and the engine publishes at most what was staged.
Publication itself is U8c's.

The multi-broker, staging-fault and failed-delivery proofs that this section
first deferred are now made. The next section gives them.

## Returned from review (run `0065-slice-two-unit-u8b-see-the--4f805c05`, visit 2)

The review of `a8618c58` sent back three MEDIUM findings under decision 0071
ruling 9. This visit answers each with a test and its removal controls. Only
one production file moves. `ledger/retention.rs`'s production stage is now
`staged(&DURABLE, …)`: its write, its data sync and its directory sync are the
three named fields of a `Steps` table, `DURABLE`, taken in the same order with
the same `?`, so each step can meet a fault on its own. The behaviour is
unchanged, and the stage's creation is as before. No public item was added,
since `Steps`, `DURABLE` and `staged` are `pub(super)`.

**M1: brokers sharing one sealed inventory.** The new test is
`session::retaining::tests::each_broker_of_one_sealed_inventory_spends_its_own_share_and_none_of_anothers`.
It writes one attempt directory whose inventory pins 32 retaining plans,
standing for every brokered slot, fallback or not, beside one plan that
retains nothing. Then 32 brokers run at once, one per thread, held at a barrier
until all have opened their ledgers. Each derives its retention through the
production `retention()` and opens its ledger with `Ledger::open`. It retains
through the production `Ledger::retain`, staging in the shared directory.
Each broker's first call is reserved for and staged, which is the exact
bound: a share of one reservation admits one call. Its second call is refused
for the budget before it is forwarded. Each broker reserves on its own, so no
other broker's spending moves its result. Once all have closed, every stage
still holds its own bytes, and every ledger cites their sha256 and length. It
proves `(Succeeded, Refused(Budget))`, a Clean closure, and no invalid
record. With 33 retaining pins, every broker's first and second calls are
refused, and nothing is staged. The engine's provisioning would refuse 33
already (`engine/broker.rs`'s `share`, `Unprovisioned`); this case is the
broker's own defence.

**M2: production write and sync failures.** The new test is
`ledger::tests::a_stage_whose_write_or_either_sync_fails_is_never_cited_and_its_ledger_kept_as_it_was`
(brokkr-protocol, in the Linux-only ledger tests). It runs the broker's own
steps, with one step at a time first meeting a medium that refuses it, put in
place of its descriptor by the module's existing `swap` (`dup3`). The write
meets `/dev/full`, and either sync meets a pipe, so each failure is the
kernel's own. `staged` returns exactly `ENOSPC`, `EINVAL` and `EINVAL`.
Through a retaining ledger, `answered` returns `Err(Record)` in each case.
The ledger file stays byte for byte the prefix it was, holding no Terminal and
no citation, and recovery reads the call as begun. The control fails no step,
and the call is staged and recorded. The CLI side was already bound:
`retention::a_stage_or_terminal_that_cannot_be_made_delivers_nothing_and_leaves_the_call_begun`
shows that `answered`'s `Err(Record)` delivers nothing. The harness is told
"broker call could not be recorded".

**M3: delivery failure after a durable stage and Terminal.** The new test is
`rpc::tests::retention::a_retained_response_whose_delivery_fails_stays_staged_and_cited_and_is_never_asked_again`.
`delivery.rs`'s session helper became `heard`, which takes an optional
retention and returns the server's frames and the ledger's bytes. `asked`
wraps it unchanged. A retaining session answers its call, and the harness's
output then refuses the frame. With `ENOSPC`, the session ends
`Failed(Undelivered)` and the ledger closes with that cause. With `EPIPE`,
the harness is gone, and the session stays served and Clean (operator ruling
2026-10-14). Either way, the call is `Succeeded`, and exactly the prepared
bytes are staged as call 1. The Terminal cites their sha256 and length, and
the server read the `tools/call` once and never again.

| Finding | Test | Mutation (compiled, then restored) | Observed failure |
| --- | --- | --- | --- |
| M1 | `each_broker_of_one_sealed_inventory_…` | R7: `share` divides by `retaining.min(1)` | `left: [(Ok(()), Ok(())); 32]`, `right: [(Ok(()), Err(Budget)); 32]` |
| M1 | the same | M10 again: `retention()` counts every pin (`pinned.retained \| true`) | `left: [(Err(Budget), Err(Budget)), …]`, `right: [(Ok(()), Err(Budget)), …]`; `every_retaining_pin_counts_…` failed too |
| M1 | the same | R8: the stage name is not keyed by its plan (`OsString::from("cap")`) | 31 broker threads panicked on `answered`'s `Err(Record)` (the name was taken), and the test failed |
| M1 (ledger) | `the_budget_is_divided_by_the_retaining_plans_and_each_call_reserves_8_mib` | R7 | `left: (268435456, false, 0)`, `right: (8388608, true, 0)` |
| M2 | `a_stage_whose_write_or_either_sync_fails_…` | R1: the write's error is ignored (`.ok()`) | `left: ("write", Err(Some(22)))`, `right: ("write", Err(Some(28)))` |
| M2 | the same | R2: the data sync is removed | `left: ("sync", Ok(()))`, `right: ("sync", Err(Some(22)))` |
| M2 | the same | R3: the directory sync is removed | `left: ("settle", Ok(()))`, `right: ("settle", Err(Some(22)))` |
| M2 | the same | R4: `DURABLE.sync` is a no-op | `left: ("sync", Ok(()))`, `right: ("sync", Err(Some(22)))` |
| M2 | the same | R5: `DURABLE.settle` is a no-op | `left: ("settle", Ok(()))`, `right: ("settle", Err(Some(22)))` |
| M2 | the same, and `a_retaining_ledger_stages_…` | R6: `answered` ignores a failed stage | `left: ("write", (Ok(()), false, [("resolve", Some(Succeeded))]))`, `right: ("write", (Err(Record), true, [("resolve", None)]))` |
| M3 | `a_retained_response_whose_delivery_fails_…` | R9: `answered` delivers first and records only where delivery held | `left: ("ENOSPC", ([("lookup", Some(Failed))], [Null], []))`, right: the call Succeeded, cited and staged; four older tests failed too |

R10 was a mutation that did not bind. It made `going()` treat an undelivered
output as the end of serving (`Ok(false)`), and the M3 test still passed,
because `drained()` latches the same `Undelivered` cause on its own. R10
removes no behaviour, so it is not counted as a control. Each mutation was
restored and checked with `cmp` against copies under `.forge/u8b2/`.

This visit's gates ran at the working tree. `cargo test -p brokkr-protocol
--all-features --locked` passed: 1020 lib, plus 6, 1 and 1. The brokkr-cli
suite passed in two parts. `--tests -- --skip capability_broker::` gave 940
lib, 27 driver_conformance, the three heap budgets, 632 `it` and 14
transcript_surfaces. `--test it capability_broker::` gave 108 in 221 s. The
real-box retained test printed no skip line when run alone with
`--nocapture`. The workspace clippy with `-D warnings` and the format check
were clean. `quality/ratchet.sh files` and `clones` held after the five test
and retention rows of `quality/file-lines.txt` were set to their `wc -l`.
The #782 flake passed in both broker unit runs.
