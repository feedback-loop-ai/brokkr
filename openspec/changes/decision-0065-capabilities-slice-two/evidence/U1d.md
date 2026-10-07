# U1d: Claude, Codex and LaneTally declarations (tasks 5.1–5.2)

U1d was built in run `0065-slice-two-unit-u1d-see-the--f6af0465` on branch
`s2/U1d`, cut from main at `bcf4d53c`, where U1c had landed. It touches the
row's three production files and no others: `adapters/claude.json`,
`adapters/codex.json` and `adapters/lanetally.json`. Each replaces its
legacy `mcp` form (`{flag, servers: {}}` for Claude and LaneTally, the bare
`"unsupported"` for Codex) with the typed `carriage` and `shapes` form that
U1b's loader in `agents/mcp.rs` decodes. No Rust production file, contract,
policy, fixture, reference or extension file moved. The MCP compile fence is
untouched. `McpSupport` still has no production consumer: U1f and U1g wire
it in, and U9b alone enables MCP grants.

Run `0065-slice-two-unit-u1d-see-the--9985bd0b` re-vouched this diff on
main at `1933aa9d`, where #500's `crates/brokkr-cli/tests/harness_probe.rs`
fixtures swap Claude's binary for a fake or wrap its driver, so they now
set `mcp` to `"unsupported"` because neither shape was measured.

## What is declared

Each fact comes from a U0 cell in
[slice-two-mcp-isolation.md](../../../../docs/evidence/adapters/slice-two-mcp-isolation.md)
and [its observations](../../../../docs/evidence/adapters/slice-two-mcp-observations.json),
read cell by cell rather than from a summary. Every shape names the harness
and binary it was measured under, which are the adapter's own (SI1), with
host `linux` and the version U0 measured. Every evidence or reason line
cites its U0 cell.

| Adapter | Shape | Version | Ambient | Native write | Store read | Process read | Cells |
| --- | --- | --- | --- | --- | --- | --- | --- |
| claude | cold, boxed | 2.1.287 | measured | measured | measured | measured | C04, C06, C07, C10 |
| claude | cold, none | 2.1.287 | measured | unsupported | unsupported | unmeasured | C03, C05, C07, C09b |
| codex | cold, boxed | 0.160.0 | unsupported | measured | unsupported | measured | X06, X07, X09, X12c |
| codex | cold, harness | 0.160.0 | unsupported | measured | unsupported | measured | X02, X02b, X03, X08, X09, X12 |
| codex | resume `work-site`, none | 0.160.0 | unsupported | unmeasured | unmeasured | unmeasured | X04, X05 |
| lanetally | cold, none | the unpublished wrapper by its two sha256 values, over claude 2.1.287 | measured | unsupported | unsupported | unmeasured | LT03, LT05, LT07, LT08, LT09, LT11 |

Carriage is measured for all three. For Claude it rests on C05 and C06, for
Codex on X02b, X04 and XA2, and for LaneTally on its own LT09 and LT10. The
Codex ambient reasons begin with the cause an SI2 refusal quotes. The cold
shapes read "project, system and managed MCP configuration cannot be
excluded", which is U0's proposed SI2 cause. The resume shape reads "project
MCP configuration cannot be excluded", because `/etc` was not planted on
that shape.

## What stays unmeasured

An absent shape reads `Unmeasured(Absent)` on every axis. These are left
absent on purpose:

- Claude's `replacement`, its declared `boxed-workspace` resume and macOS. C08
  resumed without hands, and Claude's resume shape is boxed, so C08 is
  telemetry only.
- LaneTally's boxed shape. LT10 and LT12 measured the wrapper's boxed cells,
  but `lanetally.json` declares `hands` unsupported. A measurement supplies
  no hands, and the loader refuses one (`UndeclaredHands`). Changing `hands`
  is outside this unit. Its `wrapper-work-site` resume and macOS also stay
  absent.
- Codex's `cold` with no hands (inline seats that author their own
  `--sandbox`), its `replacement` and macOS.

Three readings of the cells are assumptions, stated here for review:

