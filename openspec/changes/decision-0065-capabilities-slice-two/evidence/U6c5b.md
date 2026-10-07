# U6c5b: sealed sources compared at admission through a cancellable observer helper (its parts of tasks 28.9, 28.10 and 28.17)

Run `0065-slice-two-unit-u6c5b-see-th-3038e60d` delivered U6c5b as the
operator narrowed it on 2026-10-07, as the signed commits `0f189c02` (code and
tests) and `806163d4` (this file, task and spec text, quality rows, the
public-API snapshot) on main at `89deef2c`; its council cleared it at
residual/low (PR #581) and CI run 37619579219 then failed. Every mutation in
the first table below was run against `0f189c02`. The second repair visit, run
`0065-slice-two-unit-u6c5b-see-th-8efb7873`, carries that whole attempt onto
main at `523a1f0a` as one signed commit with the repairs in "Second repair
visit" below. The production changes stay inside the row's five files and the
one Cargo feature line:

| File | Lines at `523a1f0a` → now |
| --- | --- |
| `crates/brokkr-cli/src/broker.rs` | 92 → 101 |
| `crates/brokkr-cli/src/broker/session.rs` | 377 → 785 |
| `crates/brokkr-cli/src/cli_args.rs` | 754 → 761 |
| `crates/brokkr-protocol/src/hands/namespace.rs` | 768 → 777 |
| `crates/brokkr-protocol/src/hands/namespace/sources.rs` | 788 → 797 |
| `crates/brokkr-cli/Cargo.toml` | rustix gains `net`, nothing else |

`Cargo.lock` did not move, because a feature is no new package. Contracts,
fixtures, the policy table, `reference/`, `extensions/`, recipes, adapters
and agents are unchanged. No grant moved. Every admitted plan still ends in
`broker serving protections are incomplete` before any lookup or start, and
the realm-wide compile fence test passes unchanged.

## What the unit builds

`serve` binds the plan itself and screens what needs no source: the plan's
shape, the binding names and the fixed keys. A binding name's cause, the one
refusal carrying text, is therefore always the broker's own. It then starts
`brokkr broker observe`, a hidden `BrokerCmd` verb taking the same bounded
`--plan` and `--plan-digest`. The verb runs in a process group of its own
with stdin and stderr null, and its stdout is one end of a `SOCK_SEQPACKET`
pair. The observer:

1. refuses to begin unless the socket's creator (`SO_PEERCRED`) is still its
   parent, then starts a watcher thread that kills its whole process group
   when the broker's end of the socket closes (second repair visit; the
   first attempt set a parent-death signal instead);
2. rebinds the plan from its locator and digest;
3. screens it again, then prepares the box (program tree, reach, the source
   observer) and runs the checks that follow a standing box (identity, then
   store reach, then store in box);
4. sends one closed record with the checked handles riding it as
   `SCM_RIGHTS`, and exits.

The record has two shapes. `{"refused": "<cause>"}` means refused before a
box stood. `{"observed": {...}}` carries `entries`, `mounts`, `digest`, the
writers' uids where their privilege was proved confined, a later check's
cause, and the host path of each handle in the order they travel. A cause
travels as a closed serde enum, `Cause`, by its kebab-case name
(`unbound` … `store-in-box`), the ten refusals the observer can meet; a name
outside it parses as no record. (The first attempt sent a cause's position
in a table as a number.)

The broker reads the record, then reads the socket's end. The socket closes
only when the observer has exited, so the broker waits for that end within
the same deadline; a second message counts as no record. It reads in slices
of at most 100 ms under the absolute 30 s startup deadline, which is fixed at
admission's start and has no knob. Cancellation is the broker's starter
ending (its parent changes). On cancellation, on expiry, and after every
record, the broker kills the observer's whole group and reaps the observer.

