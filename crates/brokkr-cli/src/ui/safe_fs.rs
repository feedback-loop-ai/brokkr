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
pub struct Dir {
    inner: imp::Dir,
}

impl Dir {
    /// Open an already-canonicalized absolute directory path as the
    /// traversal root. The root itself may be reached through symlinks
    /// because canonicalization resolved them first.
    pub fn open_root(path: &str) -> io::Result<Dir> {
        Ok(Dir {
            inner: imp::Dir::open_root(path)?,
        })
    }

    /// Enumerate at most `max` names through the held handle, dot entries
    /// dropped. Returns `(names, truncated)`: `truncated` is true when the
    /// directory held more entries than `max`, so the caller can charge its
    /// discovery bound without materializing an unbounded listing.
    pub fn entries_bounded(&self, max: usize) -> io::Result<(Vec<std::ffi::OsString>, bool)> {
        self.inner.entries_bounded(max)
    }

    /// Open one direct child, following no symlink or reparse point.
    pub fn child(&self, name: &OsStr) -> io::Result<Child> {
        Ok(match self.inner.child(name)? {
            imp::Child::File(file) => Child::File(OpenedFile { inner: file }),
            imp::Child::Dir(dir) => Child::Dir(Dir { inner: dir }),
            imp::Child::Unsafe => Child::Unsafe,
            imp::Child::Absent => Child::Absent,
        })
    }
}

/// A held regular-file handle with its verified identity.
pub struct OpenedFile {
    inner: imp::File,
}

impl OpenedFile {
    pub fn identity(&self) -> io::Result<Identity> {
        self.inner.identity()
    }

    /// The source's size in bytes, measured through the held handle.
    pub fn len(&self) -> u64 {
        self.inner.len()
    }

    /// Read at most `cap` bytes plus one probe byte from the start of the
    /// file. Returns `(bytes, overflow, eof)`: `overflow` means the probe
    /// byte was present, `eof` means the read reached true end of file.
    pub fn read_bounded(&self, cap: u64) -> io::Result<(Vec<u8>, bool, bool)> {
        self.inner.read_bounded(cap)
    }
}

#[cfg(unix)]
mod imp {
    use super::{widen, Identity};
    use rustix::fs::{fstat, openat, Dir as RDir, FileType, Mode, OFlags, CWD};
    use std::ffi::{OsStr, OsString};
    use std::io::{self, Read, Seek};
    use std::os::unix::ffi::OsStrExt;

    pub enum Child {
        File(File),
        Dir(Dir),
        Unsafe,
        Absent,
    }

    pub struct Dir {
        fd: std::os::fd::OwnedFd,
    }

    pub struct File {
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
        pub fn open_root(path: &str) -> io::Result<Dir> {
            let fd = openat(CWD, path, dir_flags(), Mode::empty())?;
            Ok(Dir { fd })
        }

