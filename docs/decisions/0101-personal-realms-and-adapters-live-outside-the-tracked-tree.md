# 0101 — Personal realms and adapters live outside the tracked tree

Status: proposed
Date: 2026-10-26

Built: unbuilt (#432)

## Context

Issue #432 names the friction: a working checkout carries a personal
multi-realm `realms.json`, and `cargo test -p brokkr-cli --test it` fails
`the_repository_carries_the_bootstrap_map`
(`crates/brokkr-cli/tests/realms.rs:1326`) because that test reads the
working tree's tracked map. The assertion is right. Personal runtime
configuration and the defaults a repository distributes must not share
one file, and CI must hold the distributed file fixed. Locally customised
adapter declarations raise the same separation question.

The first slice is ruled: decision 0083's addendum, the personal recipe
shelf (#697), puts recipes at `$XDG_CONFIG_HOME/brokkr/recipes/`, else
`~/.config/brokkr/recipes/`, beside `host.json`; it composes over a
project's shelf one way only, a name on both shelves refuses, every
layer's origin is pinned, and resume refuses drift
(`docs/decisions/0083-an-adopters-brokkr-lives-in-one-folder.md:935-958`,
`:1088-1096`). The operator's 2026-10-15 ruling on #432 leaves "personal
realms and adapters, and the stock-map test" to this story (`:1141-1144`).
This decision is docs only: it inventories main, states the options,
recommends one of each choice, and gives the slices. No code, test, guide,
`realms.json`, `.gitignore` or tracked sample changes here.

The lines are at the branch point, `bc36e9302`. The code is unchanged
through this decision's head: `git diff bc36e9302..HEAD -- crates/` prints
nothing; only this file and its index row are added.

## What main holds, at HEAD bc36e9302

### The map selector

`--realms` is one field on `JournalArgs`
(`crates/brokkr-cli/src/cli_args.rs:24-33`), flattened by `CostsArgs`,
`LedgerArgs`, `AnchorArgs`, `ResumeArgs`, `RerunArgs`, `CompareArgs`,
`ConcludeArgs`, `OperatorArgs`, `ReplayArgs`, `BridgeArgs` and `UiArgs`
(`:76-114`, `:230-331`, `:436-444`, `:498-520`;
`crates/brokkr-cli/src/cli_args/ui.rs:7-16`). It is **not** flattened by
`TuiArgs`, which declares its own `realms` and `db` (`:121-134`). Other
verbs declare their own fields: `LaunchArgs` for `run` and `queue add`
(`:198-226`), `DoctorArgs` (`:138-156`), `InspectArgs` (`:333-364`),
`TranscriptArgs` (`:366-389`), `SeatsArgs` (`:391-408`), `WatchArgs`
(`:410-434`), `ExportArgs` (`:446-472`), `ImportArgs` (`:474-489`),
`RunsArgs` (`:522-542`), `RealmsArgs` (`:544-557`), `AnalyticsArgs`
(`crates/brokkr-cli/src/verbs/analytics.rs:18-24`), `GraphArgs`
(`crates/brokkr-cli/src/verbs/graph.rs:34-52`) and `MuninnCmd::Run`
(`crates/brokkr-cli/src/lib.rs:255-285`, the field at `:264`). The readout
handlers thread the parsed value as a parameter
(`crates/brokkr-cli/src/verbs/readouts.rs:102`, `:108`, `:463-470`), so
every surface above reaches one `Invocation`. `--db` outranks the map's
journal, not the map.

`compile` (`crates/brokkr-cli/src/cli_args/compile.rs:3-8`) and
`recipes show` (`crates/brokkr-cli/src/lib.rs:374-382`) reach
`print_compiled`, which asks `World::discover(workspace, None)`, so
neither can be given a personal map
(`crates/brokkr-cli/src/verbs/setup.rs:102-118`, `:117`). `recipes
list/add` and `agents list/show` also ask
`World::discover(..., None)` (`crates/brokkr-cli/src/recipes.rs:132`,
`:728`; `crates/brokkr-cli/src/verbs/setup.rs:180`).

### The default and its resolution

The default is the folder's `.brokkr/realms.json`, else `./realms.json`.
`Places::under` builds `dir.join(DEFAULT_MAP_FILE)`
(`crates/brokkr-runtime/src/places.rs:256-264`); `Layout::places` chooses
the folder root or the library workspace (`:340-346`); `default_map` is
the one presence rule (`:527-540`). `World::discover` opens the named map
or asks `default_map` (`crates/brokkr-runtime/src/realms.rs:411-449`);
`World::read` parses it (`:385-408`). A map's relative paths resolve
against the map file's own directory (`World::resolve`, `:513-520`; see
**Paths, once**), never the caller's. `Layout::typed` makes a folder
repository's flag path absolute against the starting directory and leaves
a library's as typed (`crates/brokkr-cli/src/folder.rs:321-326`).
`Invocation::open` states the rules in order: a named map is loaded or the
command refuses; otherwise the workspace's ambient map is adopted and
announced; otherwise no map (`crates/brokkr-cli/src/lib.rs:1202-1255`).
`RunMap` records named, ambient or unmapped
(`crates/brokkr-cli/src/verbs/delivery.rs:87-93`).

### The queue's read

`QueuedLaunch::of` stores `MapSource`
(`crates/brokkr-runtime/src/launch/queued.rs:273-292`); `HeldWorld` reads
it through `World::from_manifest` (`:52-101`); `MapSource::now` re-asks
`default_map` for an ambient or unmapped entry and re-reads a named map's
source on disk (`:201-218`), so a change is named at admission, not
adopted silently. The dispatcher is not built: decision 0095 records it
`unbuilt (#430)`, and `crates/brokkr-runtime/src/dispatch.rs` holds only
the lock and reconciliation, so `held_and_now` (`:382-402`) is the one
read on this path.

### The pin and the resume door

`World::pin_of` writes `source`, `sha256` over canonical JSON, and the
embedded `map` (`crates/brokkr-runtime/src/realms.rs:717-749`);
`World::pinned` inserts it (`:756-767`); a start pins it at the door
(`crates/brokkr-runtime/src/engine/start.rs:70-73`). `World::from_manifest`
requires the three fields, re-derives the digest, re-validates the
content, and refuses a missing field or digest mismatch with
`WorldError::Unpinned` and content that no longer validates with
`WorldError::Map`; it never reads the file again (`:461-510`).
`launch::resume` reads the stored manifest
(`crates/brokkr-runtime/src/launch.rs:350-380`); `compile_from_manifest`
takes the world from the pin (`:602-633`); `Engine::resume` asks the same
pin (`crates/brokkr-runtime/src/engine.rs:594-624`). The bundle comparison
strips `realms`, `crossings` and `queue_entry`
(`crates/brokkr-core/src/dispatch.rs:477-512`). Canonical JSON sorts keys
at every level (`crates/brokkr-core/src/canonical.rs:9-22`).

**The resume door has two inputs.** Before `launch::resume`,
`delivery::resume` asks `args.journal.journal(workspace)`
(`crates/brokkr-cli/src/verbs/delivery.rs:151-156`), which is
`JournalArgs::journal` (`crates/brokkr-cli/src/lib.rs:1364-1368`) calling
`Invocation::inspect` (`:1191-1197`) and so `Invocation::open`
(`:1202-1255`). That open reads the **live** named or default map
(`:1213-1214`) and uses the map's journal unless `--db` names one. It
reads and parses that map even when `--db` names the journal, so a live
map that is missing, unreadable or malformed refuses before a store
opens; `--db` wins the journal, never the map read. A default map whose
journal moved selects that journal. The **world, realm, grants, house,
dialect and bundle** then come from the pin, not that live read. Today
the pin's `source` is never compared with the file on disk, so an edited
map is not refused as source drift; ruling 7 chooses the fence.

### Adapters

Files under `.brokkr/adapters/`, else `adapters/`
(`crates/brokkr-runtime/src/places.rs:256-264`;
`crates/brokkr-cli/src/folder.rs:400-404`). `Adapters::load` reads every
`.json`, maps each abstract model to one provider, and refuses a model
mapped twice (`crates/brokkr-runtime/src/agents/load.rs:644-673`); the
adapter keys are closed (`:707-729`). `run` and `resume` have no adapter
flag: `DeliveryArgs` carries `--bundle`, `--recipe`, `--recipes-dir` and
`--secrets-file` only (`crates/brokkr-cli/src/cli_args.rs:162-179`);
`launch::compile_in_realm` takes `Places.adapters` and passes it to
`Bundle::compile_as` (`crates/brokkr-runtime/src/launch.rs:669-691`),
with `launch::shelved` choosing those places (`:697-701`).
`--adapters-dir` exists only on `probe harness`
(`crates/brokkr-cli/src/cli_args.rs:693-713`;
`crates/brokkr-cli/src/verbs/probe.rs:114-133`), `agents show`
(`crates/brokkr-cli/src/lib.rs:249-252`) and `muninn run` (`:272-275`).
`brokkr driver` selects a built-in `AdapterKind` and loads no adapter file
(`crates/brokkr-cli/src/verbs/setup.rs:282-290`). The roster chooses an
abstract model; the adapter library maps it to a concrete provider model,
and the adapter digest is folded into the manifest's `drivers`
(`crates/brokkr-runtime/src/bundle.rs:4196-4211`, `:4290-4293`). No
personal adapter root exists, and no run selects one.

The three `--adapters-dir` verbs are not interchangeable. `agents show`
is a read-only resolution against the named library. `probe harness` is
an **active measurement**: it resolves the named `--credential` bindings
from the secrets store, launches the adapter's CLI headless against a
scratch repository and HOME, and can write the report and its drift to
`--out` (`crates/brokkr-cli/src/verbs/probe.rs:130-172`; the scratch
world is built in `crates/brokkr-protocol/src/probe.rs:142-148`).
`muninn run` hires a seat and reads the fleet, so it is a run, not a
selector. Pointing `--adapters-dir` at personal bytes therefore measures
or reads that declaration; it does not make it part of a delivered run.

### The machine directory and personal shelf

`config_dir` is `$XDG_CONFIG_HOME/brokkr`, else `$HOME/.config/brokkr`; a
non-absolute value is passed over, and neither present refuses
(`crates/brokkr-runtime/src/admission/host.rs:277-290`); `host_file` is
`host.json` in it (`:267-269`). `personal_shelf` is `recipes` there
(`crates/brokkr-runtime/src/places.rs:133-146`). `--recipe` reads the
project's shelf, then the personal one; a name on both refuses
(`crates/brokkr-cli/src/recipes/personal.rs:214-262`).
`recipes add --personal` compiles against the workspace's agents and
adapters and copies no agent or charter (`:295-320`).

### What the five items already have

1. **Selectors.** Explicit `--realms`, `--bundle` and `--recipe` exist and
   are reused; a personal map needs a documented path (A), a lookup (B),
   or a local file and a walk exception (C). `compile` and `recipes show`
   lack the flag.
2. **Precedence and refusals.** Explicit `--realms` beats the default;
   paths are map-relative; unknown keys and versions refuse through
   `deny_unknown_fields` and validation
   (`crates/brokkr-core/src/realms.rs:354-355`, `:438-447`, `:515-735`).
   No personal candidate or collision rule exists. Adapters have an
   explicit selector only on the three verbs above, never on `run` or
   `resume`.
3. **Resolve once and pin.** Already the world's shape; the resume
   invocation's journal selection is the live read above.
4. **Stock-default and override tests.** The stock test reads the working
   tree; no personal-map selection test exists.
5. **No bundling or credential leak.** The walk skips a top-level
   `realms.json`, `dialects`, capability definitions, and a folder bundle's
   `state` and `recipes` (`crates/brokkr-runtime/src/places.rs:176-203`);
   a nested personal file would be pinned. Adapter trees refuse a
   `secrets.env`, and redacted export scrubs map paths
   (`crates/brokkr-runtime/src/agents/load.rs:87-104`;
   `crates/brokkr-cli/src/verbs/exchange.rs:72-114`). No option adds a
   secret store.

## Paths, once

| Declaration | Resolved against | Code |
|---|---|---|
| `realms[].path` | the map file's directory; absolute stays | `World::resolve` / `realm_root`, `crates/brokkr-runtime/src/realms.rs:513-520`, `:792-798` |
| world `journal` | the map file's directory | `World::journal`, `:522-524` |
| `realms[].journal` | the map file's directory; absent means the world journal | `World::journal_of`, `:535-537`; `crates/brokkr-core/src/realms.rs:738-740` |
| `realms[].house` | the realm's `path` | `load_realm_texts`, `:800-807` |
| `realms[].dialect` named (no `/`, `\`, not `.json`) | `<map dir>/dialects/<name>.json` | `library_path`, `crates/brokkr-runtime/src/dialect.rs:763-768` |
| `realms[].dialect` path (has `/` or `\`, or ends `.json`) | the realm's `path` | `library_path`, `:763-768` |
| `realms[].publishes[].path` | the realm's `path` | `resolve_crossings`, `:852-880` |
| capability definitions and tool dialects | `<map dir>/capabilities/` and `<map dir>/dialects/tools/` | `capability_context`, `crates/brokkr-runtime/src/launch.rs:583-595`; `crates/brokkr-runtime/src/capabilities.rs:71-76`, `:384` |

A `house`, a `dialect` path and a published crossing are **repository-
relative declarations**: absolute paths and `..` are refused
(`crates/brokkr-core/src/realms.rs:478-485`, `:618-630`, `:672-674`). A named dialect
is a library name, not a repository-relative path, and resolves beside
the map.

**The complete migration of the stock world.** Copy `realms.json` to the
personal path; make `realms[].path` the repository root, absolute or
relative to the personal map's directory; make the world (and any realm)
`journal` point at the repository's journal, absolute or relative to the
personal map's directory — the stock `.forge/forge.db` would otherwise
land under the personal directory; keep `house` as the repository-relative
`docs/house-rules.md`; do **not** leave the stock named `dialect`
`openspec`, which would look for `<personal>/dialects/openspec.json`.
Either copy `dialects/openspec.json` and its prompt directory beside the
personal map, or write the path form `dialects/openspec.json`, which
resolves inside the realm and reads the tracked file. A personal map that
declares `capabilities` must likewise carry the definition and
tool-dialect trees beside itself, because the capability root is the
map's directory. Crossings need no move: their published paths are
realm-relative.

A probe of the built binary observed exactly that: with the stock map's
content written to the personal path and the named dialect left as
`openspec`, `brokkr queue add` refused `realm 'brokkr' dialect is
unusable: reading dialect <personal>/dialects/openspec.json: No such file
or directory`; with `"dialect": "dialects/openspec.json"` the same
command queued the entry, and with `"journal": ".forge/forge.db"` the
store landed at `<personal>/.forge/forge.db` until the journal was made
absolute to the repository.

## The options

Judged by the issue's five items: (1) inventory and the minimum selector;
(2) precedence, paths and refusals; (3) resolve once and pin; (4) stock-
default and override tests; (5) no bundling or credential leak.

### A — explicit only (recommended)

The personal map lives outside the worktree, documented at
`$XDG_CONFIG_HOME/brokkr/realms/<name>.json`, else
`$HOME/.config/brokkr/realms/<name>.json`, beside `host.json` and the
personal shelf. Nothing reads it implicitly; `--realms` names it on every
verb that already declares the flag, `ui` and `tui` included. `compile`
and `recipes show` gain it in slice 2; `recipes list/add` and
`agents list/show` stay project-only because they read a shelf or a
library, not the world a run delivers in.

```text
brokkr run --feature "the next slice" --realms <config>/brokkr/realms/personal.json
brokkr realms --realms <config>/brokkr/realms/personal.json
```

- **Precedence.** `--realms` > the workspace default > no map. A named map
  is never combined with the default: the default is not read. If both
  files exist, only the named one is a candidate; no merge, no fallback. A
  missing or malformed named map refuses.
- **Paths.** Against the map's own directory, per the table above. A
  personal map writes a realm path and journal that reach the repository;
  its house is realm-relative; its dialect is a path or is copied beside
  it.
- **Pin.** The written `source`, `sha256` and map, plus the typed
  effective digest of ruling 6.
- **Cost.** The operator types `--realms`, or keeps a shell alias. No new
  storage, `.gitignore` entry or walk rule. Two verbs gain a flag.
- **Refusals.** The same loader as a tracked map: missing, unreadable or
  malformed; unknown key or version; boundary, grant or dialect the
  version does not admit; `--dispatch` beside a map
  (`crates/brokkr-runtime/src/launch.rs:423`).
- **Five items.** (1) reuses the selector, adds a documented path and two
  flags; (2) explicit wins, the default is untouched, paths are
  map-relative and the migration is complete; (3) existing resolve-once
  and pin, with ruling 6's field; (4) the HEAD stock test and
  selection/migration tests from different directories; (5) the map is
  outside the worktree, so the walk never sees it. It does **not** give an
  automatic personal world; that is the price, and the reason B and C
  exist.

### B — a personal map under XDG, found by a rule

The personal map is `$XDG_CONFIG_HOME/brokkr/realms/realms.json`, else
`$HOME/.config/brokkr/realms/realms.json`. The selection matrix is:

| `--realms` | project default | personal map | result |
|---|---|---|---|
| named | any | any | only the named map; neither default is read |
| absent | present | absent | the project default |
| absent | present | present | **refuse**, naming both |
| absent | absent | present | the personal map, announced as ambient |
| absent | absent | absent | no map |

No row merges or falls through. A variant where the personal file silently
outranks the tracked map is rejected: it would let a machine file change
the world a repository declares. This is the one matrix ruling 5 and
slice 6 use, so the personal-only row selects the personal map and the
project-plus-personal row refuses.

- **Precedence.** Explicit > project default > personal default > none;
  project plus personal refuses, never a merge and never a winner.
- **Paths.** Against the personal map's own directory, so A's complete
  migration and journal warning apply.
- **Pin.** The same `source`, written `sha256`, map and typed effective
  digest.
- **Cost.** A new lookup in `places.rs` and `realms.rs`, a new collision
  refusal and exact-text test, docs and the matrix. It makes a
  machine-dependent file a candidate in repositories with no map, so the
  adoption must be announced. In the stock repository the tracked map is
  present, so B **refuses** and still needs `--realms`.
- **Refusals.** The collision, a missing or malformed personal file, more
  than one candidate, and every schema, boundary, grant and dialect
  refusal the tracker gives, before a store opens or a seat spawns.
- **Five items.** (1) adds one path and lookup; (2) states a precedence
  and refuses a collision; (3) preserves the pin; (4) needs collision and
  fallback tests; (5) outside the tree, but the fallback is the
  machine-dependent default the issue warns about. It does **not** meet
  the stock-files-unchanged acceptance: the tracked map collides.

### C — an ignored local file in the repository

The map is `.brokkr/realms.local.json`, named by `.gitignore` (or
`.brokkr/.gitignore`). The rule is `--realms` > the local file >
`.brokkr/realms.json` (else `./realms.json`): the local file is an
override, not a layer, and the tracked map is not read when it stands. The
safer variant **refuses when both exist**; that variant does not meet the
stock-files-unchanged acceptance either, because the stock map is always
there. The override variant meets it but is an implicit local world.

- **Precedence.** Explicit wins; below it, a refusal or a local override,
  never a merge.
- **Paths.** Against `.brokkr/`, so a realm at the repository root is `..`;
  the table above still applies.
- **Pin.** The same `source`, written `sha256`, map and typed effective
  digest; the source is the local path.
- **Cost.** A `.gitignore` entry, a `default_map` change, and a walk
  exception: `unpinned_top_level` does not know `realms.local.json`, so
  the file inside the folder bundle would be pinned unless the walk learns
  the name. It is lost by `git clean -fdx`, is one copy per checkout, and
  is the option most likely to bundle personal bytes.
- **Refusals.** Schema, boundary, grant, dialect and dispatch are the same,
  but local-versus-tracked is an override rule rather than the requested
  collision rule, so the failure mode is easier to get wrong.
- **Five items.** (1) adds a file and a walk exception; (2) needs an
  override or refusal rule; (3) preserves the pin; (4) needs
  local-versus-tracked tests; (5) the file is visible to the walk. Only the
  override variant meets the stock-file acceptance, at item 5's cost.

## Adapters

**Recommendation: no personal adapter place in this build.** The adapter
library carries `trust_tier`, `egress`, `routes`, credential names,
`tool_permissions`, `native_capabilities` and hands data
(`crates/brokkr-runtime/src/agents/load.rs:630-729`). A personal file that
shadows a tracked adapter by name would silently replace those decisions;
one that adds a model would extend the provider set without a trust
ruling. That is a new authority grant, not a path-selection problem, and
belongs in a separate decision with its own threat model.

The existing selections suffice for #432: a run reads the workspace's
adapters through `Places.adapters` and pins each digest in `drivers`, the
tracked, reviewable route; `agents show` reads a named library; `probe
harness` measures one with the named credentials; `muninn run` reads the
fleet with a named library. The roster's abstract model names are already
one-to-one, which is the collision rule a personal layer would reuse. A
map carries no `trust_tier` or `egress`, so a personal map cannot widen
them.

If the operator rules that personal adapters are wanted, the shape is the
map's: `$XDG_CONFIG_HOME/brokkr/adapters/`, selected explicitly (or a
personal overlay with a collision refusal); one file name on both roots
refuses; one model mapped by both roots refuses; and no personal file is
admitted if it lowers `trust_tier`, widens `egress`, adds
`native_capabilities` or changes a boundary. That is a security decision,
not built here.

## The stock-map test

**Recommendation: read and validate the committed blob.** Replace the
working-tree read in `the_repository_carries_the_bootstrap_map`
(`crates/brokkr-cli/tests/realms.rs:1326-1347`) with
`git -C <root> show HEAD:realms.json`, assert that JSON, and read the map
facts it states from `RealmMap::parse` (`journal` is `.forge/forge.db`,
the one realm is `brokkr` at `.` with `main`, the house and the dialect
words). Do not call `World::load(&root.join("realms.json"))` at `:1344`,
which reads the working tree, and do not write a new file at the
repository root: a parallel bundle walk would pin it. The house and
dialect files are proved by the compile tests; a test that needs a
`World` loads the HEAD bytes from an isolated temporary directory whose
`docs/` and `dialects/` link the repository's, never from the tracked
path. Then no assertion depends on a developer's working map, and a
tracked edit must be committed to pass.

Git is already a test dependency: the tracked-files helper runs
`git ls-files` (`crates/brokkr-cli/tests/support/tracked.rs:9-23`). Add a
helper returning `git show HEAD:<path>` and a test in a scratch git
repository that commits one map, edits the working tree to another, and
asserts the helper returns the committed one. That test binds the HEAD
read: a helper that reads the working tree fails it.

Under A there is no personal lookup, so the stock test needs no guard.
Under B, add the matrix's guard: with a personal map planted and no
project map, `World::discover` selects the personal map and pins its
source; with both, it refuses and names both. Under C, add that
`realms.local.json` is not pinned by the walk. The mutation that proves
the stock test binds is a committed edit in a scratch worktree: change one
realm word, commit, run the test, watch it fail, restore. A working-tree
edit does not make it fail, which is the point.

## Identity and resume

### Identity

Today's identity is the `sha256` over the map's canonical JSON **as
written** (`crates/brokkr-runtime/src/realms.rs:400`, `:717-749`). It is
key-order and whitespace insensitive, but a field omitted and the same
field written with its default are different written maps and pin
differently. The stock map bumped to `forge.realms/v4`, with `boundary`
omitted and with `boundary: "namespace"`, hashes as written to
`3f01cf4a23b8cca6ec95e69f0302e9e7ccf1ec336295d4716165619de4d6dca2` and
`f382ea715e49b254c3ef29fa5b0ae79446fa65b39be979d02b31b374f92856a5`,
while both resolve `Boundary::Namespace` (`crates/brokkr-core/src/realms.rs:419-421`).
Written equality is therefore **not** effective equivalence; the
acceptance's "two equivalent effective configurations produce the same
identity" is met only by the typed effective digest below.

**Recommendation: keep the written digest and add the typed effective
one.** `sha256` stays the integrity and origin evidence of the map's
canonical written JSON: key order and whitespace never move it, while
`schema`, every written field and each number's spelling (`-0.0`
included) do. It is not a digest of the file's bytes. A new
`effective_sha256` is the digest of one canonical **typed** effective map
produced by a pure core normalizer from the **validated** `RealmMap`
(never raw `Value`). Slice 3 adds serializable `EffectiveMap`,
`EffectiveRealm`, `EffectiveGrant` and crossing entries in a new
`crates/brokkr-core/src/realms/effective.rs`, re-exported from
`crates/brokkr-core/src/realms.rs`; the parent is 751 lines and ruling 4
caps it at 800, so the types do not land in it (the existing types derive
only `Deserialize`: `:353-354`, `:437-439`, `:264-266`, `:277-278`; the
runtime's read and pin boundary is
`crates/brokkr-runtime/src/realms.rs:394-400`, `:717-722`);
`GrantRetention` gains `Serialize` for its canonical word (`unreserved`,
`inherit`, `veto`). `EffectiveMap`
serializes to `Value` and then goes through `effective::canonical_value`,
which rewrites every negative-zero `f64` to `0.0` recursively before the
existing `canonical::sha256_hex`
(`crates/brokkr-core/src/canonical.rs:17-21`) hashes it. That rewrite is
the whole numeric rule: `serde_json::Value` equality reads `-0.0` and
`0.0` as one value while `serde_json` writes their bytes differently, so
a raw `Value` hash would refuse a typed-equal rewrite. Integers keep
their spelling; `-0.0` has exactly one replacement, and the written
digest is not touched. The type is built only from a map that already
passed `RealmMap::parse`/`of`, so every refusal runs before an identity
exists.

The value carries `schema`, the world `journal`, `provisional_offices`
and each realm's loader-read fields: `name`, `path`, `default_branch`,
`journal` through `journal_of` (`:738-740`), `house`, `dialect`,
`boundary` through `Realm::boundary` (`:419-421`), and
`published()`/`consumed()` (`:426-432`). Each capability is its typed
`CapabilityGrant` (`grants.rs:55-66`); `tools`/`offices` stay `Option`,
so absence and `[]` remain opposite (`:50-54`), and the normalizer never
uses `CapabilityGrant::value()`, which rebuilds the written grant and
makes a v7 restriction and a v8 veto the same JSON (`:71-83`). The label
is the semantic discriminator (`GrantRetention`, `:25-37`), and the
typed grant records what it made the loader read (`parse_grant`,
`:232-249`).

Defaults materialize only where the loader reads absence and its explicit
spelling alike: no/`null` realm journal to the world journal (`:738-740`);
no/`null` boundary to `namespace` (`:419-421`); no/empty
`publishes`/`consumes` to none (`CrossingList::entries`, `:330-337`;
written `null` refused at `:641-658`); no/`{}` `capabilities` to no grants
(`:401-405`; `parse_grants`, `grants.rs:175-192`; `null` refused at
`:180-185`); no/`[]` `provisional_offices` to none
(`provisional.rs:27-46`; `null` refused at `:34-36`); `null`
`house`/`dialect` to absence (`:368-374`, gated only when `Some` at
`:618-631`). Realm order, names, paths, branches, the world journal and
every remaining declaration stay as written; no path, `~` or disk is
touched.

Two maps are effectively equal exactly when their `EffectiveMap` values
are equal. The v7/v8 pair in slice 3 now differs: two maps identical but
for `forge.realms/v7` against `v8`, each granting
`{"web-search":{"dialect":"search-native","retain":false}}`, parse the
same written `retain:false` as `Unreserved` with a `retain` restriction
under v7 and as `Veto` without it under v8 (`grants.rs:186-189`,
`:232-248`). An omitted default and its explicit spelling share
`effective_sha256`, a world change moves both, a key/order or whitespace
change moves neither, and a schema change that changes a grant's reading
moves it even when the rest of the written bytes match.

A probe of a serialized mirror of this value observed the false refusal a
bare `canonical::sha256_hex` would make: maps whose only difference was
`"cost": 0.0` against `"cost": -0.0` in a grant restriction parsed equal
as `RealmMap` and as the mirrored `EffectiveMap`, while the serialized
values' bytes were `0.0` and `-0.0` and their digests differed. The
`effective::canonical_value` rewrite is what makes the pair share one
identity; the written digests stay apart.

**What the effective identity does not measure.** It resolves no path,
so `repo` and `./repo` differ even when they name one directory, and a
moved repository is a map change. It reads no file: house, dialect,
crossing and capability bytes are pinned elsewhere (dialect at
`crates/brokkr-runtime/src/realms.rs:725-745`, crossings at `:695-708`,
adapters in `drivers` at
`crates/brokkr-runtime/src/bundle.rs:4196-4211`). It does not judge a
dialect's restriction schema
(`crates/brokkr-runtime/src/capabilities/dialect.rs:403-420`). The
schema label is carried, so a later label's changed reading cannot
collide before its typed field lands. Actual resume execution, the pin
field, `effective::canonical_value` and the `SourceDrift` text are
unmeasured because none is built: slices 3 and 4 carry the tests.

The pin writes both, and `from_manifest` re-derives both from the
embedded map; a pin without `effective_sha256` is an older pin and keeps
the written digest as its identity while the fence of ruling 7 derives
the typed effective digest it compares. This adds a field to the `realms`
pin, so it lands as a new `contracts/run-manifest.v15.schema.json` beside
v14, never as an edit, and only after this decision is accepted: decision
0082 mints a contract version under an accepted ruling (`contracts/` is
frozen; the `realms` property is opaque in the schema, so v15's
description carries the field). The alternative is to keep the written
digest alone: simplest, but it does not meet the effective-equivalence
acceptance, and this decision says so rather than claiming it. Replacing
`sha256` with an effective digest is not recommended: it loses the
check on the canonical written JSON and moves every existing pin. The
effective-only alternative also drops the one digest that commits to the
map as written, `schema` included, so this decision keeps both.

### The resume door

1. `delivery::resume` resolves the journal through `Invocation::inspect`,
   which reads the live named or default map
   (`crates/brokkr-cli/src/verbs/delivery.rs:151-156`;
   `crates/brokkr-cli/src/lib.rs:1202-1255`). A named map that is absent,
   unreadable or malformed refuses here; a default map that is unreadable
   or malformed refuses here too; a default map's journal may move the
   selected journal; `--db` outranks the journal, never the map read.
2. The run id is resolved in that journal; a journal with no matching run
   refuses `no run matching '<id>' in this workspace database`
   (`crates/brokkr-cli/src/selector.rs:109`).
3. `launch::resume` reads the manifest and `World::from_manifest` takes
   the world, realm, house, dialect, grants and crossings from the pin
   (`crates/brokkr-runtime/src/launch.rs:350-380`;
   `crates/brokkr-runtime/src/realms.rs:461-510`).
4. `compile_from_manifest` compiles the bundle from the pin
   (`crates/brokkr-runtime/src/launch.rs:602-633`); `Engine::resume`
   compares the pinned bundle with `realms` stripped
   (`crates/brokkr-runtime/src/engine.rs:594-624`;
   `crates/brokkr-core/src/dispatch.rs:477-512`).
5. `verify_crossings` re-reads the crossing files at the pinned source's
   directory (`crates/brokkr-runtime/src/realms.rs:671-675`;
   `crates/brokkr-runtime/src/launch.rs:489-495`), but not the map.

**Recommendation: a same-source effective fence.** Add
`World::verify_source(workspace)`, called in `resume_with` beside
`fence_crossings` (`crates/brokkr-runtime/src/launch.rs:370-377`), after
`compile_from_manifest` has read the pin (`:366`) and before `resume_with`
returns the engine `delivery::resume` drives, so no seat can spawn;
`operator_retry_members` reaches the same call through `resume_with`
(`:397`). It joins the pinned `source` as `verify_crossings` does
(`workspace.join(source)`; an absolute source stays), and:

| the pinned source on disk | result |
|---|---|
| absent | proceed under the pin; a deleted map is not drift |
| present, unreadable or malformed | refuse with the loader's error; local mistakes get the tracked refusals |
| present and effective-equal (the same `EffectiveMap`: key order, whitespace, an explicit default) | proceed |
| present with a different effective digest | refuse with a named `WorldError::SourceDrift`: the map at `<source>` has changed, the run pins `<effective>`, the file now hashes to `<effective>`; resume reads the run's pin, never the file |

The comparison is between the pinned map's typed effective form — its
`effective_sha256` when the pin carries one, else the typed digest derived
from the embedded map — and the live file's. An older pin is fenced on the
same terms; a written-only change of key order or an omitted default
cannot refuse; a rewrite that changes a typed grant, such as the v7/v8
`retain:false` pair, moves the digest and refuses even though the written
JSON matches.

A redirected `--realms` names another map for the **journal** only: it
never replaces the pinned world; if its journal does not hold the run, the
selector refuses; if the pinned source is itself changed, the fence
refuses. A run that started unmapped has no source to check and stays
unmapped if a map appears. A queued entry already re-reads its named map
at admission through `MapSource::now` and names the change
(`crates/brokkr-runtime/src/launch/queued.rs:201-218`, `:382-402`); the
fence is for resume.

The alternative is today's pin-only resume: the live map still selects the
journal, the pinned world is used, and a changed source is not refused.
That is smaller, but it leaves the invocation's live read as the only
door a changed local map can walk through, and 0083's personal recipe
shelf refuses drift
(`docs/decisions/0083-an-adopters-brokkr-lives-in-one-folder.md:1088-1096`).
The fence is a deliberate departure from today, named for the operator: it
makes a changed personal map a refusal rather than an ignored pin input,
and it means a run cannot resume after its source map is edited until the
original effective configuration is restored. A third option, refusing
whenever the invocation's live map is not the pinned source, was rejected
as too strict for a moved map and not needed for the world, which already
comes from the pin.

## What leaks

- A personal map lives outside the worktree, so the bundle walk never
  sees it. The walk skips a top-level `realms.json`, `dialects`,
  capability definitions, and a folder bundle's `state` and `recipes`
  (`crates/brokkr-runtime/src/places.rs:176-203`); a personal file placed
  inside a bundle would be pinned. The rule is "outside the repository",
  not "untracked".
- The run manifest carries the pin's `source`, embedded map and effective
  digest as evidence. Redacted export scrubs absolute paths and usernames
  through the journal's path table
  (`crates/brokkr-cli/src/verbs/exchange.rs:72-114`); the verbatim export
  still carries them by design, so published examples use placeholders.
- No map field stores a secret value. Credentials are names under
  decision 0012, and adapter and agent trees already refuse a
  `secrets.env` (`crates/brokkr-runtime/src/agents/load.rs:87-104`). This
  decision adds no secret store.
- Docs show `<config>` or `<repo>` placeholders, not a real home path;
  tests use temporary directories; `fixtures/` is not touched.

## Rulings

1. **A personal map is selected explicitly, never by a lookup.
   Recommended: A.** It lives outside the worktree at
   `$XDG_CONFIG_HOME/brokkr/realms/<name>.json`, else
   `$HOME/.config/brokkr/realms/<name>.json`; `--realms` names it on every
   verb that takes the flag, `ui` and `tui` included. `compile` and
   `recipes show` gain the flag; `recipes list/add` and `agents list/show`
   stay project-only. B and C are the operator's alternatives; neither is
   built under A.
2. **Explicit wins; one candidate, and collisions refuse. Recommended:
   A's rule.** `--realms` > workspace default > no map. A named map and a
   default map are two candidates, and only the named one is read; never
   merged. B refuses project-plus-personal; C refuses local-plus-tracked
   in its safer form. A missing or malformed named map refuses; no
   fallback.
3. **Paths keep today's bases; an external map carries the complete
   migration. Recommended: the table above for every option.** Realm path
   and world/realm journal are map-directory relative; house, a path
   dialect and a published crossing are realm-relative; a named dialect is
   `<map dir>/dialects/<name>.json`; capability definitions are
   `<map dir>/capabilities/` and tool dialects `<map dir>/dialects/tools/`.
   A personal world names the repository's
   journal and uses a path dialect or copies its dialect beside it. Tests
   name one personal map from two working directories and compile a
   migrated stock world.
4. **A run's adapters are the workspace's tracked adapters. Recommended:
   no personal adapter root in this build.** `run` and `resume` keep
   `Places.adapters`; `--adapters-dir` stays on `probe harness`, `agents
   show` and `muninn run`, with the first an active measurement; the
   one-model-one-provider rule and `drivers` pins stand. A personal
   adapter root is a separate decision with a collision refusal and a
   no-widening rule.
5. **The stock-map test reads `HEAD`. Recommended: the HEAD read.** The
   test asserts `git show HEAD:realms.json`, validates those bytes through
   `RealmMap::parse`, and reads the map facts from that parse; it never
   reads the working-tree map and writes no new file at the repository
   root. A helper test commits one map, edits the working tree, and proves
   the helper returns the committed one. Under B, add the matrix's
   personal-only and collision guards; under C, add the walk guard. A
   committed change to the tracked map still fails.
6. **Identity is the written digest plus the typed effective digest.
   Recommended: both.** `sha256` stays the origin check over the map's
   canonical written JSON (key order and whitespace aside) and commits to
   its written `schema`; `effective_sha256` is the digest
   of the validated map's typed effective form (`schema`, the world
   journal, `provisional_offices`, and each realm's loader-read fields,
   with each grant's `GrantRetention` class, tools, offices and parsed
   restrictions rather than written JSON). Two maps whose loaders read
   them differently cannot share it: a v7 and a v8 map that both write
   `retain:false` parse as Unreserved-plus-restriction and Veto and
   differ. Its bytes are canonical over the typed value, with each
   negative-zero `f64` written `0.0`, because typed equality reads `-0.0`
   and `0.0` alike; the written digest keeps their spelling, as the
   canonical written JSON does. Defaults materialize only where the loader
   reads absence and the explicit spelling alike, each cited above. It
   lands as a new `run-manifest.v15` contract beside v14, after this
   decision is accepted (0082); older pins without it keep the written
   digest, and their fence derives the effective one. The written-only
   alternative is smaller and leaves the effective-equivalence acceptance
   explicitly unmet; effective-only loses the source check.
7. **Resume is pinned, and a changed source at the same path refuses.
   Recommended: the same-source typed fence.** The live map selects only
   the journal; the world, realm, grants and bundle come from the pin.
   The fence is called in `resume_with` beside `fence_crossings`, after
   the pin is read and before any seat spawns. An absent pinned source
   proceeds under the pin; an unreadable or malformed one refuses with the
   loader's words; one whose typed `EffectiveMap` differs refuses with
   `WorldError::SourceDrift`, for an older pin as for a new one. A
   key-order or whitespace rewrite, a default written explicitly and a
   signed-zero rewrite (`0.0` against `-0.0` in a restriction) resume
   because the typed values are equal and the canonical effective bytes
   are one; a rewrite that changes a grant's reading — the v7/v8 pair —
   refuses even though the written JSON matches. A redirected `--realms`
   never replaces the world; a moved journal shows as the run not being
   in it; an unmapped run stays unmapped. Today's pin-only resume is the
   named alternative.
8. **Personal paths and credentials stay out of distributed examples.
   Recommended: map outside the bundle; credentials as names.** No map
   field carries a secret value; no new secret store; redacted export
   scrubs the pin's paths; fixtures and examples use placeholders. The
   walk is not widened to skip a nested personal map: the rule is "outside
   the repository", not "in an ignored file".

## Consequences

A is the smallest change that separates personal realms from the
distributed default, and the only option that lets a personal multi-realm
setup run with the stock tracked files unchanged without an implicit
lookup: the operator names the map. Its price is typing `--realms`, with a
shell alias as the workaround. B would give a fallback but refuses beside
the stock map, so it does not meet the stock-files acceptance; C would put
personal bytes inside the walk, or refuse beside the stock map in its
safer form.

A is consistent with #697's origin-pinned, drift-refusing shelf, and
differs from it for a stated reason: a recipe is a named layer with a
one-way composition over a project shelf, while a map is a whole world; an
automatic personal world would be an implicit world selection, which #432
forbids. The source fence of ruling 7 is the part that matches #697's
drift refusal. The typed effective digest of ruling 6 is what makes the
acceptance's "equivalent effective configurations" true; if the operator
declines it, that item is not met and the written digest stands as it does
today.

The PR asks the operator to accept all, or pick per ruling 1 to 8. B and C
are priced above; they are not built unless chosen.

## The slices

Smallest first; each is a separate pull request, and every new test is
shown to bind by decision 0071 ruling 9's removal control.

Two touched files are already near or over ruling 4's 800-line ceiling:
`crates/brokkr-core/src/realms.rs` is 751 lines and
`crates/brokkr-runtime/src/realms.rs` is 1,060 (`wc -l`;
`quality/file-lines.txt`). The core parent may grow by 49 lines; the
runtime parent, already over, may not grow. Slice 3 puts the effective
types in a new `realms/effective.rs`; slices 3 and 4 put the runtime pin
and fence additions in new `realms/` submodules and re-export them, so
neither parent grows.

1. **Committed-default test and the personal-map words.** (Small: about 60
   lines of docs, 50 of test.) Files:
   `crates/brokkr-cli/tests/realms.rs`,
   `crates/brokkr-cli/tests/support/tracked.rs`,
   `docs/guides/quickstart.md`, `docs/guides/repository-layout.md`. Add a
   `HEAD` helper, assert and validate the stock map from HEAD bytes through
   `RealmMap::parse` instead of the working tree, and test the helper
   against a scratch git repository with an edited working tree; document
   the XDG path, `--realms` and the migration.
2. **Explicit selection for the compiling verbs.** (Small: about 40 lines
   of code, 220 of tests.) Files: `crates/brokkr-cli/src/lib.rs`
   (`RecipesCmd::Show`), `crates/brokkr-cli/src/cli_args/compile.rs`,
   `crates/brokkr-cli/src/verbs/setup.rs`,
   `crates/brokkr-cli/tests/realms.rs`,
   `crates/brokkr-runtime/src/realms/tests.rs`. Add `--realms` to
   `compile` and `recipes show`; tests: the same personal map named from
   two working directories yields the same map and paths (the flag is typed
   as `Invocation::open` types it, `Layout::typed`); the stock world
   migrated to an external map compiles (absolute realm and journal, path
   dialect); the named-dialect refusal; malformed, unknown-key,
   wrong-version and boundary refusals use the tracked loader's variants;
   `--dispatch` beside a map refuses.
3. **Effective identity.** (Medium: about 120 lines of code, 200 of tests,
   one new contract file.) Files:
   `crates/brokkr-core/src/realms/effective.rs` (new; the 751-line parent
   cannot hold the types under ruling 4), `crates/brokkr-core/src/realms.rs`
   (the re-export only), `crates/brokkr-core/src/realms/tests.rs`,
   `crates/brokkr-runtime/src/realms/pin.rs` (new; the pin write and read
   live here), `crates/brokkr-runtime/src/realms.rs` (the re-export only),
   `crates/brokkr-runtime/src/realms/tests.rs`,
   `contracts/run-manifest.v15.schema.json` (new, beside v14),
   `crates/brokkr-runtime/tests/frozen_contracts.rs` and
   `crates/brokkr-runtime/tests/witness_digests.rs` if the pin moves a
   witness. Tests: a `boundary` omitted against `boundary: "namespace"`
   under a current label, whose default is namespace, shares
   `effective_sha256` (`Realm::boundary`,
   `crates/brokkr-core/src/realms.rs:419-421`); the same pair under a
   label whose default is a different boundary gives two, and that
   label's test lands with it; a key-order-only rewrite of one map gives
   one identity; a restriction `cost: 0.0` against `cost: -0.0` gives one
   identity because the typed values are equal and the normalizer writes
   one zero, while the written digests stay apart; the v7/v8 pair above
   gives two identities and their `EffectiveGrant` values differ
   (`Unreserved` plus a `retain` restriction against `Veto` without it),
   so a normalizer that reused `CapabilityGrant::value()` fails the field
   assertion; a world change moves both; a written-null `publishes` still
   refuses rather than normalizing; a pin without the field keeps the
   written digest and its fence derives the effective one; a tampered
   effective field refuses. The v15 contract file lands only after 0101 is
   accepted (0082).
4. **The resume source fence.** (Medium: about 80 lines of code, 220 of
   tests.) Files: `crates/brokkr-runtime/src/realms/source.rs` (new; the
   1,060-line parent may not grow, so `WorldError` moves beside the
   fence), `crates/brokkr-runtime/src/realms.rs` (the re-export only),
   `crates/brokkr-runtime/src/realms/tests.rs`,
   `crates/brokkr-runtime/src/launch.rs`,
   `crates/brokkr-runtime/src/launch/tests.rs`,
   `crates/brokkr-cli/tests/realms.rs`. Add `World::verify_source` and
   `WorldError::SourceDrift`, call it in `resume_with` beside
   `fence_crossings`, and pin its exact text. Tests: a changed map
   refuses; the v7/v8 pair above refuses with `WorldError::SourceDrift`;
   a key-order-only rewrite resumes; a signed-zero rewrite (`0.0` against
   `-0.0` in a restriction) resumes because both sides hash one canonical
   typed value; a boundary omitted against `boundary: "namespace"` under
   a current namespace-default label resumes, and the same pair under a
   label whose default is a different boundary refuses; an absent map
   resumes under the pin; a malformed pinned map refuses; an
   effective-equal rewrite resumes; an older pin with no effective field
   is fenced on its derived digest; a moved journal is not found; a
   malformed live default refuses before the store opens; a redirected map
   does not replace the pinned world; an unmapped run stays unmapped; the
   `--realms` missing case still refuses before the journal opens.
5. **The adapter note.** (Extra small: about 30 lines of docs, one test.)
   Files: `docs/guides/provider-adapters.md` and the CLI argument tests.
   Document that a run's adapters are the workspace's, that `probe
   harness` measures and writes, and that `--adapters-dir` is only on
   three verbs; pin that `run` and `resume` reject it with clap's
   unknown-argument refusal.
6. **Only if the operator chooses B.** (Medium: about 200 lines of code,
   250 of tests.) Files: `crates/brokkr-runtime/src/places.rs`,
   `crates/brokkr-runtime/src/realms.rs`, `crates/brokkr-cli/src/lib.rs`,
   `crates/brokkr-cli/tests/realms.rs`,
   `crates/brokkr-runtime/src/places/places_tests.rs`. Add the personal
   candidate and the collision matrix above, with exact-text collision,
   personal-only selection and no-project-map fallback tests.
7. **Only if the operator chooses C.** (Medium: about 150 lines of code,
   200 of tests.) Files: `crates/brokkr-runtime/src/places.rs`,
   `crates/brokkr-runtime/src/bundle.rs`,
   `crates/brokkr-cli/tests/realms.rs`, `.gitignore`, and the bundle-walk
   tests. Add the local name to the walk's skip rule and the override or
   refusal rule, with a test that the local file is not pinned.
