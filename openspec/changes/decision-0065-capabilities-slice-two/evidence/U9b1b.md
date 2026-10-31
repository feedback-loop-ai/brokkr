# U9b1b — Prove the enabled namespace path end to end

Branch `s2/U9b1b` from main at f0e0c9645. The first run
(0065-slice-two-unit-u9b1b-see-th-9ea6406d) made two partial visits and stopped on
F1; its record follows from "The proof the first run built". The second run
(0065-slice-two-unit-u9b1b-see-th-9d7328d4), recorded in "The second run" just
below, carries the controller's rulings. Every observation here was made on this
Linux x86_64 workstation (kernel 6.17, ext4 on LVM, shared with other lanes).

## The second run

### F1: `brokkr driver` sets `no_new_privs`

The ruling, verbatim from design.md:

> F1. Production never set `no_new_privs` on a harness, so on an ordinary Linux host MB3 refused an MCP seat under a plain `brokkr run` ("MCP server box filesystem identity is not protected") before any lookup. `brokkr driver` now sets `no_new_privs` on itself, through rustix's safe `thread::set_no_new_privs`, before it spawns a harness the engine brokers MCP for, on Linux. A driver that cannot set it refuses the launch. A launch with no brokered MCP is unchanged, and a seat that no `brokkr driver` starts keeps today's refusal. `crates/brokkr-cli/src/verbs/setup.rs` and `crates/brokkr-cli/Cargo.toml` (the `thread` feature on brokkr-cli's rustix; the lockfile does not move) join this row. Where the driver can learn of a brokered launch only through `crates/brokkr-protocol/src/adapters.rs`'s `serve`, that file joins too, size-neutral. Ruling 21's second case is then proved through real dispatch with no `setpriv` wrapper, and the fix has its own compiling removal recorded. Before this PR lands, the controller runs a real Claude and a real Codex harness through a brokered launch under the flag on a Linux host. A harness that breaks under it returns to the operator as a finding.

The driver learns of a brokered launch only from the `start` message's sealed
serving inputs, so `adapters.rs` joined the row. `serve` takes a `Confine`
(`fn() -> io::Result<()>`), a closure seam per decision 0071 ruling 2, and the
private `serve_with` loop calls `start::admitted` before `run_seat`. That checks the
resume exchange, then calls `Confine` only where the sealed set is
`SealedServers::Brokered`. A refusal is a `failed` result with no `accepted`, the
exchange's own structural path. `verbs/setup.rs`'s `confine` is the only call of
rustix's `set_no_new_privs`, on Linux; off Linux, where the fence stands, it refuses
with `Unsupported`. `adapters.rs` went from 6,989 lines to 6,988. `start::serve_io`,
`#[cfg(test)]`, keeps the suites' 4-argument call as it was. `quality/public-api`'s
one line for `serve` changes, measured with `cargo public-api` on the pinned nightly:
this is the signature change the ruling admits, adding no item.

| Test | Observed | Compiling removal, and how it failed |
|---|---|---|
| `enabled::a_seat_holding_an_mcp_grant_with_boxed_hands_dispatches_through_the_real_broker_and_completes` | `brokkr run` with no wrapper: `completed`, broker exit 0, the harness's own `NoNewPrivs:\t1` | `confine` returning `Ok(())`: `left: ("awaiting_operator", "work", Some("MCP server box filesystem identity is not protected"))`. Asking the kernel for `set_no_new_privs(false)`, which returns `EINVAL`, is the refusal through real dispatch: `left` holds "…the driver then failed: seat refused to start: its driver could not set no_new_privs before a harness the engine brokers MCP for: Invalid argument (os error 22)" |
| `enabled::dispatch::a_seat_without_hands_holding_a_native_grant_dispatches_and_retains_nothing` | A launch with no brokered MCP: the harness's flag is this process's own | `admitted` confining everything with a model surface: `left: "NoNewPrivs:\t1"`, `right: "NoNewPrivs:\t0"` |
| `adapters::mcp::tests::brokers::a_brokered_start_confines_its_driver_first_and_refuses_where_it_cannot` | A refusing `Confine`: `["capabilities", "result"]`, no `accepted`, the exact error; a hands-only start never asks | The refusal dropped (`let _ = confine()`): `left` holds the charter's refusal, reached past the check; confining everything: panics at the hands-only assertion |

Moved off `setpriv`: every launch in the `enabled` suite, whose 40 tests passed with
no wrapper (`enabled::` 40 passed, 1 ignored, in 271 s, at load near 13).
`enabled::an_unconfined_seat_refuses_by_identity_before_any_lookup` is gone with the
condition it pinned. A seat no driver starts keeps the refusal, pinned by
`capability_broker::every_managed_writer_is_observed_confined_and_sealed`, whose
broker an unconfined shell starts. Still on `setpriv`, as no `brokkr driver` starts
their harnesses: `capability_broker`'s shells and `observer.rs`, and
`writers_tests.rs`'s harness shells. The runtime's `capability_launch` and
`resume_tests` drivers pass `|| Ok(())`: they are test stand-ins, their launches
unchanged. Pending, by ruling F1, the controller's: a real Claude and a real Codex
harness through a brokered launch under the flag.

## The proof the first run built

`crates/brokkr-cli/tests/enabled.rs` and its children `enabled/transport.rs`,
`enabled/matrix.rs` and `enabled/dispatch.rs` are new, registered in `tests/it.rs`.
They drive the actual binary: `brokkr run` compiles a fixture realm's MCP grant,
admits and provisions under a protected HOME, and dispatches the real Claude driver
and the shipped Claude adapter over a `claude` on PATH that starts the sealed broker
from its `--mcp-config`, speaks MCP to it over FIFOs, and then reports success
whatever happened. The broker stands the real server box and execs the installed
script by the kernel's shebang; the engine folds the ledger at settlement; the suite
reads the result through `inspect --json`. `World` serializes its own runs, as each
walks about 205,000 system entries twice inside the shared 30 s startup bound.

## Findings for the controller

**F1, ruled and fixed above; as the first run found it: production never confines a
harness, so on an ordinary host no MCP seat can complete.** MB3 admits a multiply-linked system file only where
the broker, its harness and the harness's parent are proved confined, including
`no_new_privs`. No production code sets it (a grep for it over `crates/` finds the
reader in `hands/namespace/sources/host.rs` and tests only). Ubuntu's `/usr`
holds such files, so a seat launched as an operator launches it refuses with
"MCP server box filesystem identity is not protected" before any lookup.
`enabled::an_unconfined_seat_refuses_by_identity_before_any_lookup` pins that as it
stands. Ruling 21's second case therefore completes here only with the engine run
under `setpriv --no-new-privs`, which is how this suite launches it, and it does not
yet hold for an unconfined `brokkr run`. It fails closed, so it is no security hole,
but it is the case the operator ruled must work. The fix lies outside this row's
production files: a process must set the flag before the harness starts.

Options: (a) `brokkr driver <harness>` sets `no_new_privs` on itself before spawning
a harness whose sealed serving is brokered, with the safe `rustix::thread` call;
(b) the engine wraps such a seat's spawn; (c) operators launch the engine confined,
documented by U9b2. **I recommend (a):** the driver is the harness's parent and is
already in the chain the observer reads, nothing outside brokered seats changes, no
`unsafe` is needed, and (b) needs `pre_exec` or an external `setpriv`. It needs a
ruling on the row, as it touches the driver in `brokkr-protocol`. Whether a real
Claude or Codex harness runs under the flag has not been measured.

**F2.** A run's default store, `.brokkr/state/secrets.env`, lies inside the seat's
reach, so it refuses with "MCP secret store is reachable by workspace hands", even
though U8h masks it in the hands box
(`enabled::matrix::a_store_in_the_worktree_refuses_with_no_lookup_or_start`). The
suite uses `--secrets-file` outside the worktree. It fails closed. Options: U9b2
documents `--secrets-file` as required for MCP, or the broker admits a store its box
masks. I recommend documenting it, as admitting a masked store narrows MB4.

**F3, fixed in the row.** A dialect binding `HOME` compiled, and its broker refused
with "broker plan is not bound to this attempt", so the attempt parked on "broker
ledger is missing for this attempt". As the code reads, a binding's name is removed
from its harness's environment, so that broker's HOME was gone; I did not instrument
this. Either way the harness had started, and no startup cause was given. `Slot::seal` in
`engine/broker.rs` now refuses a fixed key of the server box with
`Unprovisioned::Box(Refusal::StartupInputs)` before anything is made, so both names
give the startup cause with no lookup and no harness.

**F4.** SD4's budgets did not hold on this loaded host; see the samples below. The
same load also failed two tests in one full `--test it` pass, and both passed when
rerun: see "What was run".

**F5.** Task 42.2 names an invalid status before and after sender completion through
real dispatch. The bootstrap writes only valid tags, so real dispatch reaches an
invalid byte only by tampering with the process. It stays proved at the private seam
(`broker::session::readiness::tests`) and is not repeated here. The controller can
accept that seam, or commission a ptrace-injected variant. The same holds for MB4's
complete-frame matrix (an open sender after a complete frame, immediate EOF, a
trailing byte, a second frame, truncation): the real broker writes one well-formed
frame and closes, so only `capability_broker::handoff` and the bootstrap's own tests
reach them. I recommend accepting those seams for both, as a tampering harness would
prove the injector more than the broker.

**F6 (second visit).** A seat whose own writable reach holds its server's program
refuses by the source lease, "dispatch refused: MCP server box filesystem identity is
not protected: site … can write …", not by MB3's table entry "MCP server launch
resolves inside seat-writable reach". With the lease removed, the same seat gives
exactly that kept cause, so the box check still stands behind it. Both refuse with
zero lookups and starts, so this is a wording gap and fails closed. The suite pins
the lease's cause. Options: amend MB3's table to say the lease answers first for a
writable reach, or have the lease skip an attempt's own writers. I recommend the
amendment, since the second option adds a path to the protection.

**F7 (second visit).** The serialized `enabled` suite takes about 270 s, which pushes
`cargo test -p brokkr-cli --test it` past this seat's 590 s limit on this host (the
rest took 303 s, at load near 14), so the two parts were run apart. CI has not run
this. If it strains CI, the suite can become a shard of its own.

