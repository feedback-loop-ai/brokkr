//! The hand-built seat journals the writer's seat rows are held to (#753,
//! decision 0089 ruling 4): what no committed journal holds, a retried
//! effect, done, in flight, interleaved and priced twice by its retry, a
//! seat whose turns and session record both
//! carry tokens, a panel, a request never started, the records that
//! name no attempt or no member, an attempt started again, and a
//! provenance entry naming no provider. brokkr-core pins each one's facts; brokkr-view holds the
//! same journals' facts equal to its run view. A test module includes this
//! file through `#[path]` beside an `envelope_builder` module, the one
//! place a test spells an envelope (#357).

use brokkr_core::envelope::{EventEnvelope, EventType};
use serde_json::{json, Value};

use super::envelope_builder::EnvelopeBuilder;

/// The stamp of the event at `seq`: one second apart.
pub(crate) fn at(seq: u64) -> String {
    format!("2026-01-01T00:00:{seq:02}Z")
}

/// A journal of `events`, each at its seq and stamp from 1.
pub(crate) fn journal(events: Vec<(EventType, Value)>) -> Vec<EventEnvelope> {
    let seqs = 1u64..;
    let events = seqs.zip(events);
    events
        .map(|(seq, (event_type, payload))| {
            EnvelopeBuilder::new(event_type, payload)
                .seq(seq)
                .at(at(seq))
                .build()
        })
        .collect()
}

/// A checkpoint of effect `effect`'s attempt `attempt`.
pub(crate) fn checkpoint(effect: &str, attempt: &str, checkpoint: Value) -> (EventType, Value) {
    let payload = json!({"effect_id": effect, "attempt_id": attempt, "checkpoint": checkpoint});
    (EventType::EffectCheckpointed, payload)
}

/// A seat whose first attempt fails and whose retry succeeds, each
/// reporting a cost: its second attempt's first turn repeats the first
/// attempt's last, and its Claude session record settles its output. The
/// retry's attempt id sorts before the first's, and the first attempt
/// records once more while the retry is in flight.
pub(crate) fn retried() -> Vec<EventEnvelope> {
    let turn = |turn: u64, input: u64, output: u64| {
        json!({"step": "seat-turn", "turn": turn, "model": "opus", "effort": "high",
               "input_tokens": input, "output_tokens": output})
    };
    let session = |cost: f64, output: u64, locator: &str| {
        json!({"step": "claude-code-session-finished", "total_cost_usd": cost,
               "output_tokens": output,
               "transcript": {"kind": "claude-session", "locator": locator, "home": "/h"}})
    };
    let provenance = |provider: &str| {
        json!([{"member": null, "agent": "implementer", "model": "opus",
                "provider": provider, "chain_index": 0}])
    };
    journal(vec![
        (EventType::PhaseEntered, json!({"phase": "implement"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "e1", "seat": "implementer", "phase": "implement"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "first", "provenance": provenance("anthropic"),
                   "boundary": [{"member": null, "boundary": "namespace", "gate": true}]}),
        ),
        checkpoint("e1", "first", turn(1, 100, 10)),
        checkpoint(
            "e1",
            "first",
            json!({"step": "item-completed", "tool": "edit"}),
        ),
        checkpoint("e1", "first", session(0.1, 40, "s1")),
        (
            EventType::EffectFailed,
            json!({"effect_id": "e1", "attempt_id": "first"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e1", "attempt_id": "again", "provenance": provenance("openai")}),
        ),
        checkpoint("e1", "again", turn(1, 100, 10)),
        checkpoint(
            "e1",
            "again",
            json!({"step": "item-completed", "tool": "edit"}),
        ),
        checkpoint(
            "e1",
            "again",
            json!({"step": "seat-turn", "turn": 2, "model": "opus-2", "boundary": "harness",
                   "input_tokens": 50, "output_tokens": 5}),
        ),
        checkpoint(
            "e1",
            "first",
            json!({"step": "late-flush", "boundary": "seatbelt"}),
        ),
        checkpoint("e1", "again", session(0.2, 30, "s2")),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "e1", "attempt_id": "again",
                   "result": {"result": "pass", "model": "opus-2", "effort": "max"}}),
        ),
    ])
}

