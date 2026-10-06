# U4c: calls normalized at the harness edge, legacy rows unchanged (tasks 15.1–15.2)

Run `0065-slice-two-unit-u4c-see-the--a610c304` built U4c on branch `s2/U4c`
from main at `19bca5ff`. It touched two of the row's three production files:
`crates/brokkr-protocol/src/adapters.rs` and the new
`crates/brokkr-protocol/src/adapters/capability_calls.rs`.
`crates/brokkr-protocol/src/lib.rs` needed no edit. The new module is a
private child of `adapters` (`mod capability_calls;` in adapters.rs, beside
`composite`, `start` and the rest), and its only consumer is in adapters.rs,
so nothing is exported.

## What changed

`capability_calls.rs` (157 lines) holds the one typed reading of a harness
tool call. An `Observation` records the telemetry `Format` (`Claude`, `Codex`
or `Dsh`; Lanetally's wrapper speaks Claude's), the harness's own call id
(`toolu_*`, `item_N` or `callId`, `None` when absent), and a `Tool`. A `Tool`
is one of:

- `Named`: the whole, unclamped name.
- `Mcp { server, tool }`.
- `McpUnidentified { reported }`: CC2's typed limitation. No server or tool
  is guessed from it.
- `Missing`.

Three constructors read U0's measured shapes (the "Call telemetry for U4"
table in `docs/evidence/adapters/slice-two-mcp-isolation.md`):

- `Observation::claude` reads a `tool_use` block's `id` and `name`.
- `Observation::codex` reads an item event's `item.id` and `item.type`. For
  `mcp_tool_call` it also reads `item.server` and `item.tool`. The category
  is never taken as a tool name, and an empty or absent server or tool gives
  `McpUnidentified`.
- `Observation::dsh` reads a `tool/call` event's `data.callId` and
  `data.name`.

Claude and dsh names of the form `mcp__<server>__<tool>` split at the first
separator after the prefix. A name under the prefix that does not split is
an unidentified MCP call, not a native tool of that name.

The legacy lowering, `Observation::legacy_tool`, is the module's one
consumer. It matches every (format, tool) pair, with no wildcard arm
(decision 0071 ruling 2). Codex shows its item category: `mcp_tool_call` for
both MCP variants, and `unknown` when the type is absent. Claude and dsh
show the name as reported (an MCP name is rebuilt byte for byte from its
parts). An empty or missing name gives `None`, which leaves the Claude row
without `tool` and skips the dsh row, exactly as before. The 80-character
clamp is applied here and only here, after the identity is typed. The call
id is bound and dropped in `legacy_tool` because no legacy row carries it.
New emission and deduplication wait for U4f2.

adapters.rs now calls the module at its three tool sites:
`fold_stream_event`'s `tool_use` loop, `fold_codex_event`'s
`item.started`/`item.completed` arm and `fold_dsh_event`'s `tool/call` arm.
Each replaces an inline `as_str` read and its `chars().take(80)` clamp.
`fold_codex_event` still emits both the start and the completion of an item,
with the same count and category-valued `tool`. The turn, usage, target,
effort, model, pre-turn and history handling is untouched. adapters.rs went
from 7,219 to 7,209 lines, and `fold_stream_event` from 112 to 108 lines.

## Tests and removal controls

Both owning protocol suites sit at their file-lines baselines
(`adapters/tests.rs` 19,093, `native_controls/tests.rs` 13,152). So the
proof is in the new module's own test child,
`crates/brokkr-protocol/src/adapters/capability_calls/tests.rs` (247 lines),
and neither parent changed. It reaches the three private folds as a
descendant of `adapters`.

