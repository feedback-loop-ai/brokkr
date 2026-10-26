//! The operator's secrets store hidden from every workspace box (decision
//! 0065 slice two, U8h; SI1, MB4). With no folder state the store lies in
//! the worktree the box binds read-write, so every name in it would be
//! readable through the hands. The reach seals the store's canonical path,
//! every symbolic link on it resolved once for the run by the engine,
//! outside any box, and kept ([`store_place`]; operator ruling 1 of
//! 2026-10-23, ruling 15 of 2026-10-24, ruling 18 of 2026-10-25), the one
//! place the engine also makes it at, empty and owner-only, before launch
//! where it is missing (ruling 2), and names it to a seat by; the
//! configured spelling is sealed beside it (ruling 17).
//!
//! Each box refuses a sealed path that no longer resolves to itself: one
//! that is not absolute and plain, or one on which a symbolic link now
//! lies anywhere, the final entry's included ([`unlinked`]), so no call can
//! retarget a link between boxes and leave a decoy for a later box to hide;
//! a store `brokkr secrets set` renames over adds no link. It refuses a
//! configured spelling that no longer leads to the sealed place
//! ([`Store::led`]), where an update would land unhidden. There the store
//! is one regular file of one link its user owns that grants its group and
//! others nothing (ruling 8), which no other mount the table shows puts
//! anywhere in the box's reach. Every place the box mounts is described
//! once, by source and destination, in the order it mounts them (rulings 7
//! and 13; [`Mounted`]): the host toolchain, the worktree, the bundle root
//! at [`SANDBOX_BUNDLE`], the Git common directory, the folders the
//! worktree pins, then each bind with its own pins. Where a writable mount
//! holds the store, each folder between its root and the store is bound
//! onto itself first, so no call renames the store's folder (ruling 6); the
//! worktree's are pinned with the artifact root's, outermost first, so
//! neither chain shadows the other's (ruling 13). Then `/dev/null` is
//! mounted read-only over the store at every place that description shows
//! it, which reads as denied, never as an empty file (ruling 10). A store
//! outside every mount is not mounted at all.
//!
//! A store absent in reach refuses, for bwrap would create the file on the
//! host to mount over it; so do a store an overlay shows, whose folders no
//! bind can pin without writing through to the host, one whose folders'
//! pins would uncover the hooks and config hidden in the Git directory, one
//! the artifact root holds (ruling 9), one whose sealed path, or a place a
//! pin or a hide of it would be spelled at, is not valid UTF-8 (ruling 12),
//! and one that any mount the description holds, a generated pin's
//! included, would shadow where it is to be hidden (ruling 13). Once the
//! argv is final the store and the description are observed again, and
//! either changed refuses. Every refusal carries one cause,
//! [`UNHIDDEN_STORE`], before the command runs. Each pin, and the artifact
//! root the box holds read-only, is observed by device and inode with the
//! description, and mounted from a handle opened beneath the sealed source
//! that holds it, which refuses unless it is that object ([`Hidden::beneath`],
//! U8k): what the box pins is what was observed, never a path resolved again.
//!
//! What this cannot close (operator rulings 3 and 14): a store renamed over
//! by `brokkr secrets set` or `rm` while one box runs detaches that box's
//! mount, so that one call or exec command can read the new store, as it
//! can the sibling files `secrets set` stages beside it during the update;
//! every later box hides it. A hidden store is still one the hands could
//! reach: MB4's refusal of an MCP plan whose store lies in reach stands
//! (ruling 5).

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use super::super::namespace::{namespace_path, Mount, Namespace};
use super::super::{expand_home, BindMode, HandsSpec, HOST_TOOLCHAIN_BINDS, SANDBOX_BUNDLE};
use super::{Beneath, Expected, Sealed};

/// The store's one place, as sealed and as the box checks it still holds.
mod place;
pub use place::store_place;
use place::unlinked;
pub(in crate::hands) use place::Store;
/// The store's own facts, judged against the host's mount table.
mod table;
use table::judged;

