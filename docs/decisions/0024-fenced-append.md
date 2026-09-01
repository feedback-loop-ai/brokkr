# 0024 — The fenced append: a writer commits onto the head it folded

Status: proposed
Date: 2026-09-01

## Context

`Store::append_next` derives an envelope's identity — `seq`,
`previous_hash` — from the journal head *inside* its own transaction, and
an `INSERT OR IGNORE` on `(run_id, seq)` turns a genuine race for the
same seq into `AppendConflict`. That much is sound: two writers cannot
fork the chain.

What it does not do is check that the head it is appending onto is the
head the caller **folded**. Every writer above the store reads the
journal, folds it, decides from the resulting cursor, and then appends —
and between the fold and the append the journal may have moved. The
appended event is perfectly well-formed and perfectly chained. It is
simply an answer to a question about a state that no longer holds.

Against a run that another process is actively driving, this is durable:

- `Engine::resume` on a fresh process takes the no-live-driver branch,
  closes the in-flight attempt `effect/indeterminate`, and drives on —
  while the real driver is still holding that attempt.
- `conclude` (this slice) does the same without even the manifest gate
  to slow it down: `operator/commanded` and `operator/accepted` (which is
  sanctioned — `brokkr operator stop` is a live kill switch by design),
  then `effect/indeterminate` for an attempt genuinely in flight
  elsewhere, then `run/stopped`.

The live driver's subsequent `effect/succeeded` then lands *after* a
`run/stopped`, out of any position the fold admits, and the run's journal
becomes permanently unfoldable. That is worse than a wrong state: it is
the loss of the audit record, and the audit record is the only thing this
system claims is authoritative.

`conclude` did not introduce this class — it inherits it from `resume`,
and it is why the review that surfaced it ruled the residual low rather
than blocking. But `conclude` raises the stakes, because its whole
purpose is to be reached for on runs an operator *believes* are already
dead. A verb whose reason to exist is the mistaken-liveness case is the
wrong place to keep an unfenced write.

The primitive is already here and already used: `Store::head_hash` is
documented as "cheap identity for fencing and anchors," and
`apply_fenced_operator_command` fences the Looper producer bridge on
`(expected_seq, expected_hash)`, journaling a `stale_cursor` rejection
when the head has moved. What the bridge does at one narrow boundary,
nothing else does anywhere.

## Decision

1. **A write states the head it read.** `Store` gains a fenced append —
   `append_next` carrying an expected `(seq, hash)`, or a
   `append_next_fenced` beside it — that refuses inside the same
   transaction when the journal head is not the one the caller folded.
   The refusal is a distinct `StoreError`, not an `AppendConflict`: a
   losing race and a stale fold are different accidents and read
   differently in a log.

2. **Every control-plane writer uses it.** `Engine::append` and
   `conclude` both pass the head their most recent fold was taken from.
   Neither carries a caller-supplied cursor to do it — the fence is a
   hash, which is derived, so law 2 is untouched.

3. **A stale fold refuses; it never guesses.** The fenced writer does not
   re-fold and retry on its own. It surfaces the drift, because every
   case where the head moved under a control-plane writer is a case where
   two processes believe they are driving one run, and picking a winner
   automatically is exactly the judgment the second law forbids.
   `conclude` in particular must say *this run is being driven by someone
   else* rather than close it.

4. **The fence, not a lease.** The rejected alternative is a driver
   heartbeat that `conclude` and `resume` consult for liveness. It closes
   strictly more — it would refuse *before* the operator events, not
   between them — but it puts authoritative mutable state outside the
   journal, invents an expiry policy, and makes a crashed driver's run
   unclosable until its lease lapses, which is the exact stranding this
   slice exists to end. An optimistic fence adds no state, no clock, and
   no new failure mode; a run whose driver is genuinely dead concludes on
   the first try, and a run whose driver is alive refuses.

5. **Not this slice.** `conclude` ships fenceless and inherits the class,
   because retrofitting the fence touches `Store`'s append signature and
   every `Engine` write path — the two verbs must move together or the
   unfenced one silently keeps the hazard for both. Landing it is its own
   slice against this ruling.

## Consequences

The window does not close entirely and this ruling should not pretend it
does: a driver that is alive but has not appended since the concluding
process folded still passes the fence. What the fence buys is that an
*active* driver — one appending at all — makes a concurrent `conclude` or
`resume` refuse instead of writing a conclusion over live work, and it
buys it without a clock or a lease. The residual case is a driver holding
a long-running effect in silence, which is also the case where a stop
riding to the attempt boundary is the operator's legitimate right.

Until this is accepted and implemented, `conclude`'s documentation says
plainly that it writes without checking for a live driver, and names
`brokkr runs` as the way to look before closing. A hazard an operator can
read is a smaller hazard than one only the reviewer knows about.
