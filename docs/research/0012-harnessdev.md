# 0012 — HarnessDev: Can LLMs Create and Evolve Their Own Agent Harness?

Source: https://arxiv.org/abs/2609.01437
Authors: Yuhao Wu, Jingyuan Zhang, Jiajun Shi and 16 co-authors (ByteDance Seed; Singapore University of Technology and Design; Georgia Institute of Technology; M-A-P; TokenWave.AI)
Read: 2026-09-04
Status: proposed
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
| 1 | Score a harness on held-out capability and execution efficiency together; cost is a quality axis, not metadata | alternative | decision 0010 and decision 0034: recipes are comparable by run id and the seat record carries per-seat usage and cost; Brokkr evaluates deliveries, not the harness as an artifact |
| 2 | Withhold the evaluation tasks from the loop that develops against them | alternative | decision 0041 and `fixtures/`: judges are separate seats that change nothing, and the evaluator corpus is frozen and versioned rather than fed back as a training signal |
| 3 | Do not expect a model to evolve its own harness reliably: gains are unstable and runtime-model dependent | alternative | decision 0001 and decision 0002: control-plane semantics are never repaired by a model; the outer machine is a fixed linear FSM whose changes arrive only as decisions |
| 4 | Harness quality couples to the model that executes it, so pin the pairing | implemented | decision 0031 and decision 0035: every model-backed seat is pinned to the provider-reported served model and every pin carries an effort pin, so a run is attributable to one exact pairing |
| 5 | Report the variance across creator models in harness quality and cost | not-planned | |

## Candidates

Finding 3 is field-side corroboration of the constitution: the
evidence that self-evolving harnesses transfer poorly is a citation
the operator can use when a self-modifying recipe is proposed.
Finding 1: recipe comparison by run id is the substrate a
HarnessDev-style capability-and-efficiency ranking of recipes would
need; muninn could produce it from journaled seat records.
