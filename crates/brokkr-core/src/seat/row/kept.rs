//! A seat row as the store keeps it (decision 0089 ruling 1): each field
//! in a column of its own, in SQLite's own types, so the store writes and
//! reads a row knowing only its columns, and a row read back is the row
//! written. A value the projection keeps with the seq that stated it
//! keeps that seq in the column before it; a part read back as no row
//! writes is refused, so a row edited past its writer reads as none.
//! Counts are kept as their bits, `u64` as `i64`, so no count is cut, and
//! so are costs, `f64` as `i64`: SQLite keeps a whole-valued REAL as an
//! integer, which reads a cost of -0.0 back as +0.0.

use super::{CarriedFrom, Carry, Last, Member, Outcome, Pin, Provider, SeatRow, Session, Stamp};
use crate::seat::meter::Meter;
use crate::seat::{TokenUsage, Transcript};

/// One column's value, in the SQLite types a row is kept in.
#[derive(Debug, Clone, PartialEq)]
pub enum Datum {
    Null,
    Integer(i64),
    Text(String),
}

const INTEGER: &str = "INTEGER";
const TEXT: &str = "TEXT";

impl Outcome {
    fn word(self) -> &'static str {
        match self {
            Outcome::Working => "working",
            Outcome::Succeeded => "succeeded",
            Outcome::Failed => "failed",
            Outcome::Indeterminate => "indeterminate",
            Outcome::Carried => "carried",
        }
    }

    fn of_word(word: &str) -> Option<Outcome> {
        Outcome::named(word).or((word == "carried").then_some(Outcome::Carried))
    }
}

/// A row's columns as they are written: [`SeatRow::COLUMNS`]' order.
struct Put(Vec<Datum>);

impl Put {
    fn int(&mut self, value: Option<u64>) {
        let value = value.map(|value| Datum::Integer(value as i64));
        self.0.push(value.unwrap_or(Datum::Null));
    }

    fn text(&mut self, value: Option<&str>) {
        let value = value.map(|value| Datum::Text(value.to_string()));
        self.0.push(value.unwrap_or(Datum::Null));
    }

    fn real(&mut self, value: Option<f64>) {
        self.int(value.map(f64::to_bits));
    }

    fn flag(&mut self, value: Option<bool>) {
        self.int(value.map(u64::from));
    }

    /// The seq `last` was stated at, and the value it states.
    fn seq<'a, T>(&mut self, last: &'a Last<T>) -> Option<&'a T> {
        self.int(last.as_ref().map(|(seq, _)| *seq));
        last.as_ref().map(|(_, value)| value)
    }

    fn stamp(&mut self, stamp: Option<&Stamp>) {
        self.int(stamp.map(|stamp| stamp.seq));
        self.text(stamp.map(|stamp| stamp.at.as_str()));
    }

    fn usage(&mut self, usage: Option<&TokenUsage>) {
        let counts: [fn(&TokenUsage) -> Option<u64>; 6] = [
            |usage| usage.input_tokens,
            |usage| usage.output_tokens,
            |usage| usage.cache_read_tokens,
            |usage| usage.cache_write_tokens,
            |usage| usage.reasoning_output_tokens,
            |usage| usage.total_tokens,
        ];
        for count in counts {
            self.int(usage.and_then(count));
        }
    }

    fn meter(&mut self, meter: Option<&Meter>) {
        self.flag(meter.map(|meter| meter.turn));
        self.flag(meter.map(|meter| meter.settles));
        self.usage(meter.and_then(|meter| meter.usage.as_ref()));
    }
}

/// A row's columns as they are read back, in the order they were put.
struct Take<I>(I);

/// Both parts of a value or neither, else no row.
fn both<A, B>(a: Option<A>, b: Option<B>) -> Option<Option<(A, B)>> {
    match (a, b) {
        (None, None) => Some(None),
        (Some(a), Some(b)) => Some(Some((a, b))),
        _ => None,
    }
}

impl<I: Iterator<Item = Datum>> Take<I> {
    fn int(&mut self) -> Option<Option<u64>> {
        match self.0.next()? {
            Datum::Null => Some(None),
            Datum::Integer(value) => Some(Some(value as u64)),
            _ => None,
        }
    }

    fn text(&mut self) -> Option<Option<String>> {
        match self.0.next()? {
            Datum::Null => Some(None),
            Datum::Text(value) => Some(Some(value)),
            _ => None,
        }
    }

    fn real(&mut self) -> Option<Option<f64>> {
        Some(self.int()?.map(f64::from_bits))
    }

    fn flag(&mut self) -> Option<Option<bool>> {
        match self.int()? {
            None => Some(None),
            Some(0) => Some(Some(false)),
            Some(1) => Some(Some(true)),
            Some(_) => None,
        }
    }

    /// A value and the seq that stated it, `of` reading the value.
    fn last<T>(&mut self, of: impl FnOnce(&mut Self) -> Option<Option<T>>) -> Option<Last<T>> {
        let seq = self.int()?;
        both(seq, of(self)?)
    }