| Test | What it pins |
| --- | --- |
| `each_format_reads_the_measured_call_identity_whole` (assertion at line 129) | Thirteen measured fixtures, each compared to the whole expected `Observation`. Claude: native `WebSearch`, plugin MCP `mcp__plugin_docs_srv__lookup`, a 92-character MCP name kept whole past the clamp, an unsplittable `mcp__engine`, a non-string name with no id. Codex: `command_execution`, `mcp_tool_call` with a long tool, an empty server, a missing tool, an empty item. dsh: `bash`, an MCP name, an unsplittable `mcp____probe`. |
| `the_folds_still_write_exactly_the_legacy_rows` (assertions at lines 206, 223, 245 and 246) | Compares the whole emitted rows of the real folds. Claude: four blocks give three rows with `tool` (one clamped to 80 characters), plus one without `tool` for the empty name, with usage on the first row only. Codex: both start and completion of an MCP item show `mcp_tool_call`, followed by an unidentified MCP item, `command_execution`, an empty category and `unknown`. dsh: two rows, the second clamped, with the empty and missing names skipped, and nothing before the first turn. No row carries a call id, server or any other new key. |

The external test was also run against main's unchanged folds: main's
`adapters.rs` plus only the `mod capability_calls;` line. It passed, so the
rows it pins are main's rows byte for byte. Each mutation below compiled and
was run with `cargo test -p brokkr-protocol --lib capability_calls`; M6 was
also run with `cargo test -p brokkr-runtime --test capability_launch legacy_journal`.
Each was then restored. M1 and M5–M7 were re-run against the final,
formatted text, which is where the line numbers come from. M2–M4 were run
on the pre-format text and failed the same loop assertion, which `cargo fmt`
moved to line 129.

| Mutation | Result |
| --- | --- |
| M1: `named` no longer recognizes the `mcp__` prefix | identity test FAILED at :129 (`Named("mcp__plugin_docs_srv__lookup")` for `Mcp { server: "plugin_docs_srv", tool: "lookup" }`). The legacy test passed, because the display is unchanged. |
| M2: `text` clamps to 80 characters before typing | identity test FAILED at the loop assertion (the long Claude MCP tool was cut to `…a_legacy_row`) |
| M3: Codex no longer treats `mcp_tool_call` as a category | identity test FAILED at the loop assertion (`Named("mcp_tool_call")` for the `Mcp` identity) |
| M4: dsh reads no call id | identity test FAILED at the loop assertion (`call: None` for `Some("call_1")`) |
| M5: Codex shows the concrete MCP tool instead of its category | legacy test FAILED at :223 |
| M6: the Claude fold emits a private observation early (an `observation` key holding the `Observation`'s debug text) | legacy test FAILED at :206. D9 matrix `legacy_journal::every_site_shape_journals_its_legacy_native_rows_through_export_and_verify` FAILED at `legacy_journal.rs:560`: the run parked with "seat record at journal seq 7 violates contracts/seat-record.v6.schema.json at /" instead of completing. |
| M7: the empty-name guard for Claude and dsh removed | legacy test FAILED at :206 (an empty `tool` on the Claude row) |

## The D9 native matrix at this merge

The existing runtime proof in
`crates/brokkr-runtime/tests/capability_launch/legacy_journal.rs` runs the
real compile, the shipped Claude driver over a deterministic harness, the
engine, store append, export and verify. It covers the ordinary, inline,
fallback, panel, sequence, resumed and replaced sites, and it passes
unedited. Its exact rows include `tool: "WebSearch"` and the `Read` row's
`target`. It finds no reserved or private field on any record, and it still
refuses a partial group and a private observation at direct append. M6 shows
the matrix catches early emission through the native path, independently of
the MCP compile fence. Codex and dsh do not run through that matrix. Their
external legacy proof is the fold-level test above, together with their
existing driver suites (`driver_conformance` 27 and the `heap_*` binaries),
which pass.

## Suites and gates

On the restored tree the brokkr-protocol suite passed: 641 lib tests (2
ignored), plus `hands_exits`, `secret_drop` and the doc test. brokkr-runtime
passed: 771 lib tests, `it` (120), `capability_launch` (70), `operated_repo`
and `queued_launch`. `witness_digests` held without blessing (6 of 6), so no
witness or compose pin moved. brokkr-core, brokkr-store, brokkr-view and
brokkr-bridge passed. brokkr-cli passed: lib 627, `it` 459,
`driver_conformance`, `transcript_surfaces` and the three heap binaries. The
self bundle compiled with exit status 0.

Formatting and clippy with `-D warnings` came back clean, and so did
`typos --hidden` and staged and unstaged `git diff --check`.
`openspec validate --all --strict` passed 20 of 20 items.
`quality/ratchet.sh files` held file size and `quality/ratchet.sh clones`
held duplication.

The seat refuses running a script, so `quality/measure.sh`'s steps were
applied by hand.

- File lines: adapters.rs went from 7,249 to 7,209, and the two new files
  were added at 157 and 247.
- Too-many-lines: re-measured with clippy's forced lint on brokkr-protocol.
  `fold_stream_event` went from 112 to 108 at its new line 1516.
  `dsh_launch_with` (109) and `run_seat_with` (342) only moved lines, to 4508
  and 6570.
- Suppressions: none were added, and the suppressions test passes against
  the unchanged baseline.

`scripts/measure-budgets.sh` ran. It measured the same package count (327)
and the same heap peaks. Its prompt-byte rewrites were small decreases at
sites this unit does not render. Those rewrites predate this branch and were
left for their owner, together with the date-only note changes.

No `pub` item was added. Exact coverage (`scripts/coverage-exact.sh`,
outside the box) and remote CI are pending.

## Second visit: the shared type and its encoding are exported

The first visit's review (run `0065-slice-two-unit-u4c-see-the--a610c304`,
review at head `4be35dfe`) returned one medium finding. The first visit
kept `Observation`, `Format` and `Tool` `pub(super)` and left their export
to U4e. But design.md's D9 handoff gives "the normalization type and
encoding" one home in this module, and U4e's row edits only runtime files,
so the engine could not have reached either. The sections above describe
the first visit. Where this section differs from them, this section is
current: "No `pub` item was added" and the M6 row no longer hold.

