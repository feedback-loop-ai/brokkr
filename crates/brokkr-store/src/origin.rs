//! Where a run is driven from (decision 0030): an opaque fingerprint of
//! the machine and the account, written into `runs.origin_host` when a
//! run is created and compared when a session is offered.
//!
//! It is asked once per process and never inside a write transaction.
//! Where no identity file answers, naming the machine can mean running
//! `hostname`, and a process spawned while the shared journal's write
//! lock is held lengthens exactly the hold every peer waits on (#354).

use std::sync::OnceLock;

/// Where a machine says who it is, in the order asked. The machine id
/// where the OS publishes one; the kernel's hostname where it does not.
const MACHINE_SOURCES: [&str; 3] = [
    "/etc/machine-id",
    "/var/lib/dbus/machine-id",
    "/proc/sys/kernel/hostname",
];

/// This process's fingerprint, once asked.
static LOCAL: OnceLock<Option<String>> = OnceLock::new();

/// This machine and this account, as [`host_from`] fingerprints them.
/// Neither changes under a running process, so the first answer is the
/// answer: it is computed once and every later caller reads it.
pub(crate) fn local_host() -> Option<String> {
    LOCAL
        .get_or_init(|| {
            host_from(&MACHINE_SOURCES, || {
                machine_name(&["HOSTNAME", "COMPUTERNAME"], hostname_command)
            })
        })
        .clone()
}

/// An opaque fingerprint of the machine and the account this process is
/// running as: the first `sources` entry that can be read, else what
/// `fallback` answers, folded together with the account's home directory
/// — which is where every driver brokkr ships keeps the credential that
/// OWNS a provider session (`~/.codex`, `~/.claude`). Hashed and clipped,
/// so what lands in the journal file is an equality token rather than an
/// operator's hostname and home path.
///
/// `fallback` is asked only when no source can be read: on Linux,
/// `/etc/machine-id` answers first and nothing is spawned.
///
/// `None` when nothing identifying can be read, and a `None` is never
/// equal to anything — an installation that cannot say where it is
/// hands out no sessions, which is exactly what brokkr did before
/// decision 0030.
pub(crate) fn host_from(
    sources: &[&str],
    fallback: impl FnOnce() -> Option<String>,
) -> Option<String> {
    let machine = sources
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .or_else(fallback)?;
    let machine = machine.trim();
    if machine.is_empty() {
        return None;
    }
    let home = account_home();
    let digest = brokkr_core::canonical::sha256_hex(&serde_json::json!([machine, home]));
    Some(digest[..16].to_string())
}

/// The account's home directory, spelled the way each platform exports
/// it: `HOME` on unix, `USERPROFILE` on Windows. Empty when neither is
/// set, so the fingerprint still folds and the machine half decides.
fn account_home() -> String {
    home_from(&["HOME", "USERPROFILE"])
}

/// The first of `variables` that is set, else empty.
fn home_from(variables: &[&str]) -> String {
    variables
        .iter()
        .find_map(|variable| std::env::var(variable).ok())
        .unwrap_or_default()
}

/// The machine's name where no identity file can be read — the case on
/// every released platform but Linux. The first of `variables` that is
/// set and non-blank wins (`HOSTNAME`, which POSIX shells set but rarely
/// export; `COMPUTERNAME`, which Windows always publishes); failing
/// both, `ask` is consulted once. Blank answers count as none.
pub(crate) fn machine_name(
    variables: &[&str],
    ask: impl FnOnce() -> Option<String>,
) -> Option<String> {
    variables
        .iter()
        .find_map(|variable| std::env::var(variable).ok())
        .filter(|name| !name.trim().is_empty())
        .or_else(ask)
        .filter(|name| !name.trim().is_empty())
}

/// What `hostname` prints: the same spelling `/proc/sys/kernel/hostname`
/// carries on Linux, and the one thing macOS ships that names the
/// machine without a daemon or a crate. `None` when the command is
/// missing or fails, and a `None` hands out no sessions.
fn hostname_command() -> Option<String> {
    hostname_from("hostname")
}

/// `program`'s standard output when it runs and succeeds; `None` when
/// it is missing or exits nonzero.
fn hostname_from(program: &str) -> Option<String> {
    let out = std::process::Command::new(program).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

#[cfg(test)]
mod tests;
