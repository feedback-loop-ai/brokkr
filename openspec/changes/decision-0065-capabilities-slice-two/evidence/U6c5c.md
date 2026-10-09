# U6c5c evidence: store identity, the checked launcher, the real writer set

Four visits built this unit. The first, run
`0065-slice-two-unit-u6c5c-see-th-21831f31`, is recorded as it was written
under "First visit"; its council held it (security-hold, three HIGHs). The
second, run `0065-slice-two-unit-u6c5c-see-th-3c9fa230`, repaired it on
main at `2d65ad24` under the operator's ruling of 2026-10-08, and is
recorded under "Repair visit", which supersedes the first visit wherever
the two differ. Its council held it again with two launcher HIGHs; the
third, run `0065-slice-two-unit-u6c5c-see-th-faa7efa2`, repaired those on
main at `1a6582c3` under the operator's rulings of 2026-10-08 and
2026-10-09, and is recorded under "Third visit", which supersedes both
wherever they differ. Its council held it a third time with two launcher
HIGHs; the fourth, run `0065-slice-two-unit-u6c5c-see-th-4f3fba47`,
repaired those on main at `9c059244` and is recorded under "Fourth visit",
which supersedes the three before it wherever they differ. Its council held
it a fourth time with two HIGHs in the header reader; the fifth, run
`0065-slice-two-unit-u6c5c-see-th-9bf69ee6`, rebuilt that reader to the
operator's closed list of 2026-10-09 on main at `4881f53e` and is recorded
under "Fifth visit", which supersedes every visit before it wherever they
differ.

## First visit

Run `0065-slice-two-unit-u6c5c-see-th-21831f31`, branch `s2/U6c5c` from main
at `339ad142`. Every result below was observed in this seat on the working
tree that became the unit's commit; nothing here was carried from an
earlier unit's file.

### What changed, by file

The row's four production files are the only production files touched.

`hands/namespace/sources/host.rs` keeps the launcher contract. `Host` gains
`found`, the launcher found without running it (`PATH`'s bubblewrap on the
live host); `Host::live` no longer runs anything, and its `launcher` reports
none until `launched` has checked one. `launched` joins the plan's sealed
writer uids to the observed writers (`Credentials::sealed`), checks the
launcher (`checked`: resolved from `/` through descriptors and opened as the
object resolved, by the shared `opened`; a regular file; no hop in either
reach; and, by the facts its own handle reports, MB3's write-exclusion
proof), runs the `between` step, and only then asks its version through
that very handle (`version`: the handle is the child's standard input and
the program is `/proc/self/fd/0`, under `env_clear`). The observation reads
that launcher's place and report, and the handle it holds for the launcher
must be the object that ran. A launcher refused before it runs reaches the
observation as none; its cause is merged with the observation's by MB3's
order, and a launcher never found keeps the distinct unavailable cause.
`credentials()` now reads three statuses: the observer's own, its broker's
(its parent) and that broker's harness (the broker's parent), each parent
counted only where the child's status names it as parent before and after
its status is read. `credited` takes the statuses together: the union of
their uids, confined only where every one is confined, and no writer known
where any status, or none at all, reads.

`hands/namespace/sources/store.rs` is new, a child of the observer declared
from host.rs with `#[path]` because sources.rs is outside this row. Its
`admitted` resolves the store through `opened`, refuses identity for a store
that is no regular file, has a second link or is not this process's user's,
and for an own mount the table does not prove (absent, another device, or
no local filesystem). Every place the store shows at, its own and each mount
alias from the table, and every hop of its route, are compared with the
seat's reach spelled and canonical: the hands cause. Then every place it
shows at is compared with the box's sources: the box cause. A place counts
only where it lies within a root, so a reach root below a directory on the
route exposes nothing. The handle is `O_PATH`: it reads nothing.

`hands/namespace.rs`: `ServerProfile` gains `writers`, the sealed uids;
`prepare_with` is no longer test-only, and `prepare` and it both observe
through `launched`; `ServerBox::store` admits the store against the seat's
reach and the box's own handles.

`cli/src/broker/session.rs`: the profile carries the plan's writer uids;
`after` calls `ServerBox::store` in place of its path comparisons; the
observer hands the store's handle back last over `SCM_RIGHTS`, after the
source handles the record names, and `compared` takes it off into
`Admitted::_store`, apart from the handles a launch mounts.

### The design reading, stated

- **Where the launcher is checked.** `served` (sources.rs, outside the row)
  calls `(host.launcher)()` before it observes. Rather than change it, the
  check runs before `served` is called and hands it a launcher whose report
  came from the checked handle; `served` then observes the launcher again
  with the other sources, and `launched` requires that observation's handle
  to be the object that ran. The pre-run check applies the observer's own
  rules: the reach rule (`BindOverlapsReach`) and the write-exclusion proof
  (`Identity`).
- **Who the writers are.** The sealed uids are joined, not compared, inside
  the proof: a sealed writer the observer cannot see still counts.
  `compared` keeps U6c5b's set comparison, but the observer reports the
  observed writers already joined with the sealed ones, so the comparison
  proves every observed writer sealed (observed ⊆ sealed), not the two sets
  equal (corrected on the third visit, R4): an observed writer the plan does
  not list still refuses identity. The observer's own status and its broker's are
  the same credentials by construction (the broker starts the observer
  itself), so no test can tell those two apart; the harness is independently
  bound (M12).
- **Owner-rooted.** Read as: resolved from `/` through descriptors, and owned
  by this process's user, the operator whose store it is. The store's mode
  stays with its reader's existing check.
- **Precedence.** Uncertainty (identity) first, then the hands cause, then
  the box cause, as `Refusal`'s order has them.

### Tests

New child module `crates/brokkr-protocol/src/hands/tests/checked.rs`
(registered in `hands/tests.rs`; no test moved), with six tests:

| Test | What it proves |
| --- | --- |
| `the_launcher_runs_through_its_checked_handle_under_a_cleared_environment` | The report is exactly `bubblewrap 0.5.0` (a script that would add ` with HOME` under an inherited environment); a launcher renamed over between check and run still reports the checked object's version and the observation holding the replacement refuses `Identity`; none found runs nothing. |
| `a_launcher_a_writer_could_write_or_replace_is_refused_before_it_runs` | Owned by an observed writer, by a sealed writer, group-writable, with writers unconfined, a directory, and replaced as its handle opens: each `Identity` with no report and no run; a launcher in read-only reach: `BindOverlapsReach`, not run. |
| `the_store_is_admitted_by_its_identity_alone_through_a_handle_that_reads_nothing` | The empty store's handle is its device and inode, `O_PATH`, and `read` on it fails `EBADF`; absent (and not created), a directory, a FIFO, a second link, `/etc/passwd` (not root here) and a directory replaced mid-route each refuse `Identity`. |
| `the_store_is_exposed_by_its_route_and_every_place_the_mount_table_shows_it_at` | Reach and a source beside it admit; read-only reach `StoreReachable`; a source directory or the store file itself `StoreInBox`; hands first; a route through writable reach `StoreReachable`; a planted `ro` alias into reach `StoreReachable`, an alias inside a source `StoreInBox`; the own record dropped, made overlay or given another device `Identity`, ahead of a known exposure. |
| `a_prepared_box_holds_no_handle_of_the_store_it_admits` | On a live box, the admitted store's handle is not among the box's handles and neither its path nor its descriptor number is in the argv; a store inside the package `StoreInBox`. |
| `the_write_exclusion_proof_holds_against_every_writer_observed_and_sealed` | Two statuses give the union of uids, confined; a harness-owned file is not excluded; a sealed writer joins; an unconfined harness unproves all; an unread status or none at all leaves no writer known, whoever is sealed. |

Changed protocol tests: `a_launcher_that_cannot_mount_a_descriptor_leaves_the_box_unavailable`
now finds script launchers through `found` under stranger writers (0.5.0
admits; 0.4.1, a garbled report and none found are `Unavailable`; a linked
package file is `Linked` even before a writer's launcher; a writer's launcher,
linked or not, `Identity`). `sources.rs` helpers became `pub(super)` for the
new module and the alias test's two table edits became the shared `overlay`
and `moved`; `credited` callers pass a slice; `ServerProfile` literals gain
`writers`.

CLI (`capability_broker.rs` and `capability_broker/observer.rs`): every
fixture holds a protected empty store; `serve` is started by a shell under
`no_new_privs` standing for the harness (`Root::harnessed`); the direct
observation is relayed by this test binary rerun as a confined broker under
a confined shell (`observer::relayed`), because the test process itself is
no confined broker. `serve_unboxed` now has no bubblewrap on `PATH`, since
any launcher this run could plant is a managed writer's. The stalled
launcher `Kind` of the deadline, cancellation and kill proofs is removed for
the same reason; those proofs keep the FUSE mount kind. New or changed
cases: the empty store admitted though a binding is declared, then a FIFO,
an absent and a linked store `Identity` with zero lookups; stores in writable
reach, readable reach, the package, the bootstrap and behind a link in reach;
`every_managed_writer_is_observed_confined_and_sealed` (an unconfined
harness refuses `Identity`); a sealed writer the observer cannot see admits
while an observed one the plan does not list refuses;
`relays_an_observation_its_confined_broker_and_harness_started` (writers
`[euid]` under a confined harness, none under an unconfined one); and
`a_launcher_a_managed_writer_owns_refuses_before_it_ever_runs` (refused
`Identity` well inside 20 s, its note never written).

### Mutations

Each was a compiling edit of the production line, run against the named
suite, then restored; every restored suite passed again in the full runs
below.

| # | Removal | Site | Failed test (line) |
| --- | --- | --- | --- |
| M1 | sealed uids not joined (`host.credentials.clone()`) | host.rs `launched` | `a_launcher_a_writer_could…` (checked.rs:118) |
| M2 | write-exclusion dropped (`file \| excluded`) | host.rs `checked` | same test (:115) |
| M3 | regular-file check dropped | host.rs `checked` | same test, directory case (:132) |
| M4 | reach check dropped | host.rs `checked` | same test (:136) |
| M5 | `env_clear` dropped | host.rs `version` | `the_launcher_runs…` (:81) |
| M6 | version asked through a fresh open of the place | host.rs `launched` | `the_launcher_runs…` (:96) |
| M7 | run-versus-held identity not required | host.rs `launched` | `the_launcher_runs…` (:97) |
| M8 | check's cause dropped when the observation passes | host.rs `launched` | `a_launcher_a_writer_could…` (:115) |
| M9 | check's cause losing to the box's (`Err(other)`) | host.rs `launched` | `a_launcher_that_cannot_mount…` (sources.rs:1499) |
| M10 | check's cause winning over earlier ones (`Err(cause)`) | host.rs `launched` | `a_launcher_that_cannot_mount…` (sources.rs:1489) |
| M11 | confined where any writer is (`any`) | host.rs `credited` | `the_write_exclusion_proof_holds…` (checked.rs:374) |
| M12 | harness left out of the observed writers | host.rs `credentials` | CLI `relays_an_observation…` (observer.rs:122) and `every_managed_writer…` (capability_broker.rs:1573) |
| M13 | no writer read counted as all confined | host.rs `credited` | `the_write_exclusion_proof_holds…` (:380) |
| M14 | `sealed` keeping only the observed uids | host.rs `Credentials::sealed` | `the_write_exclusion_proof_holds…` (:367) and `a_launcher_a_writer_could…` (:118) |
| S1 | store link count unchecked (`nlink >= 1`) | store.rs `admitted` | `the_store_is_admitted…` (:208) |
| S2 | store owner unchecked | store.rs `admitted` | same test, `/etc/passwd` (:211) |
| S3 | own mount's filesystem unchecked | store.rs `shown` | `the_store_is_exposed…` (:304) |
| S4 | own mount's device unchecked | store.rs `shown` | same test (:310) |
| S5 | route hops ignored | store.rs `admitted` | same test (:279) |
| S6 | mount aliases ignored | store.rs `shown` | same test (:284) |
| S7 | box cause dropped | store.rs `admitted` | same test (:271) and `a_prepared_box…` (:344) |
| S8 | box cause before the hands cause | store.rs `admitted` | same test (:274) |
| S9 | store handle opened readable | store.rs `admitted` | `the_store_is_admitted…` (:190) |
| C1 | admission's store check discarded | session.rs `after` | CLI `a_sealed_plan_is_admitted…` (capability_broker.rs:862) and `the_store_is_neither…` (:1542) |
| C2 | store handle not handed back | session.rs `handed` | CLI `a_sealed_plan_is_admitted…` (:856) and `the_observer_hands_back…` (observer.rs:283) |
| C3 | sealed writers not supplied to the profile | session.rs `prepared` | CLI `unprovable_or_unobserved_writers…` (:1493) |