/// The cause every refusal here begins with, and the engine's where it
/// cannot make the store.
pub const UNHIDDEN_STORE: &str = "secrets store cannot be hidden from workspace hands";
use UNHIDDEN_STORE as CAUSE;

/// Why a workspace box cannot be built with the secrets store hidden.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(in crate::hands) enum Unhidden {
    #[error("{CAUSE}: '{}' is absent where the box would show it", .0.display())]
    Absent(PathBuf),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: '{}' is not one regular file of one link its user owns", .0.display())]
    Linked(PathBuf),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: '{}' grants its group or others access (mode {:04o})", .0.display(), .1)]
    Shared(PathBuf, u32),
    #[cfg(target_os = "linux")]
    #[error("{CAUSE}: a mount shows '{}' at '{}', in the box's reach", .0.display(), .1.display())]
    Shown(PathBuf, PathBuf),
    #[error(
        "{CAUSE}: the symbolic link '{}' lies on the sealed path '{}', so it no longer resolves to \
         itself",
        .1.display(),
        .0.display()
    )]
    Routed(PathBuf, PathBuf),
    #[error(
        "{CAUSE}: the sealed path '{}' is not absolute and plain, so it no longer resolves to itself",
        .0.display()
    )]
    Unresolved(PathBuf),
    #[error(
        "{CAUSE}: the configured store '{}' no longer leads to its sealed place '{}'",
        .0.display(),
        .1.display()
    )]
    Strayed(PathBuf, PathBuf),
    #[error("{CAUSE}: '{}' is not valid UTF-8, so no mount can be spelled at it", .0.display())]
    Unspelled(PathBuf),
    #[error(
        "{CAUSE}: the artifact root '{}' holds '{}', and its read-only mount would show the store \
         again",
        .1.display(),
        .0.display()
    )]
    Rooted(PathBuf, PathBuf),
    #[error(
        "{CAUSE}: a later mount shadows '{}', where the box would hide '{}'",
        .1.display(),
        .0.display()
    )]
    Shadowed(PathBuf, PathBuf),
    #[error(
        "{CAUSE}: an overlay shows '{}' at '{}', where no bind can pin its folders",
        .0.display(),
        .1.display()
    )]
    Overlaid(PathBuf, PathBuf),
    #[error(
        "{CAUSE}: pinning the folders of '{}' would uncover the hooks and config hidden in the Git \
         directory",
        .0.display()
    )]
    Gitted(PathBuf),
    #[error("{CAUSE}: the mount table does not prove where '{}' is mounted", .0.display())]
    Unmapped(PathBuf),
    #[error("{CAUSE}: '{}' cannot be observed: {}", .0.display(), .1)]
    Unobserved(PathBuf, std::io::ErrorKind),
    #[error("{CAUSE}: '{}' changed between the box's two observations", .0.display())]
    Changed(PathBuf),
}

impl From<Unhidden> for std::io::Error {
    fn from(unhidden: Unhidden) -> Self {
        std::io::Error::other(unhidden)
    }
}

/// What one observation saw of the store: its sealed place, its device,
/// inode and mount, and each place in the box `/dev/null` is mounted over.
#[derive(Debug, PartialEq, Eq)]
struct Seen {
    place: PathBuf,
    facts: Facts,
    targets: Vec<String>,
}

/// A store's device (major, minor), inode and mount id.
type Facts = ((u32, u32), u64, u64);

/// How a box mounts a host directory that could hold the store.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::hands) enum Held {
    /// Read-write: a call could rename a folder in it, so each folder
    /// between its root and the store is pinned.
    Writable,
    /// Read-only: nothing in it is renamed in the box.
    ReadOnly,
    /// An overlay's lower layer: a pin would bind the host's folder over
    /// the overlay, writing through to the host, so a store in it refuses.
    Overlay,
    /// A folder bound onto itself, which the box generates: no folder in
    /// it is pinned again, but it shows what it holds as any mount does.
    Pinned,
}

