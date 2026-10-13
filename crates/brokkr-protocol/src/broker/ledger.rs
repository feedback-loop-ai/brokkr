//! The broker's private ledger (decision 0077; slice two U6d, design D7,
//! CR3, CC2): the durable record of one plan's one lifetime, which the
//! broker alone writes, beside its plan in the attempt's protected
//! directory, and which the engine alone folds into the journal. The
//! broker writes no journal.
//!
//! A ledger is one closed JSON object per line, at most 4 KiB with its
//! newline, each numbered by a contiguous record sequence.
//! Exactly one Opened, naming the plan's owner and digest, comes first;
//! each accepted call has the next call sequence and one Started, then at
//! most one Terminal for that call and tool, a call begun while another is
//! unresolved being a local refusal that only a refused Terminal ends; one
//! Closed comes last, once
//! every accepted call has its Terminal, and carries the session's
//! disposition: Clean, or Failed with the first fatal cause. A ledger is
//! created exclusively, so a plan's second lifetime refuses before it
//! starts anything. Every record is synced before the broker acts on it:
//! a Started is a call's acceptance, before it is forwarded or refused
//! locally; a Terminal precedes the response's delivery; the Closed
//! precedes the broker's exit. Arguments, request identities, response
//! bodies and child output are never recorded.
//!
//! Every record passes the final guard before it is appended (U6f1a; MB4,
//! CR2, operator ruling 2026-10-13): its value is masked by the shared
//! masker, which may change a call's tool and nothing else, and the very
//! line appended passes `secret::Guard`, the one guard the broker's output
//! edge shares. A record masking would change otherwise, or whose line the
//! guard still refuses, as in a sequence number that is a bound value by
//! what it is worth, is never appended, and the masking cause is latched as
//! the session's. The bindings are the session's: a ledger opened before
//! they are known is given them as soon as they are ([`Ledger::bind`]), its
//! Opened then guarded by them too.
//!
//! [`Lifecycle`] is the one statement of D7's order: the writer admits
//! each record through it before writing it, and [`recover`] reads a
//! settled ledger through it, keeping only the prefix it verifies.

use std::collections::BTreeSet;
use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::os::fd::AsFd;

use rustix::fs::{openat, Mode, OFlags};
use rustix::io::Errno;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Object, Owner, Refusal};
use crate::secret::{self, BoundSecret, Guard};

/// The most bytes one record holds, its newline included (CR3).
const RECORD_MAX: usize = 4 << 10;

/// The most calls one broker attempt accepts (CR3).
const CALLS_MAX: u64 = 4096;

// No ledger the lifecycle admits reaches its byte bound: at most one
// Opened, one Started and one Terminal per call and one Closed, each within
// the record bound. Reserving a call's Terminal and the closure is so a
// matter of the call bound and the record bound alone.
const _: () = {
    /// The most bytes one ledger holds (CR3).
    const LEDGER_MAX: usize = 64 << 20;
    assert!((2 * CALLS_MAX as usize + 2) * RECORD_MAX <= LEDGER_MAX);
};

/// One record, read only through [`Object`].
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "record", rename_all = "lowercase", deny_unknown_fields)]
enum Entry {
    Opened {
        seq: u64,
        owner: Owner,
        plan: String,
    },
    Started {
        seq: u64,
        call: u64,
        tool: String,
    },
    Terminal {
        seq: u64,
        call: u64,
        tool: String,
        outcome: Outcome,
    },
    Closed {
        seq: u64,
        disposition: Disposition,
    },
}

impl Entry {
    fn seq(&self) -> u64 {
        match self {
            Entry::Opened { seq, .. }
            | Entry::Started { seq, .. }
            | Entry::Terminal { seq, .. }
            | Entry::Closed { seq, .. } => *seq,
        }
    }

    /// The record as its line, where it fits [`RECORD_MAX`].
    fn line(&self) -> Option<Vec<u8>> {
        let mut line = serde_json::to_vec(self).ok()?;
        line.push(b'\n');
        (line.len() <= RECORD_MAX).then_some(line)
    }

    /// The record `line` holds, whole and closed, or none.
    fn read(line: &[u8]) -> Option<Entry> {
        let mut reader = serde_json::Deserializer::from_slice(line);
        let entry = Entry::deserialize(Object(&mut reader)).ok()?;
        reader.end().ok()?;
        Some(entry)
    }
}

