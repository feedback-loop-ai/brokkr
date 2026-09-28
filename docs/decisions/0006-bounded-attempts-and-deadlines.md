# 0006 — Bounded automated attempts and seat deadlines

**Status**: accepted (operator goal directive, 2026-08-23 — "fully
autonomous, end-to-end"; enumeration by Claude under that directive)

Built: built
Amended by: 0067

## Ruling

Autonomy requires bounded self-recovery. Two per-seat limits become bundle
data (`seats.<phase>.limits`), defaulting to the previous behavior:

- `max_attempts` (default 1): a **determinately failed** driver attempt —
  protocol violation, driver-reported failure, non-zero exit, or deadline
  kill — may be retried automatically with a fresh `attempt_id` on the
  same open effect, up to this limit. When it is exhausted the run parks,
  citing the attempt count and the last recorded error.
- `timeout_seconds` (default 3600): a watchdog kills the driver process at
  the deadline. Because the engine killed it before any result, the
  non-completion is **determinate**: the attempt is `failed` (retryable),
  not `indeterminate`. A hung seat session can no longer hang a run.

An `effect/indeterminate` attempt is NEVER retried automatically,
whatever the limit: completion could not be established, so a retry could
silently duplicate or re-pay for completed work. Indeterminate always
parks into operator judgment — unchanged from decision 0003's outbox
discipline.

## Why

- This is the engine-level analog of what decision 0001 explicitly
  allows *inside* an executor: bounded retries are the seat producing its
  own result, not the control plane repairing one. No retry ever selects
  a transition; the policy still rules on whatever result finally arrives.
- The failed/indeterminate boundary is the safety line: `failed` means we
  KNOW no work product was accepted; `indeterminate` means we don't know,
  and guessing is what this engine exists to refuse.
- Without deadlines, "fully autonomous" is false advertising: one hung
  CLI process wedges the run forever with no journal evidence.

## Mechanics

- Fold: `effect/failed` returns the open effect to the executable
  position with `failed_attempts` counted; `run/parked` is legal from
  that position (engine exhausted the limit). Both are recorded in
  contracts/README.md's fold semantics.
- Every attempt still follows the durable discipline: `effect/started`
  durable before spawn, exactly one terminal fact per attempt.
- Limits are compile-validated (integers ≥ 1; unknown keys rejected).

## Consequences

- Machine proof grows: transient-failure retry completes; exhausted
  limit parks with the last error; a hung driver is killed at its
  deadline; indeterminate never auto-retries even with attempts left.
- Default limits (1 attempt, 1-hour deadline) preserve every previously
  proven behavior; bundles opt into more attempts per seat.
- The self bundle adopts explicit limits in a follow-up once the
  in-flight ship-taxonomy delivery lands (avoids conflicting with that
  run's own edits to `bundles/self/`).

## Addendum — 2026-09-27, the kill takes the attempt's process tree (#403), proposed

The deadline kill above reached only the driver process. The harness a
driver starts, and that harness's tool subprocesses, outlived it: on
2026-09-25 a Claude session went on writing for 22 minutes after the
journal recorded its attempt killed, and a retry could start beside it
in the same worktree. The non-completion this decision calls
determinate was not.

- Two means reach the tree. The driver leads a process group of its
  own, and the kill is SIGKILL to the group. While the attempt runs, the
  engine also records every descendant by pid and start time, a
  descendant that left the group (`setsid`, as Node's `detached` spawn
  does, or a job that shell job control moves to a group of its own)
  included. Every kill reads the table once more first, and ends those
  identities too. A pid alone is never signalled. Nothing is chosen by
  working directory, so a concurrent run in the same checkout is never
  signalled.
- On Linux the engine and every driver are child subreapers. An orphan
  of the tree goes to its driver while the driver runs, where it is
  recorded as a descendant. Once the driver is gone, it goes to the
  engine and not to init, whatever its session. Every running child of
  the engine outside the engine's own group (a child the engine spawns
  stays in it) that no live attempt leads, records or began after is a
  stray. A stray is attributed to the attempts that could have left it:
  those it does not predate, whose driver no longer runs, and that had
  not yet ended when it was born, as the start stamps order it. One such
  attempt ends it with its own tree. When there are several, the stray
  is ended and every one of them parks. When there is none, the stray is
  the engine's own, as git's detached maintenance is. So on Linux no
  descendant the engine can see is left running while an attempt is
  certified settled.
  A kernel that refuses the engine the subreaper or a pidfd leaves this
  second means absent, and then a kill that saw any descendant parks,
  naming the means that was missing.
- Every attempt ends the same way, whatever ended it: `shutdown`, written
  without waiting past the grace; a bounded grace for the driver to exit;
  a read of the table, then SIGKILL to the group and to every identity the
  attempt owns; a bounded reap; and a bounded wait for the process table
  to show nothing of the tree running. A zombie counts as gone. The group
  is signalled only while its leader is unreaped, so its id cannot name
  another process.
- The engine ends every live attempt the same way when SIGINT, SIGTERM or
  SIGHUP tells it to stop, strays included. It then waits, within the
  settle bound, for the table to read every attempt gone, and exits with
  128 plus the signal. When it cannot prove that, it says why and exits
  125. The driver no longer shares the engine's group, so a terminal's
  Ctrl-C or hangup would not otherwise reach it. A signal the engine
  inherited as ignored (`nohup`) stays ignored.
- The report returns only once the tree is proven gone, so the retry
  this decision allows cannot overlap it.
- An end that cannot be proven is `indeterminate`, by this decision's own
  line: completion of the cleanup is not known. That covers a group or a
  recorded descendant still running, a stray that could not be attributed
  to one attempt, a kill the kernel refused on the group or on a live
  identity, a leader not reaped, a missing means, and a pipe something
  outside the tree still holds. It also covers a table that cannot be
  read whole: a row that cannot be read or parsed, a `ps` that exits
  nonzero, and a snapshot without the engine's own row. A row that
  vanished between the listing and its read is gone. The report keeps
  the outcome the attempt reached, typed, beside an unresolved cleanup,
  and the engine journals both. A checkpoint the journal refused changes
  the outcome acted on, never the outcome received. The engine never
  certifies the outcome or retries it.
- macOS has no subreaper and no pidfd. There the engine reads the table
  synchronously at the kill, ends what it attributes, and parks on any
  doubt. The operator's ruling of 2026-09-28 (LINUX CLOSED, MACOS
  RESIDUAL ACCEPTED) accepts one residual there: a descendant that
  leaves the group and whose parent exits between two reads of the
  table, faster than the tracker's 100 ms interval, is reparented to
  launchd unseen. Separately, and not a limit of settlement: without a
  pidfd, a pid reused between `ps`'s confirmation and the signal could
  be signalled.