1. Every Codex cell U0 marks `hands: false` ran under a `--sandbox` class.
   Brokkr's harness hands are exactly that fragment (decision 0046), so the
   cells are recorded as the `harness` shape and not as `none`.
2. For Claude and LaneTally without hands, process read is `unmeasured`
   with its reason, not `unsupported`. No canary was read, but no mechanism
   excluded one either. Claude's permission check blocked the one Bash
   probe, and native Read on `/proc` was not probed. MB2 refuses it either
   way.
3. Codex's boxed store read is `unsupported` on X09: Codex's own read-only
   sandbox, the class the boxed fragment passes, read the store when run
   without a model. In X06 the hands tool excluded the store, but the model
   declined the native probe (X06n, X06n2), and a declined probe is not
   isolation.

## Tests and the files they moved

The owning suites `agents/tests.rs` (5,838), `capabilities/tests.rs` (2,052)
and `bundle/agent_tests.rs` (6,788) are at or over their baselines and were
not edited. The new tests are in the existing child module
`crates/brokkr-runtime/src/agents/mcp/tests.rs`, which grew from 579 to 879
lines. They load the shipped `adapters/` through the real `Adapters::load`:

| Test | What it pins |
| --- | --- |
| `the_shipped_declarations_pin_only_what_u0_observed` | The table above exactly: each carriage, shape, version and axis variant, and the LaneTally version string. Codex's three ambient causes are pinned exactly, and every line is checked to cite a U0 cell. |
| `an_unmeasured_shape_stays_absent_and_cannot_be_claimed` | Eight unmeasured shapes read absent on every axis. Adding LaneTally's boxed cell to the shipped file refuses as `UndeclaredHands(cold, boxed, linux)`. Adding Claude's `boxed-workspace` resume without hands refuses as `ResumeHands { declared: "boxed" }`. |
| `a_declaration_byte_moves_its_adapter_resolution_and_bundle_identity` | Claude's digest equals the sha256 of the shipped file's bytes. One byte of one adapter's carriage evidence moves that adapter's digest alone. It also moves the `adapter_digest` of a resolution consulting that adapter alone, and `bundles/self`'s manifest digest for Claude and Codex. It does not move `bundles/self` for LaneTally, which `bundles/self` does not consult. |

Fixture swaps, each pinning behaviour this unit changes:

- **`engine/notice_tests.rs`**: `the_declaration_not_the_provider_name_decides_and_nothing_else_is_echoed`
  cloned `codex.json` into a provider whose binary is `fixture-cli`. It now
  sets that clone's `mcp` back to `"unsupported"`, Codex's pre-U1d form,
  because the loader rightly refused the borrowed binary. The file is
  size-neutral at 1,962 lines, with a two-line comment folded to one.
- **`tests/capability_launch.rs`**: two tests re-point a shipped adapter's
  driver at an opaque or bare command, and the loader refused the borrowed
  harness (`'<custom>'`). They now call the new child
  `tests/capability_launch/redrive.rs` (14 lines, `redrive::to`), registered
  by `#[path]` like its siblings. It sets the driver and drops `mcp` to
  `"unsupported"`. The parent stays at 11,672 lines, with a four-line
  comment folded to two.
- **`crates/brokkr-runtime/tests/witnesses.json`**: 17 bundle pins were
  re-measured, because a bundle's identity carries the digests of the
  adapters it consults. Every bundle moved except `recipes/research-dsh`.
  The seat cannot run `BROKKR_BLESS=1 …` (an environment prefix), so the new
  digests were copied from the unblessed run's moved-witness table. After
  that, `cargo test -p brokkr-runtime --test it witness_digests::` passed
  6 of 6.
- **`docs/status.md`**: the claude and lanetally "MCP flag Brokkr passes"
  cells now read `none`, as `status_pages`' rendering of a typed
  declaration says. `the_status_matrix_is_the_adapter_data` failed until
  they matched.
- **`quality/file-lines.txt`** records the two test files' counts.
  **`quality/too-many-lines.txt`** moves 19 `capability_launch.rs` entries
  two lines down, from 767–6986 to 769–6988, because the header gained the
  `mod redrive;` pair. No length changed.

## Mutations