This visit exports the type and its encoding from
`crates/brokkr-protocol/src/adapters/capability_calls.rs`, which adapters.rs
now declares `pub mod capability_calls;`. That is the house's existing
`brokkr_protocol::adapters::…` path, so `lib.rs` again needs no edit.

- `Observation`, `Format` and `Tool` are `pub`, and so are the observation's
  three fields. The constructors and `legacy_tool` stay `pub(super)`: only
  adapters read a harness event, and any other crate decodes.
- One serde encoding is defined. `Format` is snake_case. `Tool` is tagged
  under `kind` (`named`, `mcp`, `mcp_unidentified`, `missing`), and
  `Named` became the struct variant `Named { name }` so it can carry that
  tag. A missing call id encodes as `null`. `Observation` and `Tool` deny
  unknown fields (decision 0071 ruling 3).
- `OBSERVATION_KEY` (`"observation"`) is the private checkpoint key the
  observation rides under. The D9 matrix already refused that key at the
  store, and now it reads protocol's constant instead of its own literal
  (ruling 5).

The legacy lowering is still the only production consumer. Nothing is
emitted, and the rows are unchanged.

| Test | What it pins |
| --- | --- |
| `capability_calls::tests::each_reading_has_one_encoding_and_decodes_back` (protocol, assertion at line 163) | Four readings built by the real constructors (Claude named, Codex MCP, dsh unidentified, Codex missing with no id) encode to exactly the expected objects and decode back to equal observations. |
| `legacy_journal::the_shared_observation_decodes_in_the_engine_crate_and_refuses_extras` (runtime `capability_launch`) | The engine's crate decodes protocol's encoding into `brokkr_protocol::adapters::capability_calls::Observation`, equal to the typed native search call. A forged top-level `capability`, a `dialect` inside `tool` and an unknown `kind` are each refused with serde's exact cause. |
| `legacy_journal::every_site_shape_journals_its_legacy_native_rows_through_export_and_verify` (unchanged test, updated fixture) | Its direct-append fence now plants the real encoding of the search call under `OBSERVATION_KEY`, and v6 still refuses it at `/`. |

Each mutation below compiled. Each was run with
`cargo test -p brokkr-protocol --lib capability_calls` and/or
`cargo test -p brokkr-runtime --test capability_launch legacy_journal`,
then restored.

