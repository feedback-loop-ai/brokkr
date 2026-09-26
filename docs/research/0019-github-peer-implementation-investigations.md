# 0019 — GitHub peers: implementation, failure handling and setup

Source: https://github.com/microsoft/amplifier-bundle-recipes (multi-repository investigation; immutable sources in linked dossiers)
Authors: repository maintainers cited in the dossiers; analysis synthesized by Codex
Read: 2026-09-27
Status: proposed
Intake: operator-requested parallel, deep investigation of every project in research 0018, 2026-09-27

## Summary

The deeper investigation strengthens the conclusion that Brokkr has substantial architectural peers. It also changes how their guarantees should be compared: a charter, a dependency graph, a checkpoint and a hash-linked audit record each answer different questions. Their presence does not establish that an execution path enforces the complete intended contract.

This entry covers **21 repositories**: the 18 pinned in [0018](0018-github-graph-agent-charter-comparison.md), plus OpenSpec, Ruflo and the archived GSD repository that the first survey only screened. Spec Kit remains excluded because it is existing tooling. OpenSpec is also identified as existing specification tooling, with lessons for the dialect integration rather than a claim that it replaces Brokkr's engine.

The detailed record is in [21 implementation dossiers](github-peers/README.md), with an accompanying [source manifest](github-peers/source-manifest.json). Each dossier covers architecture, graph/agent/charter semantics, setup, permission boundaries, failure/recovery behavior, tests and lessons for Brokkr. SwarmForge includes separate pins for its five documented product branches. The named GSD successor is identified but its implementation is outside this review.

## Findings

The first five rows map relevant observations to work already commissioned in the architecture backlog. They do not expand those issues to include building or adopting peer integrations. The sixth is a new research candidate, still unplanned.

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Verify the complete policy and dispatch path, including every configuration entry point, rather than treating a validator or declared field as enforcement | planned | #429 owns policy validation and #430 owns dispatcher hardening |
| 2 | Qualify cancellation, descendant settlement and uncertain-effect replay separately from retry counts and state transitions | planned | #403 owns process lifecycle; #434 owns replay safety; #435 owns operational qualification |
| 3 | Establish the acknowledgement, integrity and restore contract of retained evidence under crash and concurrent access | planned | #431 owns consistent backup and restore evidence; #435 owns failure qualification |
| 4 | Prove the shipped configuration, dependencies and resource limits on the actual execution path | planned | #432 owns configuration isolation; #433 owns resource bounds; #438 owns setup and release qualification |
| 5 | Compare real delivery outcomes and maintenance changes within a declared scope before claiming superiority | planned | #436 owns outcome/usability evaluation; #437 owns maintenance evidence; #439 owns independent qualification |
| 6 | Build shared fake-effect fixtures and reproduce the source-derived peer limitations described in these dossiers | not-planned | |

## Method and limits

Three parallel source investigations covered the original 18 repositories; an additional pass covered the three previously screened projects. This is a coordinated review, not three independent audits of every claim. Investigators read implementation and selected tests, traced important call paths, inspected setup/CI declarations and rechecked GitHub identities. The review date uses Europe/Sofia; some retrieval timestamps are still 2026-09-26 UTC.

Immutable file URLs were fetched and their bytes hashed into the source manifest. This verifies citation availability and identity, not the truth of every conclusion. Raw third-party source was kept outside the repository; the checked-in material is analysis and source metadata. No third-party code, install scripts, dependencies, benchmark or provider workload was executed. Brokkr's own documentation checks are separate from that statement.

The dossiers distinguish source-established mechanisms, assertions in upstream tests, documentation claims and unestablished guarantees. A test that contains an assertion is not a passing test run in this investigation. A potential race inferred from code is not a reproduced defect. A control absent from one inspected call path is not presumed absent from every integration. Default-branch source is not automatically the behavior of a published package. No numerical quality ranking or broad security certification follows from this review.

## Material refinements to the first comparison

