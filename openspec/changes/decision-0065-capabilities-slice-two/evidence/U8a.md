# U8a evidence: protect artifact paths in workspace hands

U8a was built on branch `s2/U8a`, cut from main at `79877120`, in its second
run (`0065-slice-two-unit-u8a-see-the--64473a79`). The first run
(`0065-slice-two-unit-u8a-see-the--01a6bd8e`) parked IMPL-OVERSIZED-EXHAUSTED
with nothing committed, and its saved patch arrived staged in this worktree:
the protection in `box_argv`, the private children `hands/evidence.rs` and
`hands/environment.rs`, their tests and a seatbelt-probe fixture fix. That
patch derived the root as `<workdir>/.forge/artifacts` and created it from
hands. This run keeps the protection, points it at the root the engine
carries (operator ruling of 2026-10-17, U8a's paragraph in design.md), and
stops hands creating anything.

## The scope finding: three production files outside the row

Carrying the root through "the hands serve and exec arguments", as the
ruling requires, changes the verbs' argument vectors. Every producer and
parser of those vectors has to move in the same commit. Otherwise the
binary refuses its own engine's spawn (clap rejects `--artifacts`), or a
caller that cannot name a root builds no box. Three of them sit outside the
row and outside every module tree the row names:

| File | Why it must move | Change |
| --- | --- | --- |
| `crates/brokkr-cli/src/lib.rs` | `HandsCommand`, the clap definition of `hands serve` and `hands exec`, lives here, not in `cli/src/hands.rs` | `artifacts: PathBuf` (`#[arg(long)]`) on `Serve` and `Exec`; 1,894 → 1,900 lines; brokkr-cli public API +2 |
| `crates/brokkr-protocol/src/probe/observe.rs` | the adapter probe spawns `brokkr hands serve` through `mcp_config` and `serve_args` | the scratch world gains an `artifacts` directory beside its repository, handed to both; 509 → 516 lines |
| `crates/brokkr-cli/src/doctor.rs` | doctor's dialect-tool probe builds a box through `hands::execute` | the probe makes an empty root in its own session and passes it; 1,554 → 1,565 lines (baseline 1,572) |

The unit cannot be split around them, because the vectors change at once. These
edits are the smallest that keep every producer working, and they are made
here so the rest could be proven end to end. Every claim below was observed
with them in place. The row needs widening by these three files, plus the
brokkr-cli API rise, before this can land.

## Scope inside the row

| File | Change |
| --- | --- |
| `crates/brokkr-protocol/src/hands.rs` (frozen, baseline 1,169) | `box_argv`, `execute`, `execute_in`, `run_boxed`, `run_boxed_in`, `serve`, `handle`, `serve_args`, `mcp_config` and the `Executor` type take the artifact root; declares `mod environment; mod evidence;` and `pub use evidence::UNPROTECTED_EVIDENCE`. 1,169 → 1,156 lines. |
| `crates/brokkr-protocol/src/hands/evidence.rs` (new private child) | `Protected::observe(root, workdir)`, `ancestors`, `held`, `exclude`, the `Unprotected` causes and CR2's cause. 207 lines. |
| `crates/brokkr-protocol/src/hands/environment.rs` (new private child, staged) | `box_environment`, moved unchanged out of `hands.rs`. 63 lines. |
| `crates/brokkr-protocol/src/native_controls/mcp.rs` | `Transport::artifacts`; `ARTIFACT_ROOT`, the driver-input key; the hands server's arguments carry `--artifacts`; `Owed::is` compares it; `Servers::expand` refuses a non-UTF-8 root. 699 → 713 lines. |
| `crates/brokkr-protocol/src/native_controls.rs` (parent of the row's `mcp.rs`) | `ARTIFACT_ROOT` added to the existing `pub use mcp::{…}` line; 4,296 lines before and after. |
| `crates/brokkr-protocol/src/adapters/mcp.rs` | `hands_transport` binds the box's hands to the root the engine wrote under `artifact_root` in the dispatched input; an input without one binds the empty path, which no carried server matches. 785 → 790 lines. |
| `crates/brokkr-cli/src/hands.rs` | `serve` and `exec` pass the parsed root on. 59 → 66 lines. |
| `crates/brokkr-runtime/src/engine.rs` (frozen, baseline 3,630) | `compose_site`, `compose_site_at` and `compose_segments` take the root. `compose_at` makes it before launch under `namespace` and refuses the spawn where it cannot. `mark_serving` writes `artifact_root` beside the launch record and serving inputs. `hands_command` moved to the new child. 3,630 → 3,572 lines. |
| `crates/brokkr-runtime/src/engine/boxed.rs` (new private child, declared `mod boxed;`) | `hands_command` (moved; the exec argv gains `--artifacts`), `artifact_root`, `make_root` and the `Unrooted` causes. 145 lines. |
| `crates/brokkr-runtime/src/engine/isolation.rs` (existing child) | `served` and `beside` take the worktree pair and expand the hands beside the brokers under the same root. 283 → 284 lines. |

`docs/reference/cli.md` gains the two `--artifacts` rows and usage lines
exactly as clap renders them. `cli_reference_tests::the_cli_reference_is_rendered_from_clap`
failed before the edit and passes after it.

## The design as built

The one derivation of the root is the engine's: `artifact_root(&self.run_local())`,
which is `places::run_local(state, workdir).join("artifacts")` made absolute.
An adopter's root is under its `.brokkr/state`, a recipe library's under
`<workdir>/.forge`. `compose_at` makes it before launch for a site with hands
under `namespace`, the only boundary that builds a workspace box. It uses
`DirBuilder::recursive(true).mode(0o700)`, so every directory it makes is
owner-only, and an existing directory is kept as it is. It then observes the
root with `symlink_metadata`. A root that cannot be made or observed refuses
the spawn with `Unrooted::Unmade`, and a link or another kind of file with
`Unrooted::Foreign`. Both read "dispatch refused: capability evidence root
cannot be protected from every managed writer: …". Hands creates nothing,
so an adopter's `<workdir>/.forge` is never made.
`bootstrap_bench::a_pristine_scaffold_reaches_a_completed_run_with_no_agent`
passes in the full CLI integration binary.

The root reaches hands as a typed fact. The exec argv is
`brokkr hands exec --workdir W --artifacts A --spec S [--bundle-root B] -- …`.
The served hands are `brokkr hands serve --workdir W --artifacts A --spec S`,
built only by `serve_args` through `Transport`. The driver's final check
reads `artifact_root` from its input, which the engine writes when it seals
the launch, and compares it field by field with the carried server entry.

`Protected::observe(root, workdir)` refuses a root that is absent, reached
through a link or of another kind. It also refuses an artifact under
`sha256/` that is not one regular file with one link. It refuses a worktree
that is the root, lies inside it, or holds it by identity under another
spelling. The worktree holds the root when the root's spelling is under the
workdir and every directory between them is a directory of its own. Then
`box_argv` mounts each of those directories at its own path before the
declared binds, and the root read-only after every writable mount. A root
outside the worktree is mounted nowhere. Every writable or overlay mount
(private home and tmp, an outside git common directory, `rw` and `overlay`
binds) that is the root, holds it or lies inside it refuses. These are
judged by device and inode on the canonical path. The pins and the
read-only root are required mounts, so a mount bubblewrap cannot make fails
the box. `HandsSpec` keeps its closed vocabulary, `kind`, `network` and
`binds`.

## Proof

Each test below was run on this head, and each passed. Each mutation was a
compiling edit, run against the named filter, observed failing at the cited
line, and then restored. A grep for the marker came back empty, and the
suites passed again.

| Mutation | Test that failed | Observed failure |
| --- | --- | --- |
| M1: the read-only root mount removed from `box_argv` | `hands::tests::evidence::a_root_the_worktree_holds_is_pinned_and_mounted_read_only_after_every_writable_mount`; `…::a_box_cannot_change_the_evidence_it_reads`; `…::a_protection_mount_that_cannot_be_made_refuses_the_command`; `hands::tests::the_workspace_and_exec_argv_keep_their_exact_order_and_identity` | evidence.rs:126 (no `--ro-bind` of the root), :470 (a tamper exited 0 in a real namespace), :598 (the command ran), tests.rs:508 (argv) |
| M2: the ancestor pins removed | the first two above and the exact-order test | evidence.rs:124; :470 `mv .forge forge-old: [exit code: 0]`; tests.rs:508 |
| M3: `exclude` admits every mount | `…::a_writable_alias_of_the_artifact_root_refuses_the_box`; `…::a_root_outside_every_bind_is_mounted_nowhere_and_its_aliases_still_refuse` | evidence.rs:71 "the box was built" |
| M4: the "held under another spelling" refusal disabled | `…::a_linked_or_shared_artifact_refuses_the_box_by_its_own_cause` | evidence.rs:71, a box built over `spelled` |
| M5: the chain's directory check skipped | the same test | evidence.rs:71, the `forge` link case built |
| M6: an outside root bound writable | `…::a_root_outside_every_bind_is_mounted_nowhere_and_its_aliases_still_refuse`; `…::a_box_cannot_reach_a_root_outside_every_bind` | evidence.rs:152; :538 `cat …: evidence` read in a real namespace |
| M7: the root mount made optional (`--ro-bind-try`) | the held-root argv test; `…::a_protection_mount_that_cannot_be_made_refuses_the_command` | evidence.rs:126; :598 |
| M8: `Owed::is` stops comparing `--artifacts` | `native_controls::mcp::tests::the_hands_entry_is_read_with_its_artifact_root` (added because M8 survived every other test, the rebuilt hands segment catching it first) | mcp/tests.rs:167, the moved-root entry admitted; rerun after the fixture's rewording |
| M9: a non-UTF-8 root not refused by `expand` | `native_controls::tests::toml::a_transport_path_that_is_not_utf8_binds_no_hands` | toml.rs:190, a lossy root expanded |
| M10: `serve_args` drops `--artifacts` | `hands::tests::the_harness_config_names_this_binary_and_the_spec` and at least 29 more in the protocol suite (the listing was cut at 30) | literal and document pins |
| M11: the root made 0o755 | `engine::boundary_tests::evidence_tests::the_engine_makes_the_artifact_root_owner_only_or_refuses` | evidence_tests.rs:31, mode bits 0o045 left |
| M12: `compose_at` ignores a root it could not make | `…::a_box_whose_artifact_root_cannot_be_made_refuses_its_spawn` | evidence_tests.rs:90, `None` against the `Unmade` refusal |
| M13: `mark_serving` writes no `artifact_root` | the brokered serving tests (`a_provisioned_seat_is_served_the_whole_set_its_attempt_wrote` and four more) | brokered_tests.rs:504, the driver refused its final check |
| M14: the exec argv drops `--artifacts` | `engine::tests::an_exec_seat_with_hands_is_boxed_whole`; `…::a_zero_grant_exec_box_keeps_historical_evidence_without_an_authored_key`; `boundary_tests::every_boundary_arm_carries_the_links_segments_and_labels_its_own_as_hands` | tests.rs:5828; evidence_tests.rs:126; boundary_tests.rs:430 |
| M15: `make_root` accepts a linked root | `…::the_engine_makes_the_artifact_root_owner_only_or_refuses` | evidence_tests.rs:62, `Ok(())` |
| M16: `hands exec` hands the box its workdir as the root | `tests::the_hands_exec_verb_boxes_a_command_and_refuses_a_bad_spec`; `hands::hands_exec_runs_the_command_whole_and_returns_its_code` (CLI `it`) | cli/src/tests.rs:3995, the alias refusal; tests/hands.rs:368 |
| M17: `hands serve` likewise | `hands::hands_serve_lists_one_tool_and_runs_it_in_the_box` (CLI `it`) | tests/hands.rs:105 |
| M18: the driver ignores the engine-written root | `adapters::mcp::tests::the_hands_are_bound_to_the_artifact_root_the_engine_wrote` | adapters/mcp/tests.rs:130, the bound launch refused |
| M19: the served arguments ignore the transport's root | `native_controls::tests::the_engines_hands_transport_is_expanded_from_its_typed_inputs` (its new `artifacts` case) | tests.rs:7166 |
| M20: the root's absolutisation error swallowed | `hands::tests::evidence::each_cause_states_the_managed_writer_refusal` | evidence.rs:403, "entity not found" for the empty root |
| M21: the spec vocabulary admits `artifacts` | `hands::tests::evidence::no_authored_key_names_the_protection` | evidence.rs:418, `Ok(HandsSpec …)` |

The real-namespace proofs ran on this host and did not skip; no `skipped:`
line was printed. In `a_box_cannot_change_the_evidence_it_reads`, twelve
writes, truncations, removals, renames, hard and symbolic links, mode changes
and plants fail, from the workspace tool and from an exec box. The artifact,
the root and `.forge` keep their bytes, device and inode, the digest
directory still lists one name, and the artifact has one link. The worktree,
`.forge/result` and a declared bind stay writable.
`a_box_cannot_reach_a_root_outside_every_bind` proves an adopter's root
outside the worktree cannot be read, listed, written, removed or planted
beside, while the worktree write lands.
`a_zero_grant_exec_box_keeps_historical_evidence_without_an_authored_key`
runs the engine-composed exec argv, with an engine-made root, in a real
namespace. The box's own output shows the refusals ("Read-only file system",
"Device or resource busy"), and the historical artifact's identities are
unchanged.

Tests moved to stay inside the file-size ratchet, each unchanged except
where noted:

| From | To |
| --- | --- |
| `native_controls/tests.rs`: `toml_decoded`, `toml_strings`, `every_codex_transport_value_decodes_as_toml_to_exactly_itself` | `native_controls/tests/toml.rs` (new child); the test's non-UTF-8 half became `a_transport_path_that_is_not_utf8_binds_no_hands`, with the root's case added |
| `engine/boundary_tests.rs`: the engine-smith group (`SMITH_POLICY` through `a_codex_only_smith_compiled_under_harness_launches_the_work_fragment_alone`) | `engine/boundary_tests/smith_tests.rs` (new child) |
| `tests/capability_launch.rs`: `checked_against` | `tests/capability_launch/servers.rs`, now `pub(super)` |
| `adapters/tests.rs`: the `Seal::hands` transport | `adapters/mcp/tests.rs::bound`, the fixture's one transport |

## Quality and gates

The gates ran clean on the final tree: `cargo fmt --all -- --check`,
`cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`,
`git diff --check`, `typos --hidden`, and `openspec validate --all --strict`
with 26 passed and 0 failed. `quality/ratchet.sh files` holds,
`quality/ratchet.sh api` holds against the regenerated snapshots, and
`quality/ratchet.sh baselines 79877120` reports no baseline raised.
`cargo run --locked -p brokkr-cli -- compile --bundle recipes/self` compiled
(policy sweep: 47 valuations, 0 unruled). The witness pins did not move, and
`witness_digests` passes in the runtime `it` binary.

The suites ran crate by crate. Protocol: 972 lib tests pass, plus its three
integration binaries. Runtime: 1,037 lib tests pass, as do
`capability_launch` (76), `it` (132), `operated_repo` and `queued_launch`.
In the CLI, the lib tests (872), `driver_conformance`, `transcript_surfaces`
and the three heap binaries pass. The full CLI `it` binary passed 707 of 707
on one run, `bootstrap_bench` included. On its final rerun it passed 706.
`capability_broker::handoff::a_handoff_that_fails_after_the_lookup_starts_no_server`
failed under load and passed when run alone. It is a broker timing test this
change does not touch. Seatbelt-probe, core, view, store and bridge pass.

Public API, measured with cargo-public-api 0.52.0 on nightly-2026-09-05:

| Crate | Rise | Lines added |
| --- | --- | --- |
| brokkr-protocol | +3 | `Transport::artifacts`, `native_controls::ARTIFACT_ROOT`, `hands::UNPROTECTED_EVIDENCE`; the nine `hands` signatures and `Executor` change in place |
| brokkr-runtime | 0 | `compose_site` and `compose_site_at` change in place |
| brokkr-cli | +2 | `HandsCommand::Serve::artifacts`, `HandsCommand::Exec::artifacts`, outside the approved protocol-and-runtime allowance |

Protocol and runtime together rise +3, inside the approved +6. The CLI's +2
needs the operator's ruling, with the three files above.

`quality/file-lines.txt` has rows for the four new production and test
children, and its rows for the touched files are set to their measured
counts. `quality/ratchet.sh clones` still lists seven clones: three in
production, among the parameter lists of `compose_site`, `compose_site_at`
and `compose_segments`, and four in tests. All seven are re-keys of
baselined clones. With main's bytes restored for those six files, the
moved-out children set aside and the rest of this change in place, the
ratchet printed "duplication holds". A seat cannot refresh a jscpd baseline,
so the refresh is owed under the standing ruling of 2026-09-30 (re-keyed
clones). The three clones this change would have added were folded instead:
the exec argv, a CLI test setup and a protocol test setup.

Pending, not observed here: the exact-coverage gate, which needs
`scripts/coverage-exact.sh` on the controller's host; macOS CI; and remote CI
on the final head. No crate-scoped llvm-cov diagnostic was run.

## Follow-ups, not fixed here

- `check_final`'s non-UTF-8 refusal reads "binds its hands to an executable
  or a workdir that is not UTF-8", and now also covers the artifact root.
  The text lives in `native_controls.rs`, outside the row. LOW.
- `box_argv` (9), `execute_in` (10), `run_boxed_in` (9), `compose_site`
  (10) and `compose_site_at` (11) each gained a parameter under their
  existing `too_many_arguments` expectations, against decision 0071 ruling
  4's ceiling of 7. The ruling approved the hands signatures; a parameter
  object for the public compose pair is #288's. `compose_segments` no
  longer carries the list (third run, below). LOW.
- The driver reads `artifact_root` from its input by key, as it reads
  `workdir`. A typed driver input is out of this row.
- A root whose spelling differs from the worktree's while lying inside it
  refuses (`Alias(workdir)`). That fails closed, but an adopter whose state
  path is spelled through a link into the worktree is refused rather than
  protected.

## Third run: the scope ruling, one clone folded, the gates again

The third run (`0065-slice-two-unit-u8a-see-the--d57f2685`) started from
7a96fd31's work, replayed by prep onto main at `dce35de6` and staged. The
operator ruled on 2026-10-17 that `crates/brokkr-cli/src/lib.rs`,
`crates/brokkr-protocol/src/probe/observe.rs` and
`crates/brokkr-cli/src/doctor.rs` join U8a's row. The same ruling lets
brokkr-cli's public API rise by the two measured `artifacts` lines, beside
the protocol and runtime allowance of +6. The scope finding above is
therefore answered, and this run widened nothing further. Measured against
`dce35de6`, `lib.rs` is now 1,908 lines (baseline 1,935), `observe.rs` 516
and `doctor.rs` 1,565 (baseline 1,572).

### `compose_segments` no longer restates the public signatures

Before this run's edit, `quality/ratchet.sh clones` refused three new
production clones in `engine.rs`: lines 2994 against 3038, 2996 against
3096, and 3041 against 3097. These are the parameter lists of
`compose_site`, `compose_site_at` and the private `compose_segments`.
`compose_segments` now takes `(&Composing<'_>, Vec<Segment>)`. `Composing`
is a private struct of the nine inputs `compose_site_at` was handed beside
its command; `compose_site_at` builds it, and `compose_segments`
destructures it. Its fields borrow with a named lifetime, so neither the
struct nor its literal repeats either public list token for token. The
private function's `#[expect(clippy::too_many_arguments)]` is gone. Because
of that, `quality/suppressions.txt` lowers the production
`too_many_arguments` count from 25 to 24. The suppressions test had failed
with exactly that difference, left 25 against right 24, and it passed in
the CLI `it` binary after the edit. `engine.rs` is 3,586 lines (baseline
3,630), and the public compose signatures are unchanged.

After the edit, the same ratchet reports one production finding:
`brokkr-runtime/src/engine.rs:2994 and brokkr-runtime/src/engine.rs:3038`,
11 lines. That is `compose_site` against `compose_site_at`, the re-keyed
form of main's one pair. It is expected, and the prod baseline was left
alone for the controller to refresh under the standing ruling of
2026-09-30. The tests baseline the controller refreshed in prep was not
edited.

The removal control for the struct was a compiling mutation that set
`artifacts: workdir` in `compose_site_at`'s `Composing` literal. The engine
lib filter then failed 11 tests and passed 320. The failures included
`boundary_tests::compose_site_follows_the_boundary_and_the_class`
(boundary_tests.rs:767),
`boundary_tests::every_boundary_arm_carries_the_links_segments_and_labels_its_own_as_hands`
(boundary_tests.rs:404), both `smith_tests` boxed-hands launches
(smith_tests.rs:112) and seven brokered serving tests
(brokered_tests.rs:381, :435, :504, :574, :618, :718 and a missing input
key). Once the mutation was restored, a grep for its marker came back
empty and the runtime lib passed 1,039 of 1,039.

### 38.1 and 38.2 re-observed on this head

On this head, the nine `hands::tests::evidence` tests ran with
`--nocapture`, in real namespaces, and printed no `skipped:` line. Run 2's
M1 was repeated as a compiling removal of the read-only root mount in
`box_argv`. It failed the same four tests at the same lines:
evidence.rs:126, :470 and :598, and tests.rs:508. With the mount restored,
the `hands::` filter passed 141 tests, with 2 ignored. With run 2's table
of M1–M21 standing, 38.1 and 38.2 are ticked.

### Gates on this head

| Gate | Observed |
| --- | --- |
| format and clippy, workspace-wide with all targets and features | both clean |
| brokkr-runtime suite | lib 1,039; `capability_launch` 76; `it` 132 (witness pins unmoved); `operated_repo` 1; `queued_launch` 6; all pass |
| brokkr-protocol suite | lib 972 with 3 ignored; `hands_exits` 6; `secret_drop` 1; all pass |
| brokkr-cli suite | lib 882; `it` 716 with 3 ignored, `bootstrap_bench` and `suppressions` included; `driver_conformance` 27; `transcript_surfaces` 14; the three heap binaries; all pass |
| brokkr-seatbelt-probe suite | 99 pass, 2 ignored |
| `recipes/self` compiled | policy sweep: 47 valuations over 11 groups, 0 unruled |
| openspec strict validation | 26 passed, 0 failed |
| typos over hidden files, whitespace check against HEAD | both clean |
| `quality/ratchet.sh files` | holds; rows were set for the seven new children and for every touched file whose count moved |
| `quality/ratchet.sh api` | holds; brokkr-protocol +3, brokkr-runtime 0, brokkr-cli +2, as ruled |
| `quality/ratchet.sh clones` | the one expected production finding named above |
| `quality/ratchet.sh baselines dce35de6` | names the controller's four refreshed tests fingerprints and the two ruled API rises; no file-lines row was raised |

Still pending: the exact-coverage gate, macOS CI and remote CI on the
final head. They run in CI or on the controller's host, not in this seat.

## Fourth run: the review's security hold answered

The fourth run (`0065-slice-two-unit-u8a-see-the--fc47eccb`) started from
10fa7b35's work, replayed by prep onto `dce35de6` and staged, without
`tasks.md` and `quality/file-lines.txt`. Review had stopped that head with
two HIGH findings, H1 and H2, and three LOW ones. This run answers H1, H2
and L1 inside the row, and records L2–L4. No production file outside the
row changed. `hands/evidence.rs` now reads the mount table through the
existing `namespace::sources::host::{records, aliases}` and `LIMITS`,
which are `pub(in crate::hands)`. Neither file changed, so the mount-table
parser still has one home (decision 0071 ruling 5).

### H1: no path reaches a box lossily

`engine/boxed.rs::hands_command` now returns `Result<_, Unnamed>`, a
`thiserror` enum (ruling 8). Before either argv is built, it checks that
the engine's executable, the workdir and the artifact root are UTF-8. On
the exec branch it also checks the bundle root. Each failed check refuses
by its own variant: `Executable`, `Workdir`, `Artifacts` or `Bundle`. Each
variant's text names the rejected path. `Artifacts` carries CR2's cause:
"dispatch refused: capability evidence root cannot be protected from every
managed writer: the artifact root '…' is not UTF-8, …". The exec argv is
built from the checked `&str`s, and no `to_string_lossy` is left in
`boxed.rs`. The model branch still expands through `Transport::expand`.
Its `None` is unreachable after the checks, so it maps to `Unexpanded` and
never to an argv. `compose_segments` carries the refusal's text as it
carried the old string.

`engine::boundary_tests::evidence_tests::an_exec_seat_whose_artifact_root_is_not_utf8_refuses_before_any_argv`
runs only on Linux, because APFS refuses names that are not UTF-8. It makes
`artifacts\xff` and its decoy `artifacts\u{fffd}` side by side, and asserts
that the lossy spelling of the first is the second. It then asserts the
exec spawn's `Err(Unnamed::Artifacts(root))`, its exact text,
`compose_site`'s refusal with the argv unboxed, and a non-UTF-8 bundle
root's `Unnamed::Bundle`. `engine::tests::a_model_seats_hands_are_expanded_by_the_checks_one_encoder`
now pins `Unnamed::Artifacts` and `Unnamed::Workdir` on the model branch.
It also pins the dispatch door's refusal text, which names the workdir.

### H2: aliases made by mounts are seen

On Linux, `Protected::observe` reads `/proc/self/mountinfo` once after the
root is canonicalised (`unaliased`). The root's own mount is the one the
kernel names for it, read with `statx` and `STATX_MNT_ID`. The pure
`shown_elsewhere(table, mount, canonical)` then finds each part of the
root that a mount holds. One part is the part below the root's own mount:
that mount's `root` field joined with the rest of the way. The others are
the whole of each mount at or inside the root. Any other mount of a part's
device whose `root` holds the part or lies inside it shows the part
somewhere (`aliases`). If that place is outside the root's own path, the
box refuses with `Unprotected::Mounted(place)`. This holds for a directory
or a file bind, under the worktree or anywhere else. A place inside the
root is not an alias, because the root's own read-only mount covers it.
That includes the whole filesystem at `/` when the root's mount is a
self-bind. The device compared is the table's, never the root's own
`stat`, which a btrfs subvolume reports apart from its mount's. Three
cases refuse as `Unprotected::Unmapped(root)`: an unreadable table, a
table that does not parse (the house parser is strict), and a root whose
mount the kernel does not name or the table does not hold. On macOS,
`unaliased` is a stand-in that returns `Ok(())`, so today's checks stand.
The new variants are `#[cfg(target_os = "linux")]`, so no dead variant
exists there.

A consequence is that a host whose worktree is itself a bind of another
directory refuses every library box while that directory stays mounted
elsewhere (`/srv/w` at `/w` with `/` mounted whole). That path really is a
second writable spelling of the evidence. The refusal fails closed and
names the place.

Recorded tables, in
`hands::tests::evidence::a_mount_that_shows_the_root_elsewhere_refuses_by_the_table`:

| Table beside the root's mount 21 (`/` on 8:1) | Verdict |
| --- | --- |
| `sha256` bound at `/else` | `Mounted("/else")` |
| one published file bound at `/x/file` | `Mounted("/x/file")` |
| the root bound at `/w/alias`, inside the worktree | `Mounted("/w/alias")` |
| the worktree bound at `/mnt/w` | `Mounted("/mnt/w/.forge/artifacts")` |
| the whole filesystem at `/d` | `Mounted("/d/w/.forge/artifacts")` |
| `.forge` at `/a\040b`, an escaped name | `Mounted("/a b/artifacts")` |
| an unrelated directory of 8:1 at `/else/src` | `Ok` |
| the same path on device 8:2 | `Ok` |
| `sha256` bound at its own place | `Ok` |
| a tmpfs mounted inside the root, alone, then also at `/scratch` | `Ok`, then `Mounted("/scratch")` |
| the root's mount a bind of `/srv/w` at `/w`, with `sha256` at `/x`, or with `/` mounted whole | `Mounted("/x")`; `Mounted("/srv/w/.forge/artifacts")` |
| a line with five fields; no mount 99; mount 22 at `/other` | `Unmapped` each |

`…::a_root_whose_mount_is_unproved_refuses` proves three things through
`unaliased_with`: this host's live table admits a fresh root, an
unreadable table refuses, and so does an absent root, which the kernel
names no mount for.

`…::a_bind_mount_alias_of_the_evidence_refuses_the_box` runs the real
namespace controls. For each case it runs this test binary again under
`bwrap --dev-bind / / --bind FROM TO`, which builds a private mount
namespace, and the copy builds the box there and writes what it observed.
On this host it ran and printed no `skipped:` line. The digests bound at
`root/alias`, one artifact bound at `root/file`, the worktree bound at
`root/mirror` and the digests bound at `work/nested` each refused by the
exact `Mounted` text. An unrelated bind (`pad` at `view`) and the digests
bound at their own place built the box.

### L1: the observation is confirmed as the argv is final

`Protected` keeps three identities: each pinned directory's and the
root's, by their own entry (`symlink_metadata`), and each excluded
writable source's, as resolved, or `None` where it was absent. `box_argv`
calls `protected.confirm()?` as the last step before `namespace.enter`. A
directory, root or source that is no longer the one observed refuses with
`Unprotected::Changed(path)`, and so does a source that was absent and now
resolves.
`…::a_directory_or_source_changed_after_its_observation_refuses` covers
four cases: `.forge` replaced under its name, the root replaced, a `pad`
bind swapped for a new directory, and an absent bind path turned into a
link to the root. Each refuses by the path that changed, and the
unchanged case confirms. What stays open: bubblewrap resolves each
pathname once more when it mounts it, after `confirm`. Closing that window
needs descriptor mounts in the namespace builder, `namespace.rs`, which is
outside this row. The call site's removal (L1d below) survives, because
nothing in `box_argv` can change the world between `observe` and
`confirm` without a seam. Adding one would change the public signature.

### Removal controls of this run

Each mutation compiled. It ran against the named filter, with the result
shown below, and was then restored. A final grep for the marker found
nothing.

| Mutation | Test that failed | Observed |
| --- | --- | --- |
| H1: `named` returns the lossy spelling | `an_exec_seat_whose_artifact_root_is_not_utf8_refuses_before_any_argv`; `a_model_seats_hands_are_expanded_by_the_checks_one_encoder` | evidence_tests.rs:115, `Ok([… "--artifacts", "…/artifacts�" …])`, the decoy; tests.rs:5718, `Err(Unexpanded)` |
| H2a: a place outside the root admitted | `a_mount_that_shows_the_root_elsewhere_refuses_by_the_table`; `a_bind_mount_alias_of_the_evidence_refuses_the_box` | evidence.rs:705 `Ok(())` for "directory"; :819 "built" for `alias` |
| H2b: `observe` ignores `unaliased` | `a_bind_mount_alias_of_the_evidence_refuses_the_box` | evidence.rs:819 "built" |
| H2c: mounts inside the root not scanned | the table test | evidence.rs:712 `Ok(())` against `Mounted("/scratch")` |
| H2d: a place inside the root refused too | the table test; the namespace test | evidence.rs:705 "own place" refused; :819 the self-bind refused |
| H2e: an unparsable table admitted | the table test | evidence.rs:730 "unparsable" `Ok(())` |
| H2f: an unfound mount admitted | the table test | evidence.rs:730 "no mount" `Ok(())` |
| L1a: pinned directories not confirmed | `a_directory_or_source_changed_after_its_observation_refuses` | evidence.rs:648, `Changed(…/.forge/artifacts)` against `Changed(…/.forge)` |
| L1b: `confirm` admits every change | the same | evidence.rs:648 "pinned" `Ok(())` |
| L1c: excluded sources not confirmed | the same | evidence.rs:648 "swapped" `Ok(())` |
| L1d: `box_argv` drops `confirm()` | none: the `hands::` filter passed 145 | survives, as recorded under L1 |

### LOW findings recorded, not fixed here

L2: the driver still reads `artifact_root` from its input by key in
`adapters/mcp.rs::hands_transport`, at the edge where it reads
`launch_record`. A root the input lacks binds an empty path that no
server entry matches, so the final check refuses. A typed driver input
belongs to the input's decoder in `adapters.rs`, which is outside the row.

L3: `box_argv` (9), `execute_in` (10), `run_boxed_in` (9), `compose_site`
(10) and `compose_site_at` (11) stay over ruling 4's 7 parameters, under
their baseline `too_many_arguments` expectations. A parameter object
changes each public signature beyond the ruled API rise, so it stays
#288's.

L4: `check_final`'s text in `native_controls.rs` still reads "an executable
or a workdir that is not UTF-8" for a non-UTF-8 artifact root too. That
file is outside the row. The engine's own refusals now name the exact
input (H1).

`hands::serve_args` still renders its paths with `to_string_lossy`. It
returns a `Vec<String>` in the public API. Every engine path to it goes
through `Servers::expand`, which checks UTF-8 first, and the adapter probe
hands it its own scratch paths.

### 38.1 and 38.2, and the gates on this head

With the H1, H2 and L1 controls above in the owning suites, 38.1 and 38.2
are ticked.

| Gate | Observed |
| --- | --- |
| format and clippy, workspace-wide with all targets and features | both clean |
| brokkr-protocol suite | lib 976 with 3 ignored; `hands_exits` 6; `secret_drop` 1; all pass |
| brokkr-runtime suite | lib 1,040; `capability_launch` 76; `it` 132, witness pins unmoved; `operated_repo` 1; `queued_launch` 6; all pass |
| brokkr-cli suite | lib 882 with 1 ignored; `it` 716 with 3 ignored; `driver_conformance` 27; `transcript_surfaces` 14; the three heap binaries; all pass |
| brokkr-seatbelt-probe suite | 99 pass, 2 ignored |
| `recipes/self` compiled | policy sweep: 47 valuations over 11 groups, 0 unruled |
| openspec strict validation | 26 passed, 0 failed |
| typos over hidden files; whitespace check | both clean |
| `quality/ratchet.sh files` | holds; rows set for the seven new children and every touched file whose count moved |
| `quality/ratchet.sh api` | holds; no public item added by this run (`Unnamed` is `pub(crate)`, the evidence items `pub(super)`) |
| `quality/ratchet.sh clones` | the one expected production finding, `engine.rs:2994` against `:3038`, the controller's to refresh |
| `quality/ratchet.sh baselines dce35de6` | the controller's refreshed tests fingerprints and the two ruled API rises (brokkr-cli +2, brokkr-protocol +3); nothing else |

Still pending: the exact-coverage gate, macOS CI and remote CI on the
final head.

## Fifth run: one mount rule for every mount type

The fifth run (`0065-slice-two-unit-u8a-see-the--71511736`) started from
2cb01de8's work, replayed by prep onto `ddffa2aa` and staged, without
`tasks.md` and `quality/file-lines.txt`. Review had stopped that head with
REVIEW-SECURITY-HOLD on H3, a writable overlay whose upper directory is the
artifact root, beside M1 and L1. This run closes the alias question with
one rule instead of a fourth special case, answers M1 and L1, and keeps the
staged work and its suppressions change as they arrived.

### The rule

On Linux the mount table is read once, when the root is observed, and kept.
`host::Record` now keeps each mount's filesystem type, source and super
options as the table spells them (`tail`), and `host::unescaped` is
`pub(in crate::hands)` so the octal decoder keeps one home. No existing
caller reads the new field, and the source observer's tests pass unchanged
apart from the record literal (`hands::` filter, 150 passed). `host.rs` is
799 lines against its 800 ceiling.

Once every writable source is known, `Protected::confirm` hands the kept
table to the pure verdict `evidence::reached(records, own, canonical,
sources, resolve)`. The sources are the canonical worktree and every
excluded source that resolves: the private home and tmp, the bound git
common directory, and each authored `rw` and `overlay` bind. A mount is
judged when its point lies at or under a source, which the box binds whole
with every mount below it, or when it backs the root: the root's own mount,
or any mount at or inside the root. Every other mount is one the box cannot
see and the evidence does not stand on.

| Case | Verdict |
| --- | --- |
| (a) a same-device mount showing the root or part of it elsewhere | `Mounted`, by H2's `shown_elsewhere`, unchanged; it still refuses wherever it is mounted, not only in reach |
| (b) an overlay whose upper or work directory is the root, lies inside it or holds it | `Overlaid(point)` |
| (b) an overlay in reach, not backing the root, whose upper or work directory resolves nowhere here | `Overlaid(point)` (see the first judgement below) |
| (b) an overlay backing the root that reads or writes a layer at, under or holding a writable source | `Underlaid(point)` |
| (b) a lower or data layer inside the root, on an overlay that does not back the root | passes: a write copies the file up first; the comment on `overlaid` says so |
| (b) a layer of an overlay backing the root that resolves nowhere here (a container runtime's `/var/lib/docker/...`) | out of reach, passes |
| (c) `fuse`, `fuse.*`, `fuseblk`, `nfs`, `nfs4`, `cifs`, `smb3`, `9p`, `virtiofs`, `ceph`, `glusterfs`, `aufs`, `unionfs`, or an unknown type whose source is not under `/dev/` | `Stacked(point)`, in reach, as the root's own mount, or inside the root |
| local types (`host::LOCAL`), or an unknown type on a `/dev/` source | passes |
| (d) a table that does not parse | `Unmapped(root)`, as before |
| (d) super options that do not parse | `Unparsed(point)` |

The overlay parser reads the super options strictly. Options split on `,`,
since the kernel escapes a literal comma as `\054`. Each value's octal
escapes must be whole. A legacy `lowerdir` splits on `:` with `\` taking
the next byte as itself, and `::` before the data layers. `lowerdir+`,
`datadir+`, `upperdir` and `workdir` each name one path. Every layer must
be absolute. An overlay must read at least one layer, and it has an upper
directory exactly where it has a work directory, once. An empty option, a
layer key with no value, an empty layer other than the one `::` leaves, or
a trailing `\` refuses. A real overlay on this host's kernel (6.17), made by
bubblewrap in a throwaway probe test that was removed afterwards, showed
`lowerdir=/oldroot/.../lo2:/oldroot/.../lo\134:w\040er` and
`upperdir=/oldroot/.../u\134:p\134\054q\134\134r`. So the kernel shows the
legacy option string with its backslash escapes, octal-escaped once more.
Each single path is therefore tried in two spellings, as shown and
unescaped: the kernel unescapes a legacy option value, and a value given
through the new mount API keeps its `\`.

The verdict lives in `hands/evidence.rs`, which is 725 lines. Its pieces
are `reached`, `judged`, `overlaid`, `underlaid`, `backing`, `layers`,
`Layer::named`, `Layers::take`, `option`, `lowers`, `spelled` and
`absolute`. The new `Unprotected` variants are `Unparsed`, `Overlaid`,
`Underlaid` and `Stacked`, each `#[cfg(target_os = "linux")]` and each
beginning with CR2's cause. macOS keeps today's behaviour: the table read,
the `mounts` field and the verdict are Linux-only, so no unused field or
stand-in is left there.

### Judgements beyond the commission's wording

Each of these is stricter than the commission's text. None weakens a
refusal.

First, "a superoption path that does not resolve is out of reach" is
applied to the overlays that back the root, which is the layout the
commission gave as its reason: the CI container's `/`, `/tmp` and
`/var/tmp`. An overlay in reach that does not back the root, but whose
upper or work directory does not resolve, refuses. The reason is observed
on this host. bubblewrap names an overlay's layers under its `/oldroot`,
which is gone once the box stands, so an H3 overlay made by bubblewrap
shows an upper directory nothing here resolves. The literal reading builds
that box: removal control D below shows the real-namespace H3 case
returning "built". No CI fixture has an overlay mounted under a source.

Second, the converse check counts the lower and data layers of an overlay
backing the root, as well as its upper and work directories. It also
counts a layer that holds a source, not only one under it. A box writing
into a lower layer changes what the overlay shows of a file not yet copied
up.

Third, mounts inside the root count as backing it, so a FUSE mount at
`<root>/sha256` refuses even where the root lies outside every bind.

Fourth, super options are read only for mounts the rule judges. A
malformed option string on a mount the box cannot reach, such as a
container runtime's own overlays on the host, is never read. The table as
a whole still parses strictly or refuses.

Fifth, "a block device" is judged from the source's spelling under `/dev/`,
which keeps the verdict pure. ZFS and other pool-named sources therefore
refuse, in reach or as the root's mount.

### Recorded-table tests

All four are in `hands::tests::evidence`, Linux-only. Each builds its table
from lines in the kernel's format through the house parser, with the
sources `/w` (worktree), `/s/rw`, `/s/ov`, `/g/common`, `/p/home` and
`/p/tmp`, and the root `/w/.forge/artifacts` on mount 21.

| Test | Cases asserted (by its assertion message) |
| --- | --- |
| `an_overlay_the_box_reaches_that_writes_into_the_root_refuses` | `Overlaid` for H3 (`/w/view`, `upperdir=/w/.forge/artifacts`), upper inside, upper holds, work is root, work inside, work holds, an octal-escaped and a backslash-escaped upper, an unfollowed `/oldroot` upper, and the same overlay under each writable source class: rw bind, overlay bind (at the source itself), git common, private home, private tmp. `Ok` for out of reach, a source's parent, writes elsewhere, a copy-up lower inside the root, and a read-only overlay reading the digests. Also `Overlaid` for a new-API layer whose shown spelling, with its `\`, is the root |
| `an_overlay_the_root_stands_on_refuses_a_layer_the_box_writes` | `Underlaid("/")` for an upper in the worktree, a work directory in the private tmp, an upper holding `/p`, the fourth of several escaped legacy lower layers under the rw bind, and a `datadir+` under the git common directory. `Ok` for additive `lowerdir+`/`datadir+` apart, a `::` data separator, and escaped lowers apart. `Underlaid` for an overlay inside the root whose upper lies in the private home while the root is outside every source. `Ok` for the CI container shape, Docker layers under `/var/lib/docker`, backing `/`, `/tmp` and `/var/tmp` |
| `a_stacking_remote_or_unknown_filesystem_in_reach_refuses` | for each of the 13 named families and `zfs`: `Stacked` in reach, inside the root, and as the root's own mount; `Ok` out of reach. `Ok` for ext4, tmpfs and vfat on `/dev/sdb1`, in reach and as the root's own mount |
| `overlay_options_that_do_not_parse_refuse` | `Unparsed("/w/v")` for 11 malformed option strings, each `Ok` out of reach; the four new variants' texts pinned once |

### Real namespace controls

`a_bind_mount_alias_of_the_evidence_refuses_the_box` now takes whole
bubblewrap argument lists, and its inner box binds `root/rw` read-write.
Beside run 4's six bind cases, it runs four overlay cases:

- a writable overlay at `work/view` whose upper is the root refuses with
  the exact `Overlaid` text;
- a read-only overlay there reading the root and `pad` builds;
- the writable overlay at `root/view`, out of reach, builds;
- the writable overlay at `root/rw/view`, under the rw bind, refuses.

It ran on this seat's host. A `--nocapture` run printed no `skipped:` line,
and the 18 evidence tests passed. bubblewrap's `/oldroot` layer names mean
these cases exercise the "cannot follow" arm. The "resolves to the root"
arm is proved by the recorded tables, because `unshare -rm` is refused
here ("write failed /proc/self/uid_map: Operation not permitted").

### M1: the outer uncertainty checks

`a_root_whose_mount_is_unproved_refuses` drives `unaliased_with` with an
existing root and an unreadable table, then with an absent root and this
host's readable table. Each is bound by its own compiling removal, recorded
below as M1a and M1b, separate from the parser's H2e and H2f. The
`STATX_MNT_ID` mask filter is the one guard left unbound. Its removal
cannot fail on a kernel that sets the mask, which is every kernel since
5.8. Where the mask is unset, `stx_mnt_id` reads 0, which names no record,
so `shown_elsewhere` refuses anyway.

### L1: the confirmation bound through `box_argv`

`evidence.rs` declares a `#[cfg(test)]` thread-local, `CONFIRMING`. Its
step runs at the start of `Protected::confirm`, the one place inside
`box_argv` where a test can change the world between observation and
confirmation. No signature or public item changed.
`the_box_confirms_its_observation_as_its_argv_is_final` builds a box, then
sets the step to replace the root under its own name. It asserts
`Unprotected::Changed(root)` from `box_argv` itself. Removing the
production `protected.confirm()?` call fails it (control V).

Three windows stay open, as limitations. bubblewrap resolves every pathname
once more when it mounts it, after `confirm`; closing that needs descriptor
mounts in `namespace.rs`, outside this row. The mount table is read when
the root is observed, so a mount made after that read and before
bubblewrap runs is not seen. An overlay whose upper directory was renamed
after its mount, where something else now stands at the old name, cannot
be told apart from the table.

### Removal controls of this run

Each mutation compiled and ran against `hands::tests::evidence`, or against
`hands::` for V. The table shows the test that failed and its assertion,
and each mutation was then restored. A final grep found no mutation marker
in `evidence.rs` or `hands.rs`. The `hands::` filter then passed 150, with
2 ignored. M and W were run again after `layers` was split into
`Layer::named` and `Layers::take`.

| Mutation | Test that failed | Observed |
| --- | --- | --- |
| A: reach ignored, only mounts backing the root judged | stacking; H3; parse; namespace | `Ok` for `/w/m`; "H3" `Ok`; `Unparsed` case `Ok`; "built" for the `work/view` overlay |
| B: mounts backing the root not judged | stacking; converse | inside-root `Ok`; "upper in worktree" `Ok` |
| C: no upper or work layer judged | H3; namespace | "H3" `Ok`; "built" |
| D: an unresolvable upper out of reach (the literal reading) | H3; namespace | "unfollowed" `Ok`; "built" for the real H3 overlay |
| E: unresolvable refuses even backing the root | converse | the container case `/`: `Overlaid("/")` |
| F: an upper holding the root admitted | H3 | "upper holds" `Ok` |
| G: lower layers judged as written | H3; namespace | "copy-up" `Overlaid`; the read-only overlay refused |
| H: the converse check removed | converse | "upper in worktree" `Ok` |
| I: the converse check ignores read layers | converse | "escaped lowers" `Ok` |
| J: a stacked type on a `/dev/` source local | stacking | "fuseblk" `Ok` |
| K: every unknown type local | stacking | "zfs" `Ok` |
| L: an unknown type on a `/dev/` source refused | stacking | "vfat" `Stacked` |
| M: overlay completeness unchecked | parse | `rw,upperdir=/u,workdir=/wk` `Ok` |
| N: an empty option key admitted | parse | `rw,,lowerdir=/l` `Ok` |
| O: empty legacy layers admitted; O2: only the doubled `::` | parse | `rw,lowerdir=:/l` `Ok`; `rw,lowerdir=/a:::/b` `Ok` |
| P: the unescaped spelling dropped | H3 | "backslash" `Ok` |
| Q: the shown spelling dropped | H3 | the new-API case `Ok` |
| R: relative layers admitted | parse | `rw,lowerdir=l` `Ok` |
| S: `confirm` ignores the verdict | namespace | "built" for the `work/view` overlay |
| T: excluded sources not in the reach set | namespace | "built" for the `rw/view` overlay |
| U: the worktree not in the reach set | namespace | "built" for the `work/view` overlay |
| W: `datadir+` not a layer | converse | "data layer" `Ok` |
| M1a: an unreadable table admitted | `a_root_whose_mount_is_unproved_refuses` | `Ok(())` against `Unmapped(root)` |
| M1b: an unnamed root mount admitted | the same | `Ok(())` against `Unmapped(…/absent)` |
| V: `box_argv` drops `protected.confirm()?` | `the_box_confirms_its_observation_as_its_argv_is_final` | "the box was built" |

S, T and U are bound only by the real-namespace test, so on a host that
skips it they are pending until the controller's host check runs it.

### Files, quality and gates

Production files this run changed: `hands/evidence.rs` (725) and
`hands/namespace/sources/host.rs` (798 → 799). Test files:
`hands/tests/evidence.rs` (1,240) and `hands/tests/sources.rs`
(1,880 → 1,875). No test moved. No production file outside the row and its
named private children changed. `hands.rs` is unchanged by this run, at
1,158 against its 1,169 baseline.

`quality/file-lines.txt` now holds the measured row of every Rust file
this branch changes. It adds rows for the new `hands/environment.rs`,
`hands/evidence.rs`, `engine/boxed.rs`, `hands/tests/evidence.rs`,
`native_controls/tests/toml.rs`, `boundary_tests/evidence_tests.rs` and
`boundary_tests/smith_tests.rs`.

`quality/too-many-lines.txt` was re-measured with measure.sh's forced
`clippy::too_many_lines` run. 87 rows the staged replay had moved now
match. After that, every listed row is one measured, except the two
`machine_proof.rs` rows. That file is not changed on this branch, and its
rows drifted on main with #714, so they are left for the controller. No
function of this run is over 100 lines.

`suppressions.txt` is as staged: the suppressions test passed.
`scripts/measure-budgets.sh` measured 101 prompt sites, 329 packages and
heap peaks equal to the committed ones. Every prompt site was at or under
its budget, and the rewrite moved only dates and lower budgets drifted on
main, so the three files were restored unchanged.

| Gate | Observed |
| --- | --- |
| format; clippy, workspace-wide with all targets and features | both clean |
| brokkr-protocol suite | lib 981 with 3 ignored, `hands_exits` 6, `secret_drop` 1, the doc test 1; all pass |
| brokkr-runtime suite | lib 1,040, `capability_launch` 76, `it` 132 with witness pins unmoved, `operated_repo` 1, `queued_launch` 6; all pass |
| brokkr-cli suite | lib 882 with 1 ignored, `it` 716 with 3 ignored, `driver_conformance` 27, `transcript_surfaces` 14, the three heap binaries; all pass |
| every other workspace crate | bridge, core, seatbelt-probe, store and view suites all pass |
| `recipes/self`, `recipes/verify` compiled | 47 valuations over 11 groups and 16 over 5, 0 unruled |
| `quality/ratchet.sh files`, `api` | both hold |
| `quality/ratchet.sh clones` | only `engine.rs:2994` against `:3038`, the controller's |
| `quality/ratchet.sh baselines ddffa2aa` | the tests fingerprints and the ruled API rises (brokkr-cli 11 → 13, brokkr-protocol 1,167 → 1,170), as staged; nothing from this run |

38.1 and 38.2 are ticked on the observations above. Still pending: the
exact-coverage gate, the controller's host check of the namespace controls,
macOS CI and remote CI on the final head. A local macOS check could not
run: `cargo check --target aarch64-apple-darwin` stops for want of that
target's standard library. The macOS paths were reviewed by hand instead.

## Sixth run: the mount rule made fail-closed

The sixth run (`0065-slice-two-unit-u8a-see-the--b22bc59c`) started from
081593d8's work, staged on `ddffa2aa` with the controller's prod clone
refresh laid over it, and kept both. Review had held run 5 on H4, H5, H6
and SC3. The controller withdrew run 5's instruction that a layer which
does not resolve here is out of reach, so this run removes that arm and
every other place where an unobserved fact was let through. Every change
below makes a check stricter; none needed an operator ruling.

### The rule as built

The writable sources are unchanged: the worktree, each authored `rw` and
`overlay` bind, the bound git common directory, and the private home and
tmp. `verdict::judged` judges a mount when its point lies at or under a
source (1), when it is the mount a source lies on as the edge observed it
(2, H4), or when it backs the root: the root's own mount, or one at or
inside the root (3). No other mount is judged. Before choosing, it refuses
a source whose mount was not observed (`Unobserved(source, kind)`) or
whose mount id names no record (`Unmapped(source)`), and any record whose
point has a `.` or `..` component, which no prefix can place
(`Unparsed(point)`).

`verdict::one` then judges each chosen mount:

| Case | Verdict |
| --- | --- |
| its point or any super option does not read strictly (H6) | `Unparsed(point)` |
| a local type `host.rs` proves (`LOCAL`) | passes; a same-device alias was refused by `shown_elsewhere`, unchanged |
| overlay: an upper or work directory not observed, for any error kind | `Unfollowed(point, layer, kind)`, a new variant |
| overlay: an upper or work directory that is the root, lies inside it or holds it, by canonical path or by identity | `Overlaid(point)` |
| overlay backing the root: any layer not observed | `Unfollowed(point, layer, kind)` |
| overlay backing the root: any layer at or inside a source, or holding one, by path or identity | `Underlaid(point)` |
| overlay not backing the root: a lower or data layer | read only, never observed or judged, since a write copies up into the upper directory first; the comment in `verdict::overlay` says so |
| `fuse`, any `fuse.*`, `fuseblk`, `nfs`, `nfs4`, `cifs`, `smb3`, `9p`, `virtiofs`, `ceph`, `glusterfs`, `aufs`, `unionfs` | `Stacked(point)`, whatever its device |
| any other type | passes only where the edge observed its source to be a block device whose number is the mount's `major:minor` (H5); otherwise `Stacked(point)` |

`Overlaid`'s text no longer says "writes where this namespace cannot
follow", since that case is now `Unfollowed`, which names the layer and
the error kind.

### Observed facts, then a pure verdict (SC4, L5)

`Protected::confirm` builds the root's facts (canonical path, own mount id,
holder identities) and calls `observed::observed` once. That function
observes each source's mount with `statx(STATX_MNT_ID)` on its canonical
path (`Contained::Mount`, `Unnamed` or `Unobserved(kind)`). It asks
`verdict::wanted` which layers and sources the chosen mounts need. Each
layer is resolved to a directory: its canonical path and identity, or the
`io::ErrorKind` saying why not (`NotADirectory` for anything else). Each
unknown mount's source becomes `Block::Device((major, minor))`, `Other`
or `Unobserved(kind)`. `verdict::reached` then reads only those facts and
makes no filesystem call. A layer the edge never observed reads as
`NotFound` and refuses. `unaliased_with` now takes the root's own mount
from the same `observed::contained`, so the `statx` reading has one home.

`host::Record` keeps `filesystem` and `source` decoded and `supers` as a
list of items, each as the table spells it, instead of the positional
`tail` (C3). The verdict reads those items strictly only for judged
records, so a container runtime's malformed options on a mount the box
cannot reach are never read.

### Strict parsing (H6)

For every record in the table, `host::records` now refuses an escape that
is not three octal digits up to `\377`, a truncated escape, a decoded NUL,
an empty root, point, type or source, and a mount option list with an
empty or malformed item. Type and source are decoded like root and point.
The source observer reads this table too, so the stricter read applies to
it as well. Every test of that observer passed unchanged apart from the
record literal. Task 38.1 says the common source observer needs no edit
in this row. The controller's commission for this run directed C3 and H6
into the private `host.rs` all the same, and no observer logic changed
beyond the stricter parse.

For judged records, `options::whole` refuses any empty or malformed super
option, whatever the filesystem; `ext4` with `rw,,broken\9` refuses. An
overlay layer value must be exactly how `seq_show_option` writes it:
`options::shown` decodes the value, escapes it again with the kernel's set
`,= \t\n\\`, and refuses on any difference. So `\146` for `f`, or a bare
`=`, refuses. Each value is then read as the kernel reads that option.
This follows the implementer's knowledge of the kernel's overlay
parameter code (`fs/overlayfs/params.c`). That code was not read in this
session, and the reading is pending review. The only observation is run
5's probe, which showed a legacy upper directory octal-escaped once over
its backslash escapes. `upperdir` and `workdir` are unescaped by
`ovl_unescape`; a trailing `\` refuses. `lowerdir+` and `datadir+` are
taken as written. A legacy `lowerdir` is split on unescaped `:`, with at
most one `::` before the data layers, and each layer is unescaped. Run 5's
two-spelling guess is gone: an upper directory shown as `/w/a\134b` is
`/w/ab`, never `/w/a\b`. Every layer path, and every judged point, must be
absolute, hold no NUL, have no `.` or `..` component, and not end in the
kernel's ` (deleted)` mark.

### Recorded-table tests

All are in `hands::tests::evidence` and Linux-only. Each feeds
`verdict::reached` recorded facts through the `Observe` fixture. By
default a source lies on the record whose point is its longest prefix,
and a layer is the directory it names, except under `/var/lib/docker` or
`/oldroot`, which are unobserved. A source block device is absent unless
a test names one.

| Test | What it asserts |
| --- | --- |
| `an_overlay_the_box_reaches_that_writes_into_the_root_refuses` | `Overlaid` for H3, an upper inside, holding or being the root, a work directory likewise, the `\134` upper, and the overlay under each source class. `Ok` out of reach, writing elsewhere, copy-up and read-only. The kernel's unescape: `/w/a\134b` refuses for the root `/w/ab` and builds for `/w/a\b`. An upper at `/mnt/alias` whose identity is the root's refuses |
| `the_mount_a_writable_source_lies_on_is_judged` (H4) | run 5's accepted "source's parent" case inverted: `/s` an overlay with `upperdir=ROOT` refuses `Overlaid("/s")` for `/s/rw` and for `/s/sha256`, and for `/x/rw` that the edge places on `/s` with no path relation. `fuse.sshfs`, `nfs4` and `9p` holding `/s/rw` each refuse `Stacked("/s")`. A source mount `Unobserved(PermissionDenied)`, `Unnamed`, or naming record 99 refuses by the source |
| `an_overlay_layer_that_cannot_be_observed_refuses` (SC3) | an `/oldroot` upper refuses `Unfollowed(.., NotFound)`. A work directory unobserved for `PermissionDenied`, `InvalidData`, `NotADirectory` or `Other` refuses with that kind. An unobserved lower layer builds on an overlay in reach and refuses on the root's own overlay |
| `an_overlay_the_root_stands_on_refuses_a_layer_the_box_writes` | run 5's `Underlaid` cases, plus a lower layer whose identity is the private tmp's. The CI container shape (Docker layers under `/var/lib/docker` backing `/`) now refuses `Unfollowed("/", ".../x/diff", NotFound)` |
| `a_stacking_remote_or_unknown_filesystem_in_reach_refuses` | the 13 families and `zfs` refuse in reach, inside the root and as the root's own mount. `ext4` on `/dev/mapper/root` and `tmpfs` build |
| `an_unknown_filesystem_builds_only_on_its_own_block_device` (H5) | `vfat` builds on a block device of its own number, in reach and as the root's mount. It refuses on another number, on `/dev/shm/backing` observed as a regular file, and on an absent source. CI's `ext4` on an absent `/dev/mapper/ci--vg-root` builds. `fuseblk` on its own device still refuses |
| `options_that_do_not_parse_refuse` (H6) | 18 malformed overlay option strings refuse `Unparsed("/w/v")` and each builds out of reach. The `ext4 rw,,broken\9` case, a deleted judged point, and a `.`/`..` point (even one out of reach) refuse |
| `the_edge_observes_each_fact_the_verdict_reads` | the real edge over a temporary tree: the live `statx` mount id of the worktree, a directory layer as itself, a file as `NotADirectory`, an absent layer as `NotFound`, `/dev/null` as `Other`, an absent source as `Unobserved(NotFound)`; `named` with and without the mask; `numbered` splits `makedev(8, 17)` |
| `each_mount_verdict_states_the_managed_writer_refusal` | the five mount-verdict texts, `Unfollowed` new and `Overlaid` changed, pinned once |

`hands::tests::sources::the_mount_table_is_read_strictly_and_within_its_bound`
gained nine malformed lines for the table-wide H6 cases. These are an
escape past `\377`, a truncated escape, `\000`, a bad escape in the type
and in the source, an empty type, an empty source, `rw,,noatime` and
`rw,noatime\9`.

In the real namespace, `a_bind_mount_alias_of_the_evidence_refuses_the_box`
now expects both writable overlays in reach to refuse with `Unfollowed`.
The kernel shows the upper directory as `/oldroot/<root>`, and the exact
text was observed on this host. The read-only and out-of-reach overlays
still build. The 23 evidence tests ran with no `skipped:` line.

### Removal controls of this run

Each mutation compiled, was run against `hands::tests::evidence`, or
against `the_mount_table_is_read_strictly_and_within_its_bound` for
M10–M13, and was then restored. After the last one, `git diff` against the
staged tree was empty and a grep for the markers used found none.

| Mutation | Failed test | Observed |
| --- | --- | --- |
| M1 the containing mount not judged | `the_mount_a_writable_source_lies_on_is_judged` | `Ok` against `Overlaid("/s")` |
| M2 an unobserved source mount admitted | the same | `Ok` against `Unobserved("/s/rw", PermissionDenied)` |
| M3 a mount id naming no record admitted | the same | `Ok` against `Unmapped("/s/rw")` |
| M4 an `Unnamed` answer admitted | the same | `Ok` against `Unmapped("/s/rw")` |
| M5 any block device number admitted | `an_unknown_filesystem_builds_only_on_its_own_block_device` | `Ok` against `Stacked("/w/m")` |
| M5b a source that is no block device admitted | the same | `Ok` against `Stacked("/w/m")` |
| M6 stacking types classed unknown | the same, `fuseblk` case | `Ok` against `Stacked("/w/m")` |
| M7 the edge's block-device check removed | `the_edge_observes_each_fact_the_verdict_reads` | `/dev/null` as `Device((1, 3))` against `Other` |
| M8 the edge's directory check removed | the same | the file layer `Ok(..)` against `Err(NotADirectory)` |
| M9 the edge names no source mount | the same, and ten more evidence tests | `Unnamed` against `Mount(223)`; box builds refused `Unmapped` |
| M10 a decoded NUL admitted | `the_mount_table_is_read_strictly_and_within_its_bound` | `/mnt\000` parsed |
| M11 an empty field admitted | the same | the empty type parsed |
| M12 the options after the first unchecked | the same | `rw,,noatime` parsed |
| M13 the source not decoded | the same | `/dev/r\9` parsed |
| M14 `whole` always true | `options_that_do_not_parse_refuse` | `Ok` against `Unparsed("/w/v")` |
| M14b `whole` read for overlays alone | the same | the `ext4` case `Ok` against `Unparsed("/w/m")` |
| M15 the kernel-spelling check removed | the same | `rw,lowerdir=/arti\146acts` `Ok` |
| M16 the upper directory not unescaped | the H3 test and the parse test | "backslash" `Ok`; `upperdir=/u\134` `Ok` |
| M17 a second `::` admitted | the parse test | `rw,lowerdir=/a:::/b` `Ok` |
| M18 dotted layer paths admitted | the same | `rw,lowerdir+=/l/../x` `Ok` |
| M19 the deleted mark admitted | the same | `rw,lowerdir+=/l\040(deleted)` `Ok` |
| M20 a judged point not read strictly | the same | `Ok` against `Unparsed("/w/m (deleted)")` |
| M21 a dotted point out of reach admitted | the same | `Ok` against `Unparsed("/else/../w/m")` |
| M22 an unobserved upper or work directory skipped | `an_overlay_layer_that_cannot_be_observed_refuses`; the real namespace test | `Ok`; "built" for the `work/view` overlay |
| M23 a backing overlay's read layers unjudged | the same; the converse test | rooted `Ok` against `Unfollowed("/", "/oldroot/l", ..)`; "escaped lowers" `Ok` |
| M24 read layers judged on every overlay | the H3, unobserved and real namespace tests | "elsewhere" `Unfollowed(.., "/l", ..)`; the read-only overlay refused |
| M25 the upper's identity check removed | the H3 test | the `/mnt/alias` case `Ok` |
| M26 the source identity check removed | the converse test | the private-tmp identity case `Ok` |
| M27 rule 1 removed | seven evidence tests, the real namespace one among them | each `Ok` or "built" where it refuses |
| M28 rule 3 removed | the stacking and converse tests | the inside-root cases `Ok` |
| M29 `named` ignores the mask | the edge test | `[Mount(9), Mount(9)]` |
| M30 `numbered` ignores the device bit | the edge test | `/dev/null` as `Device((1, 3))` |

Run 5's controls S, T and U still bind the source list through the real
namespace test: M22's failure shows its `rw/view` and `work/view`
overlays are judged.

### Limitations

These limitations are recorded and none is fixed here.

A workspace box now refuses wherever a writable source or the run-local
directory sits on an overlay whose layers this namespace cannot see. A
plain container's overlay root is one such case. This matches the
broker, which refuses overlay filesystem identities. CI's workstation
runners copy their filesystem to a native disk volume, which becomes the
container root, and use RAM filesystems at `/tmp` and `/var/tmp`
(`ops/ci-runner/README.md`). The hosted runners have no overlay root.

The namespace-name limitation (L6) sits beside the pathname window and
the stale-table window recorded in run 5. A layer is named by the path
its mounter used, in the mounter's namespace. Where that name resolves
here, the verdict judges what the name is here, which need not be the
layer the kernel holds: a layer since renamed, or one named in another
namespace that happens to resolve here, is not told apart. A layer the
kernel names from a descriptor rather than an option string carries
whatever name the kernel printed, and is read by the same rule.

The pathname window and the stale-table window stand as run 5 recorded
them. bubblewrap resolves each pathname again when it mounts it, and a
mount made after the table was read is not seen.

### Test lengths (G1) and macOS imports (G2)

Four test functions that had grown were each brought back by one local
closure, which writes the repeated `--artifacts` arguments once:

| Test | `ddffa2aa` → run 5 → now |
| --- | --- |
| `hands_exec_runs_the_command_whole_and_returns_its_code` | 117 → 129 → under 100; its `#[expect(clippy::too_many_lines)]` and baseline row removed |
| `the_namespace_is_built_from_an_empty_root_and_binds_what_the_spec_names` | 199 → 208 → 145 |
| `the_box_hides_the_host_and_holds_the_worktree` | 123 → 139 → under 100; its expect and row removed |
| `compose_site_follows_the_boundary_and_the_class` | 263 → 284 → 233 |
| `git_works_in_the_box_and_cannot_plant_a_hook` | 140 → 143 → 117 |

`quality/suppressions.txt` drops from 249 to 247 test `too_many_lines`
expects, and the suppressions test passed. Four functions stay above
their baselines. Each grew only because one call gained the `artifacts`
argument, and none has a repeated call worth folding:
`every_boundary_arm_carries_the_links_segments_and_labels_its_own_as_hands`
(164 → 167), `an_unboxed_exec_dispatch_is_refused_at_spawn_when_its_layer_moved`
(176 → 177), `the_shipped_codex_harness_work_seat_composes_the_preserved_rejoin`
(112 → 113), and
`the_serving_inputs_are_sealed_beside_the_record_and_admitted_only_as_sealed`
(254 → 256). The first rewrite of `compose_site_follows…` moved the
fingerprint of an existing baseline clone in its first lines. The clone
gate refused it, and the original prologue was restored, so no baseline
was edited.

In `boundary_tests/evidence_tests.rs`, `Unnamed`, `require_bwrap`,
`run_boxed_in`, `GitFacts`, `MetadataExt` and `Stdio` are now imported
under `#[cfg(target_os = "linux")]`. In `hands/tests/evidence.rs`,
`MetadataExt` and the `identity` helper are Linux-only too, since only
the namespace and edge tests read them. Every new production child is
`#[cfg(target_os = "linux")]`, as are the new test-only re-exports.

### Files, quality and gates

The new private children of `hands/evidence.rs` are
`hands/evidence/observed.rs` (152 lines), `hands/evidence/options.rs`
(150) and `hands/evidence/verdict.rs` (214). `hands/evidence.rs` drops
from 725 to 453 lines. `hands/namespace/sources/host.rs` drops from 799
to 798: `records` collects the parsed lines in one expression, and
`record` folds its tuple, to make room. On the test side,
`hands/tests/evidence.rs` is 1,572 lines and `hands/tests/sources.rs`
1,892. `hands/tests.rs` drops to 1,678, `cli/tests/hands.rs` to 974 and
`engine/boundary_tests.rs` to 4,243, from run 5's 4,292;
`boundary_tests/evidence_tests.rs` is 265. Each row in
`quality/file-lines.txt` and `quality/too-many-lines.txt` was set from
`wc -l` and from the forced `clippy::too_many_lines` run measure.sh
makes. No production file outside the row and its private children
changed, and no public item was added.

Formatting and workspace clippy, with all targets and features, were both
clean. The brokkr-protocol suite ran 986 lib tests with 3 ignored, 6 in
`hands_exits`, the secret-drop test and the doc test. brokkr-runtime ran
1,040 lib, 76 `capability_launch`, 132 `it`, 1 `operated_repo` and 6
`queued_launch`. brokkr-cli ran 882 lib, 716 `it` with 3 ignored, 27
`driver_conformance`, 14 `transcript_surfaces` and the three heap
binaries. The core, store, view, bridge and seatbelt-probe suites all
passed. `recipes/self` compiled to 47 valuations over 11 groups and
`recipes/verify` to 16 over 5, none unruled. The `files`, `clones` and
`api` ratchets hold. `baselines ddffa2aa` lists only the four lengths
above, run 5's staged test-clone fingerprints, the controller's ruled
prod clone, and the ruled API rises (brokkr-cli 11 → 13, brokkr-protocol
1,167 → 1,170).

38.1 and 38.2 are ticked on these observations. Still pending: the exact
coverage gate, the controller's host check, macOS CI, and remote CI on the
final head. By reading, every new branch and closure is reached by a test
above, including the `statx` answer without a mount id (`named`) and a
block device (`numbered`), which no CI container has. That reading is not
a coverage run.

## Seventh run: the root narrowed to one local region, every field strict

The seventh run (`0065-slice-two-unit-u8a-see-the--287ae0fc`) started from
a9a09d63's work, staged on `ddffa2aa` with the controller's prod clone
refresh inside it, and kept both. Review had held run 6 on H7 and on the
rest of H6. Both answers below make the check stricter, so no operator
ruling was needed.

### H7: what the root may stand on

Run 6 judged an overlay that backs the root by every layer it reads or
writes. Review showed that each layer then needs the whole alias analysis
again. This run removes that case instead of answering it, in
`verdict::one`:

| Narrowing | Verdict |
| --- | --- |
| the root's own mount is an overlay, whatever its layers | `Stacked(point)` |
| the root's own mount is any stacking or remote type (`fuse.*`, `nfs4` and the rest of `STACKED`) | `Stacked(point)`, as before |
| the root's own mount is a type outside `LOCAL` | builds only on H5's observed block device of its own number, as before |
| any mount whose point lies strictly inside the root, whatever its type | `Nested(point)`, a new variant: "the mount at '…' lies inside the artifact root" |

The root's own mount point may equal the root. With both narrowings the
protected set is one region: the root on its own local mount. The rest of
run 6's rule covers every alias of it, unchanged. `shown_elsewhere` refuses
a same-device mount anywhere whose root field overlaps that region. A
judged overlay whose observed upper or work directory is the root, lies
inside it or holds it refuses `Overlaid`. An unobserved upper or work
directory refuses `Unfollowed`. The judged set, H5's block device, strict
option parsing and the read-only exception for the lower and data layers
of an overlay in reach are all unchanged.

What went away with the backing case: the `Underlaid` variant, the
`backs` flag that `verdict::judged` carried, the read layers that
`verdict::wanted` asked the edge to observe, the `read` half of the old
`options::Layers` (the function is now `options::written`, returning the
upper and work directories after the same strict reading of every layer),
and `Source.identity`, which only `Underlaid` read. `Mounts.sources` now
keeps each writable source's canonical path alone. The worktree is no
longer `stat`ed once more at observation for that identity; its
canonicalisation and the `inside` walk still refuse one that cannot be
observed.

### H6: every mountinfo field decoded strictly

`host::record` no longer skips the optional fields. Each one before the
lone `-` must be `shared:N`, `master:N` or `propagate_from:N`, with N
decimal digits only, or exactly `unbindable` (`host::tagged`). Anything
else, `shared:\9` among them, refuses the whole table. `host::unescaped`
now refuses a literal NUL as well as an escaped one. The source observer
reads this table too, so both changes apply to it, and its suite passed
unchanged.

The root field is now read with the same strict path check as the point
and the layer paths (`options::path`: absolute, no NUL, no `.` or `..`
component, no ` (deleted)` ending) in two places. `verdict::one` checks it
for every judged record. `evidence::strict`, called by `shown_elsewhere`
for each part's device, checks every record of that device, since those
roots place the aliases. A record of another device is not read by the
alias check, so a pseudo filesystem's non-path root elsewhere in the table
(nsfs) still parses.

### Kept error kinds (INFO)

`unaliased_with` now takes `FnOnce() -> io::Result<Vec<u8>>`. A table
that cannot be read refuses `Unobserved("/proc/self/mountinfo", kind)`. A
root that `statx` cannot answer for refuses `Unobserved(root, kind)`. An
answer without a mount id still refuses `Unmapped(root)`.

### Tests

All are in `hands::tests::evidence` and run on Linux only. Three new tests
replace `an_overlay_the_root_stands_on_refuses_a_layer_the_box_writes`,
whose cases no longer exist. Two more cover H6.

| Test | What it asserts |
| --- | --- |
| `the_root_stands_only_on_a_local_filesystem` | an overlay as the root's own mount refuses `Stacked("/")` in four forms: layers observed and apart from every source, additive `lowerdir+`/`datadir+`, legacy with a `::` data separator, and a container's layers under `/var/lib/docker`. `fuse.sshfs` and `nfs4` refuse `Stacked("/")`; `ext4` and `tmpfs` build |
| `nothing_mounted_inside_the_root_builds` | a `tmpfs` and a same-device bind at `<root>/sha256` each refuse `Nested`, both with every source and with only `/p/home`, which does not reach them. A root that is its own mount point builds |
| `every_alias_of_the_root_region_refuses` | review's three configurations, against the root. A bind at `/s/rw/view` of `<root>/sha256` refuses `Mounted("/s/rw/view")`. A bind of the root's parent at `/s/rw` refuses `Mounted("/s/rw/artifacts")`. An overlay at `/g/common/v` with its upper in the root refuses `Overlaid` |
| `a_field_the_kernel_never_writes_refuses_the_alias_check` | through `shown_elsewhere`: same-device roots `/tree/./root`, `/tree/../root` and `/tree\040(deleted)` refuse `Unparsed("/else")`; a literal NUL in the root and `shared:\9` refuse `Unmapped`. Every kernel-written optional field together, and none at all, parse |
| `a_field_the_kernel_never_writes_refuses_the_verdict` | through `reached`: a judged record (another device, at `/w/m`) with each of the three roots refuses `Unparsed("/w/m")`, and so does a parsed record given a NUL root. The two that never parse are asserted at the parser, `Err(Identity)`, the only way to `reached` |

These existing tests changed with the rule. In the stacking test, each
family inside the root now refuses `Nested`. The unobserved-layer test
drops its case of the root's own overlay, now `Stacked`. The edge test
names its directory, file and absent layers as upper and work directories
of two overlays, since lower layers are no longer wanted. The text test
pins `Nested` where `Underlaid` stood. The root-mount test asserts the kept
kinds. In the real-namespace test, a bind of the digests at their own
place inside the root now refuses with `Nested`'s text, observed in the
namespace. Before, it built.

### Removal controls of this run

The four production files were copied aside before the first mutation.
Each batch held at most one mutation per test it targets. Each was
compiled and run against `hands::tests::evidence`, then the copies were
restored. After the last batch, `cmp` against the copies was silent for
all four, and a grep for the mutation markers found none.

| Mutation | Failed test, case | Observed |
| --- | --- | --- |
| R1 the root's own overlay judged like one in reach | `the_root_stands_only_on_a_local_filesystem`, the disjoint-layers overlay | `Ok` against `Stacked("/")` |
| R2 a stacking type admitted as the root's own mount | the same, `fuse.sshfs`; the stacking test's "fuse own" | `Ok` against `Stacked("/")` |
| R3 the nested check removed | `nothing_mounted_inside_the_root_builds` (`tmpfs`), the stacking test ("fuse inside"), and the real-namespace test | `Ok`; `Stacked` against `Nested`; the own-place bind "built" |
| R4 an alias whose root lies inside the part dropped | `every_alias_of_the_root_region_refuses`, the exposing bind; the table test ("directory"); the namespace test | `Ok` against `Mounted("/s/rw/view")` |
| R5 an alias whose root holds the part dropped | the same test, the parent bind; the table test ("holder"); the namespace test | `Ok` against `Mounted("/s/rw/artifacts")` |
| R6 an upper inside the root not refused by path | the same test, the overlay; the H3 test ("upper inside") | `Ok` against `Overlaid("/g/common/v")` |
| R7 `.` admitted as a component | the alias-check field test ("dot"), the verdict field test (`/tree/./root`), the option test | `Ok` against `Unparsed` |
| R8 `..` admitted as a component | the two field tests at `..`, the option test | `Ok` against `Unparsed` |
| R9 the deleted mark admitted | the two field tests at `\040(deleted)`, the option test | `Ok` against `Unparsed` |
| R10 a literal NUL decoded by `unescaped` | the alias-check field test ("nul"); the verdict field test, the parser case | `Unparsed("/else")` against `Unmapped`; `Ok([Record { root: "/tree\0root", .. }])` against `Err(Identity)` |
| R11 a NUL admitted by `options::path` | the verdict field test, the planted record | `Ok` against `Unparsed("/w/m")` |
| R12 any optional field admitted | the alias-check field test ("optional"); the verdict field test, the parser case | `Ok` against `Unmapped`; `Ok([..])` against `Err(Identity)` |
| R13 the verdict's root check removed (point checked twice) | the verdict field test, `/tree/./root` | `Ok` against `Unparsed("/w/m")` |
| R14 the alias check's `strict` call removed | the alias-check field test ("dot") | `Ok` against `Unparsed("/else")` |
| R15 `Unparsed`'s text changed to "do not read" (the LOW) | `each_mount_verdict_states_the_managed_writer_refusal` | "… do not read" against "… do not parse" |
| R16 an unobserved root mount reported as `Unmapped` | `a_root_whose_mount_is_unproved_refuses` | `Unmapped(<tmp>/absent)` against `Unobserved(<tmp>/absent, NotFound)` |

R3 was run alone as well, so the namespace failure is its own: R4, in the
same batch, also failed that test at an earlier case. Each test passed
again on the restored tree.

### Limitations

These limitations are recorded and none is fixed here.

A workspace box refuses where the run-local directory stands on an
overlay, a stacking filesystem or a remote one, or where anything is
mounted inside its `artifacts/`. This matches the broker, which already
refuses overlay identities. CI keeps the root on ext4 or tmpfs.

ZFS and any other filesystem outside `LOCAL` whose source is a pool or
dataset name, not a block device, refuse wherever they are judged. That is
in reach, holding a writable source, or as the root's own mount, so a
run-local directory on ZFS refuses.

The root field's strict check refuses the ` (deleted)` ending that the
commission names. Review confirmed that the kernel can instead mark a
deleted mount root `//deleted`. Run 8 now refuses that spelling, along with
every other empty component (see the eighth run below).

The namespace-name, pathname-window and stale-table limitations stand as
run 6 recorded them.

### Files, quality and gates

No new file this run. `hands/evidence.rs` is 473 lines,
`evidence/observed.rs` 150, `evidence/options.rs` 140 and
`evidence/verdict.rs` 197. `hands/namespace/sources/host.rs` is at its
800-line ceiling. To make room for `tagged`, three expressions there were
written shorter with the same behaviour: `readable`'s refusal arms,
`others`' filter and `record`'s option list. `hands/tests/evidence.rs` is
1,648 lines. Each row in `quality/file-lines.txt` was set from `wc -l`.
No public item changed, and the `api` ratchet holds. The `files` and
`clones` ratchets hold, no baseline file was edited, and no new clone
was added. `baselines ddffa2aa` lists exactly run 6's entries.

Formatting and workspace clippy, all targets and features, were clean.
brokkr-protocol ran 990 lib tests with 3 ignored, plus `hands_exits` (6),
the secret-drop test and the doc test. brokkr-runtime ran 1,040 lib, 76
`capability_launch`, 132 `it`, 1 `operated_repo` and 6 `queued_launch`.
brokkr-cli ran 882 lib, 716 `it` with 3 ignored, 27 `driver_conformance`,
14 `transcript_surfaces` and the three heap binaries. Every other
workspace crate passed. `recipes/self` compiled to 47 valuations over 11
groups and `recipes/verify` to 16 over 5, none unruled. `openspec validate
--all --strict` passed 26 of 26, and `typos --hidden` and `git diff
--check` were clean. The real-namespace evidence test ran here, with no
`skipped:` line under `--nocapture`.

38.1 and 38.2 are ticked on these observations. Still pending: the exact
coverage gate, the controller's host check, macOS CI and remote CI on the
final head.

## Eighth run: no empty component, and an empty source parses

Run 8 starts from run 7's signed work (0ea69c71), restaged on ddffa2aa.
Review held run 7 on one HIGH, H6's last spelling, and one INFO on
availability. Nothing else in the unit moved.

### H6: an empty component refuses

In the mount-root field, the kernel marks a deleted root `//deleted`, not
only ` (deleted)`. `evidence/options.rs::path` now refuses every empty
component. That means any path other than `/` itself that holds `//` or
ends in `/`, which the kernel never writes for a live path. The existing
` (deleted)`, `.`, `..` and NUL refusals stand, and `/` stays valid.

Both consumers carry the new cases. `a_field_the_kernel_never_writes_refuses_the_alias_check`
covers the same-device alias check (`evidence.rs::strict`). Its records of
the root's device with roots `/tree//deleted`, `/a//b` and `/a/` each
refuse as `Unparsed("/else")`, and its own record, rooted at `/`, passes.
`a_field_the_kernel_never_writes_refuses_the_verdict` covers `reached`,
through `verdict.rs::one` over every judged record. A judged record at
`/w/m` with each of those three roots refuses as `Unparsed("/w/m")`, and
one rooted at `/` passes with `Ok(())`.

Each consumer has its own compiling removal control. Each control puts a
temporary copy of `path` without the empty-component rule into one
consumer only, and was restored afterwards:

| Control | Failing test | Observed |
|---|---|---|
| alias check reads the copy | `a_field_the_kernel_never_writes_refuses_the_alias_check` | case `deleted root`: `Ok(())`, expected `Err(Unparsed("/else"))` |
| `one` reads the copy for point and root | `a_field_the_kernel_never_writes_refuses_the_verdict` | root `/tree//deleted`: `Ok(())`, expected `Err(Unparsed("/w/m"))` |

Under each control the other consumer's test still passed, so each test
binds its own consumer.

### INFO: an empty source parses as empty

The stricter shared `host::records` parser had refused a record with an
empty source field. The kernel writes one for a mount made with an empty
device name, so every box on such a host would have refused. The source
field is now read as empty when it is empty, and is otherwise decoded
strictly as before. The root, point and type fields still refuse when
empty. To keep `sources/host.rs` at its 800-line ceiling, the parent id's
check joined `record`'s parsed tuple, with the same refusal.

The recorded-table tests are these. In
`an_unknown_filesystem_builds_only_on_its_own_block_device`, a `vfat` mount
at `/w/m` with an empty source refuses as `Stacked("/w/m")`, because there
is no block device to observe (H5), and an `ext4` one with an empty source
passes. In `the_mount_table_is_read_strictly_and_within_its_bound`,
`store::table` reads a recorded table whose record has an empty source as
`Ok`, with the source `""`. The empty-source line left that test's
malformed list, where run 7 had put it. In
`only_the_mount_records_the_walk_stood_on_are_counted_or_digested`, a
mount elsewhere whose source is empty moves no sealed fact in the
`sources.rs` observer, which is main's behaviour.

| Control | Failing test | Observed |
|---|---|---|
| `record` decodes the source with `unescaped` alone | `an_unknown_filesystem_builds_only_on_its_own_block_device` | the recorded table did not parse (`records(..).unwrap()` panicked) |
| same | `the_mount_table_is_read_strictly_and_within_its_bound` | `Err(Identity)`, expected `Ok("")` |
| same | `only_the_mount_records_the_walk_stood_on_are_counted_or_digested` | `Err(Identity)`, expected `Ok((2, 1, ..))` |
| `one` passes an unknown type whose source is empty | `an_unknown_filesystem_builds_only_on_its_own_block_device` | `Ok(())`, expected `Err(Stacked("/w/m"))` |

### Files, quality and gates

No new file this run. `evidence/options.rs` is 144 lines,
`sources/host.rs` stays at 800, `hands/tests/evidence.rs` is 1,665 and
`hands/tests/sources.rs` is 1,900. Their `quality/file-lines.txt` rows
were set from `wc -l`. No public item changed, no clone baseline was
edited, and `compose_site`/`compose_site_at` keep their parameter lists.

Formatting and workspace clippy, all targets and features, were clean.
brokkr-protocol ran 990 lib tests with 3 ignored, plus `hands_exits` (6),
the secret-drop test and the doc test. brokkr-runtime ran 1,040 lib, 76
`capability_launch`, 132 `it`, 1 `operated_repo` and 6 `queued_launch`.
brokkr-cli ran 882 lib with 1 ignored, 716 `it` with 3 ignored, 27
`driver_conformance`, 14 `transcript_surfaces` and the three heap
binaries. The other workspace crates' 16 test binaries passed.
`recipes/self` compiled to 47 valuations over 11 groups and
`recipes/verify` to 16 over 5, none unruled.

38.1 and 38.2 are ticked on these observations. Still pending: the exact
coverage gate, the controller's host check, macOS CI and remote CI on the
final head.

## Controller's coverage repair (2026-10-17, before the re-vouch)

PR #729's first CI run failed the exact coverage gate: 60817 of 60820 lines and 8015 of 8016 branches; every function was hit. The four misses, from the run's `coverage-exact` lcov, and their repairs:

- `hands/namespace/sources/host.rs:69`. `readable`'s refusal of an open that fails other than EACCES was its own arm, which no test reaches. It is again the one expression main had: the error is classified as denied or not, and `denied.then_some(None).ok_or(Refusal::Identity)` answers both. The behaviour is unchanged and the file stays at its 800-line ceiling.
- `hands/evidence.rs:396`. A root whose mount the kernel names no id for (`Contained::Unnamed`: statx without STATX_MNT_ID, a kernel before 5.8) refuses as `Unmapped`. No live kernel here gives that answer. `unaliased_with` now hands the observation to `unaliased_on`, which takes it as a fact, in the same way the verdict already takes each source's containment. `a_root_whose_mount_is_unproved_refuses` asserts `unaliased_on(root, Contained::Unnamed, live)` refuses `Unmapped(root)`.
- `hands/evidence.rs:345`, the branch where the guard is false. A `digests` lookup that fails other than NotFound falls to `directory`, which refuses it as unobserved. The new `an_unsearchable_artifact_root_refuses_unobserved` makes the root 0600, so the lookup is EACCES, and asserts `Unobserved(<root>/sha256, PermissionDenied)`.
- `hands/evidence/options.rs:132`. A legacy `lowerdir` value carrying a backslash escape. `an_overlay_the_box_reaches_that_writes_into_the_root_refuses` now builds a reached overlay with `lowerdir=/a\134:b:/l`, which the kernel splits as `/a:b` and `/l`. `options_that_do_not_parse_refuse` adds `lowerdir=/l\134`, a trailing escape, which refuses as Unparsed.

A scoped `cargo llvm-cov --branch` over `brokkr-protocol --lib hands::` on this host reports no zero-count line or branch in the three files, with the four spots at 1 or more. All 159 `hands::` tests pass on this host with BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1. Before this repair, the controller's host check ran the namespace proofs the seats skip, with skips forbidden: protocol `hands::` (159 passed) and `hands_exits` (6); runtime `engine::boundary_tests` (45) and `engine::capability_tests` (57); and cli `--test it` `hands::` (13), `capability_artifacts::` (2) and `doctor` (47). Clippy (`-D warnings`, all targets), fmt, and the file and clone ratchets pass.

## Re-vouch (2026-10-17, run 0065-slice-two-unit-u8a-see-the--dee77dc2)

The net diff of PR #729, with the controller's repair, was re-vouched on
ddffa2aa without a change to code or tests; this paragraph is the only
edit. Formatting, `git diff --check`, `typos --hidden` and workspace
clippy under `-D warnings` with all targets and features were clean.
brokkr-protocol's lib suite passed 991 with 3 ignored, one more than the
eighth run for the controller's new test. brokkr-runtime passed 1,040 lib
and 76 `capability_launch`, and brokkr-cli's `it` binary passed 716 with
3 ignored. `openspec validate --all --strict` passed 26 of 26. The file,
clone and public API ratchets hold. Against main, the measured public API
moves by +2 lines in brokkr-cli, +3 net in brokkr-protocol (13 added, 10
removed) and 0 net in brokkr-runtime (2 for 2), inside the operator's
rulings. L1–L5 stay recorded as follow-ups. The exact coverage gate, macOS
CI and remote CI on the new head remain pending.
