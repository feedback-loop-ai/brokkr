# The Forge

**A deterministic delivery engine for autonomous multi-agent software
delivery.** The outermost layer is a pure, event-sourced phase state machine;
agent sessions (Claude Code, Codex, dsh/LaneTally Surface, any harness) are
leaf effects whose outputs are typed results — never decisions.

Extracted from the the origin workspace workspace Forge (`origin-workspace` specs 018 + 028),
where the phase machine began life as a *referee* checking an LLM-driven
outer loop. Here the machine **is** the outer loop.

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

The production shape is established by
[decision 0003](docs/decisions/0003-native-rust-runtime.md) and detailed in
[the target architecture](docs/target-architecture.md). The Python evaluator
currently in this repository is the executable policy specification and parity
oracle for that implementation.

Determinism laws:

1. **Decisions are pure.** Given the same journal, the next action is always
   the same. All transition logic lives in a data table
   (`policy/phase-machine.json`), currently evaluated first-match-wins by
   `src/forge/machine.py`. Changing a ruling is a reviewed one-line diff.
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
| `crates/` | The Rust engine — `forge-core` (pure), `forge-store`, `forge-protocol`, `forge-runtime`, `forge-cli` — building the one `forge` binary. |
| `contracts/` | Frozen v1 contracts: event envelope, fold semantics, `forge-driver/v1`, run manifest. |
| `bundles/self/` | The self-delivery bundle: trimmed linear table, seat charters, headless Claude Code driver. |
| `fixtures/` · `tools/` | The oracle-generated evaluator behavior corpus and its generator. |
| `policy/phase-machine.json` | The production transition table (imported from origin-workspace `workspace#028`). |
| `policy/schemas/` | JSON Schemas: phase state, handoffs, council position/clash/adjudication, providers. |
| `src/forge/` | The Python policy oracle. `machine.py` is the implemented pure evaluator used for Rust parity work. |
| `tests/` | Behavior + property tests against the real production table. |
| `reference/` | The pre-extraction implementation, imported verbatim for parity work: `forge-control.py` (referee-era control plane), the Claude Workflow JS phase drivers, the `vf-*` agent charters, the skill prose, `providers.json`. Read-only; mined, then retired. |
| `docs/` | Accepted decisions, the target architecture, and the extension model. |

## Status

The engine is implemented through delivery-sequence step 5 and machine-proved:
frozen v1 contracts, the pure core at differential parity with the Python
oracle (97-case corpus), the append-only SQLite journal, the durable effect
runtime with crash recovery, `forge-driver/v1` with a scripted fake driver,
and the `forge` CLI (compile · run · resume · operator · inspect · replay ·
export · verify-run). Scope per [decision
0005](docs/decisions/0005-self-forging-first-scope.md): no UI, no containers,
no signing service, single-seat executors — all additive later behind the
frozen contracts.

The `bundles/self` bundle plus the headless Claude Code driver make the engine
self-hosting on this repository: seats implement, verify, review (security
riding along, non-removable), and ship under the linear table, while the
operator keeps push and merge authority. Next: drive the first real change
here end to end —

```
cargo run -p forge-cli -- run --bundle bundles/self --repo . --feature "..."
```

— then the the origin workspace vertical slice
([delivery sequence](docs/target-architecture.md#delivery-sequence)).

Private for now; the OSS boundary decision (naming, license, threat-model
README, sanitized example profile) comes after the engine drives a real feature
end to end in the the origin workspace workspace.
