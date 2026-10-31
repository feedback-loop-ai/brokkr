//! The engine's provisioning of one attempt's protected broker plans
//! (decision 0065 slice two, U7c; D5, D6, MB1, MB3, CR3, SD3).
//!
//! After the attempt's durable start and its deadline, and before anything
//! spawns, each MCP holding the serving candidate of each of its sites
//! brokers — a single seat's, every panel member's and every sequence
//! step's, the selected link's own, fallback or not
//! ([`super::SiteMarks::brokered`]) — is sealed into one closed plan, and
//! the attempt's one inventory pins them all. Both are written through the
//! protocol records' one `Serialize` into the attempt's own directory under
//! the protected `HOME/.local/state/brokkr/capabilities/<repo>/<run>/<attempt>/`
//! root, which `brokkr broker serve` binds a plan against. The directory is
//! created exclusively: a retry, a reconnect or a resumed engine can never
//! mint a second inventory, or fresh retention shares, for an attempt that
//! already has one. The inventory is written last, so a provisioning cut
//! short leaves none, which the broker refuses as unbound.
//!
//! A plan's box is observed by `brokkr broker survey` (U7c0b), the current
//! Brokkr binary run as a child in a process group of its own under the
//! startup deadline and the attempt's bound, its input held open until its
//! reply is read so that it ends with the engine whatever ends the engine,
//! and killed and reaped with its whole group whatever it answers
//! ([`surveyed`]): it judges the plan's
//! sealed writers as confined, as the plan seals them, and the broker
//! reobserves before any lookup. Its executable, tree, bootstrap and
//! sources are sealed as it replied them, never as live handles. The
//! network and the clearance receipt are what MB4's comparison sealed on
//! the holding (U7c0), and the retention is its one effective rule. The
//! attempt directory, Git discovery for reach and the store's path are the
//! engine's own filesystem steps. Nothing is reconciled: a holding without
//! its admitted seal, a program or box the survey refuses, a survey that
//! does not answer in time, or a budget a share cannot fit refuses the
//! attempt before launch, a wanted holding included. A site with no
//! brokered holding provisions nothing. The driver of an attempt that did
//! provision is handed only what provisioning left of the attempt's bound
//! ([`left`]), and an attempt with nothing left is refused.
//!
//! What provisioning wrote is what each site is served (U7d2; MB1, MB4):
//! every slot's plan locator and digest, as the inventory pins them, and
//! its dialect's secret names, sealed as one [`SealedBroker`] under its
//! capability and kept on the attempt's selection by site label, from which
//! the site's serving inputs are sealed only once the plans exist.

use std::cell::Cell;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::ffi::OsStrExt;
use std::os::unix::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use brokkr_core::canonical;
use brokkr_protocol::broker::{
    BoxIntent, Connection as PlanConnection, Dialect, Excluded, Inventory, Owner, Pin, Plan,
    Privilege, Provisioned, Reach, Refusal, Restrictions, Survey, Surveyed, Unread, Writers,
};
use brokkr_protocol::hands::{self, BindMode, GitFacts, HandsSpec};
use brokkr_protocol::native_controls::SealedBroker;
use rustix::fs::{fsync, mkdirat, openat, Mode, OFlags, Stat, CWD};
use rustix::io::Errno;
use rustix::process::{kill_process_group, Pid, Signal};
use serde::Serialize;
use thiserror::Error;

pub(super) use self::root::Commit;
use super::marks::Brokered;
use super::{site_label, Engine, EngineError, EventType, Selection};
use crate::capabilities::{Admitted, Connection, Implementation};
pub(super) mod admission;
mod exclusion;
pub(super) mod fold;
mod historical;
mod inventory;
mod observation;
mod root;
pub(super) mod settlement;
mod sources;
mod terminal;
pub(super) use admission::{Authored, Held};
pub(super) use observation::{observed, Observed};
pub(super) use terminal::Host;

/// One attempt's retention budget, which its retaining plans share (CR3).
const BUDGET: u64 = 256 << 20;

