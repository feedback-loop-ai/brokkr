# U9b1c evidence — MCP version negotiation, the compile's broker refusal and `Handoff`

Run 0065-slice-two-unit-u9b1c-see-th-60e69132, branch `s2/U9b1c` from main at
`c21ab42d9`. One signed commit per item: `42d54bb42` (negotiation, with the commission's
design and tasks), `695bb5dda` (the compile's refusal) and the branch head (`Handoff`),
the tasks and this file folded into the third. The tests ran on the third commit's code,
before the tasks and this file were folded in.
Every claim below names a command the implement seat ran on it, or is marked pending.

The protocol source relied on is MCP 2025-06-18's lifecycle,
<https://modelcontextprotocol.io/specification/2025-06-18/basic/lifecycle>, read in the
seat. Its Version Negotiation says: "If the server supports the requested protocol
version, it MUST respond with the same version. Otherwise, the server MUST respond with
another protocol version it supports." Its Operation section says both parties "MUST
respect the negotiated protocol version". The controller's record of the real-harness
check is `.forge/u9b1c/check-2026-10-31.md`, read but not cited by any test.

## Item 1 — negotiation (46.3, 46.4; MB3)

`calls.rs` answers every `Request::Initialize` with `greet`, MB3's 2025-06-18 result.
`calls/request.rs` (an existing private child of `calls.rs`) still decodes
`protocolVersion` as a `String`, so a missing or non-string version keeps
`Refused::Invalid` ("MCP tool call request is invalid", -32600). The harness-facing
`Refused::Version` had no other consumer and is removed (ruling 6). `Fatal::Unsupported`
now writes its own text, unchanged, for a child server off 2025-06-18. The session's
admitted methods and bounds are untouched, so the session stays at 2025-06-18. MB3's
text in `specs/mcp-capability-broker/spec.md` says so, and its "Protocol and resource
bounds" scenario is rewritten to match.

| Test | Observed | Compiling removal and failure |
|---|---|---|
| `broker::rpc::tests::the_harness_is_answered_here_and_its_granted_call_forwarded_once_recorded` | Request 3 asking 2025-11-25 is answered with the exact 2025-06-18 greeting; request 6 with `protocolVersion: 7` refuses -32600 "MCP tool call request is invalid"; ids 4 (no params) and 5 (an array) as before; the call forwarded once | `request.rs` mapping a non-2025-06-18 greeting to `Request::Other`: `left` holds `-32601 "MCP method is not admitted by this broker"` for id 3 where `right` holds the greeting (tests.rs:1020) |
| `capability_broker::enabled::negotiation::a_harness_asking_another_version_is_answered_mb3s_and_completes_a_granted_call` (new child `enabled/negotiation.rs`) | Through `brokkr run`, the real broker and server box: the harness heard exactly the 2025-06-18 greeting for its 2025-11-25 `initialize`, the listing `resolve` and `read`, -32601 for `resources/list` after negotiation, the same greeting for a 2025-06-18 `initialize`, -32600 for a numeric version, and the call's answer; broker exit 0; `inspect --capability-call` opens the retained response `{"result":{"content":[{"type":"text","text":"resolved"}]}}`, state `succeeded`, with its digest on the call row | The same removal: `left` begins `{"error":{"code":-32601,…},"id":1}` where `right` begins with the greeting (negotiation.rs:58) |

Both tests passed again with the removal restored: `cargo test -p brokkr-cli --lib
broker::rpc` gave 95 passed, and the negotiation test passed in 9.96 s through real
dispatch, not skipped. `crates/brokkr-cli/tests/capability_broker/protocol.rs`'s
`a_server_off_its_pin_or_lacking_a_granted_tool…` still pins the child server's
2024-11-05 refusal, and `rpc/tests/masking.rs`'s id-masking case for an `initialize`
asking 2024-11-05 still ends on the masking cause before any answer. Both are unchanged
and pass in the suites below.

## Item 2 — the compile's refusal (46.3, 46.4; SI2)

`bundle/mcp.rs` gains `unbrokered`, an exhaustive match on the public `AdapterKind`:
Codex and dsh carry no broker, while Claude, Lanetally (Claude's grammar), exec and an
opaque command do. `sealed` asks it after `contribute`, on the set `brokering` reads from
the site's sealed outcome, and admits its cause through `McpFence::admit`, before the SI2
verdict, the order the driver's `isolated` uses. The cause is the driver's own
`StrictCause::Unmeasured { provider, shape: "brokers" }` under the site's prefix. Nothing
in `crates/brokkr-protocol/src/adapters.rs`, `adapters/mcp.rs`, `adapters/mcp/admit.rs` or
`adapters/*.json` moved.

| Test | Observed | Compiling removal and failure |
|---|---|---|
| `bundle::agent_tests::mcp_tests::a_brokered_grant_on_a_harness_carrying_no_broker_refuses_at_compile_with_the_drivers_cause` | Under the lifted fence, one office per provider (`opus`, `astra`, shipped dsh's `flash`), each proved on every MB2 axis with carriage measured, so no earlier check refuses: `requires` gives Claude `Ok(["library-docs"])`, Codex and dsh `Err("seat 'research' (office 'researcher') in realm 'private': provider '<provider>' has no measured strict MCP configuration for 'brokers'")`; `wants` refuses Codex and dsh the same; no holding compiles all three; the standing fence (macOS's) compiles all three holding it | `Some(AdapterKind::Codex \| AdapterKind::Dsh) => true`: `left: [Ok(["library-docs"]), Ok(["library-docs"]), Ok(["library-docs"])]` where `right` holds the two refusals (mcp_tests.rs:1662) |

The refactor moves `the_pass_judges_each_resolved_candidate_and_refuses_in_the_sites_words`'s
shape and resolver setup into the shared builders `proved_boxed`, `proved_carriers` and
`holding_docs`, its assertions unchanged. With the removal restored, `cargo test -p
brokkr-runtime --lib bundle::agent_tests::mcp_tests` gave 22 passed and the whole runtime
lib 1451.

## Item 3 — `Handoff` through real dispatch (46.5, closing 46.1)

`windows.rs` held `Handoff` by a store read. It now holds it by `exec_held`, twice 200 ms
apart: a process below the broker in the tracer's stop (`t`) inside `execve`
(`/proc/<pid>/syscall` 59 on x86_64, 221 elsewhere), whose only descriptor past its
standard streams is a pipe the broker also holds. A dump taken in the seat at the held
window showed that shape: bootstrap `comm` `brokkr`, state `t`, syscall 59, fds 0–2 its
stdio pipes and fd 39 `pipe:[252692612]`, which the broker held at fd 38. Its control
and ready pipes were gone. `exec_confined_with` calls `execve` only after `received`
returned the whole frame and its end, so this establishes the delivered-frame,
held-exec window with the status pipe open.

| Test | Observed | Compiling removal and failure |
|---|---|---|
| `windows::a_harness_dying_at_a_held_handoff_ends_its_broker_before_its_deadline` | The harness SIGKILLed at the held window, its input kept by the keeper: the broker gone in 10–40 ms and, every startup window now, before its 30 s deadline from the harness's start; nothing below it or in its box's mount namespaces left; the ledger a prefix of or closed on `refused: handoff`; the park `awaiting_operator` on the handoff cause or the driver's lost result; `inspect`'s `last_seat` `work` with `result` null and no call row; the store read and the server never read | The broker's cancellation at the handoff wait (`exec_status` polling `(deadline, &\|\| false)`) together with the engine's sweep (`Host::REAL`'s `kill_group` and `kill` as no-ops): "Handoff: the broker lasted 30.041821621s" (windows.rs:485). Apart: the cancellation alone removed passes on the sweep (broker gone in 40 ms); the sweep alone removed fails on its park, which then adds "the attempt is not proven over: its process group … still had members after the kill", as U9b1b's store-read row did |
| `windows::the_engines_timeout_at_a_held_handoff_ends_it` | `limits.timeout_seconds` 25, the engine's sweep: parks "attempt exceeded its Ns deadline and was killed; its process tree is gone, and what it did is unknown"; the broker and everything below it gone at once; `inspect` keeps no call; reads held to the table | The attempt's deadline at `timeout_seconds * 4` (`engine/dispatch.rs`): it parks `Some("MCP server secret environment could not be delivered")`, the broker's own cause, not the timeout (windows.rs:579) |

The tracer's delayed exec is allowed for. A process SIGKILLed inside the delayed
`execve` ends only when the 60 s delay does, wherever the tracer outlives the broker. In
the sweep-removed run, settlement took about 59.5 s. So `Window::delayed` adds `HELD_MS`
to `TEARDOWN` for the death case's settlement at `Handoff` alone. The broker's own end
keeps `TEARDOWN`, and the timeout case keeps `TEARDOWN`, as the sweep there kills the
tracer too. Since the 2026-11-01 ruling, `TEARDOWN` (30 s) equals MB3's whole startup,
so for every window where the startup deadline still stands (all but `Call`, by
`Window::starting`), `cancelled_at` now also asserts that the broker's lifetime from the
harness's start is under `STARTUP`. That is what binds the death case. `expired_at`
shares the new `lasted` and `STARTUP`, its assertions unchanged, and the module text
again claims every window. No product bound moved, and no panel proof was added (F8).
With every removal restored, all 17 `windows::` tests passed: the six death cases in
44.8 s, the eleven others in 104.7 s.

## Inventory

Production: `crates/brokkr-cli/src/broker/rpc.rs` (786 → 784 lines),
`crates/brokkr-cli/src/broker/rpc/calls.rs` (693 → 695),
`crates/brokkr-cli/src/broker/rpc/calls/request.rs` (139 → 140, an existing private child)
and `crates/brokkr-runtime/src/bundle/mcp.rs` (770 → 788). Tests:
`crates/brokkr-cli/src/broker/rpc/tests.rs` (1,995),
`crates/brokkr-cli/tests/capability_broker/enabled.rs` (691),
`crates/brokkr-cli/tests/capability_broker/enabled/negotiation.rs` (new, 82, registered
as `mod negotiation;`), `crates/brokkr-cli/tests/capability_broker/enabled/windows.rs`
(686) and `crates/brokkr-runtime/src/bundle/agent_tests/mcp_tests.rs` (1,771). Each has
its row in `quality/file-lines.txt`. There were no test moves, no new suppression, no
new public item (`quality/ratchet.sh api`: "public API holds"), and no witness or
compose pin moved: the runtime's `it` target, `witness_digests` included, passed
unblessed. Documents: MB3's text and scenario, `tasks.md` (46.1, 46.3–46.5 and two
status lines), `evidence/U9b1b.md`'s Tasks paragraph and this file.

## Gates on the tested head

| Gate | Result |
|---|---|
| `cargo fmt --all -- --check`, `git diff --check c21ab42d9`, `typos --hidden` | clean |
| `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` | clean |
| `openspec validate --all --strict` | 36 passed, 0 failed |
| `quality/ratchet.sh files`, `clones`, `api` | file size, duplication and public API hold |
| `compile --bundle recipes/self` and `recipes/verify` | both compile; policy sweeps 48/11 and 16/5, 0 unruled |
| Workspace tests, crate by crate under `timeout 590` | core, view, store and bridge 16 targets ok; protocol 4 ok; runtime lib 1451, `capability_launch` 76, `it` 144, `operated_repo` 1, `queued_launch` 8; seatbelt-probe 99; CLI lib 1110, `it` 848 outside `capability_broker::enabled`, the enabled suite 55 plus the 17 `windows::`, `driver_conformance` 27, `transcript_surfaces` 15, every `heap_*` target ok; no failure |

## Pending

`scripts/coverage-exact.sh` and `quality/ratchet.sh crap` were not run in the seat. Boundary
tests need a namespace the box will not nest, so the controller's host and CI run them.
The macOS legs of CI are pending: the change adds no Linux-only production code, and its
new tests sit in the Linux-only `enabled` suite or under the test fence. The host
evidence is the controller's and pending: a real Claude seat under `no_new_privs`
calling the granted tool through the broker, with the call and its response read back
through `inspect --capability-call`.

## Notes for the council

A wanted brokered grant on Codex or dsh refuses at compile rather than dropping. That is
the design's word ("The compile now refuses such a site"), and it matches the driver,
which refuses whatever the ask. The before-deadline check in `cancelled_at` also binds
U9b1b's five other startup windows. They passed under it. It is a stronger assertion
in this unit's test file, not a change to their setup. SI2's text in
`specs/strict-mcp-isolation/spec.md` is not edited: the row names only MB3's. A survey
keeps the site's refusal in `refused`, which `doctor/capabilities/mcp.rs`'s
`report_refused` reads, so the doctor sees the new cause there. It is not added to
the per-serving SI2 verdicts in `Planned.strict`, and no doctor test pins it.
