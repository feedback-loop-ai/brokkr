//! A retaining plan's staging (decision 0065 slice two, U8b; CR1, CR2,
//! CR3): its share of the attempt's budget, the reservation each call it
//! forwards takes of it, the stage of each answered call's prepared bytes
//! beside the ledger, and the response its Terminal cites for them.

use std::ffi::{OsStr, OsString};
use std::fs::File;
use std::io::Write;
use std::os::fd::OwnedFd;

use rustix::fs::{openat, Mode, OFlags};
use serde::{Deserialize, Deserializer, Serialize};

use super::{ensure, Call, Denial, Ledger, Outcome, Unrecorded};
use crate::broker::Object;

/// One attempt's retention budget, which its retaining plans share
/// (CR3). Every share is at most the budget, and the engine publishes at
/// most what was staged, so newly written response data stays within
/// twice it, CR3's 512 MiB.
pub(super) const BUDGET: u64 = 256 << 20;

/// What a retaining call reserves of its plan's share before it is
/// forwarded (CR3): MB3's bound on a response, which no prepared response
/// that is delivered passes.
pub(super) const RESERVATION: u64 = 8 << 20;

/// Each retaining plan's share of the attempt's budget (operator ruling
/// 2026-10-15): the budget divided by the number of retaining plans the
/// sealed inventory pins, permitted fallback slots included, as the engine
/// divides it; none where there are none. A share under one reservation
/// refuses every call for the budget.
pub(super) fn share(retaining: usize) -> u64 {
    BUDGET.checked_div(retaining as u64).unwrap_or(0)
}

/// The staged response a retaining call's Terminal cites (CR2, CC2): the
/// sha256 of the very bytes staged, in lowercase hex, and how many there
/// are, never more than one reservation. Read past its record only through
/// [`super::Lifecycle::cited`] (U8c).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Response {
    pub(super) sha256: String,
    pub(super) bytes: u64,
}

impl Response {
    /// The longest a Terminal's response is written, which a retaining
    /// call's acceptance reserves room for.
    pub(super) fn longest() -> Response {
        Response {
            sha256: "f".repeat(64),
            bytes: u64::MAX,
        }
    }

    /// Whether it is a sha256 as the engine writes one, of at most one
    /// reservation's bytes.
    fn sound(&self) -> bool {
        let hex = |byte: u8| byte.is_ascii_digit() | (b'a'..=b'f').contains(&byte);
        (self.sha256.len() == 64) & self.sha256.bytes().all(hex) & (self.bytes <= RESERVATION)
    }
}

/// A Terminal's response, where it has one: an object read as every record
/// is ([`Object`]), never null and never positional, and sound, or the
/// record is malformed.
pub(super) fn response<'de, D: Deserializer<'de>>(member: D) -> Result<Option<Response>, D::Error> {
    let response = Response::deserialize(Object(member))?;
    match response.sound() {
        true => Ok(Some(response)),
        false => Err(serde::de::Error::custom("unsound response")),
    }
}

/// How a retained response's bytes are staged whole and synced, by the
/// number of the call they answer.
type Stage = Box<dyn FnMut(u64, &[u8]) -> std::io::Result<()>>;

/// A retaining plan's staging (CR2, CR3): how its responses are staged,
/// and what is left of its share.
pub(super) struct Retention {
    stage: Stage,
    left: u64,
}

