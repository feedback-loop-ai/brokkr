# U1e: dsh and exec declarations (tasks 6.1–6.2)

U1e was built in run `0065-slice-two-unit-u1e-see-the--a747c4ec` on branch
`s2/U1e`, cut from main at `f2b3a387`, where U1d and U1c2 had landed. Its
production change is the row's two files, `adapters/dsh.json` and
`adapters/exec.json`. No Rust production file, contract, policy, fixture,
reference or extension file moved, and the MCP compile fence is untouched:
the capabilities suite's
`an_mcp_grant_refuses_until_slice_two_even_unused_and_a_hands_grant_is_reserved`
passed inside the runtime lib run below.

## What is declared

dsh replaces the bare `"unsupported"` with the typed `carriage` and
`shapes` form U1b's loader decodes. Exec replaces it with the
`inapplicable` form, which the loader admits for the exec harness alone.
Every dsh line cites its U0 or U0c cell in
[slice-two-mcp-isolation.md](../../../../docs/evidence/adapters/slice-two-mcp-isolation.md).

| Adapter | Item | Declared | Cells |
| --- | --- | --- | --- |
| dsh | carriage | measured: the engine row in the `--patch` overlay over an engine-only `DSH_HOME` answered | D03; K02, K05, K08, K11t |
| dsh | cold, hands `none`, linux, 0.1.5-rc.1: ambient | measured on spark, spark-glm, deepseek-official, dashscope, meta and meta-contributor; no plugin bundle layer planted | D01–D04; K01–K12r |
| dsh | the same shape: native write | unmeasured: one write outside the workspace was denied, but the approval-gated escalation (policy `ask`) was not exercised | D04 |
| dsh | the same shape: store read | unsupported: dsh's native shell read the 0600 store canary | D04 |
| dsh | the same shape: process read | measured: a private pid namespace of 4 pids, no canary | D04 |
| dsh | `credentials` | adds `deepseek-official` → `DEEPSEEK_API_KEY` and `dashscope` → `DASHSCOPE_API_KEY`; the four existing entries are unchanged | U0c route table |
| dsh | `native_capabilities.unmeasured` | still unmeasured; now records that U0 and U0c listed native `web_fetch` and `web_search`, and that the `web-search-deepseek` row reads `DEEPSEEK_API_KEY` on every route family, with neither exercised | U0c route notes |
| exec | `mcp` | inapplicable: exec runs its bundle's script and serves no model | U0, exec |

Nothing else is declared. dsh's replacement, its `headless-work` resume,
boxed and harness hands, and macOS all read `Unmeasured(Absent)`. `hands`
stays unsupported, and the resume shape stays unmeasured. A route the
seat's model pin names outside the six qualified ones is still refused at
launch by U1c2's `DSH_ROUTES`, which this unit does not touch.

Three readings are assumptions, stated for review:

1. **The deepseek key is `deepseek-official`, not `deepseek`.** The
   framing names the family `deepseek`. U0c's route table records the
   route as provider `deepseek-official`. dsh's own `MISSING_CREDENTIAL`
   refusal names route `"deepseek-official"`, and U1c2's `DSH_ROUTES` and
   the protocol's `DSH_PROVIDER` use the same word. `brokkr doctor` prints
   the key as `route <key>`. So the measured route name is the key.
2. **Native write is `unmeasured`, not `measured`.** D04 saw one denial,
   but dsh offered an approval-gated escalation that was not exercised. A
   confinement that may be escalated is not shown to hold.
3. **The spark route is U0's.** U0's D03 and D04 ran the local Spark GLM
   model. The ambient line names `spark` beside `spark-glm`, as the
   controller's commission and U1c2's `DSH_ROUTES` do.

The native text was rewritten rather than extended. It reaches every dsh
seat's prompt, and appending the U0c observation grew 16 sites past their
budgets by 326 bytes. The rewrite keeps the phrase "does not establish that
dsh has no native egress", which the capabilities suite pins, and drops the
sentence calling the MCP declaration unsupported, which U1e makes untrue.
Those 16 sites are now 36 bytes smaller.

## Where the proof lives