/// How an accepted call ended (CC2): its child answered, its child's
/// answer was an error, or the broker refused it without forwarding it. A
/// call with no Terminal was interrupted, which only settlement says.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Succeeded,
    Failed,
    Refused(Denial),
}

impl Outcome {
    /// The outcome whose Terminal is longest, which a call's acceptance
    /// reserves room for.
    const LONGEST: Outcome = Outcome::Refused(Denial::Ungranted);
}

/// Why the broker refused an accepted call itself (CR3): the tool is not
/// granted, a call is already active, or the retention budget is spent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Denial {
    Ungranted,
    Active,
    Budget,
}

/// How a session ended (CR3): healthy, or failed with its first fatal
/// cause.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    Clean,
    Failed(Failure),
}

/// A session's fatal cause, latched first and never cleared (CR3), in D7's
/// order: MB3 and MB4's box admission, readiness, handoff and exec causes,
/// which a plan refused after its ledger opened takes; the server's
/// version, protocol, response limit or deadline; a response it cannot
/// safely mask; or the ledger's own limit. A served cause reads in its
/// shared refusal's words, and is latched as its own variant, never as a
/// refused plan. An ordinary tool error or a local denial is a call's
/// outcome, never one of these.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Failure {
    #[error(transparent)]
    Refused(Refusal),
    #[error("MCP server version does not match the dialect's pinned version")]
    Version,
    #[error("MCP server protocol is invalid")]
    Protocol,
    #[error("{}", Refusal::Response)]
    Response,
    #[error("{}", Refusal::Timeout)]
    Timeout,
    #[error("{}", Refusal::Masking)]
    Masking,
    #[error("broker ledger exceeds the attempt limit")]
    Limit,
}

/// Why the broker could not make its record (CR3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Unrecorded {
    /// The plan's ledger already exists: a second lifetime, refused before
    /// anything is started.
    #[error("broker plan has already been opened")]
    Reopened,
    /// A record was not written and synced whole, or would not fit: the
    /// broker acts on nothing it could not record, and records nothing
    /// after it.
    #[error("broker call could not be recorded")]
    Record,
    /// A call's Terminal would not fit the record bound, so the call is not
    /// accepted: the ledger's limit, latched as the session's cause.
    #[error("{}", Failure::Limit)]
    Limit,
    /// A record masking would change past its tool, or whose line the
    /// final guard still refuses, is never appended: the masking cause,
    /// latched as the session's.
    #[error("{}", Failure::Masking)]
    Masking,
}

/// Why a ledger verifies no further (CR3, CR4): the first record out of
/// D7's order, of another owner, conflicting with a record before it,
/// partial, malformed or out of sequence; or a ledger that ends without
/// its closure, which is out of order too.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum Invalid {
    #[error("broker ledger lifecycle is invalid")]
    Lifecycle,
    #[error("broker ledger owner does not match this attempt")]
    Owner,
    #[error("broker ledger contains a conflicting duplicate")]
    Conflict,
    #[error("broker ledger ends in a partial record")]
    Partial,
    #[error("broker ledger record is malformed")]
    Malformed,
    #[error("broker ledger sequence is not contiguous")]
    Gap,
}

/// D7's lifecycle, as far as its records go: the owner and plan digest the
/// Opened must name, how many records it has admitted, whether the ledger
/// is opened, each accepted call's tool and outcome in call order, none
/// while it is unresolved, how many are unresolved, the calls begun while
/// another was, which only a local refusal may end, and the closure, once
/// there is one.
#[derive(Debug, PartialEq, Eq)]
pub struct Lifecycle {
    owner: Owner,
    plan: String,
    records: u64,
    opened: bool,
    calls: Vec<(String, Option<Outcome>)>,
    unresolved: usize,
    local: BTreeSet<u64>,
    closed: Option<Disposition>,
}

impl Lifecycle {
    fn new(owner: &Owner, plan: &str) -> Lifecycle {
        Lifecycle {
            owner: owner.clone(),
            plan: plan.to_owned(),
            records: 0,
            opened: false,
            calls: Vec::new(),
            unresolved: 0,
            local: BTreeSet::new(),
            closed: None,
        }
    }

    /// The Opened the lifecycle admits first, its one record numbered one:
    /// what the writer appends, and what binding re-reads (decision 0071
    /// ruling 5).
    fn opened(&self) -> Entry {
        Entry::Opened {
            seq: 1,
            owner: self.owner.clone(),
            plan: self.plan.clone(),
        }
    }

