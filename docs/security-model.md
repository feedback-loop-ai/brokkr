# Security model

Brokkr runs agents that edit a repository, run its tools and commit. This
page says what stands between a seat and the host on `main` today, what
does not, how secrets move, and how the engine treats what a seat hands
back. Every statement here is read from the code, and each one cites the
file that does it. The box is built in
`crates/brokkr-protocol/src/hands.rs`. A seat's typed tools are read in
`crates/brokkr-runtime/src/agents/load.rs` and lowered in
`crates/brokkr-runtime/src/agents.rs` and
`crates/brokkr-runtime/src/bundle.rs`. What a seat holds is ruled in
`crates/brokkr-runtime/src/capabilities.rs`, and the launch is checked in
`crates/brokkr-protocol/src/native_controls.rs`. Where a gap has an
owner, the [known limitations](#known-limitations) link its issue. The
[status page](status.md) says what each harness can do.

The threat model for every gate is the operator's ruling of 2026-09-26: a
gate catches realistic **accidental** misuse and fails closed on what it
cannot read. A deliberate, exotic evasion of a gate is a low residual, not
something a gate claims to stop.

## Boundaries

A realm names the wall around its seats (decision
[0046](decisions/0046-the-boundary-is-named.md)). There are five words,
and a realm that names none gets `namespace`:

| Boundary | What stands around a seat that declares hands today |
|---|---|
| `namespace` | Brokkr's box, built by bubblewrap 0.10 or newer. Linux only, WSL2 included. |
| `seatbelt` | **Refused.** macOS's `sandbox-exec` box is not built. |
| `container` | **Refused.** The container box is not built. |
| `harness` | Nothing of Brokkr's. A model seat's harness sandbox stands only where its adapter declares a fragment for the seat's class, as that fragment addresses it (the status page's *Own sandbox for* column). An `exec` seat's script runs unboxed in a rebuilt environment. |
| `open` | Nothing at all. A model gate whose agent declares hands is refused under `open`. |