| Project | More precise conclusion | Detailed evidence |
|---|---|---|
| Attractor / Kilroy | Attractor specifies traversal of one top-level node at a time; parallel work is inside handlers. The difference from Brokkr is general branching/re-entry and policy semantics, not simply multiple active outer nodes. Goal gates do not by themselves prove every declared review was visited. | [Attractor](github-peers/strongdm--attractor.md), [Kilroy](github-peers/danshapiro--kilroy.md) |
| Amplifier Foundation / Recipes | Foundation has substantial ownership and transfer machinery. Those guarantees cannot be attributed to every recipe effect. CLI lock/provenance checks and library caller responsibilities also differ. | [Foundation](github-peers/microsoft--amplifier-foundation.md), [Recipes](github-peers/microsoft--amplifier-bundle-recipes.md) |
| BMAD Loop | Independent follow-up review is recommendation-driven by default, and explicit resume can reauthorize changed host configuration. Its lifecycle/recovery code has useful identity-aware cleanup and preservation tests. | [BMAD Loop](github-peers/bmad-code-org--bmad-loop.md) |
| BMAD Method | Workflow execution remains assistant-interpreted, while rendering has real input/renderer identity and snapshot verification. “Prompt workflow” does not mean it has no deterministic supporting machinery. | [BMAD Method](github-peers/bmad-code-org--BMAD-METHOD.md) |
| Squad | Compiled charters and SDK hooks do not establish a universal effect boundary. The inspected shell's permission callback approves requests; hook wiring and pending tests must be evaluated separately. | [Squad](github-peers/bradygaster--squad.md) |
| aistack | Its audit chain is substantive, but inspected timeout/cancellation wiring and CLI resume semantics are weaker than feature names imply. Audit failure can also be tolerated by the caller. | [aistack](github-peers/blackms--aistack.md) |
| Charter | The first survey understated its executable governance: it includes a hash-linked ledger with a head/count anchor and adversarial enforcer tests. It remains a different authority and persistence model from Brokkr. | [Charter](github-peers/cspergel--Charter.md) |
| GSD | The archived tree contains a coded delivery SDK and planning journal, not only workflow prompts. Its README explicitly names GSD Core as the successor; the successor's behavior was not inspected. | [GSD](github-peers/gsd-build--get-shit-done.md) |

The original survey remains a dated overview. These dossiers supply the more precise implementation qualifications; they should accompany any future positioning claim derived from 0018.

## Engineering lessons across projects

### 1. Compare where authority lives

There are at least four different arrangements here: an assistant interprets workflow prose; a coordinator manages workers and durable task state; a generic graph runtime schedules application-defined effects; or a delivery engine enforces a domain policy. Hybrid systems combine them. A shared word such as “agent” or “gate” does not make these arrangements interchangeable.

BMAD Loop and GSD are particularly relevant coded delivery peers. LangGraph and Microsoft Agent Framework provide deeper references for general scheduling and recovery. Metaswarm, BMAD Method and SwarmForge provide useful role/context/workflow authoring patterns whose authority depends more on their assistant host. GitHub Agentic Workflows is a separate deployment model worth studying for constrained effects and job permissions. [Dossier index](github-peers/README.md)

### 2. Trace a guarantee from configuration to effect

A declared timeout, capability intersection, tool whitelist or trust policy may be validated and recorded without reaching the actual effect. The Recipes and aistack investigations contain concrete examples of why callers and execution adapters must be followed. Ruflo's disclosed bridge incident and subsequent endpoint tests illustrate the same issue at a network boundary: a guard in one automation path is insufficient for another exposed entry point. [Recipes](github-peers/microsoft--amplifier-bundle-recipes.md), [aistack](github-peers/blackms--aistack.md), [Ruflo](github-peers/ruvnet--ruflo.md)

For Brokkr, every claimed refusal needs evidence at the real dispatch path, with malformed input and bypass attempts exercising each entry point. Counting typed fields, policy rules or helper tests cannot replace that evidence.

### 3. Separate cancellation from settlement and replay safety

LangGraph guards writes from timed-out attempts, yet its upstream tests explicitly document an event-loop-blocking case that defeats an async timeout. Microsoft Agent Framework cancels and drains sibling work before restoring checkpoint state. BMAD Loop considers descendant identity and PID reuse. These are distinct mechanisms, with different limits; none implies that every external side effect can safely run again. [LangGraph](github-peers/langchain-ai--langgraph.md), [Agent Framework](github-peers/microsoft--agent-framework.md), [BMAD Loop](github-peers/bmad-code-org--bmad-loop.md)