/// A codex seat still at work: its turns carry tokens, one of them twice,
/// its items count beside them, and its session records, the opening and
/// the finishing one, are not a Claude session's, so its turns alone
/// meter it.
pub(crate) fn turns() -> Vec<EventEnvelope> {
    let turn = |turn: u64, input: u64, output: u64| {
        json!({"step": "turn-completed", "turn": turn, "input_tokens": input,
               "output_tokens": output, "reasoning_output_tokens": 5, "cache_read_tokens": 150})
    };
    journal(vec![
        (
            EventType::EffectRequested,
            json!({"effect_id": "e2", "seat": "reviewer", "phase": "review"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "e2", "attempt_id": "b1"}),
        ),
        checkpoint(
            "e2",
            "b1",
            json!({"step": "session-started", "session_id": "thread-1"}),
        ),
        checkpoint(
            "e2",
            "b1",
            json!({"step": "item-completed", "tool": "shell"}),
        ),
        checkpoint("e2", "b1", turn(1, 200, 20)),
        checkpoint("e2", "b1", turn(1, 200, 20)),
        checkpoint(
            "e2",
            "b1",
            json!({"step": "item-completed", "tool": "read"}),
        ),
        checkpoint("e2", "b1", turn(2, 300, 30)),
        checkpoint("e2", "b1", turn(3, 400, 40)),
        checkpoint(
            "e2",
            "b1",
            json!({"step": "codex-session-finished", "input_tokens": 999,
                   "output_tokens": 999, "total_cost_usd": 1.5}),
        ),
    ])
}

/// The retried seat while its retry is in flight: at work, though its
/// first attempt failed.
pub(crate) fn retrying() -> Vec<EventEnvelope> {
    let mut events = retried();
    events.truncate(9);
    events
}

/// A panel of two members, each with its own provider, boundary, model,
/// effort, tokens and cost; one pins its own outcome and records an
/// unnumbered turn after it, the other names an outcome the view does not
/// let stand; a finish with no member pins nothing; and the next request
/// is the panel's members' no more than any other seat's.
pub(crate) fn panel() -> Vec<EventEnvelope> {
    let site = |member: &str, provider: &str| {
        json!({"member": member, "agent": "judge", "model": "m", "provider": provider,
               "chain_index": 0})
    };
    let boundary =
        |member: &str, word: &str| json!({"member": member, "boundary": word, "gate": false});
    journal(vec![
        (EventType::PhaseEntered, json!({"phase": "review"})),
        (
            EventType::EffectRequested,
            json!({"effect_id": "p1", "seat": "panel"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "p1", "attempt_id": "c1",
                   "provenance": [site("a", "anthropic"), site("b", "openai")],
                   "boundary": [boundary("a", "namespace"), boundary("b", "harness")]}),
        ),
        checkpoint(
            "p1",
            "c1",
            json!({"member": "a", "step": "seat-turn", "turn": 3, "model": "m-a",
                   "effort": "low", "input_tokens": 10, "output_tokens": 1}),
        ),
        checkpoint(
            "p1",
            "c1",
            json!({"member": "b", "step": "turn-completed", "turn": 2, "model": "m-b",
                   "effort": "high", "input_tokens": 20, "output_tokens": 2}),
        ),
        checkpoint(
            "p1",
            "c1",
            json!({"member": "a", "step": "claude-code-session-finished", "total_cost_usd": 0.5}),
        ),
        checkpoint(
            "p1",
            "c1",
            json!({"member": "b", "step": "codex-session-finished", "total_cost_usd": 0.25}),
        ),
        checkpoint(
            "p1",
            "c1",
            json!({"step": "panel-member-finished", "outcome": "failed"}),
        ),
        checkpoint(
            "p1",
            "c1",
            json!({"member": "a", "step": "panel-member-finished", "outcome": "failed"}),
        ),
        checkpoint(
            "p1",
            "c1",
            json!({"member": "b", "step": "panel-member-finished", "outcome": "unheard-of"}),
        ),
        checkpoint(
            "p1",
            "c1",
            json!({"member": "a", "step": "seat-turn", "tool": "Read"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "p1", "attempt_id": "c1", "result": {"result": "pass"}}),
        ),
        (
            EventType::EffectRequested,
            json!({"effect_id": "p2", "seat": "after"}),
        ),
    ])
}

/// A seat whose failed attempt is followed by a start that names no
/// attempt: no attempt is in flight, so its failure stands.
pub(crate) fn restarted() -> Vec<EventEnvelope> {
    journal(vec![
        (
            EventType::EffectRequested,
            json!({"effect_id": "r1", "seat": "again"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "r1", "attempt_id": "first"}),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "r1", "attempt_id": "first"}),
        ),
        (EventType::EffectStarted, json!({"effect_id": "r1"})),
    ])
}

/// Two requests no attempt started: the first closed as indeterminate,
/// the second still asked.
pub(crate) fn unstarted() -> Vec<EventEnvelope> {
    journal(vec![
        (
            EventType::EffectRequested,
            json!({"effect_id": "u1", "seat": "clarifier", "phase": "intake"}),
        ),
        (
            EventType::EffectIndeterminate,
            json!({"effect_id": "u1", "reason": "the operator closed it"}),
        ),
        (
            EventType::EffectRequested,
            json!({"effect_id": "u2", "seat": "next"}),
        ),
    ])
}

