# Implementation Plan: One derivation, two surfaces

**Feature slug**: `one-derivation-two-surfaces`
**Spec**: [spec.md](spec.md)

## Position reconciliation (how this plan was synthesized)

The panel agreed on more than it contested: `brokkr-view` is one
implementation with `Serialize`-only plain structs and public fields, no
surface trait, no caching layer, no error taxonomy (derivations are total
functions, not `Result`s); one new route `/api/view/<run>` carrying the
journal rows, with `/api/run/<id>`, `/api/session/<id>` and `/sse/<id>`
untouched as the parity baseline; the full feature in the model with
clamping as a renderer-called helper; newest-first runs with the `N runs`
trailer moving to `--json`; `--seat` matching all occurrences rather than
resolving ambiguity; `watch` exiting on `status != Running` (a park is a
stop) and reusing `finish()`'s 0/2/3/1 codes; `head_hash` polling; the
existing `summarize()` keys preserved; the `machine_proof.rs` pins
migrating to `--json` rather than to human-formatted output; and no new
dependency, subcommand, or geometry. Where they disagreed:

1. **Crate vs. module — robustness adopted.** `brokkr-view` is a crate.
   Simplicity's case is real (one consumer, the repo's own
   `ui.rs`/`ui/tests.rs` idiom, six config edits) but it argues cost,
   not correctness, and the cost it names is mostly one-time. The
   deciding argument is the one only a crate answers: a module inside
   `brokkr-cli` can call `Store::open`, `std::fs`, `std::env::var`, and
   `IsTerminal` **by accident, and nothing fails** — so the ruling's
   load-bearing property, "no I/O, no rendering, no terminal or DOM
   concepts", stays a review convention that decays. A `Cargo.toml`
   depending on exactly `brokkr-core`/`serde`/`serde_json` makes it a
   compile error, and the absence of `time` is what forces `now` to be a
   parameter, which is what makes the golden tests deterministic.
   Simplicity's gate argument does not survive inspection: the exact
   gate counts **functions**, and a crate boundary adds none — the same
   functions, in the same number, exercised by the same tests.
   *Simplicity's floor is adopted inside the crate*: `lib.rs` + `js.rs`
   and their `tests.rs`, and no `view/participants.rs`,
   `view/trail.rs`, `view/topology.rs` module tree.
   *Simplicity's exclusion is adopted*: not `brokkr-core`. The core is
   the parity oracle against the Python core; display truth has no
   business acquiring a parity obligation.

2. **Formatting: pre-baked text vs. `fmt_*` helpers — both rejected,
   reconciled.** Simplicity proposed structured values in the model with
   pure `fmt_*` helpers "called by both renderers". That cannot work:
   **the console's renderer is JavaScript and cannot call a Rust
   helper.** A model carrying only `cost: Option<f64>` forces the page
   to write `'Σ $' + cost.toFixed(4)` — a second derivation of the
   formatting rules, reintroduced in the commit that claims to remove
   them, and it drags the `toFixed` divergence back with it. The ruling:
   **every displayed scalar reaches the model as a (value, text)
   pair** — `Cell { text, absent, note }` beside `cost: Option<f64>` and
   `cost_aggregated: bool`. Simplicity's actual concern (no redundant
   fields, no newtype-per-value with a `Display` impl each) is honoured:
   one `Cell` type, no `Cost`/`Duration`/`Turns` newtypes, and the
   structured value stays so `--json` is lossless. Robustness reached
   the same conclusion for exactly one field (`payload_json`, its
   Part 2.11) without generalizing it; this generalizes it.

3. **`Value` vs. typed payload structs — robustness adopted**
   (uncontested). Derivation reads `payload.get("member").and_then(
   Value::as_str)`, mirroring the JS `typeof` guards. Typed
   deserialization with `#[serde(default)]` invents `""` where the
   console shows a deliberate absence with a reason; a wrong-typed field
   makes the whole event vanish where the console renders `?` and keeps
   the row. Both are repair (decision 0001). `Value::get` on a
   non-object payload returning `None` is also the exact twin of
   `e.payload || {}`, which a typed struct has no answer for.

