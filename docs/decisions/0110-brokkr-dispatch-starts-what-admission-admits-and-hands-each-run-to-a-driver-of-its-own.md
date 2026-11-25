# 0110 — `brokkr dispatch` starts what admission admits, hands each run to a driver of its own, and repeats its pass while it stands

Status: accepted — operator ruled in chat 2026-11-16, "go with recommended on all seven" (https://github.com/feedback-loop-ai/brokkr/issues/430#issuecomment-6913524052)
Date: 2026-11-16

Built: built
Amends: 0068

## Context

This is #430's sixth and last slice:

> 6. Provide a single-pass command and standing loop with deterministic
> clock/probe seams, readable queue state, recovery documentation and
> Linux/macOS coverage.

Decision 0068 ruling 7, in the words decision 0106 gave it on 2026-11-16,
says what the process may do:

> `brokkr dispatch` runs as one process the operator starts, or one pass on
> demand; it starts runs, records cool-downs, sweeps, and spends one entry's
> declared retry budget on the one park cause ruling 4 permits, and does
> nothing else. It never retries a run the policy parked, except a
> queue-started run parked with the typed cause `infrastructure`, with
> `retryable` true on positive non-execution evidence and while that entry's
> declared retry budget remains; it never retries a `ruled`, `attempt-cap`
> or `provider-limit` park, never concludes or stops a run, and never changes
> an entry's order — those remain operator commands.

Decision 0095's plan names what the command calls: "the `brokkr dispatch`
command and its standing loop, which first calls the claim, start, link and
spawn these slices build, all four for one entry before the next claim
(ruling 7), disposes of a refused start by its variant, holding at the
journal's bound and retrying a `Contended` disposal until it lands, leaves a
`Contended` link or spawn to reconciliation, and carries the command that
sets the bound."

Three later decisions hand this slice one task each:

- 0103 ruling 5 keeps a cool-down's wait inside the engine's own process
  until this slice, and leaves a dispatcher-paced resume to it as an option.
- 0104 ruling 9 has this slice call the sweep on every pass.
- 0106's slice 0106d spends its retry budget from this slice's pass.

Everything the pass calls is built. What is missing is the process that
calls it, the driver it starts, and the words an operator reads. This
decision says how those are built, before code.

### What main holds, measured at fb7b775c1

- **The lock and its holder.** `take_dispatch`
  (`crates/brokkr-runtime/src/dispatch/lock.rs:72`) takes
  `<journal>.dispatch.lock` and appends a holder row of the next generation.
  `Dispatcher::took` binds the dispatcher to the id its journal names itself
  by.
- **Reconciliation.** `reconcile`
  (`crates/brokkr-runtime/src/dispatch/reconcile.rs:63`) links or reclaims
  each entry left launching. For each linked entry with no `spawn` row it
  hands back `Settled::ToSpawn` (`:32`), and its module says "Spawning is
  the `brokkr dispatch` loop's, never this" (`:15`).
- **The steps.** The store has all of them:
  - `queue_claim_with` (`crates/brokkr-store/src/queue/claim.rs:669`);
  - `queue_link` (`:685`) and `queue_spawn` (`:706`);
  - `queue_reclaim` (`:733`), which counts a `StartRefusal` (`:125`:
    `Crossing`, `Compile`, `Unboxable`) and holds the entry at the bound;
  - `queue_unsettled_with` (`:766`);
  - `claim_stands` (`:521`), the fence a start runs inside its transaction.

  The bound has a reader and a writer, `dispatch_bound` and
  `set_dispatch_bound` (`crates/brokkr-store/src/queue/holder.rs:134`,
  `:141`). No command sets it.
- **Admission.** `judge_within`
  (`crates/brokkr-runtime/src/admission.rs:505`) latches and judges the
  queue. Each admitted entry's `Judged` carries a `Room` (`:397`). Its
  `claim` (`crates/brokkr-runtime/src/admission/capacity.rs:442`) re-checks
  the waits, the cool-downs and the census inside the claim's transaction.
- **The start.** `launch::start` (`crates/brokkr-runtime/src/launch.rs:313`),
  with `NewRun::claim` set:
  - starts the run under `claim_stands`;
  - pins `queue_entry` in its manifest (run-manifest v14);
  - takes the run's lock (decision 0108 ruling 1, `run_lock::take`,
    `crates/brokkr-runtime/src/run_lock.rs:118`) before `run/started`.

  The `Engine` holds that lock until a drive returns. `launch::resume`
  (`launch.rs:351`) takes it again. `QueuedLaunch::rebuild`
  (`crates/brokkr-runtime/src/launch/queued.rs:306`) makes the request and
  the run from an entry's payload, and sets no claim.
