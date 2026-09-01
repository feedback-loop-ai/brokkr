# Addendum: `tui-graph` — the return edge

**Feature slug**: `tui-graph` (addendum to [spec.md](spec.md))
**Run**: `tui-graph-the-return-edge-a-refo-ddd75534`
**Status**: Implemented
**Scope**: display refinement **within** decision 0014, rendering what
decision 0022 (`docs/decisions/0022-reforging.md`) already records.
**No new decision doc** — see "Which side of the line" below.

## Why

Decision 0022 gave the machine a real backward transition: a review that
finds a security residual sends the run BACK to implement (`REVIEW-REFORGE`
in `bundles/self/policy.json`), bounded at two reforgings. The rail drew
the revisit — the `×N` marker on the target phase's name — but not the
transition, so an operator watching the first live reforging saw
`review` finish and `implement` light up again with nothing between
them. That reads as teleportation, not as a loop. A reforging is a road,
and roads are drawn.

## The ruling

When the journal records a backward transition **between two phases the
rail has already drawn both ends of**, the graph draws one **return arc**
under the name baseline: from beneath the departing phase back to
beneath the landing one, solid line, the mirror arrowhead `ᐸ` at the
landing end.

1. **Journal-true.** One arc per distinct `(from, to)` pair the journal
   actually recorded. A run that never reforged draws nothing. Repeats
   ride the existing `×N` marker — never a second, stacked arc.
2. **Solid only.** The vocabulary is `╰ ─ ╯` from the same rounded set
   `selection_box` uses, plus `ᐸ` (U+1438), the sibling of the rail's
   operator-calibrated `ᐳ`. Never `╌` or `┆`: the dashed lesson of
   2026-08-31 is that those glyphs carry a gap at every cell boundary by
   design and can never touch a corner. Applied here from the start
   rather than after.
3. **Asymmetric head.** Only the landing wears one, matching the rail's
   own `ᐳ`, which marks arrival and never departure.
4. **Yields whole.** The arc has one reserved row and no half-measures.

## Where the fact comes from

> A renderer may branch on a model field; it may not compute one.

"Was this transition backward" was not a fact the models carried.
`Phase` gains **`returns: Vec<String>`** — the phases whose ruling sent
the run back into this one, deduped, in journal order — derived in
`phase_rail` from the `transition/decided` that CAUSED the revisit: a
transition whose `next` has already been entered. Not from `visits > 1`,
which is necessary and not sufficient: the count says a phase was
entered twice, and only the transition says where from. `tui.rs`
branches on the field and computes nothing.

`VIEW_VERSION` moves **2 → 3**. Decision 0016's precedent is that an
additive model field moves the wire version; a consumer pinning the old
one is entitled to know the shape grew.

## Geometry

Plan-level, like `Plan.rail` and `Plan.edges` and unlike a per-segment
`Join`: an arc spans from one `Seg`'s rail to another's with whole
phases possibly between them.

| element | rule |
| --- | --- |
| `Plan.arc_row` | the pane's LAST row, below `box_row`. Reserved only when `returns_of(phases)` is non-empty **and** `height >= 5` |
| `Plan.box_row` | unchanged in meaning; moves up one row when an arc row exists |
| `Plan.arcs` | `Arc { to, from }`, absolute columns, `to < from` |
| an end's column | `centre_of(seg)` — the centre of that phase's rail content, so the road's end sits under the phase's own node |
| drawn | `╰` at `to`, `ᐸ` at `to + 1`, `─` between, `╯` at `from` |

Two segments' rails are disjoint and one connector apart, so the two
ends are never the same column and never closer than the head plus its
corner: no branch is needed to keep the corner off the head.

**The collisions, answered in `plan` and never in `paint`:**

- *the selection box* — the arc's row is below `box_row` by
  construction, so a wall can never cross it.
- *the elision marks `‹`/`›`* — they live on the rail row, and a pair
  with an end outside the drawn window is dropped in `plan`. The ROW
  stays reserved, so walking the rail never lifts the baseline under the
  operator's eye.
- *a pane too short* — no row, no arc, and the rest of the layout is
  exactly what a run with no road would have had. Half an arc is the
  box's own "half a box" ruling.

`paint` keeps no clipping branch.

**Two pairs are dropped, both for want of geometry and neither as a
judgement about the journal**: a departure naming no phase on this rail
has no column to leave from, and a landing sitting LATER on the rail
than its departure is not a road drawn leftward — the head has one
direction, and `ᐳ` is not the arc's to borrow. The second is reachable
(`A → B → A`, then `A → B`); it draws no arc and the revisit still
reads on the `×N` marker.

## The fixture

`fixtures/journals/reforging-the-road-back-hand-built.ndjson` — 80
events: `REVIEW-REFORGE` twice, then `REVIEW-REFORGE-EXHAUSTED-DEBT` to
`ship`, ending `done`. It is **hand-built**, and its name says so:
when the arc needed a backward transition to draw, no run in this
repository had taken one (decision 0022 landed the same day; every
`refs/forge/*` and the local `.forge/forge.db` were checked). It is
built to the documented shape in `bundles/self/policy.json`, and it is a
real journal — it chains, verifies and folds, pinned by
`the_reforging_fixture_chains_verifies_and_folds_to_a_completed_run`.
Trim a real reforged export over it the day one exists.

## Tests

| claim | test |
| --- | --- |
| arc drawn for a journal with the reforge transitions | `a_recorded_reforging_draws_one_return_arc_under_the_names` |
| absent without | `a_run_that_never_reforged_draws_no_arc_at_all` (on the committed linear fixture) |
| single arc under `×N` repeats | same as the first: two reforgings, `visits == 3`, one arc |
| omission on a short pane | `a_pane_too_short_for_the_arc_row_draws_no_half_arc` |
| no glyph outside the solid vocabulary | `the_return_arc_names_only_the_solid_vocabulary` (over `tui.rs`'s own source) |
| an end scrolled out of the window | `an_arc_with_an_end_outside_the_window_is_not_drawn` |
| both dropped pairs | `a_road_needs_two_ends_on_the_rail_and_a_landing_that_lies_left` |
| the derivation | `the_rail_records_the_road_back_once_per_pair_and_only_where_it_was_taken` |
| the fixture is a real journal | `the_reforging_fixture_chains_verifies_and_folds_to_a_completed_run` |

## Which side of the line

Display refinement within decision 0014, not a new ruling: the parent
`tui-graph` feature set that precedent for this class of change, and
this one renders a transition the machine already records rather than
changing what the machine does. The one thing that is not pure
rendering — the new `Phase.returns` field and its `VIEW_VERSION` move —
is stated here and in the commit rather than left implicit.

## Non-goals held

No `ui.html` change (the console keeps its own answer). No contract,
policy table, recipe or bundle change. No stacked arcs. No dashed
glyphs. No clipping branch in `paint`. `render::graph_block` and every
existing golden are byte-identical.
