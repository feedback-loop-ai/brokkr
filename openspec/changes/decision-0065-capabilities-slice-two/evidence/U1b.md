# U1b: adapter MCP facts typed at load (tasks 3.1–3.2)

Run `0065-slice-two-unit-u1b-see-the--f7980aeb` built U1b on branch `s2/U1b`
from main at `ed67ed15`. It touched exactly the row's three production
files: `crates/brokkr-runtime/src/agents.rs`,
`crates/brokkr-runtime/src/agents/load.rs` and the new
`crates/brokkr-runtime/src/agents/mcp.rs`, a private child registered as
`mod mcp;` beside `load`. No adapter, scaffold, contract, policy, fixture or
reference file moved, and the MCP compile fence is untouched.

## What changed

The old `McpSupport { flag, servers }` struct and its decoder inside
`parse_adapter` are gone. `agents/mcp.rs` (461 lines) now holds the typed
facts and their loader:

| Item | What it is |
| --- | --- |
| `McpSupport` | The whole `mcp` declaration, a closed enum: `Legacy { flag }` for `"unsupported"` and `{flag, servers}`, `Inapplicable { reason }` for a harness with no model MCP surface, and `Declared { carriage, shapes }` for typed facts. |
| `McpShape` | One invocation shape: `McpInvocation` (`Cold`, `Replacement`, `Resume(name)`) under `McpHands` (`Boxed`, `Harness`, `NoHands`) on `McpHost` (`Linux`, `Macos`). |
| `McpMeasurement` | One declared shape's value: the harness `version` it was measured on and its `McpIsolation`. |
| `McpIsolation` | One shape's four axes, each its own value: `ambient`, `native_write`, `store_read`, `process_read`. |
| `McpAxis` | `Measured { evidence }`, `Unsupported { reason }`, `Unmeasured(McpUnmeasured)` or `Inapplicable { reason }`. |
| `McpUnmeasured` | Why nothing is known: `Absent`, `Legacy` or `Declared(reason)`. |
| `McpError`, `McpRefusal` | The `thiserror` refusals. `McpRefusal` renders `"{adapter} 'mcp' {problem}"` and reaches the caller as the new `LibraryError::Mcp`. |
| `McpSupport::carriage`, `McpSupport::isolation` | The reading: a shape no entry names reads `Unmeasured(Absent)` on every axis, and `Legacy` reads `Unmeasured(Legacy)` everywhere. Since the second visit both are private and `#[cfg(test)]` until a broker consumes them (review F3). |

The typed form an adapter may now write is
`{"carriage": <axis>, "shapes": [{"invocation", "hands", "measured_on": {"harness", "binary", "version", "host"}, "ambient", "native_write", "store_read", "process_read"}]}`,
where each axis is exactly one of `{"measured": …}`, `{"unsupported": …}` or
`{"unmeasured": …}`, and every text is a bounded non-empty line of at most 400
characters (the existing `RESUME_TEXT_LIMIT`, now `pub(super)` so it has one
home). The exec form is `{"inapplicable": …}`. All of these decode through
`serde` with `deny_unknown_fields` (decision 0071 ruling 3). The three forms
are chosen by the keys written, and mixing them is refused.

Loading happens in two phases. `parse_adapter` still checks `mcp` where it
always did, so the order in which refusals surface is unchanged. At that
point `McpSupport::decode` does the closed decoding, legacy server-name
grammar, the exec-only rule for `inapplicable` and duplicate shapes. Once
hands, the harness sandbox and `resume` have been read, `McpDecoded::admit`
checks each shape against them. A shape must have been measured on this
adapter's own `binary`, so a wrapper never borrows its child's result. It
must name hands the adapter declares: `boxed` needs `hands.workspace`, and
`harness` needs a `hands.harness` fragment. A resume shape must be one the
adapter's `resume` assessment names. These checks are the load-time consumers
of the new types. `parse_adapter` shrank from 198 to 195 lines.

