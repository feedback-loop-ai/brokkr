# Change Proposal: tui-graph

## Why

The operator asked for it in their own words: *"I would like a similar
visual experience in the TUI than in the WebUI — with proper navigation
and proper graphical visualization of the graph, with pulsating circles
etc."*

The web console draws a run as **one rail**
(`crates/brokkr-cli/src/ui.html::renderLoops`): phases are segments joined
by arrowed edges that read as *then*, a panel **forks** into vertically
symmetric lanes that **rejoin the rail** before the next step, phase
names share one baseline with `×N` revisit markers, node colour is an
allowlist lookup on a closed-set key, and the active node on a live run
**pulses**. Each of those carries meaning: the rejoin *is* the join
dependency, and the pulse *is* "this is happening now, and the frame you
are looking at is not stale."

`brokkr tui`'s graph pane today (`tui.rs::draw_graph`) is the 0013 tree —
one indented line per phase, `→` for a step, `⑂` for a fork, the state
printed as a word, no colour, **no join**, no animation. It lists the
topology; it does not show it, and an operator moving between the two
surfaces has to re-learn the run's shape in each one.

This is **display refinement within decision 0014**
(`docs/decisions/0014-interactive-tui.md`) — no new decision document. It
spends 0014's promise that the TUI is a *renderer* and spends nothing
else. The invariant that governs every choice below, restated from 0013:

> **`brokkr tui` is a third renderer, never a fourth derivation.**
> A renderer may branch on a model field; it may not compute one.

Geometry and layout are rendering. This change adds **no derivation, no
dependency, no decision, no contract change and no `brokkr-view` model
change**.

## What Changes

- **The graph pane becomes a character grid — box-drawing, one glyph per
  cell, and no `Canvas`.** Both design seats compiled probes against the
  pinned `ratatui 0.30.2` and agree Canvas is *available* under
  `default-features = false` with no manifest edit, so intake's open
  question is closed and the choice is on merit. The merits rule it out:
  braille colour is **per cell, last writer wins**, so per-node colour —
  the whole of deliverable 2 — is structurally unsatisfiable wherever a
  node shares a cell with an edge or a lane; `ctx.print` is a third path
  into a buffer whose apparent safety comes from `ratatui-core`'s
  undocumented filtering rather than from `Safe` (a probe printing
  U+202E, U+200B and U+0007 came back sanitized by ratatui, which means
  AC-safe-1 would pass **by accident** and a ratatui bump could regress
  terminal safety with a green suite); a braille field makes AC-draw-1's
  "lanes leave the rail, run parallel, and rejoin" an unreviewable
  golden; float world-space clipping branches fight the exact coverage
  gate; and at 80×24 the pane is five inner rows, where sub-cell
  smoothing is invisible and a one-cell `●` is a truer circle than any
  braille approximation. The ruling is held by a source assertion, not by
  memory.

- **The grammar, transliterated.** One rail row; segments joined by
  `──→`; a plain phase as a single rail node, filled when current and
  hollow otherwise; a fork leaving the rail at `┤` and **rejoining at
  `├`** with members symmetric about the rail at offset `k − (n−1)/2`;
  member labels on their own lanes and single-column labels on the shared
  step baseline; phase names on one baseline with `▸` from
  `render::keeps_phase` — **called, never reimplemented** — and `×N` only
  when `visits > 1`, matching the console rather than today's
  unconditional `×1`.

- **A pure `plan` / dumb `paint` split, inside `tui.rs`.**
  `plan(phases, lens, selection, width, height) -> Plan` is pure owned
  integer geometry; `paint(&Plan, tick, animate) -> Vec<Line>` walks it
  and emits spans through the **existing** `span()` constructor. The
  split is bought for the coverage gate, which the framing calls the
  tightest practical constraint: every degenerate branch — a zero-node
  column, a fork taller than its lane budget, a rail wider than the pane
  — is `assert_eq!` against a small struct rather than a substring search
  over a rendered buffer. `plan` returns a layout that **fits by
  construction**, so `paint` contains no clipping branch. The graph stays
  in `tui.rs` rather than moving to `tui/graph.rs`, because `SOURCE` is
  `include_str!("../tui.rs")` and a new file would silently move the
  renderer out of reach of the read-only and sanitization greps.

