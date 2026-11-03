# U7a evidence: represent the complete server set

U7a is delivered by run `0065-slice-two-unit-u7a-see-the--a34f16c8` on branch
`s2/U7a`, cut from main at `ffcc72ab`, as one signed commit. It closes tasks
33.1 and 33.2 (requirements MB1 and SI2, with SD2 and SD4's shared duty).
Everything below was observed in this run's seat on Linux x86_64; what was
not observed is named pending at the end.

## What changed in production

Only the row's three production files moved, and no new production file was
added.

`native_controls/mcp.rs` is now the one typed MCP builder and its independent
reader. `SealedServers` moved here from `native_controls.rs` (re-exported
through the parent's existing `pub use mcp::{…}` line, whose line count is
unchanged) and gained `Brokered(Vec<SealedBroker>)`: the box's hands and one
broker per sealed holding. `SealedBroker` carries the capability, the plan
locator, the plan digest and the dialect secret names the harness, and so its
broker, is served without, read closed (`deny_unknown_fields`). The private
`Servers` set replaces the singleton transport intent: `brokkr` for the hands
and exactly `cap-<capability>` per broker, every broker the engine's own
executable running `broker serve --plan <locator> --plan-digest <digest>`,
never a dialect executable or the bootstrap. A broker beside no hands, a
capability that is not one bare name (ASCII letters, digits, `_`, `-`) and two
holdings under one name refuse; nothing is shortened, normalized or suffixed.
`Servers::expand` writes the whole set's document into the adapter's
`{hands_mcp_json}`; `Transport::expand` and `Transport::document` are the
hands-alone case of the same builder. The hands profile (`hands serve
--workdir --spec`, from `hands::serve_args`) and the server profile stay
separate.

The reader never rebuilds a document. A Claude-grammar command's MCP options
must be exactly one `--strict-mcp-config` beside one `--mcp-config` whose
JSON is a closed `mcpServers` table read with each name once (a name written
twice is refused, not read last-wins) of closed entries; Codex's must be
exactly the three `mcp_servers.<bare name>.<part>` assignments of each
server, their TOML basic strings decoded by a decoder written apart from the
encoder. Each entry is compared field by field with the sealed values: the
executable, the hands entry's workdir and parsed spec, each broker's plan and
digest. The removals the spawn applies must equal the brokers' sealed
removals as a set, each named once.

`native_controls.rs` carries the set at the existing control seam. `Serving`
gains `brokers` and `removed`; `sealed_inputs` takes the set from
`Serving::servers`, which refuses an unservable set with the final-set cause,
and judges the hands fragment (R1) against the whole set. `check_final` reads
the final command's server set after delivery and before `carried`, refusing
`provider '<harness>' final MCP configuration is not the engine's sealed
server set`. Under a sealed `harness` boundary the hands are a class, not a
server, as before. The file shrank from 4,302 to 4,298 lines (4,296 after
the re-vouch below) and `sealed_inputs` stays at 326.

`hands.rs` keeps `mcp_config` for its consumers (the serving edge and the
probe) as a call into the common builder; its bytes for the hands alone are
unchanged (the hands suite's `mcp_config` pin and every hands-document literal
in the protocol and runtime suites pass). 1,170 to 1,169 lines.

Out-of-row production code was not edited. `adapters/mcp.rs` still compares
the intent's server set with the one its sealed inputs declare, so a sealed
`brokered` set refuses at the serving edge before any provider work, under
either fence, until U7d carries it. Native OFF and authored provenance are
unchanged: the authored-option refusal suites pass untouched.

## Tests

| Suite | Test | What it proves |
| --- | --- | --- |
| `native_controls/tests/servers.rs` (new child) | `hands_and_two_brokers_check_exactly_beside_native_off` | MB1's three servers, written out by hand, check exactly; the deny list's native OFF stands beside them; removals in either order |
| same | `each_departure_from_the_three_server_set_refuses_before_spawn` | missing, hands alone, extra, duplicate, dialect executable, another executable, hands profile under a cap name, another plan, altered digest, a member beside the two, strict flag dropped, a file for the document, and a removal missing, extra or twice each refuse with the final-set cause |
| same | `the_empty_and_hands_only_sets_check_exactly_and_refuse_a_counterfeit` | the empty set as composed and with the empty document, the hands-only document equal to the existing literal; a three-server document where hands alone are sealed, a counterfeit hands server where none is, brokers beside no hands and an unsealed removal refuse |
| same | `a_server_name_is_never_repaired_into_another_holding` | dotted, spaced, empty and colliding capability names refuse though the command carries the very name |
| same | `codex_does_not_carry_a_broker_set` | Codex's measured fragment carries `brokkr` alone, so a broker set refuses as hands that are not the engine's; the hands alone check |
| same | `a_sealed_broker_set_is_read_closed` | the exact serialized form, read back equal; an extra or missing member refused |
| `adapters/tests.rs` | `a_sealed_broker_set_refuses_at_the_serving_edge` | the real serving edge refuses a sealed broker set under the standing and lifted fences (operator text pinned) and a counterfeit broker member is unreadable |
| `capability_launch/servers.rs` (new child) | `a_compiled_launch_is_checked_against_its_complete_sealed_server_set` | a real namespace compile with no grant: the boxed Claude seat and its Codex fallback check as composed with their hands set, and refuse once two brokers are sealed beside it |

Test moves: the mutation oracle's `unmatched` moved from
`native_controls/tests.rs` to `native_controls/tests/servers.rs` and now
answers a mutation that moves any MCP option with the final-set refusal, as
the check does. Pins that read MCP mutations of the final command moved to
the final-set cause: three Codex `--config` mutations and the raw, unencoded
escaped-path command in `native_controls/tests.rs`, and the Claude document replaced by a
file. `capability_launch.rs`'s `checked_against` reads sealed brokers from
the serving inputs, as the edge will. `every_single_mutation_of_a_checked_command_refuses`
keeps its tally (claude 15,243, codex 557, dsh 16, lanetally 15,649) and has
no survivor.

## Mutations

Each mutation compiled, made the named assertion fail, and was restored; the
suites passed again afterwards.

| Behaviour removed | Failed |
| --- | --- |
| the final-set check in `check_final` (`if false && …`) | `each_departure…` ("missing"), `the_empty_and_hands_only…` (counterfeit three-server document), the runtime test (candidate 0, `does not carry '--mcp-config'` instead of the final-set cause) and `every_single_mutation_of_a_checked_command_refuses` |
| `Servers::expand` writing the hands-alone document | `hands_and_two_brokers…` (R1 refusal instead of `Ok`) |
| the removal comparison (`… \|\| true`) | `each_departure…` ("a removal missing"), `the_empty_and_hands_only…` (unsealed removal) |
| the plan-digest comparison | `each_departure…` ("altered digest") |
| the duplicate-name refusal in `Unique` | `each_departure…` ("duplicate") |
| `strict == 1` loosened to `<= 1` | `each_departure…` ("isolation dropped") |
| the server count (`==` to `>=`) | `each_departure…` ("extra"), `the_empty_and_hands_only…` |
| `deny_unknown_fields` on the read entry | `each_departure…` ("closed entry") |
| the executable comparison | `each_departure…` ("another executable") |
| brokers admitted beside no hands | `the_empty_and_hands_only…` (brokers beside no hands checked `Ok`) |
| the bare-name check | `a_server_name_is_never_repaired…` ("dotted") |
| the collision refusal | `a_server_name_is_never_repaired…` ("colliding") |
| R1 judging the hands alone | `codex_does_not_carry_a_broker_set`, `hands_and_two_brokers…`, `each_departure…` |
| `deny_unknown_fields` on `SealedBroker` | `a_sealed_broker_set_is_read_closed`, `a_sealed_broker_set_refuses_at_the_serving_edge` (`NotSealed` instead of `Unreadable`) |
| the edge's sealed-set comparison in `adapters/mcp.rs` | `a_sealed_broker_set_refuses_at_the_serving_edge` ("standing": served the empty set) |
| the Codex approving-mode check | `the_hands_are_bound_to_the_engines_transport_and_nothing_else` (case 16, `"prompt"`) |

The first run of the brokers-beside-no-hands mutation did not fail: that case
also sealed removals the spawn did not apply, which refused first. The test
now applies the brokers' removals so the missing hands is the only departure,
and the mutation then failed it.

## Gates and measurements

On the final tree formatting and workspace clippy with warnings denied were
clean; `typos --hidden` and `git diff --check` reported nothing. Every
crate's suite passed, crate by crate: brokkr-protocol (921 lib tests and its
three integration binaries), brokkr-runtime (973 lib, `capability_launch` 76,
`it` 132 including `witness_digests`, and the rest), brokkr-cli (823 lib, `it`
658, and the five separately declared binaries), core, store, view, bridge
and the seatbelt probe. `recipes/self` compiled. `quality/ratchet.sh` holds
`files`, `clones` and `api` after this commit's rows. No witness or compose
pin moved, since the witness test passed unblessed.

