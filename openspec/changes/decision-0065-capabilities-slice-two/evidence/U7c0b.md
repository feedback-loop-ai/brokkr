# U7c0b evidence: observe a server box for provisioning, under the startup bound

U7c0b is delivered on branch `s2/U7c0b` as one commit cut from main at
`64027718`. Its first run, `0065-slice-two-unit-u7c0b-see-th-6286d937`,
cut from `d5dbb162`, committed `5d767d43`; review returned it
(REVIEW-REFORGE, findings M1–M4, L1, L2, I1), and its reforge `742f8eeb`
answered all of them but M2, then stopped IMPL-BLOCKED for an operator
ruling. The operator ruled on 2026-10-15 (below, and in U7c0b's section of
`design.md`). This second run, `0065-slice-two-unit-u7c0b-see-th-2fb16269`,
replays `d5dbb162..742f8eeb` onto `64027718` with that ruling, regenerates
the `quality/file-lines.txt` rows the replay left out, and reruns every
gate on the result. No production or test code changed in the replay. It
contributes to tasks 35.1 and 35.2, which close with U7c (requirements
MB3, SD4 and SC1). The mutation rows below were observed in the first
run's two visits on Linux x86_64. The gates, the API measure, the
real-host contrast and coverage were observed again in this run against
the commit it records. What was not observed is named pending at the end.

## What changed in production

The row's six production files moved, with one new private child under the
standing child-module rule, and on the return one existing private child of
a row file (`broker/session.rs`) changed.

In the protocol, `ServerBox::provision(program, profile) -> Result<Sources,
Refusal>` is the provisioning entry. It lives in the new private child
`crates/brokkr-protocol/src/hands/namespace/provision.rs`, declared from
namespace.rs (798 → 799 lines, at its ceiling otherwise). On Linux it runs
the very path `prepare` runs, `prepare_with` over `prepared` and
`served`, with `sources::Host::provisioning(profile.writers)` for the
host, and returns the observed `Sources`, letting the box, its handles and
its scratch go. Off Linux it is `ServerBox::prepare(..)?`'s sources, so it
refuses exactly as `prepare` does: reach and program-tree causes first,
then `Refusal::Unavailable`.

