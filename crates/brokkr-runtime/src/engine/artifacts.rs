//! Retained responses published as content-addressed artifacts (decision
//! 0065 slice two, U8c; D6, CR2, CR5).
//!
//! Once every process an admitted attempt owned is proven settled, and
//! while its lease still stands, the engine reads the attempt's protected
//! evidence from the directory provisioning made below the coordination
//! root, every step from a handle and without following a link: its
//! inventory, each plan it pins, whose bytes must digest to their pin and
//! whose owner must be this attempt, and each plan's ledger, read through
//! the protocol's one reader ([`recover`]), whose verified prefix alone
//! says which responses are cited (`Lifecycle::cited`). The outcome
//! never stands in for a citation: a retained child error and a failure of
//! the broker's own are both failed. A plan that does not retain cites
//! nothing, and a stage no Terminal cites, a partial one included, is
//! never read.
//!
//! Each cited stage is this user's own regular file of one link in that
//! directory, and its bytes, read on the handle judged, must be exactly
//! the cited length and digest: the broker's writer takes the digest its
//! caller gives, so the engine checks the bytes itself. Those very bytes
//! are then published at `artifacts/sha256/<hex>` in the run-local
//! directory, below the artifact root as admission sealed it, its identity
//! checked on the handle every later step is taken from: written to an
//! exclusive temporary file beside the digests, synced, linked to its
//! digest path, which never replaces what stands there, and the
//! directories synced; the digest path then names the very file written.
//! An artifact already at its path is reused only once its bytes, read on
//! the handle judged, digest to its name, and it and the directories are
//! synced again; one that does not, or is not this user's own regular file
//! of one link, is refused and left as it stands.
//! Durable publication precedes any checkpoint citing it (U8d).

use std::ffi::{OsStr, OsString};
use std::fs::{File, Metadata};
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

use brokkr_core::canonical;
use brokkr_protocol::broker::ledger::recover;
use brokkr_protocol::broker::{Inventory, Pin, Plan};
use rustix::fs::{fsync, linkat, mkdirat, openat, unlinkat, AtFlags, Mode, OFlags, CWD};
use rustix::io::Errno;
use thiserror::Error;

use super::broker::{directory, INVENTORY, REPLY_BYTES_MAX};

/// The directory of an artifact root its artifacts stand in, each named by
/// its bytes' sha256.
pub(super) const DIGESTS: &str = "sha256";

/// MB3's bound of 8 MiB per response and canonical artifact, the largest
/// any artifact retention publishes may be (M1 of U8a2a's run four's second
/// review); its 1 MiB bound is the request's.
pub(super) const ARTIFACT_BYTES_MAX: u64 = 8 << 20;

/// CR3's bound on one ledger, which the broker's writer never passes.
pub(super) const LEDGER_BYTES_MAX: u64 = 64 << 20;

/// What a temporary file being published is named with, beside the
/// digests: never in them, where every name must be a digest.
const PUBLISHING: &str = ".publishing";

/// Why a cited response is not published (CR2), in the operator's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(in crate::engine) enum Unpublished {
    /// The attempt's evidence is not what this attempt sealed: its
    /// directory, inventory, a plan, a ledger or a cited stage is not this
    /// user's own regular file or directory as provisioning made it, a plan
    /// does not digest to its pin or names another owner, a stage a
    /// Terminal cites is missing, or a plan that retains nothing, or an
    /// attempt that protects no artifact root, has a citation.
    #[error("broker evidence is not bound to this attempt")]
    Unbound,
    /// A cited stage's bytes are not the cited length and digest.
    #[error("broker response digest mismatch")]
    Response,
    /// Another artifact's bytes stand at the digest path.
    #[error("retained artifact digest mismatch")]
    Collision,
    /// The artifact root is not the directory admitted, or it, its digests
    /// or the digest path is not this user's own, a link, another kind of
    /// file, or of more than one link, or the digest path was replaced
    /// while it was published.
    #[error("retained artifact path is not a regular contained file")]
    Uncontained,
    /// The artifact could not be written, synced or linked in.
    #[error("retained artifact could not be published durably")]
    Undurable,
}

/// Why a retained artifact is not read (CR5), in the operator's words.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub(super) enum Unread {
    #[error("retained response artifact is missing")]
    Missing,
    #[error("retained response artifact digest mismatch")]
    Corrupt,
    #[error("retained artifact path is not a regular contained file")]
    Uncontained,
}

