//! Every managed writer kept from every admitted MCP server's sources
//! (decision 0065 slice two, U8a2b; MB3, CR2; D5, D6). It lies beside
//! `broker.rs`, which this would take past its 800 lines (operator ruling
//! of 2026-10-11).
//!
//! A server's box is built from host inputs. The operating system's own
//! roots ([`SYSTEM_ROOTS`]) are trusted host state no managed writer may
//! reach at, above or below (D6, operator ruling 2026-10-19), so whatever
//! resolves beneath them — the dynamic loader and its configuration,
//! `/etc/ld.so.preload` among it, system libraries and programs, and the
//! device nodes and `/proc` bubblewrap's `--dev /dev` and `--proc /proc`
//! give a box — needs no reservation of its own. What can lie outside them
//! is reserved: the bootstrap; the system support sources, kept as the
//! box binds them, and the resolver file; the temporary directory its
//! identity is generated beneath; every candidate the box launcher's
//! discovery and each launch's lookup can select on the server `PATH`,
//! dangling ones included; and each program tree. Provisioning's survey
//! proves them kept from the seat it serves as they stand; what keeps them
//! so for the server's lifetime is the coordination admission already
//! holds. Before any survey spawns, every participant's writers — every
//! site of every standing attempt: this attempt's own zero-grant siblings,
//! members and steps, its run's earlier attempts and other worktrees' runs
//! alike — are compared, under the host admission lock, with the roots and
//! the attempt's fixed inputs and launch candidates, which are sealed into
//! its reservation ([`extended`]); once the surveys answer, so are their
//! program trees, the fixed inputs kept beside them. Each input is reserved
//! by its route ([`routed`]): every symlink met on its way, component by
//! component within D5's [`HOPS_MAX`] hops and the deadline, as well as
//! where it resolves, or, for a dangling one, its first component that does
//! not stand, so a writer that could replace any link of the route, or make
//! a dangling candidate live, is refused too; a link is guarded as its own entry, so a writer apart
//! from the directory it lies in, and from where it leads, is not. Every
//! later admission compares its incoming writers with the roots and every
//! input a standing attempt reserved, and with the coordination root that
//! holds those reservations ([`kept`]). In either order, and for
//! two started together, at most one of a conflicting pair is admitted,
//! and the inputs stay reserved, as the writers do, until their attempt is
//! proven settled: an engine's death or a free lock releases neither. The
//! survey starts on the server `PATH` without the loader's `LD_*`
//! ([`surveying`]), so it finds no launcher the lease does not hold.
//!
//! A writer conflicts with a source where any root it can write lies at,
//! above or below it — by canonical path or by identity
//! ([`super::exclusion::covers`]), so a symlink, an ancestor or a second
//! spelling of a system directory cannot hide it — or where the mount table
//! shows the part of a filesystem it writes holding, or held by, the part a
//! source is: its own, or that of any mount beneath it, so a bind of a
//! source's subtree elsewhere, or a bind of a source nested beneath a
//! writable root, cannot hide it either ([`Table`]). A mount table that
//! cannot be read, or that does not prove where a path lies, proves no
//! writer kept, and off Linux none is read (decision 0063). The temporary
//! directory is reserved for its own entries, which only a writer whose
//! root lies at or above it can replace. A writer outside a workspace box
//! has no confined privilege, so the survey's write-exclusion proof, which
//! holds against the managed writers' mapped uid taken as confined, does
//! not hold against it. Every participant runs as this user, whose alone
//! the coordination root and every reservation in it are: that one uid is
//! the mapped identity every plan seals. Each refuses with MB3's identity
//! cause.
//!
//! Every host observation here is a synchronous call on the admitting
//! thread, bounded as the inventory's are (D5; D6, operator ruling
//! 2026-10-19): by D5's counts, [`HOPS_MAX`] links a route and the mount
//! table's records, and by the startup deadline checked between calls,
//! never during one. These reads are not cancellable: a call that stalls
//! holds the admission lock until it returns, so other admissions wait and
//! then refuse at their deadlines; none is ever admitted by it.
//!
//! This coordinates current and concurrent managed writers. It does not
//! authenticate bytes installed earlier through a writable bind since
//! removed (D5), nor guard against the operator's own host writes.

use std::cell::Cell;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::time::Instant;

