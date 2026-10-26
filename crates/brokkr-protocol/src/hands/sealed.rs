//! The reach a workspace box is built from, sealed once (decision 0065
//! slice two, U8a2c; CR2, MB3): the worktree, its Git directory and Git
//! common directory as Git names them, and each bind's source, each by its
//! canonical path and by device and inode, beside the HOME a bind's `~/`
//! names. Admission seals it in a writer's reservation ([`seal`]) and the
//! engine carries it to hands as a typed fact, as it carries the artifact
//! root (U8a); a probe that admits nothing seals its own as it starts.
//!
//! Every box of that writer is built from it: each bind from its sealed
//! canonical source, at the path its authored spelling names under the
//! sealed HOME, and the Git directories from the sealed reach, Git never
//! asked again where they are. Before each box starts, and again once its
//! argv is final, the reach is observed again exactly as it was sealed
//! ([`drifted`]): a source, an ancestor of one or the worktree's Git pointer
//! that no longer resolves to its sealed identity refuses with MB3's
//! identity cause before anything spawns, so nothing a writer changes inside
//! its reach after admission changes what a later box of its own mounts. A
//! reach that seals no worktree, or not each bind the box declares, refuses
//! the same way: nothing derives it again. At that second observation each
//! source is opened once and its handle held to its sealed identity, and
//! the box mounts it from that handle ([`Sealed::opened`], U8k), so
//! bubblewrap never resolves its path again.
//!
//! The reach seals the operator's secrets store by its canonical path, each
//! symbolic link on the way the engine spells it at resolved once for the
//! run, which the engine keeps ([`store_place`]), beside that configured
//! spelling, or none where
//! nothing serves one (U8h): every box built from it refuses a spelling
//! that no longer leads there (ruling 17), pins the store's folders and
//! hides the store behind `/dev/null` wherever it mounts it ([`hidden`]). A
//! reach sealed before the store was sealed in it does not read.

use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::evidence::Protected;
use super::namespace::{namespace_path, Mount, Namespace};
use super::overlay::overlay_argv;
use super::{expand_home, git_dirs, BindMode, GitFacts, HandsSpec, OverlayWrites};
use crate::broker::Refusal;

/// The secrets store each box hides, observed twice (U8h).
mod hidden;
pub(super) use hidden::Hidden;
use hidden::Store;
#[cfg(all(test, target_os = "linux"))]
pub(super) use hidden::{placed, Held, Mounted, Unhidden, TABLE};
pub use hidden::{store_place, UNHIDDEN_STORE};
/// Each source opened at the box's last observation, to mount (U8k).
mod opened;
use opened::Expected;
#[cfg(test)]
pub(super) use opened::OPENED;
pub(super) use opened::{beneath, Beneath};

/// The driver-input key the box's sealed reach rides under, beside the
/// artifact root and written with it (U8a2c): the final check binds the
/// box's hands to it as the engine sealed it, never derived.
pub const SEALED_REACH: &str = "sealed_reach";

/// What one writer's boxes are built from, as it was sealed: the HOME its
/// binds' `~/` named, the worktree, its two Git directories where it is a
/// repository, each bind's source in the order the hands declare them, and
/// the secrets store its boxes hide, by its canonical place and the
/// absolute path the engine spells it at, or none. The store must be
/// written, `null` where none is sealed: a reach without it does not read.
/// The default seals nothing, and no box is built from it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sealed {
    home: PathBuf,
    worktree: Source,
    git: Option<Git>,
    binds: Vec<Source>,
    #[serde(deserialize_with = "Option::deserialize")]
    store: Option<Store>,
}

/// The worktree's Git directory and Git common directory, as sealed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Git {
    dir: Source,
    common: Source,
}

/// One path a box is built from: canonical where it resolves, as spelled
/// where it does not, and its device and inode where it stands.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Source {
    path: PathBuf,
    identity: Option<(u64, u64)>,
}

/// Why a box is not built from its sealed reach: MB3's identity cause.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(super) enum Drifted {
    #[error(
        "{}: the box's reach was never sealed for the hands it declares",
        Refusal::Identity
    )]
    Unsealed,
    #[error(
        "{}: '{}', which the box is built from, no longer resolves as it was sealed",
        Refusal::Identity,
        .0.display()
    )]
    Moved(PathBuf),
    #[error(
        "{}: '{}', which the box is built from, cannot be opened to be mounted: {}",
        Refusal::Identity,
        .0.display(),
        .1
    )]
    Unopened(PathBuf, std::io::ErrorKind),
    #[error(
        "{}: '{}', which the box holds read-only, is not one regular file",
        Refusal::Identity,
        .0.display()
    )]
    Unheld(PathBuf),
}

