//! The run's summary as every surface summarizes it: `brokkr inspect`'s
//! `state`, the run view's `summary`, and the readouts that embed either.
//! The standing is the fleet's one derivation ([`crate::fleet::Facts`]),
//! so a run whose accepted stop has no ending reads as stopping here too,
//! and its ending, citation and want are the fleet row's ([`said`]).

use std::collections::BTreeMap;

use brokkr_core::decision::Decision as Decided;
use brokkr_core::ending::{closing, Citation, Ending, Wants};
use brokkr_core::fold::RunState;
use brokkr_core::EventEnvelope;
use serde::Serialize;

use crate::fleet::{said, stopping_detail, Facts, Standing};

/// The `summarize()` keys, verbatim and in the same order the
/// existing `brokkr inspect` prints them, and the standing the fleet
/// derives beside its journal status.
#[derive(Serialize)]
pub struct Summary {
    pub consecutive_failures: BTreeMap<String, u64>,
    pub cursor: String,
    /// Why a stopping run stands so: the accepted stop's seq and its way
    /// out, in the fleet's one words.
    pub detail: Option<String>,
    /// How the run ended, once it has (decision 0064).
    pub ending: Option<Ending>,
    /// What the ending cites.
    pub ending_cited: Option<Citation>,
    pub feature: Option<String>,
    pub last_decision: Option<Decided>,
    pub park_reason: Option<String>,
    pub phase: Option<String>,
    pub run_id: String,
    pub seq: u64,
    pub standing: Standing,
    pub status: String,
    /// The word the readouts print, derived once from the standing: a
    /// terminal run's ending, and `stopping` where the fold still words its
    /// status `running` (decision 0102 ruling 5).
    pub status_word: String,
    pub strategy: Option<String>,
    /// What the run asks of the operator: a ruling, an action, or nothing.
    pub wants: Option<Wants>,
}

impl Summary {
    /// The word a readout prints; the field carries [`Standing`]'s answer.
    pub fn status_word(&self) -> &str {
        &self.status_word
    }
}

/// The run's folded state as every surface summarizes it, `events` the
/// run's journal its ending is read from: every event, or at least its
/// rulings and operator commands and acceptances.
pub fn summary(state: &RunState, events: &[EventEnvelope]) -> Summary {
    let facts = Facts::of(state);
    let standing = facts.standing();
    let status = state.run.status().as_str().to_string();
    let said = said(
        standing,
        Some(&status),
        closing(events, state).as_ref(),
        false,
    );
    Summary {
        consecutive_failures: state.consecutive_failures.clone(),
        cursor: state.run.to_string(),
        detail: facts.stop_accepted_seq.map(stopping_detail),
        ending: said.ending,
        ending_cited: said.ending_cited,
        feature: state.feature.clone(),
        last_decision: state.last_decision.clone(),
        park_reason: state
            .run
            .park_reason()
            .map(|reason| reason.as_str().to_string()),
        phase: state.phase.clone(),
        run_id: state.run_id.clone(),
        seq: state.seq,
        standing,
        status_word: said.status_word,
        status,
        strategy: state.strategy.clone(),
        wants: said.wants,
    }
}