use brokkr_protocol::broker::{Plan, Tree};
use brokkr_protocol::hands::{ServerProgram, HOST_TOOLCHAIN_BINDS, SERVER_RESOLVER};
use serde::{Deserialize, Serialize};

use super::admission::{sealed, Claim, Held, Unadmitted, Unknown};
use super::exclusion::{beneath, covers, covers_spelled, Guarded};
use super::inventory::{inventory, Budget, Reaching, Standing, Writing};
use super::resolved;
use super::root::{coordination, locked, Commit, Mounts, Root};
use super::settlement::recovered;
use crate::capabilities::McpFence;

#[cfg(target_os = "linux")]
use super::exclusion::{self, anchored, mounts, Mount, Shown, MOUNTINFO};

/// What the mount table shows, read once for one comparison: every mount it
/// names, and how the mount a path lies on is found (its `statx` mount id
/// in production).
#[cfg(target_os = "linux")]
pub(super) struct Table {
    mounts: Vec<Mount>,
    on: On,
}

/// How the id of the mount a path lies on is found.
#[cfg(target_os = "linux")]
type On = Box<dyn Fn(&Path) -> Option<u64>>;

/// One part of one filesystem: the device a mount names, and the path
/// within that device's filesystem.
#[cfg(target_os = "linux")]
struct Part<'t> {
    dev: &'t str,
    path: PathBuf,
}

#[cfg(target_os = "linux")]
impl Table {
    /// Whether the writable root `root` reaches `served` by the parts of
    /// filesystems each shows ([`Table::parts`]): a part `root` writes
    /// holding, or held by, a part a source shows; or holding the one part
    /// a directory reserved for its entries is, or the directory a link's
    /// own entry lies in. An error where the table does not prove where
    /// either lies.
    fn reaches(&self, root: &Path, served: &Served) -> Result<bool, Unknown> {
        let writes = self.parts(&resolved(root.to_path_buf()))?;
        let (shown, entries) = match served {
            Served::Source(path) => (self.parts(path)?, false),
            Served::Entries(path) => (vec![self.own(path)?.0], true),
            Served::Link(path) => (vec![self.own(holding(path))?.0], true),
        };
        let holds =
            |held: &Part<'_>, by: &Part<'_>| (held.dev == by.dev) & held.path.starts_with(&by.path);
        let met = |by: &Part<'_>| {
            let either = |part: &Part<'_>| holds(part, by) | (!entries & holds(by, part));
            shown.iter().any(either)
        };
        Ok(writes.iter().any(met))
    }

    /// The parts of filesystems `path` shows: where it lies, by its nearest
    /// ancestor that stands, first; then the root of every mount at or
    /// below it. An error where the table names no mount it lies on, or one
    /// whose root is no path.
    fn parts(&self, path: &Path) -> Result<Vec<Part<'_>>, Unknown> {
        let unproven = || Unknown::Unmounted(path.to_path_buf());
        let (part, whole) = self.own(path)?;
        let mut parts = vec![part];
        let nested = self
            .mounts
            .iter()
            .filter(|mount| mount.point.starts_with(&whole));
        for mount in nested {
            let path = mount.root.clone().ok_or_else(unproven)?;
            parts.push(Part {
                dev: &mount.dev,
                path,
            });
        }
        Ok(parts)
    }

    /// The one part `path` is, where it lies by its nearest ancestor that
    /// stands, and `path` as that ancestor resolves; no mount below it is
    /// read. An error where the table names no mount it lies on.
    fn own(&self, path: &Path) -> Result<(Part<'_>, PathBuf), Unknown> {
        let unproven = || Unknown::Unmounted(path.to_path_buf());
        let (at, rest) = anchored(path);
        let id = (self.on)(&at).ok_or_else(unproven)?;
        let (own, part) = exclusion::part(&self.mounts, id, &at).ok_or_else(unproven)?;
        let whole = at.join(&rest);
        let part = Part {
            dev: &own.dev,
            path: part.join(rest),
        };
        Ok((part, whole))
    }
}

/// This namespace's mount table, read within `budget`'s deadline, beside
/// the source `served` it is read for ([`read`]); each path's mount found
/// by its `statx` mount id.
#[cfg(target_os = "linux")]
pub(super) fn observed(budget: &Budget, served: &Path) -> Result<Table, Unknown> {
    let unmounted = Unknown::Unmounted(served.to_path_buf());
    let table = std::fs::File::open(MOUNTINFO).or(Err(unmounted))?;
    let mut table = std::io::BufReader::new(table);
    read(&mut table, budget, served, Box::new(exclusion::mounted_on))
}