Line numbers are those the failing run printed; the later clone refactor
(below) moved some test lines, and moved the launcher's and the store's
resolve-and-open into the shared `opened`. S9 was run again there, on the
final code (`OFlags::RDONLY` in `opened`): `the_store_is_admitted…` failed
at checked.rs:202, the `O_PATH` assertion, and passed once restored. Two removals survive, and are reported:
the store's regular-file check (`file \| true`) passes every test on this
ext4 host, because a directory there always has a second link and the link
rule refuses it first (a filesystem whose directories have one link, as
btrfs's do, needs the check; it is kept); and `parent`'s re-read of the
child's parent, which only a parent dying between two reads would show.

### Gates on this tree

`cargo fmt --all -- --check` and workspace clippy at `-D warnings` were
clean. Suites, crate by crate: brokkr-protocol 805 unit tests (3 ignored)
and its 6, 1 and 1 integration tests; brokkr-cli 628 unit tests (1 ignored),
512 `it` tests (2 ignored), 27 driver-conformance, 13 transcript-surface
tests and its three heap budgets; brokkr-runtime 872 unit tests and its 75,
120, 1 and 3 integration tests (the witness digests among them passed
unblessed: no witness or compose input moved); core, view, store, bridge
and the seatbelt probe all passed. `compile --bundle bundles/self` succeeded
with digest `2e085459…`, U6c5b's. `typos --hidden` and `git diff --check`
printed nothing; `openspec validate --all --strict` passed 20 of 20. `quality/ratchet.sh files`, `clones` and `api` held after the
re-measure: the clone ratchet first found one production clone (the
resolve-and-open sequence in host.rs and store.rs, now `opened`) and three
test clones (factored into `planted`, `held`, `overlay` and `moved`).
`quality/file-lines.txt` rows of the touched files were re-measured with
`wc -l`; host.rs is 796 lines, namespace.rs 790, session.rs 794, store.rs 87.
No suppression, `unsafe` or Cargo change was added.

### Pending

- **Public API.** `quality/ratchet.sh baselines 339ad142` reports
  `public-api/brokkr-protocol.txt: 923 public items (was 921)`:
  `ServerBox::store` and `ServerProfile::writers`. The snapshot is
  re-measured; the rise needs the operator's ruling at PR time.
- **Deadline controls.** This host mounts no FUSE file system for this
  user, so the stalled-mount deadline, cancellation and kill proofs skip
  here; with the launcher kind gone they rest on CI.
- **Exact coverage.** `scripts/coverage-exact.sh` was not run; it waits on
  CI.
- **macOS.** The `cfg(not(target_os = "linux"))` arm of `ServerBox::store`
  and the CLI harness's non-Linux branch were not compiled here.
- **Native alias creation** stays open under 28.9, as U6c5b left it.

### Follow-ups (not fixed, outside the row)

- `Observer::object` (sources.rs) still takes uid and mode from the earlier
  lookup rather than the handle's own facts (U6c5a's council INFO). The
  launcher and store checks added here read them from the handle; the
  observer's own path is sources.rs's, outside this row.
- store.rs is declared from host.rs with `#[path]`; registering it in
  sources.rs, its natural parent, is a one-line move for a unit that owns
  sources.rs.

## Repair visit

Run `0065-slice-two-unit-u6c5c-see-th-3c9fa230`, branch `s2/U6c5c` on main
at `2d65ad24`, the first visit's tree arriving uncommitted with the
operator's 2026-10-08 amendment, which adds `sources.rs` to the row for the
launcher's ordering. Every result here was observed in this seat on this
tree. The council's findings were three HIGHs (the launcher ran before its
admission; the store's route was resolved once and only its final object
owner-checked or aliased; the store was not compared with the hands'
actual system binds), two MEDIUMs (the held-launcher deadline proof was
deleted; two removals survived), two gate failures and one carried LOW.

### What changed

**H1, the launcher runs only once admitted.** `launched` (host.rs) now only
checks the launcher `found` names, never running it: resolved and opened
as the object resolved, a regular file, and by its handle's own facts
writable by no managed writer, observed or sealed. It hands the observation
the checked handle and canonical place (`Host::launcher` is now that pair,
not a report). `served` (sources.rs) observes the launcher as a support
source beside every other source, so its route and reach, the mount table's
proof of its mount and of every alias of it, and the second resolution all
stand before anything runs; only after the observation's verdict and the
launch-name check does `ran` (host.rs) require the handle the observation
kept at that place to be the object checked, run the `between` step, and
ask `--version` through that kept handle under `env_clear`. `checked` no
longer tests reach itself: the observation's reach cause comes first and no
run precedes it. The check's cause still merges with the observation's by
`Refusal`'s order, and a launcher never found keeps the unavailable cause.

**H2, the whole route.** Every `Hop` now carries who may write it (uid and
mode from the hop's own handle for a directory, with its ACL read through
that handle; the lookup's for a link) and its mount id. `admitted`
(store.rs) refuses identity unless `/` and every hop before the store are
`shut`: owned by the store's owner or root and, unless a link, writable by
no one else but where root's and sticky, with no extended ACL (the broker's
plan-ancestry guard in session.rs, with the ACL added). After `between`, a
second resolution must take the same route to the same object. Every hop's
own mount must be in the table, of its device and a local filesystem, and
every place another mount of its device shows that hop or a directory
holding it at counts as a place of the route: a link on the route that a
bind puts in reach exposes the store it names.

**H3, the hands' real surface.** The hands roots are the seat's declared
reach and `Profile::Workspace.system()`, the one projection of
`HOST_TOOLCHAIN_BINDS` the workspace box binds (all of `/etc/ssl`); the
box cause still compares only the places the store shows at with the
server box's own sources.

**The carried LOW.** `handle` (sources.rs) returns the facts its descriptor
reports beside it, and `Observer::object` judges a support file's link
count, owner and mode for the write-exclusion proof by them, not by the
earlier lookup; `opened` and the route's directory hops use the same.

