//! A seat's rows, as the writer keeps them (decision 0089 ruling 1): one
//! per participant of an attempt, a seat, a panel member or a sequence
//! step, keyed by its attempt and member. A row holds bounded aggregates
//! of what its records report, never the records. The writer's
//! [`Account`](crate::summary::Account) carries the rows over every event
//! it seals, and [`facts`] groups a run's rows by participant, as the run
//! view's scan groups its events, into the facts the view shows of each
//! (decision 0089 rulings 3 and 4).
//!
//! A participant is an effect and a member, as the view keys it, across
//! its requests and attempts; it reads its effect by the request it was
//! first seen under. Each request keeps a row of its own naming no
//! attempt: an event naming none lands there, and a request no attempt
//! has started is that row alone. A member its effect's last start
//! carries (decision 0093) is concluded as carried, by the verdict an
//! earlier attempt settled, and names that attempt as the view does. What
//! a participant takes from its last record is taken by seq. Its cost and
//! its turns' tokens are read in journal order across its rows, however
//! its attempts' records interleave: the writer keeps each participant's
//! running cost and last metered turn beside its rows, so a row holds the
//! cost summed through its last priced record and its turns' tokens, each
//! message once. A request's own row also keeps its effect's accounting,
//! what `brokkr costs` reads ([`costs`]; decision 0089 slice 3).

use std::collections::BTreeMap;

use super::meter::{sum_usage, Meter, Metering};
use super::{TokenUsage, Transcript};
use crate::envelope::{EventEnvelope, EventType};
use crate::residual::text as field;

mod costed;
mod deferred;
mod kept;
mod listed;
mod members;
mod said;
mod sited;

pub use costed::costs;
use deferred::Deferred;
pub use kept::Datum;
pub use listed::{inner, models};
use members::with_members;
pub use said::{live, shown, Finish};
use said::{Session, Turned};

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

    /// Its tag as the provenance entry states it, `None` for the seat's own.
    fn tag(&self) -> Option<&str> {
        match self {
            Member::Seat => None,
            Member::Named(tag) => Some(tag),
        }
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
    /// The model its site's reading last named, cut to `MODEL_BYTES` where
    /// its key is stored; `None` for a row no site opened. It is the key
    /// while `model` is the whole model the view prints (decision 0089
    /// ruling 7).
    site_model: Option<String>,
    effort: Last<String>,
    /// The provider its attempt's start recorded for it: each provenance
    /// entry restates it, [`Provider::Unnamed`] where the entry names none.
    provider: Last<Provider>,
    /// The provider the start that opened its site named, kept whole beside
    /// the cut `provider_key` where it is stored: a later start moves the
    /// participant's `provider`, never its site's (decision 0089 ruling 7).
    site_provider: Option<String>,
    /// The boundary its attempt's start recorded for it.
    boundary: Last<String>,
    /// The boundary a record carried beside its model while the attempt
    /// was open (design DD19).
    stamp: Last<String>,
    /// The terminal event that concluded the attempt in flight.
    ended: Last<Outcome>,
    /// A member's last panel finish.
    finish: Last<(Pin, Finish)>,
    /// Its last `seat-turn`.
    turn: Last<Turned>,
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
    /// Its finished steps, its member's tag, its models and its early tags (`listed`).
    listed: listed::Listed,
    /// On a request's own row, its effect's accounting so far (`costed`).
    costs: Option<super::costs::EffectAccounting>,
    /// The site whose start opened the row, where one did: what makes a
    /// seat row a site and keeps two sites of one attempt apart (decision
    /// 0089 ruling 7).
    site: Option<u64>,
    /// How its site's checkpoints meter it, from brokkr-core's reading of
    /// the run's sites (decision 0089 ruling 7).
    metering: Metering,
    /// Whether it is of the attempt the run's last start opened, while no
    /// terminal naming that start's effect and attempt has followed it.
    live: bool,
}