/// The table `table` read as exclusion reads it, within `budget`'s
/// deadline, each path's mount found by `on`: MB3's bound and deadline
/// causes where it exceeds D5's bound or the deadline passes, and where it
/// does not parse it proves nothing of where `served` lies.
#[cfg(target_os = "linux")]
pub(super) fn read(
    table: &mut dyn std::io::BufRead,
    budget: &Budget,
    served: &Path,
    on: On,
) -> Result<Table, Unknown> {
    let mounts = mounts(table, budget).map_err(|shown| match shown {
        Shown::TooMany => Unknown::Bounds,
        Shown::Expired => Unknown::Expired,
        Shown::Once | Shown::Elsewhere => Unknown::Unmounted(served.to_path_buf()),
    })?;
    Ok(Table { mounts, on })
}

/// No mount table is read off Linux (decision 0063): there is none to hold.
#[cfg(not(target_os = "linux"))]
pub(super) enum Table {}

#[cfg(not(target_os = "linux"))]
impl Table {
    /// Never asked: no table is ever read here.
    fn reaches(&self, _: &Path, _: &Served) -> Result<bool, Unknown> {
        match *self {}
    }
}

/// Off Linux no mount table proves where a source lies, so no boxed writer
/// is kept from one (decision 0063), as exclusion refuses every boxed
/// participant beside evidence there.
#[cfg(not(target_os = "linux"))]
pub(super) fn observed(_: &Budget, served: &Path) -> Result<Table, Unknown> {
    Err(Unknown::Unmounted(served.to_path_buf()))
}

/// One host input an attempt's server boxes are built from, or a link on
/// its way: a source no managed writer may write at, above or below it;
/// the directory a box's identity is generated beneath, whose own entries
/// none may replace; or a symlink met on an input's route ([`routed`]),
/// its own entry, which no writer whose root holds the directory it lies
/// in may replace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
pub(super) enum Served {
    Source(PathBuf),
    Entries(PathBuf),
    Link(PathBuf),
}

impl Served {
    /// The input's path.
    pub(super) fn path(&self) -> &Path {
        match self {
            Served::Source(path) | Served::Entries(path) | Served::Link(path) => path,
        }
    }
}

/// The directory a link's own entry lies in: what a writer able to
/// replace it writes.
fn holding(link: &Path) -> &Path {
    link.parent().unwrap_or(link)
}

/// The operating system's own roots: trusted host state no managed writer
/// may reach at, above or below, which every comparison holds beside the
/// reserved inputs ([`kept`]; D6, operator ruling 2026-10-19). `/usr`
/// first, so a writer within it is named by it on a merged layout, where
/// `/bin` and `/lib` lead into it, as on a split one.
pub(super) const SYSTEM_ROOTS: [&str; 12] = [
    "/usr", "/bin", "/sbin", "/lib", "/lib32", "/lib64", "/libx32", "/etc", "/dev", "/boot",
    "/proc", "/sys",
];

/// Every file a server box's launcher may be found as on this host, in
/// order (`hands::ServerBox::launchers`).
pub(super) fn launchers() -> Vec<PathBuf> {
    #[cfg(target_os = "linux")]
    return brokkr_protocol::hands::ServerBox::launchers();
    // Off Linux no server box stands, so none is launched (decision 0063).
    #[cfg(not(target_os = "linux"))]
    Vec::new()
}

/// The inputs every server box of an attempt may be built from before its
/// survey names its program, as spelled: the bootstrap `bootstrap`, every
/// system support source (`hands::HOST_TOOLCHAIN_BINDS`, of whose
/// `/etc/ssl` a server box keeps only its certificates), so one whose route
/// leaves the roots stays kept, and the resolver file, whatever its
/// network; the temporary directory its identity is generated beneath; and
/// every candidate of the box launcher's discovery ([`launchers`]). Each is
/// reserved by its route ([`routed`]).
pub(super) fn fixed(bootstrap: &Path) -> Vec<Served> {
    fixed_with(bootstrap, launchers())
}