/// Where one attempt's evidence is read: its run's directory below the
/// coordination root, held open, and that directory's path; and the
/// worktree's key, the run and the attempt every plan of it must name.
pub(in crate::engine) struct Evidence<'e> {
    pub(in crate::engine) run: &'e OwnedFd,
    pub(in crate::engine) at: PathBuf,
    pub(in crate::engine) owner: [&'e str; 3],
}

/// The artifact root as admission sealed it: its path, and its device and
/// inode.
pub(in crate::engine) type Sealed = (PathBuf, (u64, u64));

/// Where publication stands when it hands its hook a directory and a
/// name: the temporary file written, before it is synced; linked in at its
/// digest path and its temporary name removed, before the directories are
/// synced and the digest path read again; or an artifact standing at its
/// digest path verified, before the directories are synced to reuse it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Written,
    Linked,
    Standing,
}

/// What publication hands at each [`Step`]; an error it returns fails that
/// step as the step's own write or sync failing would. Nothing in
/// production ([`unhooked`]); a unit test's own, to fail a step or replace
/// the digest path there.
pub(super) type Hook = fn(Step, &OwnedFd, &str) -> std::io::Result<()>;

/// Production's hook: nothing.
fn unhooked(_: Step, _: &OwnedFd, _: &str) -> std::io::Result<()> {
    Ok(())
}

/// Whether `name` is a sha256's lowercase hex: a worktree's key, or an
/// artifact's name.
pub(super) fn hex(name: &str) -> bool {
    let digit = |byte: &u8| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte);
    (name.len() == 64) & name.as_bytes().iter().all(digit)
}

/// Publish every response the verified ledgers of the attempt `evidence`
/// locates cite, below the artifact root `sealed` as admitted, where it
/// protects one ([`published_with`]).
pub(in crate::engine) fn published(
    evidence: &Evidence<'_>,
    sealed: Option<&Sealed>,
) -> Result<(), Unpublished> {
    published_with(evidence, sealed, unhooked)
}

/// [`published`], each publication's steps handed to `hook`: each plan in
/// the order its inventory pins it ([`provisioned`]), and each citation in
/// call order, published as soon as it is verified.
pub(super) fn published_with(
    evidence: &Evidence<'_>,
    sealed: Option<&Sealed>,
    hook: Hook,
) -> Result<(), Unpublished> {
    let Some((dir, inventory)) = provisioned(evidence)? else {
        return Ok(());
    };
    let [_, _, attempt] = evidence.owner;
    let (at, mut root) = (evidence.at.join(attempt), None);
    for pin in &inventory.plans {
        let (name, plan) = pinned(&dir, &at, pin, evidence.owner)?;
        for (call, sha256, length) in cited(&dir, name, &plan, &pin.digest)? {
            let bytes = staged(&dir, name, call, (&sha256, length))?;
            let rooted = match &root {
                Some(rooted) => rooted,
                None => root.insert(rooted(sealed.ok_or(Unpublished::Unbound)?)?),
            };
            put(rooted, &sha256, &bytes, hook)?;
        }
    }
    Ok(())
}

/// The directory of the attempt `evidence` locates, held open, this user's
/// alone, and the inventory it holds; none where the attempt provisioned
/// no plan, or its inventory was never written, so no broker could bind
/// one.
pub(super) fn provisioned(
    evidence: &Evidence<'_>,
) -> Result<Option<(OwnedFd, Inventory)>, Unpublished> {
    let [_, _, attempt] = evidence.owner;
    let dir = match openat(evidence.run, attempt, directory(), Mode::empty()) {
        Err(Errno::NOENT) => return Ok(None),
        opened => owned(opened.or(Err(Unpublished::Unbound))?, None, ALONE),
    };
    let dir = dir.ok_or(Unpublished::Unbound)?;
    let Some(inventory) = record(&dir, OsStr::new(INVENTORY), REPLY_BYTES_MAX)? else {
        return Ok(None);
    };
    let inventory = serde_json::from_slice(&inventory).or(Err(Unpublished::Unbound))?;
    Ok(Some((dir, inventory)))
}