/// The least share a retaining plan may be given (operator ruling
/// 2026-10-15): more than 32 retaining slots exhaust the budget.
const SHARE_FLOOR: u64 = 8 << 20;

/// The protected root's components below the host HOME (D6), which the
/// broker binds a plan's locator against.
const PROTECTED_ROOT: [&str; 4] = [".local", "state", "brokkr", "capabilities"];

/// The attempt's sealed inventory, beside its plans.
pub(super) const INVENTORY: &str = "inventory.json";

/// MB3's absolute startup deadline, which provisioning's survey shares
/// with the attempt's own bound and never extends.
const STARTUP: Duration = Duration::from_secs(30);

/// The most bytes the survey's reply may hold: MB3's request bound.
pub(super) const REPLY_BYTES_MAX: u64 = 1 << 20;

/// The longest one wait on the survey lasts before the deadline is read
/// again.
const SLICE: Duration = Duration::from_millis(100);

/// Why an attempt's broker plans could not be provisioned: each refuses
/// the attempt before anything spawns.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum Unprovisioned {
    #[error("capability evidence budget is exhausted")]
    Budget,
    /// The holding carries no seal MB4's comparison admitted, or no stdio
    /// launch: nothing a plan could be sealed from.
    #[error("capability '{0}' carries no admitted MCP seal to provision a plan from")]
    Unsealed(String),
    /// MB3's or MB4's own cause for the program or the box, as the survey
    /// refused with it.
    #[error(transparent)]
    Box(#[from] Refusal),
    /// The survey read no request the engine wrote.
    #[error(transparent)]
    Unread(Unread),
    #[error("MCP server box survey did not answer before its startup deadline")]
    Expired,
    /// Provisioning left nothing of the attempt's bound for its driver.
    #[error("the attempt's bound was spent provisioning its broker plans")]
    Spent,
    /// The survey did not start, or did not answer one closed reply of
    /// the bootstrap it was started as.
    #[error("MCP server box survey gave no closed reply")]
    Unanswered,
    /// The operator's secret store's path does not resolve to one the
    /// broker can read.
    #[error("the secret store's path does not resolve")]
    Store,
    /// HOME, or a directory on the way to the attempt's, is missing, a
    /// symlink, named by anything but one plain name, or not protected as
    /// the broker requires.
    #[error("capability evidence root is not protected")]
    Root,
    /// The attempt's directory already stands: its inventory and its
    /// shares were minted once, and are never minted again.
    #[error("capability evidence for this attempt is already provisioned")]
    Provisioned,
    #[error("capability evidence could not be written")]
    Write,
    #[error("the trusted bootstrap cannot be read")]
    Bootstrap,
    /// The host may let a server inject input into an ancestor's terminal
    /// (operator ruling 4 of 2026-10-18; [`terminal`]).
    #[error(
        "MCP servers cannot start: the host's /proc/sys/dev/tty/legacy_tiocsti is absent, \
         unreadable or not \"0\""
    )]
    Terminal,
    /// A managed writer can write a source the attempt's servers are built
    /// from, or the lease could not be extended to them (U8a2b; MB3).
    #[error(transparent)]
    Unadmitted(#[from] admission::Unadmitted),
}

/// Each retaining plan's share of the attempt's budget: the budget divided
/// by the number of retaining plans the inventory pins, none where none
/// retains, or exhausted where a share would fall under the floor. A fixed
/// function of the sealed inventory, which no inventory field carries.
pub(super) fn share(retaining: u64) -> Result<Option<u64>, Unprovisioned> {
    let share = BUDGET.checked_div(retaining);
    let exhausted = share.is_some_and(|share| share < SHARE_FLOOR);
    (!exhausted).then_some(share).ok_or(Unprovisioned::Budget)
}

/// When every survey of an attempt whose provisioning began at `began`
/// must have answered: MB3's startup deadline, or the attempt's `bound`
/// where that comes first.
pub(super) fn startup(began: Instant, bound: Duration) -> Instant {
    began + STARTUP.min(bound)
}