In `sources/host.rs`, `Host::live()` now builds through one private
`Host::with(credentials)`, and `Host::provisioning(sealed)` is that same
host with only its credentials replaced by the pure
`provisioned(overflow, sealed)`: the sealed uids sorted and deduplicated,
the overflow uid read from `/proc/sys/fs/overflowuid`, and
`Confinement::Proved`, as the plan seals `Privilege::Confined`. No uid
sealed, or an overflow file that does not read as one decimal line, yields
`Credentials::unknown()`, the value `credited` already returned for an
unread chain, now written once and used by both; the overflow parse is
likewise one function, `overflowed`, used by both. `excluded`,
`untouchable`, `Credentials::sealed`, `launched` and everything the broker's
admission-time observer reads are unchanged: admission still observes
itself, the broker and the harness, joins the sealed writers and proves
their confinement (`launched`'s `host.credentials.sealed(profile.writers)`
is idempotent over provisioning's already sealed set).

In protocol `broker.rs`, two records join the existing `records!`
invocation's closed decoding: `Survey { launch, home, reach, network,
writers }`, the request, and `Provisioned { executable, tree, bootstrap,
sources }`, the observed facts, built from the existing `Reach`, `Network`,
`Tree`, `Bootstrap` and `Sources`. The reply is `Surveyed`, a closed enum
`Provisioned(Provisioned) | Refused(Refusal) | Unread(Unread)`, written
`{"provisioned": {..}}`, `{"refused": "<kebab cause>"}` or
`{"unread": "<kebab cause>"}`. `Unread` is the closed, `thiserror` cause of
a request the observer could not read: `oversized`, `truncated`, `input` or
`malformed` (a repeated field, a field the record does not name, a missing
field or anything after the record). An enum cannot pass through
`records!`, so these variant names are their one derive's. Every wire name
has its one home in these types, which U7c writes and reads.

In the CLI, `cli_args.rs` takes only the hidden unit variant
`BrokerCmd::Survey` and its two-line doc comment (788 → 792 lines), and
`broker.rs` declares `mod survey;` and dispatches it. The new private
child `crates/brokkr-cli/src/broker/survey.rs` holds the whole body.
`request` reads standard input whole through `take(1 MiB + 1)` and returns
`Result<Survey, Unread>`: more than MB3's 1 MiB request bound is
`Oversized`, an input error `Input`, input that ends inside the record
(serde's EOF class, empty input included) `Truncated`, and anything else
`serde_json::from_slice` does not read as one closed `Survey` with only
whitespace after it `Malformed`. `surveyed` then reads HOME by the broker's
one rule, `broker.rs`'s private `canonical_home` (absolute and canonical),
which admission's `session::trusted_home` now calls too; a HOME that fails
it is `ProgramTree` here and stays `Unbound` at admission, each caller's own
cause. It resolves the launch through `ServerProgram::resolve`, which keeps
MB3's fixed search path and never the cwd or an ambient `PATH`; hashes
`std::env::current_exe()`, canonical, as the bootstrap (unread:
`Identity`); and calls `ServerBox::provision`. `run` writes exactly one
reply line on standard output, whatever happened, and nothing on standard
error, which keeps the broker's one stderr writer; it exits 0 for
`Provisioned` and 1 for `Refused` or `Unread`. The verb reads no store,
plan or secret. It writes no file but the box's scratch identity, as the
operator ruled (M2 below). The CLI's handwritten `Unread`/`Display` is gone; the operator text
lives once on the protocol enum's `thiserror` derive.

## Return from review

| Finding | Answer |
| --- | --- |
| M1, an unread request propagated before any reply | Answered. `run` writes `Surveyed::Unread(cause)` as its one reply, exit 1, nothing on stderr; the end-to-end test pins all three streams for an unknown-field, a truncated and an oversized request. |
| M2, provisioning writes `Session` scratch (passwd, group, hosts, nsswitch.conf) | Ruled by the operator on 2026-10-15: the scratch is within the contract. See below. |
| M3, mutations that stopped before the assertion they claimed | Answered. Every refusing condition and every malformed-request case now has its own compiling removal, each recorded at the assertion it reached (N1–N17). |
| M4, tests that compared `Display` strings | Answered. The CLI tests match `Err(Unread::…)` and `Surveyed::Refused(…)` variants; `Unread`'s texts are pinned once, in `broker::tests::an_unread_survey_request_says_why_in_its_own_words`. |
| L1, the HOME rule written twice | Answered. `canonical_home` in `crates/brokkr-cli/src/broker.rs` is the one rule; each caller keeps its cause. |
| L2, a handwritten error in place of `thiserror` | Answered. The cause is the protocol's `thiserror` enum `broker::Unread`; brokkr-cli keeps no error type of its own for the survey. |
| I1, the public API count misstated | Answered. Corrected under Public API: +26 listing lines, which the operator approved as measured on 2026-10-15. |

**M2, the operator's ruling (2026-10-15).** The box scratch that the hands
`Session` makes for the generated identity is within the verb's contract.
`Session` removes it on drop. After a kill it is left to the #415 reaper,
exactly as `serve`'s observer leaves it. The observation is not
redesigned. U7c0b's section in `design.md` now states it: the verb "writes
no file outside the box scratch that the hands `Session` makes for the
generated identity, which the observation digests as admission does".
The scratch is needed because `served` (`hands/namespace/sources.rs`)
observes the generated identity as `Role::Generated` sources: their names,
`Facts::made()` and the scratch mount are part of the `Sources` that
admission compares with the sealed ones. So provisioning has to write the
identity to seal a digest that admission can match. The scratch is a
locked `brokkr-hands-server-<pid>-<uuid>` tree under the temporary
directory. The reaper removes a killed survey's tree once its pid reads
dead and no process holds its lock, before the next run, resume or rerun
drives. This run changed no code for M2.

## Tests that bind

Every new test asserts exact values or variants. Each mutation below
compiled, made the named test fail at the quoted assertion, and was
restored; the restored tree then passed. Both visits of the first run
observed them. M1–M8, M12–M14, M16 and M17 come from its first visit and
still stand. The reforge (`742f8eeb`) rewrote the tests that M9–M11, M15
and M18 bound, and ran the N-rows, which replace them. This run's replay
changed no code or test line they reach, so it did not rerun them; every
test they name passed again in this run's suites.

| Mutation | Failing test | Observed failure |
| --- | --- | --- |
| M1 `Host::provisioning` returns `Host::with(credentials())` | `hands::tests::provision::over_a_real_linked_system_file_provisioning_admits_where_the_unconfined_prepare_refuses`; `capability_broker::survey::the_survey_replies_the_program_bootstrap_and_sources_provisioning_observes` | `provision(..).unwrap()` on `Err(Identity)` (after the test's `prepare == Err(Identity)` held); the binary exited `(Some(1), "")`, expected `(Some(0), "")` |
| M2 `provisioned` seals `Confinement::Unproved` | all three `hands::tests::provision` tests | `confinement: Unproved` against `Proved`; `excluded(&ROOTED, &sealed)` false; `Err(Identity)` |
| M3 `provisioned` drops its empty-set guard | `provisioning_credentials_are_the_sealed_writers_confined_beside_the_overflow_uid` | `{uids: [], overflow: 65534, Proved}` against the unknown value |
| M4 `provisioned` keeps the sealed list as given | same | `[1002, 1000, 1002]` against `[1000, 1002]` |
| M5 `overflowed` takes `trim_end` for its newline | same | `Some("65534")` read as `65534` against unknown |
| M6 `provision` observes with `Host::live()` | `over_a_real_linked_system_file_…` | `Err(Identity)` |
| M7 `#[serde(rename = "program")]` on `Survey::launch` | `broker::tests::the_survey_request_and_reply_write_the_wire_names_their_readers_read` | wire `"program"` against `"launch"` |
| M8 `Surveyed` loses `rename_all = "lowercase"` | same | `{"Provisioned": ..}` against `{"provisioned": ..}` |
| M12 an unread bootstrap refuses `Establishment` | `broker::survey::tests::a_missing_program_a_relative_home_or_an_unread_bootstrap_refuses_with_its_cause` | `("unread bootstrap", Refused(Establishment))` against `Refused(Identity)` |
| M13 a refused reply exits `SUCCESS` | `capability_broker::survey::a_box_refused_or_a_program_missing_replies_its_cause_alone_and_fails` | `(Some(0), "{\"refused\":\"program-tree\"}\n", "")` against `Some(1)` |
| M14 the reply's newline dropped | same, and `the_survey_replies_…` | stdout without its `\n`; `reply` found no line |
| M16 the bootstrap digest taken over its path | `the_survey_replies_…` | digest `6975…0368` against the copy's `082c…666d` |
| M17 a package tree replied as `System {}` | `the_survey_replies_…` | `tree: System` against `Package { root: …/opt/docs }` |
| N1 `excluded`'s owner check made `\| true` | `hands::tests::provision::sealed_confined_writers_admit_a_root_owned_linked_file_the_unconfined_chain_refuses` | provision.rs:96, `("a sealed writer owns it", true)` against `false` |
| N2 `excluded`'s write mask `0o022` → `0o002` | same | provision.rs:96, `("group may write it", true)` against `false` |
| N3 `excluded`'s write mask `0o022` → `0o020` | same | provision.rs:96, `("other may write it", true)` against `false` |
| N4 `excluded`'s ACL rule `== Absent` → `!= Unknown` | same | provision.rs:96, `("an access ACL", true)` against `false` |
| N5 `excluded`'s ACL rule `== Absent` → `!= Present` | same | provision.rs:96, `("an unread ACL", true)` against `false` |
| N6 `excluded`'s overflow-owner check made `\| true` | same | provision.rs:96, `("an unmapped owner", true)` against `false` |
| N7 `excluded`'s `mapped` drops the sealed-overflow-uid check | same | provision.rs:101, `assertion failed: !excluded(&ROOTED, &overflowing)` |
| N8 `provisioned`'s empty guard and `excluded`'s empty check both removed | same; and `provisioning_credentials_…` and `over_a_real_linked_system_file_…` | provision.rs:106, `assertion failed: !excluded(&ROOTED, &unknown)`; provision.rs:48, `{uids: [], overflow: 65534, Proved}` against unknown; provision.rs:175, `Ok(Sources { entries: 205078, mounts: 2, .. })` against `Err(Identity)` |
| N9 `request` parses through `serde_json::Value`, keeping the last of a repeated key | `broker::survey::tests::a_request_repeating_a_field_is_malformed`, alone | tests.rs:61, `Ok(Survey { launch: "sh", .. })` against `Err(Malformed)` |
| N10 `records!` drops `deny_unknown_fields` | `broker::survey::tests::a_request_naming_a_field_the_record_does_not_is_malformed`, alone among the survey tests | tests.rs:69, `Ok(Survey { .. })` against `Err(Malformed)` |
| N11 an EOF error mapped to `Malformed` | `…a_request_that_ends_inside_its_record_is_truncated`, alone | tests.rs:51, `Err(Malformed)` against `Err(Truncated)` |
| N12 `request` reads only the first record of a `StreamDeserializer` | `…a_request_followed_by_another_is_malformed`, alone | tests.rs:79, `Ok(Survey { .. })` against `Err(Malformed)` |
| N13 an input error mapped to `Malformed` | `…input_that_cannot_be_read_is_unread_input`, alone | tests.rs:93, `Err(Malformed)` against `Err(Input)` |
| N14 `request`'s bound `<=` → `<` | `…a_request_is_read_whole_within_its_bound`, alone | tests.rs:43, `Err(Oversized)` against `Ok(Survey { .. })` |
| N15 `request`'s bound raised by one byte | same, alone | tests.rs:45, `Ok(Survey { .. })` against `Err(Oversized)` |
| N16 `canonical_home` drops its `is_absolute` check | `…a_missing_program_a_relative_home_or_an_unread_bootstrap_refuses_with_its_cause`, alone | tests.rs:123, `("relative home", Refused(Identity))` against `Refused(ProgramTree)`: `src` resolves from the crate cwd |
| N17 an unread request returns `FAILURE` without a reply | `capability_broker::survey::a_request_it_cannot_read_replies_why_alone_and_fails` | tests/capability_broker/survey.rs:97, `(Some(1), "", "")` against `(Some(1), "{\"unread\":\"malformed\"}\n", "")` |
| N18 `Unread` exits `SUCCESS` | same | tests/capability_broker/survey.rs:97, `(Some(0), "{\"unread\":\"malformed\"}\n", "")` against `Some(1)` |
| N19 `Unread` loses `rename_all = "kebab-case"` | `broker::tests::the_survey_request_and_reply_write_the_wire_names_their_readers_read` | protocol broker/tests.rs:254, `{"unread": "Oversized"}` against `{"unread": "oversized"}` |
| N20 `Unread::Oversized`'s text changed | `broker::tests::an_unread_survey_request_says_why_in_its_own_words` | protocol broker/tests.rs:269, `"…is over its bound"` against `"…exceeds its bound"` |

