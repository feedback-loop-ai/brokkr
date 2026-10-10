# U6c6a evidence: the private waiting bootstrap and its establishment cause

Runs `0065-slice-two-unit-u6c6a-see-th-17e97236` (three visits, from main
at `ffadca2e`), `0065-slice-two-unit-u6c6a-see-th-e1d30515` (the repair
visit, from main at `52d69f0c`) and `0065-slice-two-unit-u6c6a-see-th-4b3ec000`
(the closed-table repair and its review return, on the same base), branch
`s2/U6c6a`, under the
operator's split of 2026-10-09. Each section's results were observed in the
seat of the visit it names, on that visit's tree; the summary below is the
closed-table tree's, and the earlier visits' sections are history.
Tasks 28.11 and 28.12 stay open: U6c6b owns the box launched on the
bootstrap, the child's identity, the broker's receiver and its controls, the
absolute deadline, cancellation and the measured Linux descriptor carriage.

## What changed, by file

The row's four production files are the only production files touched.

`crates/brokkr-protocol/src/broker.rs` adds `Refusal::Establishment`, "MCP
server box could not be established", directly after `StoreInBox` and
before `ServingIncomplete`, so the derived order keeps MB3's precedence.
It is the unit's one public item (operator ruling 2026-10-09).

`crates/brokkr-cli/src/cli_args.rs` adds the hidden `BrokerCmd::Bootstrap`
beside `Observe`, with `BrokerBootstrapArgs`: `--control` and `--ready`,
each a descriptor number taken as text, with a negative number or no value
at all accepted as text too, so that an absent or malformed one is the
establishment cause, not a usage error. No plan, store, grant or exec
option exists, so clap refuses each as an unexpected argument.

`crates/brokkr-cli/src/broker.rs` registers `mod bootstrap` and dispatches
the variant exhaustively; the handler's cause is the command's error.

`crates/brokkr-cli/src/broker/bootstrap.rs` is the consumed handler. It takes
the two numbers only where `/proc/self/fd` shows each open on an anonymous
pipe (`pipe:[…]`), the two distinct and neither shared with a standard
stream; only then does it own them (one `unsafe` `from_raw_fd`, with its
safety argument). It reads one intent frame (a four-byte big-endian length,
at most 4,096 bytes of a closed `deny_unknown_fields` record: plan and
source-set sha256s, the projected `Network`, and the broker's mnt, pid, net,
ipc and uts namespace inodes). It then observes, apart from deciding: its
own namespaces from `/proc/self/ns`, `/proc/self/mountinfo`, every
environment name, and the directories `HOME` and `TMPDIR` name, read without
following a link. The decision admits the box only where the environment is
exactly `server_environment()`'s names, each once; mnt, pid, ipc and uts
differ from the broker's and net differs exactly where the network is
isolated; each private directory stands outside `/`, `/proc` and `/dev`
and neither at, above nor below a sealed source or the other, and is an
empty `0700` directory of the effective uid and the mount point of exactly
one writable tmpfs, whole (mount root `/`) and on a device no other mount
in the box shows, with nothing mounted below it; the root is bubblewrap's
tmpfs, mount root `/newroot`, alone on its device; `/proc` is exactly one
whole `proc`, and every mount below it shows that same device, and is one of
bubblewrap's four read-only covers (`/sys`, `/sysrq-trigger`, `/irq`,
`/bus`) bound at its own name; `/dev` is exactly one whole `nodev` tmpfs,
alone on its device, and below it stand only `devpts` at `/dev/pts`,
`mqueue` at `/dev/mqueue`, each whole, and `devtmpfs` binds of bubblewrap's
six host nodes (`/null`, `/zero`, `/full`, `/random`, `/urandom`, `/tty`),
each at its own name; each mount the intent seals lies outside `/`,
`/proc` and `/dev` and stands once, read-only, at its path with the sealed
device and inode; and nothing else stands anywhere, nested and read-only
mounts included. Every mount is judged by the first row of the closed table
its place meets, `/proc` and `/dev` before any source or private
directory. Being read-only explains no mount. The environment it checks is
the block
`/proc/self/environ` keeps of the one the process was started with, which
is what its loader read. Then it measures the ready message (plan and
source-set digests, its pid in the box, the namespaces it observed) against
the 4,096-byte bound, writes it on the ready pipe and closes it, and waits
on the control pipe: end of file is still SD3's incomplete-serving cause,
since no binding channel stands until U6c8, and any byte after the frame is
the establishment cause. It runs nothing.

## Tests and moves

Two owning places, no move. `crates/brokkr-cli/tests/capability_broker.rs`
(already registered in `tests/it.rs`) gains the real-binary tests and the
new cause in `each_refusal_reads_in_mb3_and_mb4s_words`. The pure decision
and the framing are unit tests in the new
`crates/brokkr-cli/src/broker/bootstrap/tests.rs`, registered as
`#[cfg(all(test, target_os = "linux"))] mod tests;`: the integration binary
cannot reach a private module, no box may be simulated in a test, and off
Linux no `/proc` names a descriptor, so the bootstrap refuses before its
pipes are taken. The unit tests drive `run_with` on real anonymous pipes with
the box's facts stood in, and `a_host_is_no_box` drives the live observer.

