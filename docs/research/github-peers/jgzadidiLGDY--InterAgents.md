# jgzadidiLGDY/InterAgents: implementation dossier

Research date: 2026-09-27. Default branch: `main`. Inspected revision: [`d0cba222108410112db271037c008cc6f6836c5a`](https://github.com/jgzadidiLGDY/InterAgents/tree/d0cba222108410112db271037c008cc6f6836c5a), committed 2026-07-18T18:18:44Z. License: MIT. GitHub metadata and raw sources were retrieved live; the revision matches the earlier survey. This is source inspection, not a benchmark, security certification, or report of tests executed. No project code was installed or run.

## Scope and architecture

InterAgents is a compact, policy-constrained research/critique/writing runtime. The current entry point is `run_loop_v1`; a separate V0 runner is explicitly retained for historical probes. An orchestrator dispatches a `MessageEnvelope` to a named role's `handle` method, while the canonical runner owns state mutation, dispatch limits and terminal STOP handling. A supervisor model proposes routes, subject to code-enforced policy checks. It is consequently inaccurate to call the entire system deterministic: its policy ordering and transition bookkeeping are deterministic while model-selected decisions and generated content are not. [S1], [S2], [S3]

```text
request → runner stop check → supervisor policy → structured model decision
 → researcher/tools → evidence packet → critic
 → research again or writer → draft → supervisor STOP
                  every dispatched output → RunState.apply_event + trace
```

The main loop initializes a fresh `RunState` from the starting envelope. Before dispatch it checks step, iteration and consecutive-critic-failure limits. It enriches supervisor input with a state snapshot, dispatches exactly one next envelope, applies the returned event to state, emits evidence lineage, and returns immediately on `control.stop`. Exhausting the outer step count is also a terminal result. This is a serial loop with controlled branching; it is not a general concurrent graph scheduler. [S1], [S4]

The repository does contain transition and fan-in abstractions, plus a minimal fan-out/fan-in experiment. Its source explicitly executes fake branches serially and accepts a simulated completion order. Aggregation sorts by ordinal, branch ID and packet ID; it rejects duplicate IDs and mismatched parents. Those are useful tested readiness contracts, but do not establish actual concurrent research execution or crash-safe multi-branch joins. [S5], [S6]

## Agent and policy contracts

Role prompts and parameters are loaded through `SystemMessageManager`; the runtime wraps a transport with prompt identity, response hashes, model/token metadata and parsed-decision linkage. These records make an individual decision more inspectable. They do not freeze an entire dependency closure, and the reviewed path does not compare an immutable charter manifest at resume. Agents implement specialized typed decision and evidence handling rather than accepting an arbitrary charter language with independently enforceable authority. [S7], [S8]

The supervisor's policy ordering is concrete. A retained draft can force STOP; missing evidence can force a return to research before post-initial critique or writing routes. Model routing remains active where policy does not override. The writer validates intended citation IDs as a subset of available evidence IDs and raises a structured error for invented IDs. This is stronger than a prompt instruction to cite sources, but it does not prove that a sentence is entailed by the cited text. Evidence production and forwarding are distinguished, so the runner avoids treating a forwarded packet as newly created research. [S3], [S9], [S10]

The inspected orchestrator checks that a recipient exists and has a `handle` method, then calls it through `safe_call`. The transition table in `policies/transitions.py` is described as a readiness contract. The basic dispatch function does not enforce the full table at each hop. Therefore list membership, typed role behavior, and globally mandatory transition enforcement must not be conflated. This is a particularly relevant distinction for Brokkr's dispatcher and policy-validation issues. [S2], [S5]

## Concrete failures and tests

`test_v1_stop_is_terminal_no_post_dispatch` supplies a supervisor transport that immediately requests STOP and makes every other transport raise if called. It then asserts that no additional dispatch occurs. This is a meaningful negative test of terminal precedence. Other tests exercise no-evidence writer containment, invalid citation IDs, critic veto and evidence packet lifecycle. These tests are source evidence of intended cases; no test command was executed in this investigation. [S11], [S12]

The fan-in policy's duplicate and parent validation is deterministic and can be tested without a provider. The phase 13 documentation sharply limits its live claim: an opt-in small supervisor smoke test validates a structured STOP exchange and transport metadata, not broad reasoning quality. Most verification uses fake or mocked transports. That honesty is valuable, especially because a large number of unit tests could otherwise be mistaken for evaluation of model quality. [S6], [S13]

## Effects, retries and recovery

`ToolRunner` generates a call ID, logs input and a start event, invokes the tool through `safe_call`, then logs either success metadata or a structured tool failure. This centralizes observation and errors; it is not a sandbox. A registered Python tool still runs with the host process's capabilities. The narrow built-in corpus tools reduce the demonstrated effect surface, but arbitrary plugin code and provider calls require their own permissions and secret handling. Full tool inputs enter the trace, so there is no established generic sensitive-data redaction boundary. [S14]

The retry utility defaults to three attempts, bounded exponential delays and optional jitter. Its default `sleep_fn` is a no-op: delay calculation alone is not backoff unless the caller supplies sleeping. More importantly, inspection did not establish this helper as a uniform retry policy around every canonical provider/tool call. The live-integration closeout explicitly leaves live retry policy out of scope. Repeating a callable after an exception is not evidence of uncertain-effect reconciliation or exactly-once operation. [S13], [S15]

The canonical runner is synchronous. Step limits are checked between dispatches, so they do not interrupt a blocked transport or tool. No global elapsed-time deadline, process-tree cleanup or propagated cancellation protocol was established in the inspected runner/tool surfaces. The provider layer normalizes configuration errors and external exceptions, but its behavior must be qualified separately from the loop's iteration caps. This is a simpler operating model than a subprocess-based coding workforce, with correspondingly different lifecycle demands.

`TraceLogger` appends schema-validated NDJSON using a fresh append-mode file handle per event. It has start/end suppression and spans, but no hash chain, signature, transaction joining the effect with the event, or explicit fsync protocol in the reviewed implementation. The `state.diff` and lineage events aid offline debugging; a fresh `run_loop_v1` invocation creates new state rather than restoring from that log. It is not an established pure journal fold or crash-recovery engine. A hash on prompts/evidence identifies content; it does not turn the whole trace into an authenticated ledger. [S1], [S16]

## Maintenance, operation and Brokkr relevance

The MIT package is version 0.1.0, targets Python 3.10+, depends only on PyYAML in its core, and has optional OpenAI and pytest extras. The `interagents` CLI and phase/golden-path documents provide a small setup surface and reproducible local demos. The inspected recursive tree contained no `.github/workflows` CI file; local test instructions therefore should not be reported as verified hosted CI. Default head is July 2026. These are repository observations, not adoption or reliability ratings. [S17], [S13]

Compared with Brokkr, InterAgents is a particularly clear reference for runner ownership, explicit control-plane STOP and evidence provenance in a narrow domain. It lacks an established counterpart to Brokkr's pinned recipe closure, SQLite hash journal and delivery/review policy. Conversely, its tests can suggest focused negative cases for Brokkr #429/#430: force illegal writer routing, inject a STOP alongside conflicting data, forward the same evidence packet twice, and ensure no agent dispatch follows terminality.

A useful qualification experiment should keep mocked transports first. Introduce a blocking tool to show that step caps are not time caps; remove a provider acknowledgement to expose retry ambiguity; change a prompt while preserving its role name and observe recorded identity; reverse simulated branch completion and verify deterministic aggregation. Treat its explicit serial fan-in experiment as a design exercise to learn from, not a demonstrated parallel runtime.

## Source evidence ledger

The following are immutable source links. Tests establish the cases maintainers encode; their presence does not establish a passing run in this investigation.

| ID | Source and inspection purpose |
|---|---|
| S1 | [interagents/core/runner_v1.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/runner_v1.py) — canonical loop, stop precedence and runner-owned state |
| S2 | [interagents/core/orchestrator.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/orchestrator.py) — recipient dispatch and exception wrapping |
| S3 | [interagents/core/agents/supervisor.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/agents/supervisor.py) — policy-first routing and structured decisions |
| S4 | [interagents/core/policies/stop.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/policies/stop.py) — hard iteration/step/critic-failure caps |
| S5 | [interagents/core/policies/transitions.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/policies/transitions.py) — serial transition catalogue and deterministic fan-in rules |
| S6 | [interagents/core/experiments/fanout_fanin.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/experiments/fanout_fanin.py) — serial fake branch experiment and aggregation validation |
| S7 | [interagents/core/system_messages.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/system_messages.py) — role prompt configuration |
| S8 | [interagents/core/llm_client.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/llm_client.py) — transport, prompt/response identity and trace metadata |
| S9 | [interagents/core/agents/writer.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/agents/writer.py) — citation membership checks |
| S10 | [interagents/core/state.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/state.py) — event application and evidence state ownership |
| S11 | [tests/test_v1_stop_is_terminal_no_post_dispatch.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/tests/test_v1_stop_is_terminal_no_post_dispatch.py) — STOP prevents further calls |
| S12 | [tests/test_v1_writer_requires_evidence.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/tests/test_v1_writer_requires_evidence.py) — writer containment regression |
| S13 | [docs/phase_13_real_llm_integration_confidence.md](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/docs/phase_13_real_llm_integration_confidence.md) — explicit limits of live smoke and offline testing |
| S14 | [interagents/tools/runner.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/tools/runner.py) — tool input tracing and failure normalization |
| S15 | [interagents/services/retry.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/services/retry.py) — attempt bounds, jitter and no-op default sleeper |
| S16 | [interagents/services/trace.py](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/services/trace.py) — unhashed NDJSON writer and spans |
| S17 | [pyproject.toml](https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/pyproject.toml) — license, dependencies, Python version and CLI |

[S1]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/runner_v1.py

[S2]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/orchestrator.py

[S3]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/agents/supervisor.py

[S4]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/policies/stop.py

[S5]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/policies/transitions.py

[S6]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/experiments/fanout_fanin.py

[S7]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/system_messages.py

[S8]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/llm_client.py

[S9]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/agents/writer.py

[S10]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/core/state.py

[S11]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/tests/test_v1_stop_is_terminal_no_post_dispatch.py

[S12]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/tests/test_v1_writer_requires_evidence.py

[S13]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/docs/phase_13_real_llm_integration_confidence.md

[S14]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/tools/runner.py

[S15]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/services/retry.py

[S16]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/interagents/services/trace.py

[S17]: https://github.com/jgzadidiLGDY/InterAgents/blob/d0cba222108410112db271037c008cc6f6836c5a/pyproject.toml