    /// Each accepted call's tool and its outcome, none where its Terminal
    /// is not verified: an interrupted call, once the broker has settled.
    pub fn calls(&self) -> &[(String, Option<Outcome>)] {
        &self.calls
    }

    /// The verified closure, if any.
    pub fn closed(&self) -> Option<&Disposition> {
        self.closed.as_ref()
    }

    /// Admit `entry` as the next record, or say why it is out of order.
    fn admit(&mut self, entry: Entry) -> Result<(), Invalid> {
        ensure(entry.seq() == self.records + 1, Invalid::Gap)?;
        let open = self.opened & self.closed.is_none();
        match entry {
            Entry::Opened { owner, plan, .. } => {
                ensure(!self.opened, Invalid::Lifecycle)?;
                ensure((&owner, &plan) == (&self.owner, &self.plan), Invalid::Owner)?;
                self.opened = true;
            }
            Entry::Started { call, tool, .. } => {
                ensure(open, Invalid::Lifecycle)?;
                ensure(call == self.calls.len() as u64 + 1, Invalid::Gap)?;
                ensure(call <= CALLS_MAX, Invalid::Lifecycle)?;
                // D7: a call begun while another is unresolved is an
                // already decoded local refusal, and nothing else.
                if self.unresolved > 0 {
                    self.local.insert(call);
                }
                self.calls.push((tool, None));
                self.unresolved += 1;
            }
            Entry::Terminal {
                call,
                tool,
                outcome,
                ..
            } => {
                ensure(open, Invalid::Lifecycle)?;
                self.ended(call, tool, outcome)?;
            }
            Entry::Closed { disposition, .. } => {
                ensure(open & (self.unresolved == 0), Invalid::Lifecycle)?;
                self.closed = Some(disposition);
            }
        }
        self.records += 1;
        Ok(())
    }

    /// Resolve accepted call `call` of `tool` with `outcome`: a call never
    /// begun has no Terminal yet; one already ended, or begun for another
    /// tool, conflicts; one begun while another was unresolved ends only
    /// refused.
    fn ended(&mut self, call: u64, tool: String, outcome: Outcome) -> Result<(), Invalid> {
        let index = usize::try_from(call)
            .ok()
            .and_then(|call| call.checked_sub(1));
        let begun = index.and_then(|index| self.calls.get_mut(index));
        let begun = begun.ok_or(Invalid::Lifecycle)?;
        ensure((begun.0 == tool) & begun.1.is_none(), Invalid::Conflict)?;
        let refused = matches!(outcome, Outcome::Refused(_));
        ensure(refused | !self.local.contains(&call), Invalid::Lifecycle)?;
        begun.1 = Some(outcome);
        self.unresolved -= 1;
        Ok(())
    }
}

/// `holds`, or `error`.
fn ensure<E>(holds: bool, error: E) -> Result<(), E> {
    match holds {
        true => Ok(()),
        false => Err(error),
    }
}

/// What a settled ledger's `bytes` prove for the plan sealed at `plan` for
/// `owner` (CR3, CR4): the lifecycle of the prefix that verifies, and why
/// verification stopped there, if it did. A ledger whose every record
/// verifies but that has no closure is out of order: closure that could not
/// be written leaves only the verified prefix, never a success.
pub fn recover(bytes: &[u8], owner: &Owner, plan: &str) -> (Lifecycle, Option<Invalid>) {
    let mut lifecycle = Lifecycle::new(owner, plan);
    let mut lines = bytes.split_inclusive(|byte| *byte == b'\n');
    let invalid = lines.find_map(|line| verified(&mut lifecycle, line).err());
    let unclosed = lifecycle.closed.is_none().then_some(Invalid::Lifecycle);
    (lifecycle, invalid.or(unclosed))
}

/// Admit the record `line` holds to `lifecycle`: a line with no newline is
/// a partial record, and one over the record bound or not one closed
/// record is malformed.
fn verified(lifecycle: &mut Lifecycle, line: &[u8]) -> Result<(), Invalid> {
    ensure(line.ends_with(b"\n"), Invalid::Partial)?;
    ensure(line.len() <= RECORD_MAX, Invalid::Malformed)?;
    lifecycle.admit(Entry::read(line).ok_or(Invalid::Malformed)?)
}

/// How a ledger's line is appended whole and synced.
type Append = Box<dyn FnMut(&[u8]) -> std::io::Result<()>>;