Each mutation compiled and failed the named test at the named assertion.
Each was then restored, and the restored test passed. M1, M3 and M4 edit
adapter data. The rest edit Rust that this unit does not change.

| # | Mutation | Failing test, assertion |
| --- | --- | --- |
| M1 | `claude.json` cold/none `store_read` `unsupported` → `measured` (guessed support) | `the_shipped_declarations_pin_only_what_u0_observed`, the table `assert_eq!` (`["measured","unsupported","measured","unmeasured"]` against `…"unsupported"…`) |
| M2 | `agents/mcp.rs` `facts`: `store_read` decoded from `native_write` | the same test, the table `assert_eq!` |
| M3 | `codex.json` resume ambient cause widened to "project, system and managed …" | the same test, the causes `assert_eq!` |
| M4 | `lanetally.json` native-write line loses its `U0 ` citation | the same test, the citation `assert_eq!` (`["LT11: native Write …"]` against `[]`) |
| M5 | `facts` also files each measurement under `replacement` | `an_unmeasured_shape_stays_absent_and_cannot_be_claimed`, the absent `assert_eq!` |
| M6 | `admit`: boxed hands also admitted for binary `claude-lanetally` | the same test: `problem` panics "expected an 'mcp' refusal, got Ok(…)" for LaneTally's boxed cell |
| M7 | `admit_resume`: the hands check fires only for a `harness` declaration | the same test: `problem` panics "expected an 'mcp' refusal, got Ok(…)" for Claude's no-hands resume |
| M8 | `load.rs`: the adapter digest hashes the parsed object without `mcp` | `a_declaration_byte_moves_its_adapter_resolution_and_bundle_identity`, the shipped-bytes `assert_eq!`. With that assertion disabled as a probe and then restored, the moved table also fails, all `false`. |
| M9 | `agents.rs`: a resolution's `adapter_digest` hashes the consulted provider names only | the same test, the moved table (resolution column all `false`) |

After the restorations, `git diff` over `agents.rs`, `agents/load.rs` and
`agents/mcp.rs` was empty. `cargo test -p brokkr-runtime --lib` then passed
834.

## Gates

Gates ran on the staged tree over `bcf4d53c`. `cargo fmt --all -- --check`
was clean, and the workspace clippy with `-D warnings` finished clean.

Tests ran crate by crate under `timeout 590`. For brokkr-runtime: the lib
passed 834, `it` 120, `capability_launch` 75, `operated_repo` 1 and
`queued_launch` 3. Its preserved controls passed by name:
`an_mcp_grant_refuses_until_slice_two_even_unused_and_a_hands_grant_is_reserved`
and `the_shipped_adapters_declare_their_harness_as_the_record_says`. For
brokkr-cli: the lib passed 627, `it` 479, `driver_conformance` 27,
`transcript_surfaces` 13, and each heap test 1. Protocol, core, view, store,
bridge and seatbelt-probe passed every target.

`compile --bundle bundles/self` and `compile --bundle bundles/verify` both
succeeded. Because the unit adds no Rust production code and no prompt
byte, no budget moved: the runtime `budgets::` tests passed inside `it`.
`quality/ratchet.sh files` and `clones` held. Strict OpenSpec validation
passed 20 of 20 items with none failed, and `typos --hidden` and
`git diff --cached --check` reported nothing.

Pending: the exact-coverage gate on a host that can nest namespaces, remote
CI on both supported operating systems, and any macOS measurement. No macOS
shape is declared.

## Follow-ups (not fixed here)

- `lanetally.json`'s `hands.unsupported` reason still says boxed
  confinement through the wrapper is unmeasured, while U0's LT10 and LT12
  now measure it cold. Re-ruling LaneTally's hands is outside U1d's
  non-goals.
- `codex.json`'s `hands.notice.discovery_tool` is `tool_search`. U0 found
  that `tool_search` was removed in codex-cli 0.160.0, and discovery goes
  through the code-mode catalogue (XA1, XA2). That belongs to U7c and U7d.
- The resume identities still name claude 2.1.266 and codex-cli 0.154.0.
  The MCP measurements name 2.1.287 and 0.160.0 in their own `version`,
  which qualifies no resume.