## Removal controls, first visit (historical)

Each mutation below compiled, made the named test fail at the named
assertion, and was restored; the suite passed again after each restore.
This table and the second and third visits' are history: they were run on
those visits' trees. The test they call
`every_other_mount_is_read_only_the_root_a_tmpfs_and_proc_and_dev_the_kernels`
is today's `each_source_is_read_only_the_root_bubblewraps_tmpfs_and_proc_and_dev_the_kernels`,
and the `Place::Procfs`, `Place::Proc` and `Place::Dev` rules they bound
were replaced in the repair visit; the controls for today's mount rules are
that visit's.

| Obligation | Mutation | Failing test and assertion |
| --- | --- | --- |
| Shared refusal order | `Establishment` moved after `ServingIncomplete` | `capability_broker::each_refusal_reads_in_mb3_and_mb4s_words`, `precedence.is_sorted()` |
| Real-CLI invalid context | `pipe`'s number check answers `Refusal::Unavailable` | `capability_broker::the_bootstrap_without_its_private_control_context_is_not_established`: left `error: MCP server box is unavailable`, right `error: MCP server box could not be established` |
| Real-CLI host is no box | `decide` bypassed in `ready` | `capability_broker::neither_a_host_nor_stdout_nor_a_marker_carries_readiness`, case `4>&1 1>&2`: the real binary wrote the ready JSON on descriptor 4 and ended "broker serving protections are incomplete" |
| Intent bound | `length > FRAME_MAX` weakened to `length > usize::MAX - 1` | `the_intent_is_one_closed_frame_of_at_most_four_kibibytes`, case `over` |
| Ready shape | `plan` filled from `sources` | `an_admitted_box_writes_one_closed_ready_message_and_still_refuses_when_released`, `a_shared_network_is_the_brokers_and_an_isolated_one_is_not` and `a_byte_after_the_frame_is_no_release`, each at its `ready_message` equality |
| Namespace | the mnt pair dropped from `separate` | `each_namespace_the_box_shares_with_the_broker_refuses`, case `mnt` |
| Network | `Network::Isolated` arm answers `true` | `a_shared_network_is_the_brokers_and_an_isolated_one_is_not`, the isolated-with-the-broker's-net case |
| Mounts | `Place::Elsewhere` admits anything | `every_other_mount_is_read_only_the_root_a_tmpfs_and_proc_and_dev_the_kernels`, case `source` |
| Private tmpfs | the one-writable-tmpfs check (`alone`) dropped | `each_private_directory_is_its_own_writable_tmpfs_with_nothing_below`, case `read-only` |
| Private directories | the mode, owner and emptiness check (`fresh`) dropped | `each_private_directory_is_a_distinct_fresh_one_only_its_owner_enters`, case `open` |
| Fixed environment names | `fixed` answers `true` | `any_environment_beside_the_fixed_names_refuses`, case `loader` (`LD_PRELOAD`) |
| No stdout readiness | the standard-stream check dropped | `readiness_leaves_on_a_distinct_private_pipe_and_no_other_descriptor`, the stdout assertion |
| No marker readiness | the anonymous-pipe filter dropped | the same test, `run_with(&marked, admitted)` answered `ServingIncomplete`, not `Establishment` |
| No release by a stray byte | `Ok(0)` widened to `Ok(_)` | `a_byte_after_the_frame_is_no_release`, `refusal == Establishment` |

