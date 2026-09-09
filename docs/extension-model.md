# Extension model — nodes, seats, and what may never be unplugged

**Status**: partially accepted. Decisions 0002 and 0003 lock the outer-machine,
runtime, and extension boundaries; the remaining deferred choices are listed
below.

Two different things get called "adding an agent", and they extend at two
different layers. Keeping them separate is the core of the model.

## Layer 1 — Phases (nodes of the outer machine)

A **phase** is a node in `policy/phase-machine.json` plus a registered
**executor** that knows how to run it. Adding a phase (e.g. `docs`,
`benchmark`, `canary`) means:

1. A table diff: the new phase in `phases`, its rules spliced into the
   ordered rule list (deny rules first), reviewed like any policy change.
2. An executor definition in the pinned Brokkr bundle. It names a declarative
   inner topology and its result schema. Executable leaf capabilities run out
   of process through `forge-driver/v1`, so an extension can use any language
   without entering the authoritative process.

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
| `trust` | `trusted` / `policy-confined` / `public-evidence-only` — decides what is mounted inside the wall. The wall itself is the realm's `boundary` (decision 0046: `namespace`, `seatbelt`, `container`, `harness` or `open`, declared in `realms.json` and never by a seat); the tier decides what the engine mounts into it. |
| `result_schema` | The typed result the seat must return (decision 0001 governs violations). |
| `driver` | Which harness runs it (`surface`, `cordis`, `claude-code`, `claude-lanetally` — the Claude Code harness through LaneTally's session-capture wrapper: `total_cost_usd` stays the harness-reported list price, capture makes the session priceable in the LaneTally ledger, and the per-session actual-cost join is deferred until readplane exposes a session query — `codex`, `fake`, or another protocol-conformant driver). |

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
overrides, workspace conventions. A host workspace's profile stays in
that workspace; this repo ships the example via `brokkr init` and the
recipe library (decision 0010). Profiles are declarative bundle
content with stable ids and content digests, not imported executable plugins.

## Resolved

1. **Phases are a list, not a DAG** — ruled 2026-08-21, see
   [decision 0002](decisions/0002-linear-outer-machine.md). The outer
   machine stays a linear FSM (constitutional). Concurrency exists in
   exactly two sanctioned forms: seat/sub-machine parallelism inside
   executors, and auxiliary tracks that join at barriers (default: ship,
   via the `aux-tracks-joined` precondition) and can never cause a
   transition. Speculative verify/review overlap and per-repo phase
   progression are pre-emptively rejected.

2. **The production extension boundary is declarative data plus isolated
   drivers** — ruled 2026-08-22, see
   [decision 0003](decisions/0003-native-rust-runtime.md). Brokkr ships as one
   native Rust executable. Third-party harness code speaks a versioned
   protocol out of process; containers are optional seat isolation.

3. **Sub-machines are declarative inner topologies** — inner loops are
   event-sourced graphs built from bounded primitives such as seats, joins,
   loops, gates, tools, and nested machines. They emit one schema-bound result
   to the linear outer FSM.

4. **Seat identity belongs to the pinned Brokkr bundle** — every seat has a
   stable id used by the journal, driver protocol, and LaneTally joins.
   LaneTally owns cost and funding truth, not orchestration identity.

5. **Version identity is content-addressed** — a run pins engine, event,
   database and driver protocol versions plus digests for its policy, topology,
   profile, roles, schemas, executors, and container images. Resume uses the
   exact bundle or refuses.

## Open questions for discussion

1. **Authored policy syntax**: retain JSON, adopt TOML, or accept both and
   compile to one canonical representation?
2. **Signing and anchoring**: how are operator keys distributed, and which
   external anchor supplements the repository digest?
3. **Remote execution**: what transport carries `forge-driver/v1` when a seat
   is not local?
4. **Store expansion threshold**: what demonstrated multi-host requirement is
   sufficient to add a PostgreSQL event-store implementation?

The complete accepted blueprint is in
[target-architecture.md](target-architecture.md).
