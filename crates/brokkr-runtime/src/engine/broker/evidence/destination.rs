//! Where an archive is written (decision 0105, the controller's ruling on
//! round 2's S1 hold on #1020): a destination is resolved once, by the
//! filesystem, compared with the coordination root and the root being
//! retired, and only then opened and written, one plain name at a time
//! beneath the directory that was compared. A link spelled on the way is
//! followed by the resolution, so `link/../archive` is the directory the
//! kernel makes it, never the one a lexical reading guesses; the write
//! follows no link at all. The opened directory is the one that was
//! resolved, checked by `fstat`, and every sync, read-back and removal
//! below it goes through that descriptor. The resolved path is the one
//! recorded.

use std::ffi::{OsStr, OsString};
use std::io::{Read, Seek, SeekFrom, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Component, Path, PathBuf};

use brokkr_core::canonical;
use rustix::fs::{mkdirat, openat, unlinkat, AtFlags, Dir, Mode, OFlags, CWD};
use rustix::io::Errno;
use sha2::{Digest, Sha256};

use super::super::historical::{self, regular};
use super::{
    unadmitted, ArchiveArtifact, Budget, EvidenceArchive, EvidenceError, EVIDENCE_ARCHIVE_FORMAT,
};
use crate::engine::artifacts::{ARTIFACT_BYTES_MAX, DIGESTS};

/// The inventory's one name inside an archive directory: the last file an
/// archive publishes, and the mark that makes it complete.
const INVENTORY_FILE: &str = "inventory.json";

/// The most of an artifact read and written before the next chunk.
const CHUNK: usize = 64 << 10;

/// An archive directory resolved, compared with both roots, and opened:
/// every file below it is created through this descriptor, one plain name
/// at a time, following no link, and so is every name a refusal removes.
#[derive(Debug)]
pub(super) struct Destination {
    /// The absolute path the filesystem resolved, and the one recorded in
    /// the retirement.
    pub(super) at: PathBuf,
    dir: OwnedFd,
    /// The directories the resolution made, deepest last, each with the
    /// parent and plain name a refusal removes it through.
    made: Vec<Made>,
}

/// One directory [`descendants`] made: its parent descriptor and its own
/// plain name, so removing it follows no link either.
#[derive(Debug)]
struct Made {
    parent: OwnedFd,
    name: OsString,
}

/// Every name an archive wrote, and the digest directory it wrote the
/// copies in, so a refusal removes exactly those names through the
/// descriptors that wrote them. `inventory` is set only once `create` made
/// the inventory: a name another writer put there is never this verb's to
/// remove.
#[derive(Debug, Default)]
struct Written {
    shas: Option<OwnedFd>,
    names: Vec<String>,
    inventory: bool,
}

impl Destination {
    /// Resolve `to` once by the filesystem; refuse it at or below
    /// `coordination` or `retired` by name, before anything is written;
    /// and open it, making the components that do not stand one plain name
    /// at a time under the deepest one that does. An existing directory is
    /// taken only where it is empty.
    pub(super) fn open(
        to: &Path,
        coordination: &Path,
        retired: &Path,
    ) -> Result<Destination, EvidenceError> {
        let Resolved {
            at,
            base,
            rest,
            identity,
        } = resolve(to)?;
        // A retirement names the resolved path as text; a path that is not
        // UTF-8 could not be recorded, so it is refused before the copy
        // rather than after it (the controller's R9 ruling on #1020).
        if at.to_str().is_none() {
            return Err(EvidenceError::Unnamed(at));
        }
        if at.starts_with(resolved(retired)?) {
            return Err(EvidenceError::ArchiveInsideRoot(to.to_path_buf()));
        }
        if at.starts_with(resolved(coordination)?) {
            return Err(EvidenceError::ArchiveInsideCoordination(to.to_path_buf()));
        }
        let opened = directory(&base, identity)?;
        let (dir, made) = match rest.is_empty() {
            true => (opened, Vec::new()),
            false => descendants(opened, base, &rest)?,
        };
        let taken = empty(&dir);
        if matches!(taken, Ok(true)) {
            return Ok(Destination { at, dir, made });
        }
        // A directory that is not empty and one whose listing cannot be
        // read are refused without keeping a directory this verb made.
        remove_dirs(&made);
        taken.and_then(|_| Err(EvidenceError::DestinationNotEmpty(at)))
    }

