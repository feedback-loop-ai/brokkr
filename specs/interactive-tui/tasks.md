# Tasks: `forge tui`

**Feature slug**: `interactive-tui`
**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Ordered; each task names the test that proves it. `AC-n` refers to
spec.md's `## Acceptance Criteria`. Every task lands with its tests in
the same commit — the coverage gate is the point of this feature, not an
afterthought at the end.

## Movement 0 — the shared pieces, before the TUI exists

These land first because they are edits to code that already has
goldens: if hardening `Safe` or splitting `tone()` moves an existing
byte, it must surface now and not inside a large new file.

- [ ] **T1 — `Safe` strips bidi and zero-width formatting characters.**
  In `render.rs`, extend `Safe::new`'s filter with an explicit,
  enumerated set — U+200B–U+200F, U+202A–U+202E, U+2060–U+2064,
  U+2066–U+2069, U+FEFF — and no Unicode-properties dependency. Update
  the module doc's terminal-safety paragraph to say so.
  *Proven by*: AC-11 — a `render/tests.rs` case where a seat label
  carrying U+202E renders with the override stripped and the line in
  source order; **and** every existing `runs`/`inspect`/`watch` golden
  byte-identical.

- [ ] **T2 — `tone()` splits out of `status_code`.**
  `pub(crate) enum Tone { Good, Bad, Live, Quiet }` and
  `pub(crate) fn tone(status: &str) -> Tone` carry the table verbatim;
  `status_code` becomes `Tone -> ANSI` and keeps its exact mapping. One
  classification in the crate.
  *Proven by*: AC-10 — the existing colour test unchanged and passing;
  goldens byte-identical; a unit test that `tone` returns `Quiet` for an
  unknown status (0001: never guessed into one of the four).

- [ ] **T3 — the scope predicate is shared and de-duplicated.**
  `keeps_participant` and `keeps_row` become `pub(crate)`; the phase
  filter written inline in `graph_block` (`render.rs:396–403`) is
  **extracted** as `pub(crate) fn keeps_phase(lens: Option<&Lens>, phase:
  &Phase) -> bool` and called from `graph_block`.
  *Proven by*: AC-17 — `inspect` goldens (scoped and unscoped)
  byte-identical, and a unit test of `keeps_phase` in both lens states.
  The crate now has one fewer copy of the predicate than before this
  slice.

- [ ] **T4 — `session_turns` factors out of `session_transcript`.**
  `pub(crate) fn session_turns(id: &str) -> Option<(Vec<Turn>, bool)>`
  carries **validation, location and parse together**; the `json!`
  construction lifts into a short converter in `handle`. Two plain
  structs, no `serde` derive, no new dependency edge.
  *Proven by*: AC-16 — the existing `/api/session` body assertions in
  `ui/tests.rs` unedited and passing (byte-identical body), plus a unit
  test that a traversal-shaped id (`../../etc/passwd`) is rejected by
  `session_turns` itself, not by the HTTP layer.

- [ ] **T5 — `WATCH_TRANSIENT_FRAMES` becomes `pub(crate)`.**
  No behaviour change; the TUI reaches it as
  `crate::WATCH_TRANSIENT_FRAMES`.
  *Proven by*: AC-9 — `watch`'s existing transient/persistent tests still
  green.

## Movement 1 — the dependency and the walking skeleton

- [ ] **T6 — `ratatui` joins the workspace.**
  One `[workspace.dependencies]` entry,
  `{ version = "0.30", default-features = false, features = ["crossterm"] }`,
  referenced by `crates/forge-cli/Cargo.toml` only. `Cargo.lock`
  committed. `forge-cli` declares no cargo features.
  *Proven by*: AC-18 — `cargo tree -p forge-view` shows no terminal crate
  and `crates/forge-view/Cargo.toml` still lists exactly `forge-core`,
  `serde`, `serde_json`; `cargo build --workspace --locked` green; the
  RustSec audit job passing over the added tree.

- [ ] **T7 — `Cmd::Tui`, `mod tui`, and the injected dispatch arm.**
  `forge tui [--run <id>] [--db <path>]` with `--db` defaulting to
  `.forge/forge.db`. `run_with` gains a `run_tui` parameter exactly as it
  has `serve_ui`. Re-check `main` for a shared run-id prefix/`latest`
  helper (`slice-run-selectors`) and route `--run` through it if present;
  otherwise verbatim. Never duplicate resolution.
  *Proven by*: AC-1 — a `src/tests.rs` case driving the arm through
  `run_with` with an injected `run_tui`, and a subprocess `forge tui
  --help` listing the verb.

- [ ] **T8 — `refuse()` and the startup gate.**
  `refuse(is_tty, size, db_is_file) -> Option<String>` naming **both**
  `forge inspect` and `forge watch`; `run()` calls it before anything
  else and `bail!`s. `Store::open` is unreachable when the db is not an
  existing file.
  *Proven by*: AC-2 — unit tests of all `refuse` branches with literal
  values; a subprocess test in `tests/machine_proof.rs` running `forge
  tui --db <nonexistent>` asserting exit 1, both verbs named on stderr,
  **and that no file, directory, `-wal` or `-shm` was created**.

