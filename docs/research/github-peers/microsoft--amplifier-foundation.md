# microsoft/amplifier-foundation: implementation dossier

Research date: 2026-09-27 (Europe/Sofia). Live metadata retrieved 2026-09-26T22:00:45.852986+00:00. Default branch: `main`. Revision: [`89575c3482e3e8afe5a03df72e723cf815fa1f6c`](https://github.com/microsoft/amplifier-foundation/tree/89575c3482e3e8afe5a03df72e723cf815fa1f6c); commit date: 2026-09-25T13:00:29Z. Repository license: [MIT](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/LICENSE), recorded without a legal interpretation.

[Inspection method and limits](../0019-github-peer-implementation-investigations.md#method-and-limits) apply to this dossier.

## What the project establishes

Foundation is a Python composition and session-support library with reusable agents, behaviors, provider declarations, and contextual Markdown. It is a strong peer for Brokkr's configuration and role composition. The deeper inspection adds substantial ownership and lifecycle machinery that the initial survey omitted. These mechanisms belong to particular Foundation APIs; they do not automatically govern every effect issued by Recipes or an embedding host.

The architectural flow is: load a bundle from local or remote sources → compose overlays → validate and prepare modules → create an Amplifier Core session → execute/delegate → retain session history or an explicitly acquired shared checkpoint. The agent's Markdown body becomes instruction context, while its frontmatter selects modules, tools and model roles. An example architect declares concrete filesystem, search and LSP modules whose sources follow `@main`; the role prose alone is not a frozen authority boundary. [Bundle implementation][F1], [agent example][F2].

## Composition, identity and validation

`Bundle.compose` is deliberately order sensitive: session/spawn mappings merge, module lists merge by identifier, and later agents replace earlier agents with the same name. This is useful authoring ergonomics, but it means a governance recipe must decide which override surfaces are legitimate. Foundation does not supply Brokkr's constitutional distinction between protected review authority and ordinary configuration merely by composing an agent named reviewer. Its separate provenance module records contributing bundle/behavior origins for modules, agents and other resources. That explains composition ancestry; it should not be described as a cryptographic attestation that a particular reviewer ran or that every override was authorized. [Composition][F1], [provenance][F3].

Structural validation checks required fields, module entry shapes, session configuration and resources. Missing context files are warnings in normal mode and errors in strict mode. Completeness validation is a distinct operation because partial provider/behavior/agent bundles are intentionally valid building blocks. This separation is a good fit for Brokkr's reusable fragments, provided the final assembled run is subject to a stricter gate than an individual fragment. [Validator][F4].

The package requires Python 3.11+, Amplifier Core, PyYAML and filelock, with a gRPC adapter extra and a development lockfile. `prepare` is an activation surface, not just a parser: it resolves module sources and can install dependencies. The reference source declarations frequently use moving branches. The Recipes lock/manifest layer must therefore be assessed separately from Foundation's convenient install-and-compose examples. [Package configuration][F5], [bundle prepare][F1].

## Ownership, durability and recovery

`SharedSessionStore` is more rigorous than ordinary editable team memory. Writing explicitly requires a POSIX local filesystem. It rejects unsafe ownership, symlinks, nonregular state files and group/other permissions; acquires a stable OS lock; and gives the holder a PID-bound, noncopyable capability. Owner JSON is advisory rather than the lock authority. Fork handling closes inherited descriptors, preventing a child from retaining the parent's authority. [Shared state][F6].

Checkpoint publication writes a private temporary file, flushes and fsyncs it, atomically replaces the target and fsyncs the parent directory. Unsupported directory-sync errors are treated differently from I/O, permission or space failures. This distinction matters: a visible file after a failed sync is not equivalent to an acknowledged durable checkpoint. The tests explicitly exercise real process contention, death/exec lock release, stale handles, fork inheritance, unsafe permissions and capability identity. [Atomic publication][F7], [ownership tests][F8].

The durable transfer fence is particularly relevant to Brokkr restore work. It survives process death and blocks ordinary acquisition by another adapter. A separate exact-transfer recovery capability cannot authorize execution; committing a source transfer keeps that source fenced. Tests cover corrupt markers, symlinked paths, mismatched receipts, failed directory synchronization, and retries that must re-establish durability before acknowledgement. This is established source-level support for preventing simultaneous source/destination execution, not a claim that the external host's receipt authentication or network transfer is implemented here. The code explicitly leaves receipt authentication to the host. [Transfer implementation][F6], [transfer tests][F9].

These are snapshot and ownership APIs, not an established equivalent of Brokkr's canonical append-only SQLite event sequence and pure fold. Foundation's transcripts and checkpoints do not by themselves reconstruct an authoritative delivery policy. Unsigned local state also does not authenticate an actor against a user who can rewrite the storage.

## Processes, cancellation and secrets

The subprocess session runner creates a private configuration file, checks the working directory, limits simultaneous subprocess sessions with a semaphore, filters inherited environment variables, and starts an isolated process group on POSIX. Windows uses a distinct process-group/taskkill path. On timeout it attempts tree termination and waits at most ten seconds for the direct process. Non-timeout unwinding, including task cancellation, also attempts a tree kill. This is a concrete lifecycle implementation, well beyond a prompt convention. [Subprocess runner][F10].

Its boundaries matter. The cancellation branch re-raises after initiating termination and does not show the same bounded reap used by the timeout branch. Both cleanup branches test the direct child's return code before killing the tree; success with detached descendants or a child that exited while grandchildren retained pipes requires a separate experiment. Capturing `communicate()` buffers output; the inspected function did not establish a total stdout/stderr memory limit. The environment allowlist intentionally forwards provider and GitHub/AWS credential prefixes, so filtering is protection against unrelated ambient variables, not a credential-free sandbox. Error redaction is pattern based. A nonzero child with a framed result is returned through the result extractor, meaning the caller must interpret that envelope correctly. [Lifecycle and environment][F10], [runner tests][F11].

An optional process-guard hook has a materially different design: before certain tools it invokes `pkill -f` for configured patterns, defaulting to pytest and node test patterns, and ignores kill errors. That is not ownership-scoped termination. Its existence should not be merged with the subprocess runner into a general guarantee that Foundation only kills descendants it owns. [Process guard][F12].

Delegation budgets and structured returns are configurable features. The delegate tests explicitly describe the call budget as disabled by default; per-call overrides and zero opt-out are tested. The structured-return feature is also disabled by default and its parser is permissive about partial returns. Reusable mandates do not imply strict, universally required evidence schemas. [Delegate implementation][F13], [budget tests][F14].

## Maintainer ergonomics and lessons for Brokkr

The repo separates composition, provenance, validation, session ownership, transfer fences and subprocess execution into inspectable modules. Tests include meaningful process and failure scenarios, not just serialization checks. CI spans Ubuntu/Windows and Python 3.11–3.13, with pinned action revisions and named test output. A scope caveat: the inspected CI command explicitly runs `pytest tests/`; `pyproject.toml` also names `modules/tool-delegate/tests` as a default test path, but an explicit directory argument does not establish that those additional tests ran in this workflow. Inspect actual workflow legs before reporting universal delegate coverage. [CI][F15], [package configuration][F5].

For Brokkr #403/#431, adapt the *test patterns*: PID-bound ownership, forked lock inheritance, crash-surviving transfer fencing, and failure after rename but before durable acknowledgement. For #432, retain origin explanations for every composed field and distinguish fragment validation from final-run completeness. For #433, test the actual output collector and opt-out budget semantics, rather than counting configured timeout fields.

A controlled comparison should load two bundles that define the same reviewer, record the winner and its provenance, then attempt a transfer with an injected fsync failure. A lifecycle fixture should emit a valid result, exit its leader, and leave a grandchild holding stdout. Compare both systems' settlement and evidence without using a provider. These experiments would establish the practical boundary of already visible mechanisms; the source review alone cannot certify them.

## Pinned evidence ledger

| ID | Source | Evidence role |
|---|---|---|
| F1 | [amplifier_foundation/bundle/_dataclass.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/bundle/_dataclass.py#L155) | Composition and activation |
| F2 | [agents/zen-architect.md](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/agents/zen-architect.md#L1) | Concrete reusable agent |
| F3 | [amplifier_foundation/bundle/_provenance.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/bundle/_provenance.py#L45) | Origin attribution |
| F4 | [amplifier_foundation/validator.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/validator.py#L51) | Validation versus completeness |
| F5 | [pyproject.toml](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/pyproject.toml#L1) | Dependencies and test discovery |
| F6 | [amplifier_foundation/session/shared_state.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/session/shared_state.py#L413) | Ownership and transfer APIs |
| F7 | [amplifier_foundation/session/shared_state.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/session/shared_state.py#L271) | Durable write sequence |
| F8 | [tests/test_shared_state.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/tests/test_shared_state.py#L82) | Real process and capability refusal tests |
| F9 | [tests/test_session_transfer_fence.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/tests/test_session_transfer_fence.py#L70) | Transfer durability and refusal scenarios |
| F10 | [amplifier_foundation/subprocess_runner.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/subprocess_runner.py#L405) | Environment and subprocess lifecycle |
| F11 | [tests/test_subprocess_runner.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/tests/test_subprocess_runner.py#L1) | Subprocess test scope |
| F12 | [modules/hooks-process-guard/amplifier_module_hooks_process_guard/__init__.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/modules/hooks-process-guard/amplifier_module_hooks_process_guard/__init__.py#L20) | Pattern based process cleanup |
| F13 | [modules/tool-delegate/amplifier_module_tool_delegate/__init__.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/modules/tool-delegate/amplifier_module_tool_delegate/__init__.py#L639) | Delegation defaults and return contract |
| F14 | [modules/tool-delegate/tests/test_delegate_call_budget.py](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/modules/tool-delegate/tests/test_delegate_call_budget.py#L88) | Budget defaults and overrides |
| F15 | [.github/workflows/ci.yml](https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/.github/workflows/ci.yml#L1) | CI invocation and platforms |

[F1]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/bundle/_dataclass.py#L155
[F2]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/agents/zen-architect.md#L1
[F3]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/bundle/_provenance.py#L45
[F4]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/validator.py#L51
[F5]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/pyproject.toml#L1
[F6]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/session/shared_state.py#L413
[F7]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/session/shared_state.py#L271
[F8]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/tests/test_shared_state.py#L82
[F9]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/tests/test_session_transfer_fence.py#L70
[F10]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/amplifier_foundation/subprocess_runner.py#L405
[F11]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/tests/test_subprocess_runner.py#L1
[F12]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/modules/hooks-process-guard/amplifier_module_hooks_process_guard/__init__.py#L20
[F13]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/modules/tool-delegate/amplifier_module_tool_delegate/__init__.py#L639
[F14]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/modules/tool-delegate/tests/test_delegate_call_budget.py#L88
[F15]: https://github.com/microsoft/amplifier-foundation/blob/89575c3482e3e8afe5a03df72e723cf815fa1f6c/.github/workflows/ci.yml#L1
