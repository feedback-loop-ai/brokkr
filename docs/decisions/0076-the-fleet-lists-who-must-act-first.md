# 0076 — The fleet lists who must act first

Status: proposed
Date: 2026-09-30

Built: built

## Context

On 2026-09-29 the fleet view of `brokkr tui` printed each run's whole
commission across about 600 rows of a 4K terminal, cut every run id inside
its hash, and listed 602 runs newest first with an event count and a phase
that read `done` for all of them (#491). The operator asked for a screen that
shows what to act on first, and the detail only for the run selected.

The fix grows the view model's wire, the pure view crate's dependencies, and
brokkr-core's typed vocabulary. Each is a semantic change the operator rules
on, so this file names them.

## Decision

1. **Sections by who must act.** `brokkr-view` deals the fleet's rows into
   *needs you* (parked, quarantined because the journal does not fold, or
   stale), *running*, *last 24h* and *older*, in that order. A finished run
   is dated by when its journal last moved, its last event's `recorded_at`,
   so a run that waited days on a ruling and ended today is today's. A run
   whose date cannot be read is never hidden among the older ones.
2. **A title, not the prompt.** A row is called by its feature's first line
   that says anything, clamped at a word to 60 display columns (a wide
   character counts two) with an ellipsis. `RunRow::feature` stays whole, so
   `--json` stays lossless. The TUI strips the zero-width joiners that hold
   an emoji sequence at two columns, so it clamps its sanitized title again
   by the same function and paints what it measured. A list with no detail
   pane beside it gives its titles the width it has (`title_within`), and
   its title column is no wider than its widest line, so the age and the id
   stand beside the titles (#503).
3. **Stale is a need, by a stated bound (#503).** A run whose journal folds
   to running and has not moved for longer than three hours is *stale*: it
   is listed under *needs you*, never as running, and the brand mark does
   not pulse for it. The journal records no attempt's deadline, so the bound
   is the longest deadline any recipe, agent or bundle in the repository
   gives an attempt (two hours, held to them by a test) and an hour's margin,
   measured from the run's last event. A clock or a last event whose time
   does not read never makes a run stale. `need(row, now)` derives what a
   run in *needs you* asks — parked, quarantined or stale — with the line a
   row prints under its title (`quarantined: conclude or inspect`, `stale:
   no event since <age>; resume or conclude`) and the detail's way out:
   `brokkr conclude` where the journal folds, or `brokkr export` and
   inspect, for a quarantined run; `brokkr resume` under the run's pinned
   bundle, or `brokkr conclude`, for a stale one. A stale run still folds
   to running, and the fold admits `brokkr operator retry` only on a parked
   run, so the way out does not name it, though #503 asked for it. A test
   parses every command a way out names with the CLI's own parser and
   checks the engine admits it on a run standing as that one does.
4. **The verdict is two cells.** A row names the ruling that shipped, stopped
   or parked it, without its own phase's prefix and clamped to 14 columns,
   and, beside it, the worst open residual. A residual finding the operator
   superseded (decision 0047) is closed, and a finding valued `none` names no
   residual. The residual is typed: `brokkr_core::policy::Severity`, the
   closed set `SEVERITY_ORDER` names, which serializes as its name.
5. **A detail pane on wide terminals.** From 225 columns the fleet list keeps
   123 columns, a whole title wide, and the selected run stands beside it:
   its full id, how it stands, its verdict and findings, and its whole
   feature wrapped at 100 columns. The pane scrolls by the lines it draws.
   The shell measures the frame before it draws it, so the frame, its footer
   and the keys read one width. The fleet opens on its first row — the first
   run that needs you, or else the first running — so a wide frame draws the
   pane before any key is pressed (#503); leaving a run lands back on it.
   Below 225 columns the list keeps
   the frame and `Enter` opens the run as before. A list too narrow for every
   column and a 12-column title folds the age away, then the residual, and
   never cuts a cell, so from the TUI's 60-column minimum every row's verdict
   and whole id hash are drawn.
6. **Every key is found where it acts (#503).** The fleet's footer names
   `Enter`, `Tab`, `a` and `/` at every width, dropping lesser keys to fit;
   below 225 columns it says `Tab detail ≥225`. The older runs' line reads
   `<n> older runs: press a to show`, and their heading, once shown, says
   `a folds them`. *Older* is the one section that folds, so its heading
   alone names a key: *needs you*, *running* and *last 24h* are always
   open, and a fold mark on them would look expandable and expand
   nothing. The operator rules whether those three should fold too. `/`
   matches a run's id and its whole title line, of which the list paints
   as much as its column holds. A running run wears `◐`, never a disclosure triangle;
   `→`, `l` or Space open it in place to its phase, seat, attempt, elapsed
   time and last event, `←` or `h` fold it, and `Enter` still opens the run.
7. **The wire and the crates.** `RunEntry` and `RunRow` gain
   `last_recorded_at`; `RunRow` gains `title` and `verdict`. `VIEW_VERSION`
   moves 11 to 12, additively, by decision 0016's precedent. The pure
   `brokkr-view` crate's closed dependency set gains `unicode-width`, already
   in the lockfile through ratatui, with no effectful items. `brokkr-view`
   exports `sections`, `title`, `wrap`, `Section`, `Standing`, `Verdict`,
   `TITLE_COLUMNS` and `VERDICT_COLUMNS`; `brokkr-core` exports `Severity`.
   #503 adds `RunRow::hire` (`Hire`: the seat the fold's cursor names for the
   current effect, and the attempt) and moves `VIEW_VERSION` 12 to 13,
   additively; `RunRow` moves from `lib.rs` to the `fleet` module, its path
   unchanged; `brokkr-view` exports `need`, `Need` (with
   `Need::commands`, the commands a way out names), `Hire` and
   `title_within`.

8. **Three columns, the whole width (#503, second round).** The detail
   pane of item 5 gives way to two columns beside the list, which share
   the whole frame, about 35/35/30 for three and 45/55 for two, the list
   never narrower than its 123 columns, and each wraps its text at its own
   width rather than at a fixed 100. The *run dashboard*, `d`, from 195
   columns, holds how the run stands, why it ended or what it needs (the
   last ruling's rule, severity and reason, the last seat's result and its
   notes' first lines, which `Enter` there reads whole), its path visit by
   visit with each ruling, residual, duration and model, its seats, the
   way out `need()` names, and its commission folded to three lines, which
   `c` opens and folds. The *live or findings* column, `f`, from 257
   columns beside the dashboard or 185 without it, streams newest first
   the checkpoints of the seat the fold names at work on a running run,
   and shows a finished run's last review's findings, or else its last
   seat's notes, whole. Both are on by default and kept per hearth; a
   column the width cannot hold is not drawn, and the footer names its key
   and the width it needs. `Tab` cycles the columns drawn and the list
   keys scroll the focused one. The list draws its older runs until it is
   full and folds only the rest (`<n> more older runs: press a`). The
   shell reads the selected run's view through the run level's own
   question, and `brokkr-view` derives the path, the last ruling and the
   last seats' notes once, as `RunView::dashboard`, with
   `working_checkpoints` and `seat_summary` beside it; `VIEW_VERSION`
   moves 13 to 14, additively.

## Consequences

- One derivation (decision 0013): the TUI paints these fields and derives
  none of them. `brokkr runs` and the web UI still print the feature, and
  adopting the title there is a follow-up.
- `a` lists the older runs and is kept per hearth, as are the runs opened in
  place. `→`, `l`, Space, `←` and `h` are bound at the fleet only.
- The staleness bound holds only for seats this repository ships: a recipe
  outside it that grants an attempt more than two hours reads stale while
  that attempt is still silent: a false alarm, never a run hidden as live.
  Journaling each attempt's deadline (a new contract version) would make
  the bound exact; the operator rules whether the fixed bound stands until
  then.
- The decision 0050 enactment slice rebuilds `policy.rs`'s loader beside
  `SEVERITY_ORDER`; `Severity` is additive there, and the loader may adopt it.