/// What is left at `now` of the attempt's `bound` for its driver, its
/// provisioning having begun at `began`; spent where nothing is.
pub(super) fn left(
    bound: Duration,
    began: Instant,
    now: Instant,
) -> Result<Duration, Unprovisioned> {
    let left = bound.saturating_sub(now.saturating_duration_since(began));
    (!left.is_zero())
        .then_some(left)
        .ok_or(Unprovisioned::Spent)
}

/// How one plan's box is surveyed: by the bootstrap at the path given,
/// answering before the deadline given, [`surveyed`] in production, or a
/// test's stand-in.
pub(super) type Surveyor<'s> =
    &'s dyn Fn(&Path, &Survey, Instant) -> Result<Provisioned, Unprovisioned>;

/// One brokered holding at one site of the attempt: the site's label and
/// instance reference, the holding and its seal.
struct Slot<'a> {
    label: String,
    instance: &'a str,
    held: Brokered<'a>,
}

/// What a brokered holding is sealed from: the admitted seal, and its
/// stdio launch, version and binding names.
struct Seal<'a> {
    admitted: &'a Admitted,
    argv: &'a [String],
    version: &'a str,
    secrets: &'a [String],
}

/// The attempt a plan is provisioned for, by the engine's own ids.
pub(super) struct Attempt<'a> {
    pub(super) effect: &'a str,
    pub(super) attempt: &'a str,
    pub(super) site: &'a str,
}

/// What one attempt's provisioning made: its directory, and the brokers
/// sealed from its plans, by the label of the site each serves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Sealed {
    pub(super) control: PathBuf,
    pub(super) brokers: BTreeMap<String, Vec<SealedBroker>>,
}

/// The facts every plan of one attempt shares, and how its boxes are
/// surveyed, and by when.
struct Shared<'h> {
    host: &'h Host,
    survey: Surveyor<'h>,
    deadline: Instant,
    repo: String,
    control: PathBuf,
    workdir: PathBuf,
    git: GitFacts,
    store: PathBuf,
}

impl Engine {
    /// Provision the plans of the attempt its effect, attempt and site
    /// name ([`Engine::provisioned`]) on the host as dispatch `observed`
    /// it, each box surveyed by its survey. Whether the attempt may go on
    /// to spawn. The host is read on every attempt, and consulted only by
    /// one that brokers a holding. Provisioning has no cancellation source
    /// but the engine's own end, which closes each survey's input and so
    /// ends the survey with its group (operator ruling 2026-10-16): the
    /// deadline is its one other bound, the attempt's one startup deadline
    /// from `began`, when its admission began (U8a2a; M8).
    pub(super) fn provision(
        &mut self,
        ids: [&str; 3],
        selection: &mut Selection,
        (began, bound): (Instant, &mut Duration),
        observed: Observed,
    ) -> Result<bool, EngineError> {
        let (host, survey) = (observed.host, observed.survey);
        self.provision_with(ids, selection, (began, bound), host, survey)
    }

    /// [`Engine::provision`] on `host`, each box surveyed by `survey`
    /// before the startup deadline or the attempt's `bound`, whichever
    /// comes first ([`startup`]), provisioning having begun at `began`:
    /// where a plan was provisioned, `bound` is left holding what remains
    /// of it for the driver, and an attempt with nothing left is refused;
    /// otherwise the attempt fails with the refusal before any driver
    /// starts. `selection` is left holding the brokers sealed for each
    /// site, none where nothing was provisioned.
    pub(super) fn provision_with(
        &mut self,
        [effect, attempt, site]: [&str; 3],
        selection: &mut Selection,
        (began, bound): (Instant, &mut Duration),
        host: Result<Host, Unprovisioned>,
        survey: Surveyor<'_>,
    ) -> Result<bool, EngineError> {
        let attempt = &Attempt {
            effect,
            attempt,
            site,
        };
        let (surveyor, commits) = ((survey, startup(began, *bound)), Cell::new(Commit::Synced));
        let provisioned = self.provisioned(attempt, selection, host, surveyor, &commits);
        // Kept whatever provisioning returns, once a seal committed (L1).
        if let Some(held) = &mut self.held {
            held.unsynced |= commits.get() == Commit::Unsynced;
        }
        let remaining = provisioned.and_then(|made| {
            let left = |sealed: Sealed| Ok((sealed.brokers, left(*bound, began, Instant::now())?));
            made.map(left).transpose()
        });
        let refusal = match remaining {
            Ok(made) => {
                (selection.brokers, *bound) = made.unwrap_or((BTreeMap::new(), *bound));
                return Ok(true);
            }
            Err(refusal) => refusal,
        };
        let failed = serde_json::json!({
            "effect_id": attempt.effect,
            "attempt_id": attempt.attempt,
            "error": refusal.to_string(),
        });
        let id = Some(attempt.attempt.to_string());
        self.append(EventType::EffectFailed, failed, id)?;
        Ok(false)
    }

