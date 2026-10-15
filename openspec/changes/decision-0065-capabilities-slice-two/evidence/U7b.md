# U7b evidence: consume adapter carriage and selected holdings

U7b is delivered by run `0065-slice-two-unit-u7b-see-the--82c24f5f` on branch
`s2/U7b`, cut from main at `c77ccf26`, as signed commits: the unit, then
its answer to the chief review's findings M1, L1 and L4. It closes tasks
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
ruling 2): `candidate_capabilities` passes the module's `resolve` to
`candidate_capabilities_with`, so a test can hand the real pass outcomes
that hold MCP. The seam is typed (ruling 8): `Resolve` returns
`Result<Outcome, CompileError>`, and `resolve` converts
`Authority::resolve`'s legacy `String` refusal to `CompileError::Capability`
at that one boundary, so no new `String` error crosses the seam and the
candidate and inline paths share it. File 440 to 622 lines.

The MCP compile fence is untouched. `Authority::load` still refuses every
`mcp` grant and `carried` still answers a valid stdio dialect with the
unbuilt cause, so no compile holds MCP before U9b and every set contributes
itself on main.

## Tests

| Suite | Test | What it proves |
| --- | --- | --- |
| `agents/mcp/tests.rs` | `only_a_measured_carriage_carries_and_each_other_fact_is_its_own_reason` | a measured carriage with no shape carries; unsupported, declared-unmeasured (with measured shapes), legacy with a server map naming `cap-library-docs`, the bare `"unsupported"` and exec's inapplicable each refuse as the typed `Uncarried { provider, reason }` with its exact reason; `Uncarried`'s text is pinned once |
| `bundle/agent_tests/mcp_tests.rs` | `each_candidate_contributes_exactly_its_own_holdings_and_never_a_fallbacks` | under `namespace` the carrying Claude primary contributes `Brokered([cap-issue-tracker, cap-library-docs])`, one holding `Brokered([cap-library-docs])`, none `Hands`; the legacy Codex fallback drops both wants with MB2's exact notices and not-held reasons, contributes `Hands`, keeps its native plan, and leaves the primary's outcome unchanged |
| same | `unsupported_carriage_refuses_requires_and_a_dead_server_map_gives_no_authority` | unsupported, declared-unmeasured and legacy-with-map carriage each refuse `requires` as the typed `Unservable::Uncarried { capability, dialect, cause: Uncarried { provider, reason } }`, every field exact; that variant's text is pinned once; the same `requires` at the carrying primary contributes `Brokered([cap-library-docs])` |
| same | `a_set_without_the_hands_server_or_a_model_channel_carries_no_broker` | the bare office refuses `NoHands` at both links for both strengths; exec's set and an opaque command refuse `NoModelChannel`; a candidate that composed nothing judges nothing; both texts pinned |
| same | `an_unrepresentable_server_name_refuses_and_is_never_shortened` | `library.docs` refuses `Unrepresentable { cap-library.docs, claude }` (text pinned) while the Codex fallback drops it first; `library_docs` contributes `cap-library_docs` |
| same | `only_an_mcp_holding_contributes_and_a_set_without_one_is_itself` | a native holding contributes nothing; with no MCP holding the hands, empty and exec sets contribute themselves |
| same | `the_pass_judges_each_resolved_candidate_and_refuses_in_the_sites_words` | through `candidate_capabilities_with` with the compile's typed `resolve` and outcomes holding `library-docs`: the primary keeps it, the fallback (carriage measured unsupported, `engine MCP loading is unavailable`) drops a want with its notice, and a `requires` refuses as `CompileError::Capability` carrying exactly `seat 'research' (office 'researcher') in realm 'private': requires capability 'library-docs' through dialect 'library-docs-mcp', but provider 'codex' cannot carry MCP (engine MCP loading is unavailable)`, matched by variant, not by its rendered line |

No test moved. The owning suites `agents/tests.rs` (5,838), `capabilities/tests.rs`
(2,047) and `bundle/agent_tests.rs` (6,676) are unchanged; the tests extend
their existing child modules `agents/mcp/tests.rs` (1,054 to 1,109) and
`bundle/agent_tests/mcp_tests.rs` (1,097 to 1,522). The holdings are typed as
the resolver seals them, through the fixture builder `mcp_holding`, on
outcomes the real pass resolved from a compiled chain in a canonicalised
temporary root.

## Mutations

The returned visit re-ran every mutation and recorded what it observed, so
each row below is one observation and not a summary. Every mutation was
applied by hand, alone, to the committed source tree
`crates/brokkr-runtime/src` = `e60b0a42` (commit `5ac965a4`, the review
fixes); the digest column is the first 16 hex digits of `git diff |
sha256sum` over the mutated tree. Each then ran
`timeout 590 cargo test -p brokkr-runtime --lib --locked mcp` (48 tests;
m17 ran the whole lib, 984), compiled, and failed exactly the tests listed,
at the `assert_eq!` line given, with the left (observed) value shown against
the asserted one. Each was restored with `git checkout -- crates/`; the
restored tree's lib then passed 984 of 984. Test files are
`agents/mcp/tests.rs` (A) and `bundle/agent_tests/mcp_tests.rs` (B), test
names shortened to their first words; full names are in the table above.

