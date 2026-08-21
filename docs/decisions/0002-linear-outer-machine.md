# 0002 — The outer machine is linear; we keep the FSM

**Status**: accepted (operator ruling, 2026-08-21)

## Ruling

The outer phase machine is and remains a linear finite state machine: one
active phase at a time, a totally-ordered journal, `evaluate()` over an
ordered first-match-wins rule list. It never becomes a DAG. This is
**constitutional** — the policy lint and the sweep tests may assume it.

## Why

- A DAG is not "an FSM with more edges": multiple active phases turn state
  into a marking, oblige every journal reducer to be interleaving-
  insensitive, turn ordered deny-before-allow rules into join conditions
  auditable only by graph analysis, and import concurrent-cancellation
  semantics into hard stops (`security-hold` must halt in-flight siblings).
- Verification cost explodes: the exhaustive sweep that found a real table
  gap on day one enumerates `phases × results × inputs`; over markings and
  interleavings it becomes model checking.
- Phases are the intervals between gates, and gates are serializing points
  by nature. Work that could run concurrently with no gate between it was
  never two phases — it is parallel work inside one phase, which executors
  already provide.
- Determinism is the product this engine exists to deliver. A DAG spends
  the exact property the machine was built to guarantee, to buy wall-clock
  that executor-internal parallelism already provides where it matters
  (seats, waves, panels, tracks).

## Sanctioned concurrency — exactly two forms

1. **Inside executors**: seat fan-out and parallel sub-machines, journaled
   as sub-events under the phase's span. The outer machine sees only the
   executor's final typed result. Concurrency inside, serialization at the
   boundary.
2. **Auxiliary tracks**: journaled background jobs (docs generation,
   release-notes drafting, coverage planning) started at a named phase and
   joined at a named barrier — ship by default, enforced as the
   `aux-tracks-joined` ship precondition. An aux track can attach evidence
   and block its barrier until concluded; it can NEVER cause a transition
   or emit a phase result. Aux events fold into a side compartment of
   state and never enter `evaluate()`.

## Pre-emptively rejected

- **Speculative verify/review overlap**: it trades the recorded guarantee
  "reviewers read live-verified code" (VERIFY-PASS) for latency, and fix
  rounds usually claw the latency back. The guarantee wins.
- **Per-repo phase progression**: waves, contract checks, and the booted
  verification stack are jointly meaningful across repos; splitting
  per-repo dissolves the cross-repo coherence the Forge exists to enforce.

## If a real need ever appears

It must name the gate being removed and show both sanctioned forms fail
it. Even then, the answer is **two linear machines composed by an explicit
join gate** — composition of auditable machines — never a generalization
of the engine to arbitrary DAGs. (Parallelism across features is already
N independent machines with N journals; shared-resource arbitration is an
effects-layer concern, not machine topology.)