- [ ] **T9 — `TerminalOps`, `production_ops()`, the RAII guard, and the
  panic hook.**
  Five `fn`-pointer fields whose values are crossterm's own function
  items — never wrappers, never closures. A guard whose `Drop` leaves the
  alternate screen, shows the cursor and disables raw mode, ignoring
  errors and never panicking. `install_panic_hook(restore: fn())`
  restores **first**, chains to the previous hook **second**, is
  idempotent, and is uninstalled on normal exit. No `std::process::exit`
  anywhere on this path.
  *Proven by*: AC-12 — a test constructing `production_ops()` (covering
  its line and function without invoking a pointer); `execute!` against a
  `Vec<u8>` asserting the alternate-screen bytes; and, under a `static
  Mutex`, installing the hook with a recording no-op restore,
  `catch_unwind`ing a deliberate panic, asserting the recorder fired and
  the previous hook ran, and restoring the prior hook.

## Movement 2 — the pure state machine

- [ ] **T10 — `Key` and `from_crossterm`.**
  Our own enum; the translation filters `KeyEventKind::Press`, binds
  `Ctrl+C` and `q` to `Quit`, and returns `None` for mouse, paste, focus
  and resize by named arms.
  *Proven by*: AC-14 — unit tests over constructed `Event` values
  including a `Release` event yielding `None` (the Windows
  double-keystroke), `Ctrl+C` yielding `Quit`, and each ignored event
  kind.

- [ ] **T11 — `move_to` / `index_of` / `keys_for`.**
  The single movement function: `Up`, `Down` with wrap-around at both
  ends, `Top`, `Bottom`, `PageUp`, `PageDown`. `keys_for` builds the
  filtered key list over **sanitized** labels.
  *Proven by*: AC-7 — unit tests for wrap at both ends, `g`/`G`, paging
  past both ends, an empty list, and `index_of` returning `None` for a
  vanished key.

- [ ] **T12 — `apply()`: the Enter/Esc ladder.**
  Every `Enter` rung and every `Esc` rung of spec §3, `Backspace`
  ascending via rungs 3/5 only, `Esc` never quitting, `Tab` as
  `(pane + 1) % panes_at(level)`, and `assign_run` clearing scope, seat,
  filter and cursors.
  *Proven by*: AC-4, AC-5 — one headless test per rung, including
  descend RUNS→RUN→PARTICIPANT and ascend back via **both** `Esc` and
  `Backspace`, and `--run`'s initial state reaching RUNS on `Esc`.

- [ ] **T13 — `apply()`: filtering.**
  `/` enters typing mode; each character narrows the focused list
  incrementally; `Backspace` deletes one char; `Esc` leaves typing mode
  and clears the filter. The filter text goes through `Safe`.
  *Proven by*: AC-6, AC-11 — a test asserting the key list shrinks per
  keystroke; a test that a filter hiding the scoped subject leaves
  `scope` intact (display fact ≠ vanish condition); a test that a pasted
  escape sequence in the filter is sanitized before it is echoed.

- [ ] **T14 — `apply()`: scoping and vanish.**
  `render::Scope` held directly; `lens_for(...).ok().flatten()` resolved
  per frame; `None` clears the scope. A second selection replaces the
  first.
  *Proven by*: AC-6, AC-7 — two consecutive `Enter`s on different phases
  leave exactly one scope; `Esc` clears it; and a test that applies the
  same state against **two different `RunView`s**, proving selection
  survives a refresh that changes the list and clears itself when the
  subject vanishes from the unfiltered model.

- [ ] **T15 — `footer_for()`.**
  Pure; a different string per (level, pane, typing, help), naming
  `Enter scope` at RUN·graph and `Enter open` on an already-scoped seat.
  *Proven by*: AC-8 — a unit test asserting two different states produce
  two different strings, so a constant footer cannot pass.

## Movement 3 — rendering

- [ ] **T16 — `cell()` / `span()` and the sanitized widget layer.**
  Exactly one `fn cell(text: &Safe) -> Cell` and one `fn span(text:
  &Safe, style: Style) -> Span`; no `&str`-taking constructor anywhere in
  the widget layer.
  *Proven by*: AC-11 — a source test asserting no `Cell::from(`/
  `Span::raw(`/`Row::new(vec![` over a bare `&str` appears in `tui.rs`.

- [ ] **T17 — `draw_runs` and `draw_run`, `Backend`-generic.**
  Bordered tables; header `id · status · phase · seq · age · feature` and
  `participant · status · attempts · turns · cost · activity`; the status
  cell tinted through `tone()`; the graph as the `⑂`/`→` tree,
  uncoloured; the trail as a paragraph; focused pane = bright border.
  *Proven by*: AC-3, AC-10 — `TestBackend` buffer assertions on both
  header rows, on a working seat showing `activity.tool` +
  `target_short` and a concluded one showing `activity.text`, and that
  nothing in the render path names `CrosstermBackend`.