`quality/file-lines.txt` rows: `native_controls.rs` 4,298, `native_controls/mcp.rs`
620, `hands.rs` 1,169, `native_controls/tests.rs` 12,816, the new
`native_controls/tests/servers.rs` 433, `adapters/tests.rs` 18,981,
`capability_launch.rs` 11,643 and the new `capability_launch/servers.rs` 80.
In `quality/too-many-lines.txt`, `the_hands_are_bound_to_the_engines_transport_and_nothing_else`
shrank from 151 to 148; no long function grew, and no suppression was added.
`scripts/measure-budgets.sh` rewrote the prompt, crate and heap budget files
with dates and prompt sizes that drifted on main; this unit touches none of
their inputs, so those files were restored and left as they were.

The brokkr-protocol public API rises from 1,116 to 1,124 items:
`SealedServers::Brokered`, `SealedBroker` with its four fields, and
`Serving::brokers` and `Serving::removed`. `quality/ratchet.sh baselines HEAD`
names the raise, which needs the pull request's ruling line.

## Recorded for later rows

`Serving::brokers`, `Serving::removed` and `SealedServers::Brokered` have no
production writer yet: U7b and U7c seal holdings (engine.rs) and U7d carries
them through the serving edge (`adapters/mcp.rs`), whose `final_set` still
compares the hands document it rebuilds; the control seam's independent
reading now covers the whole command beneath it. Until then a sealed broker
set refuses at the edge. These are LOW under ruling 6 and outside this row's
files.

