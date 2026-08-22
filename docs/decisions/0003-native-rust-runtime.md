# 0003 — The production runtime is one native Rust binary

**Status**: accepted (operator ruling, 2026-08-22)

## Ruling

Forge's production control plane will be a native Rust application shipped as
one prebuilt executable. It will embed its local web UI and a bundled SQLite
runtime. Running Forge locally must not require Python, Node, Docker, a database
server, a message broker, or an always-on daemon.

The control plane runs on the host by default. Containers are an optional
execution boundary for policy-confined seats and CI deployments, not the
default home of the coordinator.

Customization is data first:

- policies, inner topologies, profiles, seats, roles, budgets, and result
  schemas live in versioned declarative bundles;
- the policy and topology languages are deterministic and non-Turing-complete;
- executable extensions run out of process behind a versioned,
  language-neutral driver protocol; and
- no third-party code is dynamically linked into the authoritative process.

Cordis/DeepSeek Harness is an optional driver for a seat or inner sub-machine.
It is not the Forge kernel and has no authority over outer transitions.

The current Python evaluator remains the executable policy specification and
parity oracle during the port. It is not the target production runtime.

## Why

The primary constraints are operational weight, power, robustness,
customization UX, and auditability. Implementation speed is explicitly not a
selection criterion.

- A native executable gives the smallest installation and failure surface.
- Rust makes illegal ownership and lifecycle states harder to represent at the
  process-supervision and persistence boundaries.
- SQLite supplies a transactional, portable event store without an external
  service.
- Declarative graphs can be linted, hashed, diffed, signed, visualized, and
  evaluated. Behavior hidden in an in-process plugin cannot offer the same
  audit surface.
- Out-of-process drivers isolate harness crashes and allow implementations in
  any language without adding their runtimes to the Forge core.
- Keeping containers at the seat boundary preserves native access to Git and
  local tools while allowing confinement where trust requires it.

Raw coordinator throughput is not the reason to choose Rust: LLM calls, Git,
tests, networks, and sandboxes dominate elapsed time. The reason is to make the
installed product and its trusted computing base small and predictable.

## Constitutional boundaries

1. The pure reducer performs no I/O, clock reads, randomness, model calls, or
   process execution.
2. An agent or driver returns evidence and a typed result; only the pinned
   policy may select an outer transition.
3. Third-party executable code never runs in the authoritative process.
4. An active run never silently changes its policy, topology, prompt, schema,
   profile, driver protocol, or executor version.
5. Replaying the same valid journal under its pinned bundle produces the same
   state and rulings.
6. The browser UI submits commands and reads projections; it never writes
   authoritative state directly.
7. The outer machine remains the linear FSM established by decision 0002.
   Graph-shaped orchestration lives inside a phase or in an auxiliary track.

## Consequences

- The repository will become a Rust workspace and will produce a single
  `forge` binary. Multiple internal crates are permitted; multiple mandatory
  services are not.
- SQLite is the initial single-host store. Multi-host high availability is not
  implied; if it becomes a real requirement, a remote store can implement the
  same event contract without changing reducer semantics.
- The runtime store is optimized for atomicity and queries. Canonical NDJSON
  export plus an artifact manifest is the portable, human-auditable form.
- Every external effect is requested durably before execution and completed,
  failed, cancelled, or marked indeterminate by a later event.
- Python and JavaScript reference implementations are retired only after
  differential and replay parity tests pass.
- The UI is compiled at release time and embedded as static assets. Node is a
  build dependency for the UI, never an end-user runtime dependency.

## Explicit non-goals for the first production release

- no Kafka, Redis, Temporal, or required PostgreSQL service;
- no Electron or Tauri desktop shell;
- no arbitrary scripts inside transition conditions;
- no in-process native plugin ABI;
- no requirement that users install a Rust toolchain; and
- no attempt to make stochastic LLM execution reproducible from a prompt.

The detailed component boundaries, event contract, bundle shape, driver
protocol, UI, and delivery sequence are specified in
[the target architecture](../target-architecture.md).
