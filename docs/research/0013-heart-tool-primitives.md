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
| 1 | Wrap every external tool in an LLM-facing interface and make natural language, not schemas, the invocation interface | alternative | decision 0043: Brokkr takes the opposite route; the model's hands are one boxed tool over the worktree, so richness lives inside the box rather than in per-tool wrappers |
| 2 | Hold a central repository of tools and retrieve only the relevant ones into context at inference time | alternative | decision 0016: Brokkr's library names one file per agent and adapters are data; seats are hired explicitly rather than retrieved dynamically |
| 3 | Orchestrate tool use with a planner, a router and a verifier instead of one reasoning loop | alternative | decision 0002: the outer machine is a linear phase machine; the phases carry what a planner and verifier do, and there is no router |
| 4 | Bound the recovery loop: verifier feedback drives re-planning under an explicit budget | alternative | decision 0022 and decision 0006: reforging returns a finding to the implementing seat as declared input, bounded at two reforgings, under per-seat attempts and deadlines |
| 5 | Cut API cost by routing work to cheaper calls where quality allows | alternative | decision 0021: Brokkr parks rather than substitutes; the hire is pinned and a gate that cannot pass stops the run instead of descending to a cheaper model |

## Candidates

No uptake proposed. The entry records the opposite bet: HEART adds
per-tool LLM wrappers and a runtime router; Brokkr removes the tool
surface to one boxed tool and pins every hire. The paper's 28
percent on nested schemas is evidence the brittleness it fixes is
real, which is an argument for keeping the Brokkr route, not
adopting this one.
