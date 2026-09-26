# danshapiro/kilroy: implementation dossier

Research date: 2026-09-27 (Europe/Sofia). Live metadata retrieved 2026-09-26T22:00:46.409395+00:00. Default branch: `main`. Revision: [`b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f`](https://github.com/danshapiro/kilroy/tree/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f); commit date: 2026-04-27T17:57:05Z. Repository license: [MIT](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/LICENSE), recorded without a legal interpretation.

Method: read-only inspection of a complete archive fetched by this exact SHA from GitHub, plus live repository metadata. No downloaded code, tests, installation commands, or provider workloads were executed. Tests below are inspected test assertions, not independently observed passes. This is an architectural comparison, not a numerical quality or security rating.

## What the project establishes

Kilroy is a substantial Go implementation of Attractor plus a project-specific metaspec. It combines DOT workflow authoring, deterministic route selection, agent/tool execution, Git worktrees and node commits, typed CXDB events/artifacts, local run metadata and recovery. It is a direct architecture peer for Brokkr's controlled agent effects and inspectable execution, with a larger branching/re-entry vocabulary. It should not be conflated with upstream Attractor's specifications or treated as a uniform sandbox simply because some backend examples use one. [Overview][K1], [metaspec][K2].

The execution flow is graph/config parse and validation → provider/model/CXDB preflight → acquire run ownership and create worktree/run artifacts → execute one node → classify outcome and select route → checkpoint Git/code plus execution state → continue or finalize. Parallel handlers execute branch subgraphs with separate workspaces/context and join their results. Like the Attractor specification, the main graph traversal advances one node at a time; arbitrary graph routing is the major difference from Brokkr's constitutional linear outer phase machine. [Engine][K3], [parallel implementation][K4].

## Inputs, graph authority and completion

Run configuration supports provider transports, model selection/catalog snapshots, resource/stall policy, input materialization, artifact policy and preflight probes. JSON parsing disallows unknown fields and YAML uses `KnownFields(true)`. Graph validation covers topology, conditions/stylesheets, required provider/tool details and failure-loop rules. Some loop-restart hazards are warnings rather than unconditional refusal. A maintainer must distinguish validator diagnostics, runtime policy and provider settings; a single green graph-validation result does not validate every external effect. [Config parser][K5], [validation tests][K6].

Routing records condition evaluations and the source/reason of next-hop selection. A fan-in deterministic failure is prevented from blindly following its retry target, whereas transient failures can recover through it. Additional failure-cycle detection uses signatures to stop endless implement/verify cycles; tests include implementation succeeding while verification repeatedly fails. This goes beyond simplistic fixed retry counters and offers a concrete pattern for Brokkr #434. [Next-hop policy][K7], [cycle tests][K8].

Goal gates do not establish an unavoidable review phase. `checkGoalGates` iterates recorded node outcomes rather than all declared gated nodes, accepting success, degraded success and partial success. Therefore it checks visited gates; a route that never visits a declared review gate requires separate graph validation if review is mandatory. The first failing gate is selected from a Go map, so ordering among multiple simultaneously unsatisfied gates is another narrow candidate for a determinism test. This does not contradict deterministic edge sorting; it identifies a different selection surface. [Gate implementation][K9].

## Recovery and evidence integrity

A checkpoint records current/completed nodes, retries, context, Git commit SHA and extension state. The write helper creates a same-directory temporary file, writes, fsyncs, closes and renames it. The inspected helper does not fsync the parent directory after rename. Resume acquires run ownership, requires a Git SHA, reads the saved graph and configuration, reconstructs the engine and insists on the per-run model catalog when configuration is present. It also restores artifact/input policies and recorded loop-failure signatures. This is meaningful source-level recovery support; it is not replay of a single canonical journal. [Checkpoint schema][K10], [atomic write][K11], [resume][K12].

Input snapshot lineage hashes normalized parent identities and file digests, forks branch revisions and rejects conflicting promoted paths. Tests restore materialized inputs after the original workspace is unavailable and verify stable conflict payload ordering. Artifact-policy resume rejects garbage snapshots while supporting documented legacy formats. Those mechanisms make branch provenance and recovery more concrete than merely retaining a transcript. They do not authenticate the author of the snapshot or verify the truth of agent claims. [Input lineage][K13], [input resume tests][K14], [parallel guardrails][K15], [artifact resume tests][K16].

CXDB's sink serializes appends under a mutex, uses parent-turn identity and content hashes, stores artifact bytes in a content-addressed store, and supports binary-to-HTTP fallback. However, multiple event/attachment call sites deliberately discard returned errors, including checkpoint events and artifacts. Thus CXDB availability at startup does not establish a complete acknowledged event record for every runtime effect. The fallback also deserves an uncertain-acknowledgement test: if a binary append succeeds but its acknowledgement is lost, a retry through HTTP needs explicit deduplication/reconciliation semantics. No end-to-end exactly-once guarantee was established in the inspected client path. [Sink][K17], [event call sites][K18].

Run ownership uses exclusive lock-file creation with PID/start identity and stale-owner handling. This differs from Foundation's OS-held lock and Brokkr's journal transaction. The relevant lesson is to test process death, PID reuse, corrupt lock files and simultaneous resume against the exact mechanism, rather than assuming all “run locks” have equivalent behavior. [Run ownership][K19].

## Lifecycle and permission boundaries

Kilroy has real process-group termination and targeted tests. The Unix helper uses group SIGTERM/SIGKILL; the CLI watchdog watches output file growth, handles cancellation, applies a grace period, and bounds waiting after kill. But the strongest existing test exposes a qualification gap: its comment intentionally uses `exec.Command`, not `exec.CommandContext`, because direct parent termination can win the wait race and skip group cleanup. The production CLI launch still uses `exec.CommandContext`. When idle timeout is disabled, the watchdog immediately waits on the process and does not use its group-cleanup select loop. Only Codex-semantic CLI invocations receive `setProcessGroupAttr` in the inspected branch. These are specific source-derived risks, not reproduced failures or a finding that every Kilroy subprocess leaks. [Production launch][K20], [watchdog][K21], [test caveat][K22], [platform helpers][K23].

The process tests skip a cancellation scenario on macOS because signaling is described as unreliable there. Windows has a different taskkill helper and no POSIX process-group setup. Cross-platform lifecycle assurance must therefore be narrower than the existence of a generic helper suggests. A regression fixture should exercise the exact production launcher, not only a hand-constructed process with more favorable cancellation behavior. [Process tests][K22], [Windows helper][K24].

Permission posture varies materially by backend. Built-in Anthropic CLI arguments include `--dangerously-skip-permissions`; Gemini uses `--yolo`. The codex-app-server provider options set approval policy `never` and danger-full-access. The CLI launch auto-answers confirmation for non-stdin prompt modes; a special fan-in path can remove Codex sandbox flags. These choices support unattended operation, but they do not enforce Brokkr-like effect authorization boundaries. The README's workspace-write example should not be used as a universal claim about current provider profiles. [Built-in profiles][K25], [provider options][K26], [launch][K20].

Secrets and logs inherit those backend choices. Some environment isolation and conflicting-provider-key scrubbing exist, but the reviewed CLI path starts from a base environment and records provider/native traces. The inspection did not establish universal redaction or a credential-free execution environment. Treat run artifacts as potentially sensitive and evaluate the actual adapter in any controlled experiment; this is a scope conclusion, not a claim of an observed leak.

## Maintenance and actionable comparisons

The Go 1.25 module includes explicit dependency versions and many fake-CLI, recovery, graph-parser, condition and lineage tests. CI uses pinned checkout/setup actions, gofmt, vet, build, all-package tests and demo-graph validation on Ubuntu. Some tests are integration/provider gated, and no test suite was run during this review. Extensive source tests establish intended cases, not production reliability. [Dependencies][K27], [CI][K28].

For Brokkr #403/#435, reproduce cancellation with the production CommandContext path, an exited leader, and surviving descendants. For #429, test an unvisited goal gate, partial-success review and two unsatisfied gates. For #431/#434, kill after a remote fake effect but before checkpoint, inject directory-sync failure, and lose the binary CXDB acknowledgement before HTTP fallback. For #432, compare recovery when the model catalog or saved policy is corrupt. Kilroy's route explanations, failure signatures, input snapshot conflicts and artifact inspection are strong design lessons; none requires Brokkr to adopt arbitrary outer graphs or to claim its own unresolved guarantees already hold.

## Pinned evidence ledger

| ID | Source | Evidence role |
|---|---|---|
| K1 | [README.md](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/README.md#L1) | Scope, setup and operations |
| K2 | [docs/strongdm/attractor/kilroy-metaspec.md](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/docs/strongdm/attractor/kilroy-metaspec.md#L1) | Kilroy implementation choices |
| K3 | [internal/attractor/engine/engine.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/engine.go#L626) | Outer traversal and checkpoint flow |
| K4 | [internal/attractor/engine/parallel_handlers.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/parallel_handlers.go#L1) | Concurrent branch execution |
| K5 | [internal/attractor/engine/config.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/config.go#L174) | Strict JSON/YAML config |
| K6 | [internal/attractor/validate/validate_test.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/validate/validate_test.go#L197) | Topology, failure loop and gate tests |
| K7 | [internal/attractor/engine/next_hop.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/next_hop.go#L29) | Route reasons and deterministic failure policy |
| K8 | [internal/attractor/engine/deterministic_failure_cycle_test.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/deterministic_failure_cycle_test.go#L141) | Bounded repeated deterministic failure |
| K9 | [internal/attractor/engine/engine.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/engine.go#L2339) | Visited gate check and accepted statuses |
| K10 | [internal/attractor/runtime/checkpoint.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/runtime/checkpoint.go#L10) | Snapshot schema |
| K11 | [internal/attractor/runtime/atomic_write.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/runtime/atomic_write.go#L10) | File fsync and rename |
| K12 | [internal/attractor/engine/resume.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/resume.go#L49) | Resume source validation and ownership |
| K13 | [internal/attractor/engine/input_snapshot_lineage.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/input_snapshot_lineage.go#L152) | Digest lineage and promotion conflicts |
| K14 | [internal/attractor/engine/input_materialization_resume_test.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/input_materialization_resume_test.go#L10) | Restore without original inputs |
| K15 | [internal/attractor/engine/parallel_guardrails_test.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/parallel_guardrails_test.go#L42) | Cancellation and conflict refusal |
| K16 | [internal/attractor/engine/artifact_policy_resume_test.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/artifact_policy_resume_test.go#L61) | Corrupt snapshot refusal |
| K17 | [internal/attractor/engine/cxdb_sink.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/cxdb_sink.go#L49) | Append serialization and transport fallback |
| K18 | [internal/attractor/engine/cxdb_events.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/cxdb_events.go#L143) | Best-effort event/artifact writes |
| K19 | [internal/attractor/engine/run_ownership_lock.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/run_ownership_lock.go#L60) | Exclusive file ownership protocol |
| K20 | [internal/attractor/engine/agent_router.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/agent_router.go#L1258) | Production CommandContext launch |
| K21 | [internal/attractor/engine/agent_router.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/agent_router.go#L1986) | Watchdog cancellation and idle policy |
| K22 | [internal/attractor/engine/agent_process_test.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/agent_process_test.go#L111) | Test intentionally avoids production cancellation race |
| K23 | [internal/attractor/engine/process_group_unix.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/process_group_unix.go#L1) | POSIX group signals |
| K24 | [internal/attractor/engine/process_group_windows.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/process_group_windows.go#L1) | Windows-specific cleanup |
| K25 | [internal/providerspec/builtin.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/providerspec/builtin.go#L1) | Provider permission defaults |
| K26 | [internal/attractor/engine/agent_router.go](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/agent_router.go#L527) | App-server full-access options |
| K27 | [go.mod](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/go.mod#L1) | Build/runtime dependencies |
| K28 | [.github/workflows/ci.yml](https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/.github/workflows/ci.yml#L1) | Maintainer checks |

[K1]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/README.md#L1
[K2]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/docs/strongdm/attractor/kilroy-metaspec.md#L1
[K3]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/engine.go#L626
[K4]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/parallel_handlers.go#L1
[K5]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/config.go#L174
[K6]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/validate/validate_test.go#L197
[K7]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/next_hop.go#L29
[K8]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/deterministic_failure_cycle_test.go#L141
[K9]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/engine.go#L2339
[K10]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/runtime/checkpoint.go#L10
[K11]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/runtime/atomic_write.go#L10
[K12]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/resume.go#L49
[K13]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/input_snapshot_lineage.go#L152
[K14]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/input_materialization_resume_test.go#L10
[K15]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/parallel_guardrails_test.go#L42
[K16]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/artifact_policy_resume_test.go#L61
[K17]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/cxdb_sink.go#L49
[K18]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/cxdb_events.go#L143
[K19]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/run_ownership_lock.go#L60
[K20]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/agent_router.go#L1258
[K21]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/agent_router.go#L1986
[K22]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/agent_process_test.go#L111
[K23]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/process_group_unix.go#L1
[K24]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/process_group_windows.go#L1
[K25]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/providerspec/builtin.go#L1
[K26]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/internal/attractor/engine/agent_router.go#L527
[K27]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/go.mod#L1
[K28]: https://github.com/danshapiro/kilroy/blob/b55fb0f2b3d5bfc603726d8ce5c5b89de81bfa2f/.github/workflows/ci.yml#L1
