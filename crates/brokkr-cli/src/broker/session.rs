//! The broker's session over one plan (slice two U6c2): the binding and
//! admission `brokkr broker serve` consumes before anything is served.
//!
//! A plan is bound only where the attempt's sealed inventory, read beside
//! it through owner-only directories under the protected
//! `HOME/.local/state/brokkr/capabilities/<repo>/<run>/<attempt>/` root
//! (D6), pins its locator and digest, its bytes hash to that digest and
//! parse as a closed [`Plan`] written for that repository, run and attempt.
//! HOME is the engine's, which this process inherited, and never anything
//! the locator says: the locator's root must be that very directory.
//! Then every plan field is checked in MB3's refusal order, and the box
//! intent against the server box the hands' server profile prepares from
//! it (U6c4), before any secret is looked up or anything is started. The
//! box is prepared by a private observer, `broker observe`, in a process
//! group of its own under the absolute startup deadline (U6c5b): it rebinds
//! the same plan, and hands back a closed record and the checked handles
//! over a private socket, the store's admitted handle last (U6c5c). The
//! sources it observed must be the ones the plan sealed, and every writer
//! it observed one the plan sealed: it reports the writers it observed
//! joined with the sealed ones, so that set equalling the sealed one proves
//! the observed a subset of the sealed, never that the two are equal.
//! Cancellation or the deadline kills its group and reaps it. The plan and
//! where it lies stay here; only the shared protocol types leave.

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsStr;
use std::io::Read;
use std::os::fd::OwnedFd;
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant};

use brokkr_core::canonical;
use brokkr_protocol::broker::{BoxIntent, Inventory, Owner, Plan, Privilege, Refusal, Tree};
use brokkr_protocol::hands::{ServerBox, ServerProfile, ServerProgram};
use brokkr_protocol::native_controls::bounded_line;
use brokkr_protocol::{hands, secret};
use rustix::fs::{fstat, openat, FileType, Mode, OFlags, Stat, CWD};
use serde::{Deserialize, Serialize};

/// The protected root's components below the host HOME (D6).
const PROTECTED_ROOT: [&str; 4] = [".local", "state", "brokkr", "capabilities"];

/// The attempt's sealed inventory, beside its plans.
const INVENTORY: &str = "inventory.json";

/// The most bytes an inventory or a plan may hold: MB3's request bound.
const PLAN_BYTES_MAX: u64 = 1 << 20;

/// Uids that prove no owner: the kernel's overflow uid and the invalid -1.
const UNMAPPED_UIDS: [u32; 2] = [65_534, u32::MAX];

/// MB3's absolute startup deadline, which admission's observation shares
/// and never extends.
const STARTUP: Duration = Duration::from_secs(30);

/// The plan `locator` names, bound to this attempt at `digest` and checked
/// in MB3's refusal order, its box's sources observed afresh by the
/// private observer within the startup deadline and compared with what
/// the plan sealed; or the first refusal. Nothing is looked up or started.
pub(super) fn admit(locator: &Path, digest: &str) -> Result<Admitted, Refusal> {
    let deadline = Instant::now() + STARTUP;
    let (layout, plan) = bound(locator, digest)?;
    screened(&layout, &plan)?;
    let (record, handles) = observed(&layout, &plan, (locator, digest), deadline)?;
    compared(&plan.intent, record, handles)
}

/// `serve`'s private observer: the plan `locator` names rebound at
/// `digest`, screened, its box prepared and checked, and the record handed
/// back with the checked handles over the private socket on stdout.
/// Whatever cannot be handed back leaves identity unprotected.
#[cfg(target_os = "linux")]
pub(super) fn observe(locator: &Path, digest: &str) -> Result<(), Refusal> {
    helper::tethered()?;
    let observed = bound(locator, digest).and_then(|(layout, plan)| {
        screened(&layout, &plan)?;
        recorded(&layout, &plan)
    });
    let (record, server) = match observed {
        Ok(observed) => observed,
        Err(refusal) => (Record::Refused(Cause::of(&refusal)?), None),
    };
    helper::handed(&record, server.as_ref())
}