The comparison follows. The observed `entries`, `mounts` and `digest` must
be the sealed ones. The observed writers must be confined and, as a set, the
sealed uids, where the sealed privilege is `confined`, its only variant.
Otherwise identity is not protected. Drift outranks any cause the observer
found after the box, so a store sealed into a drifted package answers with
identity, not "in box". Once the sources match, the observer's own later
cause stands. The handles must be as many as the paths naming them.
Admission then holds each handle by its path until the broker ends, so the
later launch mounts these very objects.

A plan the observer cannot rebind answers "not bound"
(`"refused": "unbound"`). A filesystem that moved answers identity, so SC1's
distinction holds.

In the protocol, `Sources::mounts` counts only the mount records the walk
stood on (L4), and the box keeps the writers' uids only where their privilege
is proved confined (`confined_writers`). `ServerBox` gains three public
methods: `sources()` and `handles()`, which were test-only before, and
`writers()`. Session.rs's second copy of the entry and mount ceilings is gone
(L3): a sealed count is now bounded by having to equal an observed count,
and the observer's `LIMITS` bounds every observed count.

## Proofs

New tests live in `crates/brokkr-cli/tests/capability_broker/observer.rs`, a
child module that `capability_broker.rs` registers under
`#[cfg(target_os = "linux")]`, and in
`crates/brokkr-protocol/src/hands/tests/sources.rs`. No test moved. The
fixture's plans now seal the sources the observer itself reports for them,
and this process's uid as the writer. The program-tree test re-observes each
program it admits.

| Test | What it shows |
| --- | --- |
| `observer::admission_compares_the_sealed_sources_with_a_fresh_observation` | Unchanged sources are admitted. The server replaced by a same-bytes, same-mode, same-time file, with its directory's time restored, refuses with identity, with zero lookups (FIFO store) and zero starts (marker). Drift outranks the store-in-box cause, and a fresh seal admits again. |
| `observer::the_observer_hands_back_the_very_handles_it_checked_in_a_closed_record` | The record is exactly `{"observed": {digest, entries, handles, mounts, refused, writers}}`. Its facts are the sealed ones, `refused` is null and the writers are `[euid]`. 27 descriptors arrive for 27 paths. Each is the object its path names (dev, ino), except the four identity files, whose scratch went with the observer: those handles hold unlinked files (`nlink` 0). After the package is renamed away and replaced, its handle still holds the checked package. |
| `observer::an_observer_its_broker_did_not_start_hands_nothing_back` | Started by its broker, the observer of an unbound plan sends `{"refused": "unbound"}`. Started by `sh`, so that the socket's maker is not its parent, it exits 1 with no record. With a pipe as stdout it exits 1 and writes nothing. |
| `observer::an_operation_blocked_to_the_deadline_refuses_with_nothing_left_running` | With the observer blocked, the broker answers identity after 30.0 s (asserted 30 s ≤ elapsed < 32 s). The observer and every member of its group are gone, with zero lookups and starts. |
| `observer::a_cancelled_attempt_ends_a_blocked_observation_with_nothing_left_running` | The broker's starter (`sh`) is killed while the observer is blocked. The broker answers identity in under 5 s, its observer's group is gone, the broker ends, and nothing was looked up or started. |
| `observer::a_broker_killed_outright_takes_its_blocked_observer_with_it` | SIGKILL to the broker: the observer's watcher kills the observer's whole group, so the observer and every member of its group, the launcher's version query included, are gone (`ended(pid)`). Any survivor is killed before the verdict. |
| `capability_broker::unprovable_or_unobserved_writers_and_sources_and_exposed_control_roots_refuse` | This test was renamed from `unprovable_writers_unbounded_sources_…`. As sealed, the plan is admitted. Unmapped writers, no writers, or a second uid beside the observed one refuse with identity. So do one more entry, one more mount record, or another digest. |
| `sources::the_observers_bounds_are_mb3s_fixed_counts` | `LIMITS` is exactly `(1_000_000, 64, 40, 65_536)`, and `Host::live()` takes it (L2). |
| `sources::only_the_mount_records_the_walk_stood_on_are_counted_or_digested` | A one-file tree on one mount gives `(2, 1)`. An unrelated record added, or `/proc`'s record removed, moves no sealed fact. Changing the root of the tree's own record moves the digest (L4). |
| `sources::the_box_keeps_its_writers_uids_only_where_their_privilege_is_confined` | Proved credentials keep their uids, and unproved ones keep none. |
| `sources::the_box_mounts_the_observed_object_and_carries_its_facts` | Extended: `mounts` is now compared as a fact of the tree, and the box's `writers()` are the confined credentials' uids. |

