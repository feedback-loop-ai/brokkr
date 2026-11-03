# U4d: attribution bound to compiled holdings (tasks 16.1–16.2)

Run `0065-slice-two-unit-u4d-see-the--0afc7bc6` built U4d on branch `s2/U4d`
from main at `d9e786af`. It touched two of the row's three production files:
`crates/brokkr-runtime/src/capabilities.rs` and the new
`crates/brokkr-runtime/src/capabilities/attribution.rs`, a private child
registered as `mod attribution;` beside `binding`, `dialect`, `gates` and
`manifest`. `crates/brokkr-runtime/src/bundle.rs` needed no edit:
`bundle::site_capabilities` already calls `Authority::resolve` once per
candidate (each chain link, or an inline site's one driver), and every panel
member and sequence step is its own site there. The check therefore runs for
every candidate of every site shape, on that candidate's own holdings.

## What changed

`attribution.rs` (132 lines) holds what one candidate's selected native
holdings project onto its adapter inventory:

| Item | What it is |
| --- | --- |
| `expected` | The sealed `NativeExpectation` of each known power, held where the realm's holding reaches its adapter key and denied otherwise. It was moved out of `Authority::native_plan` unchanged. |
| `Attribution` | The reverse index: a `BTreeMap` from each held tool's exact name to its `Attributed { capability, dialect }`. `Attribution::of` looks a tool up by equality and nothing else. |
| `Refusal` | A `thiserror` enum. `Ambiguous { tool, provider }` renders CC1's cause. `Unrepresentable { capability, dialect }` renders SC4's cause (the `Display` of `binding::Unserved::Identity`, so it has one home), followed by the capability and dialect where the name was held. The tool is not echoed. |
| `Authority::attribution` | Builds the index from `provider`'s final `held` map. |

`Authority::attribution` walks each held capability. An `mcp` implementation
is skipped, because CC1 tells an MCP tool apart by its `cap-` server, and the
match on `Implementation` has no wildcard arm (decision 0071 ruling 2). Each
native holding is first judged by `binding::representable`, the existing SC4
check that `mcp` holdings already pass through, so the bounds and vocabulary
have one home: capability and dialect at most 128 bytes, and every tool at
most 256 ASCII bytes in v5's tool vocabulary. Each of the holding's tools,
narrowed by the grant, is then inserted. A tool already present from another
holding is `Ambiguous`, and neither holding is chosen. Nothing in the index
comes from the inventory: `NativeInventory::capability_of` still identifies
known tools for `agents::native_alias` and stays where it was.

`Authority::resolve` consumes the index as a compile check. Once the wanted
drops are settled, it builds the index from what the candidate finally holds
and refuses with `"{site}: {refusal}"`. Either refusal holds whatever the
ask's strength: a wanted capability is refused, never dropped. In a bundle
compile, the line goes through `CompileError::Capability`'s existing bounded
sink. M8 below observed it in full:
`bundle: seat 'fallback' (office 'chain') in realm 'private': tool 'WebSearch' maps to more than one held capability for provider 'claude'`.
The index is not stored in `Outcome`, and no lookup was added beyond the one
the builder uses. No checkpoint key, observation or emission changed (D9).

`capabilities.rs` went from 2,012 to 1,995 lines, below its baseline, and
`native_plan` from 256 to 240 lines. Its `too_many_lines` expectation stays
because the function is still over 100 lines. The `HeldPower` import moved
with `expected`.

### Behaviour that changed on purpose

Until now, a native holding was judged as slice one judged it, so a native
dialect named with 129 bytes was held. SC4 now refuses it at compile.
`capabilities/tests/dialect_policy.rs`'s
`an_mcp_identity_is_carried_whole_or_refused_and_a_native_one_is_unchanged`
pinned the old answer. It is renamed
`an_mcp_or_native_identity_is_carried_whole_or_refused`, and its native
assertion now expects the exact SC4 refusal with the dialect context. Its
`representable` table and its `mcp` half are unchanged, and the file went
from 1,024 to 1,023 lines. The doc comment on `binding::representable`
(binding.rs, outside this row) still says a native holding is judged as slice
one judged it. That is recorded as a follow-up, not edited.

## Test moves

`a_held_capability_is_switched_on_and_is_fully_attributable` moved, verbatim,
from `capabilities/tests.rs` into the new child module
`capabilities/tests/attribution.rs`. That made room for `mod attribution;`,
and `tests.rs` went from 2,103 to 2,053 lines. The test's assertions did not
change, so coverage did not either.

## New tests and their removal controls

The four new tests live in `capabilities/tests/attribution.rs` (281 lines).
They use the suite's existing builders (`cq1_root`, `authority`, `serving`,
`asks`, `define`, `dialect`, `native_dialect`) and its pinned `WHO`. A local
builder, `knowing`, declares `test-native` powers that are switched by argv.

