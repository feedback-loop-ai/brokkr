//! MB4's store identity (decision 0065 slice two, U6c5c): the operator's
//! secret store resolved from `/` through descriptors, as every source is,
//! and held by a path handle, which reads nothing and is never mounted. A
//! store that is absent, no regular file, multiply linked or another
//! user's has no identity admission can prove; nor has one whose route
//! anyone but its owner or root could change, that a second resolution
//! does not find again on the same route, or whose own mount, or any hop's,
//! the table does not prove. Every place the store shows at, its own and
//! each mount alias of it or of a directory holding it, and every place a
//! hop of its route shows at, are then compared with what the workspace
//! hands could read: the seat's reach and the system set their box binds.
//! Last, every place the store shows at is compared with the sources the
//! server box mounts. No value is read and no store is made, so a
//! secret-free plan proves its store as one with bindings does.

use std::io::Read;
use std::os::fd::{AsFd, OwnedFd};
use std::path::{Path, PathBuf};

use rustix::fs::FileType;

use super::super::super::{spellings, Profile};
use super::super::{owned, stat_at, Facts};
use super::{acl, aliases, holding, opened, records, resolve, root, route};
use super::{Acl, Hop, Host, Owned, Record};
use crate::broker::{Reach, Refusal};

/// The store at `store`, admitted against the seat's `reach` and the box's
/// `sources` on `host`: its path handle, or MB4's first cause. Identity is
/// unprotected where the store is absent, does not resolve, is not
/// [`single`], has a route not [`guarded`], resolves a second time, after
/// `host.between`, by another route or to another object, or where its own
/// mount or a hop's is not proved; then the store is reachable by the hands
/// where any place it or a hop of its route shows at lies in reach or in
/// the workspace box's system set; then it would be in the box where any
/// place it shows at lies in a source.
pub(in crate::hands) fn admitted<'s>(
    store: &Path,
    reach: &Reach,
    sources: impl Iterator<Item = &'s Path>,
    host: &Host<'_>,
) -> Result<OwnedFd, Refusal> {
    let (resolved, fd, facts) = opened(store, host)?;
    let owner = rustix::process::geteuid().as_raw();
    let held = single(facts.mode, facts.nlink, facts.uid, owner);
    let held = held & guarded(top(), route(&resolved.chain), owner);
    (host.between)();
    let again = resolve(store, host)?;
    let same = again.is_some_and(|again| {
        let routed = route(&again.chain) == route(&resolved.chain);
        routed & (again.facts.identity() == facts.identity())
    });
    (held & same).then_some(()).ok_or(Refusal::Identity)?;
    let records = table(host)?;
    let shown = shown(&records, &resolved.place, &facts)?;
    let routed = routed(&records, route(&resolved.chain))?;
    let declared = reach.writable.iter().chain(&reach.readable);
    let mut hands = spelled(declared.map(PathBuf::as_path));
    hands.extend(spelled(Profile::Workspace.system().map(Path::new)));
    let reached = exposed(routed.iter().chain(&shown), &hands);
    (!reached).then_some(()).ok_or(Refusal::StoreReachable)?;
    let boxed = exposed(shown.iter(), &spelled(sources));
    (!boxed).then_some(fd).ok_or(Refusal::StoreInBox)
}

/// Whether a `mode`, a link count `nlink` and an owner `uid` are a regular
/// file's of one link that `owner` owns.
pub(in crate::hands) fn single(mode: u32, nlink: u32, uid: u32, owner: u32) -> bool {
    let file = FileType::from_raw_mode(mode) == FileType::RegularFile;
    file & (nlink == 1) & (uid == owner)
}

/// Who may write `/`, read through the handle every resolution starts
/// from, which no resolution records as a hop; none where it does not
/// read.
pub(in crate::hands) fn top() -> Option<Owned> {
    let fd = root().ok()?;
    let facts = stat_at(fd.as_fd(), c"").ok()?;
    Some(owned(&facts, acl(fd.as_fd())))
}

/// Whether none but the store's `owner` and root can change its route:
/// `top`, the root it starts from, read, and it and each of `hops`
/// [`shut`].
pub(in crate::hands) fn guarded(top: Option<Owned>, hops: &[Hop], owner: u32) -> bool {
    let hops = hops.iter().map(|hop| hop.owned);
    top.is_some_and(|top| std::iter::once(top).chain(hops).all(|hop| shut(hop, owner)))
}

