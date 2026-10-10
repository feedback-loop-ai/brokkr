# U6c6b evidence: the box launched on its waiting bootstrap, and its readiness received behind the fence

U6c6b took three runs. The first, `0065-slice-two-unit-u6c6b-see-th-7a124092`
(from main at `3318d662`), built the launch and the receiver over three
visits and stopped on its review's M1. The controller stopped and discarded
the second run. The third, `0065-slice-two-unit-u6c6b-see-th-40cd1437`,
delivers the unit on branch `s2/U6c6b` from main at `90d94dd8`. It works
under the operator's split of 2026-10-09, the standing rule for consumed
child modules, and the addendum of 2026-10-10, which lets `hands.rs` name
`ServerEntry` in its existing re-export and rules the public API from 945
to 948. The third run kept the first run's work and answered M1 and L4. It
also added the process-group guard the controller asked for. Where the
first run's text no longer held, it is rewritten.

The third run's review, of `86ce316a`, returned M1 (the launcher's own
start was outside the startup's deadline and cancellation) and L1 (the
generated identity tree had no owner between its handover and the
broker's decoding of the record), and carried L2 and L3. The third run's
return visit answers M1 and L1 in the same commit, and its sections below
are marked "return visit".

A result marked R1 was observed in the first run, on code the third run
did not change. Every other result was observed in the third run, on the
tree its commit records; "return visit" marks those its second visit
observed, on the final tree. Anything not observed is marked pending.

## What changed, by file

The row's four production files are the only production files touched,
and two of them are the new consumed children the row names. Outside the
row, the only edit is one name added to `hands.rs`'s existing
`pub use namespace::{…}`. That is size-neutral at 1,170 lines because the
statement keeps one line under a same-line `#[rustfmt::skip]`, as the
2026-10-10 addendum admits.

`crates/brokkr-protocol/src/hands/namespace.rs` (798 lines) registers the
child `entry`. It keeps the box's argv up to its entry: the box no longer
closes on the dialect program. `ServerProfile` no longer carries the
program's arguments, which had no reader left; U6c8's exec brings them
back with their consumer. `ServerBox` keeps the bootstrap's path and the
checked launcher's found path, which `prepare_with` reads from the launcher
check. It holds its scratch session as an `Option` so the entry can hand
the tree over. Scratch creation and identity generation take
`Refusal::Establishment`, not `Identity` (U6c5a's council, L3). The `min`
of identity setup's cause and the observer's still puts every observer
cause first.

`crates/brokkr-protocol/src/hands/namespace/entry.rs` (new, 497 lines
after the return visit) holds the entry and the launch.

- `ServerEntry` is public, closed with `deny_unknown_fields`, and has
  private fields: the argv, with each `--ro-bind-fd` number replaced by
  its handle's place among `ServerBox::handles`; the launcher's place; the
  bootstrap; and the generated identity's tree.
- The tree is a private `Identity` newtype, transparent on the wire. Its
  `Drop` removes the tree only where it is this process's: directly in the
  temporary directory sessions are made in, its name naming this process
  the owner, or (since the return visit) in the process that made the
  entry. So the broker's copy removes it whatever the broker answers, and
  the observer removes it only where it could not hand it over.
- Because the `Drop` sits on a private field type, the public listing
  gains only the struct line.
- `ServerBox::entry(owner)` returns the `ServerEntry`.
- `ServerBox::launch(&ServerEntry, …)` parses nothing. It refuses, as
  identity unprotected, an entry whose tree is not this process's, or one
  that names a handle it was not given. It runs the checked launcher
  through `/proc/self/fd/<its handle>`, from `/`, with a cleared
  environment and every standard stream on `/dev/null`. The launcher stays
  in the broker's own process group and session (no `--new-session`;
  `--die-with-parent` kept).
- The launch enters the bootstrap from `/runtime/home` through
  `/usr/bin/env -u PWD` and adds `--info-fd`.