| Id | Mutation (production edit) | Digest | Failed test (file:line) | Observed instead of asserted |
| --- | --- | --- | --- | --- |
| m01 | `Brokered`: `servers.map(..).take(0).collect()` | `0b76a9b4007b3701` | `each_candidate` B:1217; `unsupported_carriage` B:1309; `an_unrepresentable` B:1404; `only_an_mcp_holding` B:1453 | `Brokered([])` for `Brokered(["cap-issue-tracker", "cap-library-docs"])` and `Brokered(["cap-library-docs"])`; `[Ok(()), Ok(())]` for `[Err(Unrepresentable { cap-library.docs, claude }), Ok(())]`. 44 passed, 4 failed |
| m02 | carriage check bypassed: `carries(..).or(Ok::<(), Uncarried>(()))` | `7acebbca49777deb` | `each_candidate` B:1217; `unsupported_carriage` B:1297; `an_unrepresentable` B:1404; `the_pass_judges` B:1501 | the legacy fallback `Brokered([both])` for `Hands`; `[Ok(()), Ok(()), Ok(())]` for three `Err(Uncarried {..})`; the fallback holding `library-docs` with no notice. 44 passed, 4 failed |
| m03 | `carries`: `Unmeasured(Legacy) => return Ok(())` | `b9b191672dcb62e7` | `only_a_measured_carriage` A:1094; `each_candidate` B:1217; `unsupported_carriage` B:1297; `an_unrepresentable` B:1404 | rows 4 and 5 `Ok(())` for `Err(Uncarried { test-provider, "its adapter declares no measured carriage" })`; the legacy-with-map refusal `Ok(())`. 44 passed, 4 failed |
| m04 | `requires` dropped like `wants`: `Some(Strength::Requires) if false`, `Requires` joined to the drop arm | `305e0e5423ebfb83` | `unsupported_carriage` B:1297; `the_pass_judges` B:1513 | `[Ok(()), Ok(()), Ok(())]` for three typed `Uncarried` refusals; the pass returned no `CompileError::Capability` and reached the `let … else` panic. 46 passed, 2 failed |
| m05 | dropped holding kept: `outcome.held.remove(&capability);` deleted | `8939dfdd00e56e19` | `each_candidate` B:1217; `an_unrepresentable` B:1404; `the_pass_judges` B:1501 | the fallback still `Brokered([both])`; `Err(Unrepresentable { cap-library.docs, codex })` for `Ok(())`; `(["library-docs"], [notice])` for `([], [notice])`. 45 passed, 3 failed |
| m06 | notice not pushed: `drop((capability, notice))` | `de5486cb2c575862` | `each_candidate` B:1238; `the_pass_judges` B:1501 | `([], [])` for the two MB2 drop notices; `([], [])` for `([], [notice])`. 46 passed, 2 failed |
| m07 | not-held reason not recorded: `.not_held.remove(&cause.to_string())` | `ea52148be32fdc2c` | `each_candidate` B:1248 | `{"web-search": ..}` alone for it plus `issue-tracker` and `library-docs` each `provider 'codex' cannot carry MCP (its adapter declares no measured carriage)`. 47 passed, 1 failed |
| m08 | `representable` admits `.` | `8968dcdc960b6fae` | `an_unrepresentable` B:1404 | `[Ok(()), Ok(())]` for `[Err(Unrepresentable { cap-library.docs, claude }), Ok(())]`. 47 passed, 1 failed |
| m09 | `representable` refuses `_` | `4db8312b15dd0397` | `an_unrepresentable` B:1412 | `Err(Unrepresentable { cap-library_docs, claude })` for `Ok(())`. 47 passed, 1 failed |
| m10 | `(Empty, Some(_))` contributes `Brokered` | `ac8404d10e99440b` | `a_set_without` B:1339 | `[[Ok(()), Err(Uncarried {..})], [Ok(()), Ok(())]]` for four `Err(NoHands("library-docs"))`. 47 passed, 1 failed |
| m11 | `(NoModelSurface, Some(_))` contributes `Brokered` | `5d9d1b69534c0c1b` | `a_set_without` B:1365 | exec's `(Ok(()), false)` for `(Err(NoModelChannel("library-docs")), true)`. 47 passed, 1 failed |
| m12 | no adapter: `return Ok(())` for `NoModelChannel` | `761a5d435263151e` | `a_set_without` B:1365 | the opaque command's `(Ok(()), true)` for `(Err(NoModelChannel("library-docs")), true)`. 47 passed, 1 failed |
| m13 | no intent judged as `self.intent.unwrap_or(McpIntent::Empty)` | `79445abbb4555827` | `a_set_without` B:1365 | `(Err(NoHands("library-docs")), true)` for `(Ok(()), true)`. 47 passed, 1 failed |
| m14 | `contribute` call in `resolved` replaced by `let _ = &mut outcome;` | `fe2bbcc5f206dc2a` | `the_pass_judges` B:1501 | the fallback `(["library-docs"], [])` for `([], [notice])`. 47 passed, 1 failed |
| m15 | `Implementation::Native` contributes `"cap-native"` | `813e0d5b14bb97b5` | `only_an_mcp_holding` B:1449 | `[Ok(Brokered(["cap-native"])), Err(NoHands("web-search")), Err(NoModelChannel("web-search"))]` for `[Ok(Hands), Ok(Empty), Ok(NoModelSurface)]`. 47 passed, 1 failed |
| m16 | `(Empty, None)` contributes `Hands` | `ef5e3361f2ae7b28` | `only_an_mcp_holding` B:1449 | `[Ok(Hands), Ok(Hands), Ok(NoModelSurface)]` for `[Ok(Hands), Ok(Empty), Ok(NoModelSurface)]`. 47 passed, 1 failed |
| m17 | the seam types a resolver refusal `CompileError::Invalid` | `90766afe4d3c2dfd` | `authored_bytes_equal_to_an_engine_fragment_stay_authored_and_refused` B:466 | `"not a capability refusal: bundle: seat 'review' .. its arguments carry '--strict-mcp-config' .."` (and the Codex `--config` row) for the capability refusal. 983 passed, 1 failed |
| m18 | `Unservable::Uncarried`'s `dialect` filled with the capability | `25778203635c8389` | `unsupported_carriage` B:1297; `the_pass_judges` B:1515 | `dialect: "library-docs"` for `dialect: "library-docs-mcp"`, in the typed refusal and in the pass's refusal. 46 passed, 2 failed |
| m19 | `Uncarried`'s `provider` emptied in `carries` | `2f2805ed98837bc2` | `only_a_measured_carriage` A:1094; `each_candidate` B:1238; `unsupported_carriage` B:1297; `the_pass_judges` B:1501 | `provider: ""` for `provider: "test-provider"` and `"codex"`, in the typed refusals and the notices. 44 passed, 4 failed |