        pub fn entries_bounded(&self, max: usize) -> io::Result<(Vec<OsString>, bool)> {
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

        pub fn child(&self, name: &OsStr) -> io::Result<Child> {
            match openat(&self.fd, name, dir_flags(), Mode::empty()) {
                Ok(fd) => return Ok(Child::Dir(Dir { fd })),
                Err(error) if absent(&error) => return Ok(Child::Absent),
                Err(rustix::io::Errno::LOOP)
                | Err(rustix::io::Errno::MLINK)
                | Err(rustix::io::Errno::ISDIR) => return Ok(Child::Unsafe),
                Err(_) => {}
            }
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
        pub fn identity(&self) -> io::Result<Identity> {
            identity_of(&self.fd)
        }

        pub fn len(&self) -> u64 {
            fstat(&self.fd)
                .map(|stat| stat.st_size.max(0) as u64)
                .unwrap_or(0)
        }

        pub fn read_bounded(&self, cap: u64) -> io::Result<(Vec<u8>, bool, bool)> {
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

#[cfg(windows)]
mod imp {
    use super::{widen, Identity};
    use std::ffi::{OsStr, OsString};
    use std::io::{self, Read, Seek};
    use std::mem::size_of;
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use std::os::windows::io::{AsRawHandle, FromRawHandle, IntoRawHandle, OwnedHandle, RawHandle};
    use windows_sys::Wdk::Foundation::OBJECT_ATTRIBUTES;
    use windows_sys::Wdk::Storage::FileSystem::{
        NtCreateFile, FILE_DIRECTORY_FILE, FILE_NON_DIRECTORY_FILE, FILE_OPEN,
        FILE_OPEN_REPARSE_POINT, FILE_SYNCHRONOUS_IO_NONALERT,
    };
    use windows_sys::Win32::Foundation::{
        ERROR_NO_MORE_FILES, HANDLE, INVALID_HANDLE_VALUE, OBJ_CASE_INSENSITIVE,
        STATUS_FILE_IS_A_DIRECTORY, STATUS_NOT_A_DIRECTORY, STATUS_OBJECT_NAME_NOT_FOUND,
        STATUS_OBJECT_PATH_NOT_FOUND, STATUS_REPARSE_POINT_ENCOUNTERED, STATUS_SUCCESS,
        UNICODE_STRING,
    };
    use windows_sys::Win32::Storage::FileSystem::{
        CreateFileW, FileIdBothDirectoryInfo, GetFileInformationByHandle,
        GetFileInformationByHandleEx, BY_HANDLE_FILE_INFORMATION, FILE_ATTRIBUTE_DIRECTORY,
        FILE_ATTRIBUTE_REPARSE_POINT, FILE_FLAG_BACKUP_SEMANTICS, FILE_FLAG_OPEN_REPARSE_POINT,
        FILE_ID_BOTH_DIR_INFO, FILE_LIST_DIRECTORY, FILE_READ_ATTRIBUTES, FILE_READ_DATA,
        FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE, OPEN_EXISTING, SYNCHRONIZE,
    };
    use windows_sys::Win32::System::IO::IO_STATUS_BLOCK;

    const DIRECTORY_LIST_BUFFER: usize = 64 * 1024;

    pub enum Child {
        File(File),
        Dir(Dir),
        Unsafe,
        Absent,
    }

    /// A held Windows directory opened with `FILE_FLAG_OPEN_REPARSE_POINT`
    /// and its stable identity. Descendants are opened relative to this
    /// handle with `NtCreateFile`, never by rebuilding an absolute path.
    pub struct Dir {
        handle: OwnedHandle,
        #[allow(dead_code)]
        identity: Identity,
    }

    pub struct File {
        file: std::fs::File,
        identity: Identity,
    }

    fn handle_identity(handle: &impl AsRawHandle) -> io::Result<Identity> {
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, &mut info) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Identity {
            device: widen(info.dwVolumeSerialNumber)?,
            inode: widen(((info.nFileIndexHigh as u64) << 32) | info.nFileIndexLow as u64)?,
        })
    }

    fn handle_attributes(handle: &impl AsRawHandle) -> io::Result<u32> {
        let mut info: BY_HANDLE_FILE_INFORMATION = unsafe { std::mem::zeroed() };
        let ok = unsafe { GetFileInformationByHandle(handle.as_raw_handle() as HANDLE, &mut info) };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(info.dwFileAttributes)
    }

    fn nt_status(error: &io::Error) -> Option<i32> {
        error.raw_os_error()
    }

    fn absent_status(status: i32) -> bool {
        status == STATUS_OBJECT_NAME_NOT_FOUND
            || status == STATUS_OBJECT_PATH_NOT_FOUND
            || status == FILE_NOT_FOUND_STATUS
    }

    /// `STATUS_NO_SUCH_FILE` keeps this mapping local so the import list
    /// stays to the statuses the reader distinguishes.
    const FILE_NOT_FOUND_STATUS: i32 = 0xC000_000F_u32 as i32;

    fn nt_open(root: &impl AsRawHandle, name: &OsStr, directory: bool) -> io::Result<OwnedHandle> {
        let wide: Vec<u16> = name.encode_wide().collect();
        let unicode = UNICODE_STRING {
            Length: (wide.len() * 2) as u16,
            MaximumLength: (wide.len() * 2) as u16,
            Buffer: wide.as_ptr() as *mut u16,
        };
        let attributes = OBJECT_ATTRIBUTES {
            Length: size_of::<OBJECT_ATTRIBUTES>() as u32,
            RootDirectory: root.as_raw_handle() as HANDLE,
            ObjectName: &unicode,
            Attributes: OBJ_CASE_INSENSITIVE,
            SecurityDescriptor: std::ptr::null(),
            SecurityQualityOfService: std::ptr::null(),
        };
        let mut iosb = IO_STATUS_BLOCK::default();
        let mut handle: HANDLE = std::ptr::null_mut();
        let access = if directory {
            FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE
        } else {
            FILE_READ_DATA | FILE_READ_ATTRIBUTES | SYNCHRONIZE
        };
        let mut options = FILE_OPEN_REPARSE_POINT | FILE_SYNCHRONOUS_IO_NONALERT;
        options |= if directory {
            FILE_DIRECTORY_FILE
        } else {
            FILE_NON_DIRECTORY_FILE
        };
        let status = unsafe {
            NtCreateFile(
                &mut handle,
                access,
                &attributes,
                &mut iosb,
                std::ptr::null(),
                0,
                FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                FILE_OPEN,
                options,
                std::ptr::null(),
                0,
            )
        };
        if status != STATUS_SUCCESS {
            return Err(io::Error::from_raw_os_error(status));
        }
        Ok(unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) })
    }

