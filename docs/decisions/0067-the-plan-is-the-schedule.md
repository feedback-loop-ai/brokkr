# 0067 — The plan is the schedule: implement runs the task graph in bounded visits, and every visit has a budget the engine enforces

Status: accepted (operator ruled in chat, 2026-09-24)
Date: 2026-09-23

## Context

A run's tasks phase writes a plan — ordered tasks, each with the files it
touches and the proof it owes — and the implement phase then ignores it. The
engine hands the whole change to one implement seat, and that seat carries
every earlier task's context into every later one until it finishes, parks,
or hits a wall-clock limit. Nothing inside a visit is scheduled, bounded or
priced by the engine.

On 2026-09-22 the operator asked why one implementation run had spent 250
million tokens. The journal answered it exactly. Run
`build-decision-0065-slice-one-th-80bfd784` repaired the second council hold
on decision 0065's first slice — a 43-task change — in one implement session:

| | |
|---|---|
| turns in the session | 677, over 86 minutes |
| context at the first turn | 10,236 tokens |
| context at the latest turn | 605,072 tokens |
| input across the session | 257,291,558 tokens, all but 1,354 read from cache |

Every turn re-sends the whole conversation, so the price of a turn grows with
everything before it: the late turns each paid about 600,000 tokens to run one
`cargo` command. The harness never compacted, because the model's window is a
million tokens. The work was sound — edits, checks, tests, a commit per group —
and that is the point: the cost was not the model misbehaving, it was the
engine giving one visit a job the plan had already divided into forty-three.

The same shape shows up at the gates. The slice reached its first full council
as about 15,000 lines across 81 files and was held twice on security findings;
each repair was again a whole-branch visit. A finding caught at the third task
group would have been cheap. Caught at the end, it re-bought the whole slice.

The one guard the phase machine has is `IMPL-OVERSIZED`: a smith may report
`oversized` and the run returns to triage. It depends on the smith noticing,
and it fired six times in the journal's history. Decision 0006 bounds a seat by
attempts and wall-clock seconds; it says nothing about tokens, turns or
context, because when it was written a seat was one short session.

The operator ruled the direction on 2026-09-23: this is about controlling the
workflow — tasks, parallelism, how the work is split — and compaction only as
one instrument. Scheduling runs against each other is a separate decision
(0068).

## Rulings

1. **The tasks artifact is a typed graph, and the dialect declares its
   shape.** Each task carries an id, the ids it depends on, the paths or areas
   it may touch, its acceptance proof, and a size class. The dialect contract
   gains a `tasks` shape (a new `dialect` contract version beside v3; v3 keeps
   loading and means "no graph"), and the analyze gate refuses a plan whose
   graph has a cycle, a dangling dependency, or a task with no proof. The
   graph is the plan's own data, written by the tasks seat and judged by
   analyze; the engine reads it and never writes it.

2. **Implement runs the graph as a series of visits, one task group at a
   time.** The engine groups ready tasks — dependencies met — into a visit by
   the plan's own grouping, or by size class where the plan names none, and
   commissions one fresh implement session per visit. A visit's input is the
   branch as it stands, the specification slice for its tasks, and the
   previous visit's handoff note; it is never the previous visit's
   conversation. A visit ends with a commit and a result per task:
   `complete`, `blocked` or `broken`. The implement phase completes when every
   task is `complete`; a `blocked` or `broken` task takes the existing rules.

3. **Parallel only where the graph proves it safe.** Two ready groups whose
   declared paths are disjoint may run at once, each in its own worktree off
   the same base, and the engine integrates them in graph order; a conflict on
   integration is a `broken` result on the later group, never a guess.
   Overlapping groups run in sequence. Whether a recipe allows parallel visits
   at all, and how many, is recipe data, default one.

4. **Every visit has a budget, and the engine enforces it without killing
   work.** A seat declares ceilings beside decision 0006's limits: tokens,
   turns and context. Crossing a ceiling does not kill the process. The engine
   asks the seat, through the result contract, to write its handoff note and
   end at the next safe point — after a commit, never mid-edit — and the next
   visit continues from the note. Only decision 0006's wall-clock deadline
   still kills. The ceilings are the same on every provider, whatever each
   harness does about compaction; a harness's own compaction is allowed and
   is not relied on. A handoff forced by a budget is journaled as such, with
   the figures that crossed.

5. **Judgment is incremental, and the council stays at the end.** Each
   visit's commit passes a crate-scoped verify gate before the next visit
   starts, and a recipe may add a light review per visit. The full review
   council still judges the whole change once, at the end, unchanged. A
   finding at a visit gate returns to that visit's tasks, not to the whole
   implement phase.

6. **The size guard becomes the engine's, not the smith's.** A plan whose
   total size class exceeds the recipe's ceiling is refused at analyze and
   returns to tasks to be split, before any implement visit is paid for.
   `IMPL-OVERSIZED` remains for a smith that discovers mid-visit that its task
   was larger than planned.

7. **Everything above is visible and priced.** The run manifest pins the
   graph's digest; each visit is an effect with its task ids, its budget, its
   figures and how it ended; the read surfaces show the graph with each task's
   state; and any ledger drawn from the journal can price a visit and a task,
   not only a phase.

## What this decision does not do

It does not schedule runs against each other: that is decision 0068. It does
not change the review council, the phase order, or decision 0006's attempt and
deadline semantics. It does not require a dialect to adopt the graph: a dialect
without the `tasks` shape runs implement as one visit, exactly as today, and
says so.

## Consequences

The 43-task repair above becomes roughly as many visits as it has task groups,
each starting near 50,000 tokens of context instead of growing to 605,000, and
the council's first findings arrive a group in rather than at 15,000 lines.
The tasks seat and the analyze gate carry more weight, because the plan now
drives the work instead of describing it. Implement costs more effects and
fewer tokens. Recipes gain three new knobs — parallel visits, budget ceilings,
a size ceiling — each with a default that reproduces today's behaviour except
the budgets, which are new.

Amends decision 0006 by adding ceilings beside its limits, and the dialect
contract by a new version. Extends `IMPL-OVERSIZED` without removing it.

## Evidence

Counted from the-forge's journal on 2026-09-23: the session figures above are
the `seat-turn` checkpoints of run `build-decision-0065-slice-one-th-80bfd784`
after sequence 725, deduplicated by turn. The council holds are runs
`review-first-full-council-verify-3c72a18a` and
`build-decision-0065-slice-one-re-25d222e6`. `IMPL-OVERSIZED` counted six
transitions across the journal. The counts will drift as the journal grows.
