# 0020 — Muninn: the raven that reads everything and rules nothing

Status: accepted — operator ruled 2026-08-31 ("forge Muninn, approved")
Date: 2026-08-31

## Context

Decision 0019 reserved the name: Muninn, Odin's raven — Memory — who flies
over the whole world and reports back at dusk, is the standing overseer
agent, and its authority model is this decision.

The idea reached this product by way of the lieutenant in SwarmForge
(acknowledged in the README per 0019 ruling 7; nothing expressive copied).
That lieutenant is exempted from its own constitution. Muninn is the
opposite: the one agent with fleet-wide sight is the one bound tightest.

Two seats already exist above the ordinary ones, and Muninn is neither:

- The **chief** is consulted fresh and blind, rules on one clash inside one
  run, and is set down. Judgment protected by *removing* context.
- **Muninn** is standing and saturated: it remembers across runs, and its
  whole worth is that memory. Judgment protected by *never being asked to
  rule at all*.

The line between them is constitutional: decomposition and counsel across
runs may come from Muninn as proposals; architecture within a run stays
with the chief, fresh and blind. A standing agent with months of context is
precisely the agent that must not judge a clash between positions — it
already has favorites.

## Decision

1. **Muninn reads only what the journal derives.** Its worldview is the
   `forge-view` models — the same derivation every read surface consumes
   (decision 0013). It opens the store read-only. It reads no repository
   tree, receives no secrets (0012 bindings are for working seats), and is
   never handed a working directory to change.

2. **Muninn proposes; it never rules and never works.** Its output is
   proposals to the operator: a suggested operator command for a parked run
   with the reasoning; residual verify findings assembled into a work
   queue; a fleet summary; a drafted feature text for a next slice. A
   proposal is never an action. Muninn issues no operator commands, starts
   no runs, edits no files, ships nothing. The two-step law of 0005 applied
   one level up: Muninn proposes the way `ready` precedes `shipped` — the
   sole entry into anything happening remains the operator's own recorded
   command.

3. **Every proposal is durable evidence.** Proposals are recorded
   append-only, timestamped, and cite the journal facts they were derived
   from — run ids and sequence numbers — so a later reader can ask what
   Muninn saw, what it advised, and whether it was right. Proposals are
   NEVER written into any run's journal: the run engine is the single
   writer of run journals, and Muninn's record lives beside them, not
   inside them.

4. **Muninn is a bounded seat, not a daemon with tenure.** An invocation
   runs under the driver fleet with 0006-style bounds — a deadline, no
   retry ladder of its own — and produces its report or nothing. Standing
   presence is an operator choosing to invoke it again, not a process that
   owns the machine.

5. **The machine's mouth stays plain** (0019 law 4). The command carries
   the persona name — that is what persona names are for — but its output,
   errors, and report text are plain mechanic language. No verse.

6. **Delegation exists only as a future, explicit, recorded grant.** v1 is
   read-and-propose, nothing else. If the operator ever grants Muninn a
   standing authority ("may retry determinate parks up to N"), that grant
   is its own decision, and every exercise of it is journaled as the
   grant's, not as the operator's.

## Consequences

The operator gains an aide that compresses the fleet into their attention
without ever holding their pen, and the product gains something no
coordination tool has: an overseer whose judgment is itself evidence —
proposals with provenance, auditable against what actually happened. The
cost is discipline at the seams: the moment a Muninn proposal executes
anything without an operator command between, this decision has been
violated, and the violation will be visible in the journal — which is the
point of writing it down.