    /// `root`'s evidence archived into this destination: every artifact
    /// verified, copied to `sha256/<hex>` and verified there, the
    /// inventory written and synced last, and the inventory's sha256
    /// answered. A refusal removes everything this verb wrote, including
    /// the directories the resolution made.
    pub(super) fn archive(
        &self,
        root: &Path,
        budget: &mut Budget,
        between: &dyn Fn(),
        now: &str,
    ) -> Result<String, EvidenceError> {
        let mut written = Written::default();
        match self.archived(root, budget, between, now, &mut written) {
            Ok(sha256) => Ok(sha256),
            Err(refused) => {
                self.removed(&written);
                Err(refused)
            }
        }
    }

    /// [`archive`] with every name it writes recorded in `written`, so a
    /// refusal removes them through the descriptors that wrote them.
    fn archived(
        &self,
        root: &Path,
        budget: &mut Budget,
        between: &dyn Fn(),
        now: &str,
        written: &mut Written,
    ) -> Result<String, EvidenceError> {
        historical::verified(root, budget).map_err(unadmitted)?;
        let found = super::digests(root)?;
        written.shas = Some(self.sha256_dir()?);
        between();
        for (name, _) in &found {
            let mut out = create(
                written
                    .shas
                    .as_ref()
                    .expect("the digest directory was just opened"),
                name,
            )?;
            // Only a name this verb created is one a refusal removes: a
            // raced-in name is never unlinked (0105, 1020b).
            written.names.push(name.clone());
            written_into(&root.join(DIGESTS).join(name), &mut out, name)?;
        }
        let shas = written
            .shas
            .as_ref()
            .expect("the digest directory was just opened");
        // The copies are durable before the inventory that names them, and
        // the inventory before the retirement that points at it (M2).
        super::sync_dir(shas)?;
        super::sync_dir(&self.dir)?;
        let total_bytes = found.iter().map(|(_, bytes)| *bytes).sum();
        let artifacts = found
            .into_iter()
            .map(|(sha256, bytes)| ArchiveArtifact { sha256, bytes })
            .collect();
        let inventory = EvidenceArchive {
            format: EVIDENCE_ARCHIVE_FORMAT.to_string(),
            root: root.to_path_buf(),
            engine: env!("CARGO_PKG_VERSION").to_string(),
            archived_at: now.to_string(),
            artifacts,
            total_bytes,
        };
        let bytes = serde_json::to_vec_pretty(&inventory)?;
        let mut file = self.inventory(&bytes, written)?;
        super::sync_dir(&self.dir)?;
        // The bytes written are the bytes the inventory names: a reader
        // parses them back through this build's one reader, from the very
        // descriptor that wrote them.
        let mut kept = Vec::new();
        file.seek(SeekFrom::Start(0))?;
        file.read_to_end(&mut kept)?;
        super::read_archive(&kept)?;
        Ok(canonical::sha256_bytes(&bytes))
    }

    /// The archive's `sha256/` directory, made new beneath the resolved
    /// destination and opened without following a link.
    fn sha256_dir(&self) -> Result<OwnedFd, EvidenceError> {
        mkdirat(&self.dir, DIGESTS, Mode::RWXU)
            .map_err(|error| EvidenceError::Io(std::io::Error::from(error)))?;
        match child(&self.dir, OsStr::new(DIGESTS), &self.at.join(DIGESTS)) {
            Ok(dir) => Ok(dir),
            Err(refused) => {
                let _ = unlinkat(&self.dir, DIGESTS, AtFlags::REMOVEDIR);
                Err(refused)
            }
        }
    }

    /// `bytes` written as the archive's inventory, created new beneath the
    /// resolved destination without following a link, and synced. `written`
    /// learns it was created only once creation succeeded.
    fn inventory(
        &self,
        bytes: &[u8],
        written: &mut Written,
    ) -> Result<std::fs::File, EvidenceError> {
        let mut file = create(&self.dir, INVENTORY_FILE)?;
        written.inventory = true;
        file.write_all(bytes)?;
        file.sync_all()?;
        Ok(file)
    }

    /// Everything this archive wrote removed through the descriptors that
    /// wrote it: the copies, the digest directory, the inventory, and the
    /// destination directories where this verb made them. A name this verb
    /// did not create is left where it was (0105, 1020b).
    fn removed(&self, written: &Written) {
        if written.inventory {
            let _ = unlinkat(&self.dir, INVENTORY_FILE, AtFlags::empty());
        }
        if let Some(shas) = &written.shas {
            for name in written.names.iter().rev() {
                let _ = unlinkat(shas, OsStr::new(name), AtFlags::empty());
            }
            let _ = unlinkat(&self.dir, DIGESTS, AtFlags::REMOVEDIR);
        }
        remove_dirs(&self.made);
    }
}

