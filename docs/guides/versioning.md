# Versioning and the v1.0 stability promise

This document states what you can build against, what may still move,
and how a change to a frozen thing is made when it has to happen.

**Where the tree is right now:** the workspace version in `Cargo.toml`
is **0.12.0**. This is pre-1.0 software. The contracts under
`contracts/` are nonetheless already frozen and have been since the
first implementation — the freeze is older than the version number and
does not wait for it. See the [v0.12.0 release notes](../releases/v0.12.0.md)
for what changed and current platform limitations, and the
[v0.10.0 notes](../releases/v0.10.0.md) for the boundary migration.

- [Two different promises](#two-different-promises)
- [The frozen-contract law](#the-frozen-contract-law)
- [The current lineages](#the-current-lineages)
- [Read-only material that is not a contract](#read-only-material-that-is-not-a-contract)
- [Semver, as of 1.0](#semver-as-of-10)
- [What may break before 1.0](#what-may-break-before-10)
- [The closed deprecation window](#the-closed-deprecation-window)
- [How a stable thing changes](#how-a-stable-thing-changes)

## Two different promises

They are worth keeping apart, because one is already in force and the
other is not yet.

| | Frozen contracts | The CLI surface |
|---|---|---|
| **Promise** | A version's bytes never change. A change is a new numbered version file beside the old one. | Semver from 1.0: breaking changes to subcommands and flags need a major bump. |
| **In force** | **Now**, and since the first implementation. Independent of the crate version. | **From 1.0.** Today, at 0.12.0, flags may still move. |
| **Enforced by** | CI compiles the frozen and additive contracts; the fixtures corpus differential-tests the evaluator; manifest schemas pin `{"const": 1}` on `event_schema`. | Convention today; release process from 1.0. |

The contracts freeze is the load-bearing one, because it is what a
driver author, a journal consumer, or an offline verifier builds
against. A `forge-driver/v1` driver written today keeps working; a
`brokkr` command line written today may need a flag renamed before 1.0.

## The frozen-contract law

From [`contracts/README.md`](../../contracts/README.md), and repeated in
the README's contributing section:

> A frozen contract changes only by a new numbered version next to the
> old one, never by editing v1 in place.

Three things follow, and all three are enforced rather than trusted:

1. **New version files sit beside the old ones.** `contracts/` today
   contains separately numbered `run-manifest.v1` through `.v14` files.
   A later version does not replace or edit the earlier versions.
2. **A version cannot quietly widen.** Schemas set
   `additionalProperties: false` and the loaders refuse unknown fields.
   `forge.realms/v1` refuses unknown keys at both levels specifically so
   that decision 0021's per-realm driver and egress constraints must
   arrive as a later version rather than as drift inside a file still
   calling itself v1. Decision 0026's per-realm `journal` did exactly
   that: it landed as `forge.realms/v2` beside an untouched v1, and the
   loader refuses the new word in a map still calling itself v1.
3. **A version cannot quietly narrow either.** The loader refuses a
   parking rule in a phase-machine table that calls itself
   `forge.phase-machine/v1` — a park is not a stop, and a machine that
   let one wear the other's version string would be lying about the
   difference the ruling exists to draw.

There is one narrow, machine-checked amendment to the freeze, made by
decision 0016 with its reasoning recorded in `contracts/README.md`. The
original rule — *"a new field is a v2 event"* — was unenforceable as
written: it forbade a field a v1 consumer can safely ignore just as
strongly as one it must read. The enforceable rule replacing it:

> Additive payload fields that are optional, absent by default, and
> published as a numbered extension schema are permitted at
> `event_schema: 1`. A field that changes the meaning of an existing
> field, or that a v1 consumer must read to fold to correct state, is a
> v2 event.

The last clause is machine-checked: `fold` never reads an extension
field, `RunState` gains nothing from one, and a run over a bundle that
references no agent journals byte-identical payloads. The extension
fields defined so far live in `effect-provenance.v1.schema.json`.

## The current lineages

There are **two** manifest lineages, not one line, and conflating them
is the mistake to avoid.

**The local lineage: `run-manifest.v1` → `v3` → `v4` → `v5` → `v6` → `v7` → `v8` → `v9` → `v10` → `v11` → `v12` → `v13` → `v14`.**

- `v3` is `v1`'s bytes plus one optional `agents` property (decision
  0016), absent when no seat references an agent — so every
  non-adopting run stores and exports the exact v1 shape.
- `v4` is `v3` plus one optional `realms` property (decision 0023)
  carrying the map's `source`, its `sha256` and the map itself. Absent
  when a run was invoked with no map, so an unmapped run stores and
  exports the exact v1/v3 shape.
- The realms digest is over the embedded content's canonical JSON, so a
  reader holding only the journal can re-derive it without the file. And
  the map is workspace data, not bundle data: the resume comparison
  drops `realms` before comparing, so pinning a world moves no bundle
  digest and makes no run unresumable.

- `v5` adds authorising adapter witnesses (`drivers`); `v6` adds boxed
  hands; `v7` records strategy-selected cases; `v8` records per-step
  result vocabularies; `v9` pins the selected boundary per hands site.
  These are bundle identity. The compiler wrote v9 until decision 0065
  and writes v14 now; older contract files remain available for their
  readers.
- `v10` is `v9` plus one optional `crossings` property (decision 0057):
  per publishing realm, per crossing name, the resolved path and the
  sha256 the loader OBSERVED on disk at run start. Absent when no realm
  publishes or consumes a crossing, so a world without one stores and
  exports the exact v9 shape. Like `realms` and unlike `drivers` or
  `boundary`, this is workspace data — the resume comparison drops it,
  so pinning a crossing moves no bundle digest. A COMPILED bundle
  manifest never carries the key at all; it rides only a RUN manifest in
  a world that draws a crossing.
  Observation is not declaration: a consumer's declared pin already
  rides inside the embedded map (`realms.map…consumes[].sha256`, from
  v4), and holding the two side by side is how a reader sees what a
  world claimed against what it stood on.
- `v11` is `v10` plus one REQUIRED `capabilities` property (decision
  0065, slice one): the operated realm, every grant it declares, every
  abstract definition and tool dialect the compile consulted with the
  sha256 of its raw bytes, and per executable site the office, its asks
  and one record per provider candidate. It is written even where nothing
  is granted and nothing is asked, so **the compiler now claims v11** and
  every bundle's digest moved once, on the day it landed. Unlike `realms`
  and `crossings` it is BUNDLE identity: the resume comparison keeps it,
  so a changed grant, scope, tool subset, restriction, definition or
  dialect byte is refused with `capabilities` named. A run started
  before this version pins no such section; resuming it recompiles a
  bundle that does, and the comparison refuses the difference rather
  than restoring a harness's old default. Historical runs are not
  rewritten.
- Realm maps now reach `forge.realms/v8`: v3 added house and dialect
  declarations, v4 adds the realm's boundary, v5 adds the crossings a
  realm publishes and the ones it consumes, each pinned by a sha256 over
  the published file's raw bytes (decision 0057), and v6 adds the
  capabilities a realm grants (decision 0065), v7 adds the provisional
  tier (decision 0075), and v8 adds the retention veto (decision 0065
  slice two). House content and its digest are pinned in the run's realms
  record. Every older version keeps loading and grants nothing.

**The Looper-bound lineage: `run-manifest.v2`, unchanged.** Its
round-trip reconstructs a bundle manifest from six named keys and drops
the rest, so an `agents`, `realms` or `crossings` key would be silently dropped and
the run would become unresumable with a diff that blames no file. Rather
than widen a contract a counterpart system reads, the engine **refuses**:
`build_run_manifest_v2` rejects every key outside its six-key round-trip, and
`brokkr run` refuses `--dispatch` together with a realms map. Lifting
either needs a jointly agreed v2-lineage manifest version. Since decision
0065 that refusal reaches **every compiled bundle**: each now pins its
capability authority under `capabilities`, the six-key round-trip cannot
carry it, and the guard names the key before any run row is written
rather than the authority being stripped to fit.

**The lifecycle of every contract.** One row per file under
`contracts/`, plus the one label that has no file. The columns are read as
data by `crates/brokkr-cli/tests/contract_lifecycle.rs`: `Minted under` by
the minting rule in `crates/brokkr-cli/tests/decisions_index.rs`, and each
`Reader` by the loader it names. A `record` row has no reader; its bytes
describe what an older journal holds. `Last written` and `Reader until`
are stated here from decision 0082 ruling 3 and are bound to the loaders
and the release window in that ruling's slice 2.

| Version | File | Class | Minted under | Writer | Reader | State | Last written | Reader until |
|---|---|---|---|---|---|---|---|---|
| `run-manifest/v1` | `contracts/run-manifest.v1.schema.json` | journal | 0003 | none | none | record | | |
| `run-manifest/v3` | `contracts/run-manifest.v3.schema.json` | journal | 0016 | none | none | record | | |
| `run-manifest/v4` | `contracts/run-manifest.v4.schema.json` | journal | 0023 | none | none | record | | |
| `run-manifest/v5` | `contracts/run-manifest.v5.schema.json` | journal | 0021 | none | none | record | | |
| `run-manifest/v6` | `contracts/run-manifest.v6.schema.json` | journal | 0043 | none | none | record | | |
| `run-manifest/v7` | `contracts/run-manifest.v7.schema.json` | journal | 0041 | none | none | record | | |
| `run-manifest/v8` | `contracts/run-manifest.v8.schema.json` | journal | 0041 | none | none | record | | |
| `run-manifest/v9` | `contracts/run-manifest.v9.schema.json` | journal | 0046 | none | none | record | | |
| `run-manifest/v10` | `contracts/run-manifest.v10.schema.json` | journal | 0057, enacted, pending acceptance | none | none | record | | |
| `run-manifest/v11` | `contracts/run-manifest.v11.schema.json` | journal | 0065 | none | none | record | | |
| `run-manifest/v12` | `contracts/run-manifest.v12.schema.json` | journal | 0065 | none | none | record | | |
| `run-manifest/v13` | `contracts/run-manifest.v13.schema.json` | journal | operator ruling 2026-10-10 (#430, PR #621), enacted, pending acceptance | none | none | record | | |
| `run-manifest/v14` | `contracts/run-manifest.v14.schema.json` | journal | 0095 | engine crates/brokkr-runtime/src/engine/start.rs | load brokkr_core::dispatch::bundle_manifest_from_run | current | | |
| `forge-run-manifest/v2` | `contracts/run-manifest.v2.schema.json` | journal | no decision (PR #28), enacted, pending acceptance | Looper | label brokkr_core::dispatch::RUN_MANIFEST_SCHEMA_V2 | current | | |
| `seat-record/v1` | `contracts/seat-record.v1.schema.json` | journal | 0034 | drivers | dispatch brokkr_store::SeatRecordVersion | reader | 0.7 | 0.12 |
| `seat-record/v2` | `contracts/seat-record.v2.schema.json` | journal | 0035 | drivers | dispatch brokkr_store::SeatRecordVersion | reader | never | 0.12 |
| `seat-record/v3` | `contracts/seat-record.v3.schema.json` | journal | 0034 | drivers | dispatch brokkr_store::SeatRecordVersion | reader | 0.8 | 0.12 |
| `seat-record/v4` | `contracts/seat-record.v4.schema.json` | journal | 0046 | drivers | dispatch brokkr_store::SeatRecordVersion | reader | 0.9 | 0.12 |
| `seat-record/v5` | `contracts/seat-record.v5.schema.json` | journal | 0056, enacted, pending acceptance | drivers | dispatch brokkr_store::SeatRecordVersion | reader | 0.11 | 0.14 |
| `seat-record/v6` | `contracts/seat-record.v6.schema.json` | journal | 0065 | drivers | dispatch brokkr_store::SeatRecordVersion | current | | |
| `forge.realms/v1` | `contracts/realms.v1.schema.json` | input | 0023 | operator | set brokkr_core::realms::SCHEMAS | reader | | decision |
| `forge.realms/v2` | `contracts/realms.v2.schema.json` | input | 0026 | operator | set brokkr_core::realms::SCHEMAS | reader | | decision |
| `forge.realms/v3` | `contracts/realms.v3.schema.json` | input | 0041 | operator | set brokkr_core::realms::SCHEMAS | reader | | decision |
| `forge.realms/v4` | `contracts/realms.v4.schema.json` | input | 0046 | operator | set brokkr_core::realms::SCHEMAS | reader | | decision |
| `forge.realms/v5` | `contracts/realms.v5.schema.json` | input | 0057, enacted, pending acceptance | operator | set brokkr_core::realms::SCHEMAS | reader | | decision |
| `forge.realms/v6` | `contracts/realms.v6.schema.json` | input | 0065 | operator | set brokkr_core::realms::SCHEMAS | reader | | decision |
| `forge.realms/v7` | `contracts/realms.v7.schema.json` | input | 0075 | operator | set brokkr_core::realms::SCHEMAS | reader | | decision |
| `forge.realms/v8` | `contracts/realms.v8.schema.json` | input | 0065 | operator | set brokkr_core::realms::SCHEMAS | current | | decision |
| `brokkr.dialect/v1` | `contracts/dialect.v1.schema.json` | journal | 0042 | none | set brokkr_runtime::dialect::SCHEMAS | reader | unmeasured | unmeasured |
| `brokkr.dialect/v2` | `contracts/dialect.v2.schema.json` | journal | 0042 (2026-09-04 addendum) | none | set brokkr_runtime::dialect::SCHEMAS | reader | unmeasured | unmeasured |
| `brokkr.dialect/v3` | `contracts/dialect.v3.schema.json` | journal | 0042 (2026-09-06 addendum) | shipped dialects/openspec.json | set brokkr_runtime::dialect::SCHEMAS | current | | |
| `forge.phase-machine/v1` | `none (decision 0082 ruling 5)` | input | 0002 | none | load brokkr_core::policy::Machine::from_table | reader | | decision |
| `forge.phase-machine/v2` | `contracts/phase-machine.v2.schema.json` | input | 0022 | shipped recipes/fast/policy.json | label brokkr_core::policy::TABLE_SCHEMA_V2 | current | | decision |
| `event-envelope/v1` | `contracts/event-envelope.v1.schema.json` | journal | 0003 | engine crates/brokkr-core/src/envelope.rs | load brokkr_core::envelope::verify_chain | current | | |
| `forge-driver/v1` | `contracts/driver-protocol.v1.schema.json` | input | 0003 | drivers | label brokkr_protocol::PROTO | current | | decision |
| `forge-dispatch/v2` | `contracts/dispatch-envelope.v2.schema.json` | input | no decision (PR #28), enacted, pending acceptance | Looper | label brokkr_core::dispatch::DISPATCH_SCHEMA_V2 | current | | decision |
| `effect-provenance/v1` | `contracts/effect-provenance.v1.schema.json` | journal | 0016 | engine crates/brokkr-runtime/src/engine.rs | load brokkr_core::seat::row::Seats::note | current | | |
| `effect-boundary/v1` | `contracts/effect-boundary.v1.schema.json` | journal | 0046 | engine crates/brokkr-runtime/src/engine.rs | load brokkr_core::seat::boundary_entries | current | | |
| `effect-cleanup/v1` | `contracts/effect-cleanup.v1.schema.json` | journal | 0006 (2026-09-27 addendum), enacted, pending acceptance | engine crates/brokkr-runtime/src/engine.rs | none | current | | |
| `phase-entered-head/v1` | `contracts/phase-entered-head.v1.schema.json` | journal | 0039, enacted, pending acceptance | engine crates/brokkr-runtime/src/engine.rs | load brokkr_core::payload::PhaseEntered::read | current | | |
| `phase-entered-case/v1` | `contracts/phase-entered-case.v1.schema.json` | journal | 0041 | engine crates/brokkr-runtime/src/engine.rs | none | current | | |
| `operator-supersede/v1` | `contracts/operator-supersede.v1.schema.json` | journal | 0047 | operator | load brokkr_core::residual::closings | current | | |
| `brokkr.tool-dialect/v1` | `contracts/tool-dialect.v1.schema.json` | input | 0065 | shipped dialects/tools | schema brokkr_runtime::capabilities::dialect::TOOL_DIALECT_SCHEMA | current | | decision |
| `forge.host/v1` | `contracts/host.v1.schema.json` | input | 0068 (2026-10-10 addendum) | operator | load brokkr_runtime::admission::host::read | current | | decision |
| `panel-carry/v1` | `contracts/panel-carry.v1.schema.json` | journal | 0093 | engine crates/brokkr-runtime/src/engine/dispatch.rs | load brokkr_runtime::engine::carry::facts::terminal_record | current | | |

`fixtures/evaluator/corpus.ndjson` is frozen contract data, never
regenerated, only versioned; it is not a file under `contracts/` and so
has no row here.

The binary reports the versions it was built against:

```console
$ brokkr doctor
…
ok       contracts: engine 0.12.0, event_schema 1, database_schema 1, driver_protocol 1
…
```

`event_schema`, `database_schema` and `driver_protocol` are all `1` and
have been since the beginning. `engine` is the crate version and moves
every release.

## Read-only material that is not a contract

Two directories are read-only but are **not** versioned contracts. They
are frozen reference, and the difference matters: you do not extend
them with a `v2`, you do not extend them at all.

- **`policy/phase-machine.json`** — the heritage transition table the
  evaluator corpus derives from. Its stability *is* the contract: the
  corpus is generated from it, and the differential tests pin the
  evaluator against that corpus.
- **`reference/`** — heritage documents: handoff-protocol lore and
  recorded schemas.

Both are named in the README's contributing rules alongside the v1
contracts and the `fixtures/` corpus: *frozen means frozen.*

## Semver, as of 1.0

From 1.0, semantic versioning applies to two surfaces, read
independently:

**The CLI surface** — `brokkr`'s subcommands, their flags, their
defaults, their exit codes, and the `--json` view models the read
surfaces emit. Removing a subcommand, removing or renaming a flag,
changing a default, or changing the shape of a `--json` payload is a
breaking change. Adding a subcommand, adding an optional flag, or adding
a field to a `--json` payload is not.

The exit codes are part of that surface and are fixed in code as one
enum, `crates/brokkr-cli/src/exit.rs`, which the
[CLI reference](../reference/cli.md#exit-codes) renders as its table.

**The contracts, individually.** Each contract carries its own version
number and moves on its own schedule; none of them is tied to the crate
version. `driver-protocol.v1` has outlived four minor releases of the
engine and will outlive more. A driver built against v1 is not affected
by a `brokkr` major bump, because the protocol version is what it reads.

## What may break before 1.0

Stated plainly, because this is a 0.x tree:

- **CLI flags and defaults may still move.** Realm-aware execution and
  read surfaces have expanded since the original realms release. Use
  each command's `--help` for its current selectors and database flags;
  pre-1.0 development may extend or change those command lines.
- **Bundle-schema additions may land.** New optional seat keys, new
  aggregates, new step forms. These are additive by construction — a
  bundle that compiles today should keep compiling — but a bundle
  written against a not-yet-existing key does not.
- **Semantic changes arrive as numbered decisions.** Every one of them
  is a file under [`docs/decisions/`](../decisions/), written as
  `proposed` and accepted only by the operator, cited by number in the
  code that enforces it. Reading the index is how you see what is
  changing before it changes. A ruling is never edited into a different
  meaning: a new number supersedes it and says so.
- **What does *not* break, even pre-1.0:** the frozen contracts. Every
  decision that has touched them so far — 0016, 0022, 0023 — added a new
  numbered version beside the old one and left the old bytes alone. That
  is the one guarantee that is already load-bearing at 0.12.0, and it is
  the one to build against.

## The closed deprecation window

Decision 0019 renamed the project, and v0.5.0 promised that the
spellings from before the rename would answer for one more release.
Issue #355 closed that window: releases after v0.11.0 read only the
current names.

- **Environment overrides.** `BROKKR_CLAUDE_BIN`, `BROKKR_LANETALLY_BIN`,
  `BROKKR_CODEX_BIN`, `BROKKR_DSH_BIN`, `BROKKR_EXEC_NAME` and
  `BROKKR_BROWSER_BIN` are the only names that configure anything. A
  pre-rename spelling set where its current name is not is refused by
  both names: the seat fails to start, `brokkr doctor` reports the DSH
  selection refused, and `brokkr ui` opens no browser. Nothing runs in
  its place. The prefix is no longer denied as a secret name, since a
  name read only to refuse it cannot aim the harness anywhere.
- **The bundle argv token.** Only `{brokkr}` expands to the engine's own
  executable. A command still written with `{forge}` passes it through
  as a literal argument, and the spawn fails on it.
- **The adapter key.** An adapter declares its destination with
  `egress`. The superseded `binding_grant` boolean is refused at load as
  an unknown key.

The `BROKKR_` prefix stays denied as a bindable secret name: harness
configuration is not a secret binding.

## How a stable thing changes

The procedure, when a contract genuinely has to move:

1. **Write the decision.** A numbered file under `docs/decisions/`,
   status `proposed`. State the problem, the ruling, and its
   consequences. Only the operator accepts it.
2. **Add a new version file beside the old one.** Never edit the old
   bytes. Name it for what it is: `<contract>.v<n+1>.schema.json`.
3. **Say which lineage it continues**, in `contracts/README.md`'s
   tables. `run-manifest.v3` and `.v4` continue the local lineage;
   `.v2` is a different line and stays where it is.
4. **Make the new property optional and absent by default**, so every
   non-adopting run stores and exports the exact previous shape. This is
   what makes the addition provably non-breaking rather than
   argued-to-be.
5. **Make the loader refuse the version mismatch.** A `v2` feature in a
   file calling itself `v1` must be a refusal, not a tolerance —
   otherwise the version string stops meaning anything.

The tell that a change was done right is byte-identity: a run that does
not use the new feature journals and exports exactly what it journaled
and exported before. Every contract extension in this tree so far has a
test asserting that.

## See also

- [`contracts/README.md`](../../contracts/README.md) — the normative
  contract inventory, event vocabulary and fold semantics.
- [`docs/decisions/`](../decisions/) — the numbered rulings, with
  [the index](../decisions/README.md).
- [quickstart.md](quickstart.md) · [recipe-authoring.md](recipe-authoring.md)
  · [driver-authoring.md](driver-authoring.md)
