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
┌─ The Forge engine (this repo, Python) ────────────────────────────┐
│  pure core: fold(journal) → state; evaluate(table, state, result) │
│  phase executors: architect · implement · verify · review · ship  │
│  effect runners: worktrees, stacks, pushes, PRs — journaled       │
└──────────────┬────────────────────────────────────────────────────┘
               ▼ spawns seats (confined workspace mount × class)
     Agent harness drivers: dsh / LaneTally Surface · Claude Code · Codex
               ▼ one ANTHROPIC_BASE_URL, repo virtual key
     LaneTally Core: funding lanes · default-deny keys · cost truth
```

Determinism laws:

1. **Decisions are pure.** Given the same journal, the next action is always
   the same. All transition logic lives in a data table
   (`policy/phase-machine.json`), evaluated first-match-wins by
   `src/forge/machine.py`. Changing a ruling is a reviewed one-line diff.
2. **State is derived, never mutated.** `state = fold(events.jsonl)`.
   Resume is replay. Counters (`consecutive_failures`, drift, reviewed
   heads) are journal-computed, never accepted from a caller.
3. **No LLM repair of the control plane** (decision 0001). An executor
   result that fails schema validation, or a `(phase, result)` pair no rule
   matches, parks the run in `awaiting-operator` with the raw evidence
   attached. It is never guessed at, coerced, or handed to a model to fix.
4. **Human gates are machine states.** `awaiting-operator(gate_id)` cannot
   be exited without an operator event — approval is a signed journal entry,
   not a prose convention.

## Repo layout

| Path | What it is |
|---|---|
| `policy/phase-machine.json` | The production transition table (imported from origin-workspace `workspace#028`). |
| `policy/schemas/` | JSON Schemas: phase state, handoffs, council position/clash/adjudication, providers. |
| `src/forge/` | The Python engine. `machine.py` (pure evaluate) is implemented; fold/journal, executors, and drivers land next. |
| `tests/` | Behavior + property tests against the real production table. |
| `reference/` | The pre-extraction implementation, imported verbatim for parity work: `forge-control.py` (referee-era control plane), the Claude Workflow JS phase drivers, the `vf-*` agent charters, the skill prose, `providers.json`. Read-only; mined, then retired. |
| `docs/` | Design decisions and the extension model. |

## Status

Skeleton + pure core. The migration plan (see the discussion that seeded
this repo): extract fold/journal from `reference/forge-control.py` → port
executors one phase at a time (review first) → retire the JS workflows →
drivers (`FakeDriver` → dsh/Surface → Claude SDK → codex exec). Private for
now; the OSS boundary decision (naming, license, threat-model README,
sanitized example profile) comes after the engine drives a real feature
end to end in the the origin workspace workspace.