- Across the exec, the started process alone keeps open exactly the
  mounted handles, the bootstrap's control read end and ready write end,
  and the info write end. A `pre_exec` hook first marks every descriptor
  from 3 up close-on-exec (`close_range` with `CLOSE_RANGE_CLOEXEC`), then
  clears the flag on those alone, in the child only.
- `Launched<'e>` borrows the entry it launched, so the entry, and with it
  the tree, outlives the box. Dropping `Launched` kills and reaps the
  launcher, which the box does not outlive, and closes the pipes.

The return visit bounded the launcher's own start (M1). `ServerBox::launch`
takes the startup, the absolute deadline and the cancellation, and starts
the launcher through `started`:

- The fork and exec are made on the calling thread. A watcher on a scoped
  thread, joined before `started` returns, polls in 10 ms slices. No
  launch worker outlives the call.
- The started process's first hook (`reporter`, then `reported`) writes
  its pid on a report pipe and waits for one byte on a release pipe. The
  watcher opens a pidfd of that pid while the process is still blocked
  and unreaped, and only then writes the release byte.
- A start that overruns before its release gets any other byte, fails its
  own start, and never runs the launch's own hook or the launcher. One
  that overruns after it, stalled in its exec or stopped, is killed through
  its pidfd and reaped by `started`. Both are `Refusal::Establishment`. A
  launcher that cannot start in time stays `Unavailable`.
- Only the process `started` forked is ever signalled, and only through a
  pidfd of it. No group, pid 0 or negative pid is signalled.
- `inherit` folds its two system calls into one result, so neither has an
  arm no test reaches.
- `Identity` carries a `made` flag that is never on the wire, so the entry
  `ServerBox::entry` made removes its tree when dropped (L1).

`crates/brokkr-cli/src/broker/session.rs` (786 lines after the return
visit) registers `readiness`. It keeps admission's absolute deadline and
cancellation as one `Startup` parameter object, shared by the observer and
readiness. Since the return visit, the cancellation is `Sync`, because the
launch's watcher asks it from its own thread.

- The observer's closed record carries `entry: Option<ServerEntry>`,
  beside `handles`. The entry is decoded with the record, once, at the
  record's edge (`helper::received`), and no JSON text crosses.
- `recorded` destructures the after-box answer by value, so the entry
  moves into the record and is never cloned.
- The observer's closed cause list carries `establishment` after
  `store-in-box`.
- `helper::tethered` now refuses `Identity`, before any watcher is armed,
  unless `getpgid(None) == getpid()`. A watcher that kills the current
  group therefore only ever kills the observer's own group.
- `Admitted` and `compared` moved into the child to make room.
- Return visit (L1): `helper::received` decodes the record as soon as its
  message arrives, before it waits for the observer's end. So a deadline,
  a cancellation or an excess message after the record drops the decoded
  entry, and its tree with it. `helper::handed` takes the record by value.
  It drops a record it could not hand back, and so its tree, and holds one
  it handed back in `ManuallyDrop`, because the tree is then the broker's.
  The crate denies `clippy::mem_forget` for the fault-seam `Guard`; the
  record is not one.

`crates/brokkr-cli/src/broker/session/readiness.rs` (new, 427 lines) holds
`Admitted` (its `entry` a `ServerEntry`), `compared` and, on Linux, the
receiver.

- A refused comparison drops the record, so drift in sources or writers, a
  handle mismatch or a missing entry each remove the handed tree (L4).
- The receiver launches the box under the startup. It is unavailable where
  the launcher cannot start, and not established where its start overruns
  (return visit) or a started launcher names no first process.
- It writes the sealed intent frame: plan and source digests, projected
  network, the broker's own namespaces, and each mount's box path with its
  handle's device and inode, at most 4 KiB.
- It reads the ready pipe until every writer has closed it, at most 4 KiB.
  Every write and read waits in 10 ms slices under the one startup
  deadline and cancellation.
- It parses one closed `Ready` and decides: both digests echoed, the first
  process the launcher's own child, and the bootstrap the message names by
  its pid in the box, in exactly the namespaces the host shows. Any other
  answer is the establishment cause.
