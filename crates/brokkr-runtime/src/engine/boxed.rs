//! A site's command put in its box (decision 0043), moved out of
//! `engine.rs` by decision 0065 slice two, U8a (operator ruling of
//! 2026-10-09), beside the artifact root the engine makes before launch and
//! hands the box as a typed fact (CR2; operator ruling of 2026-10-17).

use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use brokkr_protocol::broker::Refusal;
use brokkr_protocol::hands::{
    sealed_store, store_place, HandsSpec, Sealed, UNHIDDEN_STORE, UNPROTECTED_EVIDENCE,
};
use brokkr_protocol::native_controls::{Segment, Transport};
use thiserror::Error;

use crate::argv::Executable;

/// The artifact root's name in the run-local directory (decision 0083):
/// retained responses are published at `artifacts/sha256/<hex>` there.
const ARTIFACTS: &str = "artifacts";

/// The artifact root of the run-local directory `run_local`, absolute so
/// the box and the final check read the path the engine made.
pub(super) fn artifact_root(run_local: &Path) -> PathBuf {
    let root = run_local.join(ARTIFACTS);
    std::path::absolute(&root).unwrap_or(root)
}

impl super::Engine {
    /// The hands a single seat's box is built from, and every seat's reach
    /// is reserved by: its site's own, and under `ship` the journal's
    /// directory read-only where the journal lies outside the worktree.
    /// Moved here from `engine.rs`, frozen at its baseline, by U8a2c.
    pub(super) fn runtime_hands(&self, seat_name: &str) -> Option<HandsSpec> {
        let mut spec = self.hands_for(seat_name)?;
        let phase = seat_name
            .split_once(':')
            .map_or(seat_name, |(phase, _)| phase);
        if phase == "ship" {
            let workdir = std::fs::canonicalize(self.workdir()).unwrap_or_else(|_| self.workdir());
            let journal =
                std::fs::canonicalize(self.store.path()).unwrap_or(self.store.path().to_path_buf());
            if !journal.starts_with(&workdir) {
                let parent = journal.parent().unwrap_or(&journal);
                spec.binds.push(brokkr_protocol::hands::Bind {
                    path: parent.to_string_lossy().into_owned(),
                    mode: brokkr_protocol::hands::BindMode::Ro,
                    mask: Vec::new(),
                });
            }
        }
        Some(spec)
    }

    /// The secrets store as the operator configured it, the CLI override or
    /// `secrets.env` in the run-local directory, as spelled: the spelling
    /// a reach seals beside the place it leads to (U8h, ruling 17).
    pub(super) fn secrets_store_spelling(&self) -> PathBuf {
        (self.secrets_file.clone()).unwrap_or_else(|| self.run_local().join(crate::places::SECRETS))
    }

    /// The secrets store as this run keeps it (U8h, ruling 18): its
    /// configured spelling resolved to its canonical place the first time
    /// anything asks, and never again for the run, so making the store, the
    /// path a seat's input names and the place a box's reach seals are one.
    pub(super) fn kept_store(&self) -> &KeptStore {
        self.store_place
            .get_or_init(|| KeptStore::resolved(self.secrets_store_spelling()))
    }

    /// The operator-side store path threaded to drivers and sealed in a
    /// box's reach: the place this run keeps ([`Self::kept_store`]). Where
    /// the spelling could not be resolved, the spelling stands in a seat's
    /// input, and every spawn handed that input refuses as [`Self::prepared`]
    /// finds the kept place unresolved, before any driver or box reads it.
    pub(super) fn secrets_store_path(&self) -> PathBuf {
        let kept = self.kept_store();
        (kept.place.clone()).unwrap_or_else(|_| kept.spelling.clone())
    }

