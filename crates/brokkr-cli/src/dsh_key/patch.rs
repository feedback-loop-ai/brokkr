//! What one dsh patch layer writes of the credentials service, read by a
//! bounded line reader that recognises a flat list of rows and refuses
//! everything else (#717, finding 2).
//!
//! The workspace's reader for dsh's patches is brokkr-protocol's route
//! overlay reader (`adapters/route_overlay.rs`), which is private to that
//! crate. This reader admits a strict subset of its grammar: comments,
//! blank lines, a top-level item `- <key>: <value>` that opens a row, and
//! a row's own member `  <key>: <value>`, two spaces in, every value one
//! plain scalar. A nested block, a quoted or flow value, a tab, a document
//! marker or any other line is not read past: the layer answers "can't
//! tell" naming the line's number, never its text, so a row this reader
//! does not see can never answer ok.
//!
//! A layer is reached by a walk from the dsh home that follows no link
//! below it (#717, fix rounds 717d4 and 717d5). The home is opened once by
//! the name dsh's own seam resolves, `$DSH_HOME` else `$HOME/.dsh`, and a
//! link there is followed: it is the operator's own root, and on macOS
//! `/var` is itself a link to `/private/var`, so no walk could start at
//! `/`. Every component below it, `profiles`, the profile's name and the
//! layer's own, is looked up relative to the directory before it without
//! following a link, and a symlink in its place is refused unread; it is
//! then opened relative to that same directory with `O_NOFOLLOW`, so a
//! link swapped in after the lookup fails to open, a directory with
//! `O_DIRECTORY` and the layer with `O_NONBLOCK`, so a FIFO swapped in is
//! never waited on; and only what `fstat` tells a directory, or a regular
//! file at the last, on that one descriptor is walked or read. So a patch
//! path that names a credentials file through a link at any component is
//! never opened. One path serves both hosts: Linux's `RESOLVE_NO_SYMLINKS`
//! has no macOS counterpart. A hard link to a credentials file is a
//! regular file, which Brokkr can't tell from any other: it is read into
//! the bounded buffer, refused at its first line that is not a row, and
//! no line of it is echoed. The follow-up that retires this reader for
//! brokkr-protocol's must carry the same component-wise guard, not only
//! the leaf's: `route_overlay.rs` looks a route file up with
//! `std::fs::metadata` and reads it with `std::fs::read`, both of which
//! follow a link at every component.

use std::fmt;
use std::io::Read;
use std::os::fd::{AsFd, BorrowedFd, OwnedFd};
use std::path::Path;

use rustix::fs::{AtFlags, FileType, Mode, OFlags};
use rustix::io::Errno;

/// The row a patch writes to switch dsh's credentials service on or off.
pub(super) const SERVICE: &str = "credentials";

/// A patch layer is a handful of rows; a larger file is not read.
const LIMIT: u64 = 64 * 1024;

/// What one layer's last `credentials` row writes of its `disabled` key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Write {
    /// `disabled: false`: the service is switched back on.
    Enables,
    /// `disabled: true`: the service is switched off.
    Disables,
}

/// Why a present patch layer cannot be told. The text is the operator's,
/// and names a line by its number alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Unread {
    /// A symlink in the layer's place, which is never followed.
    Link,
    NotAFile,
    Io(std::io::ErrorKind),
    TooLarge,
    NotUtf8,
    /// A line outside the rows this reader recognises.
    Line(usize),
    /// A row's key written twice, which a YAML reader refuses or collapses.
    Repeated(usize),
    /// A row opened on this line names no `id`.
    NoId(usize),
    /// A `disabled` value other than `true` or `false`.
    Disabled(usize),
}

impl fmt::Display for Unread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Unread::Link => write!(f, "it is a symbolic link, which Brokkr never follows"),
            Unread::NotAFile => write!(f, "it is not a regular file"),
            Unread::Io(kind) => write!(f, "it cannot be read ({kind})"),
            Unread::TooLarge => write!(f, "it is over {LIMIT} bytes"),
            Unread::NotUtf8 => write!(f, "it is not UTF-8"),
            Unread::Line(line) => write!(
                f,
                "line {line} is not a row Brokkr reads (a top-level `- key: value` and its own \
                 `  key: value` members, each value one plain scalar)"
            ),
            Unread::Repeated(line) => write!(f, "line {line} repeats a key of its row"),
            Unread::NoId(line) => write!(f, "the row on line {line} names no id"),
            Unread::Disabled(line) => {
                write!(
                    f,
                    "line {line} gives `disabled` a value other than true or false"
                )
            }
        }
    }
}

