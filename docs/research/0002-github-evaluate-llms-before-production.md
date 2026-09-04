# 0002 — How to evaluate LLMs before production

Source: https://github.blog/ai-and-ml/llms/how-to-evaluate-llms-before-production/
Authors: GitHub Secret Scanning team (GitHub)
Read: 2026-09-04
Status: proposed
Intake: chat session, operator-fed, 2026-09-04

## Summary

A practitioner's account from the team that put a language model in
front of secret-scanning alerts. Its claim: a model that performs well
on a clean benchmark can still fail the cases that matter in
production, so evaluation exists to make production uncertainty
visible, measurable and manageable, and it should run like integration
testing rather than as a one-off benchmark.

The workflow has eight steps in order: define the product decision
first as one primary metric plus explicit guardrails; run the offline
evaluation on every meaningful change with prompts and configs
versioned like code; keep the evaluation set close to production;
treat production labels as signals rather than ground truth and review
disputed subsets by hand; add synthetic and open datasets for rare
cases without replacing production data; categorise every failure by
source (model, prompt, input, pipeline, dataset, label); use a model as
a judge for triage, never for verdicts; then move to online
experiments with documented risks.

The advancement rule is the sharp part: a change advances only if the
primary metric improves and every guardrail stays in range. Their case
cut false positives by 95 percent offline with recall held inside its
guardrail. Rigor rules: change one major variable at a time, compare
every run to a known baseline, record prompt version, model version,
dataset version and system configuration per run. The judge pattern:
auto-decide clear low-risk cases, route low-confidence or high-impact
cases to humans, sample high-confidence judge outputs for systematic
error, track three-way disagreement between judge, system and human,
and version the judge prompt as a model component.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Advance a change only when the primary metric improves and every guardrail holds; a large win that breaches a guardrail is rejected | alternative | decision 0002 and decision 0004: the phase table rules on typed verdicts and fails closed; there is no metric axis |
| 2 | Re-run the offline evaluation on every meaningful change, with prompts and configurations versioned like code | implemented | decision 0017: charters, tables and adapters are digest-pinned bundle identity; `crates/brokkr-runtime/tests/witness_digests.rs` |
| 3 | Build the evaluation set from production cases and treat production labels as signals, not ground truth | alternative | decision 0021: outcome scoring is placed outside the engine, in the operator's ledger; the journal is the production record |
| 4 | Categorise every failure by its source: model, prompt, input, pipeline, dataset or label | alternative | decision 0006: failures are determinate, indeterminate or result-driven; no source axis |
| 5 | A model judge triages: clear cases auto-decide, low-confidence and high-impact cases go to a human, high-confidence passes are sampled for audit, three-way disagreement is tracked | alternative | decision 0001 and decision 0021: a judge's verdict is a token the table rules on, an unmatched or invalid verdict parks for the operator; no sampling audit of passes |
| 6 | Change one major variable per experiment and compare every run to a known baseline | alternative | `recipes/wager-harness/README.md`: rival bundles differ by one line; `crates/brokkr-cli/src/compare.rs` folds two journals to their first divergence |
| 7 | Record the prompt version, model version, dataset version and system configuration per run | implemented | decision 0031 and decision 0035: the run manifest pins bundle, adapter and agent digests; the seat record carries the served model and effort |

## Candidates

Finding 4 is the cheapest of the seven to take up: a failure-source
tag on a park or stop would make the difference between a model
failure and an outage countable instead of remembered. Finding 5's
sampled audit of passes is the one piece of the judge pattern Brokkr
lacks.
