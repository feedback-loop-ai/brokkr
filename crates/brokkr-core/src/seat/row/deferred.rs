//! What a run's records state of an effect before its request (decision
//! 0089 ruling 4): a start, a checkpoint or a terminal the journal holds
//! before any request asks the effect. The records are read here, once, into
//! the typed facts the seat rows take, never kept as their payload
//! (decision 0071 ruling 3): the request that asks the effect replays them
//! in journal order, so an early start opens its site and a checkpoint
//! between it and its request still dates it, as main's walk reads them.
//!
//! A start that opens a site is held, and a providerless restart of an
//! attempt whose start is held, so the restart moves its participant's facts
//! too (`Seats::started`); a lone providerless start stays the live block's,
//! as the view passes over a start before its request. A checkpoint or a
//! terminal is held only after the start it belongs to, so a record before
//! the attempt's first start stays the live block's early line
//! (`listed::Aside`), as it did. The queue holds no row: a request replays
//! it into the rows, and the started account carries the queue as its typed
//! facts across an append (decision 0089 ruling 9).

use serde_json::Value;

use super::said::{self, Turned};
use super::sited::Start;
use super::{state, Member, Outcome, Pin, Running, SeatRow, Seats, Session, Stamp};
use crate::envelope::{EventEnvelope, EventType};
use crate::residual::text as field;
use crate::seat::meter::Meter;
use crate::seat::{count_item, reported_cost, stamp_of, transcript_of, Transcript};

/// An effect's record before its first request, typed as the rows read it:
/// a start with the site it states, a checkpoint's dated facts, or the
/// terminal that concluded its attempt.
#[derive(Debug, Clone)]
pub(super) enum Deferred {
    Start(Start),
    Checkpoint(Box<Record>),
    End(End),
}

/// One checkpoint's dated facts, as a seat row reads them: its time, the
/// model, stamp, effort and transcript it states, its step and turn, its
/// cost, its meter, its session and finish, and the site model the row
/// keeps. Read from the event once, so a replayed checkpoint costs no
/// payload.
#[derive(Debug, Clone)]
pub(super) struct Record {
    pub(super) seq: u64,
    pub(super) at: String,
    pub(super) attempt: Option<String>,
    pub(super) member: Member,
    model: Option<String>,
    stamp: Option<String>,
    effort: Option<String>,
    transcript: Option<Transcript>,
    step: Option<String>,
    step_name: Option<String>,
    turn: Option<u64>,
    cost: Option<f64>,
    meter: Meter,
    session_id: Option<String>,
    session: bool,
    finish_step: bool,
    finish: Option<(Pin, said::Finish)>,
    turned: Option<Turned>,
}

impl Record {
    /// `event`'s checkpoint as a row reads it, typed: its payload is read
    /// here and dropped.
    pub(super) fn of(event: &EventEnvelope) -> Record {
        let checkpoint = event.payload.get("checkpoint").unwrap_or(&Value::Null);
        let step = field(checkpoint, "step").map(str::to_string);
        let session = step
            .as_deref()
            .is_some_and(|step| step.ends_with("-session-finished") || step == "session-started");
        let finish_step = step.as_deref() == Some("panel-member-finished");
        let seat_turn = step.as_deref() == Some("seat-turn");
        Record {
            seq: event.seq,
            at: event.recorded_at.clone(),
            attempt: field(&event.payload, "attempt_id").map(str::to_string),
            member: Member::of(field(checkpoint, "member")),
            model: field(checkpoint, "model").map(str::to_string),
            stamp: stamp_of(checkpoint),
            effort: field(checkpoint, "effort").map(str::to_string),
            transcript: checkpoint.get("transcript").and_then(transcript_of),
            step,
            step_name: field(checkpoint, "step_name").map(str::to_string),
            turn: checkpoint.get("turn").and_then(Value::as_u64),
            cost: reported_cost(checkpoint),
            meter: Meter::of(checkpoint),
            session_id: field(checkpoint, "session_id").map(str::to_string),
            session,
            finish_step,
            finish: finish_step.then(|| said::finished(checkpoint, &event.recorded_at)),
            turned: seat_turn.then(|| Turned::of(checkpoint)),
        }
    }
}

/// One terminal's dated facts, as the row of the attempt in flight reads
/// them: its outcome, and the model, stamp and effort its result states.
#[derive(Debug, Clone)]
pub(super) struct End {
    seq: u64,
    attempt: Option<String>,
    outcome: Outcome,
    model: Option<String>,
    stamp: Option<String>,
    effort: Option<String>,
}

impl End {
    pub(super) fn of(event: &EventEnvelope, outcome: Outcome) -> End {
        let result = event.payload.get("result");
        let says = |key: &str| {
            result
                .and_then(|result| field(result, key))
                .map(str::to_string)
        };
        End {
            seq: event.seq,
            attempt: field(&event.payload, "attempt_id").map(str::to_string),
            outcome,
            model: says("model"),
            stamp: result.and_then(stamp_of),
            effort: says("effort"),
        }
    }
}