None of the three owning suites was edited: each sits at or above the
2,000-line ceiling (agents 5,838, capabilities 2,052, bundle agents 6,788).
The proof sits beside U1d's, in `agents/mcp/tests.rs`, a child of the agents
suite, now 1,051 lines where it was 879. Each test reads this checkout's
`adapters/` directory with `Adapters::load` itself, and no existing test
changed files.

| Test | What it pins |
| --- | --- |
| `dsh_declares_only_its_own_cold_shape_and_inherits_nothing` (new) | dsh's carriage and its one shape exactly, with version and four axis variants. Every line starts with `U0 `, and the ambient line names both route groups. Five other shapes read absent: cold boxed, cold harness, replacement, `headless-work` resume and macOS. `hands` is `None` and the resume status is `Unmeasured`. Four edits to the shipped file refuse by variant: Claude's cold/none entry is `BorrowedHarness { measured: "claude", harness: "dsh" }`, the shape under `boxed` or `harness` is `UndeclaredHands`, and exec's `inapplicable` form is `ModelHarness("dsh")`. |
| `exec_has_no_model_surface_and_dsh_names_its_measured_keys` (new) | exec's `Inapplicable`, read as inapplicable for carriage and every axis. Exec stays a script path: the `exec` harness kind, no models and no model flag. dsh's six credentials exactly, and the web-search sentence in its native inventory. |
| `a_declaration_byte_moves_its_adapter_resolution_and_bundle_identity` (extended from three adapters to five) | One byte of dsh's carriage moves dsh's digest alone and a `flash` office's `adapter_digest` alone. It does not move `bundles/self`, which consults no dsh office. One byte of exec's reason moves exec's digest alone and does move `bundles/self`. No office resolves to exec. |

Fixture and validator swaps, each pinning behaviour this unit changes:

- **`tests/library_data.rs`**: dsh serves an unprefixed model id on
  `deepseek-official` (`parse_dsh_model` in brokkr-protocol). `reached_routes`
  now counts that route for an adapter whose driver harness is dsh, so #358's
  guard admits the new key and still refuses a misspelt one. Exec's
  assertion reads `Inapplicable`. The file grew from 795 to 801 lines.
- **`bundle/model_policy_tests.rs`**: the dsh hands-gap test's trailing
  `Legacy { flag: None }` assertion was removed. Its replacement formats to
  four lines in a file held at its 4,645 baseline, and the exact value is
  pinned above. The file is now 4,644 lines.
- **`brokkr-cli/tests/harness_probe.rs`**: the `custom` fixture clones
  `exec.json` under a custom driver, and the loader rightly refused the
  borrowed `inapplicable`. It now sets `mcp` to `"unsupported"`, as U1d did
  for the fake Claude. The comment was folded so the change is one line.
- **`brokkr-cli/src/doctor/capability_tests.rs`**: the dsh readout line now
  expects the new native text's opening. The file stays at 1,978 lines.
- **`docs/status.md`**: the dsh native-inventory line matches the
  rendering, as `the_status_matrix_is_the_adapter_data` requires.
- **`crates/brokkr-runtime/tests/witnesses.json`**: 18 bundle pins were
  re-measured, because exec or dsh is consulted by every shipped bundle. The
  seat cannot run an environment prefix, so the digests were copied from the
  unblessed run's moved-witness table. A second pass followed the
  native-text rewrite (6 dsh bundles). After it,
  `cargo test -p brokkr-runtime --test it` passed 120 of 120.
- **`quality/`**: `file-lines.txt` records the three listed test files.
  `too-many-lines.txt` moves three `model_policy_tests.rs` entries up one
  line, from 3167, 3358 and 3856 to 3166, 3357 and 3855; no length changed.
  `prompt-bytes.json` lowers the 16 dsh sites by 36 bytes each.
  `scripts/measure-budgets.sh` also lowered 52 Claude and Codex sites by 13
  bytes. Measuring with the base adapters restored showed the same 13 bytes
  (`bundles/self/review` at 13,950 against a budget of 13,963), so that is
  base drift. It was left alone, as were the crate-count and heap files,
  whose numbers did not move.

