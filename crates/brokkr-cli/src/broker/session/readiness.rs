//! The admitted box launched on its waiting bootstrap, and its readiness
//! received (decision 0065 slice two, U6c6b; MB3, MB5, SD3). The box the
//! observer checked is launched from the very handles it handed back, on
//! the private `broker bootstrap` verb, which the box enters with its two
//! control pipes and nothing else of the broker's. bubblewrap names the
//! box's first process, by its host pid, on its info pipe. The broker
//! writes the sealed intent on the control pipe and reads the one ready
//! message, at most 4 KiB, until the bootstrap closes the ready pipe: a
//! duplicate, truncated or excess message is no readiness. The message must
//! echo the plan and source-set digests, and name, by its pid in the box, a
//! child of that first process, whose launcher is the broker's own child,
//! in exactly the namespaces the host shows it in. The startup's absolute
//! deadline and the attempt's cancellation bound every wait. The box is
//! settled either way, and readiness admits nothing yet: serving still
//! refuses before any secret is looked up or a dialect server is started.

use std::collections::BTreeSet;
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::time::Instant;

use brokkr_protocol::broker::{BoxIntent, Privilege, Refusal};
use brokkr_protocol::hands::ServerEntry;

use super::{ensure, Record};

#[cfg(target_os = "linux")]
pub(super) use linux::ready;

#[cfg(all(test, target_os = "linux"))]
mod tests;

/// What admission hands serving: each checked source's handle, by the host
/// path that named it, held until the launch mounts it; the store's
/// admitted handle, held for its reader and never mounted (MB4); and the
/// box's entry, which names those handles by their places and removes the
/// generated identity's tree when this is dropped (MB5).
pub(in crate::broker) struct Admitted {
    handles: Vec<(PathBuf, OwnedFd)>,
    _store: OwnedFd,
    entry: ServerEntry,
}

/// The startup's absolute deadline (MB3), which observation and readiness
/// share and never extend, and the attempt's cancellation: whether the
/// broker's own starter has ended, which the launch's watcher asks from a
/// thread of its own.
pub(super) type Startup<'a> = (Instant, &'a (dyn Fn() -> bool + Sync));

/// The observer's `record` against what `intent` sealed, with the
/// `handles` it handed back: its own refusal; identity unprotected where
/// the observed sources are not the sealed ones or a writer it observed is
/// not sealed (its writers, the observed joined with the sealed, are not
/// exactly the sealed set), which comes before any cause after it in MB3's
/// order, or where the handles are not those it names and then the
/// store's, or no entry names them; else the cause after the box, or the
/// admitted handles and entry. The sealed facts were bound with the plan,
/// so a difference here is the filesystem's, never the plan's (SC1). A
/// refused record's entry goes with it, and with it the identity tree it
/// handed over.
pub(super) fn compared(
    intent: &BoxIntent,
    record: Record,
    mut handles: Vec<OwnedFd>,
) -> Result<Admitted, Refusal> {
    let observation = match record {
        Record::Refused(cause) => return Err(cause.refusal()),
        Record::Observed(observation) => observation,
    };
    let sealed = &intent.sources;
    let observed = (observation.entries, observation.mounts, &observation.digest);
    let sources = observed == (sealed.entries, sealed.mounts, &sealed.digest);
    let writers = match intent.writers.privilege {
        Privilege::Confined => observation.writers.as_ref().is_some_and(|uids| {
            let set = |uids: &[u32]| uids.iter().copied().collect::<BTreeSet<u32>>();
            set(uids) == set(&intent.writers.uids)
        }),
    };
    ensure(sources & writers, Refusal::Identity)?;
    if let Some(cause) = observation.refused {
        return Err(cause.refusal());
    }
    let store = handles
        .pop()
        .filter(|_| observation.handles.len() == handles.len());
    let store = store.ok_or(Refusal::Identity)?;
    Ok(Admitted {
        handles: observation.handles.into_iter().zip(handles).collect(),
        _store: store,
        entry: observation.entry.ok_or(Refusal::Identity)?,
    })
}

