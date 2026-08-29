# Change Proposal: interactive-tui

## Why

Decision 0013 gave the terminal two static readouts (`forge runs`,
`forge inspect`) and one live one (`forge watch`), and put panes and
keyboard navigation explicitly out of scope pending their own decision.
Using them proved the gap: `forge watch` answers "what is happening now"
but cannot be *explored*. The operator cannot move a cursor over the
fleet, descend into one seat, watch it, and come back — which is exactly
what they asked for, in their words: *"a proper ASCII table with
navigation with keys, similar to claude code."* The operator sits in a
terminal while a forge runs, and that is where the readout is weakest.

Decision 0014 (`docs/decisions/0014-interactive-tui.md`, accepted
2026-08-29) is the ruling. It spends the promise 0013 made — "a future
TUI, if ever wanted, renders the same models" — and grants the one
exception it needs: `ratatui` and `crossterm` are adopted on the TUI path
only, because raw-mode entry, key decoding, resize handling and the
Windows console API cannot be hand-rolled responsibly and CI tests
Windows. This change implements that ruling in full.

The invariant that governs every choice below: **`forge tui` is a third
renderer, never a fourth derivation.** A renderer may branch on a model
field; it may not compute one.

## What Changes

- **`crates/forge-cli/src/tui.rs` — one new file, in family with
  `render.rs` and `ui.rs`.** It holds a state struct of owned scalars
  that **retains no `forge-view` model**: `RunsView` and `RunView` are
  derived fresh per refresh and dropped, so selection is by stable key
  (`RunRow.run_id`, `Phase.name`, `Participant.key`) resolved against the
  fresh models every frame. "Selection survives a refresh" and "clears
  itself when its subject vanishes" therefore become the *absence* of
  code rather than a diffing routine — and `forge-view` gains no
  `Clone`/`PartialEq` derives and no change at all, so `VIEW_VERSION`
  does not move. One `move_to()` is the only place wrap-around, `g`/`G`
  and paging exist, for every list at every level.
- **Three levels, each over models that already exist.** RUNS is a
  bordered, navigable table (id · status · phase · seq · age · clamped
  feature). RUN is the 0013 phase tree with its `⑂` and `→` markers, the
  six-column seats table, and the decision trail. PARTICIPANT is the
  seat's checkpoint stream plus its local session transcript where one
  exists, with `Participant.terminal_line` — the model's own
  `claude --resume <id>` cell, including its absence mark — shown
  unconditionally. A `format!("claude --resume {}", …)` in the TUI would
  be both a fourth derivation and a printed lie the operator may paste.
- **`Enter` pushes one rung, `Esc` pops one rung.** The ruling says both
  "Enter descends" and "selecting a phase or a participant scopes the RUN
  level"; this settles the tension without inventing a key. The rungs at
  RUN level are `unscoped → scoped → descended`, and `Esc` is a
  precedence ladder — help, then filter, then level, then scope — that
  **never quits**, so a fat-fingered `Esc` cannot kill the console. That
  also settles the one navigation edge the ruling left open: `--run <id>`
  opens at RUN, and `Esc` there walks to the full RUNS list rather than
  exiting. Only `q` and `Ctrl+C` quit; `Ctrl+C` is bound because raw mode
  disables SIGINT.
- **Reuse, and a net deletion.** `render::Scope`/`lens_for` are consumed
  directly (`.ok().flatten()` collapses the "no such phase/seat" `Err`
  into exactly the vanished-subject case, one mechanism for two
  requirements with no unreachable branch); `keeps_participant` and
  `keeps_row` become crate-visible; and the phase predicate written
  inline inside `graph_block` is **extracted** as `keeps_phase` and
  called from both callers, so the crate ends this slice with **one fewer
  copy** of the scope predicate than it has today. `status_code` splits
  into a shared `tone()` table so one classification serves three colour
  vocabularies. `ui.rs::session_transcript` factors into
  `session_turns()` carrying validation, location and parse **together**
  — the id validation is a path-traversal guard, and a factoring that
  left it in the HTTP layer would hand the TUI a traversal — with the
  `/api/session` body byte-identical. `WATCH_TRANSIENT_FRAMES` and its
  "the journal is not readable right now" sentence are reused verbatim.