## Re-vouch: the exact coverage gate and the complexity ratchet

The first re-vouch of #683 (run `0065-slice-two-unit-u7a-see-the--6dc4f9f7`,
on main at `b295eec3`) answers what the controller's host run of
`scripts/coverage-exact.sh` and `quality/ratchet.sh crap` found at `cf5c4c77`:
16 lines, 13 branches and one function of `native_controls/mcp.rs` unexecuted,
and `check_final` at cyclomatic complexity 16 over its baseline. Behaviour,
rulings and the public API are unchanged; the API snapshot still holds 1,124
entries and `quality/ratchet.sh api` holds.

`check_final`'s server-set judgment moved into one helper beside
`Serving::servers` in `native_controls/mcp.rs`, `Serving::sealed_set`, with
the same order and the same refusal: the set that cannot be served refuses
first, then the set the command carries. cargo-crap now measures
`check_final` at 15, within the house ceiling over its baseline of 14, which
was not raised. Measuring every function of `mcp.rs` also showed `toml_string`
at 23, a function new in this unit with no baseline entry, which full coverage
would not have cleared: its escape table moved verbatim into its own function,
`escaped`, and the two measure 11 and 14, the file's highest.

No arm was unreachable, so none was removed. Each miss is driven by a test:
the server-set seam where the final check can show the outcome, and a new
private test module, `native_controls/mcp/tests.rs` (91 lines), where the
seam shows only that a reading failed and not what it read or said.

| Suite | Test | What it drives |
| --- | --- | --- |
| `native_controls/tests/servers.rs` | `a_server_table_that_is_not_one_object_refuses` | `"mcpServers"` as an array, a string or a number refuses with the final-set cause (`Names::expecting`); a command carrying no MCP option where the hands are sealed refuses with it too (the `empty` short-circuit with hands present) |
| same | `a_server_name_is_never_repaired_into_another_holding` (extended) | a capability `library_docs` checks exactly as `cap-library_docs` (the `_` arm of `bare`) |
| same | `a_codex_assignment_outside_its_one_reading_refuses` | two- and four-part keys, a quoted server name, a part assigned twice to its own value, an undefined escape, a trailing comma and an array without commas each refuse with the final-set cause; a measured hands fragment carrying `--add-dir` beside its bindings is not the engine's workspace hands |
| `native_controls/mcp/tests.rs` (new) | `a_toml_basic_string_reads_back_every_escape` | `\b \t \n \f \r \" \\ / \U0001F600` and a raw tab decode to exactly their characters |
| same | `a_toml_basic_string_the_engine_never_writes_is_refused` | an undefined escape, a bare quote, a raw control character, a raw newline, DEL, `\u` and `\U` of too few digits, a signed `\u+02F`, a surrogate, a dangling backslash and an unquoted value read as nothing |
| same | `a_toml_string_array_reads_only_its_comma_separated_strings` | blanks around commas read exactly; a trailing comma, a missing comma and an unclosed string read as nothing |
| same | `a_config_control_of_two_values_or_a_quoted_name_is_no_reading` | a `--config` control of two values, and a complete server under a quoted name, read as nothing |
| same | `a_server_table_that_is_not_one_object_is_refused_in_its_words` | the reader's refusal is exactly `invalid type: sequence, expected an object naming each server once at line 1 column 14` |

