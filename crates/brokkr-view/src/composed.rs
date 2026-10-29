//! The run view composed from what the writer keeps (#753; decision 0089
//! ruling 7, slice 2b). After a run's opening, its trail (every event but
//! its checkpoints), its seat rows and a page of its journal, read in one
//! snapshot at the head the opening verified, make the run view
//! [`run_view`](crate::run_view) makes of the run read whole, all but two
//! fields: each participant's checkpoints, which the seat view pages, and
//! the journal, of which it holds the page it was read with. A reader that
//! shows the run whole composes it once with every event its opening kept
//! ([`composed_whole`]), each participant's checkpoints made from them by
//! the seat view's row maker.
//!
//! The participants are the ones the seat rows keep, in their order, each
//! under the request it was first seen under, as the participant scan
//! keys them. What their records told is the facts the rows group into
//! ([`Told::kept`]), and what the trail states stands as the scan reads it
//! there: their requests, starts and terminals, and their plans'
//! provenance. A phase's inner columns read the steps and tags its newest
//! effect's rows list, a visit's models the ones its requests' rows list,
//! the live block the rows of the run's last start, and the last visit
//! ends at the head's stamp.

use std::collections::BTreeMap;

use brokkr_core::fold::RunState;
use brokkr_core::seat::row::{facts, inner, live, Datum, Member, Outcome, SeatFacts, SeatRow};
use brokkr_core::{EventEnvelope, EventType};
use serde_json::Value;

use crate::participant::key_of;
use crate::started::CarriedEntry;
use crate::{
    bucket, commands_of, dashboard, ensure, field, ids, journal_rows, live_line, page_rows,
    scan_participants, supersedes, viewed, Build, Participant, Read, RunView, Scan, Told, CARRIED,
    FAILED, INDETERMINATE, SUCCEEDED, WORKING,
};

/// What one run's view is composed from, read at the head its opening
/// verified (ruling 7).
pub struct Parts<'a> {
    /// Every event of the run but its checkpoints, in seq order.
    pub trail: &'a [EventEnvelope],
    /// The run's seat rows, in the order they were opened.
    pub seats: &'a [SeatRow],
    /// A page of the journal, checkpoints included, in seq order.
    pub page: &'a [EventEnvelope],
    /// The seq of the event each of the page's causation ids names.
    pub causes: &'a BTreeMap<String, u64>,
    /// The head's seq, which counts the run's events from 1.
    pub head_seq: u64,
    /// The head's stamp.
    pub last_at: &'a str,
}

/// The run view of the run `parts` read, `state` its fold at the head:
/// what [`run_view`](crate::run_view) shows of the run read whole, but its
/// participants' checkpoints and its journal's other pages. `None` where a
/// seat row names a request the trail does not ask or a participant's
/// facts find no rows of its own, which no writer keeps,
/// or where two participants' effects and members name one node, which
/// only a hand-built journal's effect ids can, so the reader reads the run
/// whole.
pub fn composed_view(parts: &Parts<'_>, state: Option<&RunState>) -> Option<RunView> {
    composed(parts, |id| parts.causes.get(id).copied(), state)
}

/// The run view of the run whose every event `journal` holds, in seq
/// order, as its opening verified and kept them, composed once with its
/// `trail`, its `seats` rows and its head's stamp `last_at`, read at that
/// head, `state` its fold there: what [`run_view`](crate::run_view) shows
/// of the run read whole, every journal row naming its cause by the
/// journal's own ids, and each participant's checkpoints made from the
/// journal. `None` where [`composed_view`] gives none, or where a
/// checkpoint its participant is keyed by names no attempt, as the seat
/// view's page gives none, so the reader reads the run whole.
pub fn composed_whole(
    trail: &[EventEnvelope],
    seats: &[SeatRow],
    journal: &[EventEnvelope],
    last_at: &str,
    state: Option<&RunState>,
) -> Option<RunView> {
    let by_id = ids(journal);
    let parts = Parts {
        trail,
        seats,
        page: journal,
        // Named by the journal's own ids instead.
        causes: &BTreeMap::new(),
        head_seq: u64::try_from(journal.len()).unwrap_or(u64::MAX),
        last_at,
    };
    let mut view = composed(&parts, |id| by_id.get(id).copied(), state)?;
    checkpointed(&mut view.participants, trail, journal)?;
    Some(view)
}