| Test | What it pins |
| --- | --- |
| `a_native_tool_is_attributed_by_its_exact_name_to_the_selected_holding_alone` | Under a grant narrowed to `lookup`, `of("lookup")` is exactly `web-search` through `search-native`. `search` is known to the inventory (`capability_of` says `web-search`) but `of("search")` is `None`. `look`, `ookup`, `Lookup`, `lookup2`, `lookup ` and the empty name are `None`. A candidate on another provider that lost the want gets an empty index. |
| `a_tool_two_held_capabilities_claim_refuses_the_compile_and_neither_is_chosen` | Two held capabilities sharing `lookup` refuse with the exact CC1 line under the site prefix, for `requires` and for `wants`. Held alone, each attributes `lookup` to itself. The typed result over both holdings is exactly `Err(Refusal::Ambiguous { tool: "lookup", provider: "test-native" })`. |
| `a_held_name_v6_cannot_carry_whole_refuses_and_a_long_valid_one_stays_distinct` | Two 80-character-prefix names and a 256-byte name are each attributed whole, and the bare prefix is `None`. A 128-byte dialect name compiles. A 257-byte tool, `web search`, `Bash(git:*)`, `.hidden` and `wéb` each refuse with the exact SC4 line, for `requires` and for `wants`. So do a 129-byte dialect and a 129-byte capability. |
| `an_mcp_holding_never_enters_the_native_index` | A holding whose implementation is `Mcp` yields exactly `Ok(Attribution::default())`. It is the only reach of that arm under the U9b fence. |

Each mutation below was applied to the production code and run against the
named suite, and each failed the assertion named. Each was then restored.
Line numbers refer to `capabilities/tests/attribution.rs` unless another
file is named. The suites were run with
`cargo test -p brokkr-runtime --lib capabilities::tests::attribution` (or
`capabilities::tests::` for M5 and M9) and, for M8,
`cargo test -p brokkr-runtime --test capability_launch legacy_journal::`.

| # | Mutation | Failed assertion |
| --- | --- | --- |
| M1 | Index the dialect's whole tool list instead of the holding's narrowed tools | line 119, `index.of("search")` was `Some(web-search/search-native)`, expected `None` |
| M2 | `Attribution::of` matches any key that `contains` the tool | line 121, `of("look")` was `Some`; line 223, the 80-character prefix was `Some` |
| M3 | Stamp the capability name as the dialect | line 114, line 172 and line 218: `dialect: "web-search"` against `search-native` / `search-long` |
| M4 | Skip the ambiguity check (`if false && …`) | line 159, `unwrap_err()` on an `Ok` outcome holding both capabilities |
| M5 | Replace `representable` with `Ok(())` | line 238, `unwrap_err()` on an `Ok` index of the 257-byte tool; `dialect_policy.rs` line 914, `Ok(Outcome …)` against the SC4 `Err` |
| M6 | Insert each tool truncated to 80 characters | line 218, `of(first)` was `None`, expected `Some(web-search/search-long)` |
| M7 | Let an `Mcp` holding fall through into the index | line 277, the index held `lookup` and `search`, expected empty |
| M8 | Refuse every native tool (`if true || …`) | D9 matrix `every_site_shape_journals_its_legacy_native_rows_through_export_and_verify`, `legacy_journal.rs` line 304: "the legacy matrix compiles" failed with the CC1 line quoted above |
| M9 | Drop `resolve`'s refusal (`let _ = …`) | line 159 (ambiguity compiled), line 214 (the helper's `attribution(…).unwrap()` met the unrefused SC4 case) and `dialect_policy.rs` line 914 |

M8 is this unit's legacy boundary control. The D9 matrix compiles with
`web-search` granted through `claude-native-search` at every site shape, so a
compile check that refused valid native holdings would break native
compile-to-journal. A dropped want, a fallback, a panel member or a sequence
step that held nothing would yield an empty index. M1 and M3 bind the
selected holding's own tools and dialect.

M1 to M9 were run while `NativeInventory::capability_of` sat temporarily in
`attribution.rs`. It was moved back to `capabilities.rs` because a second
`impl NativeInventory` block changed the public-API listing (the API ratchet
reported the extra `impl` line). The mutated code (`Authority::attribution`
and `Attribution::of`) and the test file are the same bytes in both states.
After the move back, the suites below were rerun on the final tree.

## Gates on the final tree

All results below are from the final tree, base `d9e786af` plus this unit's
changes.

`cargo fmt --all -- --check` was clean, and so was workspace clippy with
`--all-targets --all-features --locked -D warnings`. brokkr-runtime passed
its lib (775), `it` (120, which includes `witness_digests` without
`BROKKR_BLESS`), `capability_launch` (71, including the D9 matrix's three
tests), `operated_repo` (1) and `queued_launch` (3) suites. brokkr-cli passed
its lib (627, 1 ignored), `it` (459, 2 ignored), `driver_conformance`,
`transcript_surfaces` and the three heap binaries. brokkr-protocol passed its
lib (642, 2 ignored) and its other targets. brokkr-core, brokkr-store,
brokkr-view and brokkr-bridge reported 14 passing result lines and no
failure.

`bundles/self` and `bundles/verify` compiled. The files, clones and API
ratchets held. `quality/file-lines.txt` records the three changed counts
and the two new files. `quality/too-many-lines.txt` records `native_plan` at 240
lines and line 1547, and the shifted line of the one long test in
`tests.rs`. No suppression changed, and no witness or compose pin moved. The
budget measurement (`scripts/measure-budgets.sh`) was not run, because no
budgeted binary or prompt input changed.

`openspec validate --all --strict` reported 20 passed and 0 failed.
`typos --hidden` and `git diff --check` printed nothing. Exact coverage (`scripts/coverage-exact.sh`), the baselines ratchet
on the PR body, and remote CI on both operating systems are pending outside
this seat.
