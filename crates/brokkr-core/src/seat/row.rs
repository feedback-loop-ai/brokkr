//! A seat's rows, as the writer keeps them (decision 0089 ruling 1): one
//! per participant of an attempt, a seat, a panel member or a sequence
//! step, keyed by its attempt and member. A row holds bounded aggregates
//! of what its records report, never the records. The writer's
//! [`Account`](crate::summary::Account) carries the rows over every event
//! it seals, and [`facts`] groups a run's rows by participant, as the run
//! view's scan groups its events, into the facts the view shows of each
//! (decision 0089 rulings 3 and 4).
//!
//! A participant is a request occurrence and a member, across attempts.
//! Each request keeps a row of its own that names no attempt: an event
//! naming none lands there, and a request no attempt has started is that
//! row alone. A member its effect's last start carries (decision 0093)
//! is concluded as carried, by the verdict an earlier attempt settled,
//! and names that attempt as the view does. What a participant takes
//! from its last record is taken by seq. Its cost and its turns' tokens are read in journal order across
//! its rows, however its attempts' records interleave: the writer keeps
//! each participant's running cost and last metered turn beside its rows,
//! so a row holds the cost summed through its last priced record and its
//! turns' tokens, each message once.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;

use super::meter::{sum_usage, Meter, Metering};
use super::{
    boundary_entries, count_item, reported_cost, stamp_of, transcript_of, TokenUsage, Transcript,
};
use crate::envelope::{EventEnvelope, EventType};
use crate::residual::text as field;

mod kept;

pub use kept::Datum;

/// Whose row it is: the seat's own, or one of its sites', by the tag its
/// records carry: a panel member, a sequence step.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Member {
    Seat,
    Named(String),
}

impl Member {
    fn of(tag: Option<&str>) -> Member {
        tag.map_or(Member::Seat, |tag| Member::Named(tag.to_string()))
    }
}

/// The provider a provenance entry states for its participant: the one it
/// names, or none, which replaces a provider an earlier entry named.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Provider {
    Unnamed,
    Named(String),
}

impl Provider {
    fn of(name: Option<&str>) -> Provider {
        name.map_or(Provider::Unnamed, |name| Provider::Named(name.to_string()))
    }
}

/// Where a participant stands: at work, or how it concluded, a carried
/// member by the verdict an earlier attempt settled (decision 0093).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Working,
    Succeeded,
    Failed,
    Indeterminate,
    Carried,
}

/// The attempt where a carried verdict settled, and its place among the
/// starts of the carrying request's phase, as the run view names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CarriedFrom {
    pub attempt_id: String,
    pub ordinal: usize,
}

/// What a start's carried entry says of its member: carried from the
/// attempt it names, or from one the view names no source for so far.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Carry {
    From(CarriedFrom),
    Unsourced(String),
}

impl Carry {
    fn source(&self) -> Option<&CarriedFrom> {
        match self {
            Carry::From(from) => Some(from),
            Carry::Unsourced(_) => None,
        }
    }

    fn attempt_id(&self) -> &str {
        match self {
            Carry::From(from) => &from.attempt_id,
            Carry::Unsourced(attempt_id) => attempt_id,
        }
    }
}

impl Outcome {
    /// The outcome a panel member's own finish names, where the view lets
    /// it stand over its effect's: `working` among them.
    fn named(word: &str) -> Option<Outcome> {
        match word {
            "working" => Some(Outcome::Working),
            "succeeded" => Some(Outcome::Succeeded),
            "failed" => Some(Outcome::Failed),
            "indeterminate" => Some(Outcome::Indeterminate),
            _ => None,
        }
    }
}

/// What a panel member's finish says of its outcome: one it pins, or a
/// word the view does not let stand, so its effect's outcome stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Pin {
    Pinned(Outcome),
    Unpinned,
}

/// A value and the seq of the event that stated it last.
type Last<T> = Option<(u64, T)>;

/// Keep `value`, stated at `seq`, as the last one stated.
fn state<T>(last: &mut Last<T>, seq: u64, value: Option<T>) {
    if let Some(value) = value {
        *last = Some((seq, value));
    }
}