4. **The `js` compatibility module — robustness adopted.** Five
   primitives in `js.rs`: `to_fixed_4`, `len`, `slice`, `round_half_up`,
   `to_display`. This is not defensive scaffolding; each closes a
   divergence that produces a *wrong displayed value* with no crash and
   no failing test. The cost case is the sharpest: `0.03125` is exactly
   representable and an entirely plausible session cost, and the naive
   Rust renders `$0.0312` where the console renders `$0.0313` — the new
   Rust surface disagreeing with the old console about money, inside the
   commit that exists to end exactly that drift. Simplicity did not
   contest this (it argued against newtypes with `Display` impls, which
   is a different thing and is separately adopted as a cut).

5. **Insertion order and effect-as-occurrence — robustness adopted**
   (uncontested). `Vec<Participant>` + `HashMap<String, usize>`.
   `HashMap` iteration would make the `brokkr inspect` golden flaky in CI
   only; `BTreeMap` would silently reorder the seats table; and Σ sums
   `f64` in members order, which is not associative.

6. **The live block as a second model — robustness adopted.** The
   framing folded it into the activity column; it is a different scan
   with different rules (global last-start, `p.seat` truthiness, the
   synthesized bare-seat row). Keeping only the participant model
   deletes an observable surface that nobody notices until a run is
   live. `LiveLine` is small — two fields — so this costs almost
   nothing and closes a real hole in the framing.

7. **Scoping as precomputed tags — robustness adopted.** If the
   predicate moves to Rust but each edge still filters with its own
   implementation of the rule, the rule is written twice again. Emitting
   `Participant.phase` and `JournalRow.phases` as data degenerates both
   edges to a membership test, so they cannot disagree because neither
   knows the rule. Scoping stays client-side (no `?phase=` round trip):
   a click must not be able to fail with a network error.

8. **Tree markers and the style gate — robustness adopted, with
   simplicity's concern answered structurally.** Simplicity's single
   gate (plain ⇒ no ANSI **and** ASCII markers) halves the golden matrix
   but doubles the *model*: the pre-baked text already carries `Σ`, `↓`,
   `…` and `—`, so an ASCII mode would need a second derivation of every
   one of those strings — the thing this feature exists to delete.
   Markers are unconditional; colour is gated. Simplicity's real
   objection, golden combinatorics, is answered by **structuring colour
   as a post-processing wrap** of an already-rendered plain string: the
   goldens all run plain, and one test proves the wrapping. Two
   combinations, not four, without degrading content.

9. **`--seat`/`--phase` with no match — robustness adopted.** Both
   positions agree on matching all occurrences. On the empty case,
   simplicity's "filtering is total, zero new failure modes" trades a
   branch for a false statement: an empty seats table reads as "this
   phase did nothing", which is a claim about a run that this tool
   cannot ship. One branch, one test. Robustness's exact-participant-key
   form is adopted too — it is the same predicate widened by an `||`,
   and it is what the console's clicks actually select. Simplicity's
   `ArgGroup` for the exclusivity is adopted over hand-written
   validation.

10. **`inspect --json` shape — reconciled.** Simplicity wanted a flat
    superset so existing scripts keep working; robustness wanted the
    keys nested under `summary` with a `view_version`. Nesting wins,
    because the migration cost simplicity is protecting against does not
    exist: `brokkr inspect` has no `--json` today, so **every** script
    must change its invocation anyway, and requiring `.summary.status`
    at the same time costs nothing extra while keeping the model shape
    honest. Robustness's `view_version: 1` and "absent serializes as
    `null`, never skipped" are adopted. The migration is stated as a
    checkable equality: `brokkr inspect --json | jq .summary` is today's
    output verbatim, all nine keys including `cursor`.

11. **`watch` redraw height gate — robustness REJECTED.** Robustness
    would redraw only when the frame fits the terminal height and append
    otherwise. Without a terminal-size syscall (no dependency), height
    can only come from `LINES`, which is usually unset — so the default
    24 would make nearly every frame append and `watch` would never
    redraw on a tty, defeating the feature to defend against a cosmetic
    outcome. Redraw is `\x1b[2J\x1b[H`, unconditional on a tty. Accepted
    and named: a frame taller than the terminal shows its tail.
    Robustness's other `watch` rules — hash-and-seq comparison,
    `SQLITE_BUSY` as a frame not an exit, `--interval` validated against
    the existing 100ms floor, read-only open — are adopted; they cost
    one branch each and each has a reachable test.

