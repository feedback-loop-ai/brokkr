//! The seal machinery a protected root writes a record with: a first
//! admission may make the record, by links, renames and removals only (M3
//! unchanged), and every reseal of a record that must still stand replaces
//! it by one atomic exchange, so no write can bring back a record removed
//! inside its window (decision 0108 ruling 3, #440 fix rounds 440d5 and
//! 440d6).
//!
//! Moved out of `root`, which this would take past its 800 lines (decision
//! 0071 ruling 4).

use std::io::Write;
use std::os::fd::OwnedFd;

use rustix::fs::{
    fsync, linkat, openat, renameat, renameat_with, unlinkat, AtFlags, Mode, OFlags, RenameFlags,
};
use rustix::io::Errno;

use super::{exchanged, regular_at, Commit, Exchange, Hook, Identity, Make, Step, PRIOR};
use crate::engine::broker::admission::{Unknown, STAGED};
use crate::engine::broker::inventory::Budget;

/// `bytes` sealed as `name` in the directory `dir`, begun before `budget`'s
/// deadline. A seal of `name` cut short before it was final is undone
/// first ([`restored`]). The bytes are written as a new owner-only file
/// beside it ([`staged_as`]); then, the deadline checked once more, the
/// record that stood linked as `name` with [`PRIOR`] after it, and the
/// staged copy committed ([`commit`]).
///
/// A seal that may create (a first admission) renames its copy over
/// `name`, and a reseal exchanges its copy with `name` atomically: the
/// exchange fails with `ENOENT` when the name is gone, so no removal at
/// any instant before it is undone, and it fails closed where the
/// filesystem has no exchange (decision 0108 ruling 3, M3 amended by #440
/// fix round 440d6). The commit is handed to `hook`, the directory synced
/// and the deadline checked. A first seal is then committed. A reseal is
/// committed only once the record that stood is retired ([`retired`]): the
/// deadline checked the last time, its prior name removed, and the
/// directory synced again. Until then every read finds the record that
/// stood ([`standing`]).
///
/// A seal that fails or runs late before its commit is undone
/// ([`undone`]): the record that stood put back by the same exchange, the
/// same file, so its bytes and its inode, or the record made removed where
/// none stood (H1; R1 of run four's review). One whose undoing fails
/// refuses as unsealed, whatever cut it short, the record that stood still
/// under its prior name for every read and for the next seal to put back
/// (M2 of run four's second review). Once committed, no seal reports
/// failure or lateness: one whose directory is not proven synced after
/// reports itself committed but [`Commit::Unsynced`] (R1 of the fourth run,
/// U8a2c).
pub(in crate::engine::broker) fn seal_with(
    dir: &OwnedFd,
    name: &str,
    bytes: &[u8],
    budget: &Budget,
    hook: Hook,
) -> Result<Commit, Unknown> {
    seal_made(dir, name, bytes, budget, hook, exchanged, Make::May)
}

/// [`seal_with`], `make` deciding whether the record may be made where none
/// stands, and `exchange` the atomic swap a reseal replaces it by.
pub(super) fn seal_made(
    dir: &OwnedFd,
    name: &str,
    bytes: &[u8],
    budget: &Budget,
    hook: Hook,
    exchange: Exchange,
    make: Make,
) -> Result<Commit, Unknown> {
    budget.check()?;
    restored(dir, name)?;
    let staged = format!("{name}{STAGED}");
    let committed = (staged_as(dir, &staged, bytes, hook))
        .and_then(|()| budget.check())
        .and_then(|()| commit(dir, &staged, name, make, hook, exchange));
    let stood = match committed {
        Ok(stood) => stood,
        Err(refused) => {
            let _ = unlinkat(dir, staged.as_str(), AtFlags::empty());
            return Err(refused);
        }
    };
    let synced =
        hook(Step::Committed, dir, &staged).and_then(|()| fsync(dir).map_err(std::io::Error::from));
    let durable = (synced.or(Err(Unknown::Unsealed))).and_then(|()| budget.check());
    if let Err(refused) = durable {
        let undone = undone(dir, name, stood, hook);
        return Err(undone.map_or(Unknown::Unsealed, |()| refused));
    }
    if !stood {
        return Ok(Commit::Synced);
    }
    retired(dir, name, budget, hook)
}

