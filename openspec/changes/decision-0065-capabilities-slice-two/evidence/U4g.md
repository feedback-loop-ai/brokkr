# U4g: the view derives call evidence once (tasks 19.1–19.2)

Run `0065-slice-two-unit-u4g-see-the--7a85b464` built U4g on branch `s2/U4g`
from main at `cd24ee5b`. It touched the row's two production files and no
other.

## What changed

| File | Change |
| --- | --- |
| `crates/brokkr-view/src/capability_calls.rs` (new, 123 lines) | The pure projection. `CallState` is seat-record v6's closed `call_state` vocabulary as an enum (observed, succeeded, failed, refused, interrupted); `CapabilityCall` carries capability, dialect, tool, call id, state and the retained response digest; `CallEvidence` pairs that structured value with its rendered `Cell`. A checkpoint yields a call only when it carries the whole group with a state in the vocabulary, and otherwise none, with the note `capability attribution unrecorded`. It reads one checkpoint `Value`: no stage grouping, no merge by call id, no grant, realm, clock, environment or I/O. |
| `crates/brokkr-view/src/lib.rs` (2,774 → 2,779; baseline 2,789) | Registers `pub mod capability_calls;`, adds `capability_call: CallEvidence` to `CheckpointRow`, fills it in `checkpoint_rows`, and bumps `VIEW_VERSION` to 16 because the wire gained a field. |

The rendered line is `<capability> via <dialect> · <tool> <state>`, followed by
` · response retained` only when the row carries `response_sha256`. A refused
row therefore reads `library-docs via cap-library-docs · admin refused`, with
no digest and no success word. A native observation reads `observed` and is
not given a completion. When a row has no digest, the view says nothing about
retention, so a missing field is never presented as a veto.

The real consumer is the existing view construction. `run_view` builds each
participant's checkpoints through `checkpoint_rows`, and `inspect --json`
serialises them. The CLI test reads the projection back through the binary.

## Tests and test moves

No test moved. The owning suite `crates/brokkr-view/src/tests.rs` sits at its
3,797-line baseline (3,796 → 3,797: one `mod capability_calls;` line; the
version-pin edit kept its three lines). Its new child module
`crates/brokkr-view/src/tests/capability_calls.rs` holds the view tests. The
new `crates/brokkr-cli/tests/capability_artifacts.rs` is registered as
`mod capability_artifacts;` in `crates/brokkr-cli/tests/it.rs`. Three wire
pins moved from 15 to 16 for the version bump: `the_wire_version_moves` in
`brokkr-view/src/tests.rs`, the fleet pin in `brokkr-view/src/fleet/tests.rs`
and the `/api/view` pin in `brokkr-cli/src/ui/tests.rs`.

The CLI fixtures go through `Store::append_next`. The 0.12.0 rows therefore
pass the seat-record v6 append fence, and the 0.10.0 rows pass v5. Each run
lives on a canonicalised `tempfile` root, and no test reads `.forge/` or needs
a provider.

## Binding mutations

Every mutation compiled, was run against the named test, and was restored.
The restored tree then passed: `cargo test -p brokkr-view` 269 passed, 3
ignored; `cargo test -p brokkr-cli --test it capability_artifacts::` 2 passed.

| Mutation (production) | Failing test | Assertion that failed |
| --- | --- | --- |
| `CallState::of` maps `"refused"` to `Succeeded` | view `every_recorded_call_state_reaches_its_checkpoint_row_exactly`; CLI `inspect_carries_every_recorded_call_state_exactly` | the exact five-call vector |
| `response_sha256` is always `None` | the same two | the succeeded call's digest |
| a v5 `WebSearch` tool with no capability is attributed `web-search`/`claude-native-search` by name | view `a_checkpoint_without_a_whole_recorded_group_reads_unrecorded`; CLI `inspect_leaves_a_historical_native_tool_unrecorded` | `call == None` on the `WebSearch` row; the exact unrecorded pair |
| `CallState::of` admits any other word as `Observed` | view `a_checkpoint_without_a_whole_recorded_group_reads_unrecorded` | `call == None` on the `started` row |
| a missing `call_id` defaults to empty | view `a_checkpoint_without_a_whole_recorded_group_reads_unrecorded` | `call == None` on the partial row: left was `Some(CapabilityCall { …, call_id: "", state: Observed, … })` |
| capability, dialect and call id default to empty, and a missing state defaults to `observed` | CLI `inspect_leaves_a_historical_native_tool_unrecorded` (rerun after the planter reshape) | the exact unrecorded pair |
| `checkpoint_rows` passes `Value::Null` instead of the checkpoint (consumer removed) | view `every_recorded_call_state_reaches_its_checkpoint_row_exactly`; CLI `inspect_carries_every_recorded_call_state_exactly` (rerun after the planter reshape) | the exact five-call vector |

## Gates observed

The commissioned format, clippy, `openspec validate --all --strict` (20
passed), `typos --hidden` and `git diff --check` gates were run and came back
clean. `quality/ratchet.sh files` printed `ratchet: file size holds`. The first
`quality/ratchet.sh clones` run found one new 10-line test clone between
`capability_artifacts.rs`'s planter and `brokkr-cli/src/muninn/tests.rs:48`.
The planter was rebuilt as an event list, and the rerun printed `ratchet:
duplication holds`.

These suites ran in this session and passed:

- `cargo test -p brokkr-view`: 269 passed and 3 ignored, plus 6 in a second
  target.
- `cargo test -p brokkr-cli --all-features --locked`:
  - `--lib`: 627 passed;
  - `--test it`: 463 passed, 2 ignored;
  - `driver_conformance`, `transcript_surfaces`, `heap_claude`, `heap_codex`
    and `heap_dsh`, plus the bins: all passed.
- `cargo test -p brokkr-runtime --all-features --locked --tests`: 831 lib
  tests and the integration binaries passed. brokkr-runtime depends on
  brokkr-view but reads none of what changed.

`cargo run --locked -p brokkr-cli -- compile --bundle bundles/self` compiled.

Still pending:

- `cargo test --workspace` as one invocation. It was run crate by crate
  instead; the core, store, protocol and bridge suites did not run, and
  none of them depends on brokkr-view.
- `scripts/coverage-exact.sh`. The new module's branches are all reached by
  the four tests above: every `CallState` arm, both `line` branches, and the
  absent path.
- Remote CI.

Witness and compose pins do not move, because no input of theirs changed.
`scripts/measure-budgets.sh` was not run. The run view's per-checkpoint work
gained one small projection, and the callgrind budget is advisory.

The re-vouch run `0065-slice-two-unit-u4g-see-the--49c23bb2` judged the
controller's snapshot of this whole diff, applied uncommitted on `cd24ee5b`
with `quality/public-api/brokkr-view.txt` regenerated (450 → 469 items under
the operator's 2026-10-06 ruling), and re-proved the coverage case the exact
gate refused at 6995/6996 branches, which the pending note above missed:
removing `capability_call`'s `if !value.is_empty()` guard fails
`a_checkpoint_without_a_whole_recorded_group_reads_unrecorded` on its
empty-`capability` row, and restoring it passes.
