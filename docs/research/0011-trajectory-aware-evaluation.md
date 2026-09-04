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
| 2 | Judge from process evidence, explored context, attempted edits, solving paths, not only pass/fail outcomes | implemented | decision 0002: the journal is the totally-ordered process record, and decision 0034: the seat record carries per-turn tool and target evidence |
| 3 | Treat historical summaries as privileged inputs whose corruption degrades the verdict: provenance matters | implemented | decision 0007: every evaluation input is engine-computed or seat-declared; everything else is dropped before the table sees it |
| 4 | Price the evaluation itself: report per-task and calibration costs as first-class results | alternative | decision 0034: Brokkr records cost per seat after the fact; the budget is fixed up front, not spent against an estimator |

## Candidates

Finding 1 becomes practical if muninn starts comparing recipes
across runs: decision 0010 makes recipes comparable by run id, and
budgeted recovery is how that comparison stays cheap when runs are
expensive. The paper's warning that corrupted trajectory summaries
poison calibration is an argument for keeping such statistics
engine-computed only, as decision 0007 already requires.