/// An event's place in its run: its seq and its stamp.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Stamp {
    seq: u64,
    at: String,
}

impl Stamp {
    fn of(event: &EventEnvelope) -> Stamp {
        Stamp {
            seq: event.seq,
            at: event.recorded_at.clone(),
        }
    }
}

/// A seat's finishing or opening session record: its meter and the cost
/// it reported.
#[derive(Debug, Clone, PartialEq)]
struct Session {
    meter: Meter,
    cost: Option<f64>,
}

/// One participant of one attempt, as its records state it.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatRow {
    request_seq: u64,
    effect_id: String,
    /// `None` for the request's own row.
    attempt_id: Option<String>,
    member: Member,
    seat: Option<String>,
    /// The run's phase when the attempt started (decision 0089 ruling 7).
    phase: Option<String>,
    /// Its attempt's first start, kept when a start names it again.
    started: Option<Stamp>,
    /// The seq of its attempt's last start, which re-opens it.
    since: Option<u64>,
    /// Its first and last checkpoint.
    first: Option<Stamp>,
    last: Option<Stamp>,
    model: Last<String>,
    effort: Last<String>,
    /// The provider its attempt's start recorded for it: each provenance
    /// entry restates it, [`Provider::Unnamed`] where the entry names none.
    provider: Last<Provider>,
    /// The boundary its attempt's start recorded for it.
    boundary: Last<String>,
    /// The boundary a record carried beside its model while the attempt
    /// was open (design DD19).
    stamp: Last<String>,
    /// The terminal event that concluded the attempt in flight.
    ended: Last<Outcome>,
    /// A member's last panel finish.
    pin: Last<Pin>,
    /// The carried entry its attempt's start recorded for it.
    carried: Last<Carry>,
    turns: Option<u64>,
    items: Option<u64>,
    /// Its participant's reported costs, every attempt's, summed in
    /// journal order through this row's last priced record.
    spent: Last<f64>,
    /// Its turns' tokens summed, each message once by its participant's
    /// turns in journal order.
    metered: Option<Meter>,
    session: Last<Session>,
    transcript: Last<Transcript>,
}

/// What a participant has stated so far, across its rows in journal
/// order: its reported costs summed, and the tokens of its last metered
/// turn, which a turn repeating its message carries again.
#[derive(Debug, Clone, Default)]
struct Running {
    spent: Option<f64>,
    last_turn: Option<Meter>,
}

impl SeatRow {
    fn checkpointed(&mut self, at: Stamp, checkpoint: &Value, open: bool, running: &mut Running) {
        let seq = at.seq;
        let says = |key: &str| field(checkpoint, key).map(str::to_string);
        state(&mut self.model, seq, says("model"));
        if open {
            state(&mut self.stamp, seq, stamp_of(checkpoint));
        }
        state(&mut self.effort, seq, says("effort"));
        let transcript = checkpoint.get("transcript").and_then(transcript_of);
        state(&mut self.transcript, seq, transcript);
        self.first.get_or_insert_with(|| at.clone());
        self.last = Some(at);
        let step = field(checkpoint, "step");
        if matches!(step, Some("seat-turn" | "turn-completed")) {
            if let Some(turn) = checkpoint.get("turn").and_then(Value::as_u64) {
                self.turns = Some(self.turns.map_or(turn, |turns| turns.max(turn)));
            }
        }
        count_item(&mut self.items, step);
        let cost = reported_cost(checkpoint);
        if let Some(cost) = cost {
            let spent = running.spent.unwrap_or(0.0) + cost;
            running.spent = Some(spent);
            self.spent = Some((seq, spent));
        }
        let meter = Meter::of(checkpoint);
        if step.is_some_and(|step| step.ends_with("-session-finished") || step == "session-started")
        {
            let meter = meter.clone();
            self.session = Some((seq, Session { meter, cost }));
        } else if step == Some("panel-member-finished") && self.member != Member::Seat {
            let outcome = field(checkpoint, "outcome").and_then(Outcome::named);
            self.pin = Some((seq, outcome.map_or(Pin::Unpinned, Pin::Pinned)));
        }
        self.metered(meter, running);
    }