#[cfg(test)]
thread_local! {
    /// A test's step once [`Sealed::confirm`] observed the reach as sealed:
    /// the window between that observation and the box's mounts, where a
    /// box built from what Git says now would differ from one built from
    /// the reach sealed.
    pub(super) static CONFIRMED: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
}

impl From<Drifted> for std::io::Error {
    fn from(drifted: Drifted) -> Self {
        std::io::Error::other(drifted)
    }
}

/// The reach a box of `spec` in `workdir` is built from, observed now: the
/// worktree, the Git directories `git` names, and each bind's source, its
/// `~/` the host `home`; and `store`, the secrets store its boxes hide, as
/// `(place, spelling)`: the canonical place the engine resolved once for
/// the run and keeps ([`store_place`]), beside the absolute spelling it was
/// configured at, sealed as handed and never resolved again here (U8h).
pub fn seal(
    spec: &HandsSpec,
    home: &Path,
    workdir: &Path,
    git: &GitFacts,
    store: Option<(&Path, &Path)>,
) -> Sealed {
    let dirs = git.git_dir.as_deref().zip(git.common_dir.as_deref());
    Sealed {
        home: home.to_path_buf(),
        worktree: source(workdir),
        git: dirs.map(|(dir, common)| Git {
            dir: source(dir),
            common: source(common),
        }),
        binds: (spec.binds.iter())
            .map(|bind| source(&expand_home(&bind.path, home)))
            .collect(),
        store: store.map(Store::sealed),
    }
}

/// The first path `sealed` was built from that, observed again now as it
/// was sealed (`spec`'s binds under its HOME, the Git directories Git names
/// in `workdir`), no longer resolves as sealed: the one sealed in its
/// place, or, where only the observation names a path there, that one;
/// none where every one resolves as sealed.
pub fn drifted(sealed: &Sealed, spec: &HandsSpec, workdir: &Path) -> Option<PathBuf> {
    let (git_dir, common_dir) = git_dirs(workdir);
    let git = GitFacts {
        git_dir,
        common_dir,
        identity: Vec::new(),
    };
    let now = seal(spec, &sealed.home, workdir, &git, None);
    let (was, now) = (sealed.sources(), now.sources());
    let at = (0..was.len().max(now.len())).find(|&at| was.get(at) != now.get(at))?;
    let moved = was.get(at).or(now.get(at));
    moved.map(|source| source.path.clone())
}

/// The canonical place of the secrets store `sealed` seals, where it seals
/// one (U8h): where the engine makes the store and names it to a seat, or
/// refuses the launch where its configured spelling now leads elsewhere
/// (ruling 17).
pub fn sealed_store(sealed: &Sealed) -> Option<&Path> {
    sealed.store()
}

impl Sealed {
    /// Refuse a box of `spec` in `workdir` this reach does not seal, or
    /// whose reach no longer resolves as sealed ([`drifted`]).
    pub(super) fn confirm(&self, spec: &HandsSpec, workdir: &Path) -> Result<(), Drifted> {
        let covered = self.worktree.identity.is_some() & (self.binds.len() == spec.binds.len());
        covered.then_some(()).ok_or(Drifted::Unsealed)?;
        if let Some(moved) = drifted(self, spec, workdir) {
            return Err(Drifted::Moved(moved));
        }
        #[cfg(test)]
        if let Some(step) = CONFIRMED.with_borrow_mut(Option::take) {
            step();
        }
        Ok(())
    }

    /// The HOME the binds' `~/` named.
    pub(super) fn home(&self) -> &Path {
        &self.home
    }

    /// The worktree's canonical path.
    pub(super) fn worktree(&self) -> &Path {
        &self.worktree.path
    }

    /// The Git common directory's canonical path, where there is one.
    pub(super) fn common(&self) -> Option<&Path> {
        self.git.as_ref().map(|git| git.common.path.as_path())
    }

