# unclebob/swarm-forge: implementation dossier

Research date: 2026-09-27. Default branch: `main`. Inspected revision: [`f4f5fbcae0de6f7dcc26e82400334227647cfdb2`](https://github.com/unclebob/swarm-forge/tree/f4f5fbcae0de6f7dcc26e82400334227647cfdb2), committed 2026-09-04T14:22:18Z. License: GitHub metadata reported no detected license; this is not a complete licensing review. GitHub metadata and raw sources were retrieved live; the revision matches the earlier survey. This is source inspection, not a benchmark, security certification, or report of tests executed. No project code was installed or run.

## Product identity and inspected branches

The `main` branch explicitly describes itself as landing page, installer, shared runtime and shared engineering law, not a runnable product. The five documented products must therefore be distinguished. In addition to the main revision above, this inspection pinned and read the following branches; dates are commit timestamps, not release dates. No license declaration was established by the cited product files, and GitHub repository metadata reports no detected license. Reuse permission must not be inferred from public source availability. [S1]

| Product | Inspected revision | Commit date UTC | Observed shape |
|---|---|---|---|
| two-pack | [`b75d16062b619e6eab06e03995e344676bc755cd`](https://github.com/unclebob/swarm-forge/tree/b75d16062b619e6eab06e03995e344676bc755cd) | 2026-09-04 14:22:19 | coder → cleaner ([configuration](https://github.com/unclebob/swarm-forge/blob/b75d16062b619e6eab06e03995e344676bc755cd/swarmforge/swarmforge.conf)) |
| four-pack | [`04662cf96ca83fb45783b4d73cd1daf0acefc0b8`](https://github.com/unclebob/swarm-forge/tree/04662cf96ca83fb45783b4d73cd1daf0acefc0b8) | 2026-09-04 14:22:19 | specifier → coder → refactorer → architect ([configuration](https://github.com/unclebob/swarm-forge/blob/04662cf96ca83fb45783b4d73cd1daf0acefc0b8/swarmforge/swarmforge.conf)) |
| six-pack | [`066a62ffa6cbc8c859262536dc579fecf8535ebb`](https://github.com/unclebob/swarm-forge/tree/066a62ffa6cbc8c859262536dc579fecf8535ebb) | 2026-09-04 14:22:19 | specifier → coder → cleaner → architect → hardender → QA |
| project-manager | [`2cc1795fbd1f5cedefd0c371aa1835464d1b4a15`](https://github.com/unclebob/swarm-forge/tree/2cc1795fbd1f5cedefd0c371aa1835464d1b4a15) | 2026-09-04 14:22:20 | host lieutenant with selectable pack templates ([product README](https://github.com/unclebob/swarm-forge/blob/2cc1795fbd1f5cedefd0c371aa1835464d1b4a15/README.md)) |
| lieutenant | [`23653942488281a3c6d9a60b7e9378be0a1ca1c2`](https://github.com/unclebob/swarm-forge/tree/23653942488281a3c6d9a60b7e9378be0a1ca1c2) | 2026-09-07 14:45:17 | host planner plus typed project-card routes |

The lieutenant branch's committed project template defines utility, component, QA and review routes. It is a meaningful product variation, not simply the main branch with a different prompt. Its default project configuration uses Codex for all six roles; the fixed six-pack combines Codex and Grok. Do not generalize one branch's current topology or backend assignments to the others. [Lieutenant template](https://github.com/unclebob/swarm-forge/blob/23653942488281a3c6d9a60b7e9378be0a1ca1c2/.swarmforge/project-pack/swarmforge/swarmforge.conf), [six-pack configuration](https://github.com/unclebob/swarm-forge/blob/066a62ffa6cbc8c859262536dc579fecf8535ebb/swarmforge/swarmforge.conf).

## Composition and execution

The installer composes shared scripts/articles with product-specific configuration, constitution entry points and role prompts. Shared engineering/workflow/handoff article names are reserved; pack-local files cannot replace them through the normal composer. A launcher validates roles, worktrees, supported backends and prompt existence, then creates worktrees, isolated tmux sessions, generated role maps, a daemon and a local dashboard. `master` is a sentinel for the current project checkout, not a required Git branch name. [S2], [S3]

```text
main shared runtime/articles + product configuration/roles
 → installer → worktrees + tmux agents + dashboard + handoff daemon
 → agent commits → validated handoff → audit challenge → repeat candidate
 → optional operator gate → recipient inbox → helper merge/process → next role
```

The launcher tells agents to read the constitution recursively and their role prompt. These instructions assign ownership and tests, while helper code enforces selected transport/workflow constraints. The six-pack QA prompt owns final verification and may also make narrow fixes; its independence is a role/context policy, not a demonstrated write-permission separation. The host lieutenant is explicitly outside the project engineering constitution. These details make SwarmForge close to Brokkr's reusable mandates while exposing a different authority model. [S3], [six-pack QA](https://github.com/unclebob/swarm-forge/blob/066a62ffa6cbc8c859262536dc579fecf8535ebb/swarmforge/roles/QA.prompt), [lieutenant product](https://github.com/unclebob/swarm-forge/blob/23653942488281a3c6d9a60b7e9378be0a1ca1c2/README.md).

## Concrete enforcement and its limits

Configuration refuses duplicate roles/worktrees, unsupported backends, missing prompts, invalid worktree names and anything other than exactly one main checkout. Handoff validation resolves configured recipients, binds commits to the sender's work, rejects invalid/ambiguous current-work shapes and derives artifacts from Git. Receive helpers support task and compatible batch modes, and reverse copies can be merge-only. These are executable mechanisms, not merely prose. [S3], [S4], [S5]

The audit gate stores a candidate containing sender, task identity, recipients, commit, artifacts and a SHA-256 draft fingerprint. First submission prints `AUDIT_REQUIRED` without queueing; an unchanged repeated candidate is accepted. Changed commits/drafts invalidate the pending audit. Tests verify those paths and that the original current task remains in process until submission. The gate proves a repeat submission of the same candidate. It does not prove that the agent actually performed the requested audit, ran tests, or supplied independently verified evidence. This distinction matters when comparing it with Brokkr's typed evidence and protected review. [S4], [S5]

Some documentation is less authoritative than current helpers. `handoff-protocol.md` retains “proposal” wording and describes a terminal broadcast to every other role. Current helper tests cover last-role automatic non-forwarding tagging and propagation modes, while the main README instructs the last role to queue its handoff without manually naming everyone. Treat protocol prose as supporting context and use the selected branch implementation/tests to establish behavior. [S1], [S5], [S6]

## Permissions, effects and lifecycle

The main launcher automatically adds `--yolo` for Codex/Copilot and `--permission-mode bypassPermissions` for Claude/Grok unless matching flags are already present. Worktree isolation therefore separates working copies but is not an operating-system sandbox or credential boundary. Agents retain ordinary host access; prose prohibiting direct transport manipulation cannot protect files from a process with that access. The local approval queue covers selected handoff transitions, such as initial master/specifier-to-worker delivery; it is not a universal approval gate around every agent shell/tool action. [S3], [S7]

The dashboard listens on `127.0.0.1`; inspection did not establish an authenticated remote-service boundary. Its controls are suitable to a local operator model. The handoff daemon has stop-file and PID tracking, responsive polling, and a stop helper that sends TERM, polls up to five seconds, then sends KILL if needed. Cleanup archives panes, stops the daemon and kills named tmux sessions; a terminal watchdog can trigger coordinated cleanup. This is concrete lifecycle infrastructure, but it is not evidence that every detached child process or inherited output pipe is settled before a run is declared complete. There is no universal per-agent budget/deadline in the launcher path reviewed. [S8], [S9], [S10]

## Durability, retries and provenance

Queue state is represented by file location: outbox, sent/failed, and new/in-process/completed inboxes. The daemon copies recipient files, sends wake notifications and moves the original to sent. Existing target files are skipped, and merge helpers check ancestry before merging a commit already present. This supports restart inspection and some duplicate avoidance. It does not establish exactly-once multi-recipient delivery: recipient writes, notifications, sender archival and board changes are separate operations. The daemon's recipient write is direct, while outbound/audit creation uses temporary-file moves. Hard-kill windows and incomplete files therefore need controlled fault tests rather than a blanket “durable queue” assurance. [S4], [S7], [S11]

Sequence allocation uses a directory lock and retries while it exists, then removes it in `finally`. A hard kill while holding it can leave the directory behind; the inspected allocator has no stale-lock deadline or ownership recovery. This is a source-derived crash-recovery concern, not a reproduced failure. Git commit ancestry gives durable code identity, but handoff timestamps and audit fingerprints do not form a single hash-chained event journal or a pure fold of system state. Runtime files remain mutable. [S4], [S12]

The installer fetches named branch tarballs, with a local Git fallback; it does not lock every composed component to a SHA or write an immutable manifest comparable to Brokkr's. A later install can combine newer shared runtime with a product branch. Research pins above make this review reproducible but are not a runtime guarantee of the installer. [S2]

## Maintenance, ergonomics and Brokkr lessons

Dependencies include zsh, Git, tmux, Babashka and at least one agent CLI, with terminal adapters and Node/Playwright required for dashboard testing. `bb.edn` runs Clojure/Babashka helper tests and browser tests, with pinned Git dependencies for quality tools. The reviewed main tree contains tests but no GitHub Actions workflow; this review did not run them or infer hosted CI success. The branch model encourages shared-runtime changes on main followed by refreshes into forge branches, creating an explicit synchronization obligation. [S1], [S13]

SwarmForge is a close practical precedent for constitutions, role packs, lieutenant coordination, worktree ownership and durable committed handoffs. Brokkr already credits the lieutenant concept. Brokkr's intended distinction is mandatory machine-evaluated phase/review policy, pinned run composition and event-derived evidence; its open enforcement/lifecycle/retry issues still require qualification.

Prioritize fake-backend experiments: skip the requested self-audit and repeat the exact handoff to expose what the gate actually establishes; kill during sequence allocation or recipient delivery; notify successfully and lose acknowledgement; detach a child before stopping tmux; and reinstall after changing one product/shared branch. Useful ideas to adopt are repair-oriented refusal messages, explicit candidate identity, committed-work transport and operator inspection. Avoid inheriting always-approve defaults or treating prompts and Git worktrees as a security boundary.

## Source evidence ledger

The following are immutable source links. Tests establish the cases maintainers encode; their presence does not establish a passing run in this investigation.

| ID | Source and inspection purpose |
|---|---|
| S1 | [README.md](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/README.md) — main/product distinction, composition ownership and setup |
| S2 | [get-swarm-forge](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/get-swarm-forge) — branch-based installer, reserved articles and local fallback |
| S3 | [swarmforge/scripts/swarmforge.bb](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/swarmforge.bb) — configuration validation, startup and broad backend permission flags |
| S4 | [swarmforge/scripts/swarm_handoff.bb](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/swarm_handoff.bb) — candidate validation, fingerprint audit gate, sequence lock and queue write |
| S5 | [test/swarmforge/handoff_test.clj](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/test/swarmforge/handoff_test.clj) — handoff refusal, audit invalidation, approval and terminal-role regressions |
| S6 | [swarmforge/handoff-protocol.md](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/handoff-protocol.md) — transport documentation and proposal/current-code caveat |
| S7 | [swarmforge/scripts/handoffd.bb](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/handoffd.bb) — approval selection, per-recipient writes and delivery/notification ordering |
| S8 | [swarmforge/scripts/pack_web.bb](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/pack_web.bb) — local dashboard and approval controls |
| S9 | [swarmforge/scripts/stop_handoff_daemon.bb](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/stop_handoff_daemon.bb) — TERM/poll/KILL daemon lifecycle |
| S10 | [swarmforge/scripts/swarm-cleanup.sh](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/swarm-cleanup.sh) — tmux/daemon/terminal cleanup scope |
| S11 | [swarmforge/scripts/merge_and_process.bb](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/merge_and_process.bb) — ancestry-based duplicate merge avoidance |
| S12 | [swarmforge/scripts/handoff_lib.bb](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/handoff_lib.bb) — mutable file state and sequence allocation utility |
| S13 | [bb.edn](https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/bb.edn) — local test tasks, browser prerequisites and pinned quality-tool dependencies |

[S1]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/README.md

[S2]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/get-swarm-forge

[S3]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/swarmforge.bb

[S4]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/swarm_handoff.bb

[S5]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/test/swarmforge/handoff_test.clj

[S6]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/handoff-protocol.md

[S7]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/handoffd.bb

[S8]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/pack_web.bb

[S9]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/stop_handoff_daemon.bb

[S10]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/swarm-cleanup.sh

[S11]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/merge_and_process.bb

[S12]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/swarmforge/scripts/handoff_lib.bb

[S13]: https://github.com/unclebob/swarm-forge/blob/f4f5fbcae0de6f7dcc26e82400334227647cfdb2/bb.edn
