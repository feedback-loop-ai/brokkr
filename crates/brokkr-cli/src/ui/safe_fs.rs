//! The narrow, handle-based local filesystem helper for the transcript
//! reader (proposed decision 0055, design D3).
//!
//! Every descendant below the canonical recorded home is opened one
//! component at a time relative to a held directory handle, without
//! following symlinks or reparse points. Enumeration happens through the
//! held handle, and a candidate's verified file handle is retained for
//! the body read, so a name swapped after validation cannot redirect the
//! bytes. This is deliberately small: it serves the local transcript
//! reader and nothing else.

use std::ffi::OsStr;
use std::io;

/// Stable device/inode identity of an opened handle.
///
/// Signed `i128` fields losslessly carry every supported target's native
/// device/inode or volume/file-index values. A native value that cannot be
/// widened is a refusal, never a wrap or a narrowing (design D3).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Identity {
    pub device: i128,
    pub inode: i128,
}

/// Widen one native identity value into its lossless `i128` spelling.
fn widen<T>(value: T) -> io::Result<i128>
where
    T: TryInto<i128>,
{
    value.try_into().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            "filesystem identity does not widen losslessly",
        )
    })
}

/// One direct child opened from a held directory handle.
pub(crate) enum Child {
    /// A regular file, opened without following a symlink.
    File(OpenedFile),
    /// A directory, opened without following a symlink.
    Dir(Dir),
    /// Present but not a regular file or directory (symlink, FIFO, …).
    Unsafe,
    /// Gone between enumeration and open.
    Absent,
}

/// A held directory handle.
pub(super) struct Dir {
    inner: imp::Dir,
}

impl Dir {
    /// Open an already-canonicalized absolute directory path as the
    /// traversal root. The root itself may be reached through symlinks
    /// because canonicalization resolved them first.
    pub(super) fn open_root(path: &str) -> io::Result<Dir> {
        Ok(Dir {
            inner: imp::Dir::open_root(path)?,
        })
    }

    /// Enumerate at most `max` names through the held handle, dot entries
    /// dropped. Returns `(names, truncated)`: `truncated` is true when the
    /// directory held more entries than `max`, so the caller can charge its
    /// discovery bound without materializing an unbounded listing.
    pub(super) fn entries_bounded(
        &self,
        max: usize,
    ) -> io::Result<(Vec<std::ffi::OsString>, bool)> {
        #[cfg(test)]
        {
            if let Some(error) = fault::fail(fault::FailAt::Entries) {
                return Err(error);
            }
        }
        self.inner.entries_bounded(max)
    }

    /// Open one direct child, following no symlink or reparse point.
    pub(super) fn child(&self, name: &OsStr) -> io::Result<Child> {
        #[cfg(test)]
        {
            fault::point(fault::ChangeAt::Child);
        }
        Ok(match self.inner.child(name)? {
            imp::Child::File(file) => Child::File(OpenedFile { inner: file }),
            imp::Child::Dir(dir) => Child::Dir(Dir { inner: dir }),
            imp::Child::Unsafe => Child::Unsafe,
            imp::Child::Absent => Child::Absent,
        })
    }
}

/// A held regular-file handle with its verified identity.
pub(crate) struct OpenedFile {
    inner: imp::File,
}

impl OpenedFile {
    pub(crate) fn identity(&self) -> io::Result<Identity> {
        #[cfg(test)]
        {
            if let Some(error) = fault::fail(fault::FailAt::Identity) {
                return Err(error);
            }
        }
        self.inner.identity()
    }

    /// The source's size in bytes, measured through the held handle.
    pub(crate) fn len(&self) -> u64 {
        self.inner.len()
    }

    /// Read at most `cap` bytes plus one probe byte from the start of the
    /// file. Returns `(bytes, overflow, eof)`: `overflow` means the probe
    /// byte was present, `eof` means the read reached true end of file.
    pub(crate) fn read_bounded(&self, cap: u64) -> io::Result<(Vec<u8>, bool, bool)> {
        #[cfg(test)]
        {
            if let Some(error) = fault::fail(fault::FailAt::Read) {
                return Err(error);
            }
        }
        self.inner.read_bounded(cap)
    }
}

mod imp {
    use super::{widen, Identity};
    use rustix::fs::{fstat, openat, Dir as RDir, FileType, Mode, OFlags, CWD};
    use std::ffi::{OsStr, OsString};
    use std::io::{self, Read, Seek};
    use std::os::unix::ffi::OsStrExt;

    pub(super) enum Child {
        File(File),
        Dir(Dir),
        Unsafe,
        Absent,
    }

