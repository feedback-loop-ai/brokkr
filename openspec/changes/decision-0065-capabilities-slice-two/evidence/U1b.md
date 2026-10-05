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
| `McpShape` | One invocation shape: `McpInvocation` (`Cold`, `Replacement`, `Resume(name)`) under `McpHands` (`Boxed`, `Harness`, `NoHands`). |
| `McpIsolation` | One shape's four axes, each its own value: `ambient`, `native_write`, `store_read`, `process_read`. |
| `McpAxis` | `Measured { evidence }`, `Unsupported { reason }`, `Unmeasured(McpUnmeasured)` or `Inapplicable { reason }`. |
| `McpUnmeasured` | Why nothing is known: `Absent`, `Legacy` or `Declared(reason)`. |
| `McpError`, `McpRefusal` | The `thiserror` refusals. `McpRefusal` renders `"{adapter} 'mcp' {problem}"` and reaches the caller as the new `LibraryError::Mcp`. |
| `McpSupport::carriage`, `McpSupport::isolation` | The reading: a shape no entry names reads `Unmeasured(Absent)` on every axis, and `Legacy` reads `Unmeasured(Legacy)` everywhere. |

The typed form an adapter may now write is
`{"carriage": <axis>, "shapes": [{"invocation", "hands", "measured_on": {"binary", "version"}, "ambient", "native_write", "store_read", "process_read"}]}`,
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
- **Version.** `measured_on.version` is required and bounded (SI1: every
  result names the harness binary and version), but it is not retained.
  Nothing compares it until U1g.

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

## Pending

- **Public-API raise.** `quality/ratchet.sh baselines ed67ed15` refuses the
  brokkr-runtime raise from 1,467 to 1,523 items until the pull request
  carries the operator's `Ruling:` line. The raise is the new public MCP
  types: `Adapter::mcp` is a public field read by the CLI status test.
- **External checks.** Exact coverage (`scripts/coverage-exact.sh`) on a
  capable host, remote CI on both operating systems, and macOS have not been
  run in this session. A local diagnostic is not the gate. It was
  `cargo llvm-cov -p brokkr-runtime --lib --lcov -- agents::`, and it
  reported `agents/mcp.rs` at LF 164 / LH 164 and FNF 24 / FNH 24.
