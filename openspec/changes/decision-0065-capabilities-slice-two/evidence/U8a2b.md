# U8a2b evidence: exclude every managed writer from admitted server sources

U8a2b was built on branch `s2/U8a2b`, cut from main at `b6475fda` (U8a2c,
#756), recipe `claude-flash-dsh-adv`, in five runs. Run 1
(`0065-slice-two-unit-u8a2b-see-th-d13589ba`) stopped at its review with two
HIGH, two MEDIUM and two LOW findings. Run 2
(`0065-slice-two-unit-u8a2b-see-th-9b9146df`) started from run 1's whole
change, staged uncommitted on main, and delivers the six repairs below.
Its review (at `1fc9f070`) returned the unit to implement with two MEDIUM
and three LOW findings, answered under "Run 2's review, as answered" (commit
`6aee831a`). Its second review (at `6aee831a`) returned it again with two
MEDIUM and three LOW findings, answered under "Run 2's second review, as
answered" (commit `fed26e57`). Its final review stopped at
REVIEW-SECURITY-HOLD on one HIGH (a link between an input's two ends was not
reserved) and LOWs. Run 3 (`0065-slice-two-unit-u8a2b-see-th-2ca7ef77`)
started from run 2's whole change, staged uncommitted on main at `d13ce473`
with 38.3 and 38.4 unticked again, and answered them under "Run 3: every link
on a route" (commit `61a04bbc`). Its review stopped at REVIEW-SECURITY-HOLD
on two HIGH findings (a bare launch name's located route, and bubblewrap's
route, were not reserved), one MEDIUM (route observation under the admission
lock is not bounded) and LOWs. Run 4 (`0065-slice-two-unit-u8a2b-see-th-a06bf67c`)
started from run 3's whole change, staged uncommitted on main at `bd15e9b0`
with 38.3 and 38.4 unticked again, and answered them under "Run 4: located
routes, the launcher and a link's own entry" (commit `c6e3d7aa`). Its review
stopped at REVIEW-SECURITY-HOLD on two HIGH findings (launcher discovery not
reserved through execution; loader inputs before execution), one MEDIUM
(unbounded reads under the lock) and LOWs. The operator then ruled on the
unit's host inputs (2026-10-19, recorded in `design.md`'s D6 and U8a2b).
Run 5 (`0065-slice-two-unit-u8a2b-see-th-a582e9f9`) started from run 4's
whole change, staged uncommitted on main at `bd15e9b0` with 38.3 and 38.4
unticked again, and answers them under "Run 5: the operating system's
roots, every candidate, and the loader's environment" (commit `dff1ab11`).
Its review returned the unit to implement on one MEDIUM (the
operating-system-root proof) and three LOWs, two of them this unit's,
answered under "Run 5's review, as answered", in the commit that carries
this file. Rows that name run 1, run 2, run 3, run 4, `dff1ab11` or an
earlier commit record those; everything else was observed in run 5's seat
on the tree committed with this file. U8a2b is the last of the three units that share tasks 38.3 and 38.4
(U8a2a, U8a2c, U8a2b), so both are ticked with this file, their remote and
host checks kept pending below.

The operator's 2026-10-18 rulings on U9b1's owed items (new units U9c1,
U9c2, U0b and U1d3, and ruling paragraphs under U9b1 and U8b), staged in
`design.md` and `tasks.md` before run 1, ride with this commit exactly as
staged. They are recorded design, not work of this unit.

## Scope

