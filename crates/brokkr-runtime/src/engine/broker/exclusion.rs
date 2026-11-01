//! The coordination comparison (decision 0065 slice two, U8a2a; D6, CR2,
//! and the operator's ruling of 2026-10-18 under U8a2a).
//!
//! Another run's standing reservation on the same worktree is its lease,
//! which refuses; a run is its worktree's key and its id, so one run may
//! hold its own worktree across its attempts. Then every participant's
//! writers — the incoming attempt's, and every standing attempt's, the
//! same run's earlier ones included — are compared in both directions with
//! every root nothing may replace: the coordination root, which holds the
//! admission lock and every reservation, the incoming attempt's artifact
//! root where it protects evidence, every standing one that does, and
//! every historical one. A writer outside a workspace box, or one whose
//! writable reach could replace any of them, cannot participate. A box
//! overlays its own run's artifact root read-only (U8a), so only that one
//! is no box's to reach. Reach is compared by canonical path and by the
//! device and inode of each root, each writable root and their ancestors,
//! and a root visible at a second mount point refuses. Each refuses with
//! CR2's cause.
//!
//! These refusals apply where evidence is at stake: the incoming attempt
//! protects it, a standing one does, or a historical root exists. The
//! lifted MCP compile fence (Linux since U9b1) does not widen them to
//! every run, so a run with nothing at stake admits a writer Brokkr cannot
//! box (operator ruling of 2026-10-29 on U9b1's fourth run). A served lease
//! depends on the coordination root whatever is at stake, so while any
//! reservation serves, [`super::sources::kept`] keeps every writer from it.
//! Since a reservation is a plain file, a boxed writer that reaches that
//! root, or whose reach cannot be proved, is refused on every run
//! ([`super::sources::records_kept`], operator ruling of 2026-10-30): here
//! under the lock where the root is open, and before the attempt proceeds
//! holding no lease, against the root's spelled path, where it cannot be
//! opened (the controller's ruling on the third run).

#[cfg(target_os = "linux")]
use std::io::BufRead;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};

// Only the Linux mount table's judgement names MB3's causes here.
#[cfg(target_os = "linux")]
use super::admission::Unknown;
use super::admission::{Claim, Conflict, Unadmitted};
use super::historical::verified;
use super::inventory::{Budget, Reaching, Standing, Writing};
use super::resolved;
use super::root::Root;
use super::sources::records_kept;

/// What admission compares under its lock: every standing reservation as
/// the view every build writes reads it (decision 0090).
pub(super) struct Comparison<'a> {
    pub(super) claim: &'a Claim,
    pub(super) standing: &'a [Standing],
    pub(super) historical: &'a [PathBuf],
    pub(super) coordination: &'a Root,
}

/// One participant: its run, the artifact root its boxes overlay
/// read-only, and its writers.
struct Participant<'a> {
    run: &'a str,
    artifacts: &'a Path,
    writers: Vec<Writing>,
}

/// A root nothing may replace, as observed now: its device and inode, and
/// its ancestors' too, where it stands.
pub(super) struct Guarded {
    path: PathBuf,
    seen: Option<Seen>,
}

impl Guarded {
    /// `path`, as observed now.
    pub(super) fn of(path: PathBuf) -> Guarded {
        Guarded {
            seen: Seen::of(&path),
            path,
        }
    }
}

/// An observed directory's identity, and its ancestors', its own first.
struct Seen {
    chain: Vec<(u64, u64)>,
}

/// The object at `at`, by its device and inode, where anything stands there.
fn identity(at: &Path) -> Option<(u64, u64)> {
    let found = std::fs::metadata(at).ok()?;
    Some((found.dev(), found.ino()))
}

impl Seen {
    fn of(path: &Path) -> Option<Seen> {
        let chain = path.ancestors().map(identity);
        Some(Seen {
            chain: chain.collect::<Option<_>>()?,
        })
    }

    fn own(&self) -> (u64, u64) {
        self.chain[0]
    }
}

/// Whether `comparison` has evidence at stake: the coordination root is
/// always guarded, and anything guarded beside it is evidence.
pub(super) fn staked(comparison: &Comparison<'_>) -> bool {
    guarded(comparison).len() > 1
}

