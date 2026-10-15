## 1. The read

*Serves: decision 0061 ruling 1's evidence standard. Design: D2.*

- [x] 1.1 Read the four 0.2.0-rc.2 packages' READMEs and the typed declarations the rules cite, at `.forge/evidence/`, and record each file's SHA-256. Verify: the four README digests equal the commission's; D2's table lists every cited file with its digest.
- [x] 1.2 Record what the read did not reach, with the read that lifts each: the version-four event catalogue, the message and block definitions, idle compression, the overlay's `packChunks`, the 0.2 usage keys and assistant source. Verify: D2 (a) to (e).

## 2. The decoder

*Serves: "A compressed DSH session reads within the plain source's bounds". Design: D6.*

- [x] 2.1 Move the zstd decoder from `probe/observe.rs` into `crates/brokkr-protocol/src/zstd.rs` as `unpack(packed, cut, bound)`, returning `Unpacked { bytes, past_bound }` or a `thiserror` `Refused`; keep the probe's `unpacked` as a wrapper with its texts and kinds. Verify: `probe::tests::compressed` passes unchanged.
- [x] 2.2 A bound inside a frame keeps the bound's bytes and says there is more; a cut read ends at its last whole frame; a cut read still refuses a frame read whole that fails. Verify: `zstd::tests`.

## 3. Discovery and the body read

*Serves: "Discovery identifies one owned local file" and the compressed-source requirement. Design: D6.*

- [x] 3.1 `DSH_SESSION_FILES` becomes the four-name, newest-first table, each name with its `Encoding`; `AdmittedSource` carries the encoding. Verify: `local_transcript::v4_tests::both_version_four_names_read_into_the_same_turns`, `the_admitted_names_are_asked_newest_first`, `no_other_compressed_name_is_read`.
- [x] 3.2 A compressed header is read from at most twice the header cap of compressed input and admitted within the header cap of decompressed bytes; a refused frame is `HeaderCheck::Io`. Verify: `a_compressed_header_is_bounded_by_what_it_decompresses_to`, `a_compressed_candidate_whose_frame_fails_is_unreadable`.
- [x] 3.3 `read_snapshot` decompresses a compressed body within `SOURCE_CAP` in and out; a refused frame is the body-stage `unreadable`. Verify: `a_compressed_body_past_the_source_cap_truncates_as_a_plain_one`, `a_compressed_body_past_the_input_cap_truncates_at_its_last_whole_frame`, `a_body_frame_that_fails_is_unreadable_with_its_path`, `the_compressed_fixture_is_the_plain_one`.
- [x] 3.4 Move the three tests that used version 4 as the foreign case to version 5, assertions unchanged. Verify: `git diff` over `local_transcript/tests.rs` shows only those fixture bytes.

## 4. The projector

*Serves: "Discovery identifies one owned local file" (admission) and "DSH version four reads as version three, save its named conversions". Design: D3 to D5.*

- [x] 4.1 Move `DshVersion` into `transcript/dsh_versions.rs` and add `Four`; `dsh_header` admits `4` by value and signature. Verify: `dsh_versions::tests::the_version_four_header_admits_by_its_value_and_its_spelling`; the view's header matrix refuses `5` where it refused `4`.
- [x] 4.2 Version four's quiet vocabulary is version three's plus `developer/message`, a delta over `DSH_QUIET_THREE`; `dsh_row`'s pre-match covers `Three | Four`. Verify: `each_quiet_version_four_name_supplies_no_turn`, `a_namespaced_unknown_is_omitted_only_with_its_marker`.
- [x] 4.3 `message_blocks` and `result_blocks` dispatch on the version: the tool-role result, and the retired wrapper counted. Verify: `a_tool_role_result_maps_field_by_field`, `the_retired_wrapper_is_counted_never_a_result`, `producer_sources_change_no_projection`, `a_remapped_reference_log_projects_as_today`, `a_version_four_session_projects_its_turns_in_order`.
- [x] 4.4 `transcript.rs` does not grow and `project_dsh` and `dsh_row` do not lengthen. Verify: `wc -l` 3,353 against the 3,356 baseline.

## 5. The driver

*Serves: decision 0086 ruling 5. Design: D1, D7.*

- [x] 5.1 Prove the cause first: with the version-four name absent from the tail's list, a working version-four seat emits no `seat-turn` and its transcript row waits for the finishing row. Verify: `adapters::dsh_v4_tests::a_version_four_seat_journals_its_transcript_while_it_works` fails with steps `transcript`, `harness-started`, `deepseek-harness-session-finished` and exit 9.
- [x] 5.2 `DSH_TRANSCRIPT_NAMES` asks for `session.v4.jsonl` first; the census keeps `DSH_TRANSCRIPT`. Verify: the same test passes with `seat-turn` before the finishing row; `the_tail_finds_the_version_four_name_first`, `the_rejoin_census_does_not_admit_the_version_four_name`; `adapters.rs` within its 7,117 baseline.

## 6. End to end

*Serves: "A version-four session reaches every surface". Design: D8.*

- [x] 6.1 A plain and a compressed version-four session through CLI text and JSON, `--turn`, the TUI pane, both doors and the browser route. Verify: `transcript_surfaces::a_version_four_session_reaches_every_surface_plain_and_compressed`.

## 7. The decision and the docs

*Serves: house rule that a semantic change carries a proposed decision.*

- [x] 7.1 Write decision 0086, `Status: proposed`, its index row, `Amended by: 0086` on 0055 and 0061 and `Superseded in part by: 0086` on 0062. Verify: `decisions_index` passes.
- [x] 7.2 Name the version-four file in `docs/status.md`'s dsh row and `docs/guides/read-surfaces.md`. Verify: `grep -rn "session.v3" README.md ARCHITECTURE.md CONTRIBUTING.md docs/guides/` finds nothing to reconcile.

## 8. Gates and the archive

- [x] 8.1 Show every new test binds by a compiling removal of its behaviour, restored afterwards. Verify: the result's notes name each removal and the test it failed.
- [x] 8.2 `cargo fmt --all -- --check`, clippy `-D warnings`, `cargo test --workspace --all-features --locked`, and both bundle compiles. Verify: each exits clean.
- [x] 8.3 Fold the delta into `openspec/specs/transcript-reading/spec.md` and archive the change here. `openspec validate --all --strict`, `quality/ratchet.sh`, `cargo shear` and the exact-coverage gate run outside this seat's box and are recorded pending, never lowered. Verify: the living spec states the four-name set, `{0, 3, 4}` and the two added requirements; the archived delta is those bytes.