## Task 42.2

| Test (`enabled::transport::`) | Observed, past the harness's success | Mutation, and how it failed |
|---|---|---|
| `the_handoff_failure_tag_fails_its_attempt_past_the_harness_success` | Parks on `Refusal::Handoff`, no call row, one harness start, no server started; `inspect`'s last-seat note agrees | Disposition judgment removed (`Some(Disposition::Failed(_)) => None` in `fold.rs`): `left: ("completed", "done", None)` |
| `the_exec_failure_tag_fails_its_attempt_past_the_harness_success` | Parks on `Refusal::Exec`, as above | The same removal; and status propagation (`Status::ExecFailed => Refusal::Handoff`): `left: ("awaiting_operator", "work", Some("MCP server protocol is invalid"))` |
| `status_eof_then_a_failed_initialization_keeps_the_protocols_cause` | Parks on `Failure::Protocol` | The disposition removal: `completed` |
| `a_zero_call_version_failure_is_not_hidden_by_the_harness` | Parks on `Failure::Version` | The disposition removal: `completed` |
| `a_closed_that_cannot_persist_reads_as_the_lifecycle_refusal_and_no_invented_cause` | Broker SIGKILLed mid-call: "broker ledger lifecycle is invalid" | `invalid` dropped from `folded`'s refusal: `left: ("completed", None)` |
| `a_restarted_engine_settles_the_faulted_attempt_indeterminate_and_executes_nothing_again` | Engine SIGKILLed with its harness alive: reservation still stands; `resume` parks "indeterminate: engine restarted while the attempt was in flight…", no call row, one harness start, the reservation standing after it; a second `resume` rules the same and starts nothing | The restarted judgment made `EffectFailed` in `fold::restarted`: `left: ("awaiting_operator", None)`, `right: (…, Some("engine restarted…"))` |

