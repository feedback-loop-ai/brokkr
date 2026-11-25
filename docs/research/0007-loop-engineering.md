# 0007 — Loop Engineering: Building Blocks, Adoption, and Impact

Source: https://arxiv.org/abs/2608.21884
Authors: Lulla, Treude (Singapore Management University); Nersesyan, Mohsenimofidi, Baltes (Heidelberg University)
Read: 2026-09-04
Status: ruled (2026-11-24)
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
| 1 | Use the eight building blocks as a checklist: trigger, stop condition, durable state, intent, isolation, independent checker, escalation, budget | implemented | decision 0006: bounded attempts and deadlines; decision 0002: the journal is the state and the table is the stop condition; decision 0043: the box, around a seat that declares hands in a namespace realm and nowhere else; decision 0021: the checker is a gate seat; decision 0001: escalation is a park |
| 2 | Commit stop conditions and budgets to the repository as machine-checkable artifacts | implemented | `policy/phase-machine.json` and each recipe's policy table (`recipes/fast/policy.json`) carry the terminal rules; each recipe's `recipes/fast/bundle.json` carries the per-seat attempts and deadlines |
| 3 | Emit a cost log per run: tokens and wall clock | implemented | decision 0034 and decision 0035: `contracts/seat-record.v2.schema.json` records usage, cost, effort and reasoning tokens per seat |
| 4 | Guard against verifier theater by keeping the checker a separate seat with its own evidence | implemented | decision 0041: a gate hires judges only and is chartered to change nothing, and the engine parks a gate that moves HEAD, though an uncommitted write passes that check; decision 0043: the verifier and shipper are pinned scripts, boxed under the namespace boundary and unboxed under harness |
| 5 | Track agent-authored edits to the loop's own configuration as a distinct signal | not-planned | |
| 6 | Evaluate autonomy levels against each other on throughput, correctness, cost and human effort | not-planned | |

## Erratum — 2026-11-25

The operator ruled this erratum on 2026-11-25, "approve all the fact and
wording fixes" for row 5 and the summary; "0007-1 approved" for row 1;
"0007 - row 6 - approved, but only if this can be done in an automated
way", then "that's not use case for us" and "the rest can be done" for
row 6 ([#1350](https://github.com/feedback-loop-ai/brokkr/issues/1350)),
after every row of this entry was checked three ways: against the
article, against main at `4912e583e`, and against the open issues and
pull requests. The text above stands as ruled on 2026-11-24 and stays
readable; where this section differs, this section is the product's
word.

- **Source.** For "https://arxiv.org/abs/2608.21884" read
  "https://arxiv.org/abs/2608.21884v1".
- **Summary.** For "The paper consolidates a definition from eleven
  gray-literature sources, measures real adoption by mining
  repositories, and pre-registers a controlled experiment." read "The
  paper consolidates a definition from eleven gray-literature sources,
  measures real adoption by mining repositories, and outlines a planned
  controlled experiment."
- **Summary.** For "Thirty repositories have agent-majority commits on
  their own loop configuration; twelve loops are disabled but still
  present." read "Thirty repositories pair an agent-majority context
  file (AGENTS.md or CLAUDE.md) with a committed loop artifact; twelve
  loops are disabled but still present."
- **Summary.** For "The sharpest finding is that the practice's
  rhetoric, budgets, verifiers, state and stop conditions, is nearly
  absent from its artifacts." read "The sharpest finding is that the
  practice's rhetoric is barely visible in its artifacts: fixed-phrase
  heuristics found no stop conditions, budgets or verifiers, manual
  annotation found a goal or stop-condition mechanism in 55 of 256
  inspected repositories, and almost none commits its state files."
- **Row 1** keeps `implemented`. Citation: decision 0006: bounded
  attempts and deadlines; decision 0002: the journal is the state and
  the table is the stop condition; decision 0043: the box, around a seat
  that declares hands in a namespace realm and nowhere else; decision
  0021: the checker is a gate seat; decision 0001: escalation is a park;
  the budget is attempts and deadlines only: decision 0067's enforced
  visit budgets are unbuilt (#463), and cost, stagnation and
  elapsed-time budgets are #604
- **Row 5** keeps `not-planned`. Finding: "Track agent-authored edits to
  the loop's own configuration as a distinct signal (the paper measures
  agent-majority context files beside loop artifacts; tracking edits to
  the loop's configuration is the analogous signal for Brokkr)".
- **Row 6** now reads `planned`, not `not-planned`, taken up only in an
  automated form. Citation: #1386, under #436. Brokkr's own autonomy
  levels are recipe variants that differ only in where a run stops for
  the operator. Each is run on the same commission through the wager
  pattern (`recipes/wager-harness/README.md`) and read with `brokkr runs
  compare`, on four axes: elapsed time, accepted results, cost (Claude
  seats only until #765), and the operator commands each run's journal
  records. Interactive prompting is not one of the levels: Brokkr has no
  workflow for it, since every seat runs headless, and it is not a use
  case for Brokkr.

## Candidates

Brokkr is a positive example the paper's detectors would have missed:
every building block it found absent is a committed artifact here. That
is an essay. Finding 5 is cheap: the journal already knows which seat
authored which commit, and a tally of edits to `recipes/` and `agents/`
by seats would answer the paper's self-modification question for this
repository.
