//! What provisioning reads of the host (decision 0065 slice two, U7c and
//! U9b1), moved out of `broker.rs`, which sits at its 800 lines (operator
//! ruling of 2026-10-26): the trusted HOME, the current Brokkr binary and
//! the managed writers' uid, and whether the host lets a process inject
//! input into a terminal it does not own.
//!
//! A server box keeps the broker's session (MB5) and its bootstrap gives
//! up the terminal (U6c8), but an ancestor's terminal stays reachable to a
//! process that can push bytes into it with `TIOCSTI`. Linux switches that
//! off with `/proc/sys/dev/tty/legacy_tiocsti` reading `0`. Where the
//! setting is absent, unreadable or anything else, MCP servers refuse to
//! start before any survey or server does (operator ruling 4 of
//! 2026-10-18, placed at provisioning by the ruling of 2026-10-26). The
//! setting is read where dispatch observes the host, and judged here.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::PathBuf;

use brokkr_protocol::hands;

use super::Unprovisioned;

/// What provisioning reads of the host: the trusted canonical HOME, the
/// current Brokkr binary, which surveys each box and is each plan's
/// bootstrap, and the managed writers' uid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::engine) struct Host {
    pub(in crate::engine) home: PathBuf,
    pub(in crate::engine) bootstrap: PathBuf,
    pub(in crate::engine) writer: u32,
}

impl Host {
    /// The host `environment` and `this` describe: HOME as the broker
    /// reads it from the environment the engine hands down —
    /// `hands::home_dir` over its UTF-8 entries — absolute and canonical;
    /// this binary, canonical; and the engine's uid, which every managed
    /// seat runs as. The entries kept are the broker's own rule
    /// (`session::trusted_home`, CLI-private), which
    /// `the_host_is_the_home_and_binary_it_is_given_canonical` binds.
    pub(in crate::engine) fn of(
        environment: impl IntoIterator<Item = (OsString, OsString)>,
        this: std::io::Result<PathBuf>,
    ) -> Result<Host, Unprovisioned> {
        let environment: BTreeMap<String, String> = (environment.into_iter())
            .filter_map(|(key, value)| Some((key.into_string().ok()?, value.into_string().ok()?)))
            .collect();
        let home = Some(hands::home_dir(&environment)).filter(|home| home.is_absolute());
        let home = home.and_then(|home| std::fs::canonicalize(home).ok());
        let home = home.ok_or(Unprovisioned::Root)?;
        let bootstrap = this.and_then(std::fs::canonicalize);
        Ok(Host {
            home,
            bootstrap: bootstrap.or(Err(Unprovisioned::Bootstrap))?,
            writer: hands::ids().0,
        })
    }

    /// This host, where the legacy TIOCSTI `setting`, as read
    /// ([`legacy_tiocsti`]), is switched off: exactly `0`, with or without
    /// the newline the kernel ends it with. A setting that is absent or
    /// unreadable is not known to be off. Provisioning alone reads the
    /// result, so a refusal reaches only an attempt that brokers a holding;
    /// admission's HOME is read from [`Host::of`] before it.
    pub(in crate::engine) fn served(
        self,
        setting: std::io::Result<Vec<u8>>,
    ) -> Result<Host, Unprovisioned> {
        match setting.as_deref() {
            Ok(b"0\n" | b"0") => Ok(self),
            Ok(_) | Err(_) => Err(Unprovisioned::Terminal),
        }
    }
}

/// The host's legacy TIOCSTI setting, as read.
#[cfg(target_os = "linux")]
pub(super) fn legacy_tiocsti() -> std::io::Result<Vec<u8>> {
    std::fs::read("/proc/sys/dev/tty/legacy_tiocsti")
}

/// Off Linux no such setting exists, so none is known to be off; the
/// standing fence keeps every `mcp` grant from compiling there.
#[cfg(not(target_os = "linux"))]
pub(super) fn legacy_tiocsti() -> std::io::Result<Vec<u8>> {
    Err(std::io::ErrorKind::Unsupported.into())
}