**Controls.** `single` (the store's regular-file, link and owner predicate)
and `guarded` (taking `/`'s facts as an argument) are `pub(in crate::hands)`
so tests reach them directly, and `parent` takes its status reader as a
closure seam (`credentials` passes the real one).

**Gate failures.** The CLI test that calls Linux-only `observer::serve_args`
is now `#[cfg(target_os = "linux")]`; the store-owner case uses
`/etc/passwd` only where this run observes its owner as another uid.

**Fixtures.** Store fixtures in the protocol tests lie under `/var/tmp`
(root's, sticky) with every directory below the fixture's root made owner
writable only, since this host's umask makes new directories 775 and the
guard refuses those; the CLI fixture gains `Root::store` for the same, and
its bootstrap copy's directory is made owner-only as its comment always
said. `grant` takes the owner's and the others' ACL permissions, so a
directory keeps `0o755` and only its ACL differs. `Host::under` (server.rs)
places a fixture root elsewhere. `linked_library` (sources.rs) is the one
builder of the linked support library both suites use, after the clone
gate found the two setups alike.

**The held launcher, restored (MEDIUM).** The observer suite's blocking
proofs run under two kinds again. Beside the stalled FUSE mount, the
`Launcher` kind runs the whole attempt under `strace -f --seccomp-bpf`,
which stops nothing but `execve`, and injects `SIGSTOP` into the one
`execve` of `/proc/self/fd/0`: the host's real bubblewrap, admitted, is the
observer's own child, stopped at the exec of the very handle admitted,
while the observer waits on it. Each proof's attempt is this test binary
rerun as a child subreaper in the run's session (`adopted`, `adopt`, beside
the existing relay), so a launcher its observer leaves is adopted and not
orphaned: the kernel hangs up an orphaned group holding a stopped member,
which would end the launcher whatever the broker killed, and did, the first
time the group kill was removed. The proofs now wait for the broker, sweep
the observer's group (survivors killed so a failure cannot hang), and only
then wait for the attempt. A host without a tracing `strace` declares the
kind skipped without failing a boundary run, since the tool is the host's,
not the boundary's.

### Tests

`hands/tests/checked.rs` holds fifteen tests now (no test moved between
files):

| Test | What it proves |
| --- | --- |
| `the_launcher_runs_once_admitted_through_the_handle_kept_under_a_cleared_environment` | A noting launcher reporting `bubblewrap 0.5.0` only under a cleared environment (it reports 0.4.1 where it inherits `CARGO_MANIFEST_DIR`) admits the box and ran once; renamed over by a 0.4.1 launcher at the second `between`, after admission, the admitted object runs again and the replacement never. |
| `a_launcher_its_own_check_refuses_never_runs` | Owned by an observed writer, by a sealed writer, group-writable, writers unconfined, a directory, replaced as its handle opens: each `Identity`, each with zero runs, in one table. |
| `a_launcher_its_observation_refuses_never_runs` | In read-only reach (`BindOverlapsReach`), its mount an overlay or another device, a directory holding it mounted again in writable reach, replaced after its check before the observation resolves it, its directory moved and rebuilt between the two resolutions: each refused with zero runs, in one table. |
| `the_store_is_admitted_by_its_identity_alone_through_a_handle_that_reads_nothing` | As before; the store-owner case by observed ownership. |
| `only_a_regular_file_of_one_link_its_owner_owns_is_a_store` | `single` refuses a directory of one link (as btrfs reports every directory), a second link and another owner. |
| `a_store_whose_route_or_resolution_changes_refuses` | Its directory moved away as the store opens and an empty one or a link to it put back, and the store replaced at its name between the two resolutions: `Identity` each. |
| `a_store_whose_route_another_could_change_refuses` | A route directory `0o775`, `0o757`, this user's `0o1777`, made group-writable only between its lookup and its handle, or given an ACL at `0o755`: `Identity`; a link on the route admits. |
| `a_route_is_guarded_from_the_root_it_starts_from` | `guarded` refuses an unread root, a root another uid owns, and a writable hop. |
| `a_hop_is_shut_only_where_none_but_its_owner_or_root_can_change_it` | `shut`, condition by condition. |
| `the_store_is_exposed_by_its_route_and_every_place_the_mount_table_shows_it_at` | As before. |
| `a_hop_of_the_route_or_the_hands_system_set_exposes_the_store` | A directory holding a link on the route mounted again in reach: `StoreReachable`, though neither the store nor its directory is mounted elsewhere; the store's directory aliased under `/etc/ssl` beside `certs`, and under `certs`: `StoreReachable`; reached through a link past `/dev`, no local filesystem: `Identity`. |
| `a_prepared_box_holds_no_handle_of_the_store_it_admits` | As before, its fixture under `/var/tmp`. |
| `a_support_file_is_judged_by_its_handle_s_own_facts` | A linked support file made group-writable between its lookup and its open: `Identity`. |
| `a_parent_counts_only_while_it_stays_the_parent` | A parent whose child names another parent on the re-read is none; an unread parent is none. |
| `the_write_exclusion_proof_holds_against_every_writer_observed_and_sealed` | As before. |

The CLI observer suite's deadline, cancellation and kill proofs run the
`Launcher` kind on this host (the FUSE kind skips here):
`test result: ok. 8 passed` in 33 s, the deadline proof's full 30 s
included.

### Mutations

Each removal was a compiling edit of the named production line, run
against the owning suite, then restored from a saved copy (`cmp` showed
every production file identical before the gates). Line numbers are those
the failing run printed; the last edits (the `Ran` alias and the shared
`linked_library`) moved later lines of checked.rs by a few, so each row
names its assertion as well.

| # | Removal | Site | Failed (file:line, assertion) |
| --- | --- | --- | --- |
| P1 | the checked launcher run before the observation | host.rs `launched` | checked.rs:253, every one of the six observation cases ran once; :98, the positive ran twice |
| P2 | kept handle not required to be the checked object | host.rs `ran` | checked.rs:253, `swapped` admitted |
| P3 | the version asked through a fresh open of the place | host.rs `ran` | checked.rs:114, `(Err(Unavailable), (1, 1, 2))` |
| P4 | `env_clear` dropped | host.rs `version` | checked.rs:98, `Err(Unavailable)` |
| P5 | sealed uids not joined | host.rs `launched` | checked.rs:189, `sealed` admitted and ran |
| P6 | write exclusion dropped from the check | host.rs `checked` | checked.rs:189, `observed`, `sealed`, `grouped` admitted and ran |
| P7 | regular-file check dropped | host.rs `checked` | checked.rs:189, `directory` `Unavailable` |
| P8 | the check's cause loses to the observation's | host.rs `launched` | checked.rs:189, five cases `Unavailable` |
| P9 | the check's cause always wins | host.rs `launched` | sources.rs:1493, `Identity` for `Linked` |
| P10 | parent's re-read dropped | host.rs `parent` | checked.rs:653, the orphaned reader gave a parent |
| P11 | write facts from the lookup | sources.rs `object` | checked.rs:641, admitted |
| P12 | a directory hop's facts from the lookup | host.rs `take` | checked.rs:430, admitted |
| M11 | confined where any writer is | host.rs `credited` | checked.rs:712, `Proved` |
| M12 | harness left out of the writers | host.rs `credentials` | CLI capability_broker.rs:1584 and observer.rs:125 |
| M13 | no writer read counted confined | host.rs `credited` | checked.rs:718, `Proved` |
| M14 | `sealed` keeps only the observed uids | host.rs `Credentials::sealed` | checked.rs:705 and :189 |
| S1 | link count unchecked | store.rs `single` | checked.rs:348 (predicate) and :331 (linked store) |
| S2 | owner unchecked | store.rs `single` | checked.rs:349 (predicate) and :336 (`/etc/passwd`) |
| S3 | regular-file check dropped | store.rs `single` | checked.rs:347, a directory of one link |
| S4 | local filesystem unchecked | store.rs `within` | checked.rs:551 (overlay) and :590 (`/dev` hop) |
| S5 | device unchecked | store.rs `within` | checked.rs:551 (another device) |
| S6 | route places not compared with reach | store.rs `admitted` | checked.rs:532 and :573 |
| S7 | the store's own mount aliases ignored | store.rs `shown` | checked.rs:540, alias inside a source |
| S8 | a hop's holding aliases ignored | store.rs `routed` | checked.rs:573 |
| S9 | a hop's unproved mount accepted | store.rs `routed` | checked.rs:590 |
| S10 | the hands' system set left out | store.rs `admitted` | checked.rs:579, `/etc/ssl` sibling |
| S11 | the server projection used for the hands | store.rs `admitted` | checked.rs:579 |
| S12 | box cause dropped | store.rs `admitted` | checked.rs:523 and :615 |
| S13 | box cause before the hands cause | store.rs `admitted` | checked.rs:526 |
| S14 | store handle opened readable | host.rs `opened` | checked.rs:313, the `O_PATH` assertion |
| G1 | route guard dropped | store.rs `admitted` | checked.rs:411, `0o775` |
| G2 | any sticky directory accepted | store.rs `shut` | checked.rs:411, `0o1777` |
| G3 | ACL ignored | store.rs `shut` | checked.rs:482 (predicate) and :433 (live) |
| G4 | a link's mode counted | store.rs `shut` | checked.rs:485, :440, :536, :571 |
| G5 | `/` left out of the guard | store.rs `guarded` | checked.rs:463 |
| A1 | no second resolution | store.rs `admitted` | checked.rs:383, `emptied` |
| A2 | second route not compared | store.rs `admitted` | checked.rs:383, `relinked` |
| A3 | second object not compared | store.rs `admitted` | checked.rs:396 |
| N1 | `ServerBox::store` given no sources | namespace.rs `store` | checked.rs:619 |
| C1 | admission's store refusal discarded | session.rs `after` | CLI capability_broker.rs:1552, :876 and observer.rs:277 |
| C2 | store handle not handed back | session.rs `handed` | CLI capability_broker.rs:508 and :1496 |
| C3 | sealed writers not supplied | session.rs `prepared` | CLI capability_broker.rs:1507 |
| C4 | the observer killed alone, not its group | session.rs `ended` | CLI observer.rs:718 (deadline) and :749 (cancellation), `Launcher` |
| C5 | the watcher kills the observer alone | session.rs `tethered` | CLI observer.rs:769, `Launcher` |

The first visit's two survivors are closed: S3 fails the `single`
predicate's directory-of-one-link case, and P10 fails a status reader that
names another parent on the re-read. C4 first survived under a plain
`SIGSTOP` hold (the orphaned-group hang-up described above) and failed as
recorded once the attempt adopts.

### Gates on this tree

The workspace formatted clean and clippy at `-D warnings` across every
target passed; `cargo +1.88 check --workspace --all-targets
--all-features` finished. Suites, crate by crate: brokkr-protocol 814 unit
tests (3 ignored) and its 6, 1 and 1 integration tests; brokkr-cli 628 unit
tests (1 ignored), 512 `it` tests (2 ignored), 27 driver-conformance and 13
transcript-surface tests and its three heap budgets; brokkr-runtime 878
unit tests and its 75, 120, 1 and 3 integration tests, the witness digests
passing unblessed (no witness or compose input moved); every other
workspace crate passed. `compile --bundle bundles/self` succeeded with
digest `2e085459…`, unchanged. `quality/ratchet.sh files`, `clones` and
`api` held after re-measuring the touched rows of `file-lines.txt` with
`wc -l` (host.rs 796, sources.rs 799, store.rs 165, namespace.rs 790,
session.rs 794). No suppression, `unsafe`, dependency or Cargo change was
added; no function crossed a clippy ceiling.

`quality/ratchet.sh baselines 2d65ad24` reports only
`public-api/brokkr-protocol.txt: 923 public items (was 921)`. Against
`origin/main`, which has moved to `1a6582c3` (#591) since this branch's
base, it also reports `suppressions.txt` at 44 (was 43): that drop is
#591's, not in this tree, and goes with the rebase.

### Pending

- **Public API.** `ServerBox::store` and `ServerProfile::writers` (921 to
  923) need the operator's ruling at PR time.
- **Exact coverage.** `scripts/coverage-exact.sh` was not run; it waits on
  CI.
- **FUSE kind.** This host mounts no FUSE file system for this user, so the
  stalled-mount kind of the three blocking proofs skips here; it runs on
  CI's boundary job.
- **Tracer on CI.** Whether CI's runner has `strace` was not observed; where
  it has none the held-launcher kind is declared skipped there, and the
  group-cleanup removals C4 and C5 bind only where it runs (here).
- **Owner-only TMPDIR.** This seat could not make a directory outside its
  worktree, so the suites ran with the inherited `TMPDIR`; the store and
  CLI fixtures lie under `/var/tmp` either way.
- **macOS.** The non-Linux arms were not compiled here.
- **Native alias creation** stays open under 28.9.

### Follow-ups (outside the row)

- Install `strace` in CI's boundary job, so the held-launcher proofs and
  C4/C5's bindings run there too (a workflow change, outside this row).
- store.rs stays declared from host.rs with `#[path]`: sources.rs is at 799
  lines, and moving the declaration would cross its ceiling.

## Third visit

Run `0065-slice-two-unit-u6c5c-see-th-faa7efa2`, branch `s2/U6c5c` on main
at `1a6582c3`, the second visit's tree arriving uncommitted with the
operator's addenda of 2026-10-08 (the launcher is a protected ELF, never a
script) and 2026-10-09 (the row gains `sources/launcher.rs`). Every result
here was observed in this seat on this tree. The store, writer and earlier
findings were judged closed and are kept; this visit answers the two
launcher HIGHs, three LOWs and three gate failures.

### What changed

**H1, the route the launcher was found by.** The launcher's checks moved
out of host.rs into the new consumed child `sources/launcher.rs`
(`launched`, `checked`, `ran`, `version`, and the ELF reading below),
declared from sources.rs; host.rs keeps the observer's host facts and calls
nothing new but `version` as the live `Host::run`. `checked` now keeps a
`Checked` record: the path the launcher was found at, the handle it was
checked through and every hop of the route it took. `served` observes the
launcher by that found path (`Checked::source`), so the observation's own
route rule, digest and second resolution cover the original route, a hop in
either reach taking `BindOverlapsReach`. Before it runs, `ran` proves every
hop's mount in the table and that no other mount shows a hop inside the
seat's reach (`routed`, through the shared `holding` and store.rs's
`within`), resolves the found path once more and requires the very hops it
was checked by, and requires the handle the observation kept at the found
path to be the object checked; only then, after `between`, does that handle
run.

**H2, what the launcher executes.** `checked` admits only what `elf`
admits, read through a reader reopened from the retained handle by `pread`,
never mapped or run. `header` refuses a `#!` script as `Script`, anything
without ELF's magic as `NotElf`, a short header as `Truncated`, and another
class, encoding, version, type (only `ET_EXEC` and `ET_DYN`) or machine
(x86_64 and aarch64 known) as `Foreign`. `headers` holds the program header
table to 64 entries of 56 bytes, at most one `PT_INTERP` and one
`PT_DYNAMIC`, an interpreter name NUL-ended within 4096 bytes, at most 512
dynamic entries ended by `DT_NULL`, a string table one loaded segment holds,
and every string NUL-ended within 4096 bytes and the table. `elf` then
requires the interpreter, each `DT_RUNPATH`/`DT_RPATH` entry (split on `:`,
a leading `$ORIGIN` or `${ORIGIN}` expanded against the launcher's admitted
directory) and each `DT_NEEDED` name holding a slash to be absolute and
token-free, to resolve through descriptors, and to be `protected`: its
canonical place inside the server box's system set, of the kind the loader
reads there, and it and every hop of its route passing MB3's `excluded`
predicate (a link judged by its owner alone). The loader's preload file and
cache (`Host::loader`, `/etc/ld.so.preload` and `/etc/ld.so.cache` live)
must be protected where they exist; the preload file lies outside the set,
so any present refuses. A needed name without an interpreter is malformed.
Each refusal is a variant of the new `Unfit` enum (thiserror, its text
pinned once), mapped to MB3's identity cause.

**R2.** The observer suite's missing tracer now skips through `skip`, that
is `skip_boundary_proof(boundary_evidence_required(), …)`, failing where
boundary evidence is required. **R3.** host.rs's alias rule is split into
`others` and `held`; `aliases` adds the descendant arm, `holding` is the
holding arm alone, and store.rs's `routed` and launcher.rs's `routed` both
use `holding`; `holding`'s comment keeps the reason a route needs no
descendant arm. **R4.** session.rs's module documentation and `compared`'s,
and this file's first visit, now say the writer comparison proves observed
⊆ sealed, the observer's writers arriving joined with the sealed ones; the
check itself is unchanged. **G1.** The store fixtures lie under the first of
`TMPDIR` and `/var/tmp` that is a directory outside `/tmp` whose every
ancestor is `shut` to all but this user and root; none skips through
`skip_boundary_proof`. **G2.** No suppression was added: against the base
`1a6582c3`, `quality/ratchet.sh baselines` reports only the ruled API rise.
**G3.** host.rs's comment on store.rs now names sources.rs's 800-line
ceiling as the reason it is declared there.

### Tests

The three launcher tests of `hands/tests/checked.rs` moved to the new child
`hands/tests/launcher.rs` (registered in `hands/tests.rs`), and
`sources.rs`'s `launcher` helper moved with them; checked.rs keeps the store
and writer tests. A run of every launcher there is counted by a stand-in
(`Host::run`, the `Ground` builder), so each refusal shows zero runs; the
stand-in reads the report an image carries through the handle it is given.
Images are built by `Image`, of this host's identification and machine as
the test binary bears them, naming this host's interpreter.

| Test | What it proves |
| --- | --- |
| `the_launcher_runs_once_admitted_through_the_handle_kept` | A copy of the host's real bubblewrap, run as live, admits and runs once; renamed over by an image after admission, at the second `between`, the admitted bubblewrap still runs and reports, through the handle kept. |
| `a_launcher_its_own_check_refuses_never_runs` | Observed writer's, sealed writer's, group-writable, writers unconfined, a directory, replaced as it opens: `Identity`, zero runs. |
| `a_launcher_its_observation_refuses_never_runs` | Read-only reach (`BindOverlapsReach`), overlay or another device, a directory holding it aliased into reach, swapped after its check (refused at the kept-handle check, the fourth open being the launcher's last resolution), its directory rebuilt between the resolutions: zero runs. |
| `the_route_a_launcher_was_found_by_is_admitted_before_it_runs` | Found through a link beside reach admits (one run); through a link inside writable reach (`BindOverlapsReach`), through a link whose directory a planted mount shows inside reach (`BindOverlapsReach`), through a link past `/dev` (`Identity`): zero runs. |
| `a_route_that_changes_once_admitted_refuses_before_the_launcher_runs` | Its directory moved and linked back once the observation is admitted (at the mount table's second read): `Identity`, zero runs. |
| `only_a_whole_elf_executable_of_this_host_is_read_and_never_run` | 24 images, each with its exact `Unfit` read directly and `Identity` with zero runs prepared: script, magic, short, class, machine, type, entry size, no headers, 65 headers, a cut table, two interpreters, two dynamic segments, an unended, empty, over-long or cut interpreter name, an uneven, over-long or unended dynamic segment, an unloaded string table, a name over 4096 bytes or past its table, a needed name without an interpreter; the whole image admits with one run. |
| `a_launcher_that_cannot_be_read_is_never_run` | Mode `0311`: `Unread` directly, `Identity` with zero runs. |
| `every_path_the_loader_reads_for_the_launcher_must_be_protected` | The host's own inputs admit; interpreters relative, absent, `/etc/passwd` (root's, outside the set) and a directory; search entries relative by either tag, empty, holding `$LIB` or `$ORIGINX`, `$ORIGIN/..`, absent and a file; needed names relative, `${ORIGIN}`-relative and absent: each its exact cause, zero runs. |
| `a_loader_input_a_managed_writer_could_change_is_unprotected` | Root among the sealed writers, and the system interpreter named through a group-writable directory: `Unprotected(Interpreter)`, zero runs. |
| `the_loaders_preload_file_and_cache_are_read_only_where_they_exist` | Both absent admits; a present preload file or cache outside the set refuses; the host's cache admits; a relative preload path is `Unresolved`. |
| `every_launcher_refusal_reads_as_the_operators_text` | `Unfit`'s text, pinned once. |
| CLI `the_admitted_launcher_runs_through_its_handle_with_no_environment` | Under `strace -f --seccomp-bpf -v`, a whole admission's one launcher run is exactly `execve("/proc/self/fd/0", ["/proc/self/fd/0", "--version"], []) = 0`. |

`a_launcher_that_cannot_mount_a_descriptor_leaves_the_box_unavailable`
(sources.rs) now plants images reporting their versions and runs them by the
stand-in. The CLI `serving` builder was split into `traced` and `started` so
the recording tracer shares it.

### Mutations

Each removal was a compiling edit of the named production line, run against
the owning tests, then restored from a saved copy (`cmp` showed every
production file identical before the gates). Rows name the case the failing
assertion printed, as `case, typed answer, (prepared answer, runs)`.

| # | Removal | Failed (test: printed row) |
| --- | --- | --- |
| R1 | kept handle not required to be the object checked (`ran`) | observation test: `swapped`, `(Ok(()), 1)` |
| R2 | last resolution's route not compared (`ran`) | route-change test: `((Ok(()), 1), 2)` |
| R3 | `routed`'s alias arm dropped | found-route test: `alias`, `(Ok(()), 1)` |
| R4 | `routed`'s mount proof dropped | found-route test: `unproved`, `(Ok(()), 1)` |
| R5 | the observation takes the canonical endpoint (the pre-fix H1) | found-route test: `reach`, `(Ok(()), 1)` |
| R6 | `elf` not called by `checked` | the ELF, loader-input, writer and loader-file tests: every refusing image ran |
| E1 | script check dropped | `script`, `Err(NotElf)` |
| E2 | magic check dropped | `magic`, `Ok(())`, `(Ok(()), 1)` |
| E3 | class, encoding and version unchecked | `class`, `Ok(())`, `(Ok(()), 1)` |
| E4 | machine unchecked | `machine`, `Ok(())`, `(Ok(()), 1)` |
| E5 | type widened to `ET_REL` | `type`, `Ok(())`, `(Ok(()), 1)` |
| E6 | entry size unchecked | `entry-size`, `Ok(())`, `(Ok(()), 1)` |
| E7 | header bound ×1000 | `headers-over`, `Err(Truncated)` |
| E8 | no headers accepted | `no-headers`, `Ok(())`, `(Ok(()), 1)` |
| E9 | a second interpreter accepted | `two-interpreters`, `Ok(())`, `(Ok(()), 1)` |
| E10 | a second dynamic segment accepted | `two-dynamics`, `Ok(())`, `(Ok(()), 1)` |
| E11 | interpreter's NUL end unchecked | `interpreter-unended`, `Ok(())`, `(Ok(()), 1)` |
| E12 | interpreter bound ×1000 | `interpreter-over`, `Err(Truncated)` |
| E13 | empty interpreter accepted | `interpreter-empty`, `Err(Relative(Interpreter))` |
| E14 | dynamic size held to 8, not 16 | `dynamic-uneven`, `Ok(())`, `(Ok(()), 1)` |
| E15 | dynamic bound ×1000 | `dynamic-over`, `Err(Truncated)` |
| E16 | `DT_NULL` not required | `dynamic-unended`, `Ok(())`, `(Ok(()), 1)` |
| E17 | string table not held to a loaded segment | `table-unloaded`, `Ok(())`, `(Ok(()), 1)` |
| E18 | string read not held to its table | `name-past-table`, `Err(Truncated)` |
| E19 | string bound ×1000 | `name-over`, `Err(Malformed)` |
| E20 | needed names without an interpreter accepted | `needed-alone`, `Ok(())`, `(Ok(()), 1)` |
| L1 | `$ORIGIN` not expanded | `runpath-origin` and `needed-origin`, `Err(Relative(…))` |
| L2 | `$ORIGIN` taken without its boundary | `runpath-joined`, `Err(Unresolved(SearchPath))` |
| L3 | a `$` token admitted | `runpath-token`, `Err(Unresolved(SearchPath))` |
| L4 | relative paths admitted | the five relative rows, `Err(Unresolved(…))` |
| L5 | system-set place unchecked | `interpreter-outside`, `Ok(())`, `(Ok(()), 1)` |
| L6 | kind unchecked | `interpreter-directory` and `runpath-file`, `Ok(())`, `(Ok(()), 1)` |
| L7 | route hops unchecked | writer test: `linked`, `Ok(())`, `(Ok(()), 1)` |
| L9 | a link's mode counted | seven launcher tests, every positive refused at `/lib64` |
| L10 | search entries unchecked | the eight search rows, `Ok(())`, `(Ok(()), 1)` |
| L11 | search list not split on `:` | `runpath-empty`, `Err(Unresolved(SearchPath))` |
| L12 | slash-named needed names unchecked | the three needed rows, `Ok(())`, `(Ok(()), 1)` |
| L13 | `DT_RPATH` ignored | `rpath-relative`, `Ok(())`, `(Ok(()), 1)` |
| L14 | `DT_RUNPATH` ignored | the seven runpath rows, `Ok(())`, `(Ok(()), 1)` |
| L15 | present loader files unchecked | `preload` and `cache`, `Ok(())`, `(Ok(()), 1)` |
| L16 | absent loader files refused | `absent` and `system`, `Err(Unresolved(Preload))` |
| L17 | an unresolvable loader file passed | `relative`, `Err(Unprotected(Cache))` |
| X1 | `env_clear` dropped | CLI tracer test: the recorded `execve` carried the inherited environment (`CARGO_PKG_…`, …) |
| X2 | the run takes a fresh open of the found path | real-bubblewrap test: `(Err(Unavailable), 2, 2)` |
| A1 | `holding` emptied | `a_hop_of_the_route_or_the_hands_system_set_exposes_the_store` and the found-route test |

One removal survives, and is reported: **L8**, the protected object's own
`excluded` check (`object | true` in `protected`), passes all eleven
launcher tests. No fixture can put a file a managed writer could write
inside the root-owned system set without root, and with root among the
writers every system route's hops refuse too; the check is kept for a host
where a writer owns a file under a root-owned system directory (a
`/usr/local` handed to a user). The unreadable case's zero runs also hold
without `elf` (R6), the observation refusing an unreadable support file it
cannot prove root's; its `Unread` binds the typed reading.

These mutations ran on the tree before the final restructuring of the test
module (the `Ground` builder and `Case` tables, made for the clone gate)
and before `as_chunks` and `is_multiple_of` replaced two equivalent forms
for clippy. On the final tree E2 and L5 were run again together and failed
`magic` and `interpreter-outside` as before; the others were not rerun.

### Gates on this tree

The workspace formatted clean, clippy at `-D warnings` across every target
passed, and `cargo +1.88 check --workspace --all-targets --all-features`
finished. Suites, crate by crate: brokkr-protocol 822 unit tests (3
ignored) with its 6, 1 and 1 integration tests; brokkr-cli 621 unit tests
(1 ignored), 515 `it` tests (2 ignored), 27 driver-conformance, 13
transcript-surface and its three heap budgets; brokkr-runtime 878 unit
tests and its 75, 120, 1 and 3 integration tests, the witness digests
passing unblessed (no witness or compose input moved); core, view, store,
bridge and the seatbelt probe passed. `compile --bundle bundles/self`
succeeded with digest `2e085459…`, unchanged. `quality/ratchet.sh files`,
`clones` and `api` held after re-measuring the touched rows of
`file-lines.txt` with `wc -l` (sources.rs 800, host.rs 750, launcher.rs
519, store.rs 161, namespace.rs 790, session.rs 799); every production file
is within 800 lines. `quality/ratchet.sh baselines 1a6582c3` reports only
`public-api/brokkr-protocol.txt: 923 public items (was 921)`, ruled by the
operator on 2026-10-09. Against `origin/main`, which has moved to
`9c059244` since this branch's base, it also lists file-lines and
too-many-lines rows of files main changed since (U1g2, #592), which go with
the rebase; it lists no suppression. No suppression, `unsafe`, dependency or
Cargo change was added.

### Pending

- **Exact coverage.** `scripts/coverage-exact.sh` was not run; it waits on
  CI outside the box.
- **Link check.** This seat may not run `lychee`, so the offline link and
  fragment check of the Markdown is not observed here. `openspec validate
  --all --strict` passed 20 of 20, and `typos --hidden` and
  `git diff --check` printed nothing.
- **Owner-only TMPDIR.** This seat could not read or set its `TMPDIR`; the
  suites ran with the inherited one. The store tests ran rather than
  skipped (no `skipped:` line under `--nocapture`), so a qualifying place
  was found.
- **FUSE kind.** The stalled-mount kind of the blocking proofs skips here
  (no FUSE mount for this user); the tracer kind and the new recording test
  ran here.
- **macOS.** The non-Linux arms were not compiled here.
- **Native alias creation** and G4's limits (alias enumeration by device
  identity; writer observation through self, parent and grandparent) stay
  open under 28.9.

### Follow-ups (outside the row)

- The object-exclusion survivor (L8) wants a host fixture with a
  writer-owned file under a root-owned system directory, which needs root.

## Fourth visit

Run `0065-slice-two-unit-u6c5c-see-th-4f3fba47`, branch `s2/U6c5c` on main
at `9c059244`, the third visit's tree arriving uncommitted. The council of
run `0065-slice-two-unit-u6c5c-see-th-faa7efa2` confirmed the earlier
repairs (the original route, chain and alias admission, the kept handle run
under a cleared environment, the script and named-input checks, the MB4
store handle, the writer subset, R2's skip routing, the shared holding
rule, G1 to G3), and they are kept. This visit answers its two launcher
HIGHs (H1, H2), one MEDIUM (M1), the LOWs L1, L2 and L4, and three fixture
failures. Every result below was observed in this seat on this tree.

### What changed

**H1, the launcher's own place.** `launcher.rs` now decides the launcher's
own facts in `fit`, apart from observing them: a regular file, its
canonical place inside the server box's system set (`system`, each root of
`Profile::Server.system()` as spelled and as it resolves), and writable by
no managed writer. `checked` refuses identity unless `fit` holds, before
the headers are read, so a copy anywhere else, the build directory
included, never runs. `protected` takes its place rule from the same
`system`, so the launcher and its loader inputs answer to one set.

**H2, the bytes the loader maps.** The program headers are read once into
`Segment`s (type, offset, address, file-backed and loaded size). `ordered`
requires every loaded segment's file-backed size to be at most its loaded
size, its address and file ranges not to overflow, and each to begin on a
later page than the last page its predecessor loads, the page size read
from the kernel's auxiliary vector (`AT_PAGESZ`): segments out of order,
overlapping, or sharing a page (where the later mapping would replace the
earlier's bytes) refuse as malformed. `Segment::holds` translates an
address range through one segment's file-backed bytes alone. The one
dynamic segment must lie wholly inside one loaded segment's file-backed
bytes, read at the very offset that segment maps its address from
(`p_vaddr - load.p_vaddr == p_offset - load.p_offset`). `DT_STRTAB` and
`DT_STRSZ` must each appear once (a second refuses even when equal), and
the table's whole extent, not only its start, must lie in one loaded
segment's file-backed bytes, translated by that segment. Every needed name
and search string already had to begin inside the table and end with a
NUL inside it; at most one interpreter, wholly within the file and
NUL-ended, is kept, now read by `interpreter`. Every cause is `Unfit`'s,
mapped to identity before anything runs.

*Correction, fifth visit.* The paragraph above overstates the fourth tree.
That reader admitted an image with no dynamic segment at all; where the
dynamic entries named no needed name or search path it returned before
requiring either `DT_STRTAB` or `DT_STRSZ` and before checking the table's
extent, so "the table's whole extent must lie in one loaded segment" held
only for a table some entry read from; and it read the program-header
table at `e_phoff` without requiring a `PT_PHDR`, or that any load map the
table. The council's H1 and H2 on that tree are these three gaps; the
fifth visit's "What changed" below closes them.

**M1 and L4, the loader input's own exclusion and `/`.** `protected`'s
write-exclusion proof moved into the decision `protects(top, object, hops,
writers)`: `top`, the facts of `/` read through the handle every
resolution starts from (store.rs's `top`, now shared), must be read and
excluded, as must the object and every hop of its route (a link judged by
its owner alone). A resolution records no hop for `/`, so before this
visit no loader input's proof covered it. The decision takes facts as
data, so a test gives an object that fails beside a root and route that
pass, which no host fixture can (L8 in the third visit).

**L1, one route derivation.** store.rs's `showing(records, hop)` is the
one derivation of every place a hop shows at: its own place and each
holding mount's (identity where the table proves no mount of it). store.rs's
`routed` and launcher.rs's `routed` both read a route through it, and both
compare places with roots through store.rs's `exposed` and `spelled`, now
shared; `exposed` keeps the reason a root below a hop exposes nothing.
Each caller keeps its own causes: the store refuses identity first, then
the hands cause, then the box cause; the launcher takes the first of each
hop's identity or reach cause by MB3's order. The launcher's route check
now compares a hop's own place with reach too, which the observation
already did.

**The fixture failures.** The three tests the commission named, and every
other positive or route control, now find the host's own bubblewrap (the
new `bubblewrap` helper: `require_bwrap`, canonical, inside the system
set), through links planted in the fixture where the route matters, on a
`Ground::launched` that is `Ground::served`. In a seat's hands box they
skip through `skip_in_box`, and where no bubblewrap lies in the system set
through `skip_boundary_proof`; both fail where boundary evidence is
required. No fixture image runs any more: the stand-in reports a fixed
version and counts runs, so `reported`, `reporting` and the image's report
trailer are gone.

**L3, recorded not redesigned.** The ruled named inputs are the launcher's
interpreter, its `DT_RUNPATH` and `DT_RPATH` entries, its slash-named
`DT_NEEDED` names and the loader's preload file and cache. A needed name
without a slash is found through the protected search path, the cache and
the loader's default directories, and is not resolved here; the libraries
those load in turn are not read. Both stay outside this repair, as the
2026-10-08 ruling scopes it.

### Tests

`hands/tests/launcher.rs`, in the order the file holds them:

| Test | What it proves |
| --- | --- |
| `the_launcher_runs_once_admitted_through_the_handle_kept` | The host's bubblewrap, found through a fixture link and run as live, admits; its link renamed at the second `between` to one naming the host's interpreter, the admission still runs bubblewrap through the kept handle: two reports, each `bubblewrap …`, two `between` calls. |
| `a_launcher_outside_the_system_set_never_runs` | A whole image of this host's, every input it names protected and no writer able to write it, planted under the build directory: `elf` reads `Ok(())`, preparing refuses `Identity` with zero runs. |
| `a_script_in_the_system_set_never_runs` | The first root-owned `#!` file `/usr/bin` holds by name, inside the set and writable by none: `Err(Script)` read, `Identity` with zero runs. |
| `a_launcher_is_fit_only_as_a_regular_file_in_the_system_set_no_writer_could_write` | `fit` on facts: a root-owned `0755` file in `/usr/bin` fits; a directory, a place in `/opt`, a group-writable mode, root among the writers and unconfined writers do not. |
| `a_launcher_its_own_check_refuses_never_runs` | The host's bubblewrap with root among the observed or the sealed writers, or the writers unconfined; its directory as the launcher; a fixture link replaced as its handle opens: `Identity`, zero runs each. |
| `a_launcher_its_observation_refuses_never_runs` | Through a fixture link: read-only reach and a planted alias of the link's directory in writable reach (`BindOverlapsReach`); the link's mount made overlay or another device; the link swapped to one naming another protected file before the observation and put back before the last resolution, so only the kept handle's identity differs; the directory rebuilt between the two resolutions (`Identity`): zero runs each. |
| `the_route_a_launcher_was_found_by_is_admitted_before_it_runs` | Links beside reach to the host's bubblewrap admit with one run; a link in writable reach and a planted alias of the links' directory refuse `BindOverlapsReach`, a link past `/dev` `Identity`, zero runs. |
| `a_route_that_changes_once_admitted_refuses_before_the_launcher_runs` | The link's directory moved and linked back at the mount table's second read: `Identity`, zero runs. |
| `only_a_whole_elf_executable_of_this_host_is_read_and_never_run` | 23 images, each its exact `Unfit` read directly and `Identity` with zero runs prepared. The uneven dynamic segment now grows its loaded segment and the image with it, so only its size's evenness is wrong; the unloaded-table case went, as `mapped` below holds the table's extent. |
| `a_launcher_is_read_exactly_as_the_loader_maps_it` | Loaded segments apart (`Ok(())`), out of order, overlapping, sharing a page, wrapping the address space or the file's offsets, and file-backed past their loaded size; the dynamic segment at an unmapped address, at a shifted address inside its segment, and past the file-backed bytes; the string table crossing the file-backed end, or past it within the loaded size; `DT_STRTAB` twice equal or conflicting and `DT_STRSZ` twice; a name the table cuts before its NUL: `Malformed` each, zero runs. |
| `a_launcher_that_cannot_be_read_is_never_run` | Unchanged: `Unread`, zero runs. |
| `every_path_the_loader_reads_for_the_launcher_must_be_protected` | Unchanged cases, each prepared answer now refused with zero runs. |
| `a_loader_input_a_managed_writer_could_change_is_unprotected` | Unchanged cases. |
| `a_loader_input_is_protected_only_where_root_object_and_every_hop_are` | `protects` on facts: all shut admits; the object alone group-writable, one hop group-writable, `/` group-writable or unread each refuse. |
| `the_loaders_preload_file_and_cache_are_read_only_where_they_exist` | Unchanged cases. |
| `every_launcher_refusal_reads_as_the_operators_text` | Unchanged. |

`hands/tests/sources.rs`'s
`a_launcher_that_cannot_mount_a_descriptor_leaves_the_box_unavailable` now
judges versions on the host's bubblewrap, a stand-in reporting each
(`0.5.0` admits, `0.4.1` and a bare name leave the box unavailable), and
refuses an image outside the set and the host's launcher with root among
the writers as `Identity`; the linked-package-first assertions keep a
fixture image.

### Mutations

Each removal below was a compiling edit of one production line, run
against `cargo test -p brokkr-protocol --lib --locked hands::tests::launcher::`
(the whole `hands::tests::` for host.rs, store.rs and `launched`, and
`cargo test -p brokkr-cli --test it --locked capability_broker::` for X1),
then undone. Every production file was compared with a copy saved before
the first removal, and `cmp` found launcher.rs, store.rs and host.rs
identical once the last was undone. All ran on the final production tree;
those whose failure lies in `only_a_whole_elf_executable_…` were rerun
after the last test edit (the uneven case above), so every row was observed
on the final test tree too. Rows give the case and the value the failing
assertion printed.

| # | Removal | Failed (test: printed) |
| --- | --- | --- |
| H1 | `fit` without its system-set place | the outside-set test: `(Ok(()), (Ok(()), 1))`; `fit` test: `outside` true; every whole image in the ELF and loader tables ran once (`(Ok(()), 1)`), each refusing image still with zero runs |
| F1 | `fit` without its regular-file check | `fit` test: `directory` true |
| F2 | `fit` without write exclusion | `fit` test: `grouped`, `observed`, `unconfined` true |
| M1 | `protects` without the object's exclusion | `protects` test: `object` true |
| L4 | `protects` without `/` | `protects` test: `root` and `root-unread` true |
| L7 | `protects` without the hops | `protects` test: `hop` true; writer test: `linked`, `Ok(())` |
| O1 | `ordered` by exact ranges, not pages | `loads-sharing-a-page`, `Ok(())` |
| O2 | `ordered` comparing starts only | `loads-overlapping` and `loads-sharing-a-page`, `Ok(())` |
| O3 | `ordered` with no order | `loads-unordered`, `loads-overlapping`, `loads-sharing-a-page`, `Ok(())` |
| O4 | file-backed size past the loaded size admitted | `load-file-over-memory`, `Ok(())` |
| O5 | address range wrapping (`wrapping_add`) | `load-wrapping`, `Ok(())` |
| O6 | file range unchecked | `load-offset-wrapping`, `Ok(())` |
| D1 | dynamic segment's mapping unchecked | `dynamic-unmapped`, `dynamic-shifted`, `dynamic-past-file`, `Ok(())` |
| D2 | dynamic segment held, translation unchecked | `dynamic-shifted`, `Ok(())` |
| D3 | dynamic segment's start alone held | `dynamic-past-file`, `Ok(())` |
| T1 | string table's start alone held | `table-crossing` and `table-past-file`, `Err(Truncated)` |
| T2 | `holds` by the loaded size | `dynamic-past-file`, `Ok(())`; `table-past-file`, `Err(Truncated)` |
| T3 | first `DT_STRTAB` taken | `table-twice` and `table-conflicting`, `Ok(())` |
| T4 | first `DT_STRSZ` taken | `size-twice`, `Ok(())` |
| T5 | a string read past its table | `name-unended-in-table` and `name-past-table`, `Ok(())` |
| R1 | kept handle not required to be the object checked | observation test: `swapped`, `(Ok(()), 1)` |
| R2 | last resolution's route not compared | route-change test: `((Ok(()), 1), 2)` |
| R3 | launcher route compared at hops' own places only | found-route and observation tests: `alias`, `(Ok(()), 1)` |
| R4 | launcher route's unproved mount accepted | found-route test: `unproved`, `(Ok(()), 1)` |
| R6 | `elf` not required by `checked` | script test: `(Err(Script), (Ok(()), 1))` |
| X1 | `env_clear` dropped | CLI tracer test: the recorded `execve` carried `CARGO_PKG_REPOSITORY=…` and the rest of the inherited environment |
| X2 | the run takes a fresh open of the found path | real-bubblewrap test: `(Ok(()), Err(Unavailable), 2, false, 2)` |
| P5 | sealed writers not joined in `launched` | own-check test: `sealed`, `(Ok(()), 1)` |
| P8 | the check's cause dropped (`observed`) | ten tests: the refused launchers answered `Unavailable` |
| P9 | the check's cause always wins | sources.rs test: `Err(Identity)` for `Linked` |
| A1 | `holding` emptied (host.rs) | store's `a_hop_of_the_route_…`: `Ok(())`; found-route and observation tests: `alias`, `(Ok(()), 1)` |
| S8 | `showing` without holding places (store.rs) | the same three failures as A1 |
| S9 | store `routed` accepting an unproved hop | `a_hop_of_the_route_…`: `Ok(())` for `Err(Identity)` |
| E1 | script check dropped | `script`, `Err(NotElf)` |
| E2 | magic unchecked | `magic`, `Ok(())` |
| E3 | class, encoding and version unchecked | `class`, `Ok(())` |
| E4 | machine unchecked | `machine`, `Ok(())` |
| E5 | `ET_REL` admitted | `type`, `Ok(())` |
| E6 | entry size unchecked | `entry-size`, `Ok(())` |
| E7 | header bound ×1000 | `headers-over`, `Err(Truncated)` |
| E8 | no headers admitted | `no-headers`, `Ok(())` |
| E9 | a second interpreter admitted | `two-interpreters`, `Ok(())` |
| E10 | a second dynamic segment admitted | `two-dynamics`, `Ok(())` |
| E11 | interpreter's NUL end unchecked | `interpreter-unended`, `Ok(())` |
| E12 | interpreter bound ×1000 | `interpreter-over`, `Err(Truncated)` |
| E13 | empty interpreter admitted | `interpreter-empty`, `Err(Relative(Interpreter))` |
| E14 | dynamic size held to 8 | `dynamic-uneven`, `Ok(())` |
| E15 | dynamic bound ×1000 | `dynamic-over`, `Err(Malformed)` |
| E16 | `DT_NULL` not required | `dynamic-unended`, `Ok(())` |
| E19 | string bound ×1000 | `name-over`, `Ok(())` |
| E20 | needed names without an interpreter admitted | `needed-alone`, `Ok(())` |
| N1 | `$ORIGIN` not expanded | `runpath-origin` and `needed-origin`, `Err(Relative(…))` |
| N2 | `$ORIGIN` without its boundary | `runpath-joined`, `Err(Unresolved(SearchPath))` |
| N3 | another `$` token admitted | `runpath-token`, `Err(Unresolved(SearchPath))` |
| N4 | relative paths admitted | the five relative rows, `Err(Unresolved(…))` |
| N5 | an input's system-set place unchecked | `interpreter-outside`, `Ok(())` |
| N6 | an input's kind unchecked | `interpreter-directory` and `runpath-file`, `Ok(())` |
| N9 | a link's mode counted | nine tests: every image, the whole ones included, `Err(Unprotected(Interpreter))`, and every bubblewrap positive refused |
| N10 | search entries unchecked | the eight search rows, `Ok(())` |
| N11 | search list not split on `:` | `runpath-empty`, `Err(Unresolved(SearchPath))` |
| N12 | slash-named needed names unchecked | the three needed rows, `Ok(())` |
| N13 | `DT_RPATH` ignored | `rpath-relative`, `Ok(())` |
| N14 | `DT_RUNPATH` ignored | the seven runpath rows, `Ok(())` |
| N15 | present loader files unchecked | `preload` and `cache`, `Ok(())` |
| N16 | absent loader files refused | `absent` and `system`, `Err(Unresolved(Preload))` |
| N17 | an unresolvable loader file passed | `relative`, `Err(Unprotected(Cache))` |

The third visit's loader-input rows L1 to L17 are N1 to N17 here, renamed
so they do not collide with the commission's L-findings; its E17 and E18
are T1 and T5, and its L8 survivor is M1, now binding. R5 (the observation
taking the canonical endpoint) was not rerun: `Checked` keeps no canonical
place to substitute, so no one-line removal expresses it on this tree; the
found-route test's `reach` row is the control it bound.

Three facts the runs showed are worth stating. First, every image a test
plants lies outside the system set, so its prepared answer is refused by
`fit` whatever its headers say; each header cause is bound by its typed
reading through `elf` instead, and H1's run shows the zero-run markers are
the header checks' own once membership is removed (only whole images ran).
Second, under F2 the integration cases `observed` and `unconfined` still
refused, by the observation's own write-exclusion proof of the system
sources; `fit`'s unit test is what binds the launcher's own exclusion, and
`sealed` binds the join (P5). Third, under the original fixture the
uneven-dynamic case was refused by the new mapping check first, so E14
survived until the fixture grew the loaded segment with it.

### Gates on this tree

The tree formatted clean; clippy at `-D warnings` on every target and
feature passed; `cargo +1.88 check --workspace --all-targets
--all-features` finished. Suites, crate by crate: brokkr-protocol 831 unit
tests (3 ignored) and its 6, 1 and 1 integration tests; brokkr-cli 621 unit
tests (1 ignored), 515 `it` tests (2 ignored), 27 driver-conformance, 13
transcript-surface and its three heap budgets; brokkr-runtime 882 unit
tests and its 75, 120, 1 and 3 integration tests, the witness digests
passing unblessed (no witness or compose input moved); the remaining crates
with no failure. The hands suite alone, under `--nocapture`, printed no
skip but the skip function's own self-test. `compile --bundle bundles/self`
gave digest `2e085459…`, unchanged. The rows of `quality/file-lines.txt`
for every file this unit touches were set to `wc -l` (launcher.rs 614,
store.rs 167, host.rs 750, sources.rs 800, namespace.rs 790, session.rs
799; the test files as measured), and `quality/ratchet.sh files`, `clones`
and `api` held. Against the branch base `9c059244`, `quality/ratchet.sh
baselines` reports only `public-api/brokkr-protocol.txt: 944 public items
(was 942)`, which is `ServerBox::store` and `ServerProfile::writers`, ruled
on 2026-10-09. Against `origin/main`, now `4881f53e` (#595, U6c7), it also
lists main's own baseline moves since the base, which go with the rebase.
`openspec validate --all --strict` passed 20 of 20; `typos --hidden` and
`git diff --cached --check` printed nothing. No suppression, `unsafe`,
dependency or Cargo change was added.

### Pending

The owner-only `TMPDIR` runs the commission asks for could not be made:
this seat refuses an environment prefix on a command, so every suite ran
with the inherited `TMPDIR`. The launcher fixtures lie in the build
directory, not `TMPDIR`, and the store fixtures in the first qualifying of
`TMPDIR` and `/var/tmp`, which ran rather than skipped. The offline link
check (`lychee`) is refused in this seat too and is not observed. The
exact-coverage gate waits on CI outside the box. The FUSE stalled-mount
kind skipped here as before (no FUSE mount for this user). macOS arms were
not compiled. Native alias creation stays open under 28.9.

### Follow-ups (outside the row)

The known cancellation race in the CLI observer test (`a_cancelled_attempt_…`)
was not touched. Main has moved past this branch's base with #595, which
reworks the secret store's reader; the rebase is the operator's.

## Fifth visit

Run `0065-slice-two-unit-u6c5c-see-th-9bf69ee6`, branch `s2/U6c5c` on main
at `4881f53e` (#595, U6c7, merged meanwhile and touching none of this
unit's files), the fourth visit's tree arriving uncommitted. The council of
run `0065-slice-two-unit-u6c5c-see-th-4f3fba47` confirmed, and this visit
keeps unchanged: the launcher endpoint membership; the original-route,
chain and mount-alias admission before execution; retained-handle
execution under `env_clear`; the store's `O_PATH` identity and its
separate, never-mounted handoff; the writer subset and the shared route
derivation; and the object, root and hop exclusion controls. It held the
unit on two HIGHs in launcher.rs's header reader (H1, H2), a MEDIUM (M1)
and a LOW (L1). The operator ruled one more visit against a closed list of
the launcher's ELF layout (the last addendum of
`operator-ruling-2026-10-03.md`); this visit implements its nine items and
refuses anything they do not name. Every result below was observed in
this seat on this tree.

### What changed

**H1 and H2, the closed list.** `headers` in `launcher.rs` now reads the
nine items in this order, each refusal a typed `Unfit` mapped to identity
before anything runs:

| Item | Where | Refused as |
| --- | --- | --- |
| 1, identity | `header`: magic, `ELFCLASS64`, host encoding, `EI_VERSION` 1, `ET_EXEC` or `ET_DYN`, host machine; `headers`: `e_phentsize` 56, 1 ≤ `e_phnum` ≤ 64 | `Script`, `NotElf`, `Truncated`, `Foreign`, `Malformed`, `Over` |
| 2, table in the file | `within(e_phoff, e_phnum × 56, length)`, the length from `fstat` of the read handle | `Truncated`, `Malformed` where the end wraps |
| 3, `PT_PHDR` | `lone` (exactly one), its place before the first `PT_LOAD`, `p_offset == e_phoff`, `p_filesz == e_phnum × 56`, and `mapped` (inside one load's file-backed bytes, `p_vaddr − load.p_vaddr == p_offset − load.p_offset`) | `Malformed` |
| 4, loads | `loaded`: each load `within` the file; `ordered`: `p_filesz ≤ p_memsz`, `p_offset ≡ p_vaddr (mod p_align)` where `p_align > 1`, ascending page spans that share no page, no two file ranges sharing a byte (every pair) | `Truncated`, `Malformed` |
| 5, `PT_INTERP` | `lone`, before the first load, `within` the file, at most 4096 bytes, its only NUL its last byte, not empty; `elf`: absolute, then protected | `Malformed`, `Over`, `Truncated`, `Relative(Interpreter)`, `Unprotected` / `Unresolved` |
| 6, `PT_DYNAMIC` | `lone`, a whole number of 16-byte entries, at most 512, `mapped`, a `DT_NULL`, nothing after the first read | `Malformed`, `Over` |
| 7, tags | `tabled`, always: no `DT_DEPAUDIT`, `DT_AUDIT`, `DT_AUXILIARY` or `DT_FILTER`; at most one `DT_RUNPATH` and one `DT_RPATH`; exactly one `DT_STRTAB` and one `DT_STRSZ` (a second refused even when equal); the table's whole extent inside one load's file-backed bytes, read through its translation | `Audited`, `Malformed` |
| 8, strings | `string`: offset inside the table, NUL inside the table, at most 4096 bytes; `elf`/`expanded`: a `$` anywhere but a leading `$ORIGIN` or `${ORIGIN}` refuses in every search entry and every needed name (with or without a slash), slash-named needed names and search entries protected, the preload file and cache where they exist | `Malformed`, `Over`, `Relative(…)`, `Unprotected(…)`, `Unresolved(…)` |
| 9, static | no `PT_INTERP` or no `PT_DYNAMIC` | `Static` |

`Unfit` gains `Static` and `Audited`, their text pinned in the module's one
text test. `Loaded::interpreter` is now required (`Vec<u8>`, not an
`Option`), so no launcher reaches `elf` without one.

Three readings are stated as assumptions. First, "neither their virtual
ranges overlap" is read at the loader's mapping unit: a load's virtual
range is the pages it maps, so two loads that share a page refuse (the
`loads-sharing-a-page` case) though their bytes do not overlap; file ranges
are compared byte for byte. Second, two checks the fourth tree made that
the list does not name were removed: `e_ehsize == 64` and the header's
`e_version` word; a `$` inside `PT_INTERP`, which the kernel takes
literally, is no longer refused, the interpreter needing only to be
absolute and protected. Third, a needed name with a `$` but no slash is now
refused (item 8: "a `$` anywhere"), which the fourth tree let through to the
loader's expansion.

**M1, the admission seam.** `Host` gains a test-only field,
`#[cfg(test)] admitted: &dyn Fn(&Path) -> bool` (`|_| false` on the live
host), and `fit` takes `granted`: in a test build `checked` grants the
membership the field answers for the launcher's canonical place, and in
every other build `granted` is the literal `false`, so no production path
can widen the system set. The table tests (`judged`) grant the fixture
root, so each planted image is decided by its headers alone and the
stand-in counts a run (never executing anything) for exactly the images
that admit; `expected` now asks `(Ok(()), 1)` of an admitted image and
`(Err(Identity), 0)` of a refused one, and `agreed` prints only the rows
that differ. The real system launcher positives are unchanged and still
run the host's own `/usr/bin/bwrap` through the new reader: the
real-bubblewrap test admitted it and reported `bubblewrap …` twice.

**INFO.** `DT_AUDIT` and `DT_DEPAUDIT` (and `DT_FILTER`, `DT_AUXILIARY`)
are now refused by item 7, beside the 2026-10-08 named-input limitation
(selection by name through the default directories, and the loader's
transitive inputs, are not read). `/etc/ld.so.preload` present outside the
protected set still fails closed (`Unprotected(Preload)`, the preload
test's `preload` row).

The line counts are launcher.rs 691, host.rs 755 and the launcher tests
1,373 (`wc -l`), set in `quality/file-lines.txt`; sources.rs, store.rs,
namespace.rs and session.rs did not move this visit.

### Tests

`hands/tests/launcher.rs`; every table case is a byte image built in the
test, planted, never executed.

| Test | Cases (items) |
| --- | --- |
| `only_a_whole_elf_executable_of_this_host_is_read_and_never_run` | 25 rows on the noted image (a fifth header, `PT_NOTE`): `elf` and `executable` admit; `script`, `magic`, `short`, `class` (ELFCLASS32), `encoding`, `version`, `machine`, `type` (`ET_REL`), `entry-size`, `no-headers`, `headers-over`, `headers-cut`, `headers-past-file`, `headers-wrapping` (1, 2); `phdr-missing`, `phdr-twice`, `phdr-after-load`, `phdr-elsewhere`, `phdr-short`, `phdr-unmapped`, `phdr-shifted`, `phdr-unloaded` (the load begun after the table), `no-loads` (3, 4) |
| `a_launcher_is_read_exactly_as_the_loader_maps_it` | 21 rows: `loads-apart`, `loads-aligned` admit; `load-unaligned`, `loads-unordered`, `loads-overlapping`, `loads-sharing-a-page`, `loads-sharing-bytes`, `loads-sharing-bytes-apart` (an empty load between), `load-wrapping`, `load-offset-wrapping`, `load-past-file`, `load-file-over-memory` (4); `dynamic-unmapped`, `dynamic-shifted`, `dynamic-past-file` (6); `table-crossing`, `table-past-file`, `table-unloaded` (the table past every load, inside the file) (7); `name-unended-in-table`, `name-past-table`, `name-over` (8) |
| `a_launcher_names_one_interpreter_one_dynamic_segment_and_one_string_table` (new) | 29 rows on the noted image: `static-uninterpreted`, `static-undynamic` (9); `other-unread` (the note far past the file) admits (9); `two-interpreters`, `interpreter-after-load`, `interpreter-unended`, `interpreter-holding-nul`, `interpreter-empty`, `interpreter-over`, `interpreter-cut`, `interpreter-wrapping` (5); `two-dynamics`, `dynamic-uneven`, `dynamic-over`, `dynamic-unended` (6); `after-null` (an audit entry after `DT_NULL`) and `bare` (no names, whole table) admit; `table-twice`, `table-conflicting`, `size-twice`, `size-conflicting`, `table-missing`, `size-missing`, `bare-table-missing`, `bare-table-unmapped`, `depaudit`, `audit`, `auxiliary`, `filter` (7) |
| `every_path_the_loader_reads_for_the_launcher_must_be_protected` | gains `runpath-twice`, `rpath-twice` (7), `rpath-token`, `needed-token` (8): 20 rows |
| `the_loaders_preload_file_and_cache_are_read_only_where_they_exist`, `a_loader_input_a_managed_writer_could_change_is_unprotected` | unchanged cases, now prepared under the grant: the admitted rows run once, the refused none |
| `every_launcher_refusal_reads_as_the_operators_text` | gains `Static` and `Audited` |

The fourth visit's `structure` table is split between the first and the
new test; its `needed-alone` row (names without an interpreter) is now the
static rows, and its `phdr`-less layout is gone, every image now carrying a
`PT_PHDR`. Under `--nocapture` the launcher module printed no skip line
(`grep -ci skip`: 0).

### Mutations

Each removal was a compiling edit of one production line, run against the
named test (`cargo test -p brokkr-protocol --lib --locked
hands::tests::launcher::<test>`, `hands::tests::checked::` for store.rs and
the writer rows, and `cargo test -p brokkr-cli --test it --locked
capability_broker::` for session.rs and M12), then undone. Rows give each
differing row the failing `agreed` printed, as `case, typed, (prepared,
runs)` against the row expected. `cmp` against copies saved before the
first removal found launcher.rs, host.rs, store.rs and session.rs identical
once the last was undone.

| # | Removal | Failed (printed) |
| --- | --- | --- |
| K1 | class unchecked (`header[5..7] == IDENT[1..]`) | `class`, `Ok(())`, `(Ok(()), 1)` |
| K2 | encoding unchecked | `encoding`, `Ok(())`, `(Ok(()), 1)` |
| K3 | `EI_VERSION` unchecked | `version`, `Ok(())`, `(Ok(()), 1)` |
| K4a | `ET_EXEC` refused (`3` only) | `executable`, `Err(Foreign)`, `(Err(Identity), 0)` |
| K4b | `ET_REL` admitted (`1..=3`) | `type`, `Ok(())`, `(Ok(()), 1)` |
| K5 | machine unchecked | `machine`, `Ok(())`, `(Ok(()), 1)` |
| K6 | `e_phentsize` unchecked | `entry-size`, `Ok(())`, `(Ok(()), 1)` |
| K8 | header bound ×64 | `headers-over`, `Err(Truncated)` |
| K9 | table extent unchecked | `headers-wrapping`, `Err(Unread)` |
| P1 | missing `PT_PHDR` taken as header 0 | `phdr-missing`, `Ok(())`, `(Ok(()), 1)` |
| P2 | `lone` admits a second | `phdr-twice`, `two-interpreters`, `two-dynamics`, `Ok(())`, `(Ok(()), 1)` |
| P3 | `PT_PHDR` order unchecked | `phdr-after-load`, `Ok(())`, `(Ok(()), 1)` |
| P4 | `p_offset == e_phoff` unchecked | `phdr-elsewhere`, `Ok(())`, `(Ok(()), 1)` |
| P5 | `p_filesz` unchecked | `phdr-short`, `Ok(())`, `(Ok(()), 1)` |
| P6 | `PT_PHDR` mapping unchecked | `phdr-unmapped`, `phdr-shifted`, `phdr-unloaded`, `Ok(())`, `(Ok(()), 1)` |
| P7 | `mapped` without the translation (`is_some()`) | `phdr-shifted`, `dynamic-shifted`, `Ok(())`, `(Ok(()), 1)` |
| P8 | `holds` by the loaded size | `dynamic-past-file`, `Ok(())`, `(Ok(()), 1)`; `table-past-file`, `Err(Truncated)` |
| L1 | load extent unchecked (`within(offset, 0, …)`) | `load-past-file`, `Ok(())`, `(Ok(()), 1)`; `load-offset-wrapping`, `Err(Truncated)` |
| L2 | alignment unchecked | `load-unaligned`, `Ok(())`, `(Ok(()), 1)` |
| L4 | file ranges unchecked | `loads-sharing-bytes`, `loads-sharing-bytes-apart`, `Ok(())`, `(Ok(()), 1)` |
| L5 | file ranges compared with the next load only | `loads-sharing-bytes-apart`, `Ok(())`, `(Ok(()), 1)` |
| L6 | virtual ranges by bytes, not pages | `loads-sharing-a-page`, `Ok(())`, `(Ok(()), 1)` |
| L7 | load order unchecked | `loads-unordered`, `loads-overlapping`, `loads-sharing-a-page`, `Ok(())`, `(Ok(()), 1)` |
| L8 | file-backed past loaded size admitted | `load-file-over-memory`, `Ok(())`, `(Ok(()), 1)` |
| L9 | address wrapping (`wrapping_add`) | `load-wrapping`, `Ok(())`, `(Ok(()), 1)` |
| N1 | interpreter order unchecked | `interpreter-after-load`, `Ok(())`, `(Ok(()), 1)` |
| N2 | no interpreter taken as `PT_PHDR` | `static-uninterpreted`, `Err(Malformed)` |
| N3 | no dynamic segment admitted (`Ok(Loaded { interpreter, .. })`) | `static-undynamic`, `Ok(())`, `(Ok(()), 1)` |
| N4 | interpreter's inner NUL unchecked | `interpreter-holding-nul`, `Err(Unresolved(Interpreter))` |
| N5 | interpreter's closing NUL unchecked | `interpreter-unended`, `Ok(())`, `(Ok(()), 1)` |
| N6 | empty interpreter admitted | `interpreter-empty`, `Err(Relative(Interpreter))` |
| N7 | interpreter extent unchecked | `interpreter-wrapping`, `Err(Unread)` |
| N8 | interpreter bound ×1000 | `interpreter-over`, `Err(Truncated)` |
| N9 | relative interpreter admitted | `interpreter-relative`, `Err(Unresolved(Interpreter))` |
| D1 | dynamic mapping unchecked | `dynamic-unmapped`, `dynamic-shifted`, `dynamic-past-file`, `Ok(())`, `(Ok(()), 1)` |
| D2 | entries read to the last `DT_NULL` | `after-null`, `Err(Audited)` |
| D3 | `DT_NULL` not required | `dynamic-unended`, `Ok(())`, `(Ok(()), 1)` |
| D4 | dynamic size held to 8 | `dynamic-uneven`, `Ok(())`, `(Ok(()), 1)` |
| D5 | dynamic bound ×1000 | `dynamic-over`, `Err(Malformed)` |
| T1 | delegating entries admitted | `depaudit`, `audit`, `auxiliary`, `filter`, `Ok(())`, `(Ok(()), 1)` |
| T1a–T1d | each of `DT_DEPAUDIT`, `DT_AUDIT`, `DT_AUXILIARY`, `DT_FILTER` dropped from `DELEGATING` alone | that one row, `Ok(())`, `(Ok(()), 1)`, each run apart |
| T2 | a search path twice admitted (`< 3`) | `runpath-twice`, `rpath-twice`, `Ok(())`, `(Ok(()), 1)` |
| T3 | first `DT_STRTAB`/`DT_STRSZ` taken | `table-twice`, `table-conflicting`, `size-twice`, `size-conflicting`, `Ok(())`, `(Ok(()), 1)` |
| T4 | a missing table tag defaulted (`(None, _) => Ok(16)`) | `table-missing`, `size-missing`, `bare-table-missing`, `Ok(())`, `(Ok(()), 1)` |
| T5 | the table's start alone held | `table-unloaded`, `Ok(())`, `(Ok(()), 1)`; `table-crossing`, `table-past-file`, `Err(Truncated)` |
| T6 | the fourth tree's H1 put back (return before `tabled` when nothing is named) | `bare-table-missing`, `bare-table-unmapped`, `Ok(())`, `(Ok(()), 1)` |
| S1 | a `$` in a slashless needed name unchecked | `needed-token`, `Ok(())`, `(Ok(()), 1)` |
| S2 | other `$` tokens admitted | `runpath-token`, `rpath-token`, `Err(Unresolved(SearchPath))` |
| O1 | every segment's extent read, not the loads' alone | `other-unread`, `Err(Malformed)` |
| G0 | the test grant ignored in `checked` | the five table tests: every admitted row `(Err(Identity), 0)` |
| H1 | `fit` without its system-set place | outside-set test `(Ok(()), (Ok(()), 1))`; `fit` test `outside` true |
| R5 | the observation takes the canonical endpoint (`found = resolved.place`) | found-route test (:602) `(Err(Identity), 0)` for the linked positive; real-bubblewrap test (:338) `(Err(Identity), Err(Identity), 0, true, 1)`; observation test (:578) `(1, 1)` |

Two removals survive, and the reason is the list's own: **K7** (`e_phnum >
0` dropped) and **F1** (`at least one PT_LOAD` dropped, `first` taken as
`usize::MAX`) each still refuse `Malformed`, because item 3 requires a
`PT_PHDR` mapped by a load, which neither zero headers nor zero loads can
give. Both checks are kept because items 1 and 4 name them. N2, N4, N6 to
N9, D2, D5, K8, K9 and S2 bind the typed cause only, the image still
refused by a later check: N2 because `Loaded` requires an interpreter, so a
removal of `Static` for the interpreter has nothing to admit. The
clippy-required reshaping of `dynamic`'s return (a `Loaded` in place of a
pair) came after these runs; `diff` against the copy the mutations ran on
showed only that reshaping, and D1 to D5 and T6, whose lines it touched,
were run again on the final tree with the same failures (D1 with D2 and
D3, D4 with D5, each pair on distinct rows).

**L1, the earlier controls on the final tree.** The commission's list was
rerun here against the final fixtures:

| # | Removal | Failed |
| --- | --- | --- |
| S1 | store link count `>= 1` (store.rs `single`) | checked.rs:152 (predicate) and :135, `Ok(())` |
| S2 | store owner unchecked | checked.rs:153 (predicate) and :140, `Ok(())` |
| S3 | store regular-file check dropped | checked.rs:151 (predicate) |
| G1 | store route guard dropped (`admitted`) | checked.rs:219, `(509, Ok(()))` (`0o775`) |
| G2 | any sticky directory accepted (`shut`) | checked.rs:219, `(1023, Ok(()))` (`0o1777`) |
| G3 | ACL ignored | checked.rs:290 (predicate) and :241, `(493, Ok(()))` |
| G4 | a link's mode counted | checked.rs:293, :383, :248, :346, `Err(Identity)` |
| G5 | `/` left out of `guarded` | checked.rs:271 |
| M11 | confined where any writer is (`credited`) | checked.rs:518, `Proved` |
| M12 | harness left out of the writers (`credentials`) | CLI capability_broker.rs:1584 `(Some(1), "error: broker serving protections are incomplete\n")` and observer.rs:126 `Array [Number(1000)]` |
| M13 | no writer read counted confined | checked.rs:524, `Proved` |
| M14 | `sealed` keeps only the observed uids | checked.rs:511 `[1000, 1002]`; launcher own-check test `sealed`, `(Ok(()), 1)` |
| C1 | admission's store refusal discarded (session.rs `after`) | CLI capability_broker.rs:1552 and :876, observer.rs:269, each `"error: broker serving protections are incomplete\n"` |
| C2 | store handle not handed back (`handed`) | fifteen CLI tests, among them observer.rs:324 (`27`) and capability_broker.rs:870 |
| C3 | sealed writers not supplied (`prepared`) | CLI capability_broker.rs:1507, `"error: MCP server box filesystem identity is not protected\n"` |

Each passed again once restored (the gate runs below).

### Gates, fifth tree

`cargo fmt --check` found nothing to change, clippy denying warnings over
all targets and features passed, and the 1.88 toolchain's workspace check
finished. By crate, brokkr-protocol passed 841 unit tests (3 ignored) and
its three integration binaries (6, 1 and 1), the hands module alone 101 (1
ignored); brokkr-cli 627 unit tests (1 ignored), 515 `it`
tests (2 ignored, `capability_broker::` 31 of them), 27
driver-conformance, its three heap budgets and 13 transcript-surface;
brokkr-runtime 882 unit tests with its 75, 122, 1 and 3 integration tests,
no witness or compose input moved; the remaining crates passed with no
failure. `compile --bundle recipes/self` finished. `quality/ratchet.sh
files`, `clones` and `api` held. `quality/ratchet.sh baselines 4881f53e`
(the branch base) reports only `public-api/brokkr-protocol.txt: 944 public
items (was 942)`, `ServerBox::store` and `ServerProfile::writers`, ruled on
2026-10-09; against `origin/main`, now `6cd3e4aa` (#597), it also lists
main's own `ui.rs` and `ui/tests.rs` rows, which go with the rebase. `openspec
validate --all --strict` passed 20 of 20; `typos --hidden`, `git diff
--check` and `git diff --cached --check` printed nothing. No suppression,
`unsafe`, dependency or Cargo change was added.

### Pending

The offline link check (`lychee`) is refused in this seat, so it is not
observed. The suites ran with the seat's inherited `TMPDIR`, which this
seat cannot read or set; the store fixtures ran rather than skipped. The
exact-coverage gate waits on CI outside the box. The FUSE stalled-mount
kind, the macOS arms and native alias creation stay open as before.

### Follow-ups (outside the row)

The page-granular reading of item 4's virtual ranges is this visit's; if
the operator reads "overlap" byte for byte, `loads-sharing-a-page` becomes
an admitted row and L6 its removal.