/// [`fixed`], the box launcher's candidates `launchers`, each reserved
/// whether or not it stands, so none a writer could make live, ahead of
/// the one found or as the one found, is left unreserved.
pub(super) fn fixed_with(bootstrap: &Path, launchers: Vec<PathBuf>) -> Vec<Served> {
    let system = HOST_TOOLCHAIN_BINDS.iter().chain([&SERVER_RESOLVER]);
    let sources = std::iter::once(bootstrap.to_path_buf()).chain(system.map(PathBuf::from));
    let mut fixed: Vec<Served> = sources.map(Served::Source).collect();
    fixed.push(Served::Entries(std::env::temp_dir()));
    fixed.extend(launchers.into_iter().map(Served::Source));
    fixed
}

/// The environment a survey starts with (D6, operator ruling 2026-10-19):
/// `inherited`, the engine's own, without the dynamic loader's `LD_*`, and
/// its `PATH` the directories of the box launcher's candidates `launchers`,
/// in order, so the survey's discovery selects among the very candidates
/// the lease holds; the inherited `PATH` where there are none, off Linux.
pub(super) fn surveying(
    inherited: impl IntoIterator<Item = (OsString, OsString)>,
    launchers: &[PathBuf],
) -> Vec<(OsString, OsString)> {
    let dirs = launchers.iter().filter_map(|launcher| launcher.parent());
    let path = std::env::join_paths(dirs)
        .ok()
        .filter(|path| !path.is_empty());
    let replaced = |name: &OsString| (name == "PATH") & path.is_some();
    let loaded = |name: &OsString| name.as_encoded_bytes().starts_with(b"LD_");
    let kept = inherited.into_iter();
    let mut kept: Vec<_> = kept
        .filter(|(name, _)| !loaded(name) & !replaced(name))
        .collect();
    kept.extend(path.map(|path| (OsString::from("PATH"), path)));
    kept
}

/// The inputs the servers `plans` seal are built from: `fixed`, then each
/// one's program tree — its package root, which holds its executable, or
/// its one system entry — and then each one's launch candidates again
/// ([`launches`]), each reserved by its route ([`routed`]).
pub(super) fn served(mut fixed: Vec<Served>, plans: &[Plan]) -> Vec<Served> {
    let trees = plans.iter().map(|plan| match &plan.intent.tree {
        Tree::Package { root } => root.clone(),
        Tree::System {} => plan.intent.executable.clone(),
    });
    fixed.extend(trees.map(Served::Source));
    fixed.extend(launches(plans.iter().map(|plan| &plan.connection.argv[..])));
    fixed
}

/// Every file the launch each argv of `argvs` starts with may name, as a
/// server box's lookup selects among them on the host
/// (`hands::ServerProgram::candidates`): an absolute launch as spelled, a
/// bare name in each directory of the server `PATH`, each whether or not it
/// stands, so a dangling candidate ahead of the one found, which a writer
/// could make live, is reserved by its route as the found one is.
pub(super) fn launches<'a>(argvs: impl IntoIterator<Item = &'a [String]>) -> Vec<Served> {
    launches_with(argvs, &ServerProgram::candidates)
}

/// [`launches`], each name's candidates given by `candidates`: the house
/// `_with` seam a test looks a bare name up in a tree of its own by.
pub(super) fn launches_with<'a>(
    argvs: impl IntoIterator<Item = &'a [String]>,
    candidates: &dyn Fn(&str) -> Vec<PathBuf>,
) -> Vec<Served> {
    let names = argvs.into_iter().filter_map(|argv| argv.first());
    let found = names.flat_map(|name| candidates(name));
    found.map(Served::Source).collect()
}

/// The most symlinks one input's route may meet: D5's hop bound (MB3).
pub(super) const HOPS_MAX: usize = 40;

/// Why an input's route cannot be reserved: each refuses by MB3's identity
/// cause naming the input, never reserving less.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(in crate::engine) enum Unroutable {
    #[error("is not absolute, as a relative or empty PATH entry or a relative TMPDIR names it")]
    Relative,
    #[error("meets a component that cannot be read")]
    Unread,
    #[error("meets more than {} symlinks", HOPS_MAX)]
    Hops,
}

