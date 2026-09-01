# Target architecture

**Status**: implementation blueprint, accepted 2026-08-22 under
[decision 0003](decisions/0003-native-rust-runtime.md).

## Product contract

Forge is an event-sourced, deterministic process manager around stochastic and
fallible effects. It coordinates autonomous software delivery while keeping
transition authority outside every agent harness.

The default installation is one native `brokkr` executable and one workspace
database. It must support unattended runs that last days, survive coordinator
and host restarts, resume without repeating a completed model call, and expose
enough evidence to compare models and inner orchestration topologies.

The guarantees are deliberately precise:

- **State replay**: the same journal and pinned bundle produce the same state
  and rulings.
- **Execution replay**: not promised. Reissuing the same stochastic request may
  produce a new result and is always a new attempt.
- **Topology comparability**: different inner topologies that emit the same
  normalized phase result and computed inputs receive the same outer ruling.
  They are not guaranteed to emit the same result.

## System shape

```text
                 brokkr CLI        system browser
                     │        localhost HTTP + SSE
                     └──────────────┬──────────────┘
                                    ▼
┌──────────────────────── one brokkr process ────────────────────────┐
│ command API · config compiler · embedded UI                       │
│                                                                  │
│ pure core                                                        │
│   fold(events, bundle) -> state                                  │
│   decide(state, bundle) -> action                                │
│   evaluate(policy, state, typed_result) -> ruling                 │
│                                                                  │
│ durable runtime                                                  │
│   SQLite journal · effect outbox · scheduler · process supervisor│
│   leases · budgets · artifact store · projections                │
└──────────────────────────────┬───────────────────────────────────┘
                               │ forge-driver/v1
                ┌──────────────┼──────────────┬──────────────┐
                ▼              ▼              ▼              ▼
             native         container       remote       Cordis/DSH
             process          runner          runner        session
```

The default is a crash-resilient single-host system with one logical writer per
run. Parallel seats and auxiliary tracks are effects coordinated around that
writer. Multi-host high availability is a later store/lease implementation,
not a reason to burden the local product with infrastructure.

## Rust workspace and one shipped binary

The implementation is separated by trust boundary rather than deployment
unit:

| Crate | Responsibility |
|---|---|
| `brokkr-core` | Event types, immutable state, fold, policy evaluation, topology semantics, invariants. No I/O dependencies. |
| `brokkr-store` | SQLite journal, migrations, append transactions, projections, artifact metadata, verification and export. |
| `brokkr-runtime` | Commands, effect outbox, scheduler, leases, recovery, budgets, cancellation, subprocess and container supervision. |
| `brokkr-protocol` | Versioned driver envelopes, capability negotiation, typed results, conformance fixtures. |
| `brokkr-api` | Local command/query API and server-sent event stream. No decision logic. |
| `brokkr-cli` | `brokkr` commands, embedded static UI, installation-facing behavior. |

All crates link into one executable. SQLite is bundled into the release. The
database has one dedicated writer thread; external concurrency is asynchronous,
but the reducer and transition evaluator remain synchronous and pure.

## State and control status

Delivery phase and control status are separate fields:

```text
phase:  intake | architecture | worktrees | implement | verify |
        review | regression | ship | done | stop

status: running | awaiting_operator | completed | stopped
```

This resolves the existing extraction mismatch: `awaiting-operator` is a
durable control status that parks a run at its current phase, not an unlisted
delivery phase. Only an operator command, recorded as an event, may leave it.

Current state is always a projection of the journal. Cached projections may be
discarded at any time. Timestamps are evidence and display data; reducer
decisions never depend on wall-clock time unless a durable timer-fired event has
first made the passage of time a fact.

## Event and effect protocol

Every event envelope contains at least:

```text
run_id · seq · event_id · event_schema_version · type · payload
causation_id · correlation_id · attempt_id? · recorded_at
previous_hash · event_hash
```

The initial vocabulary should cover:

