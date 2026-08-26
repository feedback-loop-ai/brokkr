<p align="center"><img src="assets/logo.svg" width="132" alt="The Forge — sealed anvil mark, terminal rail node pulsing"></p>

# The Forge

*The machine is the outer loop. Struck, not spun.*

**A deterministic delivery engine for autonomous multi-agent software
delivery.** The outermost layer is a pure, event-sourced phase state machine;
agent sessions (Claude Code, Codex, dsh/LaneTally Surface, any harness) are
leaf effects whose outputs are typed results — never decisions.

The Forge is a standalone engine (decision
[0011](docs/decisions/0011-standalone-identity.md)). It began life as a *referee*
checking an LLM-driven outer loop — that heritage lives, read-only,
under [`reference/`](reference/). Here the machine **is** the outer
loop.

> Agent output never decides a transition. Prompt content never decides who
> pays (that's [LaneTally](https://github.com/feedback-loop-ai/lanetally)'s
> law, one layer down). The same value, stacked.

## Architecture

```
┌─ Forge (one native Rust binary + bundled SQLite + embedded UI) ───┐
│  pure core: fold(journal) → state; evaluate(table, state, result) │
│  durable runtime: outbox · scheduler · recovery · audit          │
│  declarative phase executors and inner agent topologies          │
└──────────────┬────────────────────────────────────────────────────┘
               ▼ versioned, language-neutral driver protocol
       native process · OCI container · remote · Cordis/DSH
               ▼ spawns seats (confined workspace mount × class)
       LaneTally Core: funding lanes · default-deny keys · cost truth
```

The implemented system is described in [ARCHITECTURE.md](ARCHITECTURE.md);
its production shape was established by
[decision 0003](docs/decisions/0003-native-rust-runtime.md) and the
pre-implementation blueprint in
[the target architecture](docs/target-architecture.md). The Python evaluator that served as
executable policy specification during the port is retired to
`reference/oracle/` (decision 0009); the frozen evaluator corpus under
`fixtures/` carries its behavior as contract.

Determinism laws:

1. **Decisions are pure.** Given the same journal, the next action is always
   the same. All transition logic lives in a data table
   (`policy/phase-machine.json`), evaluated first-match-wins by
   `forge-core`. Changing a ruling is a reviewed one-line diff.
2. **State is derived, never mutated.** `state = fold(events)`.
   Resume is replay. Counters (`consecutive_failures`, drift, reviewed
   heads) are journal-computed, never accepted from a caller.
3. **No LLM repair of the control plane** (decision 0001). An executor
   result that fails schema validation, or a `(phase, result)` pair no rule
   matches, parks the run in `awaiting-operator` with the raw evidence
   attached. It is never guessed at, coerced, or handed to a model to fix.
4. **Human gates are control states.** `awaiting-operator(gate_id)` parks the
   current delivery phase and cannot be exited without an operator event —
   approval is a signed journal entry, not a prose convention.

## Repo layout

| Path | What it is |
|---|---|
| `ARCHITECTURE.md` | The implemented architecture — crates, journal, effect discipline, bundles, verification layers. |
| `crates/` | The Rust engine — `forge-core` (pure), `forge-store`, `forge-protocol`, `forge-runtime`, `forge-cli` — building the one `forge` binary. |
| `contracts/` | Frozen v1 contracts: event envelope, fold semantics, `forge-driver/v1`, run manifest. |
| `bundles/self/` | The self-delivery bundle: trimmed linear table, seat charters, headless Claude Code driver. |
| `fixtures/` | The frozen evaluator behavior corpus — contract data, never regenerated. |
| `policy/phase-machine.json` | The heritage transition table the engine was extracted around — retained as the strict evaluator's differential-test fixture (the frozen corpus derives from it). |
| `reference/` | The pre-extraction implementation, imported verbatim for parity work: `forge-control.py` (referee-era control plane), the retired Python oracle (`reference/oracle/`), the Claude Workflow JS phase drivers, `providers.json`. Read-only; mined, then retired. |
| `docs/` | Accepted decisions, the target architecture, and the extension model. |

## Status

The engine is implemented, machine-proved, and self-hosting: frozen v1
contracts, the pure core at differential parity with the Python oracle
(97-case corpus), the append-only SQLite journal, the durable effect runtime
with crash recovery, `forge-driver/v1` with scripted-fake and headless
Claude Code drivers, and the `forge` CLI (init · doctor · compile · run ·
resume · operator · inspect · replay · export · verify-run · runs). Scope per
[decision 0005](docs/decisions/0005-self-forging-first-scope.md): no UI, no
containers, no signing service, single-seat executors — all additive later
behind the frozen contracts. CI builds a release `forge` binary artifact.

The second wave (decision
[0008](docs/decisions/0008-second-wave-scope.md)) delivered the 0005
deferrals: the full driver fleet (claude · codex · exec for dsh/Surface,
ssh-remote, any template harness) behind one conformance suite, parallel
panels with declared deterministic aggregates, the embedded read-only UI
(`forge ui`) over a causally-threaded journal, git-ref journal anchoring,
container confinement for policy-confined seats, and the `forge costs`
LaneTally join surface — each slice verified after merge by the forge's
own verification agents (`bundles/verify`).

Unattended autonomy is bounded, never open-ended:

- per-seat attempt limits and deadlines — a hung or transiently failing
  seat session retries within its declared budget or parks; indeterminate
  outcomes always park
  ([decision 0006](docs/decisions/0006-bounded-attempts-and-deadlines.md));
- input provenance — every evaluation input is engine-computed or
  seat-declared in the reviewed bundle; anything else is dropped before it
  reaches the table or the journal
  ([decision 0007](docs/decisions/0007-input-provenance.md)).

Delivery strategies are **composable recipes** (decision
[0010](docs/decisions/0010-composable-recipes.md)): `forge recipes
list|add` manage a digest-identified library (installable from git),
`forge run --recipe` swaps the strategy, `forge rerun` replays a past
run's feature under another recipe, and `forge compare` renders the
aligned verdict — trails, divergence point, per-seat costs — as a pure
read over two journals.

The `bundles/self` bundle makes the engine deliver its own changes here:
seats implement, verify, review (security riding along, non-removable), and
ship under the linear table, while the operator keeps push and merge
authority. The first self-forged changes have landed —

```
cargo run -p forge-cli -- run --bundle bundles/self --repo . --feature "..."
```

— next is the first external workspace profile
([delivery sequence](docs/target-architecture.md#delivery-sequence)).

Private for now; the OSS boundary decision (naming, license, threat-model
README) comes after the engine drives a real feature end to end in an
external workspace. `forge init` already scaffolds the sanitized example
recipe.