- [ ] **T18 — the hostile-label render test.**
  A `Participant.label` carrying `\x1b]0;pwn\x07`, `\r` and U+202E.
  *Proven by*: AC-11 — the `TestBackend` buffer contains no `\x1b` and no
  `\r`, the text is in source order, **and the neighbouring column
  starts at its expected x** (the width half of the claim, not only the
  strip half).

- [ ] **T19 — `draw_participant`.**
  Checkpoints pane and transcript pane; `Participant.terminal_line`
  rendered as the model built it, **unconditionally**, including its
  absence mark when `session_id` is `None`; the transcript's `truncated`
  flag displayed; a one-sentence pane when `session_turns` returns
  `None`.
  *Proven by*: AC-3, AC-16 — buffer assertions for a seat with a session
  and one without; a test that no `claude --resume` string is constructed
  in `tui.rs` (grep), only `terminal_line` consumed.

- [ ] **T20 — `draw_footer`, `draw_help`, `draw_too_small`.**
  The footer at the bottom of every frame; `?` overlaying `Clear` + a
  centred paragraph; a centred too-small frame naming `forge inspect` and
  `forge watch` with `q`/`Ctrl+C` live.
  *Proven by*: AC-8, AC-13 — `TestBackend` at two different levels
  producing two different footer lines; a `TestBackend` sized below the
  minimum producing the degradation frame and not a corrupted one.

## Movement 4 — the shell and liveness

- [ ] **T21 — `drive()` over injected sources.**
  `drive<B: Backend>(&mut Terminal<B>, &TerminalOps, &mut dyn FnMut() ->
  Result<Snapshot>, max_iterations)`; `poll(remaining_tick)` so latency
  is bounded by the keypress; `Ctrl+C`/`q` quits; `r` forces a refresh.
  *Proven by*: AC-9, AC-12 — a fully headless run under `TestBackend`
  with a scripted key source through every navigation path; an error path
  forced by a refresh source returning `Err`, with the recorder showing
  `leave_raw` **before** the `Err` returns; the guard's `Drop` exercised.

- [ ] **T22 — the refresh source and its cadences.**
  RUN head compared on **both** seq and hash; RUNS refolded every
  `RUNS_REFRESH_TICKS` or on `r`; `state: fold(&events).ok()` so one
  unfoldable run keeps its row; transient errors render
  `WATCH_TRANSIENT_FRAMES`'s existing sentence with keys still live, and
  a persistent one returns `Err` past the constant. `Cmd::Runs` is not
  changed.
  *Proven by*: AC-9 — a scripted source whose head moves only in hash
  (equal seq) still triggering a redraw; a run whose events do not fold
  appearing with the absence mark; N transient errors drawing the
  sentence and the (N+1)th returning `Err`; a test that keys are handled
  during unreadable frames.

## Movement 5 — the read-only proofs

- [ ] **T23 — the headless read-only proof.**
  In `src/tui/tests.rs`: export the NDJSON and hash the db directory tree
  (relative path, length, bytes); drive the state machine through every
  navigation path; export and hash again; byte-compare **both**.
  *Proven by*: AC-15 — the test itself, plus a source test asserting
  `tui.rs` names no `Store`, `forge_runtime`, `append_next` or
  `create_run`. (`forge-cli` has no `[lib]` target, so this half cannot
  live in `tests/`.)

- [ ] **T24 — the subprocess read-only proof.**
  In `tests/machine_proof.rs`, on the `Workspace::exported_events` /
  `forge export` idiom: export and tree-hash, run the real `forge tui`
  binary against a real run's db, export and tree-hash again,
  byte-compare both.
  *Proven by*: AC-15, AC-2 — the test itself, which also covers `run()`'s
  environment reads and refusal branches deterministically (a
  subprocess's stdout is always a pipe).

## Movement 6 — admission

- [ ] **T25 — docs.**
  `ARCHITECTURE.md` §"Operating surface" and `README.md`'s
  install/operate block and readouts paragraph each gain `forge tui`:
  one sentence on what it is for, one on its read-only boundary.
  *Proven by*: AC-20 — the diff, and any existing docs test still green.

- [ ] **T26 — the exact coverage gate, and the rest of admission.**
  `scripts/coverage-exact.sh` reporting literal nonzero 100%
  line/branch/function equality; `cargo test --workspace`; `cargo clippy
  --workspace --all-targets --all-features` warning-free; `cargo fmt
  --check`; RustSec audit; `forge runs`/`inspect`/`watch`/`ui` output
  unchanged; `crates/forge-view` untouched and `VIEW_VERSION` unmoved.
  *Proven by*: AC-19, AC-20 — the gate's own output. Run it once at the
  end of Movement 1 on the walking skeleton, so the gate is proven on a
  small surface before the surface is large.
