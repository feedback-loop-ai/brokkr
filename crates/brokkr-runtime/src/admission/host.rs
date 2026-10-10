//! The machine's capacity as its operator declares it (decision 0068
//! ruling 3, and ruling 5's data): `forge.host/v1`, one file per machine
//! and never a realm's (the operator's ruling of 2026-10-10, option B). A
//! machine has one scratch disk, one set of provider accounts and its own
//! local hardware, whichever realm a run operates, so the tracked realms
//! map carries none of it (#432).
//!
//! The file is found where [`host_file`] says, and read by [`read`] into
//! one of three outcomes that are never confused: nothing at the path is
//! capacity undeclared; a path that holds something that cannot be read
//! (a link that loops or dangles, a directory, a permission refused) is
//! unreadable, never absent (#430's H5); and a file read but not a valid
//! `forge.host/v1` refuses, naming the problem. A ceiling of zero, a
//! negative one and one past `u32` are refused, never read as unlimited,
//! and so is a provider or route written twice: the ceiling the operator
//! reads first is the one admission obeys, or none is.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt;
use std::io::{self, ErrorKind};
use std::num::{NonZeroU32, NonZeroU64};
use std::path::{Path, PathBuf};

use serde::Deserialize;
use thiserror::Error;

use crate::capabilities::manifest::present;

/// A host configuration, read whole: every key known, every ceiling at
/// least one.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct HostConfig {
    /// Read only to refuse another version.
    #[serde(rename = "schema")]
    _schema: Schema,
    pub(crate) providers: BTreeMap<String, Provider>,
    pub(crate) scratch: Scratch,
    pub(crate) boxed_builds: Ceiling,
}

/// The file's version, its first key: a file in any other is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
enum Schema {
    #[serde(rename = "forge.host/v1")]
    V1,
}

/// One provider's ceiling, and the routes of it the machine declares.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Provider {
    pub(crate) ceiling: NonZeroU32,
    #[serde(default)]
    pub(crate) routes: BTreeMap<String, Route>,
}

/// One route's ceiling and class.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Route {
    pub(crate) ceiling: NonZeroU32,
    pub(crate) class: RouteClass,
}

/// Where a route's model runs (decision 0068 ruling 5): behind a cloud
/// account, counted against its provider's ceiling and its own, or on
/// local hardware the machine shares with other work, counted against its
/// own ceiling in place of its provider's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RouteClass {
    Cloud,
    SharedLocal,
}

impl fmt::Display for RouteClass {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RouteClass::Cloud => "cloud",
            RouteClass::SharedLocal => "shared-local",
        })
    }
}

/// The seat scratch filesystem's floor, and the path it is measured at
/// where that is not the scratch default. The path may be left out, and
/// is never `null`, as the contract says.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Scratch {
    pub(crate) floor_bytes: NonZeroU64,
    #[serde(default, deserialize_with = "present")]
    pub(crate) path: Option<Absolute>,
}

/// A ceiling on its own: the boxed builds'.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ceiling {
    pub(crate) ceiling: NonZeroU32,
}

/// An absolute path: a relative one would be measured against whatever
/// directory admission runs in.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "PathBuf")]
pub(crate) struct Absolute(pub(crate) PathBuf);

/// A scratch path that is not absolute.
#[derive(Debug, Error)]
#[error("the scratch path {} is not absolute", .0.display())]
pub(crate) struct Relative(PathBuf);

impl TryFrom<PathBuf> for Absolute {
    type Error = Relative;

    fn try_from(path: PathBuf) -> Result<Absolute, Relative> {
        match path.is_absolute() {
            true => Ok(Absolute(path)),
            false => Err(Relative(path)),
        }
    }
}

/// What the host configuration's path holds.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Hosting {
    /// Nothing: capacity is undeclared.
    Absent,
    /// Something that cannot be read, and the kind of error that said so.
    Unreadable(ErrorKind),
    Declared(HostConfig),
}