/// One mount the box makes that could show the store: the host directory
/// it shows, canonical where it resolves; where the box shows it; how; and
/// the declared bind it is, or that a pin lies in, where it is one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::hands) struct Mounted {
    pub(in crate::hands) host: PathBuf,
    pub(in crate::hands) shown: PathBuf,
    pub(in crate::hands) held: Held,
    pub(in crate::hands) bind: Option<usize>,
}

/// What one observation of a box saw: the store where one is to be hidden,
/// every mount the box makes, in order, its generated pins among them, and
/// the device and inode of each pin, then of the artifact root the box
/// holds read-only, where each stands (U8k).
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    seen: Option<Seen>,
    mounts: Vec<Mounted>,
    held: Vec<Option<(u64, u64)>>,
}

/// The store as a box's first observation saw and hid it, none where the
/// reach seals none or where it is absent outside every mount, beside the
/// mounts the box makes. The bundle root, and the artifact root and its
/// chain where the box mounts them, are kept for the second observation.
#[derive(Debug)]
pub(in crate::hands) struct Hidden {
    observed: Observed,
    bundle: Option<PathBuf>,
    root: Option<PathBuf>,
    chain: Vec<PathBuf>,
}

#[cfg(all(test, target_os = "linux"))]
thread_local! {
    /// A test's mount table in place of the host's, so an alias of the
    /// store that only a privileged mount could make is judged.
    pub(in crate::hands) static TABLE: std::cell::RefCell<Option<Vec<u8>>> =
        const { std::cell::RefCell::new(None) };
}

impl Hidden {
    /// Observe the store `sealed` seals for a box of `spec` seen at
    /// `workdir` and handed `bundle`, beside the artifact root `held` and
    /// the `chain` of folders the worktree holds above it (U8a). A store
    /// the root holds refuses: the root is mounted read-only after the
    /// hide, so its mount would show the store again (ruling 9).
    pub(in crate::hands) fn observe(
        spec: &HandsSpec,
        sealed: &Sealed,
        (workdir, bundle): (&Path, Option<&Path>),
        (held, chain): (Option<&Path>, &[PathBuf]),
    ) -> Result<Hidden, Unhidden> {
        let observed = observe(spec, sealed, (workdir, bundle), (held, chain))?;
        if let Some((seen, root)) = observed.seen.as_ref().zip(held) {
            let spelled = namespace_path(root);
            let mut targets = seen.targets.iter();
            if targets.any(|at| Path::new(at).starts_with(&spelled)) {
                return Err(Unhidden::Rooted(seen.place.clone(), root.to_path_buf()));
            }
        }
        let (bundle, chain) = (bundle.map(Path::to_path_buf), chain.to_vec());
        Ok(Hidden {
            observed,
            bundle,
            root: held.map(Path::to_path_buf),
            chain,
        })
    }

    /// Every mount the box makes, as observed, in order: a test holds the
    /// box's argv to it.
    #[cfg(all(test, target_os = "linux"))]
    pub(in crate::hands) fn mounts(&self) -> &[Mounted] {
        &self.observed.mounts
    }

    /// Bind onto itself, outermost first and each once, every folder the
    /// worktree holds above the store or above the artifact root, once the
    /// worktree is mounted and before any bind, so no write in the box
    /// renames either. Pinned apart, a shallow folder bound again from the
    /// host would shadow the other chain's deeper pins below it (ruling 13).
    pub(super) fn pin_worktree(&self, namespace: &mut Namespace) {
        self.pin_in(namespace, None);
    }

    /// Bind onto itself, shallowest first, each folder between the root of
    /// the writable bind `bind` and the store, once that bind is mounted
    /// and before anything is mounted below it.
    pub(super) fn pin(&self, namespace: &mut Namespace, bind: usize) {
        self.pin_in(namespace, Some(bind));
    }

    /// Mount each pin the description holds in the worktree (`None`) or in
    /// the declared bind `bind`, in its order.
    fn pin_in(&self, namespace: &mut Namespace, bind: Option<usize>) {
        let pins = self.observed.mounts.iter();
        for pin in pins.filter(|pin| (pin.held == Held::Pinned) & (pin.bind == bind)) {
            namespace.mount(Mount::Bind, &pin.host, &namespace_path(&pin.shown));
        }
    }