    fn stamp(&mut self) -> Option<Option<Stamp>> {
        let stamp = both(self.int()?, self.text()?)?;
        Some(stamp.map(|(seq, at)| Stamp { seq, at }))
    }

    fn outcome(&mut self) -> Option<Option<Outcome>> {
        self.text()?
            .map_or(Some(None), |word| Outcome::of_word(&word).map(Some))
    }

    fn provider(&mut self) -> Option<Last<Provider>> {
        match (self.int()?, self.text()?) {
            (None, None) => Some(None),
            (Some(seq), name) => Some(Some((seq, Provider::of(name.as_deref())))),
            (None, Some(_)) => None,
        }
    }

    fn pin(&mut self) -> Option<Last<Pin>> {
        let seq = self.int()?;
        let pin = match (seq, self.outcome()?) {
            (None, None) => return Some(None),
            (Some(_), Some(outcome)) => Pin::Pinned(outcome),
            (Some(_), None) => Pin::Unpinned,
            (None, Some(_)) => return None,
        };
        Some(seq.map(|seq| (seq, pin)))
    }

    fn carried(&mut self) -> Option<Last<Carry>> {
        let seq = self.int()?;
        let carry = match (both(seq, self.text()?)?, self.int()?) {
            (None, None) => return Some(None),
            (Some((_, attempt_id)), None) => Carry::Unsourced(attempt_id),
            (Some((_, attempt_id)), Some(ordinal)) => Carry::From(CarriedFrom {
                attempt_id,
                ordinal: ordinal as usize,
            }),
            (None, Some(_)) => return None,
        };
        Some(seq.map(|seq| (seq, carry)))
    }

    fn usage(&mut self) -> Option<Option<TokenUsage>> {
        let usage = TokenUsage {
            input_tokens: self.int()?,
            output_tokens: self.int()?,
            cache_read_tokens: self.int()?,
            cache_write_tokens: self.int()?,
            reasoning_output_tokens: self.int()?,
            total_tokens: self.int()?,
        };
        Some((usage != TokenUsage::default()).then_some(usage))
    }

    fn meter(&mut self) -> Option<Option<Meter>> {
        let flags = both(self.flag()?, self.flag()?)?;
        match (flags, self.usage()?) {
            (None, None) => Some(None),
            (Some((turn, settles)), usage) => Some(Some(Meter {
                turn,
                settles,
                usage,
            })),
            (None, Some(_)) => None,
        }
    }

    fn session(&mut self) -> Option<Last<Session>> {
        let seq = self.int()?;
        let meter = self.meter()?;
        let cost = self.real()?;
        match both(seq, meter)? {
            Some((seq, meter)) => Some(Some((seq, Session { meter, cost }))),
            None => cost.is_none().then_some(None),
        }
    }

    fn transcript(&mut self) -> Option<Option<Transcript>> {
        let named = both(self.text()?, self.text()?)?;
        let transcript = both(named, self.text()?)?;
        Some(transcript.map(|((kind, locator), home)| Transcript {
            kind,
            locator,
            home,
        }))
    }
}

