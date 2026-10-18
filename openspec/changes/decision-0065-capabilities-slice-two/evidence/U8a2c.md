# U8a2c evidence: build every workspace box from the reach sealed at admission

U8a2c was built on branch `s2/U8a2c`, cut from main at `db59dfe1` (U8a2a,
#745), in run `0065-slice-two-unit-u8a2c-see-th-54136b0d`. An earlier
implement visit of the same run left three signed work-in-progress commits
and an uncommitted tree; this visit reviewed that work, finished it, folded
the clones it added, strengthened R2's proof, ran every gate and control
below, and squashed the whole unit into one signed commit. Everything below
was observed on the tree committed with this file.

U8a2c shares tasks 38.3 and 38.4 with U8a2a and U8a2b (operator ruling
2026-10-18), so neither is ticked: this file records U8a2c's part. U8a2b
still owns the program-source and system-support alias matrix.

Rulings: operator ruling 2026-10-18 splitting U8a2c from U8a2a after its
third run (the row's files; a public API rise of at most +10 lines across
the protocol, runtime and CLI snapshots), and operator ruling 2026-10-18
after U8a2a's fourth run (R1, R2 and R3 fixed here).

## Scope

| File | Change |
| --- | --- |
| `crates/brokkr-protocol/src/hands.rs` (row; frozen at 1,158) | `box_argv`, `execute`, `execute_in`, `run_boxed`, `run_boxed_in` take the sealed reach in place of the HOME and `GitFacts`; the worktree, the Git common directory and every bind are mounted from it; the reach is confirmed before the first mount and again once the argv is final; `serve_args` and `mcp_config` carry it. Git facts moved out. 1,158 → 1,094 lines. |
| `crates/brokkr-protocol/src/hands/sealed.rs` (new child of `hands.rs`, 230 lines) | `Sealed` (HOME, worktree, Git directory and common directory, each bind's source, each by canonical path and device/inode), `seal`, `drifted`, `Sealed::confirm`, `Drifted` (MB3's identity cause) and `mount_binds`. |
| `crates/brokkr-protocol/src/hands/git.rs` (new child of `hands.rs`, 107 lines) | `GitFacts`, `git_facts`, `git_dirs`, `git_identity` and `parse_git_dirs`, moved out of `hands.rs` unchanged in behaviour. |
| `crates/brokkr-protocol/src/hands/overlay.rs` (existing child) | `overlay_argv` takes the sealed lower layer apart from the authored mount point. |
| `crates/brokkr-protocol/src/native_controls/mcp.rs` (row) | `Transport` gains `sealed`; `Owed::is` reads the carried `--reach` as `Sealed` and compares it field by field; `Servers::expand` refuses a reach that does not serialize. 713 → 720 lines. |
| `crates/brokkr-protocol/src/adapters/mcp.rs` (row) | The adapter edge parses `sealed_reach` from the sealed inputs into `Sealed` once; one absent or unreadable binds the default, which no engine hands carry. 790 → 800 lines. |
| `crates/brokkr-protocol/src/probe/observe.rs` (row) | The adapter probe seals its own reach when its scratch world is laid out. |
| `crates/brokkr-cli/src/lib.rs` (row; frozen at 1,908) | `HandsCommand` moved to `hands.rs` and re-exported. 1,908 → 1,871 lines. |
| `crates/brokkr-cli/src/hands.rs` (row) | `--reach` on `serve` and `exec`, parsed as `Sealed` before any session or box; one that does not read refuses with MB3's identity cause. 66 → 128 lines. |
| `crates/brokkr-cli/src/doctor.rs` (row; frozen at 1,565) | The box probe seals its own reach as it starts. 1,565 → 1,562 lines. |
| `crates/brokkr-runtime/src/engine.rs` (row; frozen at 3,526) | The engine holds the reach its admission sealed (`reach`), composes every box from it, writes it beside the artifact root; `compose_site`/`compose_site_at` take `(artifacts, sealed)`. `runtime_hands` moved to `boxed.rs`. 3,526 → 3,523 lines. |
| `crates/brokkr-runtime/src/engine/broker.rs` (row) | Re-exports `Authored` beside `Held`; size-neutral (779). |
| `engine/broker/admission.rs` (existing child) | The reservation's `Reach::Boxed.sealed` is `hands::Sealed`, made by `hands::seal`, the one sealing; `Authored::sealed` finds a box's reach; `admit_in` keeps the claim's reach for the attempt, lease or none; a ship member's or step's own hands are sealed beside the journal-widened ones. 778 → 777 lines. |
| `engine/boxed.rs`, `engine/isolation.rs`, `engine/dispatch.rs`, `engine/panel.rs`, `engine/sequence.rs` (existing children of `engine.rs`) | `hands_command` refuses a box with no sealed reach (`Unnamed::Unsealed`) and carries `--reach` to `hands exec`; `beside` expands with it; the single driver's spawn is composed once the attempt is admitted (`compose_single`), as members and steps already are; each `mark_serving` call passes the site's hands. |
| `engine/broker/root.rs`, `historical.rs`, `settlement.rs` (existing children) | R1, R2 (below). |

New production files: `crates/brokkr-protocol/src/hands/sealed.rs` and
`crates/brokkr-protocol/src/hands/git.rs`, each with its row in
`quality/file-lines.txt`. `brokkr-runtime` gains a `sha2` dependency edge
for R2's incremental digest: a package `Cargo.lock` already held at the
workspace version, so the package count stays 329. No new authored
`HandsSpec` key, trait, flag, daemon, suppression or `unsafe`. D6's
exclusion and the door's recheck are unchanged in what they refuse; the
door now asks `hands::drifted`, the same observation hands makes.

## The mechanism as built

Admission seals, for each boxed writer, the reach its boxes are built from:
the HOME a bind's `~/` names, the worktree, the Git directory and Git
common directory Git names, and each bind's source, each canonicalized and
paired with its device and inode (`hands::seal`). The reservation holds
that reach, and the engine holds the same one for the attempt in flight,
under a lease or holding none (C1). Every box the attempt composes takes
it: `hands exec` and `hands serve` receive it as `--reach` JSON, the final
check compares it field by field, and the adapter reads it from the sealed
inputs. A box whose hands were sealed nothing refuses before spawn with
`dispatch refused: MCP server box filesystem identity is not protected: the
box's reach was not sealed at admission, or is not UTF-8, so no box of these
hands is built from it (U8a2c)`; nothing derives a reach again.

Hands mounts the worktree from its sealed path, each bind from its sealed
canonical source at the path its authored spelling names under the sealed
HOME, and the Git common directory from the sealed reach; Git is not asked
where they are. Before the first mount, and again once the argv is final,
hands observes the reach again exactly as it was sealed: a source, an
ancestor of one or the worktree's Git pointer or `commondir` that no longer
resolves to its sealed identity refuses with `MCP server box filesystem
identity is not protected: '<path>', which the box is built from, no longer
resolves as it was sealed`; a reach that does not seal each declared bind
refuses with `…: the box's reach was never sealed for the hands it
declares`. bubblewrap still resolves each source once more as it mounts it,
a window the second observation narrows and cannot close; the `KNOWN OPEN`
common-directory write mount of decision 0054 is unchanged.

The adapter probe and doctor's box probe admit nothing: each seals its own
reach as it starts, with the same `hands::seal`.

## R1, R2 and R3

R1: a reseal is committed only once the record that stood is retired.
`root.rs::retired` hands the hook `Step::Retiring`, checks the shared
deadline the last time, and removes the prior name; a failure or lateness
up to there undoes the seal, the record that stood back under its name with
its bytes and inode. After that point the seal reports itself committed:
its last directory sync failing returns `Commit::Unsynced`, never a
failure. Join, the unresolved-cleanup mark, the positive fact, release and
recovery carry it; a lease holding an unsynced commit fails its run as it
settles where the fence rule asks it (`Unknown::Unsynced`, `MCP server box
filesystem identity is not protected: the attempt's reservation was
committed, but its directory was not proven synced`).

R2: `historical.rs::digested` hashes each chunk as it is read, the digest
so far handed to the test seam and the shared deadline checked after each
chunk; nothing but the digest is kept. This visit changed the seam from the
byte count to the hasher itself, so the test compares the digest after
each chunk with the digest of that prefix: with the earlier seam a buffered
hash still passed.

R3: the link-failure proof plants a file at the record's prior name
(`Late::Occupy`), so the link alone fails while the directory stays
writable and the rename after it stays possible.

## Proof and removal controls

Each control below is a compiling mutation of the production code, run
against the owning suite, then restored; every listed test failed under it
and passed again after restoring (the full suites below are on the
restored tree).

| Control | Mutation | Tests that failed (file:line of the failed assertion) |
| --- | --- | --- |
| Sealed carriage removed | `box_argv` seals a fresh reach from the spec, the HOME and Git now, and builds from that | `a_reach_that_does_not_seal_the_box_refuses_it` (sealed.rs, `drifted_of`), `a_reach_that_no_longer_resolves_as_sealed_refuses_before_the_box_starts` (same), and in real namespaces `a_writers_own_box_cannot_redirect_a_later_box_through_a_source` and `…_through_its_git_pointer` (`refused`: the call ran instead of `Err(moved(path))`) |
| Drift check removed | `Sealed::confirm` never reports `drifted` | `a_reach_that_no_longer_resolves_as_sealed_refuses_before_the_box_starts`, `a_source_replaced_as_the_argv_is_made_final_refuses_the_box`, both real-namespace proofs |
| Git asked at mount | the common directory taken from `git_dirs(workdir)` | `a_box_binds_the_sealed_common_directory_whatever_git_answers_meanwhile` |
| Binds from the spelling | `mount_binds` mounts from the authored path | `every_bind_is_mounted_from_its_sealed_source_at_its_authored_spelling` |
| Final-argv recheck removed | the second `sealed.confirm` dropped | `a_source_replaced_as_the_argv_is_made_final_refuses_the_box` |
| Outside common directory not bound | its in-place bind dropped | `the_git_directories_a_box_binds_are_those_sealed`, `a_box_binds_the_sealed_common_directory_whatever_git_answers_meanwhile`, and both real-namespace proofs at the commit in `attacks` |
| Transport ignores the reach | `Owed::is` accepts any `--reach` | `native_controls::mcp::tests::the_hands_entry_is_read_with_its_artifact_root` (the resealed entry was accepted) |
| Adapter drops the carried reach | `sealed_reach` always returns the default | `adapters::mcp::tests::the_hands_are_bound_to_the_reach_the_engine_sealed` (the bound launch refused) |
| CLI reads no reach | `parse_reach` falls back to the default | `hands::tests::a_reach_that_does_not_read_refuses_both_verbs_before_any_box` (a session was made) |
| Admission's reach not kept | `admit_in` drops `claim.authored` | `reach::a_seats_box_is_built_from_the_reach_its_admission_sealed` ("sealed at admission") |
| Box from a reach sealed now | `compose_at` seals a fresh reach | `reach::a_seats_box_is_built_from_the_reach_its_admission_sealed` (the carried reach), `reach::a_box_whose_hands_were_sealed_nothing_refuses_its_spawn` |
| Unsealed box falls back | `hands_command` uses `Sealed::default()` | `reach::a_box_whose_hands_were_sealed_nothing_refuses_its_spawn` |
| Member's own hands unsealed | `writers` seals only the widened hands | `reach::a_ship_members_own_hands_are_sealed_beside_the_widened_ones` |
| R1: final sync reported as failure | `retired` returns `Unsealed` when the last sync fails | `sealing::a_reseal_whose_last_sync_fails_reports_itself_committed` (join), `sealing::an_unsynced_settlement_fails_its_run_under_the_fence_rule` |
| R1: no deadline as it retires | `retired` skips its deadline check | `sealing::a_late_join_keeps_the_reservation_that_stood`, `a_late_release_…`, `a_late_unresolved_mark_…`, `a_late_recovery_…` (each at its "late as it retires the record that stood" case) |
| R1: join's commit dropped | `admit_with` ignores the join seal's `Commit` | `sealing::a_reseal_whose_last_sync_fails_reports_itself_committed` ("joined: held unsynced") |
| R1: mark's commit dropped | `witnessed_by` ignores it | the same test ("marked: held unsynced") |
| R1: recovery's commit dropped | `recovered` ignores it | the same test ("recovered: held unsynced") |
| R1: settlement ignores unsynced | every `(commit, unsynced)` pair settles | `sealing::a_reseal_whose_last_sync_fails_reports_itself_committed` ("marked: its run failed"), `sealing::an_unsynced_settlement_fails_its_run_under_the_fence_rule` |
| R2: hashed once at the end | the bytes kept and hashed after the loop | `coordination::a_deadline_that_passes_inside_an_artifacts_read_ends_its_verification` (digest after the first chunk) |
| R2: no deadline while hashing | the loop's deadline check ignored | the same test (`Err(Identity(Expired))` expected) |
| R3: link error not refused | the non-`NOENT` link error treated as no record standing | `sealing::a_reseal_that_cannot_link_the_record_that_stood_keeps_it` (`mb3(Unknown::Unsealed)` expected) |

R1's release site is bound by the R1 test's direct `release(..) ==
Ok(Commit::Unsynced)` assertion, which the "final sync reported as failure"
control breaks; no separate control mutates the `fact.and(release)` fold
in `settle`, whose `FailFinal` fixture fires at the positive fact first.

The real-namespace proofs ran in this seat: bubblewrap could build a
namespace here and `BROKKR_HANDS_BOX` was unset, as the controls above show
by failing inside the boxes. In them, after the reach is sealed, an
unchanged writer's repeated calls, a write and a commit in its linked
worktree and an exec run; its own box then replaces an ancestor of a bind
source by a link toward protected evidence, puts a link toward the
coordination root where an absent source stood, renames another directory
into that place, hard-links a file of its worktree there, rewrites the
worktree's Git pointer to name another repository's Git directory, and
rewrites its per-worktree `commondir`. Each time its next workspace call
and the following exec refuse before spawn with MB3's exact cause naming
the sealed path, the evidence, the coordination record and the other
repository's `HEAD` byte-for-byte unchanged, and once undone the writer
runs again. A box cannot hard-link across its bind mounts, so its hard
link reaches only its own worktree; one toward evidence outside is refused
by the kernel before the reach is observed. The hard-link case was added
after the controls above ran; the drift-check control breaks the same test
at its first attack, so it does not isolate that case.

## Tests moved, renamed or folded

| Test or helper | From | To |
| --- | --- | --- |
| `retiring_confine_leaves_plain_seat_member_and_step_argv_untouched` | `engine/boundary_tests.rs` (at its baseline) | `engine/boundary_tests/sealed_tests.rs` |
| `the_hands_exec_verb_boxes_a_command_and_refuses_a_bad_spec` | `brokkr-cli/src/tests.rs` (over its ceiling) | `brokkr-cli/src/hands/tests.rs` |
| `transport`, `escaped_transport` and their hand-written pins | `native_controls/tests.rs` (over its ceiling) | `native_controls/tests/transport.rs` |
| `fixture` | `hands/tests/evidence.rs` and `hands/tests/sealed.rs` | `hands/tests.rs` (one copy) |
| `boxed_from` | the box skeleton of `workspace_argv`, evidence.rs's `boxed` and sealed.rs's `built` | `hands/tests.rs`, which the three now call |
| `FENCED`, `fenced`, `settles` | the fence-rule loops of two sealing tests | `admission/tests/sealing.rs` |

New test modules: `hands/tests/sealed.rs`, `native_controls/tests/transport.rs`,
`engine/boundary_tests/sealed_tests.rs` and
`engine/broker/admission/tests/reach.rs`, each declared from its parent
suite and each with its `quality/file-lines.txt` row. The parent
boundary suite went 4,242 → 4,207 lines.

## Gates

On the committed tree: `cargo fmt --all -- --check` and the workspace
clippy with `-D warnings` were clean. `brokkr-protocol` passed 1,006 unit
tests (3 ignored) and its integration targets; `brokkr-runtime` passed
1,131 unit tests, `capability_launch` 76, `it` 134 (witness digests among
them, no row moved, `witnesses.json` untouched), `operated_repo` and
`queued_launch`; core, store, view, bridge and the seatbelt probe passed.
`brokkr-cli` passed 895 unit tests and every integration target except, in
one full run under a load average of 17 to 21, three timing-bound broker
tests outside this unit's code (`capability_broker::cleanup::a_broker_killed_mid_call_takes_its_box_with_it_and_the_next_start_reaps_its_scratch`,
`capability_broker::serving::an_answer_its_harness_takes_one_write_of_holds_no_part_of_a_number`,
`capability_broker::serving::a_standard_error_nobody_reads_never_holds_the_sessions_end`);
run again alone, all three passed, and the earlier full run of the same
suite on the WIP tree passed them. `compile --bundle recipes/self`
succeeded (policy sweep: 47 valuations over 11 groups, 0 unruled);
`openspec validate --all --strict` passed 29 of 29; `typos --hidden` and
`git diff --check` were clean; `quality/ratchet.sh files`, `clones` and
`api` held.

Public API, measured with cargo-public-api 0.52.0 on nightly-2026-09-05:
`brokkr-protocol` 1,170 → 1,175 items (`Sealed`, `SEALED_REACH`, `seal`,
`drifted`, `Transport::sealed`; six signatures changed in place),
`brokkr-runtime` 1,778 → 1,778 (`compose_site` and `compose_site_at`
changed in place), `brokkr-cli` 13 → 15 (`HandsCommand::Serve::reach`,
`HandsCommand::Exec::reach`): +7 lines, within the operator's +10.
`quality/ratchet.sh baselines db59dfe1` names only these two snapshots as
raised. `scripts/measure-budgets.sh` measured 329 packages and unchanged
heap peaks; its prompt-byte rows move only by recipe text this unit does
not touch, so its rewrite was not kept.

## Pending, not claimed

The exact coverage gate (`scripts/coverage-exact.sh`) and the CRAP ratchet
it feeds were not run in this seat; they are the host's. macOS build and
tests are CI's. The cross-worktree alias matrix of 38.3–38.4 is U8a2b's.
The U8a2a host note on a group-writable TMPDIR (`support/own_home.rs`) is
untouched by this unit and stands as recorded there.