The N-rows' line numbers are as the failures printed them, before
`cargo fmt` reflowed two assertions in the CLI test files.

One candidate was not a removal and is not counted: `Unread`'s
`rename_all = "kebab-case"` changed to `"lowercase"` passed, as it must,
since every variant is one word; N19 removes the attribute instead.

Each malformed-request test first proves its well-formed twin reads
(`written()` asserts `Ok`), so its one defect is what refuses it. The
record tests write each expected form by hand, now with the four `unread`
replies. A request missing `writers`, carrying `plan`, written as an array,
or naming `launch` twice refuses with serde's exact words; a reply naming
`admitted` (now listed against `provisioned`, `refused` and `unread`) or
`refused` twice refuses. `provisioning_credentials_…` pins the sorted,
deduplicated writers and the unknown value for an empty set and for each of
an absent, newline-less, non-decimal and empty overflow file.
`sealed_confined_…` shows a root-owned 0755, ACL-free file refused by an
unconfined chain joined with the sealed writer and admitted by
provisioning's credentials, both by `excluded` and by `untouchable`; each
refusing condition refuses under provisioning's and admission's credentials
(N1–N8 above). The program and bootstrap files' stricter single-link rule
is the observer's own `Linked` cause, which provisioning reaches through
the same `served` path unchanged.

