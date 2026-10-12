//! The bootstrap's handshake with the broker (decision 0077; MB3, MB4;
//! U6c6a, U6c8): the sealed intent, the ready message, their bound and the
//! status tags, in the one place both ends read them (decision 0071
//! ruling 5). The bootstrap reads the intent and writes the rest; the
//! broker's readiness receiver writes the intent and reads the rest.

use std::path::PathBuf;

use brokkr_protocol::broker::{Network, Refusal};
use serde::{Deserialize, Serialize};

/// The most bytes one control message holds: the intent frame's body, and
/// the ready message (MB3).
pub(in crate::broker) const FRAME_MAX: usize = 4096;

/// The broker's sealed intent for this box: the plan and source-set
/// digests readiness binds, the network the dialect's egress projects to,
/// the broker's own namespaces, which the box must not share, every mount
/// it binds into the box, and the server the bootstrap execs once its
/// binding frame is received: its canonical executable, its argv and the
/// declared binding names the frame must carry exactly. Names and numbers
/// only: no locator, value or authority.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::broker) struct Intent {
    pub(in crate::broker) plan: String,
    pub(in crate::broker) sources: String,
    pub(in crate::broker) network: Network,
    pub(in crate::broker) host: Spaces,
    pub(in crate::broker) mounts: Vec<Source>,
    pub(in crate::broker) executable: PathBuf,
    pub(in crate::broker) argv: Vec<String>,
    pub(in crate::broker) secrets: Vec<String>,
}

/// One mount the broker binds into the box, nested ones each their own
/// (MB3): where it stands in the box, and the device (`major:minor`) and
/// inode of the checked handle it mounts.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::broker) struct Source {
    pub(in crate::broker) path: PathBuf,
    pub(in crate::broker) device: (u32, u32),
    pub(in crate::broker) inode: u64,
}

/// A process's namespaces, by the inode `/proc` gives each.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::broker) struct Spaces {
    pub(in crate::broker) mnt: u64,
    pub(in crate::broker) pid: u64,
    pub(in crate::broker) net: u64,
    pub(in crate::broker) ipc: u64,
    pub(in crate::broker) uts: u64,
}

/// The one ready message (MB3): the plan and source-set digests the intent
/// sealed, the bootstrap by its process id in the box, and the namespaces
/// it observed itself in.
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::broker) struct Ready {
    pub(in crate::broker) plan: String,
    pub(in crate::broker) sources: String,
    pub(in crate::broker) pid: u32,
    pub(in crate::broker) namespaces: Spaces,
}

/// What the bootstrap reports on its status pipe where the server did not
/// start, at most once and never on stderr (MB3): one tag. A started
/// server writes none, and its exec closes the pipe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(in crate::broker) enum Status {
    HandoffFailed,
    ExecFailed,
}

impl Status {
    /// Every tag.
    pub(in crate::broker) const ALL: [Status; 2] = [Status::HandoffFailed, Status::ExecFailed];

    /// The one byte that carries this tag.
    pub(in crate::broker) fn tag(self) -> u8 {
        match self {
            Status::HandoffFailed => b'H',
            Status::ExecFailed => b'E',
        }
    }

    /// The cause this tag reports.
    pub(in crate::broker) fn refusal(self) -> Refusal {
        match self {
            Status::HandoffFailed => Refusal::Handoff,
            Status::ExecFailed => Refusal::Exec,
        }
    }

    /// The tag `bytes` hold, where they are exactly one.
    pub(in crate::broker) fn read(bytes: &[u8]) -> Option<Status> {
        Status::ALL
            .into_iter()
            .find(|status| bytes == [status.tag()])
    }
}