    /// Seal and write every brokered holding's plan of this attempt, and
    /// its inventory last, under `host`, each box surveyed by `survey`
    /// before `deadline`; the attempt's directory and the brokers sealed
    /// from its plans ([`brokers`]), or none where no site brokers a
    /// holding. The budget and every holding's seal are judged
    /// before the host is consulted, a host that could not be read refusing
    /// only an attempt that brokers a holding, and every plan is sealed
    /// before anything is made. Every seal of the attempt's lease is folded
    /// into `commits` as it is made.
    pub(super) fn provisioned(
        &self,
        attempt: &Attempt<'_>,
        selection: &Selection,
        host: Result<Host, Unprovisioned>,
        (survey, deadline): (Surveyor<'_>, Instant),
        commits: &Cell<Commit>,
    ) -> Result<Option<Sealed>, Unprovisioned> {
        let slots = self.slots(attempt.site, selection);
        let retaining = slots.iter().filter(|slot| slot.retained()).count();
        share(retaining as u64)?;
        let seals: Vec<Seal<'_>> = slots.iter().map(Slot::seal).collect::<Result<_, _>>()?;
        if slots.is_empty() {
            return Ok(None);
        }
        let host = host?;
        let workdir = std::fs::canonicalize(self.workdir()).or(Err(Unprovisioned::Root))?;
        let repo = canonical::sha256_bytes(workdir.as_os_str().as_bytes());
        let below = [repo.as_str(), self.run_id.as_str()];
        let names = PROTECTED_ROOT.iter().chain(&below);
        let control = names.fold(host.home.clone(), |path, name| path.join(name));
        let shared = Shared {
            host: &host,
            survey,
            deadline,
            control: control.join(attempt.attempt),
            git: hands::git_facts(&workdir),
            store: stored(&self.secrets_store_path())?,
            workdir,
            repo: repo.clone(),
        };
        // The lease holds every fixed input and launch before any survey
        // spawns, the program trees once the surveys answer (U8a2b).
        let extend = |serves| sources::held_extended(self.held.as_ref(), serves, deadline, commits);
        let fixed = sources::fixed(&host.bootstrap);
        let launches = sources::launches(seals.iter().map(|seal| seal.argv));
        extend([fixed.clone(), launches].concat())?;
        let plans: Vec<Plan> = (slots.iter().zip(&seals))
            .map(|(slot, seal)| self.plan(&shared, attempt, slot, seal))
            .collect::<Result<_, _>>()?;
        extend(sources::served(fixed, &plans))?;
        // Below the admitted root, from its handle, not HOME again (U8a2a).
        let run = match &self.held {
            Some(held) => (held.coordination.below(&below)).or(Err(Unprovisioned::Root))?,
            None => protected(&host.home, &below)?,
        };
        let pins = written(&run, &shared.control, plans)?;
        let brokers = brokers(&slots, &seals, pins);
        Ok(Some(Sealed {
            control: shared.control,
            brokers,
        }))
    }

    /// Every brokered holding of this attempt's executing model sites, each
    /// by the link selected for its site, in the sites' order.
    fn slots<'s>(&'s self, site_name: &str, selection: &'s Selection) -> Vec<Slot<'s>> {
        let marks = self.marks();
        let mut slots = Vec::new();
        for (site, plan) in &selection.plans {
            let label = site_label(site_name, site);
            let held = marks.brokered(&label, selection.get(site));
            slots.extend(held.into_iter().map(|held| Slot {
                label: label.clone(),
                instance: &plan.context.instance_ref,
                held,
            }));
        }
        slots
    }

    /// One slot's sealed plan (D5): its owner, the holding's dialect,
    /// launch, tools and admitted seal, and its box as the survey observes
    /// it now under the site's own hands.
    fn plan(
        &self,
        shared: &Shared<'_>,
        attempt: &Attempt<'_>,
        slot: &Slot<'_>,
        seal: &Seal<'_>,
    ) -> Result<Plan, Unprovisioned> {
        let Brokered {
            capability,
            server,
            holding,
        } = slot.held;
        let host = shared.host;
        let spec = self.runtime_hands(&slot.label).unwrap_or_default();
        let request = Survey {
            launch: seal.argv.first().cloned().unwrap_or_default(),
            home: host.home.clone(),
            reach: reach(&spec, &host.home, &shared.workdir, &shared.git),
            network: seal.admitted.network,
            // Every managed writer's mapped uid, its privilege confined, as
            // the lease's extension proves before anything starts (U8a2b).
            writers: vec![host.writer],
        };
        let provisioned = (shared.survey)(&host.bootstrap, &request, shared.deadline)?;
        Ok(Plan {
            owner: Owner {
                repo: shared.repo.clone(),
                run: self.run_id.clone(),
                effect: attempt.effect.to_string(),
                attempt: attempt.attempt.to_string(),
                site: slot.label.clone(),
                instance: slot.instance.to_string(),
            },
            server: server.to_string(),
            capability: capability.to_string(),
            dialect: Dialect {
                name: holding.dialect.clone(),
                digest: holding.dialect_sha256.clone(),
                definition: holding.definition_sha256.clone(),
                version: seal.version.to_string(),
            },
            connection: PlanConnection {
                argv: seal.argv.to_vec(),
            },
            tools: holding.tools.clone(),
            restrictions: Restrictions {},
            retained: holding.retention.effective(),
            secrets: seal.secrets.to_vec(),
            clearance: seal.admitted.clearance.clone(),
            intent: BoxIntent {
                reach: request.reach,
                executable: provisioned.executable,
                tree: provisioned.tree,
                sources: provisioned.sources,
                writers: Writers {
                    uids: request.writers,
                    privilege: Privilege::Confined,
                },
                network: request.network,
                environment: hands::server_environment().map(String::from).collect(),
                bootstrap: provisioned.bootstrap,
                excluded: Excluded {
                    store: shared.store.clone(),
                    control: vec![shared.control.clone()],
                },
            },
        })
    }
}

impl<'a> Slot<'a> {
    /// Whether the slot's responses are retained: its holding's one
    /// effective rule.
    fn retained(&self) -> bool {
        self.held.holding.retention.effective()
    }

