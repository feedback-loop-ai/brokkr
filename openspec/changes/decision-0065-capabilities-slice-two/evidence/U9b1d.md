# U9b1d — Allow a brokered server's tools on Claude's launch (tasks 46.6, 46.7)

Run 0065-slice-two-unit-u9b1d-see-th-01e1aaa2 (run 2), branch s2/U9b1d, base main at
301e2d639. This visit replays run 1's in-row patch (`SealedBroker::allowance`, the
`Servers::expand` join, `allowed()` with `McpRefusal::Unallowed`, and two fixture updates).
It adds ruling R4's two protocol files and the proofs. Every result below comes from a
command run in this session on this tree.

## What changed

The builder and its final check:

- `native_controls/mcp.rs`: beside sealed brokers, `Servers::expand` joins
  `mcp__cap-<capability>` after the value of the fragment's allow list, once per broker, in
  sealed order (`SealedBroker::allowance`, `allowing`). With no brokers the value keeps
  every byte.
- `native_controls/mcp.rs` also gains `brokered(document)`. It reads the `cap-` servers of
  one Claude MCP document with the same closed, each-name-once reader that `read` uses, and
  reads nothing from any other document.
- `adapters/mcp.rs`: once the document matches the sealed set, `final_set` calls
  `allowed()`. It requires every `mcp__cap-` allow entry to equal the sealed allowance
  exactly. Anything else refuses before spawn as `McpRefusal::Unallowed`, which is
  `pub(super)`.

The R4 files:

- `native_controls/grammar/managed.rs` is a new private child of `grammar.rs`.
  `managed_separator`, `managed_patterns` and their bounds moved into it unchanged, and
  `grammar.rs` re-exports them under the same public paths. `quality/ratchet.sh api`
  reports "public API holds".
- In that module, `plain` admits `-` only inside the server part of an `mcp__` name, never
  as its first character. Every other managed name keeps today's alphabet.
- The refusal's text is unchanged. It stays true of every name refused, because the
  reading now admits a superset of what it admitted before.
- `grammar.rs` goes from 1427 to 1356 lines.
- `native_controls/listed.rs`: W also admits `brokered(document)` for each `--mcp-config`
  that stands in the hands contribution's own argv. The comment where W is computed says
  that `final_set` binds this document, and `allowed` binds the allow list, to the sealed
  set.
- `specs/mcp-capability-broker/spec.md`: MB1 says the launch allows each served broker by
  name, and refuses a departing allowance before spawn.

Files left as they were:

- `crates/brokkr-runtime/src/engine/isolation.rs` needed no edit. `beside` already swaps the
  hands segment for `expand_beside`, which now carries the allowance. Its owning
  `brokered_tests` pin the new argv.
- No edit to `adapters.rs`, `adapters/*.json`, `adapters/mcp/admit.rs` or
  `native_controls.rs`.

## Proofs

| Proof | Test | Observed |
|---|---|---|
| One grant, through the real binary, read from the harness's own argv | `capability_broker::enabled::allowance::a_brokered_claude_seat_allows_its_hands_tool_and_its_broker_by_name` | one `--allowedTools`, exactly `mcp__brokkr__workspace,mcp__cap-library-docs`; servers `brokkr`, `cap-library-docs` |
| Two grants, sealed order, once each | `...::two_brokered_grants_are_each_allowed_once_in_sealed_order` | `mcp__brokkr__workspace,mcp__cap-library-docs,mcp__cap-more-docs` |
| No grant | `...::a_seat_holding_no_brokered_grant_allows_the_hands_tool_alone` | `mcp__brokkr__workspace` alone; servers `brokkr` only |
| Builder, by hand | `native_controls::mcp::tests::a_fragment_allows_each_broker_by_its_served_name_after_the_hands_tool` | one and two brokers as written by hand; with none, every part but the document equals the measured fragment; a TOML fragment gains nothing |
| Final check: unsealed `mcp__cap-x` added, sealed `mcp__cap-web` dropped, or a tool `mcp__cap-web__resolve` named instead | `adapters::mcp::tests::brokers::an_allowance_departing_from_the_sealed_brokers_refuses_before_spawn` | each refuses with the exact `Unallowed` text through `served`, the driver's check before spawn; the unaltered command is served |
| W never admits a `cap-` name the bound document does not carry | same test, and `native_controls::mcp::tests::only_a_documents_own_cap_servers_are_brokered` | a document missing `cap-web`, the hands alone, and a name written twice each refuse the allowance with the exact D6 "carried" cause; `brokered` reads nothing from a duplicate, an extra key, an entry with `env`, or an unexpanded token |
| Grammar: `-` only inside an `mcp__` server part, never first | `native_controls::grammar::managed::tests::a_dash_reads_only_inside_an_mcp_server_name_and_never_first` | `mcp__cap-library-docs` and `mcp__cap-web__resolve(x)` read; `Web-Search`, `Bash-(x)`, `mcp-x`, `mcp__-cap`, `mcp__-cap__resolve`, `mcp__cap-docs__re-solve`, `mcp__cap-docs__-`, `mcp_-cap` and `-mcp__cap` refuse with the unchanged cause |
| Authored `mcp__` allow still refuses | `authored_server_conflict` is unchanged; its existing tests pass in the protocol suite | — |

