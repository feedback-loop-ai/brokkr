# 0097 — A stop accepted with no ending written stands as stopping, not running

Status: proposed
Date: 2026-10-29

Built: built

## Context

Issue #909. `brokkr operator stop` journals `operator/commanded` and
`operator/accepted`; only the run's driver writes `run/stopped`. When the
driver is gone nothing writes that ending, and the fold still says
`Running`: `brokkr runs` printed `running`, `inspect` printed `running ·
phase …`, and the consoles counted the run among the running. Five runs in
one operator journal read as running for about twenty days.

The fold already records the accepted stop. With an effect in flight the
stop rides (`Cursor::EffectInFlight { stop_rides: true, .. }`), and
anywhere else the cursor becomes `Cursor::Stop`. Either way the status
stays `Running` until an ending is written, so "a stop accepted, no ending
written" is a pure read of the fold, with no liveness guess.

The readouts read `run_summary` rows (decision 0089), so the standing
needs a row column for a paged read to print what a whole read prints.
The operator's command and acceptance are already journal events the
trail keeps; the trail printed only the event's type tail, not the
command or its seq.

## Rulings

1. **The standing is `stopping`.** A run whose fold is `Status::Running`
   with `Cursor::Stop` or with `stop_rides` set is `Standing::Stopping`,
   not `Running`, on every surface. Its row's detail is one formatting
   function: `stop accepted at seq N; no ending written. If no driver is
   running it, `brokkr conclude` closes it.` N is the seq of the
   `operator/accepted` event, recorded by the fold as
   `RunState::stop_accepted_seq`. `inspect`'s status line prints
   `stopping` and the same detail.
2. **A stopping run needs the operator, and is never counted running.**
   It lists in the fleet's `needs you` section and in no running count,
   in `brokkr runs`, the fleet's sections, the TUI's counts, the muninn
   dossier and the dsh console. A live driver normally writes the ending
   within seconds.
3. **The summary row carries it, additively.** `run_summary` gains a
   nullable `stop_accepted_seq INTEGER`, set by the writer from the fold
   and refolded by the repair and backfill; the row's generated `lane`
   reads `stopping` while the run is running and the column is present,
   so `needs you` pages it and the running lane does not. A summary made
   before the column is remade and refolded at the next writable open, as
   decision 0089's aggregate columns already are. `DATABASE_SCHEMA` does
   not move.
4. **The trail shows the operator's command and its disposition.** An
   `operator/commanded` row reads `<command> · commanded at seq N`; an
   `operator/accepted` row reads `<command> · accepted at seq N (command
   at seq M)`; an `operator/rejected` row reads `<command> · rejected at
   seq N: <reason> (command at seq M)`, resolving the command by its id
   from the `operator/commanded` that stood when it answered, so a later
   command reusing an id never re-labels an earlier disposition. Every
   other trail row's bytes are unchanged.
5. **The wire moves once.** `VIEW_VERSION` moves 27 to 28: a row's
   `verdict.standing` may now be `stopping`, a verdict carries its
   `detail`, and `inspect`'s summary carries `standing` and `detail`.
   The public API additions are named on the pull request.

Not this story: whether `stop` itself should write the ending when it can
prove no driver holds the run (decision 0029's fence, the U8e lease
work). That is a separate design question for the operator.