- **The sweep.** `sweep::pass` (`crates/brokkr-runtime/src/sweep.rs:315`)
  takes a `Dispatcher`. `brokkr sweep` takes the lock for its one pass
  (`crates/brokkr-cli/src/verbs/sweep.rs:32`).
- **The CLI.** There is no `dispatch` verb. These files are at or near the
  800-line ceiling:

  | File | Lines |
  |---|---|
  | `crates/brokkr-runtime/src/admission/capacity.rs` | 800 |
  | `crates/brokkr-cli/src/cli_args.rs` | 784 |
  | `crates/brokkr-runtime/src/admission/host.rs` | 778 |
  | `crates/brokkr-runtime/src/admission.rs` | 776 |
- **Precedents in the tree.**
  - The CLI already starts its own binary as a child in a process group of
    its own (`crates/brokkr-cli/src/broker/session.rs:645-655`).
  - It ends a serve loop on SIGINT and SIGTERM through `signal_hook`
    (`crates/brokkr-cli/src/broker.rs:48-54`).
  - A driving engine makes its process a child subreaper on Linux
    (`crates/brokkr-protocol/src/process/attempts.rs:655`).
- **The host file.** `forge.host/v3` declares providers and routes, floors,
  `boxed_builds`, `cooldowns` and `containment`. Nothing in it bounds how
  many runs one journal drives at once. The census
  (`crates/brokkr-runtime/src/admission/capacity/census.rs:28`) counts every
  run of the journal that reads `Running`, direct runs among them.

### What the controller does by hand today

- **Lane scripts start runs.** Each runs under a service manager's unit of
  its own, because a driver started from an interactive session ends with
  that session. Each polls the journal to tell a parked run from a finished
  one.
- **No more than three drivers write one journal.** That rule is the
  controller's, and it is kept by hand. On 2026-09-24, six drivers on one
  journal left four of them exiting with "contended: a peer still held the
  journal's write lock". Their runs read `running` for about seven hours
  with no driver.
- **#1000: parallel writers corrupted a shared journal.** It happened in 3
  of 4 trials under the bundled SQLite 3.46. The bundled SQLite has since
  moved past that bug's fix (#1044), and the three-driver rule stays as a
  contention limit.

## Rulings

Each ruling gives its options, what each costs, and the recommended one.

### 1. The pass

**Recommended: (a) one pass, under the journal's dispatch lock, in a fixed
order.**

1. **Reconcile** (0095 ruling 3). Write a `spawn` row for each entry it
   hands back to spawn, then start that entry's driver (ruling 3).
2. **Sweep,** through 0104's `sweep::pass`, so the space it frees counts
   before the floors are judged.
3. **Read the bound.** A bound that does not read refuses the pass, which
   then claims nothing (0095 ruling 7).
4. **Judge the queue** with `judge_within`, as the operator word `dispatch`,
   latching what it finds.
5. **Start each admitted entry,** in the queue's order. One entry finishes
   its claim, start, link, `spawn` row and driver before the next entry is
   claimed (0095 ruling 7). A claim that the re-check refuses writes
   nothing, and the pass goes on to the next entry.
