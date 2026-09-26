# strongdm/attractor: implementation dossier

Research date: 2026-09-27 (Europe/Sofia). Live metadata retrieved 2026-09-26T22:00:45.886971+00:00. Default branch: `main`. Revision: [`fb57a55ed97372a27ac90102f436947e29f48426`](https://github.com/strongdm/attractor/tree/fb57a55ed97372a27ac90102f436947e29f48426); commit date: 2026-03-17T21:33:04Z. Repository license: [Apache-2.0](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/LICENSE), recorded without a legal interpretation.

[Inspection method and limits](../0019-github-peer-implementation-investigations.md#method-and-limits) apply to this dossier.

## Scope and architecture

Attractor is a specification repository, not a runnable reference implementation. The complete pinned archive contains README, license and three natural-language specifications: graph orchestration, coding-agent loop and unified LLM client. The README instructs a coding agent to implement those documents. There is no project build manifest, implementation test suite or CI workflow in this archive. The material is detailed enough to generate useful test requirements, but its guarantees must remain **specified**, not credited as observed execution. Kilroy is one separate implementation with additional choices and deviations. [Repository scope][A1].

The proposed stack has three boundaries: a DOT graph engine chooses nodes and routes; a programmable coding-agent library drives tool/model turns; a unified client normalizes provider requests, responses, retries and streams. The graph layer can use alternative agent backends instead of implementing both companion specs. That modularity resembles Brokkr's outer deterministic policy plus fallible agent effects, while leaving substantially more graph and provider behavior open to each implementer. [Graph layering][A2], [coding library][A3].

A material correction to the first comparison is needed: §3.8 specifies **one active top-level node at a time**. Parallel/fan-in handlers own concurrent branches, each with cloned context, and only handler outcome/context updates return to the parent. Attractor differs from Brokkr's fixed linear phase machine through arbitrary branching, retry jumps and re-entry, not necessarily through concurrent top-level phase authority. Whether a particular implementation preserves that constraint must be checked separately. [Concurrency model][A4].

## Graph semantics and validation

The DOT subset carries graph/node/edge attributes, default blocks and subgraphs. Shapes map to handler types for start, exit, coding, human approval, conditionals, parallel/fan-in and tools. Model stylesheets provide an additional selection layer. A maintainer can author a visible workflow without hard-coding every route, but correctness also depends on attribute precedence, transforms, handler registry and the condition language. These surfaces increase conformance obligations compared with Brokkr's deliberately closed phase vocabulary. [DSL and attributes][A2].

Routing has an explicit precedence: true conditional edges; normalized preferred label; suggested next-node IDs; maximum edge weight; lexical tie-breaking. Conditions use a small equality/inequality conjunction language rather than arbitrary host code. This is deterministic conditional on node outcomes/context; the outcome and its preferred route can still be model-produced. Route determinism does not establish trustworthy input evidence. [Routing algorithm][A5].

Goal gates deserve a narrower description than “mandatory verification.” The specified terminal check iterates **visited node outcomes**, and accepts SUCCESS or PARTIAL_SUCCESS. A declared gate that the graph never visited is not included by this algorithm. Failed visited gates jump through node/graph retry targets, or fail if none exists. This is useful completion gating, but it does not inherently force all paths to pass a review node or require a typed verifier receipt. A compiler could add that invariant; the inspected specification does not make it automatic. [Goal gates][A6].

Validation prescribes errors for malformed topology, missing targets, unreachable nodes, start/exit constraints, condition syntax and stylesheet syntax. Unknown handler types, missing prompts and invalid retry targets are warning-level rules in the listed table. Consequently “validated graph” needs its diagnostic severity policy stated. An implementation that treats warnings as fatal offers a stronger authoring gate than the minimal listed rules. [Lint rules][A7].

## Failure, retry, cancellation and recovery requirements

The retry model distinguishes retryable transport/provider errors from authentication, configuration and validation errors. `max_retries` means additional attempts; default is zero. Exhausted RETRY can become PARTIAL_SUCCESS when `allow_partial` is set. Failure routing follows explicit fail edges, retry target, fallback target and then pipeline failure. Separate API-client retries and node retries can multiply effective attempts, so equivalent Brokkr recipes must compare combined budgets rather than the same numeric field. [Retry and failure routing][A8].

Human nodes have explicit timeout/skip behavior and configurable default choice. A timeout without a default produces RETRY in the illustrated handler. This means an approval pause is an application choice with possible timeout assumptions; it is not necessarily Brokkr's protected review decision. Pre-tool hooks may refuse tool execution by exiting nonzero, while post-tool hook failures are logged without blocking an already-completed tool. The latter is an explicit example of audit failure not rolling back an effect. [Human handler][A9], [tool hooks][A10].

Checkpointing is a JSON snapshot after each completed node: last completed node, ordered completed IDs, retry counters, context and logs. Resume restores that state and finds the next node. Full in-memory model sessions cannot be serialized, so the first resumed hop degrades fidelity to a summary. This is an honest documented limitation: resume need not mean an identical model context. The spec does not prescribe a transactional outbox, idempotency key protocol or reconciliation of a tool whose external effect occurred before the checkpoint. It also does not establish an append-only event-derived state machine or authenticated journal. [Checkpoint and fidelity][A11].

The coding-loop spec is more concrete about local process lifetime: launch a new process group; on timeout send SIGTERM, wait two seconds, then SIGKILL; abort should close the model stream, kill running processes, run cleanup and close the session. Tool output limits/truncation and default/max command timeouts are specified. Those are excellent requirements to turn into tests, but the repository contains no executable result showing complete tree settlement or bounded memory before truncation. A direct filesystem environment is required; container, remote and WASM environments are extension points. [Stop conditions][A12], [execution environment][A13].

The environment policy excludes common API-key/secret/token/password/credential variable suffixes by default and preserves core paths/settings. It is customizable, including inherit-all. Thus the model is not a universal sandbox or secret non-disclosure guarantee. Arbitrary shell tools and hooks execute with the selected environment's authority. Prompt instructions and role specialization do not supply a repository-wide charter enforcement system by themselves. [Environment policy][A13].

The unified LLM document distinguishes retryable errors, Retry-After handling, total/per-step timeouts and partial streaming. Its completion checklist requires that a stream not be retried after partial data has been delivered. This is a particularly useful uncertainty boundary, but applies to stream delivery, not automatically to all tool side effects. The document's rationale also says unknown errors default to retryable; application effects need their own classification. [Client retry][A14].

## Lessons and discriminating experiments

For Brokkr #429, borrow the explicit route precedence and diagnostic table, then specify which warnings can affect authority. For #403/#433, turn group termination, stream abort and output limits into adversarial fixtures. For #434, keep model transport retries distinct from effect replay, and document which failures are deterministic, transient or uncertain.

A compact conformance experiment should use a graph with two equal-weight edges, an unvisited but graph-reachable review gate, a visited gate yielding PARTIAL_SUCCESS, and a human gate timing out. Record expected outcomes before testing any implementation. A second fixture should perform a fake external write and die before checkpoint, then ask the runtime to resume; check whether it repeats, refuses or reconciles. A third should cancel a shell process whose descendants retain output handles. These tests can compare Brokkr and Kilroy while treating this repository as the requirements source. They cannot establish that an unbuilt “Attractor” runtime is more or less reliable than either.

## Pinned evidence ledger

| ID | Source | Evidence role |
|---|---|---|
| A1 | [README.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/README.md#L1) | Repository scope and implementation instructions |
| A2 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L23) | Architecture, DSL and handlers |
| A3 | [coding-agent-loop-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/coding-agent-loop-spec.md#L23) | Programmable coding loop |
| A4 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L573) | Single-threaded outer traversal |
| A5 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L406) | Routing precedence |
| A6 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L461) | Visited goal-gate outcomes |
| A7 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L1392) | Validation severities |
| A8 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L480) | Retry and failure route rules |
| A9 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L720) | Human timeout and default behavior |
| A10 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L1651) | Pre/post tool hook failure |
| A11 | [attractor-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L1096) | Checkpoint and degraded resume context |
| A12 | [coding-agent-loop-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/coding-agent-loop-spec.md#L397) | Abort obligations |
| A13 | [coding-agent-loop-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/coding-agent-loop-spec.md#L770) | Process groups and environment filtering |
| A14 | [unified-llm-spec.md](https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/unified-llm-spec.md#L1300) | Error, retry and timeout specification |

[A1]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/README.md#L1
[A2]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L23
[A3]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/coding-agent-loop-spec.md#L23
[A4]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L573
[A5]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L406
[A6]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L461
[A7]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L1392
[A8]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L480
[A9]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L720
[A10]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L1651
[A11]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/attractor-spec.md#L1096
[A12]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/coding-agent-loop-spec.md#L397
[A13]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/coding-agent-loop-spec.md#L770
[A14]: https://github.com/strongdm/attractor/blob/fb57a55ed97372a27ac90102f436947e29f48426/unified-llm-spec.md#L1300