/// Whether `comparison` admits its claim: by the lease, by its boxed writers
/// kept from the coordination root on every run, then, where evidence is at
/// stake, by every participant's writers.
pub(super) fn compared(comparison: &Comparison<'_>, budget: &mut Budget) -> Result<(), Unadmitted> {
    leased(comparison)?;
    let root = comparison.coordination;
    records_kept(comparison.claim, root.path(), root.mounts(), budget)?;
    if !staked(comparison) {
        return Ok(());
    }
    let roots = guarded(comparison);
    for root in &roots[1..] {
        verified(root, budget)?;
    }
    let roots: Vec<Guarded> = roots.into_iter().map(Guarded::of).collect();
    let boxed = participated(comparison, &roots, budget)?;
    // A root that no longer stands shows nothing anywhere; its path is
    // still compared.
    for root in roots.iter().filter(|root| boxed & root.seen.is_some()) {
        shown_once(&root.path, budget)?;
    }
    Ok(())
}

/// Whether every participant's every writer can be kept from `roots`
/// ([`judged`]), each within `budget`'s deadline; and whether any is boxed.
fn participated(
    comparison: &Comparison<'_>,
    roots: &[Guarded],
    budget: &Budget,
) -> Result<bool, Unadmitted> {
    let participants = participants(comparison);
    // A bind source whose parent lies at or beneath any participant's
    // writable root can be replaced by its owner (R1).
    let writers = participants
        .iter()
        .flat_map(|participant| &participant.writers);
    let writable = writable(writers);
    let mut boxed = false;
    for participant in &participants {
        for writer in &participant.writers {
            budget.check()?;
            let sources = judged(participant, writer, roots)?;
            replaceable(participant, writer, sources, &writable)?;
            boxed |= writer.reach != Reaching::Unbounded;
        }
    }
    Ok(boxed)
}

/// Whether every source `sources` of `writer`'s box binds apart, of any
/// mode, as authored, is anchored outside every participant's writable reach, its
/// own included, so no managed writer can replace it, or any directory or
/// symlink on its way, before the box is rebuilt from the authored sources
/// (R1). A source is replaceable where the directory holding it, or
/// holding any of its ancestors, resolves at or below a writable root, by
/// canonical path or by the identity of its ancestors; a source's own path
/// may be a writable root. Each refuses with CR2's cause.
fn replaceable(
    participant: &Participant<'_>,
    writer: &Writing,
    sources: &[PathBuf],
    writable: &[Guarded],
) -> Result<(), Conflict> {
    let held = |at: &Path| {
        let parent = at
            .parent()
            .map(|parent| Guarded::of(resolved(parent.to_path_buf())));
        parent.is_some_and(|parent| writable.iter().any(|root| beneath(&parent, root)))
    };
    let swappable = |source: &&PathBuf| source.ancestors().any(held);
    let Some(bind) = sources.iter().find(swappable) else {
        return Ok(());
    };
    Err(Conflict::Replaceable {
        site: writer.site.clone(),
        run: participant.run.to_string(),
        bind: bind.clone(),
    })
}

/// Every writable root of `writers`' boxes, canonical and observed now.
fn writable<'w>(writers: impl Iterator<Item = &'w Writing>) -> Vec<Guarded> {
    let roots = writers.filter_map(|writer| match &writer.reach {
        Reaching::Boxed { writable, .. } => Some(writable),
        Reaching::Unbounded => None,
    });
    let roots = roots
        .flatten()
        .map(|root| Guarded::of(resolved(root.clone())));
    roots.collect()
}

/// Whether `path` lies at or below the writable root `root`: by canonical
/// path, or by `root`'s identity among `path`'s ancestors'.
pub(super) fn beneath(path: &Guarded, root: &Guarded) -> bool {
    let identical = match (&path.seen, &root.seen) {
        (Some(path), Some(root)) => path.chain.contains(&root.own()),
        _ => false,
    };
    path.path.starts_with(&root.path) | identical
}

/// Whether no other run of the claim's worktree holds it: a run is its
/// worktree's key and its id, and a reservation with no attempt standing
/// holds nothing.
fn leased(comparison: &Comparison<'_>) -> Result<(), Conflict> {
    let claim = comparison.claim;
    let other = |held: &&Standing| {
        (held.repo == claim.repo) & (held.run != claim.run) & !held.attempts.is_empty()
    };
    let holder = comparison.standing.iter().find(other);
    holder.map_or(Ok(()), |held| Err(Conflict::Held(held.run.clone())))
}