The namespace, environment, mount and private-directory removals were run
again after their case loops were folded into `refused_each`; each failed
its own test there too (`refused_each`'s assertion). On the host, the
real-binary cases refuse at several checks at once, so no single check's
removal turns them; their independent binding is the unit tests above.

## Second visit: the review's return (historical)

The review (`gpt-6-astra`, residual, medium) returned four findings this
unit could answer inside its files, and one it cannot.

The host loader environment. Three things are distinct. U6c5c's evidence
covers the launcher running `--version` on the host, admitted by its ELF
headers before it runs. U6c6b owns the host bubblewrap launch, which opens
with `--clearenv` and only the fixed table (`hands/namespace.rs`), and the
proof that no binding reaches it. What this unit owns is the bootstrap's own
start: it now reads the environment from `/proc/self/environ`, the
exec-time block its dynamic loader read, not the process's later view, so an
entry without `=` or a name given twice is seen as it was handed over, and
refuses unless that block is exactly the fixed names.
`the_environment_is_the_one_this_process_was_started_with` drives the live
observer: the test process's own start environment holds
`CARGO_MANIFEST_DIR`, which the fixed table does not, so `fixed` refuses
it; and the block parser keeps `LD_PRELOAD` (no `=`) as a name. Values
are still not compared: the fixed values live in a private protocol table,
and reading them here would be a second public item the operator's ruling
does not allow. U6c8's exec consumer owns the exact values and cwd.

The ready bound. A production guard now measures the message:
`message` refuses with the establishment cause when the serialised ready
message exceeds `FRAME_MAX`, before any byte is written.
`a_ready_message_is_written_only_within_the_bound` pins the largest real
message at exactly 319 bytes and a message whose plan is 4,096 bytes at
`Err(Establishment)`.

Malformed descriptor forms. `--control -1`, `--ready -1`, a bare
`--control` or `--ready`, `--control --ready 4` and `--control=` were usage
errors (exit 2) and are now the establishment cause, exit 1, through the
real binary.

Mount provenance, as this visit left it (superseded under `/proc` and
`/dev` by the repair visit, which admits no mount there for being
read-only). Each private directory must be a whole tmpfs on its own
device, the root a whole tmpfs, a writable proc only `/proc` itself, and a
writable host `devtmpfs` under `/dev` only one of the six named nodes. The
negative controls are the `part`, `seen` and `rooted` private cases and the
`root part`, `procfs part`, `proc below`, `dev part`, `devtmpfs`, `node`
and `renamed` cases.

Not answered here. Parsing mountinfo a second time
(`bootstrap.rs`'s `Record::parse` beside `hands/namespace/sources/host.rs`'s
`records`, which is `pub(in crate::hands)` in brokkr-protocol) stays a LOW
under ruling 5: sharing that parser needs a public protocol item beyond the
one ruled. A devpts or tmpfs bound whole from the host still reads as the
box's own, since mountinfo shows no difference; that is the residual limit
of reading the box from inside it.

Each mutation below compiled, made the named test fail at the named
assertion, and was restored; the bootstrap's 14 unit tests passed after
the last restore. Unrelated mutations were run together only where each
named a different test, and every test failed on its own case.

| Obligation | Mutation | Failing test and assertion |
| --- | --- | --- |
| Ready bound | `bytes.len() <= FRAME_MAX` widened to `<= usize::MAX` | `a_ready_message_is_written_only_within_the_bound`: left `Ok(4351)`, right `Err(Establishment)` |
| Start environment | `observe` reports `server_environment()`'s names | `the_environment_is_the_one_this_process_was_started_with`, the `CARGO_MANIFEST_DIR` assertion |
| Unusable ready descriptor | the ready write's failure ignored (`.ok()`) | `absent_or_unusable_control_context_refuses`, the `backwards` assertion: left `ServingIncomplete`, right `Establishment` |
| Host is no box (unit) | `decide` bypassed in `ready` | `a_host_is_no_box`: left `ServingIncomplete`, right `Establishment` |
| Whole private tmpfs | `record.root == "/"` dropped from `fresh` | `each_private_directory_is_its_own_writable_tmpfs_with_nothing_below`, case `part` |
| Private device seen once | `shared == 1` dropped from `fresh` | the same test, case `seen` |
| Whole root | `whole` dropped from `Place::Root` | `every_other_mount_is_read_only_the_root_a_tmpfs_and_proc_and_dev_the_kernels`, case `root part` |
| `/proc` whole | `whole` dropped from `Place::Procfs` | the same test, case `procfs part` |
| Nothing writable below `/proc` | `Place::Proc` admits a writable proc | the same test, case `proc below` |
| Whole box file systems under `/dev` | `whole` dropped from `Place::Dev` | the same test, case `dev part` |
| Named host nodes | `NODES.contains(&self.root)` replaced by `!NODES.is_empty()` | the same test, case `devtmpfs` |
| Negative descriptor | `allow_negative_numbers` dropped from `--control` | `capability_broker::the_bootstrap_without_its_private_control_context_is_not_established`, case `--control -1 --ready 4`: left exit 2 "unexpected argument '-1' found" |
| Valueless descriptor | `num_args = 0..=1` dropped from `--control` | the same test, case `--control`: left exit 2 "a value is required for '--control <CONTROL>'" |

A first draft also required a node bound at its own name; with the closed
node list on the mount root, that condition's removal left every test
passing, since a listed node at another name grants nothing. It was dropped
rather than kept unbound. The repair visit restores that condition, bound
by its own `node swapped` case.

## Third visit: mounts against the intent (historical)

The second review (`gpt-6-astra`, residual, medium) returned two medium
findings, both answered here inside `bootstrap.rs`, and kept three lows.

M1, bubblewrap's root. The review observed a live bubblewrap 0.11
namespace whose root mount reads
`0:90 /newroot / rw,nosuid,nodev,relatime - tmpfs tmpfs …`: bubblewrap
pivots into a `newroot` directory of its own tmpfs, so the root mount's
root is `/newroot`, never `/`. The previous predicate (a whole tmpfs)
refused every real box. The root is now admitted only as a tmpfs whose
mount root is `BUBBLEWRAP_ROOT` (`/newroot`), the only mount at `/`, on a
device no other mount in the box shows. The fixture's root line is the
observed shape. Controls: the `root` case (ext4), `whole root` (a tmpfs at
`/` with mount root `/`, the shape the first predicate wanted) and
`root seen` (the root's device shown again at `/usr`). This seat could not
launch bubblewrap itself (the command needs an approval it does not hold),
so the observed line is the review's.

M2, mounts against the intent. The intent now carries `mounts`, a closed
list of every mount the broker binds into the box, nested ones each their
own: a path in the box, the source's device (`major`, `minor`) and inode,
the same identity the broker's source observer keeps
(`hands/namespace/sources.rs`'s `((major, minor), ino)`). The observer
stats each listed path without following a link, and the decision admits
the box only where each path shows exactly its sealed device and inode, one
observed identity per sealed mount; each sealed path holds exactly one
mount; each such mount is read-only; and every mount outside the root,
private directories, `/proc` and `/dev` is one the intent seals. A mount
nothing explains is refused, read-only or not. Mount points are read with
mountinfo's octal escapes decoded, so a sealed package at a path holding a
space matches its record; the fixture holds one (`/opt/docs mcp`). The
integration test's intent frame gains `"mounts": []` so that it is still a
valid intent and the host refuses on what it shows, not on parsing.

Two limits stand. Device and inode come from `stat` at the sealed path,
not mountinfo's `major:minor`, which differs from `stat` on btrfs
subvolumes; the mount's own root and subpath are not compared, since the
bootstrap would need the broker's mountinfo root for each source and the
inode already names the object shown. Whether the root and the private
tmpfs are new instances rather than host tmpfs mounted whole still cannot
be shown from inside the box; `alone`'s comment now says so and names the
launcher (U6c6b) as where it is shown (L2).

Kept lows. L1: the second mountinfo parser, now with its own escape
decoder, beside `hands/namespace/sources/host.rs`'s `records` and
`unescaped`, which are `pub(in crate::hands)`; sharing them needs a public
protocol item beyond the one ruled. L2 as above. L3 concerns the review
panel's own wording and needs nothing from this unit.

Each mutation below compiled, made the named test fail at the named
case, and was restored; the bootstrap's 16 unit tests passed after the last
restore.

| Obligation | Mutation | Failing test and case |
| --- | --- | --- |
| Bubblewrap's root (M1) | the root predicate set back to mount root `/` | `an_admitted_box_writes_one_closed_ready_message_and_still_refuses_when_released` (left `Establishment`, right `ServingIncomplete`), the other three admitted-box tests, and `each_source_is_read_only_the_root_bubblewraps_tmpfs_and_proc_and_dev_the_kernels`, case `whole root` |
| Root a tmpfs alone | the root read by `only` instead of `alone` | `each_source_is_read_only_…`, case `root` |
| Device seen once | `shared == 1` widened to `shared > 0` in `alone` | the same test, case `root seen`; `each_private_directory_is_its_own_writable_tmpfs_with_nothing_below`, case `seen` |
| Sealed identity (M2, changed) | `bound` admits any seen identity | `each_sealed_source_and_nothing_else_stands_where_the_intent_puts_it`, case `inode` |
| Sealed device | `bound` compares the inode alone | the same test, case `device` |
| One identity per sealed mount | `sources.len() == seen.len()` widened to `>=` | the same test, case `short` |
| Each sealed path mounted (M2, missing) | `sealed` drops `only` | the same test, case `missing` |
| Exactly one mount at a sealed path | `only` returns the first of several | the same test, case `stacked` |
| Nothing unsealed (M2, unexpected) | `Place::Elsewhere` admitted when read-only, as before | the same test, case `unexpected` |
| Escapes decoded | the mount point taken undecoded | the four admitted-box tests (`Establishment` for `ServingIncomplete`) and `each_source_is_read_only_…`'s kept `/proc/sys` assertion (that permissive assertion is gone since the repair visit; the four admitted-box tests still bind the decoding) |
| Identity without following a link | `symlink_metadata` replaced by `metadata` in `identity` | `a_sealed_source_is_seen_by_its_own_device_and_inode_where_it_stands`: left inode 97046027, right 97046028 |
| Device halves | `major` and `minor` swapped in `identity` | the same test: left `(48, 0)`, right `(0, 48)` |

## Repair visit: `/proc` and `/dev` are the box's own (H1)

The council held the third visit's tree on one high finding: `admitted`
let any read-only mount stand at or below `/proc` or under `/dev`, so a
read-only ext4 bind at `/proc/sys`, a read-only host `devtmpfs` at `/dev`,
or a read-only node outside the list could be answered with a ready message
instead of the establishment cause, and the test file asserted the ext4
case as admitted. The repair is inside `bootstrap.rs` and its tests; the
other three production files, the cause's text and precedence, readiness,
the waiting and the incomplete-serving release are unchanged.

`Record::parse` now also reads `nodev` from the mount's own options. A new
`proc` check requires exactly one whole `proc` at `/proc` and every mount
below it to show that mount's device and type, so neither another file
system nor a host's proc bound there passes. `/dev` is read like the root
and the private directories, by `alone`: exactly one tmpfs there, mount
root `/`, `nodev`, on a device nothing else shows; the host's whole
`devtmpfs` is not a tmpfs, and its device is the nodes' own. In
`admitted`, `Place` gains `Devfs` for `/dev` itself; below `/proc` only a
read-only cover from `COVERS` bound at its own name stands, and below
`/dev` only a whole `devpts` or `mqueue` at its place in `KERNEL`, or a
`devtmpfs` node from `NODES` bound at its own name (`shows` compares the
mount root with the point, so a node or cover cannot be renamed). The
layout is bubblewrap's `--proc` and `--dev`, which `hands/namespace.rs`
asks for; it mounts no `--mqueue`, but the commission admits one at its own
place. The test fixture's `/dev` gains the `nodev` bubblewrap sets, and the
admitted table adds a second cover (`/proc/bus`), a second node
(`/dev/zero`) and `mqueue`, so the admitted-box tests prove the whole
allowed layout. `DEVICES` is gone; `PROC`, `DEV`, `COVERS` and `KERNEL` are
new private constants. No public item moved.

The permissive assertion is replaced by
`proc_and_dev_are_the_box_own_and_being_read_only_explains_no_mount`, eight
cases each asserted as exactly `(Refusal::Establishment, [])` (no ready
byte) through `refused_each`. Each mutation below compiled, made that test
fail at the named case with `ServingIncomplete` and a ready message on the
left, and was restored; the 17 bootstrap unit tests passed after the last
restore. Where a rejection meets two checks, the mutation removing the
check it binds is named; the first visit's restore of the old arms is not
needed as a control, since each new rule has its own.

| Case | Mount | Mutation | Other test failing too |
| --- | --- | --- | --- |
| `ext4 at proc/sys` | `8:1 /sys /proc/sys ro … ext4` | `proc`'s per-mount check below `/proc` replaced by `true` | none |
| `host proc at proc/sys` | `0:9 /sys /proc/sys ro … proc` | the device equality dropped from that check | none |
| `uncovered` | `0:41 /kcore /proc/kcore ro … proc`, added | `Place::Proc`'s cover check replaced by `true` | none |
| `host devtmpfs at dev` | `0:5 / /dev ro,nosuid,nodev … devtmpfs` | the `dev` conjunct of `mounted` replaced by `true` | `each_source_is_read_only_…`, case `dev part` |
| `dev without nodev` | `0:42 / /dev rw,nosuid … tmpfs` | `dev.nodev` replaced by `true` | none |
| `node outside NODES` | `0:5 /sda /dev/sda ro,nosuid,nodev … devtmpfs`, added | the `NODES` check replaced by `true` | `each_source_is_read_only_…`, case `devtmpfs` |
| `node swapped` | `0:5 /zero /dev/null … devtmpfs` | `shows`' point comparison replaced by `true` | none |
| `terminals elsewhere` | `0:43 / /dev/shm … devpts` | `KERNEL`'s place comparison replaced by `true` | none |

The earlier refusals stay bound by the same test file: `procfs part`,
`proc below`, `dev part`, `devtmpfs`, `node` and `renamed` in
`each_source_is_read_only_the_root_bubblewraps_tmpfs_and_proc_and_dev_the_kernels`
pass on the repaired tree.

Complexity was measured with `cargo crap --path crates/brokkr-cli/src
--format json`: the highest cyclomatic complexity in `bootstrap.rs` is
`place` at 9, then `Record::parse` at 8 and `Record::admitted` at 6; every
function is within 15. Coverage was audited by reading the changed lines
against the tests, not measured: every new line is straight-line code or a
closure, the admitted box runs each of them (the `proc` closure, `COVERS`,
`KERNEL` and `NODES` closures and `shows` all run on the fixture's covers,
terminals, queues and nodes), each new `Place` arm runs on the fixture,
and no new error arm, `Display` text or 64-bit-unreachable closure was
added. The exact gate's measure on this tree is pending.

Limits carried to U6c6b (L2), not shown from inside the box: whether a
tmpfs, `devpts` or `mqueue` is a fresh instance rather than a host one
bound whole, since mountinfo reads them alike; that the proc at `/proc`
belongs to the box's own PID namespace, not another's proc of a fresh
mount; and that the environment the loader read was the fixed one before
the loader ran, since `/proc/self/environ` is read only afterwards. A box
with a console bound at `/dev/console` (bubblewrap binds one when its stdin
is a terminal) is refused: the server's stdin is the broker's pipe, so that
is assumed never to occur. A host whose `/dev` nodes are not on a
`devtmpfs` likewise refuses every box. The L1 follow-up is unchanged: the
second mountinfo parser and escape decoder in `bootstrap.rs` beside
brokkr-protocol's private ones in `hands/namespace/sources/host.rs` stay a
ruling-5 low, since sharing them needs a public protocol item beyond the
one the operator ruled.

## Closed-table repair: every mount meets one row (H2)

The council held the repair visit's tree on H2: `place` classified a path
the environment chose for `HOME` or `TMPDIR` as private before it looked at
`/proc` or `/dev`, and `admitted` took a private place unconditionally, so a
perfect private tmpfs moved to `/dev/unsealed` got a ready message. This
visit writes out the commission's closed table inside `bootstrap.rs` and its
unit tests. The other three production files, the cause's text and
precedence, the bounded private readiness, the waiting and both fences are
unchanged, and no public item moved.

`place` now meets the rows in the table's order: `/proc` and below it,
`/dev` and below it, `/`, a sealed source, a private directory, below a
private directory, and anywhere else. The reserved trees therefore win
before either exemption is consulted. Before a private path is trusted, the
new `placed` requires each to be `outside` (not `/`, nor at or below `/proc`
or `/dev`) and apart from every sealed source and from the other private
directory, in both directions (neither at, above nor below). This also
replaces the old sort-and-dedup distinctness check. The same `outside` holds
every sealed source to row 4. `/dev`'s check moved into its own `dev`
function, beside `proc`. The tmpfs test moved out of `alone` into a
`Record::tmpfs(root)` each caller names, so the root, `/dev` and the private
directories each carry their own type check. `proc` compares only devices
below `/proc`, because one device is one file system; `/proc`'s own type is
checked once, on the mount at `/proc`.

Two tests are new in `crates/brokkr-cli/src/broker/bootstrap/tests.rs`, both
built on a new `rehomed(path, parent)` fixture that moves the admitted
`HOME` tmpfs, still whole, writable, alone on its device, empty, `0700` and
the owner's, to another path. `refused_each` now gathers every case that is
not exactly `(Refusal::Establishment, [])` and asserts that list is empty,
so a mutation's failure names each case it let through. The table gives
each mutation, its compiling removal (restored after the run) and the
cases that failed under it, read from `cargo test -p brokkr-cli
--all-features --locked --lib broker::bootstrap`. After the last restore,
all 19 bootstrap tests passed.

| Control (case) | Removal | Cases admitted, test |
| --- | --- | --- |
| private tmpfs at `/dev/unsealed` (`in dev`), H2 itself | `outside`'s reserved test `& false`, and `place`'s private arms moved first (the H2 order) | `in dev` (`a_private_directory_…`); `every_mount_…` too, by `outside` |
| private tmpfs at `/proc/x` (`in proc`) | the above, and `proc`'s device test `\| true` | `in dev`, `in proc`; `host proc at proc/sys`; the source in `/proc` |
| private tmpfs at `/` (`at the root`, the root a whole tmpfs) | `placed` `\| true`, and the root's `/newroot` test dropped (`root.tmpfs(root.root)`) | `at the root`, `in a source`, `over a source`, `in the other`; `whole root` |
| private tmpfs inside a source (`/usr/x`) | `apart`'s `!one.starts_with(other)` dropped | `in a source` only |
| private tmpfs containing a source (`/opt`) | `apart`'s `!other.starts_with(one)` dropped | `over a source` only |
| private tmpfs nested in the other (`/tmp/x`) | the other private directory dropped from `placed` (`others.take(0)`) | `in the other` only |
| ext4 `/` on a unique device (`root not tmpfs`) | the root's `tmpfs` replaced by `root.root == BUBBLEWRAP_ROOT` | `root not tmpfs` only |
| tmpfs at `/proc` on proc's device (`proc not proc`) | `procfs.fstype == "proc"` dropped | `proc not proc` only |
| tmpfs at `/dev/pts` (`kernel fstype`) | `KERNEL`'s fstype comparison dropped | `kernel fstype` only |
| a second tmpfs at `/dev` (`second dev`) | `dev`'s `alone` replaced by the first mount at `/dev` | `second dev` only |
| a source sealed at `/proc/irq`, over a cover there | `outside` dropped from the sealed check | that assertion in `every_mount_…` only |
| `/dev/sda` node (`node outside NODES`) | the `NODES` membership replaced by `self.shows(self.root, DEV)` | `node outside NODES`; `node` |
| `/dev` without `nodev` (`dev without nodev`) | `& dev.nodev` dropped | `dev without nodev` only |
| `/usr` mounted writable (`source`) | `Place::Source => true` | `source`, `nested` |

The placement guard is layered on purpose. Removing `outside`'s reserved
test alone let only the source sealed in `/proc` through: `in dev` and
`in proc` stayed refused by `place`'s order (and `in proc` by `proc`'s
device test too). Restoring H2's order alone, with `placed` in place,
failed nothing (19 passed). The private-at-`/` case cannot change one fact
only, because row 3 wants `/newroot` where row 5 wants a whole tmpfs. Its
removal therefore takes both rows' checks.

Complexity, from `cargo crap --path crates/brokkr-cli/src --format json`:
the highest in `bootstrap.rs` is still `place` at 9, then `Record::parse`
at 8. Each new or changed function (`placed`, `outside`, `dev`, `private`,
`mounted`, `proc`, `Record::tmpfs`) is at 1, and `alone` is at 2. Coverage
was audited by reading, not measured. Every new line is straight-line code
or a closure that the admitted box runs (`placed`'s per-directory closure,
`apart` and both chained iterators, and `outside` for each source and
directory), with `&` rather than short-circuiting operators, as the file
already did. No error arm or `Display` text was added. The exact gate is
pending.

L1 (the second mountinfo parser beside `hands/namespace/sources/host.rs`)
and L2 (fresh-instance provenance, proc's PID namespace, the pre-loader
environment) stay as recorded above: L1 is a follow-up beyond the row and
its ruled API, and L2 belongs to U6c6b.

## Review return: each placement check binds alone (M1)

The same run's review held the closed-table tree on one medium finding: the
end-to-end cases for a private tmpfs at `/dev/x`, at `/proc/x` and at `/`
are refused by more than one check, so no single removal of the placement
guard or of `place`'s order failed them. This visit changes no production
line. `bootstrap.rs` is byte-for-byte the closed-table tree's, every guard
in place, and the end-to-end cases stay. Three focused unit tests are added
to `crates/brokkr-cli/src/broker/bootstrap/tests.rs`, each asserting exact
values on one function. A `directory(path)` builder is lifted out of
`admitted()` for them, a `sealed()` builder gives the `SEALED` mounts as
sources, and a test-only `row(Place)` names each row by an exhaustive match,
so no derive is added to the production enum.

`proc_dev_and_the_root_are_judged_before_a_private_or_source_exemption`
puts a private directory, and then also a sealed source, at each of `/proc`,
`/proc/x`, `/dev`, `/dev/x`, `/`, `/usr` and `/tmp`, and asserts the exact
row `place` gives each: `procfs`, `proc`, `devfs`, `dev`, `root`, then
`source` with the sources and `private` without; `/tmp/x` is `below`.
`only_a_path_beyond_the_root_proc_and_dev_is_outside` asserts `outside` on
`/`, `/proc`, `/proc/x`, `/dev`, `/dev/x`, `/devices` and `/tmp` is exactly
`[false, false, false, false, false, true, true]`.
`a_private_path_is_placed_only_outside_and_apart` asserts `placed` for
`HOME` at `/runtime/home`, `/dev/unsealed`, `/proc/x`, `/`, `/usr/x`,
`/opt` and `/tmp/x`, with `TMPDIR` at `/tmp` and the sealed sources, is
exactly `[true, false, false, false, false, false, false]`.

Each removal below is one compiling edit to `bootstrap.rs`. It was run with
`cargo test -p brokkr-cli --all-features --locked --lib broker::bootstrap`,
then restored. After the last restore, `git checkout` of the file left
`git status` showing only the test file, and all 22 bootstrap tests passed.

| Removal | Failed, with the observed left side |
| --- | --- |
| `outside`'s reserved trees `[DEV]` (no `/proc`) | `only_a_path_…_outside`: `[false, true, true, false, false, true, true]`; `a_private_path_is_placed…`: `[true, false, true, …]`; and the source sealed at `/proc/irq` in `every_mount_…` |
| `outside`'s reserved trees `[PROC]` (no `/dev`) | `only_a_path_…_outside`: `[…, true, true, true, true]` at `/dev` and `/dev/x`; `a_private_path_is_placed…`: `[true, true, false, …]` |
| `outside`'s `& (path != Path::new("/"))` dropped | `only_a_path_…_outside` only: `[true, false, …]` |
| `placed`'s `outside(&dir.path) &` dropped | `a_private_path_is_placed…` only: `[true, true, true, false, …]` |
| a private arm put first in `place` (the H2 order) | `proc_dev_and_the_root_…`: every row `private` |
| the source arm put first in `place` | `proc_dev_and_the_root_…`: every row `source` |
| the private arm put after `/dev`, before the root | `proc_dev_and_the_root_…`: `["procfs", "proc", "devfs", "dev", "private", "private", "private"]` |
| `apart`'s `!one.starts_with(other)` dropped | `a_private_path_is_placed…` at `/usr/x`, and `in a source` end to end |
| `apart`'s `!other.starts_with(one)` dropped | `a_private_path_is_placed…` at `/opt`, and `over a source` end to end |
| the other private directory dropped (`others.take(0)`) | `a_private_path_is_placed…` at `/tmp/x`, and `in the other` end to end |

`outside`'s root test is the one check `placed` also gets from `apart`,
since `/` contains every other path. Removing it leaves `placed` false at
`/`, so the `outside` test alone binds it. No production function changed,
so the complexity figures recorded above stand. The tests file grew from
672 to 753 lines, under the 2,000-line ceiling, and its
`quality/file-lines.txt` row was set from `wc -l`.

## Gates on this tree

The M1 tree. `cargo test -p brokkr-cli --all-features --locked` passed every
target. The lib passed 649 with 1 ignored, and its 22 bootstrap tests are
among them. `it` passed 522 with 2 ignored, `driver_conformance` 27, the
three heap budgets 1 each, and `transcript_surfaces` 13. brokkr-protocol did
not change. Format, workspace clippy with `-D warnings` and `--locked`, and
`cargo +1.88 check --workspace --all-targets --all-features` were clean.
`typos --hidden` and `git diff --check` were clean, and `openspec validate
--all --strict` passed 20 of 20. `quality/ratchet.sh files`, `clones` and
`api` held. `baselines origin/main` refused only
`public-api/brokkr-protocol.txt: 945 public items (was 944)`, the ruled
rise. The seat was refused approval both to create an owner-only TMPDIR
under `/var/tmp` and to run lychee. Those two, the workspace suite as one
command, the exact-coverage gate and CI are pending.

The closed-table tree. `cargo test -p brokkr-cli --all-features --locked`
passed every target: lib 646 and 1 ignored, `it` 522 and 2 ignored,
`driver_conformance` 27, the three heap budgets and `transcript_surfaces`
13. `--test it capability_broker::` alone passed 33 under the default
TMPDIR. The seat may not create an owner-only directory outside the
worktree, so that rerun is pending. `cargo test -p brokkr-protocol
--all-features --locked` passed (lib 842 and 3 ignored, and its other
targets). Format, workspace clippy with `-D warnings` and `--locked`, and
`cargo +1.88 check --workspace --all-targets --all-features` were clean.
`typos --hidden` and `git diff --check` (staged and unstaged) were clean,
`openspec validate --all --strict` passed 20 of 20, and `recipes/self`
compiled. `quality/ratchet.sh files`, `clones` and `api` held. `baselines
origin/main` refused only `public-api/brokkr-protocol.txt: 945 public items
(was 944)`, the ruled rise. `quality/file-lines.txt` holds `bootstrap.rs` at
647 and its tests at 672, from `wc -l`. Still pending: lychee (the seat may
not run it), the owner-only-TMPDIR rerun, the workspace suite as one
command, the exact-coverage gate and CI.

The repair visit's tree. Bootstrap's unit tests passed 17; `cargo test
-p brokkr-cli --tests` passed every target (lib 644 and 1 ignored, `it`
522 and 2 ignored, `driver_conformance` 27, the three heap budgets,
`transcript_surfaces` 13); `--test it capability_broker::` alone passed 33
under the default TMPDIR, since the seat refuses the environment prefix an
owner-only TMPDIR needs, so that run is pending. `cargo test -p
brokkr-protocol` passed (lib 842 and 3 ignored, its other targets). Format,
workspace clippy with `-D warnings` and `--locked`, `cargo +1.88 check
--workspace --all-targets --all-features`, `typos --hidden` and `git diff
--check` were clean; `openspec validate --all --strict` passed 20 of 20;
`recipes/self` compiled. `quality/ratchet.sh files`, `clones` and `api`
held; `baselines origin/main` refused only `public-api/brokkr-protocol.txt:
945 public items (was 944)`, the ruled rise, which passes once the pull
request names its `Ruling:`. `quality/file-lines.txt` holds `bootstrap.rs`
at 609 and its tests at 617, from `wc -l`. Pending: the workspace suite as
one command on the final head, the owner-only-TMPDIR rerun, lychee (the
seat may not run it), the host
validation, the exact-coverage gate and CI's complexity ratchet.

The third visit's tree. `cargo test -p brokkr-cli --all-features --locked`
passed every target: lib 643 passed and 1 ignored; `it` 519 passed and 2
ignored, with `capability_broker::` alone 33 passed under the seat's own
TMPDIR (it could not create an owner-only one outside the worktree);
`driver_conformance` 27; the three heap budgets; `transcript_surfaces` 13.
Formatting, workspace clippy with `-D warnings`, `cargo +1.88 check
--workspace --all-targets --all-features`, `typos --hidden` and
`git diff --check` were clean; `openspec validate --all --strict` passed 20
of 20. `quality/file-lines.txt` was updated by hand from `wc -l` for
`bootstrap.rs` (557, under 800), its tests (571) and `capability_broker.rs`
(1,663). brokkr-protocol did not change in this visit. `quality/ratchet.sh`,
lychee and the exact-coverage gate needed approvals this seat lacked and
are pending for this tree.

The second visit's tree. `cargo test -p brokkr-cli --all-features --locked`
passed every target: lib 641 passed and 1 ignored; `it` 519 passed and 2
ignored, `capability_broker::` alone 33; `driver_conformance` 27; the
three heap budgets; `transcript_surfaces` 13. Formatting, workspace clippy
with `-D warnings` and `cargo +1.88 check --workspace --all-targets
--all-features` were clean. `quality/ratchet.sh files`, `clones` and `api`
held. `quality/ratchet.sh baselines origin/main` refused only
`public-api/brokkr-protocol.txt: 945 public items (was 944)`, the ruled
rise; the seat cannot set `PR_BODY`, so its pass with the PR's `Ruling:`
line is pending. `quality/file-lines.txt` was re-measured with `wc -l` for
the four files this visit moved. brokkr-protocol did not change in this
visit.

The first visit's tree:

| Check | Observed |
| --- | --- |
| `cargo test -p brokkr-cli --all-features --locked --lib` | 640 passed, 1 ignored |
| `cargo test -p brokkr-cli --all-features --locked --test it` | 519 passed, 2 ignored; `capability_broker::` alone 33 passed |
| The CLI's other targets (`driver_conformance`, `transcript_surfaces`, the three heap budgets, the binary, doc tests) | all passed |
| `cargo test -p brokkr-protocol --all-features --locked` | 842 lib passed, 3 ignored; integration targets passed |
| `cargo test -p brokkr-runtime --locked --test it witness_digests::` | 6 passed with no bless: no witness or compose pin moved |
| Formatting, workspace clippy with `-D warnings`, `cargo +1.88 check --workspace --all-targets --all-features` | clean |
| `cargo run --locked -p brokkr-cli -- compile --bundle recipes/self` | compiled |
| `openspec validate --all --strict` | 20 passed |
| `typos --hidden`; `git diff --check`, staged and unstaged | clean |
| Public API, measured with `cargo +nightly-2026-09-05 public-api -sss` | brokkr-protocol gains exactly `Refusal::Establishment`, the snapshot updated to match; brokkr-cli unchanged |
| `quality/file-lines.txt` | updated by hand from `wc -l` for the six touched and two new files; no production file over 800 (`cli_args.rs` 781) |

The seat could not run `quality/ratchet.sh` (files, clones, api,
baselines), `quality/measure.sh`, `scripts/measure-budgets.sh`, jscpd,
lychee or the exact-coverage gate: each script needs an approval the seat
does not hold. They are pending. No new suppression was added; the
suppressions test inside `--test it` passed. No new function nears 100
lines, nesting 5 or seven parameters, and none has more than a handful of
branches. The full `cargo test --workspace` was not run as one command;
the two touched crates' suites and the runtime witness suite were.
