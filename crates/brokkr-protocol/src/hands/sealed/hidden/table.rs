//! The secrets store's own facts, judged against the host's mount table
//! (decision 0065 slice two, U8h): one owner-only regular file of one link
//! on a local mount, which no other mount shows in the box's reach. Moved
//! out of `hidden.rs`, which the store's judgments outgrew.

use std::path::Path;

#[cfg(all(test, target_os = "linux"))]
use super::TABLE;
use super::{Facts, Mounted, Unhidden};

/// The facts of the store at its sealed `place`, refused where it is not
/// one regular file of one link its user owns ([`single`]), where its mode
/// grants its group or others anything, where the mount table proves no
/// local mount it lies on ([`within`]), or where any other mount of that
/// mount's device shows it, or a directory holding it, at another place
/// within one of `mounts` ([`aliases`], [`exposed`]); its own place, which
/// a directory mounted back onto itself shows again, is no other. The
/// device `within` matches is the table's own for the store's mount, never
/// the store's `stat`, which a btrfs subvolume reports apart from its
/// mount's.
#[cfg(target_os = "linux")]
pub(super) fn judged(place: &Path, mounts: &[Mounted]) -> Result<Facts, Unhidden> {
    use crate::hands::namespace::sources::host::aliases;
    use crate::hands::namespace::sources::host::store::{exposed, single, spelled, within};
    use rustix::fs::{statx, AtFlags, StatxFlags, CWD};
    let unmapped = Unhidden::Unmapped(place.to_path_buf());
    let mask = StatxFlags::BASIC_STATS | StatxFlags::MNT_ID;
    let held = statx(CWD, place, AtFlags::SYMLINK_NOFOLLOW, mask).or(Err(unmapped.clone()))?;
    let (mode, owner) = (
        u32::from(held.stx_mode),
        rustix::process::geteuid().as_raw(),
    );
    let one = single(mode, held.stx_nlink, held.stx_uid, owner);
    one.then_some(())
        .ok_or(Unhidden::Linked(place.to_path_buf()))?;
    let shared = Unhidden::Shared(place.to_path_buf(), mode & 0o7777);
    (mode & 0o077 == 0).then_some(()).ok_or(shared)?;
    let named = held.stx_mask & StatxFlags::MNT_ID.bits() != 0;
    let records = named.then(table).flatten().unwrap_or_default();
    let mount = held.stx_mnt_id;
    let dev = records.iter().find(|record| record.id == mount);
    let dev = dev.map(|record| record.dev).unwrap_or_default();
    let (own, inside) = within(&records, (mount, dev), place).or(Err(unmapped))?;
    let reach = spelled(mounts.iter().map(|mount| mount.host.as_path()));
    let mut shown = aliases(&records, own.id, own.dev, &inside).map(|(at, rest)| at.join(rest));
    let alias = shown.find(|alias| (alias != place) & exposed(std::iter::once(alias), &reach));
    let facts = (
        (held.stx_dev_major, held.stx_dev_minor),
        held.stx_ino,
        mount,
    );
    alias.map_or(Ok(facts), |alias| {
        Err(Unhidden::Shown(place.to_path_buf(), alias))
    })
}

/// No host but Linux builds a workspace box (decision 0063), and none has
/// the mount table this proves a store's place by: a store sealed and
/// present refuses.
#[cfg(not(target_os = "linux"))]
pub(super) fn judged(place: &Path, _: &[Mounted]) -> Result<Facts, Unhidden> {
    Err(Unhidden::Unmapped(place.to_path_buf()))
}

/// The host's mount table, read within MB3's bound; or, in a test, the
/// table it planted ([`TABLE`]).
#[cfg(target_os = "linux")]
fn table() -> Option<Vec<crate::hands::namespace::sources::host::Record>> {
    use crate::hands::namespace::sources::host::{store, Host};
    #[cfg(test)]
    if let Some(text) = TABLE.with_borrow(Clone::clone) {
        let planted = move || -> Option<Box<dyn std::io::Read>> {
            Some(Box::new(std::io::Cursor::new(text.clone())))
        };
        return store::table(&Host {
            mountinfo: &planted,
            ..Host::live()
        })
        .ok();
    }
    store::table(&Host::live()).ok()
}
