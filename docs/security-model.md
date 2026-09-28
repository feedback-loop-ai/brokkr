# Security model

Brokkr runs agents that edit a repository, run its tools and commit. This
page says what stands between a seat and the host on `main` today, what
does not, how secrets move, and how the engine treats what a seat hands
back. Every statement here is read from the code. The box is built in
`crates/brokkr-protocol/src/hands.rs`, and each statement about it cites
that file. Where a gap has an owner, the
[known limitations](#known-limitations) link its issue. The
[status page](status.md) says what each harness can do.

The threat model for every gate is the operator's ruling of 2026-09-26: a
gate catches realistic **accidental** misuse and fails closed on what it
cannot read. A deliberate, exotic evasion of a gate is a low residual, not
something a gate claims to stop.

## Boundaries

A realm names the wall around its seats (decision
[0046](decisions/0046-the-boundary-is-named.md)). There are five words,
and a realm that names none gets `namespace`:

| Boundary | What stands around a seat today |
|---|---|
| `namespace` | Brokkr's box, built by bubblewrap 0.10 or newer. Linux only, WSL2 included. |
| `seatbelt` | **Refused.** macOS's `sandbox-exec` box is not built. |
| `container` | **Refused.** The container box is not built. |
| `harness` | Nothing of Brokkr's. The harness's own sandbox stands, as its adapter's fragment addresses it. |
| `open` | Nothing at all. A model gate whose agent declares hands is refused under `open`; a gate that declares a tool list or no tools runs unboxed, as it does under every boundary. |

A realm may declare `seatbelt` or `container` and compile. A run whose
seats declare hands under either is refused before any row is written,
whatever tools the host has:

> the `seatbelt` boundary is built by slice (ii) of decision 0046 ruling
> 6, not by this engine (sandbox-exec not on PATH); the seats […] declare
> hands and cannot run under it here — a realm may declare `harness`
> today (decision 0046 ruling 2)

The box is never simulated. A `namespace` realm on a host without
`bwrap` is refused, and on macOS that is every host: `brokkr init` writes
`harness` there, and verify and ship run their pinned scripts under no
box of Brokkr's.

## What the namespace box does

The box runs two things: each call a boxed model seat makes to its one
`workspace` tool, as `bash -lc <command>`, and a boxed `exec` seat's
whole command. The harness process itself runs outside the box, with its
credential and its connection to the provider. A claude or codex seat
reaches the box only through its `workspace` tool, and a dsh seat, whose
adapter cannot express boxed hands, never does.

- **Namespaces.** `box_argv` passes `--unshare-pid`, `--unshare-ipc`,
  `--unshare-uts` and `--unshare-cgroup-try`, and `--unshare-net` unless
  the seat's hands grant the network. The user and mount namespaces are
  bubblewrap's own. `--die-with-parent` ends the box with its caller,
  and `--new-session` detaches it from the caller's terminal.
- **Capabilities.** `--cap-drop ALL`. Nothing is added back.
- **Environment.** `--clearenv`, then a fixed set: a private `HOME`,
  `USER`, `TMPDIR` and `PATH`, `C.UTF-8`, `CI=true`,
  `commit.gpgsign=false` through `GIT_CONFIG_*`, the host's git author
  and committer identity, and `CARGO_HOME`, `RUSTUP_HOME` or
  `NPM_CONFIG_CACHE` only where that path is a declared bind.
- **Filesystem.** An empty root. The host toolchain directories under
  `/usr`, `/bin`, `/lib` and the TLS roots are read-only. `/etc/passwd`,
  `/etc/group`, `/etc/hosts` (loopback only) and `/etc/nsswitch.conf`
  are generated per call. `HOME` and `/tmp` are private per call and
  removed after it. The host home is not bound. The worktree is
  read-write at its own path. An exec seat's bundle is read-only at
  `/runtime/bundle`. The git directory's `hooks` sit behind an empty
  tmpfs and its `config` is read-only. A declared bind's mode is `ro`,
  `rw` or `overlay` (an upper layer that never reaches the host), and
  its `mask` names files under it that the box covers with `/dev/null`.
- **Bounds.** A call times out after 30 seconds by default and 600 at
  most, and each output stream is capped at 256 KiB.

## What the box does not do

- **No seccomp filter.** No system call is filtered.
- **The network is on or off.** `network: true` leaves the box in the
  host's network namespace. There is no allow-list of hosts.
- **No resource limits** beyond the timeout and the output cap: no
  memory, process-count or file-size limit.
- **Provider-side tools run outside it.** Codex's server-side
  `web_search` runs at the provider, so a boxed Codex seat whose hands
  set no network can still search the web. dsh 0.1.5 turns on
  `web_search` and `web_fetch` in every seat.
- **The git common directory is writable.** For a linked worktree the
  box binds the shared git directory read-write, so a boxed command can
  move a sibling worktree's branch, rewrite the object store or another
  worktree's `config.worktree`. `hands.rs` records this as known open
  (decision [0054](decisions/0054-the-dsh-harness-sandbox-reaches-a-linked-worktree-s-git-metadata.md)'s
  consequences). The dsh runner closes it for dsh seats by giving the
  seat a private common directory.
- **Tool-list offices are not boxed, and their tool list does not
  bound them.** `implementer` and `implementer-sdd` (granted `cargo`
  and `git`), `intake` (`git`) and `researcher` (`webfetch`,
  `websearch`, `git`, `ls`, `rg`) declare a tool list, not hands. They
  run on the host as the operator's user, in the engine's environment,
  under claude's `--permission-mode acceptEdits`, which pre-approves
  file edits. Brokkr passes the tool list as `--allowedTools`, Claude
  Code's list of tools it runs without asking, which removes no other
  tool. Only `--tools` restricts which tools a seat has, and the
  adapter passes it on the boxed hands path alone. What such a seat may
  run is decided by Claude Code's permission model and the operator's
  own Claude Code permission settings, and the operator's MCP servers
  reach it ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)).
  The `bundles/verify` review seat is the same: unboxed, `acceptEdits`,
  with `cargo`, `git`, `ls`, `rg`, `gh pr view` and `gh run view`
  pre-approved. Offices that declare neither, such as `triage` and the
  position seats, get no tool flag at all.
