# Implementation Plan: `tui-graph`

**Feature slug**: `tui-graph`
**Spec**: [spec.md](spec.md) · **Tasks**: [tasks.md](tasks.md)

HOW the spec's ruling is built: the shape of the code, the files it
touches, the coverage strategy that AC-gate forces, and the risks with
their mitigations.

## Files touched

| File | Change |
| --- | --- |
| `crates/brokkr-cli/src/tui.rs` | the constants block gains `PULSE_TICKS`/`PULSE_FRAMES`; `Tui` gains `node: Option<String>` and `animate: bool`; `Key` gains `Left`/`Right` and `from_key` gains two arms; `apply` gains two arms; `step`/`keys_for`'s graph slots gain the lane cursor; `enter` **unchanged**; `assign_run` clears `node`; `footer_for`'s `(Level::Run, 0)` arm and `HELP` gain a line; `draw_graph` is replaced by `plan` + `paint` + `look` + `pulse` + `width_of` + `mode_for` |
| `crates/brokkr-cli/src/tui/tests.rs` | headless navigation cases; `TestBackend` cases for fork/join, the three modes, hostile and CJK phase names, minimum-width legibility, selection-vs-current on a colour-free channel; direct `plan`/`pulse`/`look` unit cases; the discipline test gains a no-canvas assertion; existing `Up`/`Down`-at-graph cases re-pointed at `j`/`k` |
| `crates/brokkr-cli/src/main.rs` | one argument at the `tui::start` call site in `run_tui`: `render::Style::detect().color` |
| `crates/brokkr-view/tests.rs` | **one test**: phase names are unique within a `RunView`. No `lib.rs` change, no `VIEW_VERSION` move |
| `specs/tui-graph/{spec,plan,tasks}.md`, `openspec/changes/tui-graph/proposal.md` | the committed design artifacts |

Not touched, deliberately: `render.rs` (beyond calling `keeps_phase` and
`Style::detect`, both existing `pub`), `ui.html`, `Cargo.toml`,
`policy/`, `reference/`, the corpus, every golden.

## The partition

Four layers, in the order the coverage gate cares about:

| layer | what | reached by |
| --- | --- | --- |
| **pure, no ratatui geometry** | `pulse`, `look`, `mode_for` | direct unit calls |
| **pure, owned integer geometry** | `plan` | direct unit calls, asserting the `Plan` struct |
| **paint** | `paint(&Plan, tick, animate) -> Vec<Line>` | direct calls (spans + styles) and `TestBackend` |
| **wiring** | `draw_graph` = `plan` → `paint` → `Paragraph::new(..).block(pane(..))` | `TestBackend` |

The split exists for AC-gate (spec R1). `plan` is where every degenerate
branch lives, and every one of them is reachable by constructing a
three-line model and calling a function — never by steering a terminal
buffer at a chosen width and reading glyphs back out.

### The plan type

Deliberately small and entirely owned; it exists to be asserted, not to
be extended.

```rust
enum Mode { Full, Rail, Compressed }

struct Plan {
    mode: Mode,
    rail_row: usize,
    name_row: usize,
    segments: Vec<Seg>,
    left_elided: bool,
    right_elided: bool,
}

struct Seg {
    key: String,            // Phase.name — the cursor key, S1/R7
    x0: usize,              // inclusive rail span
    x1: usize,
    name: String,           // already truncated; sanitized at paint
    visits: Option<u64>,    // Some only when > 1, clamped (R5)
    scoped: bool,           // render::keeps_phase — called, not reimplemented
    selected: bool,
    marks: Vec<Mark>,
    joins: Vec<Join>,
}

struct Mark {
    x: usize,
    row: usize,
    class: Class,
    live: bool,             // the pulse gate, from a model field
    key: Option<String>,    // Node.key; None for a phase rail node
    label: Option<String>,
    selected: bool,
}

/// A fork: leaves the rail at x0, rejoins at x1. `overflow` is the `+k`
/// of members that did not fit the lane budget (AC-mode-2).
struct Join { x0: usize, x1: usize, rows: Vec<usize>, overflow: usize }
```

`plan` returns a layout that **fits by construction**; `paint` contains
no clipping branch, because a painter that clips is a painter whose
failure mode is invisible.

### State

Two new fields on `Tui`, both scalars, both consistent with 0014's
"retain no model":

```rust
/// The node inside the selected phase the cursor has walked into.
/// Display-only: `Enter` scopes the phase whatever this says.
pub node: Option<String>,
/// Animation is enabled exactly when colour is (spec §5).
pub animate: bool,
```

`cursor[0]` still holds `Phase.name` and is still the input to
`keys_for`, `labels_for`, `selected`, `enter`, `settle` and the `/`
filter. The lane cursor is subordinate: it is never a scope key, and
`enter`'s `(Level::Run, 0)` arm is not edited at all.

`node` clears in exactly one place — `assign_run` — and otherwise clears
by **absence**: a stale key matches no drawn node.

### Keys

`Key::Left` and `Key::Right` join the enum and `from_key`
(`KeyCode::Left`/`KeyCode::Right`, two arms). `apply` gains two arms
routing to a single `graph_move(tui, views, axis, step)` helper so the
dispatcher's shape does not change. Outside the graph pane they are a
**named** no-op arm, executed by AC-nav-5 — a wildcard there would be an
untested claim about what the rest of the TUI does with an arrow key.

### Movement

Both axes go through the existing `move_to`, so wrap-around, the
empty-list case and the no-cursor case are already specified and already
tested:

- rail: `move_to(&keys_for(tui, views), &mut tui.cursor[0], step)`, then
  `tui.node = None`
