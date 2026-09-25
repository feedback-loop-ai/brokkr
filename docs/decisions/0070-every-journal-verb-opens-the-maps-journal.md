# 0070 — Every verb that opens a journal opens the map's journal

Status: proposed (implementer, 2026-09-25)
Date: 2026-09-25

## Context

Decision 0023 ruling 3 says how an invocation finds its journal: a map
named with `--realms` is loaded or the command refuses, `./realms.json` is
the map when it exists, and `--db` outranks whatever journal the map names.
Phase 1 wired that rule into `run` and the read surfaces the ruling named
(`runs`, `realms`, `tui`, `watch`, `inspect`, `transcript`, `export`,
`import`, `muninn run`), and decision 0047 added `operator supersede`.

Every other verb that opens a journal kept `--db` alone, defaulting to
`.forge/forge.db`: `resume`, `rerun`, `conclude`, `operator retry` and
`stop`, `costs`, `replay`, `ledger`, `anchor`, `keep-refs`, `compare`,
`bridge`, `ui` and `doctor`'s database line (issue #374). In a mapped
world, which the `DEFAULT_DB` documentation itself calls the normal setup,
`brokkr run` writes to the map's journal and the recovery verbs look in a
different file: they fail exactly when they are needed, and operators pass
`--db` every time. `rerun` pinned the map's world but wrote its new run to
the default journal. `ui` showed a different fleet from `tui`.

## Decision

1. **One parameter object.** `JournalArgs { realms, db }` is flattened into
   every one of those verbs and resolved once, on 0023 ruling 3's three
   rules, before the verb opens anything. With neither a map nor `--db`,
   the journal is `.forge/forge.db`, so a world that never drew a map
   notices nothing.
2. **Journal-only verbs read the map as a read surface does.** `conclude`,
   `operator retry`/`stop`, `resume`'s store, `costs`, `replay`, `ledger`,
   `anchor`, `keep-refs`, `compare` and `bridge` take the map only for its
   journal and pin no world, so a crossing that has moved does not refuse
   them (`World::inspect`). `resume` still refuses one through its own
   pinned world's fence before any seat spawns.
3. **`rerun` resolves strictly.** It starts a new run and pins the world,
   so it resolves through `World::discover` like `run`, and the new run is
   written to the journal of the world it pins.
4. **`ui` reads the fleet `tui` reads.** It resolves the hearths exactly as
   `tui` does, strictly. The web view serves one journal, so a world whose
   realms name several hearths is refused, naming them, until `--db` picks
   one. It is never served one hearth as though that were the fleet.
5. **The default flips at once.** All thirteen verbs change their default
   journal in mapped worlds in this one change. `--db .forge/forge.db`
   reproduces the old default for any single invocation.

## Asked of the operator

- **Direct flip or legacy mode (ruling 5).** Issue #374 suggests landing
  the refactor with a per-command legacy mode and flipping it later. This
  change flips directly, because the issue's acceptance (run → resume →
  conclude → operator stop with no `--db`) needs the flipped default, and
  a mode nobody selects would be dead code under the exact-coverage gate.
  If the ruling is a staged flip, the legacy mode is added in place of
  ruling 5.
- **`ui` in many-hearth worlds (ruling 4).** Refusing is the smallest
  honest answer. Serving every hearth from the web view is a feature of its
  own.
- **`RunSelector::{Readout, Exact}`.** The issue also proposes encoding
  decision 0015's scope (readouts take a prefix or `latest`, write paths
  take the full id) as a type. That scope is unchanged here and does not
  bear on which journal a verb opens, so it is left as a follow-up.

## Consequences

- A run started in a mapped world is resumed, concluded, stopped and
  retried with no `--db`.
- `operator retry` and `stop` accept `--realms`.
- In a v2 map, `run` with no `--db` still writes to the world journal even
  when the operated realm names its own. That predates this decision and
  is not changed here.
