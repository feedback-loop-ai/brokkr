# 0017 — Composable recipes: extend, override, and compose delivery strategies

Status: accepted (operator ruling in chat, 2026-08-29)

## Context

Decision 0010 gave recipes as a library: whole bundles, named,
installable, swappable, comparable. What it did not give is
**composition**. Wanting the SDD design council with a different review
panel means copying an entire recipe and editing two files, and the
copy drifts from its origin the moment either changes.

With agents referenceable (decision 0016), the remaining duplication is
structural: phases, seats and policy repeated across recipes that
differ in one place.

## Decision

A recipe may **extend** another recipe and state only its differences.

- `extends: "<recipe name>"` names the base, resolved from the recipe
  library at compile time.
- The derived recipe may **add** phases and seats, **override** named
  ones wholesale, and **remove** them explicitly.
- Composition is **one level of intent, arbitrarily deep in
  resolution**: a recipe may extend a recipe that extends another, and
  the chain resolves deterministically. A cycle is a compile error
  naming the cycle.

### Merge rules, chosen for legibility over cleverness

1. **Named things merge by name; conflicts are explicit or errors.**
   Adding a phase or seat the base does not have is an addition.
   Redefining one the base has requires marking it an override, so an
   accidental collision fails compilation instead of silently winning.
   Removal is explicit and fails if the target does not exist.
2. **Policy rules compose by first-match-wins order**, which is the
   engine's existing semantics — derived rules precede base rules, and
   the resolved table is what the machine loads. A derived recipe may
   not weaken the constitutional lint: the protected review phase stays
   unavoidable (decision 0005), checked on the RESOLVED table.
3. **The composition is resolved before anything runs.** What the
   engine sees is a single, flat, fully-resolved bundle — there is no
   inheritance at run time, no dynamic lookup, no surprise.

### Pinning

The **resolved** bundle's digest is what pins a run, as today. The
manifest additionally records the composition chain — each ancestor
recipe and its digest — so a run states not just what it ran but what
it was composed from. Changing a base recipe therefore changes the
digest of everything derived from it, which is correct: it is a
different strategy.

## Constraints

- Resolution is a **pure function** over (recipe sources) → resolved
  bundle, unit-tested; the same inputs give byte-identical output, and
  a golden test pins a composed bundle's manifest.
- `forge compile` and `forge recipes show` display the resolved result
  AND its provenance, so an operator can always see what a composition
  actually produced.
- Frozen v1 contracts, the corpus and the heritage policy table are
  untouched.
- Errors name the file and the conflicting key — a composition failure
  must be diagnosable without reading the resolver.

## Consequences

- Recipes become small: "SDD, but with the paranoid review panel" is a
  file with `extends` and one override.
- The experiment surface widens — swap one phase, rerun, `forge
  compare` — which is what decision 0010 was aiming at.
- Copies stop drifting silently, because a derived recipe now tracks
  its base by construction.