## Mutations

Each mutation compiled and failed the named test at the named assertion.
Each was then restored, and the restored test passed. M1–M3, M6, M8 and M9
edit adapter data. M4, M5, M7 and M10 edit Rust this unit does not change,
and M11 edits the swapped validator. The command was
`cargo test -p brokkr-runtime --lib agents::mcp::tests::`, or
`--test it library_data::` for M11, on the working tree over `f2b3a387`.

| # | Mutation | Failing test, assertion |
| --- | --- | --- |
| M1 | dsh native write `unmeasured` → `measured` (fabricated confinement) | `dsh_declares_only_…`, the declaration `assert_eq!` (`[m, m, u, m]` against `[m, d, u, m]`) |
| M2 | dsh adds a `headless-work` resume shape, all measured (fabricated resume) | the same `assert_eq!`, with a second shape on the left |
| M3 | ambient line drops `dashscope` | the same test, the route `assert_eq!` (the keyed group unnamed) |
| M4 | `facts`: a `claude` harness is accepted under any driver (inheritance) | the same test, the refusal `assert_eq!` (`DuplicateShape` where `BorrowedHarness` was expected) |
| M5 | `inapplicable`: the dsh harness is admitted as inapplicable | the same test: `problem` panics "expected an 'mcp' refusal, got Ok(…)" |
| M6 | exec `mcp` back to `"unsupported"` | `exec_has_no_model_surface_…` panics "exec declares no model MCP surface"; the identity test's `assert_ne!` fails ("exec declares its mcp text") |
| M7 | `isolation`: an inapplicable declaration reads `Unmeasured(Absent)` | `exec_has_no_model_surface_…`, the carriage and isolation `assert_eq!` (also U1b's legacy test) |
| M8 | dsh `credentials` drops `deepseek-official` | `exec_has_no_model_surface_…`, the keys `assert_eq!` |
| M9 | dsh native text drops the web-search clause | the same test, the native `assert!` |
| M10 | `load.rs`: dsh's and exec's digest hash the parsed object without `mcp` | `a_declaration_byte_moves_…`, the moved table (dsh and exec rows all `false`) |
| M11 | `library_data.rs`: the dsh arm of `reached_routes` disabled | `every_model_resolves_to_a_route_with_a_declared_egress`: "dsh names route 'deepseek-official', which no model it maps reaches" |

After the restorations, `git diff` over `agents/mcp.rs` and `agents/load.rs`
was empty, and the 14 `agents::mcp::tests` passed.

## Gates

Every gate below was observed on the committed tree, whose parent is
`f2b3a387`. One `cargo fmt` pass reflowed two of the new test lines, and the
format check was then silent. Clippy over all targets and features denied
no warning.

| Crate or check | Observed |
| --- | --- |
| brokkr-runtime | lib 836, `it` 120, `capability_launch` 75, `operated_repo` 1, `queued_launch` 3 — all passing |
| brokkr-cli | lib 627 (1 ignored), `it` 501 (2 ignored), `driver_conformance` 27, `transcript_surfaces` 13, three heap binaries 1 each |
| protocol, core, view, store, bridge, seatbelt-probe | every target passing |
| bundle compiles | `self` and `verify` produced manifests |
| quality ratchet | `files` and `clones` held after this file's wording was made its own |
| OpenSpec, typos, whitespace | 20 of 20 strict; no typo; no whitespace error |

Still pending, because no observation exists yet: exact coverage (it needs a
host that can nest namespaces), remote CI on Linux and macOS, and any
measurement of dsh on macOS, for which nothing is declared.

## Follow-ups (not fixed here)

- LOW (ruling 5): `library_data.rs` writes `deepseek-official` a second time
  beside the protocol's private `DSH_PROVIDER`. A shared accessor would need
  a brokkr-protocol production file, which is outside this row.
- `init.rs`'s generated dsh scaffold still writes `"mcp": "unsupported"` and
  the old native text, and its exec scaffold writes the legacy form. U1f2
  owns that parity.
- The dsh native inventory, including `web-search-deepseek`'s key use, stays
  unmeasured. No ON or OFF control is declared.