    /// One checkpoint's meter, each message once: a turn whose tokens are
    /// its participant's last metered turn's is that message again
    /// (`meter::once`).
    fn metered(&mut self, meter: Meter, running: &mut Running) {
        if !meter.metered() || running.last_turn.as_ref() == Some(&meter) {
            return;
        }
        let metered = self.metered.take();
        self.metered = Some(metered.map_or_else(|| meter.clone(), |metered| metered.and(&meter)));
        running.last_turn = Some(meter);
    }
}

/// A request occurrence: the effect it asked for, the seat it hired, the
/// phase it names, and its attempt in flight, the last started until a
/// terminal names it.
#[derive(Debug, Clone)]
struct Request {
    seq: u64,
    effect_id: String,
    seat: Option<String>,
    phase: Option<String>,
    open: Option<String>,
}

/// An attempt of a request: the request's place and the attempt's id.
type Attempt = (usize, Option<String>);

/// Every seat row of a run, carried event by event: what the writer's
/// account holds of its seats.
#[derive(Debug, Clone, Default)]
pub(crate) struct Seats {
    phase: Option<String>,
    requests: Vec<Request>,
    by_effect: BTreeMap<String, usize>,
    rows: Vec<SeatRow>,
    /// Each row's place, by its attempt and member.
    at: BTreeMap<Attempt, BTreeMap<Member, usize>>,
    /// Each participant's running state, by its request and member.
    running: BTreeMap<(usize, Member), Running>,
    /// Every start so far, by the effect and the attempt it names: what a
    /// carried source's ordinal counts.
    begun: Vec<(Option<String>, Option<String>)>,
}

impl Seats {
    /// The rows, in the order they were opened.
    pub(crate) fn rows(&self) -> &[SeatRow] {
        &self.rows
    }

    /// The rows carried over `event`, the run's next.
    pub(crate) fn note(&mut self, event: &EventEnvelope) {
        let payload = &event.payload;
        let effect_id = field(payload, "effect_id");
        match (event.event_type, effect_id) {
            (EventType::PhaseEntered, _) => {
                self.phase = field(payload, "phase").map(str::to_string)
            }
            (EventType::EffectRequested, Some(effect_id)) => self.request(event, effect_id),
            _ => {
                if let Some(&request) = effect_id.and_then(|id| self.by_effect.get(id)) {
                    self.attempted(request, event);
                }
            }
        }
        if event.event_type == EventType::EffectStarted {
            let attempt = field(payload, "attempt_id").map(str::to_string);
            self.begun.push((effect_id.map(str::to_string), attempt));
            self.sourced();
        }
    }

    /// Every carried entry's source, named again over the fold so far: a
    /// later start can name its attempt, and a later request can ask an
    /// earlier start's effect in its phase, as the view reads its source
    /// over the whole run.
    fn sourced(&mut self) {
        let mut named = Vec::new();
        for (&(request, _), members) in &self.at {
            for &at in members.values() {
                if let Some((seq, carry)) = &self.rows[at].carried {
                    named.push((at, *seq, self.source(request, carry.attempt_id())));
                }
            }
        }
        for (at, seq, carry) in named {
            self.rows[at].carried = Some((seq, carry));
        }
    }

    fn request(&mut self, event: &EventEnvelope, effect_id: &str) {
        self.by_effect
            .insert(effect_id.to_string(), self.requests.len());
        self.requests.push(Request {
            seq: event.seq,
            effect_id: effect_id.to_string(),
            seat: field(&event.payload, "seat").map(str::to_string),
            phase: field(&event.payload, "phase").map(str::to_string),
            open: None,
        });
        self.row(self.requests.len() - 1, None, Member::Seat);
        self.sourced();
    }

