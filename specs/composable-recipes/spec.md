# Feature Specification: Composable recipes

**Feature slug**: `composable-recipes`
**Run**: `implement-decision-0017-composab-0f8a7784`
**Status**: Committed (design phase ruling)
**Scope**: implements decision 0017
(`docs/decisions/0017-composable-recipes.md`, accepted 2026-08-29). No
new decision doc.
**Input positions**: `.forge/design/positions/simplicity.md`,
`.forge/design/positions/robustness.md` (run-local, uncommitted)

## Why

Decision 0010 made a recipe a named, installable, comparable whole
bundle. It did not make one **composable**. Wanting the SDD design
council with a different review panel today means copying
`recipes/sdd/` — 227 lines of `bundle.json`, 185 of `policy.json`, a
`roles/` tree and a `drivers/` tree — and editing two files. From the
moment of the copy the two drift, silently, and nothing in the system
records that one came from the other.

Decision 0017 rules the alternative: **a recipe may extend another
recipe and state only its differences**. The differences are resolved at
compile time into one flat bundle; the engine never learns that
composition happened. That is what makes the experiment surface decision
0010 was aiming at actually cheap — swap one phase, rerun, `forge
compare` — without buying a run-time inheritance mechanism nobody can
reason about at 3am.

## The design in one paragraph

Composition is a **pure pre-pass over recipe source documents**, living
in one new module, `crates/brokkr-runtime/src/bundle/compose.rs`. Its
input is a leaf bundle directory; its output is a `Resolved` value
carrying the flat `bundle.json` document, the flat policy table, a
**name-level origin map**, the ordered ancestor chain with digests, and
the layer directories. `Bundle::compile` calls it and then parses
**exactly as it does today**, over the resolved documents instead of
files it read itself. Not one existing lint, parse function, or error
message moves. Seat values are **opaque**: the resolver decides only
*which* value wins for a given name and never opens one — which is
possible only because the `override` and `remove` markers live in
resolver-owned top-level keys **beside** the values rather than inside
them. The composition chain is pinned by riding inside the existing
manifest's `files` map under a reserved `@compose/` key prefix, which
needs **no new contract version** and — decisively — is the only shape
that survives the v2 dispatch round-trip. A recipe that declares no
`extends` enters no new branch and performs no new I/O, so byte-identity
for the existing five bundles is true by construction rather than by
arithmetic.

## Position reconciliation (what was adopted, rejected, reconciled)

The panel agreed on more than the framing anticipated, and the
agreements are load-bearing:

- Resolution yields a **document**, not parsed seats. Purity is only
  testable above `parse_command`, which expands `{brokkr}` to
  `std::env::current_exe()` (`bundle.rs:633`) and would otherwise differ
  by machine; and opacity is only assertable against a document, because
  parsing discards unknown keys.
- Markers live **outside** the value. Both positions reached this
  independently and for the same reason: stripping an in-value marker
  before merging *is* rewriting an opaque value.
- A **name-level origin map**, so an inherited seat resolves its `role`
  and `./`-prefixed argv against the layer that supplied it, without the
  resolver ever reading inside the seat.
- **Multi-root confine mounts.** `confined_command` (`engine.rs:1618`)
  mounts exactly one directory; an inherited confined seat would
  otherwise fail hours into a run with a driver-level file-not-found.
- Rule override is **remove-then-prepend**, not naive prepending.
- No deep merge inside a named thing, no multiple inheritance, no
  run-time lookup, no new dependency, no temp-directory materialization
  of a composed bundle.

Where they disagreed, this spec rules:

