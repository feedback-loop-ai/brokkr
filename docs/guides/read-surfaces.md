# The read surfaces

Every readout shares ONE derivation (decision 0013): `brokkr-view` turns
a journal into view models, and each surface only renders them — so
"what did this seat cost" has a single answer, tested once. `runs`,
`inspect` and `watch` each take `--json` to emit that model verbatim.

`--run` takes a **selector**, not only the 41-character id: any unique
run-id prefix, or `latest` for the newest run in the workspace database
(decision 0015) — one resolver, shared by `watch`, `inspect`, `anchor`,
`export` and `replay`.

Colour follows `NO_COLOR` and `TERM`, width follows `COLUMNS`; without a
Unicode-width dependency, CJK and emoji columns misalign — stated rather
than pretended away.

`--realms <file>` chooses the **world** these surfaces read (decision
0023): a map of repositories and the journal they share, defaulting to
`./realms.json` when there is one. `--db` is retained and outranks the
map's journal; with neither, the journal is `.forge/forge.db` exactly as
it always was — a world that never drew a map notices nothing. A map
found rather than typed is still adopted, but not silently: when it sends
the journal somewhere other than the path that would have been opened
anyway, the surface names that journal on stderr before opening it.

A map may name **many hearths** (`forge.realms/v2`, decision 0026): each
realm may carry its own `journal`, and a realm that carries none falls
back to the world's — which is what every `forge.realms/v1` realm already
does, so a v1 map loads and reads exactly as it always has. Where a world
holds more than one journal, the fleet readers show it grouped: `runs`
prints one section per realm, `tui` grows a tab bar in the runs pane
(`[`, `]` and the number keys switch; each tab keeps its own selection,
filter and cursor), and `muninn run` reads every hearth and cites the
realm behind every fact. A realm the map names before its first run has
no journal yet, and that is an empty hearth rather than a fault: `runs`
lists it with no runs and says why, the console's tab shows an empty
fleet and says the same, and `muninn run` states the realm and reads the
rest of the world. Journals never merge: nothing folds across a journal
boundary, and no fleet read creates or writes one. Single-run verbs are
unchanged — a run id lives in exactly one journal, so naming it is a
lookup, never a merge, and a selector that matches a run in several
realms is refused by name rather than answered with the first (`latest`
means the newest run in the world, and the recorded stamp decides it).

Phase 1 wires the flag into `run` and the read surfaces the ruling names
— `runs`, `realms`, `tui`, `watch`, `inspect`, `export`, `muninn run` —
and, since decision 0047, `operator supersede`, whose citation may name a
run in another hearth. The others (`resume`, `conclude`, `rerun`,
`doctor`, `ui`, `costs`,
`compare`, `anchor`, `bridge`) still take `--db` alone, so a run started
in a world whose map names a journal other than `.forge/forge.db` is
resumed by naming that journal with `--db`.

### `brokkr realms` — the world

Every realm with its path, default branch and current HEAD, and the
journal the world writes. Read-only, like every other readout, and
`--json` emits the same derivation for scripts.

```
$ brokkr realms
map      ./realms.json
journal  ./.forge/forge.db
realm    brokkr  .  main  5a4bf4a28558d123c432d8992cfd9f13ffd81eb7
```

### `brokkr runs` — the fleet

One clamped line per run, newest first.

```
$ brokkr runs
prefix-selectors-for-the-read-su-8bf6d692 completed done seq 38 3s prefix selec…
```

In a world of many hearths, the same listing is grouped by realm — each
section naming the journal it was read from, and no run crossing into
another realm's section.

```
$ brokkr runs
alpha · 1 run · ./a/.forge/forge.db
many-hearths-8bf6d692 running implement seq 12 3s many hearths…

beta · 1 run · ./b/.forge/forge.db
the-other-slice-1a2b3c4d completed done seq 40 9m the other sli…
```

### `brokkr inspect` — one run, explained

Header, ruling, seats, decision trail, and the phase graph as a tree.
`--phase` and `--seat` are the scoping verbs the console's clicks
became.