/// Off Linux no box stands, so there is nothing to observe.
#[cfg(not(target_os = "linux"))]
pub(super) fn observe(_: &Path, _: &str) -> Result<(), Refusal> {
    Err(Refusal::Unavailable)
}

/// What admission hands serving: each checked source's handle, by the
/// host path that named it, held until the launch mounts it, and the
/// store's admitted handle, held for its reader and never mounted (MB4).
pub(super) struct Admitted {
    _handles: Vec<(PathBuf, OwnedFd)>,
    _store: OwnedFd,
}

/// `holds`, or `refusal`.
fn ensure(holds: bool, refusal: Refusal) -> Result<(), Refusal> {
    match holds {
        true => Ok(()),
        false => Err(refusal),
    }
}

/// Where a locator says its plan lies: the protected root, and the
/// repository, run and attempt directories below it. `home` is the
/// trusted host HOME, never read from the locator.
struct Layout<'a> {
    parts: Vec<&'a OsStr>,
    home: PathBuf,
    attempt: &'a Path,
}

impl<'a> Layout<'a> {
    /// `locator` as `<root>/<repo>/<run>/<attempt>/<plan>` below at least
    /// one directory, or nothing. Whether `<root>` is the trusted
    /// protected root is [`Layout::open`]'s to prove.
    fn of(locator: &'a Path, home: PathBuf) -> Option<Layout<'a>> {
        let parts: Vec<&OsStr> = locator.iter().collect();
        (parts.len() >= 6).then_some(())?;
        Some(Layout {
            attempt: locator.parent()?,
            parts,
            home,
        })
    }

    /// The directory component `back` places before the plan's own name.
    fn named(&self, back: usize) -> &OsStr {
        self.parts[self.parts.len() - 1 - back]
    }

    /// The attempt directory, opened from `/` one component at a time
    /// without following a symlink: the locator's root is the very
    /// directory `root` describes, each ancestor of it is this user's or
    /// root's and writable by no one else unless it is a root-owned sticky
    /// directory, and the root and everything below it is this user's
    /// alone.
    fn open(&self, root: &Stat) -> Option<OwnedFd> {
        // The root's index among the components below `/`.
        let private = self.parts.len() - 6;
        let below = &self.parts[1..self.parts.len() - 1];
        descend(below.iter().copied(), |index, stat| {
            let trusted = (index != private) | same(stat, root);
            trusted & guarded(stat, index >= private)
        })
    }
}

/// The directory `names` lead to from `/`, opened one component at a time
/// without following a symlink, each component's `stat` admitted by
/// `admits` with its index.
fn descend<'n>(
    names: impl Iterator<Item = &'n OsStr>,
    mut admits: impl FnMut(usize, &Stat) -> bool,
) -> Option<OwnedFd> {
    let mut held = openat(CWD, "/", directory(), Mode::empty()).ok()?;
    for (index, name) in names.enumerate() {
        held = openat(&held, name, directory(), Mode::empty()).ok()?;
        admits(index, &fstat(&held).ok()?).then_some(())?;
    }
    Some(held)
}

fn directory() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

/// Whether two directories are one: their device and inode, never their
/// spelling.
fn same(one: &Stat, other: &Stat) -> bool {
    (one.st_dev, one.st_ino) == (other.st_dev, other.st_ino)
}

/// The engine's host HOME: `hands::home_dir` over the environment this
/// process inherited from the engine, absolute and canonical.
fn trusted_home() -> Option<PathBuf> {
    let environment: BTreeMap<String, String> = std::env::vars_os()
        .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
        .collect();
    let home = hands::home_dir(&environment);
    home.is_absolute().then_some(())?;
    std::fs::canonicalize(home).ok()
}

/// The trusted protected root under `home`, reached without following a
/// symlink below it.
fn protected_root(home: &Path) -> Option<Stat> {
    let names = home.iter().skip(1).chain(PROTECTED_ROOT.map(OsStr::new));
    fstat(descend(names, |_, _| true)?).ok()
}

/// This process's effective user.
fn euid() -> u32 {
    rustix::process::geteuid().as_raw()
}