/// The writer of one plan's ledger: its records' lifecycle, how a line is
/// appended and synced, the first fatal cause, once one is latched, and the
/// final guard of the session's bindings, which every record passes.
pub struct Ledger<'b> {
    append: Append,
    lifecycle: Lifecycle,
    latched: Option<Failure>,
    broken: bool,
    guard: Guard<'b>,
}

/// Room for one more call, which holds the ledger until the call is begun
/// or the room given up: the call bound not reached, and so the call's
/// Terminal and the closure reserved.
pub struct Room<'l, 'b>(&'l mut Ledger<'b>);

/// An accepted call: its Started durable, its Terminal not yet written.
#[derive(Debug)]
pub struct Call {
    call: u64,
    tool: String,
}

impl<'b> Ledger<'b> {
    /// Create the ledger of the plan named `plan` in the attempt directory
    /// `dir`, beside the plan, exclusively and this user's alone, and open
    /// it for the plan sealed at `digest` for `owner`, every record guarded
    /// by `bindings`. A ledger already there is a second lifetime.
    pub fn open(
        dir: impl AsFd,
        plan: &OsStr,
        (owner, digest): (&Owner, &str),
        bindings: &'b [BoundSecret],
    ) -> Result<Ledger<'b>, Unrecorded> {
        let mut name = OsString::from(plan);
        name.push(".ledger");
        let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW;
        let created = openat(
            &dir,
            &name,
            flags | OFlags::CLOEXEC,
            Mode::RUSR | Mode::WUSR,
        );
        let created = created.map_err(|errno| match errno {
            Errno::EXIST => Unrecorded::Reopened,
            _ => Unrecorded::Record,
        })?;
        rustix::fs::fsync(&dir).or(Err(Unrecorded::Record))?;
        Ledger::durable(std::fs::File::from(created), (owner, digest), bindings)
    }

    /// Open the ledger created as `file`: each line written whole, then its
    /// data synced, before the writer answers.
    fn durable(
        mut file: std::fs::File,
        sealed: (&Owner, &str),
        bindings: &'b [BoundSecret],
    ) -> Result<Ledger<'b>, Unrecorded> {
        let append = move |line: &[u8]| file.write_all(line).and_then(|()| file.sync_data());
        Ledger::open_with(append, sealed, bindings)
    }

    /// Open a ledger whose lines `append` writes and syncs, every record
    /// guarded by `bindings`: its Opened for the plan sealed at `digest`
    /// for `owner`, recorded.
    pub fn open_with(
        append: impl FnMut(&[u8]) -> std::io::Result<()> + 'static,
        (owner, digest): (&Owner, &str),
        bindings: &'b [BoundSecret],
    ) -> Result<Ledger<'b>, Unrecorded> {
        let mut ledger = Ledger {
            append: Box::new(append),
            lifecycle: Lifecycle::new(owner, digest),
            latched: None,
            broken: false,
            guard: Guard::new(bindings),
        };
        let opened = ledger.lifecycle.opened();
        ledger.record(|_| opened)?;
        Ok(ledger)
    }

    /// Room for one more call, asked before the next request is read: at
    /// the call bound, none, and the ledger's limit latched, so the request
    /// stays unread and the closure says why.
    pub fn room(&mut self) -> Result<Room<'_, 'b>, Failure> {
        match (self.lifecycle.calls.len() as u64) < CALLS_MAX {
            true => Ok(Room(self)),
            false => {
                self.fail(Failure::Limit);
                Err(Failure::Limit)
            }
        }
    }

    /// Record `call`'s Terminal with its `outcome`, before its response is
    /// delivered: where it is not recorded, nothing is delivered, and the
    /// call stays begun.
    pub fn end(&mut self, call: Call, outcome: Outcome) -> Result<(), Unrecorded> {
        let Call { call, tool } = call;
        self.record(|seq| Entry::Terminal {
            seq,
            call,
            tool,
            outcome,
        })
    }

    /// Guard every record from now on by the session's `bindings`, given as
    /// soon as they are known and before any call is begun (MB4, CR2). The
    /// Opened, written before them, holds only the owner and the plan's
    /// digest: its very line is passed by their guard now, and where it is
    /// refused, the masking cause is latched, as for any record, and the
    /// Opened stays as it was written.
    pub fn bind(&mut self, bindings: &'b [BoundSecret]) -> Result<(), Unrecorded> {
        self.guard = Guard::new(bindings);
        let line = self.lifecycle.opened().line().ok_or(Unrecorded::Record)?;
        match self.guard.json(&line) {
            true => Ok(()),
            false => Err(self.unmasked()),
        }
    }

    /// Latch `failure` as the session's, unless a cause is latched already:
    /// the first fatal cause is the one the closure carries.
    pub fn fail(&mut self, failure: Failure) {
        self.latched.get_or_insert(failure);
    }

    /// Close the session, before the broker exits: Failed with the latched
    /// cause, or Clean where none was; refused while a call is unresolved.
    /// Where the closure is not recorded, only the verified prefix remains.
    pub fn close(mut self) -> Result<Disposition, Unrecorded> {
        let disposition = self
            .latched
            .take()
            .map_or(Disposition::Clean, Disposition::Failed);
        let closed = disposition.clone();
        self.record(|seq| Entry::Closed {
            seq,
            disposition: closed,
        })?;
        Ok(disposition)
    }

    /// The record `entry` makes, numbered next, masked and guarded
    /// ([`Ledger::masked`]), admitted by the lifecycle as it is written,
    /// and appended whole and synced. Once one append has failed, what it
    /// left is unknown, and nothing more is recorded.
    fn record(&mut self, entry: impl FnOnce(u64) -> Entry) -> Result<(), Unrecorded> {
        ensure(!self.broken, Unrecorded::Record)?;
        let entry = entry(self.lifecycle.records + 1);
        let (entry, line) = self.masked(&entry)?;
        self.lifecycle.admit(entry).or(Err(Unrecorded::Record))?;
        self.broken = true;
        (self.append)(&line).or(Err(Unrecorded::Record))?;
        self.broken = false;
        Ok(())
    }

    /// The record `entry` as it is written, and its line (MB4, CR2): its
    /// value masked by the shared masker, read back as a record, and its
    /// line passed by the final guard, those very bytes being what is
    /// appended. Masking may change the call's tool and nothing else: a
    /// record it would change otherwise, as in an outcome or a key, or
    /// whose line the guard still refuses, is not written, and the masking
    /// cause is latched as the session's. One over the record bound is not
    /// recorded either.
    fn masked(&mut self, entry: &Entry) -> Result<(Entry, Vec<u8>), Unrecorded> {
        let written = serde_json::to_value(entry).or(Err(Unrecorded::Record))?;
        let mut masked = written.clone();
        secret::mask_json(&mut masked, self.guard.bindings());
        let read = (but_tool(&masked) == but_tool(&written))
            .then(|| Entry::read(&serde_json::to_vec(&masked).unwrap_or_default()))
            .flatten();
        let Some(read) = read else {
            return Err(self.unmasked());
        };
        let line = read.line().ok_or(Unrecorded::Record)?;
        match self.guard.json(&line) {
            true => Ok((read, line)),
            false => Err(self.unmasked()),
        }
    }

    /// The masking cause latched as the session's, and the record refused
    /// for it.
    fn unmasked(&mut self) -> Unrecorded {
        self.fail(Failure::Masking);
        Unrecorded::Masking
    }
}

