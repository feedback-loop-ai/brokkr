# Feature Specification: `brokkr tui` — an interactive, read-only console

**Feature slug**: `interactive-tui`
**Run**: `implement-decision-0014-forge-tu-b5045614`
**Status**: Committed (design phase ruling)
**Scope**: implements decision 0014
(`docs/decisions/0014-interactive-tui.md`, accepted 2026-08-29). No new
decision doc.
**Input positions**: `.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md` (run-local, uncommitted)

## Why

Decision 0013 gave the terminal two static readouts (`brokkr runs`,
`brokkr inspect`) and one live one (`brokkr watch`), and put panes and
keyboard navigation explicitly out of scope pending their own decision.
Using them proved the gap. `brokkr watch` answers "what is happening
now"; it cannot be *explored*. The operator cannot move a cursor over
the fleet, descend into one seat, watch it, and come back — which is
exactly what they asked for, in their words: *"a proper ASCII table with
navigation with keys, similar to claude code."*

0013 promised that "a future TUI, if ever wanted, renders the same
models". **This feature spends that promise and adds no derivation.**
`ratatui` and `crossterm` arrive as the ruling's narrow, explicit
exception to the no-new-dependencies default, on the TUI path only.

## The design in one paragraph

`brokkr tui` is one new file, `crates/brokkr-cli/src/tui.rs`, plus its
conventional `tui/tests.rs` — in family with `render.rs` (534 lines,
two terminal surfaces) and `ui.rs` (396 lines, an HTTP server, an SSE
loop and a transcript parser). It holds a state struct of owned scalars
that **retains no `brokkr-view` model**: `RunsView` and `RunView` are
derived fresh per refresh and dropped, so selection is by stable key
(`RunRow.run_id`, `Phase.name`, `Participant.key`) resolved against the
fresh models every frame. "Selection survives a refresh" and "clears
itself when its subject vanishes" are therefore the *absence* of code,
not a diffing routine, and `brokkr-view` gains no `Clone`/`PartialEq`
derives and no change at all. The pure core is
`apply(&mut Tui, &Views, Key) -> Flow` over **our own** `Key` enum; the
draw path is generic over `ratatui::backend::Backend`, so `TestBackend`
reaches every widget headlessly; the five process-global crossterm calls
are function-pointer fields whose production values are crossterm's own
function items; and the whole shell is driven by two injected sources —
one `FnMut` for keys, one `FnMut` for a refresh — so the loop, its error
arms and its transient-busy arms all execute in tests without a
terminal. Restoration is RAII plus a panic hook that restores first and
chains second. `render.rs` ends the slice with **one fewer** copy of the
scope predicate than it has today.

## The rule that governs every other choice

> **`brokkr tui` is a THIRD RENDERER, never a fourth derivation.**
> A renderer may branch on a model field; it may not compute one.

Selecting, filtering, ordering for display, and laying out are
rendering. Deriving status, cost, duration, activity text, scope
membership, or topology is not. If the TUI needs a value no model
carries, the answer is to extend `brokkr-view` **with its tests** — and
this design deliberately needs none, which is the strongest form of
compliance available.

## What the positions settled

The panel agreed on more than it disputed: one file, no `Action` enum, no
`Backend` trait we own, selection by key, `Scope`/`lens_for` reused
rather than mirrored, function-pointer seams over crossterm, a
`Backend`-generic draw path, `Esc` never quitting, and `--run`'s `Esc`
ascending to RUNS. Those are adopted verbatim and are not re-argued
below. Six disputes are ruled here.

### Ruled for robustness

1. **`Store::open` is a write, and the framed proof does not prove
   read-only.** `crates/brokkr-store/src/lib.rs:76` creates the parent
   directory, creates the database file, switches it to WAL (creating
   `-wal`/`-shm`), runs the migration DDL and INSERTs a meta row. An
   NDJSON byte-compare passes cleanly through every one of those.
   `brokkr tui --db /tmp/nope.db` would create `/tmp/nope.db`. **Ruling:**
   the db path is `is_file()`-gated *before* `Store::open` is ever
   called, a missing db is a refusal, and the machine proof is a
   **directory tree hash** (relative path, length, bytes) of the `--db`
   parent, taken before and after, *in addition to* the NDJSON
   byte-compare. The operator's plain reading of "read-only, absolutely"
   is "my disk looks the same afterwards", and that is now what is
   proven.