/// Whether a directory `stat` describes protects what lies below it.
fn guarded(stat: &Stat, private: bool) -> bool {
    let mode = stat.st_mode;
    match private {
        true => (stat.st_uid, mode & 0o077) == (euid(), 0),
        false => {
            let owner = [euid(), 0].contains(&stat.st_uid);
            let sticky = (stat.st_uid, mode & 0o1000) == (0, 0o1000);
            owner & ((mode & 0o022 == 0) | sticky)
        }
    }
}

/// The bytes of `name` in the held directory `dir`: a regular file of one
/// link, this user's alone, opened without following a symlink and judged
/// and read on that one handle, at most [`PLAN_BYTES_MAX`] of them.
fn protected(dir: &OwnedFd, name: &OsStr) -> Option<Vec<u8>> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = openat(dir, name, flags, Mode::empty()).ok()?;
    let held = fstat(&file).ok()?;
    let shape = (
        FileType::from_raw_mode(held.st_mode).is_file(),
        held.st_uid,
        held.st_mode & 0o077,
        held.st_nlink,
    );
    (shape == (true, euid(), 0, 1)).then_some(())?;
    let mut bytes = Vec::new();
    std::fs::File::from(file)
        .take(PLAN_BYTES_MAX + 1)
        .read_to_end(&mut bytes)
        .ok()?;
    (bytes.len() as u64 <= PLAN_BYTES_MAX).then_some(bytes)
}

/// The plan `locator` names, where the attempt's inventory pins exactly it
/// at `digest`, its protected bytes hash to `digest`, they parse as a plan
/// and the plan names the repository, run and attempt it lies under. A
/// caller's file and its true digest alone confer nothing, nor does a
/// tree of the same shape under any root but the engine's.
fn bound<'a>(locator: &'a Path, digest: &str) -> Result<(Layout<'a>, Plan), Refusal> {
    let home = trusted_home().ok_or(Refusal::Unbound)?;
    let root = protected_root(&home).ok_or(Refusal::Unbound)?;
    let layout = Layout::of(locator, home).ok_or(Refusal::Unbound)?;
    let dir = layout.open(&root).ok_or(Refusal::Unbound)?;
    let inventory = protected(&dir, OsStr::new(INVENTORY)).ok_or(Refusal::Unbound)?;
    let inventory: Inventory = serde_json::from_slice(&inventory).map_err(|_| Refusal::Unbound)?;
    let pins: Vec<&str> = inventory
        .plans
        .iter()
        .filter(|pin| pin.locator == locator)
        .map(|pin| pin.digest.as_str())
        .collect();
    ensure(pins == [digest], Refusal::Unbound)?;
    let bytes = protected(&dir, layout.named(0)).ok_or(Refusal::Unbound)?;
    ensure(canonical::sha256_bytes(&bytes) == digest, Refusal::Unbound)?;
    let plan: Plan = serde_json::from_slice(&bytes).map_err(|_| Refusal::Unbound)?;
    ensure(owns(&layout, &plan.owner), Refusal::Unbound)?;
    Ok((layout, plan))
}

/// Whether `owner` names the repository, run and attempt directories the
/// plan lies in, the attempt one portable path component. A locator's
/// components are never empty, `.` or `..`, so equality excludes those.
fn owns(layout: &Layout<'_>, owner: &Owner) -> bool {
    let named = [layout.named(3), layout.named(2), layout.named(1)];
    let owned = [&owner.repo, &owner.run, &owner.attempt].map(OsStr::new);
    let portable = |byte: u8| byte.is_ascii_alphanumeric() | b"._-".contains(&byte);
    (named == owned) & owner.attempt.bytes().all(portable)
}

/// The fields of a bound plan that need no source observed, in MB3's
/// refusal order: its own shape, the binding names and the fixed keys.
/// The broker screens them itself before its observer starts, so a
/// binding name's cause never has to cross back from it. An invalid
/// binding name refuses with decision 0012's cause through the one bounded
/// sink: the plan wrote the name, so a control character in it is escaped
/// rather than ending the line, NUL as `\u{0}`, and a long one is cut.
fn screened(layout: &Layout<'_>, plan: &Plan) -> Result<(), Refusal> {
    ensure(well_formed(layout, plan), Refusal::Unbound)?;
    plan.secrets
        .iter()
        .try_for_each(|name| secret::validate_name(name))
        .map_err(|cause| Refusal::Name(bounded_line(&cause.replace('\0', NUL))))?;
    let reserved = plan
        .secrets
        .iter()
        .any(|name| hands::server_environment().any(|fixed| fixed == name));
    ensure(!reserved, Refusal::StartupInputs)
}

