use super::*;
use crate::envelope_builder;
use crate::summary::Account;

#[path = "../../../../../tests/support/seat_rows.rs"]
mod journals;

use journals::at;

fn facts_of(events: &[EventEnvelope]) -> Vec<SeatFacts> {
    facts(Account::whole("run", events).seats())
}

/// A participant the journal states nothing of but that it was asked.
fn asked(effect_id: &str, member: Member) -> SeatFacts {
    SeatFacts {
        effect_id: effect_id.to_string(),
        member,
        model: None,
        effort: None,
        boundary: None,
        provider: None,
        usage: None,
        usage_aggregated: false,
        cost: None,
        cost_aggregated: false,
        last_attempt_cost: None,
        turns: None,
        turns_aggregated: false,
        items: None,
        transcript: None,
        outcome: Outcome::Working,
        started_at: None,
    }
}

fn named(member: &str) -> Member {
    Member::Named(member.to_string())
}

fn words(word: &str) -> Option<String> {
    Some(word.to_string())
}

fn tokens(input: u64, output: u64) -> TokenUsage {
    TokenUsage {
        input_tokens: Some(input),
        output_tokens: Some(output),
        total_tokens: Some(input + output),
        ..TokenUsage::default()
    }
}

/// A retried seat's cost is every attempt's, summed in journal order, and
/// its last attempt's beside it; its tokens count the message its retry
/// repeated once, and its Claude session settles their output; what it
/// last stated stands, and its boundary is its last attempt's.
#[test]
fn a_retried_seat_sums_its_attempts_and_reads_its_last() {
    assert_eq!(
        facts_of(&journals::retried()),
        [SeatFacts {
            model: words("opus-2"),
            effort: words("max"),
            boundary: words("harness"),
            provider: Some(words("openai")),
            usage: Some(tokens(150, 30)),
            cost: Some(0.30000000000000004),
            last_attempt_cost: Some(0.2),
            turns: Some(2),
            items: Some(2),
            transcript: Some(Transcript {
                kind: "claude-session".to_string(),
                locator: "s2".to_string(),
                home: "/h".to_string(),
            }),
            outcome: Outcome::Succeeded,
            started_at: Some(at(3)),
            ..asked("e1", Member::Seat)
        }]
    );
}

/// A first attempt recording after its retry started reads in journal
/// order: the retry's repeat of its message counts once, its late turn
/// after it counts, and its cost sums the three as they were journaled,
/// not attempt by attempt, which reads `0.6`.
#[test]
fn an_attempt_recording_after_its_retry_started_reads_in_journal_order() {
    assert_eq!(
        facts_of(&journals::interleaved()),
        [SeatFacts {
            usage: Some(tokens(300, 30)),
            cost: Some(0.6000000000000001),
            turns: Some(2),
            outcome: Outcome::Succeeded,
            started_at: Some(at(2)),
            ..asked("i1", Member::Seat)
        }]
    );
}

/// A retry pricing two records after its first attempt priced one sums
/// the three in journal order, not its own two first, which reads `0.6`;
/// its last session record's cost stands beside.
#[test]
fn a_retry_s_two_prices_sum_after_its_first_attempt_s() {
    assert_eq!(
        facts_of(&journals::repriced()),
        [SeatFacts {
            cost: Some(0.6000000000000001),
            last_attempt_cost: Some(0.3),
            items: Some(1),
            outcome: Outcome::Succeeded,
            started_at: Some(at(2)),
            ..asked("c1", Member::Seat)
        }]
    );
}

/// Turns that carry tokens meter a seat whose session record is no
/// Claude session's: the record's tokens are not counted again, a
/// repeated message counts once, and its items count beside its turns.
#[test]
fn a_seat_s_turns_meter_it_over_its_session_record() {
    assert_eq!(
        facts_of(&journals::turns()),
        [SeatFacts {
            usage: Some(TokenUsage {
                cache_read_tokens: Some(450),
                reasoning_output_tokens: Some(15),
                ..tokens(900, 90)
            }),
            cost: Some(1.5),
            last_attempt_cost: Some(1.5),
            turns: Some(3),
            items: Some(2),
            started_at: Some(at(2)),
            ..asked("e2", Member::Seat)
        }]
    );
}