    /// Mount `/dev/null` read-only over the store at every place the box
    /// shows it, after every writable mount.
    pub(super) fn hide(&self, namespace: &mut Namespace) {
        for target in self.observed.seen.iter().flat_map(|seen| &seen.targets) {
            namespace.mount(Mount::RoBind, Path::new("/dev/null"), target);
        }
    }

    /// Confirm the reach `sealed` as the argv is final ([`Sealed::confirm`]),
    /// then observe the store and the box's mounts again: either not what
    /// the box was built from refuses.
    pub(in crate::hands) fn confirm(
        &self,
        sealed: &Sealed,
        spec: &HandsSpec,
        workdir: &Path,
    ) -> std::io::Result<()> {
        sealed.confirm(spec, workdir)?;
        let artifacts = (self.root.as_deref(), self.chain.as_slice());
        let again = observe(spec, sealed, (workdir, self.bundle.as_deref()), artifacts)?;
        let place = sealed.store().unwrap_or(Path::new(""));
        (again == self.observed)
            .then_some(())
            .ok_or_else(|| Unhidden::Changed(place.to_path_buf()).into())
    }

    /// Each folder the box pins, and the artifact root it holds read-only,
    /// as a source beneath the sealed one it lies in, the worktree seen at
    /// `workdir` or a bind's, held to what the observations saw (U8k).
    pub(in crate::hands) fn beneath(&self, sealed: &Sealed, workdir: &Path) -> Vec<Beneath> {
        let worktree = sealed.worktree();
        let pins = (self.observed.mounts.iter()).filter(|pin| pin.held == Held::Pinned);
        let pins = pins.map(|pin| {
            let within = pin.bind.map_or(worktree, |bind| &sealed.binds[bind].path);
            (&pin.host, within, pin.host.strip_prefix(within))
        });
        let root = (self.root.iter()).map(|root| (root, worktree, root.strip_prefix(workdir)));
        let held = pins.chain(root).zip(&self.observed.held);
        let beneath = held.map(|((host, within, rest), seen)| Beneath {
            host: host.clone(),
            within: within.to_path_buf(),
            rest: rest.unwrap_or(host).to_path_buf(),
            expected: Expected::Seen(*seen),
        });
        beneath.collect()
    }
}

/// The store `sealed` seals, observed now for a box of `spec` seen at
/// `workdir` and handed `bundle`, beside the box's mounts with the folders
/// of the artifact root's `chain` pinned among them, and each pin and the
/// artifact `root` by what stands there: no store where none is sealed or
/// where it is absent outside every mount; refused where it is not as
/// [`present`], [`judged`], [`pinned`] or [`placed`] needs.
fn observe(
    spec: &HandsSpec,
    sealed: &Sealed,
    seen: (&Path, Option<&Path>),
    (root, chain): (Option<&Path>, &[PathBuf]),
) -> Result<Observed, Unhidden> {
    let mounts = mounted(spec, sealed, seen);
    let worktree = sealed.worktree();
    let chained = chain.iter().map(|at| Mounted {
        host: worktree.join(at.strip_prefix(seen.0).unwrap_or(at)),
        shown: at.clone(),
        held: Held::Pinned,
        bind: None,
    });
    let mut pins: Vec<Mounted> = chained.collect();
    let Some(place) = present(sealed.store.as_ref(), &mounts)? else {
        let mounts = assembled(mounts, pins);
        let held = standing(&mounts, root);
        return Ok(Observed {
            seen: None,
            mounts,
            held,
        });
    };
    let facts = judged(place, &mounts)?;
    pins.extend(pinned(place, &mounts, sealed.common())?);
    let mounts = assembled(mounts, pins);
    let targets = placed(place, &mounts)?;
    let place = place.to_path_buf();
    let seen = Some(Seen {
        place,
        facts,
        targets,
    });
    let held = standing(&mounts, root);
    Ok(Observed { seen, mounts, held })
}

