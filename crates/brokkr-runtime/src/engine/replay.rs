//! The run as this engine last read it (#354).
//!
//! A turn used to load the whole journal, re-hash every event and fold
//! it from `run/started`, so a long run paid for its whole history on
//! every turn. The replay keeps what it read: each turn reads only what
//! landed since — the engine's own appends and any peer's, an operator's
//! command included — verified against the head it holds
//! ([`Store::load_after`]), and folds that onto the state it holds
//! ([`fold_onto`]), which is the state a whole replay reaches.

use brokkr_core::envelope::EventEnvelope;
use brokkr_core::fold::{fold, fold_onto, RunState};
use brokkr_store::Store;

use super::EngineError;

/// The events of one run and the state they fold to, as of the last
/// turn. Empty until the first turn, and again after a turn that failed
/// to catch up: an empty replay reads its run whole.
#[derive(Default)]
pub(super) struct Replay {
    pub(super) events: Vec<EventEnvelope>,
    state: Option<RunState>,
}

impl Replay {
    /// Bring the replay to `run_id`'s head and hand back the state there.
    /// A replay of another run is not continued: it is read whole.
    pub(super) fn caught_up(
        &mut self,
        store: &Store,
        run_id: &str,
    ) -> Result<RunState, EngineError> {
        let state = match self.state.take().filter(|state| state.run_id == run_id) {
            Some(state) => {
                let suffix = store.load_after(run_id, state.seq, &state.last_hash)?;
                let state = fold_onto(state, &suffix)?;
                self.events.extend(suffix);
                state
            }
            None => {
                self.events = store.load(run_id)?;
                fold(&self.events)?
            }
        };
        self.state = Some(state.clone());
        Ok(state)
    }
}

#[cfg(test)]
mod tests;
