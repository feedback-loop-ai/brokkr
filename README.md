# The Forge

**A deterministic delivery engine for autonomous multi-agent software
delivery.** The outermost layer is a pure, event-sourced phase state machine;
agent sessions (Claude Code, Codex, dsh/LaneTally Surface, any harness) are
leaf effects whose outputs are typed results — never decisions.

Extracted from the Alkemio workspace Forge (`agents-hq` specs 018 + 028),
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
| `policy/phase-machine.json` | The production transition table (imported from agents-hq `workspace#028`). |
| `policy/schemas/` | JSON Schemas: phase state, handoffs, council position/clash/adjudication, providers. |
| `src/forge/` | The Python policy oracle. `machine.py` is the implemented pure evaluator used for Rust parity work. |
| `tests/` | Behavior + property tests against the real production table. |
| `reference/` | The pre-extraction implementation, imported verbatim for parity work: `forge-control.py` (referee-era control plane), the Claude Workflow JS phase drivers, the `vf-*` agent charters, the skill prose, `providers.json`. Read-only; mined, then retired. |
| `docs/` | Accepted decisions, the target architecture, and the extension model. |

## Status

Architecture accepted; implementation remains a skeleton plus the Python pure
core. The next sequence is: freeze the event/bundle/driver contracts → port and
differential-test the evaluator in Rust → build the SQLite journal and durable
runtime → prove the whole machine with `FakeDriver` → port review as the first
real vertical slice. See the
[delivery sequence](docs/target-architecture.md#delivery-sequence).

Private for now; the OSS boundary decision (naming, license, threat-model
README, sanitized example profile) comes after the engine drives a real feature
end to end in the Alkemio workspace.
