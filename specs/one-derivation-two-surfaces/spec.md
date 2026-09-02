# Feature Specification: One derivation, two surfaces

**Feature slug**: `one-derivation-two-surfaces`
**Run**: `implement-decision-0013-one-deri-d1ff0221`
**Status**: Committed (design phase ruling)
**Scope**: implements decision 0013
(`docs/decisions/0013-one-derivation-two-surfaces.md`, accepted
2026-08-29). No new decision doc.
**Input positions**: `.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md` (run-local, uncommitted)

## Why

The read-only console grew real domain logic during the UX pass — which
member concludes when, what a phase's traffic is, what a seat cost, when
an absence is deliberate. All of it is JavaScript inside `ui.html`, and
**none of it runs under the coverage gate**. Meanwhile the CLI never got
that pass: `brokkr runs` prints each run's entire feature text, `brokkr
inspect` dumps raw `RunState` JSON — and the operator sits in a terminal
over SSH while Brokkr runs, which is exactly where the readout is worst.

Building the terminal views on a second, hand-written derivation would
fork the answer to "what did this seat cost" into two implementations
that drift. Decision 0013 rules the alternative: **derivation is Rust,
rendering is per-surface.** This feature is that extraction plus the two
render surfaces it makes honest.

## The design in one paragraph

`brokkr-view` is a **new crate** whose `Cargo.toml` depends on exactly
`brokkr-core`, `serde` and `serde_json` — so "no I/O, no rendering, no
terminal or DOM concepts" is a compile error rather than a review
convention, and `now` must be a parameter because the crate has no
clock. It derives over `serde_json::Value` with the same `typeof` guards
the JavaScript uses, never over typed payload structs, because
deserialization defaults and drops are repair (decision 0001). Every
displayed scalar reaches the model as a **(structured value, rendered
text) pair**: the console's renderer is JavaScript and cannot call a
Rust helper, so a model that carried only raw values would force the
page to re-derive its formatting — the exact duplication this decision
exists to end. `brokkr ui` gains one route, `/api/view/<run>`;
`/api/runs` is reserialized as run rows; `ui.html` keeps SVG geometry,
DOM building and interaction, and its derivation is deleted under a test
that fails if it comes back. `brokkr-cli` gains `render.rs`: two pure
string functions (`runs`, `inspect`) where a `watch` frame is `inspect`
without the trail, colour is a post-processing wrap gated on
`IsTerminal`/`NO_COLOR`/`TERM=dumb`, and every string reaching a
terminal passes through a newtype whose only constructor strips control
characters.

## The rule that makes "deleted, not duplicated" checkable

> **The page may branch on a model field. It may not compute one.**

`runs.some(r => r.status === 'running')` for the favicon, and gating the
graph pulse on the selected run's status, are permitted: they read a
field the model already carries. `buildParticipants`, `innerColumns`,
`fmtDur`, `shortTarget`, `toFixed`, `Date.parse` and the trail
classification are not, and a test asserts those tokens are absent from
`ui.html`.

## The models (normative)

All types derive `Serialize` only. `Deserialize` is not derived: README
law 2 — state is derived, never mutated; nothing here is ever accepted
from a caller. Absent values serialize as `null`, never skipped, so a
consumer can tell "the journal does not carry this" from "your version
lacks the field".

```rust
pub struct Cell { text: String, absent: bool, note: Option<String> }

pub struct RunEntry<'a> { run_id, feature, created_at: &'a str,
                          state: Option<&'a RunState> }
pub struct RunRow { run_id: String, status: Option<String>,
                    status_known: bool, phase: Option<String>,
                    seq: Option<u64>, created_at: String, feature: String }
pub struct RunsView { view_version: u32, runs: Vec<RunRow>, count: usize }

pub struct Summary { /* the nine summarize() keys, verbatim */
    run_id, seq, status, phase, cursor, park_reason,
    consecutive_failures, last_decision: Value, feature }
pub struct Ruling { rule_id: String, severity_class: String,
                    from: String, next: String, result: Option<String>,
                    inputs: Vec<(String, String)>, problem: Option<String> }

pub struct Activity { text: String, absent: bool, note: Option<String>,
                      tool: Option<String>, target_short: Option<String>,
                      target_full: Option<String> }
