# 0080 — An attempt is its own cgroup: Linux ends a seat's tree by containment, and polls only where no containment is offered

Status: proposed (implementer, 2026-10-10)
Date: 2026-10-10

Built: unbuilt (#472)

## Context

Decision 0006 kills a timed-out attempt, and its addendum (#403) made the
kill take the attempt's whole process tree by reading the process table:
the driver leads a session and group of its own, the engine and every
driver are Linux child subreapers, a tracker records every descendant by
pid and start stamp every 100 ms, and every read fails closed. Five
review rounds each found a narrower race. The fifth is inherent to
polling. A Linux read is a `/proc` listing followed by one stat read per
pid, so a descendant forked after the listing, whose parent exits before
its own row is read, is filed as the engine's own and cleanup is
certified while it runs. No read is atomic with the fork either, so an
orphan of the engine's own adopted in that instant can be killed as the
attempt's. The operator accepted both as documented Linux residuals on
2026-09-28 so #403 could land, beside the accepted macOS residual. Two
further limits follow from the design: since the driver leads a session
of its own, a SIGKILL to the engine's process group no longer reaches a
seat's tree, and every tracker read walks the whole table.

#472 asks for containment that holds by construction. Research entry
[0020](../research/0020-containing-an-attempts-process-tree.md) tables the
options against the measurements: the probe `probe/containment.sh` on
branch `probe/472-containment` (GitHub run 38057456223, on the operator's
host and GitHub's `ubuntu-latest`, `ubuntu-24.04-arm`, `macos-latest` and
an `ubuntu:24.04` container), and the controller's measurement of a
per-attempt scope from outside, on the operator's host and in GitHub run
38077308914 at the probe's commit 167085b4. In short:

- a cgroup v2 ended by one `cgroup.kill` write left no process of a
  `setsid` and double-fork escape tree, and read `populated 0`, on the
  operator's host and both Linux runners;
- a per-attempt transient user scope works from outside the scope: the
  engine needs no delegated cgroup of its own, reads the scope's cgroup
  from the user manager, and ends it, settled within 4 ms on the
  operator's host and 2 ms on each Linux runner; starting one took 23
  to 73 ms;
- the user manager answers a unit it never made exactly as it answers
  a finished scope: no control group, and `inactive`, on all three
  Linux hosts, so a unit's state cannot tell a scope not yet created
  from one that has emptied;
- the job's own cgroup and a sudo system scope cannot be written by the
  job user, unprivileged user and PID namespaces are refused by
  AppArmor on every measured Ubuntu, and a Docker container offers none
  of these;
- macOS has no cgroups.

The operator ruled on 2026-10-10 that option (b) of #472 is the default:
where Linux offers no containment, Brokkr falls back to #403's polling
with its documented residuals, saying so on stderr and in `brokkr
doctor`; a machine that has containment may require it through its host
configuration (forge.host/v1, ruled on #430). The threat model stands as
the operator ruled it on 2026-09-26: catch realistic accidental misuse,
and fail closed on what cannot be read.

## Rulings

1. **On Linux an attempt is a cgroup.** Every driver the engine spawns
   runs in a cgroup v2 of its own, the attempt's cgroup, entered before
   the driver runs any code of its own. Everything the attempt forks is
   born inside it, whatever its session, group or parent, and cannot
   leave it by forking, `setsid`, `setpgid` or reparenting. The attempt
   is ended by one write of `1` to the cgroup's `cgroup.kill`, which
   ends its whole subtree, nested cgroups included. Nothing of a
   contained attempt is chosen by reading the process table.
   *Enforcement binding:* a test in `brokkr-protocol`'s process suite,
   `a_contained_attempt_ends_a_setsid_double_fork_born_between_two_reads`,
   runs a driver whose child calls `setsid`, forks a detached grandchild
   and exits, then exits itself before the kill; the test asserts every
   recorded pid dead through its pidfd and the cgroup `populated 0`. Its
   removal control skips the move into the cgroup and watches it fail.

2. **The ladder.** At its start the engine reads the host once into a
   typed observation, and a pure function picks the first rung the host
   offers. The pick is an enum, matched exhaustively (decision 0071
   rulings 2 and 10):
   - `delegated` — the engine already stands in a cgroup systemd marked
     as delegated to its user (`user.delegate`), as an operator's own
     unit with `Delegate=yes` makes. Each attempt's cgroup is a child of
     that cgroup, created before the spawn, and the driver enters it by
     writing its own pid to `cgroup.procs` between fork and exec. The
     engine itself moves to a leaf of its own first, so the delegated
     cgroup holds no process beside its children.
   - `scope` — otherwise, where a systemd user manager runs, each driver
     is started through `systemd-run --user --scope -p Delegate=yes
     --collect --slice brokkr.slice --unit <name>`, from the absolute
     path the engine probed at start, and `systemd-run` runs not the
     driver but the launch gate of ruling 4, which execs the driver.
     `systemd-run` asks the user manager for the scope with its own pid
     and then execs the gate, so the gate and the driver are born inside
     the scope, keep that pid, and stay the engine's child with the
     engine's pipes.
   - `polling` — otherwise, #403's mechanism, unchanged, with decision
     0006's addendum's residuals standing as written.
   A pid write before exec is preferred to `clone3(CLONE_INTO_CGROUP)`:
   both place the driver before it runs, and the write needs only the
   open and write a `pre_exec` hook may make, beside the
   `std::process::Command` spawn the house already has. A failed write
   fails the spawn. Re-executing the engine under `systemd-run` to obtain
   delegation is not a rung: the scope rung needs none, and a re-exec
   runs a binary path that may have been replaced under a live engine.
   A PID namespace around the harness is not a rung either, until the
   study in ruling 9 measures the harnesses inside one.
   *Enforcement binding:* the observation is a struct of typed reads
   (cgroup v2 mounted with `cgroup.kill`, `pidfd_open` answered, the
   engine's own cgroup and its delegation mark, the user manager's
   answer, the `systemd-run` path) and the pick a
   function of it alone; a table test in `brokkr-protocol` pins the rung
   and its reason for each valuation, and the readers that fill the
   observation are tested against planted cgroup trees.

3. **An attempt's cgroup is named for the attempt, and found again by
   the journal.** A scope is `brokkr-attempt-<attempt id>.scope`, and a
   delegated child is `brokkr-attempt-<attempt id>` under the delegated
   cgroup; an attempt that spawns several drivers, as a panel does, adds
   the member's escaped site key. The attempt id is a UUID the engine
   already journals in `effect/started` before the spawn. Which rung held
   the attempt, the unit name, and the attempt's cgroup path ride a
   numbered extension of that event,
   `contracts/effect-containment.v1.schema.json`, journaled before the
   spawn as `effect-boundary.v1` is (decision 0046). The same contract
   carries the closed set of the run's other containment records: the
   lifeline's `lifeline` and `lifeline-armed` (ruling 4) and a
   `leftover` (below). The path is known
   before the spawn on both rungs: a delegated child is the engine's own
   `mkdir`, and a scope's is derived, never asked for, as the user
   manager's own cgroup (read once at start, into ruling 2's
   observation), then `brokkr.slice`, then the scope's name.
   **A unit's state is never a settlement.** The user manager answers a
   unit it has not made yet exactly as it answers a scope that has
   emptied (no control group, `inactive`; research entry 0020), and a
   unit it lost in a crash of its own may read the same (0020's owed
   measurement 11), so the lifeline, `resume` and doctor never ask
   whether a unit is loaded or active. Each opens the
   attempt's recorded path. Where the cgroup is present, the lifeline and
   `resume` write `1` to its `cgroup.kill` and settle it as ruling 5
   says, whatever the unit's state. Where it is absent, absence settles
   the attempt only because the rung makes a later driver in it
   impossible, never because nothing is there now: the kernel refuses to
   remove a populated cgroup, so an absent one holds no process; a
   delegated child is created before the fork and a pid write into an
   absent one fails the spawn (ruling 4); and a scope created after the
   cleanup holds only a launch gate that will never exec, because every
   lifeline able to release it is gone (ruling 4).
   After an engine restart, `resume` folds the journal and first ends
   every lifeline the run journaled that still lives (ruling 4), by
   SIGKILL through a pidfd opened on its journaled pid and checked
   against its start stamp, and waits for its exit, so no gate of the
   dead engine can be released after this point. It then opens the
   recorded path of every attempt the run journaled with a contained
   record, open or settled. An open attempt's present cgroup it ends and
   settles, and on the `delegated` rung removes the settled cgroup,
   retrying the kill while the removal reads busy, within the settle
   bound, so a forked child of the dead engine that has not yet written
   its pid finds no cgroup and fails before exec; a removal still busy
   past the bound leaves the cleanup unresolved, `effect/indeterminate`
   with the `effect-cleanup.v1` reason naming the busy cgroup. A settled
   attempt's present cgroup is told by its `populated` value. Reading
   `populated 1`, it is a leftover, the diagnostic of a breach, since
   settlement read `populated 0`: `resume` ends and settles it the same
   way and journals the containment contract's `leftover` record, naming
   the attempt, the `populated` value it read and the settlement. Reading
   `populated 0`, it is an empty remnant and no breach, since nothing of
   it runs. On the `scope` rung the remnant is the user manager's to
   collect (ruling 5), and `resume` leaves it and journals nothing. On
   the `delegated` rung settlement removed the cgroup it settled (ruling
   5), so an empty one present again was made by something else, and
   `resume` removes it under a `leftover` record whose `populated 0`
   names it a remnant, not a breach. On a run that has ended, `resume` is admitted for
   this cleanup alone: it ends lifelines and leftovers, drives no phase,
   and leaves the run's ending as it was, since the ending is read from
   the run's terminal event (decision 0064 ruling 2) and a `leftover`
   record is not one. A lifeline whose
   identity cannot be read, or that outlives the settle bound after its
   kill, leaves the attempt's cleanup unresolved: `effect/indeterminate`,
   never settled. Nothing is listed by prefix and swept, nothing is
   chosen by age, and nothing by working directory, so another run's
   attempt is never touched.
   *Enforcement binding:* `resume_ends_the_scope_named_for_an_open_attempt`
   and `a_scope_the_journal_does_not_name_is_never_touched` (a second
   live `brokkr-attempt-*` scope of another run survives the resume);
   `an_unloaded_unit_is_never_read_as_settled`, which stands a scope's
   cgroup up populated while a user-manager stand-in reports its unit
   `inactive`, and asserts that `resume` still kills it and reads
   `populated 0` (removal control: settle on the unit's state, watch the
   populated cgroup survive); `a_pid_write_after_resume_fails_the_spawn` on the
   `delegated` rung, asserting the late write's exact `ENOENT` refusal
   and that no driver ran; `a_busy_removal_past_the_settle_bound_parks`,
   asserting the exact `Unsettled` variant and its text naming the busy
   cgroup; ruling 8's leftover remedy, bound there; and the extension's
   schema test refusing an unknown rung, an unknown record and unknown
   fields.

4. **Engine death is covered by a lifeline, and no launch outlives it.**
   A transient scope is the user manager's unit, and a delegated child
   lives as long as its unit does; either outlives the engine. With its
   first contained attempt the engine starts one lifeline: `brokkr`
   itself under a hidden verb, as `brokkr dsh-sandbox-runner` is, in a
   session of its own (so a signal to the engine's process group misses
   it) and outside every attempt's cgroup. Its identity, pid and start
   stamp, is journaled as the containment contract's `lifeline` record
   before any attempt is armed with it, so `resume` can end it (ruling
   3). The build keeps these rules:
   - **The channel.** The lifeline holds one end of a `SOCK_SEQPACKET`
     Unix socket pair whose other end is the engine's alone,
     close-on-exec, so no driver keeps it past its exec. Each message is
     one record, received whole or not at all. The channel carries the
     arming and its acknowledgements. Its end-of-file is not what the
     lifeline waits on to learn of the engine's death: a forked child that has not yet exec'd still
     holds the engine's end, so a child stopped in that window would hold
     off the end-of-file for as long as it stays stopped.
   - **Engine death, read from a pidfd.** Before it spawns a lifeline the
     engine opens a pidfd on itself (`pidfd_open` of its own pid, Linux
     5.3), and the lifeline inherits it at a fixed descriptor. The
     lifeline polls it, and it reads ready once the engine's whole
     process has exited, SIGKILL included, whichever thread spawned the
     lifeline, with no pid to be reused, since it was opened while the
     engine lived. The parent-death signal is refused:
     `PR_SET_PDEATHSIG` fires when the thread that forked the child
     exits, not the process, and the engine runs threads of its own
     (each driver's watchdog and pipe readers,
     `crates/brokkr-protocol/src/process.rs`), so a lifeline started
     from one would end every live attempt at that thread's exit while
     the engine lives; the signal is also reset across a set-user-ID
     exec, and is never sent if the parent dies before the `prctl`.
     `cgroup.kill` needs Linux 5.14, so every host that offers a
     containment rung has `pidfd_open`; ruling 2's observation reads
     both, and a host without either takes `polling`. End-of-file on the
     channel, which the engine's orderly exit makes by closing its end,
     is the second trigger. The lifeline acts on the first of the two and
     waits for nothing more: the bound after which it acts without
     end-of-file is the engine's exit itself, with no grace for a launch
     in flight, since the removal below (on `delegated`) and the gate (on
     `scope`) leave a late launch nothing to run in, and a grace would
     only cover a window they already close.
   - **Holding.** The lifeline holds an attempt from the receipt of its
     path: the moment it reads the arming message off the channel,
     whether or not it has acknowledged it yet. On the engine's death it
     first reads every message already queued on the channel and only
     then acts, so it holds every path the engine sent before it died.
   - **Arming.** Before each spawn the engine sends the lifeline the
     attempt's cgroup path and, on the `scope` rung, one end of a fresh
     socket pair, the gate's channel, and spawns only on the lifeline's
     acknowledgement. Every such end is close-on-exec in the engine; the
     spawned child's `pre_exec` duplicates its own end onto the gate's
     fixed descriptor, so a driver spawned beside it inherits none, and
     the engine closes its copies when the spawn returns. The engine
     tells the lifeline each settled attempt after its settlement.
   - **The launch gate, on the `scope` rung.** `systemd-run` execs
     `brokkr` under a second hidden verb, the gate. The gate reads its own
     cgroup and refuses unless it stands in the attempt's derived path,
     reports in to the lifeline from there, and blocks on its channel. On
     `go` it closes the channel and execs the driver; on EOF, or anything
     else, it exits without exec, with a status the engine names as a
     cancelled launch. The lifeline answers `go` only to a gate that has
     reported in, and only while it has not read the engine's death; it
     takes the gates, the channel and the pidfd in one sequential loop,
     so every `go` it sends precedes its handling of the death, and it
     releases nothing after it. So every released driver stands in a
     cgroup that exists and that a living lifeline will end. A launch
     still in flight when the lifeline reads the death (a forked child not
     yet exec'd into `systemd-run`, or `systemd-run` waiting on the user
     manager, the 23 to 73 ms measured) may create its scope afterwards,
     but only to run a gate that reads EOF and exits, since the lifeline
     closes every unreleased gate channel first, and a dead lifeline's
     channels are closed by its death: each gate's peer is the lifeline's
     alone.
   - **The `delegated` rung needs no gate, because cleanup removes the
     cgroup.** The engine creates the attempt's cgroup before the fork,
     and the forked child moves itself into it by writing its own pid to
     `cgroup.procs` between fork and exec (ruling 1); the child never
     creates it. The lifeline's cleanup removes every held attempt's
     cgroup, so a child that writes after the cleanup finds it absent and
     its `ENOENT` fails the spawn before exec, and a child that wrote
     before is inside and ends with the rest. No helper process, second
     spawn path, or move made by the parent after the spawn may stand in
     for the child's own write, since each lets a driver run before it is
     contained; a launch that needs one takes the gate. The earlier rule,
     that the lifeline read no end-of-file while a forked child stood
     outside its cgroup and so waited on that child, is dropped: removal
     closes that window without waiting on any process.
   - **On the engine's death** the lifeline closes every unreleased gate
     channel, then, for each held attempt, opens its path, writes
     `cgroup.kill` where the cgroup is present, waits for `populated 0`
     (ruling 3: no unit's state is read), and on the `delegated` rung
     removes the cgroup, retrying the kill while the removal reads busy,
     as `resume` does. All of it runs within the settle bound counted
     from the death, and then the lifeline exits. A cgroup still
     populated, or still busy, at the bound it leaves as it is: the
     attempt is open in the journal, and the next `resume` ends it or
     parks it `effect/indeterminate` by ruling 3.
   - **Replacement.** The engine watches the lifeline all the time, by
     its pidfd and by a hang-up on its own end of the channel, not at its
     next write. When the lifeline dies, the engine spawns nothing until a
     replacement has acknowledged every live attempt: it starts a new lifeline,
     journals its `lifeline` record, arms it with each live attempt's
     path, and, once the replacement has acknowledged every one, journals
     the contract's `lifeline-armed` record naming the attempt ids it
     holds. That record, not the identity, marks the end of the
     replacement window; the engine spawns again only after it.
     Gates the dead lifeline had not released read EOF and exit, and
     their attempts end as cancelled launches. A replacement that does
     not acknowledge every live attempt within the startup bound makes
     the engine end every live contained attempt itself, by
     `cgroup.kill` and `populated 0`, and refuse the next spawn, naming
     the lifeline.
   A SIGKILL that takes the engine and the lifeline together (an
   operator's `pkill -9 brokkr`, an OOM kill of both), or the engine
   alone after a lifeline's death and before its replacement has
   received a live attempt's path, leaves that attempt's released driver
   running until the next `resume` of that run ends it by ruling 3.
   Since the replacement reads every queued message before it acts, that
   window closes once the engine has sent the path: an attempt whose path
   the replacement has received is ended at the engine's death,
   acknowledged or not. A cgroup the lifeline cannot settle, or on the
   `delegated` rung remove, within the settle bound is likewise left to
   `resume`. A launch in flight never runs its driver. That is the
   residual containment keeps, narrower than polling's, and the operator
   is asked to accept it.
   *Enforcement binding:* the issue's host test,
   `sigkill_to_the_engine_leaves_no_seat_process_alive`, runs an engine
   in a child process with an attempt whose driver leaves an escape tree,
   sends SIGKILL to the engine alone and then, in its second case, to
   the engine's process group, and asserts every escape pid gone, the
   cgroup `populated 0`, and on the `delegated` rung the cgroup absent,
   within the settle bound. It runs on the host and
   in CI on Linux; on a host that takes the `polling` rung it asserts the
   fallback notice instead and never skips in silence. Its removal
   control starts no lifeline and watches the seat survive.
   `an_engine_killed_with_a_launch_paused_still_ends_the_live_attempt`
   is a host test with one case per rung. Attempt A runs an escape tree;
   attempt B's launch is paused by a FIFO the test leaves closed until
   the settle bound has passed: on `delegated` in `pre_exec` before its
   pid write, on `scope` before its exec of `systemd-run`. The test sends
   SIGKILL to the engine alone and asserts, within the settle bound of
   the kill, every escape pid of A gone, A's cgroup `populated 0`, on
   `delegated` A's and B's cgroups absent, and the lifeline exited. It
   then releases B and asserts that no driver of B ran: on `delegated`
   the pid write's exact `ENOENT` refusal, on `scope` the gate's
   cancelled-launch status. Its removal control has the lifeline act on
   end-of-file alone, and watches A's tree outlive the bound.
   `a_launch_released_after_the_engine_dies_runs_no_driver` is the same
   kind of host test on the `scope` rung. Its probed `systemd-run` is a
   stand-in that blocks on a FIFO before it execs the real one, so the
   launcher is paused before the scope exists. In the lifeline ordering
   it sends SIGKILL to the engine alone, waits for the lifeline to exit,
   and releases the launcher; in the resume ordering it sends SIGKILL to
   the engine and the lifeline, runs `resume` of the run to its
   settlement, and then releases the launcher. Each asserts that the
   driver's marker file is never written, that the gate exits with the
   cancelled-launch status, and that the derived cgroup reads
   `populated 0` or is absent once the gate has exited. Its removal
   control lets the gate exec without waiting for `go`, and watches the
   driver run.
   `the_lifeline_reads_no_eof_while_a_forked_child_is_outside_its_cgroup`
   pauses a `delegated` child in `pre_exec` before its pid write, sends
   SIGKILL to the engine, and asserts the bounded outcome: the lifeline
   has read no end-of-file, since the child still holds the engine's
   end, and yet within the settle bound of the kill it has removed the
   child's cgroup and exited. It then releases the child and asserts the
   pid write's exact `ENOENT` refusal and that no driver ran. Its
   removal control drops the bound, having the lifeline act on
   end-of-file alone, and watches the lifeline still running at the
   bound; a second has the lifeline skip the removal, and watches the
   released child's driver run.
   `a_dead_lifeline_is_replaced_while_attempts_live` kills the lifeline
   while an attempt runs, waits for the replacement's journaled
   `lifeline-armed` record naming that attempt, sends SIGKILL to the
   engine, and asserts the escape tree gone and the cgroup `populated 0`
   within the settle bound; its removal control replaces the lifeline
   only at the engine's next write and watches the wait for the record
   run out at the startup bound. Two cases split the replacement window
   at receipt.
   `an_engine_killed_before_the_replacement_receives_a_path_leaves_it_to_resume`
   holds the engine with a FIFO after the replacement's `lifeline`
   record and before it sends the replacement the live attempt's path,
   sends SIGKILL to the engine in that window, and asserts the declared
   residual and its remedy: the escape tree still running past the
   settle bound and no `lifeline-armed` record journaled, then, after
   `resume` of the run, the tree gone and `populated 0`; its removal
   control has `resume` skip the attempts the journal shows
   unacknowledged and watches the tree survive.
   `an_engine_killed_after_the_replacement_receives_a_path_ends_it`
   holds the replacement with a FIFO after it has read the live
   attempt's path and before it acknowledges it, sends SIGKILL to the
   engine, releases the replacement, and asserts the escape tree gone
   and `populated 0` within the settle bound of the release, with no
   `lifeline-armed` record journaled; its removal control has the
   replacement hold an attempt only from its acknowledgement and watches
   the tree survive.

5. **The settle proof is `populated 0`.** On a contained attempt the end
   is the same for every cause: `shutdown` and the grace as today, one
   `cgroup.kill`, the driver reaped, and `cgroup.events` read until it
   says `populated 0`, watched for its change rather than polled, within
   the settle bound. That count is the kernel's, not a listing, so a
   descendant born at any instant is in it. A cgroup still populated
   past the bound, an `events` file that cannot be read or parsed, and a
   kill the kernel refuses leave the cleanup unresolved, and the attempt
   journals `effect/indeterminate` with its `effect-cleanup.v1` reason, as
   today. On the `delegated` rung settlement then removes the attempt's
   cgroup, retrying the kill while the removal reads busy, within the same
   settle bound, as `resume` does (ruling 3), and journals the attempt
   settled only once the cgroup is gone; a removal still busy at the
   bound leaves the cleanup unresolved, as a populated cgroup does. On
   the `scope` rung the cgroup is the user manager's: `--collect` has the
   manager unload the scope once its last process has exited, even if it
   failed, and the manager removes the cgroup on its own notification, at
   a time Brokkr does not bound. Settlement there is `populated 0` and
   does not wait for the removal, so a scope's empty cgroup may stand
   present after its attempt has settled, and is no breach (ruling 3);
   research entry 0020 lists the collection's timing as owed measurement
   13. The stop handler ends every live attempt the same way, without
   the grace, and exits as it does now. Under containment neither the
   engine nor a driver is a child subreaper, since the subreaper served
   only to attribute orphans to a tree. With it dropped, an orphan goes
   to pid 1 or to the nearest subreaper among its ancestors, which is the
   user manager only when the engine itself runs under it; the scope rung
   changes the driver's cgroup, not its parentage. Whoever adopts the
   orphan reaps it, and it stays in the attempt's cgroup, since
   reparenting moves no process between cgroups. The stray
   attribution, the descendant tracker and its 100 ms walks do not run
   for a contained attempt. The pipe drain stays, since a process that
   left the cgroup could still hold one.
   On a contained Linux attempt this ends both of decision 0006's
   addendum's accepted Linux residuals and the limit that a SIGKILL to the
   engine's group no longer reaches a seat's tree. The table-read race
   is gone because membership is decided at birth and counted by the
   kernel. The read-to-fork instant is gone because nothing is
   attributed by a read: an orphan of the engine's own was never in the
   attempt's cgroup. The SIGKILL limit is gone by ruling 4. On a
   `polling` host all three stand as written. The build rewrites the
   residual paragraphs of `crates/brokkr-protocol/src/process/tree.rs`
   and of the addendum to say which rung each holds on, by a dated
   erratum in the addendum that cites this decision, and the operator
   rules which header pointer it carries.
   *Enforcement binding:* `a_populated_cgroup_past_the_bound_parks` and
   `unreadable_cgroup_events_park`, each asserting the exact
   `Unsettled` variant and its text;
   `a_delegated_settlement_removes_the_cgroup`, asserting the attempt's
   path absent before its settlement is journaled (removal control: skip
   the removal, watch the path stay); the existing #403 tests keep running
   against the `polling` rung through the host stand-in.

6. **What the host configuration declares.** One closed field,
   `containment`, with two values: `best-effort`, the default, takes the
   first rung the ladder finds, `polling` included; `required` refuses
   every `run` and `resume` on a host that offers neither `delegated` nor
   `scope`, before any effect, with a named error that says which reads
   failed. The ladder's order is the engine's, not the host's. The field
   belongs to forge.host. A contract is frozen once it is in the tree, so
   if forge.host/v1 has not merged when the operator accepts this
   decision, v1 carries the field; otherwise forge.host/v2 adds it, and a
   v1 host reads `best-effort`.
   *Enforcement binding:* the host schema's own test refuses a third
   value and an unknown field (`deny_unknown_fields`), and
   `required_containment_without_a_rung_refuses_the_start` asserts the
   error's exact text and that no effect was journaled.

7. **The fallback is said aloud.** An engine that takes the `polling`
   rung on Linux writes one line to stderr at its start, naming the rung,
   the read that ruled out each rung above it, and that decision 0006's
   residuals stand. It is said once per engine start, never per attempt.
   *Enforcement binding:* `the_no_delegation_path_falls_back_aloud`, run
   against a host stand-in with no user manager and no delegated cgroup,
   asserts the exact line on stderr and the `polling` rung in the
   journal's containment record.

8. **What `brokkr doctor` reports.** One `containment` line: the rung
   this host would take, the reads behind it (ruling 2's observation:
   cgroup v2 with `cgroup.kill`, `pidfd_open`, the delegation mark, the
   user manager, the `systemd-run` path), and the configured
   requirement. `required` on a host
   whose rung is `polling` is a failure line; `best-effort` on such a
   host is a warning that names the residuals. Doctor also opens the
   recorded path of every attempt the journal shows settled or whose run
   has ended, and lists each whose cgroup is present, told by its
   `populated` value as ruling 3 tells it: `populated 1` is a leftover
   line that names the breach, and `populated 0` is an empty line that
   names none, and says who removes it (on `delegated` the run's
   `resume`, on `scope` the user manager's collection). It never asks
   the user manager whether a unit is loaded (ruling 3). It writes no
   `cgroup.kill` and removes nothing: ending an attempt is an effect, and
   doctor journals none (decision 0071 ruling 1), so each leftover line,
   and each empty line on `delegated`, names the `brokkr resume` of the
   run, which ruling 3 admits on a settled attempt and on an ended run,
   and which ends or removes it under its journaled `leftover` record.
   *Enforcement binding:* doctor's snapshot tests, one per rung and one
   for each requirement on a polling host, pinning each line's text, and
   one that plants a populated cgroup at a settled attempt's path while
   the user-manager stand-in answers `inactive`, pinning the leftover
   line. `an_empty_present_cgroup_is_not_a_breach`, one case per rung,
   plants an empty cgroup at a settled attempt's path and pins doctor's
   empty line, which names no breach, then runs `resume` and asserts on
   `delegated` the path absent and a `leftover` record with `populated
   0`, and on `scope` the path untouched and nothing journaled; its
   removal control tells a cgroup by its presence alone and watches the
   line name a breach. `resume_ends_the_leftover_doctor_names` plants the
   populated fixture, once in a live run and once in an ended run, runs
   the `brokkr resume` the doctor line names, and asserts the cgroup
   `populated 0`, the `leftover` record journaled with the `populated`
   value it read, the ended run's ending unchanged, and doctor's next run
   listing no leftover: on `delegated` no line for the attempt, the
   cgroup removed, and on `scope` at most its empty line; its removal
   control has `resume` take open attempts only and watches the leftover
   and its doctor line survive.

9. **macOS keeps polling, and the study is named.** macOS has no cgroups,
   so it stays on #403's polling with its accepted residual, and
   `required` refuses there. The study: a per-attempt launchd job
   (`launchctl submit`, or a job bootstrapped into the user's domain)
   running the probe's escape tree, measuring whether the job's process
   coalition holds a `setsid` double-forked descendant and whether
   removing the job ends it, on `macos-latest` and an operator's Mac.
   `sandbox-exec` confines effects, not a process's lifetime, and is not
   a candidate. Private interfaces are refused. The PID-namespace study
   for Linux hosts without systemd is named beside it: each harness,
   claude, codex or dsh, run under `bwrap --unshare-pid`, measuring
   whether its own sandbox still starts. Either study that finds a
   mechanism returns with a decision of its own.
   This ruling is judgment guidance: no gate holds a study.

## Asked of the operator

- **The ladder's order (ruling 2).** `delegated` before `scope` because
  it costs nothing per attempt and an operator who declared a delegated
  unit chose it. The other order is one line; it costs 23 to 73 ms per
  attempt on hosts that have both.
- **The lifeline's residual (ruling 4).** A kill that takes the engine
  and its lifeline together, or the engine alone after a lifeline's
  death and before its replacement has received a live attempt's path,
  leaves that attempt's released driver running until the next
  `resume`; no launch in flight runs its driver. Binding each scope to a unit of the engine's own
  (`BindsTo=`) would close it, at the price of running the engine in a
  unit; that is unmeasured, and is listed in research entry 0020.
- **Re-homing.** A process inside the attempt that asks the user manager
  for a unit of its own (`systemd-run --user`, a tmux server built with
  systemd support, a D-Bus activated user service) is started outside
  the attempt's cgroup. It is a deliberate request to the user manager,
  not a race, and polling misses most of these today too. Which of them
  a seat can reach is unmeasured; research entry 0020 lists it as owed
  measurement 4.
- **The launch gate's cost (ruling 4).** On the `scope` rung each
  attempt adds one exec of `brokkr` and one round trip to the lifeline
  before the driver runs, the price of no launch outliving the
  lifeline's cleanup.
- **Where the field lives (ruling 6),** forge.host/v1 or v2, falls on
  the order in which this decision and #430's third slice land.
- **Who ends a leftover (rulings 3 and 8).** `resume` is admitted on a
  settled attempt and on an ended run for that cleanup alone, under a
  journaled `leftover` record, and doctor stays free of effects. The
  other shape is doctor writing `cgroup.kill` under a journaled effect
  of its own, which would make doctor an effecting verb.

## What still needs measuring

Research entry 0020 lists thirteen measurements with their method. The
build waits on five of them: the lifeline end to end, triggered by its
pidfd, with a paused launch (ruling 4), whether an unreaped zombie keeps `populated 1` and
who adopts an orphan with the engine outside and inside the user
manager's tree (ruling 5), whether systemd's delegation mark reads on
the hosts (ruling 2; where it does not, the `delegated` rung is never
taken and `scope` serves), and whether a scope started with `--slice
brokkr.slice` stands at the path ruling 3 derives (where it does not,
the gate refuses every launch and the `scope` rung is not offered), and
whether the descriptor the child places at the gate's fixed number
survives `systemd-run`'s own exec into the gate (where it does not, the
gate finds no channel, every launch ends as a cancelled launch, and the
`scope` rung is not offered).

## Consequences

- On a Linux host with a user manager, which every measured host but a
  container has, a seat's whole tree ends with one write and is proven
  gone by the kernel's own count, and the engine's death no longer
  leaves it running, but for ruling 4's declared residual, which the
  next `resume` ends.
- On the `delegated` rung every cleanup removes the attempt's cgroup;
  on the `scope` rung an empty cgroup may stand until the user manager
  collects it, and doctor lists it as empty, not as a breach.
- Every attempt costs one transient unit start, one gate exec and one
  round trip to the lifeline on the `scope` rung, and no table walk
  while it runs.
- No unit's state is read as a settlement anywhere: the lifeline,
  `resume` and doctor read the attempt's cgroup path.
- The polling code stays for macOS and for Linux hosts without
  containment, so the #403 suite keeps running against it.
- The journal says which rung held each attempt, so a cleanup's proof
  can be read back to its mechanism.
- A host that requires containment cannot start a run where none is
  offered.