impl Ledger<'_> {
    /// Record `call`'s Terminal with its `outcome` once its server has
    /// answered with the prepared `bytes`, whose sha256 is `digest` (CR2,
    /// CR3). Where the plan retains its responses, those bytes are staged
    /// first, synced before the Terminal that cites them; their actual
    /// length stays charged to the share, and only the rest of the call's
    /// reservation returns. A response its call did not reserve room for,
    /// or a stage that fails, is not recorded, and the call stays begun.
    /// Where the plan retains nothing, the Terminal alone, with no digest.
    pub fn answered(
        &mut self,
        mut call: Call,
        outcome: Outcome,
        (bytes, digest): (&[u8], &str),
    ) -> Result<(), Unrecorded> {
        let Some(retention) = self.retention.as_mut() else {
            return self.end(call, outcome);
        };
        let staged = bytes.len() as u64;
        ensure(staged <= call.reserved, Unrecorded::Record)?;
        (retention.stage)(call.call, bytes).or(Err(Unrecorded::Record))?;
        call.reserved -= staged;
        self.released(&mut call);
        let response = Response {
            sha256: digest.to_owned(),
            bytes: staged,
        };
        self.terminal(call, outcome, Some(response))
    }

    /// Retain the plan's responses (CR2, CR3), its share that of one of
    /// `retaining` retaining plans ([`share`]): each staged beside the
    /// ledger of the plan named `plan` in the attempt directory `dir`
    /// ([`staged`]), a handle the ledger holds from now on, which must be
    /// closed on exec like every handle the broker holds, so no server
    /// inherits it.
    pub fn retain(&mut self, dir: OwnedFd, plan: &OsStr, retaining: usize) {
        let plan = plan.to_owned();
        let stage = move |call, bytes: &[u8]| staged(&DURABLE, (&dir, &plan), call, bytes);
        self.retain_with(stage, retaining);
    }

    /// Retain the plan's responses, each staged by `stage`, which writes
    /// and syncs them whole, its share that of one of `retaining`
    /// retaining plans.
    pub fn retain_with(
        &mut self,
        stage: impl FnMut(u64, &[u8]) -> std::io::Result<()> + 'static,
        retaining: usize,
    ) {
        self.retention = Some(Retention {
            stage: Box::new(stage),
            left: share(retaining),
        });
    }

    /// Reserve, before the accepted `call` is forwarded, room for its
    /// response (CR3), which the call holds until its Terminal: where the
    /// plan retains, one reservation of its share, or the budget's denial
    /// where less is left, so the call is refused before any external
    /// work; nothing where it retains nothing.
    pub fn reserve(&mut self, call: &mut Call) -> Result<(), Denial> {
        let Some(retention) = self.retention.as_mut() else {
            return Ok(());
        };
        ensure(retention.left >= RESERVATION, Denial::Budget)?;
        retention.left -= RESERVATION;
        call.reserved += RESERVATION;
        Ok(())
    }

    /// What `call` holds of the share returned to it, unstaged.
    pub(super) fn released(&mut self, call: &mut Call) {
        if let Some(retention) = self.retention.as_mut() {
            retention.left += std::mem::take(&mut call.reserved);
        }
    }
}

/// How a stage is made durable once its file is created, a step at a time,
/// each of which may fail: its bytes written whole, its data synced, and
/// then its directory synced, so its name is durable too.
pub(super) struct Steps {
    pub(super) write: fn(&mut File, &[u8]) -> std::io::Result<()>,
    pub(super) sync: fn(&File) -> std::io::Result<()>,
    pub(super) settle: fn(&OwnedFd) -> std::io::Result<()>,
}

/// The broker's own [`Steps`].
pub(super) const DURABLE: Steps = Steps {
    write: |file, bytes| file.write_all(bytes),
    sync: File::sync_data,
    settle: |dir| Ok(rustix::fs::fsync(dir)?),
};

/// Stage `bytes`, call `call`'s retained response, beside the ledger of the
/// plan named `plan` in the attempt directory `dir`, as
/// `<plan>.<call>.response`: created exclusively and never through a
/// symlink, this user's alone, then made durable by `steps`, every one of
/// them taken in order, the first that fails the stage's failure.
pub(super) fn staged(
    steps: &Steps,
    (dir, plan): (&OwnedFd, &OsStr),
    call: u64,
    bytes: &[u8],
) -> std::io::Result<()> {
    let mut name = OsString::from(plan);
    name.push(format!(".{call}.response"));
    let flags = OFlags::RDWR | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW;
    let created = openat(dir, &name, flags | OFlags::CLOEXEC, Mode::RUSR | Mode::WUSR)?;
    let mut file = File::from(created);
    (steps.write)(&mut file, bytes)?;
    (steps.sync)(&file)?;
    (steps.settle)(dir)
}
