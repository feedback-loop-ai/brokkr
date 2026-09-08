# 0017 — SCX Router: Streaming Zero-Shot Model Selection with a Decoder-KV Classifier and a Real-World Task Ontology

Source: https://arxiv.org/abs/2609.02292
Authors: Ihor Stepanov, Mykhailo Shtopko, Dmytro Vodianytskyi, Oleksandr Lukashov (Knowledgator); Aleksandr Smechov (SCX.ai Holdings)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

A 0.6B-parameter router, a Qwen3 decoder with a shallow
bidirectional scorer in the GLiClass family, assigns per-task
suitability scores to candidate model endpoints without
autoregressive generation; its decoder-KV execution path preserves a
text-only key-value cache across a session, encodes only new turns,
and scores transient candidate labels without adding them to the
cache. The same checkpoint predicts task type, difficulty, reasoning
mode and expected output length, and supports custom zero-shot
labels. The task ontology holds 23 families, 115 task types, 345
routable subtypes, 1,173 synthetic examples and an orthogonal axis
of 30 domains, used to generate 150,000 verifier-scored and 15,000
judged synthetic tasks.

Label-decision metrics read F1 0.9241, precision 0.9238, recall
0.9273, but the authors note many negative labels dominate such
metrics and point to routing-family and downstream results as the
relevant evidence. Four routing patterns are distinguished: direct
endpoint routing, attribute-mediated performance routing, hybrid
constrained routing, and hierarchical planner-worker routing; all
first restrict candidates to an eligible set where context,
modality, tools, privacy, residency and safety remain hard
constraints. Only the direct path ships with end-to-end evidence;
the paper names which compositions are released, implemented or
merely proposed.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Route each task at runtime to the model that best trades speed, cost and quality | alternative | decision 0031 and decision 0041; `recipes/triage/bundle.json` and `crates/brokkr-runtime/src/agents.rs`: runtime strategy selection chooses a declared office and availability resolves its pinned chain; no suitability score trades quality, speed and cost |
| 2 | Restrict the candidate set by hard constraints, tools, privacy, residency, safety, before scoring suitability | alternative | decision 0021, decision 0036 and decision 0046; `crates/brokkr-runtime/src/bundle.rs`: compilation constrains trust, judge eligibility, route egress and boundary capabilities; this is not a complete per-task context/modality/residency filter, and decision 0025 supplies no implemented signed-grant ceiling |
| 3 | Distinguish released, implemented and proposed compositions in every routing claim | alternative | decision 0046 and decision 0049; `crates/brokkr-runtime/src/engine.rs` and `crates/brokkr-cli/src/doctor.rs`: unbuilt boundaries refuse at start and doctor names them; decision acceptance alone does not certify an implementation or release |
| 4 | Evaluate routing on downstream task outcomes, because label-level classifier metrics are dominated by negative labels | alternative | `crates/brokkr-cli/src/compare.rs`: run comparison exposes verdict trails, model resolution, boundary, cost and attempts; it does not independently score downstream task success or validate a router against ground truth |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

The current tree already routes a ruled strategy to declared seats and
uses explicit availability fallbacks. It does not run a learned model
selector, and route egress policy does not enforce every hard constraint
in SCX's candidate filter. The boundary enactment demonstrates why an
accepted decision is not a release claim: namespace, harness and open
are built; seatbelt and container remain refused.

## Candidates

A learned suitability selector and task ontology remain possible future
work. They would extend strategy-based selection, not introduce routing
where none exists. Adoption would need downstream evaluation and an
explicit ruling about which hard constraints the candidate filter can
actually enforce; the existing compare command supplies run evidence,
not that evaluation.

[Issue #224](https://github.com/feedback-loop-ai/brokkr/issues/224)
already proposes selection by remaining uncertainty, accepted-change cost
and current capacity. It explicitly leaves the new model policy unaccepted;
it is not adoption of SCX's learned router. [Issue #237](https://github.com/feedback-loop-ai/brokkr/issues/237)
tracks the proposed controlled wagers that could supply comparison evidence.
