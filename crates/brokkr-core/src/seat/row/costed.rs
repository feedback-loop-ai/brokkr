//! What `brokkr costs` reads of a run's seat rows (#753; decision 0089
//! slice 3, the operator's 2026-10-25 ruling): each effect's accounting,
//! folded as [`seat_costs`](crate::seat::seat_costs) folds it, kept on its
//! latest request's own row. An effect is seated from its first request
//! naming a seat, and its accounting is read from then on, as the
//! journal's is; a request asking it again takes the accounting so far
//! onto its own row, so the effect's latest own row holds all of it, and
//! the seat the last request naming one named is its seat.

use std::collections::BTreeMap;

use super::{Member, SeatRow, Seats};
use crate::envelope::{EventEnvelope, EventType};
use crate::residual::text as field;
use crate::seat::costs::{summed, EffectAccounting};
use crate::seat::SeatCost;

impl Seats {
    /// `event`'s part in its effect's accounting, on the own row of the
    /// effect's latest request: a request takes the accounting its effect's
    /// request before it kept, or opens it where it names a seat; any other
    /// event of a seated effect is noted on it.
    pub(super) fn accounted(&mut self, event: &EventEnvelope) {
        let Some(effect_id) = field(&event.payload, "effect_id") else {
            return;
        };
        let Some(&request) = self.by_effect.get(effect_id) else {
            return;
        };
        let at = self.own(request);
        if event.event_type != EventType::EffectRequested {
            if let Some(costs) = &mut self.rows[at].costs {
                costs.note(event);
            }
            return;
        }
        let earlier = self.requests[..request]
            .iter()
            .rposition(|asked| asked.effect_id == effect_id);
        let kept = earlier.and_then(|earlier| self.rows[self.own(earlier)].costs.clone());
        let seated = self.requests[request].seat.is_some();
        self.rows[at].costs = kept.or_else(|| seated.then(EffectAccounting::default));
    }

    /// The place of `request`'s own row, which its request opened.
    fn own(&self, request: usize) -> usize {
        self.at[&(request, None)][&Member::Seat]
    }
}

impl SeatRow {
    /// Whether it is a request's own row: of no attempt and no member.
    fn is_own(&self) -> bool {
        self.attempt_id.is_none() && self.member == Member::Seat
    }

    /// Whether it holds an accounting where its writer keeps one: on a
    /// request's own row alone, and on every one naming a seat, as its
    /// effect is seated from then on.
    pub(super) fn costed(&self) -> bool {
        match (self.is_own(), &self.costs) {
            (true, Some(_)) => true,
            (true, None) => self.seat.is_none(),
            (false, costs) => costs.is_none(),
        }
    }
}

/// Each seat's record and the total cost, from a run's `rows` in the order
/// they were opened: what [`seat_costs`](crate::seat::seat_costs) gives of
/// the run's journal. Each effect is read from its latest own row holding
/// its accounting, under the seat its latest own row naming one names, in
/// effect id order, the order `seat_costs` sums them in.
pub fn costs(rows: &[SeatRow]) -> (BTreeMap<String, SeatCost>, f64) {
    let mut effects: BTreeMap<&str, (Option<&str>, Option<&EffectAccounting>)> = BTreeMap::new();
    for row in rows.iter().filter(|row| row.is_own()) {
        let effect = effects.entry(row.effect_id.as_str()).or_default();
        effect.0 = row.seat.as_deref().or(effect.0);
        effect.1 = row.costs.as_ref().or(effect.1);
    }
    summed(
        effects
            .into_values()
            .filter_map(|(seat, accounting)| seat.zip(accounting)),
    )
}

#[cfg(test)]
mod tests;