/// The plan `pin` pins in the attempt directory `dir` at `at`: its name
/// beside the inventory, and the plan, its bytes digesting to the pin and
/// its owner the worktree's key, run and attempt `owner` names.
pub(super) fn pinned<'p>(
    dir: &OwnedFd,
    at: &Path,
    pin: &'p Pin,
    [repo, run, attempt]: [&str; 3],
) -> Result<(&'p OsStr, Plan), Unpublished> {
    let beside = (pin.locator.parent() == Some(at)).then_some(());
    let name = beside
        .and(pin.locator.file_name())
        .ok_or(Unpublished::Unbound)?;
    let bytes = record(dir, name, REPLY_BYTES_MAX)?.ok_or(Unpublished::Unbound)?;
    let digested = canonical::sha256_bytes(&bytes) == pin.digest;
    let plan: Plan = (digested
        .then(|| serde_json::from_slice(&bytes).ok())
        .flatten())
    .ok_or(Unpublished::Unbound)?;
    let owner = &plan.owner;
    let ours = (owner.repo == repo) & (owner.run == run) & (owner.attempt == attempt);
    ours.then_some((name, plan)).ok_or(Unpublished::Unbound)
}

/// What the ledger of the plan named `name` in `dir`, sealed at `digest`,
/// cites in its verified prefix (`Lifecycle::cited`): each call, and the
/// sha256 and length its Terminal cites. None where the plan's broker never
/// opened its ledger; refused where a plan that retains nothing cites any.
fn cited(
    dir: &OwnedFd,
    name: &OsStr,
    plan: &Plan,
    digest: &str,
) -> Result<Vec<(u64, String, u64)>, Unpublished> {
    let ledger = beside(name, ".ledger");
    let Some(bytes) = record(dir, &ledger, LEDGER_BYTES_MAX)? else {
        return Ok(Vec::new());
    };
    let (lifecycle, _) = recover(&bytes, &plan.owner, digest);
    let cited = lifecycle.cited();
    let cited = cited.map(|(call, sha256, length)| (call, sha256.to_owned(), length));
    let cited: Vec<_> = cited.collect();
    let retained = plan.retained | cited.is_empty();
    retained.then_some(cited).ok_or(Unpublished::Unbound)
}

/// The bytes the broker staged for call `call` of the plan named `name` in
/// `dir`, exactly those its Terminal cites: read no further than one byte
/// past the cited `length`, and both of that length and digesting to the
/// cited `sha256`, for a Terminal can cite a sound digest beside a length
/// that is not its bytes'.
fn staged(
    dir: &OwnedFd,
    name: &OsStr,
    call: u64,
    (sha256, length): (&str, u64),
) -> Result<Vec<u8>, Unpublished> {
    let stage = beside(name, &format!(".{call}.response"));
    let bytes = private(dir, &stage, length)?.ok_or(Unpublished::Unbound)?;
    let exact = (bytes.len() as u64 == length) & (canonical::sha256_bytes(&bytes) == sha256);
    exact.then_some(bytes).ok_or(Unpublished::Response)
}

/// `name` with `suffix` after it.
pub(super) fn beside(name: &OsStr, suffix: &str) -> OsString {
    let mut beside = name.to_os_string();
    beside.push(suffix);
    beside
}

/// The record `name` in `dir` ([`private`]), refused where it holds more
/// than `most` bytes.
pub(super) fn record(
    dir: &OwnedFd,
    name: &OsStr,
    most: u64,
) -> Result<Option<Vec<u8>>, Unpublished> {
    let bytes = private(dir, name, most)?;
    let within = bytes
        .as_ref()
        .is_none_or(|bytes| bytes.len() as u64 <= most);
    within.then_some(bytes).ok_or(Unpublished::Unbound)
}

/// The bytes of the file `name` in `dir`, at most one more than `most`: a
/// regular file of one link, this user's alone, opened without following a
/// link or blocking and judged and read on that one handle; none where it
/// is missing.
fn private(dir: &OwnedFd, name: &OsStr, most: u64) -> Result<Option<Vec<u8>>, Unpublished> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = match openat(dir, name, flags, Mode::empty()) {
        Err(Errno::NOENT) => return Ok(None),
        opened => opened.or(Err(Unpublished::Unbound))?,
    };
    let file = File::from(owned(file, None, ALONE).ok_or(Unpublished::Unbound)?);
    let mut bytes = Vec::new();
    let read = file.take(most + 1).read_to_end(&mut bytes);
    read.map(|_| Some(bytes)).or(Err(Unpublished::Unbound))
}