Pins updated for the allowance:

- `adapters/mcp/tests/brokers.rs`: `authored_beside` writes the allowance. The departure test
  keeps its final-set cases; a missing broker and a doubled name now refuse earlier, by W,
  and the new test above pins that.
- `native_controls/tests/servers.rs`: the `allowing` fixture.
- `brokkr-runtime` `provision_tests/brokered_tests.rs`: `cold` allows the broker its document
  serves. The file stays at its 911-line baseline.

## Removals, each compiling, then restored

| Removal | Failing test and assertion |
|---|---|
| Allowance: `allowing(…, &allowance[..0])` in `Servers::expand` | `a_fragment_allows_each_broker_by_its_served_name_after_the_hands_tool` (left `mcp__brokkr__workspace`, right `…,mcp__cap-library-docs`); also `hands_and_two_brokers_check_exactly_beside_native_off` |
| Final check: `allowed(…).or(Ok(()))` | `each_departure_from_the_brokered_set_refuses_at_the_final_boundary`, the case `…,mcp__cap-x`. Without `allowed`, check_final's general "does not carry an allow list as its sealed plan composes it" refused instead of `Unallowed`. The case has since moved, unchanged, into `an_allowance_departing_from_the_sealed_brokers_refuses_before_spawn` |
| Grammar widening: server part checked with today's alphabet | `a_dash_reads_only_inside_an_mcp_server_name_and_never_first` (`Err(NAME)` for the admitted list) |
| Grammar's leading-`-` guard removed | same test, case `mcp__-cap` (`Ok(["mcp__-cap"])`) |
| W admission: the `--mcp-config` filter made `&& false` | `a_brokered_set_is_served_cold_rejoining_and_replacing_as_its_literal`, `hands_and_two_brokers_check_exactly_beside_native_off`, `a_first_cold_attempt_probes_and_launches_without_its_sealed_names`, and through the binary the one-grant and two-grant `enabled::allowance` tests (run parked: "…names tool 'mcp__cap-library-docs' … (design D6)") |

After each restore, the owning tests passed again.

With the builder's allowance removed, the three real-binary tests still passed. The driver
recomposes the handed arguments, and its composer folds W, which now holds the document's
brokers, into the allow list. So the binary path alone does not bind the builder's join; its
own protocol test does.

## Gates on this tree

The format, lint, crate-suite, spec, spelling and diff gates all came back clean:

- `cargo fmt` and `cargo clippy` (workspace, all targets, `-D warnings`).
- `brokkr-protocol`: lib 1146 passed, plus `hands_exits` and `secret_drop`.
- `brokkr-runtime`: lib 1486, `it` 144, `capability_launch` 76, `operated_repo`,
  `queued_launch`.
- `brokkr-cli`, split by target:
  - lib and bins 1110;
  - `it` `capability_broker::enabled` 75 (530 s);
  - the rest of `capability_broker` 108;
  - the rest of `it` 745;
  - `driver_conformance`, `transcript_surfaces`, and every `heap_*` target.
- `brokkr-seatbelt-probe` 99.
- `openspec validate --all --strict` 36 passed; `typos --hidden`; `git diff --check`.

Ratchets and bundles:

- `quality/ratchet.sh files` and `clones` hold.
- `recipes/self` and `recipes/verify` compile, each with 0 unruled valuations.
- New files and their `quality/file-lines.txt` rows: `grammar/managed.rs` (95),
  `grammar/managed/tests.rs` (35), `capability_broker/enabled/allowance.rs` (89). Moved rows:
  `grammar.rs` 1356, `listed.rs` 388, `native_controls/mcp.rs` 783, `adapters/mcp.rs` 787.

## Pending and unmeasured

- Exact coverage (`scripts/coverage-exact.sh`) and macOS CI are pending.
- The controller's host evidence is pending: a real Claude seat calls a granted tool
  through the broker, and the call and its retained response are read back through
  `inspect`.
- Unmeasured: a seat holding a native capability beside a brokered one. The composer folds
  native selections into the hands' lists, so `beside` might refuse `Unexpanded`. The
  enabled world holds no native capability.
- Claude Code's permission reference (<https://code.claude.com/docs/en/permissions>) says
  `mcp__<server>` matches every tool of that server, and that a server name containing `__`
  is read as a shorter server. A capability holding `__` would therefore be allowed as a
  narrower, uncallable name. That fails closed, and it is a follow-up, not this unit's.