#[cfg(test)]
impl Destination {
    /// The archive's inventory written through this destination and then
    /// removed by the refusal path: the one test that an inventory this verb
    /// created is the one `removed` unlinks (0105, 1020b).
    pub(super) fn wrote_inventory(&self) {
        let mut written = Written::default();
        self.inventory(b"{}", &mut written).unwrap();
        self.removed(&written);
    }
}

/// `to`'s absolute path resolved once by the filesystem: the deepest
/// ancestor that stands, through the links it holds, with the plain names
/// below it that do not stand yet.
pub(super) fn resolved(to: &Path) -> Result<PathBuf, EvidenceError> {
    Ok(resolve(to)?.at)
}

/// Where an absolute path resolved: the path it names, the deepest standing
/// ancestor, the plain names below that ancestor, and that ancestor's device
/// and inode.
struct Resolved {
    at: PathBuf,
    base: PathBuf,
    rest: Vec<OsString>,
    identity: (u64, u64),
}

/// [`resolved`] with the deepest standing ancestor, the plain names below
/// it and that ancestor's identity (device and inode) kept apart, so the
/// archive can be made under that one descriptor and checked to be the
/// directory that was compared. A `.` names the same directory and is
/// dropped by the components walk, so the resolved path never carries one;
/// a `..` below the deepest standing ancestor refuses, because nothing can
/// resolve it before what it names exists.
fn resolve(to: &Path) -> Result<Resolved, EvidenceError> {
    let absolute = std::path::absolute(to)?;
    let mut base = PathBuf::new();
    let mut rest: Vec<OsString> = Vec::new();
    for part in absolute.components() {
        match (rest.is_empty(), part) {
            (true, Component::RootDir) => base.push(part.as_os_str()),
            (true, Component::Normal(name)) => {
                let next = base.join(name);
                match std::fs::canonicalize(&next) {
                    Ok(at) => base = at,
                    Err(_) => rest.push(name.to_owned()),
                }
            }
            (true, Component::ParentDir) => {
                base = std::fs::canonicalize(base.join(part.as_os_str()))?;
            }
            (false, Component::Normal(name)) => rest.push(name.to_owned()),
            _ => return Err(EvidenceError::Unresolved(to.to_path_buf())),
        }
    }
    let found = std::fs::metadata(&base)?;
    let identity = (found.dev(), found.ino());
    let at = rest.iter().fold(base.clone(), |at, name| at.join(name));
    Ok(Resolved {
        at,
        base,
        rest,
        identity,
    })
}

/// The directory `at` opened without following a link, then checked by
/// `fstat` to be the very directory whose identity [`resolve`] captured:
/// a component swapped between the comparison and the open refuses rather
/// than writing somewhere never compared. A name that is not a directory
/// or is a link refuses as [`DestinationNotDirectory`]; any other open
/// error, a permission denied among them, is the io error it is (0105,
/// 1020b).
///
/// [`DestinationNotDirectory`]: EvidenceError::DestinationNotDirectory
pub(super) fn directory(at: &Path, identity: (u64, u64)) -> Result<OwnedFd, EvidenceError> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let dir = match openat(CWD, at, flags, Mode::empty()) {
        Ok(dir) => dir,
        Err(Errno::NOTDIR) | Err(Errno::LOOP) => {
            return Err(EvidenceError::DestinationNotDirectory(at.to_path_buf()))
        }
        Err(error) => return Err(std::io::Error::from(error).into()),
    };
    // Read through std's metadata of the same descriptor, whose device is a
    // u64 on every platform: rustix's `st_dev` is an i32 on macOS.
    let found = std::fs::File::from(dir.try_clone()?).metadata()?;
    let same = (found.dev(), found.ino()) == identity;
    same.then_some(dir)
        .ok_or_else(|| EvidenceError::Replaced(at.to_path_buf()))
}

