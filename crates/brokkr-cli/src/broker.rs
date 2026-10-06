//! `brokkr broker serve` (decision 0077, design D5): the broker the
//! harness starts beside `brokkr hands`, one per held MCP capability, from
//! the configuration the engine writes. Its authority is the engine's plan
//! bound to the attempt and nothing a caller names: the command line holds
//! a plan locator and its digest, never a server argv, a grant or a secret
//! value.
//!
//! A plan is bound only where the attempt's sealed inventory, read beside
//! it through owner-only directories under the protected
//! `HOME/.local/state/brokkr/capabilities/<repo>/<run>/<attempt>/` root
//! (D6), pins its locator and digest, its bytes hash to that digest and
//! parse as a closed [`Plan`] written for that repository, run and attempt.
//! HOME is the engine's, which this process inherited, and never anything
//! the locator says: the locator's root must be that very directory.
//! Then every plan field and the box intent are checked in MB3's refusal
//! order, before any secret is looked up or anything is started. The
//! serving protections are later units' (slice two U6c2–U6f), so an
//! admitted plan is still refused with SD3's incomplete-serving cause, and
//! decision 0065's compile fence still refuses every MCP grant.

use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::Read;
use std::os::fd::OwnedFd;
use std::path::{Component, Path, PathBuf};
use std::process::ExitCode;

use brokkr_core::canonical;
use brokkr_protocol::broker::{BoxIntent, Inventory, Owner, Plan, Refusal, Tree};
use brokkr_protocol::{hands, secret};
use rustix::fs::{fstat, openat, FileType, Mode, OFlags, Stat, CWD};

use crate::cli_args::{BrokerCmd, BrokerServeArgs};

/// The longest plan locator taken, in bytes: macOS's `PATH_MAX`, the
/// smaller of the two supported hosts' (decision 0063).
const PLAN_LOCATOR_MAX: usize = 1024;

/// The protected root's components below the host HOME (D6).
const PROTECTED_ROOT: [&str; 4] = [".local", "state", "brokkr", "capabilities"];

/// The attempt's sealed inventory, beside its plans.
const INVENTORY: &str = "inventory.json";

/// The most bytes an inventory or a plan may hold: MB3's request bound.
const PLAN_BYTES_MAX: u64 = 1 << 20;

/// MB3's shared system bin directories: an executable directly in one is
/// a system entry, its own program tree.
const SYSTEM_BINS: [&str; 6] = [
    "/usr/bin",
    "/usr/sbin",
    "/usr/local/bin",
    "/usr/local/sbin",
    "/bin",
    "/sbin",
];

/// MB3's bounds on one source observation: entries and mount records.
const SOURCE_ENTRIES_MAX: u64 = 1_000_000;
const MOUNT_RECORDS_MAX: u64 = 65_536;

/// Uids that prove no owner: the kernel's overflow uid and the invalid -1.
const UNMAPPED_UIDS: [u32; 2] = [65_534, u32::MAX];

/// Why the broker takes no plan locator or digest, as the command line is
/// parsed. A well-formed one the broker does not serve is a [`Refusal`].
#[derive(Debug)]
pub(crate) enum BrokerError {
    Relative,
    Parent,
    TooLong,
    Digest,
}

impl std::fmt::Display for BrokerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BrokerError::Relative => formatter.write_str("a plan locator is an absolute path"),
            BrokerError::Parent => formatter.write_str("a plan locator names no parent directory"),
            BrokerError::TooLong => {
                write!(
                    formatter,
                    "a plan locator is at most {PLAN_LOCATOR_MAX} bytes"
                )
            }
            BrokerError::Digest => {
                formatter.write_str("a plan digest is 64 lowercase hex characters")
            }
        }
    }
}

impl std::error::Error for BrokerError {}

/// `--plan`: an absolute, lexically normal path of bounded length. Its
/// shape is all this checks; whether the engine wrote it for this attempt
/// is the binding's to decide.
pub(crate) fn plan_locator(text: &str) -> Result<PathBuf, BrokerError> {
    let path = Path::new(text);
    if text.len() > PLAN_LOCATOR_MAX {
        Err(BrokerError::TooLong)
    } else if !path.is_absolute() {
        Err(BrokerError::Relative)
    } else if path.components().any(|part| part == Component::ParentDir) {
        Err(BrokerError::Parent)
    } else {
        Ok(path.to_path_buf())
    }
}

/// `--plan-digest`: a sha256 in the one spelling every reader takes.
pub(crate) fn plan_digest(text: &str) -> Result<String, BrokerError> {
    match canonical::is_sha256_hex(text) {
        true => Ok(text.to_string()),
        false => Err(BrokerError::Digest),
    }
}

