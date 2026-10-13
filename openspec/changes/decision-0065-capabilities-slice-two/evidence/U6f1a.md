# U6f1a evidence: mask output at the edge

U6f1a was first built by run `0065-slice-two-unit-u6f1a-see-th-824b002f`,
repaired by run `0065-slice-two-unit-u6f1a-see-th-f84245aa`, and repaired
by class and delivered by run `0065-slice-two-unit-u6f1a-see-th-30c5acb4`
on branch `s2/U6f1a`, on main at `ca6727f4`, as one signed commit that also
carries the controller's staged 2026-10-12 split in `design.md`,
`operator-ruling-2026-10-03.md` and `tasks.md`, with the operator's newer
U6f1b `entry.rs` ruling. It contributes to tasks 31.1 and 31.2, which close
with U6f1b (operator ruling 2026-10-12), so neither is ticked here. The
sections up to "Pending" record the first run's tree as its seat observed it
on Linux; the "Repair" section records the second run's; and the
"Principled repair" section at the end records the fourth run's tree, which
is the one delivered, and supersedes both wherever they differ.

## What changed

The incomplete-serving refusal stays exactly where it was: `broker.rs` and
`brokkr-protocol` are untouched, `serve` still ends with
`Refusal::ServingIncomplete`, and `served` in `rpc/calls.rs` keeps its one
`expect(dead_code)`. Everything this unit adds is reached through that
private call path and the startup the public `serve` already runs.

| File | Change |
| --- | --- |
| `crates/brokkr-cli/src/broker/output.rs` (new, 157 lines) | The output edge. `checked` reads a response frame exactly: `canonical::parse_strict` refuses a key twice at any depth, and every number written outside a string must read back, through `serde_json::Number`, as the same decimal value. `prepared` masks a reply with `secret::mask_json`, refuses a key masking makes collide with another (object members counted before and after), serializes the one `{"result":…}` or `{"error":…}` object once with `canonical::to_bytes`, then requires `secret::mask_bytes` to find nothing more in those bytes. `correlated` does the same for a response id. `enveloped` puts the harness's own id before the very prepared bytes. `Edge` is the parameter object that carries the bindings, the server's stderr and the diagnostics sink. |
| `crates/brokkr-cli/src/broker/rpc.rs` (631 to 649) | Declares `output` as a private child by `#[path = "output.rs"]`, since `broker.rs` is outside the row. `Server::heard` runs `output::checked` on every response, so startup answers (initialize, each listing page) are read exactly as well. `Catalog` now holds the listing as a value, masked once serving begins. `Lines` and `Line` are visible within `crate::broker` so stderr is read by the same bounded reader, and `Lines::rest` hands back the unterminated tail. `Reply` also derives `Deserialize` so the masked reply is re-read in its own shape. |
| `crates/brokkr-cli/src/broker/rpc/calls.rs` (720 to 741) | `served` and `served_within` take an `Edge`. The catalog is prepared at the edge before the harness's first frame is read. Each response's id is checked by `correlated` before it is matched, and its reply goes through `delivered`: its outcome read before masking (a non-MCP shape is still the protocol cause), `prepared`, and its outcome read again after masking, where any change is the masking cause. The delivered line is `enveloped` over the prepared bytes, and the old `Answer` serializer is gone. The server's stderr is drained every turn and its masked tail is written to the edge's diagnostics when the session ends. |
| `crates/brokkr-cli/src/broker/session.rs` (799 to 800) | One line: `pub(super) mod stderr;`. |
| `crates/brokkr-cli/src/broker/session/stderr.rs` (new, 94 lines) | `Drain`: the server's stderr read without blocking as lines, at most 64 KiB each (a longer one is passed over with a marker), each line masked by `secret::mask_bytes` on its raw bytes and only then decoded lossily, keeping a 64 KiB tail of whole masked lines. A last line without its newline is kept only after the pipe has ended. |

No production code moved between files, and no test moved. The new files
are named in the commit and have rows in `quality/file-lines.txt`. No
public API changed: every new item is `pub(super)` or
`pub(in crate::broker)`.

## How it reads the settled design

The design asks for a raw-byte overlap "large enough for each known
encoding". The broker cannot learn a bound value's length without the
plaintext accessor, which `machine_proof` holds to one call site. The drain
therefore takes its overlap from the store's own shape. A bound value is one
line of the decision 0012 store, and none of the shared encodings (raw,
base64 in four forms, hex, percent) can produce a newline from a value that
has none. So the bytes since the last newline are an overlap at least as long
as any needle that could straddle a read. A production guard for that
invariant would be unreachable, so it is left out, and
`a_bound_value_never_holds_the_newline_the_drain_splits_at` is its removal
control. That test pins the store's single-line refusal, shows that a
hand-written value running past its line binds nothing, and checks that
every shared encoding of all 255 non-newline bytes is free of newlines.

Production computes no digest in this unit. CR2's digest is
`canonical::sha256_bytes` over the prepared bytes, and its consumer is U8b's
staging. A digest computed now would be an unused value (decision 0071
ruling 6). The test pins the prepared bytes exactly and pins their SHA-256
by `canonical::sha256_bytes`. An independent `sha256sum` of the same bytes
gave the same digest, `780367ea…6d43`.

"Retention on, off and veto": the plan carries the effective
`retained`, and SC2's veto reaches the broker as `retained: false`, so the
canary and control tests run with both values of the plan's field. No
staging exists before U8b. The Terminal record has no digest field, and
nothing of the refused response is held anywhere but the dropped value.

The broker holds no bound secret today, because the handoff thread resolves
the values. `served` has no production caller until U6f1b, so the bindings
the edge masks with are U6f1b's to supply. They must come without a second
plaintext accessor, since the masker reads values through private field
access.

## Tests

All tests are in a new private child of the existing RPC test seam,
`crates/brokkr-cli/src/broker/rpc/tests/masking.rs` (594 lines), plus a new
machine-proof child, `crates/brokkr-cli/tests/machine_proof/output_edge.rs`
(85 lines). The latter is registered from `machine_proof.rs`, whose comment
over the single-egress test was reflowed by one line, so the file stays at
its 3,451-line baseline. `rpc/tests.rs` (1,892 to 1,946) gained the
edge-aware builder `edged`, with `Beside` for the plan, bindings and stderr.
`bounded` is now a thin wrapper over it. It also gained `Ran.ledger`,
`Ran.diagnostics` and `Harness::line`, the raw bytes `next` parses. Bound
values come from a store on a canonicalised temporary root. No test reads
`.forge/`, needs an installed provider or changes the process environment.
`capability_broker.rs` needed no change: its public incomplete-serving
regression still holds unchanged.

Each test was bound by a compiling mutation that removed one behaviour. The
mutation was run, the failing assertion recorded, and the behaviour restored.
`cargo test -p brokkr-cli --lib broker::` then passed 96 tests, and
`--test it machine_proof::` passed 64.

| Mutation | Removed behaviour | Failing test and assertion |
| --- | --- | --- |
| M1 | `prepared`'s check that `mask_bytes` finds nothing in the canonical bytes (scalar and structural) | `a_digits_only_canary_as_a_bare_number_fails_its_forwarded_call_unmasked`: the harness's answer was the canary's result, not `told("1", "MCP response cannot be safely masked")` (line 224 then); `the_catalog_is_masked_…` also failed. The precision test still passed. |
| M2 | `checked`'s numeric check (`all(exact)` replaced by `all(\|_\| true)`) | `a_number_read_as_another_value_is_the_servers_protocol_fault`: `("{\"n\":1.00000000000000000001}", Ok(()))` against `Err(Failed(Protocol))`. The canary test still passed. |
| M3 | `checked`'s `parse_strict` | `a_key_written_twice_at_any_depth_is_the_servers_protocol_fault`: `("{\"a\":1,\"a\":1}", Ok(()))` against `Err(Failed(Protocol))` |
| M4 | the member count before and after masking | `a_key_masking_makes_collide_with_another_is_the_servers_protocol_fault`: `None` against `Some(Failed(Protocol))` |
| M5 | the `mask_json` call | `every_shared_encoding_is_masked_into_one_canonical_buffer_whatever_the_id`, `the_canary_as_text_is_masked_…`, `the_catalog_is_masked_…`, `a_key_masking_makes_collide_…`, and `machine_proof::output_edge::the_output_edge_masks_with_the_shared_masker_and_serializes_once` (`[]` against `[("brokkr-cli/src/broker/output.rs", "prepared")]`). With the masker gone, the residual check refused every secret-bearing reply, so the change failed closed. |
| M6 | `answered`'s `correlated` call | `a_response_id_holding_a_bound_value_ends_the_session_masked_and_is_copied_nowhere`: case `number` told "MCP server protocol is invalid" instead of the masking cause |
| M7 | the catalog prepared with `&[]` instead of the edge's bindings | `the_catalog_is_masked_before_the_harness_hears_it_or_ends_the_session_unheard`: the listing carried `uses tok"é-4e1d` |
| M8 | `delivered`'s check that masking did not change what the reply says | `masking_that_changes_what_a_reply_says_cannot_be_safely_masked`: `Ok(Failed)` against `Err(Failed(Masking))` |
| M9 | the stderr overlap (each partial read masked as it arrived) | `standard_error_is_masked_across_reads_and_decoded_only_once_whole`: the tail held `raw tok"�\n�-4e1d` and the split hex and canary unmasked. It failed again on the revised version with the split word. |
| M10 | the "pipe has ended" guard on the last line | `a_last_line_is_kept_only_once_nothing_can_complete_it`: `"first\npin 8765\n"` against `"first\n"` |
| M11 | the tail bound | `a_line_past_its_bound_is_passed_over_and_the_tail_keeps_its_bound`: the 70-line tail against the last 64 |
| M12 | the diagnostics write | `a_digits_only_canary_…`: diagnostics `""` against `"starting [secret:DOCS_PIN]\nlast [secret:DOCS_TOKEN]\n"` |
| M13 | `heard`'s call of `checked` | `a_number_read_as_another_value_…` and `a_key_written_twice_…` (served case answered instead of failing), and `a_servers_startup_answers_are_read_exactly_too` (`Ok([...])` against `Err(Failed(Protocol))`) |
| M14 | the store's single-line refusal, in `brokkr-protocol/src/secret.rs` (a temporary local mutation outside the row, restored, never committed) | `a_bound_value_never_holds_the_newline_the_drain_splits_at`: `Ok(None)` against the exact single-line refusal |
| M15 | a planted `fn enc_` in `session/stderr.rs` | `machine_proof::output_edge::no_broker_file_encodes_a_secret_of_its_own`: `[("brokkr-cli/src/broker/session/stderr.rs", "fn enc_")]` against `[]` |

M1 and M2 are the independent removals the unit asks for: removing the
scalar check failed only the canary's assertions, and removing the numeric
check failed only the precision test.

The leak scans read every surface named in the scenarios. Each one looks for
every `NEEDLE_ENCODINGS` form and the JSON string escaping of both bound
values. The surfaces are the raw bytes the harness received, the ledger's
bytes, the diagnostics, and the refusal's own text. The encodings test splits
the server's response line inside the two-byte character of
`tok"é-4e1d`. The stderr test splits inside that character, inside the hex
needle, inside the canary and inside another word's two-byte character.
Every scan found nothing.

## Pipe capacity

With every RPC test pipe reduced to 16 KiB, all 49 tests under
`broker::rpc::` on the final tree passed in 13.9 s, the 14 masking tests
included. An earlier run, before the startup test existed, passed 48. The reduction was a local, uncommitted
`F_SETPIPE_SZ` (rustix `fcntl_setpipe_size`, its `pipe` feature enabled
temporarily in `crates/brokkr-cli/Cargo.toml`), and `pipe()` asserted
`fcntl_getpipe_size == 16384`. Both edits were reverted before the commit,
and `git diff` showed neither. This is evidence about pipe capacity on
Linux, not a macOS run.

## Gates run

| Gate | Result |
| --- | --- |
| Format, and workspace clippy with all targets and features under `-D warnings` | Both clean on the final tree. |
| `cargo test -p brokkr-cli` | lib 738 passed (1 ignored), `it` 594 passed (2 ignored), and `driver_conformance`, `transcript_surfaces`, `heap_claude`, `heap_codex` and `heap_dsh` passed. The masking tests were added to lib after; `--lib broker::` passed 96 on the final tree. On the final tree, `it` passed 593 of 594, and the failure is environmental. In `witness_journal::a_conclusions_gaps_reach_stderr`, the hands reaper printed `hands: reaped /tmp/brokkr-hands-serve-<pid>-…: its owner is dead and holds no lock` for a dead process's directory in the shared `/tmp`, which broke the test's stderr prefix. Rerun alone six times, it failed only on the one run that reaped such a directory and passed the other five. |
| Other crates | core, view, store and bridge (15 result lines, all ok), protocol and seatbelt-probe ok, runtime lib 930, runtime `it` 131, `capability_launch` 75, `operated_repo` 1, `queued_launch` 3, doctests ok. |
| Witness pins | `witness_digests` held without blessing; no input moved. |
| `recipes/self` | Compiles; the policy sweep reports 47 valuations over 11 groups, 0 unruled. |
| `openspec validate --all --strict` | 19 passed, 0 failed. |
| `typos --hidden`, `git diff --check` (staged and unstaged) | Clean. |
| `quality/ratchet.sh files` and `clones` | "file size holds" and "duplication holds". |
| File lines | Re-measured with `wc -l` and written to `quality/file-lines.txt` for every touched and new file. |
| Long functions and suppressions | No function crosses 100 lines and no suppression was added. The `machine_proof.rs` entries in `too-many-lines.txt` sit above the reflowed comment, so their lines are unchanged. |
| `scripts/measure-budgets.sh` | No budget moved because of this unit. The script rewrote `prompt-bytes.json`'s note and 62 of its budget rows (every one shown, lower), the dates in all three notes, and the crate-count note's history; the measured heap peaks were unchanged. Every one of those comes from drift already on main, so they were restored and not committed. |
| Coverage diagnostic, scoped | `cargo +nightly-2026-09-05 llvm-cov -p brokkr-cli --lib --branch -- broker::` showed no unhit line or branch in `output.rs`, `session/stderr.rs` or `rpc/calls.rs`. The only zeros in `rpc.rs` were the existing `initialized` (565–574), which the CLI integration suite covers. This is a diagnostic, not the gate. |

