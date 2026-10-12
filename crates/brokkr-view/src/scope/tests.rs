use serde_json::json;

use super::*;
use crate::run_view;
use crate::tests::ev;
use brokkr_core::EventType;

const T0: &str = "2026-01-01T00:00:00Z";

/// Intake, then design, review and design again, each phase seated, and
/// one seat requested with no phase at all.
fn journal() -> Vec<brokkr_core::EventEnvelope> {
    let seat = |seq: u64, effect: &str, seat: &str, phase: &str| {
        ev(
            seq,
            EventType::EffectRequested,
            json!({"effect_id": effect, "seat": seat, "phase": phase}),
            T0,
        )
    };
    let entered =
        |seq: u64, phase: &str| ev(seq, EventType::PhaseEntered, json!({"phase": phase}), T0);
    let ruled = |seq: u64, from: &str, next: &str| {
        ev(
            seq,
            EventType::TransitionDecided,
            json!({"rule_id": "R", "from": from, "next": next}),
            T0,
        )
    };
    vec![
        ev(1, EventType::RunStarted, json!({"feature": "scope it"}), T0),
        entered(2, "intake"),
        seat(3, "eff-i", "intake", "intake"),
        ruled(4, "intake", "design"),
        entered(5, "design"),
        seat(6, "eff-d", "design", "design"),
        ruled(7, "design", "review"),
        entered(8, "review"),
        seat(9, "eff-r", "review", "review"),
        ruled(10, "review", "design"),
        entered(11, "design"),
        seat(12, "eff-d2", "design", "design"),
        ev(
            13,
            EventType::EffectRequested,
            json!({"effect_id": "eff-x", "seat": "loose"}),
            T0,
        ),
    ]
}

fn kept(phases: &[&str], participants: &[&str], rows: &[u64]) -> Membership {
    Membership {
        phases: phases.iter().map(ToString::to_string).collect(),
        participants: participants.iter().map(ToString::to_string).collect(),
        rows: rows.to_vec(),
    }
}

fn lens(view: &RunView, scope: Scope) -> Lens {
    lens_for(view, Some(&scope)).unwrap().unwrap()
}

/// The scopes a served view carries.
fn served(view: &RunView) -> &Scopes {
    view.scopes.as_ref().unwrap()
}

/// A phase scope keeps that phase, the participants requested in it and
/// its journal rows, and the run view serves the same answer by name.
#[test]
fn a_phase_scope_keeps_its_phase_its_participants_and_its_rows() {
    let view = run_view(&journal(), None).with_scopes();
    let design = kept(&["design"], &["eff-d", "eff-d2"], &[5, 6, 7, 11, 12]);
    let by_lens = membership(&view, &lens(&view, Scope::Phase("design".into())));
    assert_eq!(by_lens, design);
    assert_eq!(served(&view).phases["design"], design);
    assert_eq!(
        served(&view).phases["intake"],
        kept(&["intake"], &["eff-i"], &[2, 3, 4])
    );
}

/// A seat scope keeps the seat's phases, matches participants by key and
/// keeps only those phases' rows. A label matches every occurrence; a key
/// matches its own participant, which is what the browser serves.
#[test]
fn a_seat_scope_keeps_its_phases_rows_and_matches_participants_by_key() {
    let view = run_view(&journal(), None).with_scopes();
    let design = lens(&view, Scope::Seat("design".into()));
    // Both occurrences sat in design: the lens names it once.
    assert_eq!(design.phases, ["design"]);
    let by_label = membership(&view, &design);
    assert_eq!(
        by_label,
        kept(&["design"], &["eff-d", "eff-d2"], &[5, 6, 7, 11, 12])
    );
    let by_key = kept(&["design"], &["eff-d2"], &[5, 6, 7, 11, 12]);
    assert_eq!(
        membership(&view, &lens(&view, Scope::Seat("eff-d2".into()))),
        by_key
    );
    assert_eq!(served(&view).seats["eff-d2"], by_key);
    // A seat with no phase keeps itself and no phase's traffic.
    assert_eq!(served(&view).seats["eff-x"], kept(&[], &["eff-x"], &[]));
    assert_eq!(
        serde_json::to_value(&view).unwrap()["scopes"]["seats"]["eff-i"],
        json!({"phases": ["intake"], "participants": ["eff-i"], "rows": [2, 3, 4]})
    );
}

/// The terminal's view decides no scope: a frame asks the predicates, and
/// only a serving surface pays for every scope's membership (#624).
#[test]
fn the_terminal_view_decides_no_scope_until_it_is_served() {
    let view = run_view(&journal(), None);
    assert_eq!(view.scopes, None);
    let expected = scopes(&view);
    assert_eq!(view.with_scopes().scopes, Some(expected));
}

/// No scope is no lens, and no lens keeps everything.
#[test]
fn no_scope_keeps_every_phase_participant_and_row() {
    let view = run_view(&journal(), None);
    assert!(lens_for(&view, None).unwrap().is_none());
    assert!(view.phases.iter().all(|phase| keeps_phase(None, phase)));
    assert!(view
        .participants
        .iter()
        .all(|part| keeps_participant(None, part)));
    assert!(view.journal.iter().all(|row| keeps_row(None, row)));
}

/// A scope matching nothing names what the run offers, in today's words:
/// the visited phases, and the participants with adjacent repeats folded.
#[test]
fn a_scope_matching_nothing_names_what_the_run_offers() {
    let view = run_view(&journal(), None);
    let missing = lens_for(&view, Some(&Scope::Phase("nowhere".into())))
        .err()
        .unwrap();
    assert_eq!(
        missing,
        ScopeError::UnknownPhase {
            phase: "nowhere".into(),
            visited: vec!["intake".into(), "design".into(), "review".into()],
        }
    );
    assert_eq!(
        missing.to_string(),
        "no phase 'nowhere' in this run; visited phases: intake, design, review"
    );
    let mut repeated = journal();
    repeated.push(ev(
        14,
        EventType::EffectRequested,
        json!({"effect_id": "eff-x2", "seat": "loose"}),
        T0,
    ));
    let view = run_view(&repeated, None);
    let unseated = lens_for(&view, Some(&Scope::Seat("nobody".into())))
        .err()
        .unwrap();
    assert_eq!(
        unseated.to_string(),
        "no seat 'nobody' in this run; participants: intake, design, review, design, loose"
    );
}
