//! The seal machinery a protected root writes a record with: a first
//! admission may make the record, by links, renames and removals only (M3
//! unchanged), and every reseal of a record that must still stand replaces
//! it by one atomic exchange, so no write can bring back a record removed
//! inside its window (decision 0108 ruling 3, #440 fix rounds 440d5 and
//! 440d6).
//!
//! Moved out of `root`, which this would take past its 800 lines (decision
//! 0071 ruling 4).

use std::io::{Read, Write};
use std::os::fd::OwnedFd;
use std::os::unix::fs::MetadataExt;

use rustix::fs::{fsync, linkat, openat, renameat, unlinkat, AtFlags, Mode, OFlags};
use rustix::io::Errno;

use super::{regular_at, Commit, Exchange, Hook, Identity, Make, Record, Step, PRIOR};
use crate::engine::broker::admission::{Unknown, STAGED};
use crate::engine::broker::inventory::Budget;
use crate::engine::broker::REPLY_BYTES_MAX;

/// What one seal writes and how: the directory and the record's name, the
/// staged name beside it, the budget it is begun under, the root's hook,
/// and the exchange a reseal replaces its record by — one object because
/// every step of the seal hands all of them on (decision 0071 ruling 10).
struct Seal<'a> {
    dir: &'a OwnedFd,
    name: &'a str,
    staged: String,
    budget: &'a Budget,
    hook: Hook,
    exchange: Exchange,
}

impl Seal<'_> {
    /// The record's prior name, the one link of the record that stood.
    fn prior(&self) -> String {
        format!("{}{PRIOR}", self.name)
    }
}

/// What one seal is asked for: the record's directory and name, the bytes it
/// writes, the budget it is begun under, the root's hook, the exchange a
/// reseal replaces its record by, whether it may create, and the record it
/// read and holds open beside that record's own bytes (decision 0071 ruling
/// 10).
pub(super) struct Sheet<'a> {
    pub(super) dir: &'a OwnedFd,
    pub(super) name: &'a str,
    pub(super) bytes: &'a [u8],
    pub(super) budget: &'a Budget,
    pub(super) hook: Hook,
    pub(super) exchange: Exchange,
    pub(super) make: Make<'a>,
    pub(super) read: &'a [u8],
}

/// `bytes` sealed as `name` in the directory `dir`, begun before `budget`'s
/// deadline. A seal of `name` cut short before it was final is undone
/// first ([`restored`]). The bytes are written as a new owner-only file
/// beside it ([`staged_as`]); then, the deadline checked once more, the
/// record that stood linked as `name` with [`PRIOR`] after it, and the
/// staged copy committed ([`commit`]).
///
/// A seal that may create (a first admission) renames its copy over
/// `name`, and a reseal exchanges its copy with `name` atomically: the
/// exchange fails with `ENOENT` when the name is gone, so no removal at
/// any instant before it is undone, and it fails closed where the
/// filesystem has no exchange (decision 0108 ruling 3, M3 amended by #440
/// fix round 440d6). The commit is handed to `hook`, the directory synced
/// and the deadline checked. A first seal is then committed. A reseal is
/// committed only once the record that stood is retired ([`retired`]): the
/// deadline checked the last time, its prior name removed, and the
/// directory synced again. Until then every read finds the record that
/// stood ([`standing`]).
///
/// A seal that fails or runs late before its commit is undone
/// ([`undone`]): the record that stood put back by the same exchange, the
/// same file, so its bytes and its inode, or the record made removed where
/// none stood (H1; R1 of run four's review). One whose undoing fails
/// refuses as unsealed, whatever cut it short, the record that stood still
/// under its prior name for every read and for the next seal to put back
/// (M2 of run four's second review). Once committed, no seal reports
/// failure or lateness: one whose directory is not proven synced after
/// reports itself committed but [`Commit::Unsynced`] (R1 of the fourth run,
/// U8a2c).
pub(in crate::engine::broker) fn seal_with(
    dir: &OwnedFd,
    name: &str,
    bytes: &[u8],
    budget: &Budget,
    hook: Hook,
    exchange: Exchange,
) -> Result<Commit, Unknown> {
    seal_made(Sheet {
        dir,
        name,
        bytes,
        budget,
        hook,
        exchange,
        make: Make::May,
        read: &[],
    })
}