1. **The manifest contract — simplicity adopted, and the evidence is
   stronger than either position argued.** The chain rides in `files`
   under `@compose/`; there is no `run-manifest.v3`. Robustness is right
   that `run-manifest.v1` and `v2` are `additionalProperties: false` and
   that a **top-level** `composition` member is illegal in both. But
   `files` constrains only its *values* (`^[0-9a-f]{64}$`) and places no
   pattern on its keys, so `@compose/` entries validate against both
   frozen schemas unmodified. The deciding fact is the one robustness
   itself found and then argued past: `bundle_manifest_from_run`
   (`dispatch.rs:422`) rebuilds the bundle manifest from six enumerated
   keys, and `dispatch_from_run` (`dispatch.rs:445`) then re-hashes that
   reconstruction against the stored `bundle_sha256`. A top-level
   `composition` member is therefore not merely dropped — it makes
   `bundle_sha256` disagree with its own round-trip, so **every composed
   run under a dispatch envelope fails `dispatch_from_run` with
   `BadManifest`**, not just resume. A v3 schema does not fix that by
   itself; it requires a v3 builder, a v3 branch in
   `bundle_manifest_from_run`, a version-selection branch in the engine,
   and a second frozen contract file. `files` entries are copied
   verbatim through the round-trip and need none of it.
2. **Where `extends` resolves from — simplicity adopted, with
   robustness's guard.** The library is the leaf directory's parent. In
   every real invocation that already *is* the recipe library:
   `recipes::resolve` (`recipes.rs:20`) returns `<recipes-dir>/<name>`,
   and `recipes::add` copies into `<dir>/<name>` and *then* compiles, so
   verify-on-install keeps working with no change. Threading an explicit
   `Library` would change `Bundle::compile`'s signature across four
   crates and every bundle-building test — a diff the parallel 0016
   rebase would have to reconcile — to name a directory the caller
   already stands in. Robustness's real objection is path escape, and
   that is closed completely by its own proposal: a **closed name
   grammar** validated *before* any path join.
3. **Digest shape — simplicity adopted.** Ancestor digest entries, not a
   prefixed union of every ancestor's files. The union does not satisfy
   deliverable 5 on its own (the ruling asks for *each ancestor recipe
   and its digest*, which is still a separate record), so it is strictly
   additive work plus a new cross-layer key-collision error arm. An
   ancestor's digest transitively covers every byte of that ancestor and
   of its own ancestors, so a base change moves the derived digest for
   the most direct possible reason.
4. **Case-exact directory matching — rejected.** Robustness is right
   that a case-insensitive filesystem could match `extends: "sdd"`
   against a directory named `SDD`. The error arm that would catch it
   **cannot be reached by a test on the Linux CI that enforces
   `scripts/coverage-exact.sh`**, and an unreachable arm fails the gate.
   The lowercase-only name grammar is the testable half of the
   mitigation and is adopted; the residual is recorded in `plan.md`.
5. **A composition-layer duplicate of the unreachability lint —
   rejected, replaced.** Robustness is right that `Machine::from_table`
   (`policy.rs:118`) rejects unreachable rules and duplicate ids with
   messages that name neither file. Re-implementing those checks with
   layer knowledge is two more lints to keep in sync with the heritage
   ones. Instead, **every compile error raised downstream of resolution
   on a composed bundle is wrapped once with the composition chain** —
   one arm, one test, and it names the chain for unreachable rules,
   duplicate ids, and bad seat bodies alike.
6. **`parse_role` rejecting paths that escape their origin dir —
   rejected as out of scope.** It is a pre-existing property of
   single-root bundles, not something composition introduces, and the
   framing binds this slice to the composition layer. Recorded as a
   residual risk in `plan.md`.

## Behaviour

### Declaring a composition

```jsonc
{
  "name": "sdd-paranoid",
  "extends": "sdd",
  "override": { "seats": ["review"] },
  "seats": { "review": { /* wholesale replacement, opaque */ } }
}
```

- `extends` is a **name**, matching `^[a-z0-9][a-z0-9-]*$`. It is
  validated before any path join, so `../x`, `a/b`, `SDD` and `.` are
  refused as names and never become paths.
- It resolves to `<leaf dir>/../<name>`. Built-in bundles are legal
  bases for their siblings under `bundles/` — that falls out, it is not
  a second search path.
