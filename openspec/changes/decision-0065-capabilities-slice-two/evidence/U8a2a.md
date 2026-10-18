# U8a2a evidence: hold the writer lease and durable reservations at dispatch

U8a2a was built on branch `s2/U8a2a`, cut from main at `7ed4acc6`. Run 1
(`0065-slice-two-unit-u8a2a-see-th-7599d54e`) built the admission operation
and was held by review (REVIEW-SECURITY-HOLD, six HIGH, five MEDIUM, four
LOW). Run 2 (`0065-slice-two-unit-u8a2a-see-th-ac41bc61`) rebuilt it on run
1's staged work and the operator's ruling of 2026-10-18, which design.md
carries under D6, U8a2a and U9b1. Everything below was observed on the tree
committed with this file. U8a2a shares tasks 38.3 and 38.4 with U8a2b, so
neither is ticked: this file records U8a2a's part. U8a2b owns the
program-source and system-support alias matrix and the mapped identities
and privileges source write exclusion needs.

Ruling: operator ruling 2026-10-18 (U8a2a builds the complete coordination
comparison now; `McpFence::current()` is the switch's one home).

## Scope

| File | Change |
| --- | --- |
| `crates/brokkr-runtime/src/engine.rs` (row; frozen at 3,522) | The engine holds the lease of the attempt in flight (`held`). `run_driver` hands the door that lease and witnesses each report's #403 cleanup. Size-neutral: a two-line comment became one. |
| `crates/brokkr-runtime/src/engine/broker.rs` (row) | Declares the admission's private children. `provision` takes the attempt's start from dispatch, so admission and provisioning share one startup deadline (M8). `protected` syncs every directory that gains a name before going on, through `protected_with` and a `Synced` seam (M9). 758 → 791 lines. |
| `crates/brokkr-runtime/src/engine/sequence.rs` (row) | No edit: every step spawns through `run_driver` or `run_panel`, whose doors recheck and whose reports are witnessed. |
| `engine/broker/admission.rs` (new child of `broker.rs`) | Records, refusals, `Engine::admit`, the claim, the operation, the door's `recheck`, `release`, the lock and sealing. 782 → 603 lines. |
| `engine/broker/inventory.rs` (new child, 211 lines) | The inventory's traversal under the lock, its `Budget` (D5's entry bound and the shared deadline) (H6). |
| `engine/broker/exclusion.rs` (new child, 342 lines) | The coordination comparison, the fence switch, identity chains and the Linux mount-table narrowing (H1, H2, H4). |
| `engine/broker/historical.rs` (new child, 100 lines) | Historical roots' durable records and the verification of their artifacts (H3, M10). |
| `engine/broker/settlement.rs` (new child, 154 lines) | `Engine::settle`, `witnessed`, and recovery from durable facts (H5, M7). |
| `engine/dispatch.rs`, `engine/panel.rs` (existing children of `engine.rs`) | `execute` takes `began` once for admission and provisioning, and settles through the engine's own lease. `run_panel` witnesses each member's cleanup. |

`quality/file-lines.txt` gains a row for each new file. The public API is
unchanged (`quality/ratchet.sh api`: "public API holds"). Every new module
is private to `broker`; `settlement` is `pub(super)` for the engine's two
witness calls. No new trait, flag, environment knob, authored key, daemon
or suppression.

## The mechanism as built

The coordination root is the protected `HOME/.local/state/brokkr/capabilities/`.
Its lock, `admission.lock`, must be this user's own regular file of one
link. It is held by `flock` while one admission or release reads and writes
the inventory, and never while anything runs. Admission begins at the
attempt's start (`began`), and its deadline, `startup(began, bound)`, is the
one provisioning then spends: provisioning is handed `began`, not a second
start.

Under the lock the inventory is read within D5's entry bound and the
deadline. The root holds the lock and one directory per worktree key. A
worktree's directory holds historical records (`historical-<sha256>.json`)
and one directory per run. A run's directory holds `reservation.json` and
the directory of every attempt the reservation names, standing or settled.
A record staged beside its rename target (`*.new`) is passed over, because
a writer starts only after its record was renamed in and the directory
synced. Every other entry, and every entry that cannot be read or entered,
refuses with MB3's identity cause.

A reservation names the worktree's key, the run, the canonical worktree and
artifact root, the journal its settlement facts live in, each standing
attempt with its reserved writer reach, and each settled attempt. It is
never removed. Recovery then reads, read-only and within the deadline, the
journal each reservation names. An attempt whose terminal event #403 writes
only once the tree is proven over (a success or a failure) is moved to
settled under the lock. An indeterminate end, a missing end or an
unreadable journal keeps the attempt standing.

The comparison is ordered. Another run of the same worktree with an
attempt standing refuses with CR2's cause; a run is its worktree key and
its id. Then the guarded roots are gathered: the coordination root, the
incoming artifact root where it protects, every standing protecting root
and every historical root. While the fence stands and only the coordination
root is guarded, nothing more is compared. Otherwise, or once the fence is
lifted, every artifact of every guarded artifact root is verified, and
every participant's every writer is judged: the incoming attempt's and
every standing attempt's, the same run's earlier ones included. A writer
outside a workspace box refuses. A boxed writer whose writable root is at,
above or below a guarded root refuses, compared both by canonical path and
by device and inode against each other's ancestor chains. Its own run's
artifact root is exempt, because its box overlays that root read-only
(U8a). Where any boxed writer participates, each guarded root that stands
must be shown once by the mount table. Each refusal takes CR2's cause.

The incoming reservation is sealed (staged, synced, renamed in, directory
synced) and an artifact root that protects evidence for the first time is
recorded the same way, both before the lock is released. The door rechecks
that the reservation still names the attempt standing and that a protected
root still has the identity admitted. Settlement releases the lease when
the attempt ended without error, its terminal event is journaled, and
every driver it ran reported `Cleanup::Settled`. Release moves the attempt
to settled under the lock.

## Run 2: each finding, its fix, its proof and its removal control

Test paths are relative to `crates/brokkr-runtime/src/engine/`: `adm` is
`broker/admission/tests.rs`, `coord` its child `tests/coordination.rs`, and
`att` is `capability_tests/provision_tests/brokered_tests/writers_tests.rs`.
Every removal control below compiled, failed the named tests, and was
restored; the suites then passed (see Gates).