- **Node look is one closed-set table of seven classes**, each carrying a
  **(colour, four-glyph ramp)** pair: `Visited`, `Current`, `Finished`,
  `Active`, `Failed`, `Park` — deliverable 2's vocabulary verbatim — plus
  one named `Unknown` fallback. Pairing colour with a glyph is what makes
  `Active` distinguishable from `Finished`, and selection from the
  current-phase marker, on a `NO_COLOR` terminal and with animation off;
  a `TestBackend` assertion that two things differ only by `fg` would
  pass while the requirement was violated in the only place it matters.
  `render::tone` is deliberately **not** reused: it maps
  `awaiting_operator` to `Quiet`, while the graph needs **park** as a
  distinct yellow class, and widening `tone` would move `brokkr runs`'
  goldens. One classification per question.

- **An unrecognised `summary.status` renders quiet, never live.** This is
  a deliberate, named divergence from `ui.html`, which maps an unknown
  status to `on-phosphor` (green, still). The terminal declines to guess,
  in the manner of `render::tone`'s own `_ => Tone::Quiet` and per
  decision 0001. The pulse gate is `status == "running"` on both
  surfaces, so an unknown status never animates on either. Every closed
  set gets a named fallback arm with a test that executes it — no
  `unreachable!()` over a `String` that crosses `VIEW_VERSION`, because
  that is either a panic in an operator's terminal or an uncoverable
  line. `Phase.visits` is `u64` and its marker is clamped.

- **The pulse adds no clock.** `drive()` already redraws unconditionally
  every `TICK = 250ms` and already carries `tui.ticks`, so the whole of
  deliverable 3 is one `const PULSE_TICKS`, one four-frame ramp, and
  `pulse(tick, live, animate) -> usize` — total over `usize` by modulo,
  two arms, taking no store and no clock. `live` is a model field
  (`summary.status == "running"`, `node.state == "active"`). The glyph is
  an **array index**, not a branch, so non-pulsing classes need no
  special case. The honest statement is stronger than "bounded": the
  animation adds **zero** additional wakeups and zero additional draws.
  A design that added its own animation timer would have *created* the
  idle cost the requirement exists to prevent. **The pulse touches glyph
  and style only, never a position**, so geometry stays independent of
  the tick and no layout assertion becomes tick-dependent. The stated
  no-animation rule: *animation is enabled exactly when colour is* —
  `NO_COLOR=1`, `TERM=dumb` or a non-tty all yield a still graph, through
  the existing pure `color_enabled`, with no new flag and no new env var.

- **Degradation is stated in two axes.** Height: three named modes from
  one predicate — `Full` (rail, symmetric lanes, name baseline), `Rail`
  (rail and names; forks collapse to `⑂n`), `Compressed` (one inline
  row). This axis is the one the framing never mentions, and it is the
  one that bites: at `MIN_HEIGHT = 12` the graph pane is roughly **one
  inner row**, so a fork does not fit at any terminal `refuse()` admits.
  Every mode that cannot draw lanes still emits `⑂n`, so **a graph that
  is too small never lies about the shape of the run**, and a fork that
  overflows its lane budget shows `+k` rather than dropping a member.
  Width: the rail is a **window derived each frame from the cursor** with
  `‹`/`›` elision marks — the console's answer was `overflow-x: auto`;
  ours is the arrow keys, reused, which is why no scroll-offset state
  exists to go stale. Within a segment, skeleton before text: labels
  truncate with `…`, and the rail, arrow, corners and rejoin are never
  dropped. `MIN_WIDTH` stays 60 and the 34/33/33 pane split is untouched.

- **All widths route through one `width_of` = `span(text,
  plain()).width()`.** `Safe::width()` is `chars().count()` and reports
  **6** for `設計フェーズ` where ratatui draws **12**. That is harmless
  today because `Table` and `Paragraph` do the measuring; **the rail is
  the first pane in `tui.rs` that places its own x positions**, so it is
  the first that can be lied to by a non-hostile phase name. One
  measurement system, and it is ratatui's own, taken on the sanitized
  text.

- **Navigation adds one field and two keys.** `Key::Left`/`Key::Right`
  move the rail; `Up`/`Down` in the graph pane move a **subordinate**
  lane cursor `Tui.node: Option<String>`; both go through the existing
  `move_to`, so wrap-around and the empty-list case are already specified
  and already tested. `cursor[0]` keeps holding the phase name, and
  `enter`'s `(Level::Run, 0)` arm is **unchanged byte-for-byte** — an
  overloaded cursor would silently break the `/` filter, `selected()` and
  the scope key. The lane cursor is a stable **key**, not a clamped
  index, because 0014's whole selection design is "stable key resolved
  against fresh models, retaining nothing" — which makes "survives a
  refresh" and "clears when its subject vanishes" the *absence of code*,
  where a retained index would be exactly the retained position 0014
  removed. Selection is the TUI's existing `REVERSED` idiom; current is
  the filled coloured rail glyph. One bounded, named consequence: the
  existing headless cases that pressed `Up`/`Down` to move the rail are
  re-pointed at `j`/`k`, which assert the same movement — no case
  asserting a **scope** is touched.

