# bradygaster/squad: implementation dossier

Research date: 2026-09-27 (Europe/Sofia). Live metadata retrieved 2026-09-26T22:00:46.590881+00:00. Default branch: `dev`. Revision: [`0f2586ea7ca51c0cdbf91a09b1b8911f653a643b`](https://github.com/bradygaster/squad/tree/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b); commit date: 2026-09-25T14:28:46Z. Repository license: [MIT](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/LICENSE), recorded without a legal interpretation.

Method: read-only inspection of a complete archive fetched by this exact SHA from GitHub, plus live repository metadata. No downloaded code, tests, installation commands, or provider workloads were executed. Tests below are inspected test assertions, not independently observed passes. This is an architectural comparison, not a numerical quality or security rating.

## What the project establishes

Squad is more than Markdown prompts: it has a TypeScript SDK/CLI, charter compilation, routing, multiple agent spawn backends, team-state storage, scheduling, model/session adapters and CI automation. It is the most literal charter-authoring peer among these six. The inspected default branch is `dev`; source features must not automatically be attributed to the latest published release. The package identifies itself as 0.13.1, requires Node 22.5+, and depends on the GitHub Copilot SDK. [Packages][S1], [SDK dependencies][S2].

The main authoring model is a persistent team directory containing roster, routing, decisions, agent charters/history and skills. The reusable charter separates identity, expertise, ownership, approach, boundaries, model preference and collaboration habits. It instructs an agent to consult shared decisions and submit new ones to a Scribe inbox. This is useful human-readable authority and memory organization. It is not, by itself, an immutable run constitution or a deterministic transition policy. [Charter template][S3], [architecture][S4].

## Charter and routing semantics

`compileCharterFull` parses Markdown and combines the original content with team context, routing, decisions, plugin context and optional extra prompt. Configuration overrides choose role/model/reasoning/context settings. Invalid reasoning/context values produce warnings and are ignored. Empty content receives a generic fallback prompt; the function does not itself read the `charterPath` argument, so the documentation's missing-charter exception should not be interpreted as proof this function refuses an absent file. Callers own loading and required-content validation. [Compiler][S5].

Another concrete boundary: the compiler comment says tools resolve from configuration or charter, but the actual assignment uses `configOverrides?.tools` and otherwise passes `null`. No translation of prose “Boundaries” into filesystem/shell permissions is shown in this function. Those paragraphs shape model instructions; executable restrictions must arrive through other configuration and tool hooks. This distinction is central to an honest comparison with Brokkr's protected review policy. [Compiler return][S5].

Routing Markdown is compiled into typed work-type/issue rules and regex matchers with priorities and fallback agents. That is real deterministic routing support around a human/LLM coordinator; it is not a delivery graph with an authoritative event-derived phase. Shared decisions and history can influence future compiled prompts or newly created sessions. The inspected charter/shell paths do not bind every such input to a run manifest with resume refusal on changed content. The earlier question about in-flight authority should therefore be tested at session creation/resume boundaries, rather than assuming every file edit hot-reloads into all active sessions. [Routing][S6], [session creation][S7].

## Permissions: available hooks versus live shell wiring

The SDK offers `HookPipeline` with file-write path matching, blocked shell-command strings, ask-user rate limits, email/PII scrubbing and optional reviewer lockout. The unit tests assert refusal of an unmatched write path, a listed dangerous command and a write by a locked-out agent. Those are implemented components. They do not establish a comprehensive sandbox: the file guard recognizes a short list of tool names and argument keys; unknown tools or missing paths pass, shell blocking is substring matching, and reviewer lockout matches artifact/file substrings in an in-memory map. [Hooks][S8], [hook tests][S9].

The local interactive CLI uses `approveAllPermissions`, returning `approve-once` for every permission request, and wires that callback into direct-agent session creation. No `HookPipeline` invocation was found in this shell file. The coordinator stores an optional hook pipeline but the inspected file's references only assign/reset it; no pre/post hook execution was found there. Consequently it is inaccurate to treat optional SDK hooks as an established permission boundary on every ordinary Squad CLI tool call. A host can integrate those hooks, but that integration is a separate evidence requirement. [Permission callback][S10], [direct session][S7], [coordinator][S11].

The secret-protection test file is a useful caution about test evidence. It explicitly says implementation is pending; `.env` denial and several secret-redaction scenarios are `it.todo`. Active tests mostly establish backward-compatible allowances or non-secret behavior. The current `PolicyConfig` exposes `scrubPii`, not an implemented `scrubSecrets` property. Counting that file as delivered secret protection would overstate the source. [Secret test status][S12], [policy fields][S8].

Filesystem state storage separately checks normalized paths and realpaths, including existing ancestors when the target does not yet exist. Its comment acknowledges that OS isolation is needed for complete containment. This is a meaningful safeguard for storage operations, and should not be generalized into containment of all agent shell activity. [Storage provider][S13].

## Process/session lifetime and recovery

Spawn abstraction has meaningful concurrency accounting. The App backend counts active IDs plus pending creations before admitting a spawn, rejects at the cap with an explicit “no queue configured” error, decrements pending count in `finally`, and releases active IDs when the caller reports completion. A default sixty-second timeout stops a hung factory from holding its bookkeeping slot forever. [Spawn backend][S14].

That timeout is implemented as `Promise.race`; it does not cancel the underlying factory promise or clean up a session that appears after the timeout. App creation includes kickoff work, so a late-created session is a concrete uncertainty case: the caller can see failure and release capacity while the remote work may still begin. The provided `SpawnedSession` interface lacks an abort/destroy method. This is a source-derived risk requiring a controlled late-resolution experiment, not proof that the actual Copilot service duplicates work. The task backend also sends the initial message after creation without the same explicit deadline wrapping that send. [Timeout helper and interfaces][S14].

`SessionPool` is primarily bookkeeping: it stores IDs/statuses, rejects at capacity, removes idle records, and clears maps/timers on shutdown. It does not hold an underlying session object to abort or destroy during that shutdown. Adapter APIs expose abort elsewhere, so an application may implement fuller cleanup; pool removal should not be described as confirmed process/session settlement. Scheduled-task retry similarly retries unsuccessful provider executions with exponential delay, without establishing effect idempotency. [Session pool][S15], [scheduler][S16].

## Durable state and maintainer evidence

Squad's state is more capable than “editable Markdown only.” Pluggable backends include filesystem state, Git notes, orphan branches and combined layers. The Git-notes path constructs blob/tree/commit objects and uses compare-and-swap ref updates; conflicts re-read the latest state and retry with a bound, then throw `StateBackendConcurrencyError`. Tests deliberately inject concurrent ref advancement, exhaustion and non-CAS failures. This is concrete concurrency handling and reviewable Git history. It still does not establish that every runtime transition is a pure fold over an immutable, complete, authenticated event journal. Git refs and team files remain administration surfaces. [State backends][S17], [CAS tests][S18].

The extensive CI workflow includes build, TypeScript and ESLint checks, team-isolation verification, Vitest, dependency/workflow checks, exports smoke tests and browser infrastructure. Test names and TODO status must be examined rather than inferring coverage from repository size. No tests or provider-backed sessions were run in this review. The SDK/CLI/module spread also means a maintenance experiment should follow one feature from charter through CLI session construction, not stop at its exported helper. [CI][S19].

## Brokkr lessons and experiments

For Brokkr #432/#437, adopt discoverable charter sections and explain exactly which fields are instructions, settings or enforced constraints. For #429, require each policy hook to be exercised through the shipping entrypoint. For #403/#433, combine capacity accounting with cancellation of underlying work; bookkeeping release is not settlement. For #431, compare CAS state recovery with Brokkr's journal-authoritative restore without presuming either is fully qualified.

Use a fake session factory that resolves after the configured spawn timeout and records kickoff. Verify whether the runtime leaks work or consumes more than the configured effective concurrency. Route a write restriction through the actual CLI session callback and check whether it is denied; repeat through alternate tool names and shell writes. Modify decisions/charter between initial creation and resume, then inspect which prompt/version is recorded. Finally, inject competing Git-state writers and an exhausted CAS budget. These bounded experiments separate Squad's valuable authoring/state mechanisms from the stronger enforcement claims an operator might infer from the word “charter.”

## Pinned evidence ledger

| ID | Source | Evidence role |
|---|---|---|
| S1 | [package.json](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/package.json#L1) | Package version, runtime and maintainer commands |
| S2 | [packages/squad-sdk/package.json](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/package.json#L236) | SDK dependency surface |
| S3 | [.squad-templates/charter.md](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/.squad-templates/charter.md#L1) | Literal charter responsibilities and boundaries |
| S4 | [docs/src/content/docs/concepts/architecture.md](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/docs/src/content/docs/concepts/architecture.md#L1) | Team/state architecture |
| S5 | [packages/squad-sdk/src/agents/charter-compiler.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/agents/charter-compiler.ts#L131) | Actual compiler inputs and tool assignment |
| S6 | [packages/squad-sdk/src/config/routing.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/config/routing.ts#L1) | Typed routing compilation |
| S7 | [packages/squad-cli/src/cli/shell/index.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-cli/src/cli/shell/index.ts#L400) | Ordinary direct-agent session creation |
| S8 | [packages/squad-sdk/src/hooks/index.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/hooks/index.ts#L56) | Optional heuristic guards and lockout |
| S9 | [test/hooks.test.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/test/hooks.test.ts#L17) | Active hook refusal tests |
| S10 | [packages/squad-cli/src/cli/shell/index.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-cli/src/cli/shell/index.ts#L85) | Approve-all callback |
| S11 | [packages/squad-sdk/src/coordinator/index.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/coordinator/index.ts#L102) | Optional hook reference integration |
| S12 | [test/hooks-security.test.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/test/hooks-security.test.ts#L1) | Explicit TDD/TODO secret protection status |
| S13 | [packages/squad-sdk/src/storage/fs-storage-provider.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/storage/fs-storage-provider.ts#L40) | Storage-only path containment |
| S14 | [packages/squad-sdk/src/coordinator/spawn-backend.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/coordinator/spawn-backend.ts#L54) | Timeout, pending count and session interface |
| S15 | [packages/squad-sdk/src/client/session-pool.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/client/session-pool.ts#L62) | Session bookkeeping and shutdown |
| S16 | [packages/squad-sdk/src/runtime/scheduler.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/runtime/scheduler.ts#L517) | Scheduled-task retry policy |
| S17 | [packages/squad-sdk/src/state-backend.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/state-backend.ts#L447) | Git CAS state updates |
| S18 | [test/state-backend.test.ts](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/test/state-backend.test.ts#L1469) | Concurrent ref advancement and exhaustion tests |
| S19 | [.github/workflows/squad-ci.yml](https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/.github/workflows/squad-ci.yml#L170) | Shipped maintainer gates |

[S1]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/package.json#L1
[S2]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/package.json#L236
[S3]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/.squad-templates/charter.md#L1
[S4]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/docs/src/content/docs/concepts/architecture.md#L1
[S5]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/agents/charter-compiler.ts#L131
[S6]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/config/routing.ts#L1
[S7]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-cli/src/cli/shell/index.ts#L400
[S8]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/hooks/index.ts#L56
[S9]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/test/hooks.test.ts#L17
[S10]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-cli/src/cli/shell/index.ts#L85
[S11]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/coordinator/index.ts#L102
[S12]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/test/hooks-security.test.ts#L1
[S13]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/storage/fs-storage-provider.ts#L40
[S14]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/coordinator/spawn-backend.ts#L54
[S15]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/client/session-pool.ts#L62
[S16]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/runtime/scheduler.ts#L517
[S17]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/packages/squad-sdk/src/state-backend.ts#L447
[S18]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/test/state-backend.test.ts#L1469
[S19]: https://github.com/bradygaster/squad/blob/0f2586ea7ca51c0cdbf91a09b1b8911f653a643b/.github/workflows/squad-ci.yml#L170