/// Every root nothing may replace, the coordination root first, each once.
fn guarded(comparison: &Comparison<'_>) -> Vec<PathBuf> {
    let claim = comparison.claim;
    let incoming = claim.protects().then_some(&claim.artifacts);
    let standing = comparison.standing.iter().filter(|held| held.protects());
    let mut roots = vec![comparison.coordination.path().to_path_buf()];
    let protecting = incoming
        .into_iter()
        .chain(standing.map(|held| &held.artifacts));
    for root in protecting.chain(comparison.historical) {
        if !roots.contains(root) {
            roots.push(root.clone());
        }
    }
    roots
}

/// The incoming attempt, and every run with an attempt standing.
fn participants<'a>(comparison: &Comparison<'a>) -> Vec<Participant<'a>> {
    let claim = comparison.claim;
    let incoming = Participant {
        run: &claim.run,
        artifacts: &claim.artifacts,
        writers: claim.reserved.writers.iter().map(Writing::from).collect(),
    };
    let standing = comparison.standing.iter().map(|held| Participant {
        run: &held.run,
        artifacts: &held.artifacts,
        writers: held.writers().cloned().collect(),
    });
    std::iter::once(incoming).chain(standing).collect()
}

/// Whether `writer` of `participant` can be kept from every root of
/// `roots` but its own run's artifact root, which its box overlays: the
/// sources its box binds apart where it can, for [`replaceable`].
fn judged<'w>(
    participant: &Participant<'_>,
    writer: &'w Writing,
    roots: &[Guarded],
) -> Result<&'w [PathBuf], Conflict> {
    let Reaching::Boxed { writable, sources } = &writer.reach else {
        return Err(Conflict::Unboxed {
            site: writer.site.clone(),
            run: participant.run.to_string(),
        });
    };
    let others = roots
        .iter()
        .filter(|root| root.path != participant.artifacts);
    for root in others {
        if writable.iter().any(|path| covers(path, root)) {
            return Err(Conflict::Reaches {
                site: writer.site.clone(),
                run: participant.run.to_string(),
                root: root.path.clone(),
            });
        }
    }
    Ok(sources)
}

/// Whether any writer of `writers` could write `path`: an unbounded writer
/// reaches everything; a boxed one where `path` lies at or below one of its
/// writable roots ([`beneath`]), as R1 compares a source's parent (R2). A
/// writable root below a directory writes nothing of that directory's own
/// entries. Used to prove a journal, its directory and its `-wal`/`-shm`
/// siblings outside every participant's reach before recovery trusts an
/// unkeyed hash chain in it.
pub(super) fn reaches(writers: &[&Writing], path: &Path) -> bool {
    let unbounded = writers
        .iter()
        .any(|writer| writer.reach == Reaching::Unbounded);
    let target = Guarded::of(resolved(path.to_path_buf()));
    let writable = writable(writers.iter().copied());
    unbounded | writable.iter().any(|root| beneath(&target, root))
}

/// Whether the writable root `writable` reaches `root`: at, above or below
/// it, by canonical path, or by the identity of either and the other's
/// ancestors, which a symlink, a bind alias or a second spelling cannot
/// hide.
pub(super) fn covers(writable: &Path, root: &Guarded) -> bool {
    let writable = resolved(writable.to_path_buf());
    let lexical = writable.starts_with(&root.path) | root.path.starts_with(&writable);
    let identical = match (Seen::of(&writable), &root.seen) {
        (Some(writable), Some(root)) => {
            root.chain.contains(&writable.own()) | writable.chain.contains(&root.own())
        }
        _ => false,
    };
    lexical | identical
}

/// `path`'s nearest ancestor that stands, canonical, and the rest of `path`
/// below it; `path` itself where none does.
pub(super) fn anchored(path: &Path) -> (PathBuf, PathBuf) {
    let found = path.ancestors().find_map(|at| {
        let rest = path.strip_prefix(at).ok()?.to_path_buf();
        Some((std::fs::canonicalize(at).ok()?, rest))
    });
    found.unwrap_or((path.to_path_buf(), PathBuf::new()))
}