- Chains resolve to arbitrary depth, leaf-first, each layer's own
  `extends` read from its own `bundle.json`. A **cycle is a compile
  error naming the whole loop in order** (`a -> b -> c -> a`;
  self-extends is the degenerate `a -> a`). A chain deeper than **8**
  layers is a compile error naming the chain so far.
- A recipe with **no `extends`** never reads the library, never enters a
  merge branch, and never emits a `@compose/` entry.

### Merge rules

Named things merge by name. Values are replaced **wholesale**; there is
no field-level merge inside a named thing.

| Member | Add (no marker) | Redefine | Remove |
|---|---|---|---|
| `seats.<name>` | free | `override.seats: ["<name>"]` | `remove.seats: ["<name>"]` |
| policy `rules[].id` | free | `override.rules: ["<id>"]` | `remove.rules: ["<id>"]` |
| table name arrays (`phases`, `terminal`, `shippable_from`) | union: base order first, derived-only names appended | `override.table: ["<field>"]` replaces the array wholesale | `remove.phases: ["<name>"]` |
| table scalars (`initial`, `description`) | — | `override.table: ["<field>"]` | — |
| `bundle.json` scalars (`protected_phase`) | — | `override.bundle: ["<field>"]` | — |

- `override` and `remove` are **resolver-owned top-level keys** of the
  derived `bundle.json`, keyed by member kind. They are never a key
  inside a value.
- Redefining a name the resolved base has **without** the marker is a
  compile error naming both files and the key.
- A **stale marker** is an error: listing a target the base does not
  have, or that the leaf does not actually redefine, is a lie about the
  composition and fails.
- `remove` naming a target the resolved base does not have is an error
  naming the file and the key.
- Re-declaring a **name** the base already lists in a name array is a
  no-op, not a conflict — there is no value to collide with, only a
  name. This is what keeps a derived recipe small.

### Fields with their own rule

- **`name`** is required in every layer and **must differ from every
  ancestor's name**. Inheriting it is a compile error. A derived bundle
  that reported its base's name would write that name into
  `runs.bundle_name`, `manifest.bundle_name`, `brokkr runs` output and
  the dispatch `recipe` pin, under a different digest.
- **`policy`** is per-layer, not overridden: each layer that declares it
  contributes a table document, read relative to **that layer's**
  directory. A layer that declares no `policy` contributes no table. The
  contributed tables merge by the rules above, derived over base.
- **`schema`** (policy table) must match across layers; a mismatch is an
  error naming both files.

### Policy composition

Derived rules **precede** base rules in the resolved table — the
engine's existing first-match-wins order, unchanged. Overriding a rule
by `id` **removes the base rule and prepends the derived one**, so there
is never a dead twin: naive prepending would leave the base rule
unreachable and `Machine::from_table` would reject the whole table,
making the feature's headline use case structurally impossible.

`assert_phase_unavoidable` (`bundle.rs:388`) runs where it runs today —
after `Machine::from_table`, on the **resolved** table. That requires
zero new lint code; it is a consequence of resolving first. A derived
recipe that would make the protected review phase avoidable fails
compilation with the existing lint's message, wrapped with the chain.

### Inherited paths

The resolver records, **by name**, which layer supplied each seat. It
never learns that a seat has a `role`, a `driver`, a `panel` or a
`sequence`. `Bundle::compile` passes that layer's directory to the
existing `dir` parameter of `parse_role` (`bundle.rs:551`) and
`parse_command` (`bundle.rs:596`), so an inherited seat's `role` and
`./`-prefixed argv resolve against the recipe that wrote them.

`Bundle` gains `roots: Vec<PathBuf>` (leaf first, deduplicated);
`bundle.dir` stays the leaf and every existing use of it is unchanged.
`confined_command` mounts **every** root read-only. For a non-composed
bundle `roots == [dir]`, so the emitted argv is byte-identical.

