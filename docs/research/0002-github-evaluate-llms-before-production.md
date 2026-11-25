# 0002 — How to evaluate LLMs before production

Source: https://github.blog/ai-and-ml/llms/how-to-evaluate-llms-before-production/
Authors: GitHub Secret Scanning team (GitHub)
Read: 2026-09-04
Status: ruled (2026-11-24)
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

## Erratum — 2026-11-25

The operator ruled this erratum on 2026-11-25, "0002, 0003, 0004 -
approved"
([#1350](https://github.com/feedback-loop-ai/brokkr/issues/1350)), after
every row of this entry was checked three ways: against the article,
against main at `4912e583e`, and against the open issues and pull
requests. The text above stands as ruled on 2026-11-24 and stays
readable; where this section differs, this section is the product's
word.

- **Authors.** For "GitHub Secret Scanning team (GitHub)" read "Mariko
  Wakabayashi, Zixiao Chen (Microsoft)".
- **Summary.** For "The workflow has eight steps in order: define the
  product decision first as one primary metric plus explicit guardrails;
  run the offline evaluation on every meaningful change with prompts and
  configs versioned like code; keep the evaluation set close to
  production; treat production labels as signals rather than ground
  truth and review disputed subsets by hand; add synthetic and open
  datasets for rare cases without replacing production data; categorise
  every failure by source (model, prompt, input, pipeline, dataset,
  label); use a model as a judge for triage, never for verdicts; then
  move to online experiments with documented risks." read "The workflow
  has seven steps in order: define the product decision first as one
  primary metric plus explicit guardrails; run the offline evaluation on
  every meaningful change with prompts and configs versioned like code;
  keep the evaluation set close to production; treat production labels
  as signals rather than ground truth and review disputed subsets by
  hand; add synthetic and open datasets for rare cases without replacing
  production data; categorise every failure by source (model, prompt,
  input, pipeline, dataset, label); and use a model as a judge to focus
  human review, its output treated as another prediction rather than
  ground truth. Passing them is what justifies moving to online
  experiments with documented risks."
- **Summary.** For "The advancement rule is the sharp part: a change
  advances only if the primary metric improves and every guardrail stays
  in range." read "The advancement rule is the sharp part: a change
  advances only if every guardrail stays in range (recall may fall only
  within a predefined acceptable range), and among the changes that
  qualify the one with the strongest false-positive reduction is
  chosen."
- **Row 1** keeps `alternative`. Finding: "Advance a change only when
  every guardrail holds, and choose among those by the primary metric; a
  large win that breaches a guardrail is rejected".
- **Row 2** now reads `alternative`, not `implemented`. Citation:
  decision 0017: charters, tables and adapters are digest-pinned bundle
  identity, so a change moves a witness in
  `crates/brokkr-runtime/tests/witness_digests.rs` and must be re-pinned
  on purpose; no evaluation is re-run on a change (#283 proposes a
  real-provider check after each upgrade)
- **Row 3** now reads `planned`, not `alternative`. Finding: "Keep the
  evaluation set and pipeline close to production, and treat production
  labels as signals, not ground truth". Citation: #271: the journal and
  transcripts become an evaluation corpus once outcomes are closed back
  into the journal, a verdict or a green landing being an input, not
  proof; #436 owns the versioned, held-out evaluation protocol; until
  then decision 0021 keeps outcome scoring in the operator's ledger
- **Row 4** keeps `alternative`. Citation: decision 0064 and decision
  0106 (`contracts/park-cause.v1.schema.json`, slice 0106a): a terminal
  run's ending separates `infrastructure` from `failed` and `refused`,
  and a park's cause is typed `ruled`, `attempt-cap` (with a failure
  class: provider limit, provider refusal, start failure or
  determinate), `infrastructure` or `provider-limit`; decision 0006's
  determinate and indeterminate attempts stand beneath them; there is no
  model, prompt, input, dataset or label split; the rest of decision
  0106 is #1035
- **Row 7** keeps `implemented`. Citation: decision 0031 and decision
  0035: the run manifest pins bundle, adapter and agent digests; the
  seat record carries the served model and effort; not recorded: the
  operator's host configuration a claude seat loads (`CLAUDE.md`, user
  settings, hooks; `docs/security-model.md`, #467), and the harness
  version, which is read but checked against nothing (#1240)

## Candidates

Finding 4 is the cheapest of the seven to take up: a failure-source
tag on a park or stop would make the difference between a model
failure and an outage countable instead of remembered. Finding 5's
sampled audit of passes is the one piece of the judge pattern Brokkr
lacks.
