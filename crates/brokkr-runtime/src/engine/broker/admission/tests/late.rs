//! The seal hook every admission test's root carries: what the next seal on
//! this thread does at one of its final steps (H1; R1 of run four's
//! review), and what a reseal meets when the record it would replace is
//! removed or changed inside its window ([`Late::Remove`],
//! [`Late::Replace`]; #440 fix round 440d5). Moved out of `tests.rs`, which
//! this would take past its 2,000 lines (decision 0071 ruling 4).

use std::cell::Cell;
use std::os::fd::OwnedFd;
use std::time::Duration;

use rustix::fs::AtFlags;

use crate::engine::broker::admission::STAGED;
use crate::engine::broker::root::{Step, PRIOR};

/// What the next seal on a root this thread opened does at its final steps
/// (H1; R1 of run four's review): before its commit, spends the time given,
/// removes its staged copy so its commit fails, plants a file at the
/// record's prior name so it cannot link the record that stood there while
/// the rename after the link stays possible (R3, U8a2c), or removes or
/// replaces the record that stood at its name, so a reseal meets it gone or
/// changed ([`Late::Remove`], [`Late::Replace`]; #440 fix round 440d5), or
/// removes it after the link and before the exchange, or after the exchange
/// and before the directory is synced ([`Late::RemoveLinked`],
/// [`Late::RemoveCommitted`]; #440 fix round 440d6);
/// once renamed in, spends the time given, fails as its directory sync
/// failing would, or spends it with the directory's write permission taken,
/// or spends it and then fails the sync of its undoing (M2 of run four's
/// second review); as it retires the record that stood, spends the time
/// given or fails as the removal of its prior name failing would; and once
/// committed, fails as its last directory sync failing would (R1, U8a2c).
#[derive(Debug, Clone, Copy)]
pub(super) enum Late {
    Sleep(Duration),
    Unstage,
    UnstageSuperseded,
    Occupy,
    Remove,
    RemoveLinked,
    RemoveRestored,
    Replace,
    RemoveCommitted,
    ReplaceCommitted,
    CutUndo,
    FailExchanged,
    SleepCommitted(Duration),
    FailCommitted,
    DenyCommitted(Duration),
    FailUndone(Duration),
    Undoing,
    SleepRetiring(Duration),
    FailRetiring,
    FailFinal,
}

thread_local! {
    /// What the next seal does before its commit, once ([`late`]).
    pub(super) static LATE: Cell<Option<Late>> = const { Cell::new(None) };
}

/// The hook of every root a test opens: what [`LATE`] arms, at the step of
/// the next seal on this thread it names, and then nothing.
pub(super) fn late(step: Step, dir: &OwnedFd, staged: &str) -> std::io::Result<()> {
    match (step, LATE.take()) {
        (Step::Staged, Some(Late::Sleep(spent)))
        | (Step::Committed, Some(Late::SleepCommitted(spent)))
        | (Step::Retiring, Some(Late::SleepRetiring(spent))) => std::thread::sleep(spent),
        (Step::Retiring, Some(Late::FailRetiring)) => {
            return Err(std::io::Error::other("the prior name could not be removed"))
        }
        (Step::Staged, Some(Late::Unstage)) => {
            rustix::fs::unlinkat(dir, staged, AtFlags::empty()).unwrap()
        }
        (Step::Staged, Some(Late::UnstageSuperseded)) if staged.starts_with("superseded-") => {
            rustix::fs::unlinkat(dir, staged, AtFlags::empty()).unwrap()
        }
        (Step::Staged, Some(Late::Occupy)) => occupied(dir, staged),
        (Step::Staged, Some(Late::Remove)) => {
            let name = staged.strip_suffix(STAGED).unwrap();
            rustix::fs::unlinkat(dir, name, AtFlags::empty()).unwrap()
        }
        (Step::Staged, Some(Late::Replace)) => {
            let name = staged.strip_suffix(STAGED).unwrap();
            rustix::fs::unlinkat(dir, name, AtFlags::empty()).unwrap();
            empty(dir, name);
        }
        // The last point before a reseal's exchange: the name is removed
        // after the record that stood was linked, so the exchange must meet
        // it gone and write nothing (#440 fix round 440d6).
        (Step::Linked, Some(Late::RemoveLinked)) => {
            rustix::fs::unlinkat(dir, staged, AtFlags::empty()).unwrap()
        }
        // A restore's last point before its exchange: the name is removed,
        // so the exchange must meet it gone and write nothing (#440 fix
        // round 440d6).
        (Step::Restored, Some(Late::RemoveRestored)) => {
            rustix::fs::unlinkat(dir, staged, AtFlags::empty()).unwrap()
        }
        // After a reseal's exchange: the name is removed, and the sync that
        // follows fails, so the undo must not write the old record back
        // (#440 fix round 440d6).
        (Step::Committed, Some(Late::RemoveCommitted)) => {
            let name = staged.strip_suffix(STAGED).unwrap();
            rustix::fs::unlinkat(dir, name, AtFlags::empty()).unwrap();
            return Err(std::io::Error::other("the directory sync failed"));
        }
        // The directory's sync after the commit fails, and the undo that
        // follows is cut short after its exchange, as a kill there would cut
        // it (#440 fix round 440d6).
        (Step::Committed, Some(Late::CutUndo)) => {
            LATE.set(Some(Late::FailExchanged));
            return Err(std::io::Error::other("the directory sync failed"));
        }
        // After a reseal's exchange: a file a hand made stands at the name,
        // and the sync that follows fails, so the undo must leave it there
        // (#440 fix round 440d6).
        (Step::Committed, Some(Late::ReplaceCommitted)) => {
            let name = staged.strip_suffix(STAGED).unwrap();
            rustix::fs::unlinkat(dir, name, AtFlags::empty()).unwrap();
            empty(dir, name);
            return Err(std::io::Error::other("the directory sync failed"));
        }
        // An undo's exchange has put the record that stood back at its
        // name, before the successor it swapped out is removed: the seal is
        // cut short there, as a kill would cut it (#440 fix round 440d6).
        (Step::Exchanged, Some(Late::FailExchanged)) => {
            return Err(std::io::Error::other(
                "the undo was cut short after its exchange",
            ))
        }
        (Step::Committed, Some(Late::DenyCommitted(spent))) => {
            denied(dir);
            std::thread::sleep(spent)
        }
        (Step::Committed, Some(Late::FailUndone(spent))) => {
            LATE.set(Some(Late::Undoing));
            std::thread::sleep(spent)
        }
        (Step::Committed, Some(Late::FailCommitted))
        | (Step::Undone, Some(Late::Undoing))
        | (Step::Final, Some(Late::FailFinal)) => {
            return Err(std::io::Error::other("the directory sync failed"))
        }
        (_, armed) => LATE.set(armed),
    }
    Ok(())
}

