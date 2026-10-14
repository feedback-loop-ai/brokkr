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
//! in exactly the namespaces the host shows it in. Only then are the
//! declared bindings looked up in the admitted store's handle and sent as
//! one binding frame on the control pipe, which the broker then closes, and
//! the bootstrap's status pipe read to its end: a tag is the handoff's or
//! the exec's cause, and its end permits only the server's initialization
//! (U6c8), which the box's live stdio is handed on to (U6e), and then its
//! serving, beside its drained standard error (U6f1b). The startup's
//! absolute deadline and the attempt's cancellation bound every wait. The
//! box is settled once that has answered, either way, its server with it.

use std::collections::BTreeSet;
use std::io::{ErrorKind, PipeReader, PipeWriter};
use std::os::fd::OwnedFd;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use brokkr_protocol::broker::{BoxIntent, Privilege, Refusal};
use brokkr_protocol::hands::ServerEntry;

use super::{ensure, Record};

#[cfg(target_os = "linux")]
pub(super) use linux::ready;

#[cfg(all(test, target_os = "linux"))]
mod tests;

/// What admission hands serving: each checked source's handle, by the host
/// path that named it, held until the launch mounts it; the store's
/// admitted handle, which the binding sender reads and the box never
/// holds (MB4); and the
/// box's entry, which names those handles by their places and removes the
/// generated identity's tree when this is dropped (MB5).
pub(in crate::broker) struct Admitted {
    handles: Vec<(PathBuf, OwnedFd)>,
    store: OwnedFd,
    entry: ServerEntry,
}

/// The startup's absolute deadline (MB3), which observation, readiness and
/// the server's initialization share and never extend, and the attempt's
/// cancellation: whether the broker's own starter has ended, which the
/// launch's watcher asks from a thread of its own.
pub(in crate::broker) type Startup<'a> = (Instant, &'a (dyn Fn() -> bool + Sync));

/// A launched box's server's standard input and output, its only stdio,
/// as the protocol takes them over (U6e).
pub(in crate::broker) type Link<'l> = (&'l mut PipeWriter, &'l mut PipeReader);

/// What a binding sender's supervisor takes of what it looked up (MB4):
/// nothing, where it began no lookup or its lookup refused; what it
/// published; or, where it was given up on, or failed, after its lookup
/// began and before it published, nothing known of what it holds, so that
/// no later record can be guarded by it, and none is made.
#[derive(Debug, PartialEq)]
pub(in crate::broker) enum Handed<P> {
    Nothing,
    Published(P),
    Uncertain,
}

/// The longest one wait on a pipe lasts before the deadline and the
/// cancellation are read again.
const SLICE: Duration = Duration::from_millis(10);

/// The first wait once something has moved on a pipe.
const BRIEF: Duration = Duration::from_micros(50);

/// How long the next wait lasts where nothing moved: [`BRIEF`] once
/// something has, then twice the last, up to [`SLICE`]. A pipe whose other
/// end is taking or giving is followed as fast as it moves, never a pipe's
/// worth a [`SLICE`], and an idle one costs a wait of [`SLICE`].
pub(in crate::broker) struct Pace(Duration);

impl Pace {
    pub(in crate::broker) fn new() -> Pace {
        Pace(BRIEF)
    }

    /// Wait, where nothing `moved`, the next wait; where something did,
    /// not at all, the next wait [`BRIEF`] again.
    pub(in crate::broker) fn after(&mut self, moved: bool) {
        if moved {
            self.0 = BRIEF;
            return;
        }
        std::thread::sleep(self.0);
        self.0 = (self.0 * 2).min(SLICE);
    }
}

/// Whether `startup`'s deadline is still ahead, the attempt not cancelled.
pub(in crate::broker) fn timely((deadline, cancelled): Startup<'_>) -> bool {
    (Instant::now() < deadline) & !cancelled()
}

/// Whether `step` was taken until it was done, while `startup` is
/// [`timely`]: at once again where it moved something, and after the
/// [`Pace`]'s wait wherever it would block; not where any other error, the
/// deadline or the cancellation came first.
pub(in crate::broker) fn polled(
    startup: Startup<'_>,
    mut step: impl FnMut() -> std::io::Result<bool>,
) -> bool {
    let mut pace = Pace::new();
    while timely(startup) {
        match step() {
            Ok(true) => return true,
            Ok(false) => pace.after(true),
            Err(error) if error.kind() == ErrorKind::WouldBlock => pace.after(false),
            Err(_) => return false,
        }
    }
    false
}

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
        store,
        entry: observation.entry.ok_or(Refusal::Identity)?,
    })
}