The real-host test runs from the ordinary, unconfined test process. It
skips with a printed reason as root, under a confined chain, in a seat's
hands box, or where no multiply-linked file lies in `/usr/bin` or
`/usr/sbin`. Here it did not skip: under N8 it failed at its last
assertion, provision.rs:175, after `ServerBox::prepare(sh) == Err(Identity)`
and the admitted `provision` had held, observing 205,078 entries over 2
mounts. This run reran it on its own head. It printed no skip and passed.
The run also changed line 169's assertion for a moment, to
`provisioned.entries == 0` with the entry and mount counts as its message.
That run failed at provision.rs:169 with `205078 2`, which shows that the
test got past the contrast. The line was then restored. The end-to-end tests run the suite's singly linked copy of the real
binary (`CARGO_BIN_EXE_brokkr` copied, as every capability_broker test
does): the admitted one once for `sh` and once for a package planted under
a `/var/tmp` root, each reply equal to one written from
`ServerProgram::resolve`, the copy's path and sha256, and the in-process
`ServerBox::provision`'s sources.

`machine_proof::output_edge::the_brokers_own_standard_error_has_one_writer_and_one_voice`
pins every broker function that reads stdin or writes stdout. It names
`broker/survey.rs`'s `run` beside the call path and the observer's socket,
and it passed again in this run: `run` still reads stdin and writes stdout
alone, and writes no stderr.

