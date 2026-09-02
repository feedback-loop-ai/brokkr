# Architecture

**Status**: describes the system as implemented. The pre-implementation
blueprint it grew from is [docs/target-architecture.md](docs/target-architecture.md);
divergences are recorded in numbered decisions
([docs/decisions/](docs/decisions/)), which are the authority when this
document and a decision disagree.

Brokkr is an event-sourced, deterministic process manager wrapped
around stochastic, fallible effects. Agent sessions are leaves; their
outputs are typed results; only a pinned policy table ever selects a
transition. Every claim the system makes — done, verified, parked,
stopped, paid — is a journaled, replayable fact.

## One binary, six crates

```
brokkr-cli        the shipped `brokkr` binary: commands, embedded UI, adapters entry
  brokkr-runtime  bundles · agent library + provider adapters · engine loop ·
                 recovery · confinement · anchoring
    brokkr-core   PURE: envelope · canonical hashing · fold · policy evaluation
    brokkr-store  SQLite journal (append-only, hash-chained) · export · verify
    brokkr-protocol  forge-driver/v1 · subprocess transport · built-in adapters
  brokkr-view     PURE: one display derivation — run rows, participants,
                 phase topology, decision trail; no I/O, no clock, no
                 terminal or DOM concept (decision 0013)
```

Trust separates the crates, not deployment: `brokkr-core` performs no
I/O, clock reads, randomness, or process execution — given the same
journal and bundle it always returns the same state and ruling.
Everything effectful sits above it and is journaled around it.
`brokkr-view` is pure for a different reason: it is the ONE answer to
every display question, rendered by `ui.html` as pixels and by
`render.rs` as text, so the two surfaces cannot drift. Its manifest
depends on exactly `brokkr-core`, `serde` and `serde_json`, which makes
that purity a compile error rather than a review convention.

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
forking history). `brokkr export` writes canonical NDJSON;
`brokkr verify-run` checks it offline; `brokkr import` adopts it into
another journal byte-identically, verifying the whole chain and refusing
a run-id collision outright (journals never merge — one run relocates,
decision 0027); `brokkr anchor` records the
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
phase, a totally ordered journal — and `brokkr compile` rejects any
table where the protected review phase is avoidable on a path to a
non-stop terminal.

Input provenance is compile-time (decision 0007): every evaluation
input is either engine-owned (journal-computed — `consecutive_failures`,
drift, dirty, reviewed heads, `realm_facts`, and every `visits_<phase>`
— overlaid over anything a seat claims) or declared by the seat that may
supply it; everything else is dropped before it reaches the table or the
record.

The map is the world, chosen at invocation (decision 0023,
`forge.realms/v1`). `realms.json` names the repositories a run may see —
each a realm with a name, a path and a default branch — and the journal
that world writes; `--realms <file>` chooses it on the run and on every
read surface, `--db` outranks the map's journal, and an unmapped
workspace behaves exactly as it always did. A named map that is missing
or malformed refuses before a journal is opened or a seat spawns. At run
start the map is pinned by content hash and embedded whole into the run
manifest — which rides inside `run/started` — so the world a run
believed in is answerable from the journal alone. Repository facts are
recorded keyed by the realm the repository is; reading them accepts the
unkeyed shape every earlier journal used, so nothing already recorded
changes meaning. `resume` names a journal and no map, and rehydrates the
world from that pin rather than off the disk, so a run's fact shape does
not change with the verb typed. The Looper-bound `--dispatch` lineage
carries no world: a map NAMED with `--realms` is refused there, and one
merely lying in the workspace is left unpinned out loud.

The graph has a way BACK (decision 0022, table schema
`forge.phase-machine/v2`). A rule may read `visits_<phase>_gte` — the
same count the graph renders as `×N` — so a back-edge can be bounded in
the machine's own vocabulary rather than by a seat's promise; and a rule
may rule a **park** instead of a next phase, so a run can be handed to
the operator without a stop being made to impersonate one. The seat a
run RETURNS to receives the result that sent it back as
`context.returned_from`, which is how a review's findings reach the
implementer who has to answer them. A seat on its first visit receives
nothing new.

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

