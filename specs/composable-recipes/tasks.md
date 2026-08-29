# Tasks: Composable recipes

**Feature slug**: `composable-recipes`
**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Ordered; each task names the test that proves it. AC-n refers to
`spec.md`'s `## Acceptance Criteria`. Every task lands with its tests in
the same commit — the coverage gate (`scripts/coverage-exact.sh`,
literal 100% line/branch/function) is satisfied per task, not swept up
at the end.

The ordering is deliberate: **the hard regression is pinned before any
merge logic exists** (M1), so every later movement runs against it.

## Movement 1 — the seam, and the regression that guards it

- [x] **T1 — `bundle/compose.rs` with the no-`extends` path only.**
  `resolve(leaf)` reads `bundle.json`, sees no `extends`, and returns
  `Resolved { document, table, seat_origin: all layer 0, chain: vec![],
  roots: vec![leaf] }`. `Bundle::compile` consumes it and parses as
  today. No merge code, no library lookup, no I/O beyond what `compile`
  already did.
  *Proven by*: AC-21 — the five existing bundles (`recipes/fast`,
  `recipes/panel-review`, `recipes/sdd`, `bundles/self`,
  `bundles/verify`) compile to byte-identical manifests with their
  digests pinned as goldens; plus AC-1's purity test on a non-composed
  recipe.

- [x] **T2 — `Bundle::roots`, and `confined_command` mounts every
  root.** `confined_command` takes `&[PathBuf]`; the three call sites
  (`engine.rs:609,843,1031`) pass `&self.bundle.roots`. `dir` stays the
  leaf and every existing use of it is unchanged.
  *Proven by*: AC-20's non-composed half — the emitted argv for a
  confined seat in a single-root bundle is byte-identical to today's.

## Movement 2 — extends, chains, refusals

- [x] **T3 — Name grammar and library lookup.** `extends` is validated
  against `^[a-z0-9][a-z0-9-]*$` **before any path join**, then resolved
  to `<leaf dir>/../<name>`. A missing base names the leaf file, the
  name, and the directory searched.
  *Proven by*: AC-6 (`../x`, `a/b`, `SDD`, `.` each refused pre-join)
  and AC-5.

- [x] **T4 — Chain walking, cycles, depth cap.** Each layer's own
  `extends` is read from its own `bundle.json`; a visited list detects
  cycles and the error names the whole loop in order; depth > 8 errors
  naming the chain so far.
  *Proven by*: AC-2 (depth ≥ 3 resolves), AC-3 (`a -> a`, `a -> b -> a`,
  and a three-deep loop, each error asserted to contain the loop in
  order), AC-4.

- [x] **T5 — `name` is required and must differ from every ancestor.**
  *Proven by*: AC-12 — a derived recipe omitting `name` fails; one
  declaring an ancestor's `name` fails.

## Movement 3 — the merge, and the opacity guarantee

- [x] **T6 — Seat merge with kind-keyed markers.** `override.seats` /
  `remove.seats` as resolver-owned top-level keys; addition needs no
  marker; redefinition without the marker fails naming both files and
  the key; wholesale replacement with the marker; stale markers fail;
  removing a missing seat fails.
  *Proven by*: AC-7, AC-8, AC-9, AC-10.

- [x] **T7 — The 0016 layering test.** A seat value carrying arbitrary
  unknown keys survives inheritance and override byte-identically in the
  resolved **document**.
  *Proven by*: AC-11, written as a byte comparison against
  `Resolved::document` — not against a parsed `Seat`, which would
  discard the unknown keys and make the test vacuous. This test exists
  in its own right so a future refactor cannot quietly weaken it.

- [x] **T8 — `policy` is per-layer.** Each layer that declares `policy`
  contributes a table read relative to **that layer's** directory; a
  layer that declares none contributes nothing. `schema` must match
  across layers.
  *Proven by*: AC-16, plus a two-layer composition where only the base
  declares `policy` and the resolved table is the base's.

- [x] **T9 — Table merge.** Name arrays union (base order first, derived
  appended; re-declaring a name is a no-op); `override.table` replaces
  an array or scalar wholesale; redefining a scalar without the marker
  fails; `remove.phases`; `override.bundle` for `protected_phase`.
  *Proven by*: AC-15, AC-10's `remove.phases` half.

## Movement 4 — policy order and the constitutional lint