    pub(super) struct Dir {
        fd: std::os::fd::OwnedFd,
    }

    pub(super) struct File {
        fd: std::os::fd::OwnedFd,
    }

    fn identity_of(fd: &impl std::os::fd::AsFd) -> io::Result<Identity> {
        let stat = fstat(fd)?;
        Ok(Identity {
            device: widen(stat.st_dev)?,
            inode: widen(stat.st_ino)?,
        })
    }

    fn dir_flags() -> OFlags {
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
    }

    fn file_flags() -> OFlags {
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC
    }

    fn absent(error: &rustix::io::Errno) -> bool {
        matches!(*error, rustix::io::Errno::NOENT)
    }

    impl Dir {
        pub(super) fn open_root(path: &str) -> io::Result<Dir> {
            let fd = openat(CWD, path, dir_flags(), Mode::empty())?;
            Ok(Dir { fd })
        }

        pub(super) fn entries_bounded(&self, max: usize) -> io::Result<(Vec<OsString>, bool)> {
            let mut names = Vec::new();
            for entry in RDir::read_from(&self.fd)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = OsStr::from_bytes(name.to_bytes()).to_os_string();
                if name == OsStr::new(".") || name == OsStr::new("..") {
                    continue;
                }
                if names.len() >= max {
                    // One more eligible entry exists beyond the budget.
                    return Ok((names, true));
                }
                names.push(name);
            }
            Ok((names, false))
        }

        pub(super) fn child(&self, name: &OsStr) -> io::Result<Child> {
            match openat(&self.fd, name, dir_flags(), Mode::empty()) {
                Ok(fd) => return Ok(Child::Dir(Dir { fd })),
                Err(error) if absent(&error) => return Ok(Child::Absent),
                // Any other directory-open failure falls through to the
                // file open, which classifies the same child (and yields
                // `Unsafe` for a symlink, FIFO or other non-regular node).
                Err(_) => {}
            }
            // The point between the directory attempt and the file attempt.
            // It runs before the real file open, which then classifies what
            // it finds; a test-owned change is timed here.
            #[cfg(test)]
            super::fault::point(super::fault::ChangeAt::ChildFileAttempt);
            match openat(&self.fd, name, file_flags(), Mode::empty()) {
                Ok(fd) => {
                    let stat = fstat(&fd)?;
                    if FileType::from_raw_mode(stat.st_mode).is_file() {
                        Ok(Child::File(File { fd }))
                    } else {
                        Ok(Child::Unsafe)
                    }
                }
                Err(error) if absent(&error) => Ok(Child::Absent),
                Err(rustix::io::Errno::LOOP)
                | Err(rustix::io::Errno::MLINK)
                | Err(rustix::io::Errno::ISDIR) => Ok(Child::Unsafe),
                Err(error) => Err(io::Error::from(error)),
            }
        }
    }

    impl File {
        pub(super) fn identity(&self) -> io::Result<Identity> {
            identity_of(&self.fd)
        }

        pub(super) fn len(&self) -> u64 {
            fstat(&self.fd)
                .map(|stat| stat.st_size.max(0) as u64)
                .unwrap_or(0)
        }

        pub(super) fn read_bounded(&self, cap: u64) -> io::Result<(Vec<u8>, bool, bool)> {
            // A prior header check on this handle may have consumed bytes;
            // read from the start so the body snapshot is deterministic.
            let mut file = std::fs::File::from(self.fd.try_clone()?);
            file.rewind()?;
            let mut buffer = Vec::new();
            let mut reader = std::io::Read::take(&mut file, cap.saturating_add(1));
            reader.read_to_end(&mut buffer)?;
            let overflow = buffer.len() as u64 > cap;
            if overflow {
                buffer.truncate(cap as usize);
            }
            Ok((buffer, overflow, !overflow))
        }
    }
}

/// A unit-test-only fault seam at the handle-based reader boundary.
///
/// It exists only under `#[cfg(test)]`, so no release binary, package or
/// integration-test build compiles it and a reference to it in such a build
/// is a compile error. It reads no environment variable, argument,
/// configuration key, file or journal value, and it never builds a `Child`,
/// `Identity`, handle, path or name. A plan names one target and the
/// occurrence at which it fires on the installing thread; an entry that never
/// fires fails its test, and no entry can be disarmed. See the change
/// `prove-transcript-reader-faults` D2-D8 and proposed decision 0055's
/// addendum.
#[cfg(test)]
pub(crate) mod fault {
    use std::cell::RefCell;
    use std::io;
    use std::marker::PhantomData;

