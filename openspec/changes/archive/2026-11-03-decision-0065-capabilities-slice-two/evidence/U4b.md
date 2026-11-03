## U4b: seat-record v6 published and consumed (tasks 14.1–14.2)

Run `0065-slice-two-unit-u4b-see-the--889c29bb` built U4b on branch
`s2/U4b` from main at `4238aca1`, workspace version 0.12.0. Its council
held it (R1 HIGH: v6 admitted a driver-supplied complete attribution
group before the engine owned that group). The operator ruled erasure
first, and U4a2 landed it at `37e9250a`. The controller rebuilt this work
on `37e9250a`, and run `0065-slice-two-unit-u4b-see-the--79e8137a` is the
repair visit: it re-observes R1 under v6 and answers R2 and R3 below. The
unit touches the row's three production files and no others: the two new
schema files and `seat_record.rs`. `seat_record/validation.rs` and the
store's `lib.rs` did not move.

### The repair visit (R1–R3)

R1 is closed by U4a2, and this visit observed it under v6. On the
rebuilt tree, `cargo test -p brokkr-runtime --lib -- engine::checkpoints::tests`
passed its three tests. With the erasure loop's `object.remove(field)`
replaced by `let _ = field;` in `engine/checkpoints.rs`, both
`a_single_site_driver_cannot_journal_capability_call_attribution` and
`a_panel_member_cannot_journal_capability_call_attribution` failed: the
stored second row carried the forged `capability`, `dialect`, `call_id`
and `call_state`, because v6 admits a complete group. So under v6 the
erasure is what keeps a driver's group out of the journal, and U4a2's
tests bind there. The mutation was undone and the three tests passed.