impl Seats {
    /// `event`, an effect's record before its first request, held as its
    /// typed facts: a start that opens a site, or a providerless restart of
    /// an attempt whose start is held, and a checkpoint or terminal after
    /// the start it belongs to. A lone providerless start stays the live
    /// block's: the view's scan passes over a start before its request, so
    /// holding it would read a clock and a model the view does not. Any
    /// other record is not held, and the request that asks its effect opens
    /// no row of it.
    pub(super) fn deferred(&mut self, event: &EventEnvelope) {
        let Some(effect_id) = field(&event.payload, "effect_id") else {
            return;
        };
        let attempt = field(&event.payload, "attempt_id");
        let awaiting = self.awaiting(effect_id, attempt);
        let phase = self.phase.clone();
        let held = match event.event_type {
            EventType::EffectStarted => {
                let start = Start::of(event, phase);
                (start.provider_bearing() || awaiting).then_some(Deferred::Start(start))
            }
            EventType::EffectCheckpointed => {
                awaiting.then(|| Deferred::Checkpoint(Box::new(Record::of(event))))
            }
            EventType::EffectSucceeded => {
                awaiting.then(|| Deferred::End(End::of(event, Outcome::Succeeded)))
            }
            EventType::EffectFailed => {
                awaiting.then(|| Deferred::End(End::of(event, Outcome::Failed)))
            }
            EventType::EffectIndeterminate => {
                awaiting.then(|| Deferred::End(End::of(event, Outcome::Indeterminate)))
            }
            EventType::EffectRequested
            | EventType::PhaseEntered
            | EventType::RunStarted
            | EventType::TransitionDecided
            | EventType::OperatorCommanded
            | EventType::OperatorAccepted
            | EventType::OperatorRejected
            | EventType::RunParked
            | EventType::RunCompleted
            | EventType::RunStopped => None,
        };
        if let Some(held) = held {
            self.pending
                .entry(effect_id.to_string())
                .or_default()
                .push(held);
        }
    }

    /// Whether an attempt's start is held for its effect before its
    /// request: the record of `attempt` then belongs to that start, and the
    /// live block does not read it aside.
    pub(super) fn awaiting(&self, effect_id: &str, attempt: Option<&str>) -> bool {
        self.pending.get(effect_id).is_some_and(|held| {
            held.iter().any(|event| {
                matches!(event, Deferred::Start(start) if start.attempt.as_deref() == attempt)
            })
        })
    }

    /// A checkpoint's dated facts on the row `request`'s `record` names:
    /// `open` where the attempt is the one in flight, as a checkpoint after
    /// its start is.
    pub(super) fn checkpointed(&mut self, request: usize, record: Record) {
        let open = self.requests[request].open.as_deref() == record.attempt.as_deref();
        let member = record.member.clone();
        let at = self.place(request, record.attempt.as_deref(), member.clone());
        let effect_id = self.requests[request].effect_id.clone();
        let running = self.running.entry((effect_id, member)).or_default();
        self.rows[at].checkpointed(record, open, running);
    }

    /// A terminal of the attempt in flight, read only for that attempt: a
    /// stale attempt's is passed over whole, as the view passes it over.
    pub(super) fn ended(&mut self, request: usize, end: &End) {
        if self.requests[request].open.as_deref() != end.attempt.as_deref() {
            return;
        }
        self.requests[request].open = None;
        let seq = end.seq;
        let row = self.row(request, end.attempt.as_deref(), Member::Seat);
        row.ended = Some((seq, end.outcome));
        state(&mut row.model, seq, end.model.clone());
        state(&mut row.stamp, seq, end.stamp.clone());
        state(&mut row.effort, seq, end.effort.clone());
    }
}

impl SeatRow {
    /// One checkpoint's dated facts: what it last stated stands, its first
    /// and last stamped, its turn's number, its item and cost counted, its
    /// meter kept once, and its step, member and model listed.
    fn checkpointed(&mut self, record: Record, open: bool, running: &mut Running) {
        let seq = record.seq;
        self.listed.noted(
            seq,
            record.model.as_deref(),
            record.step.as_deref(),
            record.step_name.as_deref(),
            &self.member,
        );
        state(&mut self.model, seq, record.model);
        if open {
            state(&mut self.stamp, seq, record.stamp);
        }
        state(&mut self.effort, seq, record.effort);
        state(&mut self.transcript, seq, record.transcript);
        self.first.get_or_insert_with(|| Stamp {
            seq,
            at: record.at.clone(),
        });
        if matches!(record.step.as_deref(), Some("seat-turn" | "turn-completed")) {
            if let Some(turn) = record.turn {
                self.turns = Some(self.turns.map_or(turn, |turns| turns.max(turn)));
            }
        }
        count_item(&mut self.items, record.step.as_deref());
        if let Some(cost) = record.cost {
            let spent = running.spent.unwrap_or(0.0) + cost;
            running.spent = Some(spent);
            self.spent = Some((seq, spent));
        }
        if record.session {
            self.session = Some((
                seq,
                Session {
                    meter: record.meter.clone(),
                    cost: record.cost,
                    id: record.session_id,
                },
            ));
        } else if record.finish_step && self.member != Member::Seat {
            self.finish = record.finish.map(|finish| (seq, finish));
        } else if record.step.as_deref() == Some("seat-turn") {
            self.turn = record.turned.map(|turn| (seq, turn));
        }
        self.last = Some(Stamp { seq, at: record.at });
        self.metered(record.meter, running);
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