/// The box of a screened plan, in MB3's refusal order: the program tree,
/// the seat's reach and every source the observer reads. The program and
/// the box are the hands' server profile's to prepare, and the plan must
/// have sealed the program it resolves. Nothing is looked up or started.
fn prepared(layout: &Layout<'_>, plan: &Plan) -> Result<ServerBox, Refusal> {
    let intent = &plan.intent;
    let program = ServerProgram::resolve(&plan.connection.argv[0], &layout.home)?;
    ensure(sealed(intent, &program), Refusal::ProgramTree)?;
    let profile = ServerProfile {
        reach: &intent.reach,
        network: &intent.network,
        bootstrap: &intent.bootstrap.path,
        arguments: &plan.connection.argv[1..],
        writers: &intent.writers.uids,
    };
    ServerBox::prepare(&program, &profile)
}

/// The checks after a box stands, in MB3's order: the identity facts,
/// then MB4's store identity against the seat's reach and the box (U6c5c),
/// whose handle is the store's reader's.
fn after(intent: &BoxIntent, server: &ServerBox) -> Result<OwnedFd, Refusal> {
    identity(intent, server)?;
    server.store(&intent.excluded.store, &intent.reach)
}

/// NUL as a binding name's cause spells it: `\u{0}`, never `\0`.
const NUL: &str = concat!("\\", "u{0}");

/// Whether the plan sealed the program the profile resolved: its very
/// canonical executable, and the same one of MB3's trees.
fn sealed(intent: &BoxIntent, program: &ServerProgram) -> bool {
    let tree = match (&intent.tree, program.tree()) {
        (Tree::System {}, Tree::System {}) => true,
        (Tree::Package { root }, Tree::Package { root: resolved }) => root == resolved,
        (Tree::System {}, Tree::Package { .. }) | (Tree::Package { .. }, Tree::System {}) => false,
    };
    tree & (intent.executable == program.executable())
}

/// Every root of the seat's reach, writable and readable.
fn roots(intent: &BoxIntent) -> impl Iterator<Item = &Path> {
    let reach = &intent.reach;
    reach
        .writable
        .iter()
        .chain(&reach.readable)
        .map(PathBuf::as_path)
}

/// Whether the plan is the closed shape the engine seals: every digest,
/// the owning repository's included, a sha256, every name non-empty, the
/// server `cap-<capability>` (D5), the tools a non-empty set, the fixed
/// environment names the server box's own, in its order, and the binding
/// names a set, possibly empty, every tool and argv member non-empty
/// (tool-dialect v1), the clearance receipt for this dialect, the
/// bootstrap this binary, the attempt among the control roots, and every
/// sealed path absolute and normal.
fn well_formed(layout: &Layout<'_>, plan: &Plan) -> bool {
    let intent = &plan.intent;
    let owner = &plan.owner;
    let digests = [
        &owner.repo,
        &plan.dialect.digest,
        &plan.dialect.definition,
        &plan.clearance.dialect,
        &plan.clearance.policy,
        &intent.sources.digest,
        &intent.bootstrap.digest,
    ];
    let names = [
        &plan.capability,
        &plan.dialect.name,
        &plan.dialect.version,
        &owner.effect,
        &owner.site,
        &owner.instance,
    ];
    let argv = &plan.connection.argv;
    let this = std::env::current_exe().ok();
    digests
        .iter()
        .all(|digest| canonical::is_sha256_hex(digest))
        & names.iter().all(|name| !name.is_empty())
        & (plan.server == format!("cap-{}", plan.capability))
        & set(&plan.tools)
        & plan.tools.iter().all(|tool| !tool.is_empty())
        & (plan.secrets.is_empty() | set(&plan.secrets))
        & intent
            .environment
            .iter()
            .map(String::as_str)
            .eq(hands::server_environment())
        & !argv.is_empty()
        & argv.iter().all(|arg| !arg.is_empty())
        & (plan.clearance.dialect == plan.dialect.digest)
        & (this.as_deref() == Some(intent.bootstrap.path.as_path()))
        & intent
            .excluded
            .control
            .iter()
            .any(|root| root == layout.attempt)
        & sealed_paths(intent).all(normal)
}

