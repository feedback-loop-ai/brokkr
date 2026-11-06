//! A run's sites as the journal records them (decision 0089 ruling 7): one
//! per provenance entry that names a provider, keyed by its attempt and
//! member, with the run's phase and the entry's time when the attempt
//! started, the last model any of its records names, how its tokens are
//! metered, and its metered checkpoints. Read here once, from the run's
//! events, so the seat rows and `brokkr analytics` derive the facts one
//! way (decision 0071 rulings 5 and 7).
//!
//! A checkpoint's meter is the site's turns each message once: a turn
//! whose usage repeats the site's last metered turn's is that message
//! again and meters nothing (`meter::once`). The metering rule is decided
//! over all the site's checkpoints, so a window holding only some of them
//! never reads both a turn and the session record that settles it.

use std::collections::BTreeMap;

use serde_json::Value;

use super::checkpoint::Checkpoint;
use super::entered_phase;
use crate::envelope::{EventEnvelope, EventType};
use crate::residual::text as field;
use crate::seat::meter::{self, Meter, Metering};
use crate::stamp::parse_millis;

/// One invocation site of one attempt that an agent resolution chose.
#[derive(Debug, Clone, PartialEq)]
pub struct Site {
    /// The attempt the site belongs to.
    pub attempt_id: String,
    /// The member of that attempt, where its provenance entry names one.
    pub member: Option<String>,
    /// The attempt's start, in milliseconds.
    pub at: i64,
    /// The phase the run was in when the attempt started.
    pub phase: Option<String>,
    /// The provider the provenance entry names.
    pub provider: String,
    /// The last model any of the site's records names.
    pub model: Option<String>,
    /// How the site's checkpoints meter it.
    pub metering: Metering,
    /// Its checkpoints in journal order, each message once. The whole-run
    /// projection fills it; the writer's carried account keeps the site's
    /// facts alone, so an append carries no earlier checkpoint (decision
    /// 0089 ruling 9).
    pub metered: Vec<Checkpoint>,
    /// Whether any checkpoint is a metered turn: one of
    /// [`Metering::of`]'s two inputs, carried so a later checkpoint costs
    /// the site one flag and not its history.
    turns: bool,
    /// The site's last finishing session record: [`Metering::of`]'s other
    /// input, carried as its meter.
    session: Option<Meter>,
}

/// The sites a run's starts have opened so far: each provenance entry
/// naming a provider one site, keyed by its attempt and member to the site
/// opened last for them, the one a later checkpoint belongs to. A start
/// naming an attempt and member already opened opens another site and
/// repoints the key, as the analytics walk reads it.
#[derive(Debug, Clone, Default)]
pub(crate) struct SiteMap {
    next: u64,
    at: BTreeMap<(String, Option<String>), u64>,
}

/// One provenance entry that opens a site: its member and provider.
pub(crate) struct SiteEntry<'a> {
    pub member: Option<&'a str>,
    pub provider: &'a str,
}

impl SiteMap {
    /// The sites `attempt`'s provenance opens, each naming a provider, and
    /// the key it now names.
    pub(crate) fn note<'a>(&mut self, attempt: &str, payload: &'a Value) -> Vec<SiteEntry<'a>> {
        let mut opened = Vec::new();
        let entries = payload.get("provenance").and_then(Value::as_array);
        for entry in entries.into_iter().flatten() {
            let Some(provider) = field(entry, "provider") else {
                continue;
            };
            let member = field(entry, "member");
            let id = self.next;
            self.next += 1;
            self.at
                .insert((attempt.to_string(), member.map(str::to_string)), id);
            opened.push(SiteEntry { member, provider });
        }
        opened
    }

    /// The site a record of `attempt` naming `member` belongs to: the one
    /// its start opened last, or none where no start named it a provider.
    pub(crate) fn id(&self, attempt: &str, member: Option<&str>) -> Option<u64> {
        self.at
            .get(&(attempt.to_string(), member.map(str::to_string)))
            .copied()
    }
}

/// The sites read so far, carried event by event.
#[derive(Debug, Clone, Default)]
pub(crate) struct Siting {
    phase: Option<String>,
    sites: Vec<Site>,
    /// Each site's place, by its attempt and member.
    opened: SiteMap,
    /// Whether the reading keeps each site's checkpoints: only the
    /// whole-run projection does, so the writer's carried account holds no
    /// earlier checkpoint (decision 0089 ruling 9).
    keeping: bool,
}

