# 0001 — What Survives the Next Model? Benchmarking LLM-Based Techniques Against Single-Prompts

Source: https://arxiv.org/abs/2609.00468
Authors: Salsabil, Saha, Dristi, Phair, Mozumder, Dwyer, Elbaum (University of Virginia)
Read: 2026-09-04
Status: proposed
Intake: chat session, operator-fed, 2026-09-04

## Summary

The paper asks whether freshly published LLM-based software engineering
techniques survive the next frontier model. It filters the 321 ICSE
2026 papers to 47 automatable, artifact-bearing techniques, samples 35,
and for each generates two single prompts with a meta-prompt: a
black-box prompt (role, task, input, output, examples, instructions)
and a white-box prompt that adds the paper's own steps. One call per
input to Claude Sonnet 4.6, with a fifteen-dollar budget per paper and
dataset coverage between 5 and 100 percent, scored by paper-specific
evaluators that replicate the original metric and ground truth.

Result: 13 of 35 techniques are strictly beaten by a single prompt, 9
are mixed, 13 are better than the prompt. By task, code generation
loses to the prompt in 6 of 9 cases, repair in 3 of 6, bug finding in 1
of 11, impact analysis in 0 of 2. By strategy, search-and-selection
pipelines lose 3 of 3 and structured-reasoning pipelines 5 of 9, while
knowledge-grounding pipelines hold 7 of 8. The white-box prompt cost
about 80 percent more tokens than the black-box one and won no more
often. The paper reports trends only; there is no significance
testing, and the authors say so. Cost appears only as the cap and as
the token cost of the two prompt forms against each other: the cost
of the techniques the prompts are said to beat is never reported, so
no row of the paper compares cost per outcome.

The operator's reading on 2026-09-04, recorded as the reason for the
first two rows: the findings are goal-dependent, and the paper's
single prompts are not the absence of a method but a mini-graph
written in prose, a plan the model is asked to execute in one call.
A stochastic mini-graph rots as the context bloats, and its execution
cannot be guaranteed; the operator has watched that happen. That is
the argument decision 0002 already makes for the outermost loop: the
sequence is a signed table the engine evaluates, and the model's
freedom lives inside each leaf. One paper, one model, no significance
testing and heterogeneous evaluators do not overturn it. The product
is to be very suspicious of this paper's headline until it is
replicated.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Before a multi-step pipeline is landed, run an auto-generated single prompt on the same evaluation and require the pipeline to beat it | declined | decision 0002: the outer loop is a table the engine evaluates, not a plan a model is asked to keep; a single prompt is a mini-graph whose execution is not guaranteed |
| 2 | Ablate a black-box prompt against a white-box prompt that encodes the method, to tell whether the method adds value or only prompt text | declined | decision 0002: the method here is the signed table, and its value is that it is not prompt text |
| 3 | Evaluate under a fixed budget and record the coverage that budget bought as a first-class field | alternative | decision 0034 and decision 0035: per-seat attempts, deadlines, cost and reasoning tokens are recorded in the seat record; no coverage field |
| 4 | Treat a frontier model release as an event that re-runs every baseline and retires scaffolding that lost its margin | alternative | decision 0031: the served model is recorded per seat; `crates/brokkr-runtime/tests/witness_digests.rs` moves when a hire changes; no re-baselining |
| 5 | Keep an evaluator per technique with a cross-review checklist (dataset, question alignment, metric match, example validity, evaluator correctness) | not-planned | |

## Candidates

None proposed. Should the headline be replicated across models and
with significance testing, finding 1 would become a recipe beside the
wager harnesses: the same commission, one arm a single prompt, judged
by the same gates. The operator asked for the replication first.