2. **Keys are our own enum, translated at the boundary, filtered to
   `KeyEventKind::Press`.** Windows delivers key *release* events; a
   handler matching on `KeyCode` alone processes every keystroke twice on
   the exact CI leg the ruling says crossterm exists to make survivable.
   `Ctrl+C` is bound alongside `q`, because raw mode disables SIGINT and
   an operator whose draw path wedges must have a way out. Mouse, paste
   and focus events are ignored by named arms, not by an untested
   wildcard.
3. **Restoration is RAII.** A trailing `disable_raw_mode()` is skipped by
   every `?` between setup and the end of the loop — which is precisely
   the bug the framing's "prove restoration on the error path" demand
   exists to catch. A guard whose `Drop` leaves the alternate screen,
   shows the cursor and disables raw mode makes `?` safe by construction,
   and the error-path test then proves a property the type system already
   gives. `std::process::exit` is forbidden anywhere on this path; the
   TUI returns `ExitCode` like every other arm.
4. **"Too small" is two branches, not one.** Terminals are resized while
   a program runs. Startup too-small (or not-a-tty, or missing db) exits
   with the message before raw mode is entered; a *runtime* resize below
   the minimum draws a single centred in-TUI frame naming `brokkr inspect`
   and `brokkr watch`, with `q` and `Ctrl+C` still live. An operator
   dragging a window edge does not lose their session, and no frame is
   ever corrupted.
5. **A filter must never clear a scope.** Absence from the *filtered*
   list is a display fact; absence from the *unfiltered model* is the
   vanish condition the ruling names. Cursor movement is over the
   filtered key list; scope clearing is decided against the unfiltered
   one. One predicate answering both questions is a bug.
6. **`Safe` is extended, not merely reused.** `Safe::new` strips
   `char::is_control()`, which does not include U+202E RIGHT-TO-LEFT
   OVERRIDE, the U+2066–U+2069 isolates, or U+200B/U+200E/U+200F. Those
   pass untouched and **visually reorder the rest of a rendered line** —
   forging exactly the ruling line `render.rs`'s own module doc names as
   the threat, in a surface that is full-screen and continuously
   redrawn. `Safe` is hardened in `render.rs` with an explicit,
   enumerated character list (no Unicode-properties dependency), and a
   hostile-bidi case joins `render/tests.rs`. One sanitizer for three
   surfaces; a TUI-only second sanitizer is the drift 0013 refuses, at
   the layer where drift is worst.

### Ruled for simplicity

7. **ratatui owns column arithmetic; the TUI contains zero width
   arithmetic.** The robustness position asked to compute widths
   ourselves and `clamp` before the `Cell`, leaving ratatui as a backstop.
   Rejected: the width we would compute is arithmetic over a `Rect`
   ratatui's own `Layout` produced — two measurement systems over one
   string, which is how a column silently eats its neighbour. The
   ruling's invariant is that *the widths used for layout are computed on
   the sanitized text*; if nothing but `Safe` output reaches a `Cell`,
   the text ratatui measures **is** the sanitized text, and the invariant
   holds by construction with no second implementation. Robustness's
   *enforcement* mechanism is adopted in full: exactly one
   `fn cell(text: &Safe) -> Cell` and one `fn span(text: &Safe, style:
   Style) -> Span`, no `&str`-taking constructor anywhere in the widget
   layer, and the `/` filter text — operator input, echoed in the footer,
   reachable by bracketed paste — sanitized like everything else.
   **Honest deviation, flagged not smuggled:** ratatui depends on
   `unicode-width` internally, so CJK and emoji align *better* inside the
   TUI than in `brokkr runs`/`inspect`. The non-goal "no Unicode-width
   dependency" constrains forge-authored arithmetic and `brokkr-view`'s
   manifest; the ruling adopted ratatui with its tree. No existing
   surface changes behaviour.
