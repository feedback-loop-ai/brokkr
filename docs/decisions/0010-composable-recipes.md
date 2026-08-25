# 0010 — Composable recipes: delivery strategies as swappable, comparable data

**Status**: accepted (operator directive, 2026-08-25 — "a library of
recipes; download and swap a recipe, re-run, compare the outcomes:
that's the endgame")

## Ruling

A **recipe** is a bundle directory treated as a delivery strategy:
policy, seats, charters, limits, drivers — reviewable text, identified
by its content digest. The engine gains the strategy loop:

- `forge recipes list` — the library: local `recipes/` plus the
  built-in `self` and `verify`, each compiled and digest-identified.
- `forge recipes add <path|git-url> --name <n>` — install a recipe from
  disk or a shallow git clone; compile-verified before it may join the
  library (a non-compiling recipe never installs).
- `forge run --recipe <n>` — swap strategies by name.
- `forge rerun --run <id> --recipe <n>` — re-run a past run's feature
  under another recipe: a fresh, independent journal; the feature text
  is read from the source run's own `run/started` fact.
- `forge compare <a> <b>` — the aligned outcome comparison: decision
  trails with the first divergence point, phases and attempts, per-seat
  costs, recipe digests, status pair, cost and attempt deltas. A pure
  read over two journals; works on live runs.

Deliberately NOT stored: run linkage. Comparison is by run ids, so no
event vocabulary, fold semantics, database schema, or contract changed
— treatments are ordinary runs, and any two runs can be compared.

## Why

This is the target architecture's audit promise made operator-shaped:
"topology comparability … model substitution while holding topology
constant; topology ablation; fixed-budget cost/quality frontiers."
Because every run pins its recipe by digest and every claim is a
journaled fact, comparing strategies is a fold, not a benchmark
harness.

## Evidence

Delivered BY the forge (three self-forge runs; the recipes run's review
seat found and fixed real clone-injection, ext-transport, and symlink
holes in its own implementer's work across two fix rounds). First live
A/B: the same feature run under `self` and under a `fast` recipe
(intake ablated) from the same base — identical review verdict,
divergence at ruling index 0, the framing phase measured at ~$2.41 and
~2 minutes on a small task. The comparison itself ran mid-flight
against B's live journal.

## Consequences

- Recipes are the unit the library, LaneTally treatments, and future
  Looper dispatch select by; digests are identity, names are display.
- `recipes/` is user space; `bundles/` stays system space.
- Machine proof covers the loop end to end (rerun + divergent compare).