/// The reseal of `name` in `dir`, exchanged in and synced, committed by
/// retiring the record that stood: handed to `hook`, the deadline of
/// `budget` checked the last time, and its prior name removed; whether the
/// directory is then synced. One late or whose prior name cannot be
/// removed is undone ([`undone`]) and refused, the record that stood still
/// what every read finds.
fn retired(dir: &OwnedFd, name: &str, budget: &Budget, hook: Hook) -> Result<Commit, Unknown> {
    let prior = format!("{name}{PRIOR}");
    let retiring = (hook(Step::Retiring, dir, name).or(Err(Unknown::Unsealed)))
        .and_then(|()| budget.check())
        .and_then(|()| unlinkat(dir, prior.as_str(), AtFlags::empty()).or(Err(Unknown::Unsealed)));
    if let Err(refused) = retiring {
        let undone = undone(dir, name, true, hook);
        return Err(undone.map_or(Unknown::Unsealed, |()| refused));
    }
    // Committed: its successor alone is the record, as every read finds it.
    let synced = hook(Step::Final, dir, name).and_then(|()| Ok(fsync(dir)?));
    Ok(synced.map_or(Commit::Unsynced, |()| Commit::Synced))
}

/// `bytes` written as the new owner-only file `staged` in `dir`, synced
/// and handed to `hook`: what a seal cut short staged goes first, and one
/// that cannot go is refused by the exclusive create.
fn staged_as(dir: &OwnedFd, staged: &str, bytes: &[u8], hook: Hook) -> Result<(), Unknown> {
    let _ = unlinkat(dir, staged, AtFlags::empty());
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(dir, staged, flags, Mode::RUSR | Mode::WUSR);
    let mut file = std::fs::File::from(file.or(Err(Unknown::Unsealed))?);
    let written = (file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| hook(Step::Staged, dir, staged));
    written.or(Err(Unknown::Unsealed))
}

/// The staged copy committed as `name`: a first seal (`Make::May`) links
/// the record that stands under its prior name, so a seal undone puts that
/// very file back, then renames its copy over it, or renames it in where
/// none stands. A reseal (`Make::Never`) links the record that stood, hands
/// `hook` the one point before the exchange, then exchanges its copy with
/// `name` atomically: a removal at any instant before the exchange is met
/// with `ENOENT` and writes nothing back, and a filesystem without an
/// exchange refuses by name. After the exchange the staged name holds the
/// file that stood; it must be the very inode the reseal read, or a foreign
/// file was planted at the name, and the exchange is put back and the seal
/// refused (decision 0108 ruling 3, #440 fix round 440d6). Whether one
/// stood.
fn commit(
    dir: &OwnedFd,
    staged: &str,
    name: &str,
    make: Make,
    hook: Hook,
    exchange: Exchange,
) -> Result<bool, Unknown> {
    let prior = format!("{name}{PRIOR}");
    let stood = match linkat(dir, name, dir, prior.as_str(), AtFlags::empty()) {
        Ok(()) => true,
        Err(Errno::NOENT) if make == Make::May => false,
        Err(_) => return Err(Unknown::Unsealed),
    };
    match make {
        Make::May => {
            let renamed = renameat(dir, staged, dir, name);
            if renamed.is_err() & stood {
                let _ = unlinkat(dir, prior.as_str(), AtFlags::empty());
            }
            renamed.map(|()| stood).or(Err(Unknown::Unsealed))
        }
        Make::Never(read) => exchanged_in(dir, staged, name, read, hook, exchange).map(|()| stood),
    }
}