12. **Terminal sanitization — robustness adopted** (simplicity silent).
    The framing carries the console's `textContent` discipline and says
    nothing about the terminal, yet the journal is seat-authored and a
    `\r`-bearing result token can forge a ruling line in a `watch` loop
    that redraws. `Safe(String)` with a private field and a sanitizing
    constructor, applied before width arithmetic. This is the CLI's twin
    of the console's fixed class allowlists — the same idea, the other
    surface.

13. **Where robustness argues for LESS defense — adopted in full.** The
    exact gate inverts the usual reflex: an unreachable defensive branch
    is a red gate, not safety. So the seq-0 causation truthiness is
    **not** ported (verified: `Store::load` calls `verify_chain`, which
    pins `seq == i + 1`), derivations are total functions rather than
    `Result`s, `unwrap_or_default()` is not a reflex, and view-model
    tests assert on **serialized JSON strings** rather than struct
    equality — because a derived `Debug` is codegen'd and counted but
    `assert_eq!` only calls it when the assertion fails, leaving every
    derived `Debug` at zero on a green suite.

14. **The anti-drift tests — robustness adopted.** "The JS derivation is
    deleted, not duplicated" is the load-bearing clause and nothing
    currently enforces it. A banned-token assertion over the
    `include_str!`'d page (and a twin over `brokkr-view`'s sources for
    I/O tokens) is three lines and fails the day someone adds "just one
    small helper" back. The companion rule this plan states so the test
    is honest rather than superstitious: **the page may branch on a
    model field; it may not compute one.**

Simplicity's cut list is adopted wholesale except for the two items
above (the crate, and the single style gate): no `Deserialize` derives,
no `/api/view/runs`, no ETag/caching/pagination, no `TrailEntry` enum,
no value newtypes with `Display`, no fourth `watch` renderer, no
`--color`/`--width` flags, no `--format` beyond human and `--json`, no
`N runs` trailer, no snapshot framework (inline `assert_eq!` against
literal strings), no new subcommands beyond `watch`, and no protocol,
event or checkpoint change.

## Approach

Four movements, in order, each landing green:

1. **The crate.** `crates/brokkr-view` joins the workspace with
   `js.rs` first (the primitives everything else formats through), then
   the models, then the derivations in dependency order: participants →
   activity → Σ → live lines → topology → phase rail → trail/journal →
   scope tags → summary/ruling → run rows. Each rule gets its unit test
   in the same commit as the rule; each row of the spec's divergence
   table gets a test named for the divergence.
2. **The console.** `/api/view/<run>` and the `/api/runs` reserialization
   in `ui.rs`; then `ui.html`'s derivation is deleted and its consumption
   added in one commit, so the page is never running two derivations.
   The banned-token test lands with it.
3. **The CLI.** `render.rs` with `Safe`, `Style`, `runs()` and
   `inspect(view, scope, trail, style)`; `Cmd::Runs` and `Cmd::Inspect`
   rewritten; the `machine_proof.rs` pins migrated to `--json`.
4. **`watch`.** `Cmd::Watch` as the poll loop around
   `inspect(.., trail = false, ..)`.

## Files this touches

**New**

- `crates/brokkr-view/Cargo.toml`, `src/lib.rs`, `src/js.rs`,
  `src/tests.rs`, `src/js/tests.rs`
- `crates/brokkr-cli/src/render.rs`, `src/render/tests.rs`

**Modified**

- `Cargo.toml` — `[workspace] members` and `[workspace.dependencies]`
  gain `brokkr-view` (a path entry, not a third-party dependency)
- `crates/brokkr-cli/Cargo.toml` — depends on `brokkr-view`
- `crates/brokkr-cli/src/ui.rs` — the `/api/view/<run>` arm, `/api/runs`
  reserialized, the module doc's route list
- `crates/brokkr-cli/src/ui.html` — derivation deleted, consumption added
  (a net deletion of roughly 350 lines)
- `crates/brokkr-cli/src/ui/tests.rs` — the new endpoint, the banned-token
  test; existing assertions unedited
- `crates/brokkr-cli/src/main.rs` — `Cmd::Runs`/`Cmd::Inspect` rewritten,
  `--json`/`--phase`/`--seat`, `Cmd::Watch`, `Style::detect` and the one
  clock read that keeps the derivation pure
