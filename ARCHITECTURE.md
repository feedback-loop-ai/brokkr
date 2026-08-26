# Architecture

**Status**: describes the system as implemented. The pre-implementation
blueprint it grew from is [docs/target-architecture.md](docs/target-architecture.md);
divergences are recorded in numbered decisions
([docs/decisions/](docs/decisions/)), which are the authority when this
document and a decision disagree.

The Forge is an event-sourced, deterministic process manager wrapped
around stochastic, fallible effects. Agent sessions are leaves; their
outputs are typed results; only a pinned policy table ever selects a
transition. Every claim the system makes — done, verified, parked,
stopped, paid — is a journaled, replayable fact.

## One binary, five crates

```
forge-cli        the shipped `forge` binary: commands, embedded UI, adapters entry
  forge-runtime  bundles · engine loop · recovery · confinement · anchoring
    forge-core   PURE: envelope · canonical hashing · fold · policy evaluation
    forge-store  SQLite journal (append-only, hash-chained) · export · verify
    forge-protocol  forge-driver/v1 · subprocess transport · built-in adapters
```

Trust separates the crates, not deployment: `forge-core` performs no
I/O, clock reads, randomness, or process execution — given the same
journal and bundle it always returns the same state and ruling.
Everything effectful sits above it and is journaled around it.

## The journal is the run

Every event is a hash-chained envelope
([contracts/event-envelope.v1.schema.json](contracts/event-envelope.v1.schema.json)):
`seq` contiguous from 1, `previous_hash` chaining, `event_hash` over the
canonical bytes, `causation_id` naming the event that caused it.
`fold(events) → RunState` derives everything — phase, control status
(`running | awaiting_operator | completed | stopped`), the protocol
cursor, journal-computed counters. An event impossible at the current
cursor fails the fold closed: a journal that violates the protocol is
corrupt, not reinterpretable. Replay is byte-deterministic; resume is
replay.

Storage is bundled SQLite with append-only triggers and the chain built
inside the append transaction (a concurrent writer conflicts instead of
forking history). `forge export` writes canonical NDJSON;
`forge verify-run` checks it offline; `forge anchor` records the
journal head in `refs/forge/<run>` commit chains — tamper *evidence*,
not tamper-proofing (the ref is unsigned; decision 0008 defers the
signing service).

## The effect discipline

Every external effect follows the durable outbox order: `effect/requested`
committed before execution; `effect/started` durable before the driver
spawns; checkpoints journaled as they arrive; exactly one terminal fact
per attempt — `succeeded`, `failed`, or `indeterminate`. A crash at any
boundary recovers without losing a committed fact and without
converting an uncertain effect into a success: an attempt in flight at
restart becomes `indeterminate` and parks. Seat input is a pure
function of (journal, pinned bundle); recovery rebuilds it and refuses
to execute anything whose digest differs from what was requested.

Autonomy is bounded, never open-ended (decision 0006): per-seat
`max_attempts` retries determinate failures — including deadline kills
by the watchdog that reaps hung drivers — while `indeterminate` NEVER
auto-retries, because a retry could silently duplicate or re-pay for
completed work. Exhaustion, schema violations, unmatched results, and
unknown anything park the run in `awaiting_operator` with raw evidence
attached (decision 0001: never repaired, coerced, or handed to a model
to fix). Operator commands (`retry` / `stop`) are journal events, not
prose.

## Policy is data; the vocabulary is closed

The transition table is JSON evaluated first-match-wins by the strict
core (decision 0004): a closed condition vocabulary checked at load (a
typo'd deny key refuses to load rather than silently dying), absent
inputs never satisfy a condition, unreadable inputs park. The outer
machine is a linear FSM by constitution (decision 0002) — one active
phase, a totally ordered journal — and `forge compile` rejects any
table where the protected review phase is avoidable on a path to a
non-stop terminal.

Input provenance is compile-time (decision 0007): every evaluation
input is either engine-owned (journal-computed — `consecutive_failures`,
drift, dirty, reviewed heads — overlaid over anything a seat claims) or
declared by the seat that may supply it; everything else is dropped
before it reaches the table or the record.

## Bundles: seats, panels, drivers, trust

A bundle is reviewable text: policy, one seat per phase (role charter,
declared results, declared inputs, limits), pinned by content digest in
the run manifest — resume uses the exact bundle or refuses. A seat is
either a single session or a **panel** (decision 0002's sanctioned
concurrency): members fan out inside ONE effect, join as a barrier in
declared order with each outcome journaled as checkpoint evidence, and
a closed-vocabulary aggregate (`unanimous-pass`, `review-panel`
worst-member-wins) produces the single typed result the outer machine
sees.

Drivers speak `forge-driver/v1` (NDJSON over stdio; unknown messages
fail closed; a driver that vanishes after accepting leaves the attempt
indeterminate). The adapters for Claude Code, Codex, and any
template-shaped harness (dsh/Surface profiles, ssh-carried remote
execution, prompt-in/result-file-out CLIs) are built into the binary —
`{forge} driver <kind>` in bundle data (decision 0009) — while the
protocol stays language-neutral for third-party drivers. Trust classes
are data too: no confinement is a trusted native child;
`driver.confine {image, network, mounts}` wraps the command in a
pinned container with the workdir mounted at the same path.

## Verification is layered, and the forge verifies itself

- **Differential corpus**: the evaluator's behavior is pinned by a
  frozen 97-case corpus ([fixtures/](fixtures/)) — contract data, never
  regenerated; decisions 0004 and 0009 record the oracle that produced
  it, since removed.
- **Machine proof**: end-to-end scenarios drive the real binary and
  real subprocess protocol through success, retries, hard stops,
  schema parks, indeterminate parks, operator commands, crash
  recovery at every durable boundary, panels, confinement, and bundle
  pinning.
- **Self-delivery**: `bundles/self` lets the engine deliver changes to
  this repository — seats implement, verify, review (security riding
  along, non-removable), and ship through a two-step gate where
  `shipped` is the sole entry into `done`; the operator keeps push and
  merge authority.
- **Forge-verification**: `bundles/verify` is a two-seat bundle (verify
  + strictly read-only review) that examines an already-delivered
  change named in its feature text. Its rulings are journaled runs like
  any other — and it has hard-stopped its own author's work on a real
  security finding, which is the system working as designed.

## Operating surface

```
forge init · doctor · compile · run · resume · operator · inspect ·
      replay · export · verify-run · runs · costs · anchor · ui · driver
```

Exit codes: `0` completed · `2` parked (operator needed) · `3` stopped.
`forge ui` serves an embedded read-only page on loopback (Host-pinned
against DNS rebinding, GET-only, SSE updates) — it submits no commands
and can be removed without changing execution semantics. `forge costs`
reports per-seat attempts, turns, and USD from journal checkpoints,
keyed by the stable seat ids the LaneTally layer joins on.

## The layer above and below

The Forge decides *how far a delivery advances* — nothing else. A
dispatch layer (Looper) decides what is worth delivering and routes
parks to human attention; LaneTally decides who pays, joining on seat
ids and cost checkpoints; harnesses (Claude Code, Codex, dsh) supply
capability as leaf effects. Each layer refuses a specific kind of
lying, and none can override another's law.
