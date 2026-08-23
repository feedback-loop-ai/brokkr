# 0006 — Bounded automated attempts and seat deadlines

**Status**: accepted (operator goal directive, 2026-08-23 — "fully
autonomous, end-to-end"; enumeration by Claude under that directive)

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