/// Every input of `inputs` by its route, once: each symlink met on its way,
/// walked one component at a time as the kernel resolves it, whether it
/// names the input itself, a directory above it (`/lib -> usr/lib`) or a
/// link a link names (`/usr/bin/x -> /etc/alternatives/x`), each by the
/// path it lies at; then where the input resolves, as the input it is. A
/// route that is not absolute, meets a component that cannot be read, or
/// meets more than [`HOPS_MAX`] symlinks refuses by MB3's identity cause
/// naming the input and why ([`Unroutable`]), never reserving less. One
/// that meets a component that does not stand is reserved there, as the
/// input it is, and walked no further: no managed writer can make that
/// component stand without writing the directory it would lie in, which
/// lies above it, so no writer admitted beside it can make the rest of the
/// spelling lead anywhere, `..` and all. A dangling candidate is so
/// reserved at its first missing component. Within `budget`'s deadline.
pub(super) fn routed(inputs: &[Served], budget: &Budget) -> Result<Vec<Served>, Unknown> {
    let mut routes = Vec::new();
    for input in inputs {
        for met in route(input, budget)? {
            if !routes.contains(&met) {
                routes.push(met);
            }
        }
    }
    Ok(routes)
}

/// [`routed`] for the one input `input`.
fn route(input: &Served, budget: &Budget) -> Result<Vec<Served>, Unknown> {
    let (spelled, kind): (&Path, fn(PathBuf) -> Served) = match input {
        Served::Source(path) | Served::Link(path) => (path, Served::Source),
        Served::Entries(path) => (path, Served::Entries),
    };
    let unrouted = |why| Unknown::Unrouted(spelled.to_path_buf(), why);
    let relative = || unrouted(Unroutable::Relative);
    spelled.is_absolute().then_some(()).ok_or_else(relative)?;
    let (mut met, mut at) = (Vec::new(), PathBuf::new());
    let mut pending: Vec<OsString> = spelled.iter().rev().map(OsStr::to_os_string).collect();
    while let Some(part) = pending.pop() {
        budget.check()?;
        // Each part taken as spelled: `.` nothing, `..` the parent, `/`
        // staying itself.
        if part == "." {
            continue;
        }
        if part == ".." {
            at.pop();
            continue;
        }
        let next = at.join(&part);
        match hop(&next) {
            Ok(Some(target)) => {
                met.push(Served::Link(next));
                let hops = || unrouted(Unroutable::Hops);
                (met.len() <= HOPS_MAX).then_some(()).ok_or_else(hops)?;
                pending.extend(target.iter().rev().map(OsStr::to_os_string));
            }
            Ok(None) => at = next,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                at = next;
                break;
            }
            Err(_) => return Err(unrouted(Unroutable::Unread)),
        }
    }
    met.push(kind(at));
    Ok(met)
}

/// Where the symlink `next` points, read without following it; none where
/// `next` stands and is no link.
fn hop(next: &Path) -> std::io::Result<Option<PathBuf>> {
    let found = std::fs::symlink_metadata(next)?;
    let link = found.file_type().is_symlink();
    link.then(|| std::fs::read_link(next)).transpose()
}

/// Whether every writer of `writers`, each named with its run, is kept
/// from every operating-system root ([`SYSTEM_ROOTS`]), as spelled, then
/// from every input of `serves`, and then from the coordination root
/// `coordination`, whose reservations hold those inputs: a writer able to
/// rewrite them could clear the very lease that keeps later writers out,
/// whatever evidence is at stake. Each is observed now, the mount table
/// read by `coordination`'s observer once, beside the reserved inputs, as
/// the first boxed writer is compared with them, within `budget`'s
/// deadline: MB3's identity cause naming the first that is not. With
/// nothing reserved no server is at stake, and nothing is compared.
pub(super) fn kept<'w>(
    writers: impl IntoIterator<Item = (&'w str, &'w Writing)>,
    serves: &[Served],
    coordination: &Root,
    budget: &Budget,
) -> Result<(), Unknown> {
    let Some(first) = serves.first() else {
        return Ok(());
    };
    let roots = SYSTEM_ROOTS.map(|root| Served::Source(PathBuf::from(root)));
    let records = Served::Source(coordination.path().to_path_buf());
    let mut table = None;
    for (run, writer) in writers {
        budget.check()?;
        let (site, run) = (writer.site.clone(), run.to_string());
        let Reaching::Boxed { writable, .. } = &writer.reach else {
            return Err(Unknown::Unconfined { site, run });
        };
        let table = match &mut table {
            Some(table) => table,
            None => table.insert(coordination.mounts()(budget, first.path())?),
        };
        for served in roots.iter().chain(serves).chain([&records]) {
            if writes(writable, served, table)? {
                let served = served.path().to_path_buf();
                return Err(Unknown::Serving { site, run, served });
            }
        }
    }
    Ok(())
}