/// Every path the box intent seals beside the executable and its tree.
fn sealed_paths(intent: &BoxIntent) -> impl Iterator<Item = &Path> {
    let excluded = &intent.excluded;
    let fixed = [&excluded.store, &intent.bootstrap.path];
    roots(intent).chain(excluded.control.iter().chain(fixed).map(PathBuf::as_path))
}

/// A non-empty list with no name twice.
fn set(names: &[String]) -> bool {
    let unique: std::collections::BTreeSet<&String> = names.iter().collect();
    (!names.is_empty()) & (unique.len() == names.len())
}

/// An absolute path with no `.` or `..` component.
fn normal(path: &Path) -> bool {
    let plain = |part: Component<'_>| matches!(part, Component::RootDir | Component::Normal(_));
    path.is_absolute() & path.components().all(plain)
}

/// Whether either path lies within the other.
fn overlaps(one: &Path, other: &Path) -> bool {
    one.starts_with(other) | other.starts_with(one)
}

/// The identity facts the plan seals: at least one managed writer, each
/// with a mapped uid, and the control roots outside the seat's reach and
/// every path the box mounts. The sealed sources are bounded by being the
/// observed ones, which the observer's own bounds hold ([`compared`]).
fn identity(intent: &BoxIntent, server: &ServerBox) -> Result<(), Refusal> {
    let uids = &intent.writers.uids;
    let mapped = !uids.is_empty() & !uids.iter().any(|uid| UNMAPPED_UIDS.contains(uid));
    let control = &intent.excluded.control;
    let mut exposed = roots(intent).chain(server.paths());
    let shared = exposed.any(|path| control.iter().any(|root| overlaps(root, path)));
    ensure(mapped & !shared, Refusal::Identity)
}

/// A cause the observer hands back, by its name on the socket: every
/// refusal it can meet once the broker has screened the plan. A binding
/// name's is the broker's own, and the serving cause comes after
/// admission, so neither has one; a name not here is no record.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum Cause {
    Unbound,
    StartupInputs,
    ProgramTree,
    LaunchInReach,
    BindOverlapsReach,
    Linked,
    Identity,
    Unavailable,
    StoreReachable,
    StoreInBox,
}

impl Cause {
    /// Every cause, in MB3's order.
    const ALL: [Cause; 10] = [
        Cause::Unbound,
        Cause::StartupInputs,
        Cause::ProgramTree,
        Cause::LaunchInReach,
        Cause::BindOverlapsReach,
        Cause::Linked,
        Cause::Identity,
        Cause::Unavailable,
        Cause::StoreReachable,
        Cause::StoreInBox,
    ];

    /// The refusal this cause names.
    fn refusal(self) -> Refusal {
        match self {
            Cause::Unbound => Refusal::Unbound,
            Cause::StartupInputs => Refusal::StartupInputs,
            Cause::ProgramTree => Refusal::ProgramTree,
            Cause::LaunchInReach => Refusal::LaunchInReach,
            Cause::BindOverlapsReach => Refusal::BindOverlapsReach,
            Cause::Linked => Refusal::Linked,
            Cause::Identity => Refusal::Identity,
            Cause::Unavailable => Refusal::Unavailable,
            Cause::StoreReachable => Refusal::StoreReachable,
            Cause::StoreInBox => Refusal::StoreInBox,
        }
    }

    /// `refusal`'s cause; one with none cannot be handed back, which
    /// leaves identity unprotected.
    fn of(refusal: &Refusal) -> Result<Cause, Refusal> {
        let named = Cause::ALL
            .into_iter()
            .find(|cause| cause.refusal() == *refusal);
        named.ok_or(Refusal::Identity)
    }
}

/// The observer's closed record (U6c5b): refused before a box stood, with
/// its cause, or what the box observed.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "lowercase")]
enum Record {
    Refused(Cause),
    Observed(Observation),
}