    fn attempted(&mut self, request: usize, event: &EventEnvelope) {
        let attempt = field(&event.payload, "attempt_id");
        match event.event_type {
            EventType::EffectStarted => self.started(request, attempt, event),
            EventType::EffectCheckpointed => self.checkpointed(request, attempt, event),
            EventType::EffectSucceeded => self.ended(request, attempt, event, Outcome::Succeeded),
            EventType::EffectFailed => self.ended(request, attempt, event, Outcome::Failed),
            EventType::EffectIndeterminate => {
                self.ended(request, attempt, event, Outcome::Indeterminate);
            }
            EventType::RunStarted
            | EventType::PhaseEntered
            | EventType::EffectRequested
            | EventType::TransitionDecided
            | EventType::OperatorCommanded
            | EventType::OperatorAccepted
            | EventType::OperatorRejected
            | EventType::RunParked
            | EventType::RunCompleted
            | EventType::RunStopped => {}
        }
    }

    /// The row of `member` in `request`'s `attempt`, opened if there is
    /// none: a member's row of an attempt takes its start from the seat's.
    fn row(&mut self, request: usize, attempt: Option<&str>, member: Member) -> &mut SeatRow {
        let at = self.place(request, attempt, member);
        &mut self.rows[at]
    }

    /// Where that row is among the rows.
    fn place(&mut self, request: usize, attempt: Option<&str>, member: Member) -> usize {
        let attempt = (request, attempt.map(str::to_string));
        let members = self.at.get(&attempt);
        match members.and_then(|members| members.get(&member)) {
            Some(at) => *at,
            None => {
                let row = self.opened(&attempt, &member);
                self.rows.push(row);
                let members = self.at.entry(attempt).or_default();
                members.insert(member, self.rows.len() - 1);
                self.rows.len() - 1
            }
        }
    }

    fn opened(&self, attempt: &Attempt, member: &Member) -> SeatRow {
        let asked = &self.requests[attempt.0];
        let seat = self
            .at
            .get(attempt)
            .and_then(|members| members.get(&Member::Seat));
        let seat = seat.map(|at| &self.rows[*at]);
        SeatRow {
            request_seq: asked.seq,
            effect_id: asked.effect_id.clone(),
            attempt_id: attempt.1.clone(),
            member: member.clone(),
            seat: asked.seat.clone(),
            phase: seat.and_then(|seat| seat.phase.clone()),
            started: seat.and_then(|seat| seat.started.clone()),
            since: seat.and_then(|seat| seat.since),
            first: None,
            last: None,
            model: None,
            effort: None,
            provider: None,
            boundary: None,
            stamp: None,
            ended: None,
            pin: None,
            carried: None,
            turns: None,
            items: None,
            spent: None,
            metered: None,
            session: None,
            transcript: None,
        }
    }

    /// An attempt starts: it is the one in flight, and every row of it
    /// takes its first start, this start's seq, the run's phase then, and
    /// the provider and the boundary the start records for each site. A
    /// start naming an attempt already started re-opens it, as the view
    /// re-opens its effect.
    fn started(&mut self, request: usize, attempt: Option<&str>, event: &EventEnvelope) {
        self.requests[request].open = attempt.map(str::to_string);
        self.row(request, attempt, Member::Seat);
        let seq = event.seq;
        for at in self.at[&(request, attempt.map(str::to_string))].values() {
            let row = &mut self.rows[*at];
            row.started.get_or_insert_with(|| Stamp::of(event));
            row.since = Some(seq);
            row.phase.clone_from(&self.phase);
        }
        let provenance = event.payload.get("provenance").and_then(Value::as_array);
        for entry in provenance.into_iter().flatten() {
            let row = self.row(request, attempt, Member::of(field(entry, "member")));
            row.provider = Some((seq, Provider::of(field(entry, "provider"))));
        }
        for entry in boundary_entries(&event.payload) {
            let row = self.row(request, attempt, Member::of(entry.member.as_deref()));
            row.boundary = Some((seq, entry.word));
        }
        self.carried(request, attempt, event);
    }

    /// The members a start carries (decision 0093, panel-carry.v1): each
    /// entry naming a member and an attempt opens its member's row and
    /// records that attempt, unsourced until [`Seats::sourced`] names its
    /// source; an entry naming either not is passed over, as the view
    /// passes it over.
    fn carried(&mut self, request: usize, attempt: Option<&str>, event: &EventEnvelope) {
        let carried = event.payload.get("carried").and_then(Value::as_array);
        for entry in carried.into_iter().flatten() {
            let (Some(member), Some(source)) = (field(entry, "member"), field(entry, "attempt_id"))
            else {
                continue;
            };
            let row = self.row(request, attempt, Member::of(Some(member)));
            row.carried = Some((event.seq, Carry::Unsourced(source.to_string())));
        }
    }