Settlement ordering is bound separately. With the `Cleanup::Settled | unproven` guard
removed from `fold_with`, `engine::broker::fold::tests::routes::a_live_half_record_is_not_read_while_any_driver_of_its_attempt_is_unresolved`
and `every_member_is_witnessed_before_any_member_is_read` fail with evidence read
early. The 0/1/4,096-call controls in `fold::tests` stay as they were. Each mutation
was applied, run and restored; `git status` showed those production files clean after.

## Task 46.1, as far as it went

| Test (`enabled::matrix::`) | Observed | Mutation, and how it failed |
|---|---|---|
| `a_multiply_linked_program_refuses_with_no_lookup_or_start` | `Refusal::Linked`, zero store reads (inotify), server never started, no call row | Link check neutered in `sources.rs`: `left: ("completed", None)` |
| `a_store_in_the_worktree_refuses_with_no_lookup_or_start` | `Refusal::StoreReachable`, zero reads, no start (F2) | Reach check dropped in `sources/store.rs`: `left: ("completed", None)` |
| `a_home_or_tmpdir_binding_refuses_the_launch_with_no_lookup_or_child` | `Refusal::StartupInputs` for each name, zero reads, zero harness starts (F3) | `fixed_key` made false: `left: "broker ledger is missing for this attempt"` |
| `a_normal_binding_and_private_directories_reach_the_server` | Server's env names are MB4's eight and `DOCS_TOKEN`; `/runtime/home`; both directories empty; token masked in the relay | TMPDIR's fixed value changed: `left: "awaiting_operator"`. It refused at box establishment, an earlier cause |
| `a_wanted_grant_is_served_beside_the_hands_with_native_power_off` | Whole set is `brokkr` and `cap-library-docs`; hands `network:false`; `--tools ""`, `--disallowedTools WebFetch,WebSearch` | `Holding::native`'s empty-restriction arm returning a native pair: `left: ["brokkr"]`, `right: ["brokkr", "cap-library-docs"]` |
| `a_fatal_protocol_failure_after_a_call_is_not_hidden_by_the_harness` | `Failure::Protocol`, one `failed` row, no digest | `rows.push` removed from `folded`: `left: []` |
| `the_call_past_the_ledgers_limit_fails_its_attempt_with_its_rows_kept` | `Failure::Limit`, exactly 4,096 rows | `CALLS_MAX` 4095: `left: 4095`. 8192 does not compile, by the ledger's own const assertion |
| `a_scalar_secret_is_refused_and_no_unsafe_body_is_retained` | `Failure::Masking`, one `failed` row, no digest, no `.response` stage, value in nothing left or heard | Two layers, so both removed together (`output::framed`'s guard and the ledger's in `ledger.rs`): `left: ("completed", None)`, `right: ("awaiting_operator", Some("MCP response cannot be safely masked"))` |
| `a_boxed_sibling_cannot_alter_the_attempts_evidence` | A boxed review striking at the first run's plan, ledger and directory: not reached, bytes unchanged | `/var/tmp` bound writable into every workspace box, unseen by any reach: panics "kept apart" at the strike |