- Off Linux, `ready` refuses as unavailable.

## Findings this unit met, and what it did

bubblewrap always exports `PWD`: after `--clearenv` it `setenv`s the
directory it entered. U6c6a's bootstrap admits MB4's eight fixed names
alone, so on main it could never report ready in a real box. The entry
therefore runs the bootstrap through the system set's `env` with `PWD`
unset. The live tests below prove the environment is exactly the eight
names.

bubblewrap's `--ro-bind-fd` resolves `/proc/self/fd/<n>` with `realpath`
and mounts the path it names, so a descriptor whose file was unlinked
cannot be mounted. The entry therefore hands the generated identity's tree
over. It renames the tree to name the observer's parent, the broker, as
owner, so the dead-session reaper leaves it while the broker lives. With
the typed entry, the broker owns cleanup from the moment it decodes the
record. A comparison refusal, a failed launch and a settled box all remove
the tree.

The return visit closed most of the gap the review's L1 named, the
interval in which the tree had no live owner:

- A record the observer cannot hand back is dropped, so the observer
  removes the tree, because its entry is the one `ServerBox::entry` made.
- A record the broker has received is decoded at once, so the broker owns
  the tree before it waits for the observer's end. A deadline, a
  cancellation or a message more after the record removes it.

A narrower residual remains, and the earlier claim that it was "bounded by
the broker's life" was wrong. The tree is left behind in three cases: the
observer's group is killed (at the deadline, on cancellation, or because
the broker ended) after the rename and before the send; the broker gives
up with a record queued but unread; or the observer sends a record the
broker cannot decode, which this observer never writes. In each case the
tree, named for the broker, is no longer removed by anyone. A later run,
resume or rerun's dead-session reaper may take it once the broker has
ended, and until then it stays. bubblewrap resolves a descriptor mount to
its path, so the generated files must outlive the observer, and the
handover cannot simply be dropped.

The security review's related note stands: the observer's forgotten
session lock ends with the observer, so a reaper in another PID namespace
that shares the temporary directory could take the tree while the broker
lives. That fails closed, as an establishment refusal.

The return visit met one more fact. Linux sends a parent-death signal,
which bubblewrap's `--die-with-parent` arms, when the *thread* that
started the process ends, not the process. The visit's first draft forked
the launcher on a scoped thread, and the live `capability_broker::` suite
then failed twice on two different real-box tests, refused as not
established. The second failure read `("restored", (Some(1), "error: MCP
server box could not be established\n"))` against the serving refusal.
The fork now stays on the calling thread, a test binds it (below), and the
suite then passed twice in a row.

The second run's in-progress launch work SIGKILLed its own process group
twice, by the controller's reading through `tethered`'s watcher armed in a
process that led no group of its own. The guard makes that refusal
structural. The tests that started `broker observe` directly (the relay
and the own-broker case) now give it a process group of its own, as
`supervised` does.

## Public API and module growth

The operator ruled `quality/public-api/brokkr-protocol.txt` from 945 to 948
items on 2026-10-10. The third run measured exactly that with
`quality/ratchet.sh api`, which reports that the public API holds against
the updated snapshot. The 948 items break down as follows:

- `+ServerBox::entry` and `+ServerBox::launch`.
- `+ServerEntry` through the re-export.
- A second `impl ServerBox` line, because the two functions live in the
  child's impl block.
- `-ServerProfile::arguments`.

`baselines origin/main` refuses only that raise (948, was 945), which the
PR's `Ruling:` line names.

I1 is recorded, not changed: `Launched` is still reached through
`launch`'s return value, at its private module path. Naming it in the
re-export would add its struct line, its five public fields and its `Drop`,
beyond the ruled 948.

The new child modules are the two the row names. `entry` is declared
`pub(super)`, which is what lets `hands.rs` name `entry::ServerEntry`. The
new test modules are `hands/tests/entry.rs`,
`broker/session/readiness/tests.rs` and
`tests/capability_broker/readiness.rs`. No test moved between files.

