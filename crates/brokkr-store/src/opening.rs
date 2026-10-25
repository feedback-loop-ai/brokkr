//! Opening one run (decision 0089 ruling 5): its whole chain read in seq
//! order and verified as it streams, so every readout of one run refuses a
//! corrupt run as [`Store::load`] does while holding one envelope of it at
//! a time, never the run (ruling 9); a reader that shows the run whole
//! keeps what the opening verifies ([`Store::open_kept`]), bounded by the
//! run (ruling 7).

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::envelope::{verify_chain_after, ChainError, EventEnvelope};
use rusqlite::params;

use crate::{patiently, Store, StoreError};

/// The head a run's opening verified its chain to: its last event's seq
/// and hash, or 0 and [`ZERO_HASH`] for a run with no event. A readout
/// reads the run's projections at this head once the run is opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opened {
    pub seq: u64,
    pub hash: String,
}

/// The chain verified so far: the run its first event names, which
/// [`brokkr_core::envelope::verify_chain`] holds every event to, and the
/// head the next event must continue.
struct Verified {
    run_id: String,
    head: Opened,
}

/// Verify `event` as the continuation of `chain`, or as a chain's first
/// event when nothing is verified yet, and carry the head forward to it.
fn extend(chain: &mut Option<Verified>, event: &EventEnvelope) -> Result<(), ChainError> {
    let (run_id, seq, hash) = match chain {
        Some(verified) => (
            verified.run_id.as_str(),
            verified.head.seq,
            verified.head.hash.as_str(),
        ),
        None => (event.run_id.as_str(), 0, ZERO_HASH),
    };
    verify_chain_after(run_id, seq, hash, std::slice::from_ref(event))?;
    match chain {
        // The head's hash is overwritten in place, so following the chain
        // allocates nothing per event.
        Some(verified) => {
            verified.head.seq = event.seq;
            verified.head.hash.clone_from(&event.event_hash);
        }
        None => {
            let head = Opened {
                seq: event.seq,
                hash: event.event_hash.clone(),
            };
            *chain = Some(Verified {
                run_id: event.run_id.clone(),
                head,
            })
        }
    }
    Ok(())
}

impl Store {
    /// Open `run_id`: every row of it in seq order, each verified as it is
    /// read, with the checks [`Store::load`] makes, by brokkr-core's
    /// [`verify_chain_after`] over one envelope at a time, and the head
    /// verified returned. Nothing of the run is collected. Its refusals
    /// are `load`'s, in `load`'s order, which reads every row before it
    /// parses one and parses every row before it verifies one: a row the
    /// journal cannot read outranks a row that does not parse, which
    /// outranks the first chain defect. A run the journal lacks is
    /// [`StoreError::RunNotFound`]; a run with no event opens at the
    /// empty head. It waits on a peer's lock as `load` does, and its
    /// contention is named `load`'s, so a readout that moves from `load`
    /// to this opening refuses in the same words.
    pub fn open_run(&self, run_id: &str) -> Result<Opened, StoreError> {
        patiently("load", self.patience, || self.open_once(run_id, drop))
    }

    /// [`Store::open_run`], keeping every envelope it verifies: the head
    /// verified and the run's events in seq order, as [`Store::load`]
    /// reads them, from the one pass that verifies them, for a reader that
    /// shows the run whole and so holds it, bounded by the run (decision
    /// 0089 ruling 7). Its refusals, their order and their words are the
    /// opening's, and it keeps nothing of a run it refuses.
    pub fn open_kept(&self, run_id: &str) -> Result<(Opened, Vec<EventEnvelope>), StoreError> {
        patiently("load", self.patience, || {
            let mut kept = Vec::new();
            let opened = self.open_once(run_id, |event| kept.push(event))?;
            Ok((opened, kept))
        })
    }

    /// The opening, each event it verifies handed to `keep`.
    fn open_once(
        &self,
        run_id: &str,
        mut keep: impl FnMut(EventEnvelope),
    ) -> Result<Opened, StoreError> {
        let mut statement = self
            .conn
            .prepare("SELECT envelope FROM events WHERE run_id = ?1 ORDER BY seq")?;
        let mut rows = statement.query(params![run_id])?;
        let (mut chain, mut defect) = (None, None);
        while let Some(row) = rows.next()? {
            // The row's text as SQLite holds it, borrowed, so the parse is
            // the one copy of the envelope. `load` reads it as a `String`,
            // which fails exactly where the borrow does, in its words.
            let Ok(text) = row.get_ref(0)?.as_str() else {
                return Err(row.get::<_, String>(0).expect_err("unborrowed").into());
            };
            if matches!(defect, Some(StoreError::Json(_))) {
                continue;
            }
            match serde_json::from_str::<EventEnvelope>(text) {
                Err(unparsed) => defect = Some(unparsed.into()),
                Ok(event) if defect.is_none() => match extend(&mut chain, &event) {
                    Ok(()) => keep(event),
                    Err(broken) => defect = Some(broken.into()),
                },
                Ok(_) => {}
            }
        }
        if let Some(defect) = defect {
            return Err(defect);
        }
        match chain {
            Some(verified) => Ok(verified.head),
            None => {
                // Distinguish "no such run" from "run without events".
                let _ = self.manifest(run_id)?;
                Ok(Opened {
                    seq: 0,
                    hash: ZERO_HASH.to_string(),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