- [x] **T10 — Derived rules prepend; override-by-id is
  remove-then-prepend.** The base twin is removed so
  `Machine::from_table`'s unreachability lint (`policy.rs:118`) never
  fires on a legitimate override.
  *Proven by*: AC-13, AC-14 — exactly one rule with the overridden id,
  derived, in derived position, and the table accepted.

- [x] **T11 — The lint runs on the resolved table**, with zero new lint
  code, and downstream compile errors on a composed bundle are wrapped
  once with the chain.
  *Proven by*: AC-17 — a derived recipe adding a rule that ships around
  the protected phase fails with the existing
  `assert_phase_unavoidable` message — and AC-18.

- [x] **T12 — Inherited paths resolve against their origin layer.**
  `parse_role`/`parse_command` receive `roots[seat_origin[name]]`.
  *Proven by*: AC-19 (an inherited seat's `role_path` points into the
  ancestor directory) and AC-20's composed half (one read-only mount per
  root in `confined_command`'s argv).

## Movement 5 — the proof recipe, then the pins

- [x] **T13 — `recipes/sdd-paranoid/`.** `bundle.json` with
  `extends: "sdd"`, `override: { "seats": ["review"] }` and one review
  seat; two role files; a `README.md` stating its `bundle.json` line
  count against `recipes/sdd/bundle.json`'s 227. Existing
  `forge recipes list` assertions updated in the same commit.
  *Proven by*: AC-32 (it compiles, extends `sdd`, overrides only the
  review seat, README states the contrast) and the CLI suite staying
  green (plan risk R8).
  **Lands before T14**: the README is inside the digest, so pinning the
  golden first would break it on the next commit (plan risk R7).

- [x] **T14 — `@compose/` manifest entries and the reserved-namespace
  refusal.** `manifest_for` emits `@compose/{NNNN}/{name}` → ancestor
  digest, index 0 = immediate base, zero-padded so canonical sorting
  preserves order; and refuses any real bundle file whose relative path
  starts with `@compose/`, as a sibling of the `secrets.env` refusal.
  *Proven by*: AC-22 (golden pinning `sdd-paranoid`'s composed
  manifest), AC-24 (chain read back as an ordered name/digest list),
  AC-25, and AC-21 re-run unchanged.

- [x] **T15 — Base change moves the derived digest.** Mutate a base
  source in a temporary library, re-resolve, assert the derived digest
  moved.
  *Proven by*: AC-23.

- [x] **T16 — Resume, including under a dispatch envelope.** No new
  engine code is expected; this task exists to prove the `files`-key
  ruling actually survives `bundle_manifest_from_run`'s six-key rebuild
  (`dispatch.rs:422`) and `dispatch_from_run`'s `bundle_sha256` re-hash
  (`dispatch.rs:445`).
  *Proven by*: AC-26 (start a composed run with a dispatch envelope and
  resume it) and AC-27 (a changed base refuses with `manifest_diff`
  naming the moved `@compose/` entry).

## Movement 6 — operator surfaces

- [x] **T17 — One renderer, two callers.** Extract the `Compile` arm's
  printer (`main.rs:658`); add `RecipesCmd::Show`; both emit
  `composed_from` (array of `{recipe, digest, dir}`, leaf-first),
  **omitted entirely when the chain is empty**.
  *Proven by*: AC-28 (composed output carries provenance; non-composed
  output is byte-identical to today's), AC-29 (`show` on both a composed
  and a non-composed recipe).

- [x] **T18 — Diagnosable errors at the CLI level.**
  *Proven by*: AC-30 (a merge failure asserted through the CLI, showing
  the file and the conflicting key) and AC-31 (`recipes list` warns on a
  missing base and still lists everything else).

## Movement 7 — close

- [x] **T19 — `README.md` and `ARCHITECTURE.md`** gain composition in
  the strategy-loop paragraph that already advertises decision 0010's
  library.
  *Proven by*: review; no test.

- [x] **T20 — Full gate.** `cargo test --workspace`, `cargo clippy
  --all-targets --all-features -D warnings`, `cargo fmt --check`,
  `scripts/coverage-exact.sh`, the 97-case differential evaluator
  corpus, the machine-proof suite, and the `forge-view`/render goldens —
  all green, with `fixtures/evaluator/corpus.ndjson`,
  `policy/phase-machine.json`, `reference/` and every frozen contract
  file untouched (`git diff --stat` asserted clean for those paths).
  *Proven by*: AC-33.