Files: the unit adds
`crates/brokkr-protocol/src/hands/namespace/provision.rs` (production, 37
lines), `crates/brokkr-cli/src/broker/survey.rs` (production, 112),
`crates/brokkr-cli/src/broker/survey/tests.rs` (122),
`crates/brokkr-protocol/src/hands/tests/provision.rs` (176, registered in
`hands/tests.rs` under `cfg(target_os = "linux")`) and
`crates/brokkr-cli/tests/capability_broker/survey.rs` (146, registered in
`capability_broker.rs`, so in the existing `tests/it.rs` binary). No test
moved. The replay left `quality/file-lines.txt` at main's rows, so this run
set every row the unit moves by `wc -l` on its head, from main's
`64027718` baseline. In production: CLI `broker.rs` 120 → 131,
`broker/session.rs` 776 → 775, `cli_args.rs` 786 → 792, protocol
`broker.rs` 345 → 397, `hands/namespace.rs` 798 → 799 and `host.rs` 755 →
798, with the new `broker/survey.rs` at 112 and `namespace/provision.rs`
at 37. In tests: CLI `capability_broker.rs` 1984 → 1987 and
`machine_proof/output_edge.rs` 197 → 199, protocol `broker/tests.rs` 175 →
326 and `hands/tests.rs` 1735 → 1737, with the three new test files at
the counts above. Main's own row for `cli_args.rs` lagged its file, which
had 788 lines at `64027718`, so the unit adds 4 (the hidden variant and
its two-line doc). `quality/ratchet.sh files` held. No function passes a
ceiling, so `too-many-lines.txt` and `suppressions.txt` are unchanged, and
no suppression was added.

## Public API

