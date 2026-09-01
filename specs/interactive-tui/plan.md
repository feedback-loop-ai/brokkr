# Implementation Plan: `brokkr tui`

**Feature slug**: `interactive-tui`
**Spec**: [spec.md](spec.md) · **Tasks**: [tasks.md](tasks.md)
**Ruling**: `docs/decisions/0014-interactive-tui.md`

This is the HOW. The WHAT and the rulings that settled the panel's
disputes are in [spec.md](spec.md) and are not re-argued here.

## Module layout

One new file and its conventional test file, plus targeted edits to
three existing ones.

```
crates/brokkr-cli/src/tui.rs         new   the state machine, the draw path, the shell
crates/brokkr-cli/src/tui/tests.rs   new   pure-core unit tests, TestBackend render tests,
                                          the headless read-only proof
crates/brokkr-cli/src/render.rs      edit  Safe hardened; tone() split out; keeps_phase
                                          extracted; keeps_participant/keeps_row pub(crate)
crates/brokkr-cli/src/render/tests.rs edit hostile-bidi case
crates/brokkr-cli/src/ui.rs          edit  session_turns() factored out of session_transcript
crates/brokkr-cli/src/ui/tests.rs    edit  /api/session body still byte-identical
crates/brokkr-cli/src/main.rs        edit  mod tui; Cmd::Tui; dispatch arm;
                                          WATCH_TRANSIENT_FRAMES -> pub(crate)
crates/brokkr-cli/tests/machine_proof.rs edit subprocess refusal + read-only proof
crates/brokkr-cli/Cargo.toml, Cargo.toml, Cargo.lock  edit  ratatui
ARCHITECTURE.md, README.md          edit  forge tui in the surfaces list
```

`render.rs` is 534 lines and `ui.rs` is 396; a `tui.rs` of 700–900 is
squarely in family. No module in this crate has ever been split, and the
first split should be forced by size rather than anticipated. A
`tui/{state,keys,draw,term}.rs` tree would make every crossing item
`pub(crate)` — real API surface in a binary crate that has none — and
force a reviewer to hold five files open to answer "what does `Esc` do".

## The partition

```
pure core        apply(&mut Tui, &Views, Key) -> Flow          { Continue, Quit }
                 refuse(is_tty, size, db_is_file) -> Option<String>
                 index_of / move_to / keys_for / footer_for / from_crossterm / tone
generic render   draw<B: Backend>(&mut Frame, &Tui, &Views)    nothing names CrosstermBackend
bounded shell    drive<B: Backend>(&mut Terminal<B>, &TerminalOps,
                                   &mut dyn FnMut() -> Result<Snapshot>,
                                   max_iterations) -> Result<ExitCode>
impure entry     run(db, run) -> Result<ExitCode>              a handful of statements
```

### State

```rust
pub(crate) struct Tui {
    level: Level,                  // Runs | Run | Participant
    run: Option<String>,           // RunRow.run_id
    seat: Option<String>,          // Participant.key
    scope: Option<render::Scope>,  // RUN level; exclusive by being one field
    cursor: [Option<String>; 3],   // one key per pane slot; cleared on level change
    offset: usize,                 // paragraph-pane viewport
    pane: usize,                   // 0..panes_at(level)
    filter: String,
    typing: bool,
    help: bool,
    status: Option<String>,        // "the journal is not readable right now: …"
    ticks: usize,                  // drives RUNS_REFRESH_TICKS
}
```

Owned scalars only. **No `brokkr-view` model is retained.** `Views` (a
`Snapshot` holding a `RunsView` and an `Option<RunView>`) is produced by
the refresh source, borrowed for the frame, and dropped. That single
decision deletes `Clone`/`PartialEq`/`Deserialize` derives, a snapshot, a
diff routine and any `brokkr-view` change from the design.

`fn assign_run(&mut self, id: String)` is the only writer of `run` and
clears `scope`, `seat`, `filter`, `typing` and every cursor — the
invariant a run-qualified selection path would otherwise buy with a type.

### Keys

```rust
pub(crate) enum Key { Up, Down, Top, Bottom, PageUp, PageDown, Tab,
                      Enter, Escape, Backspace, Char(char), Slash,
                      Refresh, Help, Quit }
pub(crate) fn from_crossterm(event: Event) -> Option<Key>;
```

`from_crossterm` filters `KeyEventKind::Press` (Windows sends releases),
maps `Ctrl+C` and `q` to `Quit`, and returns `None` for mouse, paste,
focus and resize by named arms. It is a pure function and unit-tested
with constructed events — no terminal.

### Movement

```rust
fn keys_for(tui: &Tui, views: &Views) -> Vec<String>;   // filtered, sanitized labels
fn index_of(keys: &[String], cursor: &Option<String>) -> Option<usize>;
fn move_to(keys: &[String], cursor: &mut Option<String>, step: Step);
```

