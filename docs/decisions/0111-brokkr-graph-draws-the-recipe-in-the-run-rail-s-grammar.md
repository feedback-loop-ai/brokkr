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
That geometry had limits: one row of arcs, arcs between states on one rail
only, and nothing below the row. The drawing extends that shared geometry
rather than copying it, and the run rail draws as it did. Each figure the
rail still cannot place is listed in the rail's glyphs, never dropped.

## The operator's rulings

The operator settled this decision's asks for #1145:

- **Re-export** (2026-11-18, "yes, rule the brokkr-view re-export for
  #1145"): `brokkr-view` re-exports `Place`, `State`, `Link` and `RuleKind`
  from its graph module, and `quality/public-api/brokkr-view.txt` grows by
  exactly those four. The drawing matches them as types, exhaustively. No
  other type is exported under this ruling. The view model,
  `GRAPH_VIEW_VERSION` and `--json` do not move.
- **Back arcs** (2026-11-19, "Draw all of it now"): every taken back rule is
  drawn as a return arc: nested arcs, arcs whose states a fold puts on two
  rails, and the road down to the terminals beneath the row. The listing
  `next ᐸ── from  ×n` stays only as the never-drop guard.
- **Narrow terminals** (2026-11-19, "Fit any width"): a known positive
  terminal width is fitted, whatever it is, down to one state per rail, with
  every line under the rails wrapped to it. Nothing is drawn wider than the
  width it was given. A size of 0 or one that cannot be read draws at the
  piped 100 columns.

## Rulings

1. **One renderer.** The drawing lays the recipe's states on `plan` and
   paints them with `paint_cells`, the painter the run rail uses
   (`tui/recipe.rs`). The view's counts are then written over those cells.
   Enforcement: the TUI's snapshots stand byte-identical, and
   `tui/source_tests.rs` reads `recipe.rs` into the TUI's source checks.
2. **Arrows are taken links.** A rule's arrow is drawn between two
   neighbours only when the range took a link between them, with the link's
   count above its head. Two neighbours that no taken link joins stand apart
   with no road between them. A self link is `↻n` above its node. A link
   across a fold runs out to the one rail's `›` and in from the next one's `‹`
   to its arrow, with its count above. Rules that never fired are listed, not
   drawn. The drawing matches the view's `Place` and `RuleKind` as types,
   exhaustively. Enforcement: the `graph_no_runs`, `graph_parked` and
   `graph_long_80` snapshots.
3. **Return arcs.** Every taken back link is a return arc with `×n`, under
   the names where the run rail draws its arcs. `layout.rs`'s `nest` deals
   each arc the shallowest arc row where it keeps a blank column from every
   other arc, the narrowest first. A nested or crossing arc rides a further
   row as deep as it needs, and has no limit. Its ends rise to the first arc
   row, as `│`, or as a junction (`┼ ├ ┤`) where they pass a road. A back
   link whose states a fold put on two rails is drawn in two halves: it
   lands on its earlier rail from that rail's `›` (`╰ᐸ── ×n ──›`), and leaves
   its later rail toward that rail's `‹` (`‹── ×n ──╯`). Two halves of one
   width are dealt rows in the order of their landings, so the halves pair
   by row as well as by count. `plan` lays only whole arcs on the first row,
   so the run rail draws as before. Only a count that no stretch of its arc
   has room for leaves its link listed as `next ᐸ── from  ×n`, under the
   never-drop guard. Enforcement: the `graph_nested`, `graph_delivery` and
   `graph_wide_80` snapshots, the TUI's snapshots standing byte-identical,
   and every drawing test asserts each taken link is drawn or listed with
   its count, never both and never neither.
4. **Nodes and terminals.** Every state is a node labelled with its runs
   entered and its median stay. A terminal's name is marked `⊗`. A live run
   lights its state's node. The row's states are followed by the state that
   ends the row. The terminals beneath the row are drawn as nodes on a rail
   under it. A taken rule into one of them is a road down from the state it
   leaves, `╰─ n ─ᐳ ⊗ stop`, on an arc row under every return arc. A park is
   listed as `from ⊙ park n · id`. Enforcement: the `graph_parked` snapshot.
5. **Counts apart.** Every count over the rail keeps a blank column from
   every other on its row. A self loop's `↻n` that would meet a forward
   count moves to a second counts row. A count with no room on either row
   leaves its link listed whole. Enforcement: the `graph_counts_apart`
   snapshot (entered `1 · 0m`, self 10000, forward 10000).
6. **Width.** The drawing takes the terminal's width whenever it reads as a
   positive number, however narrow. A pty that reports 0, an unreadable size
   and a pipe draw plain text at 100 columns (`PIPED_WIDTH`) with no colour.
   A row that does not fit folds whole states onto the next rail, marked `›`
   and `‹`, down to one state per rail. A name wider than its rail is clamped
   with `…`, as the run rail clamps it, and so is a node's label where the
   width leaves it no room. The lines that follow the drawing carry both
   whole. Under 12 columns (`RAIL_LEAST`), where a node's name would clamp to
   nothing, a rail's states are written as lines instead. Every line under
   the rails, the legend included, breaks before a word that would pass the
   width, with each further line indented three columns. A word wider than a
   line is continued: the piece that fits ends `…`, and the next line starts
   `…`. Nothing is drawn wider than the width. Enforcement: every drawing
   test holds every line within its width, at 100, 80, 39, 30, 24, 12 and 8
   columns (`graph_delivery_24`, `graph_delivery_12`, `graph_name_wider_30`,
   `graph_no_runs_8`), and `drawing_width`'s test pins 39, 12 and 1 as
   themselves and 0 as 100.
7. **What does not compile.** A recipe that does not compile and that no run
   carries has no state to draw. Its refusal is printed as before, with no
   drawing. A refused recipe whose runs carry states is drawn from those
   states. Enforcement:
   `a_recipe_that_does_not_compile_is_drawn_with_its_refusal_and_named_among_the_realms`.

## Asks

The operator's rulings above settle the asks this decision first carried:
the re-export, and the geometry for arcs across folds, nested arcs and roads
to the terminals beneath the row. What stays open is the decision itself,
which is the operator's to accept.

## Consequences

`brokkr graph` without `--json` prints the drawing, then the header and lines
it printed before. `--json`, the view model and the TUI's drawing do not
change.