/// What a participant has stated so far, across its rows in journal
/// order: its reported costs summed, and the tokens of its last metered
/// turn, which a turn repeating its message carries again.
#[derive(Debug, Clone, Default)]
struct Running {
    spent: Option<f64>,
    last_turn: Option<Meter>,
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
    /// Each site's row, by its attempt and member: the one later records
    /// of the key land on and the site's metering rule moves (decision
    /// 0089 ruling 7).
    sited: BTreeMap<(String, Option<String>), usize>,
    /// The id the next site's row is opened under, so one attempt's two
    /// sites are two rows.
    sites: u64,
    /// Each site's row, by the start that opened it and its member: what a
    /// session record a later site moves the flag off names.
    site_starts: BTreeMap<(u64, Member), usize>,
    /// Each participant's running state, by its effect and member.
    running: BTreeMap<(String, Member), Running>,
    /// Every start so far, by its effect and attempt: what a source's ordinal counts.
    begun: Vec<(Option<String>, Option<String>)>,
    /// Each record of an effect not yet asked, by the effect, in journal
    /// order and typed: the request that asks it replays them into its
    /// rows, as main's walk reads a start before its request.
    pending: BTreeMap<String, Vec<Deferred>>,
    /// Whether an effect's participant record — a start, a checkpoint or a
    /// terminal of its attempt — landed before the first request that
    /// asked its effect. No kept row represents that order exactly, so the
    /// head is not kept for the run (decision 0089 ruling 8).
    before_request: bool,
    aside: listed::Aside,
}

impl Seats {
    /// The rows, in the order they were opened.
    pub(crate) fn rows(&self) -> &[SeatRow] {
        &self.rows
    }

    /// The rows carried over `event`, the run's next; and the starts a
    /// request replays, whose site rows the account then reads its sites
    /// onto.
    pub(crate) fn note(&mut self, event: &EventEnvelope) -> Vec<u64> {
        let payload = &event.payload;
        let effect_id = field(payload, "effect_id");
        let replayed = match (event.event_type, effect_id) {
            (EventType::PhaseEntered, _) => {
                self.phase = field(payload, "phase").map(str::to_string);
                Vec::new()
            }
            (EventType::EffectRequested, Some(effect_id)) => self.request(event, effect_id),
            _ => match effect_id.and_then(|id| self.by_effect.get(id)) {
                Some(&request) => {
                    self.attempted(request, event);
                    Vec::new()
                }
                None => {
                    self.deferred(event);
                    Vec::new()
                }
            },
        };
        if event.event_type == EventType::EffectStarted {
            let attempt = field(payload, "attempt_id").map(str::to_string);
            self.begun.push((effect_id.map(str::to_string), attempt));
            self.sourced();
        }
        self.aside(event);
        self.lived(event);
        self.accounted(event);
        replayed
    }

    /// Every carried entry's source, named again over the fold so far: a
    /// later start can name its attempt, and a later request can ask an
    /// earlier start's effect in its phase, as the view reads its source
    /// over the whole run. A source is counted in the phase of the request
    /// its participant was first seen under, as the view counts it.
    fn sourced(&mut self) {
        let mut named = Vec::new();
        for (&(request, _), members) in &self.at {
            for (member, &at) in members {
                if let Some((seq, carry)) = &self.rows[at].carried {
                    let home = self.home(request, member);
                    named.push((at, *seq, self.source(home, carry.attempt_id())));
                }
            }
        }
        for (at, seq, carry) in named {
            self.rows[at].carried = Some((seq, carry));
        }
    }

    /// The first request of `request`'s effect holding a row of `member`:
    /// the one its participant was first seen under (`facts`).
    fn home(&self, request: usize, member: &Member) -> usize {
        let effect = &self.requests[request].effect_id;
        let seen = self.at.iter().filter(|((at, _), members)| {
            self.requests[*at].effect_id == *effect && members.contains_key(member)
        });
        seen.map(|((at, _), _)| *at).fold(request, usize::min)
    }