## Tests

| Suite | Test | What it proves |
| --- | --- | --- |
| protocol `hands::tests::server` | `a_server_box_stands_without_the_seats_or_the_hosts_private_paths` | a real box launched through `ServerBox::launch`, a shell standing for the bootstrap: the box checks still hold, the environment is exactly the eight fixed names, bubblewrap's info names the launcher's own child, the waiting process holds `/dev/null` on 0–2, the two control pipes at the very numbers it was given, and nothing else. Dropping `Launched` ends it. The tree stands until the entry is dropped, then is gone (third run). Ran in 1.58 s, no skip |
| protocol `hands::tests::entry` | `an_entry_names_each_mounted_handle_by_its_place_and_hands_its_identity_over_once` | the entry's wire form, word for word. The tree is renamed to its owner and the handles re-pointed. A second entry is refused. Return visit: a copy decoded in a process the tree was not handed to leaves it, and the entry its maker drops, one it could not hand over, removes it |
| protocol `hands::tests::entry` | `a_launch_starts_within_the_startup_or_never_runs_and_is_settled` (return visit) | in time, a started shell runs. A startup already past its deadline, or cancelled, never releases it: `Establishment`, the launch's own hook never ran (nothing on its pipe) and the shell never ran. Released, a start stalled five seconds before its exec is killed at a 200 ms deadline, or 200 ms into the attempt by cancellation: `Establishment`, the shell never ran, no `/proc/<pid>` left of the stalled process, under 2 s. Ran in 0.42 s |
| protocol `hands::tests::entry` | `a_launcher_that_dies_with_its_parent_outlives_its_own_start` (return visit) | a launcher whose hook arms `PR_SET_PDEATHSIG` with `SIGKILL`, as bubblewrap's `--die-with-parent` does, ends with code 0 and no signal |
| protocol `hands::tests::entry` | `a_started_launcher_names_itself_and_runs_on_only_its_release` (return visit) | the first hook, run in this process so it is measured, writes this process's pid. It returns `Ok` on the release byte, and `ECANCELED` on any other |
| protocol `hands::tests::entry` | `the_launch_hook_run_in_a_process_of_its_own_seals_all_it_does_not_name` (return visit) | rerun as itself, so the hook seals only that process: the named descriptor's close-on-exec flag is `0`, a stray's `FD_CLOEXEC`, and a descriptor that is not open gives `EBADF`. This is the in-process measure of the hook the shell test below drives in children |
| protocol `hands::tests::entry` | `a_launch_that_cannot_stand_refuses_by_its_own_cause_and_its_entry_removes_the_identity` (renamed) | a missing handle takes `Identity`, and a launcher that cannot start `Unavailable`. A tree outside the temporary directory, or there naming another owner, takes `Identity` and stays standing even once its entry is dropped. Whatever the launch answered, the tree stands until the entry is dropped |
| protocol `hands::tests::entry` | `an_entry_is_one_closed_shape` (third run) | an entry decodes and re-encodes exactly. A key beside the four, or a word of another kind, is a data error |
| protocol `hands::tests::entry` | `the_launch_hook_leaves_open_only_the_descriptors_it_names_and_reports_the_systems_error` | in started shells, with a stray descriptor made inheritable: unhooked, `0 1`; hooked, `1 0`; an unopened descriptor gives the spawn's `EBADF` |
| protocol `hands::tests::entry` | `a_scratch_that_cannot_hold_the_identity_leaves_the_box_not_established` | rerun with `TMPDIR=/proc/u6c6b`: `Establishment`, with a multiply-linked program file's `Linked` still first |
| cli lib `readiness::tests` | `the_entry_crosses_closed_and_every_refused_comparison_removes_its_identity_tree` (third run) | the record whose entry travels as JSON text is a data error. A record with drifted sources, and one whose handles are not those it names, are each refused as `Identity` with the handed tree gone. An admitted one keeps the tree until `Admitted` is dropped |
| cli lib `readiness::tests` | `a_record_its_end_does_not_follow_removes_the_identity_tree_it_carried` (return visit) | over a real sequenced-packet pair: a record whose end follows is decoded and keeps its tree until it is dropped. One followed by no end before a 200 ms deadline, by a cancellation after the record, or by one message more is refused `Identity` with the tree gone |
| cli lib `readiness::tests` | `a_record_the_observer_cannot_hand_back_removes_the_identity_tree_it_carried` (return visit) | a record over the socket's 128 KiB bound: `handed` refuses `Identity`, and the tree its entry carried is gone |
| cli lib `readiness::tests` | nine more | the 4 KiB bound, the passed deadline, cancellation, one closed message against duplicate, excess, truncated, unknown-field and empty bytes, a wrong plan, sources, child, an unnamed bootstrap and each of the five namespaces, bubblewrap's info, and the intent frame's bytes and bound. Through `ready`: a launcher that cannot start (`Unavailable`) and `/bin/false` (`Establishment`). The intent sent whole when drained late, and refused at a 200 ms deadline and on cancellation |
| cli `capability_broker::readiness` | `a_real_box_reports_ready_and_serving_still_refuses_with_nothing_of_it_left` | the real binary in a real box: still `broker serving protections are incomplete`, zero store reads, no box process left, no start. Third run: under a fresh owner-only `TMPDIR`, the admitted serve and a serve whose sealed sources digest drifted (`Identity`) each leave that directory empty |
| cli `capability_broker::readiness` | `a_box_held_at_its_entry_refuses_at_the_absolute_deadline` | strace holds the box's entry: the establishment cause between 30 s and 32 s, box gone, zero store reads, no start |
| cli `capability_broker::readiness` | `a_cancelled_attempt_ends_a_box_held_at_its_entry` | the starter killed while the box is held: the establishment cause within 5 s, box gone, zero store reads, no start |
| cli `capability_broker::readiness` | `unavailable_support_refuses_before_any_lookup_or_start` | no launcher on the broker's `PATH`: `Unavailable`, zero store reads, no start |
| cli `capability_broker::observer` | `an_observer_leading_no_process_group_of_its_own_refuses_before_its_tether` (third run) | `broker observe` started by the test, which made the socket and is its parent, but in the test's own process group: `error: MCP server box filesystem identity is not protected`, exit 1, no record. The test holds its socket end until the observer has exited and then lives on |
| cli `capability_broker::observer` | `the_observer_hands_back_…`, `relays_…`, `an_observer_its_broker_did_not_start_…` | the record's seven fields, `entry` now a closed object. The relay and the own-broker case give the observer its own process group |
| cli `capability_broker` | `unprovable_or_unobserved_writers_…` | a scratch that cannot be made (`TMPDIR` a file) reaches the broker as `Establishment` |

