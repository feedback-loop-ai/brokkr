//! POSIX `openat(2)` helpers anchoring `FsEventLog` child opens to verified
//! directory descriptors with `O_NOFOLLOW | O_CLOEXEC` and zero extra crate
//! dependencies.

use std::ffi::CString;
use std::fs::File;
use std::io;
use std::os::raw::{c_char, c_int, c_uint};
use std::os::unix::io::{AsRawFd, FromRawFd};
use std::path::Path;

use crate::StoreError;

pub(crate) const O_RDONLY: c_int = 0;
pub(crate) const O_WRONLY: c_int = 1;
pub(crate) const O_RDWR: c_int = 2;

#[cfg(target_os = "macos")]
pub(crate) const O_NONBLOCK: c_int = 0x4;
#[cfg(target_os = "macos")]
pub(crate) const O_APPEND: c_int = 0x8;
#[cfg(target_os = "macos")]
pub(crate) const O_NOFOLLOW: c_int = 0x100;
#[cfg(target_os = "macos")]
pub(crate) const O_CREAT: c_int = 0x200;
#[cfg(target_os = "macos")]
pub(crate) const O_EXCL: c_int = 0x800;
#[cfg(target_os = "macos")]
pub(crate) const O_DIRECTORY: c_int = 0x0010_0000;
#[cfg(target_os = "macos")]
pub(crate) const O_CLOEXEC: c_int = 0x0100_0000;
#[cfg(target_os = "macos")]
pub(crate) const ELOOP: c_int = 62;

#[cfg(target_os = "linux")]
pub(crate) const O_CREAT: c_int = 0x40;
#[cfg(target_os = "linux")]
pub(crate) const O_EXCL: c_int = 0x80;
#[cfg(target_os = "linux")]
pub(crate) const O_APPEND: c_int = 0x400;
#[cfg(target_os = "linux")]
pub(crate) const O_NONBLOCK: c_int = 0x800;
#[cfg(target_os = "linux")]
pub(crate) const O_CLOEXEC: c_int = 0x80000;
#[cfg(target_os = "linux")]
pub(crate) const ELOOP: c_int = 40;
#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    )
))]
pub(crate) const O_DIRECTORY: c_int = 0x4000;
#[cfg(all(
    target_os = "linux",
    not(any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    ))
))]
pub(crate) const O_DIRECTORY: c_int = 0x10000;
#[cfg(all(
    target_os = "linux",
    any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    )
))]
pub(crate) const O_NOFOLLOW: c_int = 0x8000;
#[cfg(all(
    target_os = "linux",
    not(any(
        target_arch = "aarch64",
        target_arch = "arm",
        target_arch = "powerpc",
        target_arch = "powerpc64"
    ))
))]
pub(crate) const O_NOFOLLOW: c_int = 0x20000;

extern "C" {
    fn openat(dirfd: c_int, pathname: *const c_char, flags: c_int, ...) -> c_int;
}

/// Open a single child `name` strictly relative to an already-verified
/// directory descriptor `dir` with `O_NOFOLLOW | O_CLOEXEC`.
pub(crate) fn open_at(dir: &File, name: &str, flags: c_int) -> io::Result<File> {
    let c_name = CString::new(name)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "NUL in entry name"))?;
    let full_flags = flags | O_NOFOLLOW | O_CLOEXEC;
    // SAFETY: `dir.as_raw_fd()` is a live open directory descriptor owned
    // by `dir`, `c_name` is a NUL-terminated string outliving the call,
    // and when `O_CREAT` is set the variadic mode uses `c_uint` matching
    // C default argument promotion on both Linux and macOS.
    let fd = unsafe {
        if (full_flags & O_CREAT) != 0 {
            openat(
                dir.as_raw_fd(),
                c_name.as_ptr(),
                full_flags,
                0o666 as c_uint,
            )
        } else {
            openat(dir.as_raw_fd(), c_name.as_ptr(), full_flags)
        }
    };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: `openat` returned a fresh owned descriptor with `O_CLOEXEC`.
    Ok(unsafe { File::from_raw_fd(fd) })
}

pub(crate) fn map_err(err: io::Error, display_path: &Path) -> StoreError {
    if err.raw_os_error() == Some(ELOOP) {
        StoreError::SymlinkRefused(display_path.to_path_buf())
    } else {
        StoreError::Io(err)
    }
}
