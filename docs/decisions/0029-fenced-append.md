# 0029 — The fenced append: a writer commits onto the head it folded

Status: accepted — operator ruled 2026-09-01
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

## Erratum — 2026-09-01, at the landing bench

Renumbered from the run's draft 0024: that number is reserved for the
OSS-core scope decision, and three parallel fires claiming consecutive
numbers mid-burn is the collision the decision record cannot tolerate
(the wave's recorded pattern; a reservation step is queued).

Scope moved while this run was parked. The concurrent-writers slice
landed the primitive this decision asks for (`append_next_if_head`,
compare-and-append) and fenced the operator verbs; under the operator's
park ruling (2026-09-01, resuming this run's REVIEW-REFORGE-EXHAUSTED-
UNFIXED park), `conclude` was fenced at the landing bench with that
primitive — its stop command's refusal ends the conclusion, and both
closing appends land only on the head just folded. What remains for
this decision to rule is the tail: `resume`'s fresh-process branch, and
whether every remaining control-plane `append_next` moves behind the
fence. The identity residual (a self-asserted `$USER`) is decision
0025's, not this one's.


## Addendum — 2026-09-27, a retry before any phase is refused by name (#344), operator ruling

#344 moved the operator verbs into one module and gave the acceptance
rule one home, `brokkr_core::fold::acceptance_refusal`. Both doors, the
CLI's `brokkr operator` and the bridge's fenced command, and `fold`
itself now call it, where the engine used to keep a hand-written mirror
of `fold`'s rule. Making it one rule exposed a state the mirror had
wrong. A run can park before it enters any phase:
`Engine::enter_phase` appends `run/parked` (`SELECT-NO-DEFAULT`) before
any `phase/entered`, and `fold` admits that park. A `retry` there used
to be journaled as `operator/accepted`, which `fold` then refused as
out of place, so the journal stopped folding for good.

Ruled 2026-09-27: such a retry is refused with the refusal word
`no_phase_to_retry` (`Refusal::NoPhaseToRetry`). Both doors journal the
command as `operator/commanded` and then refuse it as
`operator/rejected` with that reason, so the journal keeps what the
operator asked; only the acceptance is never written. A `stop` is still
accepted in that state. The word joins the refusal vocabulary as a new,
additive `operator/rejected` reason. `contracts/` leaves that reason an
open string, so no frozen byte moves. Every other refusal word and
payload is unchanged. The operator ruled the typed `OperatorCommand`,
`Refusal` and `acceptance_refusal` into brokkr-core's public surface,
and `CommandWord` and `FencedCommand` into brokkr-runtime's, in the
same ruling. The fence's `Head` stays private to brokkr-runtime's
`engine/operator.rs`.

## Addendum — 2026-09-28, proposed: a peer's lock in flight settles the attempt within a bound (#394)

Status: proposed; only the operator accepts this addendum.

Ruling 3 says a stale fold refuses and is never retried. A peer that only
holds the journal's write lock is a different accident:
`StoreError::Contended` wrote nothing, so the same append made later is
the same call. #394 found that such a lock, held past the store's one
patience while a seat was working, ended the engine and threw the
attempt away. The wait measured was 42 s, against a 30 s patience. The
fix changes what the journal shows, and this addendum states the rule:

1. A working seat's checkpoints that meet the lock are held in order and
   retried each time the seat hands over another. No append waits on
   the lock while the seat works, so the thread reading the seat's pipe
   never stalls and the seat is never stopped. The hold is bounded at 16 MiB of serialized checkpoints
   (`HELD_BYTES`), plus the one row that finds it empty.
2. When the seat stops, held checkpoints get three settling patiences
   (`SETTLING_PATIENCES`). A terminal event gets three, and if the lock
   outlasts them the engine holds that outcome and tries it three more
   times in the lawful end. Each marker the engine journals for a panel
   member or a sequence step gets three of its own. Every other event
   gets one.
3. Evidence lost to a full hold, or stranded when the lock outlasts the
   settlement, settles its site `effect/indeterminate` with the count
   named. It is never read as the driver's success.
4. A stopped attempt with no outcome held is settled at `EffectInFlight`
   by `effect/indeterminate` naming the lock, then `run/parked` names it
   too, so the next resume finds nothing open.
5. Past the bound nothing is writable. The engine hands the typed
   contention back with the attempt open, and the next `resume` settles
   it as restarted and parks.

The operator ruled on 2026-09-28 that this bound is accepted as designed.
Its acceptance criterion "never an unsettled attempt" cannot be met while
nothing is writable, and was ruled out of scope. A spill file and an
attempt-wide bound are #433's; the remaining refinements are #464's. No
contract, event type or frozen byte moves.
