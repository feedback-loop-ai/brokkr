# Tasks: One derivation, two surfaces

**Feature slug**: `one-derivation-two-surfaces`
**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Ordered; each task names the test that proves it. Acceptance-criteria
numbers (AC-n) refer to spec.md's `## Acceptance Criteria`. Every task
lands with its tests in the same commit — the coverage gate is the point
of this feature, not an afterthought at the end.

## Movement 1 — the crate

- [x] **T1 — `crates/forge-view` joins the workspace.** `Cargo.toml`
  with `[dependencies]` exactly `forge-core`, `serde`, `serde_json`; no
  `time`, no `forge-store`. `[workspace] members` and
  `[workspace.dependencies]` gain the path entry (no third-party
  dependency is added anywhere).
  *Proven by*: AC-1 — the anti-drift test asserting `std::fs`,
  `std::env`, `IsTerminal`, `print!`/`println!` are absent from
  `crates/forge-view/src/*.rs`, and `cargo test --workspace` compiling
  with the manifest as written.

- [x] **T2 — `js.rs`: the five compatibility primitives.**
  `to_fixed_4` (ties away from zero), `len`/`slice` (UTF-16 code units,
  never splitting a surrogate pair, dropping a lone leading surrogate
  rather than inventing U+FFFD), `round_half_up`, `to_display` (JS
  string conversion, including the literal `undefined`).
  *Proven by*: AC-7 — `to_fixed_4(0.03125) == "0.0313"` (the console's
  value; `format!("{:.4}")` gives `0.0312`), `0.00005`, and a Σ sum;
  `len`/`slice` with an emoji astride the 110 boundary and a `é`-bearing
  target; `to_display(None) == "undefined"`.

- [x] **T3 — Models and `Cell`.** All types from spec.md's normative
  block, `Serialize` only, absent as `null` never skipped,
  `view_version: 1`.
  *Proven by*: the serialized-JSON assertions that every later task
  writes (view-model tests assert on JSON strings, not struct equality,
  so no derived `Debug` sits uncovered).

- [x] **T4 — `participants()`.** Keying, labelling, insertion order
  (`Vec` + index map), `attempts`, first-`startedAt`, terminal matching
  by `attempt_id` with stale attempts ignored entirely, `turns` as a
  max, session capture, member-outcome override through the four-value
  allowlist, effect identity as the request occurrence.
  *Proven by*: AC-2 — one test per rule, including the stale-attempt
  terminal (a port matching on `effect_id` alone marks a retrying seat
  concluded) and a `working` member outcome after the effect concluded.

- [x] **T5 — Σ aggregation.** Members of the same effect **occurrence**;
  cost and turns aggregate only when the parent's own value is
  non-numeric; `Σ ` prefix; `*_aggregated` flags; sum in insertion order.
  *Proven by*: AC-3 — the aggregating case, the non-aggregating case,
  and a two-member `f64` sum whose `to_fixed_4` is order-sensitive.

- [x] **T6 — `fmt_dur`, `short_target`, `absent` cells.**
  *Proven by*: AC-5 (`<60s`, 59500ms → `1m00s`, zero-padded seconds,
  `≥60m` dropping seconds, negative and unparseable → `None`) and AC-6
  (`≤44`, the 40-char accumulation boundary, single-segment).

- [x] **T7 — `Activity`, all four branches.** Live tool (+ shortened
  target with `target_full` only when shortened), result·duration,
  `<n> members ↓`, the absence mark; member and seat start/end rules.
  *Proven by*: AC-4 — one test per branch plus the member-without-
  `panel-member-finished` end rule.

- [x] **T8 — `LiveLine`.** The separate global-last-start scan, `open`
  cleared only on matching `effect_id` **and** `attempt_id`, `p.seat`
  truthiness defaulting `'?'`, the synthesized bare-seat row, and the
  raw `turn`/`tool` rendering.
  *Proven by*: AC-14 — including the interleaved-effects case where the
  live block disagrees with per-effect status, and the `turn undefined`
  quirk named by its test.

- [x] **T9 — `inner_columns()` over a one-pass `effect_id` bucketing.**
  Newest requested effect wins; finished step order; member tags deduped
  in first-observed order; the three shapes (steps, `step:member` forks,
  tags-only, bare seat); node keys and the state chain.
  *Proven by*: AC-8 — one test per shape and per state-chain arm, plus a
  newest-wins test with two `effect/requested` events for one phase.

- [x] **T10 — Phase rail.** First-visit order, visit counts, non-string
  names skipped, empty when none, `current` selection, `plain`.
  *Proven by*: AC-9.

- [x] **T11 — Trail and journal rows.** `in_trail`, causation via a
  total map (no seq-0 truthiness — `verify_chain` pins `seq == i + 1`),
  per-type classification with the fallback chain, `What.text` as the
  plain composition, the full-journal label with its
  `effect/checkpointed` override, `payload_json`, `event_count`.
  *Proven by*: AC-11 and AC-12 — one test per event type, the fallback
  chain, a `__proto__` causation id producing no arrow, and a
  float-bearing payload pinning the `1.0` vs `1` delta.

- [x] **T12 — Scope tags.** `Participant.phase`; `JournalRow.phases`
  from `payload.phase`, `payload.from`, and effect membership.
  *Proven by*: AC-13 — both predicates, including a decision leaving a
  phase (and the negative: `next` is not a match).