- **An unboxed claude seat with no tool list keeps Claude Code's
  defaults.** Brokkr passes no tool flag, no `--settings` and no
  `--setting-sources`, so the seat has Claude Code's default tools,
  `WebSearch` and `WebFetch` included, subject to the operator's own
  permission settings, and the operator's MCP servers. The shipped
  `triage` gate is such a seat.
- **Unboxed claude seats read the operator's MCP servers.** Only the
  boxed fragment passes `--strict-mcp-config`, so an unboxed claude seat
  starts every MCP server the operator's own Claude Code configuration
  names, with their credentials. A Codex seat, boxed or not, starts the
  MCP servers in `~/.codex/config.toml`.
- **The `harness` boundary is the harness's word.** Codex restricts by
  sandbox class (`read-only`, `workspace-write`), not by tool, and
  Brokkr has not measured what that sandbox enforces. An unboxed `exec`
  script starts from a rebuilt environment with a private `HOME` and
  `TMPDIR`, which confines nothing on disk: it may open any path the
  operator's user can. On Linux the engine attempts a network narrowing
  for it and does not report when the narrowing is unavailable.
- **Process settlement.** A timed-out attempt's detached descendants can
  outlive the kill, and a dead hands server's scratch tree stays under
  `/tmp`.

## How secrets flow

Bundles and journals carry secret **names** only (decision 0012).

- **The store** is `.forge/secrets.env` beside the workdir, or the file
  `--secrets-file` names. `brokkr secrets set` reads a value from stdin
  and writes the store `0600`, and a store whose permissions are broader
  is refused on read. Names match `[A-Z][A-Z0-9_]*`. `PATH`, `IFS`,
  `LD_PRELOAD`, `LD_LIBRARY_PATH` and every `BROKKR_` name are refused.
  So is a value shorter than four bytes, and one shorter than eight
  draws a warning.
- **A seat declares** the names it binds in its `secrets` list, and
  compilation refuses a `{{secret:NAME}}` reference the seat did not
  declare.
- **Every driver kind delivers** a declared binding through the child's
  environment, never its argv: exec, claude, LaneTally, codex and dsh
  alike. In an exec command `{{secret:NAME}}` becomes the text `$NAME`.