## Removal controls

Each mutation compiled, made the named test fail at the named assertion,
and was restored, and the suites passed again afterwards. The third run's
controls come first, observed in this run.

| Mutation (third run) | Failing test and assertion |
| --- | --- |
| `Identity`'s drop removes nothing | `the_entry_crosses_closed_…`: `("drift", Err(Identity), true)` against `false`; `a_real_box_reports_ready_…`: one tree left against 0 after the admitted serve; `a_launch_that_cannot_stand_…`: `!scratch.exists()` after the drop |
| `deny_unknown_fields` dropped from `ServerEntry` | `an_entry_is_one_closed_shape`: case `extra` decoded `Ok(ServerEntry {…})` against `Err(Data)` |
| the group-leader guard always holds (`… \|\| true`) | `an_observer_leading_no_process_group_…`: `((Some(0), ""), {"refused": "unbound"})` against the identity refusal and no record; the seat's own group lived on |
| `Identity`'s drop removes any tree (`ours(…) \|\| true`) | `an_entry_names_…`: the tree handed to another owner was gone after the drop; `a_launch_that_cannot_stand_…`: the outside tree `(…, Err(Identity), false)` against `true` |
| the owned-child check always holds (re-observed) | `only_the_owned_bootstrap_…`: the other-child case `Ok(())` against `Err(Establishment)` |
| the namespace check always holds (re-observed) | the same test: the unnamed-bootstrap case `Ok(())` |
| the digest check always holds (re-observed) | the same test: the wrong-plan case `Ok(())` |

