# 0010 — ToolGate: An Executable Acceptance Pipeline for Tool-Dependent Scientific Benchmark Construction

Source: https://arxiv.org/abs/2609.02067
Authors: Ke Zhang, Yankang Liu, Roya Zandi, Maziar Raissi (University of California, Riverside)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

Language models can propose benchmark items quickly; acceptance is
the hard part. ToolGate treats every generated item for
tool-dependent scientific tasks as a proposal and keeps it only if
three executable gates pass. First, an executable solution script
must reproduce the proposed answer when run with the scientific
software. Second, randomized no-tool screening rejects candidates a
model can already solve from the prompt alone. Third, a tool-using
agent must solve each survivor within a fixed time limit.

Instantiated in FEniCSx with 500 generation attempts: the
local-verification gate retains 478. Rescreening the pool after
generation, two randomized no-tool screens exclude 222 from the
reported pool, and direct GPT-5.5 API calls at medium reasoning, the
API default, exclude another 121. Of the remaining 135, a GPT-5.5
Codex CLI agent with FEniCSx access solves 130 within the time
limit; exact deduplication leaves 128 unique protocol survivors.
The pipeline turns repeated answer checking and difficulty screening
into an auditable process while leaving domain design and final
review to experts.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Keep a proposal only when an independent executable check reproduces its claimed result | alternative | decision 0041 and decision 0046; `recipes/fast/bundle.json` and `recipes/triage/bundle.json`: exec checks and separate judging seats assess delivery; they do not independently reproduce every benchmark answer, and isolation depends on the declared boundary |
| 2 | Screen for no-op solutions: reject items the evaluated system can already resolve without the capability under test | not-planned | |
| 3 | Rescreen the pool after generation rather than trusting the pass it was built under | alternative | decision 0038; `scripts/delivered-by-brokkr.sh`: an unchanged patch retains its delivery evidence, a docs delta requires an additional preflight over the exact head, and a code delta requires a new delivery run; preflight alone cannot replace delivery evidence |
| 4 | Name the effort setting of every screening call, because default-effort calls accept and reject differently | implemented | decision 0035; `recipes/research-dsh/bundle.json` and `crates/brokkr-store/src/seat-record.v4.schema.json`: model hires carry configured effort; reasoning-token measurements are recorded only when reported, separately from that setting |
| 5 | Gates make checking auditable; final acceptance stays with the expert | implemented | decision 0044 and `docs/decisions/README.md`: classification and acceptance are the operator's; every entry and decision stays proposed until ruled |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

The dsh effort setting now reaches the harness through a seat settings
file; the research lane is explicitly xhigh. Missing reasoning-token
measurements remain missing rather than proving an effort level. The
original sweep parked at verify and never shipped, so even a successful
preflight would not by itself satisfy the delivery-evidence gate for
this PR. Finding 2 remains a candidate, not an implemented screen.

## Candidates

Finding 2 is the paper's gift to the sweep's first topic: the
randomized no-tool screen is the concrete mechanism that resists
no-op solutions. If Brokkr ever versions new evaluation fixtures
beyond the frozen `fixtures/` corpus, that screen is the check to
carry in at versioning time.
