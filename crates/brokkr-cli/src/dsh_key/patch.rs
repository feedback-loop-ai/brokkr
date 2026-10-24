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
//! A layer's name is looked up without following a link, and a symlink in
//! its place is refused unread, so a patch path that names a credentials
//! file is never opened (#717, fix round 717d4). The one open refuses a
//! link too, `O_NOFOLLOW`, so one swapped in after the lookup fails to
//! open; it does not wait, `O_NONBLOCK`, so a FIFO swapped in is never
//! waited on; and only what `fstat` tells a regular file on that one
//! descriptor is read. The follow-up that retires this reader for
//! brokkr-protocol's must carry the same guard: `route_overlay.rs` looks a
//! route file up with `std::fs::metadata` and reads it with
//! `std::fs::read`, both of which follow a link.

use std::fmt;
use std::io::Read;
use std::path::Path;

use rustix::fs::{FileType, Mode, OFlags};
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

/// What the layer at `path` writes of the credentials service: nothing
/// where it is absent or names no `credentials` row writing `disabled`.
pub(super) fn layer(path: &Path) -> Result<Option<Write>, Unread> {
    match read(path)? {
        Some(text) => writes(&text),
        None => Ok(None),
    }
}

/// The layer's text, `None` where no file is there. Its name is looked up
/// without following a link, and only a regular file is [`opened`], so a
/// symlink, a FIFO or a device in its place is never opened.
fn read(path: &Path) -> Result<Option<String>, Unread> {
    let io = |error: std::io::Error| Unread::Io(error.kind());
    let metadata = match std::fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        found => found.map_err(io)?,
    };
    if metadata.is_symlink() {
        return Err(Unread::Link);
    }
    if !metadata.is_file() {
        return Err(Unread::NotAFile);
    }
    let mut bytes = Vec::new();
    let file = opened(path)?;
    file.take(LIMIT + 1).read_to_end(&mut bytes).map_err(io)?;
    if bytes.len() as u64 > LIMIT {
        return Err(Unread::TooLarge);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Unread::NotUtf8)
}

/// `path` opened for reading where it is a regular file reached without
/// following a link, judged by `fstat` on the one descriptor that is read:
/// a link swapped in after [`read`]'s lookup fails to open, and a FIFO
/// swapped in opens without waiting and is refused unread.
pub(super) fn opened(path: &Path) -> Result<std::fs::File, Unread> {
    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::CLOEXEC;
    let file = rustix::fs::open(path, flags, Mode::empty()).map_err(unread)?;
    let held = rustix::fs::fstat(&file).map_err(unread)?;
    if FileType::from_raw_mode(held.st_mode) != FileType::RegularFile {
        return Err(Unread::NotAFile);
    }
    Ok(std::fs::File::from(file))
}

/// Why an open or `fstat` failed: `ELOOP` is the link `O_NOFOLLOW` refused
/// (Linux and macOS both answer it).
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
