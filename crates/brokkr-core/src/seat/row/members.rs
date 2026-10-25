//! A panel or sequence seat's facts where it has none of its own (#753;
//! decision 0089 ruling 4): its members' summed or joined, and flagged as
//! such, as the run view reads a seat whose members carry its telemetry.

use std::collections::BTreeSet;

use super::SeatFacts;
use crate::seat::meter::sum_usage;

/// `values` summed in their order, or `None` for none.
fn summed(values: impl Iterator<Item = f64>) -> Option<f64> {
    values.fold(None, |total, value| Some(total.unwrap_or(0.0) + value))
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
pub(super) fn with_members(own: &SeatFacts, members: &[&SeatFacts]) -> SeatFacts {
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
