# U8g — Open a cited artifact through inspect

Tasks 44.1 and 44.2; requirement CR5. Branch `s2/U8g`, cut from main at
fe0eedee. Run 1 (`0065-slice-two-unit-u8g-see-the--d7033440`, cut from
4455668b) built the verb and answered review findings F2–F7, then stopped on
F1. Prep replayed its signed work onto fe0eedee. Run 2
(`0065-slice-two-unit-u8g-see-the--176e80df`) answers F1 under the
operator's ruling of 2026-10-22.

Status: complete in this seat; 44.1 and 44.2 are ticked. Still pending, not
observed here: the exact-coverage gate, the coordination bind-alias test,
the macOS legs and the rest of remote CI.

## What was built

`brokkr inspect --run <run> --capability-call <call_id> [--repo <path>]`
opens the response a call cites. The CLI reads the selected run's journal and
the view `brokkr-view` derives from it (`CheckpointRow::capability_call`), so
the journal is the only place a call can come from. It finds the call by
`call_id` among the run's checkpoints and takes its `response_sha256`. It
resolves the run-local directory of the operated repository through the
existing `folder::run_local`, which wraps `places::run_local`, and hands that
directory and the digest to the runtime. The repository is the one `--repo`
names, or the workspace's own when the flag is absent. The CLI takes no
artifact path from the operator, checks no file itself, fetches nothing and
writes nothing.

Without `--json` it prints the call id, the view's call line and the view's
response cell, each through `render::Safe`, then a blank line and the
response. With `--json` it prints one line,
`{"call":<the view's CapabilityCall>,"response":<the verified bytes>}`, with
the bytes spliced in unparsed. They are checked to be JSON, never parsed and
written again (F2). The response is written exactly where stdout is not a
terminal: a pipe, a file, a script. On a terminal it is written as its text
with every control and reordering character removed (`render::Safe`, which
covers C1 and the bidi and zero-width marks), and its line is ended (F4).
Calling inspect without the new flags behaves exactly as before. clap refuses
`--capability-call` beside `--phase` or `--seat`, and refuses `--repo`
without `--capability-call` (`requires`).

The runtime entry follows the operator's ruling of 2026-10-21 ("A") and is
unchanged since run 1. `engine.rs` changes one line: `mod artifacts;`
becomes `pub mod artifacts;`, with no line added. `engine/artifacts.rs`
gains one public function, `retained(run_local, sha256) -> Result<Vec<u8>,
Unread>`, and one private helper, `opened`. `retained` refuses a name that is
not a digest before it opens anything. It then opens `artifact_root(run_local)`
without following a link as the last component, and judges it as this
user's own directory (`ADMITTED`). Next it opens the digests inside that
root, judged writable by no one else (`SHUT`). Finally it reads through the
existing `read`. If the root or the digests were never made, the answer is
`Missing`. Anything else that will not open, or fails its judgement, is
`Uncontained`. There is no second reader.

| Case | Refusal (operator text) | Home of the text |
| --- | --- | --- |
| The selected run records no call by that id, including a call only another run records | `capability call is not recorded in this run` | `readouts.rs` `CallRefused::Unrecorded` |
| The selected run records that id on two checkpoints | `capability call is recorded more than once in this run` | `CallRefused::RecordedTwice` |
| The call cites no digest | `no retained response is recorded for this call` | `CallRefused::NoRetained`, whose text is the view's `NO_RETAINED` |
| `--repo` names a repository with no place for run-local state | `no .brokkr/ folder from <repo> up to <repo>, and <repo> is not a recipe library (decision 0083); nothing was created: …` | `folder::run_local`, the runtime's `PlacesError::NoPlace` |
| No artifact at the digest, or no root or digests | `retained response artifact is missing` | `Unread::Missing` |
| The bytes are not their name, or exceed 8 MiB | `retained response artifact digest mismatch` | `Unread::Corrupt` |
| Not a digest, a link, not this user's own, writable by others, not one link | `retained artifact path is not a regular contained file` | `Unread::Uncontained` |
| `--json`, and the verified bytes are not JSON | `retained response artifact is not JSON` | `CallRefused::NotJson` |

## The review's findings, answered

