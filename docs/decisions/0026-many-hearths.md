# 0026 — Many hearths: per-realm journals and the tabbed fleet

Status: proposed
Date: 2026-09-01

## Context

The first full-parallel burn — five slices forging at once, each in its
own worktree with its own journal so the engines never contend — made a
gap flesh within the hour: the operator's TUI, faithful to one world,
showed none of the five fires. Realms v1 deliberately gave a world one
journal; parallelism gives a workspace many. The operator's ruling
instinct named the surface: tabs in the runs pane, per realm.

## Decision

1. **`forge.realms/v2`: a realm may carry its own `journal`.** One
   optional field per realm, falling back to the world's journal when
   absent; the schema lands beside v1, never editing it, loader refusals
   intact. A parallel workspace becomes expressible as data: one map,
   one world, many hearths.

2. **The runs pane grows tabs when the world holds more than one
   journal.** Realm names as the tab bar; `[` and `]` (and number keys)
   switch; selection, filter, and cursor state kept per tab; stores
   opened lazily and the active tab refreshing at the fleet cadence. A
   world with one journal shows no bar and behaves byte-for-byte as
   today — a world that never drew two journals notices nothing.

3. **Every fleet reader resolves the same way.** `runs` prints grouped
   by realm in a many-hearth world; `muninn` gains the whole world's
   dossier — the raven flies over every hearth the map names, findings
   cited per realm. Single-run verbs are untouched: a run id lives in
   exactly one journal, and naming the realm (or letting a unique prefix
   resolve) is the lookup, never a merge.

4. **Journals never merge.** Each hearth's journal stays its own
   append-only, hash-chained truth; the tabs are a reading surface, not
   a store operation. Nothing in this decision writes.

## Consequences

Parallel burns get one pane of glass without giving up the isolation
that makes them safe; the tmux-grid workaround retires; and Muninn's
charter finally matches its myth — the raven was always meant to fly
over more than one world before reporting at dusk. The cost is tab-state
bookkeeping in the TUI and the discipline of route-by-realm for run ids,
both bounded.