/// Records that name no attempt, a stale attempt's terminal, and events
/// the scan passes over: a request with no effect, a record of an effect
/// never requested, a ruling that names the effect.
pub(crate) fn unnamed() -> Vec<EventEnvelope> {
    journal(vec![
        (EventType::EffectRequested, json!({"seat": "nobody"})),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "ghost", "checkpoint": {"step": "seat-turn", "turn": 9}}),
        ),
        (
            EventType::EffectRequested,
            json!({"effect_id": "s1", "seat": "solo"}),
        ),
        (EventType::EffectStarted, json!({"effect_id": "s1"})),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "s1", "checkpoint": {"step": "seat-turn", "turn": 1,
                   "model": "m1", "boundary": "open", "total_cost_usd": 0.125}}),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "s1", "attempt_id": "stale"}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "s1", "result": {"model": "m2", "boundary": "container"}}),
        ),
        (
            EventType::TransitionDecided,
            json!({"effect_id": "s1", "from": "solo"}),
        ),
    ])
}

/// A seat whose first attempt keeps recording after its retry started:
/// the retry's first turn repeats the first attempt's message, and the
/// first attempt's late turn reports a new one and a cost after the
/// retry's. Its turns and costs read in journal order, not attempt by
/// attempt.
pub(crate) fn interleaved() -> Vec<EventEnvelope> {
    let turn = |turn: u64, input: u64, cost: f64| {
        json!({"step": "seat-turn", "turn": turn, "input_tokens": input,
               "output_tokens": input / 10, "total_cost_usd": cost})
    };
    journal(vec![
        (
            EventType::EffectRequested,
            json!({"effect_id": "i1", "seat": "builder"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "i1", "attempt_id": "a"}),
        ),
        checkpoint("i1", "a", turn(1, 100, 0.2)),
        (
            EventType::EffectStarted,
            json!({"effect_id": "i1", "attempt_id": "b"}),
        ),
        checkpoint("i1", "b", turn(1, 100, 0.1)),
        checkpoint("i1", "a", turn(2, 200, 0.3)),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "i1", "attempt_id": "b"}),
        ),
    ])
}

/// A seat whose retry prices two records after its first attempt priced
/// one: its cost sums the three in journal order.
pub(crate) fn repriced() -> Vec<EventEnvelope> {
    let priced = |step: &str, cost: f64| json!({"step": step, "total_cost_usd": cost});
    journal(vec![
        (
            EventType::EffectRequested,
            json!({"effect_id": "c1", "seat": "pricer"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "c1", "attempt_id": "a"}),
        ),
        checkpoint("c1", "a", priced("claude-code-session-finished", 0.1)),
        (
            EventType::EffectFailed,
            json!({"effect_id": "c1", "attempt_id": "a"}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "c1", "attempt_id": "b"}),
        ),
        checkpoint("c1", "b", priced("item-completed", 0.2)),
        checkpoint("c1", "b", priced("claude-code-session-finished", 0.3)),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": "c1", "attempt_id": "b"}),
        ),
    ])
}

/// A seat whose failed attempt a second start names again: the start
/// re-opens it, so it is at work, dated by its first start.
pub(crate) fn reopened() -> Vec<EventEnvelope> {
    let started = || {
        let payload = json!({"effect_id": "o1", "attempt_id": "a"});
        (EventType::EffectStarted, payload)
    };
    journal(vec![
        (
            EventType::EffectRequested,
            json!({"effect_id": "o1", "seat": "again"}),
        ),
        started(),
        (
            EventType::EffectFailed,
            json!({"effect_id": "o1", "attempt_id": "a"}),
        ),
        started(),
    ])
}

/// A seat whose retry's provenance names no provider: the provider its
/// first attempt named does not stand.
pub(crate) fn unprovided() -> Vec<EventEnvelope> {
    let started = |attempt: &str, site: Value| {
        let payload = json!({"effect_id": "v1", "attempt_id": attempt, "provenance": [site]});
        (EventType::EffectStarted, payload)
    };
    journal(vec![
        (
            EventType::EffectRequested,
            json!({"effect_id": "v1", "seat": "sourced"}),
        ),
        started(
            "a",
            json!({"member": null, "agent": "implementer", "provider": "anthropic"}),
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "v1", "attempt_id": "a"}),
        ),
        started("b", json!({"member": null, "agent": "implementer"})),
    ])
}

/// Every hand-built journal, by name.
pub(crate) fn journals() -> Vec<(&'static str, Vec<EventEnvelope>)> {
    vec![
        ("retried", retried()),
        ("retrying", retrying()),
        ("interleaved", interleaved()),
        ("repriced", repriced()),
        ("turns", turns()),
        ("panel", panel()),
        ("restarted", restarted()),
        ("unstarted", unstarted()),
        ("unnamed", unnamed()),
        ("reopened", reopened()),
        ("unprovided", unprovided()),
    ]
}
