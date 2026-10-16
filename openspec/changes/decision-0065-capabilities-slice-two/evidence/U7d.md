# U7d evidence: checked brokered configurations and selected discovery

U7d is delivered by run `0065-slice-two-unit-u7d-see-the--cc8c57a1`, the
unit's third run, on branch `s2/U7d` cut from main at `df112346`, as one
signed commit. The first run (`0065-slice-two-unit-u7d-see-the--5977e175`)
edited nothing and stopped blocked; the operator split the unit on
2026-10-16 into U7d, U7d2 and U7d3, and prep wrote that split into
design.md and tasks.md, which this commit carries unchanged. The second
run (`0065-slice-two-unit-u7d-see-the--dc274405`) built the protocol half
as fdd8102c, passed verify, and was stopped by review with
REVIEW-SECURITY-HOLD (H1 below) and one LOW (L1). Prep replayed fdd8102c
onto main without its tasks.md ticks, its file-lines rows and its
`capability_calls.rs` change; this run keeps that replay, repairs H1 and
L1, re-measures, re-ticks 37.1 and 37.2, and reruns every gate.

U7d does the protocol half, proved against hand-sealed serving inputs as
U7a was. It ticks tasks 37.1 and 37.2 (SD3, CC2, with SD2 and SD4's shared
duty) and leaves 36.1 and 36.2 open for U7d3. Everything below headed "this
run" was observed in this run's seat on Linux x86_64, on the tree this
commit records; the second run's own record is kept where its code did not
change, and named as the second run's; what was not observed is named
pending at the end.

## Scope

Production: the row's three files (`adapters.rs`, `adapters/mcp.rs`,
`native_controls/mcp.rs`) and one new private child of `adapters.rs`,
`adapters/discovery.rs`, declared `mod discovery;` under the standing rule
for a row file frozen at its baseline above the ceiling (adapters.rs is
7,113 lines against its 7,117 baseline). `adapters/capability_calls.rs`,
which `adapters.rs` declares `pub mod`, is unchanged from main.

Tests: `adapters/mcp/tests.rs` (registers the new child
`adapters/mcp/tests/brokers.rs`), `adapters/notice_tests.rs` (registers the
new child `adapters/notice_tests/discovery_tests.rs`), `adapters/tests.rs`
(one test moved out, below), runtime
`engine/capability_tests/provision_tests.rs` (registers the new child
`provision_tests/discovery_tests.rs`), and the CLI's
`tests/capability_broker/observer.rs` (the carried strace fix).

Bookkeeping: this change's design.md (prep's split, unchanged here),
tasks.md (37.1 and 37.2 ticked), this file, and `quality/file-lines.txt` and
`quality/too-many-lines.txt`.

## What changed in production

`native_controls/mcp.rs` (644 to 687 lines) gains three crate-private
facts on its existing types. `Transport::beside` writes the whole set's
one document, the box's hands with sealed brokers beside them, through the
same `Servers::named` and `document` builder `Servers::expand` uses, and
answers `None` where the set cannot be served as sealed.
`SealedServers::brokers` names the brokers of a brokered set and none
otherwise, matched exhaustively. `SealedServers::removals` is each broker's
sealed removals, each name once, sorted: the one list every spawn of the
attempt removes and the final check is handed. `SealedBroker::server` is
now the one home of the `cap-<capability>` name and its bare-name rule;
`Servers::named` and the discovery renderer both read it.

