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
   *needs you* (parked, or quarantined because the journal does not fold),
   *running*, *last 24h* and *older*, in that order. A finished run is dated
   by when its journal last moved, its last event's `recorded_at`, so a run
   that waited days on a ruling and ended today is today's. A run whose date
   cannot be read is never hidden among the older ones.
2. **A title, not the prompt.** A row is called by its feature's first line
   that says anything, clamped at a word to 60 display columns (a wide
   character counts two) with an ellipsis. `RunRow::feature` stays whole, so
   `--json` stays lossless. The TUI strips the zero-width joiners that hold
   an emoji sequence at two columns, so it clamps its sanitized title again
   by the same function and paints what it measured.
3. **The verdict is two cells.** A row names the ruling that shipped, stopped
   or parked it, without its own phase's prefix and clamped to 14 columns,
   and, beside it, the worst open residual. A residual finding the operator
   superseded (decision 0047) is closed, and a finding valued `none` names no
   residual. The residual is typed: `brokkr_core::policy::Severity`, the
   closed set `SEVERITY_ORDER` names, which serializes as its name.
4. **A detail pane on wide terminals.** From 225 columns the fleet list keeps
   123 columns, a whole title wide, and the selected run stands beside it:
   its full id, how it stands, its verdict and findings, and its whole
   feature wrapped at 100 columns. The pane scrolls by the lines it draws.
   The shell measures the frame before it draws it, so the frame, its footer
   and the keys read one width. Below 225 columns the list keeps the frame
   and `Enter` opens the run as before. A list too narrow for every column
   and a 12-column title folds the age away, then the residual, and never
   cuts a cell, so from the TUI's 60-column minimum every row's verdict and
   whole id hash are drawn.
5. **The wire and the crates.** `RunEntry` and `RunRow` gain
   `last_recorded_at`; `RunRow` gains `title` and `verdict`. `VIEW_VERSION`
   moves 11 to 12, additively, by decision 0016's precedent. The pure
   `brokkr-view` crate's closed dependency set gains `unicode-width`, already
   in the lockfile through ratatui, with no effectful items. `brokkr-view`
   exports `sections`, `title`, `wrap`, `Section`, `Standing`, `Verdict`,
   `TITLE_COLUMNS` and `VERDICT_COLUMNS`; `brokkr-core` exports `Severity`.

## Consequences

- One derivation (decision 0013): the TUI paints these fields and derives
  none of them. `brokkr runs` and the web UI still print the feature, and
  adopting the title there is a follow-up.
- `a` is the only new key; it lists the older runs and is kept per hearth.
- The decision 0050 enactment slice rebuilds `policy.rs`'s loader beside
  `SEVERITY_ORDER`; `Severity` is additive there, and the loader may adopt it.