The return visit's controls follow. P1 to P5 were observed twice, before
and after the fork moved to the calling thread; the table gives the
second observation, on the final code.

| Mutation (return visit) | Failing test and assertion |
| --- | --- |
| P1: the watcher ignores the deadline | `a_launch_starts_…`: `("deadline", Ok(()), ([114], true))` against `("deadline", Err(Establishment), ([], false))` |
| P2: the watcher ignores cancellation | the same test: `("cancelled", Ok(()), ([114], true))` |
| P3: a released start that overruns is not killed | the same test: `deadline: 5.010964683s`, against the 2 s bound |
| P4: an overrun start is killed but not reaped | the same test: `("deadline", Err(Establishment), false, true)`, its `/proc` entry left |
| P5: a start that overruns before its release is sent the release byte | the same test: `("deadline", Err(Establishment), ([114], false))`. The test's first form checked only the shell's marker, and this mutation survived it: the launcher ran and was killed before it touched the marker. The test now also reads the launch's own hook's pipe |
| P6: the first hook releases on any byte | `a_started_launcher_…`: `(0, Ok(()), …)` against `(0, Err(Some(125)), …)`; `a_launch_starts_…`: the deadline case's hook ran (`[114]`) |
| P7: `Identity`'s drop ignores `made` | `an_entry_names_…`: `!scratch.exists()` after the maker's drop |
| P8: the fork made on a scoped thread of its own | `a_launcher_that_dies_…`: `(None, Some(9))` against `(Some(0), None)` |
| P9: `close_range` seals nothing (it starts at `u32::MAX`) | `the_launch_hook_run_in_…`: the rerun exited unsuccessfully; `the_launch_hook_leaves_open_only_…` also failed at its hooked case |
| C1: `received` decodes the record after the observer's end | `a_record_its_end_…`: `("deadline", Err(Identity), true)` against `false` |
| C2: `handed` relinquishes the record before its bound check | `a_record_the_observer_…`: `(Err(Identity), true)` against `(Err(Identity), false)`, observed with `mem::forget` and again with `ManuallyDrop` |

The first run's controls follow. Each was observed in R1, on code the third
run did not change except where noted. Its suites passed again in the
third run.

| Mutation (R1) | Failing test and assertion |
| --- | --- |
| scratch creation answers `Identity` | `a_scratch_that_cannot_hold_…`: the rerun failed |
| identity setup's cause returned without `min` | the same test: the `Linked` case |
| the entry keeps a descriptor number instead of a place | `an_entry_names_…`: the entry equality |
| no rename to the owner | `an_entry_names_…`: `(made.exists(), scratch)` |
| a second entry answers `Identity` | `an_entry_names_…`: the second-entry equality |
| a launcher that cannot start answers `Establishment` | `a_launch_that_cannot_stand_…`: the `Unavailable` equality |
| the hook sets no flag; the carried descriptor set close-on-exec | the live launch; `the_launch_hook_leaves_open_only_…`, `Ok("1 1\n")` against `Ok("1 0\n")` |
| the hook seals nothing | `the_launch_hook_leaves_open_only_…`: `Ok("0 0\n")` against `Ok("1 0\n")` |
| no `env -u PWD` | the live launch and `a_real_box_reports_ready_…` |
| the info pipe's read end also carried | the live launch: an extra pipe in the descriptor list |
| `Launched`'s drop neither kills nor reaps | the live launch: `ended(waiting)` |
| the ready bound one byte wider; the reader returns its first chunk | `readiness_is_every_byte_…` |
| the reader takes a fresh 30 s deadline | `a_read_past_the_absolute_deadline_…` |
| cancellation ignored | `a_cancelled_attempt_refuses_…` and the live `a_cancelled_attempt_ends_a_box_held_…` |
| the first JSON value taken; one trailing byte tolerated; a truncated message parsed | `the_ready_message_is_one_closed_message`: cases `duplicate`, `excess`, `truncated` |
| the source digest unchecked; `uts` alone unchecked | `only_the_owned_bootstrap_…` |
| no first process answers `Unavailable` | `bubblewrap_info_must_name_…` and `a_launcher_that_cannot_start_…` |
| `establishment` dropped from the observer's cause list | `unprovable_or_unobserved_writers_…`: the scratch case |
| the send's deadline an hour away; its cancellation ignored | `the_intent_is_sent_whole_or_refused_…` |
| the store read before readiness is decided; on every refusal after observation | `a_real_box_reports_ready_…`; `a_box_held_at_its_entry_…`, `a_cancelled_attempt_ends_…`, `unavailable_support_…` |
| the identity tree's directory check, or its owner check, dropped | `a_launch_that_cannot_stand_…`: the outside, or the other owner's, tree launched |
| the intent bound one byte wider | `the_intent_is_one_frame_…` |
| admission does not await readiness | `a_box_held_at_its_entry_refuses_…` |
| the intent's device numbers swapped; its network inverted | `a_real_box_reports_ready_…` |
| the observer's record omits the entry | `the_observer_hands_back_…` |