    /// What the box holds of the Git common directory it retains (decision
    /// 0043 ruling 6, U8k): its `config`, opened beneath that directory's
    /// own handle and one regular file; and, where the worktree holds the
    /// directory, the directory beneath the worktree's handle, the very one
    /// sealed, so the hooks and config the box hides there are its own.
    pub(super) fn retained(&self) -> Vec<Beneath> {
        let Some(Git { common, .. }) = &self.git else {
            return Vec::new();
        };
        let config = Beneath {
            host: common.path.join("config"),
            within: common.path.clone(),
            rest: PathBuf::from("config"),
            expected: Expected::File,
        };
        let inside = common.path.strip_prefix(&self.worktree.path).ok();
        let inside = inside.map(|rest| Beneath {
            host: common.path.clone(),
            within: self.worktree.path.clone(),
            rest: rest.to_path_buf(),
            expected: Expected::Seen(common.identity),
        });
        std::iter::once(config).chain(inside).collect()
    }

    /// The canonical place of the secrets store its boxes hide, where one
    /// is sealed (U8h).
    fn store(&self) -> Option<&Path> {
        self.store.as_ref().map(Store::place)
    }

    /// This reach with `host` sealed now beside its binds: the engine's own
    /// executable an exec box binds read-only, which is no writer's reach.
    pub(super) fn beside(&self, host: &Path) -> Sealed {
        let mut sealed = self.clone();
        sealed.binds.push(source(host));
        sealed
    }

    /// Every path sealed, in order: the worktree, the Git directory and
    /// Git common directory, and each bind's source.
    fn sources(&self) -> Vec<&Source> {
        let git = self.git.iter().flat_map(|git| [&git.dir, &git.common]);
        (std::iter::once(&self.worktree))
            .chain(git)
            .chain(&self.binds)
            .collect()
    }
}

/// `path` as a box's sealed source: canonical where it resolves, and its
/// device and inode where it stands.
fn source(path: &Path) -> Source {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let found = std::fs::metadata(&path).ok();
    let identity = found.map(|found| (found.dev(), found.ino()));
    Source { path, identity }
}

/// Mount each of `spec`'s binds into `namespace` from the source `sealed`
/// holds for it, at the path its authored spelling names under the sealed
/// HOME: read-only, read-write or as an overlay whose upper layer lives
/// where `writes` says, every writable one excluded from the evidence
/// `protected` keeps (U8a) by that spelling, which [`Sealed::confirm`] has
/// just held to its sealed source, and each masked file the source holds
/// hidden behind `/dev/null`. A mask over a file that is not there would
/// make bwrap create it on the host to mount over, so only what exists is
/// hidden. The sealed secrets store is observed first, for a box seen at
/// `workdir` and handed `bundle`, with every mount the box makes
/// ([`Hidden::observe`]); each folder above it or above the artifact root
/// in the worktree is pinned from that one description, outermost first,
/// before any bind ([`Hidden::pin_worktree`]), and each above it in
/// a writable bind as that bind is mounted, before anything below it
/// ([`Hidden::pin`]). Last, after every writable mount, the store is hidden
/// wherever the box shows it ([`Hidden::hide`]); what was observed is
/// returned, to be observed again once the argv is final
/// ([`Hidden::confirm`]).
pub(super) fn mount_binds(
    namespace: &mut Namespace,
    protected: &mut Protected,
    spec: &HandsSpec,
    sealed: &Sealed,
    seen: (&Path, Option<&Path>),
    writes: OverlayWrites<'_>,
) -> std::io::Result<Hidden> {
    let root = (protected.held(), protected.ancestors());
    let store = Hidden::observe(spec, sealed, seen, root)?;
    store.pin_worktree(namespace);
    for (index, (bind, source)) in spec.binds.iter().zip(&sealed.binds).enumerate() {
        let (host, from) = (expand_home(&bind.path, &sealed.home), &source.path);
        if bind.mode != BindMode::Ro {
            protected.exclude(&host)?;
        }
        let target = namespace_path(&host);
        match bind.mode {
            BindMode::Ro => namespace.mount(Mount::RoBindTry, from, &target),
            BindMode::Rw => namespace.mount(Mount::BindTry, from, &target),
            BindMode::Overlay => {
                let argv = overlay_argv(from, &host, index, writes)?;
                namespace.overlay(from, &target, argv);
            }
        }
        store.pin(namespace, index);
        for name in bind.mask.iter().filter(|name| from.join(name).exists()) {
            let target = namespace_path(&host.join(name));
            namespace.mount(Mount::RoBind, Path::new("/dev/null"), &target);
        }
    }
    store.hide(namespace);
    Ok(store)
}
