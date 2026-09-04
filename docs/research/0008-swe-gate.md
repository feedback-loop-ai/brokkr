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
| 1 | Treat acceptance as two separately checkable dimensions, functional correctness and review-derived constraints, and report the gap between them | alternative | decision 0041: acceptance is a judging gate seat that reads the tree and the spec, not a per-constraint executable test suite |
| 2 | Give the worker the acceptance constraints in its input: explicit constraint guidance raises compliance without hurting functional success | implemented | decision 0042: the specify and design artifacts are written before the work and read from the journal by the seat that folds the change |
| 3 | Catalogue acceptance constraints by category (error semantics, compatibility, ordering, encoding, idempotence) and track which categories agents fail | not-planned | |
| 4 | Report the hidden-failure rate, the share of work that passes functional checks but fails acceptance, as a standing evaluation metric | not-planned | |

## Candidates

Finding 3: the paper's eleven categories are a ready checklist for
what a functional pass is not; they could sharpen
`agents/charters/review-chief.md`. Finding 4: Brokkr's journal
already holds both the verify evidence and the gate verdicts, so
muninn could compute the gap between verify-pass and ship-verdict as
a home-grown hidden-failure rate without any new machinery.