/// What a standing box observed: its source set, the managed writers' uids
/// where their privilege was proved confined, the cause a check after the
/// box refused with, none where it was admitted, and the host path of each
/// handle handed back with it, in the order they travel.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Observation {
    entries: u64,
    mounts: u64,
    digest: String,
    writers: Option<Vec<u32>>,
    refused: Option<Cause>,
    handles: Vec<PathBuf>,
}

/// The record of a screened plan, and its box and store handle where it
/// is admitted.
fn recorded(layout: &Layout<'_>, plan: &Plan) -> Result<(Record, Option<Held>), Refusal> {
    let server = match prepared(layout, plan) {
        Ok(server) => server,
        Err(refusal) => return Ok((Record::Refused(Cause::of(&refusal)?), None)),
    };
    let after = after(&plan.intent, &server);
    let refused = after.as_ref().err();
    let sources = server.sources();
    let handles = match refused {
        Some(_) => Vec::new(),
        None => server
            .handles()
            .map(|(path, _)| path.to_path_buf())
            .collect(),
    };
    let observation = Observation {
        entries: sources.entries,
        mounts: sources.mounts,
        digest: sources.digest.clone(),
        writers: server.writers().map(<[u32]>::to_vec),
        refused: refused.map(Cause::of).transpose()?,
        handles,
    };
    let admitted = after.ok().map(|store| (server, store));
    Ok((Record::Observed(observation), admitted))
}

/// An admitted box and its store's handle.
type Held = (ServerBox, OwnedFd);

/// The observer's `record` against what `intent` sealed, with the
/// `handles` it handed back: its own refusal; identity unprotected where
/// the observed sources are not the sealed ones or a writer it observed is
/// not sealed (its writers, the observed joined with the sealed, are not
/// exactly the sealed set), which comes before any cause after it in MB3's
/// order, or where the handles are not
/// those it names and then the store's; else the cause after the box, or
/// the admitted handles. The sealed facts were bound with the plan, so a
/// difference here is the filesystem's, never the plan's (SC1).
fn compared(
    intent: &BoxIntent,
    record: Record,
    mut handles: Vec<OwnedFd>,
) -> Result<Admitted, Refusal> {
    let observation = match record {
        Record::Refused(cause) => return Err(cause.refusal()),
        Record::Observed(observation) => observation,
    };
    let sealed = &intent.sources;
    let observed = (observation.entries, observation.mounts, &observation.digest);
    let sources = observed == (sealed.entries, sealed.mounts, &sealed.digest);
    let writers = match intent.writers.privilege {
        Privilege::Confined => observation.writers.as_ref().is_some_and(|uids| {
            let set = |uids: &[u32]| uids.iter().copied().collect::<BTreeSet<u32>>();
            set(uids) == set(&intent.writers.uids)
        }),
    };
    ensure(sources & writers, Refusal::Identity)?;
    if let Some(cause) = observation.refused {
        return Err(cause.refusal());
    }
    let store = handles
        .pop()
        .filter(|_| observation.handles.len() == handles.len());
    let store = store.ok_or(Refusal::Identity)?;
    Ok(Admitted {
        _handles: observation.handles.into_iter().zip(handles).collect(),
        _store: store,
    })
}

/// The observer's record of the plan `named` (its locator and digest) and
/// the handles it handed back: the private observer run in its own process
/// group, received before `deadline` unless the attempt is cancelled, which
/// ends the broker's own starter, and its whole group killed and reaped
/// either way. A blocked source operation holds only the observer, so
/// nothing of admission outlives it.
#[cfg(target_os = "linux")]
fn observed(
    _: &Layout<'_>,
    _: &Plan,
    named: (&Path, &str),
    deadline: Instant,
) -> Result<(Record, Vec<OwnedFd>), Refusal> {
    let parent = rustix::process::getppid();
    helper::supervised(named, deadline, &|| rustix::process::getppid() != parent)
}

/// Off Linux no box stands and no source is read: the record is taken
/// here, refused before any box.
#[cfg(not(target_os = "linux"))]
fn observed(
    layout: &Layout<'_>,
    plan: &Plan,
    _: (&Path, &str),
    _: Instant,
) -> Result<(Record, Vec<OwnedFd>), Refusal> {
    let (record, _) = recorded(layout, plan)?;
    Ok((record, Vec::new()))
}

