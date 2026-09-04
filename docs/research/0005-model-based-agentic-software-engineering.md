# 0005 — Model-Based Agentic Software Engineering (MAGE)

Source: https://arxiv.org/abs/2608.25174
Authors: Davis, Kalu, Peng (Purdue University); Patil (Amazon Robotics)
Read: 2026-09-04
Status: proposed
Intake: chat session, operator-fed, 2026-09-04

## Summary

Coding agents raise implementation capacity, which moves the
bottleneck to governance. Capacity becomes durable progress only when
the environment makes consequential properties answerable, by keeping
the smallest purposeful model that answers an engineering question
beside the code, and gives settled obligations authority, as
constraints, sensors, validators and gates. The pair is called a
governed engineering environment. Four propositions: environment fit
moderates capacity, otherwise churn; representation leverage grows with
system size; authority and modelling are complements, not substitutes;
engineering capital amortises judgment.

Mechanisms: dependency graphs, state machines, interface contracts and
quantitative models sit beside the code; obligations are promoted into
mechanically decidable checks; every failure becomes durable structure
as a fix-plus-lint pair rather than a one-off patch. The brownfield
protocol is audit, drain, promote: measure the gap between model and
reality, burn the violations down, then make the check gating.

Numbers from one product built this way over about twenty weeks, with
six to eight concurrent agents and roughly a thousand commits a week:
around 540 thousand lines of production code beside 1.6 million lines
of governance infrastructure; lint files from 0 to 747; gate scripts
from 0 to 102; 208 fix-plus-lint commit pairs; unmodelled elements from
56 percent to 7.89 percent across nine modelling stages; six instances
of one drift class caught by model-derived checks and none recurring
over 56 later features. In a two-week experiment, agents given explicit
model-usage instructions used fewer tokens, turns and wall time, with a
larger effect on the weaker model. The industrial comparison is theory
building only; the authors make no causal claim.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Every fix ships with a decidable check; track the pairing rate | implemented | `docs/decisions/README.md`: every determinable ruling names its enforcement binding, and a ruling without one must say it is judgment guidance |
| 2 | Turn a new obligation into a gate by audit, drain, promote: measure the gap, burn it down, then make the check required | alternative | `docs/decisions/README.md`: a decision is proposed and the binding lands with the slice that enacts it; there is no drain phase |
| 3 | Track governance metrics over time: support-to-production ratio, gate and lint counts, unmodelled share, drift recurrence after a gate lands | not-planned | |
| 4 | Separate producing and reviewing agents with bounded roles and tools, and keep merge authority human | implemented | decision 0021: work and gate seats; decision 0041: gates never write; decision 0033: the operator merges |
| 5 | Progressive disclosure of tools and representations to the agent | alternative | decision 0043: a seat's hands are one workspace tool, and the box expresses the restriction |
| 6 | Keep purpose-built models beside the code and test them against the reality they depict | implemented | decision 0002: the phase table is data the engine evaluates; decision 0037: a diagram of a machine-owned structure is tested against that structure; decision 0023: the realms map |

## Candidates

Finding 3 is the paper's own measure of whether a governed environment
is paying for itself. Brokkr has the raw material in git history and
the decision record and computes none of it.
