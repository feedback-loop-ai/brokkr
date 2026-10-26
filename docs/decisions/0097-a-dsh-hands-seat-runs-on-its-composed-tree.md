# 0097 — A dsh hands seat runs on the composed tree it measured, fails closed without its tool, and takes its access from its office

Status: proposed
Date: 2026-10-26
Built: unbuilt

## Context

Decision 0043 gives a hand one tool; decision 0069 names it to a model that
defers MCP tools; decision 0065 slice two isolates the servers a launch can
reach. #896 taught dsh 0.2.1 to disable its ten effectful profile rows in the
one `--patch` overlay and to insert Brokkr's workspace server as its one
`dsh-mcp-client`. The controller then measured the driver on 0.2.1-rc.2
(`docs/evidence/adapters/dsh-hands-round2/`) and found the first shape did
not run at all: a `dsh-mcp-client` row without `transport` starts no server
and dsh exits 0 silently. `failOnStartupError: true` does not refuse a failed
server either: dsh runs the session and exits 0 with
`dsh: warning: 1 entry did not activate`. The same measurement found a boxed
hands seat composing the operator's ambient `DSH_HOME` (decision 0065 slice
two's U9b1 is the engine-only home, not built here), and a review panel
position hired at work class (decision 0021) receiving dsh's
`workspace-write` even though its office's charter is strictly read-only.

This proposal records the round-two semantics. Operator acceptance is
required; only the operator accepts. It edits no frozen contract, corpus or
production table (decision 0005).

## Proposed rulings

1. **The hands overlay carries the transport it was measured with.** Brokkr's
   row carries `transport: stdio` and `failOnStartupError: true`, in the field
   order the controller's measured overlay uses, and an exact-bytes test holds
   the written row to it. The row's disables leave twelve bookkeeping tools:
   the nine session-state tools and `mcp-resources`' `list_mcp_resources`,
   `list_mcp_resource_templates` and `read_mcp_resource`. `mcp-resources` is
   kept because it is in the measured composition and can reach only the
   servers in the seat's scope — and the composed-tree check below admits
   exactly one such server, `brokkr-hands`.

2. **Every hands launch composes its own tree first.** Before a session
   spawns, the driver runs `dsh --profile headless --patch <the seat's overlay>
   --dump-config` under the seat's own environment and home. Every row must
   be one of the measured 99 ids and package names; the one
   `dsh-mcp-client` row must be Brokkr's `brokkr-hands`; each of the ten
   measured effectful rows must read `disabled: true`; a repeated id, a
   second MCP client, an enabled effectful row, a dump that cannot be read and
   a `--dump-config` that fails all refuse, each naming the row where one is
   known. The version pin stays beside the check. No engine-only `DSH_HOME`
   is built for hands seats in this round; that is decision 0065 slice two's
   U9b1, and the standing fence still serves the operator's home.

3. **An office's access is typed data, not its gate class.** An office may
   declare `access`, `read-only` or `writes`, beside its hands. Absent is
   `writes`, the behaviour every office had before the key existed. The
   loader decodes the word once into `brokkr_protocol::native_controls::Access`
   (serde, matched exhaustively); the engine carries it to the site with the
   hands and writes the private `seat_access` carrier only for a site whose
   hands Brokkr boxes; the dsh driver decodes that carrier once into the two
   modes it may run. A missing or foreign word refuses rather than being
   guessed from the phase's class. An inline site, which has no office to
   declare one, still falls back to its gate class. A read-only
   `HandsSpec` that binds the worktree read-only does not exist in this round:
   `hands.rs` is decision 0065 slice two's and this change does not reach it;
   the dsh `sandbox-policy` row and the box remain the two controls, and the
   read-only hands bind stays owed.

4. **dsh's startup decides the seat.** A hands seat reads the stderr dsh
   wrote before its session: an `entry did not activate` warning or the
   `brokkr-hands (@deepseek-ai/dsh-mcp-client): Error:` line beneath it fails
   the seat closed with a typed refusal naming `brokkr-hands`; dsh's
   `SANDBOX_UNAVAILABLE` code or its "no sandbox backend is usable" message
   fails it closed too. The Landlock launcher's
   `partial enforcement (older Landlock ABI)` line is recorded as
   `sandbox: partial` on the seat's finished row, where the operator reads it.
   The words are bound to the installed packages' own text in the evidence
   directory.

5. **What remains unmeasured.** No macOS launch is measured; macOS is a
   supported host (decision 0063) and this round proves the overlay and the
   checks on Linux only. No live `mcp__brokkr__workspace` call through THIS
   driver is measured: the controller measured an equivalent one-tool stdio
   probe server answering `mcp__probe__ping`, not Brokkr's server under the
   seat. dsh's own native egress stays unmeasured and no OFF control is
   claimed. Route scope is not carried by the shape, as the adapter's earlier
   shapes already say.

## Alternatives and costs

Trusting `failOnStartupError` alone is refused by measurement: it changed no
exit status and no warning. Reading only the version pin is refused by the
same measurement: an ambient home composes beside the pin. Serving the
operator's home without reading what it composed is the fail-open path this
round closes. A new engine-only home is the sturdier fix, but it belongs to
decision 0065 slice two's U9b1 and is not built beside that session's files.

The cost is one `--dump-config` per launch (0.06–0.07 s, about 64 MB, no model
call) and a 99-pair allowlist that moves when dsh moves. A release whose
composition changes is refused rather than guessed, and the allowlist is held
to its measured dump by a test.

## Compatibility, migration and status

An office that declares no `access` keeps `writes`; every read-only office in
the library now declares `read-only`, so a work-class review position keeps
its charter's read-only intent. The private carrier replaces the `seat_class`
key this branch introduced; no contract, fixture, reference or production
table changes. Public API grows by `Access` and `ACCESS` in brokkr-protocol
and by `Agent::access` and `Resolution::access` in brokkr-runtime; brokkr-core,
brokkr-store and brokkr-view are unmoved. Adapter `dsh.json` and eight office
definitions move their bundles' witness digests, re-pinned from the suite's
reported values. Linux and macOS are the supported hosts; production remains
Rust. This proposal grants no new capability and reports no implementation
validation; the operator alone accepts it.
