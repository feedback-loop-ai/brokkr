# 0020 — Containing an attempt's process tree: what Linux and macOS offer, measured

Source: https://github.com/feedback-loop-ai/brokkr/actions/runs/38057456223
Authors: Brokkr's controller and the #472 design seat (probe branch `probe/472-containment`, its containment.sh script at 92e85605, and its outside-scope case at 167085b4, run 38077308914)
Read: 2026-10-10
Status: proposed
Intake: design story #472, run design-story-472-contain-each-at-fc590cbc, on the operator's ruling of 2026-10-10

## Summary

#403 ends a timed-out attempt's process tree by reading the process
table: a session and group per driver, the engine and every driver as
Linux child subreapers, every descendant recorded by pid and start stamp
by a 100 ms tracker, and reads that fail closed. Decision 0006's
addendum names what that cannot close. A Linux read is a `/proc` listing
followed by one stat read per pid, so a descendant can be born and
orphaned between the two and go unseen (the table-read race); no read is
atomic with the fork, so an orphan of the engine's own adopted in that
instant can be killed as the attempt's (the read-to-fork instant); and,
since the driver leads a session of its own, a SIGKILL to the engine's
process group no longer reaches a seat's tree. The operator accepted the
two Linux residuals on 2026-09-28 so #403 could land, and filed #472 to
close them by construction.