/// Serve the plan `command` names, or refuse before looking up a secret
/// or starting anything.
pub(crate) fn run(command: BrokerCmd) -> anyhow::Result<ExitCode> {
    match command {
        BrokerCmd::Serve(BrokerServeArgs { plan, plan_digest }) => {
            let (layout, plan) = bound(&plan, &plan_digest)?;
            admit(&layout, &plan)?;
            Err(Refusal::ServingIncomplete.into())
        }
    }
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

/// Decision 0012's cause for an invalid binding name, kept on one line:
/// the plan wrote the name, so a control character in it is escaped
/// rather than ending the line.
fn one_line(cause: String) -> Refusal {
    let escaped = cause.chars().map(|char| match char.is_control() {
        true => char.escape_default().to_string(),
        false => char.to_string(),
    });
    Refusal::Name(escaped.collect())
}

/// Check every field of a bound plan in MB3's refusal order: its own
/// shape, the binding names, the program tree, the seat's reach, the
/// identity facts and the store's exclusion. Nothing is looked up or
/// started.
fn admit(layout: &Layout<'_>, plan: &Plan) -> Result<(), Refusal> {
    let intent = &plan.intent;
    ensure(well_formed(layout, plan), Refusal::Unbound)?;
    plan.secrets
        .iter()
        .try_for_each(|name| secret::validate_name(name))
        .map_err(one_line)?;
    let reserved = plan
        .secrets
        .iter()
        .any(|name| intent.environment.contains(name));
    ensure(!reserved, Refusal::StartupInputs)?;
    let source = tree(&plan.connection.argv[0], intent, &layout.home)?;
    reach(intent, source)?;
    identity(intent, source)?;
    let store = intent.excluded.store.as_path();
    let reachable = roots(intent).any(|root| overlaps(store, root));
    ensure(!reachable, Refusal::StoreReachable)?;
    let mounted = binds(intent, source).any(|bind| overlaps(store, bind));
    ensure(!mounted, Refusal::StoreInBox)
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

/// What the box binds beside the system set: the program tree `source`
/// and the bootstrap.
fn binds<'a>(intent: &'a BoxIntent, source: &'a Path) -> impl Iterator<Item = &'a Path> {
    [source, intent.bootstrap.path.as_path()].into_iter()
}

/// Whether the plan is the closed shape the engine seals: every digest,
/// the owning repository's included, a sha256, every name non-empty, the
/// server `cap-<capability>` (D5), the tools and fixed environment names
/// non-empty sets and the binding names
/// a set, possibly empty, every tool and argv member non-empty
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
        & set(&intent.environment)
        & intent
            .environment
            .iter()
            .all(|name| secret::valid_name(name))
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

/// The box source the program tree binds, where the sealed tree is MB3's
/// layout of the sealed executable: a system entry is the executable
/// itself, directly in a shared system bin directory; a package is the
/// executable's parent, or its grandparent when that parent is a `bin` or
/// `sbin`, and never `/`, the host HOME or an ancestor of it. A launch
/// path relative to the cwd resolves nothing.
fn tree<'a>(launch: &str, intent: &'a BoxIntent, home: &Path) -> Result<&'a Path, Refusal> {
    let executable = &intent.executable;
    let searched = !launch.contains('/') | launch.starts_with('/');
    ensure(searched & normal(executable), Refusal::ProgramTree)?;
    let parent = executable.parent().ok_or(Refusal::ProgramTree)?;
    let system = SYSTEM_BINS.map(Path::new).contains(&parent);
    match (&intent.tree, system) {
        (Tree::System {}, true) => Ok(executable),
        (Tree::Package { root }, false) => {
            let named = |dir: &str| parent.file_name() == Some(OsStr::new(dir));
            // A directory named `bin` is never `/`, so it has a parent.
            let derived = match named("bin") | named("sbin") {
                true => parent.parent().unwrap_or(parent),
                false => parent,
            };
            // The trusted HOME is absolute, so `/` is among its ancestors.
            let widened = home.starts_with(derived);
            ensure((root == derived) & !widened, Refusal::ProgramTree)?;
            Ok(root)
        }
        (Tree::System {}, false) | (Tree::Package { .. }, true) => Err(Refusal::ProgramTree),
    }
}

/// The seat's reach against the box: the executable in no writable root,
/// then neither the program tree nor the bootstrap overlapping any root in
/// either direction.
fn reach(intent: &BoxIntent, source: &Path) -> Result<(), Refusal> {
    let writable = &intent.reach.writable;
    let launched = writable
        .iter()
        .any(|root| intent.executable.starts_with(root));
    ensure(!launched, Refusal::LaunchInReach)?;
    let overlapping =
        binds(intent, source).any(|bind| roots(intent).any(|root| overlaps(bind, root)));
    ensure(!overlapping, Refusal::BindOverlapsReach)
}

/// The identity facts the plan seals: at least one managed writer, each
/// with a mapped uid; a source observation within MB3's bounds; and the
/// control roots outside the seat's reach and the box.
fn identity(intent: &BoxIntent, source: &Path) -> Result<(), Refusal> {
    let uids = &intent.writers.uids;
    let mapped = !uids.is_empty() & !uids.iter().any(|uid| UNMAPPED_UIDS.contains(uid));
    let sources = &intent.sources;
    let bounded = (sources.entries <= SOURCE_ENTRIES_MAX) & (sources.mounts <= MOUNT_RECORDS_MAX);
    let control = &intent.excluded.control;
    let mut exposed = roots(intent).chain(binds(intent, source));
    let shared = exposed.any(|path| control.iter().any(|root| overlaps(root, path)));
    ensure(mapped & bounded & !shared, Refusal::Identity)
}
