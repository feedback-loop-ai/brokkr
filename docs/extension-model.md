# Extension model — nodes, seats, and what may never be unplugged

**Status**: draft for discussion (2026-08-21). This is the proposal seeded by
the extraction conversation; nothing here is locked.

Two different things get called "adding an agent", and they extend at two
different layers. Keeping them separate is the core of the model.

## Layer 1 — Phases (nodes of the outer machine)

A **phase** is a node in `policy/phase-machine.json` plus a registered
**executor** that knows how to run it. Adding a phase (e.g. `docs`,
`benchmark`, `canary`) means:

1. A table diff: the new phase in `phases`, its rules spliced into the
   ordered rule list (deny rules first), reviewed like any policy change.
2. An executor registered under the phase's name — via Python entry point
   (`[project.entry-points."forge.executors"]`) so third-party packages can
   ship executors without forking the engine.

Removing a phase is a table diff that re-routes the rules around it. The
engine never hardcodes the phase sequence; it only knows `initial`,
`terminal`, and the table.

**Guardrails** (policy lint, enforced at load time, tested by the sweep
tests):

- Totality: every `(phase, result)` an executor can emit has a rule, or the
  run parks (decision 0001 makes missing rules safe, lint makes them rare).
- Reachability: no orphan phases, no rule referencing an unregistered
  executor.
- **Protected invariants**: some properties are constitutional, not
  editable-by-table-diff alone — e.g. "a path to `ship` passes through a
  concluded review with security riding along" and "security-hold routes to
  a terminal or operator state". The lint asserts these *semantically*
  (graph analysis over the table), so a plausible-looking table edit cannot
  silently disarm them. Changing a constitutional rule requires editing the
  lint too — a two-file diff that no single "helpful" edit produces.

## Layer 2 — Seats (agents inside a phase)

A **seat** is one agent session an executor spawns: council seat, wave
implementer, review-panel dimension, verification track, security reviewer.
Seats are **data, not code**: a seat definition bundles

| Field | Meaning |
|---|---|
| `role` | Charter prompt template (the `vf-*` charters become these). |
| `class` | Capacity class, not a model name: `background` / `workhorse` / `frontier` (LaneTally resolves the actual provider; model-blindness is wire-level). |
| `trust` | `trusted` / `policy-confined` / `public-evidence-only` — decides what the engine mounts into the sandbox. |
| `result_schema` | The typed result the seat must return (decision 0001 governs violations). |
| `driver` | Which harness runs it (`surface`, `claude-code`, `codex`, `fake`). |

Executors consume *seat sets* from configuration: the review executor runs
"one seat per repo × dimension in `review.dimensions`"; the council executor
runs "the seats listed in `council.seats`". So:

- **Adding an agent** = adding a seat definition (a prompt file + a config
  entry). Example: a fourth review dimension `accessibility`, or a fifth
  council seat with a performance-contrarian charter. No engine change.
- **Removing an agent** = removing the config entry — *except* protected
  seats. The security review dimension is non-removable by the same
  constitutional-lint mechanism as Layer 1; profiles may add dimensions but
  not delete that one.

## Layer 3 — Profiles (the stack-specific bundle)

A **profile** packages what a given codebase needs: contract-check
definitions, verification tracks, the security-catalog overlay, seat-set
overrides, workspace conventions. Alkemio's profile stays private in
agents-hq; the public repo ships an example. Discovery via entry points,
same as executors.

## Resolved

1. **Phases are a list, not a DAG** — ruled 2026-08-21, see
   [decision 0002](decisions/0002-linear-outer-machine.md). The outer
   machine stays a linear FSM (constitutional). Concurrency exists in
   exactly two sanctioned forms: seat/sub-machine parallelism inside
   executors, and auxiliary tracks that join at barriers (default: ship,
   via the `aux-tracks-joined` precondition) and can never cause a
   transition. Speculative verify/review overlap and per-repo phase
   progression are pre-emptively rejected.

## Open questions for discussion

2. **Sub-machines**: inner loops (waves × contracts × fix rounds) should
   graduate from executor code to their own small tables — same format,
   journaled as sub-events. When?
3. **Seat identity in the ledger**: seats need stable ids for LaneTally
   run-id joins and for eval-earned substitution ("this seat, at this class,
   succeeded N of M times at cost X"). Where does the seat registry live —
   engine config or LaneTally?
4. **Versioning**: a table/schema/seat-set version triple pinned in the
   journal header, so a resumed run replays under the policy it started
   with, not the policy that exists now.
