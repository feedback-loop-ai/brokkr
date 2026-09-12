# Architecture

**Status**: the system as implemented. The blueprint it grew from is
[docs/target-architecture.md](docs/target-architecture.md); where this
page and a numbered [decision](docs/decisions/) disagree, the decision
wins.

Brokkr is a deterministic process manager wrapped around stochastic,
fallible effects. Agent sessions are leaves. Their outputs are typed
results. Only a pinned policy table ever selects a transition, and
every claim the system makes — done, verified, parked, stopped, paid —
is a journaled fact that replays byte for byte.

## The shape

```mermaid
flowchart TB
  operator([operator]) -- "run · resume · retry · stop" --> cli
  subgraph binary["the brokkr binary — how far a delivery advances"]
    cli["brokkr-cli<br/>commands · embedded UI · driver entry"]
    runtime["brokkr-runtime<br/>engine loop · bundles · recovery · the boundary"]
    core["brokkr-core — PURE<br/>envelope · hashing · fold · policy"]
    store[("brokkr-store<br/>SQLite journal, hash-chained")]
    protocol["brokkr-protocol<br/>forge-driver/v1 over stdio"]
    view["brokkr-view — PURE<br/>one display derivation"]
    bridge["brokkr-bridge<br/>dispatch bridge over verified journals"]
    cli --> runtime & view & bridge
    runtime --> core & store & protocol & view
    view --> core
    bridge --> store & runtime
    store --> core
  end
  protocol -- "NDJSON" --> harness([Claude Code · Codex · dsh · exec<br/>capability, as leaf effects])
```

Every edge is a real dependency; all seven crates are drawn. Transitive
edges are omitted (decision 0037).

`brokkr-core` performs no I/O, clock reads, randomness or process execution.
The same journal and bundle produce the same state and ruling. Effectful work
sits above it and is journaled. `brokkr-view` is also pure: one display
answer rendered as HTML or terminal text. Its dependencies are restricted to
`brokkr-core`, `serde` and `serde_json` (decision 0013).

Brokkr decides how far delivery advances. Product priorities are decided above
it; costs are measured beside it from seat ids and checkpoints; harnesses below
supply capability. None overrides another layer's authority.

## The journal is the run

```mermaid
flowchart LR
  e1["seq 1<br/>run/started"] --> e2["seq 2<br/>phase/entered"] --> e3["seq 3<br/>effect/requested"] --> en["seq n<br/>…"]
  e2 -. "previous_hash" .-> e1
  e3 -. "previous_hash" .-> e2
  en -. "previous_hash" .-> e3
  en ==> fold{{"fold(events)"}} ==> state["RunState<br/>phase · control status · protocol cursor · counters"]
```

Every event is a hash-chained envelope
([contracts/event-envelope.v1.schema.json](contracts/event-envelope.v1.schema.json)):
`seq` contiguous from 1, `previous_hash` chaining, `event_hash` over the
canonical bytes, `causation_id` naming the event that caused it. The
fold derives everything, and an event impossible at the current cursor
fails it closed: a journal that violates the protocol is corrupt, not
reinterpretable. Replay is byte-deterministic. Resume is replay.

```mermaid
flowchart LR
  db[("forge.db<br/>append-only triggers · chain built inside the append transaction")]
  db -- "export" --> ndjson["canonical NDJSON + pinned manifest"]
  ndjson -- "verify-run" --> offline["chain · envelopes · fold — offline"]
  ndjson -- "import" --> other[("another journal<br/>byte-identical · run-id collision refused · never merged")]
  db -- "anchor" --> ref["refs/forge/‹run›<br/>tamper evidence, unsigned"]
```

A concurrent writer conflicts instead of forking history. Import
relocates one run; journals never merge (decision 0027). The anchor is
evidence, not proof: the ref is unsigned, and decision 0008 defers the
signing service.

## Every effect, in order

```mermaid
stateDiagram-v2
  direction LR
  [*] --> requested: effect/requested committed
  requested --> started: effect/started durable, then the driver spawns
  started --> started: effect/checkpointed
  started --> succeeded
  started --> failed
  started --> indeterminate: crash · driver vanished · in flight at restart
  succeeded --> [*]: typed result to the table
  failed --> requested: attempts < max_attempts
  failed --> parked: attempts exhausted
  indeterminate --> parked: never auto-retried
  parked --> [*]: awaiting_operator, raw evidence attached
```

