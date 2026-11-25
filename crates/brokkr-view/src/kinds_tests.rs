//! The view as a reader of a record's kind (decision 0075 ruling 2; the
//! controller's rulings R22 item 3 and R24 of 2026-11-22): it takes a
//! transcript kind only from a checkpoint's `transcript`, and one it does
//! not know reads as no transcript it can use, never as another kind.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::Status;
use serde_json::{json, Value};

use crate::tests::{ev, seat_journal, state};
use crate::transcript::{select_reference, Unavailable};
use crate::{run_view, Participant, Subject, Transcript};

const T: &str = "2026-01-01T00:00:05Z";
const HOME: &str = "/home/operator/.claude/projects";

/// The intake seat's participants over `events`, the run completed.
fn participants(events: &[brokkr_core::EventEnvelope]) -> Vec<Participant> {
    let state = state(Some("intake"), Status::Completed, None);
    run_view(events, Some(&state)).participants
}

/// A transcript reference of `kind` at `locator`.
fn reference(kind: &str, locator: &str) -> Value {
    json!({"kind": kind, "locator": locator, "home": HOME})
}

/// A checkpoint transcript of a kind in the grammar the view does not
/// know (`trellis-trail`) is the seat's reference, echoed as recorded and
/// read as `unsupported-kind`: no earlier reference of a known kind stands
/// in, no Claude session id is read from it, and the rest of the record
/// (its cost, the seat's turns and status) reads as it does for a known
/// kind.
#[test]
fn a_transcript_kind_the_view_does_not_know_is_unsupported_and_the_rest_reads() {
    let known = participants(&seat_journal());
    let mut events = seat_journal();
    events[4].payload["checkpoint"]["transcript"] = reference("claude-session", "sess-0");
    events[5].payload["checkpoint"]["transcript"]["kind"] = json!("trellis-trail");
    let read = participants(&events);
    let part = &read[0];
    let recorded = Transcript {
        kind: "trellis-trail".into(),
        locator: "sess-1".into(),
        home: HOME.into(),
    };
    assert_eq!(part.transcript.as_ref(), Some(&recorded));
    assert_eq!(part.session_id, None);
    assert_eq!(known[0].session_id.as_deref(), Some("sess-1"));
    assert_eq!(
        (part.cost, part.turns, &part.status),
        (known[0].cost, known[0].turns, &known[0].status)
    );
    let subject = Subject::of(0, None, "r1".into(), part);
    let selection = select_reference(
        subject.reference.as_ref(),
        subject.provenance,
        subject.legacy_id.as_deref(),
        Some(HOME),
    );
    assert_eq!(selection.outcome, Err(Unavailable::UnsupportedKind));
    assert_eq!(selection.reference, Some(recorded));
    assert!(!selection.legacy);
}

/// The controller's ruling R29: a participant whose transcript names a
/// kind the view does not know (`trellis-trail`) beside a flat
/// `session_id` has no compatibility id, and the rest of its row reads; a
/// `codex-thread` transcript beside the same flat id reads it, as before.
#[test]
fn a_transcript_kind_the_view_does_not_know_is_no_fallback_to_the_flat_session_id() {
    let flat = |kind: &str| {
        let mut events = seat_journal();
        let checkpoint = &mut events[5].payload["checkpoint"];
        checkpoint["transcript"] = reference(kind, "loc-1");
        checkpoint["session_id"] = json!("flat-1");
        participants(&events).remove(0)
    };
    let known = participants(&seat_journal()).remove(0);
    let unknown = flat("trellis-trail");
    assert_eq!(unknown.session_id, None);
    assert_eq!(
        unknown.transcript.map(|read| read.kind).as_deref(),
        Some("trellis-trail")
    );
    assert_eq!(
        (unknown.cost, unknown.turns, &unknown.status),
        (known.cost, known.turns, &known.status)
    );
    assert_eq!(flat("codex-thread").session_id.as_deref(), Some("flat-1"));
}

/// A kind of a reader's own vocabulary standing only inside evidence — a
/// result's `notes.members`, decide's `inputs.raw_result`, an unresolved
/// cleanup's `received` — moves no reader: the view shows no transcript
/// and no session for any participant.
#[test]
fn a_kind_standing_only_inside_evidence_shows_no_transcript() {
    let stray = reference("claude-session", "sess-evidence");
    let mut events = seat_journal();
    let checkpoint = &mut events[5].payload["checkpoint"];
    checkpoint.as_object_mut().unwrap().remove("transcript");
    events[6].payload["result"]["notes"] = json!({"members": {"judge": {"transcript": stray}}});
    let raw_result = json!({"result": "intook", "transcript": stray});
    let received = json!({"status": "succeeded", "result": {"transcript": stray}});
    events.extend(
        [
            (
                EventType::TransitionDecided,
                json!({"from": "intake", "result": "__schema-invalid__",
                   "inputs": {"raw_result": raw_result}}),
            ),
            (
                EventType::EffectRequested,
                json!({"effect_id": "eff2", "seat": "intake", "phase": "intake"}),
            ),
            (
                EventType::EffectStarted,
                json!({"effect_id": "eff2", "attempt_id": "att2"}),
            ),
            (
                EventType::EffectIndeterminate,
                json!({"effect_id": "eff2", "attempt_id": "att2", "received": received,
                   "cleanup": {"status": "unresolved"}}),
            ),
        ]
        .into_iter()
        .zip(8..)
        .map(|((kind, payload), seq)| ev(seq, kind, payload, T)),
    );
    let read = participants(&events);
    assert_eq!(read.len(), 2);
    for part in &read {
        assert_eq!(part.transcript, None, "{}", part.key);
        assert_eq!(part.session_id, None, "{}", part.key);
    }
}