/// A record's `value`, an object, but for its tool, the one member masking
/// may change.
fn but_tool(value: &Value) -> Value {
    let mut value = value.clone();
    value.as_object_mut().into_iter().for_each(|members| {
        members.remove("tool");
    });
    value
}

impl Room<'_, '_> {
    /// Accept a call to `tool`: its Started recorded, so it may be
    /// forwarded or refused locally, and nothing done where it is not. Its
    /// Terminal is reserved first, with the longest outcome and the tool as
    /// masking writes it: where it would not fit, the call is not accepted
    /// and the ledger's limit is latched. A call begun while another is
    /// unresolved may only be refused.
    pub fn begin(self, tool: &str) -> Result<Call, Unrecorded> {
        let ledger = self.0;
        let terminal = Entry::Terminal {
            seq: u64::MAX,
            call: u64::MAX,
            tool: secret::mask_text(tool.as_bytes(), ledger.guard.bindings()),
            outcome: Outcome::LONGEST,
        };
        if terminal.line().is_none() {
            ledger.fail(Failure::Limit);
            return Err(Unrecorded::Limit);
        }
        let call = ledger.lifecycle.calls.len() as u64 + 1;
        let started = tool.to_owned();
        ledger.record(|seq| Entry::Started {
            seq,
            call,
            tool: started,
        })?;
        Ok(Call {
            call,
            tool: tool.to_owned(),
        })
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests;
