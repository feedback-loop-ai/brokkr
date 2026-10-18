//! Historical artifact roots (decision 0065 slice two, U8a2a; D6, CR2):
//! an artifact root that once protected evidence stays protected after
//! every attempt that wrote it has settled, from every later writer of any
//! worktree on this host.
//!
//! The first admission whose artifact root protects evidence records it,
//! under the admission lock, in its worktree's directory of the protected
//! root, sealed as reservations are. No settlement removes the record, and
//! every later admission reads every one of them. Where admission relies
//! on a root's evidence, every artifact it consults there,
//! `sha256/<hex>`, is one regular file of one link whose bytes digest to
//! its name, within D5's entry bound and the startup deadline, read and
//! hashed in bounded chunks, the deadline checked once each chunk is
//! hashed (R2 of U8a2a's fourth run, fixed by U8a2c), and no larger than
//! MB3's bound per response and canonical artifact
//! ([`ARTIFACT_BYTES_MAX`]); anything else refuses with CR2's cause.

use std::io::Read;
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::path::{Path, PathBuf};

use brokkr_core::canonical;
use rustix::fs::{fstat, openat, FileType, Mode, OFlags, CWD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::admission::{Claim, Conflict, Unadmitted, Unknown};
use super::inventory::{hex, Budget};
use super::root::{Commit, Root};

/// MB3's bound of 8 MiB per response and canonical artifact, the largest
/// any artifact retention publishes may be (M1 of run four's second
/// review); its 1 MiB bound is the request's.
const ARTIFACT_BYTES_MAX: u64 = 8 << 20;

/// What a historical root's record begins with, in its worktree's
/// directory.
const PREFIX: &str = "historical-";

/// What a historical root's record ends with.
const SUFFIX: &str = ".json";

/// The directory of an artifact root its artifacts stand in, each named by
/// its bytes' sha256.
const DIGESTS: &str = "sha256";

/// The most of an artifact read and hashed before the deadline is checked
/// again (M2).
const CHUNK: usize = 64 << 10;

/// What each chunk is handed once it is hashed, before the deadline is
/// checked again: the digest of the bytes hashed so far. Nothing in
/// production; a test's own, to spend the deadline there or read what was
/// hashed.
pub(super) type Chunked<'c> = &'c dyn Fn(&Sha256);

/// One historical root's record: the artifact root, as it resolved when
/// it first protected evidence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Historical {
    pub(super) artifacts: PathBuf,
}

/// Why an artifact is not admitted as what its name says.
enum Undigested {
    /// It is not one regular file of one link holding those bytes.
    Corrupt,
    /// The deadline passed while it was read.
    Expired,
}

/// The record's name for the artifact root `artifacts`: its path's sha256.
pub(super) fn name(artifacts: &Path) -> String {
    let key = canonical::sha256_bytes(artifacts.as_os_str().as_bytes());
    format!("{PREFIX}{key}{SUFFIX}")
}

/// The key a historical record's name `file` carries, where it is one.
pub(super) fn named(file: &str) -> Option<&str> {
    let key = file.strip_prefix(PREFIX)?.strip_suffix(SUFFIX)?;
    hex(key).then_some(key)
}

/// `claim`'s artifact root recorded as historical in its worktree's
/// directory below `root` ([`Root::seal`]), within `budget`'s deadline.
pub(super) fn recorded(root: &Root, claim: &Claim, budget: &Budget) -> Result<Commit, Unknown> {
    let repo = root.below(&[&claim.repo])?;
    let record = Historical {
        artifacts: claim.artifacts.clone(),
    };
    let bytes = serde_json::to_vec(&record).or(Err(Unknown::Unsealed))?;
    root.seal(&repo, &name(&claim.artifacts), &bytes, budget)
}

/// Whether every artifact the root `root` holds is what its name says,
/// each read within `budget`: none where it holds no digests, refused
/// with CR2's cause by the first that is not, or that cannot be read, and
/// with MB3's identity cause where the deadline passes first.
pub(super) fn verified(root: &Path, budget: &mut Budget) -> Result<(), Unadmitted> {
    verified_with(root, budget, &|_| {})
}

/// [`verified`], `chunked` handed each chunk read: the house `_with` seam
/// a test spends the deadline in, inside an artifact's read; nothing in
/// production.
pub(super) fn verified_with(
    root: &Path,
    budget: &mut Budget,
    chunked: Chunked<'_>,
) -> Result<(), Unadmitted> {
    let digests = root.join(DIGESTS);
    let corrupt = |path: &Path| Conflict::Corrupt(path.to_path_buf());
    let found = match std::fs::symlink_metadata(&digests) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        found => found.or(Err(corrupt(&digests)))?,
    };
    found.is_dir().then_some(()).ok_or(corrupt(&digests))?;
    for entry in std::fs::read_dir(&digests).or(Err(corrupt(&digests)))? {
        budget.spend()?;
        let path = entry.or(Err(corrupt(&digests)))?.path();
        match digested(&path, budget, chunked) {
            Ok(()) => {}
            Err(Undigested::Corrupt) => return Err(corrupt(&path).into()),
            Err(Undigested::Expired) => return Err(Unknown::Expired.into()),
        }
    }
    Ok(())
}

/// The file at `path`, opened without following a link and without
/// blocking, where it is a regular file of one link: its type checked by
/// `fstat` before anything reads it, so a FIFO or a socket in its place is
/// refused at once rather than hanging a reader under the admission lock
/// past every deadline (R3; L5).
pub(super) fn regular(path: &Path) -> Option<OwnedFd> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = openat(CWD, path, flags, Mode::empty()).ok()?;
    let stat = fstat(&file).ok()?;
    let regular = FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile;
    (regular & (stat.st_nlink == 1)).then_some(file)
}

/// Whether the artifact at `path` is one regular file of one link
/// ([`regular`]) of at most [`ARTIFACT_BYTES_MAX`] whose bytes' sha256 is
/// its name: read and hashed a chunk at a time, the digest so far handed
/// to `chunked` and the deadline of `budget` checked once each is hashed
/// (M2; R2), never past one byte over the bound, nothing kept but the
/// digest.
fn digested(path: &Path, budget: &Budget, chunked: Chunked<'_>) -> Result<(), Undigested> {
    let name = path.file_name().and_then(|name| name.to_str());
    let name = name.filter(|name| hex(name)).ok_or(Undigested::Corrupt)?;
    let file = std::fs::File::from(regular(path).ok_or(Undigested::Corrupt)?);
    let mut file = file.take(ARTIFACT_BYTES_MAX + 1);
    let (mut hasher, mut hashed, mut chunk) = (Sha256::new(), 0_u64, vec![0; CHUNK]);
    loop {
        let read = file.read(&mut chunk).or(Err(Undigested::Corrupt))?;
        hasher.update(&chunk[..read]);
        hashed += read as u64;
        chunked(&hasher);
        budget.check().or(Err(Undigested::Expired))?;
        if read == 0 {
            break;
        }
    }
    let named = (hashed <= ARTIFACT_BYTES_MAX) & (format!("{:x}", hasher.finalize()) == name);
    named.then_some(()).ok_or(Undigested::Corrupt)
}