/// The private observer's process and its socket (U6c5b; MB3): a
/// sequenced-packet pair, so one record is one message, read whole or not
/// at all, with the checked handles riding it as `SCM_RIGHTS`.
#[cfg(target_os = "linux")]
mod helper {
    use std::io::{IoSlice, IoSliceMut};
    use std::mem::MaybeUninit;
    use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
    use std::os::unix::process::CommandExt;
    use std::path::Path;
    use std::process::{Child, Command, Stdio};
    use std::time::{Duration, Instant};

    use brokkr_protocol::broker::Refusal;
    use rustix::io::Errno;
    use rustix::net::sockopt::{self, Timeout};
    use rustix::net::{
        recv, recvmsg, sendmsg, socketpair, AddressFamily, RecvAncillaryBuffer,
        RecvAncillaryMessage, RecvFlags, ReturnFlags, SendAncillaryBuffer, SendAncillaryMessage,
        SendFlags, SocketFlags, SocketType,
    };
    use rustix::process::{getppid, kill_current_process_group, kill_process_group, Pid, Signal};

    use super::{ensure, Held, Record};

    /// The most bytes one record may hold.
    const RECORD_MAX: usize = 128 << 10;

    /// The most handles one record hands back: a server box holds one for
    /// each source it mounts, its system set's aliases included, and its
    /// launcher; the store's rides last.
    const HANDLES_MAX: usize = 64;

    /// The longest one wait on the socket lasts before the deadline and
    /// cancellation are read again.
    const SLICE: Duration = Duration::from_millis(100);