pub struct Participant {
    key, label: String, member: Option<String>, phase: Option<String>,
    status: String, status_class: String, attempts: u64,
    turns: Option<u64>, turns_aggregated: bool, turns_cell: Cell,
    cost: Option<f64>, cost_aggregated: bool, cost_cell: Cell,
    activity: Activity, member_count: usize,
    session_id: Option<String>, terminal_line: String,
    checkpoints: Vec<CheckpointRow> }

pub struct LiveLine { label: String, text: String }

pub struct Node { label: String, key: String, state: String,
                  state_class: String }
pub struct Column { label: Option<String>, nodes: Vec<Node> }
pub struct Phase { name: String, visits: u64, current: bool,
                   plain: bool, columns: Vec<Column> }

pub struct What { text: String, badge: Option<String>,
                  badge_class: Option<String>, arrow: Option<String>,
                  problem: Option<String> }
pub struct JournalRow { seq: u64, causation_seq: Option<u64>,
                        event_type: String, recorded_at: String,
                        in_trail: bool, phases: Vec<String>,
                        what: What, label: Cell, payload_json: String }

pub struct RunView { view_version: u32, summary: Summary,
                     ruling: Option<Ruling>,
                     participants: Vec<Participant>, live: Vec<LiveLine>,
                     phases: Vec<Phase>, journal: Vec<JournalRow>,
                     event_count: usize }

pub fn run_rows(entries: &[RunEntry]) -> RunsView;          // newest first
pub fn run_view(events: &[EventEnvelope],
                state: Option<&RunState>) -> RunView;