    /// The source the view names for a verdict `request` carries from
    /// `attempt_id`: the first start so far naming that attempt, counted
    /// among the starts through it whose effect was first asked in
    /// `request`'s phase; none where a start before it names no effect, or
    /// no start names that attempt.
    fn source(&self, request: usize, attempt_id: &str) -> Carry {
        let phase = &self.requests[request].phase;
        let mut ordinal = 0;
        for (effect, attempt) in &self.begun {
            let Some(effect) = effect else {
                return Carry::Unsourced(attempt_id.to_string());
            };
            let mut asked = self.requests.iter();
            let asked = asked.find(|asked| asked.effect_id == *effect);
            ordinal += usize::from(asked.is_some_and(|asked| asked.phase == *phase));
            if attempt.as_deref() == Some(attempt_id) {
                let attempt_id = attempt_id.to_string();
                return Carry::From(CarriedFrom {
                    attempt_id,
                    ordinal,
                });
            }
        }
        Carry::Unsourced(attempt_id.to_string())
    }

    fn checkpointed(&mut self, request: usize, attempt: Option<&str>, event: &EventEnvelope) {
        let checkpoint = event.payload.get("checkpoint").unwrap_or(&Value::Null);
        let open = self.requests[request].open.as_deref() == attempt;
        let member = Member::of(field(checkpoint, "member"));
        let at = self.place(request, attempt, member.clone());
        let running = self.running.entry((request, member)).or_default();
        self.rows[at].checkpointed(Stamp::of(event), checkpoint, open, running);
    }

    /// A terminal event, read only for the attempt in flight: a stale
    /// attempt's is passed over whole, as the view passes it over.
    fn ended(
        &mut self,
        request: usize,
        attempt: Option<&str>,
        event: &EventEnvelope,
        outcome: Outcome,
    ) {
        if self.requests[request].open.as_deref() != attempt {
            return;
        }
        self.requests[request].open = None;
        let seq = event.seq;
        let result = event.payload.get("result");
        let says = |key: &str| {
            result
                .and_then(|result| field(result, key))
                .map(str::to_string)
        };
        let row = self.row(request, attempt, Member::Seat);
        row.ended = Some((seq, outcome));
        state(&mut row.model, seq, says("model"));
        state(&mut row.stamp, seq, result.and_then(stamp_of));
        state(&mut row.effort, seq, says("effort"));
    }
}

// ------------------------------------------------------------------ facts

/// What the run view shows of one participant: its own facts, and a panel
/// or sequence seat's members' summed or joined where it has none of its
/// own, flagged as such.
#[derive(Debug, Clone, PartialEq)]
pub struct SeatFacts {
    pub effect_id: String,
    pub member: Member,
    pub model: Option<String>,
    pub effort: Option<String>,
    pub boundary: Option<String>,
    /// The provider its last provenance entry stated, `None` where no
    /// entry names the participant.
    pub provider: Option<Provider>,
    pub usage: Option<TokenUsage>,
    pub usage_aggregated: bool,
    /// Every attempt's reported cost, summed in journal order.
    pub cost: Option<f64>,
    pub cost_aggregated: bool,
    /// The last session record's cost alone.
    pub last_attempt_cost: Option<f64>,
    pub turns: Option<u64>,
    pub turns_aggregated: bool,
    pub items: Option<u64>,
    pub transcript: Option<Transcript>,
    pub outcome: Outcome,
    /// Where a member its effect's last start carries settled its verdict,
    /// `None` for a source the view names none for.
    pub carried_from: Option<CarriedFrom>,
    /// A seat's first attempt's start; a member's first checkpoint.
    pub started_at: Option<String>,
}

/// What a request's rows say of its effect: its first start, its last
/// start's seq, whether that start opened or re-opened an attempt it
/// named, and the last terminal that concluded an attempt in flight.
struct Effect<'a> {
    first: Option<&'a Stamp>,
    since: u64,
    /// At work while the attempt its last start opened is in flight;
    /// otherwise as its last terminal concluded it.
    outcome: Outcome,
}