The existing four-bound test (`each_bound_refuses_one_over_and_passes_at_its_limit`)
holds each traversal bound at its limit and one under it, with no deadline
involved. Its record count is now `(3, 1)`; the mount bound still applies to
the whole parsed table.

### The blocking fixtures

Each blocking proof runs once per block this host can lay.

The launcher block runs everywhere. A `bwrap` that `exec`s `sleep 600` is
put first on the broker's `PATH`, so the observer waits in the kernel
(`wait4`) on a live child of its own group while it reads the launcher's
version. That child is the "helper child" the group kill must end. This
block is a wait inside the observer's own host-fact read. It is not a delay
between observations.

The stalled-mount block is a FUSE mount whose connection is never
initialised, laid over an empty directory of the package after the plan was
sealed. The walk's attribute read of that directory then waits ('D') until
the observer is killed. **This proof is pending on this host.**
`fusermount3 -- <dir>` answers `fusermount3: mount failed: Permission
denied`, because the seat runs under `no_new_privs`, where the setuid helper
cannot mount. The test reported `skipped: this host mounts no FUSE file
system for this user, …`.

A user-namespace FUSE mount is no substitute. It shows the root-owned
ancestry of every fixture as the overflow uid, which the broker's binding
guard rightly refuses before any observation. The stalled-mount case runs
where `fusermount3` can mount, such as a runner without `no_new_privs`, and a
run that requires boundary evidence fails rather than skips there.

**It has since run in CI.** On the controller's report, CI run 37619579219's
`test (ubuntu-latest)` job passed
`observer::an_operation_blocked_to_the_deadline_refuses_with_nothing_left_running`
and its sibling blocking proofs, with boundary evidence required, so the
stalled-mount case ran there. That is the first attempt's head on the earlier
base, not this commit; this seat cannot read CI logs, and the seat itself still
cannot mount FUSE, so on the final head the stalled-mount case stays pending
until CI runs it.

### Mutations (F3)

Each mutation is a compiling edit at `0f189c02`. Each was run with
`cargo test -p brokkr-protocol --lib hands::tests::sources::<test>` or
`cargo test -p brokkr-cli --test it capability_broker::<test>`, and restored
with `git checkout -- <file>`. Line numbers are `0f189c02`'s.

