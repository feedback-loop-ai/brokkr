# 0022 — Contract version lifecycle: who writes and reads each file in `contracts/`, and under which ruling, measured from the code

Source: https://github.com/feedback-loop-ai/brokkr/issues/361
Authors: Brokkr's implementer seat (Claude Opus 5.5), from the issue's measurement of `ba52724e`
Read: 2026-10-11
Status: proposed
Intake: run `design-story-361-contract-versio-157ad85e`, on main at `3ad0f66c` (code-health epic #330)

## Summary

Issue #361 measured `contracts/` at `ba52724e` (2026-09-25): 32 schema
files in four version families, each kept for its readers indefinitely,
only `seat-record` validated by a schema at runtime, twelve files missing
from `docs/guides/versioning.md`'s status table, no schema file for
`forge.phase-machine/v1`, and four frozen versions minted under decisions
that are still `proposed`. It asks for two rulings, one on minting and one
on retirement, a lifecycle table checked against the tree, and an answer
on `forge.phase-machine/v1`.

This entry measures the same questions at `3ad0f66c`, sixteen days and
ten contract files later. Every claim cites a file and line read at that
base. The counts:

- **42 schema files in 16 families.** Run manifest 13 (two lineages),
  realms 8, seat record 6, specification dialect 3, and twelve families of
  one file each. `contracts/README.md` is the forty-third file and is not
  a schema.
- **17 files are current, 14 are older versions still read, and 11 are
  record only.** A record-only file is an older version that no reader in
  this engine accepts as such. Its bytes describe what older journals
  hold. All 11 are local-lineage run manifests.
- **7 files are validated by a schema at runtime**, against embedded
  copies: the six seat records and the tool dialect. The other 35 are
  read by hand-written Rust or by nothing, and their schemas are exercised
  only in tests, where they are exercised at all.
- **28 files are pinned by digest, and 14 are not.** Four of the 14 are
  the newest version of a multi-version family, two are superseded
  dialect versions, and eight are the only file of their family.
- **22 files have no row in `versioning.md`'s status table**, which names
  20 of the 42. Three of its sentences are stale.
- **Minting authority.** 33 files were minted under an accepted ruling.
  Five were minted under a ruling that is still `proposed`: the issue's
  four and `effect-cleanup.v1`. One was minted under a decision with no
  file in the tree, `realms.v7` under 0075. One, `run-manifest.v13`, was
  minted on the operator's in-chat ruling, which no decision text records.
  Two, the Looper pair, name no decision at all.
- **`forge.phase-machine/v1` still has no schema file.** Its only
  instance is the frozen production table. Every shipped recipe table now
  declares v2, `recipes/preflight` included, which the issue listed as a
  v1 user.

Two adjacent hazards came to light. The phase-machine loader reads any
label other than v2 under v1's rules, so it accepts a label it does not
know without refusing it. `brokkr init` writes `forge.realms/v3` or `v4`
while v8 is current. Both are named below as findings and left unruled.
Proposed decision 0082 rules on the rest.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Keep every version's bytes beside its successors, and never edit a published version | implemented | `contracts/README.md`, `crates/brokkr-runtime/tests/frozen_contracts.rs` |
| 2 | Pin each contract file's bytes by digest on the day it lands, not when its successor arrives | planned | #361 |
| 3 | Choose the contract a journal record is judged against from the engine line that wrote it | implemented | decision 0035, `crates/brokkr-store/src/seat_record.rs` |
| 4 | Validate a record at runtime against its published schema, embedded so a packaged binary verifies offline | implemented | `crates/brokkr-store/src/seat_record.rs`, `crates/brokkr-runtime/src/capabilities/dialect.rs` |
| 5 | Refuse a version label the reader does not know, by name, before giving the bytes a shape | implemented | decision 0026, `crates/brokkr-core/src/realms/unusable.rs`, `crates/brokkr-runtime/src/dialect.rs` |
| 6 | Refuse a phase-machine table whose label is neither v1 nor v2 | not-planned | |
| 7 | Mint a contract version only under a ruling the operator has accepted | planned | #361 |
| 8 | Keep one table of every version with its writer, reader and state, checked against the tree by a test | planned | #361 |
| 9 | Retire an older version's reader after a stated window, and verify older journals with the engine that wrote them | planned | #361 |
| 10 | Publish a schema for the v1 phase-machine table | alternative | `policy/phase-machine.json`, `crates/brokkr-core/tests/differential.rs` |
| 11 | Have `brokkr init` write the newest realms label | not-planned | |

Row 6 is new and unruled. `Machine::parse` reads the table's `schema` as
an optional string (`crates/brokkr-core/src/policy.rs:244`) and compares it
only with v2 (`policy.rs:229`, `:424`, `:453`). A table that calls itself
`forge.phase-machine/v3`, or misspells v1, is read under v1's open rules.
`TABLE_SCHEMA_V1` (`policy.rs:69`) is named by no production line, only by
tests (`crates/brokkr-core/src/policy/tests.rs:401`,
`crates/brokkr-core/tests/policy_lint.rs:249`). Row 11 is also new:
`init.rs:1197-1203` writes v4 when a boundary is chosen and v3 otherwise,
and Brokkr's own `realms.json:2` is v3. Neither is wrong under today's
rules, since every older realms label keeps loading. Each becomes a
question once a realms reader can retire. Row 10 is an alternative. The v1
table is pinned by its bytes and by the evaluator corpus's differential
test, and decision 0082's ruling 5 records why that stands in for a
schema.

## The families, counted

| Family | Files | Label inside the file | Current | Older, still read | Record only |
|---|---|---|---|---|---|
| Run manifest, local lineage | 12 (v1, v3–v13) | none | v13 | none | v1, v3–v12 |
| Run manifest, Looper lineage | 1 (v2) | `forge-run-manifest/v2` | v2 | none | none |
| Seat record | 6 | none | v6 | v1–v5 | none |
| Realms map | 8 | `forge.realms/vN` | v8 | v1–v7 | none |
| Specification dialect | 3 | `brokkr.dialect/vN` | v3 | v1, v2 (pins only) | none |
| Phase-machine table | 1 (v2; v1 has no file) | `forge.phase-machine/vN` | v2 | v1 (no file) | none |
| Event envelope | 1 | none (`event_schema: 1`) | v1 | none | none |
| Driver protocol | 1 | `forge-driver/v1` | v1 | none | none |
| Dispatch envelope | 1 (v2) | `forge-dispatch/v2` | v2 | none | none |
| Effect provenance, boundary, cleanup | 3 | none (payload extensions) | each v1 | none | none |
| Phase-entered head, case | 2 | none (payload extensions) | each v1 | none | none |
| Operator supersede | 1 | none (payload value) | v1 | none | none |
| Tool dialect | 1 | `brokkr.tool-dialect/v1` | v1 | none | none |
| Host configuration | 1 | `forge.host/v1` | v1 | none | none |

Current is 17 files, older still read is 14, and record only is 11, for
42 in all. "Current" is the newest version of its family or lineage.
"Older, still read" counts files only. The phase-machine v1 label has no
file, so it is not counted.

## Who writes and reads each version

The writer is who produces a file or record in that version today. The
reader is the production code that accepts it. "Schema at runtime" means
a `jsonschema` validator over an embedded copy of the published file.
"Validated" names where a test compiles the published schema. Frozen means
pinned by digest in `crates/brokkr-runtime/tests/frozen_contracts.rs`.

### Run manifest

The local lineage carries no version label. The compiler writes one shape,
today's v13. The capability section is written at
`crates/brokkr-runtime/src/bundle.rs:7092`, and the model pins are written
by `crates/brokkr-runtime/src/bundle/pins.rs:1`. No reader takes an older
local manifest as its own version. Resume recompiles the bundle and
refuses any pinned manifest that differs from it
(`crates/brokkr-runtime/src/engine.rs:651-665`). Since v11, every
compiled bundle pins a `capabilities` section, and since v13 every
candidate carries `model_pins` (`contracts/README.md:843-862`). So every
run pinned under v1 or v3–v12 meets that refusal, and none is rewritten.
Export and verify carry the manifest's bytes inside `run/started` under
the event envelope. The store reads one field from it, `engine`, to
dispatch seat records
(`crates/brokkr-store/src/seat_record.rs:144-146`).

| File | Minted under | Writer today | Reader today | Validated | Frozen |
|---|---|---|---|---|---|
| `run-manifest.v1` | decisions 0003–0005, accepted (`contracts/README.md:3-14`) | none | none as v1 (record) | no test | yes |
| `run-manifest.v2` | no decision; PR #28, in the Looper composition decision 0008 scopes | `build_run_manifest_v2` (`crates/brokkr-core/src/dispatch.rs:392`), which refuses every compiled bundle since decision 0065 (`contracts/README.md:688-694`) | `bundle_manifest_from_run` (`dispatch.rs:477`, label check `:478`) | no test | yes |
| `run-manifest.v3` | 0016, accepted | none | none as v3 (record) | no test | yes |
| `run-manifest.v4` | 0023, accepted | none | none as v4 (record) | no test | yes |
| `run-manifest.v5` | 0021, accepted | none | none as v5 (record) | no test | yes |
| `run-manifest.v6` | 0043, accepted | none | none as v6 (record) | no test | yes |
| `run-manifest.v7` | 0041 ruling 7, accepted | none | none as v7 (record) | no test | yes |
| `run-manifest.v8` | 0041 ruling 8, accepted | none | none as v8 (record) | no test | yes |
| `run-manifest.v9` | 0046, accepted | none | none as v9 (record) | `crates/brokkr-runtime/src/engine/tests.rs:4223` | yes |
| `run-manifest.v10` | 0057, **proposed** | none | none as v10 (record) | `frozen_contracts.rs:422` | yes (`frozen_contracts.rs:593-603`) |
| `run-manifest.v11` | 0065 ruling 8, accepted | none | none as v11 (record) | `frozen_contracts.rs:947` | yes |
| `run-manifest.v12` | 0065 slice two, accepted | none | none as v12 (record) | `frozen_contracts.rs:1027`, `:1169` | yes |
| `run-manifest.v13` | **no decision text**: the operator's in-chat ruling of 2026-10-10 on #430, recorded in PR #621 and `contracts/README.md:835-841`; it serves decision 0068 ruling 3 | the compiler (`bundle.rs:7092`, `bundle/pins.rs:1`) | resume comparison (`engine.rs:651-665`); admission reads the pins (`crates/brokkr-runtime/src/admission/capacity.rs:10`) | `engine/tests.rs:4167`, `crates/brokkr-runtime/tests/witness_digests.rs:326`, `frozen_contracts.rs:1100` | no |

### Seat record

Seat records carry no version marker. The store picks the contract from
the `engine` string of the run's manifest
(`crates/brokkr-store/src/seat_record.rs:121-137`). The line boundaries
are `V2_ENGINE` 0.8 (`:67`), `V4_ENGINE` 0.9 (`:76`), `V5_ENGINE` 0.10
(`:92`) and `V6_ENGINE` 0.12 (`:103`). An engine before 0.8, or one that
does not parse, reads as v1. All six schemas are embedded
(`seat_record.rs:32-37`) and held to the published files (`:39-44`).
Append, export, import verification and offline verification all use the
same dispatch (`contracts/README.md:804-812`). The engine's own line,
0.12, writes v6. Drivers write the records the engine stamps. Every
version is read by the store with a schema at runtime, and the store's
tests exercise each through its embedded copy
(`crates/brokkr-store/src/seat_record/tests.rs`: v1 from line 63, v2 from
140, the dispatch at 202-220).

| File | Minted under | Last engine line it is dispatched for | Frozen |
|---|---|---|---|
| `seat-record.v1` | 0034, accepted | 0.7 (and any unparsable engine) | yes |
| `seat-record.v2` | 0035 ruling 7, accepted | never: v3 shares its line and wins (`seat_record.rs:121-128`) | yes |
| `seat-record.v3` | 0034 ruling 7, operator ruled 2026-09-05, accepted | 0.8 | yes |
| `seat-record.v4` | 0046 ruling 3, accepted | 0.9 | yes |
| `seat-record.v5` | 0056 ruling 7, **proposed** | 0.10 and 0.11 | yes |
| `seat-record.v6` | 0065 ruling 8 (slice two), accepted | 0.12 onward (current) | no |

`seat-record.v2` is the one file in the tree that is embedded, published
and pinned but that no input can select. The code says so itself
(`seat_record.rs:121-128`).

### Realms map

Every label loads (`crates/brokkr-core/src/realms.rs:73-106`). An unknown
label is refused by name (`crates/brokkr-core/src/realms/unusable.rs:80-81`),
and a word used under a label older than the one that introduced it is
refused (`unusable.rs:43-47`). The map is the operator's file. The engine
writes one only in `brokkr init`, at v3 or v4
(`crates/brokkr-cli/src/init.rs:1197-1203`). The map rides into the run
manifest inside the realms pin. A realms label therefore has two kinds of
reader: the operator's live map, and an older run's pin.

| File | Minted under | Writer today | Reader today | Validated | Frozen |
|---|---|---|---|---|---|
| `realms.v1` | 0023, accepted | operator | core loader | `frozen_contracts.rs:668-688` | yes |
| `realms.v2` | 0026, accepted | operator | core loader | `frozen_contracts.rs:668-688` | yes |
| `realms.v3` | 0041 ruling 8, accepted | operator; `brokkr init` without a boundary; Brokkr's own `realms.json` | core loader | `frozen_contracts.rs:362` | yes |
| `realms.v4` | 0046 ruling 1, accepted | operator; `brokkr init` with a boundary | core loader | `frozen_contracts.rs:383` | yes |
| `realms.v5` | 0057, **proposed** | operator | core loader | `frozen_contracts.rs:489` | yes (`frozen_contracts.rs:593-603`) |
| `realms.v6` | 0065 ruling 3, accepted | operator | core loader, with grants judged in `crates/brokkr-core/src/realms/grants.rs` | `frozen_contracts.rs:625` | yes |
| `realms.v7` | **"proposed decision 0075 ruling 5"** (`realms.rs:96-98`, `frozen_contracts.rs:54`), a number with no file: `docs/decisions/README.md:162` lists it as claimed by draft PR #486; PR #526 ported its tier onto decision 0065 (#487) | operator | core loader (`crates/brokkr-core/src/realms/provisional.rs:31`) | `frozen_contracts.rs:704` | yes |
| `realms.v8` | 0065 slice two, accepted | operator | core loader (`grants.rs:186`) | `frozen_contracts.rs:786` | no |

### Specification dialect

A dialect file is read at v3 only (`crates/brokkr-runtime/src/dialect.rs:262-268`).
A dialect pin, the resolved content a run embeds in its manifest, is read
at any of the three (`dialect.rs:275-277`). Each version is held to its
own shape (`dialect.rs:331-362`). Both shipped dialects,
`dialects/openspec.json` and `dialects/speckit.json`, are v3.

| File | Minted under | Writer today | Reader today | Validated | Frozen |
|---|---|---|---|---|---|
| `dialect.v1` | 0042, accepted | none | `parse_pinned` only | no test | **no** |
| `dialect.v2` | 0042's addendum of 2026-09-04, operator ruled | none | `parse_pinned` only | no test | **no** |
| `dialect.v3` | 0042's addendum of 2026-09-06, operator ruled | shipped dialect files | `parse` and `parse_pinned` | `crates/brokkr-runtime/src/dialect/tests.rs:33` | no |

Decision 0081 (PR #631) rules on 0042's 2026-09-04 addendum in part. It is
not on main at this base, and it rules no contract version.

### The single-file families

| File | Minted under | Writer today | Reader today | Validated | Frozen |
|---|---|---|---|---|---|
| `event-envelope.v1` | decisions 0003–0005, accepted | the engine, every event (`crates/brokkr-core/src/envelope.rs:1`) | store, fold, export and verify | no test compiles the schema | yes |
| `driver-protocol.v1` | decisions 0003–0005, accepted | every driver | `crates/brokkr-protocol/src/lib.rs:1` | no test compiles the schema | yes |
| `dispatch-envelope.v2` | no decision; PR #28, in the Looper composition decision 0008 scopes | Looper | `DispatchEnvelopeV2::verify` (`crates/brokkr-core/src/dispatch.rs:246`) | no test | yes |
| `effect-provenance.v1` | 0016, accepted | engine (`crates/brokkr-runtime/src/engine.rs:1423`) | resume (`crates/brokkr-runtime/src/engine/resume.rs:683`), view (`crates/brokkr-view/src/lib.rs:1051`), start-failure readout (`engine.rs:2858`) | no test | yes |
| `effect-boundary.v1` | 0046 ruling 3, accepted | engine (`engine.rs:1429`) | view (`brokkr-view/src/lib.rs:1208-1210`) | `boundary_tests.rs:4023` | no |
| `effect-cleanup.v1` | 0006's addendum of 2026-09-27 (#403), **proposed** (`docs/decisions/0006-bounded-attempts-and-deadlines.md:63`) | engine (`engine.rs:107-137`, `crates/brokkr-protocol/src/lib.rs:210`) | none: the fold never reads it (`contracts/README.md:231`), and no readout does | `crates/brokkr-runtime/src/engine/cleanup_tests.rs:181` | no |
| `phase-entered-head.v1` | 0039, **proposed** | engine (`engine.rs:4636-4652`) | the docs-tier check (`engine.rs:4676-4684`) | no test compiles the schema; `is_commit_id` names it (`engine.rs:4701`) | no |
| `phase-entered-case.v1` | 0041 ruling 7, accepted | engine (`engine.rs:4646`) | none: resume recomputes selection (`contracts/README.md:214-215`) | no test | no |
| `operator-supersede.v1` | 0047 ruling 1, accepted | the verb (`crates/brokkr-runtime/src/engine/operator/supersede.rs:30-35`) | view (`brokkr-view/src/lib.rs:738`) | `engine/tests.rs:6465` | no |
| `phase-machine.v2` | 0022, accepted | the nine shipped recipe tables | `Machine::from_table` (`crates/brokkr-core/src/policy.rs:229`) | no test compiles the schema; `crates/brokkr-core/src/policy/errors.rs:48` names it | no |
| `tool-dialect.v1` | 0065 ruling 2, accepted | shipped `dialects/tools/*.json` (three files) | schema at runtime (`crates/brokkr-runtime/src/capabilities/dialect.rs:20`, `:322`) | `crates/brokkr-runtime/src/capabilities/tests.rs:374-380`, `frozen_contracts.rs:849` | no |
| `host.v1` | 0068's addendum of 2026-10-10, ruled (`docs/decisions/0068-the-dispatcher.md:138`) | operator (`$XDG_CONFIG_HOME/brokkr/host.json`) | `crates/brokkr-runtime/src/admission/host.rs:50` | `crates/brokkr-runtime/src/admission/host/tests.rs:81-83`, `frozen_contracts.rs:762` | no |

## Minting authority

The ruling each file was minted under is read from the comment that lands
it in `frozen_contracts.rs` (lines 25-157 and 189-357), from
`contracts/README.md`'s landing paragraphs, from the commit that added the
file (`git log --diff-filter=A -- contracts/`) and from the decision's own
status line.

| Authority | Files | Count |
|---|---|---|
| An accepted decision or an operator-ruled addendum | everything not listed below | 33 |
| A ruling still `proposed` | `seat-record.v5` (0056), `realms.v5` and `run-manifest.v10` (0057), `phase-entered-head.v1` (0039), `effect-cleanup.v1` (0006's #403 addendum) | 5 |
| A decision with no file in the tree | `realms.v7` (0075) | 1 |
| An operator ruling no decision records | `run-manifest.v13` (#430, PR #621) | 1 |
| No decision named | `dispatch-envelope.v2`, `run-manifest.v2` | 2 |

The issue counted four. The fifth, `effect-cleanup.v1`, landed on
2026-09-29 under an addendum to an accepted decision, and the addendum's
own heading says `proposed`. The issue's measurement could not tell an
addendum from its decision. `forge.host/v1` landed with `run-manifest.v13`
in the same commit, `3ad0f66c`. It is recorded in 0068's 2026-10-10
addendum, whose heading says `ruled`. Read by hand, it counts as
accepted. A test cannot read headings that way, as the next section
shows.
`run-manifest.v13` is named in no decision. The decisions README says a
declined proposal "stays in the PR record, not in this directory". Seven
files' authority could therefore leave the tree, or has never been in it,
while their frozen bytes stay.

## Addendum headings and reader sources

An addendum's status is written only in its heading, and the headings
have no closed form. `grep '^## Addendum' docs/decisions/*.md` finds 25
in 17 decisions. They say `operator ruled:` (0021:97, 0046:240),
`, proposed` at the end (0006:63) or `proposed:` after the date
(0043:254), `accepted` (0035:372, 0039:145), `, operator ruling`
(0029:139), `confirmed by the operator` (0041:420), `the operator's
ruling` (0035:286) and `, ruled` (0068:138). Three carry no status word
at all (0029:166, 0029:205, 0057:197), and 0057's uses a colon where the
rest use a dash. Two headings share a date in one file (0021:97 and
:112), so a date alone does not always name an addendum. A
heading such as "not yet ruled" would hold the word `ruled`. Four addenda
mint a version: 0042's of 2026-09-04 (0042:503) and 2026-09-06
(0042:579), 0068's of 2026-10-10 (0068:138) and 0006's of 2026-09-27
(0006:63). `seat-record.v3`'s ruling is not an addendum. It is 0034
ruling 7, written into the ruling list with "added 2026-09-05, operator
ruled" (`0034-the-seat-record.md:123-124`).

A label constant is not always what a loader reads. The specification
dialect keeps its three labels as constants
(`crates/brokkr-runtime/src/dialect.rs:17-23`) apart from the accepted
array `SCHEMAS` (`:34`) that `parse_pinned` reads (`:276`). The realms
map does the same (`crates/brokkr-core/src/realms.rs:73-102`, `SCHEMAS`
at `:106`, read at `:548`). Taking a version out of either array would
refuse it while its constant stayed. The typed sources each loader
exposes are:

| Family | Source | Visibility |
|---|---|---|
| Realms map | `realms::SCHEMAS`, membership (`realms.rs:106`, `:548`) | `pub` |
| Specification dialect pin | `dialect::SCHEMAS`, membership (`dialect.rs:34`, `:276`) | `pub` |
| Seat record | `SeatRecordVersion::of_engine` and `contract()` (`crates/brokkr-store/src/seat_record.rs:121`, `:151`) | `pub`, re-exported (`crates/brokkr-store/src/lib.rs:93`) |
| Driver protocol | `PROTO` (`crates/brokkr-protocol/src/lib.rs:25`), compared at `process.rs:376` | `pub` |
| Dispatch envelope, Looper manifest | `DISPATCH_SCHEMA_V2`, `RUN_MANIFEST_SCHEMA_V2` (`crates/brokkr-core/src/dispatch.rs:16-17`), compared at `:246`, `:478` | `pub` |
| Phase-machine v2 | `TABLE_SCHEMA_V2` (`crates/brokkr-core/src/policy.rs:70`), compared at `:229` | `pub` |
| Phase-machine v1 | none: `TABLE_SCHEMA_V1` (`policy.rs:69`) is read by no production line; `Machine::from_table` (`:226`) reads every non-v2 label as v1 | loader `pub` |
| Host file | a serde rename (`crates/brokkr-runtime/src/admission/host.rs:50`), read by `read` (`:282`) | `pub(crate)` |
| Tool dialect | an embedded copy (`crates/brokkr-runtime/src/capabilities/dialect.rs:20`) | `pub(super)` |

## The status table and the guide

`docs/guides/versioning.md:159` opens the "other contracts" table. Its rows
name 20 files. The 22 it does not name are `run-manifest` v1–v8, v12 and
v13; `seat-record` v1, v2, v3, v5 and v6; `realms` v3, v7 and v8;
`phase-entered-head.v1`, `phase-entered-case.v1`, `effect-cleanup.v1` and
`host.v1`. The prose above the table covers some of them: the local
lineage's bullets name the run manifests through v11, and realms through
v6. Three sentences are stale at this base. `versioning.md:50` says
`contracts/` holds `run-manifest.v1` through `.v11`, and there are thirteen.
`versioning.md:107` says the compiler "writes v11 now", and it writes v13.
`versioning.md:136` says realm maps "now reach `forge.realms/v6`", and the
current label is v8. All twelve files the issue found missing are still
missing. Of the other ten, seven arrived after the issue's base:
`run-manifest` v12 and v13, `seat-record.v6`, `realms` v7 and v8,
`effect-cleanup.v1` and `host.v1`. The issue did not count the remaining
three, `run-manifest` v1, v2 and v3, perhaps because the lineage prose
names them.

## `forge.phase-machine/v1`

The label is a constant (`crates/brokkr-core/src/policy.rs:69`) with no
production reader of its own. Any table that does not say v2 is parsed
under v1's rules: the rule level stays open, and a park is refused
(`policy.rs:424-460`). The open rule level is deliberate. The frozen
production table carries annotation keys (`crates/brokkr-core/src/policy/errors.rs:48`):
`policy/phase-machine.json` carries `detail`, `source`, `checks`,
`computed_inputs`, `ship_preconditions` and `terminal_for_ship`. It is the
only v1 table in the tree (`policy/phase-machine.json:2`). Its bytes are
pinned (`frozen_contracts.rs:163-172`), and its behaviour is pinned by the
frozen evaluator corpus through `crates/brokkr-core/tests/differential.rs:26-31`.
The test that lands `phase-machine.v2` says the v1 table schema "was never
a file here" (`frozen_contracts.rs:199-202`). No shipped recipe writes v1.
All nine `recipes/*/policy.json` declare v2.

## Method and limits

Every line number above was read at `3ad0f66c` with `grep -n` and direct
reads. The file counts come from `ls contracts/`, the dates and commits
from `git log --diff-filter=A -- contracts/`, and the statuses from each
decision's `Status:` line and addendum heading. "No test" means no Rust
file under `crates/` names the schema file outside
`frozen_contracts.rs`'s digest and title tests. A test that builds the path
from parts, as `frozen_contracts.rs:692-697` does for realms, was read by
hand. Three things were not measured. Which journals the operator still
needs to verify, by engine line, was not counted. Whether each old tag
still builds, and which tags carry release archives, was not checked.
Whether any third-party driver or Looper deployment reads a specific
version was not checked either. Decision 0082's Ruled section names all
three.