impl<'a> Effect<'a> {
    fn of(rows: &[&'a SeatRow]) -> Effect<'a> {
        // Every start opens its attempt's seat row, the row its terminal
        // concludes; a terminal before its attempt's last start concluded
        // it before that start re-opened it. A terminal and a start are two
        // events, so their seqs are never equal: the order is read as an
        // `Ordering`, with no `<` whose `<=` twin would mean the same.
        let seats = || rows.iter().filter(|row| row.member == Member::Seat);
        let last = seats()
            .filter_map(|row| Some((row.since?, row)))
            .max_by_key(|(since, _)| *since);
        let in_flight = last.is_some_and(|(since, row)| {
            row.attempt_id.is_some() && row.ended.is_none_or(|(seq, _)| seq.cmp(&since).is_lt())
        });
        let ended = rows.iter().filter_map(|row| row.ended);
        Effect {
            first: seats()
                .filter_map(|row| row.started.as_ref())
                .min_by_key(|start| start.seq),
            since: last.map_or(0, |(since, _)| since),
            outcome: match ended.max_by_key(|(seq, _)| *seq) {
                Some((_, outcome)) if !in_flight => outcome,
                _ => Outcome::Working,
            },
        }
    }
}

/// The value `pick` reads from `part`'s rows that was stated last, at or
/// after `since`.
fn latest<'a, T>(
    part: &[&'a SeatRow],
    since: u64,
    pick: impl Fn(&'a SeatRow) -> Option<&'a (u64, T)>,
) -> Option<&'a T> {
    let stated = part.iter().filter_map(|row| pick(row));
    let since = stated.filter(|(seq, _)| *seq >= since);
    since.max_by_key(|(seq, _)| *seq).map(|(_, value)| value)
}

/// `values` summed in their order, or `None` for none.
fn summed(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |total, value| Some(total.unwrap_or(0.0) + value))
}

/// The tokens `part`'s turns and session record metered, by the rule
/// [`Metering`] states, each message once across its rows.
fn usage(part: &[&SeatRow], session: Option<&Session>) -> Option<TokenUsage> {
    let session = session.map(|session| &session.meter);
    let turns = || part.iter().filter_map(|row| row.metered.as_ref());
    let by = Metering::of(turns(), session);
    let counted = turns().filter_map(|meter| meter.counted(by, false));
    sum_usage(counted.chain(session.and_then(|session| session.counted(by, true))))
}

/// A participant's own facts, from its rows and its request's effect.
fn own(part: &[&SeatRow], effect: &Effect) -> SeatFacts {
    let row = part[0];
    let session = latest(part, 0, |row| row.session.as_ref());
    let carried = latest(part, effect.since, |row| row.carried.as_ref());
    let outcome = match (carried, latest(part, 0, |row| row.pin.as_ref())) {
        (Some(_), _) => Outcome::Carried,
        (None, Some(Pin::Pinned(outcome))) => *outcome,
        (None, Some(Pin::Unpinned) | None) => effect.outcome,
    };
    let boundary = latest(part, effect.since, |row| row.boundary.as_ref());
    let stamp = || latest(part, effect.since, |row| row.stamp.as_ref());
    let first_checkpoint = part.iter().filter_map(|row| row.first.as_ref());
    let started = match row.member {
        Member::Seat => effect.first,
        Member::Named(_) => first_checkpoint.min_by_key(|first| first.seq),
    };
    SeatFacts {
        effect_id: row.effect_id.clone(),
        member: row.member.clone(),
        model: latest(part, 0, |row| row.model.as_ref()).cloned(),
        effort: latest(part, 0, |row| row.effort.as_ref()).cloned(),
        boundary: boundary.or_else(stamp).cloned(),
        provider: latest(part, 0, |row| row.provider.as_ref()).cloned(),
        usage: usage(part, session),
        usage_aggregated: false,
        cost: latest(part, 0, |row| row.spent.as_ref()).copied(),
        cost_aggregated: false,
        last_attempt_cost: session.and_then(|session| session.cost),
        turns: part.iter().filter_map(|row| row.turns).max(),
        turns_aggregated: false,
        items: part
            .iter()
            .filter_map(|row| row.items)
            .reduce(|all, items| all + items),
        transcript: latest(part, 0, |row| row.transcript.as_ref()).cloned(),
        outcome,
        carried_from: carried.and_then(Carry::source).cloned(),
        started_at: started.map(|start| start.at.clone()),
    }
}

/// The words `reported`, sorted and each once, joined: how a seat with
/// none of its own reports what its members reported.
fn joined<'a>(reported: impl Iterator<Item = &'a str>) -> Option<String> {
    let words: BTreeSet<&str> = reported.collect();
    let words: Vec<&str> = words.into_iter().collect();
    (!words.is_empty()).then(|| words.join(", "))
}