/// A panel's seat reports its members' where it has none of its own,
/// summed or joined and flagged; each member keeps its own, and its own
/// finish pins its outcome only by a word the view lets stand.
#[test]
fn a_panel_seat_reports_its_members() {
    let member = |name: &str, provider: &str| SeatFacts {
        provider: Some(words(provider)),
        started_at: Some(at(if name == "a" { 4 } else { 5 })),
        ..asked("p1", named(name))
    };
    assert_eq!(
        facts_of(&journals::panel()),
        [
            SeatFacts {
                model: words("m-a, m-b"),
                effort: words("high, low"),
                boundary: words("harness, namespace"),
                usage: Some(tokens(30, 3)),
                usage_aggregated: true,
                cost: Some(0.75),
                cost_aggregated: true,
                last_attempt_cost: Some(0.75),
                turns: Some(5),
                turns_aggregated: true,
                outcome: Outcome::Succeeded,
                started_at: Some(at(3)),
                ..asked("p1", Member::Seat)
            },
            SeatFacts {
                model: words("m-a"),
                effort: words("low"),
                boundary: words("namespace"),
                usage: Some(tokens(10, 1)),
                cost: Some(0.5),
                last_attempt_cost: Some(0.5),
                turns: Some(3),
                outcome: Outcome::Failed,
                ..member("a", "anthropic")
            },
            SeatFacts {
                model: words("m-b"),
                effort: words("high"),
                boundary: words("harness"),
                usage: Some(tokens(20, 2)),
                cost: Some(0.25),
                last_attempt_cost: Some(0.25),
                turns: Some(2),
                outcome: Outcome::Succeeded,
                ..member("b", "openai")
            },
            asked("p2", Member::Seat),
        ]
    );
}

/// While a retry is in flight its seat is at work, though its first
/// attempt failed, and reads its first attempt's cost and session record
/// until the retry reports its own.
#[test]
fn a_seat_retrying_is_at_work() {
    assert_eq!(
        facts_of(&journals::retrying()),
        [SeatFacts {
            model: words("opus"),
            effort: words("high"),
            provider: Some(words("openai")),
            usage: Some(tokens(100, 40)),
            cost: Some(0.1),
            last_attempt_cost: Some(0.1),
            turns: Some(1),
            items: Some(1),
            transcript: Some(Transcript {
                kind: "claude-session".to_string(),
                locator: "s1".to_string(),
                home: "/h".to_string(),
            }),
            started_at: Some(at(3)),
            ..asked("e1", Member::Seat)
        }]
    );
}

/// A start that names no attempt leaves none in flight: the failure
/// before it stands.
#[test]
fn a_start_naming_no_attempt_leaves_the_last_terminal_standing() {
    assert_eq!(
        facts_of(&journals::restarted()),
        [SeatFacts {
            outcome: Outcome::Failed,
            started_at: Some(at(2)),
            ..asked("r1", Member::Seat)
        }]
    );
}

/// A start naming an attempt that failed re-opens it: the seat is at work
/// again, dated by its first start.
#[test]
fn a_start_naming_a_concluded_attempt_re_opens_it() {
    assert_eq!(
        facts_of(&journals::reopened()),
        [SeatFacts {
            started_at: Some(at(2)),
            ..asked("o1", Member::Seat)
        }]
    );
}

/// A provenance entry naming no provider restates it as named by none:
/// the provider an earlier attempt's entry named does not stand.
#[test]
fn a_provenance_entry_naming_no_provider_replaces_the_last() {
    assert_eq!(
        facts_of(&journals::unprovided()),
        [SeatFacts {
            provider: Some(None),
            started_at: Some(at(2)),
            ..asked("v1", Member::Seat)
        }]
    );
}

/// A request no attempt started is a participant with nothing to report:
/// at work until a terminal naming no attempt concludes it.
#[test]
fn a_request_never_started_is_a_participant() {
    assert_eq!(
        facts_of(&journals::unstarted()),
        [
            SeatFacts {
                outcome: Outcome::Indeterminate,
                ..asked("u1", Member::Seat)
            },
            asked("u2", Member::Seat),
        ]
    );
}

/// Records that name no attempt are the request's own row's, read while
/// no attempt is in flight; a stale attempt's terminal is passed over;
/// and a request with no effect, a record of an effect never asked and an
/// event of another kind add no row.
#[test]
fn records_naming_no_attempt_land_on_the_request_s_row() {
    let account = Account::whole("run", &journals::unnamed());
    assert_eq!(
        facts(account.seats()),
        [SeatFacts {
            model: words("m2"),
            boundary: words("container"),
            cost: Some(0.125),
            turns: Some(1),
            outcome: Outcome::Succeeded,
            started_at: Some(at(4)),
            ..asked("s1", Member::Seat)
        }]
    );
    assert_eq!(account.seats().len(), 1);
}