/// The atomic half of [`commit`]'s reseal: `hook` at [`Step::Linked`], the
/// exchange of `staged` with `name`, and the proof that what the exchange
/// put under `staged` is the inode `read` the reseal read. A record gone
/// before the exchange leaves nothing behind; a foreign file planted at the
/// name is exchanged back and refused; a filesystem without an exchange
/// refuses with [`Unknown::Exchange`]. Once the proof holds, the old record
/// keeps only its prior name until [`retired`] removes it.
fn exchanged_in(
    dir: &OwnedFd,
    staged: &str,
    name: &str,
    read: Identity,
    hook: Hook,
    exchange: Exchange,
) -> Result<(), Unknown> {
    let prior = format!("{name}{PRIOR}");
    let refused = |error: Errno| {
        let _ = unlinkat(dir, prior.as_str(), AtFlags::empty());
        let _ = unlinkat(dir, staged, AtFlags::empty());
        if (error == Errno::INVAL)
            | (error == Errno::NOTSUP)
            | (error == Errno::OPNOTSUPP)
            | (error == Errno::NOSYS)
        {
            Unknown::Exchange
        } else {
            Unknown::Unsealed
        }
    };
    if hook(Step::Linked, dir, name).is_err() {
        let _ = unlinkat(dir, prior.as_str(), AtFlags::empty());
        let _ = unlinkat(dir, staged, AtFlags::empty());
        return Err(Unknown::Unsealed);
    }
    match exchange(dir, staged, dir, name) {
        Ok(()) => {}
        Err(error) => return Err(refused(error)),
    }
    let ours = regular_at(dir, staged).is_some_and(|(_, _, id)| id == read);
    if ours {
        // The record that stood keeps only its prior name until retirement.
        let _ = unlinkat(dir, staged, AtFlags::empty());
        return Ok(());
    }
    // A foreign file was planted at the name. Swap it back; where even that
    // fails, leave the prior link for the next seal's restore, which puts
    // the name's file back by the same exchange.
    if exchange(dir, staged, dir, name).is_ok() {
        let _ = unlinkat(dir, prior.as_str(), AtFlags::empty());
        let _ = unlinkat(dir, staged, AtFlags::empty());
    }
    Err(Unknown::Unsealed)
}

/// A seal that failed or ran late after its commit, undone: the record
/// that `stood` put back from its prior name over the copy that replaced
/// it by the same exchange, or, where none stood, the record it made
/// removed; then handed to `hook` and the directory synced again. Where the
/// name is gone, the removal stands: the record that stood is removed and
/// nothing is written at the name. Why not, where any of that fails: the
/// record that stood is then still under its prior name.
fn undone(dir: &OwnedFd, name: &str, stood: bool, hook: Hook) -> std::io::Result<()> {
    let prior = format!("{name}{PRIOR}");
    let staged = format!("{name}{STAGED}");
    let put = if stood {
        match regular_at(dir, name) {
            Some(_) => renameat_with(dir, prior.as_str(), dir, name, RenameFlags::EXCHANGE)
                .map_err(std::io::Error::from)
                .and_then(|()| unlinkat(dir, prior.as_str(), AtFlags::empty()).map_err(Into::into)),
            // The name is gone: the removal stands. Remove the record that
            // stood, and write nothing at the name.
            None => unlinkat(dir, prior.as_str(), AtFlags::empty()).map_err(Into::into),
        }
    } else {
        unlinkat(dir, name, AtFlags::empty()).map_err(Into::into)
    };
    // The old record may also still be under the staged name when the
    // commitment above did not get to remove it.
    let _ = unlinkat(dir, staged.as_str(), AtFlags::empty());
    put?;
    hook(Step::Undone, dir, name)?;
    Ok(fsync(dir)?)
}

/// A seal of `name` in `dir` cut short before it was final, undone: the
/// record that stood, still under its prior name, put back over `name` by
/// the same exchange, or, where both names are already that one file (a
/// seal cut short between its link and its exchange), the second name
/// removed; and the directory synced. Where the name is gone, the removal
/// stands: the record that stood is removed, and nothing is written at the
/// name. Nothing where no prior name stands; refused where it cannot be
/// undone.
fn restored(dir: &OwnedFd, name: &str) -> Result<(), Unknown> {
    let prior = format!("{name}{PRIOR}");
    let staged = format!("{name}{STAGED}");
    let Some((_, _, kept)) = regular_at(dir, &prior) else {
        return Ok(());
    };
    let one = regular_at(dir, name).is_some_and(|(_, _, at)| at == kept);
    let put = if one {
        unlinkat(dir, prior.as_str(), AtFlags::empty())
    } else if regular_at(dir, name).is_some() {
        renameat_with(dir, prior.as_str(), dir, name, RenameFlags::EXCHANGE)
            .and_then(|()| unlinkat(dir, prior.as_str(), AtFlags::empty()))
    } else {
        // The name is gone: the removal stands. Remove the record that
        // stood; write nothing at the name.
        unlinkat(dir, prior.as_str(), AtFlags::empty())
    };
    // A staged copy a cut-short seal left is the successor that was never
    // exchanged in; it goes with the undo.
    let _ = unlinkat(dir, staged.as_str(), AtFlags::empty());
    put.map_err(std::io::Error::from)
        .or(Err(Unknown::Unsealed))?;
    fsync(dir).or(Err(Unknown::Unsealed))
}
