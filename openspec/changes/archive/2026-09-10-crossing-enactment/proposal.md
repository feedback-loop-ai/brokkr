# Change: The crossing, enacted — decision 0057, Phase 2

## Why

Decision 0023 ruled the world and deferred the paths between its realms;
decision 0057 (proposed, 2026-09-09) ruled the vocabulary — a realm
publishes a file and another pins its raw bytes. Slices (i)–(vi) of
Phase 2 built the rest: the pure refusals, the loader that resolves and
verifies at world load, the `run-manifest/v10` recording, the refusals
at `run`, `rerun`, `resume` and `compile`, doctor's report, and the
`realms` and `muninn` readouts. None of it has ever been folded into the
living OpenSpec truth: the capability tree holds only the boundary
family from decision 0046. This change pays that debt in one change, so
the specs describe a machine that declares, resolves, verifies, records,
refuses and reads crossings.

## Context

A crossing is a FILE the publishing realm owns, named
repository-relative on the terms `house` and `dialect` are named. A
consuming realm names the publisher and pins a lowercase 64-character
hex sha256 over the published file's RAW bytes, never a canonical form.
The map is `forge.realms/v5`; absent both properties a v5 map reads
exactly as v4, and every earlier map keeps loading unchanged.
`brokkr-core` judges the shape and performs no I/O (decision 0003). The
loader in `brokkr-runtime` resolves each published file against the
publishing realm's own tree and holds each pin to the bytes it finds,
and a crossing that moved refuses `run`, `rerun`, `resume` and `compile`
before a seat spawns. A run records what it stood on in
`run-manifest/v10`, a SIBLING of the map pin, so declaration and
observation stay two facts. `brokkr doctor` reports and refuses nothing;
`brokkr realms` and `brokkr muninn run` read the same already-computed
report, and a moved pin is a finding of the consuming realm, whose run
would refuse.

## What Changes

1. **The word.** `contracts/realms.v5.schema.json` adds `publishes` and
   `consumes`; the six pure refusals, the self-consumption refusal and
   the written-`null` refusal are judged in `brokkr-core` (decision
   0057 rulings 1–3; commit `c02d97d`).
2. **The loader.** `World::load` resolves each publication and verifies
   each pin against the raw bytes, carrying what is not true as data
   (commit `07ed4ab`).
3. **The refusal.** `run`, `rerun`, `resume` and `compile` refuse a
   moved crossing before a seat spawns, in one wording every surface
   reads from the loader (commits `b9b2ee6`, `ca0c765`).
4. **The record.** `run-manifest/v10` carries the observed digests a run
   stood on, as a sibling of the map pin (commit `9111ee2`).
5. **The readouts.** `brokkr doctor`, `brokkr realms` and
   `brokkr muninn run` read the one report; no surface opens a crossing
   file, hashes a byte or compares a pin a second time (Phase 2 slice
   vi).

## Capabilities

### New Capabilities

- `realm-crossing`: the crossing word and its refusals, the loader that
  resolves and verifies it at world load, the `run-manifest/v10`
  recording of the digests a run stood on, the refusal of the verbs that
  start or continue a run, doctor's report, and the `realms` and
  `muninn` readouts.

### Modified Capabilities

None. `openspec/specs/` holds the boundary family; the crossing
capability is new.

## Impact

- **Contracts:** `contracts/realms.v5.schema.json` and
  `contracts/run-manifest.v10.schema.json` already landed in slices (ii)
  and (iv) beside their frozen predecessors. This change verifies their
  rows in `contracts/README.md` and changes no bytes.
- **Crates:** `brokkr-core` (the v5 shape and the pure refusals),
  `brokkr-runtime` (the loader, `CrossingReport`, the `run-manifest/v10`
  pin), `brokkr-cli` (`doctor`, `realms`, `muninn`).
- **Frozen surfaces:** untouched. No new event type, policy input,
  contract version or manifest field beyond the already-landed v10.
- **Docs:** `docs/guides/read-surfaces.md`, `ARCHITECTURE.md`, and an
  addendum to decision 0057.

## Decisions

None open. Decision 0057 and slices (i)–(vi) rule every fact this change
describes, and the fold adds no behavior. The two questions 0057 leaves
open — Ratatoskr transport for realms that are not co-located, and
whether a consumer may plant a keep-ref in a publisher's repository
(0028's gap) — stay open and are recorded in 0057's close-out addendum,
not resolved here.