- **`Safe` is hardened, not merely reused.** `Safe::new` strips
  `char::is_control()`, which does not cover U+202E RIGHT-TO-LEFT
  OVERRIDE, the U+2066–U+2069 isolates, or U+200B/U+200E/U+200F. Those
  pass untouched today and **visually reorder a rendered line** — forging
  exactly the ruling line `render.rs`'s own module doc names as the
  threat, in a surface that is full-screen and continuously redrawn. An
  explicit enumerated character set closes it with no new dependency, in
  one sanitizer serving all three surfaces, and every existing golden
  stays byte-identical because no golden contains a bidi control. Width
  arithmetic stays out of the TUI entirely: if nothing but `Safe` output
  reaches a `Cell` — enforced by there being exactly one
  `cell(&Safe)`/`span(&Safe, _)` and no `&str` constructor in the widget
  layer — then the text ratatui measures *is* the sanitized text, which
  is the ruling's invariant achieved by construction rather than by a
  second measurement system fighting the framework's.
- **Read-only is closed at the hole the panel found.** `Store::open`
  creates the parent directory, creates the database file, switches it to
  WAL (creating `-wal`/`-shm`), runs the migration DDL and INSERTs a meta
  row — so an NDJSON byte-compare passes cleanly through real writes and
  does **not** prove read-only. `forge tui` therefore refuses a `--db`
  that is not an existing file *before* `Store::open` is reachable, and
  the machine proof is a **directory-tree hash** of the db's parent
  alongside the NDJSON byte-compare, taken headlessly around a scripted
  drive and by subprocess around the real binary. Structurally, `tui.rs`
  names no `Store` and nothing from `forge_runtime`: the pure core and
  draw path take view models and the shell takes a `FnMut` refresh
  source, asserted by a source test.
- **Degradation and restoration are branches with tests, not prose.**
  Startup — not a tty, too small, or a missing db — exits with a message
  naming `forge inspect` and `forge watch`, outside any alternate screen.
  A *runtime* resize below the minimum draws a centred in-TUI frame
  naming the same two verbs with `q` still live, because an operator
  dragging a window edge should not lose their session. Restoration is
  RAII, so every `?` is safe by construction; the panic hook restores
  **first** and chains to the previous hook **second** (reversed, the
  backtrace is painted into the alternate screen and then erased), is
  idempotent, cannot itself panic, and is uninstalled on normal exit;
  `std::process::exit` is forbidden on this path.
- **Liveness that does not become the reason the forge stalls.** The RUN
  head is `head_hash(run)` compared on **both** seq and hash — a
  rewritten journal at equal seq is the tamper case `anchor` exists for.
  The RUNS fleet cannot poll that way (it has no single run), and cloning
  `Cmd::Runs` into a 250 ms tick would re-read and re-fold every event of
  every run four times a second while a `forge run` holds the write lock,
  so it refolds on a named slower cadence and on `r`. `Cmd::Runs`'s `?`
  is **not** copied: the TUI passes `fold(..).ok()` and lets
  `RunRow.status_known` produce the absence mark, because one corrupt run
  taking the operator's whole fleet table with it is what decision 0001
  forbids.
- **Testing is the shape the ruling mandates.** Key handling is a pure
  state machine over (view models, key event) with our own `Key` enum
  translated at the boundary — the translation filters
  `KeyEventKind::Press`, without which Windows handles every keystroke
  twice on the exact CI leg crossterm exists to make survivable.
  Rendering is `Backend`-generic and tested through `TestBackend`,
  asserting the table headers, a hostile seat label rendering inert
  *including that its neighbouring column starts at its expected x*, and
  a footer that differs by context. The five process-global crossterm
  calls are `fn`-pointer fields holding crossterm's **own function
  items** — never our wrappers, never closures, either of which would be
  a counted function only production could execute — so the exact
  coverage gate is designed for before code is written rather than
  discovered in CI.
