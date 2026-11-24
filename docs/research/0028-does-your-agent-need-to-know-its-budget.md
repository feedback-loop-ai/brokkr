# 0028 — Does Your Agent Need to Know Its Budget? A Placebo-Controlled Experiment on LLM Coding Agents

Source: https://arxiv.org/abs/2611.21614
Authors: Iftesam Nabi, Jarif Ul Alam, Sunzid Hassan, Sabbir Alom Shuvo, Md Faiaz Al Islam, Yead Rahman, Mehrab Hosain (Louisiana Tech University)
Read: 2026-11-24
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-5feb3fa3, 2026-11-24

## Summary

Organizations put token budgets on coding agents, but what a cap costs,
and whether telling the agent about it helps, had not been measured
apart. The paper crosses enforcement (cap or none) with disclosure (a
sentence stating the budget, or not) at three cap levels, and adds the
arm most studies lack: a placebo, an enforced cap with a content-neutral
sentence of matched length. The arms are baseline, info-only (stated,
not enforced), silent-bare (enforced, no sentence), silent-placebo and
announced. 5,835 runs on two function-level repair pools, 29 QuixBugs
programs and the 164-task HumanEvalFix Python split, with
qwen3-coder-30B served locally under a custom repair loop of at most
ten turns; caps are multiples of the 75th-percentile first-call cost
measured on held-out development tasks. The abstract page prints a
submission date of 2026-08-08; arXiv announced the paper in its
software-engineering listing of 2026-11-18.

On the larger pool at the medium cap (544 runs a cell): baseline
resolves 73.5 percent, info-only 76.5, silent-bare 53.5, silent-placebo
60.7, announced 57.9. Enforcement alone costs 20.0 points (interval
25.0 to 15.4, p = .0002) and cuts tokens per resolved task from 13,720
to 1,179, about twelvefold. Telling the agent its budget does not beat
the placebo: announced against placebo is −2.8 points (interval −6.2
to +0.7), inside the registered ±10 equivalence bounds, and −4.2 at the
tight cap. The neutral sentence alone is worth +7.2 points over silent
enforcement (interval +4.0 to +10.5). A per-turn countdown is null
(58.0 against 58.7). Resolution rises with the cap, 63.0 to 70.8 to
82.4 percent on the smaller pool, and outcome variance under a cap is
about 1.5 times the uncapped. One tight cap was raised from 766 to
1,000 tokens because 24 of 164 tasks could not fund a first call.

Limits stated: two function-level pools and no repository-scale task,
open models on CPU, one custom harness, one placebo wording, a
preregistration not deposited externally, and a second, smaller model
that resolved nothing under any cap. The paper's closing sections were
read from a fetch summary, not verbatim text.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Enforce a budget in the harness, and do not rely on telling the agent about it: enforcement moves the outcome and disclosure adds nothing beyond a placebo sentence | alternative | decision 0006 and decision 0067; `recipes/fast/bundle.json`: the engine enforces a per-seat attempt limit and wall-clock deadline today; engine-enforced token, turn and context ceilings per visit are ruled by decision 0067 and unbuilt (#463) |
| 2 | Set a cap from measured runs, in units of a first call's cost, and check that the tightest cap can fund one call | not-planned | |
| 3 | When testing any prompt-level change, run a same-length neutral arm: a sentence of no content moved resolution by 7.2 points | not-planned | |
| 4 | Report resolution, tokens per resolved task and run-to-run variance as three outcomes, since a cap moves each differently | alternative | decision 0034 and decision 0064; `docs/guides/read-surfaces.md`: the seat record carries per-turn and finishing usage and cost, the costs readout sums them, and a ledger drawn from the journal leaves out runs whose work was never judged and says how many; no readout divides cost by shipped runs or reports variance |

## Candidates

Finding 1 supports decision 0067 as ruled and sharpens one clause of
it. 0067 enforces a ceiling "by asking for a clean handoff, never a
kill"; the paper's caps are hard stops, and they cost a fifth of the
resolved work. That is the price 0067's softer mechanism is meant to
avoid, and the paper gives the number to hold it to when #463 is
built: a visit that hits its ceiling and hands off should not lose
anything like 20 points of completion.

Finding 2: when 0067's ceilings are built they need values, and the
journal already holds the distribution to draw them from (seat-record
usage per office and phase, as decision 0112 does for CPU and memory
with `brokkr recipes usage`). The paper's method is a percentile of
measured first calls and a check that the cap admits one; an authored
round number is what it argues against.

Finding 3 bears on every charter and house-rules change, and on the
notice entry 0023 flags: the paper's placebo is worth more than most of
the prompt effects anyone claims. A wager that compares two prompts
without a neutral arm of the same length cannot tell a rule from added
text.