Two R1 controls bound behaviour the third run replaced. In the first, the
session's drop ran on handover, and launch removed the tree: the drop now
belongs to the entry, and the first row of the third run's table binds it.
In the second, the identity guard was taken after the handles: launch now
removes nothing, and the third run's last `Identity` row binds the
ownership instead.

## The reviews' findings

The first run's first review returned M1–M5. M1 (`establishment` missing
from the observer's causes), M2 (no first process is not established), M3
(zero lookups measured on an admitted store) and M4 (the intent sent under
the deadline) were fixed in R1 and stay fixed. M5, the entry as JSON text,
was the second review's M1, and the third run fixes it.

The second review's findings:

| Finding | Disposition |
| --- | --- |
| M1 (ruling 3): the entry crossed protocol and CLI as JSON text | Fixed in the third run. `ServerEntry` is carried through `ServerBox::entry`, `Observation`, `Admitted` and `ServerBox::launch`, and decoded once with the record |
| L1: inherited descriptors could enter the box | Fixed in R1 with `close_range` |
| L2: the receiver restates the bootstrap's handshake vocabulary and `FRAME_MAX` | Moved to U6c8 by the 2026-10-10 addendum, since `broker/bootstrap.rs` is U6c8's |
| L3: `ServerProfile.arguments` had no reader | Removed in R1 |
| L4 (MB5): refusals after the handover left the tree | Fixed in the third run. The entry owns the tree, and a refused comparison removes it, as bound above |
| I1: `Launched` is reached through a private module path | Recorded, as above |
| I2: the counts | Measured again in the third run |

The third run's review, of `86ce316a`:

| Finding | Disposition |
| --- | --- |
| M1 (MB3; ruling 9): the launch's own spawn was outside the startup's deadline and cancellation | Fixed in the return visit. The start is bounded, its stall killed through a pidfd and reaped, and no launch worker outlives it. P1 to P5 bind it |
| L1 (MB5): the tree had no owner between its handover and the broker's decoding | Fixed for a failed send and for anything after the record arrives. The narrower residual and the corrected lifetime claim are above |
| L2 (ruling 5): `entry.rs` restates the session-name vocabulary of `hands/session.rs::owner_pid`, and matches `-<pid>-` as a substring | Carried, as the review ruled. `owner_pid` is private to `hands/session.rs`, outside the row. The substring can match a four-digit observer pid inside the uuid after handover, so the observer would remove the broker's tree. That fails closed, as an establishment refusal. The follow-up is one shared parse |
| L3 (ruling 5): the receiver restates `FRAME_MAX` and the handshake types | Carried to U6c8 by the 2026-10-10 addendum |
| R1: panel prose that claimed gate authority | Recorded by the chief as a run defect; nothing to change here |
| INFO: the TIOCSTI residual; `Launched`'s private path; no `/proc/<pid>/task/<pid>/children` refuses `Establishment` | Unchanged and recorded |

