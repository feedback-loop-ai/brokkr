# U7b evidence: consume adapter carriage and selected holdings

U7b is delivered by run `0065-slice-two-unit-u7b-see-the--82c24f5f` on branch
`s2/U7b`, cut from main at `c77ccf26`, as one signed commit. It closes tasks
34.1 and 34.2 (requirements MB1, MB2 and SI2, with SD2 and SD4's shared
duty). Everything below was observed in this run's seat on Linux x86_64;
what was not observed is named pending at the end.

## What changed in production

Only the row's three production files moved, and no new production file was
added.

`agents/mcp.rs` reads the formerly dead carriage axis on its own.
`McpSupport::carries` answers whether a provider can load an engine-written
MCP server: only a measured carriage can. An unsupported or inapplicable
carriage gives its own reason, a declared-unmeasured one `its carriage is
unmeasured: <reason>`, and the legacy forms `its adapter declares no
measured carriage`, so a legacy server map, whatever server it names, still
confers nothing. The refusal is the typed `Uncarried`, whose text is MB2's
`provider '<provider>' cannot carry MCP (<reason>)`. A carriage is never read
from an isolation shape, nor a shape from the carriage.

`agents.rs` gains one line, `pub(crate) use mcp::Uncarried;`, so the bundle
can name the type. It needed no composition fact beyond the intended set it
already records: each candidate's adapter is reached by its own provider,
as SI2's record already reaches it. 1,669 to 1,670 lines, under its 1,671
baseline, which is lowered to 1,670.