/// Each of `participants`' checkpoints, oldest first, made from the
/// checkpoints of `journal` its key names ([`key_of`] of each one's effect
/// and member), each row by the seat view's row maker ([`page_rows`]) of
/// the starts of `trail` that name its effect: `None` where a checkpoint
/// a participant is keyed by names no attempt or another member. A
/// checkpoint of an effect the trail does not ask for is no participant's,
/// as the participant scan reads it, nor is one whose key names none, as
/// no seat's page reads it.
fn checkpointed(
    participants: &mut [Participant],
    trail: &[EventEnvelope],
    journal: &[EventEnvelope],
) -> Option<()> {
    type Seat<'a> = (&'a str, Option<&'a str>);
    let asked = trail.iter();
    let asked = asked.filter(|event| event.event_type == EventType::EffectRequested);
    let asked: Vec<&str> = asked
        .filter_map(|event| field(&event.payload, "effect_id"))
        .collect();
    let mut seats: BTreeMap<Seat<'_>, Vec<&EventEnvelope>> = BTreeMap::new();
    let checkpoints = journal.iter();
    let checkpoints = checkpoints.filter(|event| event.event_type == EventType::EffectCheckpointed);
    for event in checkpoints {
        let effect_id = field(&event.payload, "effect_id");
        let Some(effect_id) = effect_id.filter(|effect_id| asked.contains(effect_id)) else {
            continue;
        };
        let checkpoint = event.payload.get("checkpoint");
        let member = checkpoint.and_then(|checkpoint| field(checkpoint, "member"));
        seats.entry((effect_id, member)).or_default().push(event);
    }
    // No two seats asked for make one key here: their seat rows make two
    // participants of one key, which `composed` composes no view of
    // (`a_run_composed_whole_is_its_run_view`, its effect `e:x` asked for).
    let seats = seats.into_iter();
    let mut keyed: BTreeMap<String, (&str, Vec<&EventEnvelope>)> = seats
        .map(|((effect_id, member), events)| (key_of(effect_id, member), (effect_id, events)))
        .collect();
    for participant in participants {
        let Some((effect_id, events)) = keyed.remove(&participant.key) else {
            continue;
        };
        let starts = trail.iter().filter(|event| {
            event.event_type == EventType::EffectStarted
                && field(&event.payload, "effect_id") == Some(effect_id)
        });
        let row = page_rows(participant.member.as_deref(), &mut starts.cloned());
        participant.checkpoints = events.into_iter().map(row).collect::<Option<_>>()?;
    }
    Some(())
}

/// The run view of the run `parts` read, `state` its fold at the head,
/// each journal row's cause the seq `cause` names for its causation id.
fn composed(
    parts: &Parts<'_>,
    cause: impl Fn(&str) -> Option<u64>,
    state: Option<&RunState>,
) -> Option<RunView> {
    let scan = scanned(parts.trail, parts.seats)?;
    let mut buckets = bucket(parts.trail);
    for (effect_id, _) in buckets.newest.values() {
        buckets
            .listed
            .insert(effect_id, inner(parts.seats, effect_id));
    }
    let marks = supersedes(parts.trail);
    let lines = live(parts.seats).into_iter();
    let read = Read {
        live: lines.map(|(label, turn)| live_line(label, turn)).collect(),
        journal: journal_rows(
            parts.page,
            &scan,
            &buckets,
            &marks,
            &commands_of(parts.page),
            cause,
        ),
        event_count: usize::try_from(parts.head_seq).unwrap_or(usize::MAX),
        dashboard: dashboard::kept(parts.trail, parts.seats, parts.last_at),
    };
    Some(viewed(parts.trail, &scan, &buckets, &marks, read, state))
}

/// The columns a row's participant is read by (`SeatRow::COLUMNS`): the
/// request it was opened under, its effect and its member; and the seq
/// and value of the model it stated and of the word its finish pinned.
const ASKED: usize = 0;
const EFFECT: usize = 1;
const MEMBER: usize = 3;
const MODEL: [usize; 2] = [13, 14];
const PIN: [usize; 2] = [25, 26];

/// A value a row keeps and the seq that stated it, `None` for a value the
/// statement left absent.
type Stated = Option<(i64, Option<String>)>;

/// What a participant's rows keep that its facts do not hold apart: the
/// request its first row names, the model it stated itself, where a seat's
/// facts give its members' joined, and the word its last finish pinned,
/// which a finish whose outcome is no status word given as one leaves
/// absent, as the scan reads it.
struct Own {
    home: Datum,
    model: Stated,
    pin: Stated,
}

/// The value at `columns` of `data`, as its seq stated it.
fn stated(data: &[Datum], [seq, value]: [usize; 2]) -> Stated {
    let Datum::Integer(seq) = data[seq] else {
        return None;
    };
    let value = match &data[value] {
        Datum::Text(text) => Some(text.clone()),
        _ => None,
    };
    Some((seq, value))
}

/// A participant's key: its effect and its member, as its rows' columns
/// name them.
type Key = (Datum, Datum);