    /// The kept place, where the configured spelling still leads to it now
    /// (ruling 18): refused where it was never resolved, where the spelling
    /// no longer resolves, or where it leads elsewhere.
    fn led<'k>(&self, kept: &'k KeptStore) -> Result<&'k Path, Unprepared> {
        let unresolved =
            |spelling: &Path, kind| Unprepared::Unresolved(spelling.to_path_buf(), kind);
        let place = (kept.place.as_ref()).map_err(|kind| unresolved(&kept.spelling, *kind))?;
        let spelling = self.secrets_store_spelling();
        match store_place(&spelling) {
            Ok(now) if now == *place => Ok(place),
            Ok(_) => Err(Unprepared::Strayed(spelling, place.clone())),
            Err(error) => Err(unresolved(&spelling, error.kind())),
        }
    }

    /// U8a, U8h: what a box of `hands` is handed made before its launch,
    /// the run's artifact root `artifacts` ([`make_root`]) and then the
    /// secrets store at the place this run keeps, where its configured
    /// spelling still leads ([`Self::led`]; [`make_store`]); either refused,
    /// the spawn is, by the refusal this returns. Where admission sealed the
    /// store at another place, the spawn refuses before anything is made
    /// there (ruling 17). An unboxed spawn, `hands` none, makes nothing, but
    /// once a seat's input has named the kept place it refuses as a box
    /// would where the spelling no longer leads there (ruling 18).
    pub(super) fn prepared(
        &self,
        artifacts: &Path,
        hands: Option<&HandsSpec>,
    ) -> Option<Unprepared> {
        let made = || -> Result<(), Unprepared> {
            let Some(spec) = hands else {
                return self
                    .store_place
                    .get()
                    .map_or(Ok(()), |kept| self.led(kept).map(drop));
            };
            make_root(artifacts)?;
            let store = self.led(self.kept_store())?;
            let sealed = self.reach.sealed(spec);
            if let Some(place) = sealed.and_then(sealed_store).filter(|at| at != &store) {
                let spelling = self.secrets_store_spelling();
                return Err(Unprepared::Strayed(spelling, place.to_path_buf()));
            }
            Ok(make_store(store)?)
        };
        made().err()
    }
}

/// The secrets store one run keeps (U8h, ruling 18): the spelling it was
/// configured at, absolute, and the canonical place that spelling resolved
/// to ([`store_place`]), or why it could not be resolved, which refuses
/// every spawn that would use it, never a place standing in for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct KeptStore {
    pub(super) spelling: PathBuf,
    pub(super) place: Result<PathBuf, std::io::ErrorKind>,
}

impl KeptStore {
    /// The store configured at `spelling`, resolved now.
    fn resolved(spelling: PathBuf) -> KeptStore {
        let spelling = std::path::absolute(&spelling).unwrap_or(spelling);
        let place = store_place(&spelling).map_err(|error| error.kind());
        KeptStore { spelling, place }
    }

    /// The store as a box's reach seals it, `(place, spelling)`. Unresolved,
    /// the spelling is sealed as both; no box is built from that reach, for
    /// every boxed spawn refuses first ([`super::Engine::prepared`]), and a
    /// box handed it refuses it unless the spelling now resolves to itself.
    pub(super) fn sealed(&self) -> (&Path, &Path) {
        let place = (self.place.as_ref()).map_or(self.spelling.as_path(), PathBuf::as_path);
        (place, &self.spelling)
    }
}

/// Why a box is not launched: what it is handed cannot be made.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Unprepared {
    #[error(transparent)]
    Root(#[from] Unrooted),
    #[error(transparent)]
    Store(#[from] Unstored),
    /// The configured store no longer leads to the place admission sealed
    /// in the box's reach (ruling 17): the configured spelling, the place.
    #[error(
        "dispatch refused: {UNHIDDEN_STORE}: the configured store '{}' no longer leads to '{}', \
         where the box's reach sealed it", .0.display(), .1.display()
    )]
    Strayed(PathBuf, PathBuf),
    /// The configured store's way cannot be walked to a canonical place
    /// (ruling 18): the configured spelling, and why.
    #[error(
        "dispatch refused: {UNHIDDEN_STORE}: the configured store '{}' cannot be resolved to its \
         place: {}", .0.display(), .1
    )]
    Unresolved(PathBuf, std::io::ErrorKind),
}

/// Why the engine cannot hand a box its artifact root, in the operator's
/// words its spawn refuses with: CR2's cause, never a box without it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Unrooted {
    #[error(
        "dispatch refused: {UNPROTECTED_EVIDENCE}: the artifact root '{}' cannot be made or \
         observed: {}", .0.display(), .1
    )]
    Unmade(PathBuf, std::io::ErrorKind),
    #[error(
        "dispatch refused: {UNPROTECTED_EVIDENCE}: the artifact root '{}' is not a directory \
         of its own", .0.display()
    )]
    Foreign(PathBuf),
}

