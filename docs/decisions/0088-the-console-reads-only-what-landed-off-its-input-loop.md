# 0088 — The console reads only what landed, off its input loop, and the fleet still verifies every body

Status: proposed
Date: 2026-10-18

Built: built

## Context

Issue #744: on the operator's journal (1.1 GB, 745,281 events, 95% of them
`effect/checkpointed`, 1,164 runs) `brokkr runs --json` took 6.7 s and
2.65 GB, and `brokkr tui` read the whole fleet that way every two seconds
on the loop that reads its keys, so a key waited seconds and an open
console idled at three quarters of a core.

**What was measured for it.** A journal of that shape was generated under
`.forge/` (734,484 events, 1,164 runs, checkpoint bodies near 1 KiB) and
each stage of today's fleet read timed on it in a release build, warm:
reading the envelope text 0.72 s, deserializing every envelope 1.27 s,
the chain's hashing 1.80 s, the fold 0.07 s, residual findings 0.005 s.
Reading only an envelope's type and time, its body skipped unparsed,
took 0.24 s where the whole parse took 1.27 s. Parse and verification are
the cost, not the fold.

Three ways were weighed for the fleet. Keep each run's fold and read only
what lands after it, which the store's suffix read (#354) already offers.
Read a checkpoint's type and time and leave its body unparsed. Add a type
column, or an index, to `events`, which is a schema change.

## Rulings

1. **The fleet is a kept read.** A console, and `brokkr runs`, keep each
   run's folded row between refreshes and ask the store only for what
   landed after the head that fold stands at. A refresh with nothing new
   reads no event; N new events are N read, verified as the continuation
   of exactly that head. Binding: `fleet::kept_tests`, which counts the
   events a refresh deserializes over a generated journal of 500,000
   (99.9% checkpoints): the first refresh reads 500,000, an idle one 0,
   one after three appends 3.
2. **What cannot be carried is read whole, never shown stale.** A gap, a
   head the journal no longer holds, or a journal swapped beneath the
   reader is read whole. A refusal is kept at its head only when the
   journal's own bytes made it (a broken chain, envelope text that is not
   JSON, a row before the chain); a refusal of timing (a peer's lock,
   SQLite's own error) is read again at the next refresh. Binding:
   `fleet::kept::lasting`, matched exhaustively over `StoreError`, and
   its kept test, which reads one of each kind made readable beneath an
   unmoved head.
3. **The selected run is kept too.** While its head stands and no rebuild
   is asked, the fleet's cadence derives the open run's view from the
   journal already read, and reads none of it. Binding: the
   `tui_read` read test, counting the events each frame reads.
4. **Input never waits for a read.** The console reads on a thread of its
   own, and its shell asks that thread at each poll without waiting: an
   answer that has arrived, or a typed `Pending`. A pending read counts
   neither as a clean read nor as a failed one, so the give-up bound
   counts reads. A frame read for a hearth, run or participant the shell
   has left is dropped and the next read forced; the frame drawn is never
   another hearth's. The reader thread is never joined: the console
   closes at once, and a run view's Ctrl+C is raised then. Binding: the
   reader tests (a key redrawn with the first read out, a slow failure
   giving up after five reads, a hearth entered with its read out, a
   console closed with its read out).
5. **The fleet still verifies every body.** The chain's hash covers each
   event's canonical content, its body included, so a checkpoint whose
   body is not parsed is a checkpoint whose link is not verified: a
   corrupt checkpoint would stop quarantining its run in the fleet while
   `inspect` and `export` refuse it, which is a weaker refusal. With
   rulings 1 and 3 the cost is paid once per console: the first kept read
   of the generated journal took 0.48 s at 81 MB peak, where the whole
   read took 3.48 s at 2.11 GB, and an idle refresh 17 ms. The cheap type
   read would save at most the 1.03 s between the whole parse and the type
   read, and the hashing besides only by not verifying, on the first read
   alone; a type column would save reading checkpoint text as well, at a
   schema change and the same loss of verification. Neither is built. This
   ruling is judgment, and the operator's to reverse.

## Consequences

- Decision 0014 stands: every read opens the journal read-only and writes
  nothing. A many-hearth world still opens each hearth read-only
  (decision 0026 ruling 5).
- The `events` schema is unchanged.
- A console holds each run's state and the transition and operator
  events its residual findings read, and the open run's events; it no
  longer holds every event of every run.
- The first read of a large journal spreads its whole reads across up to
  eight read-only connections.
