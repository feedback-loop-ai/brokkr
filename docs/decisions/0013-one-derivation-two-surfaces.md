# 0013 — One derivation, two surfaces: `forge-view` and the terminal readout

Status: accepted (operator ruling in chat, 2026-08-29)

## Context

The read-only console gained real derivation logic during the UX pass:
a participant model (per-seat status, attempts, turns, cost, activity
as result token plus wall-clock duration), Σ-aggregation for panel and
sequence parents, phase scoping, the decision-trail filter, and the
inner fork/join topology. All of it lives as JavaScript inside
`ui.html`. It is genuine domain logic — which member concludes when,
what counts as a phase's traffic, when an absence is deliberate — and
**none of it runs under the coverage gate**.

Meanwhile the CLI never received that pass. `forge runs` prints each
run's entire feature text (thousands of characters per row); `forge
inspect` dumps raw `RunState` JSON with the feature inline. The
operator sits in a terminal while a forge runs — over SSH, in tmux —
and that is precisely where the readout is worst.

Building terminal views on a second, hand-written derivation would
fork the answer to questions like "what did this seat cost": two
implementations, drifting. That is the failure mode this project
exists to refuse.

## Decision

**Derivation is Rust, rendering is per-surface.**

1. **`forge-view`** — pure functions over `&[Envelope]` (plus the
   folded `RunState`) producing serializable view models: run rows,
   run summary and ruling, participants (label, member, phase, status,
   attempts, turns, cost, activity, session id), decision-trail
   entries, and inner topology columns (sequence steps, panel members).
   No I/O, no rendering, no terminal or DOM concepts. Crate or module
   is a design choice; single implementation is not.
2. **The console keeps painting only.** `forge ui` serves the view
   models; `ui.html` consumes them for pixels (SVG geometry, DOM,
   interaction). Its JS derivation is deleted, not duplicated. Every
   behavior the UX pass established must survive unchanged — the
   collapsing run pane, phase and participant scoping with one
   exclusive chip, Σ aggregation, activity as `result · duration`,
   the decision trail with its raw-journal toggle, transcript drill,
   fork/join graph, live pulsing.
3. **The CLI renders text from the same models** (tier a): `forge
   runs` as one clamped line per run; `forge inspect` as a human
   readout — ruling line, seats table, decision trail, and the phase
   graph as a terminal tree where `⑂` marks a parallel fork and `→` a
   sequential step — with `--phase` and `--seat` as the scoping verbs
   the console's clicks became. Every such command keeps a `--json`
   form emitting the view model verbatim; scripts use that.
4. **`forge watch <run>`** (tier b): a live readout that redraws on
   journal-head change — graph, seats, last ruling, live seat
   activity — exits when the run reaches a terminal status, `--once`
   prints a single frame, and a non-tty stdout degrades to appended
   frames so pipes and CI logs stay useful.

### Constraints

- **No new dependencies.** Color via bare ANSI, gated on
  `std::io::IsTerminal`, `NO_COLOR`, and `TERM=dumb`; width from
  `COLUMNS` with a sane default. A full TUI framework (panes,
  keyboard navigation) is explicitly NOT in scope and would need its
  own decision.
- **Read-only.** These surfaces never write the journal, exactly as
  decision 0003 rules for `forge ui`.
- **Renderers are pure string functions**, golden-tested; the view
  derivations are unit-tested. This is the point: display truth lands
  under the coverage gate.
- Journal disciplines are unchanged and inherited — file-path-only
  targets, no commands or prose, secret masking (0012).

## Consequences

- One answer to every display question, in one place, tested once.
- The operator gets a glanceable terminal readout during a run; the
  console keeps deep interactive drill. Neither is the other's poor
  cousin.
- `forge runs` default output changes shape (human first, `--json` for
  machines). Acceptable: the tool is young and operator-ruled.
- A future TUI, if ever wanted, renders the same models — the work is
  not thrown away.