    /// The seal and stdio launch the slot's holding is sealed from, or the
    /// refusal of a holding without them.
    fn seal(&self) -> Result<Seal<'a>, Unprovisioned> {
        let holding = self.held.holding;
        match (&holding.admitted, &holding.implementation) {
            (
                Some(admitted),
                Implementation::Mcp {
                    connection: Connection::Stdio(argv),
                    version,
                    secrets,
                    ..
                },
            ) => Ok(Seal {
                admitted,
                argv,
                version,
                secrets,
            }),
            (None, _) | (Some(_), Implementation::Mcp { .. } | Implementation::Native { .. }) => {
                Err(Unprovisioned::Unsealed(self.held.capability.to_string()))
            }
        }
    }
}

/// The site's effective hands reach (D5): the canonical workspace, the
/// actual Git reach and its read-write binds are writable; its read-only
/// and overlay binds are only readable. A bind's `~/` is the host `home`,
/// as the hands box binds it, and each root is as it resolves, or as
/// spelled where it does not.
pub(super) fn reach(spec: &HandsSpec, home: &Path, workdir: &Path, git: &GitFacts) -> Reach {
    let mut writable = vec![workdir.to_path_buf()];
    writable.extend(git.common_dir.clone().map(resolved));
    let mut readable = Vec::new();
    for bind in &spec.binds {
        let host = spelled(&bind.path, home);
        match bind.mode {
            BindMode::Rw => writable.push(resolved(host)),
            BindMode::Ro | BindMode::Overlay => readable.push(resolved(host)),
        }
    }
    Reach { writable, readable }
}