`adapters/mcp.rs` (649 to 735) is the serving edge. `declared` admits the
intent's set where it is the one the sealed inputs declare, or at least
one broker beside the box's sealed boxed hands; an empty broker list and
brokers beside no hands are not declared. `isolated` treats a brokered set
as a hands set riding the adapter's measured fragment, and refuses it for
any harness outside Claude's grammar with SI2's existing cause, `provider
'<codex|dsh>' has no measured strict MCP configuration for 'brokers'`,
under the standing fence as past the lifted one: a sealed broker set is
never served without its brokers. `served` hands `check_final` the sealed
brokers and the removals as `Serving.brokers` and `Serving.removed`, and
`final_set` (SI2's final check) owes the whole set's document from
`Transport::beside` instead of the hands-alone one. Nothing is appended to,
removed from or edited in the checked argv.

The removals reach the spawn through `Removing` (this run's H1 repair,
below): `Removing::attempt(input)` reads the attempt's own sealed serving
input and brings `SealedServers::removals` into force; dropping the value
puts back what was in force before it. `unbind`, called last in
`spawn_harness`, every harness's one spawn door, removes each name in force
after the extra environment and the secret bindings are applied, so a
removal wins over an exported, set or bound value.

`adapters/discovery.rs` (new, 122 lines) reads U7c's `capability_servers`
carrier once into a closed type: `servers` of closed `{server, capability,
tools}` entries and a `discovery_tool` that must be present, `null` or a
name. A carrier with no server, a server that is not `SealedBroker::server`
of its capability, a server named twice, a holding with no tool, a tool
that is not ASCII graphic without a backtick, a discovery tool outside
decision 0069's identifier grammar, or any extra member supplies nothing,
as a malformed hands notice does. `capabilities_contract` is the existing
capability paragraph with the fixed guidance appended inside it, only where
that paragraph is rendered: per server, ``You reach `<capability>` through
its server `cap-<capability>`, whose tools you call as
`mcp__cap-…__<tool>`, …``, then, where the serving adapter declares a
discovery tool, ``If one of these tools is not listed, use `<tool>` to load
it before calling it.`` `render_prompt` calls it in place of
`capabilities_paragraph`, one line for one line. No provider name decides
what it says, no new carrier or registry is read, and no plan, mount, store
or ledger locator reaches it. Its private `called` spells
`mcp__<server>__<tool>` (L1, below).

The carried LOW (decision 0071 ruling 5, U6f1a's re-vouch review) is
closed: `adapters.rs`'s hand-written `masked_text` is gone and its five
callers call `secret::mask_text`.

`adapters.rs` changed by those lines alone: `mod discovery;`, the
`spawn_harness` doc line and its `mcp::unbind` call, the
`capabilities_contract` call, the five `secret::mask_text` calls, and the
`Removing::attempt` line with its comment at the top of
`invoke_with_stager`. Its run-seat ending order, `lost::exit_code`, the
spawn fact and `lost::exited_non_zero`'s replacement are untouched.

## This run: H1, the dialect names reached the version probe

The review chief's HIGH: the second run recorded the removals in a
thread-local only once `served` returned a checked command. Claude's
`claude_launch` calls `qualify` before `served`, and `qualify`'s
`observed_version` spawns the harness's `--version` through `run_cli` and
`spawn_harness`, so a first cold launch with an enabled resume assessment,
or a qualified rejoin, spawned that probe with an empty list or the
previous attempt's. MB4 and decision 0077 ruling 3 require the names absent
from every harness process of the attempt.

The repair carries the attempt's own list, taken from its sealed serving
input, to every spawn of that attempt. `invoke_with_stager`, the one
invocation every production attempt is (`run_seat`'s closure), first takes
`mcp::Removing::attempt(input)`; it holds the value until it returns or
unwinds, so the version probe, the qualification, the model command and
Codex's cold replacement inside the attempt all remove the same names, and
the list ends with the attempt. `served` no longer writes the list; it
derives `Serving.removed` from the same decoded input through the same
`SealedServers::removals`, so the spawn and the final check read one list
by one derivation. Sealed inputs that fail to decode bring no names, and
every launch reading them already refuses as unreadable in `isolated`
(`intent`) before `qualify` or any spawn; Claude, Codex and dsh all call
`isolated` before their probe.

The spawn door still reads the list from thread state, as `lost.rs` reads
the spawn fact; the value that sets it is the explicit, attempt-scoped
`Removing`. Passing the list as a parameter instead would change the
signatures of `spawn_harness`, `spawn_piped`, `run_cli`, `observed_version`,
`qualify`, `invoke_stream_json`, `invoke_codex`, the dsh spawns and their
callers in a file frozen at its line baseline; that reason is carried to
the judge (decision 0071 ruling 10's "observing split from deciding" is
kept: the attempt decides the list once, the door applies it).

Four bindings, each driving `invoke_with_stager` with a synthetic Claude
that logs, per spawn, its role (`probe` for `--version`, `model`
otherwise), each of `U7D_DIALECT_A`, `U7D_DIALECT_B` and `U7D_KEPT` it
holds, and whether it was handed `--resume`; all three are exported under
`EnvGuard`, the hand-sealed brokered set is beside boxed hands, and the
Claude resume assessment is enabled for the shim's version. No provider or
real credential is used.

| Case | Test | Logged |
| --- | --- | --- |
| (a) first cold attempt, resume assessment enabled | `a_first_cold_attempt_probes_and_launches_without_its_sealed_names` | `probe:U7D_KEPT`, `model:U7D_KEPT` |
| (b) qualified rejoin | `a_qualified_rejoin_probes_and_rejoins_without_its_sealed_names` | `probe:U7D_KEPT`, `model:U7D_KEPT`, `model:resume` |
| (c) two attempts, removals `[A]` then `[B]` | `each_attempt_removes_exactly_its_own_names_and_none_outlive_it` | first: probe and model hold B and KEPT; nothing in force between them; second: probe and model hold A and KEPT |
| (d) hands only, after a brokered attempt | `an_attempt_with_no_brokers_removes_nothing` | probe and model hold all three |

Mutations, each compiled, failed as listed, and was restored
(`cargo test -p brokkr-protocol --lib brokers`):

| Behaviour removed | Failed |
| --- | --- |
| `Removing::attempt` in `invoke_with_stager` | (a), (b) and (c): every spawn logged both dialect names |
| the reviewed head's mechanism restored (no attempt scope; `served` records its list on success) | (a) and (b): `probe:U7D_DIALECT_A`, `probe:U7D_DIALECT_B` logged, model clean, H1 exactly; (d): the no-broker probe lost `U7D_DIALECT_A`/`B` to the previous attempt's list, its model did not; (c): `["U7D_DIALECT_A"]` still in force after the first attempt |
| `Removing`'s drop putting back the names before it | (c) (`["U7D_DIALECT_A"]` after the first) and `a_brokered_set_is_served…` (`["LIBRARY_DOCS_TOKEN", "WEB_TOKEN"]` after the drop) |

The chief's reproduction, redone on this run's head through the built
`target/debug/brokkr driver claude`: a scratch, uncommitted test under
`.forge/` wrote a `hello`, one `start` and a `shutdown`, with
`BROKKR_CLAUDE_BIN` naming a synthetic harness that appends
`probe:present|absent` or `model:present|absent` for a synthetic exported
`U7D_REVIEW_DIALECT_TOKEN`, and a valid hand-sealed Brokered set (one
`library-docs` broker removing that name, its document bound to the built
binary) beside boxed hands, with a supported boxed-workspace Claude
assessment for the harness's version. The driver accepted the start, ran
the probe and the cold model command, and ended with "seat wrote no result
file", as the synthetic harness writes none. The harness logged
`probe:absent` then `model:absent`. As a control, the driver rebuilt with
the reviewed head's mechanism (the second mutation above) logged
`probe:present` then `model:absent`, the chief's output; rebuilt again on
the repaired tree it logged `probe:absent`, `model:absent`.

## This run: L1, the MCP name's two homes

`discovery.rs` imported `capability_calls::mcp_tool`, a crate-private item
the second run added to the `pub mod` `capability_calls.rs`, which the
standing child-module rule does not cover; prep left that file at main,
where `mcp_tool` does not exist. `discovery.rs` now spells
`mcp__<server>__<tool>` in its own private `called`, whose comment names
`capability_calls.rs`'s `MCP_PREFIX` and `MCP_SEPARATOR` as the other home
and the test that holds them together.
`the_guidance_names_a_tool_as_its_observed_call_shows_it` asserts the
rendered contract exactly, its tool names written as `shown_tool` shows a
Claude `Observation` of `Tool::Mcp` for the same server and tool, and that
`Observation::claude` reads the rendered name back as that server and tool.
Mutations: `called` writing one underscore failed it and three other
discovery tests; `capability_calls.rs`'s `MCP_SEPARATOR` set to one
underscore (restored; that file is unchanged) failed it alone among the
discovery tests.

## Tests

| Suite | Test | What it proves |
| --- | --- | --- |
| `adapters/mcp/tests/brokers.rs` (new child of `adapters/mcp/tests.rs`) | `a_brokered_set_is_served_cold_rejoining_and_replacing_as_its_literal` | the attempt's removals are exactly `["LIBRARY_DOCS_TOKEN", "WEB_TOKEN"]`, the shared one once, and none after the attempt; Claude and LaneTally cold launches carry hands plus two brokers exactly as a hand-written document of the whole set; a measured rejoin keeps it and adds only `--resume`; an unmeasured rejoin's cold replacement is the cold command with `restrictions-unavailable` |
| same | `each_departure_from_the_brokered_set_refuses_at_the_final_boundary` | a broker missing, the hands alone, an extra broker, a name written twice, another executable, plan or digest, the strict flag dropped and another document appended after composition, and an unbare capability each refuse with the final-set cause; an empty broker list and brokers beside no hands are not declared; a broker with a fifth member is unreadable |
| same | `an_authored_copy_of_the_sealed_set_refuses` | the recipe's own words carrying the whole set byte for byte refuse before any provider work |
| same | `codex_and_dsh_refuse_a_brokered_set_whichever_way_the_fence_stands` | `isolated` refuses Codex and dsh with `Unmeasured { shape: "brokers" }` under both fences, and serves Claude |
| same | `the_spawn_removes_each_sealed_removal_from_the_harness_and_its_servers` | within an attempt, `spawn_harness` runs `/usr/bin/env` and `/bin/sh -c /usr/bin/env` (a server the harness spawns) with one removal exported (under `EnvGuard`), also set by the driver, and the other bound from a secret store; both see `U7D_KEPT` and `U7D_SET` and neither removal |
| same | the four H1 tests above | probe and model of every attempt shape hold none of the attempt's names, and only its own |
| `adapters/notice_tests/discovery_tests.rs` (new child of `adapters/notice_tests.rs`) | `codex_deferred_discovery_names_the_capability_its_tools_and_the_discovery_tool` | SD3's scenario: the whole contract, hands notice included, is the literal naming `library-docs`, both callable tools and `tool_search`; Claude reads the same bytes |
| same | `a_fallback_hears_no_discovery_tool_and_a_dropped_holding_advertises_no_tool` | a null discovery tool renders no discovery line; a dropped holding with no carrier renders today's contract |
| same | `hostile_prose_can_neither_add_nor_suppress_the_guidance` | house rules quoting the guidance and a context holding a carrier add none; with the carrier, the same prose suppresses none; a carrier with no capability contract renders nothing |
| same | `a_carrier_that_is_not_the_engines_supplies_no_guidance` | fourteen malformed carriers, a plan locator and a store locator among them, render today's contract exactly |
| same | `native_and_no_mcp_prompts_are_unchanged` | native and no-capability contracts as literals; exec reads none beside a carrier |
| same | `the_guidance_names_a_tool_as_its_observed_call_shows_it` | L1's parity, above |
| `engine/capability_tests/provision_tests/discovery_tests.rs` (new child of `provision_tests.rs`) | `the_selected_servers_render_inside_the_serving_candidates_capability_contract` | the engine's own `SiteMarks::site` then the protocol's `render_prompt`, over one reused input: the Codex primary is told the server, tools and `tool_search` beside a hands notice naming `tool_search`; a fallback with no declaration hears no discovery tool; a dsh fallback and a dropped wanted holding hear no guidance; the prompt holds neither the server launch, the run directory nor the state root |
| `brokkr-cli/tests/capability_broker/observer.rs` | `a_tracee_dying_mid_call_adds_no_call_to_the_recorded_log` | a recorded log with strace's `???()` marker after the one `execve` reads as that `execve` alone |

Test moves: U7a's `a_sealed_broker_set_refuses_at_the_serving_edge` left
`adapters/tests.rs` (18,981 to 18,945 lines), because a brokered set beside
boxed hands is now served to Claude; its unreadable-counterfeit case moved
into `each_departure_from_the_brokered_set_refuses_at_the_final_boundary`
and its fences into `codex_and_dsh_refuse_a_brokered_set_whichever_way_the_fence_stands`.
`the_admitted_launcher_runs_through_its_handle_with_no_environment` now
reads its log through `traced_calls`, the helper the new test binds; its
exact single-`execve` assertion is unchanged.

The requested-effect digest is U7c's
`a_site_without_a_brokered_holding_provisions_nothing_and_the_requested_digest_holds`,
which passed in this run's runtime suite; the renderer reads only
spawn-time marks, outside the requested input. MB4's store-only child
bindings stay with U6's controls in `brokkr-cli/tests/capability_broker`
(`a_protected_installed_server_alone_receives_its_bindings_beside_its_fixed_environment`,
`the_server_inherits_only_its_stdio_and_its_environment_from_the_private_home`,
`binding_names_are_checked_then_fixed_keys_refuse_before_lookup`), which
passed in this run's `--test it` run.

## Mutations rerun in this run

Beside H1's and L1's above, each compiled, failed as listed, and was
restored; the filtered suites passed again afterwards.

| Behaviour removed | Failed |
| --- | --- |
| the guidance appended to the contract | `codex_deferred…`, `a_fallback_hears…`, `hostile_prose…`, `the_guidance_names…`, and the runtime `the_selected_servers_render…` |
| the tool identifier rule | `a_carrier_that_is_not…` ("a backtick and prose": the prose reached the prompt) |
| marks.rs clearing the carrier (selection, out of row, restored unchanged) | the runtime `the_selected_servers_render…` at the dsh fallback (line 62: stale guidance where no broker serves) |
| `secret::mask_text` on stream stderr (plain lossy decode) | `every_model_harness_receives_its_bindings_and_masks_its_stderr` ("Claude: leaked tok-3xample-value") |
| the `???(` filter | `a_tracee_dying_mid_call_adds_no_call_to_the_recorded_log` (the marker line kept) |

## The second run's mutations

The second run recorded these against code this run did not change; they
were not rerun here. Its rows about the served command recording its list
and the record cleared on entry are gone with that mechanism (replaced by
H1's three rows above).

| Behaviour removed | Failed |
| --- | --- |
| guidance only inside a rendered contract | `hostile_prose…` (the carrier with no contract) |
| the server named `cap-<capability>` | `a_carrier_that_is_not…` ("another server") |
| a server named once | `a_carrier_that_is_not…` ("named twice") |
| `deny_unknown_fields` on a server entry | `a_carrier_that_is_not…` ("a plan locator") |
| `deny_unknown_fields` on the carrier | `a_carrier_that_is_not…` ("a store locator") |
| the discovery member required | `a_carrier_that_is_not…` ("no discovery member") |
| the discovery tool's grammar | `a_carrier_that_is_not…` ("a spaced discovery tool") |
| at least one server | `a_carrier_that_is_not…` ("no server") |
| at least one tool | `a_carrier_that_is_not…` ("no tool") |
| a discovery line where none is declared | `a_fallback_hears…` and the runtime test (fallback heard `tool_search`) |
| guidance where the input carries no carrier | `native_and_no_mcp…`, `hostile_prose…`, `a_fallback_hears…`, `a_carrier_that_is_not…` |
| the Codex/dsh broker refusal | `codex_and_dsh…` ("codex, standing" served) |
| an empty broker list not declared | `each_departure…` ("no broker") |
| brokers declared only beside sealed hands | `each_departure…` ("no hands") |
| brokers handed to the final check | `a_brokered_set_is_served…`, `each_departure…` (final-set refusal) |
| removals handed to the final check | the same (final-set refusal) |
| `final_set` owing the whole set | `a_brokered_set_is_served…`, `each_departure…` |
| `unbind` in `spawn_harness` | `the_spawn_removes…` (`LIBRARY_DOCS_TOKEN` and `WEB_TOKEN` reached the harness) |
| `unbind` before the secret bindings | `the_spawn_removes…` (`WEB_TOKEN`, bound, reached the harness) |
| the authored duplicate refusal | `an_authored_copy…` (the authored-server refusal answered instead) |

`a_carrier_that_is_not…`'s unbare-capability and `final_set`'s
`Transport::beside` refusal are each one of two layers: with either
removed, `check_final`'s own reading (U7a, `Servers::named`) still refuses,
so their removal alone does not fail a test. The `each_departure…` case
covers the line.

## Which of 36.1's scenarios U7d proves

Against hand-sealed serving inputs, U7d proves: the expanded set wired
into the Claude and LaneTally builders and the final consumption point;
Codex and dsh refusing; no modification after checked command creation;
configuration carrying only each broker's plan locator and digest, with no
server value or bootstrap entry; cold, rejoining and replacement commands
matching an independent literal; departures refusing at the final
boundary; and every sealed removal gone from every harness process of the
attempt, its version probe included, and from a server it spawns. Not
proved here: engine-composed cold, resume and replacement commands
carrying brokers (U7d2), and the exact requires/wants
authentication-collision causes (U7d3). The box and clearance intent stay
in U7c's protected plan, and the broker's pre-secret source and readiness
checks are U6's and U7c0b's, unchanged by this unit. 36.1 and 36.2 stay
unticked.

## Gates and measurements, this run

Formatting was clean, and workspace clippy over all targets and features
with warnings denied finished with no finding. Each crate's suite passed,
run crate by crate: brokkr-protocol 958 unit tests (3 ignored, as on main)
and its integration binaries; brokkr-runtime 1,013 unit tests,
`capability_launch` 76, `it` 132 (the witness digests among them, unblessed),
`operated_repo` and `queued_launch`; brokkr-cli 862 unit tests, `it` 690 (3
ignored), `driver_conformance` 27, `transcript_surfaces` 14 and the three
heap tests; brokkr-core, brokkr-view, brokkr-store, brokkr-bridge and
brokkr-seatbelt-probe. The self bundle compiled. `openspec validate --all
--strict` passed 26 of 26, `typos --hidden` reported nothing, and `git diff
--check` was clean staged and unstaged. `quality/ratchet.sh` held file
size, duplication and the public API: every new item is `pub(crate)` or
`pub(super)`, and no public listing moved.

Measurements: `quality/file-lines.txt` rows were set from `wc -l` for every
changed file and the three new test children and one new production child
(`adapters.rs` 7,113, `adapters/mcp.rs` 735, `adapters/discovery.rs` 122,
`native_controls/mcp.rs` 687, `adapters/mcp/tests.rs` 848,
`adapters/mcp/tests/brokers.rs` 501, `adapters/notice_tests.rs` 305,
`adapters/notice_tests/discovery_tests.rs` 236, `adapters/tests.rs`
18,945, `provision_tests.rs` 1,138, `provision_tests/discovery_tests.rs`
70, `capability_broker/observer.rs` 1,041). The too-many-lines listing,
re-measured with `measure.sh`'s forced clippy lint, holds 286 entries as
the baseline does; only `run_seat_with`'s line moved (6,491 to 6,493). No
suppression was added, and the suppressions test in the CLI's `it` passed.
`scripts/measure-budgets.sh` ran; it measured 329 packages, as committed,
and prompt budgets lower than committed by amounts this unit does not move
(it adds no prompt bytes without a carrier, and no budget site carries
one), so its rewrites of the three budget files were reverted rather than
carried. No witness or compose input changed.

## Pending

Exact coverage (`scripts/coverage-exact.sh`) was not run here; it needs a
host outside the workspace box and is pending for CI or host validation.
macOS build and tests, MSRV and every remote CI job are pending on the PR.
`the_admitted_launcher_runs_through_its_handle_with_no_environment` passed
in 2.9 seconds with `/usr/bin/strace` installed and printed no skip notice;
whether it traced rather than returning early at its sealed-observation
step was not separately recorded.
