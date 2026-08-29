# Tasks: `tui-graph`

**Feature slug**: `tui-graph`
**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Ordered; each task names the test that proves it. `AC-*` refers to
spec.md's `## Acceptance Criteria`. Every task lands with its tests in
the same commit — AC-gate is the point of this feature, not an
afterthought at the end.

Movements 1–3 are pure and touch no shared code, so they can be written
before the concurrent rebase lands. Movement 4 is where every expected
collision is.

## Movement 0 — the invariant the graph will depend on

- [x] **T1 — pin phase-name uniqueness in `forge-view`.**
  One test asserting that no two `Phase`es in a `RunView` share a `name`,
  over a fixture whose journal revisits a phase. **No `lib.rs` change, no
  new field, `VIEW_VERSION` unmoved.** The graph is about to use
  `Phase.name` as a selection key; this converts a silent cross-crate
  coupling into a named one.
  *Proven by*: AC-view-1.

## Movement 1 — the pure vocabulary

- [x] **T2 — `width_of`.**
  `fn width_of(text: &str) -> usize` = `span(text, plain()).width()`,
  with the doc comment saying why `Safe::width()` is not it. Every width
  in the feature comes from here.
  *Proven by*: AC-safe-2 — a unit case asserting `width_of("設計フェーズ")
  == 12` while `Safe::new("設計フェーズ").width() == 6`, and ASCII parity.

- [x] **T3 — `Class` and `look`.**
  The seven-class enum and `fn look(class: Class) -> (Style, [&'static
  str; 4])`, plus the two classifiers that feed it:
  `class_for_phase(current: bool, status: &str)` and
  `class_for_node(state_class: &str)`, each an allowlist over a
  closed-set key with a **named fallback arm** — never `unreachable!()`.
  *Proven by*: AC-look-1 (one case per class, including `Unknown` from an
  unrecognised `state_class`), AC-look-2 (`current` + an unrecognised
  status ⇒ `Unknown`, not `Current`), AC-look-3 (`Active`'s still glyph
  differs from `Finished`'s).

- [x] **T4 — `pulse` and its constants.**
  `PULSE_TICKS` and `PULSE_FRAMES` in the constants block beside `TICK`;
  `fn pulse(tick: usize, live: bool, animate: bool) -> usize`, two arms,
  total by modulo. The glyph is `look(class).1[pulse(..)]` — an index,
  not a branch.
  *Proven by*: AC-anim-1 (a full period, the `PULSE_TICKS` boundary,
  `usize::MAX`), AC-anim-2 (`!live` ⇒ 0 at every tick), AC-anim-3
  (`!animate` ⇒ 0 at every tick).

## Movement 2 — geometry

- [x] **T5 — `Mode` and `mode_for`.**
  `mode_for(inner_rows, max_lane_span) -> Mode` over `Full` / `Rail` /
  `Compressed`, per spec §4's table.
  *Proven by*: AC-mode-1 — one call per row budget, including the exact
  budgets at 80×24 and at `MIN_WIDTH`×`MIN_HEIGHT`.

- [x] **T6 — `Plan` and `plan`.**
  The types in plan.md, and
  `fn plan(phases, lens, cursor, node, width, height) -> Plan`. It calls
  `render::keeps_phase` for `scoped` (never reimplements it), clamps
  `visits` and emits `Some` only when `> 1`, derives the rail window from
  the cursor with no stored offset, truncates labels through `width_of`,
  and returns a layout that **fits by construction**.
  *Proven by*: AC-draw-3 (the `×N` rule and the `u64::MAX` clamp),
  AC-draw-4 (`▸` follows `keeps_phase`; every phase is listed),
  AC-mode-2 (fork overflow yields `+k`, no member dropped),
  AC-mode-3 (the window contains the cursor; `‹`/`›` set correctly for
  elided-left, elided-right, both and neither),
  AC-anim-4 (`plan` output identical at two tick values),
  plus direct cases for empty `phases`, a zero-node column, a one-node
  column, and a label too wide for its segment.

## Movement 3 — paint

- [x] **T7 — `paint`.**
  `fn paint(plan: &Plan, tick: usize, animate: bool) -> Vec<Line<'static>>`,
  emitting spans through the existing `span()` **only**. No clipping
  branch; no new widget constructor. Selection is `Modifier::REVERSED` on
  the name or node label; current is the filled coloured rail glyph.
  *Proven by*: direct `Vec<Line>` inspection of spans and styles for each
  mode, and the `TestBackend` cases in T8.

- [x] **T8 — `draw_graph` is rewired.**
  `draw_graph` becomes `plan` → `paint` →
  `Paragraph::new(..).block(pane("graph", tui.pane == 0))`. The 0013 tree
  body is deleted.
  *Proven by*: AC-draw-1 (a fork/join frame through `TestBackend`: lanes
  leave the rail, run parallel, and **rejoin** before the next arrow),
  AC-draw-2 (a plain phase is one rail node; single-node columns are
  arrowed rail steps), AC-mode-1's three rendered modes, AC-width-1 (full
  grammar at 80 columns; legible and uncorrupted at `MIN_WIDTH = 60`),
  AC-safe-1 (a phase name carrying U+202E, U+200B and control characters
  renders inert **and** the following segment starts at its expected x).

