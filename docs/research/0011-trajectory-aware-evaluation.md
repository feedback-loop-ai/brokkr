# 0011 — Efficient SWE Agent Benchmarking via Trajectory-Aware Evaluation

Source: https://arxiv.org/abs/2609.01603
Authors: Kefeng Duan, Dewu Zheng, Yanlin Wang, Xiwen Wang, Ensheng Shi, Xilin Liu, Yuchi Ma, Jiachi Chen, Mingwei Liu, Zibin Zheng (Sun Yat-sen University; Huawei Cloud Computing Technologies; Zhejiang University)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

Evaluating software-engineering agents on realistic benchmarks is
expensive: SWE-bench holds more than 2,000 tasks, a single full run
has an estimated upper-bound cost above 8,000 dollars under a
4-dollar-per-task limit, and the average cost on resolved instances
is 1.59 dollars for SWE-agent with GPT-4 Turbo. Existing efficient
evaluation selects representative subsets but fits only pass/fail
matrices or static task semantics, discarding how agents solve.
PTA-IRT, a privileged trajectory-aware item response theory
framework, fuses process and outcome: historical execution
trajectories supply explored context, attempted edits and solving
paths as privileged information for calibration-subset selection and
ability estimation.

On four SWE benchmarks under low calibration budgets, PTA-IRT
achieves the lowest score error and the highest Kendall and Spearman
ranking agreement against classical IRT, neural IRT and agent-driven
item-selection baselines, and stays the best method across the full
budget range; a small budget already yields practically useful
ranking agreement. Ablations show the trajectory features carry the
gains: removing the trajectory scorer or the privileged supervision
weakens recovery, and corrupted trajectory summaries weaken it too.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Budget evaluation explicitly: recover a verdict on the whole from a priced calibration subset | alternative | decision 0006 and `recipes/fast/bundle.json`: Brokkr bounds each seat by attempts and deadlines rather than estimating the score of unrun work |
| 2 | Judge from process evidence, explored context, attempted edits, solving paths, not only pass/fail outcomes | alternative | decision 0034; `crates/brokkr-store/src/seat-record.v4.schema.json` and `crates/brokkr-cli/src/compare.rs`: bounded tool/target metadata, verdicts and accounting support process comparison; the record excludes full trajectories and Brokkr has no trajectory-aware estimator |
| 3 | Treat historical summaries as privileged inputs whose corruption degrades the verdict: provenance matters | alternative | decision 0007 and decision 0034; `crates/brokkr-store/src/seat_record.rs`: evaluator inputs have declared owners and seat records are schema-checked at append; those controls do not authenticate a summary or establish that a seat claim is true |
| 4 | Price the evaluation itself: report per-task and calibration costs as first-class results | alternative | decision 0034; `crates/brokkr-cli/src/compare.rs`: comparisons report per-seat accounting, cost and attempt deltas between runs; no calibration-subset cost or estimate of unrun tasks is produced |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

Comparison by run id already exists, including model-selection and
boundary differences. The append-time seat-record validator has also
landed, so invalid accounting records are refused before sealing.
Neither feature calibrates agent ability or validates the truth of
model-authored summaries. The privacy-bounded record carries tool and
file identifiers, not the full exploration, edits or reasoning traces
used by PTA-IRT.

## Candidates

Finding 1 would extend the existing compare command with a priced
calibration subset and an estimator; it does not require inventing run
comparison first. Such an evaluation would need an explicit source for
trajectory features, a held-out outcome check and treatment of missing
accounting. Decision 0007 controls who may supply an input; it does not
make a supplied trajectory summary reliable.

[Issue #237](https://github.com/feedback-loop-ai/brokkr/issues/237)
proposes controlled continuation wagers and cumulative graph metrics;
it is not an implemented checkpoint-fork or PTA-IRT estimator. Broader
cost-per-accepted-change evaluation is proposed in
[issue #224](https://github.com/feedback-loop-ai/brokkr/issues/224).