| Id | Mutation (file:line at `0f189c02`) | Failing test, assertion (file:line at `0f189c02`) | Observed failure |
| --- | --- | --- | --- |
| P1 | `sources.rs:60` `entries: 1_000_001` | `the_observers_bounds_are_mb3s_fixed_counts`, tests/sources.rs:1120 | left `(1000001, 64, 40, 65536)` |
| P2 | `sources.rs:61` `depth: 65` | same, :1120 | left `(1000000, 65, 40, 65536)` |
| P3 | `sources.rs:62` `hops: 41` | same, :1120 | left `(1000000, 64, 41, 65536)` |
| P4 | `sources.rs:63` `mounts: 65_537` | same, :1120 | left `(1000000, 64, 40, 65537)` |
| P5 | `sources.rs:665` `self.records = records.len()` (the whole table) | `only_the_mount_records_…`, tests/sources.rs:1142 | left `(2, 174)`, right `(2, 1)` |
| P6 | `sources.rs:778` `Some(writers.uids.clone())` unconditionally | `the_box_keeps_its_writers_…`, tests/sources.rs:1175 | left `Some([4242])`, right `None` |
| P7 | `namespace.rs:713` `writers()` returns `None` | `the_box_mounts_the_observed_object_…`, tests/sources.rs:1589 | left `None`, right `Some([1000])` |
| C1 | `session.rs:530` `let sources = observed == observed` | `admission_compares_…`, observer.rs:97; and `unprovable_or_unobserved_…`, capability_broker.rs:1431 | the replaced server and `/box/sources/entries` + 1 answer `broker serving protections are incomplete` instead of identity |
| C2 | `session.rs:534` `set(uids) == set(uids)` | `unprovable_or_unobserved_…`, capability_broker.rs:1419 | `[1000, 1001]` admitted |
| C3 | `session.rs:538` the later cause checked before the comparison | `admission_compares_…`, observer.rs:105 | `MCP secret store would be mounted in the server box` instead of identity |
| C4 | `session.rs:738` no `SCM_RIGHTS` pushed | `the_observer_hands_back_…`, observer.rs:158 | left 27 paths, right 0 handles |
| C5 | `session.rs:720` the parent check always holds | `an_observer_its_broker_did_not_start_…`, observer.rs:227 | left `(Some(0), {"refused": 0})`, right `(Some(1), Null)` |
| C6 | `session.rs:51` `STARTUP` 20 s | `an_operation_blocked_to_the_deadline_…`, observer.rs:441 | `Launcher: 20.007445662s` |
| C7 | `session.rs:51` `STARTUP` 35 s | same, :441 | `Launcher: 35.006380129s` |
| C8 | `session.rs:653` kill the observer alone, not its group | same, observer.rs:444 (`ended(pid)`) | `Launcher`: the launcher's child outlived it |
| C9 | `session.rs:561` cancellation never observed | `a_cancelled_attempt_…`, observer.rs:479 | `Launcher: 29.985836271s`, not under 5 s |
| C10 | `session.rs:716` no parent-death signal | `a_broker_killed_outright_…`, observer.rs:504 (`gone(pid)`) | `Launcher`: the observer outlived its broker |
| C11 | `session.rs:78` a rebinding failure coded as identity | `an_observer_its_broker_did_not_start_…`, observer.rs:226 | left `{"refused": 6}`, right `{"refused": 0}` |
| F2 | `session.rs:283` the reserved check skips `TMPDIR` alone, HOME still enforced | `binding_names_are_checked_then_fixed_keys_refuse_before_lookup`, capability_broker.rs:1231 | left `("TMPDIR", … serving protections are incomplete …, 0)`, right `("TMPDIR", … startup inputs are not protected …, 0)`; the earlier HOME iteration passed |

After each restore, the tree matched `0f189c02` (`git status --short crates/`
was empty). There, `hands::tests::sources::` passed 36 tests and
`capability_broker::` passed 27. On the final head, after the helper-only
refactor, they pass 36 and 27 again.

C8 and C10 each left a `sleep 600` (C10 also its observer) that the seat
could not kill. Each ends on its own within ten minutes.

## Task 28.17: the observer's cost on Linux x86_64

The measurement used the consumed observer itself: `brokkr broker observe`
of a sealed plan, run as `serve` runs it. It ran under
`prlimit … setpriv --no-new-privs`, its stdout the socket. The harness was a
scratch test, not committed. It sampled `/proc/<pid>/status` VmHWM and
`/proc/<pid>/fd` every 2 ms, and found the lowest descriptor limit still
admitted by bisection over `prlimit --nofile`. The binary was the test
profile's (debug, unoptimised) build, so the times are an upper bound on a
release build, which was not measured.

The host ran Linux 6.17.0-41-generic on x86_64. `/` is ext4 on LVM
(`/dev/mapper/ubuntu--vg-ubuntu--lv`). The writers were uid 1000, capless
under `no_new_privs`, so the observer proved them confined. Every support
file needing the write-exclusion proof showed no access ACL. The host has 8
root-only system files, listed by the protocol's live positive, and they
were admitted under the 2026-10-07 ruling. Every source was observed
unpruned, with system hard links intact.