/// An empty file of this user's own made at the prior name of the record
/// `staged` stages in `dir`: a link of the record that stood to that name
/// fails, and the directory stays writable.
fn occupied(dir: &OwnedFd, staged: &str) {
    let name = staged.strip_suffix(STAGED).unwrap();
    empty(dir, &format!("{name}{PRIOR}"));
}

/// An empty file of this user's own made at `name` in `dir`.
fn empty(dir: &OwnedFd, name: &str) {
    let flags = rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CREATE | rustix::fs::OFlags::EXCL;
    let mode = rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR;
    drop(rustix::fs::openat(dir, name, flags, mode).unwrap());
}

/// The directory `dir` left readable and searchable, but not writable.
fn denied(dir: &OwnedFd) {
    let kept = rustix::fs::Mode::RUSR | rustix::fs::Mode::XUSR;
    rustix::fs::fchmod(dir, kept).unwrap()
}

thread_local! {
    /// The error [`refusing_exchange`] reports on this thread, `EINVAL`
    /// until a test asks another.
    static REFUSAL: Cell<rustix::io::Errno> = const { Cell::new(rustix::io::Errno::INVAL) };
}

/// A filesystem's exchange refusal, reporting [`REFUSAL`]: `EINVAL`,
/// `ENOTSUP`, `EOPNOTSUPP` or `ENOSYS`, the four ways a filesystem says it
/// has no atomic exchange (decision 0108 ruling 3, #440 fix round 440d6).
pub(super) fn refusing_exchange(
    _: &std::os::fd::OwnedFd,
    _: &str,
    _: &std::os::fd::OwnedFd,
    _: &str,
) -> Result<(), rustix::io::Errno> {
    Err(REFUSAL.with(Cell::get))
}

/// The error the next [`refusing_exchange`] reports.
pub(super) fn refusing(error: rustix::io::Errno) {
    REFUSAL.with(|refusal| refusal.set(error));
}

thread_local! {
    /// How many exchanges [`failing_second_exchange`] has refused, the
    /// second and later, on this thread.
    pub(super) static SWAPS: Cell<usize> = const { Cell::new(0) };
}

/// Production's exchange for the first swap on this thread, and `EINVAL`
/// after it: a reseal whose foreign file cannot be exchanged back, which is
/// what its swap after the proof must handle (decision 0108 ruling 3, #440
/// fix round 440d6).
pub(super) fn failing_second_exchange(
    one: &std::os::fd::OwnedFd,
    a: &str,
    two: &std::os::fd::OwnedFd,
    b: &str,
) -> Result<(), rustix::io::Errno> {
    let swaps = SWAPS.with(|swaps| {
        let next = swaps.get() + 1;
        swaps.set(next);
        next
    });
    if swaps == 1 {
        rustix::fs::renameat_with(one, a, two, b, rustix::fs::RenameFlags::EXCHANGE)
    } else {
        Err(rustix::io::Errno::INVAL)
    }
}

/// [`failing_second_exchange`] counts from one again.
pub(super) fn swaps_again() {
    SWAPS.with(|swaps| swaps.set(0));
}