impl SeatRow {
    /// A row under its key, its request's seq and effect, its attempt and
    /// its member, holding nothing else yet: the one skeleton both the
    /// projection opens a row on and a kept row is read onto.
    pub(super) fn keyed(
        request_seq: u64,
        effect_id: String,
        attempt_id: Option<String>,
        member: Member,
    ) -> SeatRow {
        SeatRow {
            request_seq,
            effect_id,
            attempt_id,
            member,
            seat: None,
            phase: None,
            started: None,
            since: None,
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

    /// The columns a row is kept in, by name and SQL type, in the order
    /// [`SeatRow::data`] gives their values.
    pub const COLUMNS: [(&'static str, &'static str); 56] = [
        ("request_seq", INTEGER),
        ("effect_id", TEXT),
        ("attempt_id", TEXT),
        ("member", TEXT),
        ("seat", TEXT),
        ("phase", TEXT),
        ("started_seq", INTEGER),
        ("started_at", TEXT),
        ("since", INTEGER),
        ("first_seq", INTEGER),
        ("first_at", TEXT),
        ("last_seq", INTEGER),
        ("last_at", TEXT),
        ("model_seq", INTEGER),
        ("model", TEXT),
        ("effort_seq", INTEGER),
        ("effort", TEXT),
        ("provider_seq", INTEGER),
        ("provider", TEXT),
        ("boundary_seq", INTEGER),
        ("boundary", TEXT),
        ("stamp_seq", INTEGER),
        ("stamp", TEXT),
        ("ended_seq", INTEGER),
        ("ended", TEXT),
        ("pin_seq", INTEGER),
        ("pin", TEXT),
        ("carried_seq", INTEGER),
        ("carried_attempt_id", TEXT),
        ("carried_ordinal", INTEGER),
        ("turns", INTEGER),
        ("items", INTEGER),
        ("spent_seq", INTEGER),
        ("spent", INTEGER),
        ("metered_turn", INTEGER),
        ("metered_settles", INTEGER),
        ("metered_input_tokens", INTEGER),
        ("metered_output_tokens", INTEGER),
        ("metered_cache_read_tokens", INTEGER),
        ("metered_cache_write_tokens", INTEGER),
        ("metered_reasoning_output_tokens", INTEGER),
        ("metered_total_tokens", INTEGER),
        ("session_seq", INTEGER),
        ("session_turn", INTEGER),
        ("session_settles", INTEGER),
        ("session_input_tokens", INTEGER),
        ("session_output_tokens", INTEGER),
        ("session_cache_read_tokens", INTEGER),
        ("session_cache_write_tokens", INTEGER),
        ("session_reasoning_output_tokens", INTEGER),
        ("session_total_tokens", INTEGER),
        ("session_cost", INTEGER),
        ("transcript_seq", INTEGER),
        ("transcript_kind", TEXT),
        ("transcript_locator", TEXT),
        ("transcript_home", TEXT),
    ];

    /// This row's values, in [`SeatRow::COLUMNS`]' order.
    pub fn data(&self) -> Vec<Datum> {
        let mut put = Put(Vec::with_capacity(SeatRow::COLUMNS.len()));
        put.int(Some(self.request_seq));
        put.text(Some(self.effect_id.as_str()));
        put.text(self.attempt_id.as_deref());
        put.text(match &self.member {
            Member::Seat => None,
            Member::Named(member) => Some(member.as_str()),
        });
        put.text(self.seat.as_deref());
        put.text(self.phase.as_deref());
        put.stamp(self.started.as_ref());
        put.int(self.since);
        put.stamp(self.first.as_ref());
        put.stamp(self.last.as_ref());
        for last in [&self.model, &self.effort] {
            let value = put.seq(last);
            put.text(value.map(String::as_str));
        }
        let provider = put.seq(&self.provider);
        put.text(provider.and_then(|provider| match provider {
            Provider::Unnamed => None,
            Provider::Named(name) => Some(name.as_str()),
        }));
        for last in [&self.boundary, &self.stamp] {
            let value = put.seq(last);
            put.text(value.map(String::as_str));
        }
        let ended = put.seq(&self.ended);
        put.text(ended.map(|outcome| outcome.word()));
        let pin = put.seq(&self.pin);
        put.text(pin.and_then(|pin| match pin {
            Pin::Pinned(outcome) => Some(outcome.word()),
            Pin::Unpinned => None,
        }));
        self.put_tail(&mut put);
        put.0
    }

    fn put_tail(&self, put: &mut Put) {
        let carried = put.seq(&self.carried);
        put.text(carried.map(Carry::attempt_id));
        let source = carried.and_then(Carry::source);
        put.int(source.map(|source| source.ordinal as u64));
        put.int(self.turns);
        put.int(self.items);
        let spent = put.seq(&self.spent);
        put.real(spent.copied());
        put.meter(self.metered.as_ref());
        let session = put.seq(&self.session);
        put.meter(session.map(|session| &session.meter));
        put.real(session.and_then(|session| session.cost));
        let transcript = put.seq(&self.transcript);
        put.text(transcript.map(|transcript| transcript.kind.as_str()));
        put.text(transcript.map(|transcript| transcript.locator.as_str()));
        put.text(transcript.map(|transcript| transcript.home.as_str()));
    }

    /// The row `data` holds, read in [`SeatRow::COLUMNS`]' order, or
    /// `None` where it holds a value of another type than its column's,
    /// a part of a value without the rest, or more or fewer values.
    pub fn of_data(data: Vec<Datum>) -> Option<SeatRow> {
        let mut take = Take(data.into_iter());
        let mut row = take.opened()?;
        take.stated(&mut row)?;
        take.counted(&mut row)?;
        take.0.next().is_none().then_some(row)
    }
}

impl<I: Iterator<Item = Datum>> Take<I> {
    /// A row as it is opened: its key, its seat and phase, and its dates.
    fn opened(&mut self) -> Option<SeatRow> {
        let keyed = SeatRow::keyed(
            self.int()??,
            self.text()??,
            self.text()?,
            Member::of(self.text()?.as_deref()),
        );
        Some(SeatRow {
            seat: self.text()?,
            phase: self.text()?,
            started: self.stamp()?,
            since: self.int()?,
            first: self.stamp()?,
            last: self.stamp()?,
            ..keyed
        })
    }

    /// What its records stated last, each with its seq.
    fn stated(&mut self, row: &mut SeatRow) -> Option<()> {
        row.model = self.last(Self::text)?;
        row.effort = self.last(Self::text)?;
        row.provider = self.provider()?;
        row.boundary = self.last(Self::text)?;
        row.stamp = self.last(Self::text)?;
        row.ended = self.last(Self::outcome)?;
        row.pin = self.pin()?;
        Some(())
    }

    /// What it carries and what its records count, price and meter.
    fn counted(&mut self, row: &mut SeatRow) -> Option<()> {
        row.carried = self.carried()?;
        row.turns = self.int()?;
        row.items = self.int()?;
        row.spent = self.last(Self::real)?;
        row.metered = self.meter()?;
        row.session = self.session()?;
        row.transcript = self.last(Self::transcript)?;
        Some(())
    }
}

#[cfg(test)]
mod tests;
