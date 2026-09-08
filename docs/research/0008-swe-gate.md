# 0008 — SWE-Gate: Passing Functional Tests Is Not Enough for Software Engineering Agents

Source: https://arxiv.org/abs/2609.04167
Authors: Xin He, Yanlin Wang, Mingwei Liu, Jiachi Chen, Hongyu Zhang, Guanbin Li (Sun Yat-sen University; Zhejiang University; Chongqing University)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

Repository-level software-engineering benchmarks score a patch by
whether it passes functional tests and ignore review-derived
acceptance constraints: the backward-compatibility, error-semantics
and convention requirements maintainers raise in review before
accepting. SWE-Gate mines such constraints from real pull-request
review comments and synthesizes repository-level repair instances
around them, each carrying separate functional and constraint tests
plus a non-compliant and a gold patch, so issue resolution and
constraint compliance are measured apart.

The benchmark holds 303 instances across 75 open-source Python
repositories. Four backends (GPT-5.5, GPT-5.4-mini,
DeepSeek-V4-Flash, GPT-4o-mini) run under one coding-agent scaffold.
Of 644 repairs that pass the functional tests, 221 fail the
constraint tests: a hidden-failure rate of 34.3 percent overall,
29.5 percent for GPT-5.5 up to 53.6 percent for GPT-4o-mini.
Functional-only evaluation therefore overestimates acceptability by
about a third. Providing the constraint text to the model raises
joint success for every model (360 to 423 joint passes; GPT-5.5
joint success 41.3 to 52.8 percent) and lifts constraint compliance
by 10.2 to 25.6 percentage points, while functional success does not
improve. Eleven constraint categories are tracked; error semantics
(152 instances) and schema/typing (143) are the most common, and
ordering/argument preservation and idempotence carry the lowest joint
success rates.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Treat acceptance as two separately checkable dimensions, functional correctness and review-derived constraints, and report the gap between them | alternative | decision 0041 and decision 0042; `recipes/triage/bundle.json`: deterministic verify and dialect-validation steps precede judging seats; functional and review verdicts are distinct, but there is no executable constraint taxonomy or hidden-failure metric |
| 2 | Give the worker the acceptance constraints in its input: explicit constraint guidance raises compliance without hurting functional success | implemented | decision 0042; `recipes/triage/bundle.json` and `agents/charters/implementer-sdd.md`: the SDD path validates specify, design and tasks artifacts, journals their change identifier, and directs the smith to implement against those artifacts |
| 3 | Catalogue acceptance constraints by category (error semantics, compatibility, ordering, encoding, idempotence) and track which categories agents fail | not-planned | |
| 4 | Report the hidden-failure rate, the share of work that passes functional checks but fails acceptance, as a standing evaluation metric | not-planned | |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

The SDD enactment now supplies framework validation and bounded clarify
and analyze loops before implementation. Finding 2 applies to that
path; it is not a promise that every recipe writes a spec. The journal
carries the change identifier and validation state, while the artifacts
live in the repository. Findings 3 and 4 remain gaps: no constraint-category
report or hidden-failure-rate calculation is implemented.

## Candidates

Findings 3 and 4 remain candidates for the operator: use the paper's
categories to sharpen the review charter, and define a metric over runs
that passed verify but failed review-derived acceptance. That metric
needs a denominator and treatment of retries, residuals and later
operator supersedes (decision 0047). Muninn's current dossier and report
do not compute it; journaled verdicts are inputs for new deterministic
aggregation, not an existing measure of real-world acceptance.

[Issue #237](https://github.com/feedback-loop-ai/brokkr/issues/237)
proposes cumulative wager accounting and reconciliation of the implemented
candidate against specs and constitution. That is related backlog work;
it has not added the paper's constraint taxonomy or hidden-failure metric.