/// [`seal_with`], `make` deciding whether the record may be made where none
/// stands, `exchange` the atomic swap a reseal replaces it by, `stood` the
/// record the caller read and holds open, and `read` its own bytes.
pub(super) fn seal_made(write: Sheet<'_>) -> Result<Commit, Unknown> {
    write.budget.check()?;
    let seal = Seal {
        dir: write.dir,
        name: write.name,
        staged: format!("{}{STAGED}", write.name),
        budget: write.budget,
        hook: write.hook,
        exchange: write.exchange,
    };
    restored(&seal, write.make, write.read)?;
    let written = match staged_as(&seal, write.bytes) {
        Ok(written) => written,
        Err(refused) => {
            let _ = unlinkat(write.dir, seal.staged.as_str(), AtFlags::empty());
            return Err(refused);
        }
    };
    if let Err(refused) = write.budget.check() {
        let _ = unlinkat(write.dir, seal.staged.as_str(), AtFlags::empty());
        return Err(refused);
    }
    // A commit that fails cleans up the staged copy it still owns; one that
    // leaves a hand's file under that name keeps it there.
    let stood = commit(&seal, write.make, &written)?;
    let synced = (seal.hook)(Step::Committed, write.dir, &seal.staged)
        .and_then(|()| fsync(write.dir).map_err(std::io::Error::from));
    let durable = (synced.or(Err(Unknown::Unsealed))).and_then(|()| write.budget.check());
    if let Err(refused) = durable {
        let undone = undone(&seal, stood, &written, write.make);
        return Err(undone.map_or(Unknown::Unsealed, |()| refused));
    }
    if !stood {
        return Ok(Commit::Synced);
    }
    retired(&seal, &written, write.make)
}

/// The reseal of `name` in `dir`, exchanged in and synced, committed by
/// retiring the record that stood: handed to `hook`, the deadline of
/// `budget` checked the last time, and its prior name removed; whether the
/// directory is then synced. One late or whose prior name cannot be
/// removed is undone ([`undone`]) and refused, the record that stood still
/// what every read finds.
fn retired(seal: &Seal<'_>, written: &Record, make: Make<'_>) -> Result<Commit, Unknown> {
    let prior = seal.prior();
    let retiring = ((seal.hook)(Step::Retiring, seal.dir, seal.name).or(Err(Unknown::Unsealed)))
        .and_then(|()| seal.budget.check())
        .and_then(|()| {
            unlinkat(seal.dir, prior.as_str(), AtFlags::empty()).or(Err(Unknown::Unsealed))
        });
    if let Err(refused) = retiring {
        let undone = undone(seal, true, written, make);
        return Err(undone.map_or(Unknown::Unsealed, |()| refused));
    }
    // Committed: its successor alone is the record, as every read finds it.
    let synced = (seal.hook)(Step::Final, seal.dir, seal.name).and_then(|()| Ok(fsync(seal.dir)?));
    Ok(synced.map_or(Commit::Unsynced, |()| Commit::Synced))
}

/// `bytes` written as the new owner-only file `staged` in `dir`, synced
/// and handed to `hook`: what a seal cut short staged goes first, and one
/// that cannot go is refused by the exclusive create. The file it wrote is
/// kept open, so no hand can reuse its inode before the seal proves it.
fn staged_as(seal: &Seal<'_>, bytes: &[u8]) -> Result<Record, Unknown> {
    let staged = seal.staged.as_str();
    let _ = unlinkat(seal.dir, staged, AtFlags::empty());
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = openat(seal.dir, staged, flags, Mode::RUSR | Mode::WUSR);
    let mut file = std::fs::File::from(file.or(Err(Unknown::Unsealed))?);
    let found = file.metadata().or(Err(Unknown::Unsealed))?;
    let held = file.try_clone().or(Err(Unknown::Unsealed))?;
    let written = (file.write_all(bytes).and_then(|()| file.sync_all()))
        .and_then(|()| (seal.hook)(Step::Staged, seal.dir, staged));
    written.or(Err(Unknown::Unsealed))?;
    Ok(Record {
        _file: held.into(),
        id: (found.dev(), found.ino()),
    })
}

