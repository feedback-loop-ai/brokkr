# U4e: single and panel calls stamped by the engine (tasks 17.1–17.2)

Run `0065-slice-two-unit-u4e-see-the--6ca757e0` built U4e on branch
`s2/U4e` from main at `ed67ed15`. It changed exactly the row's three
production files: `crates/brokkr-runtime/src/engine.rs`, the new
`crates/brokkr-runtime/src/engine/capability_calls.rs`, and
`crates/brokkr-runtime/src/engine/checkpoints.rs`. No contract, fixture,
policy table, reference schema, extension, adapter, protocol or store file
moved, and no shipped driver emits an observation yet (U4f2).

## What changed

`capability_calls.rs` is the engine's one consumer of protocol's shared
`Observation`. `Calls::of` reads one site's call authority from the outcome
its spawn was sealed with: it finds, among the site's `SiteCapabilities`
outcomes, the one whose `identity()` equals the identity in the spawn's
private launch record. That is the selected candidate's own outcome, so a
fallback reads its own holdings and never its primary's, and a panel member
reads its own and never a sibling's. From that outcome it keeps two facts.
The first is each tool a native holding admits, by its exact name, with the
capability and the holding's dialect. An `mcp` holding is skipped, because
CC1 tells its tools apart by their `cap-` server. The second is every tool
the adapter inventory knows, read from the known plan's guards through
protocol's own `native_controls::managed` decoder, the one the driver reads
them with. An unmeasured plan knows nothing.

`Calls::consume` removes `OBSERVATION_KEY` from a checkpoint and judges the
call it describes. A row without the key is legacy and passes unchanged. An
observation that does not decode is refused as unattributable. A
`Tool::Named` call to a held tool yields a `Stamp`: the concrete tool,
unclamped, the capability, the dialect, and the call id
`<attempt>[:<member or step>]:<harness id>`, with call state `observed` and
no response digest. A held call without a harness id, or with an empty one,
is refused with `capability telemetry cannot be attributed`. No id is
guessed. A known tool that no holding admits is refused with `observed
capability tool '<tool>' is not held by this attempt`. Every other call
stays ordinary: a local tool, workspace hands, an MCP call whose evidence
is its broker's ledger (CC2), an unidentified MCP call, or a call that
names nothing. Its legacy fields are kept, and the observation is gone.

`engine.rs` installs the consumer beside the existing boundary and site
stamps. In `run_driver` it runs on the driver's raw data before the member
tag and stamps. In `run_panel` each member's `Calls` is built once, before
the threads start, and the receive loop consumes with the member that sent
the checkpoint. Both sinks then hand `Checkpoints::offer` the consumed
checkpoint and the call's result. `offer` erases the driver's five
attribution fields first, as U4a2 made it do, and then writes the engine's
stamp behind the erasure, so no driver value survives and the engine's
value does. A refused call latches as the fence's refusal does, in the new
`Refused` enum, which has a `Fence` variant and a `Call` variant. Rows held
before it still land at settlement, nothing later is journaled, and
`Settled::outcome` turns the attempt into `Failed` with the exact cause.

To keep `engine.rs` within its 4,862-line baseline, `refused_outcome` moved
unchanged into `checkpoints.rs`. Its only production caller is there. It
now takes any `Display`, so it serves both refusal kinds. `engine.rs` is
4,861 lines and `checkpoints.rs` 396. The new `capability_calls.rs` is 178.

## Assumptions read from the framing

The ordinary row keeps the driver's legacy `tool`. Only an attributed row
takes its `tool` from the observation. This preserves legacy input
unchanged, and it adds no second copy of protocol's private display
lowering.

The call id's shape is judged by seat-record v6's fence, so this unit adds
no second home for SC4's vocabulary. Only absence and emptiness are refused
here. An empty id would otherwise collapse every such call into
`<attempt>:`, which v6 admits (mutation M5).

`run_driver` is shared with sequence steps, so the consumer also runs there
and on retried attempts. The compile-path test below pins that behaviour as
it stands. U4f still owns the sequence, resume and replacement proofs, the
measured history filtering, and deduplication of start/completion pairs.
Today two observations of one call get one call id but two rows.

## Tests