/// What a present layer was found to hold.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Layer {
    /// No file is there, or no directory on the way to it.
    Absent,
    /// Read for its `credentials` row: what its last one writes of
    /// `disabled`, nothing where it names none.
    Read(Option<Write>),
}

/// What the layer `leaf`, in the directories `below` under `home`, holds
/// of the credentials service, reached by [`walk`].
pub(super) fn layer(home: &Path, below: &[&str], leaf: &str) -> Result<Layer, Unread> {
    let Some(file) = walk(home, below, leaf)? else {
        return Ok(Layer::Absent);
    };
    let mut bytes = Vec::new();
    let read = std::fs::File::from(file)
        .take(LIMIT + 1)
        .read_to_end(&mut bytes);
    read.map_err(|error| Unread::Io(error.kind()))?;
    if bytes.len() as u64 > LIMIT {
        return Err(Unread::TooLarge);
    }
    let text = String::from_utf8(bytes).map_err(|_| Unread::NotUtf8)?;
    writes(&text).map(Layer::Read)
}

/// The layer opened by a walk from `home`, opened by name, through each of
/// `below` to `leaf`, each [`step`] relative to the directory before it;
/// `None` where a name on the way is not there.
fn walk(home: &Path, below: &[&str], leaf: &str) -> Result<Option<OwnedFd>, Unread> {
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let mut directory = match rustix::fs::open(home, flags, Mode::empty()) {
        Err(Errno::NOENT) => return Ok(None),
        opened => opened.map_err(|errno| Unread::Io(std::io::Error::from(errno).kind()))?,
    };
    for name in below {
        let Some(next) = step(directory.as_fd(), name, Kind::Directory)? else {
            return Ok(None);
        };
        directory = next;
    }
    step(directory.as_fd(), leaf, Kind::File)
}

/// What a walk expects at a name: a directory to walk on, or the layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Directory,
    File,
}

impl Kind {
    fn file_type(self) -> FileType {
        match self {
            Kind::Directory => FileType::Directory,
            Kind::File => FileType::RegularFile,
        }
    }

    /// The open's own refusal of anything else: a directory's opens no
    /// other kind, and the layer's never waits.
    fn flags(self) -> OFlags {
        match self {
            Kind::Directory => OFlags::DIRECTORY,
            Kind::File => OFlags::NONBLOCK,
        }
    }

    /// Why a name of another kind is not walked or read.
    fn other(self) -> Unread {
        match self {
            Kind::Directory => Unread::Io(std::io::ErrorKind::NotADirectory),
            Kind::File => Unread::NotAFile,
        }
    }
}

/// `name` in `directory`, `None` where it is not there: looked up without
/// following a link, and [`opened`] only where it is of `kind`, so a
/// symlink, a FIFO or a device in its place is never opened.
fn step(directory: BorrowedFd<'_>, name: &str, kind: Kind) -> Result<Option<OwnedFd>, Unread> {
    let found = match rustix::fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW) {
        Err(Errno::NOENT) => return Ok(None),
        found => found.map_err(unread)?,
    };
    let found = FileType::from_raw_mode(found.st_mode);
    if found == FileType::Symlink {
        return Err(Unread::Link);
    }
    if found != kind.file_type() {
        return Err(kind.other());
    }
    opened(directory, name, kind).map(Some)
}

/// `name` in `directory` opened for reading where it is of `kind` reached
/// without following a link, judged by `fstat` on the one descriptor that
/// is walked or read: a link swapped in after [`step`]'s lookup fails to
/// open, and a FIFO swapped in for the layer opens without waiting and is
/// refused unread.
pub(super) fn opened(directory: BorrowedFd<'_>, name: &str, kind: Kind) -> Result<OwnedFd, Unread> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | kind.flags();
    let file = rustix::fs::openat(directory, name, flags, Mode::empty())
        .map_err(|errno| refused(directory, name, errno))?;
    let held = rustix::fs::fstat(&file).map_err(unread)?;
    if FileType::from_raw_mode(held.st_mode) != kind.file_type() {
        return Err(kind.other());
    }
    Ok(file)
}