## Pending

Still pending: exact coverage by `scripts/coverage-exact.sh` on a capable
host or in CI, the macOS build and tests, and remote CI on the final head.
U6f1b lifts the refusal, supplies the session's bindings, wires the launched
server's stderr (the launcher still gives it `Stdio::null()`, in
`brokkr-protocol`), repeats these proofs through the public session, and
then closes tasks 31.1 and 31.2.

## Repair (operator ruling 2026-10-12)

The first run's review put it on security hold with three high findings, a
medium and a low (H1, H2, H3, M1, L1). The operator ruled all five repaired
in this unit, the incomplete-serving refusal kept, and `broker.rs` and
`brokkr-protocol` untouched. Neither moved: `git diff` names only the
row's files, their existing private children, the two owning test children,
two pinned lines in `rpc/tests.rs`, one grep proof in
`tests/machine_proof/output_edge.rs`, this change's records and
`quality/file-lines.txt`. No file was added and no code moved between
files; `Answered`, the tools/call result shape, moved from `rpc/calls.rs`
into `output.rs`, where it is now the projection masking must keep.

### What changed

| Finding | Repair |
| --- | --- |
| H1, a bound number kept by its value in another spelling | `prepared` now also requires `valueless`: every number in the prepared bytes, by its decimal value, is spelled plainly (after each count of up to eight leading zeros, with eight zeros past its point) and with an exponent (point before, after the first, or after the last significant digit; `e` or `E`; a positive exponent's sign written or not), each after its sign, and the shared masker reads them one per line. A bound value found in any spelling is MB4's cause. The spellings are padded with zeros only where no bound value is made of padding alone, so a bound `0000` does not refuse every number. The precision check (`checked`) and the residual `mask_bytes` scan of the prepared bytes are unchanged and separate. The fourth run's correction: this repaired instances, not the class. The eight-zero limit and the padding exception are gone, with `valueless`, `spellings`, `plain` and `PADDING`; a number is now compared by value in `secret::bound_by_value`. |
| H2, the harness's own id echoed unchecked | Every frame the harness is told is made by `Session::framed`, which checks the id it carries with `output::correlated` (a numeric id by its value, a text id as text) before `output::enveloped` writes it. Local replies, refusals, the listing, the forwarded answer, the unrecorded errors and `failed()` all pass through it. The forwarded answer's id is checked before its Terminal is recorded, so the call fails. A broker reply whose id is unsafe ends the session with the masking cause and no frame; a closing frame (`failed()`, an unrecorded Terminal) is withheld, and the cause already ending the session stays the first. The broker's own replies are now `Reply` values enveloped like the server's, so their bytes read `{"jsonrpc":"2.0","id":…,…}`. |
| H3, masking that changes what a reply is | `prepared::<S>` reads the reply before and after masking as its typed projection `S` and requires them equal. For a tools/call result (`Answered`): every member's key, each content block's `type` and the shape of its members at any depth (text values may be redacted), `isError`, and the keys of `_meta`; structured content's own keys and values stay the server's data. For the catalog (`Listing`): each tool's name and the shape of its members, so the catalog masked still offers exactly the tools the grant offered. A JSON-RPC error is read whole by `Fault`. Anything else is MB4's cause. The old re-read of the outcome after masking is folded into this check. |
| H3's low, `Catalog` carrying a `Value` | `Catalog` now holds `Vec<Tool>`, each tool its typed name beside the server's description as a map. The fourth run's correction: this does not repair the low. The description is still an untyped `Map<String, Value>`, which the second review named F6, and the principled repair carries it as a low. |
| M1, an error's explicit null data | `Fault::data` is read through `present`: absent stays `None` and is not written, and `null` is `Some(Value::Null)` and written as `"data":null`. |
| L1, stderr lines left at shutdown | `Drain::tail` drains every whole line the pipe still holds before taking the unterminated rest, within a bound of 1 MiB of the pipe's bytes (`CLOSING_MAX`, 16 tails), through `tail_within`, the seam tests bound tighter. An unfinished line of a live pipe is still never kept. |

`Server::heard` was left at cyclomatic complexity 16 by the first run's
added `if`; with no baseline entry its ceiling is 15. The exact read now
runs in the two response arms instead, and `cargo crap --path
crates/brokkr-cli/src` measured 14. The new functions measured `spellings`
10, `Drain::turn` 6, `prepared` and `Shape::from` 5, `valueless` and
`correlated` 4, `projected`, `plain`, `Drain::tail_within` and
`Session::tell` 3, `Session::framed` and `Session::told` 2.

Files now measure `output.rs` 342 lines, `rpc.rs` 650, `rpc/calls.rs` 758,
`session/stderr.rs` 124, `rpc/tests/masking.rs` 875 and `rpc/tests.rs`
1,946 (unchanged); `session.rs` stays at 800 and was not edited.
`quality/file-lines.txt` carries those counts.

### Tests and removals

Six tests were added to `rpc/tests/masking.rs`, which now holds twenty. The
canary test, `answered_as` and the catalog test now share helpers with the
new tests (`masking_failed`, `served_as`, `unlisted`), so the clone gate
holds. In `rpc/tests.rs`, the startup helper reads the typed
catalog, and the untaken-ping test pins the broker's envelope bytes in their
new order. `machine_proof::output_edge` now names the masking function
`prepared<S>`, as the grep reads its generic signature.

| Test | What it asserts exactly |
| --- | --- |
| `a_bound_number_is_refused_by_its_value_however_either_side_spells_it` | `Err(Failed(Masking))` for the review's three (`100000000000000000000`, `1200000000000000000000`, and `12345678901234568` returned as `12345678901234568.0`) and for bound spellings `1e+20`, `1.2E21`, `8.7654321e8`, `876543210.0`, `0876543210`, `-0876543210` and `5e-3`; the exact prepared bytes for the canary as text beside `876543211`, `87654321`, `12.5`, `-42`, `0`, `0.005` and `u64::MAX`, and for `7` under a bound `0000`; and `Err(Failed(Masking))` for `[3,1,2.5]` under a bound `3,1,2.5`, which only the residual scan finds. |
| `a_bound_number_spelled_otherwise_fails_its_forwarded_call_unmasked` | Served, retained and not: the harness is told MB4's cause for call `"1"`, the session ends `Failed(Masking)` with the call failed, and neither the answer nor the ledger holds the bound, written or serialized spelling. |
| `a_harness_id_holding_a_bound_value_is_answered_on_no_path` | With the canary in the harness's id (`876543210` or `"r-876543210"`): a call answered under the broker's id 3, an ungranted call, an invalid call, a ping, an initialize at MB3's version and another, a listing and an unadmitted method each end `Failed(Masking)` with no frame, the forwarded call failed and the ungranted one kept refused, and the ledger and diagnostics pass the leak scan; the control id `87654321` is answered with the exact bytes `{"jsonrpc":"2.0","id":87654321,"result":{}}`. |
| `masking_that_renames_a_protocol_member_or_a_tool_cannot_be_safely_masked` | `Err(Failed(Masking))` where a bound value is `text` (a block's type and key), `audience` (a block's annotation key) or `structuredContent`; the exact bytes where such a word is text or a structured-content key, and where an annotated block holds nothing bound; and a bound `lookup` ends the session `Failed(Masking)` before the harness is read, with no call. |
| `an_errors_data_keeps_an_explicit_null_apart_from_its_absence` | Prepared bytes `{"error":{"code":-1,"message":"no"}}` and `{"error":{"code":-1,"data":null,"message":"no"}}`, and the delivered lines with `"id":"1"` before each, the call failed. |
| `every_whole_line_a_pipe_still_holds_is_kept_when_the_session_ends` | After one turn, `"first\nlast\n"` from a closed pipe and from a live one holding `hal` unfinished; `tail_within(4)` of `a\nb\nc\nd\ne` keeps `"a\nb\nc\n"`. |

Each removal below compiled, was run with `cargo test -p brokkr-cli --lib
broker::` (or its `masking` filter), failed as recorded, and was restored.
R1 and R8 were run again after the helper folding, with the same failures.

| Removal | Failing test and assertion |
| --- | --- |
| R1: `prepared`'s `valueless` check deleted | The value test: `("100000000000000000000", "100000000000000000000", Ok(…))` against `Err(Failed(Masking))`; the served test: the harness was answered with `"n": 1e+20` instead of the masking cause. The precision tests and the canary's bare-number test still passed. |
| R2: precision removed (`all(exact)` read `all(\|_\| true)`) | `a_number_read_as_another_value_is_the_servers_protocol_fault`: `("{\"n\":1.00000000000000000001}", Ok(()))` against `Err(Failed(Protocol))`; `a_servers_startup_answers_are_read_exactly_too` too. Both H1 tests passed. |
| R3: the residual `mask_bytes` scan of the prepared bytes deleted | The value test's residual case: `Ok(…)` against `Err(Failed(Masking))`. |
| R4: the padding guard forced on | The value test's zeros control: `Err(Failed(Masking))` against the exact bytes holding `"n":7`. |
| R5: `framed` checked no id (`correlated(None, …)`) | `a_harness_id_…`: the harness read a frame carrying `"id":876543210` where it expected none (case `call`). |
| R6: the projection check made always true | `masking_that_renames_…`: `("text", Ok(…))` against `("text", Err(Failed(Masking)))`; `masking_that_changes_what_a_reply_says_…`: `Ok(Failed)` against `Err(Failed(Masking))`. |
| R7: the catalog prepared as `IgnoredAny`, no projection | `masking_that_renames_…`'s catalog case: the session ended `Ok(())`, its closure `Refused(ServingIncomplete)`, instead of `Err(Failed(Masking))` with the closure `Failed(Masking)`. |
| R8: `Fault::data` read without `present` | `an_errors_data_…`: `{"error":{"code":-1,"message":"no"}}` against the bytes with `"data":null`. |
| R9: no drain at shutdown | `every_whole_line_…`: `(true, "first\n")` against `(true, "first\nlast\n")`, the review's reproduction. |
| R10: the shutdown bound ignored | `every_whole_line_…`: `"a\nb\nc\nd\ne\n"` against `"a\nb\nc\n"`. |
| R11: `heard`'s exact read skipped, on its new shape | `a_servers_startup_answers_are_read_exactly_too`, `a_key_written_twice_at_any_depth_…` and `a_number_read_as_another_value_…` (the carried M13, run again). |

On the repaired tree the residual scan is bound by R3, not by the canary
test the first run's M1 named: the value check now refuses the canary's
bare number too. The value removal (R1) and the precision removal (R2)
failed separate tests. The first run's M8 (the outcome re-read after
masking) no longer exists; R6 binds what replaced it.

### Pipe capacity and gates

With every RPC test pipe reduced to 16 KiB, as in U6e's procedure (rustix's
`pipe` feature added to `crates/brokkr-cli/Cargo.toml` and `pipe()` setting
and asserting 16,384 with `fcntl_setpipe_size` and `fcntl_getpipe_size`),
`cargo test -p brokkr-cli --lib broker::rpc` passed all 55 tests in 13.53 s.
Both edits were reverted, and `git diff` of `Cargo.toml` is empty.

The second run's gates, on the final tree:

| Gate | Observed |
| --- | --- |
| brokkr-cli | lib 745 passed (1 ignored), including 102 under `broker::`; `it` 594 passed (2 ignored), including 64 under `machine_proof::` and 74 under `capability_broker::`; `driver_conformance` 27, `transcript_surfaces` 13 and each `heap_*` 1. No other crate was touched. |
| Toolchains and lints | `cargo +1.88 check --workspace --all-targets --all-features` finished; format check and workspace clippy (all targets and features, locked, `-D warnings`) reported nothing. |
| Ratchets against `origin/main` | "public API holds", "file size holds", "duplication holds" (after the three test clones above were folded) and "no baseline raised since origin/main". |
| Pins and bundle | `witness_digests::` 6 passed without blessing; `recipes/self` compiled, 47 valuations over 11 groups, 0 unruled. |
| Records | `openspec validate --all --strict` 19 passed; `typos --hidden` and `git diff --check` reported nothing. |
| Coverage diagnostic | `cargo +nightly-2026-09-05 llvm-cov -p brokkr-cli --lib --branch -- broker::` left no unhit line or branch in `output.rs`, `rpc/calls.rs` or `session/stderr.rs`; in `rpc.rs` only `initialized` (568–577), which the CLI integration suite covers. A diagnostic, not the gate. |

Recorded, not repaired here: L2 is the operator's settled handoff (the
server's stderr is still `Stdio::null()` in
`crates/brokkr-protocol/src/hands/namespace/entry.rs`, which U6f1b's row now
names), and P1 needs no action. A refusal that names an ungranted tool
still echoes the harness's tool name in its message; masking that text is
outside the five findings and is left as a follow-up. The offline link
check (`lychee`) was not run: this seat's sandbox refuses the command, so
it is pending with CI. Still pending: exact
coverage by `scripts/coverage-exact.sh` on a capable host, the real-box
tests at two CPUs on the host, the macOS build and tests, and remote CI on
the final head.

