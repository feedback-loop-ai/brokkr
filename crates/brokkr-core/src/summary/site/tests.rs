//! The site projection (decision 0089 ruling 7): what the reading keeps of
//! a run's sites, the message each turn repeats, and how the metering rule
//! moves as a site's checkpoints arrive.

use super::*;
use crate::envelope::EventType;
use crate::summary::test_support::{checkpoint as turn, journal};
use crate::summary::Account;
use serde_json::json;

/// A run whose only attempt starts with the provenance entries `entries`,
/// after entering `phase`.
fn started(
    phase: &str,
    entries: serde_json::Value,
) -> Vec<(EventType, serde_json::Value, &'static str)> {
    vec![
        (
            EventType::PhaseEntered,
            json!({"phase": phase}),
            "2026-01-01T00:00:00Z",
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a", "provenance": entries}),
            "2026-01-01T00:00:01Z",
        ),
    ]
}

#[test]
fn a_site_reads_its_start_phase_provider_model_and_metering() {
    let mut steps = started(
        "implement",
        json!([{"member": "m", "provider": "anthropic"}]),
    );
    steps.push(turn(
        Some("m"),
        json!({"step": "seat-turn", "model": "opus", "input_tokens": 10, "output_tokens": 4}),
    ));
    steps.push((
        EventType::EffectSucceeded,
        json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m", "model": "sonnet"}}),
        "2026-01-01T00:00:03Z",
    ));
    let sites = super::sites(&journal(&steps));
    assert_eq!(sites.len(), 1);
    let site = &sites[0];
    assert_eq!(site.attempt_id, "a");
    assert_eq!(site.member.as_deref(), Some("m"));
    assert_eq!(site.at, 1_767_225_601_000);
    assert_eq!(site.phase.as_deref(), Some("implement"));
    assert_eq!(site.provider, "anthropic");
    assert_eq!(site.model.as_deref(), Some("sonnet"));
    assert_eq!(site.metering, Metering::Turns);
    assert_eq!(site.metered.len(), 1);
    assert_eq!(
        site.metered[0].meter.usage().unwrap().input_tokens,
        Some(10)
    );
}

#[test]
fn a_turn_repeating_its_site_s_last_message_is_not_metered_again() {
    let mut steps = started(
        "implement",
        json!([{"member": "m", "provider": "anthropic"}]),
    );
    for (input, output) in [(10u64, 4u64), (10, 4), (12, 5)] {
        steps.push(turn(
            Some("m"),
            json!({"step": "seat-turn", "input_tokens": input, "output_tokens": output}),
        ));
    }
    let sites = super::sites(&journal(&steps));
    let metered = &sites[0].metered;
    assert_eq!(
        metered
            .iter()
            .map(|checkpoint| checkpoint.repeat)
            .collect::<Vec<_>>(),
        [Some(false), Some(true), Some(false)]
    );
    assert!(metered[0].meter.usage().is_some());
    assert_eq!(metered[1].meter.usage(), None);
    assert!(metered[2].meter.usage().is_some());
}

#[test]
fn a_site_s_metering_rule_moves_as_its_checkpoints_arrive() {
    let mut steps = started(
        "implement",
        json!([{"member": "m", "provider": "anthropic"}]),
    );
    steps.push(turn(Some("m"), json!({"step": "claude-session-finished"})));
    let reading = |steps: &[(EventType, serde_json::Value, &'static str)]| {
        Account::whole("r", &journal(steps))
            .sites()
            .first()
            .map(|site| site.metering)
    };
    assert_eq!(reading(&steps), Some(Metering::Session));
    steps.push(turn(
        Some("m"),
        json!({"step": "seat-turn", "input_tokens": 10}),
    ));
    assert_eq!(reading(&steps), Some(Metering::Turns));
    steps.push(turn(
        Some("m"),
        json!({"step": "claude-session-finished", "output_tokens": 3}),
    ));
    assert_eq!(reading(&steps), Some(Metering::Settled));
}

/// [`started`] with one `opus` turn of `member` metering 10 input tokens.
fn opus_turn(
    mut steps: Vec<(EventType, serde_json::Value, &'static str)>,
) -> Vec<(EventType, serde_json::Value, &'static str)> {
    steps.push(turn(
        Some("m"),
        json!({"step": "seat-turn", "model": "opus", "input_tokens": 10}),
    ));
    steps
}

#[test]
fn a_carried_account_keeps_each_site_s_facts_and_no_checkpoint() {
    let steps = journal(&opus_turn(started(
        "implement",
        json!([{"member": "m", "provider": "anthropic"}]),
    )));
    let carried = Account::whole("r", &steps).carried();
    let site = &carried.sites()[0];
    assert_eq!(site.at, 1_767_225_601_000);
    assert_eq!(site.phase.as_deref(), Some("implement"));
    assert_eq!(site.provider, "anthropic");
    assert_eq!(site.model.as_deref(), Some("opus"));
    assert_eq!(site.metering, Metering::Turns);
    assert!(site.metered.is_empty());
}

#[test]
fn the_account_s_sites_are_the_sites_read_whole() {
    let mut steps = opus_turn(started(
        "implement",
        json!([{"member": "m", "provider": "anthropic"}]),
    ));
    steps.push((
        EventType::EffectSucceeded,
        json!({"effect_id": "fx", "attempt_id": "a", "result": {"member": "m", "model": "sonnet"}}),
        "2026-01-01T00:00:03Z",
    ));
    let steps = journal(&steps);
    let mut carried = Account::empty("r");
    for event in &steps {
        carried = carried.onto(std::slice::from_ref(event));
    }
    let whole = Account::whole("r", &steps);
    for (carried, whole) in carried.sites().iter().zip(whole.sites()) {
        assert_eq!(carried.at, whole.at);
        assert_eq!(carried.phase, whole.phase);
        assert_eq!(carried.provider, whole.provider);
        assert_eq!(carried.model, whole.model);
        assert_eq!(carried.metering, whole.metering);
    }
    assert_eq!(carried.sites().len(), whole.sites().len());
}

#[test]
fn a_settling_session_record_alone_meters_as_a_session() {
    let mut steps = started(
        "implement",
        json!([{"member": "m", "provider": "anthropic"}]),
    );
    steps.push(turn(
        Some("m"),
        json!({"step": "claude-session-finished", "output_tokens": 3}),
    ));
    let sites = super::sites(&journal(&steps));
    assert_eq!(sites[0].metering, Metering::Session);
}

#[test]
fn a_site_opens_only_from_a_provenance_entry_that_names_a_provider() {
    let mut steps = started(
        "implement",
        json!([
            {"member": "none", "provider": null},
            {"member": "m", "provider": "anthropic"}
        ]),
    );
    // A checkpoint no start opened, and one whose record names no provider
    // site, open nothing.
    steps.push((
        EventType::EffectCheckpointed,
        json!({"attempt_id": "b", "checkpoint": {"member": "m", "step": "seat-turn"}}),
        "2026-01-01T00:00:02Z",
    ));
    steps.push(turn(
        Some("none"),
        json!({"step": "seat-turn", "input_tokens": 1}),
    ));
    let sites = super::sites(&journal(&steps));
    assert_eq!(sites.len(), 1);
    assert_eq!(sites[0].member.as_deref(), Some("m"));
    assert_eq!(sites[0].provider, "anthropic");
}