This entry tables the mechanisms that could, against the measurements
the probe and the controller took. The probe ran on the operator's host
(Ubuntu, kernel 6.17) and on GitHub's `ubuntu-latest`,
`ubuntu-24.04-arm` and `macos-latest` runners, and in an `ubuntu:24.04`
Docker container on `ubuntu-latest` (run 38057456223, tabled in
#472's second comment of 2026-10-10). The controller then measured the
per-attempt scope from outside on the operator's host, and the probe's
commit 167085b4 added that case and ran it on both Linux runners and in
the container (run 38077308914). Nothing here was
measured inside a hands box: the box refuses to nest the namespaces a
probe needs, and the controller runs what the box cannot.

The operator ruled the threat model on 2026-09-26: catch realistic
accidental misuse, and fail closed on what cannot be read. On
2026-10-10 the operator ruled the fallback: where Linux offers no
containment, Brokkr falls back to #403's polling with its documented
residual and says so on stderr and in `brokkr doctor`, and a host that
has containment may require it through its host configuration
(forge.host/v1, ruled on #430). Decision 0080 proposes the rulings this
entry argues for.

## What was measured

| Probe | Operator host | `ubuntu-latest` | `ubuntu-24.04-arm` | Docker container | macOS |
|---|---|---|---|---|---|
| Child cgroup under `systemd-run --user --scope -p Delegate=yes`, the escape tree ended by one `cgroup.kill` write | 0 processes left, `populated 0` | 0 left, `populated 0` | 0 left, `populated 0` | none offered | no cgroups |
| The job's own cgroup, written by the job user | — | refused | refused | none offered | — |
| A system scope through `sudo systemd-run --scope` | — | not writable by the job user | not writable by the job user | — | — |
| Unprivileged `unshare --user --pid` | refused (`apparmor_restrict_unprivileged_userns=1`) | refused | refused | none offered | — |
| `bwrap --unshare-pid --die-with-parent`, the escape tree after its init exits | 0 left | 0 left | 0 left | unavailable | — |
| `sandbox-exec`, `launchctl` | — | — | — | — | present |
| A per-attempt transient user scope, found from outside (`systemctl --user show -p ControlGroup`) and ended by one outside `cgroup.kill` write | 0 left, settled in 4 ms, then `inactive` | 0 left, settled in 2 ms, then `inactive` | 0 left, settled in 2 ms, then `inactive` | no user manager (`offline`) | no cgroups |
| `systemctl --user show -p ControlGroup --value` and `is-active` for a unit never made | empty, `inactive` | empty, `inactive` | empty, `inactive` | no user manager | — |

The escape tree is the probe's own: a process writes its pid to the
attempt cgroup's `cgroup.procs`, then execs `setsid bash -c "(setsid
sleep &); sleep"`, so one sleep leads a session of its own and the other
is a double-forked orphan of a second session.

The controller's measurement on the operator's host, after the probe:
`systemd-run --user --scope -p Delegate=yes --unit <name> -- <driver>`
ran the same escape tree inside a per-attempt scope. The engine's
stand-in, outside the scope, read the scope's cgroup from `systemctl
--user show -p ControlGroup --value <name>.scope`, wrote `1` to its
`cgroup.kill`, and read every process gone and `populated 0` within 4 ms;
the scope then read inactive. Starting the transient scope took 23, 35,
36, 37 and 73 ms over five runs. A per-attempt scope therefore needs no
delegated cgroup for the engine itself. The probe's commit 167085b4
made the same measurement a case of the probe, and run 38077308914 ran
it on `ubuntu-latest` and `ubuntu-24.04-arm`: a `setsid` and
double-fork tree in a per-attempt scope, the scope's cgroup read from
`systemctl --user show -p ControlGroup`, one outside `cgroup.kill`
write, 0 processes left, settled in 2 ms on each, and the scope then
read inactive. In the Docker container `systemctl --user
is-system-running` read `offline`: there is no user manager to ask.

The same run asked the user manager about a unit that was never made,
`probe-never-made.scope`. On the operator's host and both runners
`systemctl --user show -p ControlGroup --value` printed nothing and
`systemctl --user is-active` printed `inactive`, exactly what a
finished scope prints. A unit's state therefore cannot tell a scope
not yet created from one that has emptied, and settles nothing.

`systemd-run --scope` runs its command in its own process: it asks the
user manager for the scope with its own pid, waits for the job, and then
execs the command. The driver is so born inside the scope, and stays the
engine's child with the pipes the engine gave it.

## The options

| Option | Guarantee | Needs | Cost per attempt | Where it works (measured) | Failure modes |
|---|---|---|---|---|---|
| (i) A per-attempt transient user scope, through `systemd-run` or the user manager's D-Bus `StartTransientUnit` | Escape-proof by fork, `setsid`, `setpgid` and reparenting: membership is decided at birth. One `cgroup.kill` ends the subtree; `populated 0` in `cgroup.events` proves it settled, counted by the kernel | cgroup v2, a running systemd user manager, `systemd-run` (or a D-Bus client); no privilege | One transient unit start, 23–73 ms measured; one write to end it | A scope starts, and `cgroup.kill` ends a child cgroup inside it: operator host, `ubuntu-latest`, `ubuntu-24.04-arm` (run 38057456223). The scope itself found from outside and ended by one outside write: operator host (4 ms), `ubuntu-latest` and `ubuntu-24.04-arm` (2 ms each, run 38077308914). Docker container: no user manager (`offline`) | No user manager (containers, a session without one, `ssh` without lingering); the scope outlives the engine; a launcher in flight can create the scope after a cleanup, and a unit not yet made reads as one that has emptied, so neither a unit's state nor a single kill settles it alone; a process that asks the user manager for a unit of its own leaves it |
| (ii) A child cgroup under a delegated cgroup the engine stands in: the engine re-execs under `systemd-run -p Delegate=yes`, or the operator's own unit declares `Delegate=yes` | As (i), and cheaper: no unit start per attempt | The engine already in a delegated cgroup, and a way to know it is (systemd marks one with the `user.delegate` extended attribute; unmeasured here) | One `mkdir`, one write before exec, one write to end it | Under a `Delegate=yes` user scope on all three Linux hosts; never in the job's own cgroup | Detecting delegation wrongly writes into a cgroup systemd owns; the re-exec variant re-execs a binary path that may have been replaced under a live engine, and needs a loop guard; engine death leaves the children unless the unit is a service whose `KillMode` ends them |
| (iii) `clone3(CLONE_INTO_CGROUP)` against a pid write to `cgroup.procs` before exec | Both place the driver in the cgroup before it runs any of its own code, so nothing it forks can be born outside | (iii) a raw `clone3` beside `std::process::Command`; the pid write needs only the open and write a `pre_exec` hook may make | Nil either way | The pid write is what the probe did, on every Linux host | `clone3` adds unsafe spawn code for no guarantee the write lacks; a failed write fails the spawn, which is fail-closed |
| (iv) A PID namespace: bwrap's `--unshare-pid --die-with-parent`, for boxed seats | Every process in the namespace is killed when its init exits, and none can leave it | bwrap, or unprivileged user namespaces (refused by AppArmor on every measured Ubuntu) | One bwrap per attempt | bwrap everywhere but a Docker container | Today it wraps only the hands box's commands, not the harness that runs the seat; the harnesses' own sandboxes (Claude Code's and dsh's use bubblewrap) would nest inside it, unmeasured; no `populated` count to read, so the settle proof is the init's exit; `--die-with-parent` is armed on the spawning thread, not the process |
| (v) #403's polling | No descendant the engine can see is left running while an attempt is certified settled, but for the residuals decision 0006's addendum names | Linux: a subreaper and a pidfd; macOS: `ps` | A full table walk every 100 ms while an attempt runs | Every host | The two accepted Linux residuals, the accepted macOS residual, and a SIGKILL to the engine's group not reaching a seat's tree |

## What engine death does under each option

- **(i)** A transient scope is the user manager's unit, not a child of
  the engine's cgroup. It outlives the engine and runs until it empties
  or is stopped, so a SIGKILL to the engine leaves the seat running
  unless something else ends it. Two things can. A lifeline, a small
  `brokkr` process in a session of its own, reads the engine's death
  from a pidfd on the engine, not from the end-of-file of its channel,
  which a forked child that has not yet exec'd still holds open, and
  writes `cgroup.kill` for every attempt whose path it has received;
  its channel, its trigger and its protocol are stated once, in
  decision 0080's ruling 4. A restart reads each attempt's cgroup path
  from the journal, which records it before the spawn, for every
  contained attempt, open or settled, as decision 0080's ruling 3
  states once; nothing is listed by prefix, age or working directory. The user manager's
  own stop (a logout without lingering) stops every scope with it.
  Neither the lifeline nor a restart can read a scope's absence from the
  user manager, since a unit not yet made answers as a finished one
  does, and `systemd-run` may still be creating a scope when either
  runs. So the launcher must not exec the driver on its own: decision
  0080 has `systemd-run` exec a gate that, once inside the scope, waits
  for the lifeline's go-ahead and exits on its EOF, and has the lifeline
  and a restart open the scope's derived cgroup path instead of asking
  for the unit.
- **(ii)** The children live as long as the delegating unit. Under a
  scope that holds the engine, the scope stays populated by the attempts
  and does not stop. Under a service whose main process is the engine,
  the service stops when the engine dies and its `KillMode` ends the
  whole cgroup, children included (unmeasured). The lifeline serves
  here as in (i), and removes each child cgroup it ends, so a launch
  that has not yet written its pid finds none to enter; where the
  lifeline dies with the engine, nothing removes it, and such a launch
  runs its driver until a restart ends it, a residual decision 0080's
  ruling 4 declares.
- **(iii)** Not a mechanism of its own: how a driver enters the cgroup
  in (ii).
- **(iv)** `--die-with-parent` delivers a parent-death signal to bwrap
  when the thread that spawned it exits. Engine death ends bwrap, its
  init, and the namespace with it.
- **(v)** The driver leads a session of its own, so only the stop
  signals the engine handles end it; SIGKILL to the engine or its group
  leaves the seat's tree running, and a restart finds it only by reading
  the table again.

## What still needs measuring, and how

The controller runs these on the host and the runners; a boxed seat
cannot reach a cgroup tree or nest the namespaces.

1. **The lifeline end to end.** Start an engine stand-in with a lifeline
   and two attempt scopes, each running the probe's escape tree; send
   SIGKILL to the engine alone, then, in a second run, to its process
   group. Read every scope `populated 0` within the settle bound and
   every escape pid gone, on the operator's host and both Linux runners.
   Then pause a launcher before its scope exists (a `systemd-run`
   stand-in blocked on a FIFO), send SIGKILL to the engine alone, let
   the lifeline exit, and release the launcher; and again with the
   engine and the lifeline killed and a restart run before the release.
   Read that no driver ran in either ordering. Last, with one attempt
   running its escape tree, pause a second launch past the settle bound
   and send SIGKILL to the engine alone; read that the lifeline, which
   reads no end-of-file while the paused child holds the engine's end,
   ends the first attempt by its pidfd on the engine within the settle
   bound, and then release the second launch. On the delegated rung the
   launch is paused before its pid write to `cgroup.procs`: read that
   the lifeline removes both child cgroups and that the released write
   fails with `ENOENT`. On the scope rung it is paused before its exec of
   `systemd-run`: the lifeline removes nothing, since a scope's cgroup is
   the user manager's, and the released launch's gate exits with the
   cancelled-launch status and runs no driver. These are the host cases
   of decision 0080 ruling 4's
   `an_engine_killed_with_a_launch_paused_still_ends_the_live_attempt`.
2. **A zombie, an orphan and `populated`.** Kill an attempt scope whose
   driver is the engine's child and do not reap it; read
   `cgroup.events`. If an unreaped zombie keeps `populated 1`, the
   settle proof reaps the driver first, as #403's already does. Orphan
   a descendant of the driver with no subreaper set, and read which
   process adopts it, that it is reaped, and that `populated` reaches 0,
   once with the engine outside the user manager's tree (started from a
   terminal session) and once inside it (a user service): the user
   manager is a subreaper only for its own descendants.
3. **The delegation mark.** `getfattr -n user.delegate` on the cgroup of
   a `systemd-run --user --scope -p Delegate=yes` scope, of a user
   service with `Delegate=yes`, and of a terminal's own scope, on each
   Linux host. Rung (ii) is offered only where the mark reads.
4. **Re-homing.** Inside an attempt scope, start `tmux new -d`, a
   `systemd-run --user` unit, and a D-Bus activated user service; read
   which cgroup each lands in. Each one outside the scope is the residual
   containment leaves.
5. **Harness tolerance.** Run one seat of each harness, claude, codex or
   dsh, inside a per-attempt scope, and confirm its own sandbox still
   starts: a cgroup is invisible to bubblewrap, so none is expected to
   notice.
6. **Concurrency cost.** Start three scopes at once, as three lanes do,
   and time each start against the 23–73 ms measured one at a time.
7. **Logout.** With lingering off, end the user's last session while a
   scope runs; confirm the scope is stopped, its escape tree gone, and
   its cgroup path removed, so a restart that opens the path finds it
   absent.
8. **Containers.** Docker with `--cgroupns=private` and a writable
   cgroup mount, and rootless Podman with `--systemd=always`: does the
   engine's own cgroup qualify for rung (ii)?
9. **macOS.** A per-attempt launchd job (`launchctl submit`, or a job
   bootstrapped into the user domain) running the escape tree: does the
   job's process coalition keep a `setsid` double-forked descendant, and
   does removing the job end it? `sandbox-exec` confines effects, not a
   process's lifetime, and is not a candidate.
10. **The derived path.** Start a scope with `systemd-run --user --scope
    --slice brokkr.slice --unit <name>` and read that its cgroup is the
    user manager's own, as the manager's `/proc/<pid>/cgroup` reads it,
    followed by `brokkr.slice` and the scope's name, on each Linux host,
    so the path can be known before the spawn.
11. **A user manager that dies.** Kill the user manager with SIGKILL
    while a scope runs the escape tree; read that the scope's cgroup
    path stays present and populated, and whether the restarted manager
    knows the unit.
12. **A descriptor through `systemd-run`.** Spawn `systemd-run --user
    --scope` with a socket at a fixed descriptor number, as decision
    0080's launch gate receives its channel, and have the command it
    execs read and write that descriptor, on each Linux host, so the
    gate's channel is known to survive the launcher's own exec.
13. **An empty scope's collection.** End a scope started with
    `--collect` by one `cgroup.kill` write, and time how long its cgroup
    path stays present after `populated 0`, on each Linux host; then
    kill the user manager while a scope is empty and read whether the
    path is ever removed. Settlement does not wait for the collection;
    this measures how long doctor may list an empty scope cgroup.
14. **Scopes bound to the engine's unit.** Run an engine stand-in in a
    transient unit of its own (`systemd-run --user --unit
    brokkr-engine-<id>`), start each attempt scope with `systemd-run
    --user --scope -p BindsTo=brokkr-engine-<id>.service -p
    After=brokkr-engine-<id>.service`, each running the escape tree,
    and send SIGKILL to the engine; read every scope stopped, every
    escape pid gone and each scope's cgroup `populated 0` or absent, and
    time it, on the operator's host and both Linux runners. Read first
    whether `systemd-run` accepts the dependency on a scope at all.
    Decision 0080 names it under Asked as the alternative that would
    close its released-driver residual on the scope rung, at the price of
    running the engine in a unit.
    **Measured 2026-10-10** (probe commit 2ca73c8d on branch
    `probe/472-containment`, GitHub run 38089932134, and the operator's
    host). `systemd-run` accepts `BindsTo=` and `After=` on a scope. A
    SIGKILL to the engine's unit stopped the bound scope with no escape
    pid left and its cgroup absent, in 38 ms on the operator's host,
    20 ms on ubuntu-latest and 16 ms on ubuntu-24.04-arm. A scope started
    with the same dependency after the engine's unit had failed was NOT
    refused: its driver ran until an outside timeout ended it, on all
    three hosts. So the dependency ends every attempt whose scope
    exists when the engine dies, and does not close a launch that
    creates its scope later; that window still needs the launch gate.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | A per-attempt transient user scope, ended by one `cgroup.kill` write and proven settled by `populated 0`, leaves no process of a `setsid` double-fork escape tree, needs no privilege, and costs 23–73 ms to start | planned | #472 |
| 2 | Where the engine already stands in a cgroup delegated to its user, a child cgroup per attempt gives the same guarantee without a unit start | planned | #472 |
| 3 | A pid write to `cgroup.procs` before exec contains a driver as surely as `clone3(CLONE_INTO_CGROUP)`, with no spawn code beside `std::process::Command` | planned | #472 |
| 4 | Re-executing the engine under `systemd-run` to obtain delegation is not needed once a per-attempt scope is available from outside | not-planned | |
| 5 | A PID namespace ends every descendant when its init exits; Brokkr's boxed hands already run in one | implemented | decision 0043, `crates/brokkr-protocol/src/hands/namespace.rs` |
| 6 | Wrapping each harness in a PID namespace, where cgroups cannot be had, is unmeasured against the harnesses' own sandboxes and unavailable in Docker | not-planned | |
| 7 | Polling the process table cannot close the table-read race or the read-to-fork instant, and stays the fallback where no containment is offered | implemented | decision 0006, `crates/brokkr-protocol/src/process/tree.rs` |
| 8 | macOS offers no cgroup; whether a launchd job's coalition holds an escape tree is unmeasured | not-planned | |
| 9 | The user manager answers a unit never made exactly as a finished scope (no control group, `inactive`), so a unit's state cannot settle an attempt; the cgroup path must be read, and a launch must not outlive the cleanup that reads it | planned | #472 |