Measured on this run's head, `quality/public-api/brokkr-protocol.txt`
goes from `64027718`'s 1136 lines to 1162: +26 listing lines, nothing
removed or renamed. That is exactly the rise the operator approved on
2026-10-15 for `ServerBox::provision`, `Survey`, `Provisioned`, `Surveyed`
and `Unread`. `quality/ratchet.sh api` held against the snapshot. (The
first run's first evidence called its +20 "four entries, inside 2–4". As
review's I1 found, that was wrong.) Every added line:

| Item | Listing lines |
| --- | --- |
| `pub enum broker::Surveyed`, its variants `Provisioned(Provisioned)`, `Refused(Refusal)`, `Unread(Unread)` | 4 |
| `pub enum broker::Unread`, its variants `Input`, `Malformed`, `Oversized`, `Truncated` (added on the reforge for M1) | 5 |
| `pub struct broker::Provisioned`, its fields `bootstrap`, `executable`, `sources`, `tree`, its `Deserialize` impl and `deserialize` fn | 7 |
| `pub struct broker::Survey`, its fields `home`, `launch`, `network`, `reach`, `writers`, its `Deserialize` impl and `deserialize` fn | 8 |
| `impl hands::ServerBox` (a second block, from the child module) and `pub fn hands::ServerBox::provision` | 2 |

`Surveyed` is the reply's closed sum the records need, and `Unread` is the
typed cause M1 requires the reply to carry. `brokkr-cli` is no library in
the listing; its new items are private.

## Gates

This run reran every gate on the replay over `64027718`. Formatting was
clean, and clippy with `-D warnings` finished clean across the workspace,
all targets and all features. The suites ran crate by crate in the
foreground:

| Suite | Result |
| --- | --- |
| `brokkr-protocol --lib` | 943 passed, 3 ignored |
| `brokkr-protocol` `hands_exits`, `secret_drop`, doc tests | 6, 1 and 1 passed |
| `brokkr-cli --lib` | 853 passed, 1 ignored; the `broker::` filter alone 157 passed |
| `brokkr-cli --test it`, `capability_broker::` | 102 of 103 passed (see below) |
| `brokkr-cli --test it`, everything else | 578 passed, 3 ignored |
| `brokkr-cli` `driver_conformance`, `transcript_surfaces`, `heap_claude`, `heap_codex`, `heap_dsh` | 27, 14, 1, 1 and 1 passed |
| `brokkr-runtime --lib` | 989 passed |

The one failure was
`capability_broker::observer::an_operation_blocked_to_the_deadline_refuses_with_nothing_left_running`,
at its timing assertion (observer.rs:891, the 30–32 s window), under a
host load average of 14–17. It passed alone in 34.28 s. The test exercises
`serve`'s observer, and this unit's only change on that path is
`trusted_home` calling the shared `canonical_home`. `recipes/self` and
`recipes/verify` both compiled (47 and 16 valuations, 0 unruled).
`openspec validate --all --strict` passed 26 of 26, `typos --hidden`
found nothing, `git diff --check` was clean, and `quality/ratchet.sh`
`files`, `clones` and `api` held.

This run also ran `cargo +nightly-2026-09-05 llvm-cov --branch` over:
the protocol library's `provision` and `broker::tests` tests, the CLI
library's `broker::` tests, and the CLI `it` binary's
`capability_broker::survey` and `capability_broker::serving` tests.
`--show-missing-lines` listed no line of `survey.rs`, `provision.rs` or
protocol `broker.rs`. In CLI `broker.rs`, `canonical_home` (101–108) and
the `Survey` arm (129) were hit, and in `broker/session.rs`
`trusted_home` (175–180) was hit. In `host.rs` none of the lines it
listed lies in a changed hunk (95–103, 410–425, 501–547), and both arms
of the changed branches at 507 and 531 were taken. The lines it listed
elsewhere in those files belong to paths this unit leaves alone.

Pending: the exact coverage gate and the CRAP ratchet on the host at
verify, macOS, and remote CI. This run added no platform-conditional code.
The first run's cfg gates were read by hand: only `provision.rs`'s Linux
arm names `sources`, and the Linux-only test module and the admitted
end-to-end test are `cfg(target_os = "linux")`. `brokkr-runtime`'s
integration binaries were not rerun: the unit changes no runtime file and
no witness or compose input.
