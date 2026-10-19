//! A site's command put in its box (decision 0043), moved out of
//! `engine.rs` by decision 0065 slice two, U8a (operator ruling of
//! 2026-10-09), beside the artifact root the engine makes before launch and
//! hands the box as a typed fact (CR2; operator ruling of 2026-10-17).

use std::os::unix::fs::DirBuilderExt;
use std::path::{Path, PathBuf};

use brokkr_protocol::broker::Refusal;
use brokkr_protocol::hands::{HandsSpec, Sealed, UNPROTECTED_EVIDENCE};
use brokkr_protocol::native_controls::Transport;
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