/// Off Linux no box stands, so none is launched.
#[cfg(not(target_os = "linux"))]
pub(super) fn ready(
    admitted: &Admitted,
    _: (&BoxIntent, &str),
    _: Startup<'_>,
) -> Result<(), Refusal> {
    let _ = (&admitted.handles, &admitted.entry);
    Err(Refusal::Unavailable)
}

/// The launch and the receiver, which read Linux's `/proc` and device
/// numbers.
#[cfg(target_os = "linux")]
mod linux {
    use std::io::{ErrorKind, PipeReader, PipeWriter, Read, Write};
    use std::os::fd::{AsFd, BorrowedFd};
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::io::RawFd;
    use std::time::{Duration, Instant};

    use brokkr_protocol::broker::{BoxIntent, Network, Refusal};
    use brokkr_protocol::hands::ServerBox;
    use serde::{Deserialize, Serialize};

    use super::{Admitted, Startup};
    use crate::broker::session::ensure;

    /// The most bytes the ready message holds, as the intent frame's body
    /// does (MB3).
    pub(super) const FRAME_MAX: usize = 4096;

    /// The most bytes bubblewrap's info holds.
    const INFO_MAX: usize = 4096;

    /// The longest one wait on a pipe lasts before the deadline and the
    /// cancellation are read again.
    const SLICE: Duration = Duration::from_millis(10);