/// `own`, or the sum of `members` where it has none, and whether it is.
fn own_or_summed(
    own: Option<f64>,
    members: impl Iterator<Item = Option<f64>>,
) -> (Option<f64>, bool) {
    match own {
        Some(_) => (own, false),
        None => {
            let summed = summed(members.flatten());
            (summed, summed.is_some())
        }
    }
}

/// A seat's facts with its `members'` where it has none of its own. A
/// member has no members, so its own facts stand.
fn with_members(own: &SeatFacts, members: &[&SeatFacts]) -> SeatFacts {
    let mut facts = own.clone();
    let each = |pick: fn(&SeatFacts) -> Option<f64>| members.iter().map(move |member| pick(member));
    (facts.cost, facts.cost_aggregated) = own_or_summed(own.cost, each(|member| member.cost));
    let last = each(|member| member.last_attempt_cost);
    facts.last_attempt_cost = own_or_summed(own.last_attempt_cost, last).0;
    if own.usage.is_none() {
        facts.usage = sum_usage(members.iter().filter_map(|member| member.usage.clone()));
        facts.usage_aggregated = facts.usage.is_some();
    }
    if own.turns.is_none() {
        facts.turns = members
            .iter()
            .filter_map(|member| member.turns)
            .reduce(|all, turns| all + turns);
        facts.turns_aggregated = facts.turns.is_some();
    }
    let words = |pick: fn(&SeatFacts) -> Option<&str>| {
        joined(members.iter().filter_map(|member| pick(member)))
    };
    facts.model = facts
        .model
        .or_else(|| words(|member| member.model.as_deref()));
    facts.effort = facts
        .effort
        .or_else(|| words(|member| member.effort.as_deref()));
    facts.boundary = facts
        .boundary
        .or_else(|| words(|member| member.boundary.as_deref()));
    facts
}

/// Each participant's facts, from a run's `rows` in the order they were
/// opened (decision 0089 ruling 1): the participants in the order the run
/// view lists them, each request's seat before its members.
pub fn facts(rows: &[SeatRow]) -> Vec<SeatFacts> {
    let mut parts: Vec<Vec<&SeatRow>> = Vec::new();
    let mut place: BTreeMap<(u64, &Member), usize> = BTreeMap::new();
    let mut requests: BTreeMap<u64, Vec<&SeatRow>> = BTreeMap::new();
    for row in rows {
        let at = *place
            .entry((row.request_seq, &row.member))
            .or_insert_with(|| {
                parts.push(Vec::new());
                parts.len() - 1
            });
        parts[at].push(row);
        requests.entry(row.request_seq).or_default().push(row);
    }
    let owned: Vec<(u64, SeatFacts)> = parts
        .iter()
        .map(|part| {
            let request = part[0].request_seq;
            (request, own(part, &Effect::of(&requests[&request])))
        })
        .collect();
    owned
        .iter()
        .map(|(request, facts)| {
            let members: Vec<&SeatFacts> = match facts.member {
                Member::Seat => owned
                    .iter()
                    .filter(|(of, member)| of == request && member.member != Member::Seat)
                    .map(|(_, member)| member)
                    .collect(),
                Member::Named(_) => Vec::new(),
            };
            with_members(facts, &members)
        })
        .collect()
}

#[cfg(test)]
mod tests;
