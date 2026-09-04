# 0014 — Spawn Freely, Act Sparingly: Progressive Risk Vesting for Recursive LLM-Agent Trees

Source: https://arxiv.org/abs/2609.01035
Authors: Molly Wang (Imperial Business School, London)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

Recursive agents broaden their search by spawning specialists, but
some branches later request tools with external effects. The paper
distinguishes sandbox spawning, where external controls prevent the
specified harm, from capability activation, where a branch crosses
an irreversible-action boundary. Progressive Risk Vesting holds a
trajectory-level risk budget in escrow and debits it as branches are
activated, and proves an anytime harm bound for adaptively generated
trees; branch outcomes may be dependent, but each local certificate
must stay valid conditional on the full pre-activation history,
including the information used to select the request. With gates,
charges and compute constraints fixed, delayed vesting preserves
every policy available under irrevocable spawn charging.

In a stylized branching model, trajectory harm phase-transitions as
the authority reproduction number crosses one: proportional to local
risk below criticality, proportional to its square root at
criticality, and with a positive floor above it. A finite-type
occupancy model yields shadow prices for risk and compute, and for
nested fanout with decreasing marginal value these prices produce a
threshold rule. The synthetic studies do not estimate safety in
deployed agents. Design rule: search broadly in the sandbox, grant
recursive authority sparingly, with an explicit risk charge.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Separate sandbox spawning from capability activation: exploration is free, irreversible action is a distinct grant | implemented | decision 0043 and decision 0025: every command runs in an empty-root box holding only the worktree, and the standing executor acts only within a signed, expiring grant |
| 2 | Condition an activation on the full history that selected it, not only the local certificate | alternative | decision 0029: the fenced append binds a write to the head it folded from and refuses when the journal has moved; the check is against the journal's head, not a risk budget |
| 3 | Hold a trajectory-level risk budget in escrow and debit it per activation | alternative | decision 0006: Brokkr's budget is per-seat attempts and deadlines, counted in attempts rather than in risk |
| 4 | Grant recursive authority sparingly and treat delegation as an explicit, revocable loan | implemented | decision 0020 and decision 0025: muninn proposes and never rules, with delegation only by future recorded grant, and the executor's grant is signed, expiring and under a compiled never-list ceiling |
| 5 | Price risk and compute with shadow prices and let the prices set fanout thresholds | not-planned | |

## Candidates

Finding 3: Brokkr's attempt bound counts attempts, not harm. A
risk-weighted debit would tell a judging seat that only reads apart
from a shipper that can write; the operator may want to rule whether
the attempts bound should carry a risk weight.