Every new test was bound by a compiling mutation in this session. Each
mutation was applied alone, made the named assertion fail, and was restored;
afterwards the production diff against the start of the visit held only the
helper, the escape extraction and the test module's declaration.

| Behaviour removed | Failed |
| --- | --- |
| `Names::expecting` writes nothing | `a_server_table…_in_its_words` (`expected  at line 1 column 14`) |
| a control that is not one `--config` value is skipped (`continue`) | `a_config_control…` (`Some({})`), `a_codex_assignment…` (`--add-dir`: `does not carry '--add-dir'` instead of the hands refusal) |
| a key that is not three parts is skipped | `a_codex_assignment…` ("two key parts": `carries '--config' its sealed plan does not compose`) |
| a part assigned twice is read last-wins | `a_codex_assignment…` ("a part twice") |
| `\b`, `\t`, `\n`, `\f`, `\r` each decoded as its letter (five mutations) | `a_toml_basic_string_reads_back_every_escape`, each with that one character changed |
| `\U` refused | `a_toml_basic_string_reads_back_every_escape` (`None`) |
| an undefined escape read as its character | `a_toml_basic_string_the_engine_never_writes…` ("an unknown escape": `Some("/bin")`), `a_codex_assignment…` ("an unknown escape"); repeated after `escaped` was extracted, with the same two failures |
| a bare `"` read as itself | `a_toml_basic_string_the_engine_never_writes…` ("a bare quote") |
| control characters admitted, DEL still refused | same ("a control character") |
| DEL admitted | same ("DEL") |
| a raw tab refused | `a_toml_basic_string_reads_back_every_escape` (`None`) |
| the digit count in `scalar` | `a_toml_basic_string_the_engine_never_writes…` ("too few digits": `Some("/")`) |
| the hexadecimal-digit check in `scalar` | same ("a sign": `Some("/")`) |
| a trailing comma ends the array | `a_toml_string_array…` (`Some(["a"])`), `a_codex_assignment…` ("a trailing comma") |
| strings without a comma read on | `a_toml_string_array…` (`Some(["a", "b"])`), `a_codex_assignment…` ("no commas") |
| `_` dropped from `bare` | `a_server_name_is_never_repaired…` (`cap-library_docs` refused) |
| `empty` without `hands.is_none()` | `a_server_table_that_is_not_one_object_refuses` (`does not carry '--strict-mcp-config'`) |
| `!bare(server)` dropped from `codex_entries` | at first, no test: every name it is compared with is already bare, so at the seam a quoted name refuses anyway. The private test's quoted-name case was added for it and then failed (`Some({"\"brokkr\"": Entry { … }})`) |

Measured on the final tree in this seat: `cargo llvm-cov -p brokkr-protocol
--lib` executed all 54 functions and 367 lines of `native_controls/mcp.rs`,
and the pinned nightly's `--branch` report counts its 84 branches all taken.
The only unexecuted lines of `native_controls.rs` in that lib-only run are
`declared_values` (2820 to 2853), which this unit does not touch and which
only brokkr-runtime calls. cargo-crap over that LCOV reports every function
of `mcp.rs` at 100% with complexity at most 14. The full exact gate was not
run: it instruments the whole workspace, which exceeds this seat's
ten-minute command limit, and its boundary tests need namespaces.

`cargo test -p brokkr-protocol` passed (928 lib tests, then 6, 1 and 1 in the
integration binaries), `cargo test -p brokkr-runtime --test capability_launch`
passed 76, and `recipes/self` compiled. Formatting, workspace clippy with
warnings denied, `typos --hidden`, `git diff --check`, `openspec validate
--all --strict` (26 passed) and `quality/ratchet.sh` `files`, `clones` and
`api` all held. File rows moved: `native_controls.rs` 4,296,
`native_controls/mcp.rs` 644, `native_controls/tests/servers.rs` 523 and the
new `native_controls/mcp/tests.rs` 91. No listed long function changed
length, and no suppression was added.

## Pending

Exact coverage (`scripts/coverage-exact.sh`) and `quality/ratchet.sh crap` on
a capable host, macOS CI and the remote CI run on the final pull-request head
were not run here.