Healthy control: `enabled::a_seat_holding_an_mcp_grant_with_boxed_hands_dispatches_through_the_real_broker_and_completes`
ends `completed`, broker exit 0, `resolved` delivered, one `succeeded` row whose
digest is the retained object's. `rows.push` removed: `left: []`.

The second visit added `enabled/reach.rs`, `enabled/sites.rs` and
`enabled/lifetime.rs`, all driven through `brokkr run`. Each mutation below was
applied, run and restored, and the suite then passed whole.

| Test | Observed | Mutation, and how it failed |
|---|---|---|
| `reach::a_writable_bind_at_above_or_below_the_package_reaches_it` | Each direction: "dispatch refused:" naming the bind, zero reads, harness starts and server starts. Replaces the first visit's `matrix::a_writer_reaching…` | The lease's `writes(…)` check made false in `sources.rs`: `left` holds "MCP server launch resolves inside seat-writable reach", the kept cause beneath it (F6) |
| `reach::a_read_only_or_overlay_bind_over_the_package_overlaps_the_box` | `Refusal::BindOverlapsReach` for `ro` and `overlay`, at, above and below; no reads, no server | Two guards. `namespace.rs`'s and `launcher.rs`'s removal left it passing. `namespace.rs`'s and the observer's `(false, true, _)` arm in `sources.rs` removed together: `left: ("completed", None)` |
| `reach::a_launch_resolved_through_a_link_in_reach_…` and `a_program_in_the_worktree_…` | The lease's cause naming the link and the program; no reads, no starts | The same lease removal: as the first row |
| `reach::a_singleton_system_entry_is_served_beside_the_systems_hard_links` | `/bin/sh`'s canonical path with its script as an argument: completes, one call, broker exit 0 | `ServerProgram::resolve` refusing `/usr`: `left` holds "program tree cannot be resolved" |
| `reach::an_empty_or_missing_store_starts_no_server` | Both: "secret 'DOCS_TOKEN' is not in the store at …", no server, no row | `store::bind_names` skipping an absent name: `left` holds "MCP server secret environment could not be delivered" |
| `reach::a_local_route_is_isolated_a_contracted_one_shared_and_the_hands_stay_off_the_network` | The server's own `/proc/$$/ns/net`: `local` differs from the host's and holds only `lo`; `contracted` is the host's; hands `network:false` in both | `binding::network` sharing `Local`: `left: true`, `right: false` with the host's interfaces listed |
| `reach::a_declared_loading_binding_reaches_the_server` | The server's names are MB4's and `PYTHONPATH` | `secret::denylisted` naming `PYTHONPATH`: FAILED |
| `reach::a_real_calls_retained_response_opens_through_inspect_masked` | `inspect --capability-call --json`: `[secret:DOCS_TOKEN]` in the response, state `succeeded`, digest of those exact bytes | Text masking removed in `output.rs`: parks "MCP response cannot be safely masked". The guard refuses the unmasked body, so this proves masking and not the digest |
| `sites::a_panel_member_is_served_beside_a_sibling_that_holds_nothing`, `a_sequence_step_is_served_and_the_next_step_runs_on`, `a_fallback_link_is_served_by_a_plan_of_its_own`, `a_cold_retry_is_served_afresh_and_keeps_only_its_own_call` | Each completes with one call row. The fallback's first adapter cannot spawn and the harness runs once on `claude-opus-5-5`; the retry mints two inventories | `Engine::slots` provisioning nothing: each parks "the site's serving candidate brokers the MCP capabilities ['library-docs'], but this attempt provisioned plans for []" |
| `sites::more_retaining_members_than_the_budget_shares_refuse_before_anything_starts` | 33 retaining members: "capability evidence budget is exhausted", no reads, no harness | The floor's check off in `share`: `left` holds the survey's deadline cause |
| `sites::a_gate_holds_by_its_class_and_named_office_only` | GP1's five rows, `requires` and `wants`, each through `brokkr run`: exact refusal with no harness; or the exact notice in `inspect` with no broker; or served | Each `gates::check` arm removed in its own run: the egress row and the writes row fail on their exact text |
| `lifetime::a_writer_arriving_beside_a_serving_broker_…`, `a_server_arriving_beside_a_running_writer_…`, `a_server_and_a_writer_started_together_…`, `a_killed_engines_reservation_refuses_a_later_writer_through_an_uncertain_settlement` | Two worktrees under one HOME, in both start orders, started together, and past SIGKILL and a restart's indeterminate settlement: the reaching side refuses naming the package and its seat never starts; where the server arrives second, zero reads and zero harness starts as well | The lease removal: each fails, the writer completing beside the server. The simultaneous case fails with both runs `completed`. That removal drops the comparison, so it says nothing about the lock's atomicity alone, which stays at the admission tests' seam |
| `lifetime::a_disjoint_writer_runs_beside_a_serving_broker_and_both_complete` | Both complete | The lease's check made true: panics waiting for the broker to serve |
| `lifetime::a_killed_engines_run_released_by_the_operator_lets_a_fresh_run_be_served` | After restart, a new run refuses naming the first; `operator release` admits a fresh run served with one call | `exclusion::leased` admitting: panics "held by the first run" |