`bundle/mcp.rs` materializes and judges each serving's contribution.
`McpIntent::contribution` derives the engine-owned servers from the set the
serving intends and the typed MCP holdings its own outcome seals: the empty
set, the hands alone, exec's no-model set, or `Brokered`, the hands beside
each holding's own `cap-<capability>` server (the holding's sealed
`Implementation::Mcp` server name) in the outcome's order. A native holding
contributes nothing. A set without the hands server refuses
`MCP capability '<capability>' requires workspace hands at this site`, and
exec's refuses `MCP capability '<capability>' requires a model harness with
an engine-owned MCP channel`, whatever the ask's strength.

`McpSite` carries one serving's intended set, its own adapter and its SI2
record. `McpSite::contribute` runs once that serving's outcome is resolved,
in this order: the set must hold the hands server and a model channel (an
opaque command, which no adapter answers for, has none); then the
provider's own carriage, on which a `requires` ask refuses
`requires capability '<capability>' through dialect '<dialect>', but provider
'<provider>' cannot carry MCP (<reason>)` under the site prefix and a `wants`
ask drops, its holding removed, its reason recorded in `not_held` and the
notice `<site>: dropped wanted capability '<capability>' through dialect
'<dialect>' because provider '<provider>' cannot carry MCP (<reason>)`
pushed, both exactly MB2's scenario forms; then each remaining server's
exact name, which refuses `MCP server name 'cap-<capability>' is not
representable by provider '<provider>'` unless it is one bare name (ASCII
letters, digits, `_`, `-`), the only name U7a's builder writes. Nothing is
shortened, normalized or renamed. Native plans are left as resolved, so an
unheld native power stays OFF. A candidate that composed nothing launches
nothing and is not judged.

`resolved` builds one `McpSite` per serving, a candidate's from its own
composition and adapter, an inline site's from its facts and driver, and
calls `contribute` on each outcome before SI2's fence admission, which is
unchanged. The resolver reaches `resolved` as a closure seam (decision 0071
ruling 2): `candidate_capabilities` passes `Authority::resolve` to
`candidate_capabilities_with`, so a test can hand the real pass outcomes
that hold MCP. File 440 to 612 lines.

The MCP compile fence is untouched. `Authority::load` still refuses every
`mcp` grant and `carried` still answers a valid stdio dialect with the
unbuilt cause, so no compile holds MCP before U9b and every set contributes
itself on main.

## Tests

| Suite | Test | What it proves |
| --- | --- | --- |
| `agents/mcp/tests.rs` | `only_a_measured_carriage_carries_and_each_other_fact_is_its_own_reason` | a measured carriage with no shape carries; unsupported, declared-unmeasured (with measured shapes), legacy with a server map naming `cap-library-docs`, the bare `"unsupported"` and exec's inapplicable each refuse with their exact text |
| `bundle/agent_tests/mcp_tests.rs` | `each_candidate_contributes_exactly_its_own_holdings_and_never_a_fallbacks` | under `namespace` the carrying Claude primary contributes `Brokered([cap-issue-tracker, cap-library-docs])`, one holding `Brokered([cap-library-docs])`, none `Hands`; the legacy Codex fallback drops both wants with MB2's exact notices and not-held reasons, contributes `Hands`, keeps its native plan, and leaves the primary's outcome unchanged |
| same | `unsupported_carriage_refuses_requires_and_a_dead_server_map_gives_no_authority` | unsupported, declared-unmeasured and legacy-with-map carriage each refuse `requires` with its exact reason; the same `requires` at the carrying primary contributes `Brokered([cap-library-docs])` |
| same | `a_set_without_the_hands_server_or_a_model_channel_carries_no_broker` | the bare office refuses `NoHands` at both links for both strengths; exec's set and an opaque command refuse `NoModelChannel`; a candidate that composed nothing judges nothing; both texts pinned |
| same | `an_unrepresentable_server_name_refuses_and_is_never_shortened` | `library.docs` refuses `Unrepresentable { cap-library.docs, claude }` (text pinned) while the Codex fallback drops it first; `library_docs` contributes `cap-library_docs` |
| same | `only_an_mcp_holding_contributes_and_a_set_without_one_is_itself` | a native holding contributes nothing; with no MCP holding the hands, empty and exec sets contribute themselves |
| same | `the_pass_judges_each_resolved_candidate_and_refuses_in_the_sites_words` | through `candidate_capabilities_with` with the compile's resolver and outcomes holding `library-docs`: the primary keeps it, the fallback (carriage measured unsupported, `engine MCP loading is unavailable`) drops a want with its notice, and a `requires` refuses `bundle: seat 'research' (office 'researcher') in realm 'private': requires capability 'library-docs' through dialect 'library-docs-mcp', but provider 'codex' cannot carry MCP (engine MCP loading is unavailable)` |

No test moved. The owning suites `agents/tests.rs` (5,838), `capabilities/tests.rs`
(2,047) and `bundle/agent_tests.rs` (6,676) are unchanged; the tests extend
their existing child modules `agents/mcp/tests.rs` (1,054 to 1,109) and
`bundle/agent_tests/mcp_tests.rs` (1,097 to 1,511). The holdings are typed as
the resolver seals them, through the fixture builder `mcp_holding`, on
outcomes the real pass resolved from a compiled chain in a canonicalised
temporary root.

## Mutations

Each mutation compiled, was applied alone, made the named test fail, and was
restored; the suites passed again afterwards.

| Behaviour removed | Failed |
| --- | --- |
| `Brokered` collects no server (`take(0)`) | `each_candidate…`, `unsupported_carriage…`, `an_unrepresentable…` |
| the carriage check (`carries(..).or(Ok(()))`) | `each_candidate…`, `unsupported_carriage…`, `an_unrepresentable…` |
| a legacy carriage carries | `only_a_measured_carriage…`, `each_candidate…`, `unsupported_carriage…`, `an_unrepresentable…` |
| a `requires` ask dropped like a `wants` | `unsupported_carriage…` |
| a dropped holding kept in `held` | `each_candidate…`, `an_unrepresentable…` |
| the drop notice not pushed | `each_candidate…` |
| the not-held reason not recorded | `each_candidate…` |
| `.` admitted in a server name | `an_unrepresentable…` |
| `_` refused in a server name | `an_unrepresentable…` |
| the empty set given brokers | `a_set_without…` |
| exec's set given brokers | `a_set_without…` |
| no adapter passes as a model channel | `a_set_without…` |
| a candidate that composed nothing judged as the empty set | `a_set_without…` |
| the `contribute` call in `resolved` | `the_pass_judges…` |
| a native holding contributing a server | `only_an_mcp_holding…` |
| the empty set without holdings read as the hands | `only_an_mcp_holding…` |

The first attempt at the carriage mutation, `.and(Ok(()))`, left the
refusal in place and every test passed; it was no mutation, and `.or` was
used. Before the closure seam existed, removing the `contribute` call from
`resolved` failed no test in the runtime lib (982 passed), since no compile
can hold MCP before U9b; `the_pass_judges…` was added for it and then failed.

## Gates and measurements

On the final tree formatting and workspace clippy with warnings denied were
clean, and `typos --hidden`, `git diff --check` and `openspec validate --all
--strict` (26 passed) reported nothing. Every crate's suite passed, crate by
crate: brokkr-runtime (984 lib after the last test, `capability_launch` 76,
`it` 132 including `witness_digests`, `operated_repo`, `queued_launch`),
brokkr-cli (829 lib, `it` 664, `driver_conformance` 27, the three heap
binaries, `transcript_surfaces` 13), and protocol, core, store, view and
bridge with their integration binaries. `recipes/self` compiled. No witness
or compose pin moved, since `witness_digests` passed unblessed.
`quality/ratchet.sh` held `files`, `clones` and `api`; the public API is
unchanged, every new item being crate- or module-private.

`cargo llvm-cov -p brokkr-runtime --lib --branch` on the pinned
`nightly-2026-09-05` executed every line, function and branch of
`agents/mcp.rs` (232 lines, 34 branches) and `bundle/mcp.rs` (330 lines, 18
branches). cargo-crap over that LCOV measures `McpSite::contribute` at 9,
`McpIntent::contribution` at 7, `McpSupport::carries` at 5, `resolved` at 5
and `candidate_capabilities_with` at 1, all at 100% coverage; no listed long
function changed and no suppression was added.

`quality/file-lines.txt` rows: `agents.rs` 1,670, `agents/mcp.rs` 611,
`bundle/mcp.rs` 612, `agents/mcp/tests.rs` 1,109 and
`bundle/agent_tests/mcp_tests.rs` 1,511. `scripts/measure-budgets.sh`
rewrote the crate, heap and prompt budget files with today's date and prompt
sizes that drifted on main; this unit touches none of their inputs, so those
files were restored.

## Recorded for later rows

MB2's site eligibility is not this row's: its boundary-`namespace` and
network-false checks, and their place in D3 before GP1, belong in the
resolver (`capabilities.rs`). Here a set without the hands server or a model
channel refuses with MB2's own causes after resolution; a boxed set under
`seatbelt` or `container` still intends the hands server, and nothing holds
MCP there until the activation adds those checks. `McpIntent::contribution`
has no reader at dispatch yet: U7c seals the selected serving's brokers from
it. The bare-name grammar is written a second time beside the protocol's
private `bare` in `native_controls/mcp.rs`, outside this row (LOW, ruling
5). The requires and wants sentences repeat the resolver's skeleton in
`capabilities/gates.rs` and `Authority::resolve` in MB2's tail-less form
(LOW, ruling 5). Two holdings cannot collide here, since each server is
`cap-` and its distinct capability key; U7a's builder keeps its collision
refusal at launch.

## Pending

Not observed in this seat, and so still owed: the whole-workspace exact gate
and the complexity ratchet over its LCOV, which need a host that can create
namespaces; the macOS build and test jobs; and remote CI on the pull
request's final head.
