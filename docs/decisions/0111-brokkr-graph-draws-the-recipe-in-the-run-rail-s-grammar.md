# 0111 — `brokkr graph` draws the recipe in the run rail's grammar, at the terminal's width, and lists what the rail cannot draw

Status: proposed
Date: 2026-11-19

Built: built

## Context

Issue #1145: the operator expected `brokkr graph` to show the recipe's
delivery graph in the terminal, and it printed a list. The operator ruled on
2026-11-18 that the drawing is in the terminal and left the rest open to the
pull request. Decision 0014 holds (the verb is read-only), as does 0071 ruling
5 (one renderer, not a copy) and ruling 7 (the figures come from
`brokkr_view::GraphView` and are derived nowhere else). `--json` and
`GRAPH_VIEW_VERSION` do not move.

The run rail in `brokkr tui` already draws phases as nodes, arrows between
them and return arcs on one reserved row (`tui/layout.rs`, `tui/painter.rs`).
That geometry has limits: one row of arcs, arcs between states on one rail
only, and nothing below the row. The rulings below record what the drawing does
inside those limits. Each figure the rail cannot place is listed in the rail's
glyphs, never dropped.

## Rulings

1. **One renderer.** The drawing lays the recipe's states on `plan` and
   paints them with `paint_cells`, the painter the run rail uses
   (`tui/recipe.rs`). The view's counts are then written over those cells.
   Enforcement: the TUI's snapshots stand byte-identical, and
   `tui/source_tests.rs` reads `recipe.rs` into the TUI's source checks.
2. **Arrows are taken links.** A rule's arrow is drawn between two
   neighbours only when the range took a link between them, with the link's
   count above its head. Two neighbours that no taken link joins stand apart
   with no road between them. A self link is `↻n` above its node. Rules that
   never fired are listed, not drawn. Enforcement: the `graph_no_runs` and
   `graph_parked` snapshots.
3. **Return arcs.** A taken back link is drawn as a return arc with `×n`,
   under the names where the run rail draws its arcs, when both its states are
   on one rail and its span shares no column with an arc kept before it (in
   the view's link order). An overlapping arc would merge into its neighbour
   on the one arc row, and its count would land on the other's, so it is
   listed as `next ᐸ── from  ×n`. So is a back link whose states a fold put
   on two rails. Enforcement: the `graph_nested` and `graph_wide_80`
   snapshots.
4. **Nodes and terminals.** Every state is a node labelled with its runs
   entered and its median stay. A terminal's name is marked `⊗`. A live run
   lights its state's node. The row's states are followed by the state that
   ends the row. The terminals beneath the row are drawn as nodes on a rail
   under it, and the rules into them are listed (`from ──ᐳ ⊗ stop  n`), since
   the rail grammar has no vertical road. A park is listed as `from ⊙ park
   n · id`. Enforcement: the `graph_parked` snapshot.
5. **Width.** The drawing takes the terminal's width when stdout is a
   terminal that reports at least 40 columns (`NARROWEST`). Otherwise,
   including a pty that reports 0, it draws plain text at 100 columns
   (`PIPED_WIDTH`) with no colour. A row that does not fit folds whole states
   onto the next rail, marked `›` and `‹`. Every line under the rails breaks
   before a word that would pass the width, with each further line indented
   three columns. A word wider than a line is continued: the piece that fits
   ends `…`, and the next line starts `…`. Only a single state too wide for
   a rail by itself has its name clamped with `…`, as the run rail clamps it;
   the lines that follow the drawing carry the whole name. Enforcement: the
   `graph_wide_80` and `graph_long_80` tests hold every line within its
   width, and `drawing_width`'s test pins the two widths.
6. **What does not compile.** A recipe that does not compile and that no run
   carries has no state to draw. Its refusal is printed as before, with no
   drawing. A refused recipe whose runs carry states is drawn from those
   states. Enforcement:
   `a_recipe_that_does_not_compile_is_drawn_with_its_refusal_and_named_among_the_realms`.

## Asks

- `brokkr_view` does not export the view's `Place` or `RuleKind`, so the
  drawing reads both from their wire words into enums of its own
  (`tui/recipe.rs`'s `Placed` and `Moved`). A word it does not know is
  refused with `DrawError::Unread`, never defaulted. Re-exporting the two
  types would let it match them directly, but that grows
  `quality/public-api/brokkr-view.txt`, which is the operator's to rule.
- Drawing a back arc across a fold, an arc nested inside another (a second
  arc row), and a road from the row down to a terminal beneath it each needs
  geometry the run rail does not have. Each is listed today. Building any of
  them is a follow-up for the operator to commission.

## Consequences

`brokkr graph` without `--json` prints the drawing, then the header and lines
it printed before. `--json`, the view model and the TUI's drawing do not
change.