/// The host path a bind's authored `path` names, unresolved: its `~/` the
/// host `home`, as the hands box binds it.
fn spelled(path: &str, home: &Path) -> PathBuf {
    match path.strip_prefix("~/") {
        Some(rest) => home.join(rest),
        None => PathBuf::from(path),
    }
}

/// `path` with every symlink resolved, or as spelled where it does not
/// resolve.
fn resolved(path: PathBuf) -> PathBuf {
    let canonical = std::fs::canonicalize(&path);
    canonical.unwrap_or(path)
}

/// The operator's store as the broker reads it (MB4): absolute, with no
/// `.` or `..` component. A `..` means the parent of what precedes it
/// resolves to, so everything up to the last one resolves as the kernel
/// resolves it; every component after it stays as spelled, a symlink
/// among them, for the broker resolves the store's own route itself.
pub(super) fn stored(store: &Path) -> Result<PathBuf, Unprovisioned> {
    let absolute = std::path::absolute(store).or(Err(Unprovisioned::Store))?;
    let parts: Vec<Component<'_>> = absolute.components().collect();
    let last = parts.iter().rposition(|part| part == &Component::ParentDir);
    let (head, tail) = match last {
        Some(at) => {
            let parent: PathBuf = parts[..=at].iter().collect();
            let parent = std::fs::canonicalize(parent).or(Err(Unprovisioned::Store))?;
            (parent, &parts[at + 1..])
        }
        None => (PathBuf::new(), &parts[..]),
    };
    let tail = tail.iter().filter(|part| part != &&Component::CurDir);
    Ok(tail.fold(head, |path, part| path.join(part)))
}

/// The bootstrap at `bootstrap` run as `brokkr broker survey` of
/// `request`, in a process group of its own, on the server `PATH` without
/// the loader's `LD_*` ([`sources::surveying`]), and its one reply received
/// before the startup `deadline`; its input then closed and its whole
/// group killed and reaped whatever it answered, so a blocked filesystem
/// call ends with it and no worker outlives it. Until then the engine
/// holds the survey's input open, so the engine's end, by whatever means,
/// ends its input and the survey with its group (`broker survey`'s
/// tether). Only a reply of that very bootstrap is an answer: a box it
/// observed, MB3's cause, or why it read no request; never its exit
/// status, which the input's close may make a kill. A child outside the
/// engine's group is one the attempt tracker (#403) may take for an
/// orphan a live attempt left; no attempt of this engine is live while it
/// provisions, the next one's driver spawning only after this survey is
/// reaped.
pub(super) fn surveyed(
    bootstrap: &Path,
    request: &Survey,
    deadline: Instant,
) -> Result<Provisioned, Unprovisioned> {
    let mut line = serde_json::to_vec(request).or(Err(Unprovisioned::Unanswered))?;
    line.push(b'\n');
    let environment = sources::surveying(std::env::vars_os(), &sources::launchers());
    let started = Command::new(bootstrap)
        .args(["broker", "survey"])
        .env_clear()
        .envs(environment)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn();
    let mut child = started.or(Err(Unprovisioned::Unanswered))?;
    let reply = replied(&mut child, line, deadline);
    ended(&mut child);
    let provisioned = answered(&reply?)?;
    let ours = provisioned.bootstrap.path == bootstrap;
    ours.then_some(provisioned).ok_or(Unprovisioned::Unanswered)
}

