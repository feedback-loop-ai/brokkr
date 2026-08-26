# 0011 — Standalone identity: the heritage is a directory, not a dependency

**Status**: accepted (operator ruling, 2026-08-26 — "this project has
nothing to do with the origin workspace")

## Ruling

The Forge is a standalone product. The workspace it was extracted from
is heritage, and heritage is confined to `reference/` — read-only
provenance (the referee-era control plane, the retired oracle, the
original charters and skill prose, and now the recorded referee-era
JSON Schemas, relocated from `policy/schemas/`).

What this changes:

- Living documents (README, extension model) no longer name the origin
  workspace or point the roadmap at it. The forward roadmap item
  formerly called "the the origin workspace vertical slice / profile" (decisions
  0003, 0005, 0008 and the target architecture) is SUPERSEDED as: **the
  first external workspace profile** — same milestone, no privileged
  workspace. Accepted decision documents themselves are historical
  record and are not rewritten.
- `policy/phase-machine.json` stays, byte-identical, under a new
  description of what it now is: the heritage transition table the
  engine was extracted around, retained as the strict evaluator's
  differential-test fixture — the frozen corpus derives from it, so its
  stability is contract, not nostalgia. The "upstream parity" rationale
  is void; the read-only rule stands on the corpus instead.
- Nothing in the engine, bundles, contracts, or tests referenced the
  relocated schemas; the differential suite reads only the heritage
  table, whose bytes are unchanged.

## Why

Provenance earned its place while the engine was a port with a parity
obligation. That obligation ended at decision 0009; keeping the origin
in the product's forward-facing voice after that misstates what this
is: a general deterministic delivery engine whose first users are its
own repository and whatever workspace adopts a profile next.