/// The staged copy committed as `name`: a first seal (`Make::May`) links
/// the record that stands under its prior name, so a seal undone puts that
/// very file back, then renames its copy over it, or renames it in where
/// none stands. A reseal (`Make::Never`) links the record that stood, hands
/// `hook` the one point before the exchange, then exchanges its copy with
/// `name` atomically: a removal at any instant before the exchange is met
/// with `ENOENT` and writes nothing back, and a filesystem without an
/// exchange refuses by name. After the exchange the staged name holds the
/// file that stood; it must be the very inode the reseal read, or a foreign
/// file was planted at the name, and the exchange is put back and the seal
/// refused (decision 0108 ruling 3, #440 fix round 440d6). Whether one
/// stood.
fn commit(seal: &Seal<'_>, make: Make<'_>, written: &Record) -> Result<bool, Unknown> {
    let prior = seal.prior();
    let linked = linkat(
        seal.dir,
        seal.name,
        seal.dir,
        prior.as_str(),
        AtFlags::empty(),
    );
    let has = match (linked, make) {
        (Ok(()), _) => true,
        (Err(Errno::NOENT), Make::May) => false,
        (Err(_), _) => {
            let _ = unlinkat(seal.dir, seal.staged.as_str(), AtFlags::empty());
            return Err(Unknown::Unsealed);
        }
    };
    match make {
        Make::May => {
            let renamed = renameat(seal.dir, seal.staged.as_str(), seal.dir, seal.name);
            if renamed.is_err() & has {
                let _ = unlinkat(seal.dir, prior.as_str(), AtFlags::empty());
            }
            if renamed.is_err() {
                let _ = unlinkat(seal.dir, seal.staged.as_str(), AtFlags::empty());
            }
            renamed.map(|()| has).or(Err(Unknown::Unsealed))
        }
        Make::Never(stood) => exchanged_in(seal, stood, written).map(|()| has),
    }
}

/// The atomic half of [`commit`]'s reseal: `hook` at [`Step::Linked`], the
/// exchange of `staged` with `name`, and the proof that what the exchange
/// put under `staged` is the inode of the record `stood`, still held open so
/// the filesystem cannot have freed and reused its number. A record gone
/// before the exchange leaves nothing behind; a foreign file planted at the
/// name is exchanged back and refused, and where that exchange back finds
/// the name gone the removal stands and nothing is made at it; a filesystem
/// without an exchange refuses with [`Unknown::Exchange`]. Once the proof
/// holds, the old record keeps only its prior name until [`retired`] removes
/// it.
fn exchanged_in(seal: &Seal<'_>, stood: &Record, written: &Record) -> Result<(), Unknown> {
    let prior = seal.prior();
    if (seal.hook)(Step::Linked, seal.dir, seal.name).is_err() {
        let _ = unlinkat(seal.dir, prior.as_str(), AtFlags::empty());
        let _ = unlinkat(seal.dir, seal.staged.as_str(), AtFlags::empty());
        return Err(Unknown::Unsealed);
    }
    match (seal.exchange)(seal.dir, seal.staged.as_str(), seal.dir, seal.name) {
        Ok(()) => {}
        Err(error) => {
            let _ = unlinkat(seal.dir, prior.as_str(), AtFlags::empty());
            let _ = unlinkat(seal.dir, seal.staged.as_str(), AtFlags::empty());
            return Err(exchange_error(error));
        }
    }
    // The held descriptor keeps the record's inode from being freed and
    // reused, so the staged name's identity alone proves it is that record.
    let ours = regular_at(seal.dir, seal.staged.as_str()).is_some_and(|(_, _, id)| id == stood.id);
    if ours {
        // The record that stood keeps only its prior name until retirement.
        let _ = unlinkat(seal.dir, seal.staged.as_str(), AtFlags::empty());
        return Ok(());
    }
    // A foreign file was planted at the name. Swap it back; where even that
    // fails, nothing is renamed over the name: a rename would make a name
    // the exchange met gone. The planted file then stays under the staged
    // name, and this seal's successor is removed rather than left as the
    // record.
    put_foreign_back(seal, &prior, written);
    Err(Unknown::Unsealed)
}

/// The file the exchange put under the staged name, which is not the record
/// the reseal read, put back at `name` by the exchange alone. Where the
/// exchange meets `ENOENT`, the name was removed: the removal stands, no
/// rename runs, and nothing is made at the name. Where it fails for any
/// other reason, no rename runs either, because a plain rename would make
/// the name if the removal had taken it. In either failure the planted file
/// stays under the staged name (the next seal of this record removes it
/// first), and this seal's own successor is removed where it is still at
/// the name: a refused seal never leaves a record standing, and never
/// deletes a file a hand made at the name. The record that stood is removed
/// either way, for the removal or replacement that met it stands.
fn put_foreign_back(seal: &Seal<'_>, prior: &str, written: &Record) {
    let (dir, staged, name) = (seal.dir, seal.staged.as_str(), seal.name);
    let put_back = (seal.exchange)(dir, staged, dir, name).is_ok();
    // A file a hand made at the name is not this seal's and stays.
    let ours = || regular_at(dir, name).is_some_and(|(_, _, id)| id == written.id);
    if !put_back && ours() {
        let _ = unlinkat(dir, name, AtFlags::empty());
    }
    let _ = unlinkat(dir, prior, AtFlags::empty());
    if put_back {
        let _ = unlinkat(dir, staged, AtFlags::empty());
    }
}