- `crates/brokkr-cli/src/tests.rs` — `summaries_costs_inspect_export_…`
- `crates/brokkr-cli/tests/machine_proof.rs` — the `brokkr runs` pins at
  ~:537-557, ~:570-585 and ~:1208-1215 migrate to `--json`
- `README.md` (the `crates/` row at :106 and the command listing),
  `ARCHITECTURE.md` (crate listing), `docs/target-architecture.md` :361

**Untouched, deliberately**: `contracts/`, `policy/phase-machine.json`,
`reference/`, `fixtures/`, `brokkr-core`, `brokkr-store`,
`brokkr-protocol`, `brokkr-runtime`, `brokkr-bridge`, and the
`/api/run/<id>` · `/api/session/<id>` · `/sse/<id>` responses.

## Console parity table

Each behaviour the framing enumerates, and what carries it after the
extraction. This table is the argument AC-18 asks for.

| Behaviour | After |
|---|---|
| collapsing runs pane, divider handle, `select()` collapsing | JS only — never derivation |
| collapsed one-line rows + `id\nstatus · phase P · seq N` tooltip | JS composes from `RunRow` fields; `status_known` gates the dot class |
| newest-first run order | `run_rows` — `runs.reverse()` deleted |
| exclusive scoping chip + `× show all` | JS state; membership from `phase` / `phases` tags |
| Σ on panel and sequence parents | `cost_cell` / `turns_cell` text, `*_aggregated` flags |
| activity as `result · duration` | `Activity.text`; live branch keeps tool/target split so the tooltip stays on the target span |
| trail default + `full journal · N events ▸` | `in_trail` filter, `event_count` |
| transcript drill + checkpoint-table fallback | `/api/session/<id>` unchanged; `Participant.checkpoints` feeds the fallback |
| terminal line | `Participant.terminal_line` |
| fork/join graph, arrow marker, symmetric lanes, legend | `Phase`/`Column`/`Node` in, SVG geometry (`LANE 26 / TOP 14 / COL_GAP 30 / PHASE_GAP 44`, `mono()`, lane paths) stays JS |
| live pulsing, stateful favicon | JS branches on `RunRow.status` and `summary.status` — permitted (branch, not compute) |
| selection survival and self-clearing across SSE | JS state reconciled against the refetched model |
| 5s `loadRuns` poll | unchanged |
| textContent-only, classes from fixed allowlists | `*_class` are closed-set keys mapped through the page's own table before touching `className` |

## Risks and mitigations

| Risk | Mitigation |
|---|---|
| A JS→Rust divergence ships silently (cost rounding, UTF-16 truncation, map order, `f64` sum order) | The spec's divergence table is the test list; each row is a named test written with the divergence as the assertion, not incidental to it |
| Non-ASCII goldens are blind to the length rules | At least one golden and one unit test carry an emoji astride a slice boundary and a `é`-bearing target |
| A hostile journal string forges terminal output | `Safe` newtype, private field, sanitizing constructor, applied before width math; ESC and `\r` goldens |
| Byte-slicing a UTF-8 feature panics `brokkr runs` | All truncation on `char` boundaries; `COLUMNS` parsed to `Option` and clamped `[20,1000]`; saturating column arithmetic |
| CJK/emoji columns misalign without a width crate | Admitted in the module doc and in spec.md; goldens do not claim to prove alignment for non-ASCII |
| The derivation returns to `ui.html` later | Banned-token test over the `include_str!`'d page, plus the stated branch-not-compute rule |
| A crate widens the covered surface | Every `pub` item is called by the two surfaces or by its own unit test; view-model tests assert on serialized JSON so no derived `Debug` sits at zero |
| `watch` re-derives per redraw | Events bucketed by `effect_id` once at the entry point, removing the O(phases × events) rescan |
| `watch` hangs a CI job on a park | Exit on `status != Running`, park reason printed first |
| A `watch` frame taller than the terminal | Accepted: the tail is shown. The alternative needs `LINES`, which is usually unset |
| `--json` becomes a de-facto contract | `view_version: 1`; absent serializes as `null` with its reason; the nine `summarize()` keys preserved under `summary` |
| The exact-coverage gate goes red on an unreachable branch | Total functions, no `Result` without a reachable `Err`, no ported seq-0 truthiness, no reflexive `unwrap_or_default()` |
| `machine_proof.rs` re-pins presentation | The three pins migrate to `--json`, never to human-formatted output |