6. **Report** what the pass did (ruling 5).

**Two of 0068 ruling 7's words, read in this order.**
- **Cool-downs.** The dispatcher records none of its own. The engine records
  the provider's refusal (0103 ruling 2), and this pass reads it through
  admission and the claim (0103 ruling 6). That is how this decision reads
  "records cool-downs".
- **The retry budget.** 0106d's spend goes after step 1 and before step 4.
  A run it retries then counts in the census the judge reads.

**Options:**
- **(b) Judge first and sweep last,** in the order 0068's sentence lists
  them. An entry waiting on a floor that the sweep would free then waits a
  whole interval.
- **(c) Sweep on a cadence of its own.** That is a second timer for one
  walk.

**Enforcement binding:** runtime tests through a seam that records each
step's order:
- a pass that reconciles, sweeps, reads the bound, judges, starts and
  reports, in that order;
- an unreadable bound row refuses the pass with no claim;
- with two admitted entries under a ceiling of 1, the first starts. The
  second's claim refuses on capacity, by the re-fold after the first's link.
  No second run is written.

### 2. One entry's start, and what each refusal writes

**Recommended: (a) a closed match from the start's outcome to its
disposal,** with one arm per `LaunchError` variant and no wildcard (0071
ruling 2):

| The start | What the dispatcher writes | Counted |
|---|---|---|
| The run is written | Drops the engine, which lets the run's lock go; `link`; `spawn` row; the driver (ruling 3) | — |
| The bundle moved (`QueueRefusal::BundleMoved`, from `claim_stands`) | `reclaim`, naming both digests | no |
| `Contended` | `reclaim` | no |
| The crossing fence; compiling the entry's launch, which takes in rebuilding it from its payload (the map, the shelf, the recipe and the bundle); `refuse_unboxable` | `reclaim`, or `hold` at the bound, with its `queue_refusals` row | yes |
| Fenced out (`QueueRefusal::Superseded`) | Nothing. The dispatcher ends. | — |
| Any other refusal | Nothing. The dispatcher ends, naming the refusal. | — |

**What "any other refusal" means.** It covers the store's I/O and
corruption errors, and a run lock that 0108 ruling 1 refuses because its
directory is not this user's own. Each one says the journal or the host
needs the operator. The entry stays `launching`. The next dispatcher's
reconciliation then reclaims it uncounted, since no run carries its claim.

**Steps after the run exists** keep 0095 ruling 7's rules:
- A `link` or `spawn` row that is `Contended` past its patience is left for
  reconciliation, and never reclaimed.
- A `Contended` disposal is retried until it lands, with no other claim in
  between.

The build lists each `LaunchError` variant's class in its pull request.

**Options:**
- **(b) Hold an unclassified refusal at once.** That needs a fourth refusal
  word, or a store step with no counted word, for a case the operator reads
  anyway.
- **(c) Reclaim it uncounted.** A refusal that recurs then claims and
  journals again on every pass, forever.

**Enforcement binding:** runtime tests, each through a fault seam:
- an edit that moves the bundle between the pass and the start reclaims
  uncounted, naming both digests;
- a `Contended` start reclaims uncounted;
- a compile refusal under a bound of 1 holds the entry, with the variant as
  the hold's reason;
- a newer holder row makes the next step refuse, and the dispatcher writes
  nothing after;
- a planted store error ends the dispatcher with the entry `launching`, and
  a second dispatcher reclaims it uncounted;
- a `Contended` link leaves the entry linked with no `reclaim`, and the next
  pass's reconciliation spawns it.

Each test binds by a mutation that drops its arm (0071 ruling 9).

### 3. A driver of its own

**Recommended: (a) after the `spawn` row, the dispatcher starts its own
binary as `brokkr dispatch drive` for that entry, a process of its own that
resumes the run.**

**The command:** `brokkr dispatch drive --db <journal> --entry <id> --claim
<seq>`, hidden from help.
- It refuses, writing nothing, unless the entry is linked under that claim
  and has a `spawn` row.
