# 0108 — A run is held by its driver, and a broken run is repaired by a record

Status: accepted — operator ruled in chat 2026-11-13, "go with all recommended" (#440)
Date: 2026-11-13

Built: unbuilt (#440)

## Context

#440 records two ways a run stops folding, `fold: event N: event after
terminal status`.

- **(A), 2026-09-26.** `brokkr conclude` closed a run whose live driver was
  mid-attempt. The driver's attempt settlement then landed after
  `run/stopped`. `conclude`'s fences (decision 0029) refuse only when the
  head moves during conclude's own writes. At an in-flight attempt it writes
  `effect/indeterminate` and `run/stopped` by design, and a seat working
  quietly moves no head. Nothing tells `conclude` whether a driver is alive.
- **(B), 2026-11-12, brokkr 0.12.0, in another workspace.** A spec run was
  concluded, then `brokkr operator … stop` was recorded for it. The command
  was journaled before it was judged (`operator/disposition.rs`), so its
  `operator/commanded` landed after the terminal event.

In (B) the damage then spread. `release` refused. `acknowledge` quarantined
the run, as documented (#511). But every later dispatch in that worktree
still refused with "run '…' holds this worktree, its writers not proven
settled" (`engine/broker/exclusion.rs`, `leased`). The settled-writers proof
reads the run, and the run no longer folds. The only way back found was to
archive the workspace's journal and re-provision the worktree.

**#440's first unit (440a) needs no decision.** It enforces a rule the fold
already has. `fold/relation.rs`' `ROWS` admits only operator annotations
after a terminal event (`Arm::Annotates`), and everything else is
`AfterTerminal`. So 440a makes the store refuse a non-annotation append to a
terminal run, makes the operator verbs judge a terminal run before they
write, and ends a late driver cleanly. That stops new corruption. Three
questions are left, and they are this decision's:

1. how `conclude` knows a driver is alive;
2. how a worktree held by a quarantined run is released;
3. how a run already broken folds again.

**The alternatives weighed:**
- **For liveness:** a per-run OS lock (taken); a heartbeat or lease in the
  journal (refused: it is clock-based and writes all the time); a scan of
  the process table (refused: it is not portable, and pids are reused).
- **For release:** proof from the broker's own records (taken); an explicit
  settle verb (taken, as the operator's route when proof cannot be read);
  acknowledgement alone (refused: an acknowledged run can still have live
  writers).
- **For repair:** an operator's record naming the late tail (taken); a
  blanket fold rule that ignores post-terminal events (refused: it weakens
  the fail-closed fold for every run, silently).

## Rulings

1. **A driving process holds its run by an OS lock.**
   - Every process that drives a run takes an exclusive, non-blocking `flock`
     on `<journal>.runs/<run-id>.lock` before its first append to the run,
     and holds it until it stops driving. That is `brokkr run`, `brokkr
     resume`, and a start from the queue.
   - **The directory** is this user's own, mode 0700. **The file** is this
     user's own regular file of one link, opened `O_NOFOLLOW | O_CLOEXEC`,
     as decision 0095's dispatcher lock is, so no seat, driver or child the
     engine spawns inherits it.
   - **The holder's facts:** the file records pid, start time, the store's
     host fingerprint and the engine's version, for the operator's eyes.
   - **Liveness is the lock.** The kernel drops it when the process dies.
     There is no heartbeat, no lease and no clock read for liveness.
   - **A second driver refuses,** naming the holder. A `resume` of a run
     another process drives is refused at once, so two drivers can no longer
     share one run.
   - This works the same on Linux and macOS (decision 0063).
   - **Enforcement binding:** a runtime test holds the lock in one handle
     and asserts that a second driver refuses, with its words. A test whose
     child process takes the lock and is killed shows the next driver taking
     it.

2. **`conclude` refuses a run its driver holds; `operator stop` does not.**
   - `conclude` tries the run's lock first. If it is held, `conclude`
     refuses before writing anything: "run '…' is driven by pid P on this
     host since T (engine V); stop it with `brokkr operator stop`, or end
     that process, then conclude".
   - If the lock is free, `conclude` holds it while it writes, so no driver
     can start beneath a conclusion.
   - **`operator stop` and `retry` stay lawful for a live run.** They are
     the cooperative path, and the driver honours a stop at the attempt
     boundary (decision 0029).
   - **After a terminal event,** every verb that would write a non-annotation
     refuses, as 440a builds.
   - A run journaled by a binary that predates this ruling has no lock file.
     Its lock is free, and the existing head fences still apply.
   - **Enforcement binding:** a runtime test drives a run in a child that
     holds the lock, and asserts that `conclude` refuses with its words and
     writes nothing. With the child killed, `conclude` closes the run.

3. **A quarantined run releases its worktree once its writers are proven
   settled, and the operator can prove it.**
   - The broker's exclusion (`leased`) no longer counts an acknowledged
     (quarantined) run as holding its worktree once both of these hold:
     - its run lock is free, or it has none;
     - every attempt the broker's own records name for it (reservations,
       standing attempts, their process groups) is proven ended by the
       broker's existing settlement proof.
   - **That proof reads the broker's records and the host, never the run's
     fold.** So a run that no longer folds can still be proven settled.
     That is the case #440's shape (B) blocked on.
   - Where the proof cannot be read, `brokkr operator settle --run <id>
     --reason <text>` re-checks the same facts now. If they hold, it journals
     an `operator/settled` annotation naming them. If not, it refuses, naming
     what still stands.
   - Acknowledgement alone never releases a worktree.
   - **Enforcement binding:** a broker test plants an acknowledged,
     unfoldable run with settled attempts and asserts that a new claim in the
     worktree is admitted. With one attempt's process group still alive, it
     asserts the claim is still refused.

4. **A run already broken is repaired by a record, never by an edit.**
   - `brokkr operator repair --run <id> --reason <text>` journals an
     `operator/repaired` annotation. It names the late tail: the first and
     last sequence numbers after the run's terminal event, and the tail's
     event types.
   - The fold reads a run's repair annotations before it applies events. An
     event inside a repaired range folds as **late**: it is kept and shown,
     with its type, by `inspect` and `brokkr runs`, and it is never applied.
     The run reads as its terminal status, with "a late tail of N events,
     repaired by <operator>".
   - **Only the tail as it stands can be named:** from the first event after
     the terminal one to the head at the time of the repair.
   - A run with an unrepaired late tail still fails to fold, as today. Repair
     is never automatic and never silent.
   - The hash chain is untouched. Nothing is edited or deleted.
   - `operator/repaired` and `operator/settled` are new event types, additive
     to the journal's event contract, and annotations on a terminal run in
     `ROWS`.
   - **Enforcement binding:** a fold test plants #440's two shapes, a
     settlement after `run/stopped` and an `operator/commanded` after it.
     Each fails to fold, and folds as `stopped` with its late tail once a
     repair names it. A repair naming a range that is not the late tail
     refuses.

## What this decision does not do

- It does not repair or settle anything automatically. Each is an
  operator's journaled act.
- It does not change decision 0095's dispatcher lock, which holds a journal
  for the dispatcher, not a run for its driver. A run the dispatcher starts
  is held by the engine that drives it, under ruling 1.
- It does not delete a run, an event or a journal. Re-provisioning a
  worktree stays the operator's choice, and an archived journal can be
  repaired later by ruling 4.
- It leaves #403's broader driver lifecycle (orphans, subreapers) to #403.

## Consequences

- `conclude` cannot close over a live driver, and a second driver cannot
  share a run. Both refuse with words that name the holder.
- A late write is refused (440a). A run already broken can be made to fold
  again by the operator, with its late tail kept as evidence. A worktree a
  broken run held is released once its writers are proven ended, so one
  corrupt run no longer blocks every run after it.
- A journal on a filesystem where `flock` does not exclude, such as some
  network mounts, is outside ruling 1's guarantee, as decision 0095 notes
  for the dispatcher's lock.
- **The slices:**
  - **440a** (running): nothing but an annotation after terminal.
  - **440b:** rulings 1 and 2, the run lock and `conclude`'s refusal.
  - **440c:** rulings 3 and 4, settle and repair, with the two new
    annotations.
  440b and 440c each stack on 440a, and they touch different files.

## Evidence

- #440, and its 2026-11-13 comment for shape (B).
- The controller's read of main at fcd25d094 (#440 c-6850594092):
  `engine/operator.rs` `conclude`, `engine/fence.rs`,
  `engine/operator/disposition.rs` lines 42–43, `engine/broker/exclusion.rs`
  `leased`, and `brokkr-core/src/fold/relation.rs` `ROWS`.