/// The device and inode of each folder `mounts` pins, then of the artifact
/// `root`, none where nothing stands, read without following a link.
fn standing(mounts: &[Mounted], root: Option<&Path>) -> Vec<Option<(u64, u64)>> {
    let pins = mounts.iter().filter(|pin| pin.held == Held::Pinned);
    let held = pins.map(|pin| pin.host.as_path()).chain(root);
    let found = held.map(|at| std::fs::symlink_metadata(at).ok());
    let found = found.map(|found| found.map(|found| (found.dev(), found.ino())));
    found.collect()
}

/// The sealed `store`'s place where a box must hide it, refused where it is
/// not valid UTF-8 (ruling 12), no longer resolves to itself ([`unlinked`])
/// or is no longer where its spelling leads ([`Store::led`]); none where
/// none is sealed, or where it is absent and none of `mounts` holds it. A
/// store absent where one does refuses: bwrap would create it on the host
/// to mount over it.
fn present<'s>(store: Option<&'s Store>, mounts: &[Mounted]) -> Result<Option<&'s Path>, Unhidden> {
    let Some(sealed) = store else {
        return Ok(None);
    };
    let store = sealed.place();
    let spelled = store.to_str().map(drop);
    spelled.ok_or_else(|| Unhidden::Unspelled(store.to_path_buf()))?;
    unlinked(store)?;
    sealed.led()?;
    if std::fs::symlink_metadata(store).is_ok() {
        return Ok(Some(store));
    }
    let held = mounts.iter().any(|mount| store.starts_with(&mount.host));
    (!held)
        .then_some(None)
        .ok_or_else(|| Unhidden::Absent(store.to_path_buf()))
}

/// Every host directory a box of `spec`, seen at `workdir` and handed
/// `bundle`, mounts from the reach `sealed` (ruling 7), in the order it
/// mounts them (ruling 13): the host toolchain, the worktree, the bundle
/// root at [`SANDBOX_BUNDLE`], the Git common directory where it lies
/// outside the worktree and each bind's source, each where the box shows
/// it and how. The private home and tmp and the generated identity files
/// are made fresh for each box under its own scratch, which the engine
/// never seals a store in, before the worktree.
fn mounted(
    spec: &HandsSpec,
    sealed: &Sealed,
    (workdir, bundle): (&Path, Option<&Path>),
) -> Vec<Mounted> {
    let worktree = sealed.worktree();
    let common = (sealed.common()).filter(|common| !common.starts_with(worktree));
    let held = |held: Held| {
        move |host: PathBuf, shown: &Path| Mounted {
            host,
            shown: shown.to_path_buf(),
            held,
            bind: None,
        }
    };
    let (fixed, writable) = (held(Held::ReadOnly), held(Held::Writable));
    let canonical =
        |host: &Path| std::fs::canonicalize(host).unwrap_or_else(|_| host.to_path_buf());
    let toolchain = HOST_TOOLCHAIN_BINDS.iter().map(Path::new);
    let mut mounted: Vec<Mounted> = toolchain.map(|host| fixed(canonical(host), host)).collect();
    mounted.push(writable(worktree.to_path_buf(), workdir));
    mounted.extend(bundle.map(|bundle| fixed(canonical(bundle), Path::new(SANDBOX_BUNDLE))));
    mounted.extend(common.map(|common| writable(common.to_path_buf(), common)));
    let binds = spec.binds.iter().zip(&sealed.binds).enumerate();
    mounted.extend(binds.map(|(index, (bind, source))| Mounted {
        host: source.path.clone(),
        shown: expand_home(&bind.path, sealed.home()),
        held: match bind.mode {
            BindMode::Rw => Held::Writable,
            BindMode::Ro => Held::ReadOnly,
            BindMode::Overlay => Held::Overlay,
        },
        bind: Some(index),
    }));
    mounted
}

