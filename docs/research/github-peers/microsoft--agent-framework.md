# microsoft/agent-framework: implementation dossier

Research date: 2026-09-27. Default branch: `main`. Inspected revision: [`6f1522a50b66f117da34cc25ea299ba24a528b15`](https://github.com/microsoft/agent-framework/tree/6f1522a50b66f117da34cc25ea299ba24a528b15), committed 2026-09-25T18:52:55Z. License: MIT. GitHub metadata and raw sources were retrieved live; the revision matches the earlier survey. This is source inspection, not a benchmark, security certification, or report of tests executed. No project code was installed or run.

## Scope and architecture

Microsoft Agent Framework is a broad agent SDK and workflow system. This dossier follows the Python core workflow engine in depth and inspects selected .NET shell/declarative surfaces; it does not claim implementation parity across languages or all hosting extensions. The Python `WorkflowBuilder` connects typed executors and agents through ordinary, conditional, fan-out and fan-in edges. A runner delivers messages in supersteps, commits shared state at their boundaries, and checkpoints executor/edge state when storage is configured. Declarative YAML is a separate authoring layer; the inspected YAML sample documentation specifically demonstrates the .NET builder. [S1], [S2], [S3]

```text
builder/typed handlers → graph validation → initial input checkpoint
 → concurrent edge delivery → executor outputs/messages
 → shared-state commit + executor/edge snapshots → checkpoint
 → next superstep / external-input pause / completion / exception
```

This is a genuine general graph engine. Messages through a particular edge runner preserve order; deliveries over different edge runners can proceed concurrently. Multiple sources targeting an executor are not evidence of a single deterministic global arrival order. The runner has a default maximum of 100 iterations and raises a convergence exception if messages remain at the limit. That bounds supersteps, not the wall time of a blocked handler. [S2]

## Validation and reusable agent contracts

Graph validation checks executor/edge duplication, type compatibility, connectivity and output designations. Typed executor handlers and workflow contexts make message and response requirements explicit. Agent wrappers adapt agent sessions and request/response behavior into those contracts. None of this automatically constitutes a project charter or mandatory independent review; instructions, tools, middleware and the chosen workflow jointly determine authority. A Brokkr-equivalent delivery policy would be application code/configuration layered on top. [S1], [S4]

A concrete resume check goes beyond generic serialization. The workflow computes a canonical SHA-256 graph signature from executor IDs and class names, nested workflow structure, edge groups, source/target IDs, condition names and selection-function names. Restoration refuses a different graph signature. Tests exercise both top-level and nested-workflow mismatch. The fingerprint deliberately captures topology, not complete configuration: changing the body of an executor, a model instruction or implementation behind the same named condition need not change it. It is therefore a useful structural guard, not a dependency/charter lock equivalent to Brokkr's pinned manifest. [S5], [S6]

## Cancellation and recovery boundaries

The engine explicitly handles a subtle resume hazard: ordinary `asyncio.gather` can propagate one exception while sibling work continues. `gather_cancelling_siblings_on_error` catches even cancellation, cancels sibling tasks and awaits their settlement before reraising. The runner also cancels/drains its active iteration and event waiter when streaming closes. These paths aim to prevent failed-superstep output from arriving after state has already been restored. [S2], [S7]

`test_runner_orphaned_delivery_cannot_repopulate_a_restored_fan_in_buffer` distinguishes stale and fresh payloads, delays one delivery, fails another, restores an earlier checkpoint and checks that the old message does not contaminate the restored join. A related fan-out test protects the pending-message queue. Other tests restore partially populated fan-in buffers and verify cancellation of an active executor. These are targeted evidence of lifecycle engineering, with clearer relevance to Brokkr #403/#431 than broad “durability” claims. They still concern cooperative Python tasks: a handler that blocks the event loop or starts an unmanaged external process has additional lifecycle requirements. [S8]

Checkpoint construction saves shared state, executor hooks, in-flight messages, edge buffers and pending requests. Save hooks must produce a dictionary with string keys; restore refuses missing executors and malformed state. Graph mismatch is checked before restoration. An initial checkpoint captures input before execution, followed by checkpoints after successful supersteps. Effects completed in an uncheckpointed superstep may consequently need replay after failure; the base engine does not establish exactly-once external writes or a generic uncertain-effect reconciliation protocol. [S2], [S6]

## Persistence and integrity

The file checkpoint store validates path containment and round-trips encoded content through its decoder before accepting a save. Its decoder restricts object reconstruction to allowed types, with application registration for additional types. Saves use unique temporary files and `os.replace`, while a process-wide destination queue preserves same-path write ordering without exhausting the executor with blocked writers. Cancellation waits for the underlying write's completion before releasing that destination, and Windows replacement failures receive a bounded retry. [S9]

This is a substantial persistence implementation, but its exact guarantee is limited. The inspected atomic-write function closes JSON and replaces the file; it does not call file/directory fsync. Process-local queues do not constitute an inter-process single-writer lock. Checkpoint IDs, parent links and topology hashes do not form an authenticated append-only journal. Backends and hosting services may strengthen durability, but the general SDK claim cannot silently inherit their guarantees. Tests cover cancelled saves, abandoned loops, concurrent writers, replacement failures and cleanup failures; they should be viewed as specified cases rather than proof of every storage failure model. [S9], [S10]

## Tools, permissions and secrets

The SDK is not intrinsically a sandbox: arbitrary executors and user tools run as code. Its optional shell package nevertheless contains a notable explicit boundary. `LocalShellExecutor.AsAIFunction` defaults to approval gating and refuses to create a non-gated function without an explicit unsafe acknowledgement. The associated `ShellPolicy` describes regex filters as convenience prefilters and makes no claim that they resist interpreter escapes. Empty default deny/allow configuration permits nonempty commands; explicit empty allowlists deny all. This is a useful separation of filter semantics from a real authority boundary. [S11], [S12]

For stateless local shell execution, timeout/cancellation triggers process-tree kill; the timeout path then waits for exit and drains output readers. Timeout is optional: the recommended constant is not an implicit deadline. Approval depends on the embedding invocation/approval pipeline, and the source warns that colliding tool names can interact with auto-approval rules. Environment values can be supplied, but this selected implementation does not establish secret minimization across all provider, remote MCP and checkpoint integrations. The shell mechanisms must not be generalized to every Python node or .NET tool. [S11]

## Maintenance and operation

Python core at this pin declares version 1.19.0, MIT and Python 3.10+, with bounded msgspec/Pydantic/YAML/OpenTelemetry dependencies and many separately optional provider/hosting packages. The repository includes .NET, Python and declarative examples, plus distinct durable/cloud extensions. This modularity supports custom adapters while increasing the operational choices a deployment must make: provider authentication, storage backend, tool permissions and durable host are not resolved by choosing a workflow builder alone. [S13], [S3]

Python CI covers Ubuntu and Windows, versions 3.10–3.15, with 3.15 allowed to fail at this pin and a comment excluding macOS pending resolution. .NET has its own build/test workflow. Such source evidence is more useful than a single maintenance grade; this review did not verify passing workflow runs or SDK release parity. Recent default-head activity is metadata, not proof of production qualification. [S14], [S15]

## Brokkr implications and experiments

MAF is a strong reference for typed edge validation, nested checkpoint state, cancellation before restore and storage conformance. Brokkr's reusable charters and composed recipes are closer to a domain policy package than to MAF's generic agent abstraction. Its pure journal fold and protected review remain separate comparisons; Brokkr's own outstanding policy/lifecycle/restore work means neither system should receive blanket reliability claims.

The most informative experiments are: fail one branch while another queues a delayed write, restore, and verify no stale state; alter graph topology versus only prompt text and observe what resume refuses; cancel a file save at each phase and distinguish atomic visibility from crash durability; and kill after an external write but before the superstep checkpoint. Adopt the refusal messages and adversarial tests even if Brokkr keeps its linear outer execution model.

## Source evidence ledger

The following are immutable source links. Tests establish the cases maintainers encode; their presence does not establish a passing run in this investigation.

| ID | Source and inspection purpose |
|---|---|
| S1 | [python/packages/core/agent_framework/_workflows/_workflow_builder.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_workflow_builder.py) — typed workflow and agent composition |
| S2 | [python/packages/core/agent_framework/_workflows/_runner.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_runner.py) — supersteps, limits, checkpoint and restore state |
| S3 | [declarative-agents/workflow-samples/README.md](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/declarative-agents/workflow-samples/README.md) — YAML workflow authoring scope |
| S4 | [python/packages/core/agent_framework/_workflows/_validation.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_validation.py) — graph/type/output validation |
| S5 | [python/packages/core/agent_framework/_workflows/_workflow.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_workflow.py) — topology-only graph signature and lifecycle entry |
| S6 | [python/packages/core/tests/workflow/test_checkpoint_validation.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/tests/workflow/test_checkpoint_validation.py) — top-level/nested graph mismatch refusals |
| S7 | [python/packages/core/agent_framework/_workflows/_edge_runner.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_edge_runner.py) — sibling cancellation and settlement helper |
| S8 | [python/packages/core/tests/workflow/test_runner.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/tests/workflow/test_runner.py) — stale delivery, fan-in restore and active cancellation regressions |
| S9 | [python/packages/core/agent_framework/_workflows/_checkpoint.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_checkpoint.py) — atomic file store, path check, decoder preflight and write queues |
| S10 | [python/packages/core/tests/workflow/test_checkpoint.py](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/tests/workflow/test_checkpoint.py) — checkpoint cancellation and storage failure tests |
| S11 | [dotnet/src/Microsoft.Agents.AI.Tools.Shell/LocalShellExecutor.cs](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/dotnet/src/Microsoft.Agents.AI.Tools.Shell/LocalShellExecutor.cs) — approval opt-out refusal and process-tree timeout cleanup |
| S12 | [dotnet/src/Microsoft.Agents.AI.Tools.Shell/ShellPolicy.cs](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/dotnet/src/Microsoft.Agents.AI.Tools.Shell/ShellPolicy.cs) — bounded regex policy with explicit security limits |
| S13 | [python/packages/core/pyproject.toml](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/pyproject.toml) — version, platform/dependency bounds and optional integrations |
| S14 | [.github/workflows/python-tests.yml](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/.github/workflows/python-tests.yml) — Python/OS matrix and experimental-version caveat |
| S15 | [.github/workflows/dotnet-build-and-test.yml](https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/.github/workflows/dotnet-build-and-test.yml) — separate .NET validation workflow |

[S1]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_workflow_builder.py

[S2]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_runner.py

[S3]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/declarative-agents/workflow-samples/README.md

[S4]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_validation.py

[S5]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_workflow.py

[S6]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/tests/workflow/test_checkpoint_validation.py

[S7]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_edge_runner.py

[S8]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/tests/workflow/test_runner.py

[S9]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/agent_framework/_workflows/_checkpoint.py

[S10]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/tests/workflow/test_checkpoint.py

[S11]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/dotnet/src/Microsoft.Agents.AI.Tools.Shell/LocalShellExecutor.cs

[S12]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/dotnet/src/Microsoft.Agents.AI.Tools.Shell/ShellPolicy.cs

[S13]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/python/packages/core/pyproject.toml

[S14]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/.github/workflows/python-tests.yml

[S15]: https://github.com/microsoft/agent-framework/blob/6f1522a50b66f117da34cc25ea299ba24a528b15/.github/workflows/dotnet-build-and-test.yml
