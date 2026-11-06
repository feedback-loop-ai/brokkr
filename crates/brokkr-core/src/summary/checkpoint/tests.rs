//! The checkpoint projection (decision 0089 ruling 7): a checkpoint reads
//! its time, the run's phase, the cost it reported or none, its tokens and
//! whether it is its site's last finishing session record, only parseable
//! stamps and named attempts are read, and every checkpoint row is named.

use super::*;
use crate::envelope::EventType;
use crate::summary::test_support::{checkpoint, journal};
use crate::summary::Account;
use serde_json::json;

#[test]
fn a_checkpoint_reads_its_time_phase_cost_tokens_and_session() {
    let events = journal(&[
        (
            EventType::PhaseEntered,
            json!({"phase": "implement"}),
            "2026-01-01T00:00:00Z",
        ),
        checkpoint(
            Some("m"),
            json!({
                "step": "seat-turn",
                "total_cost_usd": 0.25,
                "input_tokens": 10,
                "output_tokens": 4,
                "cache_read_tokens": 3,
                "cache_write_tokens": 2,
                "reasoning_output_tokens": 1,
            }),
        ),
        // A turn reporting no cost and no tokens is still a metered row.
        checkpoint(Some("m"), json!({"step": "tool-use"})),
    ]);
    let read = checkpoints(&events);
    assert_eq!(read.len(), 2);
    assert_eq!(read[0].seq, 2);
    assert_eq!(read[0].attempt_id, "a");
    assert_eq!(read[0].member.as_deref(), Some("m"));
    assert_eq!(read[0].at, 1_767_225_602_000);
    assert_eq!(read[0].phase.as_deref(), Some("implement"));
    assert!(read[0].reported);
    assert_eq!(read[0].usd, Some(0.25));
    assert_eq!(read[0].meter.usage().unwrap().input_tokens, Some(10));
    assert_eq!(read[0].meter.usage().unwrap().output_tokens, Some(4));
    assert_eq!(read[0].meter.usage().unwrap().cache_read_tokens, Some(3));
    assert_eq!(read[0].meter.usage().unwrap().cache_write_tokens, Some(2));
    assert_eq!(
        read[0].meter.usage().unwrap().reasoning_output_tokens,
        Some(1)
    );
    assert!(!read[0].session);
    assert_eq!(read[1].seq, 3);
    assert!(!read[1].reported);
    assert_eq!(read[1].usd, None);
    assert_eq!(read[1].meter.usage(), None);
}

#[test]
fn only_the_last_session_record_of_a_site_keeps_the_flag() {
    let finished = json!({"step": "claude-lanetally-session-finished"});
    let events = journal(&[
        checkpoint(Some("m"), finished.clone()),
        checkpoint(Some("m"), json!({"step": "seat-turn"})),
        checkpoint(
            Some("m"),
            json!({"step": "claude-lanetally-session-finished", "session_id": "b"}),
        ),
        // Another site's finishing record leaves this site's last one alone.
        (
            EventType::EffectCheckpointed,
            json!({"attempt_id": "b", "checkpoint": {"member": "m",
                "step": "claude-lanetally-session-finished"}}),
            "2026-01-01T00:00:05Z",
        ),
    ]);
    let read = checkpoints(&events);
    assert_eq!(
        read.iter()
            .map(|checkpoint| checkpoint.session)
            .collect::<Vec<_>>(),
        [false, false, true, true]
    );
}

#[test]
fn a_checkpoint_reads_no_facts_where_its_stamp_or_attempt_does_not_read() {
    let cases = [
        (
            json!({"attempt_id": "a", "checkpoint": {"step": "seat-turn"}}),
            "not a stamp",
        ),
        (
            json!({"checkpoint": {"step": "seat-turn"}}),
            "2026-01-01T00:00:01Z",
        ),
    ];
    for (payload, at) in cases {
        let events = journal(&[(EventType::EffectCheckpointed, payload, at)]);
        assert_eq!(checkpoints(&events), []);
        let mut checking = Checking::default();
        for event in &events {
            checking.note(event);
        }
        assert_eq!(checking.rows(), [1]);
        assert_eq!(checking.checkpoint(1), None);
    }
}