The wall stands only around a seat that declares hands. A seat without
hands is launched with no box under every boundary, so a seat that
declares a tool list or no tools runs unboxed on the host (see
[what the box does not do](#what-the-box-does-not-do)). Under `harness`
no harness sandbox is added for such a seat either, with one exception
that holds under every boundary, `namespace` included: an inline seat
whose command dispatches the codex driver and that declares a
`tools.sandbox` class gets that class through the codex adapter's
`hands.harness` fragment for its seat class, while it stays unboxed
(`lower_inline_sandbox`, `bundle.rs`; see
[typed tools](#typed-tools-per-harness)). What decision
0065 composes into its command, the [native powers](#capabilities-are-off-until-the-realm-grants-them)
it does not hold switched off, stands under every boundary.

A realm may declare `seatbelt` or `container` and compile. A run whose
seats declare hands under either is refused before any row is written,
whatever tools the host has:

> the `seatbelt` boundary is built by slice (ii) of decision 0046 ruling
> 6, not by this engine (sandbox-exec not on PATH); the seats […] declare
> hands and cannot run under it here — a realm may declare `harness`
> today (decision 0046 ruling 2)

The box is never simulated. A run whose seats declare hands in a
`namespace` realm on a host without `bwrap` is refused, and on macOS
that is every host: `brokkr init` writes `harness` there, and verify and
ship run their pinned scripts under no box of Brokkr's. A bundle whose
seats declare no hands asks nothing of the host and runs, unboxed.

## What the namespace box does

The box runs two things: each call a boxed model seat makes to its one
`workspace` tool, as `bash -lc <command>`, and a boxed `exec` seat's
whole command. The harness process itself runs outside the box, with its
credential and its connection to the provider. A boxed claude seat that
holds no native power has `workspace` as its only tool; a native power
the realm grants it adds that power's tool, `WebSearch` or `WebFetch`,
to its `--tools` and `--allowedTools` lists (`final_tools`,
`native_controls.rs`). Either way the Claude Code process still loads
the operator's own configuration on the host (see below). A boxed codex seat
also keeps Codex's native shell, read-only and outside the box (see
below). A dsh seat, whose adapter cannot express boxed hands, never
reaches the box.

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
  `/runtime/bundle`. The common git directory's `hooks` sit behind an
  empty tmpfs and its `config` is read-only, but the rest of that
  directory is read-write, so neither cover stops a boxed command from
  planting a hook (see [what the box does not
  do](#what-the-box-does-not-do)). A declared bind's mode is `ro`,
  `rw` or `overlay` (an upper layer that never reaches the host), and
  its `mask` names files under it that the box covers with `/dev/null`.
- **Bounds.** A call times out after 30 seconds by default and 600 at
  most, and each output stream is capped at 256 KiB.

## Typed tools, per harness

A seat declares its tools as data, never as flags (decision 0065, slice
one). `parse_tools` in `agents/load.rs` reads three keys and refuses any
other: `tools.allow`, an ordered list of names from the adapter's map;
`tools.sandbox`, one of `read-only`, `workspace-write` and
`danger-full-access`; and `tools.mcp`, which must be empty, because an
agent no longer names an MCP server. There is no `tools.deny` in this
build. A site may subtract from its office's list and narrow its class,
and widening either is refused (`LocalTools::narrow`, `agents.rs`).

A recipe writes no capability-bearing option. A tool list of any
polarity, a permission mode, a sandbox class, an MCP or settings load, a
Codex `-c` assignment into a capability table, `--add-dir` and a session
selector written into a driver command are refused at compile, whatever
their value (`authored_refusal`, `native_controls.rs`, called from
`Authority::resolve` in `capabilities.rs`). Only the engine writes a
seat's tools, from the adapter's data.

What each harness makes of them:

| Harness | `tools.allow` | `tools.sandbox` | What actually removes or confines |
|---|---|---|---|
| `claude` | `--allowedTools`, from the adapter's `tool_permissions` map (`lower_allow`, `agents.rs`). This is pre-approval: it removes no tool. A name that maps to `WebSearch` or `WebFetch` is refused (`native_alias`). On a seat without hands, where the list is lowered directly, an explicitly empty list is refused too; beside hands the list is dormant and passes no flag (decision 0043 ruling 2), and only a name mapped onto a native tool is refused (`agents.rs`). | Refused: a class is admitted only where an engine fragment already expresses it, and only Codex's do (`admit_local_sandbox`, `bundle.rs`). | On the boxed path, the hands fragment's `--tools ""` removes Claude Code's own tools and the engine fills that include list with the tools of the native powers the seat holds and nothing else, so a seat with no grant keeps only `workspace` (`final_tools`, `native_controls.rs`); `--strict-mcp-config` shuts out the operator's MCP servers (`adapters/claude.json`). On every seat, `--disallowedTools` names each native power the seat does not hold. |
| `codex` | Refused on a seat without hands: Codex maps no tool name, so the list cannot be expressed (`lower_allow`). Beside hands the list is dormant and passes no flag (decision 0043 ruling 2), and no name of it can map onto a native tool. | Admitted only where an engine fragment expresses exactly that class. A seat with hands: `read-only` when boxed and for a gate under `harness`, `workspace-write` for a work seat under `harness` (`admitted_sandbox`, `bundle.rs`); a class on an agent-resolved seat without hands is refused. An inline seat without hands whose command dispatches codex, under every boundary: `read-only` for a gate and `workspace-write` for a work seat (`lower_inline_sandbox`, `bundle.rs`). Every other shape, `danger-full-access` among them, is refused, never clamped. | `--sandbox read-only` on the boxed path, and the `hands.harness` fragments' `--sandbox` classes under `harness` and on an inline seat's typed class, the gate's with `--output-last-message` into the engine-owned result path (`adapters/codex.json`). On every seat, `-c web_search="disabled"` unless the seat holds `web-search`. |
| `lanetally` | Refused at compile while its native inventory is unmeasured (operator ruling R5 of 2026-09-29; `Authority::resolve`, `capabilities.rs`). | Refused, as for claude. | Nothing of Brokkr's. LaneTally takes no boxed hands and no native control. |
| `dsh` | Refused: dsh maps no tool name. | Refused. | dsh's own sandbox, which the driver's runner refines (`dsh_sandbox.rs`). No typed tool reaches it. |
| `exec` | Not applicable. | Not applicable. | The box under `namespace`. |

So two things still only pre-approve. Claude's `--allowedTools`, and the
claude adapter's own `--permission-mode acceptEdits` (its `driver`),
make what they name run without asking and take nothing away. What an
unboxed claude seat may run beyond them is Claude Code's permission model
and the operator's own settings.

## Capabilities are off until the realm grants them

A seat holds a capability, such as `web-search` or `web-fetch`, only
when three things agree: its office asks for it by name under
`capabilities`, the seat does not subtract it, and the realm grants it
to that office in `realms.json`'s `forge.realms/v6` `capabilities` map,
through a tool dialect (decision 0065 rulings 1, 3 and 5;
`Authority::resolve`, `capabilities.rs`). A recipe or an agent only
asks. A `requires` the realm does not grant refuses the compile, and a
`wants` is dropped, with a notice in the run manifest and in the seat's
prompt. This repository's own `realms.json` grants nothing.

A harness's own powers are declared in its adapter's
`native_capabilities` (decision 0065 ruling 4; `NativeInventory`,
`capabilities.rs`), and the [status page](status.md#native-powers)
renders them. For every seat, independently of what it asks:

- a known power the seat holds is switched on, and one it does not hold
  is switched off by the control its adapter declares, boxed or not;
- a known power whose OFF is `unsupported` or `unmeasured` refuses the
  seat, in a realm that has not granted it;
- the engine knows Codex carries `web-search` and Claude Code carries
  `web-search` and `web-fetch` (`known_powers`, `native_controls.rs`), so
  an adapter that omits, empties or cannot load that declaration refuses
  the seat rather than launching it with the power on (decision 0066
  ruling 1);
- an `unmeasured` inventory, which dsh, LaneTally and `exec` declare,
  switches nothing off and claims no denial. The prompt tells the seat
  so.

A grant's restriction keys, such as a host allow-list, reach no harness
yet: no adapter declares a restriction transport, so a grant with a
nonempty restriction refuses a `requires` and drops a `wants` with the
power off. Only Codex's OFF was measured live, on codex-cli 0.154.0's
cold `codex exec`; Claude's is composition (the
[status page's notes](status.md#notes-on-native-powers)). `brokkr doctor`
says per realm what is granted and which native powers are switched off.

## The final launch check

The engine seals each launch: its plan, a record of every argument by
origin (the recipe's, the adapter's template, the engine's local
permissions, its hands and its native controls), and the typed inputs it
was composed from. Before the harness is spawned, the driver checks the
final command (`served`, `adapters.rs`; `check_final`,
`native_controls.rs`):

- the record must reassemble exactly the arguments the driver was
  handed;
- the command is parsed back under the harness's grammar, and the
  capability state it expresses must carry every denial the plan
  records, and nothing beside;
- the engine recomposes the command from the sealed inputs alone, and the
  final command must equal it token for token.

A mismatch refuses the launch. So does a plan without its sealed record,
or a record without its plan. A driver run by hand, with no plan, is
served as composed, and this check gives it no guarantee. A harness with
no modelled grammar, `exec` or a custom driver, has no final command the
check can read.

## What the box does not do

- **No seccomp filter.** No system call is filtered.
- **The network is on or off.** `network: true` leaves the box in the
  host's network namespace. There is no allow-list of hosts.
- **No resource limits** beyond the timeout and the output cap: no
  memory, process-count or file-size limit.
- **A boxed Codex seat can still read the host.** The codex hands
  fragment sets Codex's native shell `--sandbox read-only` and adds the
  `workspace` tool beside it, and Codex has no switch that removes the
  native shell. So the model keeps a read-only view of the whole host
  outside the box, credential files and the host home included, and only
  its writes go through the box (decision
  [0043](decisions/0043-the-hands-are-one-tool.md)'s consequences). "The
  host home is not bound" holds for the box's own calls and for exec,
  not for what a Codex seat reads. No issue owns this yet.
- **A boxed claude seat still loads the operator's Claude Code
  configuration.** The Claude Code process runs outside the box, and
  the claude hands fragment passes no `--settings`, no
  `--setting-sources` and no configuration directory, so it reads the
  operator's user-scope settings, `CLAUDE.md` and auto-memory from the
  host home. That text reaches the model's context, and the hooks those
  settings declare run on the host, outside the box. `--tools` and
  `--strict-mcp-config` still leave the model only `workspace` and the
  tool of each native power the realm grants the seat.
  No issue owns this yet.
- **Provider-side tools run outside it.** Codex's server-side
  `web_search` runs at the provider, so a box with no network neither
  sees nor stops it. What stops it is the OFF composed into the command
  when the seat does not hold `web-search`. dsh 0.1.5 turns on
  `web_search` and `web_fetch` in every seat, and nothing of Brokkr's
  switches them off.
- **The git common directory is writable.** The box binds the
  repository's common git directory read-write, with only its `hooks`
  and `config` covered; a plain checkout's lies inside the read-write
  worktree. So a boxed command can move a sibling worktree's branch,
  rewrite or corrupt the object store, and write any worktree's
  `config.worktree`, and in a repository with `extensions.worktreeConfig`
  a `core.hooksPath` written there plants a hook the host's next `git`
  runs. `hands.rs` records this as known open
  (decision [0054](decisions/0054-the-dsh-harness-sandbox-reaches-a-linked-worktree-s-git-metadata.md)'s
  consequences). The dsh runner closes it for dsh seats by giving the
  seat a private common directory.
- **Tool-list offices are not boxed, and their tool list does not
  bound them.** `implementer` and `implementer-sdd` (`cargo` and `git`),
  `intake` (`git`) and `researcher` (`git`, `ls` and `rg`, and it asks
  for `web-search` and `web-fetch`) declare a tool list, not hands. They
  run on the host as the operator's user, in the engine's environment,
  under claude's `--permission-mode acceptEdits`, which pre-approves
  file edits. Their list becomes `--allowedTools`, which removes no
  other tool. What removes a tool is the `--tools` list the engine passes
  on the boxed hands path, and the `--disallowedTools` it passes on
  every claude seat, naming each native power the seat does not hold.
  So what such a seat may run is decided by Claude Code's permission
  model and the operator's own Claude Code permission settings, less
  `WebSearch` and `WebFetch`, and the operator's MCP servers reach it
  ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)).
  The `bundles/verify` review seat is the same: unboxed, `acceptEdits`,
  with `cargo`, `git`, `ls`, `rg`, `gh pr view` and `gh run view`
  pre-approved. So are `recipes/fast`'s own inline implement and review
  seats, with the five `Bash` prefixes `cargo`, `git`, `ls`, `rg` and
  `mkdir` pre-approved, and a recipe that `extends fast` inherits them
  unless it replaces them: the default delivery's review gate is
  unboxed. Offices that declare neither, such as `triage` and the
  position seats, get no tool list at all.
- **An unboxed claude seat with no tool list keeps Claude Code's other
  defaults.** Brokkr passes no `--tools`, no `--settings` and no
  `--setting-sources`, so the seat has Claude Code's default tools,
  subject to the operator's own permission settings, and the operator's
  MCP servers. Only `WebSearch` and `WebFetch` are denied by name, unless
  the realm grants them. The shipped `triage` gate is such a seat.
- **The `harness` boundary is the harness's word.** Codex restricts by
  sandbox class (`read-only`, `workspace-write`), not by tool, and
  Brokkr has not measured what that sandbox enforces. An unboxed `exec`
  script starts from a rebuilt environment with a private `HOME` and
  `TMPDIR`, which confines nothing on disk: it may open any path the
  operator's user can. On Linux the engine attempts a network narrowing
  for it and does not report when the narrowing is unavailable.
- **Process settlement.** A timed-out attempt's whole process tree is
  ended before its report returns, and an end that cannot be proven
  parks the attempt `indeterminate` rather than letting a retry overlap
  it (`process.rs`). A dead hands server's scratch tree waits under the
  temporary directory for the next run, resume or rerun to start. That
  start removes it when its recorded owner is dead and no process holds
  its lock, keeps it while its lock is held, and keeps and names it on
  stderr when its lock cannot be probed.

## What Brokkr does not enforce

- **The operator's own MCP servers and settings.** Decision 0065 ruling
  6 says a harness's own MCP configuration is never inherited. The code
  does not yet hold that: an unboxed claude seat gets no
  `--strict-mcp-config` (`adapters/claude.json` passes it only in
  `hands.workspace`), so it starts every server in the operator's
  user-scope Claude Code configuration, with their credentials, and
  every claude seat, boxed or not, loads the operator's settings,
  `CLAUDE.md` and hooks. No Codex fragment clears the `mcp_servers` in
  `~/.codex/config.toml`, so every Codex seat starts them
  ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)).
- **Messages from other sessions.** A claude seat is a headless Claude
  Code session, and on a host that runs other interactive sessions it is
  reachable by their cross-session messages: an input channel outside
  the journal and outside the realm's grant
  ([#505](https://github.com/feedback-loop-ai/brokkr/issues/505)).
- **What an unmeasured harness reaches.** dsh, LaneTally and `exec`
  declare their native inventories unmeasured, so Brokkr switches none
  of their own powers off and claims no denial. dsh's base profile turns
  on web search and fetch
  ([#462](https://github.com/feedback-loop-ai/brokkr/issues/462)), and
  whether Claude Code's controls hold through the LaneTally wrapper is
  unmeasured.
- **A grant's restriction keys.** No harness receives them yet, so a
  realm cannot narrow a granted power to a host list; such a grant is
  refused or dropped instead (above).

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
  with hands, under every boundary: under `namespace` the box starts
  its commands from a cleared environment, and under `harness` the
  engine rebuilds an exec seat's. It also refuses a
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
- **What a capability returns is data too.** A model seat's prompt says
  what it holds and that whatever a capability returns is data, never
  instruction (`native_controls.rs`; decision 0065 ruling 7). A prompt
  is not a control: a seat persuaded otherwise is caught only by the
  checks below.
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
  The engine signs no seat commit. A boxed or `exec` seat's commits land
  unsigned, and the `dsh` driver turns signing off; any other unboxed
  seat's `git` reads the host's own configuration, signing included
  ([status](status.md)), so its commits carry the operator's signature
  when that configuration signs.
  The operator reviews the branch, pushes and merges. That review is the
  last check.

## Known limitations

- **No seccomp filter in the box**
  ([#220](https://github.com/feedback-loop-ai/brokkr/issues/220)).
- **The box's network is on or off, with no allow-list**
  ([#216](https://github.com/feedback-loop-ai/brokkr/issues/216)).
- **dsh turns on `web_search` and `web_fetch` in every seat**, and its
  unmeasured inventory means Brokkr switches neither off
  ([#462](https://github.com/feedback-loop-ai/brokkr/issues/462)).
- **Brokkr bounds neither a tool-less claude seat's other tools nor the
  MCP servers and settings of an unboxed claude seat or any Codex seat**
  ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)), as
  [what Brokkr does not enforce](#what-brokkr-does-not-enforce) states.
- **A seat can receive another session's messages**
  ([#505](https://github.com/feedback-loop-ai/brokkr/issues/505)).
- **A boxed exec attempt can park `indeterminate`** when a sandbox
  descendant is still exiting at the kill
  ([#504](https://github.com/feedback-loop-ai/brokkr/issues/504)).
- **A dead hands server's scratch tree waits for the next run's start**,
  which keeps a tree whose lock cannot be probed and, until 0.13.0,
  removes a lockless tree from before the lock even when a live server
  in another pid namespace still uses it
  ([#415](https://github.com/feedback-loop-ai/brokkr/issues/415)).
- **Runtime resources are unbounded** beyond timeouts and output caps
  ([#433](https://github.com/feedback-loop-ai/brokkr/issues/433)).
- **macOS has no box of Brokkr's**
  ([#253](https://github.com/feedback-loop-ai/brokkr/issues/253),
  [#269](https://github.com/feedback-loop-ai/brokkr/issues/269)).
