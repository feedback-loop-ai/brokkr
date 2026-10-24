//! Opening one run (decision 0089 ruling 5): its whole chain read in seq
//! order and verified as it streams, so every readout of one run refuses a
//! corrupt run as [`Store::load`] does while holding one envelope of it at
//! a time, never the run (ruling 9).

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
fn extend(chain: &mut Option<Verified>, event: EventEnvelope) -> Result<(), ChainError> {
    let (run_id, seq, hash) = match chain {
        Some(verified) => (
            verified.run_id.as_str(),
            verified.head.seq,
            verified.head.hash.as_str(),
        ),
        None => (event.run_id.as_str(), 0, ZERO_HASH),
    };
    verify_chain_after(run_id, seq, hash, std::slice::from_ref(&event))?;
    let head = Opened {
        seq: event.seq,
        hash: event.event_hash,
    };
    match chain {
        Some(verified) => verified.head = head,
        None => {
            *chain = Some(Verified {
                run_id: event.run_id,
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
        patiently("load", self.patience, || self.open_once(run_id))
    }

    fn open_once(&self, run_id: &str) -> Result<Opened, StoreError> {
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
                Ok(event) if defect.is_none() => {
                    defect = extend(&mut chain, event).err().map(StoreError::from);
                }
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
