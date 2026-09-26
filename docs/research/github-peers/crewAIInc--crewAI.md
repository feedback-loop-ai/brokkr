# crewAIInc/crewAI: implementation dossier

Research date: 2026-09-27. Default branch: `main`. Inspected revision: [`4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec`](https://github.com/crewAIInc/crewAI/tree/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec), committed 2026-09-25T18:00:20Z. License: MIT. GitHub metadata and raw sources were retrieved live; the revision matches the earlier survey. This is source inspection, not a benchmark, security certification, or report of tests executed. No project code was installed or run.

## Scope and architecture

CrewAI combines role-oriented agents and tasks into crews, and also supplies a stateful Flow engine. At this revision the older `flow.py` path is a compatibility surface: decorators live in `flow.dsl`, the serializable contract in `flow_definition`, and execution in `flow.runtime`. This is more substantial than the earlier survey's brief “roles plus listeners” characterization. Flow Definitions can express code, tool, crew, agent, expression, script and per-item actions, with state and persistence configuration. [S1], [S2], [S3]

```text
role/goal/backstory + tasks → sequential/hierarchical Crew
                               ↑ invoked by Flow actions
Python decorators or serialized FlowDefinition
 → start methods → routers → listener groups → method outputs/state
 → completion persistence + events → pause/resume or next listeners
```

Flows have start/listen/router semantics, compound AND/OR conditions and cycles. The engine processes router outcomes and concurrently gathers ordinary triggered listeners; synchronous methods run through `asyncio.to_thread`. It retains completed methods, outputs, execution counts and resumption state, with a maximum-method-call guard against runaway cycles. A flow authored in code can make routing entirely programmatic, while a hierarchical crew or agent action delegates choices to a model. Neither surface forces Brokkr's protected-review semantics. [S3], [S4]

## Agent, task and definition contracts

Agents expose role-oriented configuration, tools, model choice, optional delegation/memory and execution/retry controls. Tasks have descriptions, expected outputs, optional structured output types and guardrails. Programmatic guardrails are callable validation; textual guardrails are converted to model-based checks. Both can trigger bounded retries. Therefore the same “guardrail” configuration word can denote deterministic code or another fallible model call. Review independence, evidence sufficiency and authority have to be assessed per task/flow rather than inferred from the role name. [S5], [S6]

Flow Definitions use Pydantic models, reject unknown fields on many action/state variants, validate method names and CEL expressions, and refuse a listener that references its own handler name. The source differentiates declarative expression evaluation from general code. Inline script execution is explicitly disabled unless opted in, but enabled scripts and imported Python actions still execute with the host's capabilities. A serialized definition is not automatically a safe, data-only policy file. [S2], [S7]

A concrete task refusal is guardrail exhaustion: after the configured maximum retries, task execution raises rather than silently forwarding the rejected output. Tests exercise a failing guardrail, recovery after retries, several guardrails in sequence and independent retry tracking. Those are useful checks of control behavior. They cannot establish that a permissive guardrail accurately judges correctness, or that repeated tool effects performed while producing another candidate are safe. [S6], [S8]

## Persistence and resume semantics

Two mechanisms deserve separate treatment. Flow persistence saves state snapshots on configured method completion, with SQLite support and a separate pending-feedback table. The implementation uses transactions, WAL mode and a named lock, records flow UUID/method/time/JSON, and loads the most recent row. The runtime records a method as completed and then invokes its persistence backend. A crash between an external effect and that persistence boundary is still an acknowledgement gap; SQLite alone cannot make a remote effect atomic with the snapshot. [S3], [S9]

The newer checkpoint system can capture runtime entities and events, records parent/branch lineage, supports JSON and SQLite providers, and can prune old checkpoints. The default automatic trigger is `task_completed`, while child entities may opt out or override inherited configuration. Runtime metadata includes CrewAI version and version-migration behavior. This is a meaningful restoration facility, but parent IDs and version fields are not a cryptographically chained, immutable authority manifest. No pure fold of an append-only execution journal equivalent to Brokkr's was established. [S10], [S11], [S12]

Failure policy is especially important. The low-level automatic checkpoint writer emits `CheckpointFailedEvent` and raises; its outer event handler catches that failure and logs a warning, allowing the execution path to continue. Pruning failure also warns. By contrast, explicitly configured method persistence is awaited through the method execution path. These should not be compressed into a blanket statement that checkpoint failure always aborts, or always succeeds. Operators who need completion to imply durable evidence must qualify the exact persistence mode. [S3], [S11]

One source test intentionally documents another permissive behavior: `test_restore_from_state_id_not_found_silent_fallback` supplies a nonexistent source UUID and expects a fresh default state, with a new ID. This is the state-fork/restore surface, not a claim that every checkpoint restore behaves this way. In a recovery-oriented tool, silently starting anew can be operationally surprising; its use should be explicit in the comparison with Brokkr's intended strict restore identity. [S13]

The listener-resumption regression tests check that completed methods are skipped and that normal cycles can execute again. One test simulates prior completion by assigning the private completed-method set; it is not a crash-injection test establishing that persisted state always reconstructs that set. Async human-feedback persistence is more detailed, including pending context and resumed routing. Distinguish the unit regression from full process-death recovery qualification. [S14], [S9]

## Lifecycle, permissions and secrets

The synchronous agent timeout submits work to `ThreadPoolExecutor`, waits on the future with a timeout, cancels it on expiry, then leaves the executor context. Source inspection implies that already-running thread work can continue and executor shutdown can wait for it; this investigation did not reproduce that behavior. It is a wait-bound mechanism, not proof of process-tree termination. The async path uses `asyncio.wait_for`, whose cancellation behavior depends on the awaited work. Flow listener concurrency and `to_thread` add another boundary between cancellation of an awaiter and settlement of the operation underneath. [S5], [S3]

The `ask` input path explicitly documents best-effort timeout and uses nonwaiting thread-executor shutdown, illustrating that even within this project timeout contracts differ. Method-call caps do not provide a global deadline for blocked tools. A fair lifecycle experiment must name the sync/async/tool path and inspect surviving work, rather than only measuring the exception seen by the caller. [S3]

CrewAI has hooks/interception points around execution and methods; authors can implement constraints there. But `SecurityConfig` itself currently contains fingerprint identity, while credentials, scoping and delegation tokens are marked TODO in that class. A fingerprint must not be reported as sandbox enforcement. Provider secrets, arbitrary tools, memory and checkpoint contents need deployment-specific treatment; this review did not establish a universal environment allowlist, secret-redaction guarantee or filesystem isolation. The default script refusal is a concrete boundary, and the generic security class is a limited identity surface. [S15], [S7]

## Maintenance and operational ergonomics

This MIT monorepo includes CLI scaffolding, visualization, provider/tools integrations and rich checkpoint/event inspection. The package targets Python 3.10 through 3.13 and pins matching core/CLI packages at 1.15.22 in the inspected manifest. Its dependency surface includes OpenAI, Instructor, Pydantic, telemetry, data/embedding stores, MCP, CEL and file/data libraries; provider/backend selection and dependency conflicts are relevant operating costs compared with a narrow Rust executable. Package versioning and component boundaries are concrete facts, not a quality grade. [S16]

The PR test workflow runs eight shards across Python 3.10–3.13 on Ubuntu and filters documentation/Actions-only changes out of this test matrix. It installs all groups/extras; other workflows cover lint/types and vulnerability scanning. This investigation read the workflow and test cases but did not run them, inspect passing CI results or assess provider output quality. The recent default-head commit indicates activity, not demonstrated production reliability. [S17]

## Brokkr implications and experiments

CrewAI is a useful reference for role/task authoring, serialized flow projection, interactive inspection and explicit pause context. Brokkr's phase policy, protected review, immutable run identity and journal fold remain additional requirements an application would need to supply. Brokkr's own unresolved lifecycle/restore/retry issues require the same scrutiny.

A focused comparison should fail an automatic checkpoint write and ask whether apparent success still has recoverable evidence; request restoration from an unknown identity; time out a blocking sync tool and observe both return latency and continued side effects; and repeat a task after guardrail failure with a fake external counter. Also change an imported action implementation while retaining its definition reference. These experiments separate attractive authoring ergonomics from reproducible authority and replay guarantees without treating every flexible default as a defect.

## Source evidence ledger

The following are immutable source links. Tests establish the cases maintainers encode; their presence does not establish a passing run in this investigation.

| ID | Source and inspection purpose |
|---|---|
| S1 | [lib/crewai/src/crewai/flow/flow.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/flow.py) — compatibility surface and runtime decomposition |
| S2 | [lib/crewai/src/crewai/flow/flow_definition.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/flow_definition.py) — serializable action/state contract and validation |
| S3 | [lib/crewai/src/crewai/flow/runtime/__init__.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/runtime/__init__.py) — router/listener scheduling, completion, pause and timeout paths |
| S4 | [lib/crewai/src/crewai/crew.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/crew.py) — crew process and orchestration |
| S5 | [lib/crewai/src/crewai/agent/core.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/agent/core.py) — agent configuration, retries and sync/async timeouts |
| S6 | [lib/crewai/src/crewai/task.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/task.py) — programmatic/model guardrails and exhaustion |
| S7 | [lib/crewai/src/crewai/flow/runtime/_actions.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/runtime/_actions.py) — action execution and default-disabled inline scripts |
| S8 | [lib/crewai/tests/test_task_guardrails.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/tests/test_task_guardrails.py) — failure/retry and multiple-guardrail tests |
| S9 | [lib/crewai/src/crewai/flow/persistence/sqlite.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/persistence/sqlite.py) — transactional snapshots, WAL and pending feedback |
| S10 | [lib/crewai/src/crewai/state/checkpoint_config.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/state/checkpoint_config.py) — automatic trigger defaults and inheritance |
| S11 | [lib/crewai/src/crewai/state/checkpoint_listener.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/state/checkpoint_listener.py) — automatic checkpoint failure handling |
| S12 | [lib/crewai/src/crewai/state/runtime.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/state/runtime.py) — runtime serialization, version and lineage |
| S13 | [lib/crewai/tests/test_flow_persistence.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/tests/test_flow_persistence.py) — unknown-state silent fallback and persistence cases |
| S14 | [lib/crewai/tests/test_flow_resumability_regression.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/tests/test_flow_resumability_regression.py) — completed listener and cyclic execution regressions |
| S15 | [lib/crewai/src/crewai/security/security_config.py](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/security/security_config.py) — fingerprint implementation and unimplemented scoping fields |
| S16 | [lib/crewai/pyproject.toml](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/pyproject.toml) — dependency and Python compatibility surface |
| S17 | [.github/workflows/tests.yml](https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/.github/workflows/tests.yml) — test sharding, matrix and path filter |

[S1]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/flow.py

[S2]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/flow_definition.py

[S3]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/runtime/__init__.py

[S4]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/crew.py

[S5]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/agent/core.py

[S6]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/task.py

[S7]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/runtime/_actions.py

[S8]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/tests/test_task_guardrails.py

[S9]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/flow/persistence/sqlite.py

[S10]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/state/checkpoint_config.py

[S11]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/state/checkpoint_listener.py

[S12]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/state/runtime.py

[S13]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/tests/test_flow_persistence.py

[S14]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/tests/test_flow_resumability_regression.py

[S15]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/src/crewai/security/security_config.py

[S16]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/lib/crewai/pyproject.toml

[S17]: https://github.com/crewAIInc/crewAI/blob/4ed2abc7bbf504a634d3b733f2a97e0fbe8d44ec/.github/workflows/tests.yml