```text
run/started                 operator/commanded
phase/entered               operator/accepted
effect/requested            operator/rejected
effect/started              run/parked
effect/checkpointed         run/completed
effect/succeeded            run/stopped
effect/failed               transition/decided
effect/indeterminate
```

External work follows a durable outbox discipline:

1. append `effect/requested` with its attempt and idempotency keys;
2. commit before invoking the driver;
3. append checkpoints without changing phase authority;
4. append exactly one terminal attempt fact: succeeded, failed, cancelled, or
   indeterminate;
5. validate and normalize the returned result;
6. evaluate the pinned policy; and
7. append `transition/decided` with rule id, normalized inputs, and evidence
   references.

A crash after a result is durable but before its ruling causes deterministic
evaluation of that recorded result. A crash with an outstanding request is
recovered through the driver's idempotency/reconnect capability. If completion
cannot be established safely, the attempt becomes `indeterminate` and the run
parks instead of silently paying for or accepting a different call as the same
attempt.

Long-running inner loops checkpoint at stable turn, step, wave, or tool
boundaries. A heartbeat is operational telemetry, never evidence that work
completed. Every loop declares cost, attempt, elapsed-time, and stagnation
limits; an operator may extend a budget through a journaled command.

## SQLite and artifacts

The runtime database contains append-only facts and replaceable read models:

```text
runs                  pinned manifest and lifecycle
events                immutable logical event stream
effect_outbox         durable work awaiting execution
effect_attempts       runtime correlation and recovery
commands              operator intent and disposition
leases                exclusive ownership with fencing token
projection_cache      reducer-versioned, disposable state
artifacts             content hashes, media types, sizes, locations
```

Application triggers reject update or deletion of event rows. This protects
against ordinary defects, not a hostile database owner.

Large prompts, transcripts, chunks, patches, test logs, and reports live in a
content-addressed artifact directory rather than the outer event table. Small
typed results may remain inline. An artifact reference includes a digest, media
type, size, producer, and retention class.

The database is runtime truth. `brokkr export <run>` produces canonical NDJSON,
the pinned run manifest, and an artifact manifest. `brokkr verify-run` verifies
sequence continuity, event hashes, artifact hashes, signatures, and anchors
without executing an agent.

## Declarative bundles

Customization source is ordinary text suitable for review in Git:

```text
forge.toml
policy/phase-machine.toml
topologies/*.toml
profiles/*.toml
roles/*.md
schemas/*.json
```

The current JSON transition table remains a valid migration input. The final
authoring format may retain JSON where it is clearer; canonical identity comes
from normalized compiled content, not source formatting.

The inner topology language is a graph of a small set of primitives:

| Primitive | Meaning |
|---|---|
| `seat` | One versioned agent session returning a schema-bound result. |
| `parallel` | Start independent child nodes subject to a concurrency budget. |
| `join` | Wait for a declared set and aggregate by an explicit deterministic rule. |
| `loop` | Repeat a subgraph under termination and budget predicates. |
| `gate` | Require a deterministic predicate or operator command. |
| `tool` | Invoke a non-agent effect behind the same durable protocol. |
| `submachine` | Run a nested event-sourced machine and return its typed result. |
| `emit-result` | Produce the sole phase result visible to the outer FSM. |

Conditions support a bounded typed vocabulary such as boolean composition,
equality, ordering, membership, counts, and severity comparisons. They do not
execute arbitrary code. Parallel results are reduced using stable node/seat
identities or an explicitly order-independent aggregation rule, never wall-clock
completion order.

`brokkr compile` must reject:

- schema-invalid configuration;
- unreachable nodes or phases;
- result variants without an outer rule;
- unbounded loops;
- missing drivers or driver capabilities;
- protected-seat removal;
- a path to shipping that violates constitutional invariants; and
- configuration whose canonical identity cannot be reproduced.

The output is a content-addressed bundle with a manifest. A run stores the
complete bundle or immutable artifact references sufficient to reconstruct it,
not merely friendly version labels.

## Run manifest and versioning

A run pins independent compatibility axes:

