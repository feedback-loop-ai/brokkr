# GitHub peer implementation dossiers

Reviewed 2026-09-27. Read [research 0019](../0019-github-peer-implementation-investigations.md) for the method, cross-project conclusions, backlog mapping and proposed experiments. [Research 0018](../0018-github-graph-agent-charter-comparison.md) is the earlier architectural survey.

These 21 dossiers inspect source and selected tests. They do not report executed third-party tests, provider trials, benchmarks or security certification. Each names its inspected revision and qualifies source-derived concerns. Spec Kit is excluded. OpenSpec is existing specification tooling; it is included for integration lessons. The GSD dossier describes the archived repository and identifies its successor without claiming to audit that successor.

## Composition, graphs and charters

| Repository | Focus of the investigation |
|---|---|
| [Amplifier Foundation](microsoft--amplifier-foundation.md) | Bundle composition, origin attribution, ownership, transfer fencing and subprocess lifetime |
| [Amplifier Recipes](microsoft--amplifier-bundle-recipes.md) | Executor parity, trust-policy wiring, CLI/library lock responsibilities, checkpoints and resume |
| [Attractor specification](strongdm--attractor.md) | Sequential outer traversal, conditional routing, goal gates, parallel handlers and the specification/implementation boundary |
| [Kilroy](danshapiro--kilroy.md) | Executable Attractor graphs, validation, checkpoint evidence, worktrees and process settlement |
| [Squad](bradygaster--squad.md) | Charter compilation, routing/context, host permissions, SDK hook wiring and incomplete test cases |
| [Charter](cspergel--Charter.md) | Executable governance checks, ledger anchoring, approval trust and adversarial tests |

## Delivery and operational coordination

| Repository | Focus of the investigation |
|---|---|
| [BMAD Loop](bmad-code-org--bmad-loop.md) | Coded delivery policy, recommendation-driven review, identity-aware cleanup, preservation and mutable resume |
| [BMAD Method](bmad-code-org--BMAD-METHOD.md) | Assistant-interpreted workflows plus deterministic rendering, snapshots and generation identity |
| [GitHub Agentic Workflows](github--gh-aw.md) | Workflow compilation, permission/job boundaries, safe outputs and runner trust |
| [aistack](blackms--aistack.md) | Workflow DSL, dispatch integration, timeout/cancellation wiring, audit chains and checkpoint/resume semantics |
| [Metaswarm](dsifry--metaswarm.md) | Specialist workflows, fresh review, evidence conventions, host adapters and procedural authority |
| [Gas Town](gastownhall--gastown.md) | Durable work coordination, formulas, worker lifecycle and operational recovery |
| [GSD, archived repository](gsd-build--get-shit-done.md) | Coded phase SDK, configurable verification, session permissions, retry and planning journal |
| [Ruflo](ruvnet--ruflo.md) | Task modules, optional checkpoint behavior, event-store durability, MCP bridge enforcement and packaging paths |

## General runtimes and narrower governance systems

| Repository | Focus of the investigation |
|---|---|
| [LangGraph](langchain-ai--langgraph.md) | Graph scheduling, attempt write guards, durability modes, interrupts and timeout limitations |
| [Microsoft Agent Framework](microsoft--agent-framework.md) | Typed graph validation, supersteps, sibling draining, checkpoint storage and topology identity |
| [CrewAI](crewAIInc--crewAI.md) | Role-oriented crews, Flow definitions, checkpoint lineage, persistence failure and timeout semantics |
| [Agent Constitution](AgentPolis--agent-constitution.md) | Structured debate, post-call governance, cost accounting and governance-artifact integrity |
| [InterAgents](jgzadidiLGDY--InterAgents.md) | Explicit local policy, worker/message boundaries, trace evidence and narrow runtime scope |
| [SwarmForge](unclebob--swarm-forge.md) | Shared constitutions, five product branches, role packs, handoff evidence and host authority |
| [OpenSpec](Fission-AI--OpenSpec.md) | Artifact dependency validation, existence-based progress, path constraints and guarded specification archive |

## Source receipts

[source-manifest.json](source-manifest.json) records immutable GitHub file URLs, inspected revisions, byte counts and SHA-256 digests for the dossiers' pinned file citations. It is a reproducibility aid, not an execution attestation or proof of correctness. Repository metadata such as archival status and default branches is dated in each dossier; those facts can change independently of the pinned source.

Raw downloaded source remains outside this repository. No framework adoption, implementation change, external vulnerability disclosure or live-provider experiment is approved by recording these findings.