`crates/brokkr-runtime/src/engine/capability_tests/call_tests.rs` (300
lines) is a new child of the owning `capability_tests.rs`, registered there
by `mod call_tests;`. Real `sh` driver processes speak the driver protocol
through `run_driver` and `execute_panel` into the store's v6 fence. Each
journal is exported and verified offline by `brokkr_store::verified_events`
before its checkpoints are compared. The fixtures reuse the parent's
`holdings`, `two_candidates`, `link`, `marked` and `canonical_engine`, and
`engine::tests`' `templated`, `checkpointing_command`, `member` and
`panel_input`. The site holds `web-search` through `codex-native-search`
on the Codex primary, and holds and knows nothing on the DSH fallback.

| Test | What it asserts exactly |
| --- | --- |
| `a_held_call_is_attributed_to_the_selected_holding_and_local_calls_stay_ordinary` | The three stored rows. The held search, sent with a forged `web-fetch` group, is stored with `web-search`/`codex-native-search`/`attempt:item_1`/`observed`. A `command_execution` call carrying the same forged group is stored as the bare legacy row. A legacy row with no observation is unchanged. `report.refused` is `None`. |
| `a_fallback_never_borrows_its_primarys_holding` | The same search, served by the DSH fallback, is stored as the bare legacy row. |
| `an_unheld_or_unattributable_call_fails_its_attempt` | Four cases: a known `web_search` under Codex's switched-off plan, a held call with no id, one with an empty id, and an unreadable observation. Each stores only the row before it and refuses with the exact `Failed` cause. |
| `an_mcp_holding_or_an_mcp_call_attributes_nothing` | With the holding turned `mcp`, `consume` returns `Err(Unheld { tool: "web_search" })` for the native search and `Ok(None)` for a `Tool::Mcp` call, and both lose the observation. |
| `a_panel_member_is_attributed_by_its_own_holding_never_its_siblings` | Per member, both rows. The Codex member's search carries the group with `attempt:searcher:item_1`. The DSH sibling's search is ordinary. Each member's `panel-member-finished` marker is exact. |

`crates/brokkr-runtime/tests/capability_launch/legacy_journal.rs` (the D9
matrix, 633 to 718 lines) gains the compile-path proof. The wrapper that
serves production's Claude driver now passes each protocol line through
`sed -f observing.sed`, one line at a time, but only when the test writes
that script under the canonicalised root. The legacy test writes none, so
its path is byte-for-byte what it was. A first attempt used `awk` and hung,
because mawk block-buffers a piped input; the per-line `read` loop is
portable to macOS. The matrix setup became the shared `driven` helper, and
its export/verify the shared `exports_and_verifies` helper. `journaled`
also spells a call id's owning attempt `<attempt>` after checking that the
id begins with it.
`every_site_shape_journals_the_engines_group_on_an_observed_held_call`
injects U4f2-shaped observations on the fake harness's `WebSearch` and
`Read` calls. The search also gets a forged call id, state and digest. The
test then asserts:

- every site's rows equal the legacy matrix's, with the engine's group on
  each search and the read ordinary. The site shapes are ordinary, inline,
  fallback, a panel's inline and agent members, a sequence's inline and
  agent steps, a resumed root, a replaced root and review.
- exactly 12 attributed rows.
- no `response_sha256` or observation on any row.
- export and offline verify to the completed run.

The existing legacy test still passes unchanged.

Other test edits change no behaviour. `contention_tests.rs` passes
`Ok(None)` at its eleven `offer` calls. `tests.rs` (still 6,681 lines) and
`cleanup_tests.rs` (222 to 225) reach the moved function as
`checkpoints::refused_outcome` and `brokkr_store::SeatRecordError`. In
`capability_tests.rs` one blank line between two items was removed to make
room for the registration, so it stays at its 2,231-line baseline. No test
was relocated.

## Removal controls

Each mutation compiled, was run, failed the named assertion, and was
restored. The single-site tests were later folded onto one `served` helper
to clear a test clone. M1–M6 and M8–M10 were then rerun on that file, with
the same failures. M1b and M7 were not rerun; the panel test's assertions
did not change, and M7 also failed the compile-path test, which the refactor
did not touch. M11 and M12 ran on the final file. After M10's restore the
working tree matched the staged tree (`git diff --stat` empty). After M12's
restore the whole runtime suite passed.