The order is the durable outbox: the request is committed before
execution, the start is durable before the driver spawns, checkpoints
land as they arrive, and each attempt ends in exactly one terminal
fact. A crash at any boundary recovers without losing a committed fact
and without turning an uncertain effect into a success. Seat input is a
pure function of the journal and the pinned bundle; recovery rebuilds
it and refuses to execute anything whose digest differs from what was
requested.

Autonomy is bounded (decision 0006). Determinate failures retry up to
`max_attempts`, deadline kills by the watchdog included. `indeterminate`
never retries, because a retry could duplicate or re-pay for finished
work. Exhaustion, schema violations, unmatched results and unknown
anything park the run with the raw evidence attached — never repaired,
coerced, or handed to a model to fix (decision 0001). Operator commands
are journal events, not prose. `operator --action supersede` records the named
residual findings an operator closes, with the actor and optional closing-run
citation (decision 0047); it does not rewrite a past verdict or resume a
completed run.

## Policy is data

```mermaid
flowchart LR
  result["typed result<br/>from the seat"] --> inputs
  engine["engine-owned inputs<br/>consecutive_failures · drift · dirty · reviewed heads<br/>visits_‹phase› · realm_facts"] --> inputs
  declared["inputs the seat declared"] --> inputs
  other["anything else"] -- "dropped before the table or the record" --> bin["nothing"]
  inputs["evaluation inputs"] --> table{{"policy.json<br/>first match wins · closed vocabulary"}}
  table --> next["next phase"]
  table --> park["park → awaiting_operator"]
  table --> stop["stop"]
  table -- "nothing matched" --> park
```

The strict core evaluates the table first-match-wins (decision 0004).
The condition vocabulary is closed and checked at load, so a misspelt
key refuses to load rather than silently never firing. An absent input
never satisfies a condition; an unreadable one parks. Provenance is
compile-time (decision 0007): engine-owned inputs overlay anything a
seat claims, declared inputs pass, everything else is dropped before it
reaches the table or the record.

The outer machine is a linear state machine by constitution (decision
0002): one active phase, a totally ordered journal. This is the `fast`
recipe's table, the one the front page runs:

```mermaid
stateDiagram-v2
  direction LR
  [*] --> implement
  implement --> implement: broken, once
  implement --> verify: complete
  implement --> review: docs-only returned fix
  verify --> review: pass
  verify --> implement: fail, bounded by visits_implement
  review --> ship: clean · low/info residual as named debt
  review --> implement: residual above low, bounded by visits_implement
  review --> parked: reforging exhausted
  ship --> ship: ready
  ship --> review: HEAD drifted
  ship --> done: shipped
  implement --> stop: blocked · broken twice
  verify --> stop: fail at exhaustion
  review --> stop: security-hold · above-medium residual at exhaustion
  ship --> stop: dirty tree
  parked --> [*]: awaiting_operator
  done --> [*]
  stop --> [*]
```

`brokkr compile` rejects any table where the protected review phase can
be skipped on a path to a non-stop terminal. The way back is bounded in
the machine's own vocabulary (decision 0022): a rule may read
`visits_<phase>_gte`, the same count the graph renders as `×N`, and a
rule may rule a park instead of a phase. The seat a run returns to
receives the result that sent it back as `context.returned_from`.

The world is chosen at invocation (decision 0023): `realms.json` names
the repositories a run may see and the journal they share, is pinned by
content hash into the run manifest, and `resume` rehydrates it from that
pin rather than from the disk. A realm publishes a file, another pins
its bytes, and the loader verifies at world load (decision 0057).

## A bundle, resolved

```mermaid
flowchart LR
  base["recipe: base"] -- "extends" --> leaf["recipe: derived<br/>differences only · override / remove markers"]
  leaf --> compose{{"compose — pure, leaf-first"}} --> flat["ONE flat bundle<br/>policy · seats · charters"]
  flat --> resolve{{"resolve"}}
  agents["agents/‹name›.json<br/>charter · model chain · limits · inputs · tools"] --> resolve
  adapters["adapters/‹provider›.json<br/>model ids · permissions · trust_tier · egress class per route"] --> resolve
  resolve --> checks{"gate site → trusted tier?<br/>secret bindings → route class ≥ minimum?<br/>every restriction expressible?"}
  checks -- "no" --> refuse["compile refuses, naming agent · provider · capability"]
  checks -- "yes" --> manifest["run manifest<br/>digest pins recipe, chain and adapters<br/>resume uses exactly this or refuses"]
```

A bundle is reviewable text: policy, one seat per phase, pinned by
content digest. A seat is a single session or a **panel**: members fan
out inside one effect, join as a barrier in declared order, and a
closed-vocabulary aggregate produces the one typed result the machine
sees.