Both operator addenda, the 2026-10-09 standing rule and the 2026-10-10
re-export ruling, ride in this unit's commit.

## Delivery gates

The third run's return visit took each gate again on the final tree.

| Gate | Result |
| --- | --- |
| format check, then workspace clippy with all targets and features, locked, denying warnings | clean, after a test type alias for the startup and `ManuallyDrop` in place of `mem::forget` |
| `cargo +1.88 check --workspace --all-targets --all-features` | finished |
| brokkr-protocol | 849 lib tests passed (3 ignored); `hands_exits` 6, `secret_drop` 1, the doc test 1. `hands::tests::` 109 (1 ignored) and `hands::tests::entry` 9 after the last change |
| brokkr-cli | 661 lib (1 ignored); `it` 527 (2 ignored); `capability_broker::` 38, twice running; `driver_conformance` 27; `transcript_surfaces` 13; the three heap tests |
| brokkr-runtime | 900 lib, `capability_launch` 75, `it` 127, `operated_repo` 1, `queued_launch` 3 |
| core, view, store, bridge, seatbelt-probe | all passed |
| `quality/ratchet.sh` for files, clones and the API | file size holds, duplication holds, the public API holds; `launch`'s snapshot line takes the startup |
| `quality/ratchet.sh baselines origin/main` | refuses only the ruled public API raise, 948 against 945 |
| `cargo crap` over `hands/namespace` and `broker` | every function in the changed files at cyclomatic complexity 12 or below; `ServerBox::launch` 11, `ready` 10, `ServerBox::entry` 9, `started` 7, `watched` 6 |
| `cargo llvm-cov` over `hands::tests::entry` and the live server launch (a diagnostic, not the exact gate) | `entry.rs`: all 31 functions hit and no line record unhit. Before the return visit, `inherit` and its hook closure were never hit, because they ran only between fork and exec |
| `cargo llvm-cov` over the CLI's `broker::session` unit tests and two live suite tests (a diagnostic) | the changed lines of `received` and `handed` are hit, the `ManuallyDrop` line through the real observer |
| `compile --bundle recipes/self` | compiled |
| `openspec validate --all --strict` | 20 of 20 passed |
| `typos --hidden`; `git diff --check` | nothing found |

The rows in `quality/file-lines.txt` for every new and changed file are set
to `wc -l`'s count. No function crossed 100 lines and no suppression was
added; `#[rustfmt::skip]` is a formatting attribute, not a lint
suppression. So `too-many-lines.txt` and `suppressions.txt` are unchanged.
No witness or compose input moved, and the runtime's `witness_digests::`
passed without blessing inside `it`'s 127.

Pending, outside the seat:

- The owner-only `TMPDIR` run of the suites. This seat may not create a
  directory outside the worktree (the return visit's `mkdir -m 700` under
  `/var/tmp` was refused), so the suites ran with its default. The
  real-box control above makes its own owner-only directory.
- `bash scripts/coverage-exact.sh`, with branch coverage. The diagnostics
  above ran without `--branch`.
- The `lychee` link check, which this seat may not run.
- CI on both operating systems.

The macOS target's standard library is not installed here, so the
non-Linux stand-ins were checked by review only. `ours`, `Identity` and its
`Drop` are no longer Linux-gated, and they use only `std`. Everything the
return visit added to `entry.rs` for the bounded start, with its imports
and constants, is behind `cfg(target_os = "linux")`, as `launch` is.

`setsid --wait` is not permitted in this seat. So before running any
observer test, the third run checked by reading the code that every
observer is either its own group's leader or refused before arming. The
return visit did not change `tethered`. The only process it signals is
the launcher its own start forked, through a pidfd of it.