```
$ brokkr inspect --run latest
run  prefix-selectors-for-the-read-su-8bf6d692
     completed · phase done · seq 38
ruling  SHIP-COMPLETE  ship → done · shipped

seats
  participant status    attempts turns cost activity
  intake      succeeded 1        —     —    resolved · 0s
  implement   succeeded 1        —     —    complete · 0s
  verify      succeeded 1        —     —    pass · 0s
  review      succeeded 1        —     —    clean · 0s
  ship        succeeded 1        —     —    shipped · 0s

trail
   1 run/started        prefix selectors for the read surfaces…
   2 phase/entered      intake
   7 effect/succeeded   intake · resolved
   8 transition/decided INTAKE-OK intake → implement · resolved
   9 phase/entered      implement
  14 effect/succeeded   implement · complete
  15 transition/decided IMPL-OK implement → verify · complete
  16 phase/entered      verify
  21 effect/succeeded   verify · pass
  22 transition/decided VERIFY-PASS verify → review · pass
  23 phase/entered      review
  28 effect/succeeded   review · clean
  29 transition/decided REVIEW-CLEAN-NO-FIXES review → ship · clean
  30 phase/entered      ship
  35 effect/succeeded   ship · shipped
  36 transition/decided SHIP-COMPLETE ship → done · shipped
  37 phase/entered      done
  38 run/completed      completed

graph
  intake ×1
    → intake · finished
  implement ×1
    → implement · finished
  verify ×1
    → verify · finished
  review ×1
    → review · finished
  ship ×1
    → ship · finished
  done ×1  ←current
```

Every line above is a rule id and a journal sequence number: the run
states which rule fired, from where, on which typed result. Nothing in
that trail was written by a model.

The graph also shows the **way back**. When a review finds a security
residual, decision 0022 sends the run back to implement — and the
record shows it, visits counted per phase. A real one, from the run
that implemented the model-policy refusals (reforged twice, then
lawfully stopped by its own third review):

```
graph
  intake ×1
  implement ×3
  verify ×3
  review ×3
  stop ×1  ←current
```

In `brokkr tui` the same journal draws the return as a solid arc under
the span with a mirrored arrowhead (`╰ᐸ╯`) at the landing phase —
drawn only when a return was actually taken, never as decoration.

### `brokkr watch` — the same, live

The same readout, redrawn whenever the journal head moves, exiting when
the run reaches a terminal status. Read-only, like every other readout.

```
$ brokkr watch --run latest
── 2026-08-29T22:27:40.474827119Z ──
run  prefix-selectors-for-the-read-su-8bf6d692
     completed · phase done · seq 38
ruling  SHIP-COMPLETE  ship → done · shipped

seats
  participant status    attempts turns cost activity
  intake      succeeded 1        —     —    resolved · 0s
  implement   succeeded 1        —     —    complete · 0s
  verify      succeeded 1        —     —    pass · 0s
  review      succeeded 1        —     —    clean · 0s
  ship        succeeded 1        —     —    shipped · 0s

graph
  intake ×1
    → intake · finished
  implement ×1
    → implement · finished
  verify ×1
    → verify · finished
  review ×1
    → review · finished
  ship ×1
    → ship · finished
  done ×1  ←current
```

### `brokkr tui` — the readouts made explorable

Decision 0014: arrow keys or `j`/`k` move, `Enter` descends from the run
list to a run to one seat's own stream, `Esc` comes back, `/` filters,
`?` opens help, and a footer names the keys of wherever you are. It is
read-only exactly as every other readout is — no operator commands, no
run starts, nothing written to the journal, and a missing database
refuses rather than creating one.

The fleet:

```
┌runs──────────────────────────────────────────────────────────────────────────────────────────┐
│id                       status    phase        seq    age      feature                       │
│prefix-selectors-for-the completed done         38     1m20s    prefix selectors for the read │
│                                                                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
runs
↑↓/jk move · Enter open run · g/G top/bottom · / filter · r refresh · ? help · q quit
```

`Enter` on a run — the phase rail, the seats, the trail, all three panes
of the same derivation, and the brand mark riding the graph pane's
border, its third rail node pulsing whenever the fleet is forging:

```
┌graph─────────────────────────────────────────────────────────────────────────[ ∙ ∙ ⏺ BROKKR ]┐
│                                                                                              │
│ ⏺ intake──ᐳ⏺ implement──ᐳ⏺ verify──ᐳ⏺ review──ᐳ⏺ ship───ᐳ∙                                   │
│ intake      implement    verify     review     ship     done                                 │
│                                                                                              │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
┌seats─────────────────────────────────────────────────────────────────────────────────────────┐
│participant            status        attempts turns  cost       activity                      │
│intake                 succeeded     1        —      —          resolved · 0s                 │
│implement              succeeded     1        —      —          complete · 0s                 │
│verify                 succeeded     1        —      —          pass · 0s                     │
│review                 succeeded     1        —      —          clean · 0s                    │
│ship                   succeeded     1        —      —          shipped · 0s                  │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
┌trail─────────────────────────────────────────────────────────────────────────────────────────┐
│1  run/started  prefix selectors for the read surfaces…                                       │
│2  phase/entered  intake                                                                      │
│7  effect/succeeded  intake · resolved                                                        │
│8  transition/decided  INTAKE-OK intake → implement · resolved                                │
│9  phase/entered  implement                                                                   │
│14  effect/succeeded  implement · complete                                                    │
└──────────────────────────────────────────────────────────────────────────────────────────────┘
runs · run prefix-selectors-for-the-read-su-8bf6d692
←→ rail · ↑↓ lanes · Enter scope phase · Tab pane · Esc back · / filter · r refresh · ? help · q
```