/// [`covers`] for a root that need not stand, as the coordination root's
/// path does not where the walk to it could not be protected: each read by
/// its nearest ancestor that stands ([`anchored`]), and `writable`'s identity
/// looked for among whichever of `root`'s ancestors stand. Below a root that
/// does not stand nothing can be identified, so none of those ancestors'
/// identities counts against a writer lying beneath them: a sibling of the
/// root under its HOME is kept apart.
pub(super) fn covers_spelled(writable: &Path, root: &Path) -> bool {
    let whole = |path: &Path| {
        let (at, rest) = anchored(path);
        at.join(rest)
    };
    let (writes, root) = (whole(writable), whole(root));
    let own = identity(&writes);
    let above = root
        .ancestors()
        .any(|at| own.is_some() & (identity(at) == own));
    let lexical = writes.starts_with(&root) | root.starts_with(&writes);
    lexical | above | covers(writable, &Guarded::of(root))
}

/// Whether the mount table shows `root` at its own place only, read
/// within `budget`'s deadline ([`refusal`]).
#[cfg(target_os = "linux")]
fn shown_once(root: &Path, budget: &Budget) -> Result<(), Unadmitted> {
    let shown = || Conflict::Shown(root.to_path_buf());
    let mount = mounted_on(root).ok_or_else(shown)?;
    let table = std::fs::File::open(MOUNTINFO).or(Err(shown()))?;
    let mut table = std::io::BufReader::new(table);
    refusal(once(&mut table, mount, root, budget), root)
}

/// The id of the mount `path` lies on, as `statx` names it: the one lookup
/// both this comparison and the sources' make.
#[cfg(target_os = "linux")]
pub(super) fn mounted_on(path: &Path) -> Option<u64> {
    use rustix::fs::{statx, AtFlags, StatxFlags, CWD};
    let found = statx(CWD, path, AtFlags::empty(), StatxFlags::MNT_ID).ok()?;
    let named = found.stx_mask & StatxFlags::MNT_ID.bits() != 0;
    named.then_some(found.stx_mnt_id)
}

/// The mount of `mounts` whose id is `id`, which `path` lies on, and the
/// part of its device's filesystem `path` is there: the mount's root joined
/// with `path` below its point. None where no such mount is named, its
/// root is no path, or `path` does not lie below its point. The one
/// derivation both this comparison and the sources' ([`super::sources`])
/// read a part by.
#[cfg(target_os = "linux")]
pub(super) fn part<'m>(mounts: &'m [Mount], id: u64, path: &Path) -> Option<(&'m Mount, PathBuf)> {
    let own = mounts.iter().find(|each| each.id == id)?;
    let (Some(root), Ok(below)) = (&own.root, path.strip_prefix(&own.point)) else {
        return None;
    };
    Some((own, root.join(below)))
}

/// No mount table is read off Linux: a root is never proved shown once,
/// and every boxed participant beside evidence refuses, as no such box is
/// built there (decision 0063).
#[cfg(not(target_os = "linux"))]
fn shown_once(root: &Path, _: &Budget) -> Result<(), Unadmitted> {
    Err(Conflict::Shown(root.to_path_buf()).into())
}

/// How the mount table proves a root shown: at its own place only, at a
/// second place, with more records than D5's bound allows read, or not
/// before the deadline.
#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Shown {
    Once,
    Elsewhere,
    TooMany,
    Expired,
}

/// What `shown` refuses for `root`: nothing where it is shown once; CR2's
/// cause where it is shown elsewhere, or proves nothing; MB3's identity
/// cause where the table exceeds D5's bound ([`Unknown::Bounds`]) or the
/// deadline passed as it was read.
#[cfg(target_os = "linux")]
pub(super) fn refusal(shown: Shown, root: &Path) -> Result<(), Unadmitted> {
    match shown {
        Shown::Once => Ok(()),
        Shown::Elsewhere => Err(Conflict::Shown(root.to_path_buf()).into()),
        Shown::TooMany => Err(Unknown::Bounds.into()),
        Shown::Expired => Err(Unknown::Expired.into()),
    }
}

/// The mount table this namespace reads.
#[cfg(target_os = "linux")]
pub(super) const MOUNTINFO: &str = "/proc/self/mountinfo";

/// The most mounts a table may name, D5's mount bound.
#[cfg(target_os = "linux")]
const MOUNTS_MAX: usize = 65_536;

/// The longest record the table may hold: two paths of `PATH_MAX` bytes,
/// each byte escaped as four, and the mount's options, with room to spare.
#[cfg(target_os = "linux")]
const RECORD_MAX: u64 = 64 << 10;