/// The request `line` written to `child`, and everything it wrote on its
/// standard output until it closed it, at most one more byte than the
/// reply's bound: both on threads of their own, so neither a survey that
/// reads nothing nor one that answers nothing holds the engine past the
/// deadline. The writer hands the survey's input back once written and
/// its handle is dropped as this returns, so the input stays open until
/// the reply is read or the deadline passes: closed then where the write
/// is done, or with the writer once the group's kill fails a write that
/// is not.
fn replied(child: &mut Child, line: Vec<u8>, deadline: Instant) -> Result<Vec<u8>, Unprovisioned> {
    let (stdin, stdout) = (child.stdin.take(), child.stdout.take());
    let (stdin, stdout) = stdin.zip(stdout).ok_or(Unprovisioned::Unanswered)?;
    let _writer = std::thread::spawn(move || {
        let mut stdin = stdin;
        // A survey that stopped reading has no reply to wait for.
        stdin.write_all(&line).ok();
        stdin
    });
    let (sender, received) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let read = stdout.take(REPLY_BYTES_MAX + 1).read_to_end(&mut bytes);
        sender.send(read.map(|_| bytes)).ok();
    });
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        (!left.is_zero())
            .then_some(())
            .ok_or(Unprovisioned::Expired)?;
        if let Ok(read) = received.recv_timeout(left.min(SLICE)) {
            return read.or(Err(Unprovisioned::Unanswered));
        }
    }
}

/// Kill the survey's whole group and reap the survey: a member blocked in
/// a filesystem call is woken by the kill.
fn ended(child: &mut Child) {
    kill_process_group(Pid::from_child(child), Signal::KILL).ok();
    child.wait().ok();
}

/// The survey's one reply in `bytes`: one closed line, within its bound,
/// its box, MB3's cause, or why it read no request.
fn answered(bytes: &[u8]) -> Result<Provisioned, Unprovisioned> {
    let within = bytes.len() as u64 <= REPLY_BYTES_MAX;
    let line = bytes
        .strip_suffix(b"\n")
        .filter(|line| within & !line.contains(&b'\n'));
    let line = line.ok_or(Unprovisioned::Unanswered)?;
    match serde_json::from_slice(line).or(Err(Unprovisioned::Unanswered))? {
        Surveyed::Provisioned(provisioned) => Ok(provisioned),
        Surveyed::Refused(refusal) => Err(Unprovisioned::Box(refusal)),
        Surveyed::Unread(unread) => Err(Unprovisioned::Unread(unread)),
    }
}

/// A directory's open flags: never through a symlink.
pub(super) fn directory() -> OFlags {
    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC
}

/// The run's directory below the protected root, reached from `/` one
/// component at a time without following a symlink, and made owner-only
/// where it is missing from the protected root's first component on. Each
/// name, the run's id among them, is one plain name, so none walks out of
/// the directory it is reached from or past an unchecked one. HOME's own
/// ancestors and the protected root's are this user's or
/// root's and writable by no one else, unless root-owned and sticky; the
/// protected root and everything below it is this user's alone. These are
/// the directories the broker binds a locator through. The user is the
/// engine's own, which every directory it makes is owned by. A directory
/// that gains a name is synced before the walk goes on (U8a2a; M9).
fn protected(home: &Path, below: &[&str]) -> Result<OwnedFd, Unprovisioned> {
    protected_with(home, below, &|parent| fsync(parent))
}

/// How a directory that gained a name is made durable: `fsync`, or a
/// test's witness.
type Synced<'s> = &'s dyn Fn(&OwnedFd) -> rustix::io::Result<()>;

/// [`protected`], each directory that gained a name handed to `synced`.
fn protected_with(
    home: &Path,
    below: &[&str],
    synced: Synced<'_>,
) -> Result<OwnedFd, Unprovisioned> {
    let home: Vec<&std::ffi::OsStr> = home.iter().skip(1).collect();
    let made = home.len();
    let private = made + PROTECTED_ROOT.len() - 1;
    let names =
        (home.into_iter()).chain(PROTECTED_ROOT.iter().chain(below).map(std::ffi::OsStr::new));
    let mut held = openat(CWD, "/", directory(), Mode::empty()).or(Err(Unprovisioned::Root))?;
    for (index, name) in names.enumerate() {
        held = root::step(&held, name, (index >= made, index >= private), synced)?;
    }
    Ok(held)
}

