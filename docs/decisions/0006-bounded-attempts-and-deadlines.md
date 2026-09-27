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
  does) included, and the kill ends those identities too. A pid alone is
  never signalled. Nothing is chosen by working directory, so a
  concurrent run in the same checkout is never signalled.
- On Linux the engine is a child subreaper, so an orphan of the tree is
  adopted by the engine and not by init. An adopted orphan in a session of
  its own that no attempt recorded cannot be attributed. While one is
  running, the attempt that is ending is not proven over.
- Every attempt ends the same way, whatever ended it: `shutdown`, written
  without waiting past the grace; a bounded grace for the driver to exit;
  SIGKILL to the group and the recorded descendants; a bounded reap; and
  a bounded wait for the process table to show nothing of the tree
  running. A zombie counts as gone. The group is signalled only while its
  leader is unreaped, so its id cannot name another process.
- The engine ends every live attempt the same way when SIGINT, SIGTERM or
  SIGHUP tells it to stop, then exits with 128 plus the signal. The driver
  no longer shares the engine's group, so a terminal's Ctrl-C or hangup
  would not otherwise reach it. A signal the engine inherited as ignored
  (`nohup`) stays ignored.
- The report returns only once the tree is proven gone, so the retry
  this decision allows cannot overlap it.
- An end that cannot be proven is `indeterminate`, by this decision's own
  line: completion of the cleanup is not known. That covers a group or a
  recorded descendant still running, an unattributed adopted orphan, a
  kill the kernel refused, a leader not reaped, a table that cannot be
  read, and a pipe something outside the tree still holds. The report
  keeps the outcome the attempt reached, typed, beside an unresolved
  cleanup, and the engine journals both. It never certifies the outcome
  or retries it.
- The residual is a descendant that is born, leaves the group and loses
  its parent between two reads of the table, and that also closes the
  attempt's pipes. On Linux the engine adopts such an orphan, and the
  attempt parks on it if it started a session of its own. One that left
  only its group, as shell job control does, is not seen. On macOS,
  where launchd adopts every orphan, neither is seen. On macOS a pid
  reused between the read and the signal is also a residual, because
  macOS has no pidfd.