/// Make the artifact root `root` where it is missing, owner-only with each
/// directory above it this makes, and observe it: a link or another kind
/// of file there refuses, and an existing directory is kept as it is.
pub(super) fn make_root(root: &Path) -> Result<(), Unrooted> {
    let unmade = |error: std::io::Error| Unrooted::Unmade(root.to_path_buf(), error.kind());
    let made = std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(root);
    made.map_err(unmade)?;
    let found = std::fs::symlink_metadata(root).map_err(unmade)?;
    if !found.file_type().is_dir() {
        return Err(Unrooted::Foreign(root.to_path_buf()));
    }
    Ok(())
}

/// Why the engine cannot hand a box its secrets store to hide (U8h): the
/// cause every box's own refusal carries, never a box without it.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("dispatch refused: {UNHIDDEN_STORE}: the store '{}' cannot be made: {}", .0.display(), .1)]
pub(super) struct Unstored(pub(super) PathBuf, pub(super) std::io::ErrorKind);

/// Make the secrets store at its kept place `store` where it is missing,
/// one empty file only its owner may read or write (operator ruling 2 of
/// 2026-10-23), in the folder that holds it: a folder that is missing is
/// never made, and refuses (ruling 18). An existing store is kept as it is,
/// for the box judges it. Nothing it holds is read.
pub(super) fn make_store(store: &Path) -> Result<(), Unstored> {
    let unmade = |error: std::io::Error| Unstored(store.to_path_buf(), error.kind());
    let mut new = std::fs::OpenOptions::new();
    let created = new.write(true).create_new(true).mode(0o600).open(store);
    match created {
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        created => created.map(drop).map_err(unmade),
    }
}

/// The tail every [`Unnamed`] refusal ends with: why a path a box is
/// handed must be named exactly.
const LOSSY: &str = "is not UTF-8, so no argument names it exactly and a lossy path would \
                     name another (rebuild unit 13-fix-c, R2; rebuild unit 14a2; U8a, H1)";

/// A path a box would be handed that no argument names exactly, refused
/// before any argv is built, exec and model alike (U8a, H1): a lossy
/// spelling names another directory, which the box would bind or protect
/// in its place. The artifact root's refusal carries CR2's cause.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(crate) enum Unnamed {
    #[error("dispatch refused: the engine's executable '{}' {LOSSY}", .0.display())]
    Executable(PathBuf),
    #[error("dispatch refused: the site's workdir '{}' {LOSSY}", .0.display())]
    Workdir(PathBuf),
    #[error(
        "dispatch refused: {UNPROTECTED_EVIDENCE}: the artifact root '{}' {LOSSY}",
        .0.display()
    )]
    Artifacts(PathBuf),
    #[error("dispatch refused: the bundle root '{}' {LOSSY}", .0.display())]
    Bundle(PathBuf),
    /// No reach was sealed for the box's hands at admission, or one that no
    /// argument names exactly: MB3's identity cause, never a box built from
    /// a reach derived again (U8a2c).
    #[error(
        "dispatch refused: {}: the box's reach was not sealed at admission, or is not UTF-8, so \
         no box of these hands is built from it (U8a2c)",
        Refusal::Identity
    )]
    Unsealed,
    /// `Transport::expand` names no other refusal for the box's hands
    /// alone; kept so one it adds later refuses rather than spawns.
    #[error("dispatch refused: the box's hands cannot be expanded into the seat's command")]
    Unexpanded,
}

/// `path` as the UTF-8 it must be, or `unnamed`'s refusal of it.
fn named(path: &Path, unnamed: fn(PathBuf) -> Unnamed) -> Result<&str, Unnamed> {
    path.to_str().ok_or_else(|| unnamed(path.to_path_buf()))
}