/// Whether a directory `stat` describes protects what lies below it: this
/// user's alone where `private` (`lock_file::alone`), otherwise this
/// user's or root's and writable by no one else, unless root-owned and sticky.
pub(super) fn owned(stat: &Stat, user: u32, private: bool) -> bool {
    let mode = stat.st_mode;
    let mine = stat.st_uid == user;
    let shut = mode & 0o022 == 0;
    let sticky = (stat.st_uid == 0) & (mode & 0o1000 != 0);
    let shared = (mine | (stat.st_uid == 0)) & (shut | sticky);
    (private & crate::lock_file::alone(stat, user)) | (!private & shared)
}

/// The attempt's directory `control`, made exclusively in the run's
/// directory `run`, each plan written in it, and the inventory pinning
/// them last. Only a directory that already stands is provisioned;
/// anything else that stops it being made is a write that failed.
fn written(run: &OwnedFd, control: &Path, plans: Vec<Plan>) -> Result<Vec<Pin>, Unprovisioned> {
    let attempt = control.file_name().unwrap_or_default();
    match mkdirat(run, attempt, Mode::RWXU) {
        Ok(()) => Ok(()),
        Err(Errno::EXIST) => Err(Unprovisioned::Provisioned),
        Err(_) => Err(Unprovisioned::Write),
    }?;
    let dir = openat(run, attempt, directory(), Mode::empty()).or(Err(Unprovisioned::Root))?;
    let mut pins = Vec::with_capacity(plans.len());
    for (index, plan) in plans.iter().enumerate() {
        let name = format!("plan-{index}.json");
        let digest = sealed(&dir, &name, plan)?;
        let locator = control.join(name);
        pins.push(Pin { locator, digest });
    }
    let inventory = Inventory { plans: pins };
    sealed(&dir, INVENTORY, &inventory)?;
    Ok(inventory.plans)
}

/// Each slot's broker (U7d2; MB1, MB4), under the label of the site it
/// serves, in the slots' order: its capability, the locator and digest its
/// `pins` entry pins its plan by, and its dialect's secret names as the
/// harness's removals. Each locator is read as the UTF-8 its inventory was
/// written in: a path that is not UTF-8 serializes to no record, so its
/// attempt was refused as a failed write before this reads it, which
/// `brokered_tests::an_attempt_directory_that_is_not_utf8_seals_no_broker`
/// binds as the removal control of the refusal this omits.
fn brokers(
    slots: &[Slot<'_>],
    seals: &[Seal<'_>],
    pins: Vec<Pin>,
) -> BTreeMap<String, Vec<SealedBroker>> {
    let mut brokers: BTreeMap<String, Vec<SealedBroker>> = BTreeMap::new();
    for ((slot, seal), pin) in slots.iter().zip(seals).zip(pins) {
        brokers
            .entry(slot.label.clone())
            .or_default()
            .push(SealedBroker {
                capability: slot.held.capability.to_string(),
                plan: pin.locator.to_string_lossy().into_owned(),
                plan_digest: pin.digest,
                removals: seal.secrets.to_vec(),
            });
    }
    brokers
}

/// `record` written through its one `Serialize` as a new owner-only file
/// `name` in `dir`, synced; the sha256 of its bytes.
fn sealed(dir: &OwnedFd, name: &str, record: &impl Serialize) -> Result<String, Unprovisioned> {
    let bytes = serde_json::to_vec(record).or(Err(Unprovisioned::Write))?;
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(dir, name, flags, Mode::RUSR | Mode::WUSR).or(Err(Unprovisioned::Write))?;
    let mut file = std::fs::File::from(file);
    let synced = file.write_all(&bytes).and_then(|()| file.sync_all());
    synced.or(Err(Unprovisioned::Write))?;
    Ok(canonical::sha256_bytes(&bytes))
}