- [x] **T13 — `Summary`, `Ruling`, `run_rows()`, `clamp()`, `age()`.**
  Nine `summarize()` keys verbatim; ruling empty unless an object with
  `rule_id`; the severity table with its `""` fallback; `?` fallbacks;
  inputs lines; newest-first rows with `status_known` and the full
  feature; `now` as a parameter.
  *Proven by*: AC-10 and AC-15.

## Movement 2 — the console

- [x] **T14 — `/api/view/<run>` and the `/api/runs` reserialization** in
  `ui.rs`; module doc route list updated.
  *Proven by*: AC-16 — the new endpoint's shape, 404 on an unknown run
  and a missing database, `request_allowed` rejecting a non-loopback
  Host and a non-GET method, and the existing `ui/tests.rs` assertions
  for `/api/run/<id>`, `/api/session/<id>` and `/sse/<id>` passing
  **unedited**.

- [x] **T15 — `ui.html`: derivation deleted, consumption added, one
  commit.** `buildParticipants`, `innerColumns`, `fmtDur`,
  `shortTarget`, the activity/duration/target logic, the trail
  classification, Σ, the phase/visit derivation, the live scan and
  `runs.reverse()` all go; SVG geometry, DOM building and interaction
  stay; `*_class` keys map through the page's own fixed table before
  reaching `className`.
  *Proven by*: AC-17 — the banned-token test over the `include_str!`'d
  page (`buildParticipants`, `innerColumns`, `fmtDur`, `shortTarget`,
  `toFixed`, `Date.parse`, `JSON.stringify`, `innerHTML`) — and AC-18,
  the parity table in plan.md argued item by item.

## Movement 3 — the CLI

- [x] **T16 — `render.rs` foundations.** `Safe(String)` with a private
  field and a sanitizing constructor (C0/C1 and `\x7f` stripped before
  any width math); `Style { color, width }` with `COLUMNS` parsed to
  `Option`, defaulted 80, clamped `[20, 1000]`; colour as a
  post-processing wrap gated on `is_terminal && !NO_COLOR && TERM !=
  "dumb"`; `char`-boundary truncation.
  *Proven by*: AC-22 (ESC-bearing feature, `\r`-bearing result token,
  width math on sanitized text) and AC-23 (`""`/`"0"`/`"abc"`/
  `"100000"`, a multi-byte truncation that does not panic, colour on and
  off, `⑂`/`→` present in every variant).

- [x] **T17 — `render::runs()` and `Cmd::Runs`.** One clamped line per
  run, newest first, batch-sized columns, feature dropped below 8
  remaining columns, no trailer; `--json` emits `RunsView`.
  *Proven by*: AC-19 — goldens at several widths including the clamp
  boundary and the drop boundary.

- [x] **T18 — `render::inspect()` and `Cmd::Inspect`.** Header, ruling
  line, park reason when present, six-column seats table, decision
  trail, and the phase tree (`→` for a single-node column, `⑂` for a
  fork, the marker alone when a fork column has no label).
  `--phase`/`--seat` in a clap `ArgGroup`; `--seat` matching every
  occurrence by label or exact key; a non-matching value exiting nonzero
  with the valid values; `--json` emitting the `RunView`.
  *Proven by*: AC-20 — goldens for the full readout, each scope flag,
  the tree with a fork, with a sequence and with both; the mutual-
  exclusion rejection; the no-match nonzero; and the equality
  `forge inspect --json | jq .summary` == today's `forge inspect`.

- [x] **T19 — Migrate the `machine_proof.rs` pins** at ~:537-557,
  ~:570-585 and ~:1208-1215 from the five-column tab shape and the
  `"1 runs"` trailer to `--json`, not to human output.
  *Proven by*: AC-19 and AC-24 — `cargo test --workspace --all-features`
  green with the migrated assertions.

## Movement 4 — `watch`

- [x] **T20 — `Cmd::Watch`.** Poll `head_hash` comparing seq **and**
  hash; redraw `inspect(.., trail = false, ..)`; `--once`; `--interval`
  with the existing 100ms floor and garbage rejected; tty redraw via
  `\x1b[2J\x1b[H`, non-tty appending timestamped frames with no ANSI;
  transient store errors rendered as a frame; exit on `status !=
  Running` with `finish()`'s 0/2/3/1 and the park reason printed first;
  store opened read-only.
  *Proven by*: AC-21 — the `--once` golden frame, a seq-change and a
  hash-only-change redraw, the interval floor and rejection, the non-tty
  append, each exit status and code, a transient-error frame, and an
  assertion that the journal is unchanged after a watch.

## Movement 5 — docs and gates

- [ ] **T21 — Docs.** `README.md` (the `crates/` row at :106 and the
  command listing), `ARCHITECTURE.md` crate listing,
  `docs/target-architecture.md` :361 — `forge-view`, `forge watch`, and
  the changed `runs`/`inspect` shapes, including the honest note that
  CJK and emoji columns misalign without a width dependency.
  *Proven by*: inspection, plus AC-24.

- [ ] **T22 — Gates green.** `cargo fmt --all --check`; `cargo clippy
  --workspace --all-targets --all-features -- -D warnings`; `cargo test
  --workspace --all-features`; `bash scripts/coverage-exact.sh`.
  *Proven by*: AC-24 — literal 100% line/branch/function equality with
  no `coverage(off)` attribute anywhere, no entry added to
  `[workspace.dependencies]` beyond the `forge-view` path, and
  `contracts/`, `policy/phase-machine.json`, `reference/` and
  `fixtures/` unmodified in the diff.
