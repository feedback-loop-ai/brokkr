# 0004 — What's in Your Agent's Context? Context Privilege Escalation Attacks against AI Agent Harness

Source: https://arxiv.org/abs/2609.01222
Authors: Zichuan Li, Jian Cui, Ashley Chen, Xiaojing Liao, Luyi Xing (University of Illinois Urbana-Champaign)
Read: 2026-09-04
Status: proposed
Intake: chat session, operator-fed, 2026-09-04

## Summary

The harness, not the model, is the attack surface. A coding-agent
harness assembles its context from many heterogeneous, opaque sources
with implicit trust, and that opacity yields two attack classes.
Message-role escalation puts low-trust content into a high-privilege
role. Cross-scope escalation lets session-scoped attacker content
persist into project- or user-scope files such as memory and
configuration, reloaded on every launch.

The method, a context risk analyzer, runs in three stages: static
analysis to enumerate candidate context sources; runtime
instrumentation that hooks the model endpoint and injects a random
canary string into each source to prove which role and scope it
reaches; exploitability validation in isolation. Sources are modelled
on three axes: role, scope (user, project, session) and lifecycle
(loaded at launch or at runtime).

Twelve harnesses were analysed, Claude Code, Codex, Gemini CLI, Aider,
OpenCode, Cline and Goose among them: 282 vulnerable context sources
and 16 novel vectors, with end-to-end proofs of concept on all twelve
reaching remote code execution, full compromise, denial of service and
tool manipulation. Notable vectors: dynamic shell blocks in discoverable
skill directories; an override file that outranks the project's agent
instructions and hijacks pull-request review; and commit messages read
through `git log` entering the context at the highest privilege and
steering review verdicts. Two vendors shipped mitigations.

## Findings

| # | Finding | Classification | Citation |
|---|---|---|---|
| 1 | Keep an explicit inventory of every context source a seat reads, with its role, scope and lifecycle, and diff it in CI | alternative | decision 0017 and `crates/brokkr-runtime/tests/witness_digests.rs`: the bytes a seat is built from are pinned as bundle identity; no per-source role and scope inventory |
| 2 | Plant a canary in each context source, intercept the model request, and assert the role and scope it reached, as a regression gate on adapter changes | not-planned | |
| 3 | Treat git metadata, agent-instruction files, skill directories and memory files as untrusted input, and keep review seats from reading them at instruction privilege | alternative | `agents/charters/review-chief.md` treats peer prose as untrusted input; decision 0043 boxes what a gate can reach; commit messages are still read by the reviewer as instructions |
| 4 | Deny session-scope writes into project or user scope without an operator ruling, and hash-pin instruction files | implemented | decision 0043: the box cannot plant a hook and gates never write; decision 0034: the seat record admits no prompt or response text |
| 5 | Ban or sandbox dynamic shell markup in skills and scope skill discovery to pinned paths | alternative | decision 0043: hands replace the harness's own tools with one boxed workspace tool, so a skill's shell block runs inside the box |
| 6 | Fail closed on unknown override-precedence files | not-planned | |

## Candidates

Finding 2 is a small deterministic test with a large payoff: it would
prove, per adapter, which of the inputs a seat sees reach the model as
instructions, and it would move every witness digest exactly once.
Finding 3's commit-message vector applies directly to every review
seat here and deserves a ruling.
