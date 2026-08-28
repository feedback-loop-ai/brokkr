# Change Proposal: one-derivation-two-surfaces

## Why

The console's UX pass produced genuine domain logic — which member
concludes when, what a phase's traffic is, what a seat cost, when an
absence is deliberate — and all of it is JavaScript inside `ui.html`,
where **none of it runs under the coverage gate**. The CLI never got
that pass: `forge runs` prints each run's entire feature text and
`forge inspect` dumps raw `RunState` JSON, which is the readout the
operator actually has while a forge runs over SSH. Building terminal
views on a second, hand-written derivation would fork the answer to
"what did this seat cost" into two implementations that drift — the
failure mode this project exists to refuse.

Decision 0013 (`docs/decisions/0013-one-derivation-two-surfaces.md`,
accepted 2026-08-29) rules the alternative: **derivation is Rust,
rendering is per-surface.** This change implements that ruling in full —
the extraction, both terminal tiers, and the deletion that makes them
honest.

## What Changes

- **`forge-view` — a new crate, not a module.** Its `Cargo.toml`
  depends on exactly `forge-core`, `serde` and `serde_json`, so the
  ruling's "no I/O, no rendering, no terminal or DOM concepts" is a
  compile error rather than a review convention, and the absence of
  `time` forces `now` to be a parameter — which is what makes the golden
  tests deterministic. Derivation reads `serde_json::Value` with the
  same `typeof` guards the JavaScript uses, never typed payload structs:
  serde defaults and drops are repair (decision 0001). Every displayed
  scalar reaches the model as a **(structured value, rendered text)
  pair**, because the console's renderer is JavaScript and cannot call a
  Rust helper — a model carrying only raw values would force the page to
  re-derive its formatting, reintroducing the duplication in the commit
  that claims to remove it. A small `js.rs` closes five places where the
  obvious Rust is not the JavaScript, the sharpest being `toFixed`:
  `0.03125` is exactly representable and renders `$0.0313` on today's
  console but `$0.0312` from `format!("{:.4}")` — the new surface
  disagreeing with the old one about money.
- **The console keeps painting only.** `forge ui` gains one route,
  `/api/view/<run>`, carrying summary, ruling, participants, live lines,
  phases and one journal array whose rows are tagged `in_trail` and with
  their scope membership; `/api/runs` is reserialized from the run-row
  model; `/api/run/<id>`, `/api/session/<id>` and `/sse/<id>` are
  untouched and their tests are the parity baseline. `ui.html`'s
  derivation is **deleted** — `buildParticipants`, `innerColumns`,
  `fmtDur`, `shortTarget`, the activity and trail logic, Σ, the phase
  derivation, the live scan and `runs.reverse()` — while SVG geometry,
  DOM building and interaction stay client-side. A test over the
  `include_str!`'d page fails if any of it returns, under the stated
  rule that **the page may branch on a model field but may not compute
  one**. The `textContent`-only discipline is unchanged and class names
  still come from fixed allowlists, now as closed-set keys the page maps
  through its own table.
- **CLI static renderers from the same models.** `forge runs` becomes
  one clamped line per run, newest first (the `N runs` trailer moves to
  `--json`). `forge inspect` becomes a human readout — header, ruling
  line, park reason, the console's six-column seats table, the decision
  trail, and the phase graph as a terminal tree where `⑂` precedes
  parallel members and `→` precedes sequential steps — with
  `--phase`/`--seat` as the scoping verbs the console's clicks became,
  mutually exclusive, matching every occurrence, and exiting nonzero
  when nothing matches rather than printing an empty table that reads as
  "this phase did nothing". Every such command keeps `--json`, emitting
  the view model verbatim with `view_version: 1` and today's nine
  `summarize()` keys preserved under `summary`.
- **`forge watch <run>`** polls `head_hash` (comparing seq **and** hash,
  so a rewritten journal redraws rather than sitting blind) and redraws
  a frame that is `inspect` without the trail; `--once` prints one
  frame; `--interval` tunes the poll at the existing 100ms floor; a
  non-tty stdout appends timestamped frames so pipes and CI logs stay
  useful; `SQLITE_BUSY` is a frame, not an exit; it exits on
  `status != Running` — a park included, because a park admits no
  further events and "keep watching" is an unbounded CI hang — with
  `finish()`'s 0/2/3/1 codes.