/// The directory `name` in `dir`, at `at`, opened without following a
/// link. A name that is not a directory or is a link refuses as
/// [`DestinationNotDirectory`]; any other open error is the io error it is
/// (0105, 1020b).
///
/// [`DestinationNotDirectory`]: EvidenceError::DestinationNotDirectory
fn child(dir: &OwnedFd, name: &OsStr, at: &Path) -> Result<OwnedFd, EvidenceError> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    match openat(dir, name, flags, Mode::empty()) {
        Ok(dir) => Ok(dir),
        Err(Errno::NOTDIR) | Err(Errno::LOOP) => {
            Err(EvidenceError::DestinationNotDirectory(at.to_path_buf()))
        }
        Err(error) => Err(std::io::Error::from(error).into()),
    }
}

/// Every plain name of `rest` made beneath `base` one at a time, each
/// opened without following a link; the directory of the last, and every
/// directory this made so a refusal removes exactly those.
fn descendants(
    mut dir: OwnedFd,
    mut at: PathBuf,
    rest: &[OsString],
) -> Result<(OwnedFd, Vec<Made>), EvidenceError> {
    let mut made = Vec::new();
    for name in rest {
        at.push(name);
        let parent = dir.try_clone()?;
        match mkdirat(&dir, name.as_os_str(), Mode::RWXU) {
            Ok(()) => made.push(Made {
                parent,
                name: name.clone(),
            }),
            Err(rustix::io::Errno::EXIST) => {}
            Err(error) => {
                remove_dirs(&made);
                return Err(std::io::Error::from(error).into());
            }
        }
        match child(&dir, name.as_os_str(), &at) {
            Ok(next) => dir = next,
            Err(refused) => {
                remove_dirs(&made);
                return Err(refused);
            }
        }
    }
    Ok((dir, made))
}

/// The directories `made` removed, deepest first, each through the
/// descriptor of the parent it was made in.
fn remove_dirs(made: &[Made]) {
    for made in made.iter().rev() {
        let _ = unlinkat(&made.parent, made.name.as_os_str(), AtFlags::REMOVEDIR);
    }
}

/// Whether the directory `dir` holds no entry but `.` and `..`.
fn empty(dir: &OwnedFd) -> Result<bool, EvidenceError> {
    for entry in Dir::read_from(dir).map_err(std::io::Error::from)? {
        let entry = entry.map_err(std::io::Error::from)?;
        let name = entry.file_name().to_bytes();
        if (name != b".") & (name != b"..") {
            return Ok(false);
        }
    }
    Ok(true)
}

/// The new owner-only file `name` in `dir`, created without following a
/// link.
fn create(dir: &OwnedFd, name: &str) -> Result<std::fs::File, EvidenceError> {
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(dir, name, flags, Mode::RUSR | Mode::WUSR)
        .map_err(|error| EvidenceError::Io(std::io::Error::from(error)))?;
    Ok(std::fs::File::from(file))
}

/// `source` copied to the new file `name` in `dir`, both digests named
/// `name`: the source read as one regular file of one link, its bytes
/// hashed as they are written, and the copy read back from the same
/// descriptor and hashed. A mismatch refuses by the source's name. The
/// archive records its own names as it creates them ([`written_into`]); a
/// test drives this one whole.
#[cfg(test)]
pub(super) fn copied(source: &Path, dir: &OwnedFd, name: &str) -> Result<(), EvidenceError> {
    let mut out = create(dir, name)?;
    written_into(source, &mut out, name)
}

/// [`copied`] into the file `out` that `create` already made: the copy's
/// digest checked as it is written and read back. Kept apart so the caller
/// records the name only once creation succeeded (0105, 1020b).
fn written_into(source: &Path, out: &mut std::fs::File, name: &str) -> Result<(), EvidenceError> {
    let refused = || EvidenceError::Corrupt(source.to_path_buf());
    let mut source = std::fs::File::from(regular(source).ok_or_else(refused)?);
    let (mut hasher, mut chunk, mut total) = (Sha256::new(), vec![0; CHUNK], 0_u64);
    loop {
        let read = source.read(&mut chunk)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        hasher.update(&chunk[..read]);
        out.write_all(&chunk[..read])?;
    }
    out.sync_all()?;
    let mut kept = Vec::new();
    out.seek(SeekFrom::Start(0))?;
    out.read_to_end(&mut kept)?;
    let kept = canonical::sha256_bytes(&kept);
    let written = (total <= ARTIFACT_BYTES_MAX)
        & (format!("{:x}", hasher.finalize()) == name)
        & (kept == name);
    written.then_some(()).ok_or_else(refused)
}
