# Admission reads another build's reservation through the view every build writes (#772)

## Why

Admission reads every reservation in the protected inventory
(`HOME/.local/state/brokkr/capabilities/`), every realm's, whole and
strictly. A live lanetally work order started by an older Brokkr wrote each
boxed writer's `sealed` as a list, which main reads as one record since
U8a2c (`b6475fda`). Main could not parse it, called it an unknown
participant, and refused every dispatch, an exec gate's included, for as
long as that run stood. This blocks every build from main on a host where
lanetally runs, and epic #790 waits on it.

## What changes

- This run's own reservation is still read whole and strictly
  (decision 0090 ruling 1).
- Every other run's reservation is read whole where this build can, and
  otherwise through a participation view of only what the lease, the
  comparison, the sources' comparison and recovery read. Unknown fields
  are ignored, an unknown `protects` reads as evidence and an unknown
  `cleanup` as unproven, and a record the view cannot read refuses by name
  (ruling 2).
- A record read only through the view is never sealed again. Its proven
  attempts are released for one admission at a time (ruling 3).
- The view reads an optional `format` (absent or 1) and refuses any other
  by name with the new `Unknown::Formatted`. This build writes none
  (ruling 4).
- Decision 0090, `Status: proposed`, amends 0065. It carries a dated marker
  in slice two's D6 and U8a2a text.

## Not changed

This run's own record's schema, `hands::Sealed`, `hands::seal` and
`drifted`. The inventory's directory-level refusals. No contract, policy,
reference, fixture, recipe or agent. No flag or knob: the fence stays the
one switch.

## Impact

`crates/brokkr-runtime/src/engine/broker/`: `inventory.rs` and the new
`inventory/standing.rs`, `admission.rs`, `exclusion.rs`, `settlement.rs`,
`sources.rs` and `root.rs`. Tests: the new `admission/tests/foreign.rs`,
`an_unknown_participant_refuses_before_anything_is_sealed` moved to this
run's own file, and call sites adapted to the view's types. Every broker
type is `pub(super)` or narrower, so no public API moves.
