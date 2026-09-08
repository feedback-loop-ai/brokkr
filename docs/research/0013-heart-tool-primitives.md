# 0013 — Harness Engineering in LLM Tool Use via Agent-Native Reusable Tool Primitives

Source: https://arxiv.org/abs/2609.01736
Authors: Haibo Jin, Xucheng Yu, Haohan Wang (School of Information Sciences, University of Illinois Urbana-Champaign); Suijin Wang, Haojing Luo (Starc Institute)
Read: 2026-09-04
Status: proposed
Intake: research sweep, run weekly-research-sweep-registry-d-2012cc4d, 2026-09-04

## Summary

Schema-based tool invocation is brittle: on NESTFUL's nested API
calls the strongest models reach only 28 percent full-sequence match
accuracy, and multi-turn performance degrades as interaction depth
grows. Tool Primitives replaces rigid API-schema invocation with
natural language as the interface: each tool is wrapped in an LLM
interface that resolves schemas and executes internally, so tool
outputs flow into the next call as text. ToolFace is a centralized
repository of 25,519 functions from which only relevant tools are
retrieved at inference time instead of enumerating schemas in
context. HEART orchestrates both with a Planner, a Router and a
Verifier supporting invocation planning, multi-step execution and
feedback-driven recovery under a capped re-planning budget.

Across five benchmarks HEART outperforms SFT-based models by 10
percent on average and GPT-5.4, Claude-4.6-Sonnet and Gemini-3.1-Pro
by 6 percent on average while reducing API cost by up to 85 percent;
on 50 real-world tasks it reaches 84 percent task completion, 3.8
times the average of three frontier commercial models.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Wrap every external tool in an LLM-facing interface and make natural language, not schemas, the invocation interface | alternative | decision 0043 and decision 0046; `crates/brokkr-runtime/src/engine.rs`: namespace hands use one workspace tool with explicit binds, while harness and open boundaries compose differently; Brokkr does not add an LLM wrapper per external tool |
| 2 | Hold a central repository of tools and retrieve only the relevant ones into context at inference time | alternative | decision 0016 and decision 0041; `crates/brokkr-runtime/src/agents.rs`: the library resolves declared office/model chains against availability and recipe strategy, rather than retrieving tools by semantic relevance |
| 3 | Orchestrate tool use with a planner, a router and a verifier instead of one reasoning loop | alternative | decision 0002 and decision 0042; `recipes/triage/bundle.json`: a linear outer phase machine includes strategy selection, artifact-authoring sequences and judging panels; routing follows declared policy rather than a tool planner |
| 4 | Bound the recovery loop: verifier feedback drives re-planning under an explicit budget | alternative | decision 0041 and decision 0042; `recipes/triage/policy.json`: declared findings return to implement, design or triage under phase-visit bounds, with bounded clarify/analyze loops and per-seat attempt/deadline limits |
| 5 | Cut API cost by routing work to cheaper calls where quality allows | alternative | decision 0021 and decision 0041; `crates/brokkr-runtime/src/agents.rs`: strategy selects declared hires and availability resolves approved fallback chains; a running failure does not trigger a cheaper replacement and no runtime quality/cost scorer chooses the model |

## Reconciliation — 2026-09-08

Against main at `7b53e92` (decision 0046 slice (i)).

The outer machine remains linear, but it now contains the enacted SDD
sequences and strategy-dependent seats; saying it has no routing would
miss those controls. The one-tool description applies to namespace
hands, not every invocation on every boundary. Declared availability
fallbacks also differ from substituting a cheaper model after a failure.

## Candidates

No uptake proposed. HEART's per-tool LLM interfaces and semantic tool
retrieval remain different from Brokkr's declared offices, pinned policy
and boundary-specific tool execution. The paper motivates examining
schema brittleness; its benchmark results do not establish that Brokkr's
alternative performs better.

[Issue #227](https://github.com/feedback-loop-ai/brokkr/issues/227)
proposes boxing the remaining work offices and removing their allow-lists;
that migration has not landed. [Issue #224](https://github.com/feedback-loop-ai/brokkr/issues/224)
proposes economic implementation selection while retaining deterministic
routing and pinned hires, rather than adopting HEART's tool router.