R2: `an_invalid_boundary_record_fails_at_append_without_writing_the_result`
in `boundary_tests.rs` now asserts the whole journal diagnostic, "seat
record at journal seq 2 violates contracts/seat-record.v6.schema.json at
/", which also pins that the refused value stays out of it. For each of
V4, V5 and V6 it asserts the exact `SeatRecordError { seq: 2, path: "/",
contract }` with that version's literal contract path, in place of
`is_err()`. The file went from 4,620 to 4,616 lines. Three compiling
removals in `seat_record.rs` each failed it: deleting `of_engine`'s v6 arm
(`:4163`, left named v5), handing V6 `CONTRACT_V5` in `contract()`
(`:4163`, the same), and handing V5 `CONTRACT_V4` (`:4172`, "V5 refuses a
word that is not the realm's", left contract v4, right v5). Each was
undone and the test passed.

R3: `v6_refuses_partial_groups_bad_states_digests_and_identities` gains a
lone `dialect`, a lone `call_id` and a lone `call_state` on the legacy
tool row, beside the lone `capability` it had, so each dependency entry is
the only rule that can refuse its row. It also gains an uppercase
`dialect`. Each case asserts `{ seq: 4, path: "/", contract: v6 }`. Four
schema mutations of the embedded copy each failed it at
`seat_record/tests.rs:780`: the `dialect` entry renamed to `x-dialect`
(left `Ok(())` for the lone dialect row), the same for `x-call_id` and
`x-call_state` (each its own lone row admitted), and the `dialect` pattern
widened to `^[A-Za-z0-9][A-Za-z0-9._-]*$` (the `Claude-Native-Search` row
admitted). Each was undone by hand, `cmp` confirmed the embedded copy
equals the published file, and the store library passed 85 tests.

### What changed

`contracts/seat-record.v6.schema.json` is v5 with five optional checkpoint
properties: `capability` and `dialect` (realm-name grammar, 128 bytes),
`call_id` (`^[A-Za-z0-9][A-Za-z0-9_.:-]*$`, 128 bytes), `call_state`
(`observed`, `succeeded`, `failed`, `refused`, `interrupted`; no `started`)
and `response_sha256` (64 lowercase hex). Five `dependencies` entries make
any attribution field require the whole group of five, `tool` included.
`target` now depends on `tool` alone. Three new `allOf` conditions do the
rest. The first keeps v5's tool-to-turn dependency for every row except a
settled broker state (any of the four outcomes). The second admits
`response_sha256` only beside `succeeded` or `failed`. The third holds an
unattributed `tool` to v5's 80 bytes. `tool` itself is widened to 256
bytes in v5's unchanged pattern. v5's two `site_ref` conditions are kept
verbatim. The embedded copy is a byte-for-byte `cp`; `jq .` reproduces
both files exactly.

`seat_record.rs` gains `SCHEMA_V6`, `CONTRACT_V6`, `VALIDATOR_V6`, the
`V6` variant in all three of the table's matches, and `V6_ENGINE = (0, 12,
0)` as the first arm of `of_engine`. The boundary follows CC3's existing
convention. 0.12 is the development line on main after the `v0.12.0`
tag. `engine` carries no position within a line, and v6 accepts every
valid v5 record, so the tagged 0.12.0 engine's rows read the same as
before. The 0.10 and 0.11 lines keep v5. The file grew from 199 to 220
lines. The `call_id` grammar is this visit's reading of SC4's "bounded
nonsecret ASCII identifiers". It admits harness ids such as `toolu_01…`
and `call_…` and an attempt prefix joined by `:`, and refuses whitespace,
`/` and non-ASCII.

### Tests

These are new in `seat_record/tests.rs`:
`the_zero_twelve_line_reads_v6_and_the_eleven_line_still_reads_v5`,
`v6_admits_one_native_observed_or_broker_settled_group`,
`v6_refuses_partial_groups_bad_states_digests_and_identities`,
`every_valid_v5_shape_is_a_valid_v6_record_and_v5s_conditions_stand` and
`append_export_and_verify_judge_one_v6_contract`. The dispatch test pins
these engines: 0.11.0, 0.11.99 and 0.11.99+build.7 read v5. 0.12.0-rc.1,
0.12.0+build.7, 0.12.0, 0.12.3 and 1.0.0 read v6. 0.12, 0.12.0.1 and the
empty string read v1. The same complete native group is admitted under a
0.12.0 manifest and refused as `SeatRecordError { seq: 2, path: "/",
contract: v5 }` under 0.11.99. With no engine named it is refused as v1.
The refusal test asserts exactly `{ seq: 4, path: "/", contract: v6 }` for
32 records:

- each of the five group fields removed from a native observed row;
- `capability`, `dialect`, `call_id` or `call_state` alone, or a digest
  alone, on a legacy tool row;
- `started` and `pending` on a row with a real turn;
- a digest beside `observed`, `refused` or `interrupted`;
- an uppercase digest and a 63-character digest;
- 129-byte capability, dialect and call_id;
- an uppercase capability, an uppercase dialect, and call_ids with a
  space or `é`;
- an attributed tool of 257 bytes or containing a space;
- an unattributed tool of 81 bytes;
- a native observed row, a legacy tool row and a legacy target row, each
  without a turn;
- a private `observation`, and an unknown `server` key.

The admit test passes these:

- native `observed` on turn 3;
- each settled state without a turn and on turn 2;
- `succeeded` and `failed` with a digest;
- an attributed row at every inclusive bound (128, 128, 128 and a 256-byte
  `mcp:`-prefixed tool).

The superset test runs six v5 shapes under v5 and v6 alike:

- a legacy tool row with a target;
- an 80-byte tool;
- the unstamped codex resume;
- a stamped root launch;
- a v5 refusal token;
- a dialect step's result with a boundary.

It then shows a stamped resume without a root is still refused under v6.
The store-level test opens one store with a 0.12.0 run and a 0.11.0 run.
A partial group is refused at seq 5 with the head unmoved, and the
module's text is pinned once: "seat record at journal seq 5 violates
contracts/seat-record.v6.schema.json at /". A native observed row and a
broker `refused` row then land. The export verifies offline to seq 6, and
the 0.11.0 run refuses the native row as v5 at seq 5. A digest beside
`refused`, planted past the fence at seq 7, is refused by both
`export_ndjson` and `verify_export` with the same v6 error. The
embedded-copy test now covers v6.

Five existing tests move with the boundary, because a run this 0.12.0
engine starts now reads v6. `the_version_is_the_one_the_runs_engine_wrote`
expects `1.0.0-rc.1` to read v6. In
`the_zero_ten_line_reads_v5_and_the_nine_line_still_reads_v4`, `1.0.0`
became `0.11.99`. `boundary_tests.rs`'s
`an_invalid_boundary_record_fails_at_append_without_writing_the_result` now
pins the exact v6 diagnostic and checks v6 too (R2 above). `contention_tests.rs`'s two refusal texts name v6.
`dialect_policy.rs`'s `the_tool_vocabulary_is_seat_records_own` reads v6's
`tool` pattern, which has the same bytes as v5's. `frozen_contracts.rs`
pins v5's digest `d0083a17…` in `FROZEN` and gains
`the_v6_seat_record_lands_beside_its_frozen_predecessor`. That pin is kept
out of the 109-line listed function so that function does not grow.

D9's legacy matrix,
`every_site_shape_journals_its_legacy_native_rows_through_export_and_verify`,
passes at this merge with every site shape and every expected row
unchanged. It still runs the shipped Claude driver and lowering, and no
record carries an attribution field or a private observation, so drivers
still emit legacy rows. Its closing helper is now
`fences_the_attribution_group`. A partial group and a private observation
are still refused, now as `{ seq: head + 1, path: "/", contract: v6 }`,
with the head unmoved. A whole engine-owned group is v6's to admit, so it
lands at `head + 1`, and the export sweep passes the journal with one line
per event. Verify is left to the store test, because verify folds and a
completed run takes no further event: the first attempt here failed
`Fold(AfterTerminal { seq: 121 })`.

### Removal mutations

Each mutation compiled and ran on this visit's tree, and was then undone.
Production edits were reverted by hand. Schema mutations were written with
`jq` over the published file into the embedded copy and restored with
`cp`, and `cmp` confirmed the restore. Every schema mutation also failed
the embedded-copy test, which this table leaves out.

| Mutation | Tests that failed (assertion) |
| --- | --- |
| M1: `of_engine`'s v6 arm removed | `the_zero_twelve_line…` (0.12.0-rc.1: left V5, right V6); `the_version_is…` (`1.0.0-rc.1`); `append_export_and_verify…` (text named v5); the D9 matrix, `legacy_journal.rs:529` (left contract v5, right v6, seq 121) |
| M2: `source()` handed V6 the v5 bytes | `v6_admits…`, `the_zero_twelve_line…` (the 0.12.0 group refused), `append_export_and_verify…` (complete rows refused at append) |
| M3a: the settled-broker turn exemption deleted | `v6_admits…` (settled states without a turn refused), `append_export_and_verify…` |
| M3b: the digest-state condition deleted | `v6_refuses…` (digest beside `observed` admitted), `append_export_and_verify…` (the planted row exported) |
| M3c: the `capability` dependency deleted | `v6_refuses…` (lone capability admitted) |
| M3d: the unattributed 80-byte condition deleted | `v6_refuses…` (81-byte legacy tool admitted) |
| M3e: `tool` made a group dependency key | `every_valid_v5_shape…` (legacy tool rows refused) |
| M3f: v5's stamped-resume condition deleted | `every_valid_v5_shape…` (stamped resume without root admitted) |
| M3g: attributed tool bound 256 → 255 | `v6_admits…` (the 256-byte tool refused) |
| M3h: `started` added to `call_state` | `v6_refuses…` (`started` admitted) |
| M3i: the tool-to-turn condition deleted | `v6_refuses…` (legacy tool without a turn admitted) |
| M3j: `observed` exempted from the turn | `v6_refuses…` (native observed without a turn admitted) |
| M4: v5's bytes perturbed by one trailing space; v6 retitled | `the_frozen_contracts_and_the_corpus_keep_their_exact_bytes` ("seat-record.v5 bytes moved"); `the_v6_seat_record_lands…` (left "Forge seat record six") |

M3h's first run did not fail. The `started` case then had no turn, so it
was refused for the missing turn rather than for its state. The case was
given a real turn, and M3h then failed as intended. The partial-group
cases were moved onto the turned native row for the same reason, and M3i
and M3j were added to bind the turn rule from both sides. After every
restore the store library passed 85 tests and the D9 matrix passed.

### 14.1 and 14.2

The v6 schemas are published and embedded byte-equal, and dispatch
consumes them at append, export and offline verify. One native observed
group, or a settled broker group with or without a real turn, is
admitted. A partial group, public `started`, a misplaced or malformed
digest and every out-of-bound identity are refused exactly, with the head
unmoved. Old-shaped rows stay valid, and v5's bytes are pinned. D9's
legacy matrix passes at this merge. M1–M4 each failed an intended
assertion and were restored, and so did the repair visit's R2 and R3
mutations. Those are the grounds both tasks are ticked on.

### Gates on the repair visit's tree

These were run on `37e9250a` plus this unit. Formatting was clean, and
workspace clippy with all targets and features under `-D warnings`
finished without a warning. `cargo +1.88 check --workspace --all-targets
--all-features` finished clean. Each crate's suite ran on its own under
`timeout 590`. The store library passed 85 tests and its other binaries
passed. The runtime library passed 771. The runtime's `it` binary passed
120, among them `frozen_contracts::` 15 and `witness_digests::` 6 with no
bless, so no witness or compose pin moved. `capability_launch` passed 70,
D9's
`legacy_journal::every_site_shape_journals_its_legacy_native_rows_through_export_and_verify`
among them, and `operated_repo` and `queued_launch` passed. The CLI
library passed 627, its `it` binary 459, and `driver_conformance`, the
three heap binaries and `transcript_surfaces` passed. Core, view,
protocol and bridge passed. `bundles/self` compiled to `11c7d0e7…`,
unchanged. OpenSpec strict validation passed 20 of 20, typos found
nothing, and git's whitespace check was clean. The `files`, `clones`,
`api` and `listings` ratchets held. `baselines 37e9250a` reports one
raise: `public-api/brokkr-store.txt` goes from 171 to 172 items, the
`SeatRecordVersion::V6` variant the row requires, which the operator
ruled on 2026-10-05; the PR body carries that `Ruling:` line.
`quality/file-lines.txt` was re-measured only for this unit's files:
`seat_record.rs` 220, `seat_record/tests.rs` 895, `boundary_tests.rs`
4,616, `capability_launch/legacy_journal.rs` 586 and `frozen_contracts.rs`
1,141; `contention_tests.rs`, `dialect_policy.rs` and the store's
`tests.rs` kept their counts. A force-warn `too_many_lines` clippy run over
the store and runtime moved one listed function, the unchanged 109-line
`the_new_contracts_exist_beside_the_frozen_ones`, from
`frozen_contracts.rs:175` to `:182`, and listed every `boundary_tests.rs`
function at its baseline line and length. No suppression was added. U4b
moves no prompt, dependency or transcript, so budgets were not
re-measured. Exact coverage and remote CI on Linux and macOS are pending.

### Follow-up

`docs/guides/journal-and-verification.md:172` names `seat-record.v5` as
the newest contract carrying `boundary`. That guide is not among this
row's documents, so it is left for a documentation follow-up to add v6
from the 0.12 line.