| Profile | Entries | Mount records used | Handles | Runs (s) | VmHWM (KiB) | Lowest admitted `nofile` |
| --- | --- | --- | --- | --- | --- | --- |
| singleton system entry (`sh` → `/usr/bin/dash`) | 205,079 | 3 | 26 | 1.339, 1.325, 1.273, 1.291, 1.317, 1.357 | 22,628–23,164 | 33 |
| protected package (`/usr/lib/apparmor/apparmor.systemd`, root `/usr/lib/apparmor`) | 205,083 | 3 | 27 | 1.255, 1.315, 1.387, 1.286, 1.272, 1.338 | 22,336–23,060 | 34 |

Each complete observation took under 1.4 s, inside SD4's 10 s observer
budget, with the 30 s deadline and every bound unchanged. The sampled
descriptor high-water was 30–31.

**Pending:** a cold-cache sample. The first run of each profile came after
earlier runs had warmed the caches, and dropping the page, dentry and inode
caches needs root (`/proc/sys/vm/drop_caches`), which the seat does not hold.
The aarch64 profile is also pending, and it blocks U9b as 28.17 says. 28.17
stays open.

## Carried items

- **L1.** The count-only memory ruling now reaches both texts.
  `specs/slice-two-delivery/spec.md` reads "exceeding an
  entry/depth/mount/symlink count bound, which is the observer's memory
  bound". Task 38.3 shares D5's four count limits with no byte budget.
  SD4's peak-metadata measurement stays, and it is recorded above.
- **L2.** See P1 to P4.
- **L3.** The second copy of the ceilings in session.rs is deleted. See C1 for
  the comparison that replaces it.
- **L4.** See P5 and the churn test. The file-lines rows of every touched file
  were re-measured. `too-many-lines.txt` and `suppressions.txt` name none of
  this unit's functions, and the diff adds no `#[expect]`, `#[allow]` or
  `unsafe`.
- **F2.** See the F2 row above.
- **F3.** See the mutation table, which cites each line at `0f189c02`.

Group 28d of tasks.md now carries U6c5c's dependency and link, and the
ownership this split leaves each unit.

## Gates on this head

Formatting and workspace clippy at `-D warnings` were clean. The suites
passed crate by crate. Core, view, store and bridge passed every target.
The protocol passed 799 unit tests with 3 ignored, and 8 integration tests.
The runtime passed 836 unit tests, the engine's boundary tests among them,
and 199 integration tests across four binaries. The CLI passed 627 unit
tests, 507 `it` tests, 27 driver-conformance tests, its three heap budgets
and 13 transcript-surface tests.

`cargo run -p brokkr-cli -- compile --bundle bundles/self` compiled
(digest `2e085459…`). `openspec validate --all --strict` passed 20 of 20.
`typos --hidden`, `git diff --check` and `git diff --cached --check` printed
nothing. `quality/ratchet.sh files`, `clones` and `api` held: four new
test-file clones were factored into helpers before the last run. The
witness digests passed without blessing, because no witness or compose input
moved. The budgets' inputs (prompts, `Cargo.lock`, transcript heaps) did not
move.

## Coverage

`cargo +nightly-2026-09-05 llvm-cov --branch` was run over the broker tests,
the spawned broker and observer processes included. It measured
`broker/session.rs` at 426/426 lines, 67/67 functions and 2/2 branches, and
`broker.rs` at 33/33 lines, 5/5 functions and 6/6 branches. The protocol
run over `hands::` covered every new line of `namespace.rs` and
`sources.rs`.

That run also found a defect, fixed before `0f189c02`. Killing the observer
as soon as its record arrived cut its own exit short and left a truncated
profile (262,144 bytes), which made `llvm-profdata merge` refuse. The broker
now waits for the socket's end before the kill. The workspace exact-coverage
gate (`scripts/coverage-exact.sh`) was not run here, and it stays pending for
CI.

## Pending and follow-ups

- **The public API.** brokkr-protocol gains `ServerBox::sources`, `handles`
  and `writers`. On `523a1f0a`, `quality/ratchet.sh baselines origin/main`
  reports `public-api/brokkr-protocol.txt: 914 public items (was 911)` and
  refuses until the pull request names the operator's ruling; it awaits that
  ruling at PR time. `quality/ratchet.sh api` holds.