- It rebuilds the launch from the entry's payload, as the start did.
- It resumes the linked run (`launch::resume`), which takes the run's lock
  (0108 ruling 1).
- It prints the plain lines that `brokkr resume --no-view` prints, and exits
  as resume does.

**The process:**
- It runs in its own session and process group.
- Its stdin is `/dev/null`. Its stdout and stderr are appended to
  `<journal>.runs/<run-id>.log`, beside 0108's lock file.
  - The log is opened the way that lock file is: this user's own regular
    file of one link, never through a link, and created mode 0600.
  - Brokkr never removes it.
- Its working directory is the entry's workspace.
- Its environment is the dispatcher's, as for any child.
- It holds none of the dispatcher's descriptors but its three standard
  streams. The dispatch lock and the run locks are opened close-on-exec
  (0095 ruling 2, 0108 ruling 1), and the spawn passes nothing else.

**The hand-off.** The dispatcher drops the engine after the start, which
lets the run's lock go, and then writes the `link` and the `spawn` row, and
spawns.
- In the window between the release and the driver's take, an operator's
  `resume` or `conclude` is lawful and wins. The driver then refuses, its log
  says why, and the dispatcher reports its exit (ruling 5).
- A spawn that fails after its row has been written leaves the run reading
  `running` with no driver. That happens when the exec is refused or no
  descriptor is free. The run is never spawned again (0095 ruling 3), and
  the report gives the command that resumes it.

**Options:**
- **(b) Drive in the dispatcher's own process.**
  - One crash or stop then ends every run.
  - One process becomes the subreaper of every run, and has to stay alive
    for as long as any run does.
  - It goes against ruling 7's "does nothing else".
- **(c) Spawn `brokkr resume` with the request spelled out in argv.** That
  derives the payload a second time (0071 ruling 5), and nothing ties the
  driver to the claim.
- **(d) Hand the held run lock's descriptor to the driver.** There is no
  window, but 0108's lock gains an inherited mode on every driving path. The
  window it closes only the operator's own act can use.
- **(e) A service manager per run** (`systemd-run`, launchd). That is
  Linux-only or macOS-only (0063), and needs a manager the host may not run.

**Enforcement binding:**
- An end-to-end test on the Linux and macOS legs runs `brokkr dispatch
  --once` on a journal with one queued entry of a provider-free recipe. It
  asserts:
  - the entry is linked and has a `spawn` row;
  - the driver's session is not the dispatcher's;
  - the log is this user's own, mode 0600;
  - on Linux, the driver's open descriptors are its standard streams and its
    own, none of them the dispatcher's lock;
  - the run ends `completed`.
- `dispatch drive` refuses a claim with no `spawn` row, writing nothing.
- An operator `resume` taken in the window makes the driver refuse with
  0108's words.

### 4. The binary a driver runs

**Recommended: (a) the dispatcher spawns only the binary it was started
from, and ends when that file is replaced.**
- **At start,** the dispatcher records the path it was started from and that
  file's device and inode.
- **Before each claim and before each `spawn` row,** it checks that the path
  still names that file.
- **If the file was replaced,** for example by an upgrade that renamed a new
  binary into place, it claims nothing more, writes no `spawn` row, and ends
  with this refusal:

  ```text
  dispatch: the brokkr at <path> is not the one this dispatcher started from; it claimed nothing after; start the dispatcher again
  ```

  An entry already linked waits for the next dispatcher's reconciliation,
  which spawns it from that dispatcher's own binary. Drivers already running
  are not touched.

**What is left.** A file replaced between the check and the exec, a matter
of microseconds, yields a driver of another version. That driver resumes
under the run's pinned bundle, and resume refuses drift, so the run reads
`running` for the operator.

**Options:**
- **(b) Exec the running image through `/proc/self/exe`.** That is
  Linux-only (0063). It also runs a deleted binary, so an upgrade silently
  never reaches new runs until someone restarts the dispatcher.
