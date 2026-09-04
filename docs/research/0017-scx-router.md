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
| 1 | Route each task at runtime to the model that best trades speed, cost and quality | alternative | decision 0031 and decision 0041: Brokkr pins the model per seat and hires one office per seat; who works is chosen by the recipe and the operator, not scored at runtime |
| 2 | Restrict the candidate set by hard constraints, tools, privacy, residency, safety, before scoring suitability | implemented | decision 0021 and decision 0036: trust tiers and egress classes are declared per route and refused at compile; a never-list caps what any grant may reach, decision 0025 |
| 3 | Distinguish released, implemented and proposed compositions in every routing claim | alternative | `docs/decisions/README.md`: the decision index separates proposed from accepted with a status line, and a declined proposal stays in the record |
| 4 | Evaluate routing on downstream task outcomes, because label-level classifier metrics are dominated by negative labels | alternative | decision 0010: recipes are compared by run id, that is, by what the runs produced, not by a classifier's label scores |

## Candidates

If the operator ever wants triage to suggest a hire rather than rule
a class, this paper is the field's current recipe for the classifier
half, and the 23-family, 115-type ontology is reusable structure.
Until then the row stays alternative: Brokkr's pinning is the
deliberate substitute.