**F1 (medium), answered in run 2 under the operator's ruling of 2026-10-22
("a").** The journal records no run's run-local directory, so `inspect`
gains `--repo <path>`, valid only with `--capability-call`. It is resolved
exactly as `brokkr ledger --repo` resolves it: `print_call` calls
`crate::folder::run_local(repo)`, the one function `ledger::write` calls, on
the path as given, so there is one resolution and `readouts.rs` derives no
state path itself. Without the flag, `inspect` passes the workspace, as run
1 built it. The flag names a repository, never an artifact: the digest is
still the one in the selected run's view, and the runtime reader still
verifies the bytes, so through `--repo` an artifact whose bytes are not its
name is still `Corrupt`. The new CLI test
`inspect_opens_a_call_in_the_repository_repo_names` binds all four of the
controller's cases. The journal lives in workspace A and the artifacts in
repository B, and the run is inspected from A. With `--repo B` it gets B's
exact text output. Without `--repo` it gets exactly `error: retained response
artifact is missing`, because A's run-local digests hold no such artifact.
`--repo` alone is refused by clap with `error: the following required
arguments were not provided:` / `  --capability-call <CALL_ID>`. A `--repo`
naming a bare repository (only `.git`) is refused with folder.rs's own
`NoPlace` text, exactly. A corrupt artifact in B is refused as `Unread::Corrupt`.

**F2 (low), fixed in run 1.** `--json` splices the verified bytes unparsed
(see above). The first test publishes a response spelled with `n` as `1.50`
and `text` as a word whose `é` is written as a JSON escape, which no
serializer writes again. It asserts the text after `,"response":` byte for
byte, and parses only the call prefix.

**F3 (low), fixed in run 1 in the house's form.** The CLI's refusals are a
typed enum, `CallRefused`. Its `Display` is the operator's text, written by
hand the way this crate's errors are (graph.rs `UnknownRealm`), because
`brokkr-cli` has no `thiserror` dependency and `Cargo.toml` is outside the
row. The call selection is a small function of its own, `call_of`.

**F4 (low, security), fixed in run 1.** A unit test, in the test-only child
`verbs/readouts/tests.rs`, writes the same bytes both ways: an escape, a C1
CSI, a right-to-left override, a directional isolate, a newline and a byte
that is not UTF-8. Off a terminal they come back exactly; on one, only the
text survives. The CLI suite's binary never writes to a terminal, so it
proves the exact path.

**F5 (info), fixed in run 1 and verified.** `UiArgs`, a readout struct with
no run/realms/db block, moved whole into the new private child
`cli_args/ui.rs` (standing rule of 2026-10-09), which made room in
`cli_args.rs` without creating the `SeatsArgs`/`WatchArgs` clone run 1's
first visit created. The controller ran `quality/ratchet.sh clones` and
`quality/ratchet.sh files` on the replayed tree over main fe0eedee on the
host and observed "ratchet: duplication holds" and "ratchet: file size
holds". In run 2 this seat ran both scripts itself on the final tree, with
`--repo` added, and got the same two lines.

**F6 (info), fixed in run 1.** The fixture's root carries its own `.git`
entry, so no repository above the temporary directory makes its `.brokkr`
nested. Run 1's control: with a TMPDIR inside the worktree and the `.git`
line removed, 3 of the then 6 tests failed with `… .brokkr is not at this
repository's root …`; restored, all passed.

**F7 (info), test added in run 1; the two comments fixed in run 2 under the
operator's ruling ("yes to the comments").** In `crates/brokkr-cli/src/lib.rs`,
inspect's doc comment now says `--json` emits the view model "or with
`--capability-call` the call and its verified response". This comment is the
verb's clap `about`, so `docs/reference/cli.md`'s two copies of that sentence
moved with it, regenerated by `the_cli_reference_is_rendered_from_clap`'s
`BROKKR_REGENERATE_CLI_REFERENCE` path (one line changed, one line added;
lib.rs goes from 1623 to 1624 lines). In `crates/brokkr-cli/src/folder.rs`,
`run_local`'s doc names its two callers, `brokkr ledger --repo` and `brokkr
inspect --capability-call`, in place of "Its one caller" (758 to 759 lines).
No non-comment line of either file changed.

**F8.** Nothing to do. The review rejected the panel's directions, and so do
I.

## Files

| File | Change | Lines (measured) |
| --- | --- | --- |
| `crates/brokkr-runtime/src/engine.rs` | `mod artifacts;` → `pub mod artifacts;`, no line added | 3512 → 3512 |
| `crates/brokkr-runtime/src/engine/artifacts.rs` | `Unread` public; `retained`, `opened` | 481 → 508 |
| `crates/brokkr-cli/src/cli_args.rs` | `capability_call` and `repo` on `InspectArgs`; `UiArgs` moved out, `mod ui;` and its re-export | 800 → 798 |
| `crates/brokkr-cli/src/cli_args/ui.rs` | NEW private child (standing rule of 2026-10-09: `cli_args.rs` sat at its 800 ceiling): `UiArgs` moved whole | 18 |
| `crates/brokkr-cli/src/verbs/readouts.rs` | `inspect` dispatches to `print_call` with `--repo` or the workspace; `CallRefused`, `call_of`, `shown`; `#[cfg(test)] mod tests;` | 386 → 500 |
| `crates/brokkr-cli/src/verbs/readouts/tests.rs` | NEW test-only child: the terminal test | 19 |
| `crates/brokkr-cli/src/lib.rs` | inspect's doc comment only (ruling 5) | 1623 → 1624 |
| `crates/brokkr-cli/src/folder.rs` | `run_local`'s doc comment only (ruling 5) | 758 → 759 |
| `docs/reference/cli.md` | the two flags' rows and inspect's about, as clap renders them | |
| `crates/brokkr-cli/src/tests.rs` | `capability_call: None` and `repo: None` in six existing `InspectArgs` literals | 4539 → 4551 (baseline 4593) |
| `crates/brokkr-cli/tests/capability_artifacts.rs` | four tests, their `Workspace` builder and its `open_with` | 194 → 464 |
| `crates/brokkr-runtime/src/engine/artifacts/tests.rs` | one test | 738 → 772 |
| `quality/file-lines.txt` | rows for each file above and the two new children | |
| `quality/public-api/brokkr-runtime.txt` | +6 lines | |