    /// The operations whose result a scripted error replaces. Child open and
    /// the two timed points accept no error (the change's S8), so they are a
    /// disjoint enum.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) enum FailAt {
        Entries,
        Identity,
        Read,
    }

    /// The points at which a test-owned filesystem change runs before the
    /// reader's next real operation.
    #[derive(Clone, Copy, PartialEq, Eq, Debug)]
    pub(crate) enum ChangeAt {
        Child,
        ChildFileAttempt,
        BeforeRewalk,
    }

    enum Entry {
        Fail {
            target: FailAt,
            occurrence: usize,
            fired: bool,
        },
        Change {
            target: ChangeAt,
            occurrence: usize,
            action: Option<Box<dyn FnOnce()>>,
            fired: bool,
        },
    }

    impl Entry {
        fn fired(&self) -> bool {
            match self {
                Entry::Fail { fired, .. } | Entry::Change { fired, .. } => *fired,
            }
        }

        fn set_fired(&mut self) {
            match self {
                Entry::Fail { fired, .. } | Entry::Change { fired, .. } => *fired = true,
            }
        }

        fn matches_fail(&self, target: FailAt, occurrence: usize) -> bool {
            matches!(
                self,
                Entry::Fail {
                    target: entry_target,
                    occurrence: entry_occurrence,
                    ..
                } if *entry_target == target && *entry_occurrence == occurrence
            )
        }

        /// Mark the matching `Change` entry as fired and take its action.
        /// Any other entry, including a `Fail` entry or a different target
        /// or occurrence, is `None`.
        ///
        /// The `fired` flag is deliberately not part of the match guard:
        /// the occurrence counter visits each `(target, occurrence)` at most
        /// once per plan, so a matching entry can never already be fired.
        /// Guarding on `!fired` would add a branch no input can reach, which
        /// the exact-coverage gate forbids (spec: unreachable handling is
        /// removed with its proof, which is
        /// `every_visit_to_a_timed_point_is_a_new_occurrence` in
        /// `ui/tests.rs`). An already-fired entry's action is
        /// `None`, so `action.take()` still yields `None` if it were ever
        /// revisited, and a duplicate entry at one occurrence still never
        /// fires because `find_map` stops at the first match and the guard's
        /// unfired check then fails the test.
        fn take_change_action(
            &mut self,
            target: ChangeAt,
            occurrence: usize,
        ) -> Option<Box<dyn FnOnce()>> {
            match self {
                Entry::Change {
                    target: entry_target,
                    occurrence: entry_occurrence,
                    action,
                    fired,
                } if *entry_target == target && *entry_occurrence == occurrence => {
                    *fired = true;
                    action.take()
                }
                _ => None,
            }
        }

        /// Name an unfired entry for the guard's panic message.
        fn describe(&self) -> String {
            match self {
                Entry::Fail {
                    target, occurrence, ..
                } => {
                    format!("{target:?} occurrence {occurrence}")
                }
                Entry::Change {
                    target, occurrence, ..
                } => {
                    format!("{target:?} occurrence {occurrence}")
                }
            }
        }
    }

    #[derive(Default)]
    struct Counts {
        entries: usize,
        identity: usize,
        read: usize,
        child: usize,
        child_file_attempt: usize,
        before_rewalk: usize,
    }

    impl Counts {
        fn bump_fail(&mut self, target: FailAt) -> usize {
            let slot = match target {
                FailAt::Entries => &mut self.entries,
                FailAt::Identity => &mut self.identity,
                FailAt::Read => &mut self.read,
            };
            *slot += 1;
            *slot
        }

        fn bump_change(&mut self, target: ChangeAt) -> usize {
            let slot = match target {
                ChangeAt::Child => &mut self.child,
                ChangeAt::ChildFileAttempt => &mut self.child_file_attempt,
                ChangeAt::BeforeRewalk => &mut self.before_rewalk,
            };
            *slot += 1;
            *slot
        }
    }

    struct State {
        entries: Vec<Entry>,
        counts: Counts,
    }

    thread_local! {
        static PLAN: RefCell<Option<State>> = const { RefCell::new(None) };
    }

    /// An append-only plan of scripted entries. It can be added to, never
    /// trimmed: there is no `clear`, `skip`, `disarm`, `optional` or
    /// `allow_unfired` method.
    pub(crate) struct Plan {
        entries: Vec<Entry>,
    }

    impl Plan {
        pub(crate) fn new() -> Plan {
            Plan {
                entries: Vec::new(),
            }
        }

        /// Script an injected I/O error at the given occurrence of one
        /// enumeration, identity or bounded-read operation.
        pub(crate) fn fail(mut self, target: FailAt, occurrence: usize) -> Plan {
            self.entries.push(Entry::Fail {
                target,
                occurrence,
                fired: false,
            });
            self
        }

        /// Script a test-owned filesystem change to run at the given
        /// occurrence of one child open or timed point, before the reader's
        /// next real operation.
        pub(crate) fn change(
            mut self,
            target: ChangeAt,
            occurrence: usize,
            action: impl FnOnce() + 'static,
        ) -> Plan {
            self.entries.push(Entry::Change {
                target,
                occurrence,
                action: Some(Box::new(action)),
                fired: false,
            });
            self
        }

        /// Install the plan on the calling thread and return a guard that
        /// clears it and fails the test if any entry never fired. A second
        /// install on the same thread is refused.
        pub(crate) fn install(self) -> Guard {
            PLAN.with(|cell| {
                let mut slot = cell.borrow_mut();
                assert!(
                    slot.is_none(),
                    "a fault plan is already installed on this thread"
                );
                *slot = Some(State {
                    entries: self.entries,
                    counts: Counts::default(),
                });
            });
            Guard {
                _not_send: PhantomData,
            }
        }
    }

    /// An opaque guard. It is `!Send`, so it always drops on the installing
    /// thread, and its `Drop` always takes the plan out of the thread-local.
    pub(crate) struct Guard {
        _not_send: PhantomData<*const ()>,
    }

    impl Drop for Guard {
        #[expect(clippy::excessive_nesting, reason = "baseline 2026-09, #288")]
        fn drop(&mut self) {
            PLAN.with(|cell| {
                let mut slot = cell.borrow_mut();
                // The guard exists only after a successful install on this
                // thread, and `install` refuses a second live plan, so the
                // thread-local always holds this guard's plan. `inspect`
                // leaves the "no plan" arm in the standard library instead
                // of adding an unreachable branch to the seam's own code.
                let _ = slot.take().inspect(|state| {
                    // A panic inside `Drop` while unwinding aborts the whole
                    // test binary, so the plan is cleared and the check
                    // skipped when the test is already failing.
                    if std::thread::panicking() {
                        return;
                    }
                    let unfired: Vec<String> = state
                        .entries
                        .iter()
                        .filter(|entry| !entry.fired())
                        .map(Entry::describe)
                        .collect();
                    assert!(
                        unfired.is_empty(),
                        "fault seam entries never fired: {}",
                        unfired.join(", ")
                    );
                });
            });
        }
    }

    /// The injected-error hook. Returns an error exactly when the plan holds
    /// an unfired entry for this target at its next occurrence; otherwise
    /// `None`, including when no plan is installed.
    pub(crate) fn fail(target: FailAt) -> Option<io::Error> {
        PLAN.with(|cell| {
            let mut slot = cell.borrow_mut();
            let state = slot.as_mut()?;
            let occurrence = state.counts.bump_fail(target);
            let entry = state
                .entries
                .iter_mut()
                .find(|entry| !entry.fired() && entry.matches_fail(target, occurrence))?;
            entry.set_fired();
            Some(io::Error::other("scripted reader fault"))
        })
    }

    /// The timed-change hook. Runs the matching entry's closure once, after
    /// releasing the thread-local borrow, and is a no-op when no entry
    /// matches or no plan is installed.
    pub(crate) fn point(target: ChangeAt) {
        let action = PLAN.with(|cell| {
            let mut slot = cell.borrow_mut();
            let state = slot.as_mut()?;
            let occurrence = state.counts.bump_change(target);
            state
                .entries
                .iter_mut()
                .find_map(|entry| entry.take_change_action(target, occurrence))
        });
        if let Some(action) = action {
            action();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::widen;

    /// Signed and unsigned native identity values widen losslessly, and a
    /// value that cannot be represented is refused rather than wrapped.
    #[test]
    fn identity_widening_is_lossless_and_checked() {
        assert_eq!(widen(0u64).unwrap(), 0);
        assert_eq!(widen(u64::MAX).unwrap(), u64::MAX as i128);
        assert_eq!(widen(-1i64).unwrap(), -1);
        assert_eq!(widen(i64::MIN).unwrap(), i64::MIN as i128);
        assert_eq!(widen(u32::MAX).unwrap(), u32::MAX as i128);
        assert_eq!(widen(i32::MIN).unwrap(), i32::MIN as i128);
        assert_eq!(widen(0i8).unwrap(), 0);
        assert!(
            widen(u128::MAX).is_err(),
            "a value beyond the lossless range is refused"
        );
    }
}
