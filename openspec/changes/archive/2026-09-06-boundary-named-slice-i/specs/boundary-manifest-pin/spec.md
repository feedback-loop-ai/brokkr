# boundary-manifest-pin

## Purpose

The resolved boundary is part of a bundle's identity: `run-manifest.v9`
pins it per site so that a run under one boundary and a run under
another are two identities (decision 0046 ruling 1; decision 0043
ruling 4; decision 0021 ruling 5).

## ADDED Requirements

### Requirement: The manifest pins the boundary per site as run-manifest/v9
`contracts/run-manifest.v9.schema.json` SHALL be v8 plus one property
`boundary`: an object from hands site label to the five-word enum,
required exactly when `hands` is present and forbidden otherwise, which
the schema states with `dependencies` in both directions. `manifest_for`
SHALL write `boundary` beside `hands` with one entry per hands site,
every entry carrying the realm's word, and SHALL write neither key for a
bundle that boxes nothing, so a plain bundle keeps its exact v8 shape
and identity. The manifest digest covers the whole manifest, so the word
is part of the identity by construction (decision 0046 ruling 1;
decision 0043 ruling 4).

#### Scenario: Present exactly with hands
- **WHEN** a bundle with two boxed sites compiles under `namespace`
- **THEN** its manifest carries `hands` with two keys and `boundary` with the same two keys, each `namespace`, and validates against `run-manifest.v9`

#### Scenario: Absent without hands
- **WHEN** a bundle with no boxed site compiles
- **THEN** its manifest carries neither `hands` nor `boundary`, validates against v9, and keeps the digest it had before this change

#### Scenario: The word is part of the identity
- **WHEN** the same boxed bundle compiles under `namespace` and under `harness`
- **THEN** the two manifests differ only in `boundary` and the two digests differ

#### Scenario: The schema rejects a half-pinned manifest
- **WHEN** a manifest carries `hands` without `boundary`, or `boundary` without `hands`, or a `boundary` value outside the enum
- **THEN** it does not validate against `run-manifest.v9`

#### Scenario: The schema file is a contract beside the frozen ones
- **WHEN** the frozen-contracts test runs
- **THEN** `contracts/run-manifest.v9.schema.json` exists with the title `Forge run manifest v9`, the v1 to v8 manifest files keep their bytes, and every witness manifest validates against v9

### Requirement: Every pinned digest that moves is re-pinned with 0046 as the reason
Every witness digest in `crates/brokkr-runtime/tests/witness_digests.rs`
and every compose pin in
`crates/brokkr-runtime/src/bundle/compose_tests.rs` that moves — because
the manifest gained `boundary`, or because an adapter file gained
`hands.harness` and an inline gate pins that adapter's digest (decision
0021's witness) — codex's in this slice, claude's when the operator's
measurement lands as a data change — SHALL be re-pinned from the tests' left/right pairs,
and the pin file's doc comment SHALL name decision 0046 as the reason
each moved. A bundle that boxes nothing and consults neither changed
adapter SHALL keep its digest, which is the witness that the key is
absent by default.

#### Scenario: The witness table names the reason
- **WHEN** the witness test's doc comment is read
- **THEN** it names decision 0046 as the reason every bundle declaring hands moved once (the manifest's `boundary` key) and as the reason an inline gate on an adapter that gained `hands.harness` moved — codex's in this slice, claude's when its measurement lands

#### Scenario: A plain bundle is a fixed point
- **WHEN** the witness table is compared with the tree before this change
- **THEN** every pinned bundle that declares no hands and pins neither the codex nor the claude adapter keeps its digest

### Requirement: The pinned boundary survives resume and is refused by the Looper lineage
`bundle_manifest_from_run` SHALL carry `boundary` through the resume
comparison unchanged, so a run resumes only under the boundary it was
started with; `manifest_diff`, which today reports a non-file difference
only as `non-file manifest fields differ (engine or contract version)`,
SHALL name `boundary` when it is the field that differs, so the refusal
says what moved rather than blaming a version; and `build_run_manifest_v2`
SHALL refuse a bundle manifest carrying `boundary` exactly as it refuses
every key beyond the six the v2 round-trip carries (contracts README,
the v2 lineage; decision 0046 ruling 1).

#### Scenario: A changed boundary is a manifest mismatch
- **GIVEN** a run started under `namespace`
- **WHEN** the engine is handed a bundle compiled under `harness` to resume it
- **THEN** resume refuses with a manifest mismatch whose diff names `boundary`

#### Scenario: Dispatch refuses a boxed bundle by its keys
- **WHEN** a manifest carrying `hands` and `boundary` is offered to the v2 lineage
- **THEN** it is refused naming the keys the round-trip cannot carry, before any journal is created