/// One mount, as the table names it: its id, device, the part of its
/// filesystem it shows where that is a path (a namespace's is not), and
/// where.
#[cfg(target_os = "linux")]
#[derive(Debug, PartialEq, Eq)]
pub(super) struct Mount {
    pub(super) id: u64,
    pub(super) dev: String,
    pub(super) root: Option<PathBuf>,
    pub(super) point: PathBuf,
}

/// Whether the mount table `table` shows `root`, which lies on the mount
/// `mount`, at that one place, the table read within `budget`'s deadline:
/// the part of its device's filesystem the root is, read by its own
/// mount's root and point, lies in no other mount of that device, and
/// holds none. This narrows U8a's comparison, which stays the protocol's:
/// a table that does not parse, names no mount holding the root, or names
/// a mount of its device whose root is no path, proves nothing.
#[cfg(target_os = "linux")]
pub(super) fn once(table: &mut dyn BufRead, mount: u64, root: &Path, budget: &Budget) -> Shown {
    let mounts = match mounts(table, budget) {
        Ok(mounts) => mounts,
        Err(shown) => return shown,
    };
    let Some((own, part)) = part(&mounts, mount, root) else {
        return Shown::Elsewhere;
    };
    let apart = |each: &Mount| {
        let shown = |at: &PathBuf| part.starts_with(at) | at.starts_with(&part);
        each.root.as_ref().is_some_and(|at| !shown(at))
    };
    let same = mounts
        .iter()
        .filter(|each| (each.id != own.id) & (each.dev == own.dev));
    if same.into_iter().all(apart) {
        Shown::Once
    } else {
        Shown::Elsewhere
    }
}

/// Every mount the table `table` names, read one record at a time and
/// counted as it is read (M3), an empty one as any other (L3 of run four's
/// second review): the record past D5's bound is the last one read
/// ([`Shown::TooMany`]), as is one longer than [`RECORD_MAX`], so no more
/// of a table than the bound is ever collected; the deadline is checked
/// before each ([`Shown::Expired`]); a record that does not parse, or is
/// not UTF-8, proves nothing ([`Shown::Elsewhere`]).
#[cfg(target_os = "linux")]
pub(super) fn mounts(table: &mut dyn BufRead, budget: &Budget) -> Result<Vec<Mount>, Shown> {
    use std::io::Read;
    let (mut mounts, mut record, mut records) = (Vec::new(), Vec::new(), 0);
    loop {
        budget.check().or(Err(Shown::Expired))?;
        record.clear();
        let read = Read::take(&mut *table, RECORD_MAX + 1).read_until(b'\n', &mut record);
        if read.or(Err(Shown::Elsewhere))? == 0 {
            return Ok(mounts);
        }
        records += 1;
        let within = ((record.len() as u64) <= RECORD_MAX) & (records <= MOUNTS_MAX);
        within.then_some(()).ok_or(Shown::TooMany)?;
        let line = std::str::from_utf8(&record).or(Err(Shown::Elsewhere))?;
        let line = line.strip_suffix('\n').unwrap_or(line);
        if line.is_empty() {
            continue;
        }
        mounts.push(parsed(line).ok_or(Shown::Elsewhere)?);
    }
}

/// One line of the table: its id, its parent's, its device, its root and
/// its point, each path's octal escapes undone.
#[cfg(target_os = "linux")]
fn parsed(line: &str) -> Option<Mount> {
    let mut fields = line.split(' ');
    let id = fields.next()?.parse().ok()?;
    let _parent = fields.next()?;
    let dev = fields.next()?.to_string();
    let root = unescaped(fields.next()?)?;
    let point = unescaped(fields.next()?)?;
    let root = root.is_absolute().then_some(root);
    point.is_absolute().then_some(())?;
    Some(Mount {
        id,
        dev,
        root,
        point,
    })
}

/// `field` with each `\ooo` escape the kernel writes undone, or none where
/// an escape is not one.
#[cfg(target_os = "linux")]
fn unescaped(field: &str) -> Option<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    let mut bytes = Vec::with_capacity(field.len());
    let mut rest = field.as_bytes();
    while let Some((&byte, after)) = rest.split_first() {
        rest = after;
        if byte != b'\\' {
            bytes.push(byte);
            continue;
        }
        let digits = std::str::from_utf8(rest.get(..3)?).ok()?;
        bytes.push(u8::from_str_radix(digits, 8).ok()?);
        rest = &rest[3..];
    }
    Some(PathBuf::from(std::ffi::OsString::from_vec(bytes)))
}