### `brokkr ui` — the browser console

`brokkr ui` serves an embedded, loopback-only, read-only surface on port
8383: runs, live seat activity, the causal event timeline. Same
derivation, same answers, a mouse instead of a keyboard.

```
$ brokkr ui --port 8383 --open
```

### `brokkr muninn` — the fleet, read and advised on

The read surfaces above show you the fleet. `brokkr muninn` reads it for
you and writes down what it would suggest — and then stops there.

One invocation opens the workspace database **read-only**, derives a
dossier from the same `brokkr-view` models every other readout uses (runs
with status, phase, age and cost; park reasons and the operator commands
each parked run admits; consecutive failures; the residual findings the
verify and review rulings recorded), and hands it to one bounded seat
under the driver fleet — a deadline, one attempt, no retry ladder. What
comes back is a fleet summary, a suggested operator command per parked
run with its reasoning, and the residual findings as a work queue.

Nothing it proposes is executed, and nothing here can execute it. Muninn
issues no operator command, starts no run, is given no repository tree
and no secrets, and writes to no run journal — proposals go to its own
append-only file, `.forge/muninn.ndjson`, beside the journal and inside
none of it. Every proposal names the run ids and sequence numbers it was
derived from; a report that cites a fact the dossier does not carry is
refused and recorded nowhere. Acting on any of it stays the operator's
own `brokkr operator` command (decision 0020).

A finding the operator has superseded (below) is still derived, still
listed and still cited — it carries the mark, and the fleet summary says
how many were closed — but it is not queued. Muninn cannot propose a
supersede: a run that has already finished admits no operator command,
and closing the record is the operator's act.

```
$ brokkr muninn run
2026-08-31T14:26:48Z · 1 proposals for parked runs · 2 findings queued
  summary: four runs; one is parked on a flaky verify, two are green
  parked prefix-selectors-8bf6d692 seq 41 · suggest 'retry' · the park
    reason names one test that has passed on re-run twice before
  queue review-the-lane-cursor-1f0a seq 33 · max_residual_severity: high
    · the only high residual in the fleet
  cites: prefix-selectors-8bf6d692 seq 41, review-the-lane-cursor-1f0a seq 33

$ brokkr muninn list          # every past invocation, citations included
```

### `brokkr operator` — the three commands that write

Everything above reads. `brokkr operator` is where an operator writes,
and it admits three commands. `retry` re-runs a parked run's phase and
`stop` ends a run where it stands; both are dispositions the engine acts
on, and both are refused — journaled as a refusal — when the run is not
in a state to take them.

`supersede` is the third, and it acts on nothing (decision 0047). It
records that residual findings on a run that has already **finished** —
completed or stopped — are closed by another run, naming the findings by
the sequence number of the ruling each was read from, and naming the run
and the ruling that closed them. Nothing derives that; only the operator
knows it, and only the operator may say it.

```
$ brokkr operator --run decision-0040-the-model-s-hands--96398324 supersede \
    --findings 190 \
    --by-run decision-0040-the-model-s-hands--415a7840 --by-seq 214 \
    --reason "both residuals fixed and shipped at 77c3099"
```

`--findings` takes one sequence number per flag or one comma-separated
list. `--by-realm` names the realm the superseding run was read in and is
omitted for the workspace journal, which is every one-hearth world;
`--realms` and `--db` choose the world the same way the read surfaces
choose it, because the run that closed a finding may live in another
hearth.

Every citation is checked before anything is written, and a refusal
writes nothing at all: the run must have finished, each `--findings`
sequence must be a residual finding that run actually carries, the
`--by-run` run must exist in the journal named and `--by-seq` must be a
`transition/decided` in it, and a run cannot supersede its own findings.
An annotation that passed those checks is never re-checked when it is
read — the journal it cites is append-only, so what was true stays true.

The annotation changes nothing else. It is one `operator/commanded`
event, folded as the no-op it is; the run's status, its park reason and
its ruling are exactly what they were, and the finding stays in the
journal and in every readout, marked:

```
$ brokkr inspect --run decision-0040-the-model-s-hands--96398324
run  decision-0040-the-model-s-hands--96398324
     stopped · phase review · seq 191
ruling  REVIEW-SECURITY-HOLD  review → ?
        security residual above the shipping bar
        superseded at seq 191 by decision-0040-the-model-s-hands--415a7840 seq 214 · operator · both residuals fixed and shipped at 77c3099
```

Un-superseding is not a command. The record is append-only, so a
mistaken annotation is answered by another `supersede` naming the right
run, or by a run that re-reviews the tree — and both stay on the record,
with a name on them.