- **One dependency entry.** `ratatui = { version = "0.30",
  default-features = false, features = ["crossterm"] }` in
  `[workspace.dependencies]`, referenced only by
  `crates/forge-cli/Cargo.toml`; crossterm arrives through ratatui's own
  re-export. The ruling adopts crossterm as a capability, not as a
  manifest line, and a separate entry can resolve a different crossterm
  than `ratatui-crossterm` links — two raw-mode implementations, one of
  them inert. `default-features = false` also keeps `widget-calendar` and
  `macros` out of the audited tree.

Design artifacts:

- [specs/interactive-tui/spec.md](../../../specs/interactive-tui/spec.md)
  — WHAT and WHY: the governing invariant, the eleven numbered rulings
  that settled the panel's disputes (six for robustness, five for
  simplicity), the behaviour of all three levels, and 20 acceptance
  criteria.
- [specs/interactive-tui/plan.md](../../../specs/interactive-tui/plan.md)
  — HOW: the module layout, the pure/generic/shell/impure partition, the
  state struct, the coverage table naming a seam for every impure line,
  the three read-only enforcement layers, and the risk register.
- [specs/interactive-tui/tasks.md](../../../specs/interactive-tui/tasks.md)
  — twenty-six ordered tasks in seven movements, each paired with the
  test that proves it.

## Impact

- **Surfaces**: three (console, CLI readouts, TUI) over one derivation.
  0013's invariant holds by construction — this change adds no derivation
  and modifies `forge-view` not at all.
- **Existing behaviour**: `forge runs`, `inspect`, `watch` and `ui` are
  unchanged; their goldens are the parity baseline and stay
  byte-identical, including through the `Safe`, `tone()` and
  `session_turns` refactors. The `/api/session` response body is
  byte-identical.
- **Codebase**: `render.rs` ends with one fewer copy of the scope
  predicate and one hardened sanitizer serving all three surfaces.
- **Dependency tree**: the forge gains ratatui and its crossterm backend
  — the price of the capability, ruled deliberately by 0014 rather than
  smuggled in, covered by CI's RustSec audit job, and confined to
  `forge-cli`. `forge-view`'s manifest stays exactly `forge-core`,
  `serde`, `serde_json`, which is what makes its purity a compile error.
- **Operator boundary**: unchanged. The TUI issues no operator commands,
  starts no runs, writes nothing to the journal, and now also creates no
  database it promised only to read. `forge operator retry|stop` stay CLI
  verbs precisely because they are consequential (0003, restated by
  0014).
- **CI**: the Windows leg gains a real dependency on crossterm's console
  API and on the `KeyEventKind::Press` filter; the coverage gate gains a
  full-screen program whose every impure line has a named seam; the
  audit job gains a tree.
- **Known deviations, flagged not smuggled**: CJK and emoji align better
  inside the TUI than in `forge runs`, because ratatui carries
  `unicode-width` internally — a consequence of the ruling's own
  adoption, improving the newer surface and changing no older one. The
  five process-global crossterm calls are never executed under coverage;
  no design can execute them without a tty or a pty crate the ruling did
  not adopt, and the mitigation is that the seam holds crossterm's own
  function items with no logic behind them. The RUNS fleet is up to one
  slow tick stale, deliberately.
- **Coordination**: `main.rs` may conflict with `slice-run-selectors`,
  which adds run-id prefix and `latest` resolution to the `--run` flags.
  The implementer re-checks `main` before writing the dispatch arm and
  routes `--run` through whatever shared helper exists there; if none
  exists, `--run` is used verbatim as every read verb does today.
  Resolution is never duplicated and never added by this slice.
