//! One sequence step's composition (decision 0103 slice 4c2b): the
//! structural site its attempt and its cool-down probe both compose under,
//! so the probe asks exactly the route the step ran. Moved here from
//! `sequence.rs` to keep that file within ruling 4's 800-line ceiling.

use serde_json::Value;

use crate::engine::{argv_for, Engine, Selection, Site, SiteSpawn};

/// One single step's composition request (decision 0071 ruling 10: a long
/// parameter list becomes a parameter object): its site label, attempt,
/// class, command, the structural site the selection keys, and that
/// selection. The step's attempt and its cool-down probe both build one.
pub(in crate::engine) struct StepComposition<'a> {
    pub(in crate::engine) label: &'a str,
    pub(in crate::engine) attempt_id: &'a str,
    pub(in crate::engine) gate: bool,
    pub(in crate::engine) command: &'a [String],
    pub(in crate::engine) site: &'a Site,
    pub(in crate::engine) selection: &'a Selection,
}

impl Engine {
    /// A single step's one composition: its own site, the link the refusal
    /// named, the boundary and hands facts, its command and result path. The
    /// step's attempt and its cool-down probe both call this, so the probe
    /// asks exactly the route the step ran (decision 0103 slice 4c2b).
    pub(in crate::engine) fn compose_step(
        &mut self,
        at: StepComposition<'_>,
        mut input: Value,
    ) -> (SiteSpawn, Value) {
        let StepComposition {
            label,
            attempt_id,
            gate,
            command,
            site,
            selection,
        } = at;
        let link = selection.get(site);
        self.marks().site(label, gate, link, &mut input);
        let hands = self.hands_for(label);
        let mut spawn = self.compose_at(
            Some(label),
            attempt_id,
            gate,
            argv_for(selection, site, command).to_vec(),
            hands.as_ref(),
            link,
            input["result_path"].as_str().unwrap_or_default(),
        );
        let brokers = selection.brokers(label);
        let served = (brokers, hands.as_ref());
        self.mark_serving(label, link, served, Some(&mut spawn), &mut input);
        (spawn, input)
    }
}
