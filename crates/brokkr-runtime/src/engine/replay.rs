//! The run as this engine last read it (#354).
//!
//! A turn used to load the whole journal, re-hash every event and fold
//! it from `run/started`, so a long run paid for its whole history on
//! every turn. The replay keeps what it read: each turn reads only what
//! landed since — the engine's own appends and any peer's, an operator's
//! command included — verified against the head it holds
//! ([`Store::load_after`]), and carries its account over that
//! ([`Account::onto`]), whose fold is the state a whole replay reaches.
//!
//! The account it reaches is handed to the store each turn
//! ([`Store::carry`]), so the turn's appends write the run's summary
//! from the fold the engine decided on (decision 0089 ruling 3). The
//! engine decides on its own fold, never on a summary row (ruling 1).

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{FoldError, RunState};
use brokkr_core::summary::Account;
use brokkr_store::Store;

use super::EngineError;

/// The events of one run and the account they come to, as of the last
/// turn. Empty until the first turn, and again after a turn that failed
/// to catch up: an empty replay reads its run whole.
///
/// By design (#354, ask 3) a replay trusts the prefix it already read and
/// verified: a turn reads and verifies only what lies past its head, so a
/// row planted at or below that head after the read is not seen until the
/// run is read whole again: [`Store::load`], and so every export, judges
/// every row of the run.
#[derive(Default)]
pub(super) struct Replay {
    pub(super) events: Vec<EventEnvelope>,
    account: Option<Account>,
}

impl Replay {
    /// Bring the replay to `run_id`'s head and hand back the state there.
    /// A replay of another run is not continued: it is read whole.
    pub(super) fn caught_up(
        &mut self,
        store: &mut Store,
        run_id: &str,
    ) -> Result<RunState, EngineError> {
        let account = match self.account.take().filter(|kept| kept.run_id() == run_id) {
            Some(account) => {
                let (seq, hash) = account.head();
                let suffix = store.load_after(run_id, seq, hash)?;
                let account = account.onto(&suffix);
                account.state().map_err(FoldError::clone)?;
                self.events.extend(suffix);
                account
            }
            None => {
                self.events = store.load(run_id)?;
                Account::whole(run_id, &self.events)
            }
        };
        let state = account.state().map_err(FoldError::clone)?.clone();
        store.carry(account.clone());
        self.account = Some(account);
        Ok(state)
    }
}

#[cfg(test)]
mod tests;
