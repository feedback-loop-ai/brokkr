//! The seal hook every admission test's root carries: what the next seal on
//! this thread does at one of its final steps (H1; R1 of run four's
//! review), and what a reseal meets when the record it would replace is
//! removed or changed inside its window ([`Late::Remove`],
//! [`Late::Replace`]; #440 fix round 440d5). Moved out of `tests.rs`, which
//! this would take past its 2,000 lines (decision 0071 ruling 4).

use std::cell::Cell;
use std::io::Write;
use std::os::fd::OwnedFd;
use std::time::Duration;

use rustix::fs::{AtFlags, Mode};

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
/// [`Late::RemoveCommitted`]; #440 fix round 440d6), or removes the record
/// that stood from its prior name, as the directory's sync fails after the
/// commit or as the seal retires it ([`Late::RemovePriorCommitted`],
/// [`Late::RemovePriorRetiring`]; #1099), or replaces that prior name
/// with a directory as the seal retires it ([`Late::PriorDirectoryRetiring`]);
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
    FailStaged,
    UnstageSuperseded,
    Occupy,
    Remove,
    RemoveLinked,
    RemoveRestored,
    Replace,
    RemoveCommitted,
    ReplaceCommitted,
    RemovePriorCommitted,
    RemovePriorRetiring,
    PriorDirectoryRetiring,
    CutUndo,
    FailLinked,
    FailRestored,
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
        (Step::Staged, Some(Late::FailStaged)) => {
            return Err(std::io::Error::other(
                "the staged copy could not be written",
            ))
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
        // The link of the record that stood made, and the step that proves
        // it failed: nothing is exchanged (#440 fix round 440d6).
        (Step::Linked, Some(Late::FailLinked)) => {
            return Err(std::io::Error::other(
                "the linked record could not be proven",
            ))
        }
        // A restore's last point before its exchange: the name is removed,
        // so the exchange must meet it gone and write nothing (#440 fix
        // round 440d6).
        (Step::Restored, Some(Late::RemoveRestored)) => {
            rustix::fs::unlinkat(dir, staged, AtFlags::empty()).unwrap()
        }
        // The restore's link of the record that stood made, and the step
        // that proves it failed: nothing is exchanged (#440 fix round 440d6).
        (Step::Restored, Some(Late::FailRestored)) => {
            return Err(std::io::Error::other(
                "the restored record could not be proven",
            ))
        }
        // After a reseal's exchange: the name is removed, and the sync that
        // follows fails, so the undo must not write the old record back
        // (#440 fix round 440d6).
        (Step::Committed, Some(Late::RemoveCommitted)) => {
            let name = staged.strip_suffix(STAGED).unwrap();
            rustix::fs::unlinkat(dir, name, AtFlags::empty()).unwrap();
            return Err(std::io::Error::other("the directory sync failed"));
        }
        // After a seal's commit: a hand removes the record that stood from
        // its prior name, and the sync that follows fails, so the undo finds
        // no prior name to put back (#1099).
        (Step::Committed, Some(Late::RemovePriorCommitted)) => {
            let name = staged.strip_suffix(STAGED).unwrap();
            remove_prior(dir, name);
            return Err(std::io::Error::other("the directory sync failed"));
        }
        // As the seal retires the record that stood: a hand removes its
        // prior name first, so the retirement meets it gone (#1099). This
        // step is handed the record's own name.
        (Step::Retiring, Some(Late::RemovePriorRetiring)) => remove_prior(dir, staged),
        // As the seal retires the record that stood: a hand replaces its
        // prior name with a directory, so the retirement cannot remove it and
        // the undo's rename back meets something other than a gone name
        // (#1099, the first seal's ENOTDIR arm).
        (Step::Retiring, Some(Late::PriorDirectoryRetiring)) => {
            remove_prior(dir, staged);
            rustix::fs::mkdirat(dir, format!("{staged}{PRIOR}"), Mode::RWXU).unwrap();
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

/// The prior name of the record `name` in `dir` removed, as a hand would.
fn remove_prior(dir: &OwnedFd, name: &str) {
    rustix::fs::unlinkat(dir, format!("{name}{PRIOR}"), AtFlags::empty()).unwrap()
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

/// How many exchanges [`failing_second_exchange`] has made on this thread.
pub(super) fn swaps() -> usize {
    SWAPS.with(|swaps| {
        let next = swaps.get() + 1;
        swaps.set(next);
        next
    })
}

/// Production's exchange for the first swap on this thread, and `after` for
/// every later one: a reseal whose proof fails must handle a swap it cannot
/// make (decision 0108 ruling 3, #440 fix round 440d6).
fn after_first(
    one: &std::os::fd::OwnedFd,
    a: &str,
    two: &std::os::fd::OwnedFd,
    b: &str,
    after: fn(
        &std::os::fd::OwnedFd,
        &str,
        &std::os::fd::OwnedFd,
        &str,
    ) -> Result<(), rustix::io::Errno>,
) -> Result<(), rustix::io::Errno> {
    if swaps() == 1 {
        return rustix::fs::renameat_with(one, a, two, b, rustix::fs::RenameFlags::EXCHANGE);
    }
    after(one, a, two, b)
}

/// A directory in place of the record's name: the exchange back fails, and
/// nothing is renamed over the name in its place.
fn name_a_directory(
    _: &std::os::fd::OwnedFd,
    _: &str,
    two: &std::os::fd::OwnedFd,
    b: &str,
) -> Result<(), rustix::io::Errno> {
    let _ = rustix::fs::unlinkat(two, b, AtFlags::empty());
    let _ = rustix::fs::mkdirat(two, b, rustix::fs::Mode::RWXU);
    Err(rustix::io::Errno::INVAL)
}

/// A directory in place of the staged name: the exchange back fails while
/// the name still holds this seal's successor.
fn staged_a_directory(
    one: &std::os::fd::OwnedFd,
    a: &str,
    _: &std::os::fd::OwnedFd,
    _: &str,
) -> Result<(), rustix::io::Errno> {
    let _ = rustix::fs::unlinkat(one, a, AtFlags::empty());
    let _ = rustix::fs::mkdirat(one, a, rustix::fs::Mode::RWXU);
    Err(rustix::io::Errno::INVAL)
}

/// A hand's regular file of `bytes` at `name` in `dir`, replacing what
/// stands there.
fn a_hands_file(dir: &OwnedFd, name: &str, bytes: &[u8]) {
    let _ = rustix::fs::unlinkat(dir, name, AtFlags::empty());
    let flags = rustix::fs::OFlags::RDWR
        | rustix::fs::OFlags::CREATE
        | rustix::fs::OFlags::EXCL
        | rustix::fs::OFlags::CLOEXEC;
    let mode = rustix::fs::Mode::RUSR | rustix::fs::Mode::WUSR;
    let file = rustix::fs::openat(dir, name, flags, mode).unwrap();
    std::fs::File::from(file).write_all(bytes).unwrap();
}

/// The second swap removes the staged name and makes it a directory, and
/// puts a hand's regular file at the record's name: the exchange back fails
/// and neither name holds this seal's successor, so nothing is unlinked
/// from the name unless it is proven this seal's own.
fn staged_a_directory_over_a_hands_file(
    one: &OwnedFd,
    a: &str,
    two: &OwnedFd,
    b: &str,
) -> Result<(), rustix::io::Errno> {
    let _ = rustix::fs::unlinkat(one, a, AtFlags::empty());
    let _ = rustix::fs::mkdirat(one, a, rustix::fs::Mode::RWXU);
    a_hands_file(two, b, b"a hand's file");
    Err(rustix::io::Errno::INVAL)
}

/// Production's exchange until the `at`th swap on this thread, and at that
/// swap the name is removed first, so the exchange meets it gone.
fn unlinking_at(
    at: usize,
    one: &OwnedFd,
    a: &str,
    two: &OwnedFd,
    b: &str,
) -> Result<(), rustix::io::Errno> {
    if swaps() < at {
        return rustix::fs::renameat_with(one, a, two, b, rustix::fs::RenameFlags::EXCHANGE);
    }
    let _ = rustix::fs::unlinkat(two, b, AtFlags::empty());
    rustix::fs::renameat_with(one, a, two, b, rustix::fs::RenameFlags::EXCHANGE)
}

/// A reseal's exchange back meets a name a hand removed at the second swap:
/// the removal stands, and no rename makes the name again.
pub(super) fn unlinking_before_second_exchange(
    one: &OwnedFd,
    a: &str,
    two: &OwnedFd,
    b: &str,
) -> Result<(), rustix::io::Errno> {
    unlinking_at(2, one, a, two, b)
}

/// An undo's exchange back meets a name a hand removed at the third swap:
/// the removal stands, and no rename makes the name again.
pub(super) fn unlinking_before_third_exchange(
    one: &OwnedFd,
    a: &str,
    two: &OwnedFd,
    b: &str,
) -> Result<(), rustix::io::Errno> {
    unlinking_at(3, one, a, two, b)
}

/// Production's exchange for the first two swaps on this thread, and
/// `EINVAL` for the third and later: an undo's exchange back, for a file a
/// hand planted at the name, fails for a reason other than a removal (#1099
/// R2).
pub(super) fn failing_third_exchange(
    one: &OwnedFd,
    a: &str,
    two: &OwnedFd,
    b: &str,
) -> Result<(), rustix::io::Errno> {
    if swaps() < 3 {
        return rustix::fs::renameat_with(one, a, two, b, rustix::fs::RenameFlags::EXCHANGE);
    }
    Err(rustix::io::Errno::INVAL)
}

/// One exchange seam: production's swap for the first call on this thread,
/// and `$after` for every later one.
macro_rules! seam {
    ($name:ident, $after:expr) => {
        pub(super) fn $name(
            one: &std::os::fd::OwnedFd,
            a: &str,
            two: &std::os::fd::OwnedFd,
            b: &str,
        ) -> Result<(), rustix::io::Errno> {
            after_first(one, a, two, b, $after)
        }
    };
}

seam!(failing_second_exchange, |_, _, _, _| Err(
    rustix::io::Errno::INVAL
));
seam!(failing_second_exchange_over_the_name, name_a_directory);
seam!(failing_second_exchange_over_the_staged, staged_a_directory);
seam!(
    failing_second_exchange_over_the_staged_and_the_hand,
    staged_a_directory_over_a_hands_file
);

/// [`failing_second_exchange`] counts from one again.
pub(super) fn swaps_again() {
    SWAPS.with(|swaps| swaps.set(0));
}