## Principled repair (operator ruling 2026-10-12, fourth run)

The second run's review (gpt-6-astra, xhigh) held it again. Each instance
fix had been bypassed by another instance of its class, and two new classes
were found: F1 numbers, F2 and F5 ids, F3 structure, F4 raw names in
evidence, F6 the untyped catalog, and F7 stderr decoded after masking. The
operator ruled a repair that closes each class (2026-10-12). The
incomplete-serving refusal stays, `broker.rs` and `hands/namespace/entry.rs`
are untouched, and `brokkr-protocol` moves only by one function in
`secret.rs`, which the ruling added to this row. The second run's M1
(absent versus null error data) and L1 (bounded shutdown drain) repairs were
confirmed by that review and are kept unchanged, with their tests.

### The one final guard

`output::guarded(bytes, bindings)` in `crates/brokkr-cli/src/broker/output.rs`
takes the exact bytes about to leave and refuses with MB4's masking cause
("MCP response cannot be safely masked") unless three things hold of them.
First, the shared masker (`secret::mask_bytes`) finds nothing in those
bytes. Second, it finds nothing in them with every JSON escape decoded
(`unescaped`). That is one pass from the first byte: a `\u` escape's UTF-16
units are read together, a lone surrogate reads as U+FFFD, and `\/`, `\"`,
`\\` and the controls read as what they name. For JSON this is every
string, keys and values, decoded, with the structure between them. Third,
no number they write is a bound value by what it is worth
(`secret::bound_by_value`). For JSON a number is one written outside its
strings. For text, such as a stderr line, any number written in it counts,
raw or decoded. Nothing is transformed after the guard: what it passes is
what is written.

| Output | Where it passes the guard |
| --- | --- |
| Every frame the harness is told | `output::framed`, on the whole enveloped line with its id and newline, immediately before it is queued. This covers forwarded answers, the broker's own replies (initialize, ping, refusals, the listing), the unrecorded-Terminal error and `failed()`'s closing frame. Local replies are now prepared (`prepared`, `Kind::Local`) like the server's. |
| The catalog | `served_within`, on the prepared listing, before the harness's first frame is read. |
| The harness's id | `Session::requested`, through `output::correlated` (its serialized bytes and its decoded value), before anything is forwarded, recorded or answered. An unsafe id ends the session through `failed(MASKING)`, so the active call, if any, fails with that cause and no path can report another. |
| The server's response id | `Session::answered`, through `correlated`, before it is matched. It is never copied anywhere. |
| A tool name, into the ledger and refusal text | `Session::call`, through `output::named`, before `ledger.begin`. |
| A stderr line | `Drain::keep`, on the exact bytes kept. The tail is written line by line as those bytes. |

The prepared result or error buffer is one run of the guarded frame's bytes,
so it holds nothing the frame does not. U8b retains that buffer.

### Each class, and what closes it

