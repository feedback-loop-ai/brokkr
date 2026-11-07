//! A metered checkpoint as the journal records it (decision 0089 ruling 7):
//! the phase the run was in, its time, the cost it reported or none, its
//! tokens, whether it is the last finishing session record its participant
//! wrote, and whether its turn repeats the usage its site's last metered
//! turn carried. Read here once, from the run's events, so the writer keeps
//! on the checkpoint's `event_index` row what `brokkr analytics` derives
//! from the run read whole (decision 0071 ruling 5).
//!
//! Two keys are read here, as main reads them: a participant is an attempt
//! and the member the checkpoint names, and only its last
//! `-session-finished` record is its session record, so a later one moves
//! the fact from the earlier; a site is a start's provenance entry, and a
//! turn whose usage is the one its site's last metered turn carried is that
//! message again: it repeats and meters nothing (`meter::once`). Both facts
//! depend only on records before the one read, so neither moves. The walk
//! reads a checkpoint only where its stamp parses and its payload names an
//! attempt, as the analytics walk it replaces reads it, but it names every
//! checkpoint row it saw, so the writer records the row processed whatever
//! it could read.

use std::collections::BTreeMap;

use crate::envelope::{EventEnvelope, EventType};
use crate::residual::text as field;
use crate::seat::meter::Meter;
use crate::seat::reported_cost;
use crate::stamp::parse_millis;

/// One metered checkpoint: the event that recorded it and the facts the
/// aggregates read of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Checkpoint {
    /// The seq of the `effect/checkpointed` event.
    pub seq: u64,
    /// The attempt the checkpoint names.
    pub attempt_id: String,
    /// The member of that attempt the checkpoint names, where it names one.
    pub member: Option<String>,
    /// The checkpoint's own time.
    pub at: i64,
    /// The phase the run was in when the checkpoint was recorded.
    pub phase: Option<String>,
    /// Whether the record reported a cost, whatever its amount.
    pub reported: bool,
    /// The cost the record reported.
    pub usd: Option<f64>,
    /// The tokens the record carries, and whether it is a turn.
    pub meter: Meter,
    /// Whether it is the last finishing session record its participant
    /// wrote, as main reads it: the attempt and member it names, whatever
    /// site its start opened.
    pub session: bool,
    /// Whether its usage is the one its site's last metered turn carried, so
    /// it is that message again and meters nothing (`meter::once`): the
    /// rule a window's tokens are read by. `None` where the checkpoint was
    /// read from its event alone, which cannot know its site's earlier
    /// turns; the writer that carries the reading writes it.
    pub repeat: Option<bool>,
}

impl Checkpoint {
    /// The checkpoint `event` records, with `phase` the run's when it was
    /// recorded, or none where its stamp does not parse or it names no
    /// attempt, as the walk the analytics readout made reads it. Its
    /// `session` says only that the record finishes a session: whether it
    /// is its participant's last such record is the walk's, which knows the
    /// ones before it. The writer reads a checkpoint it appends without a
    /// carried account this way (decision 0089 ruling 7).
    pub fn read(event: &EventEnvelope, phase: Option<&str>) -> Option<Checkpoint> {
        let at = parse_millis(&event.recorded_at)?;
        let attempt_id = field(&event.payload, "attempt_id")?.to_string();
        let record = event.payload.get("checkpoint")?;
        let usd = reported_cost(record);
        Some(Checkpoint {
            seq: event.seq,
            attempt_id,
            member: field(record, "member").map(str::to_string),
            at,
            phase: phase.map(str::to_string),
            reported: usd.is_some(),
            usd,
            meter: Meter::of(record),
            session: is_session(record),
            repeat: None,
        })
    }
}

/// Whether `record` finishes a session: its step ends with
/// "-session-finished", the mark every driver's finishing step bears.
fn is_session(record: &serde_json::Value) -> bool {
    field(record, "step").is_some_and(|step| step.ends_with("-session-finished"))
}

/// Which site a checkpoint belongs to: the site its start opened, or, for
/// a checkpoint no start's provenance names a provider for, its attempt and
/// member alone, as the reading has always keyed it.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum SiteKey {
    Site(u64),
    Loose(String, Option<String>),
}

/// How a participant is keyed: its attempt and the member it names, as
/// main keys a session record, whatever site its start opened.
type Participant = (String, Option<String>);

