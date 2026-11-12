//! One host for the CLI's evidence tests (0071 ruling 5): a HOME whose
//! coordination root stands, one worktree beside it, and the historical
//! record that names the worktree's artifact root. `verbs/evidence`'s tests
//! and doctor's evidence tests read the same fixture, so the record is
//! spelled once.

use std::fs;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};

use brokkr_core::canonical;

/// A HOME of this test's own, its coordination root, and one worktree.
pub(crate) struct Host {
    pub(crate) dir: tempfile::TempDir,
    pub(crate) home: PathBuf,
    pub(crate) work: PathBuf,
}

impl Host {
    /// The host, its coordination root and the worktree, owner-only, with
    /// the admission lock that makes its inventory readable.
    pub(crate) fn new() -> Host {
        let dir = tempfile::tempdir().unwrap();
        let home = dir.path().join("home");
        let work = dir.path().join("work");
        fs::create_dir(&home).unwrap();
        fs::create_dir(&work).unwrap();
        let root = capabilities(&home);
        let mut made = fs::DirBuilder::new();
        made.recursive(true).mode(0o700).create(&root).unwrap();
        sealed(&root.join("admission.lock"), "");
        Host { dir, home, work }
    }

    /// The worktree's artifact root.
    pub(crate) fn artifacts(&self) -> PathBuf {
        artifacts(&self.work)
    }

    /// The historical record of the host's worktree, and its retirement
    /// when one is named.
    pub(crate) fn record(&self, retirement: Option<&str>) {
        record(&self.home, &self.work, retirement);
    }

    /// The worktree's directory under the coordination root and the key its
    /// historical record is named by.
    pub(crate) fn at(&self) -> (PathBuf, String) {
        (repo(&self.home, &self.work), key(&artifacts(&self.work)))
    }
}

/// The coordination root under `home`.
fn capabilities(home: &Path) -> PathBuf {
    home.join(".local/state/brokkr/capabilities")
}

/// The artifact root of `work`.
pub(crate) fn artifacts(work: &Path) -> PathBuf {
    work.join(".forge/artifacts")
}

/// The worktree's directory under the coordination root `home`.
fn repo(home: &Path, work: &Path) -> PathBuf {
    capabilities(home).join(canonical::sha256_bytes(work.as_os_str().as_bytes()))
}

/// The key the historical record of `artifacts` is named by.
pub(crate) fn key(artifacts: &Path) -> String {
    canonical::sha256_bytes(artifacts.as_os_str().as_bytes())
}

/// The historical record of `work` in `home`, and its retirement when one
/// is named. The artifact root is not made.
pub(crate) fn record(home: &Path, work: &Path, retirement: Option<&str>) {
    let artifacts = artifacts(work);
    let key = key(&artifacts);
    let at = repo(home, work);
    let mut made = fs::DirBuilder::new();
    made.recursive(true).mode(0o700).create(&at).unwrap();
    sealed(
        &at.join(format!("historical-{key}.json")),
        &serde_json::json!({"artifacts": artifacts}).to_string(),
    );
    if let Some(retirement) = retirement {
        sealed(&at.join(format!("retired-{key}.json")), retirement);
    }
}

/// `text` written as an owner-only record, as a seal writes one.
pub(crate) fn sealed(at: &Path, text: &str) {
    let mut options = fs::OpenOptions::new();
    let opened = options.write(true).create_new(true).mode(0o600);
    std::io::Write::write_all(&mut opened.open(at).unwrap(), text.as_bytes()).unwrap();
}