| Finding | Repair | Bound by |
| --- | --- | --- |
| F1, a number equal by value | One public function in `crates/brokkr-protocol/src/secret.rs`, `bound_by_value(number, bindings) -> bool`. It reads the token and each binding whose whole plaintext is a decimal number as an exact magnitude: significant digits with no leading or trailing zero, and a base-10 power held in an `i128`. It uses no float, and every zero, `-0` included, is the one zero. Each binding is compared on its own, with no global padding switch. The comparison reads `Secret`'s private bytes inside the module, never through `expose_for_spawn`, and returns only the bool (decision 0012, stated in its doc comment). `spellings`, `plain`, `PADDING` and `valueless` are deleted. The cost is numbers × bindings comparisons, each linear in the token, within a response MB3 bounds at 8 MiB. | R2, R12 |
| F2 and F5, ids | The harness's id is guarded as it arrives, both as its JSON serialization and as its decoded value, and the frame guard covers any id a frame carries. | R5, R1 |
| F3, structure | `prepared` masks only the positions in one table, `REDACTABLE`. These are a text block's text; the title and description of a block, a tool, a tool's annotations and any string title or description at any depth of a tool's schemas; `structuredContent`, keys and values; and an error's message and data. Every other position is never passed to the masker, so it comes out byte-identical by construction, and a bound value there is refused by the guard. That covers a tool's name, a schema's type, property names, required and enum, a block's type, mimeType and uri, annotations other than a title, `isError` and `_meta`. A key that masking makes collide inside `structuredContent` or `error.data` is still the protocol cause. A tool's name is never masked, so the catalog offers, and is looked up by, the very names the grant named. A check of that after masking would be unreachable, so it is left out, and `a_bound_value_in_a_fixed_position_is_refused_and_never_rewritten` is its removal control (named in `REDACTABLE`'s comment). The `Shape`, `Opaque`, `Listed` and `Listing` projections are deleted, and `Answered` is back to main's form. | R6, R11 |
| F4, raw names in evidence | `named` returns `Safe`, `Masked(name)` or `Unsafe`. A tool name the guard refuses is recorded under its masked name, where that name passes the guard. Its Terminal is `Refused(Ungranted)`, since a name holding a bound value can never be granted: granted names pass the catalog guard. The harness is told the masking cause, and the session ends `Failed(Masking)`. Where even the masked name is unsafe, as with a number by value, nothing of the call is recorded or told. | R7 |
| F6, `Tool.described` untyped | Carried as a low under decision 0071 ruling 3, **not repaired**. The tool is still its typed name beside a `Map<String, Value>`. Typing MCP's tool shape closed would refuse members servers send, or drop them, and a schema is an open JSON Schema document. That behaviour change is outside this unit. Masking now reads the map only through the `REDACTABLE` position table. | none |
| F7, stderr decoded after masking | Each whole line is masked raw, decoded lossily, masked again as the decoded text, and kept only where the guard passes those exact bytes. Otherwise its marker `[MCP server stderr line cannot be safely masked]` is kept, or, where the marker is unsafe too, nothing is. The raw-byte overlap across reads (whole lines) and the bounded shutdown drain are unchanged. | R8, R9 |

Two more changes follow from the guard. Every frame now ends in the newline
`framed` writes, so the response bound is checked against `FRAME_MAX`, one
byte past MB3's 8 MiB. `Reply` no longer derives `Deserialize`, which only a
test used.

### Assumptions, named

The task asks whether a number "equals" a bound value. This run compares
magnitudes, so `-876543210` is refused under a bound `876543210`: a value
and its negation disclose the same digits. That refuses strictly more than
signed equality. A number written as text inside a JSON string (`"t":
"8.7654321e8"`) is free text: the masker redacts it literally, and it is not
compared by value. That follows rule 1(c) ("a number token"), and the guard
test pins it as a control. A text number joined to a word or a point
(`a8.7654321e8`) is not a number token. A number whose exponent has more
than 28 significant digits refuses wherever any binding is a number. A
binding whose exponent has more than 30 is never equal to a number within
that, and the doc comment gives the arithmetic.

`output::decimal`, the precision check's reading of the server's own
numbers, and `secret.rs`'s `Magnitude` both normalize a decimal. A binding
is not public, so the precision check cannot use `Magnitude`, and
`Magnitude` cannot be exported without handing back something derived from
a value. This is a knowingly bent decision 0071 ruling 5 (a reading written
twice), for decision 0012's boundary. The grammar that finds a number token
in text (`spelled`) and the one that reads it (`Magnitude::read`) are
likewise on either side of that boundary.

### Tests

`rpc/tests/masking.rs` now holds 25 tests, 1,135 lines. Six are new:

| Test | What it asserts exactly |
| --- | --- |
| `a_harness_id_is_guarded_as_it_arrives_whatever_it_writes` | Each of these ends `Err(Failed(Masking))` with no call recorded: id `"\u0000"` under a bound `u0000`; TOKEN as a text id, which its JSON escapes; and id `876543210` under `0876543210`. A frame holding a bound `7,"result` only across a safe id 7 and a safe body is refused the same way, for a listing and for a ping. On the unrecorded path (append 2 failing), an unsafe id still ends `Failed(Masking)` and records nothing. On the `failed()` path, an active call `"1"` is told the masking cause and recorded `Failed`, and the unsafe ping is told nothing. |
| `a_bound_value_in_a_fixed_position_is_refused_and_never_rewritten` | Bound `text`, `audience`, `user` and `structuredContent` refuse. A bound `object` (inputSchema.type) and a bound `lookup` (a granted name) end the session before the harness is read. Controls: text and structured keys are redacted, and an annotated block is kept whole. The exact listing bytes show only titles and descriptions redacted, at every schema depth, including a property named `title`, while `required`, `enum`, `_meta` and non-string titles are kept. A resource link's title is redacted and its other members kept. |
| `a_tool_name_holding_a_bound_value_reaches_no_record_refusal_or_frame` | An ungranted tool `876543210` is recorded as `[secret:DOCS_PIN]` and `Refused(Ungranted)`, the harness is told the masking cause, the session ends `Failed(Masking)`, and the frame and ledger pass the leak scan. Tool `8.7654321e8` leaves no record and no ledger bytes of it. |
| `the_final_guard_reads_the_very_bytes_and_every_form_they_decode_to` | Refused: `\u0000`, `\/`, a surrogate pair, a lone surrogate (U+FFFD then x), control escapes, an all-escaped CANARY, a JSON number by value, and text numbers in quotes, after `--`, after a date and behind an escape. Kept: a JSON string number, valid escapes, malformed `\u` escapes, an unknown escape, a trailing backslash, and joined or partial numbers. |
| `standard_error_is_proved_as_the_text_it_decodes_to` | The review's raw `FF 78 0A` under a bound U+FFFD then x is kept as `[secret:ODD]\n`. A line holding CANARY only by value is kept as the marker, and nothing of it is kept where the marker holds a bound value too. |
| `no_spelling_or_escape_of_a_bound_value_leaves_by_any_path` | The sweep. It covers 24 spellings: TOKEN and CANARY in all nine shared encodings, each escaped whole as `\u` units, and CANARY as `876543210`, `8.7654321e8`, `876543210.0` and `87654321e1`. Each goes out by every path: a call's result at `_meta` (a frame), the server's stderr, the harness's own id (a local reply), and, where it fits the tool vocabulary, a tool name (the ledger). Every harness byte, the ledger and the diagnostics hold no shared encoding or JSON escaping of either value, and not the spelling as written. |

The existing tests were changed where the class repair changed what they
assert. `a_bound_number_is_refused_by_its_value_however_either_side_spells_it`
now carries both reviews' counterexamples: `1.0e20` against `1e+20`, `1.50e3`
against `1500`, `1e020`, `87.654321e7`, `876543210.000000000`, a binding
`000000000876543210`, `8.7654321e08`, `0` against `0000` and `-0000`, and
`-0.0`. It also covers PIN=`0000` beside NUMBER=`0876543210` refusing
`876543210` and keeping `7`. `a_harness_id_holding_a_bound_value_is_answered_on_no_path`
now expects no call recorded on any path, since ids are refused on arrival.
The key-collision test moved its deep case into `structuredContent`, added
`error.data`, and pins a block's colliding keys as the masking cause, since
block keys are fixed. `masking_that_changes_what_a_reply_says_cannot_be_safely_masked`
asserts the refusal at the guard and the outcome by `delivered`. The
stderr tests read the tail as lines. `brokkr-protocol`'s secret test module
gained `a_number_is_a_bound_value_by_what_it_is_worth_however_either_side_spells_it`
(each normalization, and malformed tokens and bindings) and
`each_binding_is_read_on_its_own_and_a_vast_exponent_fails_closed`.
`machine_proof::output_edge` now pins `mask_json` to `freely`,
`canonical::to_bytes` to `prepared`, `mask_bytes` to `masked`, `unmasked`
and stderr's `keep`, and `bound_by_value` to `guarded` alone.

### Removals

Each removal compiled and was run with `cargo test -p brokkr-cli --lib
broker::rpc::tests::masking`, R12 also with `cargo test -p brokkr-protocol
--lib secret::`. Each failed as recorded and was restored. The restored tree
then passed `--lib broker::` (107 tests).

| Removal | Failing test and assertion |
| --- | --- |
| R1, `guarded` always passes | 13 tests. The sweep: TOKEN's JSON escaping at surface 0 (the harness). The value test: `("100000000000000000000", …, Ok(…))` against `Err(Failed(Masking))`. The guard test: `{"a":"\u0000"}` `Ok(())` against `Err`. Stderr: `"pin 8.7654321e8\nnext\n"` against the marker. The precision, duplicate-key, startup, canary-as-text, null-data and encodings tests still passed. |
| R2, the scalar check off (`false & bound_by_value`) | 7 tests, among them the value test, the guard test's `{"n":8.7654321e8}`, and the sweep (`8.7654321e8` held in the diagnostics). The precision tests passed. |
| R3, numeric precision off (`all(\|_\| true)`) | Only `a_number_read_as_another_value_is_the_servers_protocol_fault` (`{"n":1.00000000000000000001}` `Ok(())` against `Err(Failed(Protocol))`) and `a_servers_startup_answers_are_read_exactly_too`. Every F1 test passed. R2 and R3 are independent. |
| R4, the decoded check off | The guard test: `{"a":"a\/b\/c\/d"}` `Ok(())` against `Err`. The sweep: TOKEN's JSON escaping at the harness. |
| R5, ids checked against no binding on arrival | `…answered_on_no_path`: case `call` recorded `[("lookup", Some(Failed))]` against `[]`. `…as_it_arrives…`: `Err(Unrecorded(Record))` against `Err(Failed(Failed(Masking)))`. |
| R6, the whole result masked instead of `REDACTABLE` | The fixed-position test: `("text", Ok(…))` against `Err(Failed(Masking))`. The collision test: a block's keys gave `Err(Failed(Protocol))` against the masking cause. The changes-what-it-says test: `Ok` against `Err(Failed(Masking))`. |
| R7, `named` against no binding | The tool-name test: the harness heard "tool '[secret:DOCS_PIN]' is not granted…", the old masked-text refusal, instead of the masking cause. The sweep: TOKEN's base64 held at surface 1 (the ledger). |
| R8, stderr's decoded text not masked | `standard_error_is_proved_as_the_text_it_decodes_to`: the marker against `"[secret:ODD]\n"`. The guard still kept the unmasked line out. |
| R9, stderr lines kept unguarded | The same test: `"pin 8.7654321e8\nnext\n"` against the marker. The sweep: TOKEN's JSON escaping at surface 2 (the diagnostics). |
| R10, `framed` guarding against no binding | `a_bound_number_spelled_otherwise_…` (`1e+20` delivered), the digits-only canary test (delivered), and the sweep at surface 0. Run again on the final test (R10b), `…as_it_arrives…`: the harness heard the straddling listing `{"jsonrpc":"2.0","id":7,"result":{"tools":…` where it expected none. |
| R11, the catalog guarded against no binding | The catalog test and the fixed-position test: `(Ok(()), … ServingIncomplete)` against `(Err(Failed(Failed(Masking))), … Failed(Masking))`. |
| R12, `bound_by_value` requiring every binding (`all`) | Protocol: `("1500", "1500x", true)` against `false`, and the each-binding test. CLI: PIN=`0000` beside NUMBER=`0876543210` delivered `876543210` against `Err(Failed(Masking))`. |

### Measurements and gates, fourth run

Files measure `output.rs` 490 lines, `rpc.rs` 650, `rpc/calls.rs` 793,
`session/stderr.rs` 140, `session.rs` 800 (not edited by this run),
`crates/brokkr-protocol/src/secret.rs` 794, `rpc/tests/masking.rs` 1,135,
`rpc/tests.rs` 1,946, `secret/tests.rs` 1,225 and
`tests/machine_proof/output_edge.rs` 91. `quality/file-lines.txt` carries
those counts. No file was added and no code moved between files.
`cargo crap` measured every new or changed function at cyclomatic
complexity 14 or below. The largest are `Session::requested` at 14,
`Session::call` at 13, `numbers`, `redact` and `Power::read` at 8, and
`Magnitude::read` at 7. No function crosses 100 lines, and no suppression
was added.

The public API grows by the one ruled item,
`pub fn brokkr_protocol::secret::bound_by_value(&[u8], &[BoundSecret]) -> bool`.
Following U4c's precedent, the snapshot was regenerated by hand with
`cargo +nightly-2026-09-05 public-api -p brokkr-protocol -sss --color never`
under its unchanged provenance line. The new snapshot differs by that one
line, and `quality/ratchet.sh api` printed "public API holds".
`quality/ratchet.sh baselines origin/main` listed the one raise,
"public-api/brokkr-protocol.txt: 1007 public items (was 1006)", and asks for
the PR's `Ruling:` line, which is the operator's 2026-10-12 ruling. That
pass is pending, to be read in CI.

| Gate | Observed on the final tree |
| --- | --- |
| brokkr-cli | lib 750 passed (1 ignored), 107 of them under `broker::`. `it` 594 passed (2 ignored), including 64 under `machine_proof::` and 74 under `capability_broker::`; `machine_proof::expose_for_spawn_has_exactly_one_production_call_site` passed alone too. `driver_conformance` 27, `transcript_surfaces` 13 and each `heap_*` 1 passed. |
| Other crates | brokkr-protocol lib 880 passed (3 ignored), with 44 under `secret::`, and its three test targets passed. brokkr-runtime lib 930, `it` 131 (`witness_digests` held without blessing), `capability_launch` 75, `operated_repo` 1 and `queued_launch` 3. core, view, store and bridge, every target, all ok. Seatbelt-probe 99 (2 ignored) and the workspace doctests ok. |
| Toolchains, lints, records | `cargo +1.88 check --workspace --all-targets --all-features` finished. The format check, workspace clippy (all targets and features, locked, `-D warnings`), `typos --hidden` and `git diff --check` (staged and unstaged) reported nothing. `openspec validate --all --strict`: 19 passed. `recipes/self` compiled: 47 valuations over 11 groups, 0 unruled. |
| Ratchets | "file size holds", "duplication holds", "public API holds"; baselines as above. |
| 16 KiB pipes | U6e's procedure, local and uncommitted: rustix's `pipe` feature, and `pipe()` setting and asserting 16,384 bytes. `cargo test -p brokkr-cli --lib broker::rpc` passed 60 tests in 16.22 s. Both edits were reverted, and `git status` shows `Cargo.toml` and `rpc/tests.rs` without unstaged changes. |
| Coverage diagnostic | `cargo +nightly-2026-09-05 llvm-cov --branch` over `-p brokkr-cli --lib -- broker::`: `output.rs` 253 of 253 lines, 8 of 8 branches and 42 of 42 functions; `rpc/calls.rs` 390/390, 32/32 and 51/51; `session/stderr.rs` 50/50, 10/10 and 8/8. In `rpc.rs` only `initialized` is unhit, which the CLI integration suite covers. Over `-p brokkr-protocol --lib -- secret::`, the new lines 665 to 794 of `secret.rs` are all hit, and all 98 branches in the file. This is a diagnostic, not the gate. |

The first coverage run found the 8 MiB-at-the-bound test running out its
10 s attempt under instrumentation. `unescaped` had allocated per byte, and
it now copies the runs between backslashes. The same run found `At::item`'s
split fallback arm and `Session::tell`'s refused-frame closure unhit. The arm
was folded into one, and the straddling `7,"result` cases now reach both
refused-frame paths.

### Corrections to the second run's record

The "Repair" section's claims that `brokkr-protocol` is untouched, that
`Session::framed` checks ids, that a `Shape` projection keeps each reply's
protocol shape, that numbers are spelled with up to eight padding zeros
with a padding exception, and that `Catalog` holding `Vec<Tool>` repairs the
`Value` low are each superseded by the sections above. The stderr design
note at the top ("masked by the shared masker on its raw bytes and only then
decoded") is superseded too: the decoded line is what is proved.

### Pending, fourth run

Still pending: exact coverage by `scripts/coverage-exact.sh` on a capable
host or in CI; the real-box tests at two CPUs on the controller's host,
which skip in this seat's box; the macOS build and tests; the baselines
ratchet's read of the PR's `Ruling:` line; and remote CI on the final head.
The offline link check (`lychee`) was not run, because this seat's sandbox
refuses the command. The suites ran with the default `TMPDIR`, because the
sandbox refused creating an owner-only directory under `/var/tmp` and
refused setting `TMPDIR` for a command.

## Repair of the fourth run's findings (operator rulings 2026-10-13, fifth run)

The fourth run's review (gpt-6-astra, xhigh) confirmed that none of the
second run's F1 to F7 classes recurred, and held it on two high findings
(H1, H2), three medium (M1, M2, M3) and five low (L1 to L5). The operator
ruled on 2026-10-13. This run keeps the fourth run's design and tests and
repairs H1, H2, M2, M3, L1 and L4. The incomplete-serving refusal stays;
`crates/brokkr-cli/src/broker.rs`, `crates/brokkr-protocol/src/broker.rs`
and `hands/namespace/entry.rs` are untouched.

### One guard, in the secret module

The final guard moved from `output.rs` into a new private child of
`crates/brokkr-protocol/src/secret.rs`,
`crates/brokkr-protocol/src/secret/guard.rs`, declared there as `mod guard`.
`secret.rs` stood at 794 lines and the guard would have taken it past 800, so
the new file uses the operator's standing rule for a new private child module
beside a row file that the change would push past its ceiling (extended
2026-10-11). The move takes the number tokenizer (`numbers`, `spelled`) and
the escape decoder (`unescaped`, `units`, `unit`, `control`) out of
`output.rs` unchanged, and the exact-magnitude reading (`Magnitude`,
`Power`, and the two exponent bounds) out of `secret.rs` unchanged. A diff of
the moved bodies against the staged fourth-run files showed only the
`Debug` derives removed, so nothing in the module can print a binding's
digits. One guard now serves the output edge, the stderr drain and the
ledger, which closes the duplicate the clone gate and ruling 5 would refuse.

`secret::Guard::new(bindings)` reads each binding that is a decimal number
once, as its magnitude, and wipes those digits when it is dropped (with the
module's own `wipe`). It has no `Debug` and no accessor for them. Its answer
is a bool, or the caller's poll error. The comparison still reads `Secret`'s
private bytes inside the secret module and never through `expose_for_spawn`.
The single-injector machine proof passed unchanged.

The operator approved this public API change on 2026-10-13. Every changed
signature in `quality/public-api/brokkr-protocol.txt` (1,103 items before,
1,113 after) is listed here.

| Item | Before | After |
| --- | --- | --- |
| `secret::bound_by_value` | `pub fn (&[u8], &[BoundSecret]) -> bool` (added by the fourth run, never on main) | removed; its comparison is the private `Guard::valued` |
| `secret::Guard<'b>` | none | `pub struct`, with `new(&'b [BoundSecret])`, `json(&self, &[u8]) -> bool`, `json_within<B>(&self, &[u8], impl FnMut() -> Result<(), B>) -> Result<bool, B>`, `text(&self, &[u8]) -> bool` and its `Drop` |
| `secret::json_numbers` | none | `pub fn (&[u8]) -> Vec<&[u8]>`, the guard's tokenizer, which `output::checked`'s precision check reads |
| `broker::ledger::Ledger` | `Ledger` | `Ledger<'b>`, holding its guard |
| `Ledger::open` | `(dir, &OsStr, &Owner, &str)` | `(dir, &OsStr, (&Owner, &str), &'b [BoundSecret])` |
| `Ledger::open_with` | `(append, &Owner, &str)` | `(append, (&Owner, &str), &'b [BoundSecret])` |
| `Ledger::room` | `-> Result<Room<'_>, Failure>` | `-> Result<Room<'_, 'b>, Failure>` |
| `broker::ledger::Room` | `Room<'l>` | `Room<'l, 'b>` |
| `broker::ledger::Unrecorded::Masking` | none | new variant; its text is `Failure::Masking`'s |

`secret::mask_text` became `pub(crate)`, with no change to the public API, so
the ledger reserves a Terminal with the tool as `mask_json` writes it.

### Each finding, and what closes it

| Finding | Repair | Test |
| --- | --- | --- |
| H1, ledger output bypassed the guard | `Ledger::record` passes every record (Opened, Started, Terminal, Closed and the refused and failed ones) through `Ledger::masked` before the lifecycle admits it and before it is appended. The entry's JSON value is masked with `secret::mask_json`. Everything but the tool must come out equal, so an outcome, a record kind or a key that masking would rewrite is refused. The masked value is then read back through the closed `Entry` reader and serialized by `Entry::line`, in the old format. The guard reads that exact line as JSON, and those bytes are what is appended. A refused record is not appended: the ledger latches `Failure::Masking` as the session's cause and answers `Unrecorded::Masking`, whose text is the masking cause. The lifecycle admits the masked entry, so a Started and its Terminal carry the same masked tool and `recover` reads them back. Old ledgers still read: the line format and `recover` are unchanged, and every existing ledger test passed. `Room::begin` reserves the Terminal with the masked tool, so a name that grows under masking is the ledger's limit before anything is recorded. | `capability_broker::ledger::every_record_is_masked_and_guarded_before_it_is_appended`: STATE=succeeded (Terminal refused, call left begun, no `succeeded` in the bytes, the failed control recorded); STATE=started (Started refused, closure `Failed(Masking)`); a bound `3.00` equal by value to the third record's sequence number (Terminal refused on its bytes); a masked tool recorded and recovered whole; a 3,900-byte name that is the ledger's limit only once masked. `rpc::tests::masking::a_record_the_ledgers_guard_refuses_ends_the_session_with_the_masking_cause`: through the served session, the harness is told the masking cause and the session ends `Unrecorded(Masking)` with its call begun. `each_ledger_cause_reads_in_cr3s_words` pins the new variant's text. |
| H2, schema instance data treated as schema | A second table, `SCHEMA`, beside `REDACTABLE`. A schema's `enum`, `const`, `default` and `examples` are `At::Fixed`, every item included, so a title or description inside them comes out byte-identical and a bound value there is refused. A schema's `properties`, `patternProperties`, `dependentSchemas`, `$defs` and `definitions` are a new `At::Schemas` position, each member of which is a schema whatever its name. A property named `enum` or `default` therefore stays a schema whose description is free text. | `a_schemas_instance_data_is_never_masked_and_refuses_a_bound_value`: one case per keyword asserts the prepared listing byte-identical and the edge refusing it with the masking cause. A control over all five named-schema places times the four keyword names asserts the description redacted and the listing kept. |
| M2, uninterruptible number scan | Bindings are read once per guard, not per token. `Guard::json_within` reads the caller's poll before every batch of 64 numbers compared, and the first error ends the scan and is handed back. `output::watched(scan)` is the poll the edge uses: the frame's own deadline and the attempt's cancellation, each ending the scan with the timeout cause, which is the session's cause for both (`Session::going` latches `TIMEOUT` for each). `framed` takes it, as does the listing prepared in `served_within` (the attempt's deadline). `told`, `listing` and `answered` each pass the deadline of the frame they make, so a closing frame keeps its fixed `CLOSING` bound after the attempt's deadline has passed. | `the_final_guard_ends_a_frames_scan_at_its_deadline_or_cancellation`: a 300-number frame is `Err(TIMEOUT)` past its deadline, `Err(TIMEOUT)` with the attempt cancelled, and made whole within its deadline. `secret::tests::the_guard_polls_its_caller_between_bounded_batches_and_stops_at_its_word`: 130 numbers poll three times, and a poll error at the second batch returns `Err("deadline")` before a bound alias past it is compared, while the full scan refuses it. |
| M3, an unsafe alias left no evidence | `named` returns `Safe` or `Masked(name)`. Where neither the name nor its masked text passes the guard, the name is recorded wholly masked, as `[secret]`. The call is recorded under that name, refused as ungranted, the harness told the masking cause, and the session ends `Failed(Masking)`. | `a_tool_name_holding_a_bound_value_reaches_no_record_refusal_or_frame` now asserts, for `8.7654321e8` under a bound `876543210`, the told frame, the Started and Terminal `("[secret]", Refused(Ungranted))`, closure `Failed(Masking)`, and the alias in neither the frame nor the ledger. |
| L1, text surfaces read as JSON | The caller names the surface: frames and ledger lines are read with `Guard::json`, and ids' values, tool names and stderr lines with `Guard::text`. A text that is itself a JSON string is read as text, so `"8.7654321e8"` refuses as `pin 8.7654321e8` does. An id's serialization is still read as JSON. | `the_final_guard_reads_the_very_bytes_and_every_form_they_decode_to`, split into a JSON table and a text table, with the new text case `"8.7654321e8"`. |
| L4, stale counts | `quality/file-lines.txt` was re-measured with `wc -l` for every file this unit touches (below), and this record carries the same counts. | `quality/ratchet.sh files`: "file size holds". |

`machine_proof::output_edge::the_output_edge_masks_with_the_shared_masker_and_serializes_once`
now also pins the one guard by grep. `Guard::new` is constructed in the
broker only in `served_within` and `Drain::new`, and in the protocol only in
`Ledger::open_with`. The ledger's `mask_json` is called in `Ledger::masked`,
and a number is compared by value only in `Guard::passes`.

### Removals

Each removal compiled. It was run against the named tests, then restored
from a saved copy, and `cmp` confirmed each restored file byte-identical.
Removals in one group touched different tests, and each test's first
failing assertion is the one recorded.

| Removal | Failing test and assertion |
| --- | --- |
| `Ledger::masked` returns the unmasked entry and its line (no mask, no guard) | `every_record_is_masked…` at the STATE=succeeded Terminal (left `Ok(())`, right `Err(Masking)`); `a_record_the_ledgers_guard_refuses…` at the told frame (the harness got the result) |
| Only the byte guard on the ledger line removed | `every_record_is_masked…` at the `3.00` Terminal (left `Ok(())`, right `Err(Masking)`) |
| `Room::begin` reserves with the raw tool | `every_record_is_masked…` at the long name (left `Some(Record)`, right `Some(Limit)`) |
| `SCHEMA`'s `enum` row removed; then, separately, `const`, `default` and `examples` | `a_schemas_instance_data…` at that keyword's case each time (left: the title or description masked) |
| `SCHEMA`'s five named-schema rows removed | the same test's control at `("properties", "enum")` (left `Err(Failed(Masking))`) |
| `poll()?` removed from `Guard::passes` | `the_final_guard_ends_a_frames_scan…` at the past-deadline case (left `Ok(frame)`, right `Err(Failed(Timeout))`); `the_guard_polls…` (left 0 polls, right 3) |
| Poll on every number instead of each batch | `the_guard_polls…` (left 130 polls, right 3) |
| `named` records the masked text even where unsafe | `a_tool_name_holding…` (left `"8.7654321e8"` recorded, right `"[secret]"`) |
| `Guard::text` reads its bytes as JSON | `the_final_guard_reads…` at the text `pin "8.7654321e8"` (left `true`, right `false`) |

### Rulings recorded, not residuals

M1 is the operator's ruling of 2026-10-13: the number rule is cautious. The
same digits refuse with either sign, and a number too large to compare
safely refuses. Both stay, and both remain pinned by
`a_number_is_a_bound_value_by_what_it_is_worth_however_either_side_spells_it`
and `each_binding_is_read_on_its_own_and_a_vast_exponent_fails_closed`.

L2 (`Tool.described` untyped) is carried by the operator's ruling with its
stated reason, which is the fourth run's F6 entry above.

L3 (a decimal normalized in both `output.rs` and the secret module) is
carried by the operator's ruling with its stated reason. The shared guard did
not let `output.rs` drop its own normalization: `output::decimal` reads the
server's own numbers for the precision check, and `Magnitude` reads bindings
inside the secret module. The tokenizer, which both had copied, is now one
(`secret::json_numbers`). L5 needs no action.

### Assumptions, named

`[secret]` is the whole-name redaction for a tool name that masking text
cannot make safe. It cannot collide with the masker's `[secret:NAME]`. Where
an operator has bound `[secret]` itself, the ledger's guard refuses the
record and the session still ends with the masking cause.

In production, the ledger is opened before any binding is looked up (CR3's
order in `session::lifetime::lived`), and `serve` refuses before serving.
Its records there are the Opened and the Closed alone, holding the owner,
the plan digest and the disposition, and they are guarded by no binding:
`lived` opens with `&[]`. U6f1b, which serves, must hand the session's
bindings to the ledger's guard. This is recorded as that unit's follow-up,
and `lived`'s doc comment says so.

A record refused by the ledger's guard leaves its call begun. While a call
is unresolved the lifecycle refuses the Closed, so after a refused Terminal
the closure is not written, and recovery reads that call as interrupted. The
session ends `Unrecorded(Masking)`, whose text is the masking cause. A
refused Started, Opened or Closed leaves no call begun, so a later closure
carries `Failed(Masking)`.

### Measurements and gates, fifth run

`wc -l` measured `output.rs` 400, `rpc.rs` 650 (unchanged by this run),
`rpc/calls.rs` 799, `session.rs` 800 (not edited), `session/lifetime.rs` 95,
`session/stderr.rs` 142, `crates/brokkr-protocol/src/broker/ledger.rs` 584,
`secret.rs` 653, `secret/guard.rs` 381 (new),
`rpc/tests/masking.rs` 1,262, `rpc/tests.rs` 1,946,
`tests/capability_broker/ledger.rs` 986, `secret/tests.rs` 1,256,
`broker/ledger/tests.rs` 146 and `tests/machine_proof/output_edge.rs` 102.
`quality/file-lines.txt` carries each count. `cargo crap` over both crates'
`src` measured every new or changed function at cyclomatic complexity 14 or
below. The largest are the unchanged `Session::requested` and
`Session::serving` at 14, `Session::call` at 12, and `Guard::passes`,
`numbers`, `Power::read` and `redact` at 8. The two functions at 15 in these
files, `Lifecycle::admit` and `scan_secret_refs`, are untouched. No function
crosses 100 lines, and no suppression was added.

| Gate | Observed on the final tree |
| --- | --- |
| brokkr-cli | lib 753 passed (1 ignored), 110 of them under `broker::`; `it` 596 passed (3 ignored), with 64 under `machine_proof::` and 75 under `capability_broker::`; `driver_conformance` 27, `transcript_surfaces` 13 and each `heap_*` 1 passed. |
| brokkr-protocol | lib 888 passed (3 ignored), 45 of them under `secret::`; `hands_exits` 6, `secret_drop` 1 and the doctest passed. |
| brokkr-runtime | `it witness_digests::` 6 passed without blessing. |
| Toolchains, lints, records | `cargo +1.88 check --workspace --all-targets --all-features` finished. The format check, workspace clippy (all targets and features, locked, `-D warnings`), `typos --hidden` and `git diff --check` (staged and unstaged) reported nothing. `openspec validate --all --strict`: 19 passed. `recipes/self` compiled: 47 valuations over 11 groups, 0 unruled. |
| Ratchets | "file size holds", "duplication holds" and "public API holds". `baselines origin/main` lists "public-api/brokkr-protocol.txt: 1113 public items (was 1103)" and asks for the PR's `Ruling:` line, which is the operator's 2026-10-13 ruling. |
| 16 KiB pipes | U6e's procedure, local and uncommitted: rustix's `pipe` feature, and `pipe()` setting and asserting 16,384 bytes. `cargo test -p brokkr-cli --lib broker::rpc` passed 63 tests in 16.01 s. Both edits were reverted, and `git diff` of `crates/brokkr-cli/Cargo.toml` is empty. |
| Coverage diagnostic | `cargo +nightly-2026-09-05 llvm-cov --branch` over the protocol and CLI lib tests (`secret::`, `broker::`) and `it capability_broker::ledger`: `secret/guard.rs` 226 of 226 lines, 22 of 22 branches and 38 of 38 functions; `broker/ledger.rs` 244/244, 6/6 and 36/36; `output.rs` 150/150, 2/2 and 28/28; `rpc/calls.rs` 396/396, 32/32 and 51/51; `session/stderr.rs` 51/51, 10/10 and 8/8. The first pass found `but_tool`'s `if let` miss unhit, since an entry always serializes as an object, and the branch was removed. In `secret.rs` only the untouched `BoundSecret::name` accessor lines are unhit under this filter. This is a diagnostic, not the gate. |

The whole-crate CLI runs above came before the last production edit, the
coverage-driven `but_tool` change in `ledger.rs`. After it, brokkr-protocol's
lib (888 passed, 3 ignored), the CLI's `broker::` lib tests (110),
`it capability_broker:: machine_proof::` (139), workspace clippy, the format
check, `typos`, `git diff --check`, `openspec validate` and the files and
duplication ratchets were run again, and all passed.

### Pending, fifth run

Everything the fourth run left pending is still pending after this one, for
the same reasons: the exact gate, the host's real-box tests at two CPUs,
macOS, the `Ruling:` line's read, remote CI and the offline link check,
which this sandbox again refused. Its suites again used the default
`TMPDIR`. The one new item is U6f1b's: the ledger it serves with must be
opened with the session's bindings.

## Sixth run: repair of the fifth run's five findings (operator ruling 2026-10-13)

The fifth run's review (gpt-6-astra, xhigh) held it on one high finding (F1),
one medium (F2) and three low (F3, F4, F5). The operator ruled on
2026-10-13 that U6f1a repairs all five. This run keeps the fifth run's
design, code and tests and changes only what each finding names. The
incomplete-serving refusal stays, and `crates/brokkr-cli/src/broker.rs`,
`crates/brokkr-protocol/src/broker.rs` and `hands/namespace/entry.rs` are
untouched. M1, L2 and L3 stand as the fifth run recorded them.

### Each finding, and what closes it

| Finding | Repair | Test |
| --- | --- | --- |
| F1 (high), the guard trusted "masking changed nothing" | `secret.rs` builds the masker's needles in one private function, `needles`, which `mask_needles` and the guard share. `Guard::new` keeps each binding's needles (the raw value and its eight shared encodings), wiped on drop beside the magnitudes. `Guard::holds` reports a needle wherever it occurs, in the exact bytes and in their decoded form (`unescaped`), and never compares a masked copy with the original. `mask_bytes`, `mask_json` and `NEEDLE_ENCODINGS` behave as before. | `rpc::tests::masking::a_secret_that_is_its_own_placeholder_is_found_where_it_occurs` binds STATE=`[secret:STATE]`. It asserts that the legacy masker leaves that value as it is. It then checks four things together: the fixed `_meta` field at the edge is `Err(MASKING)`; the finding's whole frame fails `Guard::json`; the string correlation id fails `correlated`; and the stderr line is kept only as its marker. In the control, STATE=`succeeded` passes all three surfaces. `capability_broker::ledger::a_secret_is_found_in_a_record_and_bound_once_known` checks a ledger value: the Started for `get-[secret:STATE]` is `Err(Masking)`, nothing is appended, and the closure is `Failed(Masking)`. |
| F2 (medium), a refused record lost its cause | `impl From<Unrecorded> for Ended` in `session/lifetime.rs` turns `Unrecorded::Masking` into `Ended::Failed(Failed(Masking))`, and leaves every other variant unrecorded. The match is exhaustive. `Session::ended`, `Session::call` and `served_within`'s bind use it. If `Session::failed` sees its Terminal refused by the guard, it ends the session with the masking cause and tells the harness that cause. Any other refusal keeps the original fatal, as before. `settled` now takes and returns the session's ending, so a closure refused by the guard ends the session with the masking cause. Other closure errors still end it as it ended. Nothing already appended is rewritten. | `a_ledger_record_the_guard_refuses_ends_the_session_masked_on_every_path` covers two cases. STATE=failed: the server's protocol fault fails the active call, and its Terminal and then its closure are refused. The harness is told the masking cause, the session ends `Failed(MASKING)` and the call stays begun with no closure. STATE=closed: the call is answered and recorded `Succeeded`, the closure is refused, the session ends `Failed(MASKING)`, and no `closed` byte is in the ledger. `a_record_the_ledgers_guard_refuses_ends_the_session_with_the_masking_cause` (STATE=succeeded) now expects `Failed(MASKING)` instead of `Unrecorded(Masking)`. |
| F3 (low), Opened and Closed unchecked | New public `Ledger::bind(&mut self, &'b [BoundSecret]) -> Result<(), Unrecorded>`. It replaces the ledger's guard with one over the session's bindings, then passes the Opened's exact line, re-derived from the lifecycle's owner and digest, through that guard. A refusal latches the masking cause and leaves the Opened as written. `served_within` calls it first, with the edge's bindings, so every later record (Started, Terminal and the Closed `settled` writes) is guarded by them. The test harness (`rpc::tests::edged`) now opens its ledger with no bindings, as `serve` does, so the session's own bind is what guards it. | `the_opened_record_is_guarded_once_the_sessions_bindings_are_known`: a binding of the plan digest's first eight characters ends the session `Failed(MASKING)` before the harness is read, and the closure is `Failed(Masking)`. The STATE=closed case above covers the Closed. `capability_broker::ledger::a_secret_is_found_in_a_record_and_bound_once_known` checks `bind` directly: a binding inside the digest gives `Err(Masking)` and the closure is `Failed(Masking)`; an unrelated binding gives `Ok(())` and then STATE=closed refuses `close()`. |
| F4 (low), glued numbers not scanned | On a text surface, `numbers` now tries a number at every byte not preceded by a digit, so one glued to a word, a point or another number's exponent is read as the number it spells. A run of digits is read once, from its first digit, which keeps the scan linear. JSON surfaces keep their greedy reading of JSON's own numbers outside strings, which the precision check shares. Refusing more is within the operator's cautious number rule. | `the_final_guard_reads_the_very_bytes_and_every_form_they_decode_to` now refuses `x8.7654321e8`, `token8.7654321e8`, `.8.7654321e8` and `1e8.7654321e8`, and refuses `8.7654321e8th` as before. Its control text `1e5x 1.2.3 -.5e+ e8 18.7654321e8 v2.0` passes, since `18.7654321e8` is another number. `a_number_glued_to_a_word_is_read_in_ids_names_and_standard_error`: `x8.7654321e8` is refused as a harness id, recorded as `[secret]` as a tool name, and kept as the marker on stderr. The unrelated `x8.7654321e7` passes all three. |
| F5 (low), duplicate conversion | `output::masked` is removed. `secret::mask_text`, now `pub` and taking bytes, is the one mask-then-lossy-decode conversion. `output.rs` (`named`, `redact`, `prepared`) and `session/stderr.rs` (`Drain::keep`, raw bytes and then the decoded text) call it, as do `mask_json` and the ledger's reservation. `crates/brokkr-protocol/src/adapters.rs` (about line 710) holds a third copy, outside this row, and is recorded as a follow-up. | `machine_proof::output_edge::the_output_edge_masks_with_the_shared_masker_and_serializes_once` now pins the broker's `mask_text(` call sites (`named`, `prepared`, `redact` and `keep` twice) and no broker `mask_bytes(` call. It also pins `Guard::new` in the protocol at `Ledger::bind` and `Ledger::open_with`, and in the broker at `served_within<'b>` and `Drain::new`. |

Of the optional INFO items, one was cheap and is done: `Magnitude::read`
wipes its temporary copy of the digits, which for a binding holds the
binding's bytes whatever they are. The other two are recorded here, not
fixed. The occurrence scan, like the masker before it, is not polled for
the frame's deadline between needles. Only the number comparison is
batched. Unlisted schema keywords and a non-string title or description
are read as the fifth run ruled.

### Order, and what Opened holds

`session::lifetime::lived` opens the ledger before any binding is looked up,
as CR3 orders. In this unit the broker process never holds a
`BoundSecret`. The handoff thread reads the bindings from the admitted
store's handle and sends them to the box (`secret::send_environment`), and
`serve` refuses before anything is served. So in production the ledger's
records are the Opened and the Closed alone, both guarded by no binding. The
Opened holds the plan's owner (repository, run, effect, attempt) and its
digest, and the Closed holds the disposition. None of these is derived from
a secret's value. On the served path, which tests drive today and U6f1b
takes up, `served_within` binds the ledger before anything else. That
re-checks the Opened's exact line and guards every later record, the Closed
included. `lived`'s doc comment says so.

### Removals

Each removal below compiled. It was run against the named tests, then
restored by the reverse edit. All of these suites passed again after the
last restoration (the measurements below).

| Removal | Failing test and assertion |
| --- | --- |
| F1: `Guard::holds` reverted to `mask_bytes(bytes, bindings) != bytes` | `a_secret_that_is_its_own_placeholder…`: left `(Ok(body), true, Ok(()), "said [secret:STATE]\n")`, right `(Err(Failed(Masking)), false, Err(Failed(Masking)), marker)`, so all four surfaces fail. `capability_broker::ledger::a_secret_is_found…` failed at line 301, the Started's `Some(Unrecorded::Masking)` |
| F2: `ended = Unrecorded::Masking.into()` removed from `Session::failed` | `a_ledger_record_the_guard_refuses…` failed at the STATE=failed told frame, with left message "MCP server protocol is invalid" |
| F2: `settled` answers `ended` where the closure is refused by the guard | the same test failed at STATE=closed, with left `(Ok(()), ([("lookup", Some(Succeeded))], None, Some(Lifecycle)))` |
| F3: `bind`'s check of the Opened line forced to pass | `the_opened_record…`: left `(Ok(()), ([], Some(Failed(Refused(ServingIncomplete))), None))`. `capability_broker::ledger::a_secret_is_found…` failed at line 314 with left `Ok(())` |
| F3: `bind` keeps the old guard (`let _ = Guard::new(bindings)`) | `the_opened_record…` failed as above. `a_ledger_record_the_guard_refuses…` failed at the told frame (protocol cause). `a_record_the_ledgers_guard_refuses…` failed at the told frame, because the harness got the result. `capability_broker::ledger::a_secret_is_found…` failed at line 314 |
| F4: the fifth run's joined-token skip and greedy text step restored | `a_number_glued…` at `x8.7654321e8` as an id (left `Ok(())`). `the_final_guard_reads…` at `("x8.7654321e8", true)`. Each test's first failing assertion is the one recorded |

F5 is a single-conversion refactor with no behaviour of its own to remove.
The grep proof above binds it.

### Files

`crates/brokkr-cli/src/broker/rpc/calls.rs` would have reached 807 lines.
`Delivery` and `Outbox`, unchanged, moved into a new private child module,
`crates/brokkr-cli/src/broker/rpc/calls/outbox.rs`, declared from `calls.rs`
as `mod outbox` and re-exported to the tests as before. This uses the
operator's standing rule for a new private child beside a row file that the
change would push past 800 lines. `wc -l` measured `output.rs` 395,
`rpc/calls.rs` 755, `rpc/calls/outbox.rs` 66 (new), `session/lifetime.rs`
121, `session/stderr.rs` 141, `crates/brokkr-protocol/src/broker/ledger.rs`
607, `secret.rs` 666, `secret/guard.rs` 403, `rpc/tests.rs` 1,947,
`rpc/tests/masking.rs` 1,389, `tests/capability_broker/ledger.rs` 1,019 and
`tests/machine_proof/output_edge.rs` 109. `quality/file-lines.txt` carries
each count. `rpc.rs`, `session.rs`, `secret/tests.rs` and
`broker/ledger/tests.rs` were not edited in this run.

The public API grows by two items, 1,113 to 1,115 in
`quality/public-api/brokkr-protocol.txt`. `Ledger::bind` is F3's repair,
under the 2026-10-13 ruling that lets the ledger's public functions take the
session's secrets. `secret::mask_text`, now `pub` over `&[u8]`, is F5's
repair: the conversion the operator ruled must be the one in `secret.rs`.
The operator reads both at PR time.

### Measurements and gates, sixth run

`cargo crap` over both crates' `src` measured every new or changed function
at cyclomatic complexity 10 or below. Those are `numbers` 10,
`served_within` 9, `Guard::passes` 8, `Magnitude::read` 7, `settled` and
`Session::failed` 6, `needles` 5, `Ledger::bind` 4 and `Ended::from` 3. The
two functions at 15 in these files, `Lifecycle::admit` and
`scan_secret_refs`, are untouched. No suppression was added.

| Gate | Observed on the final tree |
| --- | --- |
| brokkr-cli | lib 757 passed (1 ignored), 114 of them under `broker::`. `it` 597 passed (3 ignored), with 64 under `machine_proof::` and 76 under `capability_broker::`. `driver_conformance` 27, `transcript_surfaces` 13 and each `heap_*` 1 passed. |
| brokkr-protocol | lib 888 passed (3 ignored), 45 of them under `secret::`. The integration tests (6 and 1) and the doctest passed. |
| brokkr-runtime | `it witness_digests::` 6 passed without blessing. |
| Toolchains, lints, records | `cargo +1.88 check --workspace --all-targets --all-features` finished. The format check, workspace clippy (all targets and features, locked, `-D warnings`), `typos --hidden` and `git diff --check` (staged and unstaged) reported nothing. `openspec validate --all --strict`: 19 passed. `recipes/self` compiled: 47 valuations over 11 groups, 0 unruled. |
| Ratchets | Files, duplication and public API all held. `baselines origin/main` lists "public-api/brokkr-protocol.txt: 1115 public items (was 1103)" and asks for the PR's `Ruling:` line. |
| Coverage diagnostic | `cargo llvm-cov` (line data, `--ignore-run-fail`) over the CLI and protocol lib tests (`broker::`, `secret::`) and over `it capability_broker::ledger machine_proof::output_edge`, with both packages instrumented. `output.rs`, `rpc/calls.rs`, `rpc/calls/outbox.rs`, `session/stderr.rs`, `secret/guard.rs` and `broker/ledger.rs` have no unhit line. `session/lifetime.rs`'s unhit lines in each run are hit in the other. In `secret.rs` only the untouched `BoundSecret::name` accessor is unhit under this filter. Instrumented, two timing tests missed their bounds (`a_cancellation_behind_an_answer…` and `delivery::what_waits…`). Both pass uninstrumented in the lib run above. This is a diagnostic, not the gate. |

### Corrections to the fifth run's record

The fifth run recorded that a refused Terminal ends the session
`Unrecorded(Masking)`. It now ends `Failed(Masking)` (F2). It also recorded
that U6f1b must hand the session's bindings to the ledger. `Ledger::bind`
now does that on the served path (F3).

### Pending, sixth run

These are still pending, for the same reasons as before: the exact gate, the
host's real-box tests at two CPUs, macOS, the `Ruling:` line's read and
remote CI. The offline link check (`lychee`) was again refused by this
sandbox. Its suites used the default `TMPDIR`, because the sandbox refused
an environment prefix. `crates/brokkr-protocol/src/adapters.rs`'s copy of
the mask-and-decode conversion (F5) is outside this row and is recorded as a
follow-up.

## Seventh run: repair of the sixth run's H1 and L1–L3 (operator ruling 2026-10-13)

The sixth run's review confirmed F1, F2, F4 and F5 repaired and held the
unit on one high finding (H1) and three lows (L1, L2, L3). The operator
ruled on 2026-10-13 that U6f1a repairs all four. This run keeps the sixth
run's code and tests, and changes only what each finding names. The
incomplete-serving refusal stays. L4 (the catalog's `Map`), L5 (two
normalizers, decision 0012), L6 (`adapters.rs`'s conversion, out of row)
and M1 (the cautious number rule) are carried unchanged.

### Each finding, and what closes it

| Finding | Repair | Test |
| --- | --- | --- |
| H1 (high), numbers inside JSON strings were not compared by value | `Guard::passes` in `secret/guard.rs` now reads every number the bytes spell as the text they decode to, on JSON surfaces as on text ones: `numbers(&decoded, false)` beside the JSON tokens `numbers(bytes, json)` reads. Every surface the guard sees is covered by the one change. These are frames (text blocks, structured content, keys, an error's message and data, schema text), the catalog, ids, tool names, stderr lines and every ledger line. The comparison is still `Guard::valued`, with the either-sign and too-large rule. The precision check still reads JSON's own tokens alone (`json_numbers`), so the two stay independent. | `rpc::tests::masking::a_bound_number_a_string_spells_fails_its_forwarded_call_on_every_surface` binds DOCS_PIN=876543210 and serves `8.7654321e8` in a text block, a structured string, a key and an error's message. Each runs with retention on, off and vetoed. A served plan carries only its effective retention, and CR1 makes a veto `retained: false`, so the veto case serves that plan. Each case asserts the masking cause told to the harness, the call `Failed` with the session `Failed(MASKING)`, so no Terminal digest, no `8.7654321e8` in the answer or the ledger, and `leaks` empty. Its control serves `8.7654321e7` on each surface and expects the answer as written and the session admitted. `the_final_guard_reads_the_very_bytes…` now expects `{"t":"8.7654321e8",…}` to refuse; it wrongly expected a pass before. It adds a key, an escaped-digit string (`8.7654321e8`) and a glued string, all refused, and an unrelated `8.7654321e7 v2.0` that passes. The text-redaction and unrelated-number controls (`the_canary_as_text_is_masked…`, `a_bound_number_is_refused_by_its_value…`) are unchanged and pass. |
| L1 (low), production bound the ledger too late | `secret::send_environment` now returns the bindings it looked up beside its answer, `(Vec<BoundSecret>, Result<(), Refusal>)`, whether or not the frame was sent. None come back where the lookup itself refused. The frame's building moved into a private `framed`. `readiness::handed` carries the sender thread's value back. `delivered` gives the bindings to a `bind` closure as soon as the sender answers, before the send's answer, the control pipe's close or the exec status is read. `ready` takes that closure. `session::lifetime::lived` now owns a `OnceCell` that keeps the bindings for the ledger's lifetime and hands the session a `Binder`. `Binder::bind` keeps them and calls `Ledger::bind`, which re-reads the Opened and guards every later record, the Closed included. `admit` passes the binder to `ready`. The serving refusal is unchanged. | `session::lifetime::tests::bindings_given_once_known_guard_the_closure_a_later_cause_writes` drives `lived` with a session that binds STATE and then ends with the protocol cause. STATE=closed gives `Err(Failed(Masking))`, with only the Opened verifying (`([], None, Some(Lifecycle))`). STATE=opened gives `Err(Failed(Masking))` from the Opened's re-read, with the closure `Failed(Masking)`. The unrelated control keeps `Failed(Protocol)` and its closure. `readiness::tests::the_bindings_are_looked_up_after_readiness…` now also asserts the names the binder gets: `DOCS_TOKEN` after a send, after a bootstrap gone before the frame, and after a bootstrap's reported cause; none for a missing name. A binder's refusal (`Masking`) is the answer before the send's own cause. `secret::tests::the_sender_frames_the_injectors_environment…` asserts the bindings handed back when the transport refuses (`Exec`), when it sends, and none for a name the store lacks, with that refusal's exact text. The production lifetime: `capability_broker::protocol::a_binding_the_startups_closure_would_hold_ends_the_session_masked_and_unclosed` serves a real box whose store binds `DOCS_TOKEN=closed` and whose server speaks no MCP. It expects exit 1 with `error: MCP response cannot be safely masked`, one lookup, and a ledger whose Opened alone verifies. The box stood in this sandbox: the test took 2.85 s and printed no skip. |
| L2 (low), two derivations of Opened | `Lifecycle::opened` in `broker/ledger.rs` is the one derivation (decision 0071 ruling 5). `Ledger::open_with` appends it and `Ledger::bind` re-reads it. | `broker::ledger::tests::binding_rereads_the_very_opened_the_ledger_appended` pins the appended bytes exactly: `{"record":"opened","seq":1,"owner":{…},"plan":"5ea1ed…"}\n`. It binds that whole line, newline aside, as STATE and expects `Err(Masking)` with the appended bytes unchanged. The control, an unrelated binding, gives `Ok(())`. |
| L3 (low), F5's removal evidence | No code change. The removal is recorded below. | `machine_proof::output_edge::the_output_edge_masks_with_the_shared_masker_and_serializes_once`. |

One existing test moved because of H1's rule.
`capability_broker::ledger::every_record_is_masked_and_guarded_before_it_is_appended`
bound `3.00` to refuse the third record's sequence number. The fixture
owner's effect UUID `3f6c2a0e-…` now spells 3, so the Opened itself
refused. That is the ruled cautious behaviour. The case now binds `11.00`,
a value no earlier record spells. Four calls succeed, and the fifth call's
Terminal, the eleventh record, is refused. The test's purpose and
assertions are otherwise as before.

Of the INFO items, none was taken: none was both cheap and in the row
beside these four repairs.

### Removals

Each removal below compiled. It was run against the named tests, then
restored by the reverse edit, and the tests passed again.

| Removal | Failing test and assertion |
| --- | --- |
| H1: `Guard::passes` skips the decoded scan for JSON again (`match json { true => Vec::new(), false => numbers(&decoded, false) }`) | `a_bound_number_a_string_spells…` failed at `masking_failed` (masking.rs:176): left `(("text", "on"), …"text": "pin 8.7654321e8"…)`, right the masking cause. The same removal was run with the surface list cut to each other surface alone: `("structured", "on")` delivered `"structuredContent": {"pin": "8.7654321e8"}`, `("key", "on")` delivered `{"8.7654321e8": 1}`, and `("error", "on")` delivered the error `"pin 8.7654321e8"`. `the_final_guard_reads_the_very_bytes…` failed at `("{\"t\":\"8.7654321e8\",…}", true)`. |
| L1: `Binder::bind` keeps the bindings but never calls `Ledger::bind` | `bindings_given_once_known…` failed: left `(Err(Failed(Failed(Protocol))), ([], Some(Failed(Protocol)), None))`, right `(Err(Failed(Failed(Masking))), ([], None, Some(Lifecycle)))`, which is the finding itself. |
| L1: `delivered` reads the send's answer before binding (`sent?; bind(bindings)?`) | `the_bindings_are_looked_up_after_readiness…` failed at its missing-name case: left binder names `None`, right `Some([])`. |
| L1: `admit` hands the bindings to a no-op (`\|_\| Ok(drop(binder))`) | `capability_broker::protocol::a_binding_the_startups_closure…` on a real box failed: left `((Some(1), "error: MCP server protocol is invalid\n"), 1, ([], Some(Failed(Protocol)), None))`, right the masking exit and the unclosed ledger. |
| L2: `Ledger::bind` builds its own Opened (`seq: 0`) instead of `Lifecycle::opened` | `binding_rereads_the_very_opened…` failed at the bind: left `Ok(())`, right `Err(Masking)`. |
| L3 (F5): `output::redact` converts by `String::from_utf8_lossy(&secret::mask_bytes(…)).into_owned()` instead of `mask_text` | `the_output_edge_masks_with_the_shared_masker…` failed at output_edge.rs:66. Its `mask_text(` call sites were left `[named, prepared, keep, keep]`, right `[named, prepared, redact, keep, keep]`. |

### Files

`crates/brokkr-cli/src/broker/session.rs` was at its 800-line ceiling, and
binding in `admit` would have pushed it past. Under the operator's standing
rule, `admit`, the code this change touches, moved into its existing private
child `session/lifetime.rs`, beside `lived`. `session.rs` re-exports it on
its existing `use lifetime::…` line (`pub(super) use lifetime::{admit,
Ended};`), so its callers and its own line count of that re-export are
unchanged. One new test file is registered as `#[cfg(test)] mod tests;` in
`lifetime.rs`: `crates/brokkr-cli/src/broker/session/lifetime/tests.rs`.
Its owner and store helpers are written so that jscpd finds no clone of the
ledger suites' helpers. A test in
`crates/brokkr-cli/tests/machine_proof/egress.rs` pins `delivered`'s
signature as the grep reads it, now `delivered<E: From<Refusal>>`.

`wc -l` measured `session.rs` 774, `session/lifetime.rs` 178,
`session/lifetime/tests.rs` 98 (new), `session/readiness.rs` 533,
`session/readiness/tests.rs` 828, `broker/ledger.rs` 608,
`broker/ledger/tests.rs` 189, `secret/guard.rs` 405, `secret/handoff.rs`
340, `secret/tests.rs` 1,280, `rpc/tests/masking.rs` 1,457,
`tests/capability_broker/ledger.rs` 1,025 and
`tests/capability_broker/protocol.rs` 179. `quality/file-lines.txt` carries
each count. `output.rs`, `rpc.rs`, `rpc/calls.rs` and `secret.rs` were not
changed in this run.

No public item is added. One public signature changes:
`secret::send_environment` returns `(Vec<BoundSecret>, Result<(), Refusal>)`
instead of `Result<(), Refusal>`, so that L1's bindings reach the ledger from
the handoff's own lookup. Decision 0012 keeps every read of a value in the
secret module: the CLI receives `BoundSecret`s, as `resolve_bindings` already
returns them, and reads none. `expose_for_spawn`'s one call site is
unchanged, as the machine proof below shows. The snapshot
`quality/public-api/brokkr-protocol.txt` carries the new line. The operator
rules on it at PR time.

### Measurements and gates, seventh run

`cargo crap` measured every new or changed function within 15. In the
protocol: `Guard::passes` 6, `send_environment` 5, `framed` 4, `Ledger::bind`
4, `Ledger::open_with` 2 and `Lifecycle::opened` 1. In the CLI: `ready` 13,
`delivered` 8, `readiness::handed` 3, `admit` 2, `lived` 2 and `Binder::bind`
1. `Lifecycle::admit`, at 15, is untouched. No suppression was added.

| Gate | Observed |
| --- | --- |
| brokkr-cli | lib 759 passed (1 ignored), 116 of them under `broker::`. `it` 598 passed (3 ignored), with 64 under `machine_proof::`. `driver_conformance` 27, `transcript_surfaces` 13 and each `heap_*` 1 passed. |
| brokkr-protocol | lib 889 passed (3 ignored). The integration tests (6 and 1) and the doctest passed. |
| Rest of the workspace | `cargo test` over every other member passed, brokkr-runtime's lib 941, `capability_launch` 75 and `it` 132 among them. |
| Toolchains, lints, records | `cargo +1.88 check --workspace --all-targets --all-features` finished. The format check, workspace clippy (all targets and features, locked, `-D warnings`), `typos --hidden` and `git diff --check` reported nothing. `openspec validate --all --strict`: 19 passed. `recipes/self` compiled: 47 valuations over 11 groups, 0 unruled. |
| Ratchets | Files, duplication and public API held, the API after its snapshot took the one changed signature. `baselines origin/main` lists "public-api/brokkr-protocol.txt: 1115 public items (was 1103)", the earlier runs' ruled growth, unchanged in count by this run, and asks for the PR's `Ruling:` line. |
| Coverage diagnostic | `cargo llvm-cov` over the protocol lib, the CLI lib and `it capability_broker:: machine_proof::`. Every line this run added or changed in `secret/guard.rs`, `secret/handoff.rs`, `broker/ledger.rs`, `session/lifetime.rs` and `session/readiness.rs` is hit. `admit`'s binder closure was hit 53 times through real `serve` runs. This is a diagnostic, not the gate. |

One existing timing test missed its bound under load:
`capability_broker::protocol::a_server_that_never_answers_ends_at_the_startup_deadline`
asserts 29 s to 32 s. It failed once when `capability_broker::protocol` ran
its six tests in parallel, and once under instrumentation. It passed alone
(31.54 s) and in the full `it` run above. It is unchanged by this unit.

### Corrections to the sixth run's record

The sixth run's "Order, and what Opened holds" said the broker process
never holds a `BoundSecret` in this unit, and that the production ledger's
Opened and Closed are guarded by no binding. Both were the finding (L1). The
handoff's sender thread resolves the bindings in this process. They now come
back to the session and reach the ledger before anything after their lookup
can close it. The sixth run also said F5 had no behaviour of its own to
remove. Its grep proof does bind it, as the L3 removal above shows.

### Pending, seventh run

The exact-coverage gate, the controller's two-CPU host run of the real-box
tests, macOS, the `Ruling:` line's read and remote CI are pending, and so
is the offline link check (`lychee`), which this sandbox again refused. Real
boxes stood in this sandbox for `capability_broker::`, but that is not the
host's run. The suites used the default `TMPDIR`, because the sandbox
refused both an environment prefix and an owner-only directory under
`/var/tmp`. A sender given up at the startup deadline is not waited for
(operator ruling 2026-10-11), so a lookup it finishes after that never
reaches the ledger. Its frame is then never sent either, and the session
ends with the handoff cause. This is recorded as a residual, not repaired.

## Eighth run: the guard reads a frame's numbers in one pass (controller's state, 2026-10-13)

The seventh run passed the council, but CI failed on Linux, macOS and the
exact-coverage gate. Two existing tests failed:
`rpc::tests::an_answer_that_with_the_callers_own_id_passes_mb3s_bound_ends_the_session`
and `rpc::tests::delivery::what_waits_for_the_harness_is_bounded_and_taken_whole_in_order`.
The controller traced the cost to the guard's number reading. This run
repairs that reading and changes nothing else. The seventh run's net diff is
kept as it was. The only production file this run changes is
`crates/brokkr-protocol/src/secret/guard.rs`. `secret.rs` and every CLI file
are unchanged here, and no public item moves.

### What changed

Run 7's `numbers()` called `spelled` at every byte that no digit came just
before, through closures and iterator adaptors. It ran twice per frame, and
it built the decoded copy even when nothing was escaped. `Numbers` now
replaces it. It is an iterator over one reading of the bytes, and it calls
the full reader only at a digit, a sign or a point. Those are the only bytes
`spelled` can begin a number at, so the bytes between them are passed over
one comparison each. A JSON string's bytes are passed over in the same way:
an escape skips two bytes and a closing quote ends the string. Where the
reading is not `quoted`, a number that begins with a digit moves the reading
past its run of digits and the byte after it. Every one of those bytes has a
digit just before it, so run 7 read no number from any of them.

Runs of digits are counted by `Runs::end`. It remembers the last three runs
it counted, so a number and the numbers that overlap it (`-1.5e-3`, then
`1.5e-3`, `5e-3`, `-3` and `3`) share each run instead of counting it again.
A temporary instrumented check, removed before the commit, recorded the
start of every run counted. Over the named cases and the 4,000 generated
inputs below, in both modes, no start was counted twice. Each number is
handed back as a slice of the bytes, and nothing is allocated per number.

`unescaped` now returns a `Cow`. Where `bytes` holds no backslash it returns
`bytes` itself, so the guard neither builds the copy nor searches it for
needles a second time. `readings` chains the guard's two readings: the bytes
as JSON writes them (or as text), then the text they decode to. A JSON frame
keeps both readings, because JSON's tokens outside strings and the numbers
its strings spell are different sets. A text with no backslash is read once,
because its second reading would be its first again. The set of numbers
compared is the same as before on every surface, and so are their order and
`Guard::valued`'s reading of each, either sign and the vast-exponent rule
included.

Each reading also yields `Reading::Stretch` once its position passes the end
of a `STRETCH` (64 KiB). `passes` polls its caller on each one, as it
already did before every `BATCH` of compared numbers. As this run left it, a
long span with no number was polled within, but a run of digits was counted
whole before the next mark, so an 8 MiB run went unpolled. This section
first claimed otherwise; the review found it wrong, and the ninth run below
repairs it. The needles are still searched before any number is read.
`json_numbers`, which the precision check in `broker/output.rs` shares,
reads through the same iterator and drops its stretches.

### Equivalence with the seventh run's reader

Run 7's `numbers()` and `spelled()` moved verbatim into a new child test
module, `crates/brokkr-protocol/src/secret/guard/tests.rs`. It is declared as
`#[cfg(test)] mod tests;` in `guard.rs`. There they are the reference, and
production no longer contains them, so they are not a clone. `agrees(bytes)`
asserts three things. First, `Numbers` yields exactly the reference's slices,
in order and with no stretch, for both `quoted` modes. Second, `unescaped`
borrows exactly when the bytes hold no backslash. Third, `readings` yields
the reference's `numbers(bytes, json)` followed by `numbers(decoded, false)`.
For a text with no backslash it yields only the first of these, and the test
asserts that the omitted second reading equals it.

| Test | What it covers |
| --- | --- |
| `the_reader_reads_every_number_the_reference_reads_in_order` | It covers 48 named inputs: empty and one-byte inputs; lone signs, points, `e`, a quote and a backslash; leading, doubled and mixed signs; points before, after and between digits; exponents with and without a sign, dangling, doubled and chained, and after a point; digits glued to letters; `12-3` and `1-2+3`; JSON arrays and objects whose keys and values spell numbers; escaped quotes and backslashes; `\u` digits; surrogate pairs and lone surrogates; `\n`/`\t` beside digits; and a trailing backslash inside and outside a string. |
| `the_reader_agrees_with_the_reference_on_generated_inputs` | It generates 4,000 inputs with a fixed-seed xorshift (seed `0x2026_1013_0065_6f1a`). Each input is up to 24 pieces drawn from digits, `+ - . e E`, a quote, a backslash, a space, `u`, letters, `5`, `\ud83d`, `\ude00`, `\n` and `\"`. |
| `a_long_reading_is_marked_every_stretch_and_its_numbers_read_whole` | A string of letters three stretches long yields exactly three stretches between its numbers in both modes. A run of digits as long is one number, read whole, followed by one stretch. |
| `secret::tests::the_guard_polls_its_caller_through_a_long_reading_and_stops_at_its_word` | A JSON frame with three stretches of letters and one number is polled exactly 7 times: 3 stretches in each of its two readings and one batch. The bound value spelled `8.7654321e8`, placed after three stretches of letters or after a run of digits as long, is never compared once the second poll answers `Err`. With every poll answering `Ok`, the same frame refuses. |

Run 7's `the_guard_polls_its_caller_between_bounded_batches_and_stops_at_its_word`
is unchanged and still counts 5 polls. Its frame is shorter than a stretch.

### Removals

Each removal below compiled, failed the named tests and was then restored.
The suites in the gate table below ran on the restored tree.

| Removal | Failed |
| --- | --- |
| The reader drops the numbers inside decoded strings: the quote arm in `Numbers::before` loses its `if self.quoted` guard, so an unquoted reading also skips strings. | `the_reader_reads_every_number…` and `the_reader_agrees…` at guard/tests.rs:77 (`(bytes, quoted, read)` against the reference), and `a_long_reading_is_marked…` at :187. The CLI's `masking::a_bound_number_a_string_spells_fails_its_forwarded_call_on_every_surface` failed at masking.rs:176, and `masking::the_final_guard_reads_the_very_bytes_and_every_form_they_decode_to` at :1163. These are the seventh run's string-number regressions. |
| The scan is never marked: `let end = self.marked + usize::MAX / 2`. | `a_long_reading_is_marked…` at guard/tests.rs:187, and `the_guard_polls_its_caller_through_a_long_reading…` at secret/tests.rs:648 (`(passed, polls)`, expecting `(Ok(true), 7)`). |
| An escaped text is read once: `let again = json \| (escaped & false)`. | `the_reader_reads_every_number…` and `the_reader_agrees…` at guard/tests.rs:93 (`(bytes, json, read)`). |

### Timings

The two tests ran alone in a debug build (`test` profile), pinned to CPU 0
through cargo's runner:
`cargo test --config 'target.x86_64-unknown-linux-gnu.runner="taskset -c 0"' -p brokkr-cli --lib -- --exact <test>`.
Each figure is libtest's "finished in" for one run on this host. "Guard off"
means `Guard::passes` answered `Ok(true)` at once. That bypass was
measurement-only, and it was removed before any verification.

| Test | Run 7, guard on | Run 7, guard off | This run, guard on | This run, guard off | Ratio, this run |
| --- | --- | --- | --- | --- | --- |
| `an_answer_that_with_the_callers_own_id_passes_mb3s_bound_ends_the_session` | 9.26 s | 3.55 s | 2.20 s, 2.17 s | 1.86 s | 1.18× |
| `delivery::what_waits_for_the_harness_is_bounded_and_taken_whole_in_order` | 22.77 s | 13.83 s | 13.84 s, 13.79 s | 13.57 s | 1.02× |

Run 7's guard-off figures are higher than this run's because `json_numbers`
still used the old reader there, and the precision check calls it even with
the guard bypassed. Against run 7's own guard-off figures, this run's
guard-on times are 0.62× and 1.00×. The tests' sources, sizes, bounds and
deadlines are unchanged. The controller's own one-CPU measurement is
pending.

### Files

`wc -l` measured `secret/guard.rs` at 542 and `secret/guard/tests.rs` at 197
(new). `secret/tests.rs` is at 1,315. `quality/file-lines.txt` carries each
of these counts, set by hand from `wc -l` in the row order that
`quality/lib.sh`'s `file_lines` sorts. No other production file changed. The
public API is unchanged by inspection: `Guard`, its methods and
`json_numbers` keep their signatures, and `STRETCH` is `pub(super)` inside the
private `guard` module, for the polling test.

### Measurements and gates, eighth run

No suppression was added. The workspace's denied clippy lints
(`too_many_lines` and `excessive_nesting` among them) passed over every new
function. `cargo crap` was not run, because it needs the exact gate's LCOV,
so each function's measured complexity is pending with that gate.

| Gate | Observed |
| --- | --- |
| brokkr-protocol | `cargo test -p brokkr-protocol --all-features --locked`: lib 895 passed (3 ignored), and the integration tests (6 and 1) and the doctest passed. |
| brokkr-cli | The lib had 759 passed (1 ignored). In `it`, `capability_broker:: machine_proof::` had 141 passed and the rest 457 passed (3 ignored). `driver_conformance` 27, `transcript_surfaces` 13 and each `heap_*` 1 passed. |
| Formatting, lints, records | The format check and workspace clippy (all targets and features, locked, `-D warnings`) reported nothing, and so did `typos --hidden` and `git diff --check`. `openspec validate --all --strict`: 19 passed. `recipes/self` compiled: 47 valuations over 11 groups, 0 unruled. |
| Coverage diagnostic | `cargo +nightly-2026-09-05 llvm-cov -p brokkr-protocol --lib --branch -- secret::` hit every line `Numbers`, `Runs`, `readings` and the changed `passes` and `unescaped` hold. The only zero counts in that range were closing braces. This is a diagnostic, not the gate. |

### Pending, eighth run

This seat's sandbox refused to run `quality/ratchet.sh files` and `clones`,
`jscpd` itself and `quality/measure.sh`. All three are pending, and so is
the API ratchet. The other workspace members' suites were not rerun, because
none of them reads the guard. The exact-coverage gate, the controller's
one-CPU timings, macOS and remote CI are also pending.

## Ninth run: a run of digits is polled within (review of 6597de45)

The chief review of the eighth run's head returned a medium finding (M1).
`Runs::end` counted a whole run of digits before `Numbers::next` could mark
a stretch, so an 8 MiB run was read with no poll inside it. The review also
returned two new test clones in `secret/tests.rs` (I1). This run repairs
both. The only production file it changes is
`crates/brokkr-protocol/src/secret/guard.rs`, and no public item moves.

### What changed

`Runs::end` now takes the end of the current stretch as a limit. A run that
is still going at that limit stops counting there, and `Runs` keeps where it
began and how far it was counted (`counting`). `spelled`, `read` and `before`
pass the limit down and return `None` through it. `before` then marks the
stretch at the limit and yields `Reading::Stretch`, leaving its position on
the number's first byte. On the next call, `spelled` starts again from that
byte. The runs it has already counted come from `known`, and the run that
stopped resumes from where its count stopped. So no byte past a stretch's
end is read before that stretch is marked. The one exception is the byte
the stop test itself examines. By construction a resumed run is not
counted again from its start, though no test observes that. The number is
still handed back whole, as one slice, after the stretch its last
digit falls in. The set of numbers, their order and their slices are
unchanged.

The two poll tests in `secret/tests.rs` now share one helper, `polled`. It
returns the guard's answer and its poll count, with an optional poll that
fails, so the counting and stopping closure is written once.

### Tests

| Test | What it now asserts |
| --- | --- |
| `secret::guard::tests::a_long_reading_is_marked_every_stretch_and_its_numbers_read_whole` | Three stretches of letters are still marked three times in both modes. A run of digits three stretches long, followed by ` 2`, yields `[stretch, stretch, the whole run, stretch, 2]` in both modes. `1e` followed by a run of digits as long yields three stretches, then the whole number. Where not quoted, it then also yields the digits after the letter as a number of their own, as the reference reads them. |
| `secret::tests::the_guard_polls_its_caller_through_a_long_reading_and_stops_at_its_word` | The frame `["<three stretches of 7>"]` passes with exactly 7 polls: 3 in the JSON reading, which passes over the string, 3 within the run in the reading of the text it spells, and 1 before the run is compared. The eighth run's assertions are kept: 7 polls for the letters frame, and a stop at the second poll before `8.7654321e8` after letters or after digits. |
| `secret::tests::the_guard_polls_its_caller_between_bounded_batches_and_stops_at_its_word` | It keeps its 5 polls and its stop at the second poll. A refusal of the bound value is now pinned at exactly 3 polls, where the old test checked only the answer. |

The equivalence tests (`the_reader_reads_every_number…` and
`the_reader_agrees…`, 48 named cases and 4,000 generated inputs) are
unchanged and pass. The run 7 reference is unchanged.

### Removals

Each removal below compiled, failed the named tests and was then restored.

| Removal | Failed |
| --- | --- |
| A run of digits is counted whole: `if end >= limit && false` in `Runs::end`. | `a_long_reading_is_marked…` at guard/tests.rs:199, which read the run first with no stretch before it. `the_guard_polls_its_caller_through_a_long_reading…` at secret/tests.rs:651, `left: (Ok(true), 5)` against `right: (Ok(true), 7)`. |
| The reader drops the numbers inside decoded strings: the quote arm in `Numbers::before` loses `if self.quoted`. This is rerun on this head as the controller asked. | `the_reader_agrees…` and `the_reader_reads_every_number…` at guard/tests.rs:77, `a_long_reading_is_marked…` at :190, and `the_guard_polls_its_caller_through_a_long_reading…` at secret/tests.rs:651. In the CLI, `broker::rpc::tests::masking::a_bound_number_a_string_spells_fails_its_forwarded_call_on_every_surface` failed at masking.rs:176 and `masking::the_final_guard_reads_the_very_bytes_and_every_form_they_decode_to` failed at :1163. The masking module had 31 passed and 2 failed. |

A resume that is dropped, so every stretch counts its run again from the
start, changes no reading, only the cost. No test can see it, so it is not
claimed as bound.

### Timings, ninth run

Each test ran alone in a debug build, pinned to CPU 0, with the same
command as the eighth run. "Guard off" means `Guard::passes` returned
`Ok(true)` at once. That bypass was used only for this measurement, and it
was removed before the suites below ran.

| Test | Guard on | Guard off | Ratio |
| --- | --- | --- | --- |
| `an_answer_that_with_the_callers_own_id_passes_mb3s_bound_ends_the_session` | 2.39 s, 2.35 s | 2.02 s, 1.99 s | 1.18× |
| `delivery::what_waits_for_the_harness_is_bounded_and_taken_whole_in_order` | 13.92 s, 13.90 s | 13.61 s | 1.02× |

The tests' sources, sizes, bounds and deadlines are unchanged. The
controller's own one-CPU measurement is pending.

### Gates, ninth run

| Gate | Observed |
| --- | --- |
| brokkr-protocol | `cargo test -p brokkr-protocol --all-features --locked`: lib 895 passed (3 ignored), the integration tests 6 and 1, and the doctest 1. |
| brokkr-cli | The lib had 759 passed (1 ignored), and `it` had 598 passed (3 ignored). `driver_conformance` had 27, `transcript_surfaces` 13, and each `heap_*` 1. |
| Ratchets | `quality/ratchet.sh clones`: duplication holds. It refused I1's two clones before this run. `files`: file size holds. `api`: public API holds. |
| Records and lints | The format check, workspace clippy (all targets and features, locked, `-D warnings`), `typos --hidden` and `git diff --check` reported nothing. `openspec validate --all --strict` passed 19. `recipes/self` compiled, with 47 valuations over 11 groups and 0 unruled. |

`wc -l` gives `secret/guard.rs` 565 lines, `secret/guard/tests.rs` 210 and
`secret/tests.rs` 1,309. `quality/file-lines.txt` carries each count. No
suppression was added.

### Residuals and pending, ninth run

The guard's other passes over a frame are still not polled within. These
are the needle search (`holds`), the escape decoding (`unescaped`), and the
reading of one compared number by `Magnitude::read`. Each is linear in the
frame. The controller measured that skipping the needle search, or the
escape decoding, changed nothing. They are named here as follow-ups, not
repaired, because the finding and the directive concern the number scan.
The exact-coverage gate, `cargo crap`, the controller's timings, macOS and
remote CI are pending.