/// Why no host configuration can be read at all.
#[derive(Debug, Error)]
pub enum HostError {
    #[error(
        "no host configuration can be found: neither XDG_CONFIG_HOME nor HOME names an absolute \
         directory"
    )]
    Unplaced,
    #[error("the host configuration at {} is not a valid forge.host/v1 file", path.display())]
    Invalid {
        path: PathBuf,
        #[source]
        source: Problem,
    },
}

/// What makes a read host configuration invalid.
#[derive(Debug, Error)]
pub enum Problem {
    /// Not JSON, or not the contract's shape.
    #[error(transparent)]
    Shape(#[from] serde_json::Error),
    /// A key written twice in one object, in the strict reader's words.
    #[error("{0}")]
    Repeated(String),
}

/// Where the machine's host configuration is: `brokkr/host.json` under
/// `$XDG_CONFIG_HOME`, else under `$HOME/.config`, on Linux and macOS
/// alike. A value that is not an absolute directory is passed over, as
/// the XDG base directory specification asks of a relative one.
pub fn host_file(xdg: Option<OsString>, home: Option<OsString>) -> Result<PathBuf, HostError> {
    let absolute = |dir: Option<OsString>| dir.map(PathBuf::from).filter(|dir| dir.is_absolute());
    absolute(xdg)
        .or_else(|| absolute(home).map(|home| home.join(".config")))
        .map(|root| root.join("brokkr").join("host.json"))
        .ok_or(HostError::Unplaced)
}

/// Read the host configuration at `file`. Only a path with nothing at it
/// is absent; whatever is there and cannot be read is unreadable, and so
/// is a path that cannot be reached because a directory on it is a link
/// that loops or dangles.
pub(crate) fn read(file: &Path) -> Result<Hosting, HostError> {
    if let Err(absent) = std::fs::symlink_metadata(file) {
        if absent.kind() == ErrorKind::NotFound {
            return Ok(unreached(file).map_or(Hosting::Absent, Hosting::Unreadable));
        }
    }
    let bytes = match std::fs::read(file) {
        Ok(bytes) => bytes,
        Err(unread) => return Ok(Hosting::Unreadable(unread.kind())),
    };
    config(&bytes)
        .map(Hosting::Declared)
        .map_err(|source| HostError::Invalid {
            path: file.to_path_buf(),
            source,
        })
}

/// A host configuration's bytes, read as the contract's shape and then
/// through the house's strict reader, which refuses a key written twice
/// in one object where serde's map keeps the last copy and says nothing.
/// The shape is judged first, so its refusal keeps the parser's place in
/// the file; bytes it admits are UTF-8, so the lossy text is the file.
fn config(bytes: &[u8]) -> Result<HostConfig, Problem> {
    let config = serde_json::from_slice(bytes)?;
    brokkr_core::canonical::parse_strict(&String::from_utf8_lossy(bytes))
        .map_err(Problem::Repeated)?;
    Ok(config)
}

/// Why the deepest component of `file`'s path that is there cannot be
/// followed, where it is a link that loops or dangles; `None` where it is
/// a directory below which the rest of the path is simply absent.
fn unreached(file: &Path) -> Option<ErrorKind> {
    let there = (file.ancestors().skip(1)).find(|dir| std::fs::symlink_metadata(dir).is_ok());
    there
        .and_then(|dir| std::fs::metadata(dir).err())
        .map(|error| error.kind())
}

/// The bytes free to an unprivileged writer on the filesystem holding
/// `path`, or that would hold it once made: that of its deepest component
/// that is there, since a seat's scratch tree is made when its attempt
/// starts. Only a component with nothing at it is passed over: one that
/// is a link that dangles or loops is there, and cannot be measured,
/// never measured on its parent's filesystem. The free-space probe
/// production admission measures with.
pub fn free_bytes(path: &Path) -> io::Result<u64> {
    let there = (path.ancestors()).find_map(|at| match std::fs::symlink_metadata(at) {
        Err(absent) if absent.kind() == ErrorKind::NotFound => None,
        reached => Some(reached.map(|_| at)),
    });
    let stat = rustix::fs::statvfs(there.unwrap_or(Ok(path))?)?;
    Ok(stat.f_bavail.saturating_mul(stat.f_frsize))
}

#[cfg(test)]
mod tests;