`move_to` is the only place wrap-around, `g`/`G` and paging exist, for
every list at every level. `index_of` returning `None` is how selection
clears itself; there is no re-anchoring pass and no per-level special
case. Filtering happens in `keys_for` (over sanitized labels) — scope
clearing is decided separately, against the *unfiltered* model, in the
per-frame `lens_for` resolution.

### Widgets

`Block`+`Borders`, `Table`+`Row`+`Cell`, `Paragraph` (`.scroll` for the
viewport panes), `Clear` (help overlay), `Layout::vertical`. Not `List`,
`Tabs`, `Gauge`, `Scrollbar`, `Flex` or `Sparkline`.

Five draw functions: `draw_runs`, `draw_run`, `draw_participant`,
`draw_footer`, `draw_help`, plus `draw_too_small`. The focused pane is
the one with a bright border — the only focus affordance, needing no
legend. Columns are `Constraint::Length` for the fixed fields and
`Constraint::Min` for the feature/activity text; ratatui clips.

Exactly two constructors reach a widget:

```rust
fn cell(text: &Safe) -> Cell;
fn span(text: &Safe, style: Style) -> Span;
```

No `&str`-taking constructor exists in the widget layer, so "did this one
get sanitized" is answerable by grep and a hostile string cannot reach a
buffer without a visible `Safe::new` at the call site.

## Coverage

`scripts/coverage-exact.sh` demands literal nonzero 100% of **lines,
branches and functions**, workspace-wide, with `coverage(off)` banned by
a `git grep` inside the script. This is designed for here, before any
code is written, using the three mechanisms the repo already has —
injected seams (`run_with(cli, serve_ui, …)`, `serve_listener(…, opener)`),
really running the impure thing with a bounded resource
(`serve(db, port, false).is_err()` against a bound port), and subprocess
execution of the binary (`env!("CARGO_BIN_EXE_forge")`, which is how
`fn main` and the dispatch arms are covered today).

| Impure item | How it is covered |
|---|---|
| `run()`'s environment reads (`is_terminal`, `size`, `db.is_file()`) and its refusal `bail!` | **Subprocess** `brokkr tui` in `tests/machine_proof.rs`. A subprocess's stdout is a pipe, so the refusal fires deterministically in CI *and* on a developer's terminal — an in-process assertion on `is_terminal()` is true locally and false in CI, which is exactly the flake to avoid. |
| `refuse(is_tty, size, db_is_file)` | Pure; all branches unit-tested with literal values. |
| `production_ops()` | A test **constructs** it — `let _ = production_ops();` — covering its line and its function record while invoking no pointer. |
| `enable_raw_mode` / `disable_raw_mode` / `poll` / `read` / `size` | Held as `fn`-pointer fields whose production values are **crossterm's own function items**, never our wrappers and never closures (either would be a counted function only production can execute). Their bodies live in crossterm's files, which `cargo llvm-cov` excludes from the workspace report. |
| `EnterAlternateScreen` / `LeaveAlternateScreen` / cursor `Hide`/`Show` | These take a writer. Point `execute!` at a `Vec<u8>` in a test: real code path, real coverage, no effect on the process's terminal — and assert on the bytes while there. |
| `drive()` — the loop, its quit arm, its error arm, its transient-busy arms | Fully executed under `TestBackend` with a scripted key source and a scripted refresh source, bounded by `max_iterations` exactly as `watch_loop` is. The error path is forced by a refresh source that returns `Err`; the recorder then shows `leave_raw` before the `Err` returns. |
| Every `draw_*`, every layout branch, the too-small arm | `Terminal<TestBackend>`; nothing in the render path names `CrosstermBackend`. |
| The terminal guard's `Drop` | Exercised by every `drive()` test; `Drop` is a counted function and is on the list deliberately. |
| The panic hook's closure | It is a counted function and must **execute**. `install_panic_hook(restore: fn())` takes restore as a `fn()`, so the test installs it with a recording no-op (a real restore would spray `\x1b[?1049l` through the test output), runs `catch_unwind(|| panic!(…))`, asserts the recorder fired, and puts the prior hook back. `set_hook`/`take_hook` are process-global, so this test holds a `static Mutex` shared with any other test that panics. |
| The `Cmd::Tui` dispatch arm | `run_with` gains a `run_tui` parameter exactly as it has `serve_ui`, so the arm is reachable from `src/tests.rs` without a terminal. |

Two structural notes that shape the code:

- **No `Action`/`Command` enum** between keys and state. Nothing replays,
  records or inspects actions, and an enum would mean two match arms per
  key instead of one — roughly thirty extra branches under a 100% branch
  gate for zero additional truth.