```yaml
engine: 0.1.0
event_schema: 1
database_schema: 1
driver_protocol: 1
policy: phase-machine/v1@sha256:...
topology: council-v3@sha256:...
profile: default-v1@sha256:...
roles:
  architect: sha256:...
  reviewer: sha256:...
schemas:
  phase-result: sha256:...
drivers:
  codex: { version: "...", executable: "sha256:..." }
images:
  verifier: registry/image@sha256:...
```

Friendly names are display metadata; digests are identity. Resume uses the exact
pinned bundle or refuses with a diagnostic. Event payloads are immutable. A new
reader may apply pure, tested in-memory adapters for supported old events, but
never rewrites historical facts in place. Projection caches are keyed by reducer
version and rebuilt on mismatch.

## Drivers and isolation

Executable extensions speak `forge-driver/v1` over NDJSON on stdin/stdout or a
local socket. The initial message family is:

```text
hello / capabilities
start / accepted
resume / accepted
checkpoint / evidence
result
cancel / cancelled
shutdown
```

Every message has a protocol version, message id, run/effect/attempt identity,
and schema-valid payload. Unknown required messages fail closed. Stdout is
protocol-only; stderr is captured as an artifact.

Drivers may be implemented in any language. Trust selects the runner:

| Trust | Default runner |
|---|---|
| `trusted` | Native child process in its assigned worktree. |
| `policy-confined` | OCI container with pinned digest and least-privilege mounts/network. |
| `public-evidence-only` | Container or remote runner with no private repository or credential mount. |

The coordinator never exposes its database, signing material, or container
control socket to a seat. It grants only declared worktree, artifact, tool, and
network capabilities.

## Cordis and other long-horizon harnesses

Cordis/DeepSeek Harness implements one driver, not a special control path:

```text
Forge phase/topology node
└── CordisDriver
    └── persisted DSH session
        ├── turns and tool calls
        ├── subagents and context changes
        ├── inner checkpoints
        └── final typed result
```

Forge records the external session identity, driver configuration digest,
checkpoints, accounting, artifact digest, and result. DSH owns its detailed
model-visible trajectory. The same outer node can instead use Codex, Claude,
another harness, or a replay driver.

Dynamic harness reconfiguration is disabled during controlled evaluations
unless mount/unmount/configuration changes are themselves recorded as part of
the topology treatment. Otherwise the `topology` label would not identify what
actually ran.

## Local API and embedded UI

The UI is a static browser application compiled at release time and embedded in
`brokkr`. `brokkr ui` binds to loopback, opens the system browser on request, and
streams new events with server-sent events. There is no desktop shell and no
Node runtime at installation time.

Required views:

1. **Outer machine** — current phase/status, legal transitions, last ruling,
   blockers, and ship preconditions.
2. **Inner topology** — seats, dependencies, active attempts, joins, loops,
   checkpoints, costs, and residual findings.
3. **Timeline** — immutable events with causal links and referenced evidence.
4. **Comparison** — policy/topology/model treatments aligned by common states
   and normalized results.
5. **Configuration** — graph editor and form views that produce an ordinary
   source diff, compile result, and new bundle digest.

The API separates commands from queries. Queries read projections. Mutations
submit typed commands to the coordinator, which validates authorization and
records acceptance or rejection. The UI never updates event or state tables.

## Audit and evaluation

Each ruling records the exact normalized result and computed inputs it used.
Each attempt records topology, seat, model route, prompt/role digest, tool view,
repository heads, cost, latency, and evidence references. This supports:

- controller conformance replay;
- model substitution while holding topology constant;
- topology ablation while holding model and budget constant;
- model-by-topology interaction analysis;
- fixed-budget and unconstrained cost/quality frontiers; and
- counterfactual candidate-policy evaluation in shadow mode.

Historical facts never change during a counterfactual replay. Candidate rulings
are a separate derived report or journal, not edits to the original run.

Events are hash-chained. Signed checkpoints and a final signed receipt make the
chain meaningful relative to a trusted key; the final shipping digest is also
anchored in the repository. A hash chain without a trusted signature or anchor
is not represented as tamper-proof.

