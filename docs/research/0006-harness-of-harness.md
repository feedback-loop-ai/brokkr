# 0006 — Harness-of-Harness: Multi-Day Autonomous Software Development with Continual Improvement

Source: https://arxiv.org/abs/2609.01481
Authors: Yan, Su, Zhang, Li, Zhang, Zhang, Chen, Bai, Hu (Shanghai Artificial Intelligence Laboratory)
Read: 2026-09-04
Status: proposed
Intake: chat session, operator-fed, 2026-09-04

## Summary

Long-horizon autonomous development is a loop-structure problem, not a
model problem. Wrapping an existing coding-agent harness in repeated
planning, implementation and independent quality-assurance loops with
persistent state beats single-pass and repeated-baseline runs at a
matched token budget.

Each loop maps an artifact state and an evidence state to the next
pair: the code, and the validated knowledge of its behaviour. Three
roles share one harness and one model and differ in prompt, authority
and deliverable schema. The planner is read-only and scopes a bounded,
verifiable increment. The developer is the sole writer and tests
baseline-then-retest. The tester evaluates a frozen, read-only
candidate against scenario criteria. An output that violates its
deliverable schema triggers a retry. Context is bounded by progressive
disclosure: a categorised index, details fetched on demand.

Over three iterations on a game-development benchmark, scores rose
from 49.6 to 71.5 for one harness and model pair, 26.9 to 49.0 and 42.2
to 58.8 for two others; on a frontier software-engineering set,
dominance rose from 44 to 71 percent, 72.7 at ten iterations.
Budget-matched, two loops scored 64.8 on 5.67 million tokens against
58.2 for three baseline passes on 6.33 million. Ablations: a frozen
plan costs 8.1 points; no evidence feedback 6.3; a cold start every
iteration 7.9 points and 30 percent more tokens. A multi-day case ran
70 iterations, opened 81 issues, closed 65 and reopened 17 on
regression.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | One writer: only the implementer mutates the tree, while the planner and the judge read a frozen snapshot | implemented | decision 0041: gates change nothing and the engine checks; the reviewed head is recorded and a moved head returns to review (`crates/brokkr-runtime/src/engine.rs`) |
| 2 | Carry an evidence state forward between iterations: what has been verified, not only what changed | alternative | decision 0028 and decision 0029: the journal is the evidence and every cited commit is kept, per run; nothing is carried into the next run's planning |
| 3 | Validate each role's deliverable against a schema and retry on violation | alternative | decision 0001: a schema mismatch parks with the raw evidence and is never repaired or retried |
| 4 | The developer tests baseline-before and retest-after, separately from independent quality assurance | implemented | `agents/charters/implementer.md` commits the work with its tests; decision 0043: the verifier is a boxed script, not the implementer's word |
| 5 | Warm-start each iteration and re-plan every time; both ablate hard | alternative | decision 0030: a retry rejoins its own session with the sandbox class re-imposed; decision 0022: a finding returns to implement as declared input |
| 6 | Compare configurations budget-matched (score per token at equal passes), by dominance, and by issue close and reopen counts | alternative | `crates/brokkr-cli/src/compare.rs` folds two journals to their first divergence with per-seat cost; outcome economics stay in the ledger (decision 0021) |

## Candidates

Finding 2 is the one to weigh. Every run here starts cold, and the
paper's cold-start ablation is its largest single cost. A carried
evidence ledger that the next run's triage reads is a small change to
the intake and a large one to the epistemics.