/// The checkpoints read so far, carried event by event: the run's phase and
/// every checkpoint's facts, each keyed by its seq. A later finishing
/// session record of a participant moves the flag off the earlier one as it
/// is read, so the facts are current at every head (ruling 7).
#[derive(Debug, Clone, Default)]
pub(crate) struct Checking {
    phase: Option<String>,
    facts: BTreeMap<u64, Checkpoint>,
    /// The seq of every `effect/checkpointed` event read, an ignored one
    /// included: its stamp not parsing reads no facts, but its row was still
    /// read and the writer records it processed.
    rows: Vec<u64>,
    /// Each site's place, by its attempt and member, so a site opened
    /// again is another site and a turn repeating its own key's last
    /// message is not read as one site's (ruling 7).
    sites: super::site::SiteMap,
    /// The last finishing session record of each participant read so far,
    /// as main reads it: a re-opened attempt's later record moves the flag
    /// off the earlier.
    sessions: BTreeMap<Participant, u64>,
    /// The usage each site's last metered turn carried: what a turn
    /// repeating it meters nothing for.
    last: BTreeMap<SiteKey, crate::seat::TokenUsage>,
}

impl Checking {
    /// The reading carried over `event`, the run's next.
    pub(crate) fn note(&mut self, event: &EventEnvelope) {
        if event.event_type == EventType::PhaseEntered {
            self.phase = super::entered_phase(event).map(str::to_string);
            return;
        }
        if event.event_type == EventType::EffectStarted {
            let at = parse_millis(&event.recorded_at);
            if let (Some(_), Some(attempt)) = (at, field(&event.payload, "attempt_id")) {
                self.sites.note(attempt, &event.payload);
            }
            return;
        }
        if event.event_type != EventType::EffectCheckpointed {
            return;
        }
        self.rows.push(event.seq);
        let Some(mut checkpoint) = Checkpoint::read(event, self.phase.as_deref()) else {
            return;
        };
        let site = match self
            .sites
            .id(&checkpoint.attempt_id, checkpoint.member.as_deref())
        {
            Some(id) => SiteKey::Site(id),
            None => SiteKey::Loose(checkpoint.attempt_id.clone(), checkpoint.member.clone()),
        };
        // A metered record carries a usage, so it is read once and
        // inserted only where it meters a new message (decision 0089
        // ruling 7): no `if let` waits on a usage `metered` already
        // implies.
        let metered = checkpoint.meter.metered();
        let usage = checkpoint.meter.usage();
        let repeat = metered && usage.is_some_and(|usage| self.last.get(&site) == Some(usage));
        checkpoint.repeat = Some(repeat);
        if let Some(usage) = usage.filter(|_| metered && !repeat) {
            self.last.insert(site.clone(), usage.clone());
        }
        if checkpoint.session {
            let participant = (checkpoint.attempt_id.clone(), checkpoint.member.clone());
            if let Some(previous) = self.sessions.insert(participant, event.seq) {
                if let Some(earlier) = self.facts.get_mut(&previous) {
                    earlier.session = false;
                }
            }
        }
        self.facts.insert(event.seq, checkpoint);
    }

    /// Keep only what a carried account needs to read the events a later
    /// append adds: the run's phase, each participant's last finishing
    /// session seq, each site's last metered turn's usage and the sites'
    /// places. The checkpoint facts and row seqs already read are dropped,
    /// so a writer that carries an account clones no earlier checkpoint's
    /// facts with every append (decision 0089 ruling 9). The events an
    /// append seals are read again by [`Account::onto`].
    pub(crate) fn carried(&mut self) {
        self.facts.clear();
        self.rows.clear();
    }

    /// Every checkpoint read so far, in seq order, its session flag its
    /// participant's last finishing record's.
    pub(crate) fn checkpoints(&self) -> Vec<Checkpoint> {
        self.facts.values().cloned().collect()
    }

    /// The checkpoint read at `seq`, where its facts could be read.
    pub(crate) fn checkpoint(&self, seq: u64) -> Option<&Checkpoint> {
        self.facts.get(&seq)
    }

    /// The seq of every `effect/checkpointed` event read so far, in order,
    /// an ignored one included.
    pub(crate) fn rows(&self) -> &[u64] {
        &self.rows
    }
}

/// Every metered checkpoint of the run `events` hold, in seq order.
pub fn checkpoints(events: &[EventEnvelope]) -> Vec<Checkpoint> {
    let mut checking = Checking::default();
    for event in events {
        checking.note(event);
    }
    checking.checkpoints()
}

#[cfg(test)]
mod tests;