Two proofs were tried and dropped, as neither could be shown to bind. A harness dying
with its server holding initialization ended its broker within 5 s, but that test still
passed with `lifetime.rs`'s parent check removed, with `tree::kill_group` removed as
well, and with `Attempt::kill` removed as well, so some further route ends the broker
that I did not identify. A host `LD_LIBRARY_PATH` absent from the server stayed absent
with the box's `--clearenv` and the bootstrap's filter both removed, so something
before them clears it, which I also did not identify. Both remain owed.

Owed by 46.1: a user-installed package whose program is a native binary beside the
singleton (the fixture's is a shebang script); cancellation at every startup window
through dispatch, where `capability_broker`'s `a_cancelled_attempt_ends_a_box_held_at_its_entry`,
`a_store_read_held_when_the_attempt_is_cancelled_is_refused_at_once`,
`an_attempt_cancelled_during_initialization_or_a_call_settles_the_server_and_its_child`
and `a_cancelled_attempt_ends_a_blocked_observation_with_nothing_left_running` hold
them at the public broker today; a server's private tmpfs against a second server,
through dispatch; the host loader environment above; the bind-alias row through
dispatch; and quota beyond the door (spent shares) through dispatch.

## Ruling 21

The second case is the healthy control above, under F1's condition. The first and
fourth now go through real dispatch:

| Test (`enabled::dispatch::`) | Observed | Mutation, and how it failed |
|---|---|---|
| `a_fresh_scaffolds_first_seat_dispatches_with_nothing_at_stake` | `brokkr init .` then `brokkr run`: harness started, no "dispatch refused" | `staked` early return removed from `exclusion::compared`: fails |
| `each_shipped_recipes_first_seat_dispatches_with_nothing_at_stake` | All 19 shipped recipes, unconfined, under a namespace realm: no "dispatch refused", each started a harness or moved past its start | The same removal: fails |
| `a_seat_without_hands_holding_a_native_grant_dispatches_and_retains_nothing` | `claude-native-fetch` on a hands-less seat, hands-less review: completes, no call row, no broker in its config | The same removal: fails |

The third case stands by MB2's compile refusal, as ruled.

## U9b1a's owed items

Ruling 4's wiring is now bound. `observation::observed_under` takes the setting;
`dispatch_hands_provisioning_a_host_only_where_the_setting_reads_off` pins `Ok` for
`0\n` and `Unprovisioned::Terminal` for `1\n`, not-found and permission-denied, with
admission keeping its HOME. The controller's stand-in `.or(Ok(b"0".to_vec()))`:
`left: None`, `right: Some(Terminal)`, on this host's own setting of 0.

| Named test | Mutation | Failure |
|---|---|---|
| `past_the_lifted_fence_an_mcp_holding_is_the_brokers_and_its_native_namesake_stays_off` | The empty-restriction arm of `Holding::native` returning a native pair | FAILED |
| `past_the_lifted_fence_a_nonempty_mcp_restriction_refuses_or_drops_with_the_deferred_cause` | `false => Ok(None)` | FAILED |
| `a_host_whose_legacy_tiocsti_is_not_switched_off_serves_no_mcp_server` | `Ok(_) => Ok(self)` | `left` holds `Ok(Host…)` for `1\n` |
| `the_terminal_refusal_reaches_only_a_brokered_attempt_and_before_any_survey` | `host?` moved above `slots.is_empty()` | Panics at its first `assert!`, line 862 |
| `an_inline_site_is_served_with_its_own_adapters_authentication_names` | `auth: &[]` in `inline_capabilities` | `left` holds the fence's words, not the collision |
| `either_fence_refuses_unboxed_writers_at_stake_and_root_reaching_boxes_on_every_run` | `if false && !staked(comparison)` | FAILED |

L1: `Unknown::Homeless` is the case's own cause, in `records_kept`.
`a_boxed_writer_is_refused_by_its_own_cause_where_the_host_names_no_home` binds it
through `Engine::admit` with `observed.home` None, its text pinned; with the old
`Unmounted`, that test and `an_unopened_roots_path_keeps_boxed_writers_on_every_host`
fail. `Unadmitted::Evidence` took `#[from]` to pay for the variant's lines.

L3: `exclusion::identity` is the one metadata-to-(dev, ino) reading.

L2 is recorded, from a scratch variant. As committed,
`a_bind_alias_of_protected_evidence_is_refused` skips in this seat ("unshare: write
failed /proc/self/uid_map: Operation not permitted"), and under `bwrap --cap-add ALL`
its `mount --bind` is denied. So for the record only, and not committed, bwrap itself
made the bind (`--unshare-user --dev-bind / / --bind <kept>/.forge <alias>`) and the
assertions before the bind and the `mount` step were left out. The namespace half then
ran and passed. Removing `covers_spelled`'s `above` arm panicked at "by the bind's
identity"; removing `covers`'s `identical` panicked at "from below". The committed
test's own namespace run on the host stays pending.

The stale comments named were checked: `Unbound::Mcp` and the MB4 comment in
`capabilities/tests/binding.rs` already say where the fence stands, so neither moved.

## SD4, x86_64 warm samples

Sources: 205,082 entries across 2 mounts, unpruned, ordinary hard links intact,
sealed and observed writer uid 1000, seen on the diagnostic run; ACL facts were left
to the observer and not recorded. Taken with `sd4_warm_samples`, under a load average near
17. "Engine" is `brokkr run`'s start to the harness's start: compile, admission and
the provisioning survey. "Broker" is the harness's start to the broker's first
answer: reobservation, box, handoff and initialization. Neither is a pure observation.

| Sample | Engine ms | Broker ms | Total ms |
|---|---:|---:|---:|
| 1 | 5,089 | 1,995 | 7,084 |
| 2 | 5,066 | 2,092 | 7,158 |
| 3 | 3,885 | 2,222 | 6,108 |
| 4 | 6,376 | 15,922 | 22,299 |
| 5 | 16,483 | 2,142 | 18,625 |

Samples 1 to 3 are within the budgets. Sample 4's total exceeds the 20 s total, and
samples 4 and 5 each hold a stage over 10 s, on a contended host. That does not prove
a defect or a pass: the samples need retaking on a quiet host, each observation timed
alone. One run during mutation testing, with a parallel build, also parked on
"admission did not complete before its startup deadline".

The second visit's five, the same way, under a load average near 7:

| Sample | Engine ms | Broker ms | Total ms |
|---|---:|---:|---:|
| 1 | 3,541 | 1,632 | 5,174 |
| 2 | 3,493 | 1,598 | 5,092 |
| 3 | 3,468 | 1,646 | 5,114 |
| 4 | 3,474 | 1,603 | 5,078 |
| 5 | 3,567 | 1,651 | 5,219 |

All five are within the budgets, so the first set's outliers track contention. These
are still whole stages, not each observation timed alone, and the host was not quiet.

Pending, not taken here: the fully cold x86_64 sample (operator drops the caches, the
controller samples, ruling 3); every aarch64 sample; each complete observation timed
apart from compile, box and handoff; FD high-water; peak metadata memory.

## What was run

Formatting and clippy with `-D warnings` over the workspace were clean, as were
`quality/ratchet.sh files` and `clones` (two clones folded, by sharing
`writer_lease::POLICY` and adding `World::completes`). `cargo test -p brokkr-runtime
--locked`: `--lib` 1447 passed, `--test it` 144 passed. `cargo test -p brokkr-cli
--locked --test it` once: 844 passed, 2 failed, 4 ignored, in 483 s. The two were
`enabled::matrix::a_store_in_the_worktree…` and the existing
`capability_broker::serving::the_brokers_standard_streams_keep_their_status_flags_whether_served_or_failed`
("filesystem identity is not protected"), and both passed when rerun together. A
second full pass on the final tree had none failed: 846 passed, 4 ignored, in 310 s.
This suite adds survey load to that shared pool, so the second failure may be this
visit's doing; treat it as an open risk, as a loaded CI could see it too. Typos and
strict openspec validation (36 of 36) were clean, and `recipes/self` compiled with
"48 valuations over 11 groups, 0 unruled".

Not run by this seat: exact coverage and the crap ratchet (the controller's, and
CI's); macOS (`enabled.rs` is `cfg(target_os = "linux")`, and this host cannot compile
Darwin); the CLI's `--lib`, the protocol's suite and the heap suites, as the production
files of those crates are unchanged after the mutations were restored.

Production files changed: `engine/broker.rs` (F3), and its children `observation.rs`,
`exclusion.rs`, `sources.rs`, `admission.rs`. Test files beyond the new ones gained
only `pub(crate)` on shared helpers: `capability_broker.rs`, its `readiness.rs` and
`serving.rs`, `writer_lease.rs`. No test moved. `quality/file-lines.txt` gains the
four new files and the changed counts.

The second visit's runs, on the tree it commits: `cargo test -p brokkr-cli --locked
--test it -- --skip enabled::` 825 passed, 3 ignored, in 303 s; `enabled::` 41 passed,
1 ignored (the sampler), in 271 s; one whole pass together hit the 560 s bound (F7).
No production file moved from part one, so the runtime suites were not repeated, and
the protocol's and the CLI's `--lib` remain unrun, as before. No test moved.
`enabled/matrix::a_writer_reaching_the_servers_package_starts_nothing` was replaced by
`reach::a_writable_bind…`, which covers its case among the three directions.

## Owed, and the ruling this needs

Done: this suite; 42.2's real faults, its restart removal, and its settlement and
disposition removals; ruling 21's first and fourth cases, and the second under F1;
U9b1a's named mutations, L1, L2 (scratch variant), L3 and the terminal seam; F3's fix;
GP1's table, the sites, reach, stores, networks and retention through dispatch; MB3's
two-worktree matrix; ten warm samples.

42.2 stays unticked on F5's invalid-status case, for the seam ruling. 46.1 stays
unticked on F1 and on its "Owed by 46.1" list above. 46.2 stays unticked on F1, F5's
frame matrix, its pending host evidence (the fully cold x86_64 sample, every aarch64
sample, each observation timed alone, FD high-water, peak metadata memory) and the
exact descriptor fixture through dispatch.

**The wall is F1.** No visit can close this unit without it: ruling 21's second case
completes only with the engine under `setpriv --no-new-privs`, and every fix lies
outside the row. Recommended: rule option (a) for F1 with `brokkr driver` admitted to
the row, accept the seams for F5, and amend MB3 for F6. The remaining 46.1 list is
then one narrow visit.