- **(c) No check.** A newer binary then drives runs, and may migrate the
  journal under the peers that share it.

**Enforcement binding:** a CLI test copies the binary to a temporary path,
starts the dispatcher from that copy, and replaces the copy. It then asserts
that the next pass claims nothing and ends with the refusal's words.

### 5. The standing loop

**Recommended: (a) `brokkr dispatch` repeats the pass every `--every
<seconds>`,** default 60 and from 5 to 3600, reconciling before each pass
(0095 ruling 3). `brokkr dispatch --once` runs one pass and returns without
waiting on its drivers.

- **Time.** The loop reads `now` and sleeps through one seam, which the
  tests drive. Nothing else in it reads the clock (0071 ruling 1).
- **Signals.**
  - SIGINT or SIGTERM ends the loop once the entry in flight has its `spawn`
    row and driver. No claim follows, and the sleep wakes at once.
  - A second signal ends it at once. That is a crash, which the next
    reconciliation settles (0095 ruling 3).
  - The drivers are in their own sessions, and are not signalled.
- **Reaping.**
  - Between passes, the dispatcher reaps every exited child without waiting,
    and reports each driver's exit.
  - Where that driver's run still reads `running`, it says the run needs the
    operator's `resume`.
  - It never resumes, adopts, stops or concludes a run (0068 ruling 7). 0106
    leaves adoption to a follow-up decision.