Three attempts were not mutations and are recorded as such. The first
visit's carriage edit `.and(Ok(()))` left the refusal in place and every test
passed. This visit's m02 first read `.or(Ok(()))`, which did not compile
(E0282, type annotations needed), and m04 first guarded the `Requires` arm
alone, which did not compile (E0004, `Some(&Strength::Requires)` not
covered); each was restored and re-applied in the compiling form above.
Before the closure seam existed, removing the `contribute` call from
`resolved` failed no test in the runtime lib (982 passed), since no compile
can hold MCP before U9b; `the_pass_judges…` was added for it, and m14 shows
it binding.

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

The returned visit (review findings M1, L1 and L4) changed `bundle/mcp.rs`
and the two test modules only, then repeated the gates on its final tree.
Formatting and workspace clippy were clean; the runtime crate passed
(lib 984, `capability_launch` 76, `it` 132 with `witness_digests`
unblessed, `operated_repo` 1, `queued_launch` 6); `recipes/self` compiled;
`openspec validate --all --strict` passed 26; `typos --hidden` and
`git diff --check c77ccf26` were clean; and `quality/ratchet.sh` held
`files` and `clones`. No other crate's source moved, every change being
module-private. Re-measured with `cargo +nightly-2026-09-05 llvm-cov -p
brokkr-runtime --lib --branch`, `agents/mcp.rs` executed 232 of 232 lines,
34 of 34 branches and 28 of 28 functions, and `bundle/mcp.rs` 337 of 337
lines, 18 of 18 branches and 45 of 45 functions, the new `resolve`
included.

`quality/file-lines.txt` rows: `agents.rs` 1,670, `agents/mcp.rs` 611,
`bundle/mcp.rs` 622, `agents/mcp/tests.rs` 1,109 and
`bundle/agent_tests/mcp_tests.rs` 1,522. `scripts/measure-budgets.sh`
rewrote the crate, heap and prompt budget files with today's date and prompt
sizes that drifted on main; this unit touches none of their inputs, so those
files were restored.

## Recorded for later rows

MB2's site eligibility is not this row's: its boundary-`namespace` and
network-false checks, and their place in D3 before GP1, belong in the
resolver (`capabilities.rs`). Here a set without the hands server or a model
channel refuses with MB2's own causes after resolution, so a wanted MCP ask
that GP1 or compatibility drops inside `Authority::resolve` never reaches
them (review L5). That ordering is dormant while the realm-wide MCP refusal
stands, and U9b1's activation must place these checks before GP1; a boxed set under
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