impl Siting {
    /// A reading that keeps each site's checkpoints, for the whole-run
    /// projection `brokkr analytics` reads.
    fn keeping() -> Siting {
        Siting {
            keeping: true,
            ..Siting::default()
        }
    }

    /// The sites read so far.
    pub(crate) fn sites(&self) -> &[Site] {
        &self.sites
    }

    /// The site a record of `attempt` naming `member` belongs to now, the
    /// one its start opened last.
    pub(crate) fn latest(&self, attempt: &str, member: Option<&str>) -> Option<&Site> {
        let at = self.opened.id(attempt, member)?;
        self.sites.get(at as usize)
    }

    /// The reading carried over `event`, the run's next: its checkpoints
    /// as brokkr-core read them, where it wrote one.
    pub(crate) fn note(&mut self, event: &EventEnvelope, checkpoint: Option<&Checkpoint>) {
        if event.event_type == EventType::PhaseEntered {
            self.phase = entered_phase(event).map(str::to_string);
            return;
        }
        let (Some(at), Some(attempt)) = (
            parse_millis(&event.recorded_at),
            field(&event.payload, "attempt_id"),
        ) else {
            return;
        };
        match event.event_type {
            EventType::EffectStarted => self.started(at, attempt, &event.payload),
            EventType::EffectCheckpointed => {
                self.recorded(attempt, event.payload.get("checkpoint"), checkpoint)
            }
            EventType::EffectSucceeded => self.recorded(attempt, event.payload.get("result"), None),
            _ => {}
        }
    }

    /// One attempt's provenance entries, a site each, as the analytics
    /// walk reads them: an entry naming no provider opens no site, and an
    /// entry naming a key already opened leaves the earlier site where it
    /// was and opens a new one for the later entry.
    fn started(&mut self, at: i64, attempt: &str, payload: &Value) {
        for entry in self.opened.note(attempt, payload) {
            self.sites.push(Site {
                attempt_id: attempt.to_string(),
                member: entry.member.map(str::to_string),
                at,
                phase: self.phase.clone(),
                provider: entry.provider.to_string(),
                model: None,
                metering: Metering::Session,
                metered: Vec::new(),
                turns: false,
                session: None,
            });
        }
    }

    /// One record of an attempt: the model it names, and, where the record
    /// is a checkpoint, its facts. A record naming a site no start opened is
    /// passed over, as the analytics walk passes it over.
    fn recorded(&mut self, attempt: &str, record: Option<&Value>, checkpoint: Option<&Checkpoint>) {
        let Some(record) = record else { return };
        let member = field(record, "member");
        let Some(at) = self.opened.id(attempt, member) else {
            return;
        };
        let site = &mut self.sites[at as usize];
        if let Some(model) = field(record, "model") {
            site.model = Some(model.to_string());
        }
        let Some(checkpoint) = checkpoint else {
            return;
        };
        if checkpoint.meter.metered() {
            site.turns = true;
        }
        if checkpoint.session {
            site.session = Some(checkpoint.meter.clone());
        }
        site.metering = Metering::decide(
            site.turns,
            site.session.as_ref().is_some_and(|meter| meter.settles),
        );
        if self.keeping {
            site.metered.push(checkpoint.clone());
        }
    }

    /// The sites read so far, each its checkpoints' meters read once and
    /// its metering rule decided over them: the whole-run projection.
    fn settled(mut self) -> Vec<Site> {
        for site in &mut self.sites {
            let meters = site
                .metered
                .iter_mut()
                .map(|checkpoint| &mut checkpoint.meter);
            meter::once(meters);
            site.metering = Metering::of(
                site.metered.iter().map(|checkpoint| &checkpoint.meter),
                site.session.as_ref(),
            );
        }
        self.sites
    }
}

/// Every site of the run `events` hold, in the order they were opened,
/// each its checkpoints read once.
pub fn sites(events: &[EventEnvelope]) -> Vec<Site> {
    let checkpoints: BTreeMap<u64, Checkpoint> = super::checkpoints(events)
        .into_iter()
        .map(|checkpoint| (checkpoint.seq, checkpoint))
        .collect();
    let mut siting = Siting::keeping();
    for event in events {
        siting.note(event, checkpoints.get(&event.seq));
    }
    siting.settled()
}

#[cfg(test)]
mod tests;
