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
| 1 | Judge independence is a property of evidence paths, not of model labels: model-diverse reviewers over shared evidence co-fail | alternative | decision 0041: Brokkr requires two model families on every panel but does not separate the evidence paths the judges read; every judge reads the same tree and journal |
| 2 | Count fault-separated epistemic paths, not agent instances, when authorizing | not-planned | |
| 3 | Separate prospective selection over planned ancestry from commit-time authorization over realized provenance | alternative | decision 0041: triage rules the delivery class before work and the gate rules the realized work after; neither control computes an epistemic cut |
| 4 | Treat reviewer disagreement as a runtime signal of upstream faultiness | alternative | decision 0021 and decision 0006: a gate that does not pass parks the run rather than diagnosing why the evidence disagreed |
| 5 | Ship a frozen external benchmark with a standardized endpoint contract for validating the controller | alternative | `fixtures/`: Brokkr keeps its evaluator corpus frozen and versioned rather than regenerated, but it validates the product, not a quorum controller |

## Candidates

Finding 2 is the sharpest question this sweep brings. Every judge on
a Brokkr panel reads the same tree, the same journal and the same
spec: one misleading commit message or one corrupted instruction file
is a single epistemic root covering the whole panel, which is the
same vector registry entry 0004 recorded for commit messages. The
operator may want a ruling on evidence-path separation for judges, or
at least on recording which evidence each judge read. Finding 4: a
muninn tally of panel disagreement across runs would give the
operator the paper's runtime signal from evidence already journaled.