pub fn clamp(s: &str, width: usize) -> String;              // pure helper
pub fn age(created_at: &str, now: &str) -> Option<String>;  // now is a parameter
```

`What` is a flat struct of optional parts, not a tagged enum: both
surfaces branch on `Option`s rather than on six variants, and the CLI
prints `text` without branching at all.

`Participant.phase` and `JournalRow.phases` are the **precomputed scope
membership**. Neither surface implements the scoping predicates; the
console filters `part.phase === sel` / `row.phases.includes(sel)` and
the CLI does the same membership test, so the two edges cannot disagree
because neither knows the rule.

## Rules ported verbatim from `ui.html`

`crates/brokkr-cli/src/ui.html` as it stands today **is** the
specification. Line references are to that file. The rules the framing
enumerates (§1a–§1g) are adopted in full and are not restated here;
what follows is the list of places where the obvious Rust is **not** the
JavaScript, each of which must be pinned by a named test.

| # | Rule | Faithful port |
|---|---|---|
| 1 | `cost.toFixed(4)` (:1070) | JS ties **away from zero** (ECMA-262: "if there are two such n, pick the larger n"); Rust `format!("{:.4}")` ties **to even**. `0.03125` → `$0.0313` on the console, `$0.0312` from naive Rust. `js::to_fixed_4` implements the JS rule. |
| 2 | `String.length` / `.slice` (:245, :249, :297, :1204, :1208) | UTF-16 code units, not bytes and not `char`s. `js::len` / `js::slice`; `slice` never splits a surrogate pair and drops a lone leading surrogate rather than inventing U+FFFD (that would be repair). |
| 3 | `feature.slice(0,110) + '…'` (:1208) | The ellipsis is **unconditional** — a 5-character feature renders `hello…`. Ported as-is. Comment and test name it so no reviewer "fixes" it. |
| 4 | `Math.round(ms/1000)` (:266) | Half-up. 59500ms → `1m00s`, not `59s`. `<60m` zero-pads seconds; `≥60m` **drops seconds entirely** (:270). |
| 5 | `Date.parse` → `NaN` | Unparseable or negative deltas produce `None`, never an `Err`. |
| 6 | Participant map order (:400) | JS `Map` insertion order drives the seats table AND the Σ `f64` sum order, and `f64` addition is not associative. `Vec<Participant>` + `HashMap<String, usize>`; never `HashMap`/`BTreeMap` iteration. |
| 7 | `effects.set` replaces the object (:420); Σ partitions on `q.effect === part.effect` (:1010) | Effect identity is the **request occurrence** (a `Vec<Effect>` index), with `effect_id` a last-write-wins lookup key. Keying Σ on `effect_id` would merge what the console splits. |
| 8 | Terminal matching (:430) | By `attempt_id` against the open attempt. A terminal event for a stale attempt is ignored entirely: it does not clear `open`, does not set the status, contributes no duration and no result token. |
| 9 | `turns` (:441) vs Σ turns (:1015) | A participant's own `turns` is a **max**; Σ across members is a **sum**. Identical in a one-member test. |
| 10 | Member outcome allowlist (:458) | `working` is in `STATUS_CLASS`, so a member can be pinned to `working` by its own `panel-member-finished` outcome after the effect concluded. Mapping only the three terminal outcomes shows `succeeded` where the console shows `working`. |
| 11 | `innerColumns` effect selection (:747-752) | **Newest** `effect/requested` for the phase wins (last write), not first. |
| 12 | `JSON.stringify(p)` (:1233) | Moves into the derivation as `payload_json` so both surfaces agree with each other. Named delta versus today's console: `serde_json` renders `1.0` where JS renders `1`. Key order is unaffected (`serde_json::Map` is a `BTreeMap`; `JSON.stringify` preserves the received order). |
| 13 | `turnCp.turn` / `turnCp.tool` printed raw (:981) | JS string concatenation of `undefined` yields the literal `undefined`. `js::to_display` reproduces it. A quirk, ported deliberately, named by its test. |
| 14 | `byId[e.causation_id]` truthiness (:1179) | **Not** ported. `Store::load` calls `verify_chain`, which pins `seq == i + 1` (`envelope.rs:106`), so seq 0 is unrepresentable in a loaded journal; a zero check would be an uncoverable branch and a red gate. `HashMap::get` is total; unknown ids produce no arrow. The `__proto__` hazard (`Object.create(null)`, :1179) evaporates in Rust and survives only as a test. |
| 15 | `innerColumns` rescans the journal per phase (:747, :756) | Bucket events by `effect_id` once at the entry point. Same semantics — newest-wins preserved — but not O(phases × events), which `watch` would pay every redraw. |

The live block (`<p class="live">`, :959-984) is a **second model**, not
the activity column. It comes from a different scan: `open` tracks the
last `effect/started` **globally** (:952, overwritten by any effect), its
labels come from `p.seat` truthiness defaulting `'?'` (:964), and it
synthesizes one bare seat row when the scan found nothing (:978). Folding
it into `Participant.activity` would delete an observable surface.

## Surfaces

### `brokkr ui` — the console keeps painting only

- **`GET /api/view/<run>`** returns `RunView`: one journal array whose
  rows carry `in_trail` and `phases`, so the full-journal toggle filters
  an array it already has and its count is `event_count`.
- **`/api/runs`** returns `RunsView.runs` (a JSON array, its existing
  shape) with the fields reserialized from `RunRow`; `runs.reverse()` is
  deleted from the JS because ordering is a derivation rule.
- **`/api/run/<id>`, `/api/session/<id>`, `/sse/<id>` are untouched.**
  Their tests in `ui/tests.rs` are the parity baseline. The transcript
  drill keeps `/api/session/<id>` with its checkpoint-table fallback.
- The new route lives inside `handle`, so it inherits the
  `request_allowed` DNS-rebinding guard (`ui.rs:54-61`) structurally, and
  a missing database stays a 404 — reads never create.
- Class allowlists (`RULING_SEVERITY_CLASS`, `STATUS_CLASS`,
  `INNER_STATE_CLASS`) become closed-set `*_class` strings in the model,
  and the JS **maps the key through its own fixed table** before it
  touches `className` — the wire value is an index into an allowlist,
  never a class name. `textContent` discipline is unchanged; the models
  are still untrusted seat-authored content.

### `brokkr runs`

One clamped line per run, newest first: id, status, phase, `seq N`, age,
feature. Columns are sized to the widest value in the batch (saturating);
the feature takes the remaining width and is clamped with `view::clamp`;
below 8 remaining columns the feature is omitted rather than mangled.
The `N runs` trailer leaves the human output and survives as
`RunsView.count` in `--json`.

### `brokkr inspect`

Header, ruling line, park reason when present, the seats table with the
console's six columns (participant, status, attempts, turns, cost,
activity), the decision trail, and the phase graph as a terminal tree:

```
graph
  intake ×1
    → intake · finished
  design ×2  ←current
    ⑂ positions
      simplicity · finished
      robustness · finished
    → chief · active