| Mutation | Result |
| --- | --- |
| ME (replaces M6): the Claude fold inserts `serde_json::to_value(Observation::claude(..))` under `capability_calls::OBSERVATION_KEY`, which is the real early emission | The protocol legacy-row test FAILED at `tests.rs:242`. The D9 matrix FAILED at `legacy_journal.rs:607`: "seat record at journal seq 7 violates contracts/seat-record.v6.schema.json at /". |
| MA: `Observation` loses `deny_unknown_fields` | The cross-crate test FAILED at `legacy_journal.rs:537` (`unwrap_err` on an `Ok`, because the forged `capability` was read around). |
| MB: `Tool` loses `deny_unknown_fields` | The cross-crate test FAILED at `legacy_journal.rs:537` (the `dialect` stamp was read around). |
| MC: `Tool`'s tag key becomes `type` | The protocol encoding test FAILED at `tests.rs:163`. The cross-crate decode FAILED at `legacy_journal.rs:516`. |

The restored tree passes all of the following:

- brokkr-protocol (`--all-features --locked`): lib 642 (2 ignored),
  `hands_exits` 6, `secret_drop` 1, and the doc test.
- brokkr-runtime `--tests`: lib 771, `capability_launch` 71, `it` 120
  (including `witness_digests`, without blessing, so no pin moved),
  `operated_repo` 1 and `queued_launch` 3.
- brokkr-cli: lib 627, `driver_conformance` 27, all three heap binaries,
  `it` 459 and `transcript_surfaces` 13.

The self bundle compiled. Formatting and workspace clippy with
`-D warnings` were clean. `quality/ratchet.sh files` and
`quality/ratchet.sh clones` both held.

`quality/file-lines.txt` moved for three files:

- `capability_calls.rs`: 157 to 173.
- Its `tests.rs`: 247 to 283.
- `legacy_journal.rs`: 586 to 633.

adapters.rs stays at 7,209, and no function's length changed. Exact
coverage and remote CI are still pending.

Ruling 6 is bent knowingly here. The serde encoding and `OBSERVATION_KEY`
have no production reader until U4e. Their consumers in this PR are the
cross-crate test and the D9 fence. The accepted design puts their one
home here, and U4e's row cannot add them.

## Third visit: the API snapshot, two guard branches, and the task text

Run `0065-slice-two-unit-u4c-see-the--cc701db3` re-vouched the whole U4c
diff, applied uncommitted on main at `19bca5ff`. The chief of the second
visit's run flagged two defects, and CI's exact-coverage gate on `76381e77`
found two unexercised branches. Where this section differs from the two
above, it is current.

The public-API snapshot was stale. Before this visit `quality/ratchet.sh api`
refused with "brokkr-protocol: the public API differs", and its diff added
exactly the 19 `adapters::capability_calls` lines (the module, `Format` and
its three variants, `Tool` with its four variants and four fields,
`Observation` with its three fields, and `OBSERVATION_KEY`). The operator
ruled that raise on 2026-10-05. The snapshot was regenerated the way
`api_file` in `quality/lib.sh` does it: `cargo +nightly-2026-09-05
public-api -p brokkr-protocol -sss --color never` under the unchanged
provenance line (`cargo public-api --version` printed 0.52.0, and
`rust-nightly-version.txt` reads `nightly-2026-09-05`). The seat refuses
running a sourced script, so the two steps were run by hand. The result
differs from the old snapshot by 19 added lines and no removed line, and the
api ratchet now prints "public API holds". `quality/ratchet.sh baselines
19bca5ff`, run without a PR body, refuses and lists one raise only:
"public-api/brokkr-protocol.txt: 730 public items (was 711)". The seat cannot
set `PR_BODY` for the run, so the pass with the PR's `Ruling:` line is
pending, to be read in CI.

Task 15.1's and 15.2's evidence lines still described the first visit: a
private module, no exports, an unedited D9 matrix, M6 and 70
`capability_launch` tests. Both lines now state the exported module, the
updated D9 fixture and the cross-crate decoding test, ME in place of M6, and
the totals measured below.

Both empty-tool guards, `!server.is_empty() && !tool.is_empty()` at
`capability_calls.rs:92` (Codex) and `:164` (Claude and dsh names), had
never been reached with a non-empty server and an empty tool. Two fixtures
join `each_format_reads_the_measured_call_identity_whole`, which now holds
fifteen. The production code did not change.