Composition (decision 0017) is a pre-pass over recipe sources that
resolves the chain into one flat bundle before anything is parsed: no
inheritance at run time, no dynamic lookup. Named things merge by name,
redefining needs the marker, removal fails if its target is absent, and
the constitutional lint runs on the resolved table.

Agents and adapters are data (decision 0016). Unsupported capabilities are
explicit refusals. Each driver-bearing site declares `work` or `gate`; gates
require a trusted adapter. Secret bindings require the resolved route to meet
the bundle's `egress_minimum` (decisions 0021 and 0036).

Egress classes are `local`, `contracted` and `uncontracted`, declared for the
adapter's destination and separately for each route it serves. Unknown tiers
are untrusted; unknown classes and routes are uncontracted. A local endpoint
gets no gate authority merely by being local. Fallback is bounded to failure
before acceptance; sessions cannot switch models midway. Authorising adapters
are pinned, so a changed trust declaration changes bundle identity.

## Drivers

```mermaid
sequenceDiagram
  participant E as engine
  participant D as brokkr driver ‹kind›
  participant H as harness
  E->>D: hello
  D-->>E: capabilities
  E->>D: start — seat prompt, result path, deadline
  D->>H: spawn, behind the realm's boundary
  D-->>E: accepted
  loop each turn
    H-->>D: session stream
    D-->>E: checkpoint — bounded: turns · tools · usage · cost
  end
  H-->>D: exit, typed result file
  D-->>E: result
  Note over E,D: unknown message types fail closed · a driver that vanishes after accepted leaves the attempt indeterminate
```

Drivers speak `forge-driver/v1`
([contracts/driver-protocol.v1.schema.json](contracts/driver-protocol.v1.schema.json)):
NDJSON over stdio, stdout protocol-only, stderr captured as an artifact.
The adapters for Claude Code, Codex, dsh and any
prompt-in/result-file-out harness are built into the binary as
`{brokkr} driver <kind>` (decision 0009), while the protocol stays
language-neutral for third-party drivers. What stands around a seat is
the realm's **boundary** (decision 0046): `namespace`, `seatbelt`,
`container`, `harness` or `open`, pinned per site, rendered *unboxed*
under `harness` or `open`. Decision 0008's `driver.confine` is refused
(0046 ruling 5) until slice (iii) builds `container`.

The implemented boundaries are `namespace` (Linux/WSL2 with bubblewrap),
`harness` and `open`. `seatbelt` and `container` refuse at start until their
implementation lands. Harness gates require a measured adapter fragment;
therefore not every shipped recipe is available under `harness`. The manifest
pins the selected boundary, and CLI, TUI and web readouts retain that fact.

## Verification, in layers

| Layer | What it pins |
|---|---|
| Differential corpus | A frozen 97-case corpus in [fixtures/](fixtures/) pins the evaluator: contract data, never regenerated. |
| Machine proof | End-to-end scenarios drive the real binary and real subprocess protocol through success, retries, stops, parks, crash recovery at every durable boundary, panels, boxed hands and bundle pinning. |
| Self-delivery | `bundles/self` lets the engine deliver changes to this repository; `shipped` is the sole entry into `done`, and the operator keeps push and merge. |
| Brokkr verification | `bundles/verify` examines an already-delivered change with a verify seat and a strictly read-only review seat. It has hard-stopped its own author's work on a real security finding. |

## The operating surface

```
brokkr init · doctor · compile · run · resume · operator · inspect · watch ·
       replay · export · import · verify-run · runs · costs · anchor ·
       ui · tui · muninn · driver
```

Exit codes: `0` completed · `2` parked (operator needed) · `3` stopped.
`brokkr ui`, `brokkr tui` and `brokkr inspect` are three renderers over
the same `brokkr-view` models (decision 0014): read-only, no operator
command, nothing written to the journal. `brokkr costs` reports per-seat
attempts, turns and USD from journal checkpoints, keyed by the stable
seat ids a cost ledger can join on.

## Release preparation

`release-manager` prepares versions, commit-derived notes, documentation and
organization-profile patches. Its charter is portable; the realm's house file
supplies extensible configuration, covered by the existing house digest and
prompt assembly. Configuration describes required work and evidence; it does
not execute commands or add checks to a gate.

`recipes/release` combines the manager and library reviewer with `fast`'s policy
and boxed exec gates. The shipped verifier is Rust-specific; another stack
replaces it through recipe composition. External patches and base commits are
reviewed in the handoff. Publication and cross-repository application are
verified separately from local run completion.
