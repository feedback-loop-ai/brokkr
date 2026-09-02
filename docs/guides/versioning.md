# Versioning and the v1.0 stability promise

This document states what you can build against, what may still move,
and how a change to a frozen thing is made when it has to happen.

**Where the tree is right now:** the workspace version in `Cargo.toml`
is **0.5.0**. This is pre-1.0 software. The contracts under
`contracts/` are nonetheless already frozen and have been since the
first implementation — the freeze is older than the version number and
does not wait for it.

- [Two different promises](#two-different-promises)
- [The frozen-contract law](#the-frozen-contract-law)
- [The current lineages](#the-current-lineages)
- [Read-only material that is not a contract](#read-only-material-that-is-not-a-contract)
- [Semver, as of 1.0](#semver-as-of-10)
- [What may break before 1.0](#what-may-break-before-10)
- [The live deprecation window](#the-live-deprecation-window)
- [How a stable thing changes](#how-a-stable-thing-changes)

## Two different promises

They are worth keeping apart, because one is already in force and the
other is not yet.

| | Frozen contracts | The CLI surface |
|---|---|---|
| **Promise** | A version's bytes never change. A change is a new numbered version file beside the old one. | Semver from 1.0: breaking changes to subcommands and flags need a major bump. |
| **In force** | **Now**, and since the first implementation. Independent of the crate version. | **From 1.0.** Today, at 0.5.0, flags may still move. |
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
   contains `run-manifest.v1`, `.v2`, `.v3` and `.v4` as four separate
   files. None of the earlier three were touched when the later ones
   landed.
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

**The local lineage: `run-manifest.v1` → `v3` → `v4`.**

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

**The Looper-bound lineage: `run-manifest.v2`, unchanged.** Its
round-trip reconstructs a bundle manifest from six named keys and drops
the rest, so an `agents` or `realms` key would be silently dropped and
the run would become unresumable with a diff that blames no file. Rather
than widen a contract a counterpart system reads, the engine **refuses**:
`build_run_manifest_v2` rejects a bundle manifest carrying `agents`, and
`brokkr run` refuses `--dispatch` together with a realms map. Lifting
either needs a jointly agreed v2-lineage manifest version.

**The other contracts, and where they stand:**

| Contract | Files | Status |
|---|---|---|
| Event envelope | `event-envelope.v1.schema.json` | Frozen v1. Extension fields per the amendment above. |
| Driver protocol | `driver-protocol.v1.schema.json` | Frozen v1. This is what an outside driver builds against. |
| Effect provenance | `effect-provenance.v1.schema.json` | The numbered extension schema for `effect/started.provenance` and `effect/failed.start_failure`. |
| Attempt-bound dispatch | `dispatch-envelope.v2.schema.json` | The Looper-bound envelope, embedded whole in `run-manifest.v2`. |
| Phase-machine table | `phase-machine.v2.schema.json` | `v2` = `v1` plus exactly one thing: a rule may rule a park. `v1` tables are read exactly as they always were. |
| The world's map | `realms.v1.schema.json` | `forge.realms/v1`: realms (name, path, default branch) and the world's single `journal`. |
| The world's map, many hearths | `realms.v2.schema.json` | `v2` = `v1` plus exactly one thing: a realm may name its own `journal`, falling back to the world's when it does not. `v1` maps are read exactly as they always were, and the one new word is refused under a `v1` label. |
| Evaluator behavior | `fixtures/evaluator/corpus.ndjson` | Frozen contract data. Never regenerated, only versioned. |

The binary reports the versions it was built against:

```
$ brokkr doctor
ok       contracts: engine 0.5.0, event_schema 1, database_schema 1, driver_protocol 1
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

The exit codes are part of that surface and are already fixed in code
as one shared mapping: **0** completed, **2** parked (operator needed),
**3** stopped, **1** error or still running, **4** contended — a peer
held the shared journal's write lock past this process's whole patience,
so nothing was written and nothing was lost. Its own code because it is
its own thing: 1 says a defect, and a command that met a busy peer has
not found one.

**The contracts, individually.** Each contract carries its own version
number and moves on its own schedule; none of them is tied to the crate
version. `driver-protocol.v1` has outlived four minor releases of the
engine and will outlive more. A driver built against v1 is not affected
by a `brokkr` major bump, because the protocol version is what it reads.

## What may break before 1.0

Stated plainly, because this is a 0.x tree:

- **CLI flags and defaults may still move.** Decision 0023's realms flag
  reached `run` and seven read surfaces in its first phase; `resume`,
  `conclude`, `rerun`, `doctor`, `ui`, `costs`, `compare`, `anchor` and
  `bridge` still take `--db` alone. Closing that gap will change those
  command lines.
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
  is the one guarantee that is already load-bearing at 0.5.0, and it is
  the one to build against.

## The live deprecation window

Decision 0019 renamed the project, and two spellings from before the
rename **still answer, for one more release**. This is a deprecation
window with an end, not a permanent guarantee.

**Environment overrides.** Six harness variables moved to a `BROKKR_`
prefix and answer to their old `FORGE_` spelling:

| Current | Still answers to |
|---|---|
| `BROKKR_CLAUDE_BIN` | `FORGE_CLAUDE_BIN` |
| `BROKKR_LANETALLY_BIN` | `FORGE_LANETALLY_BIN` |
| `BROKKR_CODEX_BIN` | `FORGE_CODEX_BIN` |
| `BROKKR_DSH_BIN` | `FORGE_DSH_BIN` |
| `BROKKR_EXEC_NAME` | `FORGE_EXEC_NAME` |
| `BROKKR_BROWSER_BIN` | `FORGE_BROWSER_BIN` |

**The bundle argv token.** `{forge}` still expands to the same path as
`{brokkr}`.

When an old spelling is what answered, the process writes one line to
**stderr** — never stdout, so piped readouts and every `--json` consumer
read exactly what they read without the fallback:

```
notice: FORGE_CODEX_BIN is now named BROKKR_CODEX_BIN; the old name works for one more release.
```

The latch is **one per process**, not one per spelling: the first old
spelling used in a run prints the notice and later ones are silent. An
operator needs telling, not nagging. If you are relying on either
spelling, migrate now — "one more release" is the stated window, and
the notice is the only warning you get.

One related rule that is *not* deprecating: both the `BROKKR_` and
`FORGE_` prefixes are denied as bindable secret names. Harness
configuration is not a secret binding, under either spelling.

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

## The name

This engine was renamed to **Brokkr** — in the myth, the dwarf whose
whole task was to work the bellows and not stop; Loki, as a biting fly,
made him flinch once, and Mjölnir's handle came out short. Steadiness
under distraction, and the cost of one lapse, is this engine's core loop
told as a story a thousand years old. The old name was also the most
collided word in software, and so was never findable.

**"Forge" survives as the verb.** Slices are forged, runs are forged,
Brokkr forges. The proper noun retired from the marquee, not from the
vocabulary — and the mechanism keeps its plain names, so a new operator
can still guess what a command does with no glossary: `.forge/`,
`forge.db`, `refs/forge/`, the wire protocols.

**The binary is `brokkr`, and now the only one.** The `forge` shim that
rode along for one release is gone, and the crates are `brokkr-*`.
Environment override names carry one-release legacy fallbacks documented
in the [versioning guide](versioning.md), and the `{forge}`
token in bundle argv still answers to `{brokkr}` for the same window.

[Decision 0019](../decisions/0019-brokkr.md) is the ruling, with the
reasoning and the five laws that bound it. [The Edda](../lore/edda.md)
is the lore layer those laws govern: commentary, never specification —
if it burned, the constitution would still be whole.