    fn request(&mut self, event: &EventEnvelope, effect_id: &str) -> Vec<u64> {
        self.by_effect
            .insert(effect_id.to_string(), self.requests.len());
        self.requests.push(Request {
            seq: event.seq,
            effect_id: effect_id.to_string(),
            seat: field(&event.payload, "seat").map(str::to_string),
            phase: field(&event.payload, "phase").map(str::to_string),
            open: None,
        });
        let request = self.requests.len() - 1;
        self.row(request, None, Member::Seat);
        let pending = self.pending.remove(effect_id).unwrap_or_default();
        let mut opened = Vec::new();
        for deferred in pending {
            match deferred {
                Deferred::Start(start) => {
                    self.started(request, &start);
                    opened.push(start.seq);
                }
                Deferred::Checkpoint(record) => self.checkpointed(request, *record),
                Deferred::End(end) => self.ended(request, &end),
            }
        }
        self.sourced();
        opened
    }

    fn attempted(&mut self, request: usize, event: &EventEnvelope) {
        match event.event_type {
            EventType::EffectStarted => {
                let start = sited::Start::of(event, self.phase.clone());
                self.started(request, &start);
            }
            EventType::EffectCheckpointed => {
                self.checkpointed(request, deferred::Record::of(event));
            }
            EventType::EffectSucceeded => {
                self.ended(request, &deferred::End::of(event, Outcome::Succeeded));
            }
            EventType::EffectFailed => {
                self.ended(request, &deferred::End::of(event, Outcome::Failed));
            }
            EventType::EffectIndeterminate => {
                self.ended(request, &deferred::End::of(event, Outcome::Indeterminate));
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
                let row = self.aside.lit(self.opened(&attempt, &member));
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
        let keyed = SeatRow::keyed(
            asked.seq,
            asked.effect_id.clone(),
            attempt.1.clone(),
            member.clone(),
        );
        SeatRow {
            seat: asked.seat.clone(),
            phase: seat.and_then(|seat| seat.phase.clone()),
            started: seat.and_then(|seat| seat.started.clone()),
            since: seat.and_then(|seat| seat.since),
            ..keyed
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
    /// Its last checkpoint's time.
    pub last_at: Option<String>,
    /// Its last `seat-turn`'s turn and tool as shown ([`shown`]), and the
    /// target it names: what a working participant's activity shows.
    pub turn: Option<String>,
    pub tool: Option<String>,
    pub target: Option<String>,
    /// A member's last panel finish.
    pub finish: Option<Finish>,
    /// The session its last session record names.
    pub session_id: Option<String>,
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
    let finish = latest(part, 0, |row| row.finish.as_ref());
    let outcome = match (carried, finish.map(|(pin, _)| pin)) {
        (Some(_), _) => Outcome::Carried,
        (None, Some(Pin::Pinned(outcome))) => *outcome,
        (None, Some(Pin::Unpinned) | None) => effect.outcome,
    };
    let turn = latest(part, 0, |row| row.turn.as_ref());
    let last = part.iter().filter_map(|row| row.last.as_ref());
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
        last_at: last.max_by_key(|last| last.seq).map(|last| last.at.clone()),
        turn: turn.map(|turn| turn.turn.clone()),
        tool: turn.map(|turn| turn.tool.clone()),
        target: turn.and_then(|turn| turn.target.clone()),
        finish: finish.map(|(_, finish)| finish.clone()),
        session_id: session.and_then(|session| session.id.clone()),
    }
}

/// Each participant's facts, from a run's `rows` in the order they were
/// opened (decision 0089 ruling 1): the participants in the order the run
/// view lists them, each request's seat before its members, each by its
/// effect and member, read by the request whose row it first had.
pub fn facts(rows: &[SeatRow]) -> Vec<SeatFacts> {
    let mut parts: Vec<Vec<&SeatRow>> = Vec::new();
    let mut place: BTreeMap<(&str, &Member), usize> = BTreeMap::new();
    let mut requests: BTreeMap<u64, Vec<&SeatRow>> = BTreeMap::new();
    for row in rows {
        let at = *place
            .entry((row.effect_id.as_str(), &row.member))
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

#[cfg(test)]
mod contract_lifecycle_tests;