- **macOS.** The `cfg(not(target_os = "linux"))` paths (an `observe` that
  answers unavailable, an in-process record, the test fixture's empty
  observation) are not compiled here, because the darwin standard library is
  not installed. They wait on CI.
- **The launcher's child under a killed broker (carried low, fixed here).** The
  observer's watcher now kills its whole group when the broker ends, so the
  launcher's `--version` child no longer outlives a broker killed outright
  (M1 below). The launcher's own execution through a checked handle stays
  U6c5c's.
- **The attempt's remaining deadline.** The plan carries no such deadline and
  the broker's command line takes none, so admission holds the absolute 30 s
  alone. The remaining deadline needs an input a later unit adds.
- **The record parser.** It is `serde` with `deny_unknown_fields`, which, like
  every serde struct, also accepts a positional array. Only the broker's own
  observer writes the record, over a socket pair the broker made. A cause is
  now the closed `Cause` enum, so an unknown cause name parses as no record
  (identity). No test feeds the broker a forged record: the broker starts
  only its own binary as the observer, so no test can stand in for it without
  a production seam, which this unit does not add.
- **Native alias creation** is not proved by this unit. It stays open under
  28.9.

## Second repair visit (run `0065-slice-two-unit-u6c5b-see-th-8efb7873`)

CI run 37619579219 failed three tests on the first attempt. This visit
answers each failure and the two lows the council carried. Every command below
ran in this seat on the tree this commit holds, unless it says otherwise.

### Failure 1: `a_field_changed_after_sealing_acquires_no_authority` on ubuntu-latest

The `/box/writers/uids` case sealed the literal `[1001]`. GitHub's runner user
is uid 1001, and the fixture seals the observed uid, so on that runner the
"changed" plan was byte-for-byte the sealed one. It bound, and the broker
rightly answered "serving protections are incomplete". The case now seals
`[euid() ^ 1]`, which differs from the observed uid on every host. Each case
in the table also asserts that its change moved the sealed bytes
(`assert_ne!` on the two digests). M3 restores the observed uid and reproduces
the runner's situation on this host. The new assertion fails with identical
digests on the left and right.

### Failure 2: `only_the_protected_inventory_binds_a_plan` on macos-latest

The cause is the fixture, not the order in `session.rs`. `Sealed::observed`
seals the plan, which writes and pins an inventory, before it asks the
observer for sources. Off Linux no observer runs, so the plan keeps its
authored sources. The bytes the test then wrote were the ones that inventory
already pinned. The "no inventory beside the plan" case therefore had an
inventory that pinned that very plan, so it bound and met the box, which is
unavailable off Linux. On Linux the same case refused for an unrelated
reason: the observed sources changed the bytes, so the pinned digest did not
match. No host ever tested a missing inventory.

The order was already MB3's on every host. `admit` binds, then screens, then
observes. The helper itself rebinds before it prepares anything, and
preparing a box needs the bound plan. `session.rs` therefore keeps its order.
The test now removes the inventory before that case. A new
`Sealed::serve_unboxed` puts a `bwrap` that reports 0.4.0, older than
descriptor mounts, first on the broker's `PATH`, so no box can stand on Linux
either. The test asserts that a missing inventory is unbound there too, and,
as the positive control, that a plan pinned exactly answers "MCP server box is
unavailable" where a box would otherwise be admitted.

M4 makes a missing inventory read as pinning the plan:

- The first attempt's fixture still passes under M4, which is the evidence
  that it never tested a missing inventory.
- The new fixture fails under M4 with "serving protections are incomplete".
- With only the unboxed assertion kept, M4 fails with exactly macOS's text,
  "MCP server box is unavailable".

M5 lets an old launcher mount descriptors, which binds the positive control.

### Failure 3: `the_program_tree_is_mb3s_layout_of_the_executable` in the coverage job only