/// A seal that failed or ran late after its commit, undone: the record
/// that `stood` put back from its prior name by the same exchange, or,
/// where none stood, the record it made removed; then handed to `hook` and
/// the directory synced again. Where the name is gone, the removal stands:
/// the record that stood is removed and nothing is written at the name. A
/// first seal's undo uses links, renames and removals alone, so a
/// filesystem without an exchange still undoes what it may create
/// (decision 0108 ruling 3, M3 amended). Why not, where any of that fails:
/// the record that stood is then still under its prior name.
fn undone(seal: &Seal<'_>, stood: bool, written: &Record, make: Make<'_>) -> std::io::Result<()> {
    let prior = seal.prior();
    match (make, stood) {
        (Make::May, true) => put_made_back(seal, &prior, written)?,
        (Make::May, false) => remove_made(seal, written)?,
        (Make::Never(_), _) => put_stood_back(seal, &prior, written)?,
    }
    let _ = unlinkat(seal.dir, seal.staged.as_str(), AtFlags::empty());
    (seal.hook)(Step::Undone, seal.dir, seal.name)?;
    fsync(seal.dir).map_err(Into::into)
}

/// The record a first seal found standing put back at `name` by a plain
/// rename: the file it made is the one this seal wrote, or a hand's file
/// stays and the undo refuses.
fn put_made_back(seal: &Seal<'_>, prior: &str, written: &Record) -> std::io::Result<()> {
    match regular_at(seal.dir, seal.name) {
        None => {
            let _ = unlinkat(seal.dir, prior, AtFlags::empty());
            Ok(())
        }
        Some((_, _, id)) if id == written.id => {
            renameat(seal.dir, prior, seal.dir, seal.name).map_err(std::io::Error::from)?;
            Ok(())
        }
        Some(_) => {
            let _ = unlinkat(seal.dir, prior, AtFlags::empty());
            Err(std::io::Error::other(
                "the name holds a file this seal did not write",
            ))
        }
    }
}

/// The record a first seal made removed, where it is still the file this
/// seal wrote; a hand's file stays and the undo refuses.
fn remove_made(seal: &Seal<'_>, written: &Record) -> std::io::Result<()> {
    match regular_at(seal.dir, seal.name) {
        None => Ok(()),
        Some((_, _, id)) if id == written.id => {
            unlinkat(seal.dir, seal.name, AtFlags::empty()).map_err(std::io::Error::from)
        }
        Some(_) => Err(std::io::Error::other(
            "the name holds a file this seal did not write",
        )),
    }
}

/// The record a reseal replaced put back at `name` by the exchange: the
/// record that stood is linked under the staged name first, so the exchange
/// moves it back in one step and the successor it replaced is what the
/// staged name then holds. A name gone before the exchange means the
/// removal stands. A name holding a file this seal did not write is left
/// as it is and the undo refuses.
fn put_stood_back(seal: &Seal<'_>, prior: &str, written: &Record) -> std::io::Result<()> {
    let (dir, staged, name) = (seal.dir, seal.staged.as_str(), seal.name);
    let gone = || {
        let _ = unlinkat(dir, prior, AtFlags::empty());
        let _ = unlinkat(dir, staged, AtFlags::empty());
    };
    let _ = unlinkat(dir, staged, AtFlags::empty());
    linkat(dir, prior, dir, staged, AtFlags::empty()).map_err(std::io::Error::from)?;
    match (seal.exchange)(dir, staged, dir, name) {
        Ok(()) => {
            (seal.hook)(Step::Exchanged, dir, name)?;
            let ours = regular_at(dir, staged).is_some_and(|(_, _, id)| id == written.id);
            if !ours {
                put_foreign_back(seal, prior, written);
                return Err(std::io::Error::other(
                    "the name holds a file this seal did not write",
                ));
            }
            gone();
            Ok(())
        }
        Err(Errno::NOENT) => {
            gone();
            Ok(())
        }
        Err(error) => {
            let _ = unlinkat(dir, staged, AtFlags::empty());
            Err(std::io::Error::from(error))
        }
    }
}

/// A seal of `name` in `dir` cut short before it was final, undone: the
/// record that stood, still under its prior name, put back over `name` by
/// a reseal's exchange, or a first seal's successor replaced by the record
/// it stood over with a plain rename. Where the name is gone, the removal
/// stands: the record that stood is removed, and nothing is written at the
/// name. The record that stood is told from a successor left by a reseal
/// cut short by its own bytes (`read`). Nothing where no prior name stands;
/// refused where it cannot be undone.
fn restored(seal: &Seal<'_>, make: Make<'_>, read: &[u8]) -> Result<(), Unknown> {
    let Some((prior, kept)) = cut_short(seal) else {
        let _ = unlinkat(seal.dir, seal.staged.as_str(), AtFlags::empty());
        return Ok(());
    };
    match make {
        Make::May => restored_made(seal, &prior, kept),
        Make::Never(_) => restored_stood(seal, read, &prior),
    }
}

