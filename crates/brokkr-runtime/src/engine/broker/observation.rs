//! What dispatch observes of the host once for each attempt (decision 0065
//! slice two, U8a2a; decision 0071 ruling 10): observed here, before
//! admission and provisioning decide on it, and handed to each through its
//! seam. Moved out of `broker.rs`, which this would take past its 800
//! lines (operator ruling of 2026-10-11).

use std::path::{Path, PathBuf};

use super::root::Root;
use super::{surveyed, Host, Surveyor, Unprovisioned};

/// The host provisioning reads and the survey it runs, and the HOME
/// admission opens its coordination root under, by `open`, only once its
/// claim is made.
pub(in crate::engine) struct Observed {
    pub(in crate::engine) host: Result<Host, Unprovisioned>,
    pub(in crate::engine) survey: Surveyor<'static>,
    pub(in crate::engine) home: Option<PathBuf>,
    pub(in crate::engine) open: fn(&Path) -> Option<Root>,
}

/// The host as dispatch observes it ([`Observed`]): the one a unit test
/// stood in, as `fence::tests::peer_between` reaches between a fold and an
/// append; the host's otherwise, its root judged from `/` ([`Root::of`]).
pub(in crate::engine) fn observed() -> Observed {
    #[cfg(test)]
    if let Some(observed) = super::admission::tests::observed() {
        return observed;
    }
    observed_under(super::terminal::legacy_tiocsti)
}

/// [`observed`] as the host's legacy TIOCSTI `setting` reads: an attempt's
/// provisioning is handed a host only where it reads switched off, and its
/// admission the HOME whatever it reads.
fn observed_under(setting: impl FnOnce() -> std::io::Result<Vec<u8>>) -> Observed {
    let host = Host::of(std::env::vars_os(), std::env::current_exe());
    let home = host.as_ref().ok().map(|host| host.home.clone());
    let host = host.and_then(|host| host.served(setting()));
    let open = Root::of;
    Observed {
        host,
        survey: &surveyed,
        home,
        open,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Ruling 4's production wiring: whatever this host's own setting reads,
    /// dispatch hands provisioning a host only where the setting reads
    /// switched off; absent, unreadable or on, the terminal's cause, and
    /// admission keeps its HOME in every case.
    #[test]
    fn dispatch_hands_provisioning_a_host_only_where_the_setting_reads_off() {
        let observed = |setting: std::io::Result<Vec<u8>>| {
            let observed = observed_under(move || setting);
            let host = observed.host.map(|host| host.home);
            let home = observed.home.expect("this process names a HOME");
            assert_eq!(host.as_ref().ok().map_or(&home, |host| host), &home);
            host.err()
        };
        assert_eq!(observed(Ok(b"0\n".to_vec())), None);
        for setting in [
            Ok(b"1\n".to_vec()),
            Err(std::io::ErrorKind::NotFound.into()),
            Err(std::io::ErrorKind::PermissionDenied.into()),
        ] {
            assert_eq!(observed(setting), Some(Unprovisioned::Terminal));
        }
    }
}