A recipe may **extend** another and state only its differences
(decision 0017). Composition is a pure pre-pass over recipe sources —
`bundle/compose.rs` is the only module that knows the word `extends` —
resolving the chain leaf-first into ONE flat bundle before anything is
parsed: no inheritance at run time, no dynamic lookup. Seat values are
opaque to it; it decides only which value wins for a name, which is why
the `override` and `remove` markers are resolver-owned top-level keys
BESIDE the values rather than inside them. Named things merge by name;
adding is free, redefining needs the marker, removal is explicit and
fails if its target is absent. Policy rules compose by the engine's
existing first-match-wins order with derived rules first (override by id
is remove-then-prepend, so no dead twin survives), and the
constitutional lint runs on the RESOLVED table — a derived recipe cannot
make review avoidable. The resolved digest pins the run as ever, and the
chain rides in the manifest's `files` map under `@compose/`, so changing
a base moves the digest of everything derived from it and resume names
the ancestor that moved.

Drivers speak `forge-driver/v1` (NDJSON over stdio; unknown messages
fail closed; a driver that vanishes after accepting leaves the attempt
indeterminate). The adapters for Claude Code (`claude` directly, or
`lanetally` — the same harness through LaneTally's session-capture
wrapper), Codex, and any template-shaped harness (dsh/Surface profiles,
ssh-carried remote execution, prompt-in/result-file-out CLIs) are built
into the binary — `{brokkr} driver <kind>` in bundle data (decision
0009) — while the protocol stays language-neutral for third-party
drivers. On cost provenance: `total_cost_usd` stays the
harness-reported list price for both claude and lanetally; LaneTally
capture makes the session priceable in the LaneTally ledger (marked by
the checkpoint's constant `capture:"lanetally"`), and the per-session
actual-cost join is deferred until readplane exposes a session query. Trust classes
are data too: no confinement is a trusted native child;
`driver.confine {image, network, mounts}` wraps the command in a
pinned container with the workdir mounted at the same path.

Model policy is data with two compile-time refusals behind it (decision
0021). Every driver-bearing site — a seat, a panel member, a sequence
step — declares a `class`: `work` sites produce output the machine
checks, `gate` sites ARE the check. Every adapter declares a
`trust_tier` (`trusted`/`untrusted`, an operator ruling cited to the
scorecard, never a vendor name in an arm) and a `binding_grant` (the
other axis: clearance to RECEIVE). `Bundle::assemble` then refuses a
gate site whose driver is not trusted, and a site under a seat with
declared secret bindings whose driver holds no grant — a lookup and a
comparison, before any prompt exists to leak, in the manner of a digest
mismatch. Both fail closed on absence: an undeclared tier is untrusted,
an undeclared grant is none, and a driver no adapter declares has
neither. Because the class is read by absence, the key vocabulary of
every site — seat, panel member, sequence step — is CLOSED: a key the
compiler does not read is refused where it is written, so `"clas":
"gate"` cannot manufacture the silence the fail-closed reading trusts.
An agent's whole fallback chain is checked, not just its first
link, because a chain that could fall back to an untrusted judge at run
time would have defeated the gate at compile time. An inline site's
driver is read structurally off the token after `driver` in its command
(decision 0009's dispatch shape); a command that is no dispatch names no
driver, and so declares nothing. What ALLOWED a site is pinned where the
bundle's identity can see it: an agent site through its resolution
record, an inline one through a `drivers` manifest key naming, per
judging or binding seat, the digest of the adapter file that answered —
so demoting a tier in `adapters/` moves the identity of every bundle
that was standing on it. Absent, like `agents`, when nothing was
consulted. `brokkr init` scaffolds this whole shape: the starter's
verify, review and ship seats are classed `gate`, and the scaffold
carries its own `adapters/` declaring the tier they judge on, which is
why brokkr is then run from inside it.

## Agents are defined once; adapters are data

A seat may name an agent instead of inlining what it is (decision 0016).
`agents/<name>.json` carries a description, a charter, an ORDERED chain
of abstract model names, abstract tool/MCP configuration, the 0006
limits and the 0007 declared inputs; `adapters/<provider>.json` maps the
abstract onto one provider — driver invocation, model ids, how tool
permissions and MCP servers are spelled, and which of those the provider
**cannot** express, declared as the explicit string `"unsupported"`
because an empty map is ambiguous between "cannot" and "not filled in
yet". No Rust match arm over provider names is ever written; adding a
provider or a model is a file.

`crates/brokkr-runtime/src/agents.rs` is the resolver and it is pure by
signature: availability is an argument and the module reaches for no
filesystem, environment, process or clock — the I/O that loads the two
trees lives behind a named boundary in `agents/load.rs`. `Bundle::compile`
passes availability *unspecified*, so compile-time resolution depends on
exactly two digested inputs and one bundle cannot have two digests.
`brokkr doctor` is the real consumer of the probed arms.

Resolution is pinned into the run manifest under one `agents` key,
**absent** when no seat references an agent, keyed by invocation site,
carrying the agent, charter and adapter digests, the full chain and the
chosen index — names and digests only, never resolved argv, whose
`{brokkr}` expansion is a machine-local absolute path. Per-invocation
provenance reaches the journal as optional, absent-by-default payload
fields at `event_schema: 1`, published as
`contracts/effect-provenance.v1.schema.json`; `fold` never reads them,
which is what makes the amended rule in `contracts/README.md` honest.

The honesty rules are mechanised, not described. A restriction the
provider cannot express is a compile failure naming agent, provider and
capability (the agent would get MORE power than it declares, so
`optional` is unrepresentable there); a grant gap is a failure unless
marked optional, and then a notice that lands in the manifest and in
every readout. Both run over every chain entry. Fallback is bounded by a
structural predicate — `Failed`, never `Accepted`, no checkpoint — so
decision 0016's mid-session boundary is unreachable-by-construction
rather than a comment, and the chain index is derived by scanning the
effect's own events so a restart cannot change which model runs next.

## Verification is layered, and Brokkr verifies itself

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
- **Brokkr verification**: `bundles/verify` is a two-seat bundle (verify
  + strictly read-only review) that examines an already-delivered
  change named in its feature text. Its rulings are journaled runs like
  any other — and it has hard-stopped its own author's work on a real
  security finding, which is the system working as designed.

## Operating surface

```
brokkr init · doctor · compile · run · resume · operator · inspect ·
      replay · export · import · verify-run · runs · costs · anchor ·
      ui · tui · driver
```

Exit codes: `0` completed · `2` parked (operator needed) · `3` stopped.
`brokkr ui` serves an embedded read-only page on loopback (Host-pinned
against DNS rebinding, GET-only, SSE updates) — it submits no commands
and can be removed without changing execution semantics. `brokkr tui` is
the same fleet explored with the keyboard: a navigable table of runs,
one run's phase graph, seats and decision trail, and one seat's
checkpoint and session stream (decision 0014). Its read-only boundary is
the same one: a third renderer over `brokkr-view`'s models that issues no
operator command, starts no run and writes nothing to the journal — a
missing database is a refusal, never an initialized empty store. `brokkr costs`
reports per-seat attempts, turns, and USD from journal checkpoints,
keyed by the stable seat ids the LaneTally layer joins on.

## The layer above and below

Brokkr decides *how far a delivery advances* — nothing else. A
dispatch layer (Looper) decides what is worth delivering and routes
parks to human attention; LaneTally decides who pays, joining on seat
ids and cost checkpoints; harnesses (Claude Code, Codex, dsh) supply
capability as leaf effects. Each layer refuses a specific kind of
lying, and none can override another's law.