/// Decision 0043: put a site's hands in the box. A model seat's argv has
/// the adapter's fragment already; its two tokens are expanded here,
/// where the workdir and `engine`, this binary's path, are known — the MCP
/// server a harness spawns is `brokkr hands serve` on that executable. An
/// `exec` dispatch is boxed whole instead: `brokkr hands exec` builds the
/// namespace at run time and passes the driver's stdio straight through.
/// Either is handed `artifacts`, the run's artifact root (U8a), and
/// `sealed`, the reach admission sealed for these hands, which every box
/// of them is built from (U8a2c); a box with none refuses. A site without
/// hands gets its command back untouched.
///
/// Rebuild unit 14a2: a model seat's tokens are expanded by
/// [`Transport::expand`], the one TOML-safe encoder the final check
/// expects them from, so the spawn and the check cannot disagree on an
/// escape. Before either argv is built, the executable, the workdir, the
/// artifact root, the sealed reach and an exec dispatch's bundle root must
/// each be UTF-8, or the spawn refuses by the path that is not
/// ([`Unnamed`]).
pub(crate) fn hands_command(
    command: Vec<String>,
    hands: Option<&HandsSpec>,
    workdir: &Path,
    (engine, artifacts, sealed): (&Executable, &Path, Option<&Sealed>),
    roots: &[PathBuf],
) -> Result<Vec<String>, Unnamed> {
    let Some(spec) = hands else {
        return Ok(command);
    };
    let brokkr = engine.or_empty();
    let exe = named(brokkr, Unnamed::Executable)?;
    let work = named(workdir, Unnamed::Workdir)?;
    let evidence = named(artifacts, Unnamed::Artifacts)?;
    let sealed = sealed.ok_or(Unnamed::Unsealed)?;
    let reach = serde_json::to_string(sealed).or(Err(Unnamed::Unsealed))?;
    if super::is_exec_dispatch(&command) {
        let bundle_root = roots
            .iter()
            .find(|root| {
                command
                    .iter()
                    .any(|part| Path::new(part).strip_prefix(root).is_ok())
            })
            .or_else(|| roots.first());
        let bundle = bundle_root.map(|bundle| named(bundle, Unnamed::Bundle));
        let bundle = bundle.transpose()?;
        let command = command
            .into_iter()
            .map(|part| {
                bundle_root
                    .and_then(|root| Path::new(&part).strip_prefix(root).ok())
                    .map(|relative| {
                        brokkr_protocol::hands::namespace_join(
                            brokkr_protocol::hands::SANDBOX_BUNDLE,
                            relative,
                        )
                    })
                    .unwrap_or(part)
            })
            .collect::<Vec<_>>();
        let spec = spec.to_value().to_string();
        let verb = [
            "hands",
            "exec",
            "--workdir",
            work,
            "--artifacts",
            evidence,
            "--reach",
            &reach,
        ];
        let mut boxed: Vec<String> = std::iter::once(exe)
            .chain(verb)
            .chain(["--spec", &spec])
            .chain(
                bundle
                    .into_iter()
                    .flat_map(|bundle| ["--bundle-root", bundle]),
            )
            .map(String::from)
            .collect();
        boxed.push("--".to_string());
        boxed.extend(command);
        return Ok(boxed);
    }
    Transport {
        brokkr,
        workdir,
        artifacts,
        sealed,
        spec,
    }
    .expand(&command)
    .ok_or(Unnamed::Unexpanded)
}

/// A site's spawn under `namespace`, moved here from `compose_segments`,
/// at its line baseline, by #832: `command` put in the box by
/// [`hands_command`], each of `segments` mapped onto its own tokens, or
/// left unmapped and refused by the path that is not named. An exec
/// dispatch is re-walked at spawn as an unboxed one is (decision 0048;
/// operator ruling "accept all", 2026-10-22): the walk compares the host's
/// files with the compiled map, so the directory is read from the host
/// argv, before the box maps it.
pub(super) fn boxed_spawn(
    site: &super::Composing<'_>,
    spec: &HandsSpec,
    command: Vec<String>,
    segments: Vec<Segment>,
) -> super::SiteSpawn {
    let (workdir, boxed, roots) = (site.workdir, site.boxed, site.roots);
    let rewalk = super::script_directory(&command, roots);
    let (parts, refusal) = match hands_command(command, Some(spec), workdir, boxed, roots) {
        Ok(mapped) => (super::behind(mapped, &segments), None),
        Err(reason) => (segments, Some(reason.to_string())),
    };
    super::SiteSpawn {
        rewalk,
        refusal,
        ..super::SiteSpawn::of(parts)
    }
}