`brokkr-view` is untouched: no production change, no `VIEW_VERSION` bump,
no I/O. No test moved between files. Nothing under `contracts/`, `policy/`,
`fixtures/`, `reference/` or `extensions/` moved, and `engine/broker.rs`'s
`STARTUP` line is unchanged.

## Public API (operator's cap: +12 lines)

Run 1 measured with `cargo +nightly-2026-09-05 public-api -p <crate> -sss
--color never`, the body `api_file` writes (cargo-public-api 0.52.0).
brokkr-runtime gains six lines, and no other crate's snapshot moves. Run 2
added only crate-private items (`InspectArgs::repo` is `pub(super)`), and
`bash quality/ratchet.sh api` on the final tree printed "ratchet: public API
holds".

```text
pub mod brokkr_runtime::engine::artifacts
pub enum brokkr_runtime::engine::artifacts::Unread
pub brokkr_runtime::engine::artifacts::Unread::Corrupt
pub brokkr_runtime::engine::artifacts::Unread::Missing
pub brokkr_runtime::engine::artifacts::Unread::Uncontained
pub fn brokkr_runtime::engine::artifacts::retained(&std::path::Path, &str) -> core::result::Result<alloc::vec::Vec<u8>, brokkr_runtime::engine::artifacts::Unread>
```

## Tests and the mutations that bind them

| Test | Proves |
| --- | --- |
| `capability_artifacts::inspect_opens_exactly_the_response_a_call_cites_and_writes_nothing` | the text output byte for byte; the JSON envelope's call exactly and its response as the stored bytes, unparsed; the journal file's bytes and the digests directory unchanged after both reads; no server exists in the fixture to be asked |
| `capability_artifacts::inspect_opens_only_a_call_the_selected_run_records_once` | another run's call and an unknown id refused as `Unrecorded`, a duplicated id as `RecordedTwice`; healthy controls: each run opens its own call against the same sound artifact; clap refuses the selector beside `--phase` and `--seat` |
| `capability_artifacts::inspect_refuses_a_response_it_cannot_verify_with_its_cause` | no digest, missing, corrupt, a link at the digest path, and non-JSON under `--json`, each with its exact cause; healthy controls: the sound call opens, and the non-JSON artifact opens as text |
| `capability_artifacts::inspect_opens_a_call_in_the_repository_repo_names` | run 2, F1: B's exact bytes from workspace A with `--repo B`; a corrupt artifact in B refused as `Corrupt`; without `--repo`, exactly `Missing` from A; a bare `--repo` refused with folder.rs's `NoPlace` text; `--repo` without `--capability-call` refused by clap with its exact two lines |
| `verbs::readouts::tests::a_response_is_exact_off_a_terminal_and_cannot_steer_one` | the same bytes written exactly off a terminal, and as safe text with its line ended on one |
| `engine::artifacts::tests::a_run_local_directory_opens_only_the_artifact_its_root_holds` | healthy bytes; a path-like name refused as `Uncontained` before anything is opened (no root exists); no root and no digests `Missing`; a root that is a link and digests mode 0770 `Uncontained`; read's own `Missing` |

Each mutation below compiled, failed the named test at the stated assertion,
and was restored. M1–M9 and R1–R4 ran in run 1. M10–M14 ran in run 2, each
with `cargo test --locked -p brokkr-cli --test it capability_artifacts::`
(M14 filtered to the new test), and after each restore the suite passed
again, 7/7.

