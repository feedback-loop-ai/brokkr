//! Which of glibc's two program-name rules the running library follows.
//!
//! glibc through 2.42 bounds the name it copies — `file_len = __strnlen
//! (file, NAME_MAX) + 1` (`posix/execvpe.c` 86–106, revision `glibc-2.42`)
//! — and refuses nothing by that length: the `file_len - 1 > NAME_MAX`
//! test beside it cannot be true of a length `__strnlen` already capped
//! at `NAME_MAX`, and `__libc_alloca_cutoff (path_len + file_len + 1)`
//! bounds at most 4,353 bytes, which no thread stack this resolver runs
//! on refuses. A long name therefore MEETS THE KERNEL under each entry,
//! and the kernel's answer is decided by the entry: `ENOENT` under a
//! missing directory, `ENOTDIR` under a file, `ENAMETOOLONG` under a
//! directory that exists — each through the switch, none invented here
//! (fourth hold, R4; the inherited unconditional `NAME_MAX` refusal was
//! not that source's translation).
//!
//! glibc 2.43 makes that test true: `file_len - 1 == NAME_MAX &&
//! file[NAME_MAX] != '\0'` refuses a name longer than `NAME_MAX` with
//! `ENAMETOOLONG` before the search, as musl does (commit `efc8642051e6`,
//! BZ #33627, first released in 2.43; the 2.39–2.42 release branches do
//! not carry it). glibc is linked dynamically, so the compiled target
//! cannot answer which rule a child meets: the running library is asked.
//! A distribution that backports the commit under an older version
//! number would be read as the old rule; the native matrix on such a
//! host is what would show it.

/// Whether the glibc this process runs refuses a name longer than
/// `NAME_MAX` before searching. Asked once, through
/// `gnu_get_libc_version`.
pub(super) fn bounds_name() -> bool {
    static BOUNDS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *BOUNDS.get_or_init(|| version_bounds_name(&running_version()))
}

/// `bounds_name` over a version string, so the boundary is a plain test.
/// A version this does not read as `major.minor` is taken as the current
/// rule: glibc has always answered `major.minor[.patch]`.
fn version_bounds_name(version: &str) -> bool {
    let mut parts = version.split('.').map(str::parse::<u32>);
    match (parts.next(), parts.next()) {
        (Some(Ok(major)), Some(Ok(minor))) => (major, minor) >= (2, 43),
        _ => true,
    }
}

#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn running_version() -> String {
    // SAFETY: `gnu_get_libc_version` takes nothing and answers a pointer
    // to a NUL-terminated string glibc holds for the life of the process.
    unsafe { std::ffi::CStr::from_ptr(libc::gnu_get_libc_version()) }
        .to_string_lossy()
        .into_owned()
}

/// Only glibc targets ask; elsewhere `Library::Glibc` is never chosen.
#[cfg(not(all(target_os = "linux", target_env = "gnu")))]
fn running_version() -> String {
    String::new()
}

#[cfg(test)]
mod tests;
