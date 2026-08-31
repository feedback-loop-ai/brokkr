# 0023 — Realms: the map is the world, chosen at invocation

Status: accepted — operator ruled 2026-08-31/09-01
Date: 2026-09-01

## Context

The heritage protocol was born polyrepo — per-repo implementation and
gates, every decision recording repo HEADs — and the Rust port
deliberately scoped to one repository. Meanwhile the practical question
"how does Brokkr drive several projects?" kept resolving to per-project
journals with no shared surface.

The operator's insight collapsed both questions into one mechanism:
**invoke Brokkr with a different map.** A map of repositories is a world;
the engine reads whichever world it is handed; a single project is the
degenerate map with one entry. The lore was waiting: the map is
Yggdrasil, the tree that holds every realm and the paths between them —
not a realm itself, just the connective truth. A repository is a realm.
The bridges between them (interface contracts, Bifröst) and the messenger
along the trunk (change propagation, Ratatoskr) are later phases.

Naming obeys 0019 ruling 10: the file is a mechanism and reads plainly —
`realms.json` — while the Edda names it Yggdrasil. And it is JSON, never
YAML: the whole family is JSON, the digest machinery speaks it, and
implicit typing has no place in evidence.

## Decision

1. **`realms.json` is the world's map, and this repo carries its own.**
   The bootstrap map lives at the repository root and lists one realm:
   this repository. A workspace of many projects is another file; a
   different client is a different world; the engine is furniture.

2. **Minimal first schema, by ruling.** A map names its `realms` — each
   with a name, a path, and a default branch — and the world's
   `journal`. Nothing else in v1: the 0021 per-realm driver and egress
   constraints are a LATER amendment, deliberately not speculatively
   schema'd. The schema lands as a versioned contract file beside the
   frozen ones.

3. **`--realms <file>` on the run AND on every read surface** — `run`,
   `runs`, `tui`, `watch`, `inspect`, `muninn` — defaulting to
   `./realms.json`, with today's `--db` retained as an override that
   outranks the map's `journal`. Absent both map and override, today's
   default db path behaves exactly as it always has: a world that never
   drew a map notices nothing.

4. **Pinned AND embedded.** At run start the engine records the map —
   its content hash into the run's manifest and the map content itself
   into the journal — so "what world did this run believe in?" is
   answerable from the journal alone, forever, whatever later became of
   the file.

5. **Per-realm truth on decisions.** The facts decisions already record
   about the repository (HEAD, drift, dirty worktrees) are recorded per
   realm, keyed by realm name — one realm today, several when Phase 3
   arrives, the shape ready either way.

6. **`brokkr realms`** lists the world: each realm, its path, branch,
   and current HEAD, and the world's journal — a read surface like the
   others, no writes.

7. **Phases 2 and 3 are their own decisions.** Bifröst crossings
   (published, digest-pinned inter-realm contracts) and multi-realm runs
   (one feature, per-realm implement and gates, a join before done —
   the recorded heritage shape) build on this map when ruled.

## Consequences

Multi-project use stops being a convention and becomes an argument:
`--realms clientX.json` opens that client's whole world — fleet, TUI,
raven — with zero further flags. The single-project case pays nothing.
And because the map is pinned and embedded, worlds are evidence like
everything else: a run can always testify about the tree it grew in.