- **Terminal safety, the CLI's twin of the console's XSS discipline.**
  The journal is seat-authored, and printed straight to a tty a
  `\r`-bearing result token can forge a ruling line — continuously,
  under a `watch` that redraws. Renderers interpolate a `Safe(String)`
  newtype whose only constructor strips C0/C1 controls, applied before
  any width arithmetic. Colour is bare ANSI gated on `IsTerminal`,
  `NO_COLOR` and `TERM=dumb`, applied as a post-processing wrap; `⑂` and
  `→` are unconditional, because the pre-baked text already carries `Σ`,
  `↓`, `…` and `—` and an ASCII mode would need a second derivation of
  every one of them.
- **No new dependencies.** The only manifest additions are the
  `forge-view` path entries in `[workspace] members` and
  `[workspace.dependencies]`. Frozen contracts v1,
  `policy/phase-machine.json`, `reference/` and the `fixtures/`
  evaluator corpus are untouched: view models are derived output, not
  journal or protocol schema.

Design artifacts:

- [specs/one-derivation-two-surfaces/spec.md](../../../specs/one-derivation-two-surfaces/spec.md)
  — WHAT and WHY, the normative models, the ported-rule divergence
  table, and 24 acceptance criteria.
- [specs/one-derivation-two-surfaces/plan.md](../../../specs/one-derivation-two-surfaces/plan.md)
  — HOW: the position reconciliation (fourteen numbered rulings, with
  what was adopted, rejected and reconciled), the four movements, the
  files touched, the console parity table, and the risk register.
- [specs/one-derivation-two-surfaces/tasks.md](../../../specs/one-derivation-two-surfaces/tasks.md)
  — twenty-two ordered tasks, each paired with the test that proves it.

## Impact

- **New**: `crates/forge-view/` (`Cargo.toml`, `src/lib.rs`,
  `src/js.rs`, `src/tests.rs`, `src/js/tests.rs`);
  `crates/forge-cli/src/render.rs` + `src/render/tests.rs`.
- **Edited**: `Cargo.toml` (`[workspace] members`,
  `[workspace.dependencies]` — path entries only),
  `crates/forge-cli/Cargo.toml`, `crates/forge-cli/src/ui.rs` (one route
  arm, `/api/runs` reserialized, module doc),
  `crates/forge-cli/src/ui.html` (a net deletion of roughly 350 lines of
  derivation, consumption added), `crates/forge-cli/src/ui/tests.rs`,
  `crates/forge-cli/src/main.rs` (`Cmd::Runs`/`Cmd::Inspect` rewritten,
  `Cmd::Watch` added, the style/width detection and the one clock read),
  `crates/forge-cli/src/tests.rs`,
  `crates/forge-cli/tests/machine_proof.rs` (three `forge runs` pins
  migrate to `--json`, never to human output), `README.md`,
  `ARCHITECTURE.md`, `docs/target-architecture.md`.
- **Untouched**: `forge-core`, `forge-store`, `forge-protocol`,
  `forge-runtime`, `forge-bridge`; the `/api/run/<id>`,
  `/api/session/<id>` and `/sse/<id>` responses and their tests; frozen
  contracts v1; `policy/phase-machine.json`; `reference/`; the frozen
  evaluator corpus; `forge costs`, `compare`, `export`, `replay`,
  `verify-run`, `anchor` and the bridge; the engine, reducer, evaluator,
  journal schema and checkpoint vocabulary. No new dependency, no new
  runtime, no TUI framework, no panes or keyboard navigation.
- **Operational, and deliberately breaking**: `forge runs` and
  `forge inspect` change default output shape — human first, machines
  move to `--json`, which decision 0013 accepts by name.
  `forge inspect --json | jq .summary` reproduces today's `forge inspect`
  output verbatim, all nine keys including `cursor`. The console's
  observable behaviour is unchanged. Two honest admissions ship with it:
  without a Unicode-width dependency, CJK and emoji columns misalign,
  and a `watch` frame taller than the terminal shows its tail.
- **The point**: display truth lands under `scripts/coverage-exact.sh` —
  literal 100% line, branch and function equality — where it has never
  been before.
