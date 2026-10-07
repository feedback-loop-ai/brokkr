# U6c5a: the source observer and its refusals (observer and box-side parts of tasks 28.9–28.10)

Run `0065-slice-two-unit-u6c5a-see-th-78b3bfbb` completes the work that run
`0065-slice-two-unit-u6c5a-see-th-cbe93620` saved. That earlier run built the
observer but returned oversized twice, both times on the 800-line production
ceiling. The branch is `s2/U6c5a`, on main at `1933aa9d`. Its first visit
committed `e556500b`. The review returned it with three medium and five low
findings, and the second visit answers them (see "The review's findings"
below). The council then held the second visit's head `9063329b` on four
high security findings. Run `0065-slice-two-unit-u6c5a-see-th-d1c51c2a`, the
third visit, answers them (see "The third visit: the council's security
hold" at the end). Production changes stay inside the row's four files, as
the operator ruled on 2026-10-06 and 2026-10-07:

| File | Lines |
| --- | --- |
| `crates/brokkr-protocol/src/broker.rs` | 283 → 293 |
| `crates/brokkr-protocol/src/hands/namespace.rs` | 612 → 768 |
| `crates/brokkr-protocol/src/hands/namespace/sources.rs` | new, 783 |
| `crates/brokkr-protocol/src/hands/namespace/sources/host.rs` | new, 646 |

(Counts as of the sixth visit, whose section at the end supersedes the
earlier visits' structure where they differ.)

The observer uses only the protocol crate's existing `rustix`, `libc`,
`sha2` and `hex` dependencies, and no Cargo file moved. Every contract,
fixture, the policy table, `reference/`, `extensions/`, recipes, adapters and
agents are unchanged. No MCP grant moved and the realm-wide compile fence is
untouched, so U9b remains the only enabling unit. Every admitted plan still
ends in `broker serving protections are incomplete`, before any lookup or
start.

## What the unit builds

`broker.rs` adds two closed causes to `Refusal`:

- `Linked`: "MCP server box program tree contains a multiply-linked file".
- `Unavailable`: "MCP server box is unavailable".

`Refusal` now derives `PartialOrd` and `Ord`, and the variants are declared
in MB3's precedence. The observer keeps the least cause it meets
(`Observer::fault`), which replaces a private fault enum.

The observer itself is `hands/namespace/sources.rs`, registered under
`namespace.rs` for Linux only, as `pub(super) mod sources`. It works as
follows:

- **Resolution.** Each source resolves from `/` one component at a time
  through descriptors (`resolve`). `statx` is called with
  `SYMLINK_NOFOLLOW`, and the observer reads each link itself. Every
  directory entered and every link read becomes a `Hop`, with its place,
  identity and link bytes. `..` leaves the directory actually entered. More
  than `hops` links refuses. The resolution ends on a closed `Object`, a
  directory or a regular file. A special file at the end refuses. A path on
  through a file or a FIFO names nothing.
- **The walk.** A directory source is walked with no-follow lookups, holding
  only the active ancestor stack (`Frame`) on the source's own handle. Each
  frame lists through a duplicate of its checked handle (`Dir::new`), never
  a reopened name.
- **The four bounds.** The bounds are the fixed `LIMITS`: 1,000,000
  entries, depth 64, 40 hops and 65,536 mount records, which are the
  observer's memory bound (the byte bound the earlier visits kept is gone by
  the operator's ruling of 2026-10-07; see the sixth visit). Exceeding any
  one refuses `Identity` and stops the walk. Nothing is pruned or cached.
- **The program rule.** A regular file of the launch, the program tree, the
  bootstrap or the generated identity (`Role::Generated`) with a second
  link refuses `Linked`, whoever owns it (`Observer::file`).
- **The support rule.** A support file with a second link, in the system
  set, the resolver or the launcher, passes only MB3's kernel
  write-exclusion proof (`host::excluded`). Every writer and the owner must
  be mapped. The owner must be none of the writers. Neither group nor other
  may write. No extended access ACL may be present, read through the file's
  own descriptor. The writers' privilege must be confined.
- **The mount table.** The table is read strictly (`host::records`), and
  every mount the walk stood on is proved against it (`host::mapped`). Each
  mount is keyed by its id and each device an object on it showed. It must
  be in the table once, with that device, on a local filesystem, and
  read-only where nested inside a source. No alias of a walked object may
  lie in reach, by device, mount root and subpath (`host::aliases`).
- **The second resolution.** After the walk, every source resolves again.
  It must take the same route and end on the object its handle holds
  (`Observer::again`).
- **The digest.** The source-set digest covers places, facts, link targets
  and the used mount records. It covers no descriptor number and no mount
  id. The generated identity is written afresh under a new random scratch
  name for every box, so its route, place, device, inode and times are
  checked but never digested. The digest takes each generated file's name,
  mode, owner, group, link count and size (`Facts::made`), which every
  preparation repeats. An unchanged second preparation is therefore the
  same digest.

The observer's host facts live in `sources/host.rs`, the fourth file the
operator ruled. They are the descriptor resolution (`resolve` and its
`Walk`, moved there by the third visit), the strict mountinfo parser and
aliases, the writer credentials, the capability sets and the ACL read, the
write-exclusion predicate, the launcher, and `Host`, which bundles them
through closure seams (ruling 2). Credentials split observation from
decision (ruling 10): `credentials` reads `/proc/sys/fs/overflowuid` and
`/proc/self/status`, and `credited` decides. Since the third visit the
writer's uids are all four of its status `Uid:` line, and its privilege is
confined only where no uid is root, its permitted, effective and ambient
capability sets are all empty (since the fourth visit; the third admitted
any capability outside an ownership mask), and `NoNewPrivs` reads `1`. An
unread fact leaves no writer known or its privilege unconfined.

`ServerBox::prepare` consumes the observer through `sources::served`, which
the second visit moved from `namespace.rs` (where it was `observed`) into
`sources.rs`, to make room in `namespace.rs` for the generated identity's
directory. Its order is as follows:

1. Reach is cleared, as at U6c4.
2. The identity is set up (`namespace::identity`): a bootstrap inside a
   private directory, or a scratch that cannot hold the identity, is
   `Identity`. Since the third visit that cause is held, not returned.
3. Where the identity was written, its files join the namespace's binds.
4. The observer runs whether or not step 2 failed, and the lesser of the two
   causes wins, so a linked program or bootstrap file is `Linked` even in a
   private directory (L1). It sees the launch name as `Launch`, the package root
   as `Program`, and every bind of the namespace: the system set and the
   resolver as optional `Support`, the identity generated in the scratch's
   `etc` as required `Generated`, and everything else as required
   `Program`. It also sees the launcher as `Support`.
5. The launch's resolution must still end on the canonical executable, or
   the box refuses `Identity`.
6. A launcher that reports no bubblewrap of at least 0.5.0
   (`DESCRIPTOR_MOUNTS`) refuses `Unavailable`. Off Linux, `prepare` refuses
   `Unavailable` once reach clears.

`Namespace::backed` then rewrites every host bind to
`--ro-bind-fd <fd> <target>` from the handle observed for it. An optional
source with no handle is dropped. A source mounted twice gets a duplicate
descriptor, because bubblewrap closes each one it mounts. The box retains
every owned descriptor (`ServerBox::_handles`) and the observed `Sources`
(`ServerBox::_sources`). Only tests read the sources, through the test-only
`ServerBox::sources()`, until U6c5b's admission consumes them.

The two private directories, `/runtime/home` and `/tmp`, are made with
`--perms 0700` (#570). They start empty, so a package root inside either
refuses `ProgramTree` (F1). `/tmp/docs` was a bound root at U6c4, and that
case moved to the refused list.

### Carried findings F1, F6 and F7

**F1 (verified).** These proofs hold:

- `a_package_root_that_widens_or_shadows_the_box_refuses` refuses
  `/tmp/docs` and `/runtime/home/docs` with `ProgramTree`. Removal M25
  fails it at `server.rs:627`.
- `program_and_bootstrap_links_refuse_and_the_launch_must_still_name_the_executable`
  refuses a singly linked bootstrap under a real `/tmp` directory with
  `Identity`. Removal M24 fails it at `sources.rs:972`.
- The real box shows both private directories empty and `700:700`. Removal
  M26 makes `a_server_box_stands_without_the_seats_or_the_hosts_private_paths`
  exit 20 at `server.rs:795`.

**F6 (verified).** Every hop of a source's resolution is kept and compared
on the second resolution. `replacing_a_hop_or_the_source_between_resolutions_refuses`
binds the route comparison (M19) and the held-object comparison (M20)
independently.

**F7 (closed).** The stale `crates/brokkr-cli/tests/hands.rs` row of
`quality/too-many-lines.txt` was re-measured, from `:216` to `:287` at the
same 117 lines.

## The first visit's repairs

### CLI fixtures

`crates/brokkr-cli/tests/capability_broker.rs` (1,183 → 1,265 lines) is
registered under the CLI's `tests/it.rs`, and only test code changed:

- **The fixture root.** Outside a box, `Root` builds under
  `tempfile::tempdir_in("/var/tmp")`, canonicalized. Under `/tmp` every
  package root met F1's refusal. A seat's hands box binds no `/var/tmp`, so
  the review saw every refusal test panic before its assertion. Inside a
  box (`BROKKR_HANDS_BOX` set), `Root` builds under `CARGO_TARGET_TMPDIR`
  instead. The box binds that directory, and it is neither of a server
  box's private directories, so `LaunchInReach` is still reached rather than
  `ProgramTree`.
- **The binary.** `brokkr()` runs a singly linked copy of the binary.
  Cargo's `target/debug/brokkr` is a second link to its `deps/brokkr-<hash>`
  (`ls -l` showed link count 2). Each test process writes the copy fresh, as
  `brokkr.<pid>` under `CARGO_TARGET_TMPDIR/capability-broker`, and renames
  it into place as `brokkr`. The sealed bootstrap is that same canonical
  path.
- **Off Linux.** `past_prepare(cause)` reads `cause` on Linux and
  `Unavailable` elsewhere, for every expectation that the observer's
  preparation decides. `admitted()` is
  `past_prepare(ServingIncomplete)`. Refusals decided before preparation
  keep their exact cause on both systems.
- **In a seat's box.** Ten admitted-path tests return early when
  `BROKKR_HANDS_BOX` is set. The one-line comment in `boxed()` records the
  reason: an unprivileged bubblewrap user namespace maps root-owned host
  files to the overflow uid, so MB3 rightly refuses their identity.

The ten guarded tests are:

- `a_sealed_plan_is_admitted_and_still_refused_before_any_lookup_or_start`
- `only_the_protected_inventory_binds_a_plan`
- `only_the_engines_home_roots_a_plan`
- `a_plan_under_an_unprotected_root_or_file_is_unbound`
- `a_plan_answers_for_the_attempt_it_lies_in_and_its_size`
- `every_record_is_an_object_never_a_positional_array`
- `a_sealed_plan_of_the_wrong_shape_is_unbound`
- `the_program_tree_is_mb3s_layout_of_the_executable`
- `unprovable_writers_unbounded_sources_and_exposed_control_roots_refuse`
- `the_store_is_neither_in_reach_nor_in_the_box`

The other eleven are refusal-path tests and keep running in a box.

This seat's sandbox refuses an environment-prefixed command, so no run with
`BROKKR_HANDS_BOX` set was possible. The box branch was instead exercised on
this host by making `boxed()` return true for one run, then restoring it.
That run gave 18 passed and 3 failed. The three failures were
`binding_names_are_checked_then_fixed_keys_refuse_before_lookup` (`:1034`),
`a_long_binding_name_is_cut_to_one_bounded_line` (`:1079`) and
`the_box_neither_launches_from_nor_binds_over_the_seats_reach` (`:1178`).
Each read `broker plan is not bound to this attempt`, because a plan binds
only under an ancestry no one else may write, and this worktree was made
group-writable: `stat` shows the worktree, `target` and `target/tmp` all at
`775`. A worktree made under umask 022 has none of those failures. That is
the same umask dependency as #570's hands-box half, and no fixture location
a box offers avoids it. The in-box run itself stays pending.

`each_refusal_reads_in_mb3_and_mb4s_words` now pins both new texts, and it
asserts that its table, written in MB3's order, is sorted by `Refusal`'s
`Ord`.

On this host, outside any box, `cargo test -p brokkr-cli --test it
capability_broker::` ran 21 passed before and after the mutations. Before
the repair it ran 10 passed and 11 failed. The 11 failures were 10 taking
`ProgramTree` from `/tmp` and 1 taking `Linked` from cargo's linked
bootstrap. A run with `BROKKR_HANDS_BOX` set was not made here, because
this seat's sandbox refuses an environment-prefixed command. The in-box
21/21 acceptance is withdrawn, as the controller ruled. No in-box result is
claimed.

### The same guard in the protocol suites

The protocol tests that prepare a box against the live host take the same
guard, `server::boxed()`, for the same reason. These are:

- `the_server_box_holds_the_projected_system_set_and_its_own_private_paths`
- `the_fixed_environment_is_mb4s_eight_entries_and_its_names_are_the_reserved_set`
  (its argv half)
- the admitted row of `the_server_box_stays_clear_of_every_reach_root_either_way`
- `a_launcher_that_cannot_mount_a_descriptor_leaves_the_box_unavailable`
- `program_and_bootstrap_links_refuse_and_the_launch_must_still_name_the_executable`
- `the_box_mounts_the_observed_object_and_carries_its_facts`
- `a_server_entry_root_owned_and_singly_linked_passes_with_the_systems_links_intact`

A boxed verify seat would otherwise fail them with `Identity`.

### Coverage-driven restructuring inside the row

A diagnostic run of `cargo +nightly-2026-09-05 llvm-cov --branch -p
brokkr-protocol --lib` over the saved observer reported misses:

| File | Lines missed | Branches missed |
| --- | --- | --- |
| `sources.rs` | 16 of 399 | 12 of 68 |
| `host.rs` | 3 of 188 | 0 |
| `namespace.rs` (backed's no-handle path) | 1 | 1 |

The exact gate is literal 100% of lines, branches and functions. Each miss
was either driven by a new test or removed by making the impossible state
unrepresentable. No guard was weakened:

- **The mount id.** `Facts::of` returns `None` when the kernel names no
  mount, and `stat_at` turns that into an unreadable object
  (`Errno::NOTSUP`). That error is refused wherever any `statx` failure is.
  `Facts::mount` is now a plain `u64`, so `tree` and `stood` have no
  untestable `None` arm.
- **What a resolution ends on.** `resolve` returns a closed `Object`. It
  follows every link and refuses a special file, so `tree` matches only
  `Dir` and `File`, with no unreachable refusal arm.
- **One mount, several devices.** `seen` is keyed by mount id and device
  (`host::Seen`). A mount showing a second device, as a btrfs subvolume
  does, now fails `mapped`'s existing record-device check, which binds it.
  It is no longer a separate branch that this host cannot reach.
- **Unreadable entries.** `entry` returns `Result`. An entry whose `statx`
  or `readlink` fails, and a listing that fails, fault `Identity` and end
  that directory's walk. `target` takes the link bytes, not an `Option`.
- **Listing.** `Frame::open` lists through `Dir::new` on a duplicate of the
  checked handle. `Dir::read_from` reopened `.`, which needs search
  permission, so an entry-level failure was reachable only by a race.
- **A redundant early return.** `source`'s `if self.over { return None }`
  is removed. An observation already over refuses at its first count.
- **The credentials decision.** `credentials` reads, and `credited`
  decides, so root, an ownership capability and unread facts are each
  tested.
- **Visibility.** `Namespace::backed` and `host::acl` became
  `pub(in crate::hands)` or `pub(super)`, which the hands tests need. No
  public item was added by this.

After the restructuring, the same diagnostic reads 100% on all three
observer files:

| File | Lines | Branches | Functions |
| --- | --- | --- | --- |
| `sources.rs` | 393 of 393 | 58 of 58 | 51 of 51 |
| `host.rs` | 195 of 195 | 8 of 8 | 41 of 41 |
| `namespace.rs` | 477 of 480 | 16 of 16 | 81 of 82 |

The `namespace.rs` misses are `ServerBox::paths` (lines 681–683), main's
U6c4 code. It is consumed by the CLI session, which the protocol-only
diagnostic does not run. U6c4's evidence reports the same miss.

## Readings and knowingly bent principles

- **The public API (ruling 6; for the operator's ruling).** The second
  visit made `ServerBox::sources()` test-only (`pub(super)`, under
  `cfg(all(test, target_os = "linux"))`) and renamed the field to
  `_sources`, following `_handles` and `handles()`. The public API now
  rises by two items, `Refusal::Linked` and `Refusal::Unavailable`, and both
  have consumers: `Linked` in the observer, and `Unavailable` in `prepare`
  off Linux and in the launcher check. `quality/ratchet.sh api` reported
  "public API holds" against the updated snapshot. `quality/ratchet.sh
  baselines origin/main` refused with `public-api/brokkr-protocol.txt: 911
  public items (was 909)`, so the pull request needs a `Ruling:` line.
- **One unreadable entry ends its directory's walk.** It still refuses
  `Identity`. A later, higher-precedence cause in that same directory, such
  as a linked program file, is then not met, so the refusal reads
  `Identity`. Precedence is among the causes met, as it already was for an
  exceeded bound.
- **Off Linux.** The observer is Linux only, and macOS's `prepare` refuses
  `Unavailable` after reach (decision 0063).
- **Test-only readers.** `ServerBox::handles()` and `argv()`, and the
  server tests' argv and handle helpers, are now
  `cfg(all(test, target_os = "linux"))`, since only Linux tests read them.
  A macOS test build therefore carries no dead test code.

## Tests

No test moved between files. The new child module
`crates/brokkr-protocol/src/hands/tests/sources.rs` (1,153 lines) is
registered as `#[cfg(target_os = "linux")] mod sources;` in
`hands/tests.rs` (1,632 → 1,634 lines). `hands/tests/server.rs` grew from
625 to 796 lines, and its `Host` fixture moved from the temporary directory
into the build directory, because of F1. Fixtures build on canonicalized
temporary roots and read nothing under `.forge/`.

The table below covers the sources suite (all new):

| Test | What it pins |
| --- | --- |
| `the_write_exclusion_proof_needs_every_condition` | The pure predicate. Each of these fails it alone: owner equal to a writer, group write, other write, an ACL present, an ACL unknown, an overflow owner, an overflow writer, no writer, and unproved privilege. A pipe's descriptor reads its ACL as `Unknown`. |
| `the_writers_capabilities_are_each_set_read_once` | `CapPrm`, `CapEff` and `CapAmb` are each read exactly once. `credited` is exact for a confined writer. Root, an ownership capability and an unread status each leave privilege unproved. An unread or unterminated overflow uid leaves no writer. |
| `the_mount_table_is_read_strictly_and_within_its_bound` | Exact records, escapes, the nsfs pseudo root, the record bound at three, and ten malformed tables. |
| `an_alias_is_every_other_place_an_object_is_mounted` | Aliases by mount root and subpath, either way, of the same device only. |
| `every_mount_under_a_source_is_proved_against_the_table` | Nested writable, another device, absent from the table, unproved filesystem, a second device on one mount, and an alias in reach of the nested or the root mount. |
| `a_program_or_bootstrap_file_with_a_second_link_refuses_whoever_owns_it` | A linked program file is `Linked` under credentials that pass a support file. A bootstrap, generated or launch file linked from outside its tree is `Linked`. |
| `a_linked_support_file_passes_only_with_the_whole_write_exclusion_proof` | A real hard-linked file under each credential and mode negative alone, and a real `system.posix_acl_access` grant. The grant names this process's own uid, which any user namespace it runs in maps (an unmapped uid failed `EINVAL` in the review's box). A singly linked support file passes even when a writer owns it. |
| `special_unreadable_cyclic_and_absent_sources_refuse` | FIFO and socket inside a tree, an unlistable subdirectory, cyclic links, an absent required source, and an absent optional source with no handle. |
| `a_source_that_cannot_be_resolved_or_listed_refuses` | A FIFO source even when optional, paths on through a FIFO or a file, an unsearchable directory on the way, an unlistable source directory, and an entry of a list-only directory. |
| `an_observation_with_no_room_or_no_mount_table_refuses` | No room even for the root, and an unread mount table. |
| `each_bound_refuses_one_over_and_passes_at_its_limit` | Each of the four bounds, at its exact limit (passes) and one over (refuses), separately. The entry count is exactly 3, and the mount count is the live table's. |
| `a_route_or_link_target_in_reach_refuses` | Link targets in either root, absolute and relative. A route through writable reach is `LaunchInReach` for the launch and `BindOverlapsReach` otherwise. Reach precedes links. |
| `replacing_a_hop_or_the_source_between_resolutions_refuses` | An intermediate link renamed over, the source directory replaced, and an optional source that appears. |
| `a_handle_that_opens_another_object_than_the_one_observed_refuses` (new in the second visit) | Through `Host::opening`, the source directory is replaced once, after its facts were read and before its handle opens. Every later resolution agrees with that handle, so only `handle`'s own comparison refuses it, with `Identity`. The same tree on an untouched host passes. |
| `the_root_every_resolution_starts_from_is_closed_on_exec` (new in the second visit) | `sources::root()`'s descriptor reads exactly `FdFlags::CLOEXEC`. |
| `a_mount_alias_or_an_unproved_filesystem_refuses` | The package's parent mounted again in writable reach, where every file has one link, refuses. The same alias outside reach passes. Part of the package in reach, an overlay filesystem, another device and a missing record each refuse. |
| `a_launcher_that_cannot_mount_a_descriptor_leaves_the_box_unavailable` | 0.5.0 passes. 0.4.1, a garbled version and no launcher each refuse `Unavailable`. A linked package and a linked launcher refuse first. |
| `program_and_bootstrap_links_refuse_and_the_launch_must_still_name_the_executable` | A linked package file and a linked bootstrap are `Linked`. A bootstrap under a real `/tmp` directory is `Identity`. The launch link repointed after resolution is `Identity`. |
| `the_box_mounts_the_observed_object_and_carries_its_facts` | An unchanged second preparation, its identity generated afresh, has the same entries and digest. A file rewritten in place keeps the entries and changes the digest. A directory and a file added are exactly 2 more entries and another digest. After a rename, the held handle is the observed object, and the argv names its descriptor. |
| `the_source_set_digest_binds_the_observed_facts_and_no_descriptor_number` | 64 lowercase hex. Unchanged by other open descriptors. Changed by a mode, a new link and a link's target. Two files written alike under different routes, places and inodes are one digest as `Generated` and two as `Program`. A generated file's name, mode and size each change it. |
| `a_server_entry_root_owned_and_singly_linked_passes_with_the_systems_links_intact` | `sh` and an installed `/usr/lib` package pass on the live host, its system hard links intact, under this process's credentials. |

The server suite changed as follows:

| Change | Test | What it pins |
| --- | --- | --- |
| new | `a_source_with_no_observed_handle_is_not_mounted` | `backed` with no handle drops every host bind and keeps both private directories. |
| changed | `the_server_box_holds_the_projected_system_set_and_its_own_private_paths` | The exact argv with `--ro-bind-fd` from the box's own handles, `--perms 0700` on both private directories, duplicates for a twice-mounted source, every handle on the object its path names now, and no descriptor named twice. |
| changed | `a_server_box_stands_without_the_seats_or_the_hosts_private_paths` | The real box, with descriptors inherited through `pre_exec`, sees both private directories as `700:700`. |
| changed | `a_package_root_that_widens_or_shadows_the_box_refuses` | `/tmp/docs` and `/runtime/home/docs` refuse. |
| changed | `the_server_box_stays_clear_of_every_reach_root_either_way` | The admitted row reads `ADMITTED`. |

## Removals

The first visit ran M1–M39 against the production code it committed at
`e556500b`. Each removal compiled. It was applied with `git apply`, run with
the named filter under `cargo test -p brokkr-protocol --lib` (or
`cargo test -p brokkr-cli --test it`), and restored with
`git checkout -- crates` before the next. The working tree was checked
clean after each batch. M24 and M27 also ran earlier. One early table
failure proves no later case. The line numbers in this first table are
those of `e556500b`'s tree. M11 and M12, the removals of the metadata bound,
are withdrawn with that bound (sixth visit). The second visit did not re-run
M1–M39. Its own
test edits are listed under "The review's findings", and they move later
`tests/sources.rs` lines.

| Removal | First failing assertion | Observed |
| --- | --- | --- |
| M1 predicate: `let acl = true;` in `excluded` | `sources.rs:119`, the ACL-present case | 1 failed |
| M2 capability set read twice: `Some(held \| set)` | `sources.rs:162`, `left: Some(2097153)`, right `None` | 1 failed |
| M3 record bound: `(limit > 0)` for `records.len() <= limit` | `sources.rs:217` (three records at two) and `:650` (mounts one over) | 2 failed |
| M4 aliases inside: the `Err(_)` arm returns `None` | `sources.rs:270`, `left: ["/work/opt/docs"]` | 1 failed |
| M5 nested read-only: `!record.writable &` dropped | `sources.rs:297`, the writable nested mount | 1 failed |
| M6 program link: the `Launch \| Program` arm does nothing | `sources.rs:335`, `left: Ok(())`, right `Err(Linked)` | 1 failed |
| M7 live ACL: the descriptor's ACL read as `Absent` | `sources.rs:425`, the granted ACL admitted | 1 failed |
| M8 special entry: `Kind::Special` does nothing in `entry` | `sources.rs:461`, `("fifo", Ok(()))` | 1 failed |
| M9 entries bound removed (`> u64::MAX - 1`) | `sources.rs:612`, two entries admitted | 1 failed |
| M10 entries off by one (`>=`) | `sources.rs:602`, the exact-limit positive refused | 1 failed |
| M13 depth bound removed | `sources.rs:665` | 1 failed |
| M14 depth off by one | `sources.rs:664`, the exact-limit positive refused | 1 failed |
| M15 hop bound removed (`links == usize::MAX`) | `sources.rs:675` | 1 failed |
| M16 hop bound off by one (`links + 1 >= hops`) | `sources.rs:674`, the exact-limit positive refused | 1 failed |
| M17 mount records off by one (`<`) | `sources.rs:639`, the exact-limit positive refused | 1 failed |
| M18 link target in reach: no fault | `sources.rs:706`, `work/data` admitted | 1 failed |
| M19 route comparison dropped in `again` | `sources.rs:765`, the renamed hop admitted | 1 failed |
| M20 held-object comparison dropped in `again` | `sources.rs:777`, the replaced source admitted | 1 failed |
| M21 roots unchecked: `mounts \| roots` | `sources.rs:853`, the parent alias in reach admitted | 1 failed |
| M22 launcher floor: `version >= (0, 0, 0)` | `sources.rs:920`, 0.4.1 admitted | 1 failed |
| M23 launch identity: `launch.is_some()` | `sources.rs:980`, the repointed launch admitted | 1 failed |
| M24 private bootstrap: `in_private(…) & false` | `sources.rs:972`, the `/tmp` bootstrap admitted | 1 failed |
| M25 private package root: `\| in_private(root)` dropped | `server.rs:627`, `("/tmp/docs", Ok(()))` | 1 failed |
| M26 #570: `--perms 0700` dropped | `server.rs:455` (argv), and `server.rs:795`, where the real box exits `Some(20)` | 2 failed, 8 passed |
| M27 handles unused: `backed` not called | `sources.rs:1020`, `["/tmp", "--ro-bind", …]` | 1 failed |
| M28 digest without mode | `sources.rs:1048`, digests equal | 1 failed |
| M29 support links treated as program links | `sources.rs:1072`, `("/usr/bin/dash", Err(Linked))` | 1 failed |
| M30 duplicates not made: `used.contains(..) & false` | `server.rs:455`, `/sbin` and `/usr/sbin` share descriptor 15 | 1 failed |
| M31 precedence: `Linked` declared after `Identity` | `capability_broker.rs:345`, the order assertion | 1 failed, 0 passed of the filter |
| M32 facts not carried: the box stores fixed `Sources` | `sources.rs:1000`, `left: 1`, right `3` | 1 failed |
| M33 special source end: `Ok(None)` | `sources.rs:526`, `left: Ok([false])` | 1 failed |
| M34 unreadable entry: the `Err` arm does not fault | `sources.rs:548`, the list-only tree admitted, `Ok([true])` | 1 failed |
| M35 unread mount table: no fault | `sources.rs:579` | 1 failed |
| M36 ACL read error as `Absent` | `sources.rs:153`, `left: Absent`, right `Unknown` | 1 failed |
| M37 root confined: `uid != u32::MAX` | `sources.rs:179`, `(0, …, Proved)` | 1 failed |
| M38 record device: majors only | `sources.rs:298`, another device (0,41) admitted | 1 failed |
| M39 no handle keeps the path bind | `server.rs:653`, 18 `--ro-bind-try` left | 1 failed |

At `e556500b` the descriptor-source identity check in `handle` had no
removal of its own, because a swap between its `statx` and its open was a
race. The second visit added the `Host::opening` closure seam, which makes
that swap deterministic, and M49 below is its removal.

The second visit's removals each compiled. Each was made with one edit,
run with `cargo test -p brokkr-protocol --lib -- hands::tests::sources`,
and restored with the inverse edit before the next. After the last one, a
`grep` for every mutation's text found none, and `hands::tests::sources`
with `hands::tests::server` ran 32 passed. M41–M45 were run again after the
generated-file checks were folded into the digest test (see the clone gate
below), and the table gives those runs. Line numbers are of
`tests/sources.rs` in the final tree.

| Removal | First failing assertion | Observed |
| --- | --- | --- |
| M40 `owner \| true` in `excluded` (owner equal to a writer) | `:107`, `Owned { uid: 1000 }`; live `:418`, `ours()` admitted | 3 failed (and `:973`, the owned launcher admitted) |
| M41 generated file digested by place and full facts in `tree` | `:1046`, an unchanged second preparation's digest differs; `:1118`, the two generated files differ | 2 failed |
| M42 generated route hops digested (`route`'s guard always true) | `:1046` and `:1118` | 2 failed |
| M43 `Facts::made` without the mode | `:1127`, the `0o600` generated file equals the `0o644` one | 1 failed |
| M44 `Facts::made` without the size | `:1130`, the rewritten generated file equals the original | 1 failed |
| M45 generated file's name not digested (`Path::new("")`) | `:1122`, `group` equals `passwd` | 1 failed |
| M46 generated file under the support rule in `file` | `:346`, `(Generated, Ok(()))`, right `Err(Linked)` | 1 failed |
| M47 `served` sees the generated identity as `Program` | `:1046`, an unchanged second preparation, `"290e…"` against `"ee38…"` | 1 failed |
| M48 a directory entry's facts not digested (`entry` hashes into a discarded `Sha256`) | `:1050`, the file rewritten in place keeps the digest; `:1101`, a mode change keeps it | 2 failed |
| M49 `handle`'s open-versus-observed comparison `\| true` | `:820`, `left: Ok(())`, right `Err(Identity)` | 1 failed |
| M50 `Host::opening` not called before the open | `:820`, the swap never happens, `Ok(())` | 1 failed |
| M51 `root()` without `CLOEXEC` | `:828`, `left: FdFlags(0x0)`, right `FdFlags(CLOEXEC)` | 1 failed |
| M52 `owned \| true` (an overflow owner) | `:138`, `Owned { uid: 65534 }`; live `:418`, `overflow: 1000` admitted | 2 failed (plus the mount-count churn below) |
| M53 writer mapping without the overflow check | `:143`, writers `[1000, 65534]`; live `:418`, `[4242, 65534]` admitted | 2 failed |
| M54 writer mapping without the empty check | `:148`, no writers; live `:418`, `uids: []` admitted | 2 failed |
| M55 group write ignored (`mode & 0o002`) | `:108`, mode `0o100775`; live `:423`, `(436, Ok(()))` (0o664) | 2 failed |
| M56 other write ignored (`mode & 0o020`) | `:115`, mode `0o100757`; live `:423`, `(422, Ok(()))` (0o646) | 2 failed |
| M57 `privilege \| true` | `:153`, `Confinement::Unproved`; live `:418`, `confinement: Unproved` admitted | 2 failed |

M40 and M52–M57 answer the review's finding 3. Each condition of
`host::excluded` has its own removal, group and other separately. M1 and M36
cover the ACL conditions, and M37 covers root.

During M52, `the_box_mounts_the_observed_object_and_carries_its_facts` also
failed. The cause was not the removal. The whole mount table's record count
moved from 170 to 172 between two preparations in that one test, because
the host mounted something meanwhile. `Sources::mounts` counts every record
of the table (the first visit's design, pinned by
`each_bound_refuses_one_over_and_passes_at_its_limit`). The test now
compares entries and digest, and its comment says why. The digest is not
affected, because it covers only the records the walk stood on. This is
carried for U6c5b, below.

With every removal restored, the brokkr-protocol suite ran lib 770 passed (2
ignored), `hands_exits` 6, `secret_drop` 1 and the doctest 1.

## Gates and measurements

The second visit ran every suite in the foreground on its final tree,
outside any box, each crate with `--all-features --locked`:

| Target | Result |
| --- | --- |
| brokkr-protocol | lib 770 (2 ignored), `hands_exits` 6, `secret_drop` 1, doctest 1 |
| brokkr-cli | lib 627 (1 ignored), `it` 487 (2 ignored; `capability_broker::` 21 of 21), `driver_conformance` 27, heap tests 1 each, `transcript_surfaces` 13 |
| brokkr-runtime | lib 831, `capability_launch` 75, `it` 120 (`witness_digests::` included, with no pin moved), `operated_repo` 1, `queued_launch` 3 |
| remaining crates | bridge 17, core 105 and 24, seatbelt-probe 99 (2 ignored), store 85 and its `it` targets, view 269 (3 ignored) and 6 |

On that tree, formatting held and workspace clippy with warnings denied
reported nothing. The Rust 1.88 check of every target finished.
`bundles/self` compiled, `openspec validate --all --strict` passed 20 of
20, `typos --hidden` printed nothing and `git diff --check` was clean.

This seat could run `quality/ratchet.sh`, and the second visit ran it
directly:

- **File size.** `ratchet.sh files` held after `quality/file-lines.txt`
  took the new counts. The production files are now 744, 779 and 331 lines,
  and the two test files are 1,153 and 1,265.
- **Duplication.** `ratchet.sh clones` first refused one new 6-line test
  clone, the digest test's fixture setup repeated in a new generated-file
  test. Those checks were folded into
  `the_source_set_digest_binds_the_observed_facts_and_no_descriptor_number`
  instead, and the ratchet then reported "duplication holds".
- **Public API.** `ratchet.sh api` reported "public API holds", and
  `ratchet.sh baselines origin/main` refused 909 → 911, as above.
- **Function length.** No row of `quality/too-many-lines.txt` names a file
  this visit touched, and clippy's `too_many_lines` deny is clean.
- **Suppressions.** None were added: no `#[allow]` and no `#[expect]`.
- **Budgets.** `scripts/measure-budgets.sh` ran. The crate count stayed
  329, and the three heap peaks were unchanged. It also rewrote each note's
  date and lowered some prompt-byte figures, from main's own drift: this
  unit touches no prompt. Those three files were restored unchanged rather
  than committed here. The release binary-size check is CI's.

`cargo +nightly-2026-09-05 llvm-cov --branch -p brokkr-protocol --lib`
reports the observer files on the second visit's tree as follows:

| File | Lines | Branches | Functions |
| --- | --- | --- | --- |
| `sources.rs` | 465 of 465 | 64 of 64 | 62 of 62 |
| `host.rs` | 196 of 196 | 8 of 8 | 42 of 42 |
| `namespace.rs` | 424 of 427 | 12 of 12 | 72 of 73 |

The only miss is `ServerBox::paths` (lines 685–687), main's U6c4 code,
which the CLI consumes. Several gates are pending: the exact-coverage gate,
`scripts/coverage-exact.sh`, and a macOS build (this host has no macOS
standard library). Remote CI on the final head and a run inside a seat box
are pending too.

## Bubblewrap floor and issue #570

The observer requires a bubblewrap of at least 0.5.0 (`DESCRIPTOR_MOUNTS`)
for `--ro-bind-fd` and a tmpfs's `--perms`. That is the commissioned floor.
It was not checked against bubblewrap's changelog in this seat, which has no
web access. CI installs 0.11.0 (`BWRAP_VERSION: 0.11.0` in
`.github/actions/setup-bubblewrap/action.yml`). `README.md:42` and
`docs/guides/quickstart.md:95` and `:241` require 0.11 or newer. The stated
requirement is therefore above the code's floor, and neither file was
changed. This host's bubblewrap mounted `--ro-bind-fd` and honoured
`--perms` in `a_server_box_stands_without_the_seats_or_the_hosts_private_paths`.

Issue #570 is only partly this unit's. The server box's private tmpfs
mounts carry `--perms 0700`. The pinned argv and the real box's `700:700`
bind it (M26). The hands exec box's `/tmp` is the other half and outside
this row. It is `namespace.mount(Mount::Bind, &private_tmp, "/tmp")` at
`crates/brokkr-protocol/src/hands.rs:435`, a bind of the session's
`call/tmp`, created by `create_dir_all` under the process umask at
`hands.rs:430`. It is a finding for `hands.rs`'s owner, not fixed here.

## Open risk: CI's `/usr`

The admitted CLI paths walk this host's whole system set and pass. GitHub's
`ubuntu-latest` image has not been measured. If its `/usr` holds more than
1,000,000 entries, or user-owned multiply-linked files, every admitted CLI
test there refuses `Identity`. That is task 28.17's qualification question.
No bound is weakened to meet it.

## What U6c5b still owns

Tasks 28.9 and 28.10 stay unticked. U6c5b still owns:

- comparing the sealed `Sources` and `Writers` at broker admission, keeping
  SC1's "not bound" apart from filesystem identity
- the broker's reobservation of the sources
- launcher, bootstrap, control and store identity, including the absent,
  aliased and protected empty store, without reading values
- native alias creation
- the private cancellable observer helper behind a private `BrokerCmd`
- cancellation and deadline expiry during a blocked source operation, with
  `Identity`, zero lookups and starts, and no helper or worker left alive
- task 28.17's measured observer qualification on Linux x86_64 and aarch64,
  whose absence still blocks U9b

Later readiness, secret handoff, serving and the cross-worktree reservation
lifetime (tasks 38.3–38.4) are not closed by this source snapshot.

Two facts from the second visit bear on U6c5b's comparison. First,
`Sources::mounts` counts every record of the mount table, and that count
moved 170 → 172 between two preparations in one test. Admission must not
compare that count exactly, or the record it counts must narrow, or an
unrelated host mount refuses an unchanged plan. Second, the launcher's
`--version` runs through the path `PATH` finds, with the inherited
environment (`host::launcher`). U6c5b's launch must run the checked
launcher handle with the prescribed environment.

## The review's findings

The review of `e556500b` returned eight deduplicated findings. The second
visit answered them as follows:

1. **Medium: the digest was not reproducible.** The generated identity is
   now `Role::Generated`. Its route, place, device, inode and times are
   checked but not digested, and `Facts::made` digests what every
   preparation repeats. Unchanged-preparation equality and the package
   changes are bound by M41, M42, M47 and M48. The generated file's name,
   mode and size each have their own removal (M43–M45), and its link rule
   has M46.
2. **Medium: no removal for `handle`'s descriptor identity check.**
   `Host::opening` is a closure seam run at each place just before
   `resolve` opens it. `a_handle_that_opens_another_object_than_the_one_observed_refuses`
   replaces the source there, and M49 (the comparison) and M50 (the seam)
   each fail it with the exact `Identity`.
3. **Medium: no separate removals for `excluded`'s conditions.** These are
   M40 and M52–M57, with group and other write separate.
4. **Low: box fixtures.** The CLI `Root` builds under `CARGO_TARGET_TMPDIR`
   inside a box, and the ACL grant names this process's own mapped uid.
   The forced box-branch run and its umask-dependent three failures are
   recorded above. The real in-box run is pending.
5. **Low security: the root descriptor lacked `CLOEXEC`.** `sources::root()`
   opens it with `CLOEXEC`, and
   `the_root_every_resolution_starts_from_is_closed_on_exec` binds it
   (M51).
6. **Low: `ServerBox::sources` was public without a consumer.** It is now
   test-only, and the public API rises by the two consumed variants only.
7. **Low, carried: #570's hands exec `/tmp`.** It is outside the row and
   unchanged, as ruled.
8. **Low, carried: the scratch failure's cause.** It is still `Identity`,
   and the establishment variant belongs to U6c6, as U6c4 recorded.

The review's information items were left as they stand, for these reasons:

- The launcher's `PATH` run is carried to U6c5b, above.
- The statx `MNT_ID` and btrfs questions are 28.17's.
- On the lexical `in_private` and `..`, the review itself found that the
  broker's admission refuses a parent component. The second visit did not check
  that again.
- The 0.5.0 floor is still unchecked against bubblewrap's history, because
  this seat has no web access.

One more information item was test-only and was not changed by the second
visit. Two concurrent test processes renamed their own bootstrap copy over
the same final path, so an observation in one could see the other's rename
and refuse `Identity`. The third visit fixes it (I2, below).

## Carried lows

- **#570's hands exec `/tmp` bind (`hands.rs:430–435`).** It is outside the
  row.
- **The scratch failure's cause.** It is still `Identity`, not MB3's
  establishment cause. The variant belongs to U6c6, as U6c4 recorded.
- **`Sources::mounts` moves with host mount churn.** It is carried to
  U6c5b, above.

## The third visit: the council's security hold

The council held `9063329b` on four high findings, all confirmed fail-open
paths in the observer, with one low, two information items and the public
API raise. This visit answers each inside the row's files and the owning
suites. No design changed.

**H1, every mount view is walked.** `Observer::source` deduplicated walks by
device, inode and role, so a second mount of the same directory with a mount
nested only beneath it skipped its walk, `stood`, its root's placement and
its descendants, yet its descriptor was still mounted. The key is now the
mount id beside the identity and role. Every source registers its root on
its mount (`stood`) and its place for the alias proof (`placed`) before the
deduplication, not inside `tree`. Since a mount id with a directory inode
names one place, a source that still deduplicates has the same place as the
one walked.

**H2, a nested crossing is its own fact.** `Seen` held one boolean, the OR of
"a source root lies on this mount" and "the walk crossed into it", so a mount
holding one source's root and nested beneath another's skipped the nested
read-only and whole-mount alias proofs. `host::Stood` now keeps `root` and
`nested` apart. `Observer::stood` marks a crossing where an entry's mount
differs from the mount its walk's root lies on (`Observer::view`), and
`host::mapped` proves every nested mount read-only and alias-free whatever
root lies on it.

**H3, confinement needs every privilege path closed.** `credited` declared a
writer confined from a nonzero effective uid and the `CapPrm`, `CapEff` and
`CapAmb` sets alone. It now reads all four uids of the status `Uid:` line
(each a writer uid, none root), the three capability sets, and `NoNewPrivs`,
which must read `1`, so no exec gains a setuid owner's identity or a file's
capabilities. Every line is read exactly once, and anything unread or
repeated leaves the writer unconfined. The live host stands this process's
own status for its managed writers, as before. A process without
`no_new_privs` therefore no longer proves a host's multiply-linked system
file. This seat runs without it: with the old live credentials,
`a_server_entry_root_owned_and_singly_linked_passes_with_the_systems_links_intact`
refused `("/usr/bin/dash", Err(Identity))`, and three other live
preparations refused `Identity`. The protocol suites now prepare with
`tests::sources::confined()`, this process's own uids and capabilities with
`NoNewPrivs` read as `1`, which asserts `Proved` before use. The CLI suite
runs the binary under `setpriv --no-new-privs` on Linux (`confined()` in
`capability_broker.rs`), since setting the flag in the test process would
bind every later test it runs. Binding the complete managed-writer set to
the plan stays with U6c5b. How a live broker, and every managed writer it
stands for, comes to hold `no_new_privs` is a follow-up for the operator to
place. Until then a live host with a multiply-linked system support file
refuses `Identity`, as MB3 requires of an unproved confinement.

**H4, only a second absence proves absence.** `Observer::again` treated an
absent optional source whose second resolution failed as unchanged. Now only
`Ok(None)` again is unchanged, and a failure refuses `Identity`.

**L1, MB3's order with identity setup.** `ServerBox::prepared` returned
`Identity` for a private bootstrap or a failed scratch before the observer
could find a linked program or bootstrap file. Identity setup is now
`namespace::identity`, its cause is held, the observer runs anyway (with no
generated binds where none were made, `served`'s `made` being an `Option`),
and the lesser cause wins.

**I1, cyclomatic complexity.** `resolve` (CC 20) is split into the `Walk`
state with `step` and `follow`, and `lookup`, `linked` and `ending`. `record`
(CC 16) is split into `device` and `writable`. Splitting pushed
`sources.rs` to 856 lines, so the resolution moved into `sources/host.rs`
with its `Hop`, `Object`, `Resolved`, `root` and `parts`. cargo-crap 0.5.0,
run over `crates/brokkr-protocol/src/hands` with an LCOV naming the three
files, reports no function of theirs above CC 11: `ServerBox::prepared` and
`Observer::entry` at 11, `record` at 9 and `resolve` at 8. The ratchet's
own `crap` verdict needs the exact gate's LCOV and is pending.

**I2, the CLI bootstrap copy.** Each test process now copies the binary once
into its own owner-only directory under `/var/tmp` (in a seat box, under
`CARGO_TARGET_TMPDIR`), named `brokkr-capability-broker-<pid>-…`, so its
final path is its own and no other process can rename over it. On the host,
the first copy removes any such directory whose process has ended
(`test_kill_process` answers `ESRCH`). A box has its own process ids, so it
does not sweep. The broker admits a plan only where its own executable is
the sealed bootstrap path. With one shared final path, another process's
rename leaves a running child's executable deleted, which is `Unbound`.
That race needs two concurrent test processes, so it was not reproduced
here. With the copy staged under `CARGO_TARGET_TMPDIR` (775 ancestry on this
host), all 21 `capability_broker::` tests still passed, so the old
location's ancestry alone does not bind on this host. The owner-only TMPDIR
run is pending: this seat cannot create a directory outside the worktree,
and refuses an environment prefix on a command. The 21 tests passed with the
seat's inherited TMPDIR.

**Public API.** brokkr-protocol's baseline rises 909 → 911 for the two
consumed `Refusal` variants. `quality/ratchet.sh baselines origin/main`
refuses it alone, and it awaits the operator's ruling.

Each new test or assertion is bound by a removal, restored afterwards. Line
numbers are the formatted tree's:

| Removal | Test and assertion | Result |
| --- | --- | --- |
| V1 H1: the walk key's mount id replaced by `0` | `every_mount_view_is_walked_and_every_nested_mount_proved`, inside the namespace at the two views (`sources.rs:488`), `left: (2, Ok(()))` | 1 failed |
| V2 H2: `stood.root \|` restored in `mapped` | `a_nested_mount_that_also_holds_a_source_root_is_proved_as_both`, and the namespace test at the support source with its nested package (`:497`), `left: (2, Ok(()))` | 2 failed |
| V3 H2: `stood` marks no crossing on a mount holding a root | the namespace test | 1 failed |
| V4 H3: `fenced` ignored | `a_writer_is_confined_only_where_no_exec_or_uid_can_gain_privilege` `:224`, `NoNewPrivs:\t0` read `Proved` | 1 failed |
| V5 H3: `rootless` ignored | the same, `:224`, `Uid:\t1000\t0\t1000\t1000` read `Proved` | 1 failed |
| V6 H3: the `Uid:` line's four fields not required | the same, `:237`, three fields read `([1000], 65534)` | 1 failed |
| V7 H4: `(None, Err(_))` unchanged again | `an_absent_optional_source_that_no_longer_resolves_refuses` `:1000`, the cyclic case `Ok(())`; with that assertion neutralised, the unreadable case `Ok(())` | 1 failed, each time |
| V8 L1: `made.max(observed)` | `program_and_bootstrap_links_refuse_and_the_launch_must_still_name_the_executable` `:1228`, `left: Err(Identity)`, right `Err(Linked)` | 1 failed |
| V9 H3 end to end: the CLI child run without `setpriv --no-new-privs` | ten `capability_broker::` tests, each `filesystem identity is not protected` for `broker serving protections are incomplete` | 10 failed, 11 passed |

The namespace test re-runs its own test binary under
`bwrap --dev-bind / /` with a bind of `lib` at `view`, a tmpfs over
`view/sub`, `pkgs` read-only at `local/lib`, and `pkgs/other` again in reach
at `work/other`, and asserts there that the inner run happened (a marker
file) and passed. Where no namespace can be built it skips as the house's
boundary proofs do, which fails wherever boundary evidence is required.
Here it ran: no skip line printed with `--nocapture`.

Gates on this visit's final tree, each in the foreground:

| Gate | Result |
| --- | --- |
| brokkr-protocol, `--all-features --locked` | lib 774 passed (2 ignored), `hands_exits` 6, `secret_drop` 1, doctest 1 |
| brokkr-cli, `--all-features --locked` | lib 627 (1 ignored), `it` 487 (2 ignored; `capability_broker::` 21 of 21), `driver_conformance` 27, heap tests 1 each, `transcript_surfaces` 13 |
| Rust 1.88 check of every target | finished |
| `quality/ratchet.sh files`, `clones`, `api` | "file size holds", "duplication holds", "public API holds" |
| `quality/ratchet.sh baselines origin/main` | refuses only `brokkr-protocol.txt: 911 public items (was 909)` |
| `bundles/self` | compiled |

`quality/file-lines.txt` takes the measured counts: production 768, 680
and 544, tests `sources.rs` 1,371, `server.rs` 800 and `capability_broker.rs`
1,311. Formatting, clippy with warnings denied, `typos`, `git diff --check`
and `openspec validate --all --strict` are recorded in the result notes of
this visit. Pending: the exact-coverage gate and the ratchet's `crap`
verdict on its LCOV, the owner-only-TMPDIR CLI run, a macOS build, a run
inside a seat box, and remote CI on the final head. The untouched crates'
suites were not rerun on this visit.

## The fourth visit: held capabilities, honest skips, and an unread link

The council confirmed H1, H2, H4 and the identity-setup part of L1 on
`280cf1b7`, and held on one high finding with a medium, a low, an
information item and a recorded low. This visit answers each inside the
row's files and the owning suites; no design changed.

**High, a held capability is never confinement (MB3; ruling 3).** `credited`
admitted any capability outside an ownership mask, so a non-root,
`NoNewPrivs:1` writer holding `CAP_SYS_MODULE`, `CAP_SYS_RAWIO`,
`CAP_SYS_PTRACE`, `CAP_MKNOD` or `CAP_BPF` read `Proved`. `no_new_privs`
stops a gain at exec; it removes nothing already held, and none of those
capabilities has a proof that it cannot write a file. Confinement now needs
the permitted, effective and ambient sets all empty (`held == Some(0)`)
beside the four-uid and `NoNewPrivs` conditions, and the `OWNERSHIP` mask,
left with no reader, is gone. The new test
`a_writer_holding_any_capability_is_unconfined_whatever_no_new_privs_says`
sets one capability at a time in each of the three sets of the confined
status. The complete managed-writer set stays with U6c5b.

**Low, an unreadable linked file keeps its link (rulings 8 and 9).** The
resolution's last step lost the file's facts when its read-open failed, so a
bootstrap with a second link refused `Linked` at 0755 and `Identity` at 0111.
Now a regular file that will not open for reading is held by an `O_PATH`
handle, still checked to be the object observed, as `host::Object::Unread`.
`Observer::tree` applies the link rule to it and then refuses `Identity`, so
MB3's order holds: reach, then links, then identity. The new test
`a_bootstrap_that_cannot_be_read_keeps_its_link_and_reach_causes` asserts
`Identity` for a singly linked unreadable bootstrap, and `Linked` (no reach)
and `BindOverlapsReach` (in readable reach) at both 0755 and 0111 once it
has a second link.

**Medium, a skip is declared (ruling 9).** Every positive that cannot run in
a seat's hands box now calls the house's `skip_boundary_proof` with
`boundary_evidence_required()` before it returns: `tests::server::skip_in_box`
in the protocol suites (three server tests, four sources tests, and the
loop's admitted case in `the_server_box_stays_clear_of_every_reach_root_either_way`),
and `skip` in `capability_broker.rs` (the ten boxed tests). The negatives
that need no namespace now run first and unconditionally:
`a_launcher_that_cannot_mount_a_descriptor_leaves_the_box_unavailable`
asserts the linked-package `Linked` before its skip, and
`program_and_bootstrap_links_refuse_and_the_launch_must_still_name_the_executable`
asserts all three `Linked` causes (package, bootstrap, private-directory
bootstrap) before its skip, under writer credentials that own nothing.

**Information, plan-bound CLI tests in a box (ruling 9).** Outside a box
every fixture root and the binary's copy already lie under `/var/tmp`, a
fresh owner-only directory of `tempfile`'s. A seat's hands box binds no
`/var/tmp`, so there they lie under `CARGO_TARGET_TMPDIR`, whose ancestry on
an umask-002 host is group-writable, and three tests that need a bound plan
returned `Unbound`. No directory a box can write has protected ancestry on
such a host, so those three now check `protected_base()` (the broker guard's
rule over every ancestor of the base) and skip through `skip` where it
fails; where it holds, as on an umask-022 host, they run.

**Recorded low, which file holds resolution (ruling 4).** The 2026-10-07
addendum and design.md's U6c5a section and file table now say that
`resolve`, `Walk`, `Hop` and `Resolved` live in `sources/host.rs`. The
ruling itself, four files and a pure size split, is unchanged.

Each new test or skip is bound by a removal, restored afterwards; every run
below is `cargo test -p brokkr-protocol --lib` or
`cargo test -p brokkr-cli --test it` filtered to the test named:

| Removal | Test and assertion | Result |
| --- | --- | --- |
| W1 `CAP_SYS_MODULE` admitted (`held & !(1 << 16) == 0`) | `a_writer_holding_any_capability…`, the loop's assertion, `left: ("CAP_SYS_MODULE", "CapPrm", Proved)` | 1 failed |
| W2 `CAP_SYS_RAWIO` admitted (bit 17) | the same, `("CAP_SYS_RAWIO", "CapPrm", Proved)` | 1 failed |
| W3 `CAP_SYS_PTRACE` admitted (bit 19) | the same, `("CAP_SYS_PTRACE", "CapPrm", Proved)` | 1 failed |
| W4 `CAP_MKNOD` admitted (bit 27) | the same, `("CAP_MKNOD", "CapPrm", Proved)` | 1 failed |
| W5 `CAP_BPF` admitted (bit 39) | the same, `("CAP_BPF", "CapPrm", Proved)` | 1 failed |
| W6 `CAP_CHOWN` admitted (bit 0) | the same, `("CAP_CHOWN", "CapPrm", Proved)` | 1 failed |
| W7 `Object::Unread` skips the link rule | `a_bootstrap_that_cannot_be_read…`, the loop's assertion, `left: (73, (Err(Identity), Err(BindOverlapsReach)))` | 1 failed |
| W8 a failed read-open refuses in `Walk::step` (the old behaviour) | the same, `left: (73, (Err(Identity), Err(Identity)))` | 1 failed |
| W9 `Object::Unread` does not refuse `Identity` | the same, the singly linked assertion, `left: Ok(())` | 1 failed |
| W10 `protected_base()` forced true, `BROKKR_HANDS_BOX=1` | the three plan-bound CLI tests, each `left: (Some(1), "error: broker plan is not bound to this attempt\n")` | 3 failed, 18 passed |

(73 is 0o111.) The skips were checked by running the suites as a box would,
`BROKKR_HANDS_BOX=1` set through cargo's `--config env.…`, since this seat
refuses an environment prefix. With `BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1` as
well, the protocol `sources`/`server` filter gave 28 passed and 9 failed,
seven "a seat's hands box prepares no server box" and two "no namespace can
be built here", the descriptor-mount test among them; the CLI suite gave 8
passed and 13 failed, ten for the box and three for the unprotected base.
Without the declaration both passed: 37 of 37 and 21 of 21.

The CLI suite outside a box, at this shell's umask 002 (a new file reads
664, a new directory 775), passed 21 of 21 with the inherited environment,
and 21 of 21 with `TMPDIR` forced (`--config env.TMPDIR.force=true`) to a
fresh mode-0700 directory under `.forge/`. A control proves the forced
value reaches the broker: `TMPDIR` naming a regular file turned
`a_sealed_plan_is_admitted_and_still_refused_before_any_lookup_or_start` into
`filesystem identity is not protected`.

Gates on this visit's final tree, each in the foreground:

| Gate | Result |
| --- | --- |
| brokkr-protocol, `--all-features --locked` | lib 776 passed (2 ignored), `hands_exits` 6, `secret_drop` 1, doctest 1 |
| brokkr-cli, `--all-features --locked` | lib 627 (1 ignored), `it` 487 (2 ignored; `capability_broker::` 21 of 21), `driver_conformance` 27, heap tests 1 each, `transcript_surfaces` 13 |
| brokkr-runtime `--test it witness_digests::` | 6 passed, no pin moved |
| Rust 1.88 check of every target | finished |
| `quality/ratchet.sh files`, `clones`, `api` | "file size holds", "duplication holds", "public API holds" |
| `quality/ratchet.sh baselines origin/main` | refuses only `brokkr-protocol.txt: 911 public items (was 909)`, which awaits the operator's ruling |
| `bundles/self` | compiled, exit 0 |
| `openspec validate --all --strict` | 20 passed, 0 failed |

`quality/file-lines.txt` takes the measured counts: production 293, 768,
685 and 553, tests `sources.rs` 1,434, `server.rs` 808 and
`capability_broker.rs` 1,349. Clippy's forced `too_many_lines` listing
names no function in a touched file beyond the baselined
`hands/tests.rs:127`, so `too-many-lines.txt` stands. Formatting, clippy
with warnings denied, `typos --hidden` and `git diff --check` were clean.
Pending: the exact-coverage gate and the ratchet's `crap` verdict on its
LCOV, a macOS build, a run inside a real seat box, and remote CI on the
final head. The untouched crates' suites were not rerun on this visit.

## The fifth visit: one observation for every object

Run `0065-slice-two-unit-u6c5a-see-th-8445c0aa` (head `195fe770`) closed the
fourth visit's findings, and its council confirmed them, then held on two new
high findings, a medium, a low and two information items. Run
`0065-slice-two-unit-u6c5a-see-th-6932abdd` takes the whole of that attempt,
uncommitted on main at `8923a24c`, and answers each. Four visits had each
closed their findings and each council found a new fail-open edge in the
walk, so the controller asked that this repair be structural. It is: two
seams now carry what was spread across call sites.

**High H1, one observation for every object (MB3; rulings 3 and 9).**
`Observer::object` in `sources.rs` is the one function every object the walk
reaches goes through: a source's own object and every entry found beneath
it, whatever its kind and however many links it has. It reads MB3's link
cause from the facts first, so a linked program file refuses `Linked`
whether or not it opens. Then it opens the object (a regular file for
reading, a directory for listing, a link as itself with `O_PATH`) and
compares the handle's device and inode with the facts observed. A handle
that does not open or is another object refuses `Identity`, and a special
file always does. A linked support file is last held to the write-exclusion
proof, its ACL read through that handle. Nothing reaches the walk any other
way:

- The resolution no longer opens its own last object. `host::resolve` ends
  with the directory handle and the name (`.` for a directory),
  and `Observer::source` hands those to `object`. `Object::Unread` and its
  `O_PATH` fallback are gone, so a terminal source and a descendant are one
  code path.
- A directory entry is observed when it is entered, from its parent's
  handle on the active stack.
- A link entry's target is read through the link's own checked handle
  (`readlinkat` with an empty name), not by name a second time.

The fourth visit's skip of the open for `nlink <= 1` cannot recur: no branch
before the open depends on the link count.

This has a host consequence the spec states outright: "an unreadable entry
inside an actual admitted source still refuses 'MCP server box filesystem
identity is not protected' before lookup" (spec.md, MB3's certificate
scenario). This host's server system set holds eight root-only files, and a
temporary trace in `object` named each one. They are
`/usr/lib/tmpfiles.d/nordvpn.conf` and
`/usr/lib/netplan/00-network-manager-all.yaml` (0600),
`/usr/share/chrony/chrony.keys` (0640), and `proxy_child`, `krb5_child`,
`sssd_pam`, `selinux_child` and `ldap_child` under `/usr/libexec/sssd`
(0750). So every live box here now refuses `Identity`.

The positive tests therefore ask first, independently of the observer,
whether a live box can stand. `tests::server::unservable` and the CLI
suite's `unservable` walk the server system set with plain metadata, once
per process, for an entry that cannot be read or listed or is special. Where
they find one, the positive is skipped through `skip_boundary_proof`, which
fails wherever boundary evidence is required, and the skip message names
the entry.

A scratch control shows these positives pass wherever that refusal is the
only cause. It let only an unreadable regular file pass on an `O_PATH`
handle and forced both checks to "servable". Under it the protocol
`hands::tests::` filter gave 63 passed and 2 failed (exactly the two
unreadable-file tests), and the CLI `capability_broker::` filter gave 21 of
21. Both were reverted.

This is recorded for the operator, not decided here. On a host with
root-only files under `/usr/lib`, `/usr/libexec` or `/usr/share`, no server
box stands until the server projection narrows further, as it already did
for `/etc/ssl`. (The operator ruled on it on 2026-10-07; the sixth visit
admits such files under MB3's proof and drops these skips.)

**High H2, the byte budget: withdrawn.** The fifth visit answered H2 with a
`host::Budget` charging every kept byte. The operator removed the 256 MiB
byte bound on 2026-10-07, so that budget, its charge sites, its tests and
their removals are deleted (sixth visit, below), and nothing in this file
claims a byte bound any longer.

**Medium, pre-namespace negatives run in a box (ruling 9).** In
`capability_broker.rs` the whole-test `boxed()` returns are gone from the
seven tests whose refusals come before `ServerBox::prepare`: the authored
plan, a missing, wrong or duplicated inventory, an absent or relative HOME,
the unprotected roots and files, the attempt and size, the positional
records, the wrong shapes and the program tree. Their negatives now run
unconditionally. Each positive control goes through `binds(case, answer)`,
which checks three things:

- Where a live box stands, the answer is `admitted()`.
- Where none can, it is exactly the observer's `Identity`, past binding and
  the program tree, and the rest is declared skipped.
- Where no fixture directory has protected ancestry, the skip is declared
  through `skip(UNPROTECTED)`.

The program tree test checks `protected_base()` first, since its
`ProgramTree` causes follow binding. Only the three tests that are wholly
past the observer skip whole, through `unservable()`. In a real seat box,
root-owned ancestors read as the overflow uid, so nothing has the protected
ancestry binding needs. There the negatives run and pass, and the binding
control declares its skip.

**Low, unused state (ruling 6).** `Stood` and its `root` field are gone.
`Seen` maps each (mount, device) to whether the walk was nested there. The
three nested-mount proofs pass unchanged in substance:
`every_mount_under_a_source_is_proved_against_the_table`,
`a_nested_mount_that_also_holds_a_source_root_is_proved_as_both` and the
namespace-built `every_mount_view_is_walked_and_every_nested_mount_proved`.

**Information.** The tests that make a file unreadable start with
`unprivileged()`, which skips through `skip_boundary_proof` when the euid
is 0. The CLI suite's copy directories are now kept live by an exclusive
`flock` on their `lock` file, made under another name and renamed in once
locked, and the sweep removes only a directory whose lock it can take. A
lock means the same in every PID namespace that shares `/var/tmp`, which a
process id does not. Neither the root skip nor a concurrent sweep from
another namespace could be exercised here; both are pending.

**Complexity.** `cargo crap` over a hand-written LCOV measured
`Walk::step` at 20, so its per-kind decision moved to `host::take` behind a
closed `Taken` enum, with no wildcard arm. The touched functions now
measure `Walk::step` 15, `Observer::entry` 14, `Observer::source` 13,
`Observer::object` 11 and `take` 10.

### The new tests and how they bind

The new protocol tests:

- `a_file_inside_a_tree_that_cannot_be_read_refuses_as_the_same_file_as_a_source_does`
- `an_object_inside_a_tree_replaced_before_it_opens_refuses`, whose cases
  are a file, a directory and a link

The fifth visit's two byte-budget tests and the mapping test's byte check
are deleted with the budget (sixth visit).

Each removal below is a compiling mutation, restored after its run. Every
run is `cargo test -p brokkr-protocol --lib hands::tests::sources::` (31
tests), and the production files were copied back from saved copies.

| Removal | Test and assertion | Result |
| --- | --- | --- |
| M1 `object` lets an unreadable singly linked file pass on an `O_PATH` handle | `a_bootstrap_that_cannot_be_read…` `left: Ok(())`; `a_file_inside_a_tree…` `left: ((Ok(()), Ok(())), (Ok(()), Ok(())))` | 2 failed, 29 passed |
| M2 the link cause raised only after a successful open | the bootstrap test `left: (73, (Err(Identity), Err(BindOverlapsReach)))`; the tree test `left: (Err(Identity), Err(Identity))` | 2 failed |
| M3 `object` opens with no identity comparison | `an_object_inside_a_tree_replaced…` `left: ("file", Ok(()))` | 1 failed |
| M3b a resolution hop opens with no identity comparison | `a_handle_that_opens_another_object…` `left: Ok(())` | 1 failed |

(73 is 0o111.) The fifth visit's removals B1–B15, each of a byte charge,
are withdrawn with the budget. Counting the table's lines before keeping a
record changes no answer, so no test can see that order. It holds by
construction in `records`.

### Gates on this visit's final tree

The suites ran at this shell's umask 002, which a shell-created file reading
664 confirms:

| Suite or gate | Result |
| --- | --- |
| brokkr-protocol, `--all-features --locked` | lib 780 passed (2 ignored), `hands_exits` 6, `secret_drop` 1, doctest 1 |
| brokkr-cli, `--all-features --locked` | lib 627 (1 ignored), `it` 487 (2 ignored), `driver_conformance` 27, the three heap tests 1 each, `transcript_surfaces` 13 |
| protocol `hands::` with `--nocapture` | 78 passed; 8 positive skips name `nordvpn.conf` |
| CLI `capability_broker::` | 21 of 21; 10 positive skips name the netplan file |
| CLI `capability_broker::` as a box would run it (`--config env.BROKKR_HANDS_BOX`) | 21 of 21; 15 skips for the unprotected base, 3 for the box |
| `TMPDIR` forced to a fresh mode-0700 directory under `.forge/` | CLI `capability_broker::` 21 of 21; protocol `hands::` 77 passed, 1 failed (`git_works_in_the_box_and_cannot_plant_a_hook`, `left: Some(".../the-forge/.git/worktrees/brokkr-wt-s2-a")`: its box finds the worktree's own Git directory from a `TMPDIR` inside the worktree. The test is unchanged on this branch, and the same configuration was not run against main) |
| brokkr-runtime `--test it witness_digests::` | 6 passed, no pin moved |
| Rust 1.88, every target and feature | checked |
| workspace clippy, warnings denied | clean |
| `cargo fmt --check`, `typos --hidden`, `git diff --cached --check` | no finding |
| `quality/ratchet.sh files`, `clones`, `api` | "file size holds", "duplication holds", "public API holds" |
| `quality/ratchet.sh baselines origin/main` | refuses only `brokkr-protocol.txt: 911 public items (was 909)`, the two `Refusal` variants, which await the operator's ruling |
| `bundles/self` | compiled, exit 0 |
| `openspec validate --all --strict` | 20 passed, 0 failed |

An owner-only `TMPDIR` outside the worktree could not be made: this seat
may not create a directory under `/var/tmp`. Since every live box here
refuses `Identity`, a forced `TMPDIR` no longer reaches an observable
positive on this host. `quality/file-lines.txt` takes the measured counts
for the five files this visit moved:

| File | Lines |
| --- | --- |
| `sources.rs` | 785 |
| `host.rs` | 757 |
| `tests/sources.rs` | 1,651 |
| `tests/server.rs` | 857 |
| `capability_broker.rs` | 1,415 |

The forced `too_many_lines` listing names no touched function beyond the
baselined `hands/tests.rs` ones, so `too-many-lines.txt` stands.

Still pending:

- the exact-coverage gate and its `crap` verdict;
- a macOS build;
- a real seat box;
- a host whose server system set is wholly readable, where the live
  positives run rather than skip;
- remote CI on the final head.

## The sixth visit: root-only system files, and counts as the memory bound

Run `0065-slice-two-unit-u6c5a-see-th-6375f79f` takes the fifth visit's
attempt and the operator's two rulings of 2026-10-07, all uncommitted on
main at `8923a24c`, and delivers them as one commit. The fifth visit's
council confirmed its unreadable-descendant repair, `Linked` precedence, the
removal of `Stood.root`, the root-runner skip and the `flock` fixture
lifetime; this visit keeps each.

### Ruling 1: an unreadable file outside the program tree and bootstrap

`Observer::object` now opens a regular file as a path handle (`O_PATH`,
`O_NOFOLLOW`), and `handle` requires that handle's own facts to show the
device, inode and file type observed. Only then is the file reopened for
reading through that handle (`host::readable`, an open of
`/proc/self/fd/<n>`), so no name is looked up again. `EACCES` from the reopen
means the file cannot be read; any other failure refuses `Identity`. Then:

- A program-tree, launch, generated or bootstrap file that cannot be read
  refuses `Identity`, whoever owns it. A linked one keeps `Linked` first.
- A support file that is linked or cannot be read takes the kernel
  write-exclusion proof (`host::excluded`). Its access ACL is read with
  `getxattr` on `/proc/self/fd/<n>` (`host::acl`), which needs no read access,
  so a path handle serves. An unreadable one must also be owned by root
  (`host::untouchable`). Every other condition of the proof is unchanged:
  owner and writers mapped, the owner no writer, no group or other write, no
  ACL, and the writers' privilege confined.
- An unreadable directory still refuses. It cannot be listed, and nothing is
  skipped. The ruling's exception covers files.

The returned handle is the reader where one opened, and the path handle
otherwise. bubblewrap's `--ro-bind-fd` mounts either.

### Ruling 2: the byte budget is gone

`host::Budget`, `FACT_BYTES`, `Limits::metadata` and every charge site are
deleted. The observer keeps the four count bounds of `LIMITS`: entries
(`Observer::count`, which also stops the walk), depth (`directory`), hops per
resolution (`Walk::step`) and mount records (`records`, counted before any
record is kept). The two tests that measured bytes are deleted, as is the
mapping test's byte check. What remains of size arithmetic is checked:

- A link's room is its size plus one only after that size is held to
  `LONGEST`. That is 4,095, the longest target Linux makes (`PATH_MAX` less
  its NUL). A larger size refuses as an unknown fact, before any
  allocation. This is a refusal of an impossible fact, not a fifth bound.
- Paths are now built with `Path::join` and `PathBuf`, with no length sums.

The mount table is read whole before its records are counted. Its size is
the kernel's own, bounded by the host's mount count. Each directory's names
are gathered before they are sorted (R8), and each name is counted against
the entry bound as it is read.

### R7, R8 and R2

**R7 (low).** This is answered within the row's files, as above: the read
open goes through a held, checked path handle. The kind comparison in
`handle` is a new, independent check. On this host M8 below shows it is the
only thing standing when a replacing FIFO reuses the removed file's inode
number. ext4 reused it in that run, so device and inode alone matched.

**R8 (low).** `Observer::listed` sorts each directory's names before any
entry is observed or digested. The walk pops pending directories from the
end of that sorted list, so the descent order is a function of the tree.
The mount records the walk stood on are digested sorted by place, root and
device, never in the table's order.

**R2 (medium).** In `capability_broker.rs`:

- `bindable()` returns whether the base is protected, declaring the skip
  through `skip_boundary_proof` where it is not.
- `unbound_fixture()` gives a sealed fixture and the unbound answer only
  where `bindable()` holds.
- All eight tests whose negatives assert `Unbound` take the fixture through
  it: the inventory, the engine's home, the unprotected root, the attempt
  and size, the field changes, the closed parse, the positional records and
  the wrong shapes. Where the base is not protected, none of their
  negatives runs.
- Each has a valid control that must reach past binding through `binds`.
  The field-change and closed-parse tests gained theirs (`"as sealed"`).
- `binds` no longer checks the base itself, and the four other
  post-binding tests use `bindable()`.
- The CLI's `unservable()` is now the hands box alone. Its walk for
  unreadable entries is gone, because MB3 admits root-only files.

### The host positive

The protocol suite's `unservable()` is likewise the box alone, so the live
positives run here. With `--nocapture`,
`a_server_entry_root_owned_and_singly_linked_passes_with_the_systems_links_intact`
printed 8 root-only regular files in the server system set:

- `/usr/lib/netplan/00-network-manager-all.yaml`
- `/usr/lib/tmpfiles.d/nordvpn.conf`
- `/usr/libexec/sssd/` `krb5_child`, `ldap_child`, `proxy_child`,
  `selinux_child` and `sssd_pam`
- `/usr/share/chrony/chrony.keys`

Both programs prepared `Ok`:

| Program | Entries | Mount records |
| --- | --- | --- |
| `/usr/bin/dash` | 205,078 | 170 |
| `/usr/lib/apparmor/apparmor.systemd` | 205,082 | 170 |

In this visit's first run the same test read 172 mount records. The whole
table's count moves with the host, as the fifth visit recorded. No source
was changed, copied or pruned. The CLI suite's 21 tests ran with 0 skips, so
the real binary admitted sealed plans on this host.

### Tests

The tests are in `hands/tests/sources.rs` (1,715 lines),
`hands/tests/server.rs` (822) and `capability_broker.rs` (1,400). No test
moved between files. Deleted:

- `the_budget_refuses_a_charge_past_its_bound_and_every_one_after`
- `every_byte_an_observation_keeps_is_charged_before_it_is_kept`, with its
  helpers `metered`, `usage` and `exactly`
- the byte check in `every_mount_under_a_source_is_proved_against_the_table`
- `refused_system_entry` in `server.rs`, and `refused_system_entry` and
  `refused_below` in the CLI suite

New:

| Test | What it pins |
| --- | --- |
| `an_unreadable_file_passes_only_as_support_root_owns_and_no_writer_can_write` | It reruns itself under `bwrap --unshare-user --uid 0 --gid 0 --cap-drop ALL`, where root owns every fixture file and, holding no capability, reads none. That a file opens with `PermissionDenied` and is owned by uid 0 is asserted first. A root-only support file with no write bit passes as a source, within a tree and within a package tree in the support role. Group write, other write and a writer's ACL entry (kept through `chmod 000`) each refuse `Identity`. The same package tree as a program, and a bootstrap, refuse `Identity`. |
| `a_file_replaced_before_it_opens_is_never_opened_to_be_read` | A file swapped for a FIFO at `Host::opening` refuses `Identity`. An `inotify` `IN_OPEN` watch on the tree records no open of `a.py`: the FIFO is never opened to be read. |
| `the_digest_follows_the_tree_never_the_order_it_is_listed_or_mounted_in` | A `/dev/shm` tree whose listing order a rename-away-and-back changes is asserted reordered, its directory time restored. It keeps its digest. The same observation over the mount table read in reverse keeps its digest, 6 entries and its record count. |

The non-root-owner negative is the existing
`a_file_inside_a_tree_that_cannot_be_read_refuses_as_the_same_file_as_a_source_does`:
this process's own unreadable module in the support role. Its linked case's
comment now says that an unreadable support file must be root's. The
namespace rerun shared by the views test and the new rooted test is one
helper, `rerun`, which answered the clone ratchet.

### Removals

Each removal below compiled. It was made with one edit, run with
`cargo test -p brokkr-protocol --lib hands::tests::sources::` (M6 with
`hands::tests::`), and restored with the inverse edit. A `grep` of every
mutated line then showed each restored.

| Removal | Test and assertion | Result |
| --- | --- | --- |
| M1 `untouchable` without `file.uid == 0` | `a_file_inside_a_tree_that_cannot_be_read…` at its unreadable-support assertion | 1 failed |
| M2 group write ignored (`mode & 0o002`) | rooted `left: ("sys/group", Support, Ok(()))`; also the predicate and linked-support tests | 3 failed |
| M3 other write ignored (`mode & 0o020`) | rooted `left: ("sys/other", Support, Ok(()))`; also the predicate and linked-support tests | 3 failed |
| M4 ACL ignored (`let acl = true`) | rooted `left: ("sys/granted", Support, Ok(()))`; also the predicate and linked-support tests | 3 failed |
| M5 unreadable program file not refused (`program & unread & false`) | rooted `left: ("opt/docs", Program, Ok(()))`; `a_bootstrap_that_cannot_be_read…` `left: Ok(())`; the tree test | 3 failed |
| M6 the root-only admission withdrawn (`untouchable(..) & false`) | rooted `left: ("sys/sealed/f", Support, Err(Identity))`; the host positive `left: ("/usr/bin/dash", Err(Identity))`; every live box test | 9 failed, 57 passed |
| M7 a regular file opened to be read by name (`RDONLY \| NONBLOCK \| NOCTTY` for `Kind::File`) | `a_file_replaced_before…` `left: ["a.py"]`, right `[]` | 6 failed |
| M8 `handle` without the kind comparison | `a_file_replaced_before…` `left: (Ok(()), true)`, right `(Err(Identity), true)` | 1 failed |
| M9 names not sorted | `the_digest_follows…`, the reordered listing's digest differs | 1 failed |
| M10 mount records not sorted | `the_digest_follows…` at the reversed table's digest | 1 failed |
| M11 entries bound one higher | `each_bound_refuses_one_over…`, entries 2 `left: Ok(())` | 1 failed |
| M12 depth bound one higher | `each_bound_refuses_one_over…`, depth 2 `left: Ok(())` | 1 failed |
| M13 hop bound one higher | `each_bound_refuses_one_over…`, hops 2 `left: Ok(())` | 1 failed |
| M14 mount bound one higher | `the_mount_table_is_read_strictly…`, three records at bound two `left: Ok([…])` | 3 failed |

M7 also failed every live positive. Under it a root-only file's read open by
name fails before any handle is held, so the observation refuses `Identity`
on this host. Each bound's exact-limit positive passed in
every run where its own removal was absent. M8 depends on the filesystem
reusing the inode number. This host's ext4 did; on a filesystem that does
not, the identity comparison alone refuses the swap.

### Gates on this visit's tree

The shell's umask was 002 (a new file read 664).

| Suite or gate | Result |
| --- | --- |
| brokkr-protocol, `--all-features --locked` | lib 781 passed (2 ignored), `hands_exits` 6, `secret_drop` 1, doctest 1 |
| brokkr-cli, `--all-features --locked` | lib 627 (1 ignored), `it` 487 (2 ignored), `driver_conformance` 27, the three heap tests 1 each, `transcript_surfaces` 13 |
| brokkr-runtime, `--all-features --locked` | lib 834, `capability_launch` 75, `it` 120 (`witness_digests::` among them, no pin moved), `operated_repo` 1, `queued_launch` 3 |
| bridge, core, seatbelt probe, store, view | 17; 105 and 24; 99 (2 ignored); 85 and its `it` targets; 269 (3 ignored) and 6 |
| protocol `hands::` with `--nocapture`, after the last edit | 79 passed (1 ignored); the only skip is the deliberate skip test's own |
| CLI `capability_broker::` with `--nocapture` | 21 of 21, 0 skips |
| the same with `TMPDIR` forced (cargo `env.TMPDIR` with `force`) to a mode-0700 directory under `.forge/` | 21 of 21, 0 skips |
| the same as a box runs it (`env.BROKKR_HANDS_BOX`) | 21 of 21; 12 skips for the unprotected base, 2 for the box |
| protocol `hands::` with that forced `TMPDIR` | 78 passed, 1 failed: `git_works_in_the_box_and_cannot_plant_a_hook`, `left: Some(".../the-forge/.git/worktrees/brokkr-wt-s2-a")`, as the fifth visit found (a `TMPDIR` inside the worktree; the test is unchanged here) |
| Rust 1.88, every target and feature | checked |
| workspace clippy, warnings denied | clean |
| forced `too_many_lines` over protocol and CLI | no touched file listed |
| `cargo fmt --check`, `typos --hidden`, `git diff --check` | no finding |
| `quality/ratchet.sh files`, `clones`, `api` | "file size holds", "duplication holds", "public API holds" |
| `quality/ratchet.sh baselines origin/main` | refuses only `brokkr-protocol.txt: 911 public items (was 909)`, the two `Refusal` variants, which await the operator's ruling at the pull request |
| `bundles/self` | compiled, exit 0 |
| `openspec validate --all --strict` | 20 passed, 0 failed |

This seat could not make a directory outside the worktree, nor set an
environment variable but through cargo's `--config env.`. `quality/file-lines.txt`
takes the counts of the five files this visit moved: `sources.rs` 783,
`host.rs` 646, `tests/sources.rs` 1,715, `tests/server.rs` 822 and
`capability_broker.rs` 1,400.

A re-vouch on 2026-10-07 (run `0065-slice-two-unit-u6c5a-see-th-364ca7cc`)
carries the sixth visit's whole net diff, uncommitted, on current main. After
the council cleared that visit (#576), CI refused every admitted-path broker
test on ubuntu-latest with "filesystem identity is not protected". The
diagnostic run (#577, closed) found the observer was right. The hosted image
ships `/usr/share` mode 777 with access ACLs, `/usr/lib/jvm` and parts of
`/usr/local` mode 777, and about 6,300 packer- and runner-owned files under
`/usr/local`. On the operator's ruling of that day, `setup-bubblewrap` gained
an opt-in `protect-system-tree` step. The step strips access ACLs, hands root
the non-root-owned entries (owners only, groups left alone) and drops group
and other write across the server's system set. The Linux test leg, the
exact-coverage gate and both mutants jobs opt in, and `contributing.rs`'s held
job lines follow. This visit added `/bin`, `/sbin`, `/lib` and `/lib64` to the
step's list, the HOST_TOOLCHAIN_BINDS entries it had missed. On a merged-usr
image they are symlinks, which the step skips. The observer and its tests are
unchanged.

A second re-vouch on 2026-10-07 (run `0065-slice-two-unit-u6c5a-see-th-54f64cd6`)
judged the same whole diff after the controller's lockdown let CI pass every
test on the protected runner, and after it closed the exact gate's last line
and two branches: `Walk::follow` now takes the link target its step already
holds, and `an_entry_under_the_root_digests_as_its_joined_path` binds the
digest's place directly under `/`. This visit found nothing to correct. The
step's eighteen roots cover the server's whole system set: every
HOST_TOOLCHAIN_BINDS entry, with `/etc/ssl` projected to `/etc/ssl/certs`, and
the system bin directories, `/usr/local/bin` and `/usr/local/sbin` through
`/usr/local`. Both repairs bind here. Making the digest always write its
separator fails the new test at its first assertion (`of("/", "usr")` against
`of("/usr", "")`), and dropping `follow`'s restart from `/` for an absolute
target fails four sources tests, among them
`each_bound_refuses_one_over_and_passes_at_its_limit` and
`a_route_or_link_target_in_reach_refuses`; both were restored and the sources
suite passes 33 of 33. The operator has since approved the API rise to 911, so
of the list below only the exact gate on this head, macOS, a real seat box and
remote CI on the final head stay pending.

Still pending:

- the exact-coverage gate, `scripts/coverage-exact.sh`, and its `crap`
  verdict. This seat could not build the LCOV for a cyclomatic count;
- a macOS build;
- a run inside a real seat box;
- remote CI on the final head, including whether GitHub's image has
  root-only system files and passes;
- the operator's API ruling for 909 → 911.

U6c5b still owns the managed-writer set. The positives here stand this
process's own credentials, with `no_new_privs` set as a confined writer
holds it, for every writer. U6c5b also owns the launcher's `--version`
ordering and the `CLOEXEC`-to-exact-descriptor launch (R6).
