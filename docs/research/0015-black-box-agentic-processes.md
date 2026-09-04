# 0015 — A Black Box for Agentic Processes: Blockchain-Anchored Evidence for AI Agent Communication, Human Oversight, and GRC Audits

Source: https://arxiv.org/abs/2609.04017
Authors: Arslan Brömme (Independent Researcher)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

A position and architecture paper motivated by the 2026
OpenAI/Hugging Face incident, in which models under internal
security evaluation circumvented isolation, communicated through
unauthorized channels and accessed third-party systems. It proposes
a vendor-neutral black box that creates blockchain-anchored
cryptographic commitments of selected agent communications,
human-in-the-loop approvals, tool calls and process artifacts,
keeping sensitive content off-chain.

The evidence model is the paper's core distinction: hash anchoring
alone buys temporal anchoring and artifact integrity, an existence
and integrity proof for a byte sequence at or before commitment
time; event ordering, capture authenticity, authorized anchoring and
causal traceability each require additional architectural controls.
Anchoring does not prove authorship, authorization or truth. GRC
uses discussed: compliance testing, risk-based evidence selection,
monitoring evidence streams, incident reconstruction and reporting
readiness under the EU AI Act, NIS2 and the Cyber Resilience Act.
The paper presents no empirical evaluation and says plainly that it
prevents no misbehavior.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Anchor evidence of agent acts externally so a later auditor can verify existence and integrity without trusting the producing system | alternative | decision 0028 and decision 0033: Brokkr keeps the anchor inside the repository; every SHA the journal cites is planted as a keep-ref, and the ship anchor carries an offline-verifiable journal, rather than an external chain |
| 2 | Separate the evidence properties: temporal anchoring and integrity are cheap, while ordering, capture authenticity, authorized anchoring and causal traceability each need their own control | implemented | decision 0002, decision 0007 and decision 0025: the totally-ordered journal gives ordering, provenance rules give capture authenticity, and the signed grant gives authorized anchoring |
| 3 | Keep sensitive content out of the anchor: commit to the payload, never publish it | implemented | decision 0012 and decision 0032: secret bindings travel as names only and the journal carries paths or ids, never transcript bodies |
| 4 | Anchor to a vendor-neutral external chain for regulatory reporting | declined | decision 0003: the production runtime is one native binary with no services; an external chain is a dependency the product refuses |
| 5 | Reconstruct incidents from the evidence stream: who, what, when, under which policy | implemented | decision 0020: muninn reads only journal-derived models and records every proposal with its provenance |

## Candidates

The paper's five-property split of evidence is a clean skeleton for
an audit story: if the operator ever wants a compliance document for
the journal, temporal anchoring, integrity, ordering, capture
authenticity, authorization and traceability are the headings, and
each already has a decision behind it here. No external chain
needed; the ruling would say so.