/// What each participant of `rows` keeps of its own, by its key: its
/// first row's request, and its model and pin as the last of its rows to
/// state them did.
fn owned(rows: &[SeatRow]) -> Vec<(Key, Own)> {
    let mut owned: Vec<(Key, Own)> = Vec::new();
    for row in rows {
        let data = row.data();
        let key = (data[EFFECT].clone(), data[MEMBER].clone());
        let at = owned.iter().position(|(known, _)| *known == key);
        let at = at.unwrap_or_else(|| {
            let home = data[ASKED].clone();
            let own = Own {
                home,
                model: None,
                pin: None,
            };
            owned.push((key, own));
            owned.len() - 1
        });
        let own = &mut owned[at].1;
        own.model = own.model.take().max(stated(&data, MODEL));
        own.pin = own.pin.take().max(stated(&data, PIN));
    }
    owned
}

/// Each of `listed`, the facts of `rows`' participants, with what its own
/// rows keep, found by its effect and member and never by its place in
/// the list: `None` where facts name a participant no row keeps, or one
/// another's facts already took, so none is paired with another's.
fn paired(listed: Vec<SeatFacts>, rows: &[SeatRow]) -> Option<Vec<(SeatFacts, Own)>> {
    let mut owned = owned(rows);
    let mut take = |facts: SeatFacts| {
        let member = match &facts.member {
            Member::Seat => Datum::Null,
            Member::Named(member) => Datum::Text(member.clone()),
        };
        let key = (Datum::Text(facts.effect_id.clone()), member);
        let at = owned.iter().position(|(known, _)| *known == key)?;
        Some((facts, owned.swap_remove(at).1))
    };
    listed.into_iter().map(&mut take).collect()
}

/// The participants `rows` keep, in their order, each under the request
/// of the trail it was first seen under, with its plan's provenance as the
/// trail's starts state it: `None` where a row names a request the trail
/// does not ask, where a participant's key is another's, or where its
/// facts and its rows do not pair.
fn scanned(trail: &[EventEnvelope], rows: &[SeatRow]) -> Option<Scan> {
    let read = scan_participants(trail);
    let mut scan = Scan {
        effects: read.effects,
        parts: Vec::new(),
        by_key: BTreeMap::new(),
        boundaries: read.boundaries,
    };
    let asked: Vec<Datum> = trail
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .filter(|event| field(&event.payload, "effect_id").is_some())
        .map(|event| Datum::Integer(event.seq as i64))
        .collect();
    for (facts, own) in paired(facts(rows), rows)? {
        let slot = asked.iter().position(|asked| *asked == own.home)?;
        let member = match &facts.member {
            Member::Seat => None,
            Member::Named(member) => Some(member.as_str()),
        };
        // Each participant the rows keep is its own, so it adds a node; one
        // that finds another's, an effect named `e:m` beside effect `e`'s
        // member `m`, is a key the whole read collapses too.
        let fresh = scan.parts.len();
        let at = ensure(&mut scan, slot, &facts.effect_id, member);
        if at != fresh {
            return None;
        }
        let planned = read.by_key.get(&scan.parts[at].key);
        let planned = planned.and_then(|index| read.parts[*index].provenance.clone());
        kept(&mut scan.parts[at], &facts, own, planned);
    }
    Some(scan)
}

/// `part` as `facts` and its rows' `own` keep it, `provenance` its plan's.
/// Its model is its own, as the scan's is: the view joins a seat's
/// members' where it shows them.
fn kept(part: &mut Build, facts: &SeatFacts, own: Own, provenance: Option<Value>) {
    let member = part.member.as_deref().unwrap_or_default();
    let carried = facts.carried_from.as_ref();
    part.carried = carried.map(|from| CarriedEntry::of(member, &from.attempt_id));
    part.provenance = provenance;
    part.model = own.model.and_then(|(_, model)| model);
    part.effort.clone_from(&facts.effort);
    part.boundary.clone_from(&facts.boundary);
    part.transcript.clone_from(&facts.transcript);
    part.turns = facts.turns.filter(|_| !facts.turns_aggregated);
    part.items = facts.items;
    part.told = Told::kept(facts, own.pin.and_then(|(_, word)| word));
    (part.status, part.status_class) = status(facts.outcome);
}

/// The status and its class a participant's outcome shows.
fn status(outcome: Outcome) -> (&'static str, &'static str) {
    match outcome {
        Outcome::Working => WORKING,
        Outcome::Succeeded => SUCCEEDED,
        Outcome::Failed => FAILED,
        Outcome::Indeterminate => INDETERMINATE,
        Outcome::Carried => CARRIED,
    }
}

#[cfg(test)]
mod tests;