- lanes: `move_to(&lane_keys(view, cursor), &mut tui.node, step)`, where
  `lane_keys` is the selected phase's nodes in draw order

`lane_keys` on a plain phase returns an empty `Vec`, and `move_to` on an
empty list sets `None` — inert by construction, not by a special case.

### Widths

One function, and every width in `plan` comes from it:

```rust
/// ratatui's own measurement of the sanitized text — which is what the
/// buffer will actually draw. `Safe::width()` is a char count and
/// reports 6 for a 12-column CJK phase name.
fn width_of(text: &str) -> usize {
    span(text, plain()).width()
}
```

It routes through the existing sanitized constructor, so it adds no
dependency, introduces no second measurement system, and keeps
`Span::styled(` at exactly one occurrence.

### Animation

`PULSE_TICKS` and `PULSE_FRAMES` join the constants block next to `TICK`
and `RUNS_REFRESH_TICKS`. `pulse` is a two-arm total function of
`(usize, bool, bool)`. The glyph is `look(class).1[pulse(..)]` — an array
index, not a branch, so the frame costs nothing under the gate and
non-pulsing classes need no special case.

**No change to `drive()`.** It already redraws unconditionally every
iteration and already increments `tui.ticks`.

## Coverage

AC-gate is the tightest constraint in the framing, so it is designed for
before code is written.

| construct | arms | how each is executed |
| --- | --- | --- |
| `look(class)` | 7 | one unit case per class, including `Unknown` reached from an unrecognised `state_class` and from an unrecognised `summary.status` |
| `pulse` | 2 | live+animate across a period and at `usize::MAX`; `!live`; `!animate` |
| `mode_for` | 3 | one `plan` call per row budget |
| `plan` geometry | plain phase · single-node column · fork · fork overflow · empty `phases` · zero-node column · one-node "fork" · window elision left/right/both/neither · label truncation | direct `plan` calls asserting `Plan` |
| `from_key` | +2 | two key-translation cases |
| `apply` | +2 | AC-nav-1 and AC-nav-5 |
| `paint` | mode × mark × join | `TestBackend` for AC-draw-*, direct `Vec<Line>` inspection for styles |

Two rules that keep the matrix from multiplying:

1. **Geometry does not depend on `tick`.** The pulse touches glyph and
   style only. So layout cases run at one tick, and animation cases run
   at one layout.
2. **No `unreachable!()` anywhere.** Every closed-set match ends in a
   named fallback arm with a test that executes it. An `unreachable!()`
   over a `String` crossing `VIEW_VERSION` is either a panic in an
   operator's terminal or an uncoverable line.

The one impure addition, `render::Style::detect().color` in `run_tui`, is
on the same line kind as the existing `std::io::stdout().is_terminal()`
argument, inside the function that is already injected into `run_with` as
a parameter — so it inherits that seam and adds no new uncovered line.

## Safety

Three properties, all held by tests over `SOURCE` rather than by memory:

- **Two constructors.** `paint` emits spans through `span()` only, so
  `Cell::from(` and `Span::styled(` stay at exactly one occurrence each.
  The discipline test is **unamended** — it gains an assertion, never
  loses one.
- **No canvas.** One new assertion, so the spec's ruling is enforced
  rather than remembered.
- **The graph stays in `tui.rs`.** `SOURCE` is
  `include_str!("../tui.rs")`. Moving the renderer to `tui/graph.rs`
  would silently drop it out of reach of the read-only
  (`Store`, `brokkr_runtime`, `process::exit`) and sanitization greps.
  Concurrency argues for a new file to dodge the rebase; that trade is
  refused. A contiguous block rebases as a block move.

## Risks and mitigations

| risk | mitigation |
| --- | --- |
| **Mechanical rebase** against the trail-reader overlay and two parallel runs | Collision surface confined to intake's four named points: `Key`+`from_key`, the `(Level::Run, 0)` slots of `keys_for`/`step`/`apply`, `footer_for`+`HELP`, and the constants block. The graph's volume sits behind `plan`/`paint` in one contiguous section, which rebases as a block. `HELP`'s length changes — take the mechanical merge and keep both lines. |
| **AC-gate fails on an unreachable geometry branch** | The `plan`/`paint` split exists for exactly this; the coverage table above names the executor for every arm before code is written. |
| **A CJK or combining phase name breaks the rail** | `width_of` is the single width source, proven on `設計フェーズ` (AC-safe-2), and labels are clamped to a named maximum through the same measurement. |
| **The pane is ~1 row at `MIN_HEIGHT` and a fork cannot fit** | `mode_for`'s three modes; `Compressed` is a real mode with a real test, never a blank pane or a refusal. |
| **A compacted fork reads as a sequential step** | Every mode that cannot draw lanes still emits `⑂n`. Asserted by AC-mode-1. |
| **An unknown `summary.status` paints a dead run live-green** | The `Unknown` class is the fallback, and AC-look-2 executes it. Divergence from `ui.html` is stated in spec R3, not smuggled. |
| **Selection and current look alike under `NO_COLOR`** | They differ in modifier and glyph, and AC-nav-4 asserts on the non-colour channel. Same rule for `Active` vs `Finished` (AC-look-3). |
| **Phase-name collision would make `Enter` scope the wrong segment** | The dependency is stated in spec §8 and pinned by one `brokkr-view` test (AC-view-1). |
| **Re-pointing `Up`/`Down` breaks existing tests** | Bounded and named: only rail-movement cases move to `j`/`k`, which assert the same movement. No case asserting a scope is touched (AC-nav-2). |
| **A ratatui bump changes glyph widths** | Every width is ratatui's own measurement taken at runtime, so a bump changes the frame's tightness, never its correctness. |
</content>
