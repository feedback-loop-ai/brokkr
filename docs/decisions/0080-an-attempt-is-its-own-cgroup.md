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
per-attempt scope from outside. In short:

- a cgroup v2 ended by one `cgroup.kill` write left no process of a
  `setsid` and double-fork escape tree, and read `populated 0`, on the
  operator's host and both Linux runners;
- a per-attempt transient user scope works from outside the scope: the
  engine needs no delegated cgroup of its own, reads the scope's cgroup
  from the user manager, and ends it within 4 ms; starting one took 23
  to 73 ms;
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
     --collect --unit <name>`, from the absolute path the engine probed
     at start. `systemd-run` asks the user manager for the scope with its
     own pid and then execs the driver, so the driver is born inside the
     scope and stays the engine's child with the engine's pipes.
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
   (cgroup v2 mounted, the engine's own cgroup and its delegation mark,
   the user manager's answer, the `systemd-run` path) and the pick a
   function of it alone; a table test in `brokkr-protocol` pins the rung
   and its reason for each valuation, and the readers that fill the
   observation are tested against planted cgroup trees.

3. **An attempt's cgroup is named for the attempt, and found again by
   the journal.** A scope is `brokkr-attempt-<attempt id>.scope`, and a
   delegated child is `brokkr-attempt-<attempt id>` under the delegated
   cgroup; an attempt that spawns several drivers, as a panel does, adds
   the member's escaped site key. The attempt id is a UUID the engine
   already journals in `effect/started` before the spawn. Which rung held
   the attempt, and the unit name or cgroup path, ride a numbered
   extension of that event, `contracts/effect-containment.v1.schema.json`,
   journaled before the spawn as `effect-boundary.v1` is (decision 0046).
   After an engine restart, `resume` folds the journal, takes every
   attempt it finds open with a contained record, and for each asks the
   user manager for that scope's cgroup, or opens the recorded path. A
   cgroup still present is ended by `cgroup.kill` and settled as ruling 5
   says, and the attempt's cleanup is then proven. A scope the user
   manager no longer has loaded, or a delegated path that is gone, has
   no process left in it, since neither is removed while populated.
   Nothing is listed by prefix and swept, nothing is chosen by age, and
   nothing by working directory, so another run's attempt is never
   touched.
   *Enforcement binding:* `resume_ends_the_scope_named_for_an_open_attempt`
   and `a_scope_the_journal_does_not_name_is_never_touched` (a second
   live `brokkr-attempt-*` scope of another run survives the resume), and
   the extension's schema test refusing an unknown rung and unknown
   fields.

4. **Engine death is covered by a lifeline.** A transient scope is the
   user manager's unit, and a delegated child lives as long as its unit
   does; either outlives the engine. With its first contained attempt the
   engine starts one lifeline: `brokkr` itself under a hidden verb, as
   `brokkr dsh-sandbox-runner` is, in a session of its own (so a signal
   to the engine's process group misses it) and outside every attempt's
   cgroup. It holds the read end of a pipe whose only write end is the
   engine's, close-on-exec so no driver inherits it. The engine tells it
   each attempt's cgroup before the spawn and each settled one after.
   When the pipe reads EOF, the engine has died, SIGKILL included, and
   the lifeline writes `cgroup.kill` for every attempt it holds, waits
   for `populated 0` within the settle bound, and exits. A lifeline that
   dies first is seen by the engine at its end of the pipe: the engine
   starts a new one and hands it every live attempt before it spawns
   again, and one it cannot start refuses the next spawn, naming the
   lifeline. A SIGKILL that takes the engine and the lifeline together
   (an operator's `pkill -9 brokkr`, an OOM kill of both) leaves the seat
   running until the next `resume` of that run ends it by ruling 3; that
   is the residual containment keeps, narrower than polling's, and the
   operator is asked to accept it.
   *Enforcement binding:* the issue's host test,
   `sigkill_to_the_engine_leaves_no_seat_process_alive`, runs an engine
   in a child process with an attempt whose driver leaves an escape tree,
   sends SIGKILL to the engine alone and then, in its second case, to
   the engine's process group, and asserts every escape pid gone and the
   cgroup `populated 0` within the settle bound. It runs on the host and
   in CI on Linux; on a host that takes the `polling` rung it asserts the
   fallback notice instead and never skips in silence. Its removal
   control starts no lifeline and watches the seat survive.

5. **The settle proof is `populated 0`.** On a contained attempt the end
   is the same for every cause: `shutdown` and the grace as today, one
   `cgroup.kill`, the driver reaped, and `cgroup.events` read until it
   says `populated 0`, watched for its change rather than polled, within
   the settle bound. That count is the kernel's, not a listing, so a
   descendant born at any instant is in it. A cgroup still populated
   past the bound, an `events` file that cannot be read or parsed, and a
   kill the kernel refuses leave the cleanup unresolved, and the attempt
   journals `effect/indeterminate` with its `effect-cleanup.v1` reason, as
   today. The stop handler ends every live attempt the same way, without
   the grace, and exits as it does now. Under containment neither the
   engine nor a driver is a child subreaper, since the subreaper served
   only to attribute orphans to a tree: an orphan goes to the user
   manager, which reaps it, and stays in the attempt's cgroup. The stray
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
   `Unsettled` variant and its text; the existing #403 tests keep running
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
   this host would take, the reads behind it (cgroup v2, the delegation
   mark, the user manager, the `systemd-run` path, the time one scope
   took to start), and the configured requirement. `required` on a host
   whose rung is `polling` is a failure line; `best-effort` on such a
   host is a warning that names the residuals. Doctor also lists every
   attempt the journal shows settled or whose run has ended whose scope
   the user manager still has loaded, by the names ruling 3 derives. It
   reads; it ends nothing.
   *Enforcement binding:* doctor's snapshot tests, one per rung and one
   for each requirement on a polling host, pinning each line's text.

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
  and its lifeline together leaves the seat running until the next
  `resume`. Binding each scope to a unit of the engine's own
  (`BindsTo=`) would close it, at the price of running the engine in a
  unit; that is unmeasured, and is listed in research entry 0020.
- **Re-homing.** A process inside the attempt that asks the user manager
  for a unit of its own (`systemd-run --user`, a tmux server built with
  systemd support, a D-Bus activated user service) is started outside
  the attempt's cgroup. It is a deliberate request to the user manager,
  not a race, and polling misses most of these today too. Research
  entry 0020 measures which of them a seat can reach.
- **Where the field lives (ruling 6),** forge.host/v1 or v2, falls on
  the order in which this decision and #430's third slice land.

## What still needs measuring

Research entry 0020 lists nine measurements with their method. The build
waits on three of them: the lifeline end to end (ruling 4), whether an
unreaped zombie keeps `populated 1` (ruling 5), and whether systemd's
delegation mark reads on the hosts (ruling 2; where it does not, the
`delegated` rung is never taken and `scope` serves).

## Consequences

- On a Linux host with a user manager, which every measured host but a
  container has, a seat's whole tree ends with one write and is proven
  gone by the kernel's own count, and the engine's death no longer
  leaves it running.
- Every attempt costs one transient unit start on the `scope` rung, and
  no table walk while it runs.
- The polling code stays for macOS and for Linux hosts without
  containment, so the #403 suite keeps running against it.
- The journal says which rung held each attempt, so a cleanup's proof
  can be read back to its mechanism.
- A host that requires containment cannot start a run where none is
  offered.