## Movement 4 — navigation and the shared edits

Every expected rebase collision is in this movement. Re-check `main` for
the trail-reader overlay before starting it.

- [x] **T9 — `Key::Left` / `Key::Right`.**
  Two enum variants, two `from_key` arms (`KeyCode::Left`,
  `KeyCode::Right`).
  *Proven by*: two key-translation cases, and the existing
  `from_crostterm`/`from_key` cases still green.

- [x] **T10 — the lane cursor.**
  `Tui.node: Option<String>`; `lane_keys(view, cursor)` in draw order;
  `apply` routes `Left`/`Right` to the rail (clearing `node`) and
  `Up`/`Down` in the graph pane to `node`, both through the existing
  `move_to`. Outside the graph pane, `Left`/`Right` are a **named**
  no-op. `assign_run` clears `node`. `enter` is **not edited**.
  *Proven by*: AC-nav-1 (rail and lane movement headlessly, including
  wrap-around at both ends of each), AC-nav-2 (`Enter` scopes the phase
  whatever `node` says; the enter/escape ladder's scope assertions
  unchanged), AC-nav-3 (a vanished phase clears; a vanished node
  highlights nothing and does not panic; a persisting selection
  survives), AC-nav-5 (`Left`/`Right` elsewhere change no state).

- [x] **T11 — existing `Up`/`Down`-at-graph cases re-pointed.**
  The headless cases that pressed `Key::Up`/`Key::Down` at
  `(Level::Run, 0)` to move the rail now press `j`/`k`, which assert the
  same movement. **No case asserting a scope is touched.**
  *Proven by*: the re-pointed cases green, and AC-nav-2's byte-for-byte
  scope assertions untouched.

- [x] **T12 — selection is visibly not the current phase.**
  *Proven by*: AC-nav-4 — asserted on a frame where the selected phase is
  not the current one **and** on one where it is, both on the
  glyph/modifier channel rather than on `fg`.

- [x] **T13 — footer and help.**
  `footer_for`'s `(Level::Run, 0)` arm gains `←→ rail · ↑↓ lanes`;
  `HELP` gains one line and its array length changes. Take the mechanical
  merge with the reader overlay and keep both lines.
  *Proven by*: the existing footer-differs-by-context case, extended to
  assert the graph arm names the arrow keys.

- [x] **T14 — `animate` is plumbed.**
  `Tui.animate: bool`, a new `animate` parameter on `tui::start`, and
  `render::Style::detect().color` at the single `run_tui` call site —
  the same line kind as the existing `is_terminal()` argument.
  *Proven by*: AC-anim-3 — tests set `Tui.animate` directly and every
  tick renders the still frame, with no environment touched.

## Movement 5 — the proofs

- [x] **T15 — the discipline test.**
  Add the no-canvas assertion. Confirm `Cell::from(` and `Span::styled(`
  are still exactly one each, `Span::raw(` still zero, and the
  read-only greps unchanged. **The test gains an assertion and loses
  none.**
  *Proven by*: AC-safe-3.

- [x] **T16 — admission.**
  `cargo fmt`, `cargo clippy` (workspace, `-D warnings`),
  `cargo test --workspace`, the machine proof, the differential corpus,
  and `scripts/coverage-exact.sh`. Confirm `render::graph_block`'s
  goldens and every `runs`/`inspect`/`watch`/`/api` byte are unchanged.
  *Proven by*: AC-gate and AC-green.
</content>