- **`brokkr-cli` has no `[lib]` target** (verified: `Cargo.toml` declares
  only `[[bin]] forge`). This corrects the framing's file list: an
  integration test in `tests/` **cannot** call the state machine, only
  spawn the binary. The headless read-only proof therefore lives in
  `src/tui/tests.rs`; `tests/machine_proof.rs` carries the subprocess
  half. Both are real proofs and together they are the requirement.

## Read-only enforcement

Three independent layers, cheapest first:

1. **Structural.** `tui.rs` imports nothing from `brokkr_runtime` and
   never names `Store`: the pure core and the draw path take view models,
   and the shell takes `&mut dyn FnMut() -> Result<Snapshot>`. A source
   test asserts the absence of the `Store`, `brokkr_runtime`,
   `append_next` and `create_run` tokens in `tui.rs`, the idiom the
   `ui.html` anti-drift test already established.
2. **Gated.** `run()` refuses when `!db.is_file()`, *before* `Store::open`
   is reachable — closing the write hole the panel found:
   `brokkr-store/src/lib.rs:76` creates the parent directory, creates the
   file, switches it to WAL (creating `-wal`/`-shm`), runs the migration
   DDL and INSERTs a meta row.
3. **Proven.** A directory-tree hash (relative path, length, bytes) of
   the `--db` parent **and** the NDJSON export, both taken before and
   after — headlessly around a full scripted drive, and by subprocess
   around the real binary.

## Refresh

The refresh source closure — constructed in `run()`, the only place
`Store` is named on this path — does:

```
open the store
RUN level:  head_hash(run) as (seq, hash); refold only when the tuple moves
RUNS level: refold every RUNS_REFRESH_TICKS ticks, or on `r`
            state: fold(&events).ok()   // NOT `?` — one bad run keeps its row
```

`fold(..).ok()` feeds `RunEntry.state: Option<&RunState>` and lets
`RunRow.status_known` produce the absence mark, per decision 0001. This
is choosing which input to hand the one derivation, not a fourth
derivation, and `Cmd::Runs` is **not** changed.

Input uses `poll(remaining_tick)` so latency is bounded by the keypress,
not by the tick — an operator holding `j` must not feel a 250 ms sleep.
The store read is bounded and off the blocking path so keys stay live
during unreadable frames; `WATCH_TRANSIENT_FRAMES` and its existing
sentence are reused verbatim, the constant promoted to `pub(crate)`.

## Dependency

```toml
# Cargo.toml [workspace.dependencies]
ratatui = { version = "0.30", default-features = false, features = ["crossterm"] }
# crates/brokkr-cli/Cargo.toml [dependencies]
ratatui = { workspace = true }
```

0.30.2 is current stable at design time; commit whatever `cargo update`
resolves, since the gate runs `--locked`. `default-features = false`
drops `widget-calendar` and `macros` from the audited tree; verify at
implementation that nothing used needs `layout-cache` or
`underline-color` and add the feature explicitly if so, rather than
reverting to defaults. crossterm is reached through ratatui's own
re-export; confirm the exact path (`ratatui::crossterm` in 0.30) at
implementation. **Do not declare any cargo feature on `brokkr-cli`** — the
coverage gate runs `--all-features` and would flip it on.

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| The exact coverage gate is discovered at the end | The table above is committed before code; each impure line names its seam. The first task is the walking skeleton *with* its coverage run, so the gate is proven on a small surface before the surface is large. |
| A closure sneaks into the `TerminalOps` fields | Written as a rule in spec §Risks and enforced by review plus a source test asserting `production_ops` assigns bare paths; a closure there is invisible at review and fatal at CI. |
| Hardening `Safe` changes a golden | Run the golden suites immediately after the `Safe` change and before anything else lands. No existing golden should contain a bidi or zero-width character; if one changes, either the extension is wrong or the golden is hostile — investigate, do not re-bless. |
| `tone()` refactor changes colour output | `status_code` keeps its exact ANSI mapping, expressed through `tone`. The one existing colour test is the proof; goldens are colourless by construction (`Style::plain`). |
| `/api/session` body drifts during the `session_turns` factoring | The existing `ui/tests.rs` assertions on the body are the parity baseline and must not be edited. If they need editing, the factoring is wrong. |
| The panic-hook test flakes against other panicking tests | A `static Mutex` owns the hook lifecycle; the test takes it, installs, catches, asserts, restores, releases. |
| ratatui's `Backend` API differs from expectation on 0.30 | The draw path is generic over `Backend` and touched by `TestBackend` from task 2 onward, so any API surprise surfaces in the first render test rather than at integration. |
| `main.rs` conflicts with `slice-run-selectors` | Re-check `main` before writing the dispatch arm; route `--run` through the shared prefix/`latest` helper if one exists there, otherwise use it verbatim. One line either way; never duplicate resolution. |
| The file grows past ~900 lines | Split then. Splitting late is cheap; unsplitting is not. |