/// Whether only `owner` or root can change the hop `owned` describes, as
/// the broker guards a plan's ancestry: theirs, and, unless a link, whose
/// mode means nothing, writable by no one else but where it is root's and
/// sticky, and with no extended access ACL.
pub(in crate::hands) fn shut(owned: Owned, owner: u32) -> bool {
    let theirs = (owned.uid == 0) | (owned.uid == owner);
    let link = FileType::from_raw_mode(owned.mode) == FileType::Symlink;
    let sticky = (owned.uid, owned.mode & 0o1000) == (0, 0o1000);
    let closed = (owned.mode & 0o022 == 0) | sticky;
    theirs & (link | (closed & (owned.acl == Acl::Absent)))
}

/// Each of `roots` as spelled and as it resolves.
pub(in crate::hands) fn spelled<'r>(roots: impl Iterator<Item = &'r Path>) -> Vec<PathBuf> {
    roots.flat_map(spellings).collect()
}

/// Whether any of `places` lies within any of `roots`: a route's hops
/// hold its end, so a root that only lies below a hop exposes nothing.
pub(in crate::hands) fn exposed<'p>(
    mut places: impl Iterator<Item = &'p PathBuf>,
    roots: &[PathBuf],
) -> bool {
    places.any(|place| roots.iter().any(|root| place.starts_with(root)))
}

/// The mount table, read within its bound.
pub(in crate::hands) fn table(host: &Host<'_>) -> Result<Vec<Record>, Refusal> {
    let mut text = Vec::new();
    let mut table = (host.mountinfo)().ok_or(Refusal::Identity)?;
    table
        .read_to_end(&mut text)
        .map_err(|_| Refusal::Identity)?;
    records(&text, host.limits.mounts)
}

/// The record of the mount `mount` of the device `dev` that `place` lies
/// on, which must be a local filesystem's, and where in that filesystem
/// `place` lies; identity unprotected where the table proves no such mount.
pub(in crate::hands) fn within<'r>(
    records: &'r [Record],
    (mount, dev): (u64, (u32, u32)),
    place: &Path,
) -> Result<(&'r Record, PathBuf), Refusal> {
    let own = records.iter().find(|record| record.id == mount);
    let own = own.filter(|record| (record.dev == dev) & record.local);
    let own = own.ok_or(Refusal::Identity)?;
    let rest = place
        .strip_prefix(&own.point)
        .map_err(|_| Refusal::Identity)?;
    Ok((own, own.root.join(rest)))
}

/// Every place the store at `place` shows at, as the mount table proves
/// it: its own, and each place another mount of its device shows it or a
/// directory holding it at.
fn shown(records: &[Record], place: &Path, facts: &Facts) -> Result<Vec<PathBuf>, Refusal> {
    let (own, inside) = within(records, (facts.mount, facts.dev), place)?;
    let aliased = aliases(records, own.id, own.dev, &inside).map(|(point, rest)| point.join(rest));
    Ok([place.to_path_buf()].into_iter().chain(aliased).collect())
}

/// Every place a hop of `hops` shows at ([`showing`]), so a link on the
/// route that a mount puts in reach exposes the store it names.
fn routed(records: &[Record], hops: &[Hop]) -> Result<Vec<PathBuf>, Refusal> {
    let places = hops.iter().map(|hop| showing(records, hop));
    Ok(places.collect::<Result<Vec<_>, _>>()?.concat())
}

/// Every place the hop `hop` shows at: its own, and each place another
/// mount of its device shows it, or a directory holding it, at
/// ([`holding`]); identity unprotected where the table proves no mount of
/// it ([`within`]). The store and the box launcher both read a route by it.
pub(in crate::hands) fn showing(records: &[Record], hop: &Hop) -> Result<Vec<PathBuf>, Refusal> {
    let (own, inside) = within(records, (hop.mount, hop.identity.0), &hop.place)?;
    let shown = holding(records, own.id, own.dev, &inside).map(|(point, rest)| point.join(rest));
    Ok([hop.place.clone()].into_iter().chain(shown).collect())
}