    /// The sealed intent the broker writes on the control pipe: the plan
    /// and source-set digests readiness binds, the network the dialect's
    /// egress projects to, the broker's own namespaces, and every mount the
    /// box binds, by its device and inode. The bootstrap reads it closed.
    #[derive(Serialize)]
    pub(super) struct Intent<'a> {
        pub(super) plan: &'a str,
        pub(super) sources: &'a str,
        pub(super) network: Egress,
        pub(super) host: Spaces,
        pub(super) mounts: Vec<Source>,
    }

    /// The network as the bootstrap's intent names it.
    #[derive(Serialize)]
    #[serde(rename_all = "lowercase")]
    pub(super) enum Egress {
        Isolated,
        Shared,
    }

    /// One mount of the box: where it stands in the box, and the device
    /// (`major:minor`) and inode of the checked handle it mounts.
    #[derive(Serialize)]
    pub(super) struct Source {
        pub(super) path: String,
        pub(super) device: (u32, u32),
        pub(super) inode: u64,
    }

    /// A process's namespaces, by the inode `/proc` gives each.
    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Spaces {
        pub(super) mnt: u64,
        pub(super) pid: u64,
        pub(super) net: u64,
        pub(super) ipc: u64,
        pub(super) uts: u64,
    }

    /// The one ready message (MB3): the digests the intent sealed, the
    /// bootstrap by its pid in the box, and the namespaces it observed
    /// itself in.
    #[derive(Debug, Deserialize)]
    #[serde(deny_unknown_fields)]
    pub(super) struct Ready {
        pub(super) plan: String,
        pub(super) sources: String,
        pub(super) pid: u32,
        pub(super) namespaces: Spaces,
    }

    /// What bubblewrap's info names: the box's first process, by its host
    /// pid. Its other keys, which vary by version, are no authority here.
    #[derive(Deserialize)]
    struct Info {
        #[serde(rename = "child-pid")]
        child: u32,
    }

    /// What the host shows of a launched box: the parent of its first
    /// process, and the namespaces of that process's child the ready
    /// message names by its pid in the box, where there is one.
    #[derive(Debug, PartialEq)]
    pub(super) struct Seen {
        pub(super) parent: Option<u32>,
        pub(super) bootstrap: Option<Spaces>,
    }

    /// Launch `admitted`'s box on its waiting bootstrap and receive its
    /// readiness for `sealed` (the intent and the plan's digest) within
    /// `startup`, the launcher's own start included: the box unavailable
    /// where its launcher cannot start, and not established where its start
    /// overruns, the started launcher names no first process,
    /// the intent cannot be sent or the readiness is not the owned
    /// bootstrap's, whole, for this plan and these sources. The box is
    /// settled once this returns, either way.
    pub(in crate::broker) fn ready(
        admitted: &Admitted,
        (intent, plan): (&BoxIntent, &str),
        startup: Startup<'_>,
    ) -> Result<(), Refusal> {
        let handles = admitted.handles.iter().map(|(_, fd)| fd.as_fd());
        let handles: Vec<BorrowedFd<'_>> = handles.collect();
        let mut launched = ServerBox::launch(&admitted.entry, &handles, bootstrap, startup)?;
        let first = first(&read(&mut launched.info, INFO_MAX, startup)?)?;
        let intent = Intent {
            plan,
            sources: &intent.sources.digest,
            network: egress(&intent.network),
            host: spaces("self").ok_or(Refusal::Establishment)?,
            mounts: mounts(&launched.mounts, &handles)?,
        };
        sent(&mut launched.control, &framed(&intent)?, startup)?;
        let ready = message(&read(&mut launched.ready, FRAME_MAX, startup)?)?;
        let seen = seen(first, ready.pid);
        decided(&ready, (plan, intent.sources), launched.child.id(), &seen)
    }

    /// The box's first process bubblewrap's `info` names, by its host pid:
    /// the box not established where it names none, as where the launcher
    /// started but could not set the box's namespaces up (MB3). A launcher
    /// that cannot start at all is the launch's to answer, unavailable.
    pub(super) fn first(info: &[u8]) -> Result<u32, Refusal> {
        let info: Info = serde_json::from_slice(info).or(Err(Refusal::Establishment))?;
        Ok(info.child)
    }

    /// The one closed ready message `bytes` hold, and nothing beside it: a
    /// duplicate, a truncated or an excess message is no box established.
    pub(super) fn message(bytes: &[u8]) -> Result<Ready, Refusal> {
        serde_json::from_slice(bytes).or(Err(Refusal::Establishment))
    }

    /// The bootstrap's arguments for its control and ready descriptors.
    fn bootstrap(control: RawFd, ready: RawFd) -> Vec<String> {
        let numbers = [control, ready].map(|fd| fd.to_string());
        let [control, ready] = numbers;
        [
            "broker",
            "bootstrap",
            "--control",
            &control,
            "--ready",
            &ready,
        ]
        .map(String::from)
        .to_vec()
    }

    /// The network the bootstrap's intent names.
    pub(super) fn egress(network: &Network) -> Egress {
        match network {
            Network::Isolated => Egress::Isolated,
            Network::Shared => Egress::Shared,
        }
    }

    /// Each mount of the box, by the device and inode of the handle at its
    /// place among `handles`.
    fn mounts(
        mounted: &[(usize, String)],
        handles: &[BorrowedFd<'_>],
    ) -> Result<Vec<Source>, Refusal> {
        let mount = |(place, path): &(usize, String)| {
            let handle = handles.get(*place).ok_or(Refusal::Identity)?;
            let held = rustix::fs::fstat(handle).or(Err(Refusal::Identity))?;
            let device = (
                rustix::fs::major(held.st_dev),
                rustix::fs::minor(held.st_dev),
            );
            Ok(Source {
                path: path.clone(),
                device,
                inode: held.st_ino,
            })
        };
        mounted.iter().map(mount).collect()
    }

    /// The intent as one control frame: a four-byte big-endian length, then
    /// that many bytes, at most [`FRAME_MAX`], as the bootstrap reads it.
    pub(super) fn framed(intent: &Intent<'_>) -> Result<Vec<u8>, Refusal> {
        let body = serde_json::to_vec(intent).or(Err(Refusal::Establishment))?;
        ensure(body.len() <= FRAME_MAX, Refusal::Establishment)?;
        let length = u32::try_from(body.len()).or(Err(Refusal::Establishment))?;
        Ok([&length.to_be_bytes()[..], &body].concat())
    }

    /// Every byte `pipe` holds until each of its writers has closed it, at
    /// most `most` of them, read within `startup` ([`polled`]); the box not
    /// established otherwise.
    pub(super) fn read(
        pipe: &mut PipeReader,
        most: usize,
        startup: Startup<'_>,
    ) -> Result<Vec<u8>, Refusal> {
        rustix::io::ioctl_fionbio(&*pipe, true).or(Err(Refusal::Establishment))?;
        let mut bytes = Vec::new();
        let mut chunk = [0; 1024];
        polled(startup, || {
            let read = pipe.read(&mut chunk)?;
            bytes.extend_from_slice(&chunk[..read]);
            match bytes.len() <= most {
                true => Ok(read == 0),
                false => Err(ErrorKind::FileTooLarge.into()),
            }
        })?;
        Ok(bytes)
    }

    /// Every byte of `frame` written to `pipe` within `startup`
    /// ([`polled`]), however long the bootstrap leaves the pipe full; the
    /// box not established otherwise.
    pub(super) fn sent(
        pipe: &mut PipeWriter,
        frame: &[u8],
        startup: Startup<'_>,
    ) -> Result<(), Refusal> {
        rustix::io::ioctl_fionbio(&*pipe, true).or(Err(Refusal::Establishment))?;
        let mut left = frame;
        polled(startup, || {
            left = &left[pipe.write(left)?..];
            Ok(left.is_empty())
        })
    }

    /// `step` taken until it is done, within `startup`'s absolute deadline
    /// unless the attempt is cancelled, waiting at most [`SLICE`] wherever
    /// it would block; any other error, the deadline or the cancellation
    /// leaves the box not established.
    fn polled(
        (deadline, cancelled): Startup<'_>,
        mut step: impl FnMut() -> std::io::Result<bool>,
    ) -> Result<(), Refusal> {
        while (Instant::now() < deadline) & !cancelled() {
            match step() {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(error) => {
                    let blocked = error.kind() == ErrorKind::WouldBlock;
                    ensure(blocked, Refusal::Establishment)?;
                    std::thread::sleep(SLICE);
                }
            }
        }
        Err(Refusal::Establishment)
    }

    /// The namespaces `/proc` shows the process `pid` in, by inode, in
    /// [`Spaces`]'s order.
    fn spaces(pid: &str) -> Option<Spaces> {
        let inodes = ["mnt", "pid", "net", "ipc", "uts"].map(|space| {
            let held = std::fs::metadata(format!("/proc/{pid}/ns/{space}"));
            held.ok().map(|held| held.ino())
        });
        let [mnt, pid, net, ipc, uts] = inodes;
        let (mnt, pid, net, ipc, uts) = (mnt?, pid?, net?, ipc?, uts?);
        Some(Spaces {
            mnt,
            pid,
            net,
            ipc,
            uts,
        })
    }

    /// What the host shows of the box whose first process is `first`, and
    /// of that process's child whose pid in the box is `pid`.
    fn seen(first: u32, pid: u32) -> Seen {
        let children = std::fs::read_to_string(format!("/proc/{first}/task/{first}/children"));
        let children = children.unwrap_or_default();
        let named = children
            .split_whitespace()
            .find(|child| boxed_pid(child) == Some(pid));
        Seen {
            parent: parent(first),
            bootstrap: named.and_then(spaces),
        }
    }

    /// The parent of `pid`, from `/proc/<pid>/stat`.
    fn parent(pid: u32) -> Option<u32> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        let mut fields = stat.rsplit_once(')')?.1.split_whitespace();
        fields.nth(1)?.parse().ok()
    }

    /// The pid `/proc` shows `pid` by in its own, innermost, PID namespace.
    fn boxed_pid(pid: &str) -> Option<u32> {
        let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
        let line = status
            .lines()
            .find_map(|line| line.strip_prefix("NSpid:"))?;
        line.split_whitespace().last()?.parse().ok()
    }

    /// Whether `ready` is the readiness of the box launched by `launcher`
    /// for the sealed `(plan, sources)` digests, as the host `seen` shows
    /// it: both digests echoed, the box's first process the launcher's own
    /// child, and the bootstrap it names that process's child, in exactly
    /// the namespaces it reports. Anything else is no box established.
    pub(super) fn decided(
        ready: &Ready,
        (plan, sources): (&str, &str),
        launcher: u32,
        seen: &Seen,
    ) -> Result<(), Refusal> {
        let digests = (ready.plan == plan) & (ready.sources == sources);
        let owned = seen.parent == Some(launcher);
        let spaces = seen.bootstrap.as_ref() == Some(&ready.namespaces);
        ensure(digests & owned & spaces, Refusal::Establishment)
    }
}
