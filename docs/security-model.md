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
| `namespace` | Brokkr's box, built by bubblewrap 0.11 or newer on Linux 5.11 or newer. Linux only, WSL2 included. |
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

- **Namespaces.** The workspace box is built by `Namespace::open` and
  launched by `execute` and `run_boxed` (`hands.rs`); `box_argv` is
  retired, refuses every call and is removed in 0.13.0 (U8k). Every
  box, workspace and MCP server alike, opens with `--unshare-user
  --disable-userns --assert-userns-disabled`: a user namespace of its
  own in which no further one can be made, so a nested `unshare -U` or
  bubblewrap meets the box's own limit; a bubblewrap that cannot
  disable them refuses the option and the box does not start (U8j,
  `hands/namespace/isolation.rs`). Then `--unshare-pid`, `--unshare-ipc`,
  `--unshare-uts` and `--unshare-cgroup-try`, and `--unshare-net`
  unless the seat's hands grant the network. `--die-with-parent` ends
  the box with its caller, and `--new-session` detaches it from the
  caller's terminal.
- **What is observed is what is mounted.** Every sealed source, Git's
  config, the store's folder pins, the artifact root and an overlay's
  lower layer is mounted from the handle opened at the box's last
  observation, `--bind-fd` where it is writable and `--ro-bind-fd`
  where it is not (U8k, `hands/namespace/held.rs`), so a path swapped
  after the check is not what the box sees. Handing those descriptors
  to bubblewrap uses `close_range` with `CLOSE_RANGE_CLOEXEC`, which
  needs Linux 5.11 or newer. On an older kernel every boxed launch
  fails before bubblewrap runs, with `hands could not spawn bwrap:
  Invalid argument (os error 22)` (5.9 and 5.10) or `hands could not
  spawn bwrap: Function not implemented (os error 38)` (before 5.9)
  (`hands/namespace/entry.rs`; pinned by
  `a_kernel_without_close_range_s_close_on_exec_form_fails_the_launch`).
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
one). `parse_tools` in `crates/brokkr-runtime/src/agents/load.rs` reads three keys and refuses any
other: `tools.allow`, an ordered list of names from the adapter's map;
`tools.sandbox`, one of `read-only`, `workspace-write` and
`danger-full-access`; and `tools.mcp`, which must be empty, because an
agent no longer names an MCP server: one reaches a seat only through
the realm's grant, served by Brokkr's broker
([below](#mcp-capabilities-through-the-broker)). There is no `tools.deny` in this
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
| `claude` | `--allowedTools`, from the adapter's `tool_permissions` map (`lower_allow`, `agents.rs`). This is pre-approval: it removes no tool. A name that maps to `WebSearch` or `WebFetch` is refused (`native_alias`). On a seat without hands, where the list is lowered directly, an explicitly empty list is refused too; beside hands the list is dormant and passes no flag (decision 0043 ruling 2), and only a name mapped onto a native tool is refused (`agents.rs`). | Refused: a class is admitted only where an engine fragment already expresses it, and only Codex's do (`admit_local_sandbox`, `bundle.rs`). | On the boxed path, the hands fragment's `--tools ""` removes Claude Code's own tools and the engine fills that include list with the tools of the native powers the seat holds and nothing else, so a seat with no grant keeps only `workspace` (`final_tools`, `native_controls.rs`); `--strict-mcp-config` shuts out the operator's MCP servers (`adapters/claude.json`), as the empty set the engine adds does for a seat without hands on Linux. On every seat, `--disallowedTools` names each native power the seat does not hold. |
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
to that office in `realms.json`'s `capabilities` map (`forge.realms/v6`
or later), through a tool dialect (decision 0065 rulings 1, 3 and 5;
`Authority::resolve`, `capabilities.rs`). A recipe or an agent only
asks. A `requires` the realm does not grant refuses the compile, and a
`wants` is dropped, with a notice in the run manifest and in the seat's
prompt. Capabilities are off by default: no older realm file is read
as granting anything, no earlier run is grandfathered into a holding,
and a grant the seat cannot hold refuses or drops rather than being
reconciled into something narrower. A grant's dialect is either
provider-native (the harness's own search or fetch) or `mcp`, a server
Brokkr's broker runs ([below](#mcp-capabilities-through-the-broker)).
`forge.realms/v8` adds one reserved grant key, `retain`, which only
`false` may fill: the realm's veto of an MCP dialect's `retained`
response, never a demand for it; omitting it inherits the dialect's
declaration. In a v6 or v7 file a `retain` key is still an ordinary
restriction, never the veto, so moving to the veto means moving the
file to v8 ([contracts](../contracts/README.md)). This repository's own
`realms.json` grants nothing.

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

## MCP capabilities through the broker

Decision 0065's slice two serves a realm's `mcp` grant through
`brokkr broker serve`, which runs the dialect's server in a namespace
box of its own (decision 0077; `crates/brokkr-cli/src/broker.rs`,
`crates/brokkr-runtime/src/engine/broker.rs`,
`crates/brokkr-protocol/src/hands/namespace.rs`). Everything below is
what the merged code does and what the slice's evidence under
`openspec/changes/decision-0065-capabilities-slice-two/evidence/`
measured; what was not measured is said so.

**Where a grant can be held.** The compile fence is lifted on Linux
only (`McpFence::current`, `bundle/mcp.rs`; U9b1a, f0e0c9645). On macOS
every `mcp` grant, used or not, still refuses the compile: "realm
'<realm>' grants capability '<capability>' through dialect '<dialect>'
of kind 'mcp', whose broker support is not implemented until decision
0065 slice two". On Linux a seat holds one only where every check of
MB2 passes (`bundle/mcp.rs`); each refuses a `requires` and a `wants`
alike, rather than dropping it:

- the site has workspace hands in a `namespace` realm: "MCP capability
  '<capability>' requires workspace hands at this site", which is also
  what a seat without hands, or under `harness` or `open`, meets;
- those hands keep the network off: "… requires workspace hands with
  network false";
- the harness can keep the broker's evidence, and for a holding that
  binds secrets the store, from its own native tools: "… cannot
  protect broker evidence from this provider's native tools" and "…
  cannot protect its secret bindings from this provider's native
  tools";
- the harness's hands fragment carries a broker, which only Claude's
  does: a brokered grant on Codex or dsh refuses at compile with the
  driver's own cause, "provider '<provider>' has no measured strict MCP
  configuration for 'brokers'" (U9b1c, #940 f63c229ce).

So today a brokered MCP grant works on **Claude** alone. A Claude
work seat with boxed hands whose dialect binds a secret compiles since
U1d7 (#958, 794f7b171): Claude's boxed replacement shape is recorded
as measured on Claude Code 2.1.316, its store read denied by the U8h
box (U0b pass 9), so a keyed MCP server can be granted to such a seat.
A Claude seat without hands still refuses one with "cannot protect its
secret bindings from this provider's native tools", because native
Read reads the store there (U9b1b's F10). The dialect itself must be a stdio `argv` with an empty
restriction: a URL connection meets "MCP URL connections are not
implemented in decision 0065 slice two", an `argv` that holds a
`{{secret:NAME}}` reference meets "MCP connection argv cannot contain
secret references; declare environment bindings in secrets", and a
nonempty restriction meets "MCP nonempty restrictions are deferred to
the restriction-transport slice". A `requires` refuses with these, a
`wants` drops, and a valid grant no seat uses stays inactive. Neither
cause changes `forge.tool-dialect/v1`: a reference is still valid data
there, and a value reaches the server through its environment alone.

**What a Claude seat is launched with.** Its hands server and one
broker per holding, served as `cap-<capability>`, ride the one
`--mcp-config` document under `--strict-mcp-config`, and the launch's
`--allowedTools` names each broker once, as `mcp__cap-<capability>`, in
sealed order after the hands tool (U9b1d, #952 6071d04df). The final
check refuses a launch whose allow list names a `mcp__cap-` server the
set does not seal, or leaves one out. The broker is still the one gate
on which tools run: it lists and admits only the grant's tools, and
refuses any other call with "tool '<tool>' is not granted to capability
'<capability>'". It answers an `initialize` asking any protocol version
with its own, 2025-06-18, and refuses a method that version does not
admit with "MCP method is not admitted by this broker" (U9b1c). It
takes its harness's SIGINT or SIGTERM as the session's end and closes
its ledger, as Claude Code 2.1.316 ends every MCP server (U9b1e, #955
3bb0e566c). With both, the controller's check on Claude Code 2.1.316
saw a real seat holding a brokered grant complete its run, the call
settled `succeeded` and its response read back through `brokkr inspect
--capability-call`, also with the native `web-search` granted beside
it (`evidence/U9b1d.md`, `evidence/U9b1e.md`). The suites' harness is a
deterministic fixture: it proves the engine's path, and a real
harness's isolation is the U0 and U0b measurements', never the
fixture's.

**What the host must have.** Before it spawns a harness the engine
brokers for, `brokkr driver` sets `no_new_privs` on itself (U9b1b's
F1); on an ordinary Linux host there is nothing to do, and a driver
that cannot set it refuses: "seat refused to start: its driver could
not set no_new_privs before a harness the engine brokers MCP for: …". A
host whose `/proc/sys/dev/tty/legacy_tiocsti` is not `0` refuses before
any server starts: "MCP servers cannot start: the host's
/proc/sys/dev/tty/legacy_tiocsti is absent, unreadable or not "0"". The
store must lie outside the seat's reach, which the default store inside
the worktree does not ([secrets](guides/secrets.md)). And a boxed
project at or above `HOME` is refused, because its writable reach holds
the coordination root under `~/.local/state/brokkr/capabilities`: the
cause ends "can write '<root>', which an MCP server's box or lease rests
on" (U9b1a).

**The server's box.** The broker resolves the server's program as
written when absolute, and a bare name only on
`/usr/local/bin:/usr/bin:/bin`. The installed-entry layout it mounts is
deliberately conservative: a program in a system `bin` or `sbin`
directory is its own one-file tree, and any other program's package
root is its directory, or the one above when that is named `bin` or
`sbin`; a root of `/`, `HOME` or an ancestor of `HOME` refuses. That
tree is mounted read-only and nothing beside it, so a server that loads
code from elsewhere fails inside its box rather than being given more
(`ServerProgram::resolve`, `hands/namespace.rs`). Its sources are
protected: no seat may be able to write the program or its tree, which
may hold no multiply-linked file and must keep the identity observed at
admission, refused by "MCP server launch resolves inside seat-writable
reach", "MCP server box bind overlaps seat reach", "MCP server box
program tree contains a multiply-linked file" and "MCP server box
filesystem identity is not protected". A seat whose own writable reach
covers its server's program meets the lease's identity cause first
(U9b1b's F6). The box starts from `--clearenv` with a fixed
environment, `PATH`, `HOME=/runtime/home`, `TMPDIR=/tmp`, `USER`,
`LOGNAME`, `LANG` and `LC_ALL`, beside the dialect's declared secrets,
and a private `0700` tmpfs at `/runtime/home` and `/tmp` for each
server. The host-side launcher carries no secret and no code-loading
name: a declared loading binding such as `PYTHONPATH` reaches only the
confined server, at its exec, and a host name past the fixed set
refuses "MCP server box could not be established". Only
`/etc/ssl/certs` of the TLS configuration is mounted, never
`/etc/ssl/private`.

A dedicated installation is still not a credential check or a history
check. Admission discovers no credential the server's tree holds, and
masks only the values the dialect declares; it does not prove the
installed bytes were not planted earlier, by a seat or by anything else
on the host, and a pinned version is not provenance. Install a server
for Brokkr in a place of its own that holds no credential.

**Networks.** The hands and the server have separate networks: the
hands stay `network: false` whatever the server is given. A dialect
whose egress is `local` gets a network namespace of its own; one that
is `contracted` or `uncontracted` shares the host's network, with only
`/etc/resolv.conf` mounted for it (`network`,
`crates/brokkr-runtime/src/capabilities/binding.rs`).
A shared network reaches the host's loopback services and Linux
abstract Unix sockets as the host does: that is a declared reach, not a
filter, and there is no allow-list of hosts.

**Refused before the secret lookup, or failed after it.** Every box
admission refusal (layout, reach, links, identity, the store's reach,
the startup keys below, the terminal check and coordination) comes
before any store lookup and any server start, and the engine journals
the attempt's failure with its exact cause. Past the lookup the causes
are later ones: "MCP server secret environment could not be delivered",
"MCP server could not start inside its box", and, once it runs, "MCP
server version does not match the dialect's pinned version", "MCP
server protocol is invalid" or "MCP server call timed out". A broker
session's failure is durable: its ledger latches the first fatal cause
in its closing record, the engine reads the ledger only once the
attempt's processes are proven settled, and a failed close fails the
attempt with that cause even when the harness reported success
(`engine/broker/fold.rs`). A ledger left without its close refuses
"broker ledger lifecycle is invalid".

**Masking and the fixed startup keys.** The broker masks every bound
value in a server's response, and refuses one it cannot mask safely, a
value left in a JSON number, in the response's structure or in its id: "MCP
response cannot be safely masked" fails the call and retains nothing. A
dialect secret named `HOME`, `TMPDIR`, `USER`, `LOGNAME`, `LANG` or
`LC_ALL` collides with the box's fixed environment and refuses "MCP
server startup inputs are not protected from seat writes" before any
lookup; one the harness authenticates by, Claude's cloud credential
names among them, refuses "MCP secret binding '<name>' collides with
harness authentication". These are the broker's checks. The shared
secret grammar is unchanged (`[A-Z][A-Z0-9_]*`, with `PATH`, `IFS`,
`LD_PRELOAD`, `LD_LIBRARY_PATH` and `BROKKR_` refused first), and so is
the legacy `mask_json` every other surface uses, which still leaves a
digits-only value carried as a bare JSON number unrewritten (below).

**Retained responses.** A dialect that declares `retained: true`, in a
realm that does not veto it, has each response kept. Run-manifest v12
records the held capability's implementation and its retention as
declared, the realm's `inherit` or `veto`, and effective. The broker
publishes a response under the run's `artifacts/sha256/<hex>` before
anything cites it, and only once the attempt's processes are proven
settled, before its result is admitted, does the engine append one
`capability-call-settled` checkpoint per accepted call, carrying
seat-record v6's capability, dialect, tool, call id, state and, for an
answered retained call, the response's digest. Those checkpoints are
therefore late: they appear when the attempt settles, not as the call
happens. A retained response proves what the server answered and the
broker prepared, never that the harness or the model received it.
`brokkr inspect --run <run> --capability-call <id>` prints the call and
the verified bytes, and `--json` both. The budgets are fixed: 256 MiB
of retained responses per attempt, shared equally by its retaining
plans with an 8 MiB floor, so more than 32 retaining slots refuse;
8 MiB per response; 4,096 calls and 64 MiB per ledger. An exhausted
share refuses the next call before it is forwarded, "capability
evidence budget is exhausted", and nothing is evicted.

**Evidence is protected, and runs are serialized.** Every run takes an
exclusive lease on its canonical worktree, and so on the artifact root
beside it; a second run there refuses "run '<run>' holds this worktree,
its writers not proven settled" until the first's writers are proven
settled. An artifact root that once protected evidence stays protected
from every later writer of any worktree on the host. Where evidence is
at stake (the incoming attempt protects some, a standing one does, or
such a historical root exists), a writer that can reach it refuses with
"dispatch refused: capability evidence root cannot be protected from
every managed writer: …", and a writer Brokkr cannot box refuses with
"site '<site>' of run '<run>' writes outside a workspace box, which
cannot be kept from protected evidence or the coordination root". With
nothing at stake, an unboxed writer is admitted as before (operator
ruling of 2026-10-29; `engine/broker/exclusion.rs`).

Separately, a broker reserves its server's sources in the coordination
root, across every worktree on the host, until its attempt's processes
are proven settled; neither an engine's death nor a free lock releases
the reservation. A writer of any worktree that could write a reserved
source refuses with MB3's identity cause, and a boxed writer whose
reach covers the coordination root refuses on every run (U9b1a). A
record this build cannot read refuses rather than being guessed at:
"the protected inventory holds an unknown participant at '<path>'", or
one in another build's format, which names concluding its run with the
build that wrote it. A survivor not proven settled keeps its
reservation; `brokkr operator release` is the operator's attestation
for a run whose attempts ended with no such proof.

**Quiesce older writers before you rely on this.** An engine older than
this build takes no lease, so this one cannot see it, and upgrading the
binary does not stop an engine already running. Before a run that
retains evidence, let every older Brokkr engine finish, and stop any
other writer whose reach could overlap a server's sources or an
artifact root: a different worktree does not rule that overlap out.

**Startup budgets.** A broker's start, its host survey included, has
one 30 s bound; observing its sources is bounded at 1,000,000 entries,
depth 64, 40 link hops and 65,536 mount records, and a call at 1 MiB
of request, 8 MiB of response and 120 s. Each retaining member of a
panel surveys the host inside that one bound, so a large panel parks
under load with "MCP server box survey did not answer before its
startup deadline": U9b1b measured a 32-member panel parking at a load
near 6 and an 8-member one near 20 (`evidence/U9b1b.md`, F8). Judge
those budgets on a release build: in the controller's SD4 samples of
2026-10-31 a debug build's start was much slower on aarch64, because
provisioning hashes the whole running binary.

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
  `WebSearch` and `WebFetch`; the operator's MCP servers are shut out
  only where the engine gives the seat its strict empty set, on Linux
  ([below](#what-brokkr-does-not-enforce);
  [#467](https://github.com/feedback-loop-ai/brokkr/issues/467)).
  The `recipes/verify` review seat is the same: unboxed, `acceptEdits`,
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
  subject to the operator's own permission settings, and, off Linux, the
  operator's MCP servers. Only `WebSearch` and `WebFetch` are denied by name, unless
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
  6 says a harness's own MCP configuration is never inherited. On
  Linux the engine now holds that for the launches it seals, where the
  seat's shape is measured: a Claude or LaneTally seat without hands is
  given `--strict-mcp-config` and an empty `--mcp-config`
  (`with_empty_set`, `crates/brokkr-protocol/src/adapters/mcp.rs`), a
  boxed one its fragment's,
  and a Codex seat `--ignore-user-config --disable apps --disable
  plugins` (U9c2). What U0 and U0b measured those against is in
  [provider adapters](guides/provider-adapters.md#measured-mcp-isolation).
  It does not hold everywhere. On macOS, where no shape is measured, a
  claude seat without hands is served as before and starts the
  operator's user-scope servers; a driver run by hand is served as composed; beside
  `/etc/claude-code/managed-mcp.json` Claude Code itself refuses the
  strict flag (U0 C07), and Brokkr refuses a Codex seat on a host
  holding `/etc/codex/config.toml` or `/etc/codex/managed_config.toml`,
  so neither is excluded; and Codex's enterprise cloud configuration
  was never measured. Every claude seat,
  boxed or not, still loads the operator's settings, `CLAUDE.md` and
  hooks ([#467](https://github.com/feedback-loop-ai/brokkr/issues/467)).
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

- **The store** is `.brokkr/state/secrets.env` in the repository's
  folder, a recipe library's `.forge/secrets.env` beside the workdir, or
  the file `--secrets-file` names (decision 0083). `brokkr secrets set` reads a value from stdin
  and writes the store `0600`, and a store whose permissions are broader
  is refused on read. Names match `[A-Z][A-Z0-9_]*`. `PATH`, `IFS`,
  `LD_PRELOAD`, `LD_LIBRARY_PATH` and every `BROKKR_` name are refused.
  So is a value shorter than four bytes, and one shorter than eight
  draws a warning. Every workspace box hides the store and pins the
  folders around it, and refuses a store it cannot place safely; the
  [secrets guide](guides/secrets.md#the-store-and-the-box) says what
  it refuses and what is still exposed.
- **An MCP server's secrets** are the names its dialect declares,
  read from the store by the broker after its box is admitted and
  handed to the server's environment inside the box, never the
  seat's ([above](#mcp-capabilities-through-the-broker)).
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
  carried as a bare JSON number is not rewritten (the broker instead
  refuses such a value in a server's response). An encoding of an
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
- **Brokkr bounds neither a tool-less claude seat's other tools nor any
  claude seat's settings, and it shuts out the operator's MCP servers
  only on Linux, for a launch it seals**
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
