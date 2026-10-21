# U8f — Expose retained evidence in the pure view

Tasks 43.1 and 43.2; requirements CC2 and CR5. Branch `s2/U8f`, cut from
main at 2095773e, run `0065-slice-two-unit-u8f-see-the--f576ba48`.

Status: 43.1 and 43.2 are met and ticked. Exact coverage and the macOS
legs are pending until CI runs them on the pull request.

## What was built

U4g's projection already carried the call's attribution and its optional
digest. U8f adds the shared display value of the retained response beside
the call's line, so the three readings the journal can hold are told apart
by one derivation in `brokkr-view` rather than by each surface:

| The checkpoint holds | `call` | `response` cell |
| --- | --- | --- |
| No whole v6 attribution group (a v5 record, an ordinary tool, a non-tool step, a partial or unknown group) | `null` | absent, note `capability attribution unrecorded` |
| An attributed call of any settled outcome with no `response_sha256` | the call, digest `null` | absent, note `no retained response is recorded for this call` (CR5's words) |
| A succeeded or failed call that cites a digest | the call with its digest | the digest |

The second note says only that no digest is recorded. It infers no realm
veto, no grant, no delivery and no model receipt, and a refused call stays
`refused`. The view reads no path, assembles no stage, merges no two rows
sharing a call id and looks up no grant. The provenance of a retained
response is the call's capability, dialect, tool and call id, which the
projection already carried; U8f reads it as such and adds no second
provenance type, so U8g's inspect can paint this one.

`CallEvidence` gains the public field `response` and the module the public
constant `NO_RETAINED`. Their consumers are `inspect --json` (through
`CheckpointRow`) and the CLI integration test, and U8g's no-digest refusal
uses the same words. Both are public API growth, recorded for the
operator's ruling at PR time.

The two LOW findings carried from U4g's council (PR #562; decision 0071
rulings 3 and 5) are fixed in `capability_calls.rs`. `CallState::word` is
now the only spelling of the five states. Serialization writes it through a
hand-written `Serialize`, because the layering scanner refuses the
`"&'static str"` string that `#[serde(into)]` needs. Parsing finds a state
by comparing against `word`. The attribution group is parsed once, by
`CapabilityCall::deserialize` over the checkpoint, and no field is read by a
string key any more. Each group field must be a nonempty string and the
state must be in the vocabulary, or the whole group fails. A digest that is
not a nonempty string reads as no digest and leaves the attribution intact.
The serialized form is unchanged: U4g's exact tests pass unedited, except
that the historical test's expected object gains the new `response` cell.

`lib.rs` was not edited, because its consumer, `CheckpointRow`'s
`capability_calls::call_evidence(checkpoint)`, already carries the new
field. No file was moved or created in production.

## Tests and their bindings

Each new test was run against a compiling mutation of production code,
failed at the assertion named, and passed again after the mutation was
restored. The commands were
`cargo test -p brokkr-view --lib tests::capability_calls::` and
`cargo test -p brokkr-cli --test it capability_artifacts::`.

| Test | Mutation | Failing assertion |
| --- | --- | --- |
| `tests::capability_calls::the_response_cell_tells_unrecorded_from_uncited_from_retained` | An attributed call's absent note becomes `UNRECORDED` | `assert_eq!(responses, …)`, tests/capability_calls.rs:259 |
| same | The digest is dropped from the cell | same, :259 |
| same | An unrecorded row's note becomes `NO_RETAINED` | same, :259 |
| `tests::capability_calls::the_call_evidence_serializes_exactly` | The digest is dropped from the cell | `assert_eq!(wire, …)`, :303 |
| same | An unrecorded row's note becomes `NO_RETAINED` | same, :303 |
| same | `CallState` serializes its Debug name instead of `word` | the five-state pin, :282 |
| `tests::capability_calls::a_wrongly_typed_field_fails_the_group_but_a_bad_digest_only_itself` | `word` stringifies a non-string field instead of failing | `assert_eq!(calls, …)`, :347 |
| same | `response_sha256` is deserialized as a plain `Option<String>` | same, :347 |
| `capability_artifacts::inspect_tells_a_retained_response_from_one_never_recorded` | An attributed call's absent note becomes `UNRECORDED` | `assert_eq!(responses, …)`, capability_artifacts.rs:178 |
| same | The digest is dropped from the cell | same, :178 |
| `capability_artifacts::inspect_leaves_a_historical_native_tool_unrecorded` (extended) | An unrecorded row's note becomes `NO_RETAINED` | `assert_eq!(evidence(…), …)`, :142 |

U4g's tests also bind the vocabulary refactor. Under the Debug-name
mutation, `every_recorded_call_state_reaches_its_checkpoint_row_exactly`
failed at :148, and `inspect_carries_every_recorded_call_state_exactly`
failed at capability_artifacts.rs:101. Under the lenient-digest mutation the
compiler warned that `digest` was unused, and the test still failed.

The CLI rows were planted through the store's append fence under engine
0.12.0, so the v6 contract admitted every one of them, the failed call's
digest included. No test moved. The view child suite grew from 199 to
355 lines; the owning `tests.rs` is untouched.

## Gates observed

On this tree on 2026-10-21, brokkr-view's suite passed with 359 lib and 6
integration tests. brokkr-cli passed its 944 lib tests and the 760 of its
`it` binary (after the one failure described below), and also its
`driver_conformance`, `transcript_surfaces` and three heap binaries.
brokkr-runtime's lib (1257) and `it` (137) suites passed too, although
runtime, bridge and store read none of the view's call evidence.
Formatting, workspace clippy under `-D warnings`, the strict OpenSpec
validation (31 items), typos, the whitespace check and the files and clones
ratchets all came back clean, and the self recipe compiled.

The first full run of the `it` binary failed one test,
`layering::scan::the_pure_crates_name_nothing_outside_the_allowlists`, on
the `#[serde(into = "&'static str")]` attribute. The hand-written
`Serialize` replaced it, and the layering suite (36 tests) and the whole
`it` binary then passed.

No witness, compose or prompt-bytes input moved, and no suppression was
added. Neither touched file is listed in `quality/file-lines.txt`.

The exact-coverage gate (`scripts/coverage-exact.sh`) and macOS were not
run here, and stay pending until CI runs them.
