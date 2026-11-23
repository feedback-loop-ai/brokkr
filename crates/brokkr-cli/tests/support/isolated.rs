//! The built `brokkr doctor`, alone in a PID namespace of its own
//! (#1137i, Z1 and Z2): `crate::built`'s door for the verbs whose run
//! reaches doctor's version probe, which delivers real kills.
//!
//! Inside the namespace a guard ([`GUARD`]) reads and parses the
//! namespace's identity as `pid:[N]` before doctor starts, and refuses,
//! recording its typed reason ([`crate::pidns::Isolation`]), one it cannot
//! read or parse, or its parent's. Whatever ran the command, the test
//! fails with that reason when the command is dropped.
//!
//! Only an [`Alone`] builds one, and a test has one only where this host
//! can build the namespace, so a test cannot reach doctor without first
//! having skipped or failed where it cannot: [`alone`]. macOS has no PID
//! namespace, so there every such test skips with a printed notice.

use std::ffi::OsStr;
use std::process::Command;

impl Alone {
    /// The built brokkr given `argv`, a verb that reaches doctor's probe
    /// first, alone in a PID namespace of its own.
    pub(crate) fn brokkr<I, S>(&self, argv: I) -> Isolated
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let argv = crate::built::verbed(argv);
        assert!(
            crate::built::PROBING.iter().any(|verb| argv[0] == *verb),
            "{:?} reaches no probe: run it through built::brokkr",
            argv[0]
        );
        self.running(OsStr::new(crate::built::BIN), argv)
    }

    /// `program` given `argv`, alone in a PID namespace of its own.
    pub(crate) fn running<I, S>(&self, program: &OsStr, argv: I) -> Isolated
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        let (mut command, apart) = self.launcher();
        command.arg(program).args(argv);
        Isolated { command, apart }
    }
}

/// A command alone in a PID namespace of its own, behind its guard: a
/// [`Command`] to configure and run as any other.
pub(crate) struct Isolated {
    command: Command,
    apart: Apart,
}

impl Isolated {
    /// Fail the test, with its reason, if the guard refused.
    pub(crate) fn judged(&self) {
        self.apart.judged();
    }
}

impl std::ops::Deref for Isolated {
    type Target = Command;

    fn deref(&self) -> &Command {
        &self.command
    }
}

impl std::ops::DerefMut for Isolated {
    fn deref_mut(&mut self) -> &mut Command {
        &mut self.command
    }
}

impl Drop for Isolated {
    /// Whatever ran it, a refusal is the test's failure; an unwinding
    /// test has failed already.
    fn drop(&mut self) {
        if !std::thread::panicking() {
            self.judged();
        }
    }
}

/// Proof that this host builds a PID namespace, held by a test that may
/// therefore run doctor.
#[cfg(target_os = "linux")]
#[derive(Clone, Copy)]
pub(crate) struct Alone {
    bwrap: &'static str,
    outer: u64,
}

/// A test's [`Alone`], or `None` once a host that cannot build the
/// namespace has skipped the test; where `crate::pidns::REQUIRE` demands
/// one, such a host fails it instead. This process's own identity,
/// unreadable or unparsable, refuses.
#[cfg(target_os = "linux")]
pub(crate) fn alone() -> Option<Alone> {
    use crate::pidns;
    let bwrap = pidns::isolating()?;
    let outer = pidns::identity().unwrap_or_else(|refused| panic!("{}: {refused}", pidns::REFUSED));
    Some(Alone { bwrap, outer })
}

#[cfg(target_os = "linux")]
impl Alone {
    /// The launcher a namespace's command follows, and its guard.
    fn launcher(&self) -> (Command, Apart) {
        let guard = Guard::of(self.outer, std::path::Path::new(crate::pidns::IDENTITY));
        let mut command = Command::new(self.bwrap);
        command.args(crate::pidns::ALONE);
        guard.wrap(&mut command);
        (command, Apart(guard))
    }
}

/// The guard of a command alone in its namespace.
#[cfg(target_os = "linux")]
struct Apart(Guard);

#[cfg(target_os = "linux")]
impl Apart {
    fn judged(&self) {
        if let Some(refused) = self.0.refused() {
            panic!("{}: {refused}", crate::pidns::REFUSED);
        }
    }
}

/// Elsewhere there is no PID namespace, so no test holds an [`Alone`].
#[cfg(not(target_os = "linux"))]
#[derive(Clone, Copy)]
pub(crate) enum Alone {}

/// Elsewhere every test that runs doctor skips.
#[cfg(not(target_os = "linux"))]
pub(crate) fn alone() -> Option<Alone> {
    eprintln!(
        "skipped: a test that runs brokkr doctor needs a PID namespace of its own, and {} has \
         none",
        std::env::consts::OS
    );
    None
}

#[cfg(not(target_os = "linux"))]
impl Alone {
    fn launcher(&self) -> (Command, Apart) {
        match *self {}
    }
}

