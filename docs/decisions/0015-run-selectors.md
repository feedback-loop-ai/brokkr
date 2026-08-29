# 0015 — Run selectors: a prefix or `latest`, resolved in one place

Status: proposed (implementer, 2026-08-29)

## Context

A run id is 41 characters — a feature slug plus a hash. Every readout
takes one: `forge watch --run …`, `inspect`, `anchor`, `export`,
`replay`. The moment an operator most wants a readout is the moment a
run has just started, and the id is a thing to be copied out of stderr
or hunted for in `forge runs` first. Typing it in full is friction
exactly where the tool should be quickest.

Two ways to spell "the run I mean" cost nothing to resolve and remove
almost all of that friction: an unambiguous prefix of the id, and the
word `latest`. The risk is not the feature but its shape — five
commands each growing their own resolution, drifting in what they
accept and in what they say when a selector matches two runs.

## Decision

**One resolver, in `forge-cli`, over the run list.**

1. `selector::resolve(runs, requested)` is a pure function over
   `[(run_id, created_at)]` plus the operator's string. It is testable
   without a database, and every rule below is a unit test.
2. `selector::resolve_run(store, requested)` is the only store-facing
   form: it reads the run table and calls the pure core. Every command
   that takes `--run` calls it; none re-implements a rule.
3. The rules:
   - `latest` is the run with the greatest `created_at`. On an empty
     database it is an error, not a panic and not an empty readout.
   - An exact id wins outright, before any prefix is considered. A run
     whose id is a prefix of another run's is therefore never
     ambiguous with itself.
   - A prefix matching exactly one run resolves to it.
   - A prefix matching several is an error that **names the
     candidates**: choosing one would be a guess about which run the
     operator meant.
   - A prefix matching none says "no run matching '<selector>'".
4. Resolution is a read. It touches the `runs` table and never appends
   a journal event — the readouts stay read-only (decision 0013).
5. Error text is sanitized through `render::Safe` like every other
   string that reaches a terminal: the selector is operator input and
   the candidate ids come from the database.

Scope: the five readouts named above. The write paths (`resume`,
`rerun`, `operator`, `bridge`) keep requiring the full id — abbreviating
the run you are about to *change* is the operator's call to make, not
an implementer's.

## Why

The alternative — resolution per command — is the same bug five times:
`watch` accepting a prefix that `export` rejects, or two different
messages for the same ambiguity. Making the core pure is what makes the
rules provable: "exact wins over prefix", "ambiguous names its
candidates" and "`latest` on an empty database errors" are assertions
over a list, with no temporary database and no run to drive.

`latest` shadows a run literally named `latest`, which a 41-character
generated id cannot be. Ordering by the recorded `created_at` string
rather than by the query's `ORDER BY` keeps "newest" a property of the
runs; it is the same string ordering `forge runs` already sorts by.

## Consequences

- `forge watch --run latest` after `forge run` is the common path, with
  no id copied anywhere.
- A prefix resolves once, before `watch` enters its poll loop: a run
  started while watching cannot silently change which run is on screen.
- `--json` shapes are unchanged; the resolved id is what every readout
  prints and what `export` names its files for.
