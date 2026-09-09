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
| 1 | Separate sandbox spawning from capability activation: sandbox exploration and irreversible authority are distinct grants | alternative | decision 0046; `crates/brokkr-runtime/src/engine.rs`: sites with hands use a pinned namespace, harness or open boundary; Brokkr has no recursive spawn/activation controller or per-action risk certificate, and decision 0025 describes a signed executor grant not implemented in this tree |
| 2 | Condition an activation on the full history that selected it, not only the local certificate | alternative | decision 0029; `crates/brokkr-store/src/lib.rs`: fenced appends reject a write when the journal head differs from the one the caller folded; this is a concurrency check, not a certificate conditioned on the full risk history |
| 3 | Hold a trajectory-level risk budget in escrow and debit it per activation | alternative | decision 0006; `recipes/triage/policy.json`: seat attempts, deadlines and phase visits bound work and retries; there is no trajectory-level harm budget or activation escrow |
| 4 | Grant recursive authority sparingly and treat delegation as an explicit, revocable loan | alternative | decision 0020; `crates/brokkr-cli/src/muninn.rs`: Muninn reads a dossier and records validated proposals without executing them; the signed, expiring grant and recursive-authority ceiling of decision 0025 are not implemented here |
| 5 | Price risk and compute with shadow prices and let the prices set fanout thresholds | not-planned | |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

Findings 1 and 4 are corrected from implemented to alternative. This
tree contains no Skírnir executor or signed-grant loader; acceptance of
decision 0025 is not implementation evidence. The dispatch contract does
check expiry, digest, bounds and forbidden actions
(`crates/brokkr-core/src/dispatch.rs`); those checks do not implement the
operator-GPG-signed grant. Decision 0046 slice (i)
now supports namespace, harness and open, while seatbelt and container
are refused as unbuilt. A boundary grant is also not the paper's
per-activation risk certificate; sandbox spawning still consumes compute.

## Candidates

Finding 3 remains a possible future design question, not an attempt
counter with an extra label. Risk-weighted activation would require a
harm model, conditional certificates and an enforcement point; none is
provided by seat deadlines or an isolation boundary. The operator would
need to rule the scope before an escrow mechanism could be planned.
