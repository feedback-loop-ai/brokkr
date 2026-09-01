# Addendum: the console learns the road back

**Feature slug**: `tui-graph` (addendum to [return-arc.md](return-arc.md),
on the console side of [`one-derivation-two-surfaces`](../one-derivation-two-surfaces/spec.md))
**Run**: `the-browser-console-s-phase-grap-11843f5f`
**Status**: Implemented
**Scope**: display refinement, rendering on `brokkr ui` the fact
`brokkr-view` already derives and the TUI already draws.
**No new decision doc** — same side of the line as the arc it mirrors.

## Why

[return-arc.md](return-arc.md) held one non-goal deliberately: "no
`ui.html` change (the console keeps its own answer)". The console's
answer turned out to be silence. Operator finding of 2026-09-02:
*brokkr ui does not match brokkr tui in terms of the graph going back* —
the terminal rail drew the reforging as a road and the browser drew the
run as strictly forward, so the same journal read as a loop in one
surface and as teleportation in the other. Two surfaces over one
derivation are allowed to render differently; they are not allowed to
disagree about what happened.

## The ruling

Every rule of the arc carries over unchanged, because it is the same
ruling on a second surface:

1. **Journal-true.** One road per distinct `(from, to)` pair the model
   carries. Repeats ride the revisit marker the console already painted
   — never a stacked arc. The console spelled it `xN` where the terminal
   and `inspect` both spell it `×N`; review closed that gap, since the
   operator's finding asked for the counts *as the other surfaces show
   them*. The terminal's extra clamp at `VISITS_MAX` stays a terminal
   concern: it exists because a cell budget is finite, and an SVG text
   node's is not.
2. **Solid only.** In SVG the dashed-glyph lesson does not bite the same
   way, but its meaning does: a dashed road reads as one not taken, and
   this one was. `.loops .ret` strokes `var(--line)` at the rail's own
   width.
3. **Asymmetric head.** Only the landing wears one, mirroring the
   forward edges' `marker-end`, which marks arrival and never departure.
4. **Additive, or absent.** A forward-only run's graph is the graph it
   was: same SVG height, same nodes, same legend.

## Where the fact comes from

`Phase.returns` — derived once in `phase_rail`, already on the wire:
`/api/view/<id>` serves the whole `RunView`, so the field reached the
browser before this change and nothing in `brokkr-view` or `ui.rs`
moved. The page branches on it and computes no fact of its own
(decision 0013): `returnPairs(rail)` turns the names the model carries
into `[landing, departure]` rail INDICES and drops the two pairs that
have no geometry — a departure naming no phase on this rail, and a
landing at or after its departure — exactly as `tui.rs`'s `returns_of`
does. Backwardness is never inferred from `visits > 1`.

The reforging fixture exercises both drops: it carries three `returns`
entries (`implement ← review`, `verify ← implement`, `review ← verify`,
all three real backward transitions), and only the first lies leftward
on the rail, so one road is drawn.

## Geometry

| element | rule |
| --- | --- |
| `roads` | `returnPairs(view.phases)`, computed once per render |
| the rows | `retTopY = nameY + 18`, `retY = nameY + 30` — clear of the revisit markers at `nameY + 12` |
| the SVG's height | `retY + 8` when a road exists, `nameY + 24` when none does |
| the road | one `path.ret`: down from the departure's centre, rounded corner, across, rounded corner, up to the landing's centre |
| the head | one `path.ret-tip`, a triangle inside the landing corner pointing into it |
| paint order | last, so a road passes UNDER the selection tint and the names rather than through them — the console's answer to the TUI's reserved arc row |

Both ends use `seg.cx`, the centre of the phase's own rail content, so a
road's end sits under the phase's node and not beside it.

## Tests

There is no JS runtime in this Rust-only workspace (decision 0009), so
the tests pin the pair the surfaces actually share: the MODEL the
console is served, asserted on committed journal fixtures through the
console's own route, and the rendering rules the page states about it,
asserted on the served page's source.

| claim | test |
| --- | --- |
| the road is on the wire and the page draws from it | `the_console_is_served_the_road_back_and_the_page_draws_it` |
| a forward-only run's graph is unchanged — height, elements and legend all gated on the same empty list | `a_run_that_never_went_back_renders_the_graph_it_did_before` |
| drawn only when taken: no road from a visit count, both geometry drops present | `the_road_back_is_never_inferred_from_a_repeated_visit` |

## Non-goals held

No change to `brokkr-view` (`Phase.returns`, `phase_rail`) — the parity
oracle was already right, which is why `VIEW_VERSION` does not move. No
change to `tui.rs`, `ui.rs` or any route. No contract, policy table,
recipe or bundle change. No new visit-count UI: the revisit marker the
console already painted is the repeat's answer here as in the terminal.
