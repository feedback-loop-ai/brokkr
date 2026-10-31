//! The run's summary as every surface summarizes it: `brokkr inspect`'s
//! `state`, the run view's `summary`, and the readouts that embed either.
//! The standing is the fleet's one derivation ([`crate::fleet::Facts`]),
//! so a run whose accepted stop has no ending reads as stopping here too.

use std::collections::BTreeMap;

use brokkr_core::decision::Decision as Decided;
use brokkr_core::fold::RunState;
use serde::Serialize;

use crate::fleet::{stopping_detail, Facts, Standing};

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
    pub feature: Option<String>,
    pub last_decision: Option<Decided>,
    pub park_reason: Option<String>,
    pub phase: Option<String>,
    pub run_id: String,
    pub seq: u64,
    pub standing: Standing,
    pub status: String,
    /// The word the readouts print, derived once from the standing:
    /// `stopping` where the fold still words its status `running`
    /// (decision 0102 ruling 5).
    pub status_word: String,
    pub strategy: Option<String>,
}

impl Summary {
    /// The word a readout prints; the field carries [`Standing`]'s answer.
    pub fn status_word(&self) -> &str {
        &self.status_word
    }
}

/// The run's folded state as every surface summarizes it.
pub fn summary(state: &RunState) -> Summary {
    let facts = Facts::of(state);
    let standing = facts.standing();
    let status = state.run.status().as_str().to_string();
    Summary {
        consecutive_failures: state.consecutive_failures.clone(),
        cursor: state.run.to_string(),
        detail: facts.stop_accepted_seq.map(stopping_detail),
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
        status_word: standing.status_word(Some(&status)).to_string(),
        status,
        strategy: state.strategy.clone(),
    }
}
