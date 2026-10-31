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
    let host = Host::of(std::env::vars_os(), std::env::current_exe());
    let home = host.as_ref().ok().map(|host| host.home.clone());
    let host = host.and_then(|host| host.served(super::terminal::legacy_tiocsti()));
    let open = Root::of;
    Observed {
        host,
        survey: &surveyed,
        home,
        open,
    }
}
