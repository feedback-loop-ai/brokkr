# Change Proposal: composable-recipes

**Spec**: [../../../specs/composable-recipes/spec.md](../../../specs/composable-recipes/spec.md)
**Plan**: [../../../specs/composable-recipes/plan.md](../../../specs/composable-recipes/plan.md)
**Tasks**: [../../../specs/composable-recipes/tasks.md](../../../specs/composable-recipes/tasks.md)

## Why

Decision 0010 made a recipe a named, installable, comparable whole
bundle. It did not make one **composable**. Wanting the SDD design
council with a different review panel today means copying
`recipes/sdd/` — 227 lines of `bundle.json`, 185 of `policy.json`, plus
`roles/` and `drivers/` trees — and editing two files. From the moment
of the copy the two drift, silently, and nothing records that one came
from the other.

Decision 0017 (`docs/decisions/0017-composable-recipes.md`, accepted
2026-08-29) rules the alternative: **a recipe may extend another recipe
and state only its differences**, resolved at compile time into one flat
bundle the engine never knows was composed. This change implements that
ruling in full. It writes no new decision doc.

## What Changes

- **Composition is a pure pre-pass, in one new module.**
  `crates/brokkr-runtime/src/bundle/compose.rs` takes a leaf bundle
  directory and returns the flat `bundle.json` document, the flat policy
  table, a name-level origin map, the ordered ancestor chain with
  digests, and the layer directories. `Bundle::compile` consumes that
  and parses **exactly as it does today**; no existing lint, parse
  function, or error message moves. Resolution stays *above* parsing for
  two hard reasons: `parse_command` expands `{brokkr}` to
  `std::env::current_exe()`, so a resolver returning parsed seats could
  never be byte-stable across machines; and the required 0016-layering
  test — an opaque seat surviving inheritance unchanged — is vacuous
  against a parsed `Seat`, which discards unknown keys.

- **Seats stay opaque by construction, not by care.** The `override` and
  `remove` markers are resolver-owned **top-level, kind-keyed** objects
  (`override.seats`, `override.rules`, `override.table`,
  `override.bundle`; `remove.seats`, `remove.rules`, `remove.phases`),
  never a key inside a value. An in-value marker would force the
  resolver to open and rewrite a seat to strip it — the exact operation
  the parallel decision-0016 layering forbids. A seat that references an
  agent therefore survives composition byte-untouched, and this slice
  adds no `agent:` handling of any kind.

- **Merge rules, legibility over cleverness.** Named things merge by
  name; values are replaced wholesale, never field-merged. Adding is
  free; redefining requires the marker and otherwise fails naming both
  files and the key; removal is explicit and fails on a missing target;
  a stale marker is itself an error. `name` is required in every layer
  and must differ from every ancestor's — inheriting it would write the
  base's name into `runs.bundle_name` and the dispatch `recipe` pin
  under a different digest.

- **Policy composes by the engine's existing first-match-wins order**,
  derived rules first. Overriding a rule by `id` is **remove-then-
  prepend**: naive prepending would leave the base rule unreachable and
  `Machine::from_table` (`policy.rs:118`) would reject the whole table,
  making the feature's headline use case structurally impossible with an
  error mentioning neither composition nor a file. The constitutional
  lint runs on the **resolved** table with zero new lint code — that is
  a consequence of resolving first, not an addition.

- **Inherited paths resolve against the layer that supplied them.** The
  resolver records origin **by name** and never learns that a seat has a
  `role`, `driver`, `panel` or `sequence`; `Bundle::compile` passes that
  layer's directory to the existing `dir` parameter of `parse_role` and
  `parse_command`. `Bundle` gains `roots` and `confined_command`
  (`engine.rs:1618`, which mounts exactly one directory today) mounts
  every root — otherwise a confined inherited seat compiles cleanly,
  runs for hours, and then cannot see its own role file.

- **The composition chain rides inside the digested manifest's `files`
  map** as `@compose/{NNNN}/{recipe}` → that ancestor's own digest, with
  **no new contract version**. This is the sharpest call in the slice
  and the evidence decides it: `run-manifest.v1`/`v2` are
  `additionalProperties: false`, so a **top-level** `composition` member
  is illegal — but `files` constrains only its values
  (`^[0-9a-f]{64}$`), not its keys. More decisively,
  `bundle_manifest_from_run` (`dispatch.rs:422`) rebuilds the bundle
  manifest from six enumerated keys and `dispatch_from_run`
  (`dispatch.rs:445`) re-hashes that reconstruction against the stored
  `bundle_sha256` — so a top-level member would make **every composed
  run under a dispatch envelope fail with `BadManifest`**, and a v3
  schema alone would not fix it without a v3 builder, a round-trip
  branch, and an engine version-selection branch. `files` entries are
  copied verbatim. `manifest_for` gains one refusal, a sibling of the
  existing `secrets.env` refusal, so the reserved namespace cannot be
  forged from disk.

- **`extends` resolves from the leaf directory's parent**, over a closed
  lowercase name grammar validated before any path join. In every real
  invocation the parent already *is* the library: `recipes::resolve`
  returns `<recipes-dir>/<name>`, and `recipes::add` copies into
  `<dir>/<name>` and then compiles, so verify-on-install keeps working
  unchanged. Built-in bundles are legal bases for their siblings; that
  falls out rather than being a second search path.

- **Both operator surfaces show the resolved result and its
  provenance**, from one renderer: the `Compile` arm's printer is
  extracted, `brokkr recipes show <name>` is added, and both emit a
  `composed_from` chain that is **omitted entirely when empty** — an
  empty array would be different bytes and would break the hard
  regression on all five existing bundles.

- **The proof is a real composition.** `recipes/sdd-paranoid/` extends
  `sdd` and overrides only its review seat; its README states the line
  count against `recipes/sdd/bundle.json`'s 227.

## Impact

**Behaviour that changes**: none for any existing recipe. A bundle that
declares no `extends` enters no new branch, performs no new I/O, and
emits no `@compose/` entry, so byte-identity of its manifest and digest
is true by construction — pinned as goldens for `recipes/fast`,
`recipes/panel-review`, `recipes/sdd`, `bundles/self` and
`bundles/verify`.

**Surfaces added**: `brokkr recipes show <name>`; a `composed_from`
member on `brokkr compile` and `brokkr recipes show`, present only for
composed bundles.

**Contracts**: unchanged. No new schema file, no edit to a frozen one.
`fixtures/evaluator/corpus.ndjson`, `policy/phase-machine.json` and
`reference/` are untouched. No new dependency.

**Signatures**: `Bundle` gains `roots` and `chain` (the resolved
ancestors — `composed_from` renders `{recipe, digest, dir}`, and `dir`
is not recoverable from the manifest, which carries names and digests
only); `confined_command` takes `&[PathBuf]` instead of one path.
`manifest_for` takes the chain. `Bundle::compile(dir)` keeps its
signature — deliberately, so the parallel decision-0016 agent slice
rebases mechanically: that slice edits seat *parsing*, this one edits
what is handed *to* parsing.

**Accepted residuals**, stated rather than hidden: a derived recipe does
not compile outside its library; confine mounts widen to ancestor
directories; `files` widens from "file bytes" to "identity ingredient";
a composed recipe inherits its base's workdir assumptions along with its
seats; and the case-insensitive-filesystem check is declined because its
error arm is unreachable on the Linux CI that enforces literal 100%
coverage. Each is recorded with its reasoning in
[plan.md](../../../specs/composable-recipes/plan.md).
