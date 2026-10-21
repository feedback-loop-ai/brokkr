# U8e — Settle every attempt a running engine ends

Task 42.1, as amended by the operator's ruling of 2026-10-21; requirements
CR3, CR4 and MB5, with MB3's and MB4's observed-stage causes. Branch
`s2/U8e`, cut from main at db8ff76f. Run 1
(`0065-slice-two-unit-u8e-see-the--fcfc4990`, cut from 1f5f1470) built the
work and stopped IMPL-BLOCKED; its signed commit 8db7f4d9 was replayed onto
db8ff76f and finished in run 2 (`0065-slice-two-unit-u8e-see-the--bfe32b1d`).
Operator ruling 1 of 2026-10-20 admitted `engine/panel.rs`; rulings 2 and 3
(whole-ledger recovery under CR3's cap; no refusal cause in seat-record v6,
#812) stand unchanged.

Status: 42.1 is met and ticked. Under ruling 4 of 2026-10-21, recovering
an attempt in flight at engine restart is U8e2's (42.3), and driving the
broker's real transport faults through the barrier is U9b1's (42.2). Both
stay unticked. This unit proves that the restart route keeps today's
judgment, and, while the MCP compile fence stands, brings the broker's
faults to the barrier as the ledgers the broker leaves.

## What was built

U8d's barrier is met by every route by which a running engine ends an
attempt's drivers, and it reads evidence only once every process the
attempt owned is proven settled, not only the driver whose report it holds.

The gate in `Engine::fold_with` (`engine/broker/fold.rs`) is widened by one
fact the lease already carries. Besides this report's `Cleanup::Settled`, no
driver of the attempt may have been witnessed unresolved (`Held::unproven`,
which #403's `settlement::witnessed` sets the first time any of the
attempt's drivers ends with its tree not proven over). A panel member that
settled is therefore not read while a sibling's broker may still write, and
an attempt whose earlier driver was unresolved is not read later either.
Such an attempt stands as its report says, already indeterminate, its
ledgers untouched on disk and its lease never released. No second fold,
ledger reader, inventory reader or plan reader was added: every route calls
U8d's `begun` and `folded`.

A panel's members meet the barrier (`engine/panel.rs`). `run_panel` takes
each member's head with `Engine::begun` before anything spawns, and each
member's thread hands back its report beside that head. A member whose door
refused it hands back no head: it started no broker, so its failed start
stands and no ledger is reported missing for it (CR3, "a proven
never-started broker records failed start"). Once every member has ended
and the sink has settled, the new private `Engine::settle_members`
witnesses every member's cleanup on the lease first, then hands the sink's
error up if it failed, then carries each member's checkpoint outcome and
folds each member's report through `Engine::folded` in declared order before
any is returned. The witnessing loop moved there from `run_panel`
unchanged, and still runs before a sink failure is handed up.
`MemberRun::report` returns the engine's existing `DriverRun` (`Ran` or
`SpawnFailed`), the typed outcome `run_driver` hands back for a single
seat, so no `Result<_, String>` is added (decision 0071 ruling 8; run 1's
review L1, confirmed at `panel.rs` `fn report` on this head). The
failed-start report it used to build inline is the new private
`unspawned`, word for word.

Every route a running engine takes, on this head:

| Route | Where it meets the barrier |
| --- | --- |
| Ordinary single seat, failed, timed out, cancelled by its deadline | `dispatch.rs` 292 calls `run_driver`, which folds at `engine.rs` 1746 from the head taken at 1673 (U8d), unchanged |
| Sequence single step and validator step | `sequence.rs` 297 and 409 call `run_driver` |
| Sequence panel step | `sequence.rs` 349 calls `run_panel`, which folds each member |
| Panel seat | `panel.rs` 61 (`execute_panel`) calls `run_panel` |
| A driver whose door refused it | `run_driver`'s `SpawnFailed` and the panel's head-less member: proven never started, nothing read, the failed start stands |
| An error handed up before the report | no result is admitted; `Engine::settle` does not release the lease after an error, so the reservation stands |
| A cleanup not proven settled | the gate above: not read, indeterminate, lease standing |
| An engine restarted with the attempt in flight | today's judgment, kept by ruling 4: `advance_running`'s `EffectInFlight` arm (`engine.rs` 982) journals `EffectIndeterminate` without reading, appending or replaying anything, and the reservation stands. Recovery is U8e2's |

Provisioning and admission refusals (`broker.rs` `provision_with`,
`admission.rs` `refuse_admission`) journal their failure before any driver
spawns, so no broker of the attempt started; they are the same
proven-never-started case and are unchanged.

## Owning tests

While the MCP compile fence stands no public engine can provision a plan,
so, as for U8a2b, U8c and U8d, the proofs are in-crate: the new child
module `crates/brokkr-runtime/src/engine/broker/fold/tests/routes.rs` (341
lines), declared from `broker/fold/tests.rs`. The three owning suites the
row names were not changed: `tests/capability_ledger.rs` cannot reach an
engine that provisions a plan, and `engine/cleanup_tests.rs` and
`engine/resume_tests.rs` hold no broker fixture (`resume_tests.rs` is
frozen at its baseline). No test moved. `broker/fold/tests.rs` (1,105
lines) gained the shared builders the new module uses: `brokered` (a spawn
sealed with given brokers, also used by `Site::new`), `Site::pinned` (one
more plan pinned last by the inventory, also used by `Site::second`),
`serving_input` and `concluded` (a site driven through `run_driver` and
`conclude_single`, also used by U8d's
`a_dispatched_driver_is_settled_by_its_broker_calls_before_its_result_is_admitted`,
whose assertions are unchanged except that a separate check that the driver
ran was dropped, its exact event type and cause already proving it).

Every harness below reports success, or is killed, whatever its broker
latched. Each ledger is written by the protocol's own `Ledger`, which is how
the broker's faults reach the barrier while the fence stands.

| Test | Exact assertions |
| --- | --- |
| `a_live_half_record_is_not_read_while_any_driver_of_its_attempt_is_unresolved` | With a sibling driver witnessed unresolved, a settled report over a half-written ledger keeps `refused == None`, journals no row, and the ledger bytes are unchanged. The same bytes in an attempt whose every process settled fail with "broker ledger ends in a partial record", rows exactly [call 1 succeeded, call 2 interrupted]. |
| `every_member_is_witnessed_before_any_member_is_read` | Through `settle_members`: a settled member declared before an unresolved sibling is returned with `refused == None`, as is the sibling, and nothing is journaled. |
| `every_panel_member_is_settled_by_its_own_broker_calls` | A real three-member panel through `run_panel`: docs (one answered call, clean) succeeds; wiki (its box failed, harness success) fails with exactly "MCP server box could not be established"; notes (door refused, its plan's ledger never written) fails with exactly "member driver did not spawn: the door refused it"; rows exactly [docs call 1 succeeded]. |
| `a_driver_that_did_not_spawn_is_a_failed_start_without_a_missing_ledger` | A refused single driver journals `EffectFailed` with payload exactly {effect, attempt, "driver did not spawn: the door refused it"}, and no row. |
| `a_successful_harness_fails_with_its_brokers_observed_stage_cause` | Dispatched through `run_driver`: Establishment (the box before the secret wait), Unavailable (readiness), Handoff (a handoff failure, or status lost before sender completion), Exec (an exec failure, or status lost after it), Protocol and Timeout (status EOF followed by no functioning child) each journal `EffectFailed` with error exactly "<cause>; stderr tail: " and no row, so none closes Clean because a pipe ended. A ledger whose closure never persisted fails with exactly "broker ledger lifecycle is invalid; stderr tail: ", rows exactly [call 1 succeeded]. |
| `a_driver_killed_at_its_deadline_settles_its_interrupted_call` | A driver killed at its 1 s deadline, its broker cut off mid-call: `EffectIndeterminate` with payload exactly {effect, attempt, "attempt exceeded its 1s deadline and was killed; its process tree is gone, and what it did is unknown; stderr tail: "}, rows exactly [call 1 succeeded, call 2 interrupted]. |
| `an_attempt_its_engine_could_not_conclude_is_restarted_indeterminate_without_a_read` | The restart route keeps today's judgment. (1) Calls folded, then the engine dies before its result (folded outcome exactly "MCP server protocol is invalid"); a restarted engine with no lease journals `EffectIndeterminate` exactly {effect, attempt, "engine restarted while the attempt was in flight; completion cannot be established"}; rows still exactly [call 1] once; ledger bytes unchanged; `recheck` of the dead engine's lease is `Ok(())`, so the reservation stands. (2) The journal refuses a live checkpoint: `concluded` and then `settle` each hand up the forced failure, matched by variant and data (`StoreError::Sqlite(SqliteFailure(..))`, extended code `SQLITE_CONSTRAINT_TRIGGER`, message exactly "forced event failure"), never by `Display` (run 1's review M3, confirmed at `routes.rs` `fn forced`); the lease still stands after settle; the restart journals the same indeterminate payload; nothing is folded; ledger bytes unchanged; the lease still stands. |

CR4's separate deterministic controls are U8d's and untouched, and pass on
this head: `version_protocol_and_capacity_endings_keep_their_causes_despite_complete_calls`
(exactly 0, 1 and 4,096 rows with their causes),
`a_healthy_zero_call_session_settles_nothing_and_succeeds` and
`a_clean_session_settles_one_checkpoint_per_accepted_call_and_its_result_stands`.
The private exec-status tags, invalid status on each side of sender
completion and status EOF followed by no functioning child are told apart
in the broker (brokkr-cli `broker/session/readiness.rs`, untouched); what
reaches the engine is the one cause each latches in Closed, which the stage
test judges. Native lost and stranded checkpoint handling
(`checkpoints.rs`, `contention_tests.rs`) is unchanged and its tests pass in
the lib suite.

## Removal controls

Each mutation was a compiling edit, run with `cargo test -p brokkr-runtime
--all-features --locked --lib broker::fold`, and restored from a copy under
`.forge/mut/`, `cmp` printing "restored". M1–M11 were run in run 1 on
1f5f1470; M1, M5 and M7 were run again in run 2 on this head, with the
results shown.

| Mutation | Property | Failing tests (assertion, and what it got) |
| --- | --- | --- |
| M1: `fold_with`'s `unproven` forced false | settlement ordering, attempt-wide | run 2: `a_live_half_record_…` (`routes.rs:71`, got the partial-record failure and two rows) and `every_member_is_witnessed_…` (`routes.rs:118`, got docs failed with the partial record); 29 passed, 2 failed |
| M2: `settle_members`' witnessing loop witnesses nothing | settlement ordering, panel | `every_member_is_witnessed_…` (docs read before its sibling was witnessed) |
| M3: `settle_members` folds with no head | panel members meet the barrier | `every_panel_member_…` (wiki `Succeeded`) |
| M4: a member that did not spawn keeps its head | proven never-started is not read | `every_panel_member_…` (`routes.rs:163`, notes "broker ledger is missing for this attempt; the driver then failed: member driver did not spawn: the door refused it") |
| M5: `ledger()` reads a failed Closed as no failure | the engine disposition check | run 2: `every_panel_member_…` (`routes.rs:163`), `a_successful_harness_…` (`routes.rs:213`), `an_attempt_its_engine_…` (`routes.rs:312`), and U8d's `version_protocol_…` (`tests.rs:421`), `a_failed_zero_call_…` (`tests.rs:375`), `a_dispatched_driver_…` (`tests.rs:1068`); 25 passed, 6 failed |
| M6: `run_driver` folds a failed report for a door refusal | proven never-started is not read | `a_driver_that_did_not_spawn_…` (payload "broker ledger is missing for this attempt; the driver then failed: driver did not spawn: the door refused it; stderr tail: ") |
| M7: the restart arm journals `EffectFailed` | restart keeps today's indeterminate judgment | run 2: `an_attempt_its_engine_…` (`routes.rs:282`, got `EffectFailed` with the restart reason); 6 of 7 route tests passed |
| M8: `settle` ignores `ran` and forces `proven` | uncertain settlement keeps the reservation (a cleanup-family control, kept apart) | `an_attempt_its_engine_…` (the `recheck` right after `settle`: `Err(Identity(Lost))`) |
| M9: `run_driver` passes no head | the single-seat barrier | `a_successful_harness_…` (`(EffectSucceeded, Null)`), `a_driver_killed_…` (no rows), U8d's `a_dispatched_driver_…` (`EffectSucceeded`) |
| M10: `forced` expects the message "forced event failurx" | the storage failure is matched by its data | `an_attempt_its_engine_…` (`routes.rs:332`) |
| M11: `forced` expects extended code `SQLITE_CONSTRAINT_TRIGGER + 1` | the same, by its code | `an_attempt_its_engine_…` (`routes.rs:332`) |

M5 is the disposition removal, and M1 and M2 are the ordering removals.
Each was made alone, apart from the cleanup control M8. No mutation touches
the CLI's exec-status propagation, which is the broker's and 42.2's. After
the run-2 restores, `git status` showed no unstaged change and the fold
suite passed 31 of 31.

## Scope, files and API

| File | Change |
| --- | --- |
| `crates/brokkr-runtime/src/engine.rs` (row) | No hunk; 3,523 lines. `git diff db8ff76f` over it prints nothing, so both `bundle.run_started(...)` calls from #809 and their surroundings are untouched. |
| `crates/brokkr-runtime/src/engine/sequence.rs` (row) | No hunk: its steps reach the barrier through `run_driver` and `run_panel`. |
| `crates/brokkr-runtime/src/engine/broker.rs` (row) | 800 lines before and after; line 74 `mod fold;` becomes `pub(super) mod fold;`, so `engine/panel.rs` can name `fold::Begun`, already `pub(in crate::engine)`. `const STARTUP: Duration = Duration::from_secs(30);` is untouched on line 100. |
| `crates/brokkr-runtime/src/engine/broker/fold.rs` (existing private child of a row file) | The attempt-wide gate and the module and method docs; 399 lines (row 387 → 399). |
| `crates/brokkr-runtime/src/engine/panel.rs` (operator ruling 1) | Heads before spawn, `settle_members`, `unspawned`, `report` returning `DriverRun`; 510 lines, with no `quality/file-lines.txt` row, so the 800-line ceiling holds it. |
| `crates/brokkr-runtime/src/engine/broker/fold/tests.rs` | Shared builders and `mod routes;`; 1,105 lines (row 1,081 → 1,105). |
| `crates/brokkr-runtime/src/engine/broker/fold/tests/routes.rs` | New test child, 341 lines (new row). |

No public item was added: `settle_members` is `pub(super)` (engine-wide,
for its test) and `unspawned` is private. `git diff db8ff76f --stat` over
`crates/brokkr-cli`, `crates/brokkr-protocol`, `crates/brokkr-store`,
`contracts`, `policy`, `reference`, `fixtures` and `extensions` printed
nothing. No `#[expect]` or `#[allow]` was added.

## Gates observed in run 2, on this head

rustfmt's check and workspace clippy (all targets and features, locked,
warnings denied) were clean. brokkr-runtime: lib 1,257 passed; `it` 137
(its `witness_digests::` among them, unblessed, so no pin moved);
`capability_launch` 76; `operated_repo` 1; `queued_launch` 8. brokkr-cli:
lib 944 passed (1 ignored); `it` 759 passed (3 ignored); `driver_conformance`
27; `heap_claude`, `heap_codex` and `heap_dsh` 1 each;
`transcript_surfaces` 14. Every other workspace crate (bridge, core,
protocol, store, view and their test binaries) reported only passing
results. None of the commission's named load flakes failed in this run.
`recipes/self` compiled, ending "policy sweep (decision 0050): 48
valuations over 11 groups, 0 unruled". `openspec validate --all --strict`
totalled 31 passed and 0 failed. `typos --hidden` printed nothing and the
staged `git diff --check` was clean. The ratchet said "file size holds",
"duplication holds" and "no baseline raised since db8ff76f".

In run 1, `scripts/measure-budgets.sh` was run (review I1) and moved only
the dates in the notes of three `quality/` files, which were restored; this
unit changes no prompt, package or heap input, so nothing of them is
committed.

Pending, not observed here: the exact-coverage gate
(`scripts/coverage-exact.sh`, which needs a host able to create
namespaces), the coordination bind-alias test, macOS build and test, and
remote CI.

## Notes for review

A settled call row of a panel member carries no member tag, as U8d's typed
`BrokerCheckpoint` has none. Its call id is derived from the plan's own
site label, which names the member's site. Live member checkpoints keep
their tags as before.

Carried, not changed: the commission's LOWs (the duplicated budget, floor
and share rule; artifacts' restated bounds and stage names; `.ledger`
spelled three times; the shared conflict wording; fail-closed coordination
refusal; masked tool names under v6), and the refused-checkpoint cause owed
to seat-record v7 (#812). Restart recovery is U8e2's (42.3), whose design
comes to the operator first; the real-broker transport faults are U9b1's
(42.2).