- **What ends the loop,** each case exiting non-zero with its words:
  - being fenced out;
  - a replaced binary (ruling 4);
  - an "any other" start refusal (ruling 2);
  - a journal whose queue storage this binary does not write.

  Everything else a pass meets is reported, and the next pass judges again,
  because none of it latches (0068's 2026-10-10 addendum). That includes an
  undeclared or unreadable host, an entry that does not read, a run that
  does not fold, and a contended write.
- **The report.**
  - A pass prints what it did: reconciled, swept, started (with the entry,
    the run, the driver's pid and its log), reclaimed, held, refused and
    reaped.
  - One line counts the entries still waiting and held.
  - `brokkr queue list` stays where each entry's verdict is read.

**Options:**
- **(b) Wake on the journal's changes** (inotify, kqueue). That is two
  platform paths, for a file every driver appends to.
- **(c) Also wake at the earliest cool-down's end.** That saves minutes on
  waits that last minutes, and can be added later if wanted.
- **(d) Restart itself after an ending refusal.** That hides a refusal meant
  for the operator.

**Enforcement binding:** runtime tests drive three ticks through the seam.
- A signal during an entry's steps ends the loop after its driver, with no
  further claim.
- A second signal ends it before the `spawn` row, and reconciliation then
  spawns the entry.
- An exited driver is reaped, and its still-running run is reported.
- An unreadable host is reported, and the next tick judges again.
- A fence-out ends the loop non-zero.

### 6. The bound's command, and what the operator reads

**Recommended: (a) two subcommands, `set-bound` and `status`.**

- **`brokkr dispatch set-bound <n> --reason <text>`** sets the journal's
  bound through `set_dispatch_bound`, as the operator's journaled command.
  It refuses 0 and anything that is not a whole number (0095 ruling 7).
- **`brokkr dispatch status`** opens the journal read-only, as
  `brokkr sweep --dry-run` does, and writes and migrates nothing.
  - It reads the latest holder row.
  - It tries the lock shared and non-blocking, and lets it go at once. It
    then says either "held by generation N (pid P on this host since T)" or
    "not held; last held by generation N …".
  - It also names the bound in force, and each entry that is launching, or
    linked with no `spawn` row.
- **The probe's race.** A dispatcher that starts in the instant the probe
  holds the lock refuses once, naming the holder row. Started again, it
  takes the lock.

**Options:**
- **(b) The holder row alone,** claiming no liveness.
- **(c) Check the holder's pid with `kill(pid, 0)`.** A reused pid reads
  alive.

**Enforcement binding:**
- CLI tests: `set-bound 0` and `set-bound x` refuse, and `set-bound 2`
  governs the next pass.
- `status` names a live holder as held and a dead holder's journal as not
  held, and leaves the journal's bytes unchanged.

### 7. A ceiling on the runs one journal drives (amends 0068 ruling 3)

**Recommended: (a) `forge.host/v4` adds a required `runs` object, `{"ceiling":
n}`, with n at least 1.** It is the most runs that may read `running` in the
journal a pass judges, counted as the census counts them, direct runs
included.

- **A full journal.** An entry that would take the count past the ceiling
  waits as `runs_full`, naming the ceiling and the count. The claim's
  re-check counts the runs that moved past the high-water against it, as it
  does for providers (0095 ruling 7).
- **No ceiling declared.** Every v1, v2 and v3 file declares none. Under such
  a host, each entry the capacity checks would judge waits as
  `runs_undeclared`, naming the host file, as an undeclared provider does.
  - `brokkr doctor` names the version that declares the ceiling.
  - v3 stays readable and frozen, as each earlier version is.
- **0068 ruling 3's second bullet** gains a clause. Its text "every provider
  its compiled bundle will seat has capacity — a concurrency ceiling per
  provider and per route, declared in the realm's host configuration — and
  is not in a cool-down" is read with "and the journal drives fewer runs
  than the ceiling the host configuration declares for it" added. The rest
  of 0068 stands.

**Options:**
- **(b) A fixed number in code.** That makes a machine fact into code,
  against 0068's 2026-10-10 addendum.
- **(c) None: the provider and route ceilings bound it.** Three providers at
  three each would admit nine drivers on one journal, three times what the
  controller runs by hand.
- **(d) An optional key, absent meaning unbounded.** A capacity left
  undeclared would then admit entries, which 0068's addendum refuses.

**Enforcement binding:**
- `contracts/host.v4.schema.json` beside v3, with v3 added to `FROZEN`.
- Admission tests:
  - at the ceiling, the entry waits `runs_full`, with its words;
  - a run started between the pass and the claim refuses the claim on the
    ceiling;
  - a v3 host makes the entry wait `runs_undeclared`;
  - a direct run counts.
- A doctor test for the v3 line.

## What stays out

- **Adopting a run whose driver died.** 0106 leaves it to a follow-up
  decision on stable-identity engine ownership. Until then, the report names
  such a run and the operator resumes it.
- **A dispatcher-paced resume** in place of 0103's in-process wait. The
  engine keeps the wait. A later decision may move it.
- **0106d's budget.** This decision gives it its place in the pass (ruling
  1), and builds nothing of it.
- **Other journals and other hosts.** One dispatcher holds one journal, and
  runs in other journals are not counted (0095 rulings 2 and 6). Decision
  0074's many-host claims stay deferred.
- **A readout of the dispatcher in the run view or the TUI.** 0103's slice
  4f paints the waits. A dispatcher panel would be its own slice.
- **Rotating or removing driver logs.**
- **Service files.** The guide shows a user unit and a launchd agent. Brokkr
  installs neither.
- **Windows** (0063).
- **0068 ruling 2's named endings,** which wait for #317. *Built
  2026-11-23 by #1192:* `--after ENTRY:ENDING` waits on any of decision
  0064's endings but `unclassified`, and a run over that ended otherwise
  holds the entry as `ended_otherwise`.

## The slices

Smallest first. Each lands alone, tested, through the merge queue.

1. **6a — the pass and one entry's start, with a spawn seam.**
   - **Files:** `crates/brokkr-runtime/src/dispatch.rs` (its module lines),
     and two new files: `dispatch/pass.rs` (ruling 1) and `dispatch/start.rs`
     (ruling 2's steps and its disposal match). The seam takes `(entry,
     claim, run)` in place of a process.
   - **No CLI.**
   - **Size:** about 300 production lines and 400 test lines.
   - **Tests:** rulings 1 and 2.
2. **6d — the run ceiling.** Independent of 6a, so it can go in parallel.
   - **Files:**
     - `contracts/host.v4.schema.json`;
     - `crates/brokkr-runtime/src/admission/host/shape.rs` and its v4 read;
     - a new `admission/capacity/runs.rs`, since `capacity.rs` is at 800
       lines;
     - the doctor line;
     - `docs/guides/versioning.md`;
     - `FROZEN`.
   - **Size:** about 150 production lines and 200 test lines.
   - **Tests:** ruling 7. It lands before 6b, which is the first slice that
     spawns a driver from the CLI.
3. **6b — the verbs and the driver.**
   - **Files:**
     - new `crates/brokkr-cli/src/cli_args/dispatch.rs` and
       `crates/brokkr-cli/src/verbs/dispatch.rs`: `--once`, `drive`,
       `set-bound` and `status`;
     - one variant line in `cli_args.rs` and one arm in `lib.rs`;
     - new `crates/brokkr-runtime/src/dispatch/spawn.rs`: the session, the
       log and the binary's identity.
   - **Size:** about 350 production lines and 350 test lines.
   - **Tests:** rulings 3, 4 and 6, on the Linux and macOS legs.
4. **6c — the standing loop and the guide.**
   - **Files:**
     - new `crates/brokkr-runtime/src/dispatch/standing.rs`: the seam, the
       signals and the reaping;
     - the verb's loop;
     - `docs/guides/dispatching.md`: running the dispatcher under a user
       unit with `KillMode=process`, or a launchd agent; what a crash, a
       fence-out, a replaced binary and a driver that died each leave; and
       how each is recovered.
   - **Size:** about 200 production lines and 250 test lines.
   - **Tests:** ruling 5.
   - It moves the `Built:` markers this slice finishes: this decision's,
     0095's and 0068's.

## Asked of the operator

Each is listed with its recommendation first.

1. **The pass's order:** reconcile, sweep, read the bound, judge, start, then
   report, with 0106d's spend placed after reconciliation (ruling 1). The
   alternatives are judging first and sweeping last, or a separate sweep
   cadence.
2. **A start refused for a reason outside 0095's classes** ends the
   dispatcher, and the entry is reconciled uncounted (ruling 2). The
   alternatives are holding it at once, or reclaiming it uncounted every
   pass.
3. **The driver** is the dispatcher's own binary as `dispatch drive`, in its
   own session, logging to `<journal>.runs/<run-id>.log`. The window between
   the start's lock release and the driver's take is accepted (ruling 3).
   The alternatives are driving in-process, spawning `brokkr resume`,
   handing the lock descriptor over, or a service manager per run.
4. **The binary's identity is checked before each claim and each `spawn`
   row,** and a replaced binary ends the dispatcher (ruling 4).
5. **The standing loop:** a fixed interval of 60 seconds by default, ending
   on a signal after the entry in flight, reaping its drivers, and ending
   only on the refusals ruling 5 names.
6. **`dispatch set-bound` and `dispatch status`,** with status's lock probe
   and its race (ruling 6).
7. **A declared run ceiling in `forge.host/v4`,** amending 0068 ruling 3,
   with v1 to v3 hosts waiting as `runs_undeclared` (ruling 7). The
   alternatives are a constant in code, no run ceiling, or an optional key.
8. **Public API, estimated and not measured:**
   - `brokkr-runtime`: about eight items, namely the pass, its report, the
     start's disposal, the spawn's facts, the standing loop's entry and its
     ending refusals, and the two new `Capacity` arms.
   - `brokkr-store`: none.
   - `brokkr-core` and `brokkr-view`: none predicted.

   Each crate's `quality/public-api/` snapshot moves in its slice, by this
   ruling.

   **Witness digests,** predicted and not measured: none. No bundle changes,
   and a dispatched run's `run/started` is the one 0095 already pins.

## Ruled (operator, in chat, 2026-11-16)

The operator answered "go with recommended on all seven" (#430,
https://github.com/feedback-loop-ai/brokkr/issues/430#issuecomment-6913524052), and each ruling above was taken as recommended:
1. 1(a), one pass in a fixed order: reconcile, sweep, read the bound, judge,
   start each admitted entry one at a time, report; 0106d's spend after
   reconciliation.
2. 2(a), a closed match from a start's outcome to its disposal; a refusal
   outside 0095's classes ends the dispatcher.
3. 3(a), each run driven by the dispatcher's own binary as `brokkr dispatch
   drive`, in a session of its own, logging beside the run's lock, with the
   lock window accepted.
4. 4(a), the binary's identity checked before each claim and each `spawn`
   row, a replaced binary ending the dispatcher.
5. 5(a), the standing loop on a fixed interval, 60 seconds by default,
   ending on a signal after the entry in flight, reaping its drivers, and
   ending only on the refusals ruling 5 names.
6. 6(a), `brokkr dispatch set-bound` and `brokkr dispatch status`, with the
   probe's race accepted.
7. 7(a), a required `runs.ceiling` in `forge.host/v4`, amending 0068 ruling
   3, with a v1 to v3 host making each entry wait as `runs_undeclared`.

The eighth ask, the public API estimate, was taken as recommended under the
operator's standing ruling of 2026-11-04 ("go with recommended on anything
else that comes up").

## Consequences

- **What the operator runs.** One command per journal replaces the lane
  scripts' start-and-poll loops. The host file now declares, as data, the
  controller's rule of three drivers per journal.
- **A driver outlives its dispatcher.** Stopping the dispatcher stops no
  run. A driver that dies leaves its run `running` for the operator, as a
  crashed `brokkr run` does, and the standing loop names it on the pass
  after.
- **An upgrade cannot reach a live dispatcher's new runs unseen.** The
  dispatcher ends at the first claim after its binary is replaced, and the
  next one spawns from the new binary.
- **Hosts on an older file wait.** Every entry under a host file older than
  v4 waits as `runs_undeclared` until the operator declares a ceiling. No
  dispatcher runs today, so nothing that runs now stops.
- **Crash safety is 0095's,** and this decision only calls it: reconciliation
  before every pass, one entry at a time, and the disposal by variant.
- **The accepted costs:**
  - the lock window of ruling 3, which only an operator's act can use;
  - status's probe race (ruling 6);
  - a replaced binary in the microseconds between ruling 4's check and the
    exec;
  - a driver log that grows until the operator removes it.
- **Accepting this decision builds nothing,** moves no digest, and certifies
  no host run, remote CI or publication.

## Evidence

- **The three-driver rule** is the controller's, kept since 2026-09-24. That
  day, six drivers on one journal left four exiting "contended" and their
  runs reading `running` with no driver for about seven hours. It is
  recorded in the session that ran them.
- **#1000** records the corruption of a shared journal by parallel writers
  in 3 of 4 trials under SQLite 3.46. #1044 moved the bundled SQLite past
  that fix.
- **The lane scripts and their units** are the controller's.
- **Every file and line cited** was read at fb7b775c1, main on 2026-11-16.

## Addendum — 2026-11-25, proposed for #1374

Status: proposed

A machine can report memory pressure during a dispatch pass and clear it
before the next command. The pass's own report can explain the wait
using the verdict it already received from admission.

1. A pass retains the entry ID and the ordered reasons from each waiting
   verdict. The waiting count is the length of that list. The runtime
   dispatch tests bind the report to the verdict's typed reasons.
2. The pass prints one safe line per waiting entry in the form
   `waiting: entry <id>: <kind>: <words>`. It joins reasons with a
   semicolon and a space. The queue counts still end the pass report.
   The CLI dispatch report tests bind the exact lines and their order.