/// What [`owned`] asks of the mode of the attempt's evidence: this user's
/// alone.
const ALONE: u32 = 0o077;

/// What [`owned`] asks of the mode of the digests and each artifact:
/// writable by no one else.
const SHUT: u32 = 0o022;

/// What [`owned`] asks of the mode of the artifact root: nothing, for it
/// is judged by its identity, the very directory admission made or found.
const ADMITTED: u32 = 0;

/// `handle` where it is this user's own directory, or regular file of one
/// link, none of whose `mode` bits is set, and the very one `identity`
/// names where one is given.
fn owned(handle: OwnedFd, identity: Option<(u64, u64)>, mode: u32) -> Option<OwnedFd> {
    let file = File::from(handle);
    let found: Metadata = file.metadata().ok()?;
    let shape = found.is_dir() | (found.is_file() & (found.nlink() == 1));
    let shut = found.mode() & mode == 0;
    let same = identity.is_none_or(|identity| identity == (found.dev(), found.ino()));
    let mine = found.uid() == brokkr_protocol::hands::ids().0;
    (shape & shut & same & mine).then(|| OwnedFd::from(file))
}

/// The artifact root as admitted, and its digests in it, held open.
struct Rooted {
    root: OwnedFd,
    digests: OwnedFd,
}

/// The artifact root `sealed` opened without following a link as its last
/// component and judged as the very directory admission sealed, so no
/// ancestor redirected since names another; and its digests in it, made
/// owner-only where missing, the root then synced.
fn rooted((path, identity): &Sealed) -> Result<Rooted, Unpublished> {
    let opened = openat(CWD, path, directory(), Mode::empty());
    let root = opened
        .ok()
        .and_then(|root| owned(root, Some(*identity), ADMITTED));
    let root = root.ok_or(Unpublished::Uncontained)?;
    if mkdirat(&root, DIGESTS, Mode::RWXU).is_ok() {
        fsync(&root).or(Err(Unpublished::Undurable))?;
    }
    let digests = openat(&root, DIGESTS, directory(), Mode::empty()).ok();
    let digests = digests.and_then(|digests| owned(digests, None, SHUT));
    let digests = digests.ok_or(Unpublished::Uncontained)?;
    Ok(Rooted { root, digests })
}

/// `bytes`, whose sha256 is `sha256`, published in `rooted`'s digests: an
/// artifact already there reused once verified ([`read`]) and made durable
/// ([`reused`]), or one made ([`linked`]), each step handed to `hook`.
fn put(rooted: &Rooted, sha256: &str, bytes: &[u8], hook: Hook) -> Result<(), Unpublished> {
    match read(&rooted.digests, sha256) {
        Ok((found, _)) => reused(rooted, &found, sha256, hook),
        Err(Unread::Missing) => linked(rooted, sha256, bytes, hook),
        Err(unread) => Err(standing(unread)),
    }
}

/// The verified artifact `found`, standing at its digest path `sha256` in
/// `rooted`, synced, and the digests and the root synced after it
/// ([`synced`], at [`Step::Standing`]), before it is reused: one an earlier
/// publication linked in, whose directories' sync then failed, is durable
/// only once a later publication makes it so, as is one a racing writer
/// linked in first.
fn reused(rooted: &Rooted, found: &File, sha256: &str, hook: Hook) -> Result<(), Unpublished> {
    let durable = fsync(found)
        .map_err(std::io::Error::from)
        .and_then(|()| synced(rooted, Step::Standing, sha256, hook));
    durable.or(Err(Unpublished::Undurable))
}

/// The digests in `rooted` and the root synced, `hook` handed `step` and
/// the digest path `sha256` first.
fn synced(rooted: &Rooted, step: Step, sha256: &str, hook: Hook) -> std::io::Result<()> {
    hook(step, &rooted.digests, sha256)?;
    fsync(&rooted.digests)?;
    Ok(fsync(&rooted.root)?)
}

/// What an artifact that should stand at its digest path says, where it
/// cannot be read as its name.
fn standing(unread: Unread) -> Unpublished {
    match unread {
        Unread::Corrupt => Unpublished::Collision,
        Unread::Missing | Unread::Uncontained => Unpublished::Uncontained,
    }
}