Brokkr should separately prove that work stops, stale writes cannot affect a later attempt, evidence accurately records uncertainty, and retry admission respects the effect's semantics. This is central to #403/#434/#435 and is more valuable than adding arbitrary outer-graph concurrency for its own sake.

### 4. Name exactly what is durable and authenticated

A checkpoint can be atomically visible without every acknowledged effect being durably recorded. An audit hash chain can detect edits without authenticating the writer. A topology hash can detect structural changes while leaving prompt or code changes outside its identity. Some inspected callers log persistence errors and continue. These choices may suit their products, but need explicit comparison with Brokkr's journal acknowledgement and pinned-run contract. [Agent Framework](github-peers/microsoft--agent-framework.md), [CrewAI](github-peers/crewAIInc--crewAI.md), [Agent Constitution](github-peers/AgentPolis--agent-constitution.md), [aistack](github-peers/blackms--aistack.md)

The important questions are what a successful return promises, which bytes the identity covers, who can rewrite the anchor, and how a fresh machine restores the evidence. Source-level distinctions guide #431 qualification; they do not award Brokkr a proven advantage before the restore drills exist.

### 5. Study ergonomics and conformance alongside refusal behavior

Foundation's composition provenance, Recipes' executor-parity fixtures, OpenSpec's dependency diagnostics and guarded archive rollback, GSD's generated-surface freshness checks, and Metaswarm's review/context conventions offer useful maintenance ideas. Setup should be evaluated through the installed artifact, supported host, dependency resolution and ordinary maintainer workflow. Large feature lists and convenient source scripts alone do not answer those questions. [Foundation](github-peers/microsoft--amplifier-foundation.md), [Recipes](github-peers/microsoft--amplifier-bundle-recipes.md), [OpenSpec](github-peers/Fission-AI--OpenSpec.md), [GSD](github-peers/gsd-build--get-shit-done.md), [Metaswarm](github-peers/dsifry--metaswarm.md)

## Candidates: a shared experiment suite

These are proposed research fixtures, not completed experiments or newly approved backlog work. Use deterministic fake effects before separately authorized provider trials:

| Experiment | Observable result | Useful references |
|---|---|---|
| Malformed evidence, unknown policy references, missing review | Which layer refuses, and whether any effect already ran | Brokkr, Recipes, BMAD Loop, OpenSpec |
| Parent exits while a child retains stdout; cancel during parallel writes | Process settlement, bounded wait and stale-write isolation | Foundation, Kilroy, BMAD Loop, LangGraph, Agent Framework |
| External write succeeds and acknowledgement is lost | Explicit uncertainty and whether another attempt duplicates the effect | Every executable peer; compare only supported effect paths |
| Crash or disk failure around checkpoint/journal acknowledgement | Recovered records and the last honestly acknowledged durable point | Brokkr, Recipes, aistack, CrewAI, GSD, Ruflo |
| Change topology, charter, prompt, dependency and model separately before resume | Exact identity fields refused, migrated or accepted | Brokkr, Recipes, Agent Framework, BMAD Loop |
| Fresh maintainer installs a pinned release and adds one role/adapter | Setup success, semantic edit sites, documentation gaps and required privileges | #437/#438 qualification, informed by each dossier |

## What this changes for Brokkr

The strongest potential distinction remains a coherent and demonstrably enforced delivery contract: reusable role data, composed recipes, protected review, bounded effects, pinned identity and inspectable event-derived evidence. The deeper review narrows that positioning; it does not establish uniqueness or erase Brokkr's open enforcement gaps.

The most useful next investment is closing and qualifying that contract while borrowing test patterns documented in the inspected source and useful authoring ergonomics. The current evidence does not justify replacing the architecture, claiming superior reliability, or assigning peers grades from source inspection. Adoption or comparative implementation should be proposed with a specific workload, guarantee and measurable result.