/// Whether any root of `writable` can write `served`: by canonical path or
/// identity, or by the parts of filesystems the mount table `table` shows
/// each in; an error where the table does not prove where one lies. A
/// link is its own entry, never what it names: a root writes it where it
/// lies at or above the directory the link lies in, by path or identity,
/// as the table shows a root holding that directory ([`Table::reaches`]);
/// what it names is reserved as the route's next input.
fn writes(writable: &[PathBuf], served: &Served, table: &Table) -> Result<bool, Unknown> {
    let guarded = match served {
        Served::Source(path) | Served::Entries(path) => Guarded::of(path.clone()),
        Served::Link(link) => Guarded::of(holding(link).to_path_buf()),
    };
    for root in writable {
        let named = match served {
            Served::Source(_) => covers(root, &guarded),
            Served::Entries(_) | Served::Link(_) => {
                beneath(&guarded, &Guarded::of(resolved(root.clone())))
            }
        };
        if named || table.reaches(root, served)? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Whether every writer of the incoming `claim` is kept from every input
/// an attempt of `standing` reserved, and from the coordination root
/// `coordination` that holds them ([`kept`]), within `budget`.
pub(super) fn unserved(
    claim: &Claim,
    standing: &[Standing],
    coordination: &Root,
    budget: &Budget,
) -> Result<(), Unknown> {
    let serves: Vec<Served> = standing
        .iter()
        .flat_map(Standing::serves)
        .cloned()
        .collect();
    let writers: Vec<Writing> = claim.reserved.writers.iter().map(Writing::from).collect();
    let writers = writers.iter().map(|writer| (claim.run.as_str(), writer));
    kept(writers, &serves, coordination, budget)
}

/// Whether every boxed writer of the incoming `claim` is kept from the
/// coordination root at `root`, on every run, whatever is at stake and
/// whatever is served, by the test [`kept`] holds that root to, the root's
/// path read even where it does not stand ([`covers_spelled`]; operator ruling
/// of 2026-10-30 on the chief's H2, the controller's on H3): a reservation is
/// a plain file, so a boxed writer that reached the root could clear its own
/// record's writers before a broker extended beside it, and one admitted where
/// the root could not be opened could repair the walk and stand beside a later
/// broker unrecorded. MB3's identity cause naming the root, or naming why its
/// reach is not proved: a mount table that does not prove where it lies, or,
/// by its own cause, a root with no absolute path, where the host names no
/// HOME. Admission asks it
/// under the lock, the root open ([`super::exclusion::compared`]), and
/// [`records_unopened`] where it cannot be. A writer Brokkr cannot box holds
/// the user's write anyway, so a forged record gives it nothing more, and it
/// is left to [`kept`] beside a live broker and to the comparison beside
/// evidence (ruling 21).
pub(super) fn records_kept(
    claim: &Claim,
    root: &Path,
    mounts: Mounts,
    budget: &Budget,
) -> Result<(), Unknown> {
    let records = Served::Source(root.to_path_buf());
    let mut table = None;
    for writer in claim.reserved.writers.iter().map(Writing::from) {
        budget.check()?;
        let Reaching::Boxed { writable, .. } = &writer.reach else {
            continue;
        };
        root.is_absolute().then_some(()).ok_or(Unknown::Homeless)?;
        if reach_records(writable, &records, (mounts, budget), &mut table)? {
            return Err(Unknown::Serving {
                site: writer.site,
                run: claim.run.clone(),
                served: root.to_path_buf(),
            });
        }
    }
    Ok(())
}

/// [`records_kept`] before an attempt proceeds holding no lease (C1): the
/// coordination root under `home` could not be opened, so its spelled path is
/// compared, the mount table read as production reads it, before `deadline`.
/// No reservation or lock is involved: there is no root to hold them.
pub(super) fn records_unopened(
    claim: &Claim,
    home: &Path,
    deadline: Instant,
) -> Result<(), Unknown> {
    let budget = Budget::new(deadline);
    records_kept(claim, &coordination(home), observed, &budget)
}

/// Whether `writable` writes the coordination root `records`, the mount table
/// read by `mounts` once, as the first boxed writer is compared.
#[cfg(target_os = "linux")]
fn reach_records(
    writable: &[PathBuf],
    records: &Served,
    (mounts, budget): (Mounts, &Budget),
    table: &mut Option<Table>,
) -> Result<bool, Unknown> {
    let table = match table {
        Some(table) => table,
        None => table.insert(mounts(budget, records.path())?),
    };
    let spelled = writable
        .iter()
        .any(|root| covers_spelled(root, records.path()));
    Ok(spelled || writes(writable, records, table)?)
}

/// Off Linux no mount table is read (decision 0063), so a run with nothing at
/// stake is not refused for the want of one: path and identity decide alone.
#[cfg(not(target_os = "linux"))]
fn reach_records(
    writable: &[PathBuf],
    records: &Served,
    _: (Mounts, &Budget),
    _: &mut Option<Table>,
) -> Result<bool, Unknown> {
    let spelled = |root: &PathBuf| covers_spelled(root, records.path());
    Ok(writable.iter().any(spelled))
}

/// The lease `held` extended to the inputs `serves` its attempt's servers
/// are built from, before any of them starts: under the admission lock
/// taken before `deadline`, the inventory read and proven attempts
/// recovered as admission reads and recovers them, `serves` routed
/// ([`routed`]) beside every input the attempt reserved before, which it
/// keeps, every participant's writers kept from them all ([`kept`]), its
/// own attempt's among them, and its reservation sealed again holding
/// them, all before the lock is released. Every seal it made is folded into `commits` as it is made, so
/// one that committed unsynced is kept whatever fails after it.
pub(super) fn extended(
    held: &Held,
    serves: Vec<Served>,
    deadline: Instant,
    commits: &Cell<Commit>,
) -> Result<(), Unadmitted> {
    extended_with(held, serves, (deadline, commits), &|| {})
}

/// [`extended`] where `held` names the attempt's lease. With the fence
/// lifted every attempt that brokers holds a lease whatever is at stake,
/// so one admitted without one, its coordination root unprotectable with
/// nothing at stake ([`super::admission::coordinated`]), is refused as an
/// attempt whose reservation no longer stands
/// (`a_brokered_attempt_holding_no_lease_is_refused_once_the_fence_lifts`).
/// While the fence stands no public engine brokers, and a test provisioning
/// an attempt it never admitted extends nothing.
pub(super) fn held_extended(
    held: Option<&Held>,
    serves: Vec<Served>,
    deadline: Instant,
    commits: &Cell<Commit>,
) -> Result<(), Unadmitted> {
    match held {
        Some(held) => extended(held, serves, deadline, commits),
        None => match McpFence::current() {
            #[cfg(any(test, target_os = "linux"))]
            McpFence::Lifted => Err(Unknown::Lost.into()),
            #[cfg(any(test, not(target_os = "linux")))]
            McpFence::Standing => Ok(()),
        },
    }
}

/// [`extended`], `between` run between the comparison and the seal: the
/// house `_with` seam a test holds the comparison open at; nothing in
/// production.
pub(super) fn extended_with(
    held: &Held,
    serves: Vec<Served>,
    (deadline, commits): (Instant, &Cell<Commit>),
    between: &dyn Fn(),
) -> Result<(), Unadmitted> {
    let root = &held.coordination;
    let _lock = locked(root.dir(), deadline)?;
    let mut budget = Budget::new(deadline);
    let found = inventory(
        root.dir(),
        root.path(),
        [&held.repo, &held.run],
        &mut budget,
    )?;
    let (standing, commit) = recovered(root, found.standing, &[], &budget)?;
    commits.set(commits.get().and(commit));
    let mine = |found: &&Standing| (found.repo == held.repo) & (found.run == held.run);
    let mine = standing
        .iter()
        .find(mine)
        .and_then(|found| found.whole.clone());
    let mut reservation = mine.ok_or(Unknown::Lost)?;
    let mut reserved = (reservation.attempts.remove(&held.attempt)).ok_or(Unknown::Lost)?;
    let mut reserves = reserved.serves.clone();
    for met in routed(&serves, &budget)? {
        if !reserves.contains(&met) {
            reserves.push(met);
        }
    }
    let writers = standing.iter().flat_map(|found| {
        let run = found.run.as_str();
        found.writers().map(move |writer| (run, writer))
    });
    kept(writers, &reserves, root, &budget)?;
    between();
    reserved.serves = reserves;
    reservation.attempts.insert(held.attempt.clone(), reserved);
    commits.set(commits.get().and(sealed(root, &reservation, &budget)?));
    Ok(())
}