/// Off Linux no box stands, so none is launched and nothing is served.
#[cfg(not(target_os = "linux"))]
pub(super) fn ready<T, B, E: From<Refusal>>(
    admitted: &Admitted,
    _: (&brokkr_protocol::broker::Plan, &str),
    _: Startup<'_>,
    _: impl FnOnce(Handed<Vec<brokkr_protocol::secret::BoundSecret>>) -> Result<B, E>,
    _: impl FnOnce(Link<'_>, &mut PipeReader, B) -> Result<T, E>,
) -> Result<T, E> {
    let _ = (&admitted.handles, &admitted.store, &admitted.entry);
    Err(Refusal::Unavailable.into())
}

/// The launch and the receiver, which read Linux's `/proc` and device
/// numbers.
#[cfg(target_os = "linux")]
mod linux {
    use std::io::{ErrorKind, PipeReader, PipeWriter, Read, Write};
    use std::os::fd::{AsFd, BorrowedFd};
    use std::os::unix::fs::MetadataExt;
    use std::os::unix::io::RawFd;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex, PoisonError};

    use brokkr_protocol::broker::{Plan, Refusal};
    use brokkr_protocol::hands::ServerBox;
    use brokkr_protocol::secret;
    use serde::Deserialize;

    use super::{polled, Admitted, Handed, Link, Startup};
    use crate::broker::bootstrap::handshake::Status;
    pub(super) use crate::broker::bootstrap::handshake::{
        Intent, Ready, Source, Spaces, FRAME_MAX,
    };
    use crate::broker::session::ensure;

    /// The most bytes bubblewrap's info holds.
    const INFO_MAX: usize = 4096;

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

    /// Launch `admitted`'s box on its waiting bootstrap, receive its
    /// readiness for the plan sealed at `digest` within `startup`, the
    /// launcher's own start included, then hand it the server's bindings
    /// and see the server start ([`delivered`]): the box unavailable where
    /// its launcher cannot start, and not established where its start
    /// overruns, the started launcher names no first process, the intent
    /// cannot be sent or the readiness is not the owned bootstrap's, whole,
    /// for this plan and these sources. The bindings looked up are given
    /// to `bind` as soon as they are ([`delivered`]). Only then is the
    /// server's stdio handed on, live, to `served` (U6e), with its standard
    /// error and what `bind` answered (U6f1b), and its answer is this one.
    /// The box is settled once this returns, either way, its server with
    /// it.
    pub(in crate::broker) fn ready<T, B, E: From<Refusal>>(
        admitted: &Admitted,
        (plan, digest): (&Plan, &str),
        startup: Startup<'_>,
        bind: impl FnOnce(Handed<Vec<secret::BoundSecret>>) -> Result<B, E>,
        served: impl FnOnce(Link<'_>, &mut PipeReader, B) -> Result<T, E>,
    ) -> Result<T, E> {
        let handles = admitted.handles.iter().map(|(_, fd)| fd.as_fd());
        let handles: Vec<BorrowedFd<'_>> = handles.collect();
        let mut launched = ServerBox::launch(&admitted.entry, &handles, bootstrap, startup)?;
        let first = first(&read(&mut launched.info, INFO_MAX, startup)?)?;
        let sealed = &plan.intent;
        let intent = Intent {
            plan: digest.to_string(),
            sources: sealed.sources.digest.clone(),
            network: sealed.network,
            host: spaces("self").ok_or(Refusal::Establishment)?,
            mounts: mounts(&launched.mounts, &handles)?,
            executable: sealed.executable.clone(),
            argv: plan.connection.argv.clone(),
            secrets: plan.secrets.clone(),
        };
        let control = launched.control.as_mut().ok_or(Refusal::Establishment)?;
        sent(control, &framed(&intent)?, startup)?;
        let ready = message(&read(&mut launched.ready, FRAME_MAX, startup)?)?;
        let seen = seen(first, ready.pid);
        let launcher = launched.child.id();
        let pipes = (&mut launched.control, &mut launched.status);
        let child = &mut launched.child;
        let end = || {
            child.kill().ok();
        };
        decided(&ready, (digest, &intent.sources), launcher, &seen)?;
        let bound = delivered(pipes, (admitted, plan), startup, (end, bind))?;
        let link = (&mut launched.input, &mut launched.output);
        served(link, &mut launched.errors, bound)
    }

    /// Hand the ready bootstrap its server's bindings on the box's
    /// `control` pipe and see the server start on its `status` pipe (MB4,
    /// MB3): the plan's declared names looked up in the admitted store's
    /// own handle and sent as one binding frame
    /// ([`secret::send_environment`]), the control pipe then closed, and the
    /// status pipe read to its end ([`exec_status`]). The lookup and the
    /// send run on a thread of their own, on its own duplicates of the
    /// store's handle and the control pipe, which it closes after its one
    /// frame; this one supervises it by the same `startup` as every other
    /// wait ([`handed`]). The sender marks its lookup begun, and publishes
    /// the bindings it looked up before it writes a byte of their frame;
    /// what it published is given to `bind` once the sender has answered
    /// or been given up on, at its deadline, on the attempt's cancellation
    /// or on its failure, before any of its answer is read: however long
    /// the write stalls, no cause after their lookup ends the session
    /// unguarded by them. One given up on after its lookup began and before
    /// it published leaves what it holds unknown ([`Handed::Uncertain`]),
    /// which `bind` records nothing more by, the closure included (MB4,
    /// CR2; operator ruling 2026-10-13). None are given where none were
    /// looked up. `bind`'s refusal is the session's, and what it answers is
    /// this.
    pub(super) fn delivered<B, E: From<Refusal>>(
        (control, status): (&mut Option<PipeWriter>, &mut PipeReader),
        (admitted, plan): (&Admitted, &Plan),
        startup: Startup<'_>,
        (end, bind): (
            impl FnOnce(),
            impl FnOnce(Handed<Vec<secret::BoundSecret>>) -> Result<B, E>,
        ),
    ) -> Result<B, E> {
        let open = control.as_ref().ok_or(Refusal::Handoff)?;
        let mut open = open.try_clone().or(Err(Refusal::Handoff))?;
        let store = admitted.store.try_clone().or(Err(Refusal::Handoff))?;
        let (path, names) = (plan.intent.excluded.store.clone(), plan.secrets.clone());
        let deadline = startup.0;
        let thread = std::thread::Builder::new();
        let (bindings, sent) = handed(thread, startup, end, move |given_up, published| {
            ensure(published.begin(), Refusal::Handoff)?;
            let given_up = || given_up.load(Ordering::SeqCst);
            let sending: Startup<'_> = (deadline, &given_up);
            let send = |frame: &[u8]| sent(&mut open, frame, sending).or(Err(Refusal::Handoff));
            let found = |found| published.publish(found);
            secret::send_environment(store.as_fd(), &path, &names, found, send)
        });
        let bound = bind(bindings)?;
        sent?;
        drop(control.take());
        exec_status(status, startup)?;
        Ok(bound)
    }

    /// Where a sender stands, as its supervisor takes it: no lookup begun;
    /// one begun, nothing published; what it published; or taken by its
    /// supervisor already.
    enum Slot<P> {
        Unbegun,
        Begun,
        Published(P),
        Taken,
    }

    /// What a sender begins and publishes before it can stall, held behind
    /// one lock with its supervisor's take: whatever is published before the
    /// take is taken, a lookup begun and not published by then is known to
    /// be, and a sender that begins or publishes after it begins nothing
    /// and publishes nothing, and sees itself given up on.
    pub(super) struct Published<P>(Arc<Mutex<Slot<P>>>);

    impl<P> Published<P> {
        /// Mark the lookup begun: whether it may be, which it may not once
        /// the supervisor has taken what stands.
        pub(super) fn begin(&self) -> bool {
            let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            match *slot {
                Slot::Unbegun => {
                    *slot = Slot::Begun;
                    true
                }
                Slot::Begun | Slot::Published(_) | Slot::Taken => false,
            }
        }

        /// Publish `value`, in place of anything published before it,
        /// unless the supervisor has taken what stands.
        pub(super) fn publish(&self, value: P) {
            let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            if !matches!(*slot, Slot::Taken) {
                *slot = Slot::Published(value);
            }
        }

        /// What stands, taken once: what was published; nothing where no
        /// lookup began, or where the sender `returned` from one it began,
        /// which publishes whatever it looked up before it returns; and
        /// uncertain where it began one and neither published nor returned.
        fn taken(&self, returned: bool) -> Handed<P> {
            let mut slot = self.0.lock().unwrap_or_else(PoisonError::into_inner);
            match (std::mem::replace(&mut *slot, Slot::Taken), returned) {
                (Slot::Published(value), _) => Handed::Published(value),
                (Slot::Begun, false) => Handed::Uncertain,
                (Slot::Unbegun | Slot::Begun | Slot::Taken, _) => Handed::Nothing,
            }
        }
    }

    /// `sender`'s answer, run on a detached thread of its own, made by
    /// `thread`, while this one waits within `startup`'s absolute deadline
    /// unless the attempt is cancelled ([`polled`]), and what it published
    /// by then ([`Published`]). A thread that cannot be made is the handoff
    /// cause, with nothing sent or published. A sender that has not
    /// answered by then, a store read held or a frame not taken, is the
    /// handoff cause at once: the sender is told it was given up on, `end`
    /// ends the box, and it is not waited for, so neither a held read nor a
    /// full pipe holds the refusal; what it published before it was given
    /// up on is still taken, and so is what a sender that failed published,
    /// and one that began its lookup and published nothing before either is
    /// uncertain. The sender owns all it holds, so nothing the box's
    /// settling releases is borrowed, and it ends with `serve`'s own exit on
    /// that refusal (operator ruling 2026-10-11).
    pub(super) fn handed<P: Send + 'static, T: Send + 'static>(
        thread: std::thread::Builder,
        startup: Startup<'_>,
        end: impl FnOnce(),
        sender: impl FnOnce(&AtomicBool, &Published<P>) -> Result<T, Refusal> + Send + 'static,
    ) -> (Handed<P>, Result<T, Refusal>) {
        let given_up = Arc::new(AtomicBool::new(false));
        let published = Published(Arc::new(Mutex::new(Slot::Unbegun)));
        let (giving, publishing) = (Arc::clone(&given_up), Published(Arc::clone(&published.0)));
        let sending = thread.spawn(move || sender(&giving, &publishing));
        let Ok(sending) = sending else {
            return (Handed::Nothing, Err(Refusal::Handoff));
        };
        let answered = polled(startup, || match sending.is_finished() {
            true => Ok(true),
            false => Err(ErrorKind::WouldBlock.into()),
        });
        if !answered {
            // Told first, so a sender that publishes after the take below
            // sees itself given up on and writes nothing.
            given_up.store(true, Ordering::SeqCst);
            end();
            return (published.taken(false), Err(Refusal::Handoff));
        }
        let answer = sending.join();
        let returned = answer.is_ok();
        let answer = answer.unwrap_or(Err(Refusal::Handoff));
        (published.taken(returned), answer)
    }

    /// What the bootstrap's status pipe holds once each of its writers has
    /// closed it, within `startup`: nothing where the server's exec closed
    /// it, which permits only the server's initialization; one tag's cause
    /// where the bootstrap reported one; the boxed-exec cause for any other
    /// bytes, the sender having completed. A pipe still open at the
    /// deadline, or on cancellation, is the handoff cause.
    pub(super) fn exec_status(
        status: &mut PipeReader,
        startup: Startup<'_>,
    ) -> Result<(), Refusal> {
        rustix::io::ioctl_fionbio(&*status, true).or(Err(Refusal::Handoff))?;
        let mut bytes = Vec::new();
        let mut chunk = [0; 2];
        let ended = polled(startup, || {
            let read = status.read(&mut chunk)?;
            bytes.extend_from_slice(&chunk[..read]);
            Ok((read == 0) | (bytes.len() > 1))
        });
        ensure(ended, Refusal::Handoff)?;
        if bytes.is_empty() {
            return Ok(());
        }
        Err(Status::read(&bytes).map_or(Refusal::Exec, Status::refusal))
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

    /// The bootstrap's arguments for its control, ready and status
    /// descriptors.
    fn bootstrap(control: RawFd, ready: RawFd, status: RawFd) -> Vec<String> {
        let [control, ready, status] = [control, ready, status].map(|fd| fd.to_string());
        let flags = [
            "--control",
            &control,
            "--ready",
            &ready,
            "--status",
            &status,
        ];
        ["broker", "bootstrap"]
            .into_iter()
            .chain(flags)
            .map(String::from)
            .collect()
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
                path: path.into(),
                device,
                inode: held.st_ino,
            })
        };
        mounted.iter().map(mount).collect()
    }

    /// The intent as one control frame: a four-byte big-endian length, then
    /// that many bytes, at most [`FRAME_MAX`], as the bootstrap reads it.
    pub(super) fn framed(intent: &Intent) -> Result<Vec<u8>, Refusal> {
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
        let whole = polled(startup, || {
            let read = pipe.read(&mut chunk)?;
            bytes.extend_from_slice(&chunk[..read]);
            match bytes.len() <= most {
                true => Ok(read == 0),
                false => Err(ErrorKind::FileTooLarge.into()),
            }
        });
        ensure(whole, Refusal::Establishment)?;
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
        let whole = polled(startup, || {
            left = &left[pipe.write(left)?..];
            Ok(left.is_empty())
        });
        ensure(whole, Refusal::Establishment)
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