```

A single-node column renders `→ <label> · <state>`; a multi-node column
renders `⑂ <column label>` (the marker alone when the column has no
label — nothing is invented) followed by its indented nodes. `⑂` and `→`
are **unconditional**: they are content, not colour, and the model
already emits `Σ`, `↓`, `…` and `—` in pre-baked text, so an ASCII mode
would require a second derivation of every one of those strings.

`--phase <name>` and `--seat <label>` are mutually exclusive via clap's
`ArgGroup`, mirroring the console's exclusive scoping. `--seat` matches
**all** occurrences by label or by exact participant key: a re-entered
phase really did run that seat twice and hiding one is a false statement
about the run. A `--phase`/`--seat` value matching nothing exits nonzero
naming the valid values; an empty table would read as "this phase did
nothing".

### `brokkr watch <run>`

Polls `Store::head_hash` and compares **both** seq and hash — a rewritten
journal at equal seq is the tamper case `anchor` exists for, and `watch`
should redraw rather than sit blind. On change it reloads, re-derives and
redraws a frame that is `inspect` without the trail. `--once` prints one
frame. `--interval` tunes the poll with the existing 100ms floor
(`main.rs:596`) and rejects garbage rather than defaulting silently. A
transient store error (`SQLITE_BUSY` while a `brokkr run` holds the write
lock) is a frame that says so, not an exit; a persistent error exits
nonzero. Exit is on `status != Running` — including `AwaitingOperator`,
because a park admits no further events until a human acts and "keep
watching" is an unbounded CI hang; the park frame prints the park reason
first. Exit codes are `finish()`'s existing 0/2/3/1 mapping.

On a tty, redraw is `\x1b[2J\x1b[H` before each frame, unconditionally.
Non-tty degrades to appended, timestamped frames so pipes and CI logs
read as a timeline. No alternate screen, no cursor hiding, no spinner —
those are TUI moves and TUI is out of scope.

### Terminal safety

The journal is seat- and driver-authored: feature text, seat labels,
result tokens, `park_reason`, `problem`, error strings. Printed straight
to a tty, `\x1b]0;…\x07` retitles the operator's terminal, `\x1b[2J`
clears the frame that was supposed to be evidence, and `\r` plus spaces
overwrites the line above — so a hostile result token can **forge a
ruling line**, continuously under `watch`. Decision 0012 masks secrets on
the way in; nothing sanitizes on the way out, and the console's
`textContent` discipline has no terminal twin.

Renderers interpolate a `Safe(String)` newtype in `render.rs` whose only
constructor strips C0/C1 controls and `\x7f` (excluding the newlines the
renderer itself emits), applied **before** any width arithmetic so an
escape sequence cannot smuggle invisible width. Enforced by construction
— a private field, not discipline. `--json` needs none of this
(`serde_json` escapes control characters), which is itself an argument
for scripts using `--json`.

### Width and colour

`COLUMNS` parses to `Option<usize>`, defaults 80, clamps to `[20, 1000]`.
All column arithmetic saturating; all truncation on `char` boundaries —
byte-slicing a UTF-8 feature panics, and a panic in `brokkr runs` is worse
than any misalignment. Without a Unicode-width dependency (forbidden),
CJK and emoji columns **will** misalign; this is stated in the module doc
and here, not pretended away.

Colour is `stdout.is_terminal() && NO_COLOR unset && TERM != "dumb"`, at
most four codes (dim, bold, red, green) plus reset, mapped from the
existing fixed status table. It is applied as a **post-processing wrap**
of an already-rendered plain string, so goldens run in plain and exactly
one test proves the wrapping. No `--color` flag, no `--width` flag: the
ruling specified env gates.

## Non-goals

No TUI framework, panes or keyboard navigation. No new dependencies of
any kind. No writes from any of these surfaces. No change to the engine,
reducer, evaluator, policy semantics, journal schema or checkpoint
vocabulary. No new geometry or visual redesign of the console — this is
an extraction, not a UX pass. No second derivation anywhere. No changes
to `brokkr costs`, `compare`, `export`, `replay`, `verify-run`, `anchor`
or the bridge. No transcript-format changes.

## Constitutional constraints

Frozen contracts v1 untouched — view models are derived output, not
journal or protocol schema. `policy/phase-machine.json`, `reference/` and
the `fixtures/` evaluator corpus untouched. Decision 0003 (read-only,
loopback-only, embedded, no external assets) inherited by the new route.
Decision 0009 (Rust only, one binary). Decision 0012 and the journal
disciplines unchanged. README law 2. Decision 0001 — no repair: a
malformed payload renders as the deliberate-absence mark with its reason
or falls through the documented fallback chain; it is never guessed at,
coerced or defaulted into a plausible value.

## Acceptance Criteria

- **AC-1 — `brokkr-view` purity is structural.** The crate's
  `[dependencies]` are exactly `brokkr-core`, `serde`, `serde_json`. Its
  sources contain no `std::fs`, `std::env`, `IsTerminal`, `print!`/
  `println!`, and no clock; `run_rows`/`age` take `now` as a parameter.
  Proven by an anti-drift test asserting the banned tokens are absent
  from `crates/brokkr-view/src/*.rs`, plus the manifest.
- **AC-2 — Participants.** Keying (`effect_id` / `effect_id:member`) and
  labelling (`seat` / `seat:member`); `seat` defaults `'?'`; `phase` only
  when a string; unknown-`effect_id` events skipped; insertion order
  preserved; `attempts` counts starts; `startedAt` set on the first start
  only; terminals matched by `attempt_id` and a stale-attempt terminal
  ignored entirely; `turns` a max of numeric `cp.turn`; session from a
  `*-session-finished` step; member outcome overriding only through the
  `working|succeeded|failed|indeterminate` allowlist.
- **AC-3 — Σ aggregation.** A member-less participant collects the member
  participants of the same **effect occurrence**; cost aggregates only
  when its own is non-numeric and members have numeric
  `session.total_cost_usd`; same for turns; the Σ prefix and
  `*_aggregated` flags set; the non-aggregating case (parent has its own
  value) pinned; summation order is the participant insertion order.
- **AC-4 — Activity, all four branches.** Live `tool` (+ ` · ` +
  shortened target) while working with a `lastTurn`; else result token
  joined by ` · ` with the duration; else `<n> members ↓`; else the
  absence mark with `no activity recorded`. Start/end selection per the
  member and seat rules; `target_full` present only when shortened.
- **AC-5 — `fmt_dur` boundaries.** `<60s` → `Ns`; 59500ms → `1m00s`;
  `<60m` zero-pads seconds; `≥60m` → `Hh<MM>m` with seconds dropped;
  negative and unparseable → `None`.
- **AC-6 — `short_target` boundaries.** `≤44` verbatim; the 40-char
  accumulation boundary; single-segment; the `…/` prefix.
- **AC-7 — JS compatibility primitives.** `to_fixed_4(0.03125)` ==
  `"0.0313"` and a Σ sum case; `len`/`slice` on UTF-16 code units with an
  emoji astride the 110 boundary and a lone-surrogate case; the
  unconditional ellipsis on a short feature; `to_display(None)` ==
  `"undefined"`.
- **AC-8 — Inner topology.** No observed effect → no columns; newest
  effect wins; sequence steps only; `step:member` fork columns labelled
  by the step with nodes labelled by the member suffix; tags without step
  order → one parallel column; neither → one node labelled by the seat's
  participant label else the phase name; dedup and first-observed order;
  node keys `effect_id` / `effect_id:<tag>`; each node state through the
  `memberOutcome` → `stepDone` → `status` → `active` chain.
- **AC-9 — Phase rail.** First-visit order; visit counts from
  `phase/entered` with non-string names skipped; empty when there are
  none; `current` is `summary.phase` when it is a visited phase else the
  last visited; `plain` is no columns or exactly one column of one node.
- **AC-10 — Ruling and summary.** `ruling` is `None` unless
  `last_decision` is an object carrying `rule_id`; severity through the
  fixed table with an unlisted severity falling back to `""`; absent
  `from`/`next` render `?`; inputs as `k: <json(v)>`; `problem` only when
  present. `summary` carries all nine `summarize()` keys verbatim,
  including `cursor`.
- **AC-11 — Trail classification.** Per type: `transition/decided`,
  `phase/entered`, the three effect terminals (participant label else
  `effect_id` else `?`, result token, `error` clamped to 160), and the
  `rule_id`/`phase`/`reason` fallback chain ending in the type's second
  path segment. Default trail skips `effect/checkpointed`,
  `effect/requested`, `effect/started` via `in_trail`; a `__proto__`
  causation id resolves to no arrow; `what.text` is the plain composition
  of the structured parts.
- **AC-12 — Full-journal rows.** Label from the first non-empty of
  `rule_id`, `phase`, result token, `command`, `reason`; the
  `effect/checkpointed` override (`cp.tool ?? 'seat-turn'` for
  `seat-turn`, else `cp.step`, prefixed `<member> · `); `payload_json`
  from the derivation with a float-bearing payload golden-tested;
  `event_count` counts **all** events, unfiltered.
- **AC-13 — Scoping tags.** `Participant.phase` and `JournalRow.phases`
  reproduce both predicates exactly: seats by `effect.phase`; events by
  `payload.phase`, by `payload.from` (a decision leaving the phase — note
  `next` is deliberately not a match), or by `effect_id` requested in
  that phase. No surface implements the predicate.
- **AC-14 — Live lines.** The global last-start scan; `open` cleared only
  by a terminal matching both `effect_id` and `attempt_id`; labels from
  `p.seat` defaulting `'?'`; `label · turn N · tool` else
  `label · working`; the synthesized bare-seat row when the scan found
  nothing.
- **AC-15 — Run rows.** Newest first; status validated against
  `[completed, stopped, awaiting_operator, running]` into `status_known`;
  the **full** feature carried (the model stays terminal-agnostic and
  `--json` stays lossless); `count` present.
- **AC-16 — Endpoints.** `/api/view/<run>` returns the `RunView` for a
  known run, 404 for an unknown one and for a missing database, and is
  rejected by `request_allowed` for a non-loopback Host and a non-GET
  method. `/api/runs` answers with the reserialized rows.
  `/api/run/<id>`, `/api/session/<id>` and `/sse/<id>` keep their current
  responses — the existing `ui/tests.rs` assertions pass unedited.
- **AC-17 — The JS derivation is gone.** `ui.html` contains none of
  `buildParticipants`, `innerColumns`, `fmtDur`, `shortTarget`,
  `toFixed`, `Date.parse`, `JSON.stringify`, `innerHTML`. Asserted by a
  test over the `include_str!`'d page.
- **AC-18 — Console behaviour survives.** Each item the framing
  enumerates (§2) is argued against the enumerated list in the plan's
  parity table: collapsing runs pane and divider handle, collapsed
  one-line rows and their tooltip, exclusive scoping chip and
  `× show all`, Σ on parents, activity as `result · duration`, the trail
  default and `full journal · N events ▸` toggle, transcript drill with
  the checkpoint-table fallback, the terminal line, the fork/join graph
  with arrowed sequential edges and symmetric parallel lanes and the
  legend, live pulsing and the stateful favicon, selection survival and
  self-clearing across SSE re-renders, the 5s `loadRuns` poll, and the
  textContent-only discipline with class names from fixed allowlists.
- **AC-19 — `brokkr runs`.** Golden line output at several widths
  including the feature-clamp boundary and the width below which the
  feature column is dropped; newest-first order; no `N runs` trailer;
  `--json` emits `RunsView` verbatim.
- **AC-20 — `brokkr inspect`.** Golden readout: header, ruling line, park
  reason present and absent, the six-column seats table, the trail, and
  the tree with a fork, with a sequence, and with both. `--phase` and
  `--seat` each golden-tested; the two are rejected together by
  `ArgGroup`; `--seat` returns every matching occurrence and also matches
  an exact participant key; a non-matching value exits nonzero naming the
  valid values. `--json` emits the `RunView` verbatim with
  `view_version: 1`, and `brokkr inspect --json | jq .summary` reproduces
  today's `brokkr inspect` output verbatim.
- **AC-21 — `brokkr watch`.** `--once` golden frame (graph, seats, last
  ruling, live activity; no trail); a redraw triggered by a seq change
  and by a hash-only change; `--interval` floored at 100ms and rejecting
  garbage; non-tty appends timestamped frames with no ANSI; exit on
  `Completed`/`Stopped`/`AwaitingOperator` with 0/3/2 and the park reason
  printed before exit; a transient store error renders a frame rather
  than exiting; the store is opened read-only and no journal write
  occurs.
- **AC-22 — Terminal safety.** Golden tests with an ESC-bearing feature
  and a `\r`-bearing result token prove no control character reaches
  stdout and that width arithmetic ran on the sanitized text.
  `Safe`'s field is private and its only constructor sanitizes.
- **AC-23 — Width and colour.** `COLUMNS` of `""`, `"0"`, `"abc"` and
  `"100000"` each resolve to a value in `[20, 1000]`; a multi-byte
  feature truncates on a `char` boundary without panicking; colour is off
  under any of non-tty, `NO_COLOR`, `TERM=dumb` and on under none of
  them; `⑂`/`→` appear in every variant.
- **AC-24 — Gates.** `cargo test --workspace --all-features`,
  `cargo fmt --all --check`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, and `bash scripts/coverage-exact.sh`
  all pass. No entry is added to `[workspace.dependencies]`;
  `contracts/`, `policy/phase-machine.json`, `reference/` and `fixtures/`
  are unmodified.