    /// The observer of `named` run, and its record received before
    /// `deadline` unless `cancelled`; its process group, whatever it
    /// started included, then killed and reaped.
    pub(super) fn supervised(
        named: (&Path, &str),
        deadline: Instant,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(Record, Vec<OwnedFd>), Refusal> {
        let (unix, packets) = (AddressFamily::UNIX, SocketType::SEQPACKET);
        let pair = socketpair(unix, packets, SocketFlags::CLOEXEC, None);
        let (ours, theirs) = pair.ok().ok_or(Refusal::Identity)?;
        let mut helper = spawned(named, theirs)?;
        let received = received(&ours, deadline, cancelled);
        ended(&mut helper);
        received
    }

    /// This binary as `broker observe` of `named`, a process group of its
    /// own, `channel` its stdout and nothing else inherited. The command,
    /// and the broker's copy of `channel` with it, is gone once it starts,
    /// so the observer's end closing is the socket's end.
    fn spawned((locator, digest): (&Path, &str), channel: OwnedFd) -> Result<Child, Refusal> {
        let this = std::env::current_exe().ok().ok_or(Refusal::Identity)?;
        let started = Command::new(this)
            .args(["broker", "observe", "--plan"])
            .arg(locator)
            .args(["--plan-digest", digest])
            .stdin(Stdio::null())
            .stdout(Stdio::from(channel))
            .stderr(Stdio::null())
            .process_group(0)
            .spawn();
        started.ok().ok_or(Refusal::Identity)
    }

    /// Kill the observer's whole group and reap the observer: a member
    /// blocked in a filesystem call is woken by the kill.
    fn ended(helper: &mut Child) {
        kill_process_group(Pid::from_child(helper), Signal::KILL).ok();
        helper.wait().ok();
    }

    /// The one record on `socket` with the handles riding it, and then the
    /// observer's end, which closes the socket only once it has exited:
    /// both before `deadline` unless `cancelled`. Nothing, or anything
    /// more, is no record.
    fn received(
        socket: &OwnedFd,
        deadline: Instant,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(Record, Vec<OwnedFd>), Refusal> {
        let (record, handles) = message(socket, deadline, cancelled)?;
        let (after, more) = message(socket, deadline, cancelled)?;
        let one = !record.is_empty() & after.is_empty() & more.is_empty();
        ensure(one, Refusal::Identity)?;
        let record = serde_json::from_slice(&record);
        Ok((record.ok().ok_or(Refusal::Identity)?, handles))
    }

    /// The next whole message on `socket` and the handles riding it, read
    /// before `deadline` unless `cancelled`, in waits of at most [`SLICE`]:
    /// empty at the socket's end.
    fn message(
        socket: &OwnedFd,
        deadline: Instant,
        cancelled: &dyn Fn() -> bool,
    ) -> Result<(Vec<u8>, Vec<OwnedFd>), Refusal> {
        let mut bytes = vec![0; RECORD_MAX];
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(HANDLES_MAX))];
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            ensure(!left.is_zero() & !cancelled(), Refusal::Identity)?;
            let wait = left.clamp(Duration::from_millis(1), SLICE);
            let timed = sockopt::set_socket_timeout(socket, Timeout::Recv, Some(wait));
            timed.ok().ok_or(Refusal::Identity)?;
            let mut control = RecvAncillaryBuffer::new(&mut space);
            let mut iov = [IoSliceMut::new(&mut bytes)];
            let message = match recvmsg(socket, &mut iov, &mut control, RecvFlags::CMSG_CLOEXEC) {
                Err(Errno::AGAIN | Errno::INTR) => continue,
                answered => answered.ok().ok_or(Refusal::Identity)?,
            };
            // The observer's handles ride its record as the one control
            // message; a message without one, as the socket's end is,
            // carries none. Any other is closed with the buffer, unread.
            let handles = match control.drain().next() {
                Some(RecvAncillaryMessage::ScmRights(fds)) => fds.collect(),
                _ => Vec::new(),
            };
            let truncated = message
                .flags
                .intersects(ReturnFlags::TRUNC | ReturnFlags::CTRUNC);
            ensure(!truncated, Refusal::Identity)?;
            bytes.truncate(message.bytes);
            return Ok((bytes, handles));
        }
    }

    /// Tie this observer's life, and its whole group's, to the broker that
    /// started it: not begun where that broker has already ended, as the
    /// socket's creator is then no longer its parent, and otherwise
    /// watched. The broker sends nothing and its end of the socket is its
    /// alone, so a read of it returns only when the broker has ended, by
    /// whatever means, or closed it after killing the group itself: the
    /// watcher then kills the group, whatever the observer started
    /// included. A parent-death signal would kill the observer alone, and
    /// could do so before the watcher ran.
    pub(super) fn tethered() -> Result<(), Refusal> {
        let peer = sockopt::socket_peercred(std::io::stdout().as_fd());
        let peer = peer.ok().ok_or(Refusal::Identity)?;
        ensure(getppid() == Some(peer.pid), Refusal::Identity)?;
        let socket = std::io::stdout().as_fd().try_clone_to_owned();
        let socket = socket.ok().ok_or(Refusal::Identity)?;
        let watcher = std::thread::Builder::new().spawn(move || {
            recv(&socket, &mut [0; 1], RecvFlags::empty()).ok();
            kill_current_process_group(Signal::KILL).ok();
        });
        watcher.ok().ok_or(Refusal::Identity).map(drop)
    }

    /// Hand `record` back over stdout as one message, with the handles of
    /// `held`'s box, where it is admitted, riding it in the record's order
    /// and its store's last: within [`HANDLES_MAX`], they take one control
    /// message, and none.
    pub(super) fn handed(record: &Record, held: Option<&Held>) -> Result<(), Refusal> {
        let bytes = serde_json::to_vec(record).ok().ok_or(Refusal::Identity)?;
        let handles: Vec<BorrowedFd<'_>> = held
            .into_iter()
            .flat_map(|(server, store)| server.handles().map(|(_, fd)| fd).chain([store]))
            .map(AsFd::as_fd)
            .collect();
        let bounded = (bytes.len() <= RECORD_MAX) & (handles.len() <= HANDLES_MAX);
        ensure(bounded, Refusal::Identity)?;
        let mut space = [MaybeUninit::uninit(); rustix::cmsg_space!(ScmRights(HANDLES_MAX))];
        let mut control = SendAncillaryBuffer::new(&mut space);
        let mut messages = handles.chunks(HANDLES_MAX);
        let carried = messages.all(|fds| control.push(SendAncillaryMessage::ScmRights(fds)));
        ensure(carried, Refusal::Identity)?;
        let iov = [IoSlice::new(&bytes)];
        let sent = sendmsg(std::io::stdout(), &iov, &mut control, SendFlags::NOSIGNAL);
        ensure(sent == Ok(bytes.len()), Refusal::Identity)
    }
}