| Fixture | Expected reading |
| --- | --- |
| Codex `mcp_tool_call` item `item_3` with server `"s"` and tool `""` | `Observation { format: Codex, call: Some("item_3"), tool: McpUnidentified { reported: "mcp_tool_call" } }` |
| dsh `tool/call` `call_4` named `mcp__server__` | `Observation { format: Dsh, call: Some("call_4"), tool: McpUnidentified { reported: "mcp__server__" } }` |

Each guard was removed by a compiling edit, run with
`cargo test -p brokkr-protocol --lib capability_calls`, and restored. After
the restore the source matched the reviewed text (`git diff` of the file
printed nothing), and the three tests passed.

| Mutation | Result |
| --- | --- |
| MF: the Codex guard drops `&& !tool.is_empty()` | The identity test FAILED at `tests.rs:140`: left `tool: Mcp { server: "s", tool: "" }`, right `McpUnidentified { reported: "mcp_tool_call" }`. |
| MG: the name split drops `&& !tool.is_empty()` | The identity test FAILED at `tests.rs:140`: left `tool: Mcp { server: "server", tool: "" }`, right `McpUnidentified { reported: "mcp__server__" }`. |

The rest of the diff was reviewed against U4c's row. No further defect was
found against CC1, CC2 or D9. Each fold's output still comes from
`legacy_tool` alone. Codex rows still show the item category, or `unknown`.
Claude and dsh rows show the name as reported, rebuilt byte for byte from an
MCP split. Nothing writes the observation, its key or a call id.

These results come from the restored tree. brokkr-protocol
(`--all-features --locked`) passed lib 642 (2 ignored), `hands_exits` 6,
`secret_drop` 1 and its doc test. brokkr-runtime passed lib 771, `it` 120
(with `witness_digests` unblessed, so no pin moved), `capability_launch` 71,
`operated_repo` 1 and `queued_launch` 3. brokkr-cli passed lib 627
(1 ignored), `driver_conformance` 27, the three heap binaries, `it` 459
(2 ignored) and `transcript_surfaces` 13. brokkr-bridge (17), brokkr-core
(105, `it` 24, `seatbelt_lifetime_probe` 99), brokkr-store (85, `it` 5) and
brokkr-view (267) passed. `cargo +1.88 check --workspace --all-targets
--all-features` finished without error. The self bundle compiled. Formatting
and workspace clippy with `-D warnings` were clean, and so were
`typos --hidden` and staged and unstaged `git diff --check`.
`openspec validate --all --strict` passed 20 of 20. The api, files, clones,
listings and table ratchets held.

`bash scripts/coverage-exact.sh` ran to completion on this host. It reported
lines 45,618 of 45,618, branches 6,894 of 6,894 and functions 5,028 of
5,028. In its lcov, `capability_calls.rs` has 18 of 18 branches hit, and
the branches that were missing at lines 92 and 164 each now have one hit.
Remote CI on the final head is pending.

`quality/file-lines.txt` moved for one file: the test child went from 283 to
292. No function length changed, so `too-many-lines.txt` and the
suppressions are as the second visit left them.

The LOW finding is carried, as the commission directs. The serde encoding
and `OBSERVATION_KEY` are public with no production reader until U4e
(decision 0071 ruling 6). Their consumers in this PR are the cross-crate
test and the D9 fence. Design D9 gives the type and its encoding one home
in this module, and U4e's row edits only runtime files.

## Notes for later units

U4e decodes `Observation` from the checkpoint's `OBSERVATION_KEY` and
removes it before `Checkpoints::offer`. U4f2 switches `legacy_tool`'s caller
to emit the encoding under that key, inside adapters.rs and this module.
CC2's cause text ("MCP telemetry does not identify its server and tool")
belongs to the consumer that reports the `McpUnidentified` variant. This
unit defines no error type for it, so nothing goes unused. The Codex
code-mode gap U0 recorded (an `exec` call wrapping several MCP calls, and an
X08 `command_execution` missing from the stream) is unchanged here.
