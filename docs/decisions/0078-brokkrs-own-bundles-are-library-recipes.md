# 0078 — Brokkr's own bundles are library recipes

Status: accepted — operator ruled 2026-10-08 (PR #590). The same day the operator ruled that fast's wording stands for the three rule reasons node's and panel-review's copies had drifted on, so both overlays now inherit fast's rules whole.
Date: 2026-10-08

Built: built
Amends: 0010

## Context

Decision 0010 gave the recipe library its verbs and two consequences that
story #359 no longer holds: `recipes list` showed the local `recipes/`
plus the built-in `self` and `verify`, and "`recipes/` is user space;
`bundles/` stays system space". Story #359 composes fast's constitution
instead of copying it. `review-first`, `node` and `panel-review` extend
`fast`, and Brokkr's own `self` became an overlay of `panel-review`. An
overlay resolves its base from the library its leaf sits in (decision
0017), so `self` cannot compose from `bundles/` while its bases live in
`recipes/`. Keeping a second directory would mean keeping `self` a copy.

Two alternatives were weighed. Copying `panel-review` and `fast` into
`bundles/` keeps 0010's split but restates every table the story removes.
Teaching composition to look in a second library widens what a confined
seat's read-only mount must hold, for no recipe's benefit.

## Rulings

1. **Brokkr's own bundles are library recipes.** `bundles/self` and
   `bundles/verify` are `recipes/self` and `recipes/verify`, and there is
   no `bundles/` directory. Brokkr's library is the repository's
   `recipes/`, which an operator's library mirrors rather than stands
   beside. Enforcement: CI and release compile `recipes/self` and
   `recipes/verify`, and `crates/brokkr-runtime/tests/witnesses.json`
   pins every recipe's resolved table.
2. **`recipes list` lists the library and nothing else.** It no longer
   adds built-ins read from the workspace, so `self` and `verify` appear
   where they are installed, like any recipe. Enforcement:
   `crates/brokkr-cli/src/recipes.rs` `list` reads only `--dir`.
3. **A derived recipe installs with its bases.** `recipes add` copies,
   beside the leaf, each base its `extends` chain reaches that the target
   library does not already hold, under the name it is extended by. It
   stops at the first base the library holds. If any copy fails to
   compile, every copy is removed. Enforcement: the `recipes::tests` cases
   for a derived and a refused install in `crates/brokkr-cli`.
4. **A recipe that composes `fast` runs `fast`'s verifier.** The seat
   scripts decision 0048 pins in each recipe directory are copies of one
   canonical source under `recipes/fast/scripts/`. So `review-first`,
   `panel-review`, `self` and `verify` run the newer verifier: formatting,
   the non-Rust lints, clippy with warnings denied, the suite, the self
   compile and #287's failure filter. They no longer run the older copy,
   which ran only the suite and the self compile. The verdict is still
   each command's exit status, so the change makes the gate stricter.
   Enforcement: `every_pinned_script_and_charter_copy_is_its_canonical_source`
   in `recipe_library.rs`, with its named exemptions.

## Consequences

- 0010's verbs stand. Its listing sentence and its `bundles/` consequence
  read as rulings 1 and 2 here.
- An operator who installed `node` before #359 keeps a whole copy. A new
  `recipes add` of `node` also brings `fast`, and both show in
  `recipes list`.
- History keeps its paths. Decisions, evidence, essays, frozen fixtures
  and in-flight changes that name `bundles/self` record what was true
  when they were written.