**Not reproduced, and its cause is not established. This item is open.** This
seat cannot read the CI log: `gh` and every path outside the worktree are
refused. It cannot set an environment prefix, a umask or a directory in
`/var/tmp` either, so the commissioned form of the gate cannot run here. What
ran, and what each run excluded:

| Run | Result | Excludes |
| --- | --- | --- |
| `cargo +nightly-2026-09-05 llvm-cov --no-report --branch -p brokkr-cli --test it -- capability_broker::the_program_tree` | passed in 24.8 s | instrumentation alone |
| The same over the whole instrumented `it` binary | 507 passed in 52.3 s | instrumentation with the CLI suite's own concurrency, on this 24-CPU host |
| `bash scripts/coverage-exact.sh`, default `TMPDIR`, boundary evidence not required, so FUSE cases skipped | every suite passed, and lines 51,255/51,255, branches 7,364/7,364, functions 5,903/5,903 | the gate's own target directory, profile names and order; the test "has been running for over 60 seconds" there |
| Broker suite, instrumented, 32 test threads, broker timed by a scratch line since removed | observations 2.0–4.7 s | contention on this host (30 s needs a further 6× slowdown) |
| Package directories made group-writable (umask 002's effect) | passed | umask: sealing records the same facts the broker then observes |

The fixture's only shared mutable state is `/var/tmp` and the one bootstrap
copy, and no test changes either. Profiles are about 650 KiB each. The
leading hypothesis, which is untested: the hosted runner's system set, with
`/usr/local` and `/usr/share` far larger than this host's 205,079 entries,
makes each instrumented observation under four-way contention long enough
that one admission met MB3's absolute 30 s deadline. That gives the identity
cause, while the fixture's own sealing observation has no deadline. Settling
it needs the failing job's log, or one CI run that reports each admission's
elapsed time. Nothing in the fixture was changed for this failure.

### The carried lows

- **The refusal vocabulary at the socket edge** (rulings 2, 3, 8).
  `Record::Refused` and `Observation::refused` now carry `Cause`, a closed
  serde enum of the ten causes the observer can meet. It maps to `Refusal` by
  an exhaustive match and back by `Cause::ALL`, and a cause with none is
  identity. M2 renames the vocabulary and is caught by the record's exact
  later cause (`"store-in-box"`, with no handle beside it).
- **A broker killed outright** (the launcher's child). The parent-death
  signal is gone. It killed the observer alone, and could do so before
  anything else ran. `tethered` now starts a watcher on a close-on-exec
  duplicate of the socket. The broker sends nothing, so the watcher's read
  returns only at the broker's end, and the watcher then kills the observer's
  whole group. M1 parks the watcher before its kill, and the launcher child
  survives.

### Mutations

Each mutation is a compiling edit. Each ran with
`cargo test -p brokkr-cli --locked --test it capability_broker::<test>` and
was then restored by hand. Lines are this commit's.

| Id | Mutation | Failing test, assertion | Observed failure |
| --- | --- | --- | --- |
| M1 | `session.rs:759`: `std::thread::park()` before the group kill | `a_broker_killed_outright_…`, observer.rs:517 `assert!(ended, …)` | `Launcher` (the observer's group outlived its broker) |
| M2 | `session.rs:436`: `rename_all = "snake_case"` | `admission_compares_…`, observer.rs:128 | left `(String("store_in_box"), 0)`, right `(String("store-in-box"), 0)` |
| M3 | capability_broker.rs: the uid case seals `[euid()]` | `a_field_changed_after_sealing_…`, capability_broker.rs:1037 `assert_ne!` | both digests `1212eb98…2159` |
| M4 | `session.rs:241`: a missing inventory reads as pinning this plan at this digest | `only_the_protected_inventory_…`, capability_broker.rs:862 | left `… serving protections are incomplete`, right `… not bound to this attempt`; the first attempt's fixture passed under M4 |
| M4′ | M4, with only the unboxed missing-inventory assertion kept | the same test, capability_broker.rs:863 | left `MCP server box is unavailable`, right `… not bound …` |
| M5 | `sources.rs:768`: any launcher version mounts descriptors | the same test, capability_broker.rs:881 | left `… filesystem identity is not protected`, right `MCP server box is unavailable` |

After the restores, `capability_broker::` passed 27 of 27.

### Gates on this visit's head

Several gates passed: `cargo fmt --all -- --check`, workspace clippy at
`-D warnings`, and `cargo +1.88 check --workspace --all-targets
--all-features`. `quality/ratchet.sh` held for files, clones and api.
`openspec validate --all --strict` passed 20 of 20, `typos --hidden` was
clean, and `cargo run --locked -p brokkr-cli -- compile --bundle bundles/self`
compiled.

The CLI crate passed all its suites: 628 unit tests, 27 driver-conformance,
the three heap budgets, 507 `it` tests and 13 transcript-surface tests. The
protocol's `hands::` passed 85 tests with 2 ignored, and the runtime's
`engine::boundary_tests` passed 41. The coverage gate's run above also ran
every workspace suite green: the protocol's 799 unit tests and the runtime's
865 unit tests and 199 integration tests, the witness digests among them.
Formatting is the only difference between the production code that run
measured and this head.

`quality/ratchet.sh baselines origin/main` refuses only the public-API
growth that awaits the ruling. `scripts/measure-budgets.sh` measured every
prompt site and transcript heap at or under its committed budget; its
rewrite of three budget files reflects main's drift, not this unit, and is
not kept. No witness or compose input moved.

**Pending:**

- The commissioned gate form,
  `TMPDIR=/var/tmp/<owner-only dir> BROKKR_REQUIRE_BOUNDARY_EVIDENCE=1 bash scripts/coverage-exact.sh`.
- The broker suite under umask 002 with an owner-only `TMPDIR`. This seat
  refuses the environment prefix, the umask and the directory.
- The stalled-mount cases on this head.
- macOS's paths, which this host cannot compile.
- Failure 3.

**Re-vouch (run `0065-slice-two-unit-u6c5b-see-th-fe41a3e2`).** The repair
visit stopped as blocked only on the items above, so its work never reached a
council. The controller then pushed `2f0081d7` to PR #581 as a CI diagnostic,
and on the controller's report `test (ubuntu-latest)`, `test (macos-latest)`
and the exact coverage gate all passed there. That closes Failure 3, the
commissioned coverage form and macOS's paths; this seat did not read those
logs. This visit applied the same net diff uncommitted on `523a1f0a`, and
`git diff --cached 2f0081d7` printed nothing, so the head that CI ran is this
tree. A review against the 2026-10-07 row found nothing to fix, and no code
changed. The gates run on this tree all held: formatting, workspace clippy,
`cargo +1.88 check --workspace --all-targets --all-features`, and
`quality/ratchet.sh files`, `clones` and `api`. `quality/ratchet.sh baselines
origin/main` refuses only the ruled rise to 914 public items. Also clean were
`openspec validate --all --strict` (20 of 20), `typos --hidden`, both
`git diff --check` forms and `compile --bundle bundles/self`. The suites
passed too: protocol 799 unit tests, CLI 628 unit and 507 `it` tests, the
runtime's 865 unit tests, and `engine::boundary_tests` 41. `capability_broker::`
passed 27 of 27, but under the seat's inherited `TMPDIR`: the seat refused to
create an owner-only directory outside the worktree, and the worktree's own
ancestry is group-writable. That form therefore rests on the CI diagnostic. The
stalled-mount cases still cannot run in this seat.

A second re-vouch (run `0065-slice-two-unit-u6c5b-see-th-3348b600`) carried
this same net diff onto main at `7fea28e9`, where the only change from
`68d2a5ea` is the operator-ruling file now holding both U6c5b's and U1f2's
2026-10-07 addenda, and on that tree the commissioned gates held again
(`capability_broker::` 27 of 27, still under the inherited `TMPDIR`) with
`quality/ratchet.sh baselines origin/main` refusing only the ruled rise to 914
public items.
