//! A PID namespace of a test's own, for a test that delivers a real
//! signal (#1137h): whatever pid or group id it names, it reaches no
//! process outside the namespace. Linux only: macOS has no PID namespace,
//! so a test that delivers a real signal is `cfg(target_os = "linux")`.
//! Shared by doctor's re-runs (`src/doctor/hermetic_tests.rs`) and the
//! built doctor's runs (`tests/support/isolated.rs`).
//!
//! A host that cannot build one, because it has no `bwrap` or is a box
//! that refuses to nest one (decision 0054), fails the test with its
//! reason where [`REQUIRE`] is `1`, as CI's Linux test and coverage jobs
//! declare it; where it is absent the test skips with a printed notice,
//! and any other value fails the test (#1137i, Z3).

#![cfg(target_os = "linux")]

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::OnceLock;

/// The bwrap arguments that put a command alone in a PID namespace. Every
/// process of it sees the host's files through a writable bind of `/`, so
/// a coverage profile is still written (#1137g, P3); only `/proc` is the
/// namespace's own.
pub(crate) const ALONE: [&str; 8] = [
    "--dev-bind",
    "/",
    "/",
    "--die-with-parent",
    "--unshare-pid",
    "--proc",
    "/proc",
    "--",
];

/// The variable that makes a namespace this host cannot build a failure.
pub(crate) const REQUIRE: &str = "BROKKR_REQUIRE_PIDNS";

/// Where a process reads the identity of the PID namespace it runs in.
pub(crate) const IDENTITY: &str = "/proc/self/ns/pid";

/// What a test that is not isolated says as it refuses.
pub(crate) const REFUSED: &str =
    "a test that delivers a real signal refused before it delivered anything";

/// Why no namespace can be built here.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Unavailable {
    /// No bwrap to build one, in its finder's words.
    #[error("{0}")]
    NoBwrap(String),
    /// The bwrap found does not start.
    #[error("{bwrap} does not run: {error}")]
    DoesNotRun {
        bwrap: String,
        error: std::io::Error,
    },
    /// The bwrap ran and could not build one, as a nesting box refuses.
    #[error("{bwrap} cannot build one here ({status}): {said}")]
    Refused {
        bwrap: String,
        status: ExitStatus,
        said: String,
    },
}

/// Why a process is not proved isolated: each refuses delivery.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum Isolation {
    /// A namespace's identity could not be read, and what was read.
    #[error("a PID namespace identity is unreadable: {0}")]
    Unreadable(String),
    /// A namespace's identity read as something other than `pid:[N]`.
    #[error("a PID namespace identity reads {0:?}, not pid:[N]")]
    Unparsable(String),
    /// The child's is the parent's.
    #[error("it runs in its parent's PID namespace, pid:[{0}]")]
    Shared(u64),
}

/// The number of the PID namespace `link` names (#1137i, Z2).
/// Its identity is exactly `pid:[N]`, where N is a canonical ASCII
/// decimal that fits a `u64`, with no sign or leading zero (except `0`).
/// Nothing follows the closing bracket.
pub(crate) fn identity_at(link: &Path) -> Result<u64, Isolation> {
    let read = std::fs::read_link(link)
        .map_err(|error| Isolation::Unreadable(format!("{}: {error}", link.display())))?;
    let read = read.to_string_lossy();
    let number = read
        .strip_prefix("pid:[")
        .and_then(|rest| rest.strip_suffix(']'))
        .filter(|number| !number.is_empty() && number.bytes().all(|b| b.is_ascii_digit()))
        .filter(|number| number.len() == 1 || !number.starts_with('0'));
    number
        .and_then(|number| number.parse().ok())
        .ok_or_else(|| Isolation::Unparsable(read.into_owned()))
}

/// The number of the PID namespace this process runs in.
pub(crate) fn identity() -> Result<u64, Isolation> {
    identity_at(Path::new(IDENTITY))
}

/// The bwrap that can build [`ALONE`] here, or `None` once a test that
/// cannot have one has skipped with its notice; where [`REQUIRE`] demands
/// one, a host that cannot build it fails the test instead. [`REQUIRE`]
/// is read first, whatever the host can build, so a value that is
/// neither `1` nor absent fails the test everywhere.
pub(crate) fn isolating() -> Option<&'static str> {
    let demand = demand();
    match namespace() {
        Ok(bwrap) => Some(bwrap),
        Err(unavailable) => {
            unisolated(demand, unavailable);
            None
        }
    }
}

/// [`built`], asked once per test process.
fn namespace() -> &'static Result<String, Unavailable> {
    static BUILT: OnceLock<Result<String, Unavailable>> = OnceLock::new();
    BUILT.get_or_init(built)
}

/// The bwrap that can build [`ALONE`] here, proved by running `true` in
/// it, or why none can.
fn built() -> Result<String, Unavailable> {
    let bwrap = brokkr_protocol::hands::require_bwrap().map_err(Unavailable::NoBwrap)?;
    let bwrap = bwrap.to_str().expect("bwrap's path is UTF-8").to_owned();
    let tried = Command::new(&bwrap)
        .args(ALONE)
        .arg("true")
        .stdin(Stdio::null())
        .output();
    let tried = match tried {
        Ok(tried) => tried,
        Err(error) => return Err(Unavailable::DoesNotRun { bwrap, error }),
    };
    match tried.status.success() {
        true => Ok(bwrap),
        false => Err(Unavailable::Refused {
            bwrap,
            status: tried.status,
            said: String::from_utf8_lossy(&tried.stderr).trim().to_owned(),
        }),
    }
}

/// Whether a namespace this host cannot build fails the test or skips it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Demand {
    /// [`REQUIRE`] is `1`: the test fails with the reason.
    Required,
    /// [`REQUIRE`] is absent: the test skips with a printed notice.
    Optional,
}

/// A [`REQUIRE`] value that is neither `1` nor absent.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("{require} is `1` or absent, not {0:?}", require = REQUIRE)]
pub(crate) struct Undeclared(pub(crate) OsString);

/// The demand [`REQUIRE`]'s `value` makes. Only an absent one is
/// optional: an empty value, as a workflow expression that yields nothing
/// leaves it, or a typo, is refused, so nothing but absence turns a
/// required proof into a skip.
pub(crate) fn demanded(value: Option<&OsStr>) -> Result<Demand, Undeclared> {
    match value {
        None => Ok(Demand::Optional),
        Some(value) if value == "1" => Ok(Demand::Required),
        Some(other) => Err(Undeclared(other.to_owned())),
    }
}

/// The demand this process's environment makes; a value it refuses fails
/// the test with why.
fn demand() -> Demand {
    demanded(std::env::var_os(REQUIRE).as_deref()).unwrap_or_else(|refused| panic!("{refused}"))
}

/// A test that needs a namespace this host cannot build: under `demand`
/// it fails with why, or skips with a notice and runs nothing.
pub(crate) fn unisolated(demand: Demand, unavailable: &Unavailable) {
    let reason = format!(
        "a test that delivers a real signal needs a PID namespace of its own, and none can be \
         built here: {unavailable}"
    );
    match demand {
        Demand::Required => panic!("{REQUIRE} is 1, so this host must build it: {reason}"),
        Demand::Optional => eprintln!("skipped: {reason}"),
    }
}