/// Each row is one participant of one attempt, opened in journal order,
/// keyed by its request, attempt and member, with the seat it hired, the
/// run's phase when its attempt started, that start, and its first and
/// last checkpoint.
#[test]
fn a_row_is_keyed_by_attempt_and_member_and_dated_by_its_checkpoints() {
    let account = Account::whole("run", &journals::retried());
    let place = |stamp: &Option<Stamp>| stamp.as_ref().map(|stamp| (stamp.seq, stamp.at.clone()));
    let rows: Vec<_> = account
        .seats()
        .iter()
        .map(|row| {
            (
                (
                    row.request_seq,
                    row.effect_id.as_str(),
                    row.attempt_id.as_deref(),
                ),
                (&row.member, row.seat.as_deref(), row.phase.as_deref()),
                [place(&row.started), place(&row.first), place(&row.last)],
            )
        })
        .collect();
    let stamp = |seq: u64| Some((seq, at(seq)));
    let seat = (&Member::Seat, Some("implementer"), Some("implement"));
    assert_eq!(
        rows,
        [
            (
                (2, "e1", None),
                (&Member::Seat, Some("implementer"), None),
                [None, None, None]
            ),
            (
                (2, "e1", Some("first")),
                seat,
                [stamp(3), stamp(4), stamp(12)]
            ),
            (
                (2, "e1", Some("again")),
                seat,
                [stamp(8), stamp(9), stamp(13)]
            ),
        ]
    );
}

/// A member's row of an attempt takes the attempt's start and phase
/// whether it opened at the start or at its first checkpoint, and a start
/// dates the rows of its attempt opened before it.
#[test]
fn a_member_s_row_takes_its_attempt_s_start() {
    let account = Account::whole("run", &journals::panel());
    let started: Vec<_> = account
        .seats()
        .iter()
        .map(|row| {
            (
                &row.member,
                row.phase.as_deref(),
                row.started.as_ref().map(|start| start.seq),
            )
        })
        .collect();
    let review = Some("review");
    assert_eq!(
        started,
        [
            (&Member::Seat, None, None),
            (&Member::Seat, review, Some(3)),
            (&named("a"), review, Some(3)),
            (&named("b"), review, Some(3)),
            (&Member::Seat, None, None),
        ]
    );
    let early = journals::journal(vec![
        (
            EventType::EffectRequested,
            serde_json::json!({"effect_id": "p1", "seat": "panel"}),
        ),
        journals::checkpoint("p1", "c1", serde_json::json!({"member": "a"})),
        (
            EventType::EffectStarted,
            serde_json::json!({"effect_id": "p1", "attempt_id": "c1"}),
        ),
    ]);
    let account = Account::whole("run", &early);
    let started: Vec<_> = account
        .seats()
        .iter()
        .map(|row| (&row.member, row.started.as_ref().map(|start| start.seq)))
        .collect();
    assert_eq!(
        started,
        [
            (&Member::Seat, None),
            (&named("a"), Some(3)),
            (&Member::Seat, Some(3))
        ]
    );
}

/// A run's rows are its account's however its events are carried: each
/// split of each journal reads as the journal read whole.
#[test]
fn rows_carried_over_any_split_are_the_rows_read_whole() {
    let committed = crate::committed_journals::committed_journals();
    let committed = committed.into_iter().map(|(path, events)| {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        (name, events)
    });
    let hand_built = journals::journals().into_iter();
    for (name, events) in hand_built
        .map(|(name, events)| (name.to_string(), events))
        .chain(committed)
    {
        let whole = Account::whole("run", &events);
        // Every split of a hand-built journal; a committed one's every
        // seventh, which still falls inside each of its seats.
        let step = if events.len() > 20 { 7 } else { 1 };
        for split in (0..=events.len()).step_by(step) {
            let (head, tail) = events.split_at(split);
            let carried = Account::whole("run", head).onto(tail);
            assert_eq!(carried.seats(), whole.seats(), "{name} split at {split}");
        }
    }
}

/// The words a panel member's finish pins its outcome by, and no other.
#[test]
fn a_member_s_finish_pins_only_the_view_s_words() {
    let pinned = ["working", "succeeded", "failed", "indeterminate", "pass"].map(Outcome::named);
    assert_eq!(
        pinned,
        [
            Some(Outcome::Working),
            Some(Outcome::Succeeded),
            Some(Outcome::Failed),
            Some(Outcome::Indeterminate),
            None,
        ]
    );
}