| Finding | Fix | Proof (test, decisive assertion) | Removal control and observed failure |
| --- | --- | --- | --- |
| H1 | `exclusion::compared` gathers every guarded root and judges every participant in both directions; `everywhere()` reads `McpFence::current()` as the one switch. A writer outside a box, or one that can reach a guarded root, refuses with CR2's cause. | `coord::a_lifted_fence_refuses_unprotectable_writers_with_nothing_at_stake` (unboxed → `Unboxed`, HOME bind → `Reaches` coordination root, confined admitted); `coord::a_standing_fence_refuses_them_only_where_evidence_is_at_stake` (admitted with nothing at stake, refused where the incoming, a standing or only a historical root protects); `adm::every_steps_door_rechecks_the_lease_its_attempt_holds` (lifted: the driver that would delete its reservation is refused `Unboxed{work:draft}` before it spawns, no checkpoint, no `ran`). | M-C `McpFence::Lifted => false`: the lifted-fence test and the door test failed. M-D stakes ignored (`roots.len() >= 1`): eleven tests failed, among them the standing-fence test and `att`'s `a_retaining_step_beside_an_unboxed_sibling_starts_nothing`. |
| H2 | The lease exemption is the lease alone (`leased`, keyed on repo and run); every standing attempt, the run's own earlier ones included, is a participant. | `coord::a_runs_earlier_unsettled_attempt_is_compared_with_its_next` (`Unboxed{work, run-a}`); `coord::a_run_id_reused_in_another_worktree_is_compared_not_exempted` (`Reaches{work, same, other's root}`, then a claim on the other worktree under that run id holds no lease). | M-E own run left out of participants: the earlier-attempt test failed. M-F run id alone (run 1's rule): both tests failed. |
| H3 | `historical::recorded` seals `historical-<sha256>.json` in the worktree's directory, under the lock, the first time a root protects; release never touches it; every admission reads every record host-wide. | `coord::settled_evidence_stays_protected_from_a_later_worktree` (the record exists after release; a later writer of another worktree → `Reaches`); `att::settled_evidence_is_kept_from_binds_ancestors_aliases_and_hard_links` (protected bytes unchanged). | M-B2 record never written: five tests failed; `att`'s proofs failed on "the protected bytes" with `forged` in place of `historical`. That proof was then two tests, folded into one to remove a clone; the folded test keeps both as its first cases. |
| H4 | `covers` compares canonical paths, then device and inode of the writable root and the guarded root against each other's ancestor chains. `shown_once` reads `/proc/self/mountinfo` and refuses a guarded root whose device shows a part holding it, or inside it, at another mount; off Linux it refuses. Hard links are refused by M10's single-link rule. No protocol code is copied and no protocol item made public. | `coord::a_writer_that_can_reach_protected_evidence_is_refused` (symlink spelling of the writable root); `coord::a_root_spelled_through_a_symlink_is_compared_by_identity` (a protected root spelled through a symlink); `coord::the_mount_table_shows_a_root_once_or_proves_nothing` (fixture tables: bind of a parent, bind of a part, namespace roots of the same device, unnamed mount, unparsable table); `coord::a_bind_alias_of_protected_evidence_is_refused` (real bind in a user and mount namespace: identity → `Reaches`, holder of the alias → `Shown`); `coord::historical_artifacts_are_verified_against_their_names` and `att::settled_evidence_is_kept_from_binds_ancestors_aliases_and_hard_links` (hard link → `Corrupt`). | M-G identity removed: `a_root_spelled_through_a_symlink_is_compared_by_identity` failed. M-G2 identity and resolution removed: it and the symlink case failed. M-H `once` always true: the mount-table test failed. M-J single-link rule removed: the hard-link cases failed, `att` on "the protected bytes" (`forged`). The bind-alias proof skips in this seat (`unshare: write failed /proc/self/uid_map`); it is the controller's host check. |
| H5 (closes L15) | Release needs positive proof: `witnessed` records each report's typed `Cleanup` from `run_driver` and `run_panel`; `settle` releases only when none was unresolved and the attempt's terminal event is journaled, read by `EventEnvelope::attempt_id`, never a payload key. Recovery certifies only a success or failure. Settlement reads facts the moved-HEAD rewrite cannot remove, so the rewrite is left as it is. | `adm::a_lease_is_released_only_on_proof_its_processes_settled` (error, no end, unresolved then indeterminate, unresolved then success: standing; settled then failure, settled then indeterminate, no driver then indeterminate: settled); `adm::a_moved_head_gate_with_unresolved_cleanup_keeps_its_reservation` (real driver, standard output held open outside its tree, `GATE-MOVED-HEAD` end with no `cleanup` field, reservation standing); `adm::a_panel_member_with_unresolved_cleanup_keeps_its_reservation`. | M-K proof ignores `unproven`: all three failed. M-L `run_driver`'s witness removed: the moved-HEAD test failed. M-M `run_panel`'s witness removed: the panel test failed. |
| H6 | Unknown inventory refuses with MB3's identity cause: anything but the lock and worktree keys in the root, anything but records, staged records and run directories in a worktree's, anything but the reservation, its staged copy and the attempts it names in a run's, and any entry that cannot be read or entered. | `adm::an_unknown_participant_refuses_before_anything_is_sealed` (stray file at each level, a link, a directory no key names, an attempt directory not named, a directory mode 000, an extra field, a reservation naming another run); `adm::a_name_that_is_not_utf8_is_an_unknown_participant` (Linux); `adm::a_writer_starts_only_after_its_reservation_is_renamed_in` (nothing sealed while compared, renamed on return, a staged-only record passed over). | M-O staged record not passed over: both tests failed. M-O2 any directory taken for an attempt: the unknown-participant test failed. |
| M7 | `settlement::recovered` releases, under the lock, each attempt its sealed journal certifies; an unreadable journal, an indeterminate end or no end keeps it. | `coord::a_crash_after_a_certified_end_is_recovered_at_the_next_admission`; `coord::an_uncertain_survivor_keeps_its_reservation_and_refuses` (three survivors, each `Held` and its reservation byte for byte). | M-N recovery removed: the crash test failed. M-B1 every attempt certified: ten tests failed, among them the survivor test and `att::an_already_running_writer_holds_off_a_retaining_member`. |
| M8 | One deadline from `began`; the inventory, recovery, the comparison, the verification and sealing each check it. | `adm::an_observation_blocked_past_the_deadline_seals_nothing` (`Expired`, nothing sealed); `adm::the_inventory_reads_within_its_entry_bound`; `coord::every_step_under_the_lock_spends_the_shared_deadline` (comparison, recovery and verification each `Expired`, verification `Bounds` at one entry short). | M-P seal's check removed: the blocked-observation test failed. M-U comparison's check, M-V recovery's check and M-W verification's spend removed, each alone: the shared-deadline test failed at its own assertion. |
| M9 | `protected` syncs each directory that gained a name before going on. | `adm::every_directory_admission_makes_is_synced_in_its_parent` (the synced identities are exactly HOME, `.local`, `state`, `brokkr`, the root and the worktree's directory; none on a second walk). | M-Q sync removed: the test failed. |
| M10 | Every consulted `sha256/<hex>` entry is opened without following a link, must be one regular file of one link, and must digest to its name, within D5's bound and the deadline; else `Corrupt` (CR2). | `coord::historical_artifacts_are_verified_against_their_names` (valid admits; differing bytes, a non-digest name, a symlink and a hard link refuse); `adm::historical_evidence_refuses_a_panel_member_that_cannot_be_kept_from_it` now writes `retained` under its real sha256. | M-I verification skipped: the verification test and `att`'s hard-link proof failed. M-J as above. |
| M11 | Dispatch-level proofs with a real retaining panel member (the Claude fixture's brokered `library-docs`, fence lifted). | `att::a_zero_grant_attacker_beside_a_retaining_member_never_starts`; `att::a_boxed_exec_attacker_that_can_reach_the_coordination_root_never_starts`; `att::an_already_running_writer_holds_off_a_retaining_member` (refused while a real engine's unboxed driver runs on another worktree; once it settled, the panel runs beside a member writing its own worktree); `att::settled_evidence_is_kept_from_binds_ancestors_aliases_and_hard_links`. Each asserts the protected bytes first, then the exact refusal, that the keeper never spawned and that nothing was sealed or provisioned. | M-A atomic admission's comparison removed: twenty-four tests failed; the zero-grant proof and the bind proof failed on "the protected bytes" (`forged`). M-B1 and M-B2 (retention through settlement removed) failed the already-running and settled-evidence proofs. |
| L12 | Recorded, not changed: the lease is per attempt, joined into one run reservation, so a run's later attempt shares its earlier ones' lease; nothing between them is unleased. | `adm::a_runs_later_attempt_joins_its_reservation_beside_an_unsettled_one`. | M-R a reservation with nothing standing still held (precise form): three tests failed. |
| L13 | The coordination home is injected through the house's stand-in seam (`tests::stood_in`, as provisioning's `brokered_tests::stood_in`); production reads `Host::of` and the integration engines run with HOME set to a fixture. Every admission fixture is a fresh canonical 0700 directory in canonical TMPDIR; where TMPDIR and its ancestors do not protect a HOME, the test skips with that reason (or fails under `BROKKR_REQUIRE_BOUNDARY_EVIDENCE`), never falling back to `/tmp`. The simultaneous-admission test hands its sender to the admitting thread, so a failed first admission ends the wait. | `adm::the_hosts_home_is_read_only_where_nothing_stood_in` reaches the production read; the focused suites' runs are under Gates. | — |
| L14, INFO | Recorded: `ENTRIES_MAX` repeats the protocol's private `LIMITS.entries`; refusal texts name host paths, as U8a's do. | — | — |

Run 1's reasoning that retention stays fenced, and so that unboxed writers
may run wherever nothing protects evidence, is replaced by the ruling: while
the fence stands that holds only where nothing is at stake (no incoming,
standing or historical protection), and once U9b1 lifts it every run is
compared.

## Tests changed beside them

| Test | Why |
| --- | --- |
| `tests/capability_broker_launch.rs` (71 → 81) and `tests/support/writers.rs` | A settled reservation stays with no attempt standing; `Coordination::standing` reads the standing attempts, replacing the root listing. |
| `capability_tests/provision_tests/brokered_tests.rs` (789 → 798) | `Unboxed` names its run; `Claude::executed` split so the attack proofs share `executed_as_is`; `mod writers_tests;`. |
| `engine/tests.rs` (frozen, 6,565) | `git_commit` made `pub(super)` for the moved-HEAD proof. Size-neutral. |
| `broker/admission/tests.rs` (868 → 1,106), new child `tests/coordination.rs` (544) | Run 1's mechanism tests moved to the new records and refusals; the comparison tests moved to the child. |

## Gates

Formatting, workspace clippy with `-D warnings`, `git diff --check`,
`typos --hidden` and `openspec validate --all --strict` (26 passed) were
clean at the tested tree. The `recipes/self` compile succeeded with its
policy sweep at 0 unruled. `quality/ratchet.sh` printed "file size holds",
"duplication holds" and "public API holds". The long-function listing for
touched files matched the baseline rows exactly.

Every brokkr-runtime binary passed: lib 1,085, `it` 136 (witness pins and
budgets included, so no pin moved), `capability_launch` 76,
`operated_repo` 1, `queued_launch` 6. For brokkr-cli the lib (884), `it`
outside `capability_broker` (615), `driver_conformance` (27),
`transcript_surfaces` (14) and the three heap tests passed. In the
`capability_broker` run (311 s, 107 tests) one test,
`cleanup::an_attempt_cancelled_during_initialization_or_a_call_settles_the_server_and_its_child`,
failed once under load and passed alone (6.45 s); that binary runs no
engine dispatch. brokkr-protocol (991 and its binaries), brokkr-core,
brokkr-view, brokkr-store and brokkr-bridge passed.

The focused suites (`engine::broker`, `brokered_tests::writers_tests`) ran
with TMPDIR unset: 44 passed, the bind-alias proof the one skip. With TMPDIR
set (through `cargo --config env.TMPDIR`) to a fresh 0700 directory in the
worktree's `.forge`, every fixture skipped with "TMPDIR … protects no HOME
(D6)" and none fell back to `/tmp`: this checkout's ancestors are
group-writable. A fresh 0700 TMPDIR under a protecting ancestor could not be
made from this seat (`/tmp` is outside its writable directories); that run
is pending for the controller.

Pending, and not claimed: the exact-coverage gate and the CRAP ratchet on
its LCOV, macOS and remote CI, and the host-run proofs. The host-run proofs
are `coord::a_bind_alias_of_protected_evidence_is_refused` (it needs a user
and mount namespace), and the fresh-TMPDIR run above.

## Limits and residuals

- **Historical records pin stakes host-wide.** Once any root has protected
  evidence on a host, every later admission there compares every writer,
  and a writer outside a box refuses, fence standing or not. The record
  outlives its root; a root that no longer stands is still compared by
  path. A test suite that dispatches with the host's HOME over existing
  evidence leaves such a record there.
- **Reservations accumulate.** A run's reservation and directory stay after
  every attempt settled, so attempt directories stay named; D5's
  1,000,000-entry bound eventually refuses with MB3's identity cause. No
  verb prunes them.
- **Indeterminate ends are recovered by no one but their engine.** A
  deadline kill proves the tree over to the engine that killed it, which
  releases; a crash between that end and the release leaves the attempt
  standing, as does any engine restart, until the operator removes it.
- **Unboxed writers.** Sites under `harness` or `open`, or without hands,
  refuse beside any evidence at stake now, and on every run once U9b1 lifts
  the fence, as the ruling names for U9b1. On macOS no writer is boxed and
  the mount table is never read, so every writer refuses where anything is
  at stake.
- **The mount check's own deadline check** runs after the comparison's
  per-writer checks and cannot be the first to see expiry
  deterministically; no test isolates it.
- **What U8a2b adds.** Writers carry their writable reach only; U8a2b adds
  the mapped identities and privileges for source write exclusion. A
  changed record makes earlier reservations unknown participants, so
  migration must quiesce.

## Run 3 (0065-slice-two-unit-u8a2a-see-th-88d31f37), first visit

The first visit of run 3, committed as `1e49e339` and reported `broken`,
is recorded as it was written below. Its second visit, after the section,
supersedes it wherever the two differ: the R1 source rule, the journal
reach rule and the door's fence rule were each rebuilt there.

This visit repaired a subset of the review's run-2 findings on the staged
tree and verified each repair locally; the run was reported `broken` because
the remaining items below could not be completed and proved to the recipe's
no-HIGH/no-MEDIUM bar within one headless visit. Test paths are relative to
`crates/brokkr-runtime/src/engine/`: `adm` is `broker/admission/tests.rs`,
`coord` its child `tests/coordination.rs`. Every removal control named ran
as a compiling mutation, failed the named test at the named assertion, and
was restored; the broker suite then passed (47 lib tests, from 44 at
arrival). The whole `brokkr-runtime` library (1,092) and the consolidated
`it` binary (136) passed, and `brokkr-runtime` clippy was clean under
`-D warnings`.

| Item | Fix | Proof (test, decisive assertion) | Removal control, observed failure |
| --- | --- | --- | --- |
| C1 | `admission::admit_with` branches at the top: a coordination root that cannot be protected or made refuses with MB3's identity cause only where the fence is lifted or the claim protects evidence; otherwise it returns `Ok(None)` and `admit_in` proceeds holding no lease (`unreserved`). No engine-authored journal marker is written, because a checkpoint before the driver is refused by the seat-record fence (decision 0034) and would lose the attempt; the evidence it took none is the durable absence of any reservation. | `adm::an_unprotectable_root_proceeds_unreserved_only_where_nothing_is_at_stake` (`admit` returns `Ok(None)` and makes no lock; a protecting claim and a lifted fence each give `Unknown::Root`); `engine::tests::a_selected_single_publishes_its_own_confinement_at_dispatch` now passes with TMPDIR at a group-writable directory. | The second match arm forced to `Err(Unknown::Root.into())`: the unprotectable-root test failed on its `Ok(None)` assertion. |
| HOME leak (part of R5) | `tests/two_engines_one_journal.rs` gives each child engine an explicit owner-only HOME beside its journal, so its admission resolves a coordination root under the temporary tree, never the operator's real HOME. | The `it` binary (which registers `two_engines_one_journal`, `capability_ledger`, `capability_broker_launch`) ran with HOME forced to an empty probe directory; all 136 passed and the probe directory held no `.local/state/brokkr` afterward. | — (isolation check, not a behaviour mutation). |
| R1 (sources) | `exclusion::replaceable`/`swappable`, run per writer inside `participated` (so under the fence rule), refuse a bind source, or an ancestor still inside a writable root, that is not itself a writable root: its owner could swap it for a symlink. `Reach::Boxed` now carries `readable` so every bind source is compared, and the refusal is `Conflict::Replaceable`. | `coord::a_replaceable_bind_source_is_refused_before_spawn` (a read-only source beneath the writable worktree → `Replaceable`; a disjoint source admitted). | `replaceable(..)?` changed to `.ok()`: the test admitted the replaceable source and failed. |
| R1 (door) | `admission::recheck` re-resolves every sealed writable root and bind source (`Held::reach`) and refuses `Conflict::Redirected` where one now resolves elsewhere. Applied unconditionally as a strictly-additive door guard (a redirected reach is never lawful); named here as a knowing, safe deviation from "the fence rule applies to both", since the attack it guards is already refused outright at admission by R1 (sources). | `adm::the_door_refuses_a_lease_whose_reach_was_redirected` (a disjoint source admitted, then swapped for a symlink → `Redirected`). | The redirect loop guarded by `if false`: the door admitted the redirected reach and the test failed. |
| R2 | `settlement::journal` refuses to read a reservation's journal where any participant's writer — the incoming attempt's or any standing one's — could reach the journal, its directory or a `-wal`/`-shm` sibling (`exclusion::reaches`), because an unkeyed hash chain is forgeable. Positive settlement (`Cleanup::Settled` on every report plus the journaled end) was already required; that is kept. | `coord::a_settled_end_in_a_reachable_journal_does_not_release_the_reservation` (a genuine settled end in a journal inside the writer's box keeps a1 standing → next run refused `Held`); contrast `coord::a_crash_after_a_certified_end_is_recovered_at_the_next_admission` (journal outside every reach → released). | The `reachable` guard guarded by `if false`: the forged end was trusted, a1 released, and the next run was admitted, failing the `Held` assertion. |
| R3 | `historical::digested` opens digests `O_NONBLOCK`, `fstat`s and admits regular files only, so a FIFO or socket refuses `Conflict::Corrupt` without blocking. `admission::seal_with` (a new house `_with` seam) rechecks the shared deadline after the final rename and withdraws a late record under the lock. | `coord::a_fifo_artifact_refuses_without_blocking` (`Corrupt`); `adm::a_seal_that_finishes_past_the_deadline_is_withdrawn` (`Unknown::Expired`, the record gone). | Seal's post-rename check guarded by `if false`: the late seal stood and the withdrawal test failed. The FIFO binding is the open itself: without `O_NONBLOCK` the open blocks for a writer and the test hangs (not run, to avoid the hang). |
| R4 | `exclusion::mounts` enforces D5's bound (now 65,536) as it reads the table, bailing at record 65,537 with `Shown::TooMany`, which `shown_once` maps to MB3's `Unknown::Bounds`; `once` returns a three-state `Shown`. | `coord::the_mount_table_is_bounded_as_it_is_read` (exactly the bound → `Shown::Once`; one over → `Shown::TooMany`); `coord::the_mount_table_shows_a_root_once_or_proves_nothing` reworked onto `Shown`. | The `== MOUNTS_MAX` guard guarded by `if false`: the over-bound table parsed whole and the bound test failed. |
| INFO | `coord` imports `Command` behind `cfg(target_os = "linux")`; `adm::the_door_refuses_a_lease_that_no_longer_stands` asserts the macOS refusal (`Conflict::Shown` at the coordination root) where a boxed protecting admission cannot pass the (absent) mount table; the between/seal seams are named `admit_with`/`seal_with`. | Compiles and runs on this Linux host; the macOS arm is `cfg`-gated and reasoned. | — (compile/correctness INFO). |

Run 2's integration-isolation claim is corrected: run 2 left `two_engines_one_journal` resolving the process HOME, which is why its child engines wrote reservations into the operator's real HOME on a protected host. Run 3 isolates that suite's HOME; the broad CLI-launch isolation (`support/engine.rs`) and the admission-core directory-handle injection remain (see below).

### Not completed this visit (why it is `broken`)

- **R5 core (test-isolation refactor).** Admission's core still takes
  `home: &Path` and rebuilds paths, rather than an injected owner-only
  coordination-root directory handle through a `_with` seam. So the
  `Host`-based admission tests still skip under a group-writable TMPDIR
  (`Host::new`'s skip and the `temp_dir` fallbacks are unchanged), rather
  than executing on an injected handle. The three-TMPDIR executed/skipped
  matrix is therefore not produced. CLI-launch HOME isolation in
  `support/engine.rs` is likewise not applied (only `two_engines` and the
  already-staged `driver_conformance` are isolated).
- **R6 (real namespace attack matrix).** The `writers_tests.rs` attacks
  still refuse before any spawn through the admission comparison, not
  through the engine's real boxed-exec (`compose`/`run_boxed`) path, so they
  do not yet establish boxing. Rebuilding them on the boxing path, with
  separate removal controls for atomic admission and retention through
  settlement, is outstanding; these take the `HANDS_BOX_ENV` skip in this
  seat and are a host handoff regardless.
- **R7 (integration removal evidence).** The removal controls for
  `capability_ledger::an_engine_killed_mid_attempt_keeps_its_worktree_reserved`,
  `capability_broker_launch::of_two_runs_started_together_on_one_worktree_exactly_one_runs`
  and `the_hosts_home_is_read_only_where_nothing_stood_in` were not run this
  visit.
- **R2 (preferred form) and the 940 fixture.** Durable settlement written
  into the protected reservation at settle time (the "prefer" half of R2) is
  not built; the recovery journal guard above is the security fix. The
  `adm` settlement fixture at line ~1052 still names a sibling journal:
  pointing it at the engine's real journal makes recovery release the
  attempt the test means to hold through `settle`, so the change was
  reverted and left as a fixture-cleanliness residual.

### Still pending, not claimed

The exact-coverage gate and its CRAP ratchet, macOS CI, and the host-run
proofs (the real-box matrix and any group-writable full-suite run that
executes rather than skips the admission fixtures) are pending until their
actual results exist. The real-HOME absence audit on a fresh host HOME
remains the controller's handoff.

## Run 3, second visit

This visit kept the staged tree and `1e49e339`, and finished R5, R6 and
R7, rebuilt R1 and R2 where the first visit fell short, and recorded the
LOW residuals. Paths are as above; `rec` is the new child
`broker/admission/tests/recovery.rs`, `att` is `writers_tests.rs`.

### Files

| File | Change |
| --- | --- |
| `engine/broker.rs` (row) | Declares `mod root`. `protected_with`'s walk calls `root::step`, so one function makes, enters and judges a directory, here and below the root. `spelled` names a bind's host path as authored, shared by `reach` and admission. 791 → 775 lines. |
| `engine/broker/root.rs` (new private child, 206 lines) | `Root`: the coordination root as an opened owner-only handle and its path. `Root::of(home)` is production's resolution through the protected walk; `Root::opened` (test only) takes a handle a test made. `below` reaches every directory beneath it from the handle. The lock, `read`, `seal` and `seal_with` moved here from `admission.rs`; `read` now opens records non-blocking too. |
| `engine/broker/admission.rs` | `admit`/`admit_with` take `&Arc<Root>` and return the lease; `Held` carries the root, `staked`, the worktree and the authored hands. `Engine::admit` resolves the root only after the claim is made, and never from an unusable HOME. `coordinated` is C1's decision. `Reserved.cleanup` is the durable `Witnessed` fact. `Reach::Boxed` gains `sources`. `recheck` rebuilds each boxed writer's reach from its authored hands. `amend` is the one locked read and reseal behind `release` and the unresolved mark. 735 lines. |
| `engine/broker/exclusion.rs` | `replaceable` judges every authored source by the directory holding it and each of its ancestors, against every participant's writable roots, by path and identity. `reaches` (the journal check) is one-way. `staked` is the stakes test in one place. 456 lines. |
| `engine/broker/settlement.rs` | `witnessed` seals `Witnessed::Unresolved` into the reservation the first time; recovery certifies only `Settling` attempts with a certified end. 213 lines. |
| `historical.rs`, `inventory.rs` | Seal and read through the root's handle. |
| Tests | `adm` (1,231 → 1,369), new `rec` (136), `att` (275 → 471), `coord` (size-neutral), `tests/support/writers.rs`, `capability_broker_launch.rs`, `capability_ledger.rs`, `operated_repo.rs`, and `brokkr-cli/tests/support/engine.rs`. |

`quality/file-lines.txt` carries every new size and the two new rows.
`engine.rs`, `sequence.rs`, `dispatch.rs` and `panel.rs` did not move in
this visit. No public item, trait, flag, environment knob, authored key,
daemon or suppression was added.

### Each finding

| Item | Fix | Proof (test, decisive assertion) | Removal control, observed failure |
| --- | --- | --- | --- |
| C1 | Resolution happens after the claim. Where no root resolves, `coordinated` proceeds unreserved only with the fence standing and nothing claimed at stake; otherwise it refuses `Unknown::Root`. | `adm::an_unprotectable_root_proceeds_unreserved_only_where_nothing_is_at_stake`: `coordinated(None, plain) == Ok(None)`; a real dispatch on an unrooted HOME succeeds, its driver runs and no reservation exists; with evidence in the artifact root, and with the fence lifted, the dispatch fails `Unknown::Root` before its driver. `engine::tests::a_selected_single_publishes_its_own_confinement_at_dispatch` passes under the group-writable TMPDIR (below). | M1 `None => Err(Unknown::Root)`: failed at `left: Err(Root)`, `right: Ok(None)`. |
| Real-HOME leak | Unit tests never resolve a HOME (R5). `brokkr_in`, the CLI's shared launcher, gives each `brokkr` an owner-only `dir/home`. `operated_repo`'s one test, alone in its binary, points HOME at its own directory through a guard that restores it. `support/writers.rs` children already set HOME and now place it in a protected location. | The CLI modules launching through `brokkr_in` (agent_fallback, agent_readouts, mutants_gate, effect_replay, hands, witness_journal, machine_proof, capability_broker::cleanup) passed 121; `operated_repo` passed. | Isolation, not behaviour. The host audit with a fresh HOME is the controller's. **Corrected in run 4:** this was not complete. Launchers outside `brokkr_in` (muninn, realms, init_stacks and others) and the in-process engines of `capability_launch`'s `legacy_journal` and `driver_conformance` still admitted under the inherited HOME, and `operated_repo`'s guard duplicated the house `EnvGuard` (I1). Run 4's M4 row lists the full audit. |
| R1 sources | `Reach::Boxed.sources` holds every source hands binds apart, as authored: the worktree, a Git common directory outside it, each bind's `~/`-expanded path. A source is refused `Replaceable` where the directory holding it, or holding any ancestor, resolves at or below any participant's writable root, its own included, by canonical path or by identity. A Git common directory inside the worktree is neither a source nor a writable root (hands binds it as part of the worktree). Under the fence rule, inside the comparison. | `coord::a_replaceable_bind_source_is_refused_before_spawn` (a source under the worktree → `Replaceable{work/src}`; a disjoint one admitted). `att::the_attack_matrix…`'s R1 case: a rw source under the attacker's worktree, which another writer could swap for a symlink to the coordination root, is refused `Replaceable` before the keeper starts, the admission lock's bytes unchanged; its last case binds a disjoint source and is admitted. | M5 the swappable search made empty: the coord test admitted (`left: Ok(Held …)`), and the matrix failed at its refusal assertion (`EffectIndeterminate` for `EffectFailed`). The box then ran with the swapped source, and the lock stayed unchanged. So for this case the binding assertion is the refusal, not the bytes. |
| R1 door | `recheck` rebuilds each boxed writer's reach from its authored hands and `git_facts`, as hands rebuilds the box, and refuses `Redirected` where any path differs. The fence rule applies (`everywhere()` or `Held.staked`), so the first visit's unconditional deviation is withdrawn. | `adm::the_door_refuses_a_lease_whose_reach_was_redirected`: unchanged → `Ok`; source swapped for a symlink → `Ok` with the fence standing and nothing at stake, `Redirected(src)` when staked, `Redirected(src)` with the fence lifted. | M3 check never applies: failed "evidence at stake". M4 fence rule dropped: failed "fence standing, nothing at stake". |
| R2 | The reservation carries the typed negative fact. The engine seals `Witnessed::Unresolved` into the attempt's entry when it witnesses an unresolved cleanup, before the terminal event. Recovery releases only a `Settling` attempt whose end in a journal no participant can reach is a success or failure. The reach test is now one-way (a writable root below the journal's directory writes none of its entries; the two-way test had refused every engine whose worktree sits beside its journal). The 940 fixture now names the engine's real journal. | Fresh admissions on each engine's actual sealed journal, after its reservation is put back as it stood mid-attempt: `rec::a_settled_end_in_a_protected_journal_releases_its_attempt` (admitted, attempt settled); `rec::a_moved_head_end_keeps_its_attempt_reserved` (`GATE-MOVED-HEAD` indeterminate end, `Held`); `rec::an_end_in_a_journal_its_writer_could_forge_keeps_its_attempt_reserved` (the box binds the journal's directory, `Held`); unresolved cleanup through `unsettled`, shared by `adm::a_moved_head_gate_with_unresolved_cleanup_keeps_its_reservation` (now boxed) and `adm::a_panel_member_with_unresolved_cleanup_keeps_its_reservation`: the reservation reads `[Unresolved]`, a fresh admission is `Held`. `adm::a_lease_is_released_only_on_proof_its_processes_settled` on the real journal keeps a4 (unresolved, then a success). | M6 mark ignored by recovery: the 940 test released a4. M7 mark never sealed: that test and both `unsettled` tests failed (`[Settling]` for `[Unresolved]`). M8 reach guard off: `rec` forge test and `coord::a_settled_end_in_a_reachable_journal…` admitted. M9 indeterminate certified: `rec` moved-head test and `coord::an_uncertain_survivor…` admitted. M10 recovery releases nothing: `rec` settled-end and `coord::a_crash_after…` refused `Held`. M11 two-way reach restored: the `rec` settled-end test refused `Held`. |
| R3 | First visit's non-blocking digest open and late-seal withdrawal kept; records are opened non-blocking too. | `coord::a_fifo_artifact_refuses_without_blocking`; `adm::a_seal_that_finishes_past_the_deadline_is_withdrawn`. | M17 `O_NONBLOCK` removed: the FIFO test never finished; `timeout 60` killed it (exit 124). |
| R4 | Kept from the first visit. | `coord::the_mount_table_is_bounded_as_it_is_read`. | First visit's control. |
| R5 | Admission's core takes the opened root (`admit_with(&Arc<Root>, …)`). Production resolves it from HOME (`Root::of`). Every other test admits in a root it opened with no ancestor judged, under any TMPDIR: `Host::new` never skips, and the `temp_dir` fallback is now harmless. Only the resolution tests use `Host::resolving` (TMPDIR, else `/tmp`, else `/var/tmp`, else skip with reason). `support/writers.rs` picks such a location or skips. | Resolution tests: `adm::the_root_resolves_from_home_only_where_every_ancestor_protects`, `adm::every_directory_admission_makes_is_synced_in_its_parent`, and `adm::the_hosts_home_is_read_only_where_nothing_stood_in` (which needs none). | M2 the walk's ownership judgement removed: failed "a root its group may write". M18 sync removed: the M9 test failed (`left: []`). M16 the production seam returning before the claim: failed (`EffectStarted` for `EffectFailed`). |
| R6 | `att::the_attack_matrix_keeps_every_protected_byte_on_the_real_boxed_exec_path` (Linux). Beside the retaining keeper with the fence lifted, every attacker is an exec seat. Dispatch decides admission; every admitted attack then runs in the real box `hands exec` builds from the engine's composed argv (`hands_command`, `run_boxed_in`); then the protected bytes are compared. Cases: an already-running unboxed writer (`Unboxed{work, run-s}`); historical artifacts reached by a bind of their run-local directory, by a bind of their worktree with an ancestor replacement, and by a symlink alias (`Reaches`); a hard link in the attacker's worktree (`Corrupt`); the R1 swapped source (`Replaceable`); and a zero-grant box with a disjoint bind, admitted. That box's writes to its own retained artifact and to the historical one fail ("Read-only file system", "Directory nonexistent"), its `mv .forge gone` fails, and `work/made` holds `w`. Last, both digests equal their names. The bind alias stays `coord::a_bind_alias_of_protected_evidence_is_refused`. | bubblewrap ran in this seat, so the matrix executed here, real boxes included; it is not host-pending. **Corrected in run 4:** the boxes ran after the dispatch had already ended, outside the attempt and its lease, and the already-running writer was a synthetic lease, so this was no proof on the dispatch path; the host run of the real box was still owed. Run 4's M7 row replaces it. | Atomic admission removed (M12, the comparison returns after the lease): failed at the first case's refusal (`EffectIndeterminate` for `EffectFailed`). Retention through settlement removed (M13, no historical record): failed on the protected bytes themselves (`forged` for `historical`), the real box having overwritten the artifact. `coord::settled_evidence_stays_protected…` failed too. |
| R7 | — | — | `capability_ledger::an_engine_killed_mid_attempt_keeps_its_worktree_reserved`: M14 recovery certifying every standing attempt → failed "while-it-runs" (`completed` for `awaiting_operator`). `capability_broker_launch::of_two_runs_started_together_on_one_worktree_exactly_one_runs`: M15 lease never refuses → failed "one of them is refused never held" after 30 s, and `a_running_writer_holds…` failed. `the_hosts_home_is_read_only_where_nothing_stood_in`: M16 above. `adm::an_unprotected_lock_refuses` (renamed): lock judgement removed → admitted for `Err(Identity(Lock))`. Every control above compiled, failed as named and was restored; the suites then passed. |
| INFO | Kept from the first visit; the matrix's helpers and imports are `cfg(target_os = "linux")`. | — | — |

### TMPDIR matrix

TMPDIR was varied with `cargo --config env.TMPDIR` (`force = true`). Each
run is the whole brokkr-runtime library (1,096 tests) plus `it`'s
`capability_ledger`, `capability_broker_launch` and
`two_engines_one_journal` (6).

| TMPDIR | Library | Integration | Skipped |
| --- | --- | --- | --- |
| inherited (`/tmp` here) | 1,096 passed | 6 passed | 1: `coord::a_bind_alias…` (no user namespace) |
| `.forge/tmp700`, mode 0700, inside this checkout | 1,084 passed, 12 failed | 6 passed | the same 1 |
| `.forge/tmp700/g775`, mode 0775 under 0700, inside this checkout | 1,084 passed, 12 failed | 6 passed | the same 1 |

The same 12 tests fail in the 0700 and 0775 runs, and the two lists match
line for line. All 12 are places, realm, adopted-map and dsh-resume tests
(`places::places_tests::*`, `admission::tests::an_adopted_map_*`,
`engine::tests::realm_facts_*`, `the_protected_phase_is_entered_at_a_recorded_head`,
`engine::resume_tests::the_real_dsh_driver_*`,
`bundle::agent_tests::opened_tests::every_reason_to_open_*`). They see the
git checkout that encloses a TMPDIR placed inside it. The seat could make
no directory outside the checkout. Every U8a2a test, and
`a_selected_single_publishes_its_own_confinement_at_dispatch`, passed in
all three runs. The resolution tests and `support/writers.rs` found
`/tmp` protecting when TMPDIR did not. A group-writable TMPDIR outside any
repository is pending for the controller.

**Corrected in run 4:** these runs were not a full-suite validation. Two
of the three left twelve library tests failing, and only the library and
three integration modules ran; brokkr-cli's suites were not run with the
TMPDIR varied at all. Run 4's TMPDIR and Gates sections record what it
ran, and the three host modes remain the controller's.

### Gates

Formatting check, workspace clippy with `-D warnings`, `git diff --check`
and `typos --hidden` over the touched paths were clean. `quality/ratchet.sh`
printed "file size holds" and "duplication holds". Two new clones were
folded, not baselined: an import run shared with `inventory.rs`, and a
second unresolved-cleanup test, which now lives in `unsettled`. The
`recipes/self` compile ended "0 unruled". brokkr-runtime passed: lib
1,096, `it` 136, `capability_launch` 76, `operated_repo` 1,
`queued_launch` 6. brokkr-cli passed `it` outside `capability_broker`
(615, the layering, suppressions and doc-truth tests among them), and the
launcher modules above.

### Residuals (LOW), with reasons

| Residual | Reason |
| --- | --- |
| The lease is per attempt, joined into one run reservation (L12). | As built: a run's later attempt joins its reservation, and each attempt is released alone. |
| A genuine settled indeterminate or moved-HEAD end is never certified after a crash. | An indeterminate end proves nothing about the tree. Such a reservation remains until its engine releases it or the operator removes it. |
| Historical verification rehashes every artifact on each admission at stake. | Its cost grows with history, bounded by D5's entry bound and the deadline. |
| Settled reservations accumulate; nothing prunes them. | Pruning was not built; D5's entry bound eventually refuses with MB3's cause. |
| No-grant refusals read MB3's MCP identity cause. | The cause vocabulary is the spec's. |
| `ENTRIES_MAX` repeats the protocol's private bound (L14); refusal texts show host paths. | Making the protocol's bound public is outside this row. |
| `capability_launch`'s in-process `legacy_journal` drive admits under its process HOME. | That binary runs its tests on parallel threads, where a process-wide HOME change would race the others' reads. Moving its engine into a child with an explicit HOME, as `support/writers.rs` does, is the follow-up. The controller's fresh-HOME run covers it. Closed in run 4 (M4): those tests run again in a child whose HOME is their own. |
| Hands recomputes Git facts when it rebuilds a box. | Replaced in run 4 by the operator's second ruling of 2026-10-18 (after U8a2a's third run): U8a2a seals each writer's bind sources and both Git directories by canonical path and device/inode, includes an external common directory in its writable reach, and observes them again at the dispatch door; holding later calls of one running driver to the sealed reach is U8a2c's, not a residual of this row. See run 4, H2. |

### Pending, not claimed

The exact-coverage gate and its CRAP ratchet; macOS and remote CI; the
bind-alias proof, which needs a user namespace this seat lacks; the
group-writable TMPDIR outside any repository; and the fresh-HOME
real-root absence audit on the host.

## Run 4 (0065-slice-two-unit-u8a2a-see-th-3a0cc19c)

Run 3's second visit was held again (REVIEW-SECURITY-HOLD: two HIGH,
eight MEDIUM, two INFO gate findings, six LOW). Run 4 kept run 3's staged
work on main at `05e6787c` and answers each finding below. Two operator
rulings of 2026-10-18 bind it; design.md records both. The first is the
fence rule: with `McpFence::current()` standing, coordination refusals
apply where evidence is at stake, and once the fence is lifted they apply
to every participant on every run. The second, after run 3, moves
mid-run redirection to U8a2c: U8a2a seals each writer's resolved reach,
compares it as D6 says and rechecks it at the dispatch door. Tasks 38.3
and 38.4 stay unticked, because U8a2c and U8a2b close them with this unit.

Paths below are relative to `crates/brokkr-runtime/src/engine/`: `adm` is
`broker/admission/tests.rs`, `coord`, `rec` and `seal` its children
`tests/coordination.rs`, `tests/recovery.rs` and the new
`tests/sealing.rs`, and `att` is
`capability_tests/provision_tests/brokered_tests/writers_tests.rs`.

### Files

| File | Change |
| --- | --- |
| `broker/root.rs` (child of row file `broker.rs`; 206 → 222) | A seal writes and syncs its staged copy, hands it to the root's hook, checks the deadline once more and only then renames it in. The rename is the one commit; a failure or expiry before it removes only the staged copy (H1). `Root` carries a `Hook`, which is `unhooked` in production and a test's own on a root a test opened (`Root::opened`, test only). `Root::seal` seals through it. |
| `broker/admission.rs` (735 → 778) | `Witnessed::Settled` is the positive settlement fact, and `Unknown::Unproven` the refusal of a release without it (M1). `Reach::Boxed` gains `sealed: Vec<Source>`: the worktree, Git directory, Git common directory and each bind source, each by canonical path and device/inode (H2). `recheck` compares those sources observed again, and `redirected` reports the path that changed (L2). `release`/`amend` only settle an attempt that holds the fact. `Engine::admit` takes the dispatch's observation through its seam (L1). |
| `broker/settlement.rs` (213 → 249) | `settle` seals the fact (`proved`) and only then releases. `witnessed_by` is the deadline seam behind `witnessed`. Recovery releases an attempt only on the fact plus a certified end in a journal no writer reaches. That journal, and each SQLite sibling that stands, is opened without following a link or blocking, and must be a regular file before SQLite opens it (L5). |
| `broker/historical.rs` (105 → 160) | Each artifact is read and hashed in 64 KiB chunks, with the deadline checked after each (M2). `verified_with` is the `_with` seam. `regular` is the one non-blocking, no-follow, fstat'd open for an artifact or a journal. |
| `broker/exclusion.rs` (456 → 492) | The mount table is read record by record with `BufRead::read_until` (M3). Each record is bounded at 64 KiB and counted, with the deadline checked before each. The record past 65,536 is the last one read. `Shown::Expired` is new. `refusal` maps each outcome to its cause. The `Unknown` import is gated to Linux, where alone it is used (I2). |
| `broker/observation.rs` (**new** private child of `broker.rs`, 39 lines) | `Observed`/`observed()`: dispatch observes the host once for each attempt and hands that one observation to admission and to provisioning (L1; decision 0071 ruling 10). It holds the one test stand-in branch, which is provisioning's existing one (U7d2), moved out of `Engine::provision`. |
| `broker.rs` (row; 775 → 779) | `provision` takes the observation. Provisioning reaches the attempt's run directory from the handle of the root the attempt was admitted in (`held.coordination.below`) instead of walking from HOME again (M5). Declares `mod observation` and re-exports its two names. |
| `dispatch.rs` (child of row file `engine.rs`) | `execute` observes once and hands the observation to `admit` and `provision`. |
| `crates/brokkr-runtime/Cargo.toml`, `Cargo.lock` | A direct edge to the workspace's `sha2`, which brokkr-core already takes, for the incremental hash M2 needs. No registry version moved; the lockfile gains one dependency line. This is outside the row's Rust files, so it is recorded for the operator's ruling at PR time. |

`engine.rs`, `sequence.rs` and `panel.rs` did not move in run 4.
`quality/ratchet.sh api` printed "public API holds": every new item is
private to `broker`, or `pub(in crate::engine)` where the engine's
dispatch consumes it. No trait, flag, setting, environment knob, authored
key, daemon or suppression was added. The fence is still the switch's one
home (`exclusion::everywhere`).

Test files: `adm` (1,369 → 1,482), `coord` (679 → 845), `rec` (136 →
179), new `seal` (171), `att` (471 → 644), `brokered_tests.rs` (798 → 802;
`Claude::of`), `provision_tests.rs` (`Fixture::inline_in` made
`pub(super)`), runtime `tests/support/writers.rs` (305 → 408), new
`tests/support/own_home.rs` (66), `tests/{operated_repo,recovery}.rs`,
`tests/capability_launch/legacy_journal.rs` and `legacy_journal/codex.rs`,
`tests/it.rs`. In brokkr-cli: `tests/support/engine.rs` and the launchers
named under M4. No test moved between files. `quality/file-lines.txt`
carries the new sizes and the three new files' rows, and
`quality/too-many-lines.txt` the three rows whose lines moved in
`boundary_verbs.rs` and `delivered_by_brokkr.rs`.

### Each finding

| Finding | Fix | Proof | Removal control (failure observed, then restored) |
| --- | --- | --- | --- |
| H1 | A seal's rename is its only commit, and the last deadline check comes before it. Nothing before the commit touches the standing record, so a failed or late reseal leaves it the same bytes in the same inode, with no staged copy left. Join, the unresolved mark, the positive fact, release, recovery and the historical record all seal through `Root::seal`. A directory sync that fails after the commit refuses with the successor record in place, and never deletes it (residual below). | Each case starts from an existing unsettled reservation and expires at the final seal through the root's hook: `seal::a_late_join_keeps_the_reservation_that_stood`, `…_late_unresolved_mark_…` (then settles, and still holds), `…_late_release_…`, `…_late_recovery_…` (the next run refused `Expired`). `seal::a_positive_fact_that_cannot_be_sealed_keeps_the_reservation` fails the commit itself (the staged copy removed). Each asserts bytes and `(dev, ino)` unchanged and nothing staged. `seal::a_seal_that_would_finish_past_the_deadline_is_never_committed` covers a fresh record. | M11 (a refused seal unlinks the record, run 3's bug): all four plus the fact test failed, the reservation gone. M12 (run 3's order: rename, then check, then withdraw): the same five failed. M13 (no check before the commit): the probe test and the four failed, the record replaced. |
| H2 | Operator ruling 2026-10-18, after U8a2a's third run. `boxed_reach` seals the worktree, `git_facts`' Git directory and common directory, and each bind source, by canonical path and device/inode, into the reservation's `Reach::Boxed.sealed`. A common directory outside the worktree is a writable root, as hands binds it read-write. At the dispatch door `recheck` asks Git again and rebuilds the sealed list, refusing `Redirected` before spawn under the fence rule. It does not refuse a writer merely because writes in its reach could redirect a later call: holding a running driver's later calls to the sealed reach is U8a2c's, by that ruling. Run 3's authored-source refusal (`Replaceable`) stays as it was. | `adm::the_door_refuses_a_worktree_whose_git_names_another_repository`: a linked worktree's sealed list is the worktree, `main/.git/worktrees/linked` and `main/.git`, and its writable reach includes `main/.git`. The unchanged worktree spawns through the door. Once `commondir` names another repository's `.git`, the door refuses `Redirected(main/.git)` and nothing spawns. `adm::the_door_refuses_a_lease_whose_reach_was_redirected` adds another directory put at the source's path (identity alone). | M23 (Git directories left out of the sealed list), M24 (common directory not writable), M19 (the door never rechecks) and M20 (the redirect check never applies) each failed the Git test. M22 (sealed sources without identity) failed "another directory at its path". M21 (fence rule dropped) failed "fence standing, nothing at stake". |
| M1 | `settle` journals nothing new. Once the body ran, its terminal event is journaled and every driver it ran reported `Cleanup::Settled`, it seals `Witnessed::Settled` into the attempt's entry (`proved`), and only then releases. `release` refuses `Unproven` without the fact. A persistence failure keeps the reservation, and later runs are refused by its lease. Recovery releases only an attempt holding the fact whose certified end (success or failure) stands in a journal no participant's writer can reach. Run 3's journal-reach exclusion is kept. | `rec::a_settled_end_without_its_positive_fact_keeps_its_attempt_reserved` (a genuine success, no fact: `Held`). `seal::a_positive_fact_that_cannot_be_sealed_keeps_the_reservation` (persistence fails: unchanged, next run `Held`). `rec::a_forged_end_in_a_journal_its_writer_could_reach_keeps_its_attempt_reserved` (a real dispatch whose gate moved its head ends indeterminate; its fact is planted; a success is forged into that very sealed journal, which its box can reach: `Held`). `rec::a_settled_end_in_a_protected_journal_releases_its_attempt` (released). `seal::a_release_without_its_positive_fact_is_refused`. | M50 (recovery without the fact) failed the missing-fact test and the persistence test. M49 (reach guard off) failed the forged test and `coord::a_settled_end_in_a_reachable_journal…`. M48 (recovery releases nothing) failed the genuine release. M57 (release without the fact) failed the release test. M58 (M57 plus `settle` releasing after a failed fact; the behaviour is guarded twice) failed the persistence test. |
| M2 | `historical::digested` reads and hashes 64 KiB chunks and checks the shared deadline after each, so one artifact cannot hold the lock past it. The non-blocking, no-follow, regular-only open is kept. | `coord::a_deadline_that_passes_inside_an_artifacts_read_ends_its_verification`: a 192 KiB artifact whose first chunk outlasts a 200 ms deadline ends `Identity(Expired)` after one chunk; within the deadline it verifies in four reads. | M46 (no check per chunk): `Ok(())` for `Err(Identity(Expired))`. M45a (`O_NONBLOCK` dropped): `coord::a_fifo_artifact_refuses_without_blocking` hung until `timeout 60` killed it. |
| M3 | `exclusion::mounts` streams `/proc/self/mountinfo` through `BufRead`, one record of at most 64 KiB at a time. It checks the deadline before each record, counts records as it reads them and stops at the first past 65,536. `TooMany` takes MB3's `Bounds`, and `Expired` MB3's `Expired`. | `coord::the_mount_table_is_collected_one_record_at_a_time_within_its_bounds`. A never-ending reader is read to record 65,537 and no further (`(TooMany, 65537)`). A never-ending record is read to 65,537 bytes (`TooMany`). An expired deadline reads nothing (`(Expired, 0)`). Each outcome's refusal is pinned. The byte-slice exact and one-over test stays (`coord::the_mount_table_is_bounded_as_it_is_read`), and a blank record is passed over. | M53 (count off by one): `(TooMany, 65538)`, and the one-over table read `Once`. M54 (record bound dropped): `Elsewhere`. M55 (no deadline check): `(TooMany, 65537)` for `(Expired, 0)`. M56 (causes swapped). M52 (`once` always shown once). |
| M4 | No test writes the process environment. Runtime in-process engines: `tests/support/own_home.rs` runs a test again, alone, in a child process whose HOME is a fresh owner-only directory (bounded at 300 s), and each engine-driving helper asserts it stands in such a child, so a later caller is held to it. Applied to `operated_repo.rs` (its `Home` env writer removed, I1), `recovery.rs` (both tests, through `started`), `capability_launch/legacy_journal.rs` and `legacy_journal/codex.rs` (all four tests that drive, through `drive`). Children already given HOME: `support/writers.rs`, `two_engines_one_journal.rs`. Not dispatching: `capability_launch.rs`'s `start_in_world` probe and `queued_launch.rs`'s `launch::start`. Unit tests observe through `admission::tests::observed`, never the host's HOME unasked. brokkr-cli: `run`, `resume` and `rerun` are the only verbs that drive an engine. Every launcher of them now gives a HOME of the test's own, unless the test names one: `support/engine.rs` (`brokkr_in` and its callers, through the new `home_in`), `brokkr_folder.rs`'s `Repo::command` (used by `shelf.rs`), `init_stacks.rs`'s `brokkr_with` (used by `doc_samples.rs`), `bootstrap_bench.rs`, `capability_verbs.rs`, `delivered_by_brokkr.rs`, `hands.rs` (both shipped-recipe runs), `boundary_verbs.rs`, `recipes.rs`, `realms.rs` and `muninn.rs`. A run that binds `~/.cargo`, `~/.rustup` or `~/.npm` links those into its own HOME (`toolchain_home_in`). Launchers that already set HOME: `transcript_privacy.rs`, `transcript_surfaces.rs`, `transcript_command.rs`, `retired_overrides.rs`, `capability_broker` and its children, and `driver_conformance.rs`, whose two in-process proof engines now set HOME through the house `EnvGuard` inside their shared helper `proof_engine`. The remaining launchers run no dispatching verb (analytics, acknowledge, dsh_sandbox_runner, agent_fallback and agent_readouts through `brokkr_in`, boundary_readouts, capability_broker's survey and observer, harness_probe, witness_journal, binary_names, init_doctor, contributing, doctor_dsh_selection). The brokkr-cli library's in-process `run`/`resume`/`rerun` calls (`tests.rs`, `verbs/tests.rs`, `verbs/delivery/tests.rs`) all refuse before an attempt starts, and `run_view/tests.rs`'s resumes conclude before any seat spawns. | The runtime and CLI suites pass, counts under Gates. `tests::env_guard::nothing_but_the_guard_writes_the_process_environment` passed. This seat cannot list or stat the operator's HOME, so the fresh-HOME absence check is the controller's: host result pending. | Isolation, not behaviour. The helpers' own guard (`own_home::required`) refuses a helper call outside the child. |
| M5 | Every ordinary dispatch, settlement and recovery test admits in a root its test opened, with no ancestor judged, under any TMPDIR. Provisioning now reaches its plans from that same handle, so `att`'s retaining keeper is built on any TMPDIR (`Claude::retaining` uses `Fixture::inline_in(scratch())`, never `Fixture::protected`) and never skips. | Executed and skipped in this seat, below. | — |
| M6 | `Running` reads the child's standard output on a thread of its own. Every wait is bounded by its patience (120 s): `recv_timeout` for the run's name and for each later line, and a `try_wait` loop for the exit. `Drop` kills a child still running and reaps it, on unwind too. A waiting driver also stops once its fixture's root is gone. | `writers::a_stalled_engine_is_given_up_on_killed_and_reaped`: a child naming no run is given up on within 300 ms (`"the engine named no run (Timeout): "`) and reaped. One that names its run and never ends panics `"the engine of r never ended: "` in `ended`, and its unwinding reaps it. | IM3 (`Drop` kills nothing): "killed and reaped" failed. IM4 (unbounded `recv`): the control hung until `timeout 150` killed it. |
| M7 | `att::the_attack_matrix_runs_every_admitted_attacker_in_its_own_box`. Every attacker is dispatched through the door beside the retaining keeper, with the fence lifted. One that is admitted starts under its lease and builds its own box as `brokkr hands exec` does: `hands::run_boxed` over the spec, workdir and artifact root of the argv `boxed::hands_command` composes for an exec seat of its hands. It does so while its reservation holds an attempt standing (its mark: `{"standing": true, "ran": true}`). This unit-test binary cannot stand in for `brokkr` itself, so the attacker's driver runs it on `att::boxed_attacker`. The cases: an already-running writer that really dispatched (a real engine on another thread, `Waiting`); historical artifacts reached by a bind of their run-local directory, by a bind of their worktree with an ancestor replaced, by a symlink alias, and through a hard link; the swapped source; and last a zero-grant box with a disjoint bind. That box's writes to both evidence files fail, and `work/made` holds `w`. Both digests are asserted equal to their names. The bind alias is `att::a_bind_alias_attacker_is_refused_on_the_dispatch_path`, which runs again inside a bwrap mount namespace binding the evidence directory at a second place, and is refused `Reaches` by identity. | Both executed in this seat: bubblewrap builds boxes here and `BROKKR_HANDS_BOX` is unset (no "skipped" line in the run). The controller's host run is still owed: host result pending. | M30 (atomic admission removed): the matrix failed at its first refusal, and the zero-grant and bind-alias tests failed on the protected bytes (`forged`). M31 (retention through settlement removed): the matrix failed on the historical bytes; the attacker's real box, inside its attempt, overwrote the artifact. M44 (verification removed): the hard-link case wrote through the link. M41 (identity removed): the bind-alias attacker was still refused, by the mount table (`Shown`, not `Reaches`). |
| M8 | Every U8a2a test, run 3's and run 4's, has a compiling control that failed it, listed in the next table. Each was restored by its inverse edit, and `git diff --stat HEAD -- crates/*/src` against the signed checkpoint then showed no production change. Two controls first failed nothing: M18, because every test removed the reservation file, which the read already catches; and M33, because `proved` already refuses an attempt sealed unresolved. The tests were extended (a released lease at the door; a settlement after an unresolved mark that failed to persist), and the controls then failed them. After the clone folds, R47, R48, R51, R33 and R58 re-ran those controls against the folded tests and failed them again. | Below. | Below. |
| I1 | `operated_repo.rs`'s `Home` env writer is gone; the test runs in an `own_home` child. `driver_conformance.rs` sets HOME through the house `EnvGuard` only. | `cargo test -p brokkr-cli --lib --all-features --locked tests::env_guard::nothing_but_the_guard_writes_the_process_environment -- --exact`: 1 passed. | — |
| I2 | Positive protecting boxed admissions are Linux only, and other hosts assert the refusal. `adm::historical_evidence_refuses_a_panel_member_that_cannot_be_kept_from_it` and `att::an_already_running_writer_holds_off_a_retaining_member` assert `Shown(<coordination root>)` off Linux. `adm::a_sequence_reserves_every_steps_reach_before_its_first` now holds the evidence with a protecting run that starts no writer, which every host admits. `exclusion.rs` imports `Unknown` on Linux only. | Compiles and passes on Linux. macOS is not run here: CI pending. | — |
| L1 | `Engine::admit` has no `cfg(test)` branch. It decides on the observation dispatch hands it, through `admit_in`'s seam. The one stand-in branch left is at `observed()`, and it is provisioning's existing one, moved. | `adm::the_hosts_home_is_read_only_where_nothing_stood_in` reaches the production observation. | M28. |
| L2 | `Redirected` reads "'…', which a box of the attempt is built from, no longer resolves as when its attempt was admitted". It names the sealed path that changed, or the path that appeared where none was sealed. | `adm::every_refusal_reads_as_its_cause`. | M01 (text). |
| L3 | Described as built: the lease is per attempt, joined into its run's one reservation. An attempt whose positive fact was never sealed (a crash, an uncertain end, an unresolved driver) stays reserved until the operator removes it. | — | — |
| L4 | Recorded, not pruned (residual). | — | — |
| L5 | Fixed: the guarded open above, after `budget.check()`. | `coord::a_journal_that_is_no_regular_file_proves_nothing_at_once` (a link to a journal that would certify, then a FIFO: each `Held` at once, the reservation unchanged). | M47 (no check): the link case was admitted. M45b (`O_NONBLOCK` dropped): the FIFO case hung until `timeout 60`. |
| L6, I3 | Recorded (residuals). | — | — |

### Controls

Each row is one compiling edit to production code, its suite run
(`cargo test -p brokkr-runtime --lib --all-features --offline -- engine::broker writers_tests retaining_step`,
69 tests; IM rows the runtime `it` binary), and the tests it failed.
Every control was then restored.

| Control | Edit | Failed (assertion) |
| --- | --- | --- |
| M01 | `Unproven`'s text shortened | `adm::every_refusal_reads_as_its_cause` (text) |
| M02 | the lease never refuses | 14 tests at their `Held` refusals: `adm::a_second_run_on_a_held_worktree_waits_for_the_first_to_settle`, `…_lease_its_engine_never_released_…`, `simultaneous_…`, `a_dispatch_on_a_held_worktree_…`, `a_runs_later_attempt_joins_…`, both unresolved-cleanup tests, `coord::an_uncertain_survivor_…`, `…_reachable_journal_…`, `…_no_regular_file_…`, `seal::a_positive_fact_…`, and `rec`'s missing-fact, moved-head and forged tests |
| M03 | the lease ignores the worktree's key | `adm::separate_worktrees_hold_their_leases_side_by_side` (second lease `Held`), and nine more |
| M04 | the lock dropped before the comparison | `adm::simultaneous_admissions_on_one_worktree_admit_exactly_one` (both admitted) |
| M05 | non-key root entries passed over | `adm::an_unknown_participant_refuses_before_anything_is_sealed` (first case admitted) |
| M06 | non-UTF-8 names passed over | `adm::a_name_that_is_not_utf8_is_an_unknown_participant` |
| M07 | a staged reservation is unknown | `adm::a_writer_starts_only_after_its_reservation_is_renamed_in` ("a staged record admitted nothing"), `adm::an_unknown_participant_…` |
| M08 | the lock's single-file judgement dropped | `adm::an_unprotected_lock_refuses` |
| M09 | the walk's ownership judgement dropped | `adm::the_root_resolves_from_home_only_where_every_ancestor_protects` ("a root its group may write") |
| M10 | the parent sync dropped | `adm::every_directory_admission_makes_is_synced_in_its_parent` |
| M11 | a refused seal unlinks the standing record | the four `seal::a_late_*` and `seal::a_positive_fact_…` (the record gone) |
| M12 | run 3's rename-check-withdraw order | the same five |
| M13 | no deadline check before the commit | `seal::a_seal_that_would_finish_…` and the four `seal::a_late_*` (record replaced) |
| M14 | `Budget::check` never expires | `adm::an_observation_blocked_past_the_deadline_seals_nothing`, `adm::the_inventory_reads_within_its_entry_bound`, `coord::every_step_under_the_lock_spends_the_shared_deadline`, `coord::…_inside_an_artifacts_read_…`, `coord::the_mount_table_is_collected_…`, five `seal` tests |
| M15 | the entry bound never spent | `adm::the_inventory_reads_within_its_entry_bound`, `coord::every_step_under_the_lock_…` (`Bounds`) |
| M16 | an unprotectable root always refuses | `adm::an_unprotectable_root_proceeds_unreserved_only_where_nothing_is_at_stake` |
| M17 | the door ignores the root's identity | `adm::the_door_refuses_a_lease_that_no_longer_stands` (`Replaced`) |
| M18 | the door ignores a released attempt | `adm::the_dispatch_door_spawns_only_under_a_lease_that_stands` ("released") |
| M19 | the door never rechecks | that test, `adm::the_door_refuses_a_worktree_whose_git_…`, `adm::every_steps_door_rechecks_the_lease_its_attempt_holds` |
| M20 | the redirect check never applies | `adm::the_door_refuses_a_lease_whose_reach_was_redirected`, the Git test |
| M21 | the redirect check ignores the fence rule | the redirect test ("fence standing, nothing at stake") |
| M22 | sealed sources carry no identity | the redirect test ("another directory at its path") |
| M23 | Git directories left out of the sealed list | the Git test (sealed list) |
| M24 | external common directory not writable | the Git test (writable roots) |
| M25 | a join ignores its run's reservation | `adm::a_runs_later_attempt_joins_…`, `adm::a_lease_is_released_only_on_proof_its_processes_settled` |
| M26 | `settle` never releases | `adm::dispatch_holds_the_lease_while_its_driver_runs_and_releases_it_once_settled`, `adm::a_lease_is_released_only_on_proof_…`, `att::an_already_running_writer_…`, the matrix |
| M27 | a refused admission proceeds unreserved | `adm::a_dispatch_on_a_held_worktree_fails_before_its_driver_starts`, `adm::a_sequence_reserves_every_steps_reach_before_its_first`, `adm::historical_evidence_refuses_…`, `adm::an_unprotectable_root_…`, `adm::every_steps_door_…`, `a_retaining_step_beside_an_unboxed_sibling_starts_nothing`, and the five `att` attackers |
| M28 | `evidence` ignores a root that is no directory | `adm::the_artifact_root_holds_evidence_only_where_anything_stands_in_it`, `adm::the_hosts_home_is_read_only_where_nothing_stood_in` (`Unmade`, not `Foreign`; refused before HOME, so nothing written there) |
| M29 | only a body's first site reserved | the sequence test, `adm::historical_evidence_refuses_…`, `a_retaining_step_beside_…` |
| M30 | the comparison stops after the lease (atomic admission removed) | 21 tests: twelve `coord` comparison tests, three `adm` dispatch tests, `a_retaining_step_beside_…`, and the five `att` attackers on their bytes or refusals |
| M31 | no historical record (retention through settlement removed) | `coord::settled_evidence_stays_protected_from_a_later_worktree`, `coord::historical_artifacts_…`, `coord::a_standing_fence_…`, the matrix (historical bytes `forged`) |
| M32 | a writer outside any box passes | `coord::a_lifted_fence_…`, `coord::a_protecting_attempt_refuses_its_own_unprotectable_writers`, `coord::a_protecting_run_is_refused_beside_a_writer_that_can_reach_its_root`, `coord::a_writer_that_can_reach_protected_evidence_is_refused`, `att::a_zero_grant_attacker_…`, and seven more |
| M33 | settlement ignores an unresolved driver | `seal::a_late_unresolved_mark_…` ("settled after") |
| M34 | the unresolved mark never sealed | `adm::a_moved_head_gate_with_unresolved_cleanup_keeps_its_reservation`, `adm::a_panel_member_with_unresolved_…` (`[Unresolved]`) |
| M35 | existing evidence never protects | `adm::the_artifact_root_holds_evidence_…`, `adm::historical_evidence_refuses_…`, `adm::an_unprotectable_root_…`, the matrix |
| M36 | a lifted fence ignored | `coord::a_lifted_fence_…`, `coord::a_replaceable_bind_source_…`, the redirect and Git tests, `coord::every_step_under_the_lock_…`, `adm::an_unprotectable_root_…`, `adm::every_steps_door_…` |
| M37 | stakes ignored, everything compared | 22 tests, among them `coord::a_standing_fence_refuses_them_only_where_evidence_is_at_stake` |
| M38 | replaceable sources never refused | `coord::a_replaceable_bind_source_is_refused_before_spawn` (refused case), the matrix's swapped case |
| M39b | every source but the worktree replaceable | the same test's disjoint positive (`Replaceable(apart)` for `Ok(())`) |
| M40 | a run's own earlier attempts not compared | `coord::a_runs_earlier_unsettled_attempt_is_compared_with_its_next`, `coord::a_run_id_reused_in_another_worktree_is_compared_not_exempted` |
| M41 | `covers` without identity | `coord::a_root_spelled_through_a_symlink_is_compared_by_identity`, `att::a_bind_alias_attacker_…` |
| M42 | standing reservations not participants | `coord::a_protecting_run_is_refused_…`, `coord::a_run_id_reused_…`, `coord::a_runs_earlier_…`, `att::an_already_running_writer_…`, the matrix |
| M43 | the incoming attempt's writers not compared | 16 tests, among them `coord::a_protecting_attempt_refuses_its_own_unprotectable_writers` |
| M44 | historical artifacts not verified | `coord::historical_artifacts_are_verified_against_their_names`, the matrix (hard link) |
| M45a, M45b | `O_NONBLOCK` dropped | the FIFO artifact and FIFO journal tests hung until `timeout 60` |
| M46 | no deadline check per chunk | `coord::a_deadline_that_passes_inside_an_artifacts_read_ends_its_verification` |
| M47 | the journal not judged regular | `coord::a_journal_that_is_no_regular_file_proves_nothing_at_once` (link) |
| M48 | recovery releases nothing | `coord::a_crash_after_a_certified_end_is_recovered_at_the_next_admission`, `rec::a_settled_end_in_a_protected_journal_…`, `seal::a_late_recovery_…` |
| M49 | the journal reach guard off | `coord::a_settled_end_in_a_reachable_journal_does_not_release_the_reservation`, `rec::a_forged_end_…` |
| M50 | recovery without the positive fact | `rec::a_settled_end_without_its_positive_fact_…`, `seal::a_positive_fact_…` |
| M51 | an indeterminate end certified | `coord::an_uncertain_survivor_keeps_its_reservation_and_refuses`, `rec::a_moved_head_end_keeps_its_attempt_reserved` |
| M52 | `once` always shown once | `coord::the_mount_table_shows_a_root_once_or_proves_nothing` |
| M53–M56 | as in M3 | as in M3 |
| M57 | release without the fact | `seal::a_release_without_its_positive_fact_is_refused` |
| M58 | M57 plus `settle` releasing after a failed fact | `seal::a_positive_fact_that_cannot_be_sealed_keeps_the_reservation` |
| IM1 | the lease never refuses | `capability_broker_launch::a_running_writer_holds_its_worktree_and_a_disjoint_one_runs_beside_it`, `capability_broker_launch::of_two_runs_started_together_on_one_worktree_exactly_one_runs` ("one of them is refused never held"), `capability_ledger::an_engine_killed_mid_attempt_keeps_its_worktree_reserved` |
| IM2 | recovery certifies every standing attempt | the same three; the release's own fact check then refused with MB3's `Unproven` instead of the `Held` expected |
| IM3, IM4 | as in M6 | as in M6 |

`coord::a_bind_alias_of_protected_evidence_is_refused` skipped in this
seat ("no user and mount namespace"), so no control was observed against
it: host result pending. `att::boxed_attacker` and `writers::engine_child`
are inert child entries, not proofs.

### TMPDIR

| TMPDIR | Focused unit suites (69) | Integration (`it -- capability_ writers:: two_engines`) |
| --- | --- | --- |
| unset (the system `/tmp`; this seat cannot stat it) | 69 passed; 1 of them skipped (`coord::a_bind_alias…`, no user namespace) | 12 passed, none skipped |
| `.forge/tmpg775`, mode 775 under 775 parents, inside this checkout | 69 passed; the same 1 skipped | 12 passed, none skipped |

The tests that may skip, and why: `adm::the_root_resolves_from_home_only_where_every_ancestor_protects` and `adm::every_directory_admission_makes_is_synced_in_its_parent` (`Host::resolving` tests production's resolution from HOME, so it needs a protecting location: TMPDIR, else `/tmp`, else `/var/tmp`); `capability_broker_launch`'s two tests and `capability_ledger`'s one (child engines resolve their root from HOME in production; `Coordination::new` uses the same fallback); `coord::a_bind_alias…` (user namespace); `att`'s matrix and dispatch-path bind alias (`boxes_here`: bubblewrap). Under the group-writable TMPDIR none of the HOME-resolution tests skipped: each found `/tmp` protecting. A fresh 0700 TMPDIR and a group-writable one outside any repository: host result pending.

### Gates

At the tested tree, `cargo fmt --all -- --check` and workspace clippy
with `--locked -D warnings` were clean, as were `git diff --check`,
`typos --hidden` and `openspec validate --all --strict` (26 passed).
`recipes/self` compiled with its policy sweep at "0 unruled".
`quality/ratchet.sh` printed "file size holds", "duplication holds" and
"public API holds". The first clone run found eight new test clones from
run 4's edits. Each was folded, not baselined: `survives`,
`died_after_its_end`, `leasing`, `proof_engine`, `Workspace::fresh` and
`started`; `realms.rs` was put back to a HOME derived in its launcher; and
`capability_launch.rs` was kept at its frozen 11,592 lines by declaring
`own_home` from `legacy_journal.rs`.

brokkr-runtime: lib 1,109, `capability_launch` 76, `it` 137,
`operated_repo` 1, `queued_launch` 6. brokkr-cli: lib 884,
`it` outside `capability_broker` 615, `capability_broker` 107 (305.7 s),
`driver_conformance` 27, `transcript_surfaces` 14, and one each for
`heap_claude`, `heap_codex` and `heap_dsh`. brokkr-protocol 995 plus
`hands_exits` 6, `secret_drop` 1 and `seatbelt_lifetime_probe` 99;
brokkr-core 138 plus 26; brokkr-store 104 plus its binaries; brokkr-view
323 plus 6; brokkr-bridge 17. All passed. Witness pins and budgets ran
inside `it` and did not move.

### Residuals (LOW), with reasons

| Residual | Reason |
| --- | --- |
| A directory sync that fails after a seal's rename returns `Unsealed` with the successor record in place. | The rename is the commit. The successor is always the stricter or the earned record: the joined reservation, the unresolved mark, the fact, or a proven release. Restoring the predecessor would need a second durable name, which the inventory would then have to admit. |
| The positive fact is sealed after the terminal event is journaled. | A crash between the two leaves the attempt without the fact, and it stays reserved: refusal, not admission (L3). |
| Historical verification rehashes every artifact at each admission at stake, and settled reservations accumulate (L4). | The cost grows with history, bounded by D5's entry bound and the deadline, and now chunked within the deadline (M2). Pruning settled reservations that hold no historical record was not built: it would remove an attempt directory the inventory checks against its reservation, and is left as a follow-up. |
| No-grant refusals read MB3's MCP identity cause (L6). | The cause vocabulary is the spec's (CR2, MB3). |
| `ENTRIES_MAX` and `MOUNTS_MAX` repeat the protocol's private `LIMITS` (L6). | Making the protocol's bounds public is `brokkr-protocol`, outside this row. |
| C1 (L6). | With the fence standing and nothing at stake, an unprotectable root admits the dispatch holding no lease. That run is not protected, coordinated or recorded: no reservation, no recovery. It is the operator's ruling, not protection. |
| Refusal texts show host paths (I3). | As U8a's refusals do. The operator reads them, and they are not digested. |
| The dispatched matrix's attacker builds its box with `hands::run_boxed` in this test binary, not `brokkr hands exec`. | The unit-test binary cannot be `brokkr`. The function, spec, workdir and artifact root are the ones `brokkr hands exec` uses with the argv the engine composes. The controller's host run is the binary-level check. |
| brokkr-cli rows in `quality/file-lines.txt` were already stale against main before this unit, and are left as they were. | Re-measuring them would rewrite rows this unit did not move. The ratchet holds: every one is under its ceiling, and the one over (`driver_conformance.rs`) shrank. |
| The `sha2` dependency edge. | M2 needs an incremental digest, and no crate in the runtime's graph exposes one. Recorded for the operator's ruling. |

### Pending, not claimed

The exact-coverage gate and its CRAP ratchet; macOS and remote CI;
`coord::a_bind_alias_of_protected_evidence_is_refused` (user namespace);
the controller's host runs of the real-box matrix and the dispatch-path
bind alias; the three TMPDIR modes; and the fresh-HOME check that
`HOME/.local/state/brokkr` stays absent across the brokkr-cli and
brokkr-runtime suites.

## Run 4, returned from review (same run, `REVIEW-REFORGE`)

The review of `ae2a4956` returned residual, maximum medium, with seven
MEDIUM findings (R1–R7), six LOW, two INFO and one gate INFO. This visit
answers each. The production change stays inside the row's tree:
`engine/broker/root.rs`, `engine/broker/settlement.rs` and
`engine/broker/historical.rs` (existing private children of
`engine/broker.rs`), and `engine.rs` gains one `EngineError` variant
(3,526 lines, under main's 3,586 baseline). `crates/brokkr-runtime/Cargo.toml`
and `Cargo.lock` are main's again. No test moved between files. One test
was renamed: `coord::a_settled_end_in_a_reachable_journal_does_not_release_the_reservation`
is now `coord::an_end_forged_into_a_reachable_journal_with_no_attempt_behind_it_keeps_its_reservation`.
One test file is new, `crates/brokkr-cli/tests/writer_lease.rs`, registered
in that crate's `tests/it.rs` (458 lines; its `quality/file-lines.txt` row
is set).

### Each finding

| Finding | Fix | Proof | Removal control (failure observed, then restored) |
| --- | --- | --- | --- |
| R1 | A seal now commits by atomically exchanging its staged copy with the record that stands (`renameat_with(.., RenameFlags::EXCHANGE)`). The record it replaced stays under the staged name until the seal is final. After the commit, the root's hook runs (`Step::Committed`), then the directory sync, then the last deadline check. A seal that fails or runs late there is undone (`root::undone`): the old record is exchanged back (the same file, so the same bytes and inode) and the successor removed. Where no record stood, the new one is removed. A directory that cannot exchange two names refuses `Unsealed` and never replaces a record it could not put back. This answers the residual run 4 recorded: the predecessor's second durable name is the staged name the inventory already admits. | `seal::finals` arms three ways to fail at the final steps: late before the commit, late once committed, and the directory sync failing once committed. Each of the four reseal tests (`seal::a_late_join_…`, `…_late_unresolved_mark_…`, `…_late_release_…`, `…_late_recovery_…`) starts from an existing unsettled reservation and runs all three, asserting the refusal (`Expired`, `Expired`, `Unsealed`), the record's bytes and `(dev, ino)`, and that nothing is staged. `seal::a_first_seal_that_fails_or_runs_late_leaves_no_record` covers a fresh record in the same three ways. `seal::a_reseal_that_cannot_exchange_keeps_the_reservation_that_stood` covers a directory that refuses the exchange (write permission taken). | N1 (`undone` does nothing): all five failed, each at "late once committed: the reservation that stood" (the fresh case at "late once committed: no record"). N1b (a failed post-commit sync ignored): all five failed at "its sync failed once committed". N2 (no post-commit deadline check): all five failed at "late once committed". N3 (a refused exchange counted as committed): the exchange test failed, `Ok(Held { … })` for `Err(Identity(Unsealed))`. |
| R2 | `Engine::settle` no longer discards the seal's result. When the positive fact or the release cannot be sealed, the reservation stands, and where the fence rule asks it (`everywhere() \| held.staked`) the run fails with the typed `EngineError::Unreleased(settlement::Unreleased(Unadmitted))`. Its text: "the attempt settled, but its writer lease stands, for its settlement was not sealed: …". With the fence standing and nothing at stake, settlement returns as before, and the reservation still stands. The forged-only proof already existed in `coord`, so it is renamed and re-documented instead of cloned: no attempt runs, the positive fact is sealed, and a settled end is appended to the engine's actual sealed journal, which the attempt's own writer box reaches. | `seal::a_positive_fact_that_cannot_be_sealed_keeps_the_reservation` runs three cases: nothing at stake with the fence standing (`Ok(())`); evidence at stake with the fence standing; and the fence lifted with nothing at stake. The last two return `Err(Unreleased(Identity(Unsealed)))`. In all three the reservation keeps its bytes and inode, and a later run is refused `Held`. Forged-only: `coord::an_end_forged_into_a_reachable_journal_with_no_attempt_behind_it_keeps_its_reservation` (`Held`, `a1` still standing). | N4 (the guard never fires): failed at "Evidence, the fence lifted: false", `Ok(())` for `Err(Unreleased(Identity(Unsealed)))`. N4b (the fence half dropped): failed at "Nothing, the fence lifted: true". N5 (`settlement::journal`'s reach guard off): the forged-only test failed, `Ok(Held { … })` for `Err(Evidence(Held(..)))`. |
| R3 | No ordinary behaviour proof can skip. `brokered_tests::a_retaining_step_beside_an_unboxed_sibling_starts_nothing` is built on `Fixture::inline()` (any TMPDIR; admission opens its root with no ancestor judged and refuses before provisioning reads the HOME) instead of `Claude::new`, which could skip. The process-level suites (`capability_broker_launch`, `capability_ledger`, through `support/writers.rs`) are the production resolution proofs. An engine in a process of its own is the public engine, which resolves its root from HOME through the protected walk; no seam reaches across the process. Their doc comments now say so, and name their injected-root counterparts, which run on every TMPDIR and never skip: `adm::a_dispatch_on_a_held_worktree_fails_before_its_driver_starts`, `adm::separate_worktrees_hold_their_leases_side_by_side`, `adm::dispatch_holds_the_lease_while_its_driver_runs_and_releases_it_once_settled`, `adm::simultaneous_admissions_on_one_worktree_admit_exactly_one` and `adm::a_lease_its_engine_never_released_keeps_refusing_with_the_lock_free`. `Coordination::new` tries the target's own temporary directory first, whatever TMPDIR is. | Executed and skipped counts are under "TMPDIR, this visit". | N10 (an unbounded writer admitted, `exclusion::judged`): the retaining-sequence test failed, `EffectIndeterminate` for `EffectFailed`. |
| R4 | `crates/brokkr-cli/tests/writer_lease.rs`, `a_real_boxed_exec_strikes_from_inside_its_lease_and_every_protected_byte_stands`, drives the real executable. `brokkr run` composes the production exec command (`{brokkr} hands exec` around `{brokkr} driver exec`) for seats with workspace hands under `namespace` and launches it through its dispatch door. The fence stays standing in a real binary, so evidence is put at stake as the fence rule allows: a worktree's artifact root already holds a published artifact. First, another worktree's run is admitted, which records its root as historical. Then an attacker run on a worktree holding retained bytes starts. Its driver, inside its box, waits until its lease is observed standing (one attempt). A second `brokkr run` on its worktree is then refused with CR2's `Held` text, and its driver never starts. Released, the attacker strikes: its own retained bytes, the historical artifact, the admission lock, a hard link of its own artifact, a move of its artifact root, and a legitimate `made` write. Afterwards every protected file holds its bytes, one link and the digest its name says. The artifact root is the same directory (the move only copied it across the read-only mount), the lock is unchanged, no link was made, `made` holds `w`, and the lease was released once settled. Five more runs, each on a fresh worktree, are refused before their drivers start: a read-write bind of the historical run-local directory, of its worktree (the ancestor), of a symlink alias of it, and of the HOME holding the coordination root (`Reaches`), and a hard link of the historical artifact into the worktree (`Corrupt`). The protected bytes are checked after each. The in-crate matrix stays as the fence-lifted, retaining-member proof. | It executed in this seat. Bubblewrap builds boxes here, `BROKKR_HANDS_BOX` is unset, and the protected-location search took `/tmp` after refusing the target's 0775 directory. The test's earlier failures, from inside the run, show it really ran: a refusal of the 0775 location, then a `gone` copy. Passed in 1.2 s. The controller's host run stays owed. | N8 (atomic admission removed: `compared` skipped in `admit_with`): failed at the second run's `Held` assertion, `[]` for the CR2 text. N8b (the participant comparison removed, the lease kept): failed at case "bind", `[]` for `Reaches`. N7 unrestricted (below) failed "its lease stands", 0 for 1. |
| R5 | No production change. This is the missing independent control. | `att::an_already_running_writer_holds_off_a_retaining_member` and `att::the_attack_matrix_…`: a real engine's dispatch on another worktree (`Waiting`) keeps its unboxed driver alive while the retaining panel is admitted beside it. | N7, a premature release at the dispatch door: `recheck` moved the attempt from standing to settled once its door passed, for attempts that put no evidence at stake, so the waiting writer ran with no standing reservation. Both tests failed at `writers_tests.rs:118`, `refused`'s event assertion, `EffectSucceeded` for `EffectFailed`: the retaining attempt was admitted and its members ran while the cross-worktree writer was alive. Restored, both pass. Unrestricted N7 (every attempt released at its door) failed the binary test at "its lease stands", 0 for 1. |
| R6 | The `sha2` edge is gone; the manifest and lockfile are main's. `historical::digested` still reads in 64 KiB chunks with the deadline checked after each (M2), but reads no further than one byte past MB3's per-operation allowance (`REPLY_BYTES_MAX`, 1 MiB). It refuses anything larger as `Corrupt` and hashes what it read once, with `canonical::sha256_bytes`. That also removes the hand-rolled hex rendering (L4). Assumption: retention publishes the masked response prepared at the bounded edge (CR2), which MB3's allowance bounds. If U9 publishes larger artifacts, this bound must move with it. | `coord::an_artifact_past_the_reply_allowance_is_refused_though_its_name_digests_it` (an artifact of exactly the allowance verifies; one byte over is `Corrupt`, though named by its digest). `coord::a_deadline_that_passes_inside_an_artifacts_read_ends_its_verification` still binds the chunked deadline. | N6 (the bound raised): the allowance test failed, `Ok(())` for `Err(Evidence(Corrupt(..)))`. |
| R7 | `Running::started` returns `Result<Running, writers::Unnamed>` (`Timeout`, `Ended`), a `thiserror` enum. The test asserts the variant, and pins the `Display` once, separately. | `writers::a_stalled_engine_is_given_up_on_killed_and_reaped`: `Some(Unnamed::Timeout(""))`, and "the engine named no run within its patience: seen\n". | N9 (a timeout reported as `Ended`): failed, `Some(Ended(""))` for `Some(Timeout(""))`. |

### LOW and INFO

| Finding | Disposition |
| --- | --- |
| L1 (settle drops `held` before propagating `ran`'s error) | Carried. When the terminal append meets contention and `lawful_end_under_contention` lands the genuine settled outcome, no positive fact is sealed. The attempt stays reserved and refuses later runs, which fails closed. Sealing the fact on that path means threading `held` through the contention recovery in `engine.rs`, which is behaviour beyond the findings this visit owns. |
| L2 (`git_facts`' Git subprocesses have no deadline) | Carried. The function is `brokkr-protocol`'s, outside this row. No admission lock is held while it runs, and it admits nothing. |
| L3 (realms.rs HOME inside the operated workspace) | Fixed. The `Workspace` struct is unchanged, so its baselined clone with `agent_readouts.rs` holds. The workspace is now `ws` inside the test's temporary directory, and HOME is `home_in` that directory, beside the workspace. The realms suite passed (17 tests with `writer_lease`). |
| L4 (repeated bounds; hex rendering) | Hex rendering is fixed (R6). `ENTRIES_MAX` and `MOUNTS_MAX` still repeat the protocol's private bounds: sharing them changes `brokkr-protocol`, outside this row. |
| L5 (`reaped` accepted any error) | Fixed: `reaped` holds only for `Err(Errno::SRCH)`. |
| L6 (rehash cost and accumulation; MB3 wording; host paths) | Carried, for run 4's reasons. Pruning would remove attempt directories the inventory checks; the cause catalogue is fixed; refusals are read by the operator and not digested. |
| I1 (recovery reopens the journal by path) | Carried. The remaining window is non-managed, same-user host mutation, and managed writers are excluded by reach first. |
| I2 (sleep-based seal tests) | Unchanged. Each mode's arming names its step (`Step::Staged` or `Step::Committed`), so a late mode is spent only at its own step. Under load, an earlier expiry would still fail the mode's own assertion of its refusal. |
| P1 | No instruction in the panel's text was taken as a direction. |
| G1 (a reviewer's run on a 1777 `/tmp` owned by 65534) | Not reproduced here. The ordinary tests no longer depend on `/tmp`'s owner (R3). The controller's three TMPDIR modes stay owed. |

### Corrections to run 4's evidence

Run 4's residual row "A directory sync that fails after a seal's rename
returns `Unsealed` with the successor record in place" is superseded by R1:
such a seal is now undone. The row "The dispatched matrix's attacker builds
its box with `hands::run_boxed` …" is answered by R4's binary-level test;
the in-crate matrix remains, but it is not the binary-level proof. The row
"The `sha2` dependency edge" is withdrawn (R6). Run 4's M5 row overstated
its claim: `brokered_tests::a_retaining_step_…` could still skip (now fixed),
and the process-level suites are production resolution proofs that may skip
where no location protects a HOME, as M5's exception allows. Run 4's
attribution of M31 to "retention through settlement" was wrong: that
mutation removed the historical record. R5's N7 is the retention control.

### TMPDIR, this visit

The seat's TMPDIR is the system `/tmp`: every fixture path read
`/tmp/…`. This seat cannot stat `/tmp`, but it passed the protected-location
predicate (owned by this user or root, writable by no one else unless root's
and sticky), which `writer_lease` reached after refusing the 0775
`target/tmp`. Focused unit suites (`engine::broker writers_tests
retaining_step`): 71 passed, of which `coord::a_bind_alias_of_protected_evidence_is_refused`
skipped (`unshare` could not write `uid_map`), and `att::boxed_attacker` is
the matrix's inert child entry. The matrix and the dispatch-path bind alias
executed. Runtime `it`: 137 passed, no skip line; the three process-level
tests and the stalled child executed. brokkr-cli `writer_lease`: executed,
passed. Host result pending for a fresh 0700 TMPDIR and a group-writable one.

### Gates, this visit

Formatting is clean, clippy is clean across the workspace (all targets,
all features, locked), and `git diff --check` is clean. `typos --hidden`
reported nothing, and `openspec validate --all --strict` passed 26 of 26.
`quality/ratchet.sh files` and `clones` hold; the four new clones the
first clone run found were folded, not baselined. `recipes/self` compiled
with 0 unruled valuations. The I1 gate, `tests::env_guard::nothing_but_the_guard_writes_the_process_environment`,
passed. Suites: brokkr-runtime lib 1,111, `it` 137, `capability_launch` 76,
`operated_repo` 1 and `queued_launch` 6, all passed. brokkr-cli: lib 884
(1 ignored); `driver_conformance` 27, `transcript_surfaces` 14, and one
each for `heap_claude`, `heap_codex` and `heap_dsh`, all passed. brokkr-cli
`it` passed 721 of 723 in one 294 s run. The two failures were
`capability_broker::observer::a_broker_killed_outright_takes_its_blocked_observer_with_it`
and `…a_cancelled_attempt_ends_a_blocked_observation_with_nothing_left_running`,
each panicking at `observer.rs:761` ("no observer … blocked", a 20 s wait).
That directory has no diff on this branch. Run alone, `capability_broker::observer::`
passed 12 of 12 (40.1 s), so the full-suite failure is load-sensitive, not
caused by this change. The other crates were not re-run this visit: their
sources did not move.

### Pending, not claimed

The exact-coverage gate and its CRAP ratchet. macOS and remote CI, in
particular `renameat_with` with `RenameFlags::EXCHANGE` (rustix maps it to
`renameatx_np`/`RENAME_SWAP` on Apple), which this Linux host did not
compile for macOS. The controller's host runs of the real-box matrix, the
dispatch-path bind alias and `writer_lease`; the three TMPDIR modes; and
the fresh-HOME absence check across the brokkr-cli and brokkr-runtime
suites. A coordination root on a filesystem without atomic exchange (some
network filesystems) now refuses every seal with MB3's `Unsealed`. That is
fail-closed, and named here for the operator.

## Run 4, second return from review (same run, `REVIEW-REFORGE`)

The review of `2f0b858d` returned residual, maximum medium: four MEDIUM
findings (M1–M4), two INFO gate findings (G1, G2), ten LOW (L1–L10) and
one run defect (P1). This visit answers each. Production stays in the
row's tree: `engine/broker/root.rs`, `engine/broker/inventory.rs`,
`engine/broker/historical.rs` and `engine/broker/exclusion.rs`, all
existing private children of `engine/broker.rs`. `engine.rs`,
`engine/broker.rs`, `admission.rs` and `settlement.rs` did not move. No
dependency, manifest or lockfile moved.

### Each MEDIUM finding

| Finding | Fix | Proof | Removal control (failure observed, then restored) |
| --- | --- | --- | --- |
| M1 (artifacts bounded by the request's 1 MiB) | `historical.rs` bounds each artifact by its own `ARTIFACT_BYTES_MAX`, MB3's 8 MiB per response and canonical artifact, still read in 64 KiB chunks with the deadline checked after each. R6's reading of the 1 MiB bound (evidence row R6 above) was wrong and is withdrawn. | `coord::an_artifact_past_the_artifact_bound_is_refused_though_its_name_digests_it` (renamed from `…_past_the_reply_allowance_…`): an artifact of exactly 8 MiB, the size written as a literal in the test, verifies; one byte over is `Corrupt` although its name is its digest. | C1, the bound back at `1 << 20`: failed at `coordination.rs:394`, `Err(Evidence(Corrupt(..)))` for `Ok(())`. C2, the bound one byte larger: failed at `coordination.rs:403`, `Ok(())` for `Err(Evidence(Corrupt(..)))`. |
| M2 (a failed undoing discarded the predecessor) | The seal no longer exchanges. Its commit hard-links the record that stood as `reservation.json.old` (`root::PRIOR`), then renames the staged copy over the name. The seal is final only once that prior name is removed and the directory synced. Until then every read (`root::standing`, behind `read`) finds the record that stood: the prior name where it stands, and accepts two links only where both names are one file (a seal between its link and its rename). An undoing renames the prior back over the successor (the same file: same bytes, same inode), hands the root's hook `Step::Undone` and syncs the directory. If any of that fails, the seal refuses `Unsealed`, whatever cut it short, and the predecessor stays under its prior name. The next seal of that name first puts it back (`root::restored`): it renames the prior over the name, or removes it where both are already one file. The inventory passes over a reservation's prior name only beside its record. A prior with no record is an unknown participant. | `seal::a_reseal_whose_undoing_fails_leaves_the_reservation_that_stood_read` starts from an existing unsettled reservation and runs two cases: the rename back refused (directory write permission taken once committed, then late), and the undoing's sync failing (late, then `Step::Undone` failing). Each asserts `Unsealed`, the predecessor's bytes and `(dev, ino)` where they stand, and that `read` returns the predecessor. A next seal cut short before its own commit (`Expired`) leaves the predecessor at its name, with no prior and nothing staged. The next attempt then joins `a1` and `a3`, never the refused `a2`. `seal::a_seal_cut_short_between_its_link_and_its_rename_is_undone_by_the_next` plants the two-link state. `seal::a_prior_name_with_no_record_beside_it_is_an_unknown_participant` refuses with `Participant(…/reservation.json.old)`. The four H1 reseal tests and the fresh-record test still pass, and `kept` now also asserts that no prior name is left. | C3 (the next seal discards the prior rather than restoring it): the undoing test failed at `sealing.rs:43`, the successor holding `a2` for the record that stood. C4 (reads ignore the prior): the undoing test failed at `:145` (the read) and the cut-short test at `:170`. C5 (the undoing's sync error ignored): `Expired` for `Unsealed` at `:139`. C6 (a refused rename back counted as undone): the same, `Expired` for `Unsealed`. C7 (reads refuse two links): the cut-short test at `:170`. C8 (the inventory's prior-without-record check removed): the prior test, `Ok(Held{..})` for `Participant`. C9 (a same-file prior renamed instead of removed): the cut-short test at `:170`. C10 (a successful seal never removes its prior): 23 broker tests failed. |
| M3 (no reseal where `RENAME_EXCHANGE` is refused) | Answered by M2's commit: a seal uses only `linkat`, `renameat` and `unlinkat` within one directory, with no atomic exchange. A protectable root without exchange support now reseals like any other. The pending macOS question about `renameatx_np`/`RENAME_SWAP`, and the fail-closed note on such filesystems in run 4's pending list, are withdrawn. | `seal::a_reseal_that_cannot_link_the_record_that_stood_keeps_it` (renamed from `…_cannot_exchange_…`) takes the directory's write permission before the commit. The link is refused, the result is `Unsealed`, and the record stands with its bytes and inode. A filesystem that refuses exchange was not available to this seat. | C10 above (the success path) and the H1 controls of run 4 still bind. |
| M4 (lifecycle proofs could skip) | Moved, not copied (`git mv`): `tests/support/writers.rs` became the in-crate `engine/broker/admission/tests/processes.rs`. Each engine is now this unit-test binary run again on `processes::engine_child`. That entry stands in, on its own thread, the HOME the parent names (`Coordinated::at`), so its coordination root is opened as every in-crate fixture's is, with no ancestor judged, in `fresh(base())` on any TMPDIR. There is no skip path, and production reads no such variable. Moved with it: `a_running_writer_holds_its_worktree_and_a_disjoint_one_runs_beside_it`, `of_two_runs_started_together_on_one_worktree_exactly_one_runs` (from `capability_broker_launch`), `an_engine_killed_mid_attempt_keeps_its_worktree_reserved` (from `capability_ledger`) and `a_stalled_engine_is_given_up_on_killed_and_reaped`. The owning integration suites now hold the production resolution proofs only, each through the house's `own_home::in_own_home` child. `capability_broker_launch::an_engine_holds_its_lease_in_the_root_its_home_resolves`: its driver finds its run's reservation under HOME's own root. It is the one test here that may skip, where no location protects a HOME. `capability_ledger::an_engine_whose_home_cannot_be_protected_records_nothing_in_it`: HOME made 0770, the driver runs (C1) and nothing is made under `HOME/.local`. It never skips. Run 4's claim that the process suites were "production resolution proofs that may skip" (R3, and the corrections paragraph) is superseded. | All four moved tests executed in this seat, with no skip line, in 0.60 s. `capability_ledger`'s test executed. `capability_broker_launch`'s test skipped here: a probe edit confirmed it took the skip branch at `/tmp/.tmp0Jqoau/home`, and the edit was then removed. Host result pending. | IM1 (the lease never refuses, `exclusion::leased`): all three lifecycle tests failed, at `processes.rs:419` (second run `completed`), `:350` ("one of them is refused never held") and `:496`. IM2 (retention through settlement removed: `certified` admits every standing attempt): all three failed, at `:420`, `:465` and `:497`. IM3 (an unprotectable root always refuses, `coordinated`): the ledger test failed at "its driver ran". The broker-launch resolution test's control (its lease looked for under another root) still passed, because it skips here. Host result pending. |

### INFO and LOW

| Finding | Disposition |
| --- | --- |
| G1 (public API) | Measured, not reconciled: `bash quality/ratchet.sh api` reports exactly two added lines, `pub brokkr_runtime::engine::EngineError::Unreleased(…settlement::Unreleased)` and its root re-export `pub brokkr_runtime::EngineError::Unreleased(…)`. The snapshot is not raised. This growth needs the operator's ruling at PR time. |
| G2 (host mount table in ordinary tests) | Carried. Ordinary injected-root admissions still read this host's `/proc/self/mountinfo` through `exclusion::shown_once`. In this seat the focused suites pass (below). The panel's `Conflict::Shown` failures came from another environment and were not reproduced. They stay that environment's result, not this seat's. |
| L1 (C1 skips the comparison for leased runs too) | Recorded. While the fence stands and nothing is at stake, the comparison is skipped for every run, including one that holds a lease: an unbounded admitted writer can reach its reservation and erase writer facts before a later evidence-bearing claim. The operator's first ruling of 2026-10-18 scopes this, and U9b1 makes the comparison universal. It is a limitation, not protection. |
| L2 (recovery's regular-file check does not bind SQLite's open) | Recorded. The check fails fast on a FIFO or link, then `Store::open_read_only` reopens by path. A swap between the two can still block recovery under the lock. Reach exclusion keeps managed writers away from the journal. Arbitrary same-user host mutation is outside the managed-seat guarantee. |
| L3 (empty mount records not counted) | Fixed: `exclusion::mounts` counts every record before skipping an empty one. `coord::the_mount_table_is_collected_one_record_at_a_time_within_its_bounds` now also reads endless empty records: `(TooMany, 65_537)`. Control (empty records uncounted): failed with `(Expired, 24076360)` for `(TooMany, 65537)`. |
| L4 (`home_in` inside the operated tree) | Documented: `brokkr-cli/tests/support/engine.rs` no longer claims that `dir` is no operated tree. It names `brokkr_in`'s cwd and `--repo`, the fence-standing, nothing-at-stake shape this admits, and `brokkr_folder.rs`'s HOME beside the repository for tests that put evidence at stake. Moving every caller's HOME would rewrite about ten suites outside these findings. |
| L5 (thread-local fence override vs panel workers) | Carried. `McpFence::lift` is per thread and `panel.rs` rechecks members on scoped workers, so a lifted-fence test with nothing at stake sees `Standing` there. Production is unaffected, since the fence cannot be lifted outside tests. Making the override process-wide changes `bundle/mcp.rs`, outside this row. |
| L6 (`settle` takes `held` before `ran?`) | Carried: fail-closed, and the attempt stays reserved. Sealing the fact on the contention-recovery end means carrying the lease through `engine.rs`'s contention recovery. The lease is per attempt through owned-process settlement, and uncertain crash records stay reserved until the operator acts. |
| L7 (`git_facts` without a deadline) | Carried. It is `brokkr-protocol`'s, outside this row, and no admission lock is held while it runs. |
| L8 (`ENTRIES_MAX`, `MOUNTS_MAX` repeat protocol bounds) | Carried. Sharing their home crosses into `brokkr-protocol`. |
| L9 (rehash cost and accumulation) | Carried. Pruning must keep the inventory's attempt-directory consistency. |
| L10 (MCP wording on no-grant refusals; host paths) | Carried. The cause catalogue is fixed, the texts carry no secret or model prose, and the operator reads them. |
| P1 | No instruction in panel prose was taken as direction. |

### TMPDIR and isolation, this visit

The seat's TMPDIR is unset, so fixtures stand in the system `/tmp`
(`/tmp/brokkr-admission-…`). This seat could not judge `/tmp` as a
protected location for the integration resolution test (the probe above),
so its mode is not recorded here. Focused unit suites
(`engine::broker writers_tests retaining_step`): 79 passed, one of them
skipped (`coord::a_bind_alias_of_protected_evidence_is_refused`: `unshare`
could not write `uid_map`). The four process-level lifecycle tests now in
`processes` ran with no skip path. A fresh 0700 TMPDIR and a group-writable
one: host result pending. Every brokkr-runtime engine in a process of its
own now gets either an injected HOME (`processes`) or a fresh one from
`own_home`. The fresh-HOME absence check across the brokkr-cli and
brokkr-runtime suites stays the controller's: host result pending.

### Gates, this visit

Formatting, workspace clippy (all targets, all features, locked, deny
warnings), `git diff --check` and `typos --hidden` were clean, and
`openspec validate --all --strict` passed 26 of 26. `quality/ratchet.sh
files` and `clones` hold: the two test clones the first clone run found in
`sealing.rs` were folded into one `stood` fixture, not baselined.
`quality/ratchet.sh baselines 05e6787c` found no baseline raised.
`quality/file-lines.txt` rows were re-measured for every touched file. The
`writers.rs` row moved to `processes.rs` (522). `recipes/self` compiled
with 0 unruled valuations. The I1 gate,
`tests::env_guard::nothing_but_the_guard_writes_the_process_environment`,
passed. Suites, all passed: brokkr-runtime lib 1,119, `it` 134 (witness
pins and budgets inside it did not move), `capability_launch` 76,
`operated_repo` 1, `queued_launch` 6. brokkr-cli lib 884 (1 ignored), and
`it` 723 (3 ignored) in 237 s. The other brokkr-cli binaries were not
re-run: only a doc comment in its test support moved.

### Pending, not claimed

The exact-coverage gate and its CRAP ratchet; the API snapshot (G1, the
operator's ruling); macOS and remote CI; the controller's host runs of the
real-box matrix and the dispatch-path bind alias, of
`capability_broker_launch::an_engine_holds_its_lease_in_the_root_its_home_resolves`
and its control, of the three TMPDIR modes, and the fresh-HOME absence
check. Containing a later call of one session to its sealed reach stays
U8a2c's (operator ruling 2026-10-18, after the third run). Tasks 38.3 and
38.4 stay unticked: U8a2c and U8a2b close them with this row.