| # | Mutation | Failing test, assertion |
| --- | --- | --- |
| M1 | text mode drops the response write | `inspect_opens_exactly…` text tuple (left ended at the blank line); also `inspect_refuses…` (`ends_with("\n\nprose")` false) |
| M2 | the selection's filter ignores `call_id` | `inspect_opens_only…` (`recorded more than once` where `not recorded` was expected) |
| M3 | `call_of` answers the first of two (`map_or(Ok(found), \|_\| Ok(found))`) | `inspect_opens_only…`: `a1:b:2` printed its response where `RecordedTwice` was expected |
| M4 | a missing digest falls through as `""` | `inspect_refuses…` at `a1:b:2` (`Uncontained` where `NO_RETAINED` was expected) |
| M5 | the JSON check's error is dropped (`let _ = parsed.map_err(..)`) | `inspect_refuses…` at `a1:b:6`: an envelope with `"response":prose` printed where `not JSON` was expected |
| M6 | the envelope's `call` is only the id | `inspect_opens_exactly…` JSON assertion |
| M7 | `conflicts_with_all` removed from `capability_call` | `inspect_opens_only…`: the call opened beside `--phase` where clap's refusal was expected |
| M8 | `--json` parses the response and writes it again (`serde_json::Value` round trip) | `inspect_opens_exactly…` JSON assertion: `1.5` and a literal `é` where `1.50` and the escaped letter were expected |
| M9 | `shown` never takes its terminal arm (`match terminal && false`) | `a_response_is_exact…` terminal assertion: the raw bytes, escape, CSI and marks included, where the safe line was expected |
| M10 | `--repo` ignored (`repo.as_deref().map(\|_\| workspace).unwrap_or(workspace)`) | `inspect_opens_a_call_in_the_repository…` first assertion (:440): `("", "error: retained response artifact is missing\n", false)` where B's provenance and bytes were expected |
| M11 | without `--repo`, a repository other than the workspace (`unwrap_or(Path::new("/"))`) | `inspect_opens_a_call_in_the_repository…` at the no-`--repo` assertion (:444): the `NoPlace` text for `/` where `missing` was expected (the three earlier CLI tests failed too) |
| M12 | the folder's refusal swallowed (`folder::run_local(repo).unwrap_or_else(\|_\| repo.join(".brokkr/state"))`) | `inspect_opens_a_call_in_the_repository…` at the bare-repository assertion (:454): `missing` where the `NoPlace` text was expected |
| M13 | `requires = "capability_call"` removed from `repo` | `inspect_opens_a_call_in_the_repository…` at the clap assertion (:463): the run's whole readout printed and success where clap's refusal was expected |
| M14 | the reader's digest check skipped (`(named \|\| true).then_some(..)` in `read`) | `inspect_opens_a_call_in_the_repository…` at the corrupt assertion (:442): B's mismatched bytes served where `digest mismatch` was expected |
| R1 | `retained` drops the up-front digest check | runtime test at `("absent", "../true")` (`Missing` where `Uncontained` was expected) |
| R2 | `opened`'s missing arm never matches (`NOTSUP` for `NOENT`) | runtime test at `("absent", <digest>)` (`Uncontained` where `Missing` was expected) |
| R3 | `opened` judges no mode bits | runtime test at `("open", …)` (`Ok(bytes)` where `Uncontained` was expected) |
| R4 | `retained` maps every read error to `Missing` | `inspect_refuses…` at `a1:b:4` (`missing` where `digest mismatch` was expected) |

## Gates run in this seat (Linux, run 2, on the final working tree)

Formatting checks clean. Workspace clippy, with all targets and features,
locked and `-D warnings`, is clean after the last code edit; its first run
in this visit flagged one redundant closure in the new test, which was
removed. The brokkr-cli suites all passed: `--lib` 997 passed and 1 ignored
(its first run failed only `the_cli_reference_is_rendered_from_clap` until
the reference was regenerated); `--test it` 779 passed and 3 ignored; and
`driver_conformance`, `heap_claude`, `heap_codex`, `heap_dsh`, `heap_fleet`
and `transcript_surfaces` passed 27, 1, 1, 1, 2 and 14. brokkr-runtime's
code did not change in run 2; its `engine::artifacts` tests passed 14/14
here, and its other suites passed in run 1. `quality/ratchet.sh files`
printed "ratchet: file size holds", `quality/ratchet.sh clones` printed
"ratchet: duplication holds" and `quality/ratchet.sh api` printed "ratchet:
public API holds". `openspec validate --all --strict` passed 31 of 31.
`typos --hidden` is clean. `compile --bundle recipes/self` compiled, ending
with its policy sweep's "0 unruled". `git diff --cached --check` is clean.
No known load flake fired.

Pending, not observed here: the exact-coverage gate; the coordination
bind-alias test; macOS; and the rest of remote CI.

## Follow-ups (not fixed here)

None owed by this unit. The comment follow-ups run 1 named are fixed under
ruling 5.