## Assumptions

- **Invocation shapes.** The framing lists cold, resumed and replacement.
  Replacement is typed as its own invocation, distinct from cold, because the
  framing says each must not borrow another's result. Hands mode is part of
  the shape key because U0 measured hands and no-hands cells separately (C09b
  and C10).
- **Carriage.** Carriage is one adapter-level axis, separate from strictness
  (design D3: "the adapter's former dead mcp key becomes carriage data,
  separate from strictness").
- **Exec.** `Inapplicable` exists now because U1e records exec's surface
  without a production file. It is refused on every harness except exec.
- **The legacy flag.** It is kept only so the status page's existing cell
  still renders (`crates/brokkr-cli/tests/status_pages.rs`). Without it,
  `docs/status.md` would have to move, and that file is outside this row. The
  server map is checked and discarded. Nothing grants on either.
- **Version and host.** Superseded by the second visit below: both are
  now kept. Nothing compares them with the running harness or host until
  U1g, and the MCP fence still grants nothing.

## Tests

The new tests live in `crates/brokkr-runtime/src/agents/mcp/tests.rs`, a child
of the new module. This keeps the owning `agents/tests.rs` at its 5,838-line
baseline. Each test loads through the real `Adapters::load`, from a fresh
canonicalised temporary root per load. One compile-level test sits in the
owning `bundle/agent_tests/gate_tests.rs`. Tests that pinned the old shape
were updated to exact variants: `agents/tests.rs` (exec is
`Legacy { flag: None }`), `bundle/model_policy_tests.rs` (shipped dsh),
`tests/library_data.rs` (shipped exec) and `brokkr-cli/tests/status_pages.rs`
(the renderer). `capabilities/tests.rs` is above its ceiling at 2,053 lines and
needed no edit. Its fence test
`an_mcp_grant_refuses_until_slice_two_even_unused_and_a_hands_grant_is_reserved`
passes unchanged in the runtime lib run below.

| Test | What it proves |
| --- | --- |
| `each_shape_reads_its_own_axes_and_an_absent_shape_is_unmeasured` | Cold/boxed reads its four declared axes exactly. Cold with no hands, replacement and resume `work-site` each read `Unmeasured(Absent)` on every axis. Carriage reads its own value. |
| `write_confinement_and_ambient_exclusion_never_supply_a_secret_read_proof` | With ambient and read-only write confinement both measured, `store_read` stays `Unsupported` and `process_read` stays `Unmeasured(Declared)`. Omitting `store_read` is `Decode("missing field \`store_read\`")`, never an inferred value. |
| `legacy_maps_grant_nothing_and_only_exec_is_inapplicable` | A legacy map naming a server, and bare `"unsupported"`, read `Unmeasured(Legacy)` carriage and isolation. Exec's `inapplicable` reads `Inapplicable`. LaneTally writing `inapplicable` is `ModelHarness("lanetally")`. |
| `a_wrapper_hands_or_resume_shape_cannot_borrow_a_measurement` | LaneTally carrying Claude's measurement is `BorrowedHarness`. Boxed hands without `hands`, and harness hands without a fragment, are `UndeclaredHands`. An undeclared resume shape is `UndeclaredResume`. |
| `declared_harness_hands_and_resume_shapes_are_admitted` | A declared `hands.harness.work` fragment and a declared `resume` shape admit their entries under their exact keys. |
| `closed_decoding_refuses_each_malformed_fact_by_variant` | Unknown key, two mixed-form cases, a legacy server name, an unknown shape field, an unknown axis word, empty and 401-character text, and a duplicate shape each return their exact variant and text. |
| `each_refusal_reads_as_the_operator_sees_it` | The module's operator text, pinned once for every variant and every shape-display arm. |
| `typed_or_legacy_adapter_mcp_facts_grant_nothing_and_the_fence_holds` (gate_tests) | A seat asking nothing composes the same command under typed facts, a legacy map and `"unsupported"`. An asked grant under typed facts with every axis measured, or under a legacy map naming `cap-library-docs`, meets SC5's fence word for word. |

## Mutations

Each mutation compiled and was applied alone, except one pair run together
(noted below). The named test failed, and production was restored. The
restored files were compared byte for byte with saved copies (`cmp`), and the
suites below then passed. The "final" rows ran against the committed test
fixture. The rows marked "pre-reshape" ran on the same assertions before the
test fixture moved from a shared temporary root to a fresh root per load.
That change touched only the builder, not any assertion.

| # | Mutation (production) | Failing test, assertion | Tree |
| --- | --- | --- | --- |
| M1 | `isolation` falls back to `shapes.values().next()` | `each_shape_…` at `mcp/tests.rs:123`, `assert_eq!(observed, expected)` | final (run with M3: different code paths and different tests) |
| M2 | `store_read` takes `native_write`'s value | `write_confinement_…` at `:144`, the `McpIsolation` `assert_eq!` | final |
| M3 | `Legacy` carriage reads `Measured` | `legacy_maps_…` at `:192`, the observed-rows `assert_eq!` | final |
| M4 | binary check disabled (`false &&`) | `a_wrapper_…`, `problem` panics at `:27`: "expected an 'mcp' refusal, got Ok(… lanetally … Declared …)" | final |
| M5 | `Boxed => true` | `a_wrapper_…`, `problem` panic (hands-less adapter loaded) | two-phase production, pre-reshape |
| M6 | `Harness => true` | `a_wrapper_…`, `problem` panic | pre-reshape |
| M7 | resume check disabled | `a_wrapper_…`, `problem` panic | two-phase production, pre-reshape |
| M8 | a model harness may be `Inapplicable` | `legacy_maps_…`, `problem` panic | pre-reshape |
| M9 | duplicate-shape check disabled | `closed_decoding_…`, `problem` panic | pre-reshape |
| M10 | `(true, _, false)` reads mixed forms as legacy | `closed_decoding_…` at `:355`, the table `assert_eq!` | final |
| M11 | text length bound removed | `closed_decoding_…`, `problem` panic (401-character row loaded) | pre-reshape |
| M12 | legacy server-name grammar removed | `closed_decoding_…`, `problem` panic | pre-reshape |
| M13 | unknown-key check disabled | `closed_decoding_…` table, and the existing `the_adapter_loader_names_the_file_and_the_key_it_refuses` (`agents/tests.rs:2006`) | pre-reshape |
| M14 | shape display word `no` → `none` | `each_refusal_…` at `:410`, the text `assert_eq!` | final |
| M15 | `load.rs` passes `workspace: true` | `a_wrapper_…`, `problem` panic | two-phase production, pre-reshape |
| M16 | `Harness` admitted only by a gate fragment | `declared_harness_…`, `support`'s `unwrap` at `:19` | final |
| M17 | `agents.rs` `driver_template` appends `--mcp-config` under `Declared` | gate_tests at `:547`, row "typed asks nothing" | final |
| M18 | `McpDecoded::admit` skips every shape (`take(0)`) | `a_wrapper_…`, `problem` panic | two-phase production, pre-reshape |
| M19 | legacy flag dropped | `legacy_maps_…`, the observed-rows `assert_eq!` | two-phase production, pre-reshape |

## Gates observed in this session

Formatting is clean (`cargo fmt --all -- --check`). Clippy is clean across the
workspace with all targets and features, locked and warnings denied. The last
full `cargo test --workspace` gave 34 result lines, all ok and none failed.
The runtime lib passed 783 tests, `--test it` 120 (witness digests included,
so no witness pin moved), `capability_launch` 71, and brokkr-cli `--test it`
459 with 2 ignored. `cargo run --locked -p brokkr-cli -- compile --bundle
bundles/self` succeeded. `openspec validate --all --strict` passed 20 of 20.
`typos --hidden` and `git diff --check` were clean. `quality/ratchet.sh files`
reported "file size holds", `clones` "duplication holds" and `api` "public API
holds".

Measured listings: `quality/file-lines.txt` gives `agents.rs` 1,673 → 1,671,
`load.rs` 1,335 (held at its baseline), new `agents/mcp.rs` 461, new
`agents/mcp/tests.rs` 411, `gate_tests.rs` 476 → 548 and
`tests/library_data.rs` 793 → 795. In `quality/too-many-lines.txt`,
`parse_adapter` went 198 → 195 (now at `:770`) and `resume_assessment` moved
to `:1033`. `quality/public-api/brokkr-runtime.txt` was regenerated with
`cargo +nightly-2026-09-05 public-api -p brokkr-runtime -sss`. Its diff holds
only the MCP types and `LibraryError::Mcp`.
`scripts/measure-budgets.sh` measured 327 packages and unchanged heap peaks.
Its prompt-byte differences come from earlier changes on main (no prompt
path reads the adapter's `mcp`). Those files were restored unchanged.

## Second visit: the review return

The council's review of `3a4284f7` (result `residual`, medium) returned
findings F1 to F11. This visit answers them in the same three production
files (`agents.rs` changes only its re-exports; `load.rs` is untouched and
stays at its 1,335-line baseline).

| Finding | What changed |
| --- | --- |
| F1 (medium) version and host discarded | `measured_on` now reads `harness`, `binary`, `version` and a typed `host` (`linux` or `macos`, decision 0063). The version is kept in `McpMeasurement`. The host is part of `McpShape`, so a Linux and a macOS result for one invocation are two facts, and a host no entry names reads `Unmeasured(Absent)`. |
| F2 (medium) expectations built by the production helper | The tests build their all-axes expectation with their own `every`. The production helper is gone; `isolation` builds its own value inside the test-only readout. |
| F3 (low) readouts exported without a consumer | `carriage` and `isolation` are private and `#[cfg(test)]`. |
| F4 (low) resume hands not checked | A resume shape's MCP entry must name the same hands its `resume` assessment declares, or it is refused as `ResumeHands`. `McpHands::word` is the one place the module writes the three words: the shape display and this comparison both read it. Typing `ResumeShape::hands` itself as an enum would touch its other readers and is left as the low residual the review allowed. |
| F5 (low) a same-binary wrapper passed | `measured_on.harness` must equal the driver's harness (`capabilities::harness_of`), checked at decode. Otherwise the shape is refused as `BorrowedHarness`. The binary mismatch is now `BorrowedBinary`. |
| F6 (low) legacy values newly bounded | The legacy form is decoded by hand again. Any non-empty flag and server path loads whatever its length. Its refusals read as the old loader wrote them: `needs a non-empty string 'flag'`, `needs 'servers' as an object of strings`, `'servers' names …` and `'servers.<name>' must be a non-empty string`. |
| F7 (low) wildcard in the status-page test | `status_pages.rs` names `Legacy { flag: None }`, `Inapplicable` and `Declared` explicitly. The match moved into a helper, `mcp_flag`, so `render` stays within 100 lines. |
| F8 (info) control characters accepted | A measured text that holds a control character is refused, with the same bounded-line text. |
| F9 (info) resume name grammar | Not changed. A name that is not declared is already refused as `UndeclaredResume`, and every declared name obeys the name grammar. The echo of an undeclared name stays unbounded, as the loader's other name refusals are. |
| F10 (info) public-API baseline | Still pending. See below. |
| F11 (low) review-run integrity | Concerns the review run, not this code. Nothing to change here. |

New and changed tests in `agents/mcp/tests.rs`:
`the_measured_version_and_host_survive_loading` (F1) and
`legacy_values_keep_their_old_bounds_and_refusals` (F6) are new.
`each_shape_reads_its_own_axes_and_an_absent_shape_is_unmeasured` adds the
macOS row. `a_wrapper_hands_or_resume_shape_cannot_borrow_a_measurement`
adds the same-binary LaneTally row (F5) and the resume declared under
harness hands but measured boxed (F4).
`closed_decoding_refuses_each_malformed_fact_by_variant` adds a two-line
text (F8) and a `windows` host. `each_refusal_reads_as_the_operator_sees_it`
pins every new variant's text. The gate test's typed fixture gained
`harness` and `host`.

Each mutation below compiled and was applied alone to `agents/mcp.rs`, and
the named test failed. Production was then restored from a saved copy, and
`cmp` confirmed the bytes. With the restored file, all nine module tests
passed.

| # | Mutation (production) | Failing test, assertion |
| --- | --- | --- |
| N1 | the readout's all-axes value sets `store_read` to `Measured` | `each_shape_…` at `mcp/tests.rs:147` and `legacy_maps_…` at `:268`, the `assert_eq!` on the expected rows |
| N2 | `McpMeasurement::version` stored empty | `the_measured_version_and_host_…` at `:179`, the `Declared` `assert_eq!` |
| N3 | the shape's host forced to `Linux` | `the_measured_version_and_host_…`, `support`'s `unwrap` at `:19` (`DuplicateShape` on the cold, boxed, Linux shape) |
| N4 | the harness check disabled (`false &&`) | `a_wrapper_…`, `problem` panics at `:27` (the same-binary LaneTally adapter loaded) |
| N5 | the resume-hands guard disabled (`false &&`) | `a_wrapper_…`, `problem` panics at `:27` (the boxed measurement of a harness-hands resume loaded) |
| N6 | an empty legacy server path accepted | `legacy_values_…`, `problem` panics at `:27` |
| N7 | the 400-character bound put back on the legacy flag | `legacy_values_…`, `support`'s `unwrap` at `:19` (`LegacyFlag`) |
| N8 | the control-character check disabled | `closed_decoding_…`, `problem` panics at `:27` (the two-line row loaded) |

Gates on this visit. `cargo fmt --all -- --check` and workspace clippy
(all targets and features, locked, warnings denied) are clean. The
brokkr-runtime lib passed 785 tests, `--test it` 120 (witness digests
included, so no witness pin moved), `capability_launch` 71,
`operated_repo` 1 and `queued_launch` 3. brokkr-cli passed 627 lib tests
(1 ignored), `--test it` 459 (2 ignored), `driver_conformance` 27, each
heap test, and `transcript_surfaces` 13. The self bundle compiled.
`openspec validate --all --strict` passed 20 of 20. `typos --hidden` and
`git diff --check` reported nothing. `quality/ratchet.sh` reported "file
size holds", "duplication holds" and "public API holds". In
`quality/file-lines.txt`, `agents/mcp.rs` is 549 lines, its tests 579 and
`gate_tests.rs` 549. No function moved in `quality/too-many-lines.txt`, and
no suppression was added. `scripts/measure-budgets.sh` was not rerun on
this visit: nothing on a prompt or heap path changed.

## Pending

- **Public-API raise.** `quality/ratchet.sh baselines ed67ed15` refuses the
  brokkr-runtime raise from 1,467 to 1,537 items (1,523 on the first visit)
  until the pull request carries the operator's `Ruling:` line. The raise
  is the new public MCP types: `Adapter::mcp` is a public field read by the
  CLI status test.
- **External checks.** Exact coverage (`scripts/coverage-exact.sh`) on a
  capable host, remote CI on both operating systems, and macOS have not been
  run in this session. A local diagnostic is not the gate. It was
  `cargo llvm-cov -p brokkr-runtime --lib --lcov -- agents::`, and it
  reported `agents/mcp.rs` at LF 164 / LH 164 and FNF 24 / FNH 24.
