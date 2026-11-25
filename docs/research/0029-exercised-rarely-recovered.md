# 0029 — Exercised, Rarely Recovered: What Public Reports Say About Recovery in 50 AI Agent Harnesses and Runtimes

Source: https://arxiv.org/abs/2611.27090
Authors: Kaviru Hapuarachchi (University of Colombo School of Computing)
Read: 2026-11-24
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

Do the recovery mechanisms of agent systems work in real use, and do
the guarantees research runtimes promise (saga compensation,
transactional rollback) reach practice? The paper grades public
evidence for 50 systems under a protocol frozen and hashed before
discovery, with 21 departures logged: 12 open-source persistent
runtimes carried over from a predecessor study, and 26 coding-agent
harnesses and 12 research runtimes chosen from 414 candidates. Eight mechanisms are coded: checkpoint and resume,
retry and timeout, rollback, compensation, validation gates, human
approval, sandboxing and trace logs. Each system gets an evidence level
per mechanism; the top level is an operator-attributable report of the
mechanism used in a real run, classed recovered, partial, failed or
unclear. Two LLM coding passes on different models were adjudicated
blind and the author checked the final labels (kappa 0.80 on mechanism
levels).

Recovery is exercised almost everywhere: 11 of 12 runtimes and 25 of 26
harnesses have at least one real-run report. It rarely completes. Of 128
such reports, 19 recovered, 54 were partial, 45 failed and 10 are
unclear. Checkpoint and resume, the most exercised mechanism, recovered
in 1 of 27 reports and failed in 13; typical failures are a session that
needs stale locks deleted by hand, or that resumes without its settings.
Rollback recovered in 1 of 15; one undo discarded the user's uncommitted
work (Aider), and one reverted the main agent's edits but not its
sub-agents' (Kilo Code). Human approval recovered 7 times and failed 9,
one "allow always" having authorised destruction. Trace logs recovered 7
of 25 and failed outright in none. On the research side 11 of 12
research runtimes are evidenced only on benchmarks, and none of 56
stated guarantees is shown in operation. No harness exposes
compensation; of 38 practice systems 2 show it (Langflow and Letta),
both as framework clean-up of their own writes. No harness measures its
own recovery.

Limits stated: LLM coders with no second human coder, a bounded search
where absence means not found, 98 of the 128 reports resting on GitHub
issues alone, and counts that are not failure rates. The paper has one
author. One of the research runtimes is co-authored by the author.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Hold resume and rollback to the testing that gates and sandboxes get: they are the mechanisms users rely on and report failing | implemented | decision 0002 and decision 0029; `crates/brokkr-runtime/src/engine/tests.rs`: state is the fold of an append-only journal, a resume folds it again and commits only onto the head it folded, and the engine's suite drives a moved head and a stale fold; one carve-out is open, a quiet attempt holder's own settlement is not fenced against a resumer's (#609) |
| 2 | Scope an undo so it cannot destroy work it did not make | alternative | decision 0022 and decision 0028: there is no rollback verb; a seat works on a branch in a worktree, a finding returns the run to the implementer instead of reverting it, and every commit the journal cites is kept under a ref only the operator deletes |
| 3 | Persist an event log, so a failed run can be diagnosed and repaired from it | implemented | decision 0002; `docs/guides/journal-and-verification.md`: every effect is requested in the hash-chained journal before it is dispatched, and decision 0108 repairs a broken run by a journaled record naming its late tail; parallel runs on one shared journal can corrupt it (#1000) |
| 4 | Make a resume restore the session's full state and clear a dead holder's lock without hand work | planned | #226 (session resumption for Claude and dsh; rests on decision 0056, still proposed); decision 0108: a run is held by an operating-system lock that dies with its driver, so no lock is deleted by hand |
| 5 | Do not rely on prevention alone for side effects: harnesses prevent them and none compensates them; make an external effect compensable or safe to repeat (the second is this entry's extension) | planned | #434 (decision 0006's 2026-10-11 addendum, rulings 3 and 4: a site's pinned "deduplicates" declaration and a stable effect identity, unbuilt); today an attempt that ends after start with no result read parks and is never replayed, and no compensation is offered |
| 6 | Scope an approval narrowly: a standing "allow always" is how an approval authorised destruction | alternative | decision 0043 and decision 0065: there is no interactive approval to widen; a seat's reach is the box its realm's boundary builds and the capabilities the realm lists, pinned in the manifest for the run |
| 7 | Measure your own recovery: say how often an interrupted run was resumed to its end | planned | #435 (operational reports from the journals, recovery outcomes included); decision 0064: every terminal run carries a typed ending, so the share of parks that resumed and shipped can be counted |

## Candidates

Finding 7. Brokkr has the paper's evidence at its best level for every
run it ever parked, in its own journal, and does not report it.
Decision 0064 counted 149 stopped runs by hand to make its case; the
same fold can say, per park cause (decision 0106, accepted, unbuilt),
how many parks were resumed to a ship, how many were concluded and
re-driven as a new run, and how many were abandoned. The paper's
headline (19 of 128 recovered) is the figure a harness should be able
to state about itself, and none of the 50 does.

Finding 5 is a standing gap rather than a new one: decision 0006's
addendum parks what it cannot prove, which is prevention. The paper
finds compensation nowhere in practice either, so this is the field's
gap too; nothing here argues for building it ahead of need.

The checkpoint failures the paper catalogues (stale locks, a resume
that loses settings) are the shape of problem decision 0108's lock and
decision 0056's re-imposed restrictions were ruled to remove. This
entry is evidence those rulings were aimed at the right failures; it
does not show they hold under an unclean host death, which only
Brokkr's own measurement can.