| # | Mutation | Failed (test: assertion) |
| --- | --- | --- |
| M1 | `run_driver` consumes `&mut Value::Null` instead of the checkpoint (bypass) | `a_held_call…`, `a_fallback…`: stored rows (journal empty). `an_unheld…`: refusal reads `seat record at journal seq 6 violates contracts/seat-record.v6.schema.json at /`. Compile path: `driven`'s status is `AwaitingOperator` on `… seq 7 violates contracts/seat-record.v6.schema.json at /` |
| M1b | `run_panel` consumes `&mut Value::Null` | `a_panel_member…`: per-member rows (searcher's `outcome: failed`, no rows) |
| M2 | `Calls::of` takes the first outcome whatever the sealed identity | `a_fallback…`: the DSH row carries Codex's group. `a_panel_member…`: the sibling is attributed |
| M3 | a known unheld tool returns `Ok(None)` | `an_unheld…`: stored rows, case "observed capability tool 'web_search' is not held by this attempt" |
| M4 | a missing id becomes `<owner>:guessed` | `an_unheld…`: stored rows show `attempt:guessed` |
| M5 | the empty-id filter is removed | `an_unheld…`: stored rows show `attempt:`, admitted by v6 |
| M6 | an unreadable observation returns `Ok(None)` | `an_unheld…`: stored rows, case "capability telemetry cannot be attributed" |
| M7 | the panel's owner drops the member tag | `a_panel_member…`: per-member rows. Compile path: `journaled == wanted` |
| M8 | every member uses `calls[0]` | `a_panel_member…`: the sibling is attributed |
| M9 | `without_driver_attribution` erases nothing | `a_held_call…`, `a_panel_member…`: rows. `a_fallback…`: the forged `web-fetch` group with its digest is stored, since v6 admits it. Compile path: `AwaitingOperator` on the v6 refusal at seq 7, from the forged digest beside `observed` |
| M10 | `offer` drops the stamp | `a_held_call…`, `a_panel_member…`: rows. Compile path: `journaled == wanted` |
| M11 | an `mcp` holding enters the native map | `an_mcp_holding…`: `Ok(Some(Stamp { … }))` against `Err(Unheld { tool: "web_search" })` |
| M12 | `Tool::Mcp`'s tool is read as a native name | `an_mcp_holding…`: `Err(Unheld …)` against `Ok(None)` |

Commands: `cargo test -p brokkr-runtime --lib engine::capability_tests::call_tests`
and `cargo test -p brokkr-runtime --test capability_launch legacy_journal::`.

## Gates on the final tree

Base `ed67ed15` plus this unit's changes, before commit. Formatting was
clean and workspace clippy with all targets, all features, `--locked` and
`-D warnings` finished without a diagnostic. brokkr-runtime passed its lib
(780), `capability_launch` (72), `it` (120, with `witness_digests`
unblessed), `operated_repo` (1) and `queued_launch` (3). brokkr-cli passed
its lib (627, 1 ignored), `it` (459, 2 ignored, including the suppressions
and test-target gates), `driver_conformance` (27), `transcript_surfaces`
(13) and the three heap binaries. Protocol (642 lib), core, store, view
and bridge passed. `bundles/self` and `bundles/verify` compiled with exit
status 0. The files and clones ratchets held. `openspec validate --all
--strict` reported 20 passed, 0 failed. `typos --hidden` and
`git diff --cached --check` printed nothing. `scripts/coverage-exact.sh` ran
on this host and reported lines 45,764 of 45,764, branches 6,900 of 6,900
and functions 5,048 of 5,048.

`quality/file-lines.txt` was edited by hand, because this seat refuses a
shell redirect. It records the changed counts above, the two new files and
`legacy_journal.rs`. `quality/too-many-lines.txt` records the shifted lines
of `seat_input`, `execute` and `decide` in `engine.rs`, and of the four long
tests after the removed blank line in `capability_tests.rs`. No length
changed and no suppression was added. No witness or compose pin moved.
`scripts/measure-budgets.sh` ran, and this unit moves none of its counts.
Crate count and heap peaks matched. Several prompt sites measured 13 bytes
under their budgets, from prompt inputs this diff does not touch, so the
rewritten budget files were reverted rather than re-baselined here. Remote
CI on both operating systems is pending.

## Findings carried

LOW (decision 0071 ruling 5): the exact-name map from native holdings to
capability and dialect is derived again in `Calls::serving`, from the
sealed `Outcome.held`. U4d's `Attribution` cannot be reached from the
engine. Its module is private to `capabilities`, and its only constructor,
`Authority::attribution`, needs the compile-time `Authority`. The compile
has already refused ambiguous and unrepresentable names. A follow-up can
give `capabilities/attribution.rs` a constructor from an `Outcome`, and the
engine can consume it.

The known-tool set is read from the sealed plan's rendered `controls`
through protocol's typed `managed` decoder, with an `expect` that the
engine's own known plan reads back. The engine holds no adapter at spawn,
and carrying the inventory on `Outcome` would edit `capabilities.rs`.