8. **No `Journal` trait, and no `Store` in the TUI at all.** The
   robustness position wanted a three-method read-only trait so a write
   is a compile error. The same guarantee is bought more cheaply: the
   pure core and the draw path receive **only view models**, and the
   shell receives a `&mut dyn FnMut() -> Result<Snapshot>` refresh source
   — so no function in `tui.rs` ever holds a `Store`, tests supply a
   scripted refresher, and a source-level test asserts `tui.rs` contains
   no `brokkr_runtime`, `append_next` or `create_run` token (the idiom the
   `ui.html` anti-drift test already established). A trait plus a test
   double is three more functions and three more impls under a
   100%-*function* gate to buy what one `FnMut` parameter and one grep
   already give. The actual write hole — `Store::open` — is closed by
   ruling 1, not by a trait.
9. **No new `brokkr-store` query for RUNS liveness.** Robustness is right
   that cloning `Cmd::Runs` into a 250 ms poll re-folds every event of
   every run four times a second against a `brokkr run` holding the write
   lock. Its own stated fallback is adopted, because it costs no
   brokkr-store change and answers the complaint: **the RUNS list refolds
   on a slower, named cadence** (`RUNS_REFRESH_TICKS`, ≈2 s) than the RUN
   level's head poll, and on `r`. Stated as a decision in the spec so it
   is not an accident.
10. **`fold(..).ok()`, not `?`.** `Cmd::Runs` propagates a fold error, so
    one corrupt run kills the whole listing. In a CLI verb that is an
    error and a nonzero exit; in a TUI it is the operator's entire fleet
    table vanishing because one old run is bad — which decision 0001
    forbids. The TUI passes `state: fold(..).ok()` into `RunEntry` and
    lets `RunRow.status_known` produce the absence mark. This is not a
    fourth derivation: it is choosing which input to hand the one
    derivation. `Cmd::Runs` itself is **not** changed.
11. **One dependency entry, not two.** `ratatui = "0.30"` with
    `default-features = false, features = ["crossterm"]`; crossterm is
    reached through ratatui's own re-export. The ruling adopts crossterm
    as a *capability*; it does not require a second manifest line to
    prove it, and a separate `crossterm = "0.29"` entry can resolve a
    different crossterm than `ratatui-crossterm` links — two raw-mode
    implementations, one of them inert. `default-features = false` also
    drops `widget-calendar` and `macros` from the audited tree.

## Behaviour

### 1. The command

