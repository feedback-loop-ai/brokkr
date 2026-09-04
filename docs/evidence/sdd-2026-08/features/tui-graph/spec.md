# Feature Specification: `tui-graph` — the console's graph grammar, in the terminal

**Feature slug**: `tui-graph`
**Run**: `the-tui-graph-should-feel-like-t-f464bd78`
**Status**: Committed (design phase ruling)
**Scope**: display refinement **within** decision 0014
(`docs/decisions/0014-interactive-tui.md`), over the models decision 0013
(`docs/decisions/0013-one-derivation-two-surfaces.md`) established.
**No new decision doc.**
**Input positions**: `.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md` (run-local, uncommitted)

## Why

The operator asked for it in their own words: *"I would like a similar
visual experience in the TUI than in the WebUI — with proper navigation
and proper graphical visualization of the graph, with pulsating circles
etc."*

The console (`crates/brokkr-cli/src/ui.html::renderLoops`) draws the run
as **one rail**: phases are segments joined by arrowed edges that read as
*then*, a panel **forks** into vertically symmetric lanes that **rejoin
the rail** before the next step, phase names share one baseline with `×N`
revisit markers, node colour is an allowlist lookup on a closed-set key,
and the active node on a live run **pulses**. Every one of those carries
meaning. The rejoin *is* the join dependency. The pulse *is* "this is
happening now, and the frame you are looking at is not stale."

`brokkr tui`'s graph pane today (`tui.rs::draw_graph`) is the 0013 *tree*:
one indented line per phase, `→` for a sequential step, `⑂` for a fork,
`state` printed as a word, no colour, no rail, **no join**, no animation.
It lists the topology; it does not show it. An operator moving between
the two surfaces has to re-learn the run's shape in each one.

This feature spends 0014's promise that the TUI is a *renderer*, and
spends nothing else: it adds **no derivation**, no dependency, no
decision, and no contract change. It is geometry and paint over models
that already exist.

## The design in one paragraph

The graph pane becomes a **character grid** — box-drawing, one glyph per
cell, **no `Canvas`** — produced by a two-function split inside `tui.rs`:
a pure `plan(phases, lens, selection, width, height) -> Plan` of owned
integer geometry, and a `paint(&Plan, tick, animate) -> Vec<Line>` that
walks the plan and emits spans through the **existing** `span()`
constructor, so the widget layer keeps exactly two sanitized
constructors and the source-level discipline test stays unamended. The
plan **fits by construction** — the painter never clips — through three
named modes (`Full`, `Rail`, `Compressed`) chosen by inner height, and a
rail **window derived each frame** from the selection rather than a
stored scroll offset. Node look is a single closed-set table over seven
classes, transliterated from the console's `NODE_CLASS` plus its
`phase.current` × `summary.status` branch, each class carrying a
**(colour, four-glyph marker ramp)** pair so active and finished differ
without colour. Animation is one `const PULSE_TICKS`, one four-frame
ramp, and one total function `pulse(tick, live, animate) -> usize`
indexing that ramp — **no new clock, no new wakeup, no new draw**,
because `drive()` already redraws every `TICK` and already carries
`tui.ticks`. Navigation adds `Key::Left`/`Key::Right` (the rail, over the
existing `move_to`) and re-points `Key::Up`/`Key::Down` in the graph pane
onto a **subordinate** lane cursor; `cursor[0]` keeps holding the phase
name and `enter`'s `(Level::Run, 0)` arm is unchanged byte-for-byte.

## The rule that governs every other choice

> **`brokkr tui` is a THIRD RENDERER, never a fourth derivation.**
> A renderer may branch on a model field; it may not compute one.

Selecting, filtering, ordering for display, **geometry and layout** are
rendering. Deriving status, topology or scope membership is not. This
feature is entirely on the rendering side of that line, and it stays
there: `brokkr-view` gains **no field, no model change, no
`VIEW_VERSION` move** — only one test (§8).

---

## What the positions settled

Both seats compiled probes against the pinned `ratatui 0.30.2` and
deleted them. Their evidence agrees where it overlaps, and I take it as
established fact rather than re-deriving it.