| File | Change |
| --- | --- |
| `crates/brokkr-runtime/src/engine/broker.rs` (row) | Declares the child `sources`. `provisioned` extends the attempt's lease to every fixed server input and every launch (run 4) before any survey spawns, and to the program trees beside them once the surveys answer, before the attempt's directory or any plan is written; every seal of those extensions is folded into a `Cell<Commit>` that `provision_with` owns and marks on the lease whatever provisioning then returns (run 1's L1). `Sealed` no longer carries the fact. `Unprovisioned::Unadmitted` carries MB3's refusal. `Commit` is re-exported for the provisioning tests. Each extension goes through `sources::held_extended`. Run 5: `surveyed` spawns the survey under `env_clear` with `sources::surveying`'s environment. 779 → 800 lines, at the ceiling. |
| `engine/broker/sources.rs` (new private child of `broker.rs`, which this would take past 800 lines; operator rule of 2026-10-11) | `Served` (a source, a directory reserved for its own entries, or a link met on a route), `SYSTEM_ROOTS` (run 5: the operating system's roots), `launchers` (run 5: the box launcher's candidates, `hands::ServerBox::launchers`), `fixed`/`fixed_with` (the fixed inputs as spelled, and, run 5, every launcher candidate), `surveying` (run 5: the survey's environment), `launches`/`launches_with` (run 5: every candidate of each launch, `hands::ServerProgram::candidates`), `served` (the program trees, and each launch's candidates again), `HOPS_MAX`, `Unroutable` (run 5: why a route cannot be reserved), `routed`/`route`/`hop`/`lexical` (each input's route, run 3), `kept` (run 5: every writer compared with the roots first), `writes` (run 4: a link guarded as its own entry), `unserved`, `extended`/`extended_with` (routing beside what the attempt reserved before), `held_extended` (an attempt holding no lease refused once the fence lifts), `holding`, and on Linux the mount-table identity: `Table` (`reaches`, `parts`, `own`), `Part`, `anchored`, `observed`, `read`; off Linux an uninhabited `Table` and an `observed` that refuses. 622 lines (618 at `dff1ab11`; run 5's review: `route` stops at a missing component). |
| `engine/broker/admission.rs` (existing child) | `Reserved.serves: Vec<Served>`, `Reservation::serves`, `Unknown::Serving`, `Unknown::Unconfined`, `Unknown::Unmounted` and (run 3) `Unknown::Unrouted`, which (run 5) carries its `Unroutable` cause, its text on one attribute line; `admit_with` compares the incoming writers with every reserved input after the CR2 comparison and before anything is sealed. `coordination()` moved to `root.rs`, to make room. 777 → 796 lines. |
| `engine/broker/root.rs` (existing child) | The mount observation is the root's own seam, beside its seal hook: `Mounts`, the `mounts` field (production `sources::observed`), `Root::mounts`, and `Root::opened` taking it for a unit test's root; `coordination()` lives here now; `Commit` is `pub(in crate::engine)`. 414 → 450 lines. |
| `engine/broker/exclusion.rs` (existing child) | `Guarded`, `Guarded::of`, `covers`, `beneath`, `Mount` and its fields, `mounts` and `MOUNTINFO` made `pub(super)` for `sources`, unchanged in behaviour: the mount table is parsed in one place. Run 3: the one mount-identity derivation, `mounted_on` (the `statx` mount id) and `part` (the mount of that id, its root joined with the path below its point), which `once`, `shown_once` and `sources::Table::own` all read. 511 lines. |
| `engine/broker/settlement.rs` (existing child) | U8a2c's L1 (run 1) and run 1's L2: every `Commit` variant matched explicitly. 272 → 319 lines. |
| `crates/brokkr-protocol/src/hands.rs` (operator ruling 1, 2026-10-19) | `pub const SERVER_RESOLVER: &str = "/etc/resolv.conf"`, the resolver file a server box binds on a shared network. The file stays at its 1,094-line baseline: the constant and its doc line take the blank line before `HOST_TOOLCHAIN_BINDS`' doc. Run 4 did not touch it: both lookups it exposes reach `hands` as associated functions of types its existing re-export already names. |
| `crates/brokkr-protocol/src/hands/namespace.rs` (operator ruling 2, 2026-10-19; runs 4 and 5) | Run 5: `pub fn ServerProgram::candidates(launch)`, every file a box's lookup of `launch` may select, in order, whether or not one stands (an absolute launch as written, a bare name in each `SERVER_PATH` directory, a relative one holding a `/` none); `ServerProgram::resolve` names the first that is an executable regular file, the rule run 4's public `located` held, which is private again inside `resolve`. `SERVER_PATH` is `pub(in crate::hands)` for the protocol test that pins its directories inside the system set. The private `RESOLV_CONF` constant is gone: `use super::SERVER_RESOLVER as RESOLV_CONF`, under the name `namespace/sources.rs` imports it by, since that file lies outside the ruling. 799 → 799 lines. |
| `crates/brokkr-protocol/src/hands/namespace/sources/launcher.rs` (operator ruling 2; runs 4 and 5) | Run 5: `pub fn ServerBox::launchers()`, every file a server box's launcher may be (`ServerProgram::candidates("bwrap")`: the bare name in each server `PATH` directory), replacing run 4's `launcher()` and `launcher_on`; and `checked` admits a launcher only where it was found as one of those candidates, after its `fit` and `elf` checks and before anything runs it. A test alone extends its `admitted` grant to that membership. 700 → 717 lines. |

`crates/brokkr-runtime/src/engine.rs` (row, frozen at 3,523) did not move.
No other production module tree moved. No new trait, flag, authored key,
daemon, suppression, `unsafe` or dependency. `quality/public-api/brokkr-protocol.txt`
rose by one line under ruling 1, `pub const brokkr_protocol::hands::SERVER_RESOLVER: &str`,
and by three under ruling 2, its limit: `impl brokkr_protocol::hands::ServerBox`
(the impl block in `launcher.rs`), `pub fn brokkr_protocol::hands::ServerBox::launchers() -> alloc::vec::Vec<std::path::PathBuf>`
and `pub fn brokkr_protocol::hands::ServerProgram::candidates(&str) -> alloc::vec::Vec<std::path::PathBuf>`.
Run 5 replaced run 4's `launcher()` and `located` with these two, within the
same four lines, as ruling 2 asks. `quality/ratchet.sh api` held.
`ServerBox::launchers` is Linux-only, as the module that holds it is; off
Linux `sources::launchers` is empty, so `sources::fixed` reserves no launcher
and the survey keeps the engine's `PATH`, since no server box stands there
(decision 0063).

## The mechanism as built

A server box is built from host inputs. The operating system's own roots
(`sources::SYSTEM_ROOTS`: `/usr`, `/bin`, `/sbin`, `/lib`, `/lib32`,
`/lib64`, `/libx32`, `/etc`, `/dev`, `/boot`, `/proc`, `/sys`) are trusted
host state (D6, operator ruling 2026-10-19): every comparison (`kept`) holds
each writer against them, as spelled, before any reserved input, so a writer
at, above or below one is refused with MB3's identity cause naming it. The
roots are not stored in reservations; they are the same for every attempt.
`/usr` comes first so a writer within it is named `/usr` on merged and split
layouts alike. What can lie outside the roots is reserved: the bootstrap,
the toolchain set (`hands::HOST_TOOLCHAIN_BINDS`, of whose `/etc/ssl` a
server box keeps only the certificates, compared whole here, kept by route
so a bind whose route leaves the roots stays covered), the resolver file
whatever the network, the temporary directory its identity is generated
beneath, the box launcher's candidates, each launch's candidates and the
program tree. `sources::fixed` names the first four as spelled, the
temporary directory for its own entries, and (run 5) every candidate of the
launcher's discovery, `hands::ServerBox::launchers`: the bare name `bwrap`
in each server `PATH` directory, whether or not it stands, so a dangling
candidate ahead of the one found is reserved at its first component that
does not stand (since run 5's review; it was reserved where it would lead).
`sources::launches` (run 5) names every candidate of each launch,
`hands::ServerProgram::candidates`: an absolute launch as written, a bare
name in each server `PATH` directory, again whether or not it stands. Both
are reserved before any survey. `sources::served` adds each plan's program
tree, its package root or its one system entry, and each launch's
candidates again.

Run 5: the survey is spawned with `sources::surveying`'s environment under
`env_clear`: the engine's own, without any `LD_*` variable, and its `PATH`
the directories of the launcher's candidates (`/usr/local/bin:/usr/bin:/bin`),
so the survey's discovery (`require_bwrap`, through `Host::provisioning`)
selects among exactly the candidates the lease holds. A broker, which its
harness starts in the operator's own environment (trusted by D5, as the
ruling says), finds its launcher on its own `PATH`, but `launcher::checked`
admits it only where it was found as one of those candidates, after its
`fit` and ELF checks and before its version is asked or it launches
anything. Bubblewrap itself already ran without an environment, through
`env_clear` both for its version (`launcher::version`) and for the launch
(`namespace/entry.rs`), and run 5 binds both.

Each input is reserved by its route (`sources::routed`, run 3). The route is
walked one component at a time, as the kernel resolves it, never by
canonicalizing it whole: every symlink met is reserved by the path it lies
at (`Served::Link`), whether it is the input itself (`/etc/resolv.conf`), a
directory above it (`/lib -> usr/lib`) or a link a link names
(`/usr/bin/x -> /etc/alternatives/x -> …`); then where the input resolves,
as the input it is. A relative link continues from the directory holding
it, `..` from what has resolved so far. A component that does not stand
ends the walk's reads, and the rest is kept as spelled. A route that is not
absolute, meets a component it cannot read, or meets more than D5's
`HOPS_MAX` (40) symlinks refuses with MB3's identity cause,
`Unknown::Unrouted(<input>, <cause>)`, the cause (run 5) `Relative`,
`Unread` or `Hops`, never with a shorter route; the deadline is checked
before every component, not during one (see run 5's bound). The walk runs
inside the extension, under the admission lock and its budget.
The extension keeps every input its attempt reserved before and adds the new
routes, so a route that changes between the two extensions reserves both.

Before the first survey spawns, under the host admission lock and within the
attempt's one startup deadline, the extension reads the protected inventory,
recovers proven attempts as admission does, compares every participant's
writers with the fixed inputs and the launches — every site of every standing attempt: the
attempt's own zero-grant siblings, members and steps, its run's earlier
attempts and other worktrees' runs — and seals the attempt's reservation
again holding them, all before the lock is released. Once the surveys
answer, the same extension runs again with the program trees beside the
fixed inputs, so nothing reserved is dropped. Every later admission compares
its incoming writers with every input a standing attempt reserved. The
inputs leave the reservation only with their attempt, which U8a2a's release
moves to settled only on durable settlement.

A writer conflicts with a source where a root it can write lies at, above or
below it: by canonical path or by device and inode of either and the other's
ancestors (`exclusion::covers`), or by the mount table (H1). For each root
and each source the table gives the parts of filesystems it shows: where it
lies — the mount `statx` names for its nearest ancestor that stands, read in
`/proc/self/mountinfo`, its device and its root within that filesystem
joined with the path below the mount point — and the device and root of
every mount at or below it. Two paths conflict where a part of one holds a
part of the other on the same device, either way. A bind of a source's
subtree elsewhere is therefore caught by its own part, and a source bound
beneath a writable root by that root's nested part. The temporary directory
is reserved for its entries: only a writer whose part holds its own part,
or whose root lies at or above it by path or identity
(`exclusion::beneath`), conflicts, so worktrees inside it stay parallel. A
link is its own entry (run 4): a writer conflicts with it where its root
lies at or above the directory the link lies in, by path or by that
directory's ancestors' identity (`exclusion::beneath`), and the table
compares a writer's parts with the one part of that directory, so a bind of
it elsewhere is caught too. What the link names is reserved as the route's
next input, never through the link's own identity. A
table that cannot be read or parsed, a path on no mount it names, or a mount
whose root is no path refuses with `Unknown::Unmounted`; a table past D5's
mount bound or read past the deadline refuses with MB3's bound and deadline
causes. The table is read once per comparison, beside the reserved inputs,
as the first boxed writer is compared with them; that it is read says
nothing yet of any overlap. Off Linux no table is read, and every
boxed writer beside a reserved input is refused, as exclusion refuses every
boxed participant beside evidence there (decision 0063).

A writer outside a workspace box has no confined privilege, so the survey's
write-exclusion proof, held against the managed writers' mapped uid taken as
confined, does not hold against it; beside any reserved input it is
refused. Every participant runs as the one user whose alone the coordination
root and every reservation in it are, so that uid, sealed in every plan's
`Writers`, is every managed writer's mapped identity.

The refusals take MB3's identity cause, distinct from CR2's artifact-root
cause, which U8a2a's comparison keeps and runs first: `dispatch refused: MCP
server box filesystem identity is not protected: site '<site>' of run
'<run>' can write '<input>', which an MCP server's box is built from`;
`… site '<site>' of run '<run>' writes outside a workspace box, its
privilege not confined, beside an MCP server's sources`; `… the mount
table does not prove where '<path>' lies beside a managed writer`; and `…
the route of '<input>', which an MCP server's box is built from, ` followed
(run 5) by `is not absolute, as a relative or empty PATH entry or a relative
TMPDIR names it`, `meets a component that cannot be read` or `meets more
than 40 symlinks`, the number written from `HOPS_MAX`. At an
incoming writer's admission the attempt fails before its first driver; at a
broker's extension it fails before its attempt directory, so no plan is
written, no broker binds, looks a secret up or starts a server, and no
member starts; refused at the first extension, no survey child is spawned
either.

## Run 1's findings, as repaired

H1 (aliases a bind can hide) is the mount-table identity above, used by
`kept` for every participant and every input, at admission and at both
extensions, so in both start orders. H2 (inputs not reserved, survey before
reservation) is `fixed` and the first extension before any survey; the
resolver is reserved by the public `SERVER_RESOLVER`, which the protocol's
real server-box test pins to the path the box binds. The identity files a
server box generates are covered by reserving the temporary directory for
its entries, with the residual named under Notes. M1 is the matrix below, for
a program source and a system source each. M2 is the removal control in the
controls table. L1 is the `Cell<Commit>` folded on every path; L2 the
explicit arms in `settlement.rs`.

## Run 2's review, as answered

Its first MEDIUM (no removal recorded for
`an_extension_committed_unsynced_says_so`) and its first LOW (six tests
whose own removals were not recorded) are the second controls table below.
Its second MEDIUM (the engine-death proof admitted and extended directly,
never dispatching an engine, and released by a lease the test made) is
`att::a_dispatched_engine_killed_while_its_seat_lives_keeps_its_servers_sources`:
a real engine's dispatch of a brokered seat, killed while its seat lives,
and real dispatches of incoming writers of a program and a system source,
refused until the facts of a proven settlement are recovered by production's
own recovery. The LOW on the unproven `None` lease is now a refusal with its
test (`held_extended`). The LOW on the temporary directory is carried below,
unchanged: closing it needs production files outside this row. The panel's
run-integrity note asks nothing of this unit.

## Run 2's second review, as answered

Its first MEDIUM: the engine-death proof killed a real engine while the
seat's driver lived, but no broker or server ran beneath that seat. It is now
`att::a_dispatched_engine_killed_while_its_server_box_lives_keeps_its_sources`,
beside the seat-only proof, both bodies one function (`att::killed_while`).
The seat's driver is this test binary run again as the seat's broker
(`att::serving_broker`). It reads the plan the attempt's inventory pins,
checks the plan's bytes against the pin, and prepares that plan's server box
on the live host with `hands::ServerBox::prepare`, from the plan's own reach,
network and writers. It launches the box on its entry with
`ServerBox::launch`, with the shell standing for the bootstrap, as the
protocol's own server-box test launches one. The package's program then runs
inside the box and waits. As the CLI suite's `confined()` does for
`brokkr broker serve`, the broker runs under `setpriv --no-new-privs`, beneath
two harness shells. The live observer holds its write-exclusion proof against
itself, its parent and theirs, and refuses a host's multiply-linked system
file otherwise: this test's first run here was refused with MB3's identity
cause until that chain stood.

The test then asserts the following. The server box's first process holds
the program, waiting as `sleep`, once its engine was killed (SIGKILL) and
again after the incoming dispatches. Incoming writers of the package and of
`/usr/bin` fail with MB3's exact text before their drivers start, with the
sources' bytes and the dead engine's reservation unchanged. A disjoint
worktree's dispatch runs beside it. Once the seat is killed the box ends,
the same writers are still refused, and only a proven settlement's facts
release the sources. Both variants now carry the disjoint control. Neither
variant skipped here: bubblewrap builds boxes in this seat, so the box
variant ran (no `skipped:` line in the suite's output). It needs a box, so a
host without one skips it, and there it is "host result pending".

Its second MEDIUM:
`att::a_seal_committed_unsynced_is_kept_whatever_provisioning_refuses_after_it`
now matches `EngineError::Unreleased` and compares it with
`Unreleased::released(Unadmitted::Identity(Unknown::Unsynced))`. The text
stays pinned once, in
`sealing::a_failed_settlement_says_whether_its_lease_was_released`.

Its first LOW, the temporary directory, stays carried under Notes, unchanged.
Its second LOW is "Gates and measurements" below: each figure now names the
commit it was measured at, and the current head's record stands alone. Its
third LOW, the panel's routing note, asks nothing of this unit.

## Run 3: every link on a route

The HIGH: run 2 reserved each input as spelled and where it resolved, and
the mount comparison canonicalized every hop between. A route such as
`/etc/resolv.conf -> /srv/links/resolver -> /opt/resolver/resolv.conf` left
a writer confined to `/srv/links` free to replace the middle link. Each
input is now reserved by its route, as "The mechanism as built" says, for
the bootstrap, every `HOST_TOOLCHAIN_BINDS` source, `SERVER_RESOLVER`, the
temporary directory, each plan's program tree and its launch path. A writer
whose root lies at, above or below any link of the route, or holds the
directory it lies in, is refused. A survey's observation is not relied on,
since a standing writer could swap the link while the survey looked and
swap it back after. The extension walks the route itself, under the lock,
and every link on it stands reserved from then until the attempt settles.

The proofs are listed under "Tests" with the rest (the `Run 3` rows). They
cover a writer confined to a link's directory for three shapes of fixed
route, in both start orders before any survey, with a disjoint control. A
third case of the shared alias matrix, `srv/links`, carries the launch
route's middle link through server first, writer first, simultaneous
starts, the uncertain settlement and the real dispatch whose later step
could write it. Further rows cover the provisioning path in both phases (0
surveys for the bootstrap, 1 for the launch), the swap between the survey
and the extension, a bind of the link's directory, a route kept across
extensions, and the exact routes, bounds and refusals.

The LOWs. The mount-identity derivation is one (`exclusion::mounted_on`
and `exclusion::part`). `sources::Table::own` reads it and `parts` adds the
nested mounts to it, so a link's directory, `/` say, is placed without
reading every mount beneath it. The owed removal control for
`a_dispatch_whose_later_step_could_write_a_reserved_source_starts_nothing`
is the first row of the run-3 controls table. `writers_tests::served_box`
keeps the protocol's `Refusal` typed through its setup (`Unserved::Refused`,
or `Unserved::Unready` with how the box's first process ended), and
`serving_broker` renders only that child's diagnostic, its `Display`. The
INFO on `setpriv`: discovery moved to the parent test (`writers_tests::setpriv`),
which skips the server-box proof naming the missing `setpriv` the way
`boxes_here` skips a missing namespace, and passes the path down in the
fixture's `payload` file, so no child unwraps it. The INFO on `kept`: its
doc and the mechanism above now say the table is read once, beside the
reserved inputs, as the first boxed writer is compared, without implying an
overlap was found. The LOW on the launcher's temporary directory is carried,
not repaired (Notes).

One more repair, to a test of run 2's. The engine-death proofs failed now
and then under the full parallel library suite, once at their final
`kill_process` of the seat (`ESRCH`: the seat was already gone). The cause
is how an orphan is adopted. Once the killed engine is gone the seat goes
to its nearest subreaper, and a test binary is one once any attempt has run
in it. A concurrent test's attempt, begun before the seat was born, could
then claim the seat as its stray and end it. Each death proof now runs in a
test process of its own (`writers_tests::killed_engine`, through
`in_a_process_of_its_own`). That process makes itself the subreaper before
it dispatches, so the seat is its own, and every attempt in it begins after
the seat. It reaps everything it adopted before it exits (`reaped`), so
nothing passes to the parent binary's attempts. As run 2 left it, 2 of 3 full
library runs here failed the seat variant. In a process of its own but not
yet its subreaper, 1 of 3 failed it. With the subreaper, 14 full runs passed
the seat variant. The server-box variant passed 13 of them and failed once,
during a window when the host was loaded (that run took 27 s, against 14 to
18 s). It failed at `in_a_process_of_its_own`'s status assertion, and its
child's output was not captured; the next five full runs, saved to scratch
logs, passed it. That one failure is unexplained, and is carried under
Notes.

## Run 4: located routes, the launcher and a link's own entry

Everything run 3 repaired is kept as it was: the explicit inputs'
intermediate hops, the shared mount derivation, the later-step removal
control, the typed setup errors, the named `setpriv` skip, `kept`'s
documentation and unsynced-commit propagation, and U8a2c's three LOWs.

The first HIGH: `served` reserved a launch only where its argv[0] was
absolute, so a bare name the box locates on its search path, as
`/usr/bin/server -> /srv/links/server -> /opt/docs/bin/server`, left a
writer confined to `/srv/links` free. The launch is now located by the box's
own lookup, `hands::ServerProgram::located` (ruling 2: the function the
survey and the broker call through `ServerProgram::resolve`, made public,
never copied), and its whole route reserved by `routed`. It is reserved
before any survey, from the attempt's seals, and located and walked again in
`served` once the surveys answer, so a launch that changed between the two
is reserved both ways. It stays reserved until durable settlement with every
other input. The directories the lookup searches ahead of the one it finds
need no reservation of their own: each lies in a reserved system source
(`/usr/local`, `/usr/bin`, `/bin`), which
`server::every_server_search_directory_lies_in_a_system_source` pins.

The second HIGH: the box launcher's route was checked only against the
selected seat's reach. The engine now reserves, before any survey, the
launcher by the box's own discovery, `hands::ServerBox::launcher` (ruling
2), and every directory searched before it, for its own entries. The
discovery is `hands::bwrap_on` over a `PATH`, the function `require_bwrap`
runs and so the observer's `Host::with` (`namespace/sources/host.rs:430`).
The authoritative one is the survey's. The survey is spawned by
`broker::surveyed` with `Command::new(bootstrap)` and no `env_clear`, so it
inherits the engine's `PATH`, and the engine's call finds exactly what its
survey finds: `the_launcher_is_found_beside_every_directory_searched_before_it`
pins `ServerBox::launcher()` to `require_bwrap()` and to `launcher_on` over
this process's `PATH`. A serving broker finds its launcher on its own
`PATH`, which its harness sets and may differ. It does not launch whatever
it finds there: the launcher's found path and every hop of its route are
digested into the observation the plan seals (`namespace/sources.rs`'s
`served` lists the launcher by its found path; `digest` covers each place
and link target), and `broker/session/readiness.rs`'s `compared` refuses
with MB3's identity cause unless the broker's observation equals the sealed
one. A broker whose `PATH` finds another launcher, or the same one by
another route, is refused before it launches anything. Off Linux no server
box stands and none is reserved.

(Run 5 corrects that claim: run 4's review found it false. The serving
broker's observation runs the launcher's `--version` (`launcher::ran`)
before the CLI compares the sealed observation, and a dangling candidate
skipped by `bwrap_on` could be made live after admission. Both are
answered under "Run 5".)

Through this, the existing launcher admission, and the launcher run only
through the handle it was checked by, are kept unchanged.

The MEDIUM, not repaired: route observation is not bounded. `route` checks
the deadline before each `hop()`, but `symlink_metadata` and `read_link`
run synchronously on the admitting thread while `extended_with` holds the
admission lock. So do the comparison's own identity reads (`Guarded::of`,
`canonicalize`, the `statx` mount id) and, as before this unit, the
inventory's reads. A call that stalls holds the lock until it returns,
outrunning the absolute startup deadline and blocking other admissions. No
house pattern bounds it inside these files. A thread with a timeout would
leave the blocked call on a detached admission worker, which D5 and SD4
forbid. The pattern that does bound one is `broker::surveyed`'s: an
observer process in a group of its own, killed and reaped at the deadline.
Putting route observation there needs a new `brokkr broker` verb in
`brokkr-cli`, outside this row and both rulings. This is recorded as a
limitation, and the `sources.rs` module documentation now says so. It is
not claimed bounded, and no blocked-operation proof exists for it. (Run 5:
the operator's 2026-10-19 ruling settles it. See "Run 5".)

The LOW on link identity: `writes` guarded a `Served::Link` through
`exclusion::covers`, whose metadata follows the link, so with
`users -> var/users` a writer below `var/users` beside the server's tree
was refused by the link. A link is now its own entry. It is written by a
root at or above the directory it lies in (`exclusion::beneath` over that
directory's identity, as the mount table already compared it), and what it
names is reserved as the route's next input.
`every_fixed_input_is_reserved_before_the_survey` now names a writer
through a link toward the resolver by where the resolver resolves, which it
writes, not by the link at `/etc/resolv.conf`, which it cannot replace.

The LOW on measurements: every row recorded under "At this head (run 4)"
was measured with `wc -l` at the final tree, and `quality/file-lines.txt`
carries the same counts. Run 3's head carried `hands/tests/server.rs` at its
main baseline of 1,005 though it had grown. It is 1,069 now.

The carried LOW (identity tree under the serving process's `TMPDIR`) is the
controller's for U9b1, and stays under Notes.

Run 4's table of every host path resolved is superseded by run 5's, below.

## Run 5: the operating system's roots, every candidate, and the loader's environment

Everything run 4's review confirmed repaired is kept as it was: the direct
located-launch and launcher routes (now every candidate's), link-entry
identity, the measurements, the shared mount derivation, typed errors and
unsynced propagation, and U8a2c's three LOWs. The operator's ruling of
2026-10-19 (D6 and U8a2b in `design.md`) draws the line of "every input".
It is built as written.

The operating system's roots. `kept` compares every managed writer with
`SYSTEM_ROOTS`, as spelled, before any reserved input, through the same
`writes` a reserved source goes through: by canonical path, by identity of
either and the other's ancestors (`exclusion::covers`), and by the mount
table's parts. So an alias or a bind of a root cannot hide one. A writer at,
above or below one is refused with MB3's identity cause naming the first
root it reaches. Since `kept` runs in the first extension, before any survey
spawns, and in every later admission, the refusal precedes the survey in
both orders. Whatever resolves beneath the roots therefore needs no
reservation of its own. That covers the dynamic loader and its
configuration (`/etc/ld.so.preload`, `/etc/ld.so.cache`, `/etc/ld.so.conf`
and `.d`), the system libraries and the bootstrap's standard dependencies,
the system programs, and the device nodes and `/proc` that bubblewrap's
`--dev /dev` and `--proc /proc` give a server box (run 4's L2). The roots
are compared at every comparison, not stored in reservations.

Every candidate (run 4's H1). The engine reserves, by its whole route and
before any survey, every candidate of the launcher's discovery
(`ServerBox::launchers`) and of each launch's lookup
(`ServerProgram::candidates`), dangling ones included. A dangling one is
reserved at its first component that does not stand, where `route` stops
(run 5's review, L1; at `dff1ab11` it walked on as spelled). A writer able
to make it live is refused.
The guidance asked that the server box's bubblewrap be found on the server
`PATH` by engine, survey and broker alike. The engine and the survey do
that: the survey is spawned with `PATH` set to the candidates' directories.
The broker was first built that way too, but that took away the only way
the CLI broker suite has to plant a launcher or make a box unavailable (its
`serve_unboxed` and the observer's planted `stall/bwrap` both set the
broker's `PATH`). Four existing CLI proofs failed on it
(`ledger::a_box_that_cannot_stand_closes_failed_with_no_call`,
`only_the_protected_inventory_binds_a_plan`,
`observer::a_launcher_a_managed_writer_owns_refuses_before_it_ever_runs`,
`readiness::unavailable_support_refuses_before_any_lookup_or_start`), each
reaching `broker retained-response support is incomplete` instead of its
cause. So the broker keeps finding its launcher on its own `PATH`, the
operator's environment the ruling trusts. `launcher::checked` refuses, with
MB3's identity cause and before the launcher ever runs, one found as any
file but a candidate. No serving `PATH` can therefore select an unreserved
launcher: what runs is always a reserved candidate. Where the broker's
`PATH` orders two present candidates differently from the server `PATH`, it
selects another reserved candidate than its survey did. Its observation then
differs from the sealed one, and readiness refuses it before launch. That
refuses more, never less. The four CLI proofs pass again unchanged.

The loader's environment (run 4's H2). `broker::surveyed` spawns the survey
under `env_clear` with `sources::surveying(std::env::vars_os(), …)`: every
inherited variable but `LD_*`, and `PATH` replaced as above. Bubblewrap for
a server box already started with no environment at all. Its `--version`
runs through `launcher::version`, and the box's launch through
`namespace/entry.rs`, both under `env_clear`. Run 5 binds both. The harness
and its broker inherit the operator's environment, which D5 trusts. The
bootstrap is trusted with its sealed bytes.

The bound (run 4's MEDIUM). Under the ruling, admission's host reads under
its lock — the inventory, the mount table and the routes — are bounded as
the inventory's already were: by D5's count limits (1,000,000 inventory
entries, depth 64, `HOPS_MAX` = 40 symlinks a route, 65,536 mount records)
and by the attempt's startup deadline, checked between calls, never during
one. These reads are not cancellable. A stalled filesystem call holds the
admission lock until it returns, so other admissions wait and then refuse at
their deadlines. It never admits a writer. No separate cancellable observer
is built, as the ruling says. The `sources.rs` module documentation says
exactly this.

The LOWs. `Unrouted` now carries its cause (L3): `Relative`, for a route
that is not absolute, the shape a relative or empty `PATH` entry or a
relative `TMPDIR` gives; `Unread`; `Hops`, its text written from
`HOPS_MAX`. After run 5 no `PATH` entry reaches a route: the candidates'
directories are the fixed server `PATH`. So the relative and the empty cases
are bound at `routed` directly, by an `Entries` input spelled `hops/file`
and one spelled empty. The INFO: the `SERVER_PATH` containment test compares
by `Path` components and asserts `/usr/localx` lies in no `/usr/local`.

Every host path resolved while a server box is surveyed, built and launched:

| Path | Resolved by | Reserved, or why not |
| --- | --- | --- |
| The operating system's roots and everything beneath them: the loader and `/etc/ld.so.preload`, `/etc/ld.so.cache`, `/etc/ld.so.conf(.d)`, system libraries, system programs | the kernel and loader, for the survey, bubblewrap and the box | `kept`: no managed writer may reach a root at, above or below it, before any survey (ruling 3). Nothing beneath needs a reservation of its own. |
| Bubblewrap's `--dev /dev` device nodes and `--proc /proc` | bubblewrap, building every server box | Under the `/dev` and `/proc` roots: refused as above (run 4's L2). |
| The bootstrap (`Host.bootstrap`, the canonical current binary) | the engine, spawning the survey; the observer, opening it; bubblewrap, binding it through its checked handle | `fixed`: a source, by its route, before any survey. Its own standard libraries lie under the roots; it is otherwise trusted with its sealed bytes (D5). |
| Every candidate of the box launcher, `/usr/local/bin/bwrap`, `/usr/bin/bwrap`, `/bin/bwrap`, dangling ones included | the survey (`require_bwrap` on the server `PATH` the engine sets); a serving broker (on its own `PATH`, admitted only as a candidate) | `fixed`: each by its whole route, before any survey. A candidate whose route stays under the roots is covered by them too. A launcher found as any other file never runs (`checked`). |
| The launcher's own loader inputs: program interpreter, `DT_NEEDED`, `DT_RUNPATH`/`DT_RPATH` | the loader, when the survey and broker run it | Launcher admission requires each in the protected system set, which lies under the roots. |
| Every candidate of each launch, in each server `PATH` directory, or the absolute launch | `ServerProgram::resolve`: the survey's, and the broker's again at serve time | `launches`: each by its whole route, before any survey, again in `served`. |
| The executable and each program tree | the survey's `resolve` and `layout` (never run); bubblewrap binds the tree | `served`: the tree a source by its route, before any plan is written. |
| `HOST_TOOLCHAIN_BINDS` and their canonical aliases | bubblewrap's `--ro-bind-try` binds; the observer | Under the roots, and also `fixed` by route, so a bind whose route leaves the roots stays covered. |
| The resolver (`SERVER_RESOLVER`) | bubblewrap, on a shared network | `fixed`: a source by its route, whatever the network. |
| The identity tree | generated under the serving process's temporary directory; the survey's under the engine's, which it inherits | `fixed`: the engine's temporary directory, for its entries. **The carried LOW**: a broker whose `TMPDIR` differs is not covered (U9b1). |
| Lookups inside the box (its fixed `PATH`, the program's own loads) | the box | Not reserved separately. They resolve only within the box's binds, which are reserved above. |
| `/proc/self/mountinfo`, `/proc/sys/fs/overflowuid`, `/proc/<pid>/status` | the engine and the observer | Under the `/proc` root. |
| The operator's store and the attempt's control roots | the broker resolves the store's own route at serve time (MB4) | Not box sources: the box mounts neither, and excludes both. The control root is the coordination root U8a2a protects. |
| The trusted HOME | the survey, classifying a program tree | Not a box source, and never mounted. |

## Run 5's review, as answered

The review of `dff1ab11` found production holding all twelve roots and
refusing every writer it tried; its findings are about the proof and one
route shape.

M1, the operating-system-root proof. Two defects, both answered in
`adm::src::a_writer_reaching_an_operating_system_root_is_refused_before_the_survey`.
First, its cases and its oracle came from production's `SYSTEM_ROOTS`, so a
root omitted there left the matrix too. The test now writes the operator's
twelve roots itself (`OPERATOR_ROOTS`, one string, in production's order),
builds every case and the oracle from it, and ends by pinning `SYSTEM_ROOTS`
equal to it. Replacing `/dev` in `SYSTEM_ROOTS` with a second `/usr`
compiles and fails the matrix at `/dev/brokkr-u8a2b`, before the pin
(removal controls below). Second, the oracle `first_root` compared canonical
paths only, while production also compares identities and the mount
table's parts. The reviewer's box binds `/bin` from `/usr/bin`, so
production named `/usr` where the oracle named `/bin`. The test now shows
the mount table itself (`Shown`), whatever the host's own binds: one
filesystem at `/`, and `/etc` bound at `sysalias` beside the HOME.
`first_root` takes the same binds and reads each path where it resolves,
then through the longest bind it lies in. A writer within `sysalias` is
refused naming `/etc`, which only a bind-aware oracle names. Each root's
own case is named where it resolves on the host: on a host whose `/bin`
resolves into `/usr`, as here, `/bin`'s cases are named `/usr`, and its
omission is caught by the pin rather than by the matrix. The other uses of
`first_root` (`aliases`' system case, `every_fixed_input…`) read the real
table and pass no bind.

L1, an absent component then `..`. `route` walked on lexically past a
component that does not stand, so `srv/a/missing/../../x` reserved
`srv/x`, although a writer confined to `srv/a` could make `missing` a link
and lead the input to `srv/a/x`. `route` now stops at the first component
that does not stand and reserves it, as the input's kind. No managed writer
can make that component stand without writing the directory it would lie
in, which lies above it, so every route its rest could take is covered.
This refuses more, never less: with no `..` after it, everything at, above
or below the path it walked on to lies at, above or below that component.
`every_link_on_an_inputs_route_is_reserved` binds the counterexample. Every
other dangling route the suites assert misses only its last component, so
its reservation is the same path as before, and those tests pass unchanged.
Two expectations moved: that test's
link toward `missing/../opt`, now reserved at `missing` rather than `opt`,
and `every_fixed_input…`'s absent bootstrap (`/opt/brokkr-u8a2b/bin/brokkr`),
now reserved at `/opt/brokkr-u8a2b`.

L2. The dangling test asserts its target's `symlink_metadata` is exactly
`Err(NotFound)`, so a permission or other observation failure no longer
counts as absence.

L3, the identity tree under the serving process's `TMPDIR`, stays carried
to U9b1 as before (Notes and residuals). L4 is the review's ruling on its
own panel's prose and asks nothing of this unit. The INFO on the broker's
operator-`PATH` discovery is fail-closed and unchanged.

Production moved only in `engine/broker/sources.rs` (`route` and two doc
paragraphs), 618 → 622 lines. No public item, witness pin, suppression or
other production file moved.

## Tests

`adm::src` is `engine::broker::admission::tests::sources` (Linux only, as it
reads the mount table; registered from `admission/tests.rs`); `att` is
`engine::capability_tests::provision_tests::brokered_tests::writers_tests`;
`adm` is `engine::broker::admission::tests`; `server` is the protocol's
`hands::tests::server`. The matrix lives in the library's own tests: a server
is provisioned under a held lease only when a bundle may hold an MCP grant,
which the compile fence refuses to every public engine until U9b1, and the
proofs need the library's seams (an injected root and mount table, its seal
hooks). The owning suites `tests/capability_broker_launch.rs` and
`tests/capability_ledger.rs` name these tests in their headers.

| Requirement (38.3/38.4) | Program source | System source |
| --- | --- | --- |
| Server first | `adm::src::a_writer_that_can_write_a_reserved_source_is_refused_before_dispatch` (a link toward the package, a directory inside it, one holding it); `adm::src::a_dispatch_whose_later_step_could_write_a_reserved_source_starts_nothing` (a real dispatch) | the same two tests, a link toward `/usr/bin` |
| Writer first | `adm::src::a_broker_whose_sources_a_standing_writer_can_write_is_refused_before_lookup`; `att::a_standing_writer_of_a_servers_input_refuses_the_broker_before_lookup` (run 4: refused before any survey, naming the launch that lies in the package) | the same two tests (refused before any survey) |
| Simultaneous starts | `adm::src::simultaneous_starts_admit_at_most_one_of_a_conflicting_pair`, both orders | the same test, second case |
| Engine death, payload alive | `att::a_dispatched_engine_killed_while_its_seat_lives_keeps_its_servers_sources` (its seat's driver alive) and `att::a_dispatched_engine_killed_while_its_server_box_lives_keeps_its_sources` (a real server box alive beneath the seat, launched by `att::serving_broker`), each through its child `att::serving_dispatch` (a real engine's dispatch, killed) | the same two tests, a link toward `/usr/bin` |
| Uncertain settlement, free lock | `adm::src::a_free_lock_or_an_uncertain_settlement_releases_no_source` | the same test, second alias |
| Disjoint worktree runs | the closing admission of each server-first test, the disjoint standing run of the writer-first test, and a disjoint dispatch beside each dead engine's living payload | the same |

Since run 5 the system column's writer, a link toward `/usr/bin`, is refused
by the operating-system root it reaches, named `/usr` (in `adm::src`
through `first_root`, the test's own reading of the host's layout, given no
bind), before
any reserved input is compared. The same rows therefore prove the roots'
refusal server first, writer first, for simultaneous starts, through engine
death and through uncertain settlement.

Since run 3 the `adm::src` rows above prove a third case beside the two
columns, through the same tests: `aliases` adds a link toward `srv/links`,
which holds the launch route's middle link (`launch -> srv/links/server ->
<package>/bin/server`) and neither the launch as spelled nor the program.
Each of those tests asserts its refusal naming `srv/links/server`, the
program's bytes unchanged. That covers server first, writer first,
simultaneous starts, the uncertain settlement and the dispatch whose later
step could write it.

What each asserts beyond the table: the server-first tests assert the exact
`Unknown::Serving` (site, run, input), that nothing of the refused attempt
was sealed, and the source's bytes unchanged; the dispatch test that no
step's driver started. The writer-first test asserts the extension's exact
refusal and the attempt's reservation byte for byte as it stood (through
`writer_first`); `att`'s asserts `EffectFailed` with the exact text, the
keeper never started, the run's directory holding only `reservation.json`
(no plan for a broker to bind, so no lookup and no server start), and the
survey count (0 for the bootstrap's directory, `/usr/bin` and, since run 4,
the package, whose launch is reserved before any survey; a writer of the
package is then named by that launch). The engine-death test starts this test binary again as a real
engine (`serving_dispatch`) that dispatches the single brokered Claude seat
with the fence lifted, through `execute`: admitted, its lease extended to
its server's fixed inputs, surveyed, extended to the package, its plan
written and its driver spawned. That driver is a shell that writes its pid
and waits, or, in the server-box variant, the seat's broker, whose box serves
the package's program. The test kills that engine with SIGKILL while the
seat lives, asserts the seat (and the box) outlived it, and
then dispatches, through `execute` on a real engine of another worktree,
a zero-grant run whose box binds a link toward the package, and one toward
`/usr/bin`: each fails with `EffectFailed` and MB3's exact text naming its
own site, run and the source, its driver never starts, and the source's
bytes and the dead engine's reservation stay as sealed — while the seat
lives, and again once it was killed, for a death proves no settlement (the
admission lock was free each time: each was refused by the source, not by
`Expired`). A dead engine never proves its attempt settled itself; the
release is shown from the facts an engine leaves once it did, its terminal
event journaled and its positive settlement fact sealed, as a crash just
before its release leaves them (the same facts the recovery suite puts
back): the next incoming dispatch's own admission recovers them through
production's `recovered`, and its driver runs (`EffectSucceeded`). The
uncertain-settlement test asserts the same for a lease dropped
unreleased, one whose positive fact was sealed but never released, and one
whose cleanup was witnessed unresolved, each with the lock taken and freed,
and that the reservation still names the package; a release on durable
settlement frees it.

The other proofs:

| Test | What it asserts |
| --- | --- |
| `adm::src::a_servers_fixed_inputs_are_the_bootstrap_the_system_and_the_resolver` | Run 3: `fixed` is the bootstrap, then every toolchain source and the resolver, each as spelled, then `Entries(<temporary directory>)` as spelled; each is reserved by its route. Run 5: then every launcher candidate, standing or not, as `Source` (`fixed_with`'s exact list with two candidates and with none), and `fixed` is `fixed_with` over `ServerBox::launchers()`. |
| `adm::src::a_writer_reaching_an_operating_system_root_is_refused_before_the_survey` (run 5, ruling 3; run 5's review, M1) | The twelve roots are the test's own literal, `OPERATOR_ROOTS`, the operator's list in production's order, never read from `SYSTEM_ROOTS`. For each, a writer at it and one below it (`<root>/brokkr-u8a2b`), one at `/` above them all, and one within `sysalias`, a bind of `/etc` the injected mount table shows beside the HOME, each its reach injected: server first, an incoming writer is refused naming the first root it reaches as `first_root` reads it (each path where it resolves, then through the longest bind it lies in), nothing of it sealed; writer first, it refuses the lease's first extension of `fixed`, before any survey, its reservation as it stood. A writer apart (`srv`) beside them extends. Last, `SYSTEM_ROOTS` equals `OPERATOR_ROOTS`, in order. Nothing of the roots is written. |
| `adm::src::a_dangling_earlier_candidate_is_refused_before_anything_launches_through_it` (run 5, run 4's H1) | For the launcher's discovery (`fixed_with`) and a bare launch's (`launches_with`), candidates `[early/<name> -> srv/later/<name>` dangling, `usr/bin/<name>]`: a writer confined to `srv/later` is refused server first naming `srv/later/<name>`, nothing of it sealed, and refuses the extension before any survey writer first; one confined to `srv/other` is admitted beside and extended beside (`refused_in_either_order`); the candidate stays dangling, its target's `symlink_metadata` `Err(NotFound)` exactly (run 5's review, L2). |
| `adm::src::a_survey_starts_without_the_loaders_environment_on_the_launchers_directories` (run 5) | `surveying` over `LD_PRELOAD`, `PATH=/opt/early:/usr/bin`, `HOME`, `LD_LIBRARY_PATH` and `OLD_LD_PATH` with two candidates is exactly `[HOME, OLD_LD_PATH, PATH=/usr/local/bin:/bin]`; with none, the inherited `PATH` kept in place; and the production candidates' directories join to `/usr/local/bin:/usr/bin:/bin`. |
| `survey_tests::a_survey_is_spawned_without_the_loaders_environment` (run 5, `engine::capability_tests::provision_tests`) | In a child of the test binary whose environment holds `LD_BROKKR_U8A2B` (and cargo's own `LD_LIBRARY_PATH`) and a `PATH` led by `/opt/brokkr-u8a2b`, `surveyed` spawns a stand-in that records `env`: it answers `Unanswered`, and the record holds no `LD_*` line and exactly `PATH=/usr/local/bin:/usr/bin:/bin`. |
| `launcher::a_launcher_found_as_no_reserved_candidate_never_runs` (run 5, protocol `hands::tests::launcher`) | `ServerBox::launchers()` is exactly `/usr/local/bin/bwrap`, `/usr/bin/bwrap`, `/bin/bwrap`. This host's own bubblewrap found through a link `early/bwrap` in a fixture directory, as a serving `PATH` could lead to it, is refused `(Err(Identity), 0)`, never run; found as its candidate, `(Ok(()), 1)`. Skipped, with its reason, where the host's bubblewrap is no candidate. |
| `launcher::the_launchers_version_runs_with_no_environment` (run 5) | A stand-in script run through its handle as `launcher::version` runs bubblewrap reports, from `env`, no variable this process holds but those its own shell sets. |
| `server::a_server_box_stands_without_the_seats_or_the_hosts_private_paths` (run 5's assertion) | Beside its existing checks, bubblewrap's own `/proc/<pid>/environ` is empty once the box reports ready, so no `LD_*` reached it; the helper reports variable names only, never values. It ran here in a real box. |
| CLI `capability_broker::observer::a_launcher_a_managed_writer_owns_refuses_before_it_ever_runs` (unchanged, cited) | End to end, a broker whose own `PATH` leads with a planted `stall/bwrap` refuses with MB3's identity cause, and the planted launcher never ran. |
| `adm::src::every_link_on_an_inputs_route_is_reserved` (run 3) | `routed`'s exact list for a resolver-like chain (a relative link, then an absolute one), a directory link above the input (`lib -> usr/lib`), an alternatives chain (`usr/bin/x -> ../../etc/alternatives/x -> ./../../usr/lib/x/libc`, its final already reserved, so once) and a dangling link toward `missing/../opt` reserved for its entries: every link by its path, then each final as its input's kind; a link given as an input routes as a source. Since run 5's review (L1): the dangling link's route is reserved at `missing`, and `srv/a/missing/../../x` as `Source(srv/a/missing)`, never `srv/x`; a writer confined to `srv/a` is refused by `kept` naming `srv/a/missing`. |
| `adm::src::a_route_past_its_bound_or_unread_refuses_and_reserves_nothing_less` (run 3) | A chain of exactly `HOPS_MAX` links routes whole (41 entries, the file last); one more → `Unrouted(<input>, Hops)`; an input spelled `hops/file` and one spelled empty → `Unrouted(<spelled>, Relative)` (run 5, L3); one beneath a regular file (`ENOTDIR`) → `Unrouted(<input>, Unread)`; a spent deadline → `Expired`; an extension whose bootstrap route is over the bound → `Identity(Unrouted(…, Hops))`, the reservation byte for byte as it stood. |
| `adm::src::a_writer_confined_to_a_link_on_a_fixed_route_is_refused_in_either_order` (run 3) | For a chain of file links (`boot1 -> srv/links/brokkr -> …`), a directory link toward a directory link (`lib -> srv/links/lib -> ../../usr/lib`) and an alternatives chain (`usr/bin/brokkr -> ../../etc/alternatives/brokkr -> …`), each the bootstrap of the first extension (the one before any survey): a writer confined to the directory holding the middle link is refused server first and refuses the extension writer first, naming that link; one confined to `srv/other` is admitted beside and extended beside. |
| `adm::src::a_link_swapped_between_the_survey_and_the_extension_still_refuses` (run 3) | A standing writer confined to `srv/links`, kept from every fixed route, swaps the launch's middle link toward another program while the survey resolves the launch (which sees the swap), and back; the program extension is refused naming that link. |
| `adm::src::a_bind_of_the_directory_a_link_lies_in_is_refused` (run 3) | Injected table: a writer whose root is a bind of `srv/links` is refused naming `srv/links/server`; one whose root binds a directory beside it is kept. |
| `adm::src::an_extension_keeps_every_input_reserved_before` (run 3) | A route walked through `srv/links/brokkr` at the first extension, replaced by a plain file before the second, stays reserved: a writer of `srv/links` is refused after both. |
| `att::a_standing_writer_of_a_link_on_a_servers_route_refuses_the_broker_before_lookup` (run 3; run 4's counts) | Through provisioning: a standing writer confined to `srv/links` refuses the keeper's attempt naming `srv/links/brokkr` (on the bootstrap's route) or `srv/links/server` (on the keeper's launch route), each with 0 surveys, the keeper never started and only `reservation.json` written; one confined to `srv/apart` is kept apart and the keeper runs. |
| `adm::src::a_bare_launch_name_is_reserved_by_the_route_it_is_located_by` (run 4, the first HIGH) | The launch `server`, located as `usr/bin/server -> srv/links/server -> opt/docs/bin/server` in a tree of the test's own (through `launches_with`): a writer confined to `srv/links` refuses the extension writer first, naming `srv/links/server`; one confined to `srv/other` does not. The two started together, in both orders, admit at most one (`raced`). With a writer of `srv/links` standing, the launch `later`, a link straight to the program, extends; repointed through `srv/links` between that extension and the next (the survey's window), the next refuses naming `srv/links/server`. Server first, an incoming writer of `srv/links` is refused, and still refused once the engine holding the lease died without releasing it. The program's bytes are unchanged. |
| `adm::src::every_candidate_of_a_launch_is_reserved` (run 5, replacing run 4's `a_launch_is_located_as_its_box_locates_it`) | `launches` over production's `ServerProgram::candidates`: `sh` → its three server `PATH` candidates; an absolute launch naming no file → as spelled; a bare name naming none → its three candidates; a relative one holding a `/`, and an empty argv → nothing. |
| `att::a_launch_swapped_during_its_survey_refuses_the_extension_after_it` (run 4) | Through `provision_with`: another worktree's writer confined to `srv/links` standing, the keeper's launch a link straight to its program. The survey's stand-in repoints the launch through `srv/links`, and provisioning fails with MB3's exact text naming `srv/links/server`, its attempt directory never made. Unswapped, it provisions. |
| `adm::src::the_box_launchers_route_and_every_candidate_before_it_are_reserved` (run 4, the second HIGH; renamed in run 5) | The launcher found as `srv/links/bwrap -> opt/bwrap/bwrap` after a candidate `early/bwrap` that does not stand (through `fixed_with`): a writer confined to `srv/links` is refused server first naming `srv/links/bwrap`, and refuses the extension writer first; one confined to `early` likewise, naming `early/bwrap` (run 5: the candidate, where run 4 reserved the directory's entries). One confined to `srv/other` is admitted beside and extended beside, nothing of a refused attempt is sealed, and the launcher's bytes are unchanged. Its body is `refused_in_either_order`, shared with the dangling test. |
| `adm::src::a_link_is_guarded_as_its_own_entry_not_as_what_it_names` (run 4, the link LOW) | The route of `users/u/srv/bin/server` through `users -> var/users` is exactly `[Link(users), Source(var/users/u/srv/bin/server)]`. A writer of `var/users/u/other` is kept; one holding the link's directory is refused naming `users`; one of the program's directory is refused naming the program. |
| `server::every_server_search_directory_lies_in_a_system_source` (run 4; run 5's INFO) | Every directory of `SERVER_PATH` lies in a `HOST_TOOLCHAIN_BINDS` source by `Path` components: the list of those that do not is empty, and `/usr/localx` lies in no `/usr/local`. |
| `adm::src::every_fixed_input_is_reserved_before_the_survey` | A standing writer of the bootstrap's directory, of the temporary directory itself, or of a link toward `/etc/resolv.conf` refuses the first extension naming exactly that input: since run 4, for the resolver, the file it resolves to, which that writer writes; since run 5's review, for the bootstrap, which does not stand there, its first missing component (`/opt/brokkr-u8a2b` on this host, computed from the bootstrap's ancestors). |
| `adm::src::a_bind_of_a_sources_subtree_is_refused_in_either_order` | H1, injected table: a root within a bind of the package's `bin` is refused server first and refuses the extension writer first; within the same bind of a directory beside the package, admitted and extended. |
| `adm::src::a_source_bound_beneath_a_writable_root_is_refused_in_either_order` | H1, injected table: a root holding a bind of the package beneath it, refused in both orders; holding a bind of a directory beside it, admitted. |
| `adm::src::a_mount_table_that_proves_nothing_keeps_no_writer` | A table that does not parse → `Unmounted(<first input>)`; one naming no mount a root lies on, or a mount whose root is no path → `Unmounted(<root>)`; 65,537 records → `Bounds`; a spent deadline → `Expired`; the temporary directory bound beneath a writable root → `Serving` naming it. |
| `adm::src::real_binds_of_a_source_are_refused_by_the_mount_table` | H1 in real mounts: bubblewrap gives the child `bound_engine` a mount namespace in which the package's `bin`, the package and a directory beside it are really bound; reading `/proc/self/mountinfo` and `statx` as production does, a root within the first bind and one holding the second are refused with the exact cause and one within the third admitted. It ran here (no `skipped:` line); a host without a mount namespace skips it as "host result pending". |
| `adm::src::a_zero_grant_sibling_that_can_write_its_attempts_server_refuses_it` | The attempt's own zero-grant sibling refuses the extension, naming `work:sibling` of `run-a`. |
| `adm::src::a_runs_later_attempt_is_kept_from_its_earlier_attempts_sources` | A run's next attempt (a retry, or a fallback link's) whose writer could write its earlier unsettled attempt's sources is refused; a disjoint one is admitted. |
| `adm::src::a_writer_outside_a_box_is_kept_from_no_source` | `Unknown::Unconfined`, standing and incoming; admitted with nothing reserved. |
| `adm::src::an_extension_refuses_what_admission_refuses` | An unknown participant, a lock held past the deadline, a released lease (`Lost`) and a spent deadline refuse, the reservation unchanged (M2's test). |
| `adm::src::an_extension_committed_unsynced_says_so` | An extension whose seal commits unsynced folds `Commit::Unsynced`. |
| `att::a_seal_committed_unsynced_is_kept_whatever_provisioning_refuses_after_it` | L1: the first extension's seal commits unsynced, then a survey that does not answer, or an attempt directory that already stands, refuses provisioning with its exact text; the lease still fails its run as it settles under the fence rule (`the attempt's writer lease was released, but a seal of its reservation was not proven durable: … not proven synced`). |
| `att::a_brokered_attempt_holding_no_lease_is_refused_once_the_fence_lifts` | With the fence lifted, provisioning a brokered attempt that holds no lease is `Err(Unprovisioned::Unadmitted(Identity(Lost)))` before any survey (its survey panics), its run's directory never made. |
| `att::a_zero_grant_sibling_that_can_write_the_servers_package_never_starts` | Run 1's dispatch proof, passing here; since run 4 the refusal names the launch that lies in the package, reserved before any survey (`Claude::exposed`). |
| `att::a_zero_grant_sibling_that_can_write_a_system_servers_source_never_starts` | The keeper's server the system `true`: a sibling able to write its directory, or `/usr/share`, is refused before any survey, naming (run 5) `/usr`, the root it reaches. |
| `adm::every_refusal_reads_as_its_cause` | Pins `Unmounted`'s text beside run 1's two, and (run 3) `Unrouted`'s; since run 5, `Unrouted`'s three, one per `Unroutable` cause. |
| `server::the_server_box_holds_the_projected_system_set_and_its_own_private_paths` | Its expected argv now builds the shared-network resolver bind from `hands::SERVER_RESOLVER`, so the public path and the box's private `RESOLV_CONF` are pinned equal; it ran here in a real box. |
| `adm::off_linux_no_boxed_writer_is_kept_from_a_servers_inputs` | macOS only: the extension of a boxed writer's lease is refused with `Unmounted(<bootstrap>)`. Not compiled or run here: macOS CI pending. |

Fixture changes: every unit test's root observes the mount table through
`adm::src::shown` (a table a test stands in, else production's own);
`brokered_tests::stood_in` counts each survey (`SURVEYED`); the provisioning
fixtures pass a `Cell<Commit>`; `adm::src::{leased, proven_settled,
unsynced_next, standing_writer}` are re-exported for `att`. One proof moved
between files: the engine-death proof, first written in `adm::src`
(`an_engine_killed_with_its_payload_alive_keeps_its_sources_reserved` and
its child `serving_engine`, which admitted and extended directly), was
replaced in `att` by the dispatched engine's above; `adm::src` keeps only
the `proven_settled` helper it needs.

Run 3's fixture changes: `adm::src::package` also makes the launch route
(`launch`, `srv/links/server`), and `inputs` reserves the launch after the
package; `adm::src::{linked, written}` make links and files beside the
HOME. In `att`, `Claude::launched` sets the keeper's `library-docs` launch,
and the system-server test now uses it in place of its own copy.
`Payload::Server` carries the parent's `setpriv`, and both death proofs run
through `killed_engine` in a process of their own. No test moved between
files.

Run 4's fixture changes: the body of `simultaneous_starts…` is now `raced`,
which it and the bare-launch test share; `killed_engine`'s cases name what a
writer links toward apart from the input its refusal names (the package, and
its launch); `Claude::exposed` names the launch. No test moved between
files.

Run 5's fixture changes: `adm::src::first_root` reads which root a writer
reaches on this host's layout (since run 5's review, through the binds a
test shows, from the test's own `OPERATOR_ROOTS`); `aliases`' system case and
`every_fixed_input…` name a writer by it where it reaches one. The launcher
test and the dangling test share `refused_in_either_order`. The bare-launch
test gives `launches_with` its candidates rather than a located file. The
protocol's `Ground::laid` grants the test-only `admitted` hook to the four
launcher route tests that find the host's bubblewrap through a fixture link
(`the_launcher_runs_once…`, `a_launcher_its_observation_refuses…`,
`the_route_a_launcher_was_found_by…`, `a_route_that_changes_once…`), so
their routes are still decided by their hops. `att`'s three system
expectations name `/usr`. `server`'s `environ` helper reports
names only. Run 4's `launcher::the_launcher_is_found_beside_every_directory_searched_before_it`
is replaced by `a_launcher_found_as_no_reserved_candidate_never_runs`, with
`launcher_on` gone. No test moved between files.

## Removal controls

Each mutation compiled, was run against the named tests, failed as shown,
and was restored; the restored tree passes them.

| Control | Mutation | Failed, with the assertion |
| --- | --- | --- |
| H1, a bind of a subtree | `Table::reaches` skips the writer's own part | `a_bind_of_a_sources_subtree…` at server first: `left: None`, `right: Some(Identity(Serving { site: "work", run: "run-b", served: <package> }))`; the nested-bind test passed. |
| H1, a bind beneath a root | `Table::parts` adds no nested mount | `a_source_bound_beneath…`: the same assertion; the subtree test passed. |
| H1, an unproven table | `Table::parts` returns no part for a path on no mount | `a_mount_table_that_proves_nothing…`: `left: Ok(())`, `right: Err(Unmounted(<b>))`. |
| H1, real mounts | `Table::reaches` always `false` | `real_binds…`: its child `bound_engine` failed, so the parent's status assertion failed. |
| H2, before the survey | the first extension moved after the surveys | `att::a_standing_writer_of_a_servers_input…`: `left: 1`, `right: 0` (a survey ran). |
| H2, the identity tree | `fixed` reserves no temporary directory | `a_servers_fixed_inputs…` (the list) and `every_fixed_input…`: `left: Ok(Synced)`, `right: Err(Identity(Serving { …, served: "/tmp" }))`. |
| H2, the resolver path | `SERVER_RESOLVER` names `/etc/resolv.conf.mutated` | `server::the_server_box_holds…` at its shared-network argv: the box binds `/etc/resolv.conf`, the expected argv does not. |
| Atomic admission | `extended_with` drops its lock right after taking it | `simultaneous_starts…`: `left: Ok(Ok(()))`, `right: Ok(Err(Identity(Serving { … })))`. |
| Complete writer reach | `extended_with` compares only other runs' writers | `a_zero_grant_sibling_that_can_write_its_attempts_server_refuses_it`: `left: Ok(Synced)`. |
| Retention until durable settlement | `unserved` skips a reservation once its positive fact is sealed | `a_free_lock_or_an_uncertain_settlement…` at its unreleased case: `left: Ok(Held { … run: "run-b0" … })`. |
| Retention through engine death | `unserved` compares no reserved input (`&serves[..0]`) | `att::a_dispatched_engine_killed…` at `while-it-lives`: `left: EffectSucceeded`, `right: EffectFailed`; `a_writer_that_can_write_a_reserved_source…` failed too: `left: Ok(Held { … run: "run-b", attempt: "b0" … })`, `right: Err(Identity(Serving { site: "work", run: "run-b", served: <package> }))`. |
| M2 | `extended_with` takes no lock | `an_extension_refuses_what_admission_refuses`: `left: Ok(())`, `right: Err(Identity(Expired))`; `simultaneous_starts…` failed too. |
| L1 | `provision_with` marks the fact only on `Ok(Some(_))` | `att::a_seal_committed_unsynced…`: `unwrap_err()` on `Ok(())`. |
| L2 | `(Ok(Some(Commit::Synced)), true)` moved to the standing arm | `att::a_seal_committed_unsynced…`: `left: "the attempt's writer lease stands, …"`, `right: "the attempt's writer lease was released, …"`. |

Run 2's review found seven tests whose own removal was not recorded, and
this visit adds two. Each row below was run in this visit against exactly
the tests it names, failed
as shown, and was restored; the restored tree passed all 30 tests of
`adm::src` and `att` (`30 passed; 0 failed`).

| Test | Mutation | Failed, with the assertion |
| --- | --- | --- |
| `adm::src::an_extension_committed_unsynced_says_so` (review M1) | `extended_with` seals without folding its commit: `sealed(root, &reservation, &budget)?;` | sources.rs:808: `left: Ok(Synced)`, `right: Ok(Unsynced)`. |
| `att::a_brokered_attempt_holding_no_lease_is_refused_once_the_fence_lifts` (review L5) | `held_extended`'s guard made `everywhere() & false` | the survey ran: writers_tests.rs:851 panicked, `a box was surveyed`. |
| `att::a_dispatched_engine_killed_while_its_seat_lives_keeps_its_servers_sources` (review M2) and `adm::src::a_writer_that_can_write_a_reserved_source_is_refused_before_dispatch` | `unserved` compares no reserved input | as the engine-death row above. |
| `adm::src::a_runs_later_attempt_is_kept_from_its_earlier_attempts_sources` | `unserved` skips its own run's reservations (`.filter(\|found\| found.run != claim.run)`) | sources.rs:469: `left: Ok(Held { … run: "run-a", attempt: "a2" … })`, `right: Err(Identity(Serving { site: "work", run: "run-a", served: <package> }))`. |
| `adm::src::a_writer_outside_a_box_is_kept_from_no_source` | `kept` passes over an unboxed writer (`continue`) | sources.rs:753: `left: Ok(Synced)`, `right: Err(Identity(Unconfined { site: "work", run: "run-b" }))`. |
| `adm::src::a_broker_whose_sources_a_standing_writer_can_write_is_refused_before_lookup`, `att::a_zero_grant_sibling_that_can_write_the_servers_package_never_starts`, `att::a_zero_grant_sibling_that_can_write_a_system_servers_source_never_starts` | `extended_with` compares no standing writer (`writers.take(0)`) | sources.rs:263: `left: Ok(Synced)`, `right: Err(Identity(Serving { site: "work", run: "run-b", served: <package> }))`; both `att` tests at writers_tests.rs:676: `left: EffectSucceeded`, `right: EffectFailed`. |
| `att::a_zero_grant_sibling_that_can_write_a_system_servers_source_never_starts`, alone | `fixed` reserves no system support source (`HOST_TOOLCHAIN_BINDS[..0]`) | writers_tests.rs:680: the refusal named `/usr/bin/gnutrue` after the survey, not `/usr/bin` before it; the package test passed. |
| `att::a_zero_grant_sibling_that_can_write_the_servers_package_never_starts`, alone | `served` adds no program tree (`.take(0)`) | writers_tests.rs:676: `left: EffectSucceeded`, `right: EffectFailed`; the system test passed. |

The visit answering run 2's second review added the rows below. Each
mutation compiled and was run against exactly the tests named, failed as
shown, and was restored. `git diff` then showed no production file changed,
and the restored tree passed all 32 tests of `adm::src` and `att`
(`32 passed; 0 failed`).

| Test | Mutation | Failed, with the assertion |
| --- | --- | --- |
| `att::a_dispatched_engine_killed_while_its_server_box_lives_keeps_its_sources` and the seat variant | `unserved` compares no reserved input (`&serves[..0]`) | both, at `while-it-lives`: `left: EffectSucceeded`, `right: EffectFailed`. |
| the disjoint control of both death tests | `unserved` also reserves `/` wherever any input is reserved, so every incoming writer meets one | both, at `a disjoint worktree`: `left: EffectFailed`, `right: EffectSucceeded`, its error `… site 'work' of run '…' can write '/', which an MCP server's box is built from`. |
| the server box alive (test-side control) | the package's program exits at once (`#!/bin/sh` alone) instead of waiting | at `once its engine died: the server's box serves`: `left: Some(false)`. It ran again after `serving` and `ended` were folded onto `within`, and failed the same way. |
| `att::a_seal_committed_unsynced…` (review M2, typed) | `settlement.rs`'s released-unsynced arm returns `Unreleased::standing` | at `kept on the lease`: `left: Unreleased { lease: Standing, cause: Identity(Unsynced) }`, `right: Unreleased { lease: Released, cause: Identity(Unsynced) }`. |

Recorded by run 1 and not repeated in run 2, for the U8a2c LOWs it carried:
the settlement outcome (`sealing::an_unsynced_settlement_fails_its_run_under_the_fence_rule`)
and its text (`sealing::a_failed_settlement_says_whether_its_lease_was_released`),
and the two hard-link tests (`hands::tests::sealed::a_hard_link_where_a_source_stood_absent_refuses_the_next_box`
and `…::a_hard_link_toward_protected_evidence_or_coordination_fails_in_the_box`),
each failed by its own mutation. All four pass at this head.

Run 3's controls follow. Each mutation compiled and was run against
`adm::src` (and, where named, `att`). It failed as shown and was restored.
The restored tree then passed the whole library suite (1,168 passed), and
`git diff` showed every mutation gone. Line numbers are as each failing run
reported them, before `cargo fmt` last moved the test file.

| Test | Mutation | Failed, with the assertion |
| --- | --- | --- |
| `adm::src::a_dispatch_whose_later_step_could_write_a_reserved_source_starts_nothing` (run 2's owed control) | `admission.rs`'s `writers` takes only the first site (`invocation_sites(body).into_iter().take(1)`), so the later step's member is never reserved | `admission/tests.rs:1295` (its `refused_with`): `left: "sequence step 'draft': reported 'complete', outside its declared results [\"drafted\"]"` (the first step's driver ran), `right: "dispatch refused: MCP server box filesystem identity is not protected: site 'work:finish:m1' of run '…' can write '<package>', …"`. |
| Every link of a route (the HIGH), across the matrix | `route` reserves only the first link met (`if met.is_empty() { met.push(…) }`) | 11 tests: `a_writer_confined_to_a_link_on_a_fixed_route…` (sources.rs:370, `left: Ok(Held { … })`, `right: Err(Identity(Serving { …, served: <d>/srv/links/brokkr }))`); `a_link_swapped_between_the_survey…` (:450, `left: Ok(Synced)`); `every_link_on_an_inputs_route…` (:286, the list without `srv/links/resolver`); `a_route_past_its_bound…` (:312, `left: 2`, `right: 41`); `an_extension_keeps_every_input…` (:420); and, at the `srv/links` alias case, `a_writer_that_can_write_a_reserved_source…` (:479), `a_broker_whose_sources…` (:502, `left: Ok(Synced)`), `simultaneous_starts…` (:735, `left: Ok(Ok(Synced))`), `a_free_lock_or_an_uncertain_settlement…` (:798), `a_dispatch_whose_later_step…` (its first step ran), and `att::a_standing_writer_of_a_link_on_a_servers_route…` (`left: EffectSucceeded`, `right: EffectFailed`). |
| D5's 40 hops, over | `HOPS_MAX` 41 | `a_route_past_its_bound…` (:316): `left: Ok([Link(<d>/hops/b0), …])`, `right: Err(Unrouted(<d>/hops/b0))`. |
| D5's 40 hops, the valid bound | `HOPS_MAX` 39 | the same test, at the 40-link chain's `unwrap()` (:311): `Err(Unrouted(…))`. |
| An absolute route | `route`'s `is_absolute` check made `\| true` | the same test (:318): `left: Ok([Source("hops/file")])`, `right: Err(Unrouted("hops/file"))`. |
| An unread component | any read error other than `NotFound` taken as absent | the same test (:321): `left: Ok([Source(<d>/hops/file/below)])`, `right: Err(Unrouted(…))`. |
| The deadline, per component | `route`'s `budget.check()?` removed | the same test (:324): `left: Ok([Source(<d>/hops/file)])`, `right: Err(Expired)`. |
| `..` as the kernel resolves it | `lexical`'s `..` no longer pops | `every_link_on_an_inputs_route…` (:286) and `a_writer_confined_to_a_link_on_a_fixed_route…` (:370). |
| A bind of a link's directory | `Table::reaches` reads a link as a source (`self.parts(path)`, not its directory's own part) | `a_bind_of_the_directory_a_link_lies_in_is_refused` (:398): `left: None`, `right: Some(Serving { …, served: <d>/srv/links/server })`. |
| Never reserving less across extensions | `extended_with` starts from no earlier reservation (`Vec::new()`) | `an_extension_keeps_every_input_reserved_before` (:420): `left: Ok(Held { … })`, `right: Err(Identity(Serving { …, served: <d>/srv/links/brokkr }))`. |
| The launch's route | `served` reserves no launch (`is_absolute() & false`) | `att::a_standing_writer_of_a_link_on_a_servers_route…` at its launch case: `left: EffectSucceeded`, `right: EffectFailed`. |

The shared mount derivation, the typed setup error, the `setpriv` skip and
the death proofs' own process are refactors of code or tests that the rows
above and run 2's already bind. They add no behaviour of their own, so they
carry no separate control.

Run 4's controls follow. Each mutation compiled and was run against exactly
the tests named, failed as shown, and was restored. A scan of `git diff`
then found none left, and the restored tree passed the whole runtime library
(1,173) and the protocol library (1,013) after `cargo fmt`. Line numbers are
as each failing run reported them, before `cargo fmt` last moved the test
files. `<d>` is the test's own root.

| Test | Mutation | Failed, with the assertion |
| --- | --- | --- |
| `adm::src::a_bare_launch_name_is_reserved_by_the_route_it_is_located_by`, `adm::src::a_launch_is_located_as_its_box_locates_it` (the first HIGH) | `launches_with` never asks the lookup: an absolute name as spelled, nothing else | sources.rs:1132: `left: Ok(Synced)`, `right: Err(Identity(Serving { site: "work", run: "run-b", served: <d>/srv/links/server }))`; sources.rs:245: `left: [Source("/opt/brokkr-u8a2b/server")]`, `right: [Source("/usr/bin/sh"), Source("/opt/brokkr-u8a2b/server")]`. |
| The launch before any survey | `provisioned`'s first extension reserves no launch (`&seal.argv[..0]`) | 5 `att` tests: `a_standing_writer_of_a_link_on_a_servers_route…` at its launch case (writers_tests.rs:840, `left: 1`, `right: 0`: a survey ran); `a_standing_writer_of_a_servers_input…` and `a_zero_grant_sibling_that_can_write_the_servers_package…` (:683, named `<package>` after the survey, not `<package>/bin/server` before it); both death proofs through `killed_engine` at `while-it-lives` (:1354, the same). |
| The launch located again after the survey | `served` locates no launch (`&plan.connection.argv[..0]`) | `att::a_launch_swapped_during_its_survey…` (writers_tests.rs:882, `swapped: true`): `left: true` (provisioned), `right: false`. |
| The launcher (the second HIGH) | `fixed_with` drops the launcher it found | `a_servers_fixed_inputs…` (sources.rs:229, the list without `Source(<d>/bwrap)`); `the_box_launchers_route…` (:1178): `left: Ok(Held { … workdir: <d>/b0 … })`, `right: Err(Identity(Serving { …, served: <d>/srv/links/bwrap }))`. |
| The directories searched before it | `fixed_with` reserves no searched directory | `the_box_launchers_route…` at its `early` case (:1178): `left: Ok(Held { … workdir: <d>/b1 … })`, `right: Err(Identity(Serving { …, served: <d>/early }))`. |
| The box's own discovery | `fixed` passes no searched directory and no launcher | `a_servers_fixed_inputs…` (:231, `fixed` against `fixed_with` over `ServerBox::launcher()`). |
| `launcher_on`'s searched directories | the `take_while` never stops | protocol launcher.rs:1409: `left: ([<d>/early, <d>/srv/links, <d>/later], Some(<d>/srv/links/bwrap))`, `right: ([<d>/early], Some(<d>/srv/links/bwrap))`. |
| A link's own entry (the LOW) | `writes` guards a link through `covers` and the link's followed identity, as before | `a_link_is_guarded_as_its_own_entry…` (:1212, `apart from the link and the program`): `left: Some(Serving { …, served: <d>/users })`, `right: None`. |
| The search path inside the system set | `SERVER_PATH` gains `:/opt/bin` | protocol server.rs:1054: `left: ["/opt/bin"]`, `right: []`. |

Run 4's rows naming `launcher_on`, `ServerBox::launcher` or the searched
directories' entries record run 4's tree; run 5 replaced those functions.

Run 5's controls follow. Each mutation compiled, was run against exactly the
tests named, failed as shown, and was restored. `diff` against a copy taken
before each mutation showed every production file back byte for byte. The
restored tree then passed the whole runtime crate, the protocol crate and the
CLI suites listed under "At `dff1ab11` (run 5)". `<d>` is the test's own root;
line numbers are as each failing run reported them.

| Test | Mutation | Failed, with the assertion |
| --- | --- | --- |
| `adm::src::a_writer_reaching_an_operating_system_root_is_refused_before_the_survey` (ruling 3; recorded at `dff1ab11`, and by itself no proof of the twelve, as run 5's review found: see the rows after this table) | `kept` compares no root (`roots.iter().take(0)`) | sources.rs:331, at `"/"`: `left: Err(Identity(Serving { site: "work", run: "run-b", served: "<d>/boot/brokkr" }))`, `right: Err(Identity(Serving { …, served: "/usr" }))`. |
| `adm::src::a_dangling_earlier_candidate…` and `adm::src::the_box_launchers_route_and_every_candidate…` (the launcher's candidates) | `fixed_with` reserves only a candidate that stands (`.filter(\|found\| found.is_file())`) | both, after `refused_in_either_order` was shared: `left: Ok(Held { … })`, `right: Err(Identity(Serving { …, served: "<d>/srv/later/bwrap" }))` and `… served: "<d>/early/bwrap" }))`. |
| `adm::src::a_dangling_earlier_candidate…` and `adm::src::every_candidate_of_a_launch_is_reserved` (a launch's candidates) | `launches_with` keeps only a candidate that stands | the dangling test at `server`: `left: Ok(Held { … })`, `right: Err(Identity(Serving { …, served: "<d>/srv/later/server" }))`; sources.rs:255: `left: [Source("/usr/bin/sh"), Source("/bin/sh")]`, `right:` the seven candidates. |
| `adm::src::a_survey_starts_without_the_loaders_environment…` and `survey_tests::a_survey_is_spawned_without_the_loaders_environment` (the loader's variables) | `surveying`'s `loaded` always false | sources.rs:279: `left:` the list with `LD_PRELOAD` and `LD_LIBRARY_PATH`; survey_tests.rs:253, in the child: `left: ["LD_LIBRARY_PATH=…", "LD_BROKKR_U8A2B=1"]`, `right: []`. |
| `survey_tests::a_survey_is_spawned_without_the_loaders_environment` alone (the spawn) | `surveyed` drops `env_clear` and passes no environment (`.envs(environment.into_iter().take(0))`) | the same child assertion; the pure `surveying` test passed. |
| both survey tests (the survey's `PATH`) | `surveying` takes no candidate's directory (`.take(0)`) | sources.rs:279: `left: [("PATH", "/opt/early:/usr/bin"), …]`; in the child, `left: ["PATH=/opt/brokkr-u8a2b:…"]`, `right: ["PATH=/usr/local/bin:/usr/bin:/bin"]`. |
| `adm::src::a_route_past_its_bound…` (L3, hops) | `route`'s hop cause `Unread` | `left: Err(Unrouted("<d>/hops/b0", Unread))`, `right: Err(Unrouted(…, Hops))`. |
| `adm::src::a_route_past_its_bound…` (L3, relative) | `route`'s relative cause `Unread` | `left: Err(Unrouted("hops/file", Unread))`, `right: Err(Unrouted("hops/file", Relative))`. |
| `launcher::a_launcher_found_as_no_reserved_candidate_never_runs` (a serving `PATH`) | `checked` admits any found path (`… \| true`) | launcher.rs:1431 at `early`: `left: (Ok(()), 1)`, `right: (Err(Identity), 0)`: the link ran. |
| `launcher::the_launchers_version_runs_with_no_environment` | `version` drops `env_clear` | launcher.rs:1461: `left:` this process's variable names (`CARGO_PKG_NAME`, `LD_LIBRARY_PATH`, `PATH`, …), `right: []`. |
| `server::a_server_box_stands_without_the_seats_or_the_hosts_private_paths` (bubblewrap's launch) | `namespace/entry.rs`'s launch drops `env_clear` (a control only; that file is restored and not in this diff) | server.rs:822: `left: ([114, 101, 97, 100, 121], ["AI_AGENT", "CARGO", …])`, `right: ([…], [])`. |
| `server::every_server_search_directory_lies_in_a_system_source` (the INFO) | the comparison back to `str::starts_with` | server.rs:1061: `a component, not a prefix of one`. |

Run 5's review's controls, each run against `adm::src` (alone, or the one
test named), failed as shown and restored; the restored tree passed all 30
tests of `adm::src` and the whole runtime crate:

| Test | Mutation | Failed, with the assertion |
| --- | --- | --- |
| `adm::src::a_writer_reaching_an_operating_system_root_is_refused_before_the_survey` (M1, one root omitted) | `SYSTEM_ROOTS`' `"/dev"` replaced by a second `"/usr"`, compiling | sources.rs:358, at `"/dev/brokkr-u8a2b"`: `left: Ok(Held { …, run: "run-b", attempt: "b17", … })`, `right: Err(Identity(Serving { site: "work", run: "run-b", served: "/dev" }))`, before the closing pin was reached. |
| the same test (M1, the oracle's binds) | the test's `first_root` given no bind (`&[]`) in the server-first loop | sources.rs:359, `first_root(…).unwrap()` on `None`, at the writer within `sysalias`: a bind-blind oracle names no root for it, though production refuses it naming `/etc`. |
| `adm::src::every_link_on_an_inputs_route_is_reserved` and `adm::src::every_fixed_input_is_reserved_before_the_survey` (L1) | `route` walks on past a missing component (`continue` for `break`) | sources.rs:459 (`routed`'s exact list) and sources.rs:796 (the bootstrap's missing component); the other 28 passed. |
| `adm::src::a_dangling_earlier_candidate…` (L2) | the dangling candidate's target written before the assertion (a test-side control: no production path writes it) | sources.rs:1444: `left: Ok(())`, `right: Err(NotFound)`, `still dangling`. |

A first form of the oracle control, with the injected table binding `/bin`
from `/usr/bin` as the reviewer's box lays it out, did not fail: on this
host a bind shown at `/bin` changed nothing, so `/bin` does not resolve to
itself here. That is why the test's bind is `/etc` at `sysalias`, which
binds on every host.

Run 5 first had the broker find its launcher on the server `PATH`
(`Host::serving`). The control replacing that discovery with
`require_bwrap()` failed the test then written for it. The four CLI proofs
above then failed against that design, and it was replaced by the candidate
check before this commit. Neither that test nor `Host::serving` is in this
diff.

## U8a2c's carried LOWs

Kept from run 1 as built: `Unreleased` says whether the lease stands or was
released (its private `Lease`); `Authored.home` is gone, `redirected` reading
the sealed HOME through `hands::drifted`; the hard link into an absent source
is its own test, beside the kernel's cross-mount refusal toward protected
evidence and the coordination root.

## Gates and measurements

### At this head (run 5's review, answered)

Observed in run 5's seat after the review's answer, on the tree committed
with this file. Only brokkr-runtime's code moved. Format was clean, and so
was the workspace clippy with all targets and features, locked, warnings
denied. `cargo +1.88 check --workspace --all-targets --all-features`
finished. brokkr-runtime passed whole: lib 1,177, `it` 135 (its
`witness_digests::` unblessed among them, so no witness or compose pin
moved), `capability_launch` 76, `operated_repo` 1, `queued_launch` 6.
`adm::src` with `provision_tests`, run alone with `--nocapture`, printed no
line containing `skipped` (79 passed). brokkr-cli's `it` (all features,
locked) passed 732 with 3 ignored and failed one:
`capability_broker::protocol::a_server_that_never_answers_ends_at_the_startup_deadline`,
at its 29–32-second elapsed bound (protocol.rs:674; the elapsed value was
not captured), under the full suite's parallel load. It passed alone
(31.5 s for the test binary). That test drives the CLI broker's own
deadline, which this change does not reach. `typos --hidden` over the
touched trees and `git diff --check` were clean, and `openspec validate
--all --strict` passed 29 of 29. `compile --bundle recipes/self` reported
47 valuations over 11 groups, 0 unruled. The rows of the two moved files are set from
`wc -l`: sources.rs 622 and admission/tests/sources.rs 1,483. No function
crosses 100 lines and no suppression moved. `quality/ratchet.sh` (`files`,
`clones`, `api`) could not be run in this seat, whose sandbox asked for an
approval no one could give: pending, for the controller's host run. The
change adds no public item, so `api` cannot move. The protocol crate did
not move and was not re-run.

### At `dff1ab11` (run 5)

Observed in run 5's seat on the tree committed at `dff1ab11`. The format
check was clean. The workspace clippy with `--all-targets --all-features
--locked -- -D warnings` was clean once one real-box test's added assertion
had been folded into its existing `ready` check (it first crossed 100
lines). `cargo +1.88 check --workspace --all-targets --all-features`
finished. Suites passed crate by crate with `--all-features --locked`.
brokkr-runtime: lib 1,177, `it` 135 (its `witness_digests::` unblessed among
them, so no witness or compose pin moved), `capability_launch` 76,
`operated_repo` 1, `queued_launch` 6. `adm::src`, `att` and `survey_tests`
alone with `--nocapture` printed no `skipped:` line (53 passed), so the
real-namespace proofs ran here. brokkr-protocol: lib 1,014 with 3 ignored,
its other targets 6, 1 and 1; its `launcher` and `server` modules alone with
`--nocapture` printed no `skipped:` line (33 passed), so
`a_launcher_found_as_no_reserved_candidate_never_runs` and the real-box
`environ` assertion ran. brokkr-cli: lib 905 with 1 ignored,
`driver_conformance` 27, the three heap tests, `it` 733 with 3 ignored. In
one earlier full `it` run, under load, `readiness::a_box_held_at_its_entry_refuses_at_the_absolute_deadline`
failed at its 20-second hold ("no box … held"). It passed alone and in the
final full run. No other crate's code moved, so their suites were not run in
this seat. `compile --bundle recipes/self` reported 47 valuations over 11
groups, 0 unruled. `openspec validate --all --strict` passed 29 of 29.
`typos --hidden`, `git diff --check` and `git diff --cached --check` were
clean. `quality/ratchet.sh` held `files`, `clones` (after the launcher and
dangling tests' shared body became `refused_in_either_order`; it first found
one new 12-line test clone) and `api` (run 4's three ruled lines rewritten in
place).

The rows, measured by `wc -l` at this tree, in `quality/file-lines.txt` as
here. Production: broker.rs 800 (at the ceiling), sources.rs 618,
admission.rs 796, exclusion.rs 511, root.rs 450, settlement.rs 319, hands.rs
1,094 (its baseline), namespace.rs 799 and namespace/sources/launcher.rs
717. Tests: admission/tests.rs 1,680, admission/tests/sources.rs 1,438,
writers_tests.rs 1,492, survey_tests.rs 654, capability_broker_launch.rs
119, and the protocol's hands/tests/launcher.rs 1,467 and
hands/tests/server.rs 1,085; the others as at run 4. engine.rs (3,523) did
not move. No function crosses 100 lines, so neither `too-many-lines.txt` nor
`suppressions.txt` moves. No new suppression, `unsafe` or dependency. No
budget input moved, so `scripts/measure-budgets.sh` was not run.

### At run 4's head (`c6e3d7aa`)

Observed in run 4's seat on the tree it committed. The format
check and the workspace clippy with `--all-targets --all-features --locked
-- -D warnings` were clean, and `cargo +1.88 check --workspace --all-targets
--all-features` finished. Suites passed crate by crate with
`--all-features --locked`. brokkr-runtime: lib 1,173, `it` 135 (its
`witness_digests::` unblessed among them, so no witness or compose pin
moved), `capability_launch` 76, `operated_repo` 1, `queued_launch` 6. A run
of `adm::src` and `att` alone with `--nocapture` printed no `skipped:` line
(45 passed), so `real_binds…`, both death proofs, the attack matrix and the
bind-alias proof ran in real bubblewrap namespaces here. brokkr-protocol: lib
1,013 with 3 ignored, its other targets 6, 1 and 1. brokkr-cli: lib 905 with
1 ignored, `driver_conformance` 27, the three heap tests, `it` 733 with 3
ignored, `transcript_surfaces` 14. brokkr-bridge, brokkr-core,
brokkr-seatbelt-probe, brokkr-store and brokkr-view passed with no failure.
`compile --bundle recipes/self` reported 47 valuations over 11 groups, 0
unruled. `openspec validate --all --strict` passed 29 of 29. `typos
--hidden`, `git diff --check` and `git diff --cached --check` were clean.
`quality/ratchet.sh` held `files`, `clones` and `api` (the API after its
three ruled lines were written in).

The rows, measured by `wc -l` at this tree, in `quality/file-lines.txt` as
here. Production: broker.rs 796, sources.rs 558, admission.rs 800 (at the
ceiling), exclusion.rs 511, root.rs 450, settlement.rs 319, hands.rs 1,094
(its baseline), namespace.rs 798 and namespace/sources/launcher.rs 720.
Tests: admission/tests.rs 1,678, admission/tests/sealing.rs 532,
admission/tests/sources.rs 1,270, provision_tests.rs 1,150,
brokered_tests.rs 850, writers_tests.rs 1,489, capability_broker_launch.rs
114, capability_ledger.rs 39, and the protocol's hands/tests/launcher.rs
1,418, hands/tests/sealed.rs 519 and hands/tests/server.rs 1,069. engine.rs
(3,523) did not move. No function crosses 100 lines (clippy's
`too_many_lines` is denied and passed), so neither `too-many-lines.txt` nor
`suppressions.txt` moves. No new suppression, `unsafe` or dependency. No
budget input moved, so `scripts/measure-budgets.sh` was not run.

### At `61a04bbc` (run 3's head)

Observed in run 3's seat on the tree it committed. `cargo fmt
--all -- --check` and the workspace clippy with `--all-targets
--all-features --locked -- -D warnings` were clean, and `cargo +1.88 check
--workspace --all-targets --all-features` finished. Suites passed crate by
crate with `--all-features --locked`. brokkr-runtime: lib 1,168 (no
`skipped:` line, so `real_binds…`, both death proofs, the attack matrix and
the bind-alias proof ran in real bubblewrap namespaces here), `it` 135,
`capability_launch` 76, `operated_repo` 1, `queued_launch` 6; `it`'s
`capability_broker_launch::` and `capability_ledger::` alone passed 2.
brokkr-protocol: lib 1,011 with 3 ignored, and its three other targets 6, 1
and 1. brokkr-cli: lib 905 with 1 ignored, `driver_conformance` 27, the
three heap tests, `it` 733 with 3 ignored (the layering and suppression
gates among them), `transcript_surfaces` 14. Every other workspace crate
passed with no failure. `witness_digests::` passed 6 unblessed, so no
witness or compose pin moved. `compile --bundle recipes/self` reported 47
valuations over 11 groups, 0 unruled. `openspec validate --all --strict`
passed 29 of 29, and `typos --hidden`, `git diff --check` and `git diff
--cached --check` were clean. `quality/ratchet.sh` held `files`, `clones`
and `api`.

The rows set by `wc -l`: admission.rs 800 (at the ceiling), exclusion.rs
511, sources.rs 503, admission/tests.rs 1,678, admission/tests/sources.rs
1,062, writers_tests.rs 1,411 (run 3's review measured 1,429 at that head:
the stale row its LOW names) and capability_broker_launch.rs 114. broker.rs
795, root.rs 450, settlement.rs 319 and hands.rs 1,094 did not move, nor did
engine.rs (3,523). No function crosses 100 lines (clippy's
`too_many_lines` is denied and passed), so neither `too-many-lines.txt` nor
`suppressions.txt` moves. No new suppression, `unsafe`, dependency or public
item: the API ratchet held at run 2's one ruled line. No budget input moved
(no prompt, dependency or transcript change), so `scripts/measure-budgets.sh`
was not run.

### At `fed26e57` (the commit answering run 2's second review)

These were observed in that visit, on the tree it committed. Only
brokkr-runtime's tests, `quality/file-lines.txt`, `tasks.md` and this file
changed in this visit. `cargo fmt --all -- --check` and `cargo clippy
--workspace --all-targets --all-features --locked -- -D warnings` were clean.
`cargo +1.88 check --workspace --all-targets --all-features` finished.
brokkr-runtime passed crate by crate: lib 1,159, its `it` binary 135 (witness
digests unblessed and budgets among them), `capability_launch` 76,
`operated_repo` 1 and `queued_launch` 6. The suite printed no `skipped:` line
for `adm::src` or `att`: both death tests, `real_binds…`, the attack matrix
and the bind-alias proof ran in real namespaces here. brokkr-cli's `it`
passed 733 with 3 ignored (the layering, suppression and documentation gates
among them), after this file was written. `compile --bundle
recipes/self` reported 47 valuations over 11 groups, 0 unruled. `openspec
validate --all --strict` passed 29 of 29, and `typos --hidden` and `git diff
--check` were clean. This seat ran `quality/ratchet.sh` itself: `files` held
the file sizes, `clones` held duplication, and `api` held the public API.
`clones` first found one new 7-line clone, the new `ended` against the
protocol's own `hands/tests/server.rs`. Both of the new polling helpers were
then folded onto one `within`, and the ratchet held.

The rows set in that visit, by `wc -l`: writers_tests.rs 1,279 and
capability_ledger.rs 39. At that commit the production files measured:
broker.rs 795, admission.rs 794, root.rs 450, settlement.rs 319, sources.rs
406, exclusion.rs 493 and hands.rs 1,094 (engine.rs 3,523, unmoved). The
tests measured: admission/tests.rs 1,677, admission/tests/sources.rs 809,
provision_tests.rs 1,150, brokered_tests.rs 850 and
capability_broker_launch.rs 113 (its header changed within its lines). No
witness or compose pin moved, and no budget input moved.

### Recorded at earlier commits

At `1fc9f070`, run 2's first commit, the format check and the workspace
clippy with `-D warnings` were clean, the 1.88 check finished, and suites
passed crate by crate: brokkr-runtime lib 1,156, its `it` binary 135 (witness digests and budgets among them),
`capability_launch` 76, `operated_repo` 1, `queued_launch` 6; brokkr-protocol
lib 1,011 with 3 ignored, `hands_exits` 6, `secret_drop` 1; brokkr-cli lib
905 with 1 ignored, `driver_conformance` 27, the three heap tests, `it` 733
with 3 ignored (the suppression and layering gates among them),
`transcript_surfaces` 14; every other workspace crate with no failure.
`compile --bundle recipes/self` compiled with 0 unruled valuations. `openspec
validate --all --strict` passed 29 of 29; `typos --hidden` and `git diff
--check` were clean. `quality/ratchet.sh` held file size, duplication (after
folding the H1 tests' shared body into `shown_in_either_order` and
`writer_first`) and the public API.

At `1fc9f070`, `quality/file-lines.txt` rows were set for admission.rs (794), root.rs (450),
settlement.rs (319), sources.rs (385), and the tests admission/tests.rs
(1,677), admission/tests/sources.rs (929), provision_tests.rs (1,150),
brokered_tests.rs (850), writers_tests.rs (842), capability_broker_launch.rs
(113) and capability_ledger.rs (37); broker.rs (799), hands.rs (1,094) and
the other rows run 1 set did not move. No function crosses 100 lines
(clippy's `too_many_lines` is denied and passed, and no `too-many-lines.txt`
row names a changed file), so neither it nor `suppressions.txt` moves. No
witness or compose pin moved (`it`'s witness digests passed unblessed); no
budget input moved.

At `6aee831a`, the visit that answered run 2's first review measured its own tree again. Format
check and workspace clippy with `-D warnings` were clean, and the 1.88
check finished. brokkr-runtime's lib passed 1,157 (the real engine-death
test among them, its child engine really started and killed, and the
real-namespace `real_binds…` and the `att` box proofs run, not skipped),
`it` 135 (witness digests unblessed, budgets), `capability_launch` 76,
`operated_repo` 1 and `queued_launch` 6; brokkr-cli's `it` passed 733 with 3
ignored (the layering and suppression gates among them). No other crate's
code moved in this visit. `compile --bundle recipes/self` reported 0 unruled
valuations, `openspec validate --all --strict` 29 of 29, and `typos
--hidden` and `git diff --check` were clean. The rows it moved were set by
`wc -l`: broker.rs 795, sources.rs 406, admission/tests/sources.rs 809,
writers_tests.rs 1,034 and capability_ledger.rs 38. `quality/ratchet.sh`
(files, clones, api) was not run at `6aee831a`; review then observed it pass
there, and it passed at `fed26e57`.

Pending, as this seat cannot produce them: exact coverage
(`scripts/coverage-exact.sh`, outside the workspace box) and the CRAP
ratchet it feeds; the macOS build and tests, the off-Linux code and its test
among them (the route walk, `launches`, `surveying` and `holding` compile
there unchanged; `sources::launchers` is empty there, so `fixed` reserves no
launcher and the survey keeps the engine's `PATH`, `ServerBox::launchers`
being Linux-only); remote CI on the final head. The real-namespace tests ran
in real bubblewrap namespaces in run 5's seat, as in runs 3 and 4: the two
death proofs (the server-box one launching a real box), the attack matrix,
the bind-alias proof, `real_binds…`, the survey spawn test, the protocol's
server-box test with its `environ` assertion, the candidate test and the two
hard-link tests. The controller's host run of all of them remains the record
before merge: host result pending.

## Notes and residuals

LOW, outside the row: the temporary directory reserved is the engine's
`std::env::temp_dir()`, which the survey inherits. `brokkr broker serve`
generates its box's identity under its own process's temporary directory,
set by whatever launches it; where that differs from the engine's, this
reservation does not cover its identity tree, and the survey's generated-file
facts (mode, owner, links, size) do not digest the files' bytes. Closing it
needs the plan to seal that directory, or the broker to generate beneath one
the engine names, in `brokkr-protocol` or the CLI's broker: a follow-up, not
this row. Run 3 leaves it as it was, not repaired. The controller carries it
to U9b1, before the fence lifts: one authoritative directory, sealed and
consumed, needs protocol `hands/namespace.rs` (about :679 and :761) and
CLI production files outside this row and outside the operator's ruling.

Run 3's LOW on a bare launch word is repaired in run 4 (its first HIGH):
the launch is located by the box's own public lookup and its whole route
reserved.

Run 4's MEDIUM (unbounded reads under the lock) and its finding on
`/etc/ld.so.preload` and the bootstrap's loader inputs are settled by the
operator's ruling of 2026-10-19, as built under "Run 5": the reads are
bounded as the inventory's are and are not cancellable, and every loader
input lies under a root no managed writer may reach. A bootstrap linked
against libraries outside the roots is the operator's installed binary,
trusted with its sealed bytes (D5).

Run 5, assumptions. The broker keeps finding its launcher on its own `PATH`
and admits only a reserved candidate, where the controller's guidance asked
that it find it on the server `PATH`. The reason is under "Run 5": the
guidance's form left the CLI broker suite no way to plant a launcher or make
a box unavailable, and four existing proofs failed on it. The ruling's line
holds either way: no unreserved launcher ever runs. The toolchain binds and
the identity tree's temporary directory stay reserved by route, beside the
ruling's list, since a bind whose route leaves the roots must stay covered
and the carried TMPDIR LOW is not this unit's to narrow.

A mount hidden beneath another at the same point is still counted as nested,
and a nested mount whose root is no path refuses: both refuse more, never
less. `/etc/ssl` is compared whole although a server box keeps only its
certificates.

`Reserved.serves` is `serde(default)` and skipped when empty: an attempt that
brokers nothing writes the same record as before, and a reservation an
engine before this one sealed reads as serving nothing, which is true of
every such engine, as none could broker under the standing fence.

An attempt provisioned holding no lease is refused once the fence lifts
(`held_extended`): with it lifted, `coordinated` refuses every attempt whose
root it cannot open, so dispatch provisions none without a lease. While the
fence stands no public engine brokers, and a test that provisions an
attempt it never admitted walks from HOME without extending, as before.

Observed once in this visit, while the new server-box test was still
failing: its stand-in returned early, and in that one parallel run of `att`
two unrelated tests saw their waiting writer end `EffectIndeterminate`. The
seat-only death test's seat was also already gone when it was killed. In
three later parallel runs of `att` (15 of 15 each) and in the whole library
suite, with the test passing, nothing failed. It looks like the known class
of reaped strays across parallel engine tests. It is named here, not chased:
it is not a refusal this unit changes. (Run 3: that seat-gone failure was
the orphan adoption under "Run 3", now repaired.)

Observed in run 3's full library runs, after the death proofs moved into a
process of their own. Two of main's own U8a2a tests failed once each, in
different runs, and passed in every other run (at least twelve):
`adm::every_steps_door_rechecks_the_lease_its_attempt_holds` and
`adm::dispatch_holds_the_lease_while_its_driver_runs_and_releases_it_once_settled`
(at `assert_eq!(event, EffectSucceeded)`). Both bodies are main's, which
this unit's diff of `admission/tests.rs` does not touch; only the root
helper they share now carries run 2's mount seam. The second failed alongside the server-box
death proof's one failure, in the loaded run under "Run 3". This visit made
no change to either, and their texts were not captured. They look like the
same class of stray attributed across parallel in-process attempts. The
controller's host run is the record.

D5's trust limit stands: this coordinates current and concurrent managed
writers; it does not authenticate bytes installed earlier through a writable
bind since removed, nor guard against the operator's own host writes or
older engines that do not participate.

## The controller's fix after CI

CI's exact coverage gate on #778 (run 39266573379) counted 64,636 of 64,637 lines and 8,301 of 8,302 functions. The uncovered line was the route walk's helper `lexical`, whose arm joining a plain name could never run, because `route` called it only for `.` and `..`. The uncovered function was `Table::parts`'s `unproven` closure, for a mount at or below a writable root whose root is no path. The controller folded `lexical` into `route` (`.` passes, `..` pops the parent, as before) and added that mount table to `a_mount_table_that_proves_nothing_keeps_no_writer`, which refuses it with MB3's identity cause naming the writable root. This landed by hand under the operator's ruling of 2026-10-19 ("A"). On the host, a branch coverage run of the broker tests (`cargo +nightly-2026-09-05 llvm-cov --branch`) hits both, and all 113 broker tests pass apart from the bind-alias proof this host's AppArmor blocks. Clippy with `-D warnings`, `cargo +1.88 check`, fmt and the files ratchet are clean, and `quality/file-lines.txt` records the two files' new counts.