- **`brokkr-view` gains one test and nothing else.** No field, no
  rendering concern, no `VIEW_VERSION` move, no `/api` shape change. The
  test pins that phase names are unique within a `RunView` — the
  invariant the `visits` fold already holds, and which the graph now
  depends on for selection identity. Without it, two segments could share
  a cursor key and `Enter` would scope the wrong one, invisibly. Intake's
  open question therefore resolves to **option (a)**: branch in `tui.rs`
  on `phase.current` + `summary.status`. Option (b) — a phase-level
  `Phase.state_class` — is a cross-surface contract change bought to save
  match arms in one renderer, and would either force a `ui.html` edit the
  non-goals forbid or leave the console computing the same fact anyway.

- **The safety discipline is unweakened.** `paint` emits spans through
  `span()` only, so `tui.rs` keeps **exactly two** constructors reaching a
  widget and the source-level discipline test is **unamended** — it gains
  assertions and loses none.

Design artifacts:

- [specs/tui-graph/spec.md](../../../specs/tui-graph/spec.md) — WHAT and
  WHY: the governing invariant, what the two positions settled (six
  agreements adopted, seven rulings for robustness, five for simplicity,
  three reconciliations), the behaviour in nine sections, and 24
  acceptance criteria.
- [specs/tui-graph/plan.md](../../../specs/tui-graph/plan.md) — HOW: the
  files touched, the four-layer partition, the `Plan` type, the coverage
  table naming the executor for every arm before code is written, the
  three safety properties, and the risk register.
- [specs/tui-graph/tasks.md](../../../specs/tui-graph/tasks.md) — sixteen
  ordered tasks in six movements, each paired with the test that proves
  it.

## Impact

- **Surfaces**: still three renderers over one derivation. 0013's
  invariant holds by construction — this change adds no derivation and
  modifies `brokkr-view`'s models not at all.
- **Existing behaviour**: `brokkr runs`, `inspect`, `watch` and `ui` are
  unchanged. `render::graph_block` and every golden stay byte-identical;
  the `/api` response shape is unchanged; `ui.html` is the reference
  being matched, not a file being edited.
- **Operator boundary**: unchanged. The TUI stays read-only (decision
  0003); `tui.rs` continues to name no store, no runtime and no journal
  write, and the graph renderer stays inside the file those greps cover.
- **Dependency tree**: unchanged. No new dependency and **no manifest
  edit** — verified under the pin, not assumed.
- **Frozen contracts, the corpus, `policy/phase-machine.json` and
  `reference/`**: untouched.
- **Runtime cost**: unchanged. The animation adds zero wakeups and zero
  draws, because the shell's cadence already exists and is already
  bounded by a named `const`. An idle TUI costs exactly what it costs
  today.
- **Known divergences from the console, flagged not smuggled**: an
  unrecognised `summary.status` renders DIM in the terminal where the
  console renders it green-still (decision 0001's "never guess into a
  class"); the pulse is a four-frame glyph ramp rather than an SVG halo,
  because one cell is the resolution limit; and a long rail shows a
  window rather than the whole run, which is what the console's own
  horizontal scrolling also means.
- **Known limit, named**: `NO_COLOR` stills the animation without
  stilling the TUI's colour, because `tui.rs` never consulted
  `color_enabled` at all. Making TUI colour honour `NO_COLOR` is coherent
  and probably right; it threads a flag through every style function and
  doubles the arms under the exact gate, and it is not in this framing.
  Left as named follow-on work.
- **CI**: the coverage gate gains a geometry function whose every arm has
  a named executor, and the discipline test gains a no-canvas assertion.
- **Coordination**: two other runs (the agent library, composable
  recipes) and a trail-reader overlay are landing in `tui.rs`
  concurrently. A **mechanical rebase is expected**, and the collision
  surface is deliberately confined to intake's four named points:
  `Key` + `from_key`, the `(Level::Run, 0)` slots of `keys_for`/`step`/
  `apply`, `footer_for` + `HELP` (whose array length changes — take the
  mechanical merge and keep both lines), and the constants block. The
  graph's volume sits behind `plan`/`paint` in one contiguous section
  that rebases as a block move. Moving it to a new file would dodge the
  rebase and drop the safety greps; that trade is refused.
</content>
