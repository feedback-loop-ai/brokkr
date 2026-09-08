# 0009 — The Illusion of Independent Quorums: Epistemic Fault Domains and Correlated Cognitive Failures in Agentic Quorums

Source: https://arxiv.org/abs/2609.02925
Authors: Jun He, Deying Yu (OpenKedge.io)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

Multi-agent quorums authorize high-stakes mutations, but distinct
reviewers often share upstream telemetry, documents and tool
backends, so one corrupted cause collapses many votes: replication is
not epistemic redundancy. The paper defines Epistemic Fault Domains,
the participants reachable from a common modeled cause, and the
Structural Epistemic Cut, the minimum number of root faults whose
exposure covers a decisive coalition, and proves it lower-bounds the
Semantic Compromise Cut under closed causal accounting, conservative
exposure and authorization alignment. Arbitrarily large quorums can
retain a cut of one; recognizing shared ancestry never increases
credited resilience; adding voters at a fixed threshold cannot raise
the cut. A Dependency-Aware Quorum Controller separates prospective
quorum selection over planned ancestry from commit-time
authorization over realized provenance, enforcing structural cuts at
runtime admission.

Simulated with 8,400 reviewer calls per configuration: under shared
evidence a model-diverse 2-of-3 quorum commits unsafe actions in
96.8 to 97.3 percent of cases, while separating the evidence paths of
the same model drops the faulted failure rate to 6.7 percent and
raises inter-reviewer disagreement from 25.8 to 90.8 percent; under
unanimity, path separation drops failures to about 0.1 percent.
Scaling does not help: seven voters on shared evidence fail 100
percent of the time, seven on separated paths 0.2 percent. A frozen
120-task benchmark with a standardized endpoint contract ships for
external validation. Submitted 2026-08-24, announced on the 2026-09-03
listings.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Judge independence is a property of evidence paths, not of model labels: model-diverse reviewers over shared evidence co-fail | alternative | decision 0045; `crates/brokkr-runtime/tests/roster.rs`: shipped panels require two adapter vendors among first-choice hires; the judges still share the repository and journal, and no evidence-path independence is established |
| 2 | Count fault-separated epistemic paths, not agent instances, when authorizing | not-planned | |
| 3 | Separate prospective selection over planned ancestry from commit-time authorization over realized provenance | alternative | decision 0041 and decision 0042; `recipes/triage/bundle.json`: triage selects a strategy before work and later gates judge the resulting artifacts; neither stage records evidence ancestry or computes an epistemic cut |
| 4 | Treat reviewer disagreement as a runtime signal of upstream faultiness | alternative | decision 0041 and decision 0047; `recipes/triage/policy.json`: typed findings return through bounded repair paths, park or stop by severity; residual closure is an operator-cited annotation, not an inference about shared upstream faults |
| 5 | Ship a frozen external benchmark with a standardized endpoint contract for validating the controller | alternative | `fixtures/evaluator/corpus.ndjson` and `crates/brokkr-cli/tests/machine_proof.rs`: the frozen corpus checks evaluator semantics; it is not an external benchmark of reviewer independence or quorum safety |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

Decision 0045 strengthened roster diversity from model families to
vendors at the first-choice hires. That is an enforced roster property,
not measured evidence separation, and availability fallbacks need not
preserve it among the models actually served. Decision 0046 also records
the boundary each seat used; isolation does not establish that judges
read independent evidence. Finding 2 remains not-planned.

## Candidates

Finding 2 remains the main open question: panel judges share the tree,
journal and spec, so an upstream source can expose several judges to
the same misleading material (also the concern in registry entry 0004).
Consider recording source ancestry and evaluating separated evidence
paths. A disagreement tally would require deterministic aggregation
across panel verdicts; current typed residuals and the operator's
supersede records do not diagnose evidence correlation.
