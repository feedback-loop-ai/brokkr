# 0012 — HarnessDev: Can LLMs Create and Evolve Their Own Agent Harness?

Source: https://arxiv.org/abs/2609.01437
Authors: Yuhao Wu, Jingyuan Zhang, Jiajun Shi and 16 co-authors (ByteDance Seed; Singapore University of Technology and Design; Georgia Institute of Technology; M-A-P; TokenWave.AI)
Read: 2026-09-04
Status: ruled (2026-11-24)
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

HarnessDev shifts the unit of evaluation from task outputs to the
runnable infrastructure that executes them. In Creation, an agent
starts from a minimal runnable seed and a few cases and builds a
complete harness; in Evolution, it starts from its own harness and
revises it iteratively using downstream execution feedback. Each
constructed harness is evaluated on capability, task success on
held-out benchmarks with hidden evaluation tasks withheld from
development, and on efficiency, execution-token cost.

Creation covers six creator LLMs, four domains and five downstream
benchmarks totaling 2,207 unique instances. Generated harnesses
remain substantially behind mature human-engineered references on
code and on search and research, while matching or exceeding them on
writing and machine-learning experimentation. Execution cost varies
widely across creators and higher cost does not reliably buy better
results, so harness quality must be scored on capability and
efficiency together. Evolution produces some gains but they are
unstable, performance rises and falls across revisions, transfer
only partially to held-out tasks, and depend strongly on the model
executing the harness: changing the runtime model changes both the
starting performance and whether revisions help.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Score a harness on held-out capability and execution efficiency together; cost is a quality axis, not metadata | alternative | decision 0010 and decision 0034; `crates/brokkr-cli/src/compare.rs`: run comparison reports recipe identity, verdict trails, attempts and reported cost, but does not score held-out harness capability or efficiency |
| 2 | Withhold the evaluation tasks from the loop that develops against them | alternative | decision 0041; `fixtures/evaluator/corpus.ndjson` and `crates/brokkr-cli/tests/machine_proof.rs`: judge roles and frozen evaluator cases provide separate checks, but the cases are visible to implementers and are not held-out evaluation tasks |
| 3 | Do not expect a model to evolve its own harness reliably: gains are unstable and runtime-model dependent | alternative | decision 0001 and decision 0002; `crates/brokkr-runtime/src/bundle.rs`: runs use a compiled, pinned policy and invalid control-plane results are not repaired by a model; changes to the machine require a separately reviewed change |
| 4 | Harness quality couples to the model that executes it, so pin the pairing | implemented | decision 0031 and decision 0035; `crates/brokkr-cli/src/compare.rs`: model and effort configuration are pinned and selected hires are reported alongside provider-reported served models; the served name is a provider claim, not independently verified identity |
| 5 | Report the variance across creator models in harness quality and cost | not-planned | |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

The current compare command exposes recipe, selected-hire, served-model,
effort and boundary differences. Declared availability fallbacks mean a
recipe's first choice is not necessarily the served model; missing
provider evidence stays missing. The evaluator corpus is public in the
tree, so freezing it does not provide HarnessDev's hidden-task protocol.
No creator-variance or held-out harness ranking has landed.

## Erratum — 2026-11-25

The operator ruled this erratum on 2026-11-25, "approve all that move to
planned as well" for rows 1 and 2
([#1350](https://github.com/feedback-loop-ai/brokkr/issues/1350)), after
every row of this entry was checked three ways: against the article,
against main at `4912e583e`, and against the open issues and pull
requests. The text above stands as ruled on 2026-11-24 and stays
readable; where this section differs, this section is the product's
word.

- **Row 1** now reads `planned`, not `alternative`. Citation: #436 owns
  held-out, repeated evaluation of delivery quality with total cost per
  accepted result; today `crates/brokkr-cli/src/compare.rs` compares two
  runs' recipe identity, verdict trails, attempts and reported cost, and
  does not score held-out harness capability or efficiency
- **Row 2** now reads `planned`, not `alternative`. Citation: #436 owns
  a versioned evaluation corpus whose held-out cases are kept from
  candidate seats; today decision 0041 separates judge seats from
  implementers, and no held-out task set exists
  (`fixtures/evaluator/corpus.ndjson` is the policy evaluator's
  differential test corpus, read by
  `crates/brokkr-core/tests/differential.rs`)

## Candidates

Finding 3 remains supporting evidence for keeping runtime control-plane
repair deterministic. A model may still propose a separately reviewed
recipe change; this is not a ban on agent-authored harness work. Finding 1
would build a capability-and-efficiency evaluation on the existing run
comparison, with genuinely held-out tasks and consistent cost coverage.
Muninn does not currently produce that ranking.

[Issue #237](https://github.com/feedback-loop-ai/brokkr/issues/237)
already proposes controlled model wagers with cumulative delivery and
repair costs. It remains a backlog proposal and does not supply hidden
evaluation tasks or a measured harness ranking.