/// Why `name`'s open in `directory` failed. Linux answers a link opened
/// `O_DIRECTORY|O_NOFOLLOW` with `ENOTDIR`, where macOS answers `ELOOP`, so
/// an `ENOTDIR` is told a link where a lookup that follows none finds one.
fn refused(directory: BorrowedFd<'_>, name: &str, errno: Errno) -> Unread {
    let link = || {
        let found = rustix::fs::statat(directory, name, AtFlags::SYMLINK_NOFOLLOW);
        found.is_ok_and(|found| FileType::from_raw_mode(found.st_mode) == FileType::Symlink)
    };
    match errno {
        Errno::NOTDIR if link() => Unread::Link,
        errno => unread(errno),
    }
}

/// Why a lookup, an open or an `fstat` failed: `ELOOP` is the link
/// `O_NOFOLLOW` refused.
fn unread(errno: Errno) -> Unread {
    match errno {
        Errno::LOOP => Unread::Link,
        errno => Unread::Io(std::io::Error::from(errno).kind()),
    }
}

/// One top-level row: the line it opens on, its id and what it writes of
/// `disabled`.
struct Row {
    line: usize,
    id: Option<String>,
    disabled: Option<Write>,
    keys: Vec<String>,
}

/// What `text`'s LAST `credentials` row writing `disabled` writes, read
/// only once every line of it is recognised.
pub(super) fn writes(text: &str) -> Result<Option<Write>, Unread> {
    let rows = rows(text)?;
    let last = rows
        .iter()
        .rev()
        .find(|row| row.id.as_deref() == Some(SERVICE) && row.disabled.is_some());
    Ok(last.and_then(|row| row.disabled))
}

/// Every row of `text`, or the first line that is not one this reader
/// recognises.
fn rows(text: &str) -> Result<Vec<Row>, Unread> {
    let (mut rows, mut row) = (Vec::new(), None::<Row>);
    for (index, line) in text.split('\n').enumerate() {
        let number = index + 1;
        if line.chars().any(char::is_control) || line == "---" || line == "..." {
            return Err(Unread::Line(number));
        }
        let body = line.trim_start_matches(' ');
        if body.is_empty() || body.starts_with('#') {
            continue;
        }
        // An item opens a row; a member is exactly two spaces in.
        let member = match line.strip_prefix("- ") {
            Some(item) => {
                rows.extend(row.replace(Row {
                    line: number,
                    id: None,
                    disabled: None,
                    keys: Vec::new(),
                }));
                item
            }
            None if line.len() - body.len() == 2 => body,
            None => return Err(Unread::Line(number)),
        };
        let (Some(row), Some(pair)) = (row.as_mut(), pair(member)) else {
            return Err(Unread::Line(number));
        };
        member_of(row, pair, number)?;
    }
    rows.extend(row);
    match rows.iter().find(|row| row.id.is_none()) {
        Some(row) => Err(Unread::NoId(row.line)),
        None => Ok(rows),
    }
}

/// `key: value` into `row`, typed where it is a key this check reads.
fn member_of(row: &mut Row, (key, value): (&str, &str), number: usize) -> Result<(), Unread> {
    if row.keys.iter().any(|seen| seen == key) {
        return Err(Unread::Repeated(number));
    }
    row.keys.push(key.to_string());
    match key {
        "id" => row.id = Some(value.to_string()),
        "disabled" => {
            row.disabled = Some(match value {
                "true" => Write::Disables,
                "false" => Write::Enables,
                _ => return Err(Unread::Disabled(number)),
            })
        }
        _ => {}
    }
    Ok(())
}

/// A member's identifier key and its one plain scalar, trailing blanks
/// aside; `None` for any other text, a block opener `key:` among them.
fn pair(member: &str) -> Option<(&str, &str)> {
    let (key, value) = member.split_once(": ")?;
    let value = value.trim_end_matches(' ');
    let mut head = key.chars();
    let identifier = head.next().is_some_and(|first| first.is_ascii_alphabetic())
        && head.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    let plain = value
        .chars()
        .next()
        .is_some_and(|first| !"!&*{[|>%@`\"'?-:,#".contains(first))
        && !value.contains(" #")
        && !value.contains(": ");
    (identifier && plain).then_some((key, value))
}