### Where both positions already agreed — adopted without argument

1. **Canvas is out.** Both verified it is *available* under the pin
   (`ratatui-widgets 0.3.2` is an unconditional edge; no manifest edit
   either way), so intake's open question is closed on availability and
   the choice is on merit. Both rule against it, and the merits are
   decisive — see §1 below.
2. **`cursor[0]` keeps holding the phase key** and `enter` is untouched.
3. **No new clock.** `tui.ticks` is the counter; the pulse touches glyph
   and style only, never position.
4. **No scroll offset.** The visible window is derived from the
   selection each frame.
5. **The graph stays inside `tui.rs`.** `SOURCE` is
   `include_str!("../tui.rs")`; a new module would move the renderer out
   of reach of the read-only and sanitization greps.
6. **Intake's open question resolves to option (a)**: branch in `tui.rs`
   on `phase.current` + `summary.status`. Option (b) — a phase-level
   `Phase.state_class` in `brokkr-view` — is a cross-surface contract
   change (`VIEW_VERSION`, the `/api` shape, the console's allowlist)
   bought to save match arms in one renderer, and would either force a
   `ui.html` edit the non-goals forbid or leave the console computing
   the same fact anyway. Rejected.

### Ruled for robustness

**R1 — `plan` / `paint` is a real split, and it is bought for AC-gate.**
Simplicity argued for a single `graph_lines() -> Vec<Line>`, on the
grounds that `Line`/`Span` is already the vocabulary and is directly
unit-testable. It is testable — but asserting *"the fork left the rail at
column 21 and rejoined at 37"* against a `Vec<Line>` is a substring
search over rendered rows, which is the same fragile assertion as reading
a buffer, one indirection earlier. AC-gate demands literal nonzero 100%
line/branch/function equality, and the branches hardest to reach that way
are exactly the degenerate ones (a zero-node column, a fork taller than
its lane budget, a rail wider than the pane). With the split those are
`assert_eq!` against a small owned struct. **The cost is one struct and
one function boundary, not a module** — and simplicity's constraint on
the painter is adopted whole: `paint` emits `Vec<Line<'static>>` through
`span()` and nothing else, so no third widget constructor appears and the
discipline test is unamended.

**R2 — all widths come from `width_of`, never `Safe::width()`.**
Robustness measured it: `Safe::width()` is `chars().count()`, which
returns **6** for `設計フェーズ` where ratatui draws **12**. Today that is
harmless because `Table` and `Paragraph` do the measuring; **the rail is
the first pane in `tui.rs` that places its own x positions**, so it is
the first that can be lied to. Rule: one
`fn width_of(text: &str) -> usize`, implemented as
`span(text, plain()).width()`. Every width in `plan` comes from it and
from nowhere else. Tested on a CJK name, not only on ASCII.

**R3 — an unrecognised `summary.status` renders quiet, never live.**
Simplicity's table had `current, otherwise => Green`, which is the arm an
unknown status falls through. Robustness is right that the fallback must
be the quiet class. This is a **deliberate, named divergence from
`ui.html`**, which maps an unrecognised status to `on-phosphor` (green,
still). The terminal declines to guess, in the manner of
`render::tone`'s own `_ => Tone::Quiet`, and per decision 0001. Stated
here rather than discovered later; the pulse gate is
`status == "running"` on both surfaces, so an unknown status never
animates on either.

**R4 — degradation is two-dimensional, and the floor does not lie.**
The framing argues width all the way down and never mentions rows, but
**lanes are vertical**: at `MIN_HEIGHT = 12` the body is 10 rows, the
graph pane is 34% of it, and the inner rect is roughly **one row**. A
fork does not fit at any terminal `refuse()` admits. Rule: `plan` takes
`(width, height)` and returns a layout that fits, and the compacted forms
**still say "fork"** (`⑂n`) rather than collapsing a parallel column into
something that reads as a sequential step.

**R5 — every closed set gets a named fallback arm that a test executes.**
`Node.state_class` and `Summary.status` are `String`s crossing
`VIEW_VERSION`, not enums. No `unreachable!()` anywhere: it is either a
panic in an operator's terminal or an uncoverable line, and AC-gate fails
on the second. `Phase.visits` is `u64` and its `×N` marker is clamped, so
a corrupt fold cannot put `×18446744073709551615` on the baseline.

**R6 — selection ≠ current, and active ≠ finished, in a channel that is
not colour.** A `TestBackend` assertion passes if two things differ only
by `fg`, and then a `NO_COLOR` terminal renders them identically. The
state vocabulary is therefore **(colour, marker) pairs**, and AC-nav-4 is
asserted on the glyph/modifier channel.

**R7 — phase-name uniqueness is a stated cross-crate dependency.**
`visits` is a fold keyed by name, so a revisited phase is one segment.
The graph now *depends* on that for selection identity; if it stopped
holding, two segments would share a cursor key and `Enter` would scope
the wrong one, invisibly. Pinned by one `brokkr-view` test (§8).

### Ruled for simplicity

**S1 — the lane cursor is a stable key, not an index.**
Robustness asked for a `(column, node)` index clamped against fresh
models every frame. Simplicity's `node: Option<String>` holding
`Node.key` is better *on 0014's own terms*: 0014's whole selection
design is "stable key resolved against fresh models, retaining nothing",
which makes "survives a refresh" and "clears when its subject vanishes"
the **absence of code**. A clamped index is retained *position*, which is
the thing 0014 removed. A stale key simply matches no drawn node.
Robustness's underlying requirement — that the lane cursor is subordinate
and never becomes a scope key — is fully satisfied: `Enter` reads
`cursor[0]`.

**S2 — the horizontal window is the arrow keys, not a new feature.**
The console's answer was `overflow-x: auto`. Ours is a window over the
phase list that always contains the graph cursor. This is deliverable 4's
movement, reused; it adds no field, no binding and no clamping tests, and
it is why AC-nav-3 needs no diff routine.

**S3 — `MIN_WIDTH` stays 60 and the pane proportions stay 34/33/33.**
The full grammar fits at 80×24. Moving either would ripple through
`refuse()`, its message text, `tui/tests.rs:942,959` and
`tests/machine_proof.rs` for no gain the framing asked for.

**S4 — animation is enabled exactly when colour is.** One rule, no new
flag, no new env var, no new doc surface; `NO_COLOR=1`, `TERM=dumb` and a
non-tty all yield a still graph through the existing pure
`color_enabled`. The `animate` fact arrives at `tui::start` as a `bool`
parameter, computed at the same call site and in the same manner as
`is_tty`, so tests set it directly and AC-anim-3 executes without
touching an environment.

**S5 — the ruling against Canvas is held by a test, not by memory.**
One line in the discipline test asserting the source names no canvas.

### Where I reconciled them

**C1 — the vertical ladder is exactly three modes.** Robustness wanted a
graded ladder; simplicity wanted two modes. Both are right about
something: a long ladder multiplies arms under AC-gate, and two modes
leave a gap between "full lanes" and "one line". Ruling: **three named
modes and one predicate** (§4). Each is one test. Robustness's honesty
requirement is met inside all three, because every mode that cannot draw
lanes still draws `⑂n`.

**C2 — the fallback for an over-wide fork is `+k`, not silence.**
Simplicity's proposal, and it is exactly robustness's "honest, not
silent" rule; they were arguing past each other. Adopted.

**C3 — `render::tone` is *not* reused for the graph, and that is not an
oversight.** `tone` maps `awaiting_operator` to `Quiet` (it falls to
`_`), while the graph's vocabulary needs **park** as a distinct, yellow
class — deliverable 2 names it. Widening `tone` would move `brokkr runs`
colour, and the goldens must stay byte-identical. The graph's table is
the *console's* classification (`NODE_CLASS` + the phase branch), which
is a different classification from "how a run status reads in a table
row". One classification per question, not one table for two questions.

---

## Behaviour

### 1. Character grid, not Canvas

The graph is drawn with box-drawing characters, one glyph per cell.
`ratatui::widgets::canvas` is not used. The reasons, from both positions'
probes:

- **Braille destroys per-node colour.** Colour on a Canvas is stored
  **per cell, last writer wins**. A node circle drawn over a rail
  recolours the rail's cells while they still carry the rail's dots.
  Deliverable 2 — per-node colour from a fixed vocabulary — is
  structurally unsatisfiable wherever a node shares a cell with an edge
  or an adjacent lane, which at terminal densities is most nodes. It
  would trade the deliverable for the decoration.
- **`ctx.print` is a third path into a buffer, and its apparent safety is
  borrowed.** Printing `in\u{202E}take\u{200B}x\u{07}y` through
  `ctx.print` yields `intakexy` — ratatui silently dropped the override,
  the zero-width space and the bell. AC-safe-1 would pass **by
  accident**, on `ratatui-core::set_string`'s undocumented filtering
  rather than on `Safe`'s enumerated `reorders` table. A ratatui bump
  could regress terminal safety with a green suite.
- **Deliverable 5's assertions cannot be written over it.** A one-unit
  horizontal rail splits across two cell rows as a doubled ghost
  (`⣀⣀⣀` / `⠉⠉⠉`); a circle renders `⢀⣀⣀⡀` / `⠈⠉⠉⠁`. AC-draw-1's
  "lanes leave the rail, run parallel, and rejoin" becomes an
  unreviewable golden on a feature whose entire point is that a human can
  see the shape.
- **It fights AC-gate.** Float world-space clipping and marker-resolution
  branches we do not own and cannot deterministically drive.
- **It buys resolution the pane does not have.** At 80×24 the graph pane
  is 5 inner rows. Sub-cell smoothing inside 5 rows is invisible, and a
  `●` at one-cell scale is a truer circle than any braille approximation
  of one.

What is conceded: no sub-cell smoothness, no true circles. The console's
grammar is **topological** — rail, arrow, fork, rejoin, baseline — and
every element of it survives at cell resolution. Stroke fidelity was
never the thing being matched.

### 2. The grammar, in characters

Illustrative, at 80 columns (exact column arithmetic is `plan`'s):

```
┌graph─────────────────────────────────────────────────────────────────────┐
│                    ┌── ● robustness ──┐                                   │
│ ‹ ○ ──→ ○ frame ──→┤                  ├──→ ○                              │
│                    └── ◉ simplicity ──┘                                   │
│   intake    ▸design ×2                    review                        › │
└───────────────────────────────────────────────────────────────────────────┘
```

- **One rail row.** Segments run left to right, joined by `──→` — the
  console's arrowed edge, reading as *then*.
- **A plain phase** (`phase.plain`) is one node on the rail: filled when
  current, hollow otherwise. That is the console's `r=7` vs `r=5.5`
  expressed in the only axis a cell has.
- **A single-node column** is a node on the rail with its label to its
  right on the rail row.
- **A fork** (a column of many nodes) leaves the rail at `x0` and
  **rejoins at `x1`**. The rail-row characters are `┤` at `x0` and `├` at
  `x1`, or `┼` at both when the member count is odd and a member occupies
  the rail row. Member `k` of `n` sits at row offset `k − (n−1)/2`,
  symmetric about the rail, with `┌ └ ┐ ┘` corners and `─` runs. **The
  rejoin is drawn, always** — it is the join dependency, and it is the
  one thing the current tree cannot say.
- **Fork member labels ride their own lane**, to the right of the node,
  exactly as the console does; only non-forked columns put their label on
  the shared step baseline.
- **Phase names share one baseline** (the last row): `▸` when
  `render::keeps_phase(lens, phase)` — the crate's one scope predicate,
  **called, never reimplemented** — then the name, then `×N` **only when
  `visits > 1`**, matching the console rather than today's unconditional
  `×1`.
- **Elision marks** `‹` and `›` sit in the pane's edge columns when
  phases fall outside the window (§4).

### 3. Node look: one closed-set table, seven classes

One `Class` enum and one `fn look(class) -> (Style, [&'static str; 4])`,
in the manner of `tone_style`: one classification, one rendering of it.
The four-glyph array is the marker **ramp** (§5); classes that do not
pulse repeat one glyph, so the frame index is inert for them and costs no
branch.

| class | source | colour | ramp | still |
| --- | --- | --- | --- | --- |
| `Visited` | rail node, `!phase.current` | Magenta | `○ ○ ○ ○` | `○` |
| `Current` | rail node, `current` + status `running`\|`completed` | Green | `● ◉ ○ ◉` | `●` |
| `Park` | `current` + `awaiting_operator`; or node `on-park` | Yellow | `⊙ ⊙ ⊙ ⊙` | `⊙` |
| `Failed` | `current` + `stopped`; or node `on-halt` | Red | `⊗ ⊗ ⊗ ⊗` | `⊗` |
| `Finished` | node `state_class == on-phosphor` | Green | `● ● ● ●` | `●` |
| `Active` | node `state_class == in-active` | Magenta + BOLD | `◉ ◎ ○ ◎` | `◉` |
| `Unknown` | **fallback**: unrecognised `state_class`, or `current` with an unrecognised `summary.status` | DIM | `· · · ·` | `·` |

The six named classes are deliverable 2's vocabulary verbatim — visited,
current, finished, active, failed, park — plus the one fallback arm Rust
obliges and R5 requires a test to execute. Colour is looked up from the
model's closed-set key; **no journal string reaches this table**.

`Active`'s still glyph `◉` differs from `Finished`'s `●` and `Current`'s
`●` in the **glyph** channel, so the distinction survives `NO_COLOR` and
survives animation being off (R6).

### 4. Degradation, in two axes, by a stated rule

**Height — three modes, one predicate.** `mode_for(inner_rows,
max_lane_span)`:

| mode | when | what is drawn |
| --- | --- | --- |
| `Full` | rail + name baseline + at least one lane row fits | the §2 grammar in full |
| `Rail` | only rail row + name baseline fit | rail, arrows, names; **every fork collapses to one rail node bearing `⑂n`** and the worst member state |
| `Compressed` | one row | one inline line: `○ intake ──→ ● design ×2 ⑂2 ──→ ○ review` |

Row allocation in `Full` is fixed and ordered: the rail row, the name
row, then whatever remains becomes lane rows distributed symmetrically
outward. A fork with more members than lane rows draws the members that
fit and puts `+k` on the outermost lane — honest, not silent.

At `MIN_HEIGHT = 12` the graph pane has one usable inner row, so
`Compressed` is the mode an operator dragging a window edge lands in.
**It is never a blank pane and never a refusal**: losing the graph while
resizing is worse than a compact one. And in all three modes a parallel
column still reads as parallel (`⑂n`), so the small forms do not lie
about the shape of the run.

**Width — navigation is the scroll.** The rail is a **window over the
phase list that always contains the graph cursor** (or, with no cursor,
the current phase); phases outside it are elided and marked `‹` / `›` in
the pane's edge columns. There is **no scroll offset field** — the window
is derived each frame, so there is no second piece of state that can
desynchronise from the selection.

Within the window, **skeleton before text**: if a segment cannot fit, its
labels truncate with `…`; the rail, the arrow, the fork corners and the
rejoin are **never** dropped. All widths are `width_of` (R2).

`MIN_WIDTH` stays **60**; `refuse()`, its message and its tests do not
move; the 34/33/33 pane split is untouched.

### 5. The pulse

```rust
/// One pulse frame per this many shell ticks: 4 frames × 2 × TICK(250ms)
/// ≈ a 2s breath, the terminal's answer to the console's 1.8s.
const PULSE_TICKS: usize = 2;
const PULSE_FRAMES: usize = 4;

fn pulse(tick: usize, live: bool, animate: bool) -> usize {
    match live && animate {
        true => (tick / PULSE_TICKS) % PULSE_FRAMES,
        false => 0,
    }
}
```

The glyph is `look(class).1[pulse(tick, live, animate)]`. Every hard
constraint the framing set falls out of this rather than being enforced
against it:

- **Pure function of a tick and the models.** `live` is
  `summary.status == "running"` for the current plain node, and
  `node.state == "active"` for an inner node — both model fields, passed
  in. `pulse` takes a `usize` and two `bool`s.
- **Never reads the store.** There is nothing to read *from*.
- **Bounded at a modest, named rate.** `PULSE_TICKS`, a multiple of the
  existing `TICK`. **No new timer, no new thread, no new event source,
  no change to `drive()`** — the shell already redraws unconditionally
  every iteration and already carries `tui.ticks`. The honest statement
  is stronger than "bounded": animation adds **zero** additional wakeups
  and zero additional draws.
- **Stops entirely when no run is live.** `!live` returns frame 0 —
  the still frame — at every tick. There is no idle cost to remove
  because none was added. (Honest limit: this does not make an idle TUI
  *cheaper* than it is today; it costs nothing extra.)
- **Total over `usize`,** including `usize::MAX`, by modulo.
- **The pulse moves the glyph, never the colour and never a position.**
  Colour is the state vocabulary; an animation that borrowed it would
  make a live node briefly indistinguishable from a parked one. And
  because geometry is independent of `tick`, `plan` stays a pure function
  of the models and the rect, and no layout assertion becomes
  tick-dependent.

**The no-animation rule, stated:** *animation is enabled exactly when
colour is.* `run_tui` passes `render::Style::detect().color` as a new
`animate: bool` argument to `tui::start`, which stores it on `Tui`. So
`NO_COLOR=1`, `TERM=dumb` and a non-tty all yield a still graph, through
the existing pure `color_enabled(is_terminal, no_color, term_is_dumb)`.
No new flag, no new env var, no new doc surface.

### 6. Navigation

One new field: `pub node: Option<String>` on `Tui` — the `Node.key` the
cursor has walked into. Display-only: `Enter` scopes the phase whatever
it says.

| key | in the graph pane (`Level::Run`, pane 0) | elsewhere |
| --- | --- | --- |
| `Left` / `Right` | move `cursor[0]` along the rail via the existing `move_to` over `keys_for`; clear `node` | a **named** no-op arm |
| `Up` / `Down` | move `node` via the same `move_to` over the selected phase's node keys in draw order (`columns.iter().flat_map(\|c\| &c.nodes)`) | unchanged |
| `j` / `k` | move the rail, unchanged | unchanged |
| `Enter` | `(Level::Run, 0) => scope = Scope::Phase(key)` — **unchanged, byte-for-byte** | unchanged |

Because both movements go through `move_to`, wrap-around at the ends of
the rail and at the edges of a lane group is already specified, already
implemented and already tested behaviour. A plain phase has no nodes, so
`move_to` over an empty list sets `None`: `Up`/`Down` there are inert by
construction, not by a special case.

`assign_run` clears `node` alongside the selections it already clears.
Vanishing is again the **absence of code**: a stale node key matches no
drawn node, so nothing highlights, and the next `Up` lands on the first
node.

**Selection versus current.** Selection is `Modifier::REVERSED` on the
phase name (or on the node label when a lane node is selected) — the
TUI's existing selection idiom, identical to the runs table, the seats
table and the trail, so an operator already knows it. Current is a
**filled, coloured rail glyph**. Different attribute, different cell,
different axis; where the selected phase *is* the current phase both
marks are present and still separable.

The footer's `(Level::Run, 0)` arm gains `←→ rail · ↑↓ lanes`, and `HELP`
gains one line (its `[&str; 12]` length changes).

**One bounded, named test edit.** `Up`/`Down` at `(Level::Run, 0)` moves
lanes rather than the rail, so existing headless cases that press
`Key::Up`/`Key::Down` there are re-pointed at `j`/`k`, which assert the
same rail movement. That is a mechanical test edit, not a behaviour
regression, and it does not touch any case asserting a **scope** — those
stay green byte-for-byte, as AC-nav-2 requires.

### 7. Safety, unchanged and unweakened

Every string reaching a cell goes through `Safe`. `paint` emits spans
only via the existing `span()`, so `tui.rs` keeps **exactly two**
constructors that reach a widget (`Cell::from(` and `Span::styled(`, one
occurrence each) and the source-level discipline test is **unamended** —
it gains assertions, never loses one. Widths are `width_of` =
`span(text, plain()).width()`, which *is* ratatui's measurement of the
sanitized text, so there is no second width implementation to disagree
with the first. The graph stays inside `tui.rs` so `SOURCE`'s read-only
and sanitization greps keep covering it.

### 8. `brokkr-view`: no model change, one test

`brokkr-view` gains **no field and no rendering concern**; `VIEW_VERSION`
does not move. It gains **one test**, pinning that phase names are unique
within a `RunView` — the invariant the `visits` fold already holds, and
which the graph now depends on for selection identity (R7). A test is not
a fact added to the model; it converts a silent cross-crate coupling into
a named one.

---

## Acceptance Criteria

**AC-nav-1** `apply` moves the graph cursor **along the rail** with
`Left`/`Right` and **into lanes** with `Up`/`Down`, headlessly, with no
terminal — including wrap-around at both ends of the rail and at both
edges of a lane group.

**AC-nav-2** `Enter` on a graph selection scopes **the phase** exactly as
today, whatever the lane cursor says; every existing enter/escape ladder
case that asserts a scope stays green byte-for-byte.

**AC-nav-3** A phase selection whose subject vanishes on the next refresh
clears itself; one whose subject persists survives. A lane selection
whose node vanishes highlights nothing and does not panic.

**AC-nav-4** The selection affordance and the current-phase affordance
differ in the rendered buffer **in a channel that is not colour** —
asserted on a frame where the selected phase is *not* the current phase,
and on one where it is.

**AC-nav-5** `Left`/`Right` outside the graph pane hit a named no-op arm
and change no state.

**AC-draw-1** A fork/join layout renders through `TestBackend`: two or
more lanes leaving the rail, running in parallel, and **rejoining** the
rail before the next step's edge.

**AC-draw-2** A plain phase renders as a single rail node; a sequence of
single-node columns renders as arrowed rail steps.

**AC-draw-3** Phase names sit on one shared baseline; `visits > 1`
renders `×N` and `visits == 1` renders no marker; a `u64::MAX` visit
count renders clamped.

**AC-draw-4** `render::keeps_phase` decides the `▸` scope marker, and the
graph lists every phase whether scoped or not — the lens marks, it does
not hide.

**AC-look-1** Each of the seven classes is produced by a test that
executes its arm, including the `Unknown` fallback reached from an
unrecognised `Node.state_class`.

**AC-look-2** A current phase whose `summary.status` is outside the known
set renders `Unknown` (quiet), **not** `Current` (green).

**AC-look-3** `Active` and `Finished` differ in the glyph channel on a
frame with animation disabled.

**AC-anim-1** `pulse` is pure and total: same `(tick, live, animate)` ⇒
same frame, asserted across a full period, across the `PULSE_TICKS`
boundary, and at `usize::MAX`.

**AC-anim-2** A run whose `summary.status` is not `running` produces the
**still** frame at every tick.

**AC-anim-3** With `animate == false`, every tick produces the still
frame — executed by setting the flag directly, without touching an
environment.

**AC-anim-4** The named tick rate is a `const`, the drawn frame at tick
*t* depends on no store call, and geometry does not vary with `tick` —
held as source-level properties of `tui.rs` plus a test asserting that
`plan`'s output is identical across two tick values.

**AC-mode-1** `mode_for` returns `Full`, `Rail` and `Compressed` at their
stated row budgets; each mode renders through `TestBackend`; every mode
that cannot draw lanes still marks a fork as `⑂n`.

**AC-mode-2** A fork with more members than lane rows draws the members
that fit and a `+k` count; no member is silently dropped.

**AC-mode-3** The rail window always contains the graph cursor; phases
outside it render `‹` / `›` elision marks. No scroll-offset state exists.

**AC-safe-1** A hostile phase name (U+202E bidi override, U+200B
zero-width, control characters) renders **inert** and does not displace
its neighbours: the following segment starts at its expected x. Layout
widths are ratatui's measurement of the sanitized text.

**AC-safe-2** A CJK phase name (`設計フェーズ`) does not break the rail:
`width_of` reports 12, not 6, and the next node lands where `plan` says.

**AC-safe-3** `tui.rs` still contains exactly one `Cell::from(` and one
`Span::styled(`, names no `Store`/`brokkr_runtime`, and names no canvas.

**AC-width-1** At `MIN_WIDTH = 60` the graph is legible by the stated
degradation rule and no frame is corrupted; at 80 columns the full
grammar — rail, arrow, fork, rejoin, name baseline — is present.

**AC-view-1** Phase names are unique within a `RunView`, asserted in
`brokkr-view`'s own tests.

**AC-gate** `scripts/coverage-exact.sh` passes: literal nonzero 100%
source-line / branch / function equality. Every class arm, every layout
branch and every animation arm has a test that executes it.

**AC-green** `cargo test --workspace`, clippy, fmt, the machine proof and
the differential corpus stay green; `render::graph_block` and every
existing golden stay byte-identical; the `/api` response shape is
unchanged.

---

## Non-goals

- **No new decision document.** Refinement inside 0014.
- **No new dependencies and no manifest edit.** Verified: none is needed
  either way.
- **No `Canvas`, braille, float world-space or `ctx.print`.**
- **No `brokkr-view` model change**, no `Phase.state_class`, no
  `VIEW_VERSION` move, no `/api` shape change.
- **No `ui.html` edit.** It is the reference being matched, not a target.
- **`brokkr inspect` / `brokkr watch` / `brokkr ui` untouched.**
  `render::graph_block` and every golden stay byte-identical.
- **No new module file.** The graph stays in `tui.rs`.
- **No animation timer, thread, `Instant`, or frame budget.**
- **No `--no-animation` flag and no new env var.**
- **No `h`/`l` bindings**; the framing asks for arrow keys and `j`/`k`
  already move.
- **No scroll-offset field.**
- **No drilling into a participant from a lane node.** The console does
  it; this framing says `Enter` scopes the phase exactly as today, and
  the seats pane already drills.
- **No mouse, no new pane, no re-proportioned panes, no filter changes,
  no `MIN_WIDTH`/`MIN_HEIGHT` move.**
- **No operator actions.** The TUI stays read-only (decision 0003).
- **Frozen contracts, the corpus, `policy/phase-machine.json` and
  `reference/` are untouched.**

## Risks accepted

1. **A long run shows only a window of its rail at 60–80 columns.** The
   arrow keys walk it, the status line names the scoped phase, and
   `brokkr inspect` prints the whole tree. The console's own answer was
   also "you cannot see it all at once — scroll".
2. **Forks wider than the available lane rows collapse to `+k`.** Panels
   are small in practice, and a visible honest count beats a silently
   dropped member (decision 0001's spirit).
3. **The pulse is a four-frame glyph ramp, coarser than an SVG halo.**
   One cell is the resolution limit, and braille would not have bought a
   halo either — it would have cost per-node colour.
4. **`● ○ ◉ ◎ ⊗ ⊙` are East-Asian-Ambiguous width.** `→`, `⑂` and `▸`
   already ship in `tui.rs` and in `graph_block`'s goldens, so the
   precedent is set; and because every layout width is ratatui's
   measurement of the sanitized text, a terminal that renders them wide
   gets a loose frame, never a corrupted one.
5. **`NO_COLOR` stills the animation without stilling the TUI's colour**,
   because `tui.rs` never consulted `color_enabled` at all. Making TUI
   colour honour `NO_COLOR` is coherent and probably right; it threads a
   flag through every style function and doubles the arms under the exact
   gate, and it is not in this framing. Left out deliberately, named here
   as follow-on work.
6. **In the graph pane `↑↓` and `j`/`k` do different things**, and some
   existing tests move from one to the other. Documented in the footer
   and in `HELP`: the graph is the one pane whose primary axis is
   horizontal, and the framing explicitly requires arrow semantics there.
7. **`Compressed` mode is a different picture from `Full` mode.** It is
   today's information in one line, and the alternative — a blank pane
   below three rows — is worse for an operator resizing a window.
8. **An unrecognised `summary.status` renders DIM in the terminal and
   green-still in the console** (R3). A deliberate one-glyph divergence,
   in a case that should not occur, on the side of not guessing.
9. **A mechanical rebase is expected.** Three other efforts touch
   `tui.rs` concurrently. The collision surface is deliberately confined
   to intake's four named points; see plan.md's risk register.
</content>
</invoke>