    fn into_file(handle: OwnedHandle) -> std::fs::File {
        let raw = handle.into_raw_handle();
        unsafe { std::fs::File::from_raw_handle(raw as RawHandle) }
    }

    impl Dir {
        pub fn open_root(path: &str) -> io::Result<Dir> {
            // The caller canonicalizes the recorded home first, so the root
            // itself is opened without following a reparse point.
            let wide: Vec<u16> = OsStr::new(path)
                .encode_wide()
                .chain(std::iter::once(0))
                .collect();
            let handle = unsafe {
                CreateFileW(
                    wide.as_ptr(),
                    FILE_LIST_DIRECTORY | FILE_READ_ATTRIBUTES | SYNCHRONIZE,
                    FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                    std::ptr::null(),
                    OPEN_EXISTING,
                    FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT,
                    std::ptr::null_mut(),
                )
            };
            if handle == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }
            let handle = unsafe { OwnedHandle::from_raw_handle(handle as RawHandle) };
            let identity = handle_identity(&handle)?;
            Ok(Dir { handle, identity })
        }

        pub fn entries_bounded(&self, max: usize) -> io::Result<(Vec<OsString>, bool)> {
            let mut names = Vec::new();
            let mut buffer = vec![0u8; DIRECTORY_LIST_BUFFER];
            loop {
                let ok = unsafe {
                    GetFileInformationByHandleEx(
                        self.handle.as_raw_handle() as HANDLE,
                        FileIdBothDirectoryInfo,
                        buffer.as_mut_ptr() as *mut _,
                        buffer.len() as u32,
                    )
                };
                if ok == 0 {
                    let error = io::Error::last_os_error();
                    if error.raw_os_error() == Some(ERROR_NO_MORE_FILES as i32) {
                        return Ok((names, false));
                    }
                    return Err(error);
                }
                let mut offset = 0usize;
                loop {
                    let info =
                        unsafe { &*(buffer.as_ptr().add(offset) as *const FILE_ID_BOTH_DIR_INFO) };
                    if info.FileNameLength > 0 {
                        let length = (info.FileNameLength / 2) as usize;
                        let slice =
                            unsafe { std::slice::from_raw_parts(info.FileName.as_ptr(), length) };
                        let name = OsString::from_wide(slice);
                        if name != OsStr::new(".") && name != OsStr::new("..") {
                            if names.len() >= max {
                                return Ok((names, true));
                            }
                            names.push(name);
                        }
                    }
                    if info.NextEntryOffset == 0 {
                        break;
                    }
                    offset += info.NextEntryOffset as usize;
                }
            }
        }

        pub fn child(&self, name: &OsStr) -> io::Result<Child> {
            match nt_open(&self.handle, name, true) {
                Ok(handle) => {
                    // A directory reparse point (a junction or directory
                    // symlink) is not a traversal-safe ancestor.
                    if handle_attributes(&handle)? & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                        return Ok(Child::Unsafe);
                    }
                    let identity = handle_identity(&handle)?;
                    return Ok(Child::Dir(Dir { handle, identity }));
                }
                Err(error) => {
                    if nt_status(&error).is_some_and(absent_status) {
                        return Ok(Child::Absent);
                    }
                }
            }
            match nt_open(&self.handle, name, false) {
                Ok(handle) => {
                    let file = into_file(handle);
                    let attributes = handle_attributes(&file)?;
                    if attributes & (FILE_ATTRIBUTE_REPARSE_POINT | FILE_ATTRIBUTE_DIRECTORY) != 0 {
                        return Ok(Child::Unsafe);
                    }
                    let identity = handle_identity(&file)?;
                    Ok(Child::File(File { file, identity }))
                }
                Err(error) => match nt_status(&error) {
                    Some(status) if absent_status(status) => Ok(Child::Absent),
                    Some(STATUS_FILE_IS_A_DIRECTORY)
                    | Some(STATUS_NOT_A_DIRECTORY)
                    | Some(STATUS_REPARSE_POINT_ENCOUNTERED) => Ok(Child::Unsafe),
                    _ => Err(error),
                },
            }
        }
    }

    impl File {
        pub fn identity(&self) -> io::Result<Identity> {
            Ok(self.identity)
        }

        pub fn len(&self) -> u64 {
            self.file.metadata().map(|meta| meta.len()).unwrap_or(0)
        }

        pub fn read_bounded(&self, cap: u64) -> io::Result<(Vec<u8>, bool, bool)> {
            let mut clone = self.file.try_clone()?;
            clone.rewind()?;
            let mut buffer = Vec::new();
            let mut reader = std::io::Read::take(&mut clone, cap.saturating_add(1));
            reader.read_to_end(&mut buffer)?;
            let overflow = buffer.len() as u64 > cap;
            if overflow {
                buffer.truncate(cap as usize);
            }
            Ok((buffer, overflow, !overflow))
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
