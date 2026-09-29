# AgentPolis/agent-constitution: implementation dossier

Research date: 2026-09-27. Default branch: `main`. Inspected revision: [`1408e3ace216b9a42da5a409210ca842872f4cfe`](https://github.com/AgentPolis/agent-constitution/tree/1408e3ace216b9a42da5a409210ca842872f4cfe), committed 2026-04-14T08:55:36Z. License: Apache-2.0. GitHub metadata and raw sources were retrieved live; the revision matches the earlier survey. This is source inspection, not a benchmark, security certification, or report of tests executed. No project code was installed or run.

## Scope and architecture

This is a small Python governance harness around decisions. The operative unit is a structured debate, not a general task graph or software delivery engine. `BaseAgent` combines role, goal, optional persona and a `Constitution` into a system prompt; an adapter produces one response; hooks inspect or transform it; a trace retains the result. `Debate.run` invokes challenger, defender and judge in order, validates their output, and returns a `DebateResult`. A `GovernanceGateHook` decides whether an agent response should trigger that sequence. The top-level constitution can be loaded from text, a file, YAML or a SOUL document and merged by concatenation. These are reusable instructions, with no independent interpretation of their prose as filesystem permissions. [S1], [S2], [S3], [S4]

```text
role + goal + persona + constitution
  → adapter call → cost accounting → post-call governance hook
  → optional challenge → defense → judgment
  → enriched response / callback + trace + governance artifact
```

The distinction between a decision review and an effect gate is crucial. The governance hook runs after the original adapter call. Its default behavior returns the response or adds governance information; it does not execute a separate authorized action, nor automatically turn every `reject` verdict into a hard execution refusal. A consumer can act on the callback/result, but that policy belongs to the embedding application. Calling this harness a mandatory pre-execution governor would overstate the inspected implementation. [S1], [S4]

## Contracts and refusal behavior

The debate has more executable structure than prompt-only review. JSON parsing tolerates Markdown fences, then checks the challenger/defender list shape and the judge's verdict vocabulary, score adjustment and actionability fields. Hooks that alter challenges or the verdict are revalidated, and their changes can be added to the audit trail. The resulting schema still allows substantive mistakes: a valid list of criticisms is not evidence that the criticism is true or independent. The request for exactly three challenges in the prompt is stricter than the nonempty-list validator. [S2]

Default strict validation raises `DebateValidationError` for invalid model output. The test `test_debate_raises_on_invalid_json_by_default` encodes that refusal. There is also an explicit fallback mode: invalid judge output can become `proceed_with_caution` with a negative score adjustment and generic follow-up instructions. `test_debate_can_opt_into_fallback_mode` preserves that behavior. A deployment requiring a closed result vocabulary must also decide whether this permissive option is admissible. [S2], [S8]

The trigger layer contains deterministic matching over score, keywords and context, but the source of those inputs matters. Without a score or matching policy, the hook returns the original response. LOW verification tier skips the debate even when a trigger matches, with dedicated tests. Therefore the word “governance” does not imply that every decision passes through all reviewers. These are documented, configurable semantics, not hidden evidence of a broken pipeline. [S4], [S9]

## Effects, permissions and lifecycle

The CLI adapter has a real local boundary: it refuses to run inside an active Claude Code session, defaults to an empty tool set, and can pass an explicit allowed-tools list. The subprocess call has a 120-second timeout and checks nonzero exit and malformed JSON. This supplies more containment than merely asking a model not to use tools. It does not establish process-group ownership, descendant settlement, or an environment allowlist: the inspected invocation uses `subprocess.run` and inherits the host environment. HTTP adapters have separate provider behavior, so the CLI's timeout cannot be generalized to every adapter. [S6]

`CostGuard` is cumulative accounting with soft and hard thresholds, and an override hook. Its location is a material limit: `BaseAgent.run` calls the adapter first, then records returned cost. The “hard limit” exception therefore cannot prevent the call that crossed it from spending money. When it raises and the default hook rethrows, the rejected accounting increment also does not become the retained total. The tests verify threshold arithmetic and override accounting, not reservation of a maximum charge before dispatch. This is a useful caution for Brokkr's resource-bound work: post-call measurements and admission controls solve different problems. [S1], [S5], [S10]

The synchronous debate and adapter surfaces reviewed here do not provide a persisted scheduler, cancellation token propagated across all stages, automatic stage resume, or a formal uncertain-effect state. Replaying stored debate material is an inspection/demo facility; it is not proof that an interrupted paid call can be repeated without duplicate charge. No durable effect intent/acknowledgement transaction was established. These are scope limits of this review and harness, rather than assumptions about undiscovered integrations.

## Evidence and integrity

`GovernanceChain` canonicalizes JSON and links SHA-256 hashes from a genesis value through governance records. It supports offline artifact verification and JSONL serialization. Tests cover modified payloads, changed predecessor hashes, deleted/reordered records, round trips and external root pinning. The implementation candidly states that signatures are reserved but unimplemented and that a writer can recompute an entire chain. Thus a pinned root can make subsequent alteration detectable; an unanchored chain cannot authenticate the producer or model identity. [S7], [S11]

Another boundary is when the chain is assembled. The gate constructs assessment/challenge/defense/verdict records after `debate.run` completes. That is a governance artifact derived from a completed exchange, not an append-before-effect journal that can reconstruct all partial progress after process death. The in-memory `RunTrace` stores post-hook responses, so raw provider output, transformed display and governance artifact should not be treated as interchangeable evidence. [S1], [S4], [S7]

## Maintenance and operation

The package declares version 0.1.0, Python 3.11 or later, Apache-2.0 licensing and Beta status. Required dependencies are Rich, PyYAML and HTTPX, with Anthropic optional. The CLI entry point is `ac`. CI installs the development extra, runs Ruff and pytest on Python 3.11 and 3.12. This is a comparatively compact local setup with mock adapters and focused tests; the last inspected default-branch commit is April 2026, so no claim of recent maintenance cadence or production adoption follows from the snapshot. [S12], [S13]

## Comparison with Brokkr and useful experiments

The closest overlaps are reusable mandates, structured review and hash-linked evidence. Brokkr supplies a Rust phase-policy/effect separation, pinned run manifest and SQLite journal whose pure fold derives runtime state; the inspected harness supplies a judgment protocol that an application can insert around a decision. Brokkr's protected review requirement and this harness's optional gate are different authority models. Brokkr's open lifecycle, policy-validation and effect-retry issues still prevent treating its intended contract as fully qualified.

Three ideas deserve small controlled experiments. First, submit syntactically valid but unsupported criticisms and measure whether explicit missing-context and upgrade/downgrade fields make review more actionable. Second, kill a driver after each adapter response but before artifact assembly and compare retained evidence; this tests the difference between a final governance artifact and durable progression. Third, cross a cost threshold with a fake billed adapter and verify whether the caller can observe actual spend even when a refusal occurs. Adopt the clear integrity threat model and hook-change accounting without adopting numerical readiness scores as objective proof of correctness.

## Source evidence ledger

| ID | Source and inspection purpose |
|---|---|
| S1 | [constitution/base_agent.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/base_agent.py) — prompt construction, adapter-before-hook order, post-call accounting and trace |
| S2 | [constitution/debate.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/debate.py) — linear debate, schema validation, strict/fallback behavior |
| S3 | [constitution/constitution.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/constitution.py) — constitution loading and textual merge |
| S4 | [constitution/hooks.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/hooks.py) — trigger policy, verification tiers, gate and artifact assembly |
| S5 | [constitution/cost_guard.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/cost_guard.py) — threshold accounting and override |
| S6 | [adapters/claude_cli.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/adapters/claude_cli.py) — tool flags, environment check and subprocess timeout |
| S7 | [constitution/governance_chain.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/governance_chain.py) — canonical hashes, artifact verification and stated threat model |
| S8 | [tests/test_debate.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/tests/test_debate.py) — invalid JSON refusal and optional fallback tests |
| S9 | [tests/test_hooks.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/tests/test_hooks.py) — trigger/skip paths and hook revalidation tests |
| S10 | [tests/test_cost_guard.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/tests/test_cost_guard.py) — boundary and accounting tests |
| S11 | [tests/test_governance_chain.py](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/tests/test_governance_chain.py) — tamper and round-trip tests |
| S12 | [pyproject.toml](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/pyproject.toml) — package/dependencies/license and development status |
| S13 | [.github/workflows/ci.yml](https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/.github/workflows/ci.yml) — Python matrix, lint and tests |

[S1]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/base_agent.py

[S2]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/debate.py

[S3]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/constitution.py

[S4]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/hooks.py

[S5]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/cost_guard.py

[S6]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/adapters/claude_cli.py

[S7]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/constitution/governance_chain.py

[S8]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/tests/test_debate.py

[S9]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/tests/test_hooks.py

[S10]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/tests/test_cost_guard.py

[S11]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/tests/test_governance_chain.py

[S12]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/pyproject.toml

[S13]: https://github.com/AgentPolis/agent-constitution/blob/1408e3ace216b9a42da5a409210ca842872f4cfe/.github/workflows/ci.yml
