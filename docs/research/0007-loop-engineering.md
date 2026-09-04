# 0007 — Loop Engineering: Building Blocks, Adoption, and Impact

Source: https://arxiv.org/abs/2608.21884
Authors: Lulla, Treude (Singapore Management University); Nersesyan, Mohsenimofidi, Baltes (Heidelberg University)
Read: 2026-09-04
Status: proposed
Intake: chat session, operator-fed, 2026-09-04

## Summary

Loop engineering, the design of automated control structures that
invoke coding agents on schedules or events with machine-checkable stop
conditions, emerged as a named practice around June 2026 and has no
empirical validation. The paper consolidates a definition from eleven
gray-literature sources, measures real adoption by mining repositories,
and pre-registers a controlled experiment.

The building blocks: a trigger, a goal with a stop condition, durable
state, intent encoding, worktree isolation, independent maker and
checker verification, an escalation gate, and a budget with pause.

Adoption: of 36,645 engineered repositories, 217 confirmed loops, 0.59
percent, with heuristic precision 0.868. All but one trigger is a
GitHub Actions workflow: 35 schedule-only, 205 event-only, 13 both.
Stop-condition phrases: zero in 36,645 repositories. Budgets, verifier
subagents and cost logs: zero detected. State files: two matches, both
false positives. Tooling: Claude Code in 189 of 217 loops, Codex 11,
OpenCode 6. Thirty repositories have agent-majority commits on their
own loop configuration; twelve loops are disabled but still present.
Catalogued failure modes: runaway cost, about eight million tokens in
48 hours; verifier theater; self-reported benefit claims. The
sharpest finding is that the practice's rhetoric, budgets, verifiers,
state and stop conditions, is nearly absent from its artifacts.

The planned experiment compares interactive prompting, goal-driven runs
and scheduled loops on throughput, correctness, cost and human effort.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Use the eight building blocks as a checklist: trigger, stop condition, durable state, intent, isolation, independent checker, escalation, budget | implemented | decision 0006: bounded attempts and deadlines; decision 0002: the journal is the state and the table is the stop condition; decision 0043: the box; decision 0021: the checker is a gate seat; decision 0001: escalation is a park |
| 2 | Commit stop conditions and budgets to the repository as machine-checkable artifacts | implemented | `policy/phase-machine.json` and each recipe's policy table (`recipes/fast/policy.json`) carry the terminal rules; each recipe's `recipes/fast/bundle.json` carries the per-seat attempts and deadlines |
| 3 | Emit a cost log per run: tokens and wall clock | implemented | decision 0034 and decision 0035: `contracts/seat-record.v2.schema.json` records usage, cost, effort and reasoning tokens per seat |
| 4 | Guard against verifier theater by keeping the checker a separate seat with its own evidence | implemented | decision 0041: a gate hires judges only and changes nothing; decision 0043: the verifier and shipper are boxed scripts |
| 5 | Track agent-authored edits to the loop's own configuration as a distinct signal | not-planned | |
| 6 | Evaluate autonomy levels against each other on throughput, correctness, cost and human effort | not-planned | |

## Candidates

Brokkr is a positive example the paper's detectors would have missed:
every building block it found absent is a committed artifact here. That
is an essay. Finding 5 is cheap: the journal already knows which seat
authored which commit, and a tally of edits to `recipes/` and `agents/`
by seats would answer the paper's self-modification question for this
repository.