- **Where a secret cannot go.** Compilation refuses a binding on a seat
  with hands, because the box clears the environment. It also refuses a
  binding on a route whose egress class is below the bundle's
  `egress_minimum`, `contracted` by default. At that default the shipped
  claude and exec adapters may bind. The codex and LaneTally adapters
  may not, and neither may any dsh route but the `local` `spark` and
  `spark-glm`.
- **Masking.** Before anything reaches the journal, the driver masks
  each value the seat was bound to `[secret:NAME]` in stdout and stderr, each
  harness stream event, every checkpoint and the parsed result file. The
  value is matched raw and in base64, hex and percent encodings. `brokkr
  transcript`, the TUI and the browser mask a harness's session file as
  they read it, against every value in the store beside the journal.
- **What masking does not cover.** A value the seat was not bound to
  is not masked in the journal, even when the harness inherited it from
  the environment. The harness's own session file on
  disk holds what the model echoed, in plaintext. A digits-only value
  carried as a bare JSON number is not rewritten. An encoding of an
  encoding is not matched. A read surface masks only the current values
  of the store beside the journal, so a rotated value, or a store kept
  elsewhere, is shown as written.
- **The ambient environment.** A model seat inherits the engine's own
  environment, so whatever the launching shell exports reaches the
  harness and its model. `brokkr doctor` warns when an adapter's route
  credential comes from the process environment rather than the store.

## Seat output is untrusted input

A seat's output was written by a model that read the repository, its
prompt and, where it could, the web. Anything it read may have steered
it. The engine treats what a seat hands back as input to check, never as
an instruction:

- **The result is data.** The seat writes a typed result file. A file
  that does not parse or does not match its schema parks the run with
  the raw evidence, and nothing repairs it (decision
  [0001](decisions/0001-no-llm-repair-of-control-plane.md)). A seat
  never chooses the next phase: the pinned policy table rules on the
  result.
- **The wire fails closed.** The driver protocol refuses an unknown
  message type, and a checkpoint keeps only bounded turn, tool and usage
  fields, never prose or commands.
- **A gate is a trusted model or a pinned script.** Only a `trusted`
  adapter's declared judges may sit at a gate, and an untrusted harness
  such as dsh can never judge. An `exec` gate runs a pinned script.
- **The journal is hash-chained, not signed.** The chain is unkeyed
  SHA-256, so a verified chain detects an edit that did not recompute
  the hashes, not one that did: whoever can write the journal can
  rewrite it and recompute every link. Nor does it say who wrote it:
  operator events and anchors are unsigned, and decision 0008 defers the
  signing service. A green gate is not independent acceptance either.
- **What the engine does not check.** It does not read a diff for
  intent. A reviewer is a model and can be persuaded by what it reviews.
  Seat commits land unsigned in the worktree, and the operator reviews
  the branch, pushes and merges. That review is the last check.

## Known limitations

- **No seccomp filter in the box**
  ([#220](https://github.com/feedback-loop-ai/brokkr/issues/220)).
- **The box's network is on or off, with no allow-list**
  ([#216](https://github.com/feedback-loop-ai/brokkr/issues/216)).
- **Codex's server-side web search is on in every Codex seat.** Its off
  switch exists only on the unmerged decision 0065 slice
  ([#319](https://github.com/feedback-loop-ai/brokkr/pull/319)).
- **dsh turns on `web_search` and `web_fetch` in every seat**, with no
  realm grant ([#462](https://github.com/feedback-loop-ai/brokkr/issues/462)).
- **Brokkr bounds neither a tool-less claude seat's tools nor the MCP
  servers of an unboxed claude seat or any Codex seat**
  ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)), as
  [what the box does not do](#what-the-box-does-not-do) states. Decision
  0065 slice one ([#319](https://github.com/feedback-loop-ai/brokkr/pull/319))
  brings the capability grants meant to close it.
- **A timed-out attempt's detached descendants can outlive the kill**
  ([#403](https://github.com/feedback-loop-ai/brokkr/issues/403)).
- **Dead hands servers leak their scratch trees**
  ([#415](https://github.com/feedback-loop-ai/brokkr/issues/415)).
- **Runtime resources are unbounded** beyond timeouts and output caps
  ([#433](https://github.com/feedback-loop-ai/brokkr/issues/433)).
- **macOS has no box of Brokkr's**
  ([#253](https://github.com/feedback-loop-ai/brokkr/issues/253),
  [#269](https://github.com/feedback-loop-ai/brokkr/issues/269)).
