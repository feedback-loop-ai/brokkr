# 0031 — The served model is evidence; every model seat is pinned

Status: accepted
Date: 2026-09-02

## Context

The model named in an agent resolution is a plan: it says which mapping
produced the driver's argv. It is not evidence of what the provider
served. Claude's stream already carried the latter on assistant
messages, while Codex and dsh records exposed no equivalent fact to a
reader. The `fast` recipe made the distinction concrete: it pinned no
model, the account default served `claude-fable-5-1` in run
`scaffold-tool-grants-per-stack-b-919e2e14`, and the readout could be
mistaken for a different model.

Filling the gap from an adapter default or the agent's abstract model
would make the record internally tidy and evidentially false. Leaving
model selection implicit would also let one recipe mean something
different under two accounts without moving its manifest digest.

The operator ruled in chat on 2026-09-02: “we should pin the model no
matter the adapter, always.”

## Rulings

1. **The record carries the served model from the driver's own report.**
   Every turn checkpoint and final successful result produced by a
   built-in model adapter carries one field named `model`. Claude reads
   it from assistant stream-json messages, Codex from JSON events or its
   own usage header, and dsh from session-usage chunks. Exec reports
   `not applicable`. A model adapter that cannot learn the value reports
   `not reported`; it never substitutes an adapter default, recipe pin,
   or abstract agent model.

   **Enforcement binding:** the built-in folds in
   `brokkr-protocol::adapters`, driver conformance tests, and the shared
   `brokkr-view` derivation. The terminal, TUI, web console, phase graph,
   decision trail, `seats`, `export`, `costs`, and `compare` render that
   derivation rather than resolving a model again.

2. **Every model-backed invocation is explicitly pinned.** Inline
   Claude, LaneTally, Codex, and dsh commands carry a non-empty
   `--model <concrete-model-id>`. Agent-backed invocations remain pinned
   by their resolved candidate argv. Exec has no model and needs no pin.

   **Enforcement binding:** bundle compilation scans the fully composed
   seat tree before execution, reports every unpinned seat/member/step in
   one refusal, and gives the `--model <concrete-model-id>` repair.
   `brokkr doctor --bundle` exposes that same complete refusal. Repository
   recipes pin missing Claude seats to `claude-fable-5-1`, dsh seats to
   `deepseek/deepseek-v4-flash`, and Codex seats to `gpt-5.6-sol`.

3. **Configured and served models remain separate facts.** Decision
   0016 provenance continues to name the selected agent model and
   provider; it is labelled as a selection. The plain `model` label is
   reserved for the driver-reported served value. Existing journal rows
   are not rewritten, and their absent value stays visibly absent.

   **Enforcement binding:** view version 4 carries separate `provenance`
   and `model` cells and derives old-journal absence without fallback.

4. **Decision 0021 is orthogonal.** Pinning and reporting a model do not
   promote a driver, grant secrets, widen tools, change work/gate seat
   classes, or alter fallback-chain policy. Its trust tiers and binding
   grants remain exactly where the operator placed them.

   **Enforcement binding:** the existing 0021 compile refusals run after
   the pin refusal and keep their independent tests.

## Consequences

An operator can compare the pin to the served model without confusing
one for the other. Unknown provider evidence is explicit on new
built-in records. Recipe and bundle digests move where concrete pins
were added, including descendants whose base changed; frozen evaluator
fixtures and historical journals do not.

The wire protocol does not change. Its checkpoint `data` and successful
`result` objects already admit driver-owned fields, so `model` is an
additive payload fact under `forge-driver/v1`, not a new control-plane
message or a reinterpretation of a frozen field.