## Installation and operation

Normal installation downloads one signed platform executable. Package-manager
recipes and `cargo install` are secondary channels. The core has no required
runtime service.

```text
brokkr init                 create a minimal reviewable bundle
brokkr doctor               verify tools, drivers, credentials, and sandboxes
brokkr compile              validate and hash a bundle
brokkr run <feature>        start or resume a run
brokkr runs                 one clamped line per run, newest first
brokkr inspect <run>        explain state, the last ruling, seats, and the
                            phase graph as a terminal tree (--phase/--seat
                            scope it; --json emits the view model)
brokkr watch <run>          the same readout, live, until the run concludes
brokkr replay <run>         rebuild and verify state without effects
brokkr ui                   launch the local visual surface
brokkr export <run>         write the canonical audit bundle
brokkr verify-run <bundle>  verify an audit bundle offline
```

An OCI image is published for CI and server use, but local users do not need a
container runtime. A long-running installation may place the same binary under
systemd, launchd, or the Windows service manager; process supervision does not
change journal semantics.

## Delivery sequence

1. **Freeze contracts**: event envelope, run/status model, manifest, driver
   protocol, canonical hashing, and schema behavior.
2. **Port the pure core**: implement table loading and evaluation in Rust;
   differential-test every production rule and exhaustive sweep against the
   Python oracle.
3. **Build the journal**: SQLite append path, fold, projection cache, export,
   verification, crash/torn-write tests, and exact-bundle resume.
4. **Build the durable runtime**: commands, outbox, leases, attempts,
   idempotency, checkpoints, cancellation, and indeterminate recovery.
5. **Prove the full machine with `FakeDriver`**: success, retry, hard stop,
   operator park, malformed result, crash at every boundary, and resume.
6. **Port one real vertical slice**: review first, including the protected
   security seat and residual-debt rules.
7. **Add driver adapters**: existing dsh/Surface, Cordis/DeepSeek Harness,
   Codex, and Claude integrations behind protocol conformance tests.
8. **Add worktrees, verify, and ship effects**: implement all ship
   preconditions before permitting a push or PR effect.
9. **Ship the embedded UI**: begin read-only, then add operator commands and
   configuration editing as diff-producing workflows.
10. **Self-host under observation**: Forge prepares its own later changes;
    humans retain review and merge authority until crash, replay, audit, and
    security acceptance suites are consistently green.

## First-release acceptance criteria

- One downloaded executable can initialize, run, resume, inspect, and replay a
  local feature without installing a language runtime or database.
- Killing Forge at every durable boundary never loses a committed fact and
  never converts an uncertain effect into a success.
- Replaying a run twice yields byte-identical canonical state and ruling output.
- An unknown event, invalid result, missing artifact, version mismatch, open
  blocker, drift, or dirty worktree fails closed.
- No agent process can append an authoritative transition.
- A policy/topology edit has a visible source diff and a new bundle digest.
- `brokkr verify-run` can validate a run without network access or model keys.
- The UI can be removed entirely without changing execution semantics.
- The same recorded normalized phase results produce the same outer rulings
  regardless of which driver originally produced them.

## Deferred choices

The architecture intentionally does not yet lock:

- the exact authored policy format where JSON versus TOML remains a usability
  question;
- the signing-key distribution and external anchoring service;
- the remote-runner transport;
- the threshold for introducing a PostgreSQL store; or
- the visualization library used to implement the embedded graph UI.

These choices do not alter the trusted boundaries above and can be resolved by
small, evidence-backed decisions when their implementation begins.

## References

- [SQLite as an application file format](https://www.sqlite.org/appfileformat.html)
- [SQLite write-ahead logging](https://www.sqlite.org/wal.html)
- [DeepSeek Harness architecture](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/architecture.md)
- [DeepSeek Harness persistence](https://github.com/deepseek-ai/deepseek-harness/blob/master/docs/subsystems/persistence.md)
- [Cordis paper](https://github.com/cordiverse/paper)