/// The prior name a seal of this record was cut short under, with the file
/// that stood, where one stands: the two restores' shared first look.
fn cut_short(seal: &Seal<'_>) -> Option<(String, Identity)> {
    let prior = seal.prior();
    let (_, _, kept) = regular_at(seal.dir, &prior)?;
    Some((prior, kept))
}

/// [`restored`] for a first seal: a plain rename puts the record that stood
/// back over this seal's predecessor, and a name gone means the removal
/// stands.
fn restored_made(seal: &Seal<'_>, prior: &str, kept: Identity) -> Result<(), Unknown> {
    let (dir, staged, name) = (seal.dir, seal.staged.as_str(), seal.name);
    match regular_at(dir, name) {
        None => {
            let _ = unlinkat(dir, prior, AtFlags::empty());
        }
        Some((_, _, id)) if id == kept => {
            let _ = unlinkat(dir, prior, AtFlags::empty());
        }
        Some(_) => {
            renameat(dir, prior, dir, name).or(Err(Unknown::Unsealed))?;
        }
    }
    let _ = unlinkat(dir, staged, AtFlags::empty());
    fsync(dir).or(Err(Unknown::Unsealed))
}

/// [`restored`] for a reseal: the record that stood put back by the same
/// exchange, or, where the name is gone, the removal stands. A prior name
/// holding a successor (an undo cut short after its exchange) is removed,
/// never exchanged in: the record that stood is the one whose own bytes
/// `read` names. A name holding neither refuses, leaving both as they are.
fn restored_stood(seal: &Seal<'_>, read: &[u8], prior: &str) -> Result<(), Unknown> {
    let (dir, staged, name) = (seal.dir, seal.staged.as_str(), seal.name);
    let named = regular_at(dir, name);
    let reads = |at: &str| bytes_at(dir, at).is_some_and(|bytes| bytes == read);
    if named.is_none() || reads(name) {
        // The name is gone, or holds the record that stood already: a
        // successor left under the prior name goes, and nothing is written.
        let _ = unlinkat(dir, prior, AtFlags::empty());
        let _ = unlinkat(dir, staged, AtFlags::empty());
        return fsync(dir).or(Err(Unknown::Unsealed));
    }
    if !reads(prior) {
        return Err(Unknown::Unsealed);
    }
    let _ = unlinkat(dir, staged, AtFlags::empty());
    linkat(dir, prior, dir, staged, AtFlags::empty()).or(Err(Unknown::Unsealed))?;
    if (seal.hook)(Step::Restored, dir, name).is_err() {
        let _ = unlinkat(dir, staged, AtFlags::empty());
        return Err(Unknown::Unsealed);
    }
    match (seal.exchange)(dir, staged, dir, name) {
        Ok(()) | Err(Errno::NOENT) => {
            let _ = unlinkat(dir, prior, AtFlags::empty());
            let _ = unlinkat(dir, staged, AtFlags::empty());
            fsync(dir).or(Err(Unknown::Unsealed))
        }
        Err(error) => {
            let _ = unlinkat(dir, staged, AtFlags::empty());
            Err(exchange_error(error))
        }
    }
}

/// The bytes of the file `name` in `dir`, as it stands, within MB3's
/// request bound; none where it cannot be read as this user's own regular
/// file.
fn bytes_at(dir: &OwnedFd, name: &str) -> Option<Vec<u8>> {
    let (file, _, _) = regular_at(dir, name)?;
    let mut bytes = Vec::new();
    let read = std::fs::File::from(file)
        .take(REPLY_BYTES_MAX + 1)
        .read_to_end(&mut bytes);
    read.ok()
        .filter(|_| bytes.len() as u64 <= REPLY_BYTES_MAX)?;
    Some(bytes)
}

/// Why an exchange refused: the filesystem has none where its `EINVAL`,
/// `ENOTSUP`/`EOPNOTSUPP` or `ENOSYS` says so, and every other error is the
/// seal's own failure.
fn exchange_error(error: Errno) -> Unknown {
    if (error == Errno::INVAL)
        | (error == Errno::NOTSUP)
        | (error == Errno::OPNOTSUPP)
        | (error == Errno::NOSYS)
    {
        Unknown::Exchange
    } else {
        Unknown::Unsealed
    }
}