`brokkr tui [--run <id>] [--db <path>]`. `--db` defaults to
`.forge/forge.db` exactly as every other read verb. `--run` opens
directly at the RUN level for that run. Clean quit exits `SUCCESS`;
refusal or error exits `1` through `main`'s existing `Err` arm. `forge
tui` does **not** inherit `watch`'s status→exit-code mapping: that
mapping exists so CI can wait on a run, and an interactive console is
never in a pipeline.

### 2. Three levels

| Level | Panes | Source model |
|---|---|---|
| **RUNS** | one bordered, navigable table: id · status · phase · seq · age · clamped feature | `brokkr_view::run_rows` → `RunsView`/`RunRow` |
| **RUN** | graph (the 0013 tree with `⑂` and `→`), seats table (participant · status · attempts · turns · cost · activity), decision trail | `brokkr_view::run_view` → `RunView.phases`, `.participants`, `.journal` (`in_trail`) |
| **PARTICIPANT** | checkpoint stream; local session transcript when one exists; the `claude --resume <id>` line, always | `Participant.checkpoints`, `.terminal_line`, `.session_id` |

Every displayed value is a model field. The seats table shows
`activity.text` for a concluded seat and `activity.tool` +
`activity.target_short` for a working one — both model fields, neither
computed. The `claude --resume` line is `Participant.terminal_line`
rendered as the model built it, including its `absent` mark when
`session_id` is `None`; a `format!("claude --resume {}", …)` in the TUI
would be both a fourth derivation and a printed lie the operator may
paste.

### 3. Keys and the ladder

`↑`/`↓` and `j`/`k` move · `Enter` descends · `Esc`/`Backspace` ascends ·
`Tab` cycles panes · `g`/`G` top/bottom · `PageUp`/`PageDown` page · `/`
filters the focused list incrementally · `r` forces a refresh · `?` help
overlay · `q` and `Ctrl+C` quit.

The ruling says both "Enter descends" and "selecting a phase or a
participant scopes the RUN level". That is settled without inventing a
key: **`Enter` pushes one rung, `Esc` pops one rung**, and the rungs at
RUN level are `unscoped → scoped → descended`.

`Enter`:
1. RUNS, cursor on a run → set `run`, `level = Run`, `pane = 0`, cursors
   and scope cleared.
2. RUN · graph → `scope = Some(Scope::Phase(name))`, replacing any
   existing scope (exclusivity *is* one `Option` field, not a rule).
3. RUN · seats, this seat not currently scoped → `scope =
   Some(Scope::Seat(key))`.
4. RUN · seats, **this** seat already scoped → `level = Participant`,
   `seat = Some(key)`.
5. RUN · trail, PARTICIPANT panes, or filter mode → no-op / commit.

`Esc` — a precedence ladder; **`Esc` never quits**, so a fat-fingered
`Esc` cannot kill the console:
1. help open → close it;
2. typing a filter → leave filter mode **and clear the filter**;
3. `level == Participant` → `level = Run`, `seat = None`, scope retained
   (you land back on the seat you were reading — the symmetric pop);
4. `scope.is_some()` → `scope = None`;
5. `level == Run` → `level = Runs`;
6. `level == Runs` → nothing.

`Backspace`: while typing, delete one char — this is what makes `/`
incremental; otherwise identical to `Esc` rungs 3 and 5 only, ascending
without ever clearing a scope.

`Tab` is `pane = (pane + 1) % panes_at(level)` with `panes_at` = 1 · 3 ·
2. At RUNS that is a visible no-op, which is correct and needs no
message.

This settles the framing's risk 7 in the same breath: `--run <id>` starts
at rung `Run/unscoped`, and `Esc` walks it to the full RUNS list rather
than exiting. No special case for the flag.

### 4. Selection

Every navigable list is a `Vec<String>` of stable keys — `RunRow.run_id`,
`Phase.name`, `Participant.key`, `JournalRow.seq.to_string()`; the
PARTICIPANT panes are paragraphs with an `offset`, not a cursor. One
function `move_to(keys, &mut cursor, step)` is the only place wrap-around,
`g`/`G` and paging exist, at every level. When `index_of` returns `None`
— the subject vanished across a refresh, or the filter excluded it —
movement restarts from the top and no highlight renders.

Invariant, tested: **assigning `run` clears `scope`, `seat`, `filter` and
every cursor.** That is what a run-qualified selection path buys, bought
by an assignment rather than a type.

### 5. Scoping

`render::Scope` and `render::lens_for` are consumed directly, never
mirrored. Resolution is once per frame against the fresh models:

```rust
let lens = render::lens_for(&view, self.scope.as_ref()).ok().flatten();
if lens.is_none() { self.scope = None; }   // the subject vanished
```

`.ok().flatten()` is load-bearing: `lens_for`'s `Err` arm means "this run
has no such phase/seat", which for the TUI *is* the vanished-subject
case. Collapsing it gives one mechanism for two requirements and costs
zero unreachable branches — a design that matched on the `Err` and
rendered a banner would add a branch the TUI can never reach and the gate
can never cover. (`lens_for`'s `Err` arm stays covered by the existing
`inspect --seat nobody` test.)

`render.rs`'s `keeps_participant` and `keeps_row` become `pub(crate)`,
and — this is a **deletion** — the phase predicate written inline inside
`graph_block` (`render.rs:396–403`) is extracted as
`pub(crate) fn keeps_phase(lens: Option<&Lens>, phase: &Phase) -> bool`
and called from both `graph_block` and the TUI. The crate ends the slice
with one fewer copy of the scope predicate than it has today.

### 6. Liveness

Input polls with a bounded timeout so a keypress is never waiting on a
tick; the tick checks the journal head. At RUN level the head is
`store.head_hash(run)` compared on **both** seq and hash — a rewritten
journal at equal seq is the tamper case `anchor` exists for. At RUNS
level the fleet refolds on the slower `RUNS_REFRESH_TICKS` cadence, or on
`r`. `r` forces a refresh at any level.

A transient store error increments a failure count, renders
`WATCH_TRANSIENT_FRAMES`'s existing sentence — "the journal is not
readable right now: …" — and keeps drawing; past the constant it returns
`Err` and the guard restores the terminal. **Keys stay live during
unreadable frames**: a TUI that blocks on a store read behind a 10-second
`busy_timeout` is a TUI the operator cannot quit for ten seconds.

### 7. Colour

`render::status_code`'s fixed four-status table is split, not copied:

```rust
pub(crate) enum Tone { Good, Bad, Live, Quiet }
pub(crate) fn tone(status: &str) -> Tone;          // the table, moved verbatim
fn status_code(status: &str) -> &'static str;      // Tone -> ANSI, unchanged
```

`tui.rs` maps `Tone` to a `ratatui::style::Style`. One classification in
the crate, three renderings of it — which is what "three surfaces, one
derivation" means. Existing goldens are unchanged by construction. A
status outside the known four falls to the quiet arm; it is never guessed
into one of the four (0001). The graph tree is **not** coloured, exactly
as `graph_block` does not colour it: its `⑂`/`→` markers are content.

### 8. Session transcript

`ui.rs::session_transcript` is factored into
`pub(crate) fn session_turns(id: &str) -> Option<(Vec<Turn>, bool)>`
carrying **validation, location and parse together** — the id validation
(`!empty && len <= 64 && all hex-or-'-'`) is a path-traversal guard,
since the id is joined into `~/.claude/projects/*/<id>.jsonl`, and a
refactor that extracts the parse while leaving the validation in the HTTP
layer hands the TUI a traversal. `Turn`/`Block` are two plain structs
with no `serde` derive, so no dependency edge is added; the `json!`
construction lifts into a short converter in `handle` and the
`/api/session` response body stays **byte-identical** (its existing tests
are the proof).

This stays in `brokkr-cli`. The transcript is journal-independent and
reads `HOME`; putting it in `brokkr-view`, whose manifest is exactly
`brokkr-core`/`serde`/`serde_json`, would destroy the purity that manifest
makes a compile error. The 0013 invariant governs derivations *over the
journal*; it does not reach a file on the operator's disk.

Transcript text is arbitrary prose from outside the store: it goes
through `Safe` like everything else, and the `truncated` flag is
**shown** — 0001 forbids silently short evidence. The transcript is read
on descend and on refresh (which is head-gated), never unconditionally
per frame.

### 9. Footer and help

`footer_for(&Tui) -> String` is pure and returns a different string per
(level, pane, typing, help) — at RUN · graph it names `Enter scope`, at
RUN · seats on an already-scoped seat it names `Enter open`. It is
unit-tested for difference, so "a context footer that always prints the
same string" cannot pass. `?` overlays `Clear` + a centred help
paragraph.

### 10. Read-only, absolutely

No operator command, no run start, no journal write — not behind a
confirmation, not behind a flag. `brokkr operator retry|stop` stay CLI
verbs precisely because they are consequential (0003, restated by 0014).
The property is structural (`tui.rs` names no `Store`, no
`brokkr_runtime`), gated (missing db refuses before `Store::open`), and
proven twice (tree hash + NDJSON byte-compare, headless and by
subprocess).

## Acceptance Criteria

- **AC-1 — The command.** `brokkr tui [--run <id>] [--db <path>]` exists,
  `--db` defaults to `.forge/forge.db`, `--help` lists it among the read
  verbs. *(Subprocess test; `--run` resolved through whatever shared
  prefix/`latest` helper exists on `main` at implementation time — see
  Risk 8 — never a duplicate resolution.)*
- **AC-2 — Startup refusals precede every side effect.** Not a tty
  (`std::io::stdout().is_terminal()`, the same `IsTerminal` `render.rs`
  uses), terminal smaller than the minimum, or `--db` not an existing
  file → a message on stderr naming **both** `brokkr inspect` and `forge
  watch`, exit 1, printed outside any alternate screen, with
  `Store::open` never called and no file, directory, `-wal` or `-shm`
  created.
- **AC-3 — Three levels over existing models.** RUNS shows id · status ·
  phase · seq · age · clamped feature; RUN shows the `⑂`/`→` phase tree,
  the six-column seats table and the decision trail; PARTICIPANT shows
  checkpoints, the local transcript when one exists, and
  `Participant.terminal_line` unconditionally. Every cell is a model
  field.
- **AC-4 — Key handling is a pure state machine.** `apply(&mut Tui,
  &Views, Key) -> Flow` requires no terminal, no store and no I/O, and
  every key in §3 is unit-tested headlessly.
- **AC-5 — The Enter/Esc ladder.** Each `Enter` rung and each `Esc` rung
  in §3 has its own test, including `Backspace` ascending via rungs 3/5
  only, `Esc` never quitting, and `--run`'s `Esc` reaching RUNS.
- **AC-6 — Scope is exclusive, cleared by `Esc`, never by a filter.** A
  second selection replaces the first; `Esc` at rung 4 clears it; a
  filter that hides the scoped subject leaves the scope intact.
- **AC-7 — Selection by key, resolved per frame.** Selection survives a
  refresh that reorders or extends the list, clears itself when its
  subject vanishes from the *unfiltered* model, and assigning `run`
  clears scope, seat, filter and cursors.
- **AC-8 — The footer names context keys.** Two different (level, pane,
  mode) states produce two different footer strings, asserted both as a
  pure function and through a `TestBackend` buffer.
- **AC-9 — Liveness.** The RUN head is compared on both seq and hash; the
  RUNS fleet refolds on the named slower cadence and on `r`; a transient
  store error renders `WATCH_TRANSIENT_FRAMES`'s existing sentence with
  keys still live, and a persistent one returns `Err` after the constant;
  a run whose journal does not fold **keeps its row** with the model's
  absence mark.
- **AC-10 — One status table.** `tone()` is the single classification;
  `render::status_code` is expressed through it; existing `runs`/
  `inspect`/`watch` goldens are byte-identical; the graph is uncoloured.
- **AC-11 — Sanitization and width.** Only `Safe` reaches a `Cell` or
  `Span` (one `cell(&Safe)` / one `span(&Safe, _)`, no `&str`
  constructor in the widget layer, greppable); the filter input is
  sanitized; `Safe` additionally strips the enumerated bidi and
  zero-width formatting characters (U+200B–U+200F, U+202A–U+202E,
  U+2060–U+2064, U+2066–U+2069, U+FEFF) with no new dependency and with
  every existing golden byte-identical; and a hostile seat label carrying
  `\x1b]0;pwn\x07`, `\r` and U+202E renders inert in a `TestBackend`
  buffer — no `\x1b`, no `\r`, no reordering, **and the neighbouring
  column starts at its expected x**.
- **AC-12 — Restoration on quit, error and panic.** An RAII guard
  restores on every path including `?`; the panic hook restores **first**
  and chains to the previous hook **second**, is idempotent, cannot
  itself panic, and is uninstalled on normal exit; `std::process::exit`
  appears nowhere on the TUI path. Proven, not asserted in prose: the
  error path is forced and the recorder inspected; the hook is installed,
  a deliberate panic is caught, the recorder is checked, and the prior
  hook is put back — under a serializing lock, since hooks are
  process-global.
- **AC-13 — Runtime degradation.** A resize below the minimum draws a
  single centred in-TUI frame naming `brokkr inspect` and `brokkr watch`
  with `q`/`Ctrl+C` live, and never tears down the session.
- **AC-14 — The Windows leg.** Key translation filters
  `KeyEventKind::Press`, so a keystroke is handled exactly once; `Ctrl+C`
  quits; mouse, paste and focus events are ignored by named arms.
- **AC-15 — Read-only, proven twice.** A source test asserts `tui.rs`
  names no `Store`, `brokkr_runtime`, `append_next` or `create_run`; a
  headless test in `src/tui/tests.rs` exports the NDJSON, hashes the db
  directory tree, drives the state machine through every navigation path,
  and byte-compares **both**; a subprocess test in
  `tests/machine_proof.rs` runs the real `brokkr tui` binary and does the
  same around it.
- **AC-16 — One session lookup.** `session_turns` carries validation,
  location and parse; the `/api/session` body is byte-identical to
  today's; the transcript is `Safe`-sanitized and its `truncated` flag is
  displayed.
- **AC-17 — No fourth derivation.** `tui.rs` contains no status
  classification, cost or duration arithmetic, activity text, topology or
  scope predicate; `brokkr-view` is unchanged (its manifest stays exactly
  `brokkr-core`, `serde`, `serde_json`, and `VIEW_VERSION` does not move);
  `render.rs` ends the slice with **one fewer** copy of the phase
  predicate.
- **AC-18 — Dependencies.** Exactly one new `[workspace.dependencies]`
  entry, `ratatui`, `default-features = false, features = ["crossterm"]`,
  referenced only by `crates/brokkr-cli/Cargo.toml`; no other crate gains
  a terminal dependency; `Cargo.lock` is committed because the gate runs
  `--locked`; `brokkr-cli` declares no cargo features (the gate runs
  `--all-features`).
- **AC-19 — The exact coverage gate passes.** `scripts/coverage-exact.sh`
  reports literal nonzero 100% line **and** branch **and** function
  equality with `--workspace --all-features --locked --branch`, and no
  `coverage(off)` attribute exists. Every impure line is covered by the
  seam named for it in plan.md §Coverage, including the panic hook's
  closure, the guard's `Drop`, and `production_ops()`.
- **AC-20 — Nothing else moves.** `cargo test --workspace` green, `cargo
  clippy --workspace --all-targets --all-features` warning-free, `cargo
  fmt --check` clean, RustSec audit passing over the added tree, and
  `brokkr runs`/`inspect`/`watch`/`ui` output unchanged. `ARCHITECTURE.md`
  and `README.md` list `brokkr tui` with one sentence each on what it is
  for and its read-only boundary.

## Non-goals

No operator actions of any kind. No fourth derivation. No terminal
dependency outside `brokkr-cli`, and none at all in `brokkr-view`. No
change to `brokkr runs`/`inspect`/`watch`/`ui` behaviour or output. No
mouse, no config file, no theming, no persisted UI state, no cross-run
search, no log tailing beyond the two streams named. No `tokio`, no
background thread, no `--interval` flag, no `Action` enum, no
`Backend`/event trait of our own, no `TuiError`, no `Clone`/`PartialEq`
derives on `brokkr-view`. No new decision document. No
Unicode-width dependency of our own — see §7 for the one honest
consequence of the ruling's own adoption.

## Risks accepted

1. **Two-beat `Enter` at RUN level.** It is the only way to honour both
   "Enter descends" and "selecting scopes" without a key the ruling does
   not name, and the required context footer announces the difference
   (`Enter scope` vs `Enter open`). It also makes `Esc` the exact
   inverse, removing special cases rather than adding them.
2. **`enable_raw_mode`/`disable_raw_mode`/`poll`/`read`/`size` are never
   executed under coverage.** No design can execute them without a tty or
   a pty crate the ruling did not adopt. Mitigation: they are held as
   crossterm's own **function items** — never our wrappers, never
   closures, because either would be a counted function only production
   can reach — behind a struct with no logic; their bodies live in
   crossterm's files, which `cargo llvm-cov` excludes from the workspace
   report, and the three-OS matrix runs the real binary.
3. **CJK/emoji align better inside the TUI than in `brokkr runs`.** A
   consequence of the ruling's own dependency adoption; it improves the
   newer surface and changes no older one.
4. **The transcript's 4 MB budget can truncate a long session.**
   Inherited behaviour; the `truncated` flag is shown and the
   `claude --resume` line is the escape hatch that already exists.
5. **The RUNS fleet is up to `RUNS_REFRESH_TICKS` stale.** Deliberate:
   the alternative is re-folding every event of every run four times a
   second against a `brokkr run` holding the write lock. `r` is always
   available, and the RUN level — where the operator watches — polls at
   full cadence.
6. **`tone()` and `Safe` refactors touch `render.rs`.** Preferable to the
   alternative: goldens stay byte-identical by construction, and the
   crate ends with one status table and one hardened sanitizer instead of
   two of each.
7. **One file of roughly 700–900 lines.** In family with `render.rs` and
   `ui.rs`; split when size forces it, not in anticipation.
8. **`main.rs` may conflict with `slice-run-selectors`,** which adds
   run-id prefix and `latest` resolution to the `--run` flags. The
   implementer re-checks `main` before writing the dispatch arm and
   routes `--run` through whatever shared helper exists there; if none
   exists, `--run` is used verbatim as every read verb does today. Do not
   duplicate resolution and do not add resolution in this slice.