#[test]
fn the_account_carries_every_checkpoint_it_reads() {
    let events = journal(&[
        checkpoint(Some("m"), json!({"step": "seat-turn"})),
        checkpoint(Some("m"), json!({"step": "tool-use"})),
    ]);
    let account = Account::whole("r", &events);
    assert_eq!(account.checkpoint(1).unwrap().seq, 1);
    assert_eq!(account.checkpoint(2).unwrap().seq, 2);
    assert_eq!(account.checkpoint(3), None);
    assert_eq!(account.checkpoint_rows(), [1, 2]);
    let mut checking = Checking::default();
    for event in &events {
        checking.note(event);
    }
    assert_eq!(checking.rows(), [1, 2]);
    assert_eq!(checking.checkpoint(1).unwrap().seq, 1);
}

/// The facts one event states are read with the phase the run was in, by
/// the same reading the whole walk makes; the writer's no-carry append
/// takes them from here (decision 0089 ruling 7).
#[test]
fn one_checkpoint_reads_the_facts_its_event_states() {
    let events = journal(&[
        checkpoint(
            Some("m"),
            json!({"step": "seat-turn", "total_cost_usd": 0.5, "input_tokens": 10}),
        ),
        checkpoint(Some("m"), json!({"step": "claude-session-finished"})),
    ]);
    let read = Checkpoint::read(&events[0], Some("implement")).unwrap();
    assert_eq!(read.seq, 1);
    assert_eq!(read.attempt_id, "a");
    assert_eq!(read.member.as_deref(), Some("m"));
    assert_eq!(read.at, 1_767_225_602_000);
    assert_eq!(read.phase.as_deref(), Some("implement"));
    assert!(read.reported);
    assert_eq!(read.usd, Some(0.5));
    assert_eq!(read.meter.usage().unwrap().input_tokens, Some(10));
    assert!(!read.session);
    assert!(Checkpoint::read(&events[1], None).unwrap().session);
    assert_eq!(
        Checkpoint::read(
            &crate::summary::test_support::journal(&[(
                EventType::EffectCheckpointed,
                json!({"checkpoint": {"step": "seat-turn"}}),
                "2026-01-01T00:00:01Z",
            )])[0],
            None,
        ),
        None
    );
}

/// A carried account keeps the phase and each site's last session record
/// and drops the checkpoint facts and row seqs it already read, so an
/// append's clone of it holds no earlier checkpoint (decision 0089 ruling
/// 9). The event the append seals is read again as it is carried.
#[test]
fn a_carried_account_drops_the_checkpoint_facts_it_already_read() {
    let events = journal(&[
        checkpoint(Some("m"), json!({"step": "seat-turn"})),
        checkpoint(Some("m"), json!({"step": "claude-session-finished"})),
        checkpoint(Some("m"), json!({"step": "seat-turn"})),
    ]);
    let carried = Account::whole("r", &events[..2]).carried();
    assert_eq!(carried.checkpoint_rows(), Vec::<u64>::new());
    assert_eq!(carried.checkpoint(1), None);
    let account = carried.onto(&events[2..]);
    assert_eq!(account.checkpoint_rows(), [3]);
    assert_eq!(account.checkpoint(3).unwrap().seq, 3);
    assert_eq!(account.checkpoint(1), None);
}

/// A checkpoint that is not a metered turn never repeats the site's last
/// message, and never becomes the message a later turn repeats: only a
/// turn is a message again (decision 0089 ruling 7).
#[test]
fn a_non_turn_checkpoint_never_repeats_a_message() {
    let events = journal(&[
        checkpoint(
            Some("m"),
            json!({"step": "seat-turn", "input_tokens": 10, "output_tokens": 4}),
        ),
        checkpoint(
            Some("m"),
            json!({"step": "tool-use", "input_tokens": 12, "output_tokens": 5}),
        ),
        checkpoint(
            Some("m"),
            json!({"step": "seat-turn", "input_tokens": 12, "output_tokens": 5}),
        ),
    ]);
    let read = checkpoints(&events);
    assert_eq!(
        read.iter()
            .map(|checkpoint| checkpoint.repeat)
            .collect::<Vec<_>>(),
        [Some(false), Some(false), Some(false)]
    );
}
