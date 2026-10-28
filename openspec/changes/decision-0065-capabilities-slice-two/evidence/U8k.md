# U8k — Mount the sealed sources of the workspace box by descriptor

Tasks 45.15 and 45.16; requirements SI1 and MB3, with SD2 and SD4 for the
verification duty. Branch `s2/U8k`, cut from main at 49e20e92c, recipe
`claude-flash-dsh-adv`. This is U8k's second run,
`0065-slice-two-unit-u8k-see-the--7adaf111`. It replays run 1
(`0065-slice-two-unit-u8k-see-the--03dd3c9d`, stopped REVIEW-SECURITY-HOLD)
from the controller's staged patch and repairs that run's review findings
H1 and L1 to L4 under the operator's rulings of 2026-10-26 ("1.a 2.a").
Everything below was observed in this run's implement seat, on this tree,
on Linux 6.17 with the bubblewrap on `PATH`. Anything this seat did not
observe is listed as pending at the end. Run 1's evidence was rewritten
rather than carried. Its claims about a scratch test that was later deleted
(a mount table showing the lower layer's `lowerdir`) are dropped, since no
command in this run showed them.

The controller's staged preparation rides in this change. It adds U8k's
design row and unit section with both 2026-10-26 ruling paragraphs, tasks
45.15 and 45.16, U8k in U9b1's dependencies, and ticks U0b's tasks 1.5 and
1.6. Those two ticks are the controller's, made when U0b landed as #890; this
seat did not rerun U0b. Run 1 reworded the unit heading to "Mount the sealed
sources of the workspace box by descriptor" because the slug of the
apostrophe form tripped `typos --hidden`. The tasks link names that anchor.

## What the box does now

At the workspace box's last observation, after `protected.confirm()` and
the store's `confirm`, each sealed source is opened once (`Sealed::opened`)
and its handle is held to the sealed device and inode. The argv mounts each
source from its handle. Writable sources take `--bind-fd`: the worktree, a
Git common directory outside it, and a declared `rw` bind. Read-only sources
take `--ro-bind-fd`: a declared `ro` bind and the exec box's engine binary.
An overlay's lower layer is read as `--overlay-src /proc/self/fd/<n>`, and
the same handle is mounted read-only beneath the overlay so that bubblewrap
closes it. A bind sealed absent is not mounted. An overlay over a source
sealed absent refuses. All of this was carried from run 1.

This run adds the sources the box mounts from inside a sealed one
(`sealed/opened.rs`, `Beneath`). Each is opened beneath the handle of the
sealed source that holds it, with one `openat2` using
`RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS | RESOLVE_NO_MAGICLINKS` and
`O_PATH | O_NOFOLLOW`, and is held to what the box expects there:

- **H1, Git's config.** It is opened beneath the retained common
  directory's own handle and must be one regular file. It is mounted
  `--ro-bind-fd`, no longer as an optional `--ro-bind-try` by path. A
  `config` that is gone refuses with `Drifted::Unopened(<config>, NotFound)`.
  A `config` that is not one regular file (a link included, since the path
  handle opens a link as itself) refuses with the new
  `Drifted::Unheld(<config>)`.
- **Common directory inside the worktree.** Where the worktree holds the
  common directory, the directory is opened beneath the worktree's handle
  and held to its sealed identity. Another directory at `.git` refuses with
  `Drifted::Moved(<common>)`. The `hooks` tmpfs and the config's target are
  box paths, which bubblewrap resolves inside the box on the directory as
  the box mounts it.
- **L3, the store's folder pins and the artifact root.** The second
  observation (`hidden.rs`) now records, beside its description, the
  device and inode of each pin and of the artifact root the worktree holds.
  `Hidden::confirm` compares them with the first observation. Each pin is
  opened beneath the worktree's or its bind's handle, and the root beneath
  the worktree's handle, and each must be the object the observations saw.
  Otherwise the box refuses with `Drifted::Moved(<path>)`. Pins mount
  `--bind-fd` and the root mounts `--ro-bind-fd`, in the same order as
  before.

The box's own skeleton keeps its names: `/proc`, `/dev`, the folders, the
toolchain set, the generated identity files, the private home and tmp, the
bundle root, the Git `hooks` tmpfs, and the `/dev/null` masks and hides.
The store parity test still holds the box's mount order to the store's
description, step for step.

**L1, retiring `box_argv`.** `hands::box_argv` now carries
`#[deprecated(since = "0.12.0", note = "its argv names descriptors closed as it
returns; launch the box through `execute` or `run_boxed`, which hold them;
removed in 0.13.0")]`, and its documentation says the same. It has no
production caller. Its test callers moved. Inside the crate they build
through `Workspace::boxed`, which holds the handles, and one launches through
`Boxed::launch`. Outside it, the runtime's
`the_sealed_reach_names_each_hands_root_where_the_hands_box_binds_it` and the
seatbelt probe's
`the_check_verdict_does_not_depend_on_hands_spec_home_or_git_facts` call
`execute_in` with `/bin/echo` standing in for bubblewrap and read the argv
from its stdout. `quality/ratchet.sh api` reported "public API holds", and
`git status` shows `quality/public-api/` unchanged, so the snapshot does not
record the attribute and no line moved.

## Files

| File | Lines (main → now) | What |
| --- | ---: | --- |
| `crates/brokkr-protocol/src/hands.rs` | 1094 → 1010 | `box_argv` is a deprecated wrapper over `Workspace::boxed`. `execute_in` and `run_boxed_in` launch through `Boxed`. Size-neutral rule held by shrinking. |
| `crates/brokkr-protocol/src/hands/namespace.rs` | 793 → 783 | Each recorded source keeps its target and `Layer`. `overlay` records its lower layer. `backed` moved out. |
| `crates/brokkr-protocol/src/hands/namespace/held.rs` (new, run 1) | 157 | One descriptor rewrite for the server's `backed` and the workspace's `held`. `Mount::descriptor`. |
| `crates/brokkr-protocol/src/hands/namespace/workspace.rs` (new, run 1) | 231 | `Workspace::boxed`, `Boxed::launch`. This run opens the inner sources and mounts Git's config `RoBind` from its handle. |
| `crates/brokkr-protocol/src/hands/namespace/entry.rs` (existing child) | 508 → 508 | `DESCRIPTOR_MOUNT` reads `Mount::RoBind.descriptor()`. |
| `crates/brokkr-protocol/src/hands/sealed.rs` | 291 → 339 | `Drifted::Unopened` (run 1) and `Drifted::Unheld`. `Sealed::retained`, which describes Git's config and an inside common directory. |
| `crates/brokkr-protocol/src/hands/sealed/opened.rs` (new, run 1) | 158 | `Sealed::opened`. This run adds `Beneath`, `Expected` and `beneath`, with `openat2` beneath the parent's handle on Linux and a read-only `openat` elsewhere, where nothing is mounted. |
| `crates/brokkr-protocol/src/hands/sealed/hidden.rs` | 498 → 547 | The observation records each pin's and the artifact root's identity. `Hidden::beneath` describes them as sources beneath their sealed parent. |

No production file outside the row's four files and their module trees
moved. `quality/file-lines.txt` carries the measured rows of every file in
this table and of the test files below. `quality/too-many-lines.txt` carries
`the_namespace_is_built_…` at 142 lines, down from main's 145. Three pairs of
`let`s in that test were folded to absorb the moved `box_argv` call.
`quality/ratchet.sh files`, `clones` and `baselines 49e20e92c` each held ("no
baseline raised since 49e20e92c").

## The proofs

The owning suite is `crates/brokkr-protocol/src/hands/tests.rs` and its
children, `hands/tests/held.rs` (420 lines) and `hands/tests/store.rs` (1267)
among them. No test moved between files.

The real-namespace proofs ran with `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1`
declared. The seat's shell refuses an environment prefix, so the variable was
given to the test binaries as `cargo --config
'env.BROKKR_REQUIRE_BOUNDARY_EVIDENCE="1"' test …`. To check that it reached
them, a throwaway test asserting `boundary_evidence_required()` was run once
with that flag (passed) and once without it (failed), then deleted. With it,
the whole `hands::` suite passed 208 under `--nocapture`. Its only `skipped:`
lines and panics came from the skip helper's own two self-tests, so no
boundary proof skipped.

| Test | What it pins |
| --- | --- |
| `held::every_sealed_source_is_mounted_from_its_handle_and_none_by_its_path` | A linked worktree with its common directory outside, plus `ro`, `rw`, absent and overlay binds. The box holds exactly `[worktree, common, tools, pad, layer, common/config]`. The argv carries `--ro-bind-fd <n> <layer> --overlay-src /proc/self/fd/<n>`. No `--bind`, `--bind-try`, `--ro-bind`, `--ro-bind-try` or `--overlay-src` names a sealed source or Git's config by path. Read with the descriptors named, the argv has `--bind-fd` for the worktree, the common directory and `pad`, and `--ro-bind-fd` for `tools` and `common/config`. |
| `held::a_source_replaced_after_the_last_observation_is_not_the_object_mounted` | Real namespaces. Once the box holds its handles, a primary repository's worktree and the `ro`, `rw` and overlay sources are each renamed to `<name>.old` with a decoy in their place. The call's stdout is exactly `base\nobserved\nlayered\nread-only\n0\n1\n2\n3\n`, so it read the observed files and its write to the retained `.git/config` failed. Its `pad` write landed in `pad.old`, and its overlay write reached neither `layer/new` nor `layer.old/new` (L2). `work.old/.git/config` has no `planted`. The command held only its standard streams and `ls`'s own descriptor. An exec whose decoys replace the worktree and the lower layer returns `Ok(0)` from a check that it read `base` and `layered`. |
| `held::the_retained_common_directory_keeps_its_config_read_only_when_its_path_goes` | H1, real namespace. A linked worktree whose `main` is renamed away once the box holds its handles. The call prints exactly `read-only\ncommitted\n`: its append to `<common>/config` failed, then it committed. `main.old/.git/config` has no `planted`. `main.old/.git/hooks/post-commit` does not exist, because the touch landed on the tmpfs. `git log -1 --format=%s <HEAD of main.old/.git/worktrees/linked>` reads `boxed\n`, so the commit is in the retained repository. |
| `held::git_s_config_the_box_cannot_hold_on_the_retained_directory_refuses` | H1's refusals (Linux). The box builds (`Ok(())`). Config renamed away gives `Err(Unopened(<config>, NotFound))`. A link in its place gives `Err(Unheld(<config>))`, whose words are pinned once: "MCP server box filesystem identity is not protected: '<config>', which the box holds read-only, is not one regular file". The worktree's `.git` replaced once the box holds its handles gives `Err(Moved(<work>/.git))`. |
| `store::the_store_s_pins_and_the_artifact_root_mount_the_objects_observed_or_refuse` | L3: a store in `work/s/keys`, the artifact root at `work/s/run/artifacts`, and the pins `s`, `s/keys` and `s/run`. Each of the four, replaced inside the worktree once the box holds its handles, makes the build return `Err(Moved(<that path>))`. A real workspace call returns `Err("hands namespace: <that cause>")`, and `ran` is never made. With the whole worktree replaced by a decoy, a real call prints exactly `observed\nobserved\nobserved\nread-only\n`, exits 1 and is denied the store in ruling 10's words. `work.old`'s evidence still reads `observed`, the decoy's reads `decoy`, and the host store is unchanged. A real exec under the same decoy returns `Ok(0)` from a check that it reads the three observed files and cannot read the store. |
| `held::a_kernel_without_close_range_s_close_on_exec_form_fails_the_launch` | L4, below. |
| Existing pins moved to the new shape | Run 1's moves stand: the `the_namespace_is_built_…` and `the_workspace_and_exec_argv_…` argv pins, three `tests::sealed` pins, the `tests::evidence` pins and four `tests::store` pins. This run additionally moves config to `--ro-bind-fd`, pins to `--bind-fd` and the artifact root to `--ro-bind-fd` in those pins and in the store parity test's plan. `evidence::a_protection_mount_that_cannot_be_made_refuses_the_command` now launches through `Boxed::launch`. Since the box holds the root it observed, the test puts a file in the root's place between build and launch: bubblewrap cannot mount the directory over it, the status is 1 and the command never runs. |

What held as before, in real namespaces on this tree: the U8h store suite
(`hands::tests::store`, 26 passed, being U8h's 24, U8j's
`a_workspace_box_makes_no_nested_user_namespace_and_still_writes_commits_and_hides`
and the new proof), `git_works_in_the_box_and_cannot_plant_a_hook`,
`a_plain_repositorys_seat_commits_in_the_box`,
`an_exec_box_writes_its_overlays_to_ram` and
`every_mount_the_box_makes_is_the_store_s_description_in_order`. All are
within the 208 above.

## Removals, each compiled and restored

Each removal compiled and was run against the named tests with the
declaration given. Each failed as shown and was restored, and a `git diff`
of the touched file showed the restore. Every test then passed again in the
suite runs above.

| Removal | Failed, with the assertion's own words |
| --- | --- |
| M1, H1 in run 1's shape: Git's config dropped from `Sealed::retained` (`.skip(1)`) and mounted `RoBindTry` by path | `the_retained_common_directory_…`: left `"committed\n"`, right `"read-only\ncommitted\n"`, so the config was writable once its path went. `a_source_replaced_…`: left `"base\nobserved\nlayered\n0\n1\n2\n3\n"`, without `read-only`. `every_sealed_source_…`: the held list lacked `…/main/.git/config`. `git_s_config_…`: left `Ok(())`, right `Err(Unopened(…/work/.git/config, NotFound))`. |
| M2: no inside-worktree common directory check (`.filter(\|_\| false)`) | `git_s_config_…`: left `Ok(())`, right `Err(Moved(…/work/.git))`. |
| M3: `Expected::File` accepts anything (`file \|\| true`) | `git_s_config_…`: left `Ok(())`, right `Err(Unheld(…/work/.git/config))`. |
| M4: `Expected::Seen` accepts anything (`seen == Some(now) \|\| true`) | `store::the_store_s_pins_…`: left `Err(Unopened(…/work/s/keys, NotFound))`, right `Err(Moved(…/work/s))`, meaning the decoy `s` was taken. `git_s_config_…`: left `Ok(())`, right `Err(Moved(…/work/.git))`. |
| M5: pins and artifact root by path again (`Hidden::beneath` yields none, `.take(0)`) | Seven tests failed: `store::the_store_s_pins_…` (left `Ok(())`, right `Err(Moved(…/work/s))`), the parity test, `a_store_in_the_worktree_…` and `a_call_cannot_move_…` (their `--bind-fd` pin positions), the exact argv pin, `evidence::a_root_the_worktree_holds_…` (`--bind-fd` pin not found), and `evidence::a_protection_mount_…` (left `Some(0)`, right `Some(1)`: by path the root's file was mounted and the command ran). With that test's refusal loop briefly emptied as a scratch check (restored), its real-namespace half read left `("decoy\ndecoy\ndecoy\nread-only\n", 1)` against `("observed\nobserved\nobserved\nread-only\n", 1)`. By path, the box shows the decoys. |
| M6: the descriptor rewrite removed (`Workspace::boxed` drops the handles; everything by path) | `every_sealed_source_…`: left `[]`. `a_source_replaced_…` and `the_retained_common_directory_…`: stdout `""`, because by path the required config of the moved repository is missing and the box does not start. `an_overlay_over_a_source_sealed_absent_refuses`: built. `store::the_store_s_pins_…`: left `("decoy\ndecoy\ndecoy\nread-only\n", 1)`. |
| M7, L2: the overlay's lower layer by path again (`Namespace::overlay` records no source) | `a_source_replaced_…`: left `"base\nobserved\ndecoy\nread-only\n0\n1\n2\n3\n"`, so by path the overlay read the decoy. `every_sealed_source_…`: the held list lacked `…/layer`. `an_overlay_over_a_source_sealed_absent_refuses`: built. |
| M8, L4: the hand-off ignores a failed `close_range` (`(_, true) => Ok(())` in `entry::inherit`) | `a_kernel_without_…`: left `Ok(0)`, right `Err("hands could not spawn bwrap: Invalid argument (os error 22)")`. The launch went ahead without the close-on-exec sweep. |

## The kernel prerequisite (L4)

The descriptor hand-off (`entry::inherit`, which `Boxed::launch` installs
between fork and exec) marks every descriptor above the standard streams
close-on-exec with one `close_range(3, ~0U, CLOSE_RANGE_CLOEXEC)`, then
clears the flag on the handles the argv mounts. That form of `close_range`
needs Linux 5.11, and `openat2`, which opens the inner sources, needs Linux
5.6. So the workspace box needs Linux 5.11 beside bubblewrap 0.11 (README's
prerequisite). The exact diagnostic was established through the real
launch. `a_kernel_without_close_range_s_close_on_exec_form_fails_the_launch`
installs, on the one thread that launches, a seccomp filter. The filter makes
`close_range` fail with EINVAL, the errno close_range(2) documents for an
unknown flag on 5.9 and 5.10, and then with ENOSYS, its errno before 5.9.
`execute_in` then returns exactly `Err("hands could not spawn bwrap: Invalid
argument (os error 22)")` and `Err("hands could not spawn bwrap: Function not
implemented (os error 38)")`. No bubblewrap ran. The stand-in path
`/bin/true` would have exited 0, as M8 showed. This is a filter standing in
for such a kernel. No pre-5.11 kernel was run. README and the quickstart are
not in this row: the public prerequisite text follows in U9b2 (docs), with
this diagnostic.

## Gates run in this seat

Format and the workspace's clippy were clean on the final tree. Tests ran
crate by crate under the seat's time limit, with `--all-features --locked`.
`brokkr-protocol`, with the declaration, passed 1096 in its lib (3 ignored),
6 in `hands_exits`, 1 in `secret_drop` and 1 doc test. `brokkr-runtime`
passed 1428 in its lib and 143 in `it` without the declaration, where
`witness_digests::` passed 6, so no pin moved. Its `capability_launch`
passed 76, `operated_repo` 1 and `queued_launch` 8.
`brokkr-seatbelt-probe`, with the declaration, passed 99. `brokkr-cli`
passed 1087 in its lib, 813 in `it`, 27 in `driver_conformance` and 14 in
`transcript_surfaces`, and each `heap_*` binary passed. Every other crate,
run together, printed 16 result lines with no failure.

Four runtime tests fail only under the declaration, each because a skip
becomes a failure on this host. Two survey tests need the `brokkr` binary a
workspace build makes; after `cargo build -p brokkr-cli` both passed (5 of
`survey_tests`). `admission::tests::coordination::a_bind_alias_of_protected_evidence_is_refused`
fails because util-linux `unshare --user --map-root-user` is refused here
("write failed /proc/self/uid_map: Operation not permitted"). Without the
declaration it logs its skip and passes. `it`'s
`capability_broker_launch::an_engine_holds_its_lease_in_the_root_its_home_resolves`
finds "no protected location for a HOME (TMPDIR; D6)". None of the four
builds a workspace box.

`cargo run --locked -p brokkr-cli -- compile --bundle recipes/self` ended
"0 unruled". On the final tree `openspec validate --all --strict` passed 36
of 36, `typos --hidden` reported nothing, and `git diff --check` found no
whitespace error in the staged or unstaged tree.

## Carried and follow-ups

The production doc comments in `hands/evidence.rs` (lines 12 and 156) and
`hands/environment.rs` (line 3), and `brokkr-runtime/src/engine.rs` (line
932), still name `box_argv`. These files are outside this row, so they are
left for the 0.13.0 removal. Two test names (`box_argv_narrows_…` and
`box_argv_makes_…`) keep their #570 names. The bundle root is mounted
`--ro-bind` by path. It is no sealed source and not among this run's
findings. The too-many-lines row for `git_works_in_the_box_and_cannot_plant_a_hook`
reads 117 on main, while this toolchain measures that unchanged function at
120. This run left that row as main has it.

## Pending

The exact coverage gate (`scripts/coverage-exact.sh`), which runs outside
the box. macOS, where `below` takes the read-only `openat` branch and nothing
is carried, and which this host cannot build. Remote CI on the final head. A
real pre-5.11 kernel.
