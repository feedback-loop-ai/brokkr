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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct Identity {
    pub device: u64,
    pub inode: u64,
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

    /// Enumerate names through the held handle. Dot entries are dropped.
    pub fn entries(&self) -> io::Result<Vec<std::ffi::OsString>> {
        self.inner.entries()
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
    pub fn identity(&self) -> Identity {
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
    use super::Identity;
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
            device: stat.st_dev,
            inode: stat.st_ino,
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

        pub fn entries(&self) -> io::Result<Vec<OsString>> {
            let mut names = Vec::new();
            for entry in RDir::read_from(&self.fd)? {
                let entry = entry?;
                let name = entry.file_name();
                let name = OsStr::from_bytes(name.to_bytes()).to_os_string();
                if name == OsStr::new(".") || name == OsStr::new("..") {
                    continue;
                }
                names.push(name);
            }
            Ok(names)
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
        pub fn identity(&self) -> Identity {
            identity_of(&self.fd).unwrap_or(Identity {
                device: 0,
                inode: 0,
            })
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
    use super::Identity;
    use std::ffi::{OsStr, OsString};
    use std::io::{self, Read, Seek};
    use std::os::windows::fs::OpenOptionsExt;
    use std::path::{Path, PathBuf};

    const FILE_FLAG_OPEN_REPARSE_POINT: u32 = 0x0020_0000;
    const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x0000_0400;
    const FILE_ATTRIBUTE_DIRECTORY: u32 = 0x0000_0010;

    pub enum Child {
        File(File),
        Dir(Dir),
        Unsafe,
        Absent,
    }

    /// A held Windows directory. Handle-relative opening with reparse
    /// rejection is approximated with `FILE_FLAG_OPEN_REPARSE_POINT` and
    /// attribute checks; Windows execution remains a controller gate.
    pub struct Dir {
        path: PathBuf,
    }

    pub struct File {
        file: std::fs::File,
        identity: Identity,
    }

    fn attributes(metadata: &std::fs::Metadata) -> u32 {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes()
    }

    fn identity_of(metadata: &std::fs::Metadata) -> Identity {
        use std::os::windows::fs::MetadataExt;
        Identity {
            device: metadata.volume_serial_number().unwrap_or(0) as u64,
            inode: metadata.file_index().unwrap_or(0),
        }
    }

    impl Dir {
        pub fn open_root(path: &str) -> io::Result<Dir> {
            let _metadata = std::fs::metadata(path)?;
            Ok(Dir {
                path: Path::new(path).to_path_buf(),
            })
        }

        pub fn entries(&self) -> io::Result<Vec<OsString>> {
            let mut names = Vec::new();
            for entry in std::fs::read_dir(&self.path)? {
                names.push(entry?.file_name());
            }
            Ok(names)
        }

        pub fn child(&self, name: &OsStr) -> io::Result<Child> {
            let path = self.path.join(name);
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Child::Absent),
                Err(error) => return Err(error),
            };
            let attributes = attributes(&metadata);
            if attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
                return Ok(Child::Unsafe);
            }
            if attributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                return Ok(Child::Dir(Dir {
                    identity: identity_of(&metadata),
                    path,
                }));
            }
            let file = std::fs::OpenOptions::new()
                .read(true)
                .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT)
                .open(&path)?;
            let handle_metadata = file.metadata()?;
            let handle_attributes = attributes(&handle_metadata);
            if handle_attributes & FILE_ATTRIBUTE_REPARSE_POINT != 0
                || handle_attributes & FILE_ATTRIBUTE_DIRECTORY != 0
            {
                return Ok(Child::Unsafe);
            }
            Ok(Child::File(File {
                identity: identity_of(&handle_metadata),
                file,
            }))
        }
    }

    impl File {
        pub fn identity(&self) -> Identity {
            self.identity
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
