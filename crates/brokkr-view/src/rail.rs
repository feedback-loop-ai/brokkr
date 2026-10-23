//! The rail's order comes from the recorded table (decision 0094).
use super::*;
use brokkr_core::policy::Direction;

fn recorded_order(payload: &Value) -> Option<Vec<String>> {
    serde_json::from_value(payload.get("phases")?.clone()).ok()
}

fn is_return(order: Option<&[String]>, visits: &[(String, u64)], from: &str, next: &str) -> bool {
    match order {
        Some(order) => Direction::of(order, from, next) == Some(Direction::Return),
        None => {
            let place = |name: &str| visits.iter().position(|(known, _)| known == name);
            matches!((place(next), place(from)), (Some(to), Some(at)) if to < at)
        }
    }
}

pub(super) fn phase_rail(
    events: &[EventEnvelope],
    scan: &Scan,
    buckets: &Buckets,
    state: Option<&RunState>,
) -> Vec<Phase> {
    let order = events
        .first()
        .and_then(|event| recorded_order(&event.payload));
    let mut visits: Vec<(String, u64)> = Vec::new();
    // The roads back, as `(landing, departure)` pairs deduped in the
    // order the journal recorded them. A transition is BACKWARD when the
    // phase it names as `next` stands strictly before its `from` in the
    // recorded table order, or first entry for a legacy journal. A forward
    // step into a phase already entered (the second pass after a reforge) and a phase
    // ruled onto itself (a retry, ship's ready then shipped) are not
    // roads back: `visits` counts those arrivals, and the seats show them.
    let mut returns: Vec<(String, Road)> = Vec::new();
    for event in events {
        match event.event_type {
            EventType::PhaseEntered => {
                let Some(name) = field(&event.payload, "phase") else {
                    continue;
                };
                match visits.iter_mut().find(|(known, _)| known == name) {
                    Some((_, count)) => *count += 1,
                    None => visits.push((name.to_string(), 1)),
                }
            }
            EventType::TransitionDecided => {
                // A ruling that parks carries no `next` at all, and a
                // ruling this crate cannot read is never repaired into
                // one (README law 2).
                let (Some(from), Some(next)) =
                    (field(&event.payload, "from"), field(&event.payload, "next"))
                else {
                    continue;
                };
                let backward = is_return(order.as_deref(), &visits, from, next);
                if !backward {
                    continue;
                }
                let at = match returns
                    .iter()
                    .position(|(to, road)| to == next && road.from == from)
                {
                    Some(at) => at,
                    None => {
                        let road = Road {
                            from: from.to_string(),
                            taken: 0,
                            rules: Vec::new(),
                        };
                        returns.push((next.to_string(), road));
                        returns.len() - 1
                    }
                };
                let road = &mut returns[at].1;
                road.taken += 1;
                if let Some(rule) = field(&event.payload, "rule_id") {
                    road.rules.retain(|known| known != rule);
                    road.rules.push(rule.to_string());
                }
            }
            _ => continue,
        }
    }
    let current = match state.and_then(|state| state.phase.as_deref()) {
        Some(phase) if visits.iter().any(|(name, _)| name == phase) => Some(phase.to_string()),
        _ => visits.last().map(|(name, _)| name.clone()),
    };
    if let Some(order) = &order {
        visits.sort_by_key(|(name, _)| {
            order
                .iter()
                .position(|phase| phase == name)
                .unwrap_or(usize::MAX)
        });
    }
    visits
        .iter()
        .map(|(name, count)| {
            let columns = inner_columns(scan, buckets, name);
            let plain = match columns.first() {
                Some(column) => columns.len() == 1 && column.nodes.len() == 1,
                None => true,
            };
            Phase {
                name: name.clone(),
                strategy: if name == "triage" {
                    state.and_then(|state| state.strategy.clone())
                } else {
                    None
                },
                visits: *count,
                current: current.as_deref() == Some(name.as_str()),
                plain,
                columns,
                returns: returns
                    .extract_if(.., |(to, _)| to == name)
                    .map(|(_, road)| road)
                    .collect(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests;