A non-`./` argv entry (such as `recipes/sdd`'s `bash
recipes/sdd/drivers/speckit_check.sh`) resolves against the **run's
workdir** and the resolver must not rewrite it — rewriting requires
reading the seat. A composed recipe therefore inherits its base's
workdir assumptions along with its seats.

### Pinning and provenance

The **resolved** bundle's digest pins the run, exactly as today. The
chain rides inside the digested manifest's `files` map:

```
"@compose/0000/sdd": "<sdd's own manifest_for digest>"
```

Index `0000` is the leaf's immediate base, increasing toward the root
ancestor, zero-padded so canonical key sorting preserves chain order.
An ancestor's digest is its own `manifest_for` digest — structural
identity, computed without compiling it; an ancestor is not required to
pass the constitutional lint standalone, because validity is a property
of the resolved bundle and the resolved table is linted anyway.

Whole ancestor trees are hashed, including files an override made
unreachable. Computing the live set would require knowing which roles
and drivers the seats reference, which requires reading inside seat
values — the exact thing the 0016 layering forbids. Over-inclusion fails
closed (a diagnosable resume refusal); under-inclusion fails open (a
changed role that does not move the pin). **There are no exclusions:
the derived recipe's own `README.md` is inside its digest.**

`manifest_for` gains one refusal, a sibling of the existing
`secrets.env` refusal (`bundle.rs:742`): a real bundle file whose
bundle-relative path starts with `@compose/` is rejected, so the
reserved namespace cannot be forged from disk.

Resume needs nothing new. It recompiles, re-resolves from the same
parent library, and the existing digest-mismatch refusal
(`engine.rs:187`) fires; `manifest_diff` (`engine.rs:1651`) already
reports which `files` entry moved, so "a base changed under you"
surfaces as `changed: @compose/0000/sdd`, by name.

### Operator surfaces

`brokkr compile` and a new `brokkr recipes show <name>` print the
**resolved** result and its provenance, from **one renderer** — the
`Compile` arm's printer (`main.rs:658`) is extracted and both surfaces
call it. Both gain a `composed_from` member (an array of
`{recipe, digest, dir}`, leaf-first) which is **omitted entirely when
the chain is empty**, so non-composed output does not shift by a byte.

Every error the composition layer raises names the **file path**, the
**key** at issue, and — where two layers disagree — **both** paths.

## Acceptance Criteria

Each criterion is testable and is referenced by `tasks.md` as AC-n.

**Resolution**

- **AC-1** — `resolve` is pure: the same recipe sources resolve to
  byte-identical canonical output across repeated calls in one process,
  with no dependence on the clock, the environment, `read_dir` order, or
  `HashMap` iteration (`BTreeMap` throughout).
- **AC-2** — A chain of depth ≥ 3 resolves deterministically, with each
  layer's own `extends` honoured.
- **AC-3** — A cycle is a compile error naming the whole loop in order.
  Covered for self-extends (`a -> a`), `a -> b -> a`, and a
  three-deep loop.
- **AC-4** — A chain deeper than 8 layers is a compile error naming the
  chain so far.
- **AC-5** — `extends` naming a recipe not present in the parent library
  is an error naming the leaf file, the missing name, and the directory
  searched.
- **AC-6** — `extends` whose value fails `^[a-z0-9][a-z0-9-]*$` is
  refused **before any path join**, naming the file and the offending
  value. Covered for `../x`, `a/b`, `SDD`, and `.`.

**Merge**

- **AC-7** — Adding a phase or seat the base lacks succeeds with no
  marker.
- **AC-8** — Redefining a base seat **without** `override.seats` fails,
  naming both files and the seat name; **with** the marker it succeeds
  and replaces wholesale.
- **AC-9** — A stale marker fails: `override.seats` naming a seat the
  base lacks, and `override.seats` naming a seat the leaf does not
  redefine.
- **AC-10** — `remove.seats` of an existing seat succeeds; of a missing
  seat fails, naming the file and the key. Likewise `remove.rules` and
  `remove.phases`.
- **AC-11** — **The 0016 layering guarantee.** A seat value carrying
  arbitrary keys the resolver has never heard of survives inheritance
  and override **byte-identically** in the resolved document. Asserted
  as a byte comparison against the resolved document, not against a
  parsed `Seat`.
- **AC-12** — A derived recipe that omits `name`, or declares a `name`
  equal to any ancestor's, fails compilation.

**Policy**

- **AC-13** — Derived rules precede base rules in the resolved table.
- **AC-14** — Overriding a rule by `id` leaves exactly one rule with
  that id, the derived one, in derived position; the base twin is gone
  and `Machine::from_table` accepts the table.
- **AC-15** — Name arrays union (base order first, derived appended);
  re-declaring an existing name is a no-op; `override.table` replaces an
  array or scalar wholesale; redefining a scalar without the marker
  fails.
- **AC-16** — A policy-table `schema` mismatch across layers is an error
  naming both files.
- **AC-17** — **The constitutional lint runs on the resolved table.** A
  derived recipe that adds a rule making the protected review phase
  avoidable fails compilation with the existing
  `assert_phase_unavoidable` message.
- **AC-18** — A compile error raised downstream of resolution on a
  composed bundle is wrapped once with the composition chain, naming the
  derived recipe and its base.

**Inherited paths**

- **AC-19** — An inherited seat's `role_path` points into the
  **ancestor's** directory, not the leaf's.
- **AC-20** — A confined inherited seat's `confined_command` argv
  contains one read-only mount per root; for a non-composed bundle the
  argv is byte-identical to today's.

**Pinning**

- **AC-21** — **Hard regression.** `recipes/fast`,
  `recipes/panel-review`, `recipes/sdd`, `bundles/self` and
  `bundles/verify` compile to byte-identical manifests and identical
  digests, pinned as goldens. None emits a `@compose/` entry.
- **AC-22** — A composed bundle's manifest is pinned by a golden test,
  including its `@compose/` entries in chain order.
- **AC-23** — Mutating a base source in a temporary library and
  re-resolving **moves the derived digest**.
- **AC-24** — The chain is readable back from the manifest as an ordered
  list of (recipe name, digest).
- **AC-25** — A real bundle file whose bundle-relative path starts with
  `@compose/` is refused by `manifest_for`.
- **AC-26** — A composed run **started under a dispatch envelope
  resumes**: the v2 round-trip preserves the chain and
  `dispatch_from_run`'s `bundle_sha256` check passes.
- **AC-27** — Resume of a composed run whose base changed refuses with
  `manifest_diff` naming the moved `@compose/` entry.

**Operator surfaces**

- **AC-28** — `brokkr compile` on a composed recipe prints the resolved
  result and a `composed_from` provenance chain; on a non-composed
  bundle the output is byte-identical to today's (no `composed_from`
  member).
- **AC-29** — `brokkr recipes show <name>` prints the resolved result and
  provenance, proven for both a composed and a non-composed recipe.
- **AC-30** — At least one merge failure is asserted at the CLI level,
  showing the file and the conflicting key.
- **AC-31** — `brokkr recipes list` prints a warning line for a recipe
  whose base is missing and still lists everything else.

**Proof and suites**

- **AC-32** — `recipes/sdd-paranoid/` exists, declares `extends: "sdd"`,
  overrides only the review seat, compiles, and its README states its
  `bundle.json` line count against `recipes/sdd/bundle.json`'s 227.
- **AC-33** — The 97-case differential evaluator corpus, the
  machine-proof suite, `brokkr-view`/render goldens, workspace `cargo
  test`, `cargo clippy --all-targets --all-features -D warnings`,
  `cargo fmt --check` and `scripts/coverage-exact.sh` are green;
  `fixtures/evaluator/corpus.ndjson`, `policy/phase-machine.json`,
  `reference/` and every frozen contract file are untouched.

## Out of scope

Everything from decision 0016 (agents, `agent:` keys, adapter mappings,
`brokkr agents *`); multiple inheritance; field-level merge inside a
named thing; run-time inheritance or dynamic lookup; run linkage, new
event vocabulary or schema change; new dependencies; a new UI surface
beyond displaying resolved provenance.