/// `mounts` in the order the box makes them, the folder `pins` among them:
/// the worktree's own and the artifact root's, outermost first and each
/// once, after every mount but the binds; each bind's right after it.
fn assembled(mounts: Vec<Mounted>, pins: Vec<Mounted>) -> Vec<Mounted> {
    let (mut all, binds): (Vec<_>, Vec<_>) = mounts.into_iter().partition(|at| at.bind.is_none());
    let (mut own, bound): (Vec<_>, Vec<_>) = pins.into_iter().partition(|at| at.bind.is_none());
    own.sort_by_key(|pin| (pin.shown.components().count(), pin.shown.clone()));
    own.dedup_by(|later, kept| later.shown == kept.shown);
    all.extend(own);
    for bind in binds {
        let index = bind.bind;
        all.push(bind);
        all.extend(bound.iter().filter(|pin| pin.bind == index).cloned());
    }
    all
}

/// Each folder a writable one of `mounts` holds between its root and the
/// store at `place`, shallowest first, as a pin of that mount's. Refused
/// where the store lies in the Git `common` directory or a pin holds it,
/// for the pin would cover the hooks and config the box hid there before,
/// and where a pin's place in the box is not valid UTF-8 (ruling 12).
fn pinned(
    place: &Path,
    mounts: &[Mounted],
    common: Option<&Path>,
) -> Result<Vec<Mounted>, Unhidden> {
    let mut pins = Vec::new();
    for mount in mounts.iter().filter(|mount| mount.held == Held::Writable) {
        let Ok(rest) = place.strip_prefix(&mount.host) else {
            continue;
        };
        pins.extend(folders(rest).into_iter().map(|folder| Mounted {
            host: mount.host.join(folder),
            shown: mount.shown.join(folder),
            held: Held::Pinned,
            bind: mount.bind,
        }));
    }
    let gitted = common.is_some_and(|common| {
        let holds = pins.iter().any(|pin| common.starts_with(&pin.host));
        place.starts_with(common) | holds
    });
    (!gitted)
        .then_some(())
        .ok_or(Unhidden::Gitted(place.to_path_buf()))?;
    match pins.iter().find(|pin| pin.shown.to_str().is_none()) {
        Some(lossy) => Err(Unhidden::Unspelled(lossy.shown.clone())),
        None => Ok(pins),
    }
}

/// Where the box shows the store at `place` through each of `mounts` that
/// holds it, generated pins included, each once. Refused where an overlay
/// shows it; where the last of `mounts` over a place it would be hidden at,
/// a pin's or a bind's alike, shows another file there, for the hide would
/// land on that one (ruling 13); and where a place it would be hidden at is
/// not valid UTF-8 (ruling 12).
pub(in crate::hands) fn placed(place: &Path, mounts: &[Mounted]) -> Result<Vec<String>, Unhidden> {
    let mut targets: Vec<PathBuf> = Vec::new();
    for mount in mounts {
        let Ok(rest) = place.strip_prefix(&mount.host) else {
            continue;
        };
        let shown = mount.shown.join(rest);
        if mount.held == Held::Overlay {
            return Err(Unhidden::Overlaid(place.to_path_buf(), shown));
        }
        let mut over = mounts.iter().rev();
        let last =
            over.find_map(|over| Some(over.host.join(shown.strip_prefix(&over.shown).ok()?)));
        if last.as_deref() != Some(place) {
            return Err(Unhidden::Shadowed(place.to_path_buf(), shown));
        }
        targets.extend((!targets.contains(&shown)).then_some(shown));
    }
    if let Some(lossy) = targets.iter().find(|target| target.to_str().is_none()) {
        return Err(Unhidden::Unspelled(lossy.clone()));
    }
    Ok(targets
        .iter()
        .map(|target| namespace_path(target))
        .collect())
}

/// Each folder strictly between a mount's root and the store `rest` below
/// it, as the way from that root, shallowest first: none where the mount
/// is the store itself.
fn folders(rest: &Path) -> Vec<&Path> {
    let above = rest.parent().into_iter().flat_map(Path::ancestors);
    let mut folders: Vec<&Path> = above.filter(|at| !at.as_os_str().is_empty()).collect();
    folders.reverse();
    folders
}