/// `bytes` written as an exclusive owner-only temporary file beside the
/// digests in `rooted`, synced, linked in at its digest path `sha256`,
/// which replaces nothing standing there, and its temporary name removed;
/// the digests and the root then synced. The digest path must then name
/// the very file written; one an artifact took first is reused once
/// verified and made durable ([`reused`]). A temporary file left by an
/// earlier publication cut short goes first.
///
/// The temporary name is predictable and stands in the artifact root,
/// whose mode is not judged, only its identity: a writer that may write the
/// root can take, replace or remove that name. What it leaves at the digest
/// path is then refused, for the digest path must name the very file
/// written, or verify as its name; such a writer can only stop publication.
fn linked(rooted: &Rooted, sha256: &str, bytes: &[u8], hook: Hook) -> Result<(), Unpublished> {
    let temporary = format!(".{sha256}{PUBLISHING}");
    let _ = unlinkat(&rooted.root, temporary.as_str(), AtFlags::empty());
    let written = written(&rooted.root, &temporary, bytes, hook).map(|file| {
        let digests = &rooted.digests;
        let link = linkat(
            &rooted.root,
            temporary.as_str(),
            digests,
            sha256,
            AtFlags::empty(),
        );
        (file, link)
    });
    let removed = unlinkat(&rooted.root, temporary.as_str(), AtFlags::empty());
    let (ours, link) = written?;
    match link {
        Ok(()) => {}
        Err(Errno::EXIST) => {
            let (found, _) = read(&rooted.digests, sha256).map_err(standing)?;
            return reused(rooted, &found, sha256, hook);
        }
        Err(_) => return Err(Unpublished::Undurable),
    }
    let durable = removed
        .map_err(std::io::Error::from)
        .and_then(|()| synced(rooted, Step::Linked, sha256, hook));
    durable.or(Err(Unpublished::Undurable))?;
    let (found, _) = read(&rooted.digests, sha256).map_err(standing)?;
    let same = |file: &File| file.metadata().ok().map(|found| (found.dev(), found.ino()));
    let ours = same(&ours).is_some_and(|ours| Some(ours) == same(&found));
    ours.then_some(()).ok_or(Unpublished::Uncontained)
}

/// `bytes` written as the new owner-only, read-only file `temporary` in
/// `root`, handed to `hook` and synced; the file, held open.
fn written(root: &OwnedFd, temporary: &str, bytes: &[u8], hook: Hook) -> Result<File, Unpublished> {
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let made = openat(root, temporary, flags, Mode::RUSR);
    let mut file = File::from(made.or(Err(Unpublished::Undurable))?);
    let durable = (file.write_all(bytes))
        .and_then(|()| hook(Step::Written, root, temporary))
        .and_then(|()| file.sync_all());
    durable.map(|()| file).or(Err(Unpublished::Undurable))
}

/// The artifact `sha256` in the digests `digests` (CR5): the handle judged
/// and its bytes, read on it, where it is this user's own regular file of
/// one link, opened without following a link or blocking, writable by no
/// one else, whose bytes digest to its name; or why not.
pub(super) fn read(digests: &OwnedFd, sha256: &str) -> Result<(File, Vec<u8>), Unread> {
    hex(sha256).then_some(()).ok_or(Unread::Uncontained)?;
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let opened = match openat(digests, sha256, flags, Mode::empty()) {
        Err(Errno::NOENT) => return Err(Unread::Missing),
        opened => opened.ok().and_then(|file| owned(file, None, SHUT)),
    };
    let file = File::from(opened.ok_or(Unread::Uncontained)?);
    file.metadata()
        .ok()
        .filter(Metadata::is_file)
        .ok_or(Unread::Uncontained)?;
    let mut bytes = Vec::new();
    (&file)
        .take(ARTIFACT_BYTES_MAX + 1)
        .read_to_end(&mut bytes)
        .or(Err(Unread::Corrupt))?;
    let named =
        (bytes.len() as u64 <= ARTIFACT_BYTES_MAX) && canonical::sha256_bytes(&bytes) == sha256;
    named.then_some((file, bytes)).ok_or(Unread::Corrupt)
}

#[cfg(test)]
pub(super) mod tests;