/// Elsewhere no command is ever apart.
#[cfg(not(target_os = "linux"))]
enum Apart {}

#[cfg(not(target_os = "linux"))]
impl Apart {
    fn judged(&self) {
        match *self {}
    }
}

/// The shell a guarded run starts inside its namespace: `$0` is where it
/// records why it refused, `$1` the parent's namespace number and `$2`
/// the link it reads its own from. It reads and parses its own as
/// `pid:[N]`, refuses one unreadable, unparsable or its parent's, and
/// only then hands the rest to its program, through `timeout` where
/// [`LIMIT`] names seconds, so the deadline's kill is sent inside the
/// namespace and nowhere else.
#[cfg(target_os = "linux")]
pub(crate) const GUARD: &str = "refuse() { printf '%s\\n' \"$1\" > \"$0\"; exit 125; }; \
    own=$(/usr/bin/readlink \"$2\") || refuse \"unreadable $2\"; \
    case $own in \"pid:[\"*[!0-9]*\"]\" | \"pid:[]\") refuse \"unparsable $own\";; \
    \"pid:[\"*\"]\") ;; *) refuse \"unparsable $own\";; esac; \
    [ \"$own\" != \"pid:[$1]\" ] || refuse \"shared $1\"; shift 2; \
    limit=${BOUNDED_SECONDS-}; unset BOUNDED_SECONDS; \
    [ -z \"$limit\" ] || exec /usr/bin/timeout -s KILL \"$limit\" \"$@\"; exec \"$@\"";

/// The variable that gives a guarded run its deadline in seconds; the
/// guard removes it before its program starts.
#[cfg(target_os = "linux")]
pub(crate) const LIMIT: &str = "BOUNDED_SECONDS";

/// A deadline as whole seconds for `timeout`, rounded up, and never 0,
/// which `timeout` reads as no deadline at all (#1137i, Z4).
#[cfg(target_os = "linux")]
pub(crate) fn seconds(limit: std::time::Duration) -> String {
    let rounded = limit.as_secs() + u64::from(limit.subsec_nanos() > 0);
    rounded.max(1).to_string()
}

/// [`GUARD`]'s inputs, and the record of why it refused.
#[cfg(target_os = "linux")]
pub(crate) struct Guard {
    outer: u64,
    link: std::path::PathBuf,
    record: tempfile::TempDir,
}

#[cfg(target_os = "linux")]
impl Guard {
    /// A guard that holds its child's namespace apart from `outer`, read
    /// from `link`.
    pub(crate) fn of(outer: u64, link: &std::path::Path) -> Guard {
        let record = tempfile::tempdir().expect("a guard's record has a directory");
        let link = link.to_path_buf();
        Guard {
            outer,
            link,
            record,
        }
    }

    /// `launcher` given the guard: what follows it is the program the
    /// guard runs once it has proved the namespace.
    pub(crate) fn wrap(&self, launcher: &mut Command) {
        launcher.args(["/bin/sh", "-c", GUARD]);
        launcher.arg(self.record.path().join("refused"));
        launcher.arg(self.outer.to_string()).arg(&self.link);
    }

    /// Why the guard refused, if it did.
    pub(crate) fn refused(&self) -> Option<crate::pidns::Isolation> {
        use crate::pidns::Isolation;
        let said = match std::fs::read_to_string(self.record.path().join("refused")) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return None,
            said => said.unwrap_or_else(|error| panic!("the guard's record reads: {error}")),
        };
        let said = said.trim_end_matches('\n');
        Some(match said.split_once(' ') {
            Some(("unreadable", what)) => Isolation::Unreadable(what.to_owned()),
            Some(("unparsable", read)) => Isolation::Unparsable(read.to_owned()),
            Some(("shared", number)) => match number.parse() {
                Ok(number) => Isolation::Shared(number),
                Err(_) => panic!("the guard recorded no namespace number: {said:?}"),
            },
            _ => panic!("the guard refused in words it does not use: {said:?}"),
        })
    }
}

/// The drop's judgment, proved where no namespace is built at all.
#[cfg(target_os = "linux")]
mod tests {
    use super::*;

    /// Z2: a command its guard refused fails its test when it is dropped,
    /// whatever ran it: here one launched with no namespace, in this
    /// process's own, which the guard refuses before its program starts.
    #[test]
    #[should_panic(
        expected = "a test that delivers a real signal refused before it delivered \
                               anything: it runs in its parent's PID namespace, pid:["
    )]
    fn a_command_its_guard_refused_fails_its_test_when_dropped() {
        let outer = crate::pidns::identity().unwrap();
        let guard = Guard::of(outer, std::path::Path::new(crate::pidns::IDENTITY));
        let mut command = Command::new("/usr/bin/env");
        guard.wrap(&mut command);
        command.arg("/bin/true");
        let mut isolated = Isolated {
            command,
            apart: Apart(guard),
        };
        assert_eq!(isolated.status().unwrap().code(), Some(125));
    }
}
