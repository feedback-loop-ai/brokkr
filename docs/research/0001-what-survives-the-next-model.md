# 0001 — What Survives the Next Model? Benchmarking LLM-Based Techniques Against Single-Prompts

Source: https://arxiv.org/abs/2609.00468
Authors: Salsabil, Saha, Dristi, Phair, Mozumder, Dwyer, Elbaum (University of Virginia)
Read: 2026-09-04
Status: ruled (2026-11-24)
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

## Erratum — 2026-11-25

The operator ruled this erratum on 2026-11-25, "approve all the fact and
wording fixes" for rows 1, 2, 3 and 4 and the summary
([#1350](https://github.com/feedback-loop-ai/brokkr/issues/1350)), after
every row of this entry was checked three ways: against the article,
against main at `4912e583e`, and against the open issues and pull
requests. The text above stands as ruled on 2026-11-24 and stays
readable; where this section differs, this section is the product's
word.

- **Summary.** For "By strategy, search-and-selection pipelines lose 3
  of 3 and structured-reasoning pipelines 5 of 9, while
  knowledge-grounding pipelines hold 7 of 8." read "By strategy,
  search-and-selection pipelines lose 3 of 3 and structured-reasoning
  pipelines 5 of 9, while knowledge-grounding pipelines lose only 1 of 8
  (4 better than the prompt, 3 mixed)."
- **Summary.** For "The white-box prompt cost about 80 percent more
  tokens than the black-box one and won no more often." read "The
  white-box prompt itself averaged about 80 percent more tokens than the
  black-box one (about $7.5 against $6.5 per paper in total spend) and
  won no more often."
- **Summary.** For "That is the argument decision 0002 already makes for
  the outermost loop: the sequence is a signed table the engine
  evaluates, and the model's freedom lives inside each leaf." read "That
  is the argument decision 0002 already makes for the outermost loop:
  the sequence is a digest-pinned table the engine evaluates, and the
  model's freedom lives inside each leaf."
- **Row 1** keeps `declined`. Finding: "Before a multi-step pipeline is
  landed, run an auto-generated single prompt on the same evaluation and
  require the pipeline to beat it (a gate drawn from the paper's method;
  the paper itself recommends no such gate)". Citation: decision 0002:
  the outer loop is a table the engine evaluates (`evaluate()` over an
  ordered first-match-wins rule list), not a plan a model is asked to
  keep; the operator's reading (2026-09-04) is that a single prompt is a
  mini-graph whose execution is not guaranteed; #436 and #439 plan a
  simpler-baseline comparison as qualification, not as a landing gate
- **Row 2** keeps `declined`. Citation: decision 0002 and decision 0017:
  the method here is the digest-pinned table
  (`crates/brokkr-runtime/tests/witness_digests.rs`), and its value is
  that it is not prompt text
- **Row 3** keeps `alternative`. Citation: decision 0006: per-seat
  attempts and deadlines are bundle limits (`seats.<phase>.limits`, e.g.
  `recipes/fast/bundle.json`), pinned in the run manifest; decision 0034
  and decision 0035: cost, effort and reasoning tokens are recorded in
  the seat record (`contracts/seat-record.v8.schema.json`); no coverage
  field
- **Row 4** keeps `alternative`. Finding: "Treat a frontier model
  release as an event that re-runs every baseline and retires
  scaffolding that lost its margin (a practice drawn from the paper's
  question, not its recommendation)". Citation: decision 0031: the
  served model is recorded per seat;
  `crates/brokkr-runtime/tests/witness_digests.rs` moves when a hire
  changes; no re-baselining; #439 would re-run qualification on provider
  and tool upgrades, with no retirement of scaffolding

## Candidates

None proposed. Should the headline be replicated across models and
with significance testing, finding 1 would become a recipe beside the
wager harnesses: the same commission, one arm a single prompt, judged
by the same gates. The operator asked for the replication first.
