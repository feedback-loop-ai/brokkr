# 0022 — Reforging: the graph gets its way back into the fire

Status: accepted — operator ruled 2026-09-01
Date: 2026-09-01

## Context

The review phase's security rule is a severity-blind boolean:
`has_security_residual: true` → stop, "regardless of severity." With
reviewers thorough enough to always find *something* on security-adjacent
work, stop became the default ending — three consecutive substantive runs
died on low- and info-tier notes with fixes already applied. The failure
mode is not the stopping but what it causes: every stopped-but-good diff
lands through an operator-driven PR, *bypassing the ship phase entirely*,
so the rule meant to guarantee eyes-before-ship now mostly guarantees
ship-without-the-run. The wager proved it twice in one afternoon: two
rival crews delivered judged, verified artifacts and both were guillotined
on notes they were never allowed to answer.

The operator's ruling: the graph should have a way back — journal the
finding, return to implement, re-verify, re-review, bounded.

In the myth, the flaw shipped because the work never got back into the
hearth: the fly bit, the bellows paused, and Mjölnir's handle came out
short with no second heat. The remediation loop is the machine getting
the chance Brokkr never did — reheat, restrike, re-judge, before anything
is called finished or failed.

## Decision

1. **The back-edge.** A review ruling that records a security residual
   sends the run BACK to implement — any severity, high and critical
   included — carrying the finding: the review effect's result (findings,
   severities, notes) reaches the returning implement seat as declared
   input under 0007's provenance. The forward path then reruns as itself:
   implement → verify → review. No new phases; the graph already draws
   revisits.

2. **Bounded, in the machine's own vocabulary.** The rule table gains a
   phase-visit predicate (the fold already counts visits — the graph's
   `×N` markers are the proof), and the back-edge fires only while
   implement's returns stay within the bound: **two reforgings**,
   0006's temperament at phase scale.

3. **The terminal ladder, when the loop exhausts:**
   - still **above medium** → `stop` — the machine tried; now it is the
     operator's.
   - still **medium** → the run **parks awaiting the operator**, the
     residual as its reason: your ruling lands inside the run's journal
     instead of after its death. If the recorded vocabulary cannot yet
     express a rule-driven park, the implementing slice extends it
     honestly — never by mislabeling a stop.
   - still **low/info with fixes applied** → **ship as tracked debt**,
     flagged and named — the ledger Muninn already patrols.
   - still **low/info, unfixed** → park, the same operator door as
     medium.

4. **A clean re-review ships.** The remediation arc — finding, return,
   fix, re-verify, re-review — is one journal's story, and a run that
   answers its findings reaches ship with the whole arc as evidence.

5. **Scope and symmetry.** This decision moves the SECURITY rules of
   every bundle and recipe sharing the verify/review/ship constitution
   (self, fast, sdd family, panel-review). Non-security residuals keep
   today's rules; extending reforging to them is a future ruling, not a
   silent side effect. Policy digests move because policy moved — the
   witness pins are re-recorded as the identity change they are.

6. **Judges report; smiths fix.** With a real remediation path, a review
   seat's `fixes_applied` habit loses its excuse: the review's craft is
   the precision of the finding, because a precise finding is exactly
   what the returning implement seat receives.

## Consequences

Stop regains its meaning — it marks work the machine could not save
within bounds, not work a diligent reviewer annotated. The operator's
manual-PR escape hatch returns to being an escape hatch. Runs get more
expensive when findings are real (a reforging is a real implement pass),
which is the cost of the machine finishing its own arcs — and cheaper in
operator time by the same amount. The bound is the guard against the
tail risk: a reviewer and an implementer disagreeing forever now costs
at most two extra heats before a human holds the piece.
