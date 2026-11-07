use super::*;
use crate::envelope_builder;
use crate::summary::Account;
use serde_json::json;

#[path = "../../../../../tests/support/seat_rows.rs"]
pub(super) mod journals;
#[path = "../../../../../tests/support/seat_steps.rs"]
pub(super) mod seat_steps;

use seat_steps::at;

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
        carried_from: None,
        started_at: None,
        last_at: None,
        turn: None,
        tool: None,
        target: None,
        finish: None,
        session_id: None,
    }
}

fn named(member: &str) -> Member {
    Member::Named(member.to_string())
}

fn words(word: &str) -> Option<String> {
    Some(word.to_string())
}

/// A finish at `seq`'s stamp, its outcome shown as `outcome`.
fn finish(seq: u64, outcome: &str) -> Option<Finish> {
    Some(Finish {
        at: at(seq),
        outcome: words(outcome),
    })
}

fn provider(name: &str) -> Option<Provider> {
    Some(Provider::Named(name.to_string()))
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
            provider: provider("openai"),
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
            last_at: Some(at(13)),
            turn: words("2"),
            tool: words("undefined"),
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
            last_at: Some(at(6)),
            turn: words("2"),
            tool: words("undefined"),
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
            last_at: Some(at(7)),
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
            last_at: Some(at(10)),
            ..asked("e2", Member::Seat)
        }]
    );
}

/// A panel's seat reports its members' where it has none of its own,
/// summed or joined and flagged; each member keeps its own, and its own
/// finish pins its outcome only by a word the view lets stand.
#[test]
fn a_panel_seat_reports_its_members() {
    let member = |name: &str, named_by: &str| SeatFacts {
        provider: provider(named_by),
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
                last_at: Some(at(8)),
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
                last_at: Some(at(11)),
                turn: words("undefined"),
                tool: words("Read"),
                finish: finish(9, "failed"),
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
                last_at: Some(at(10)),
                finish: finish(10, "unheard-of"),
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
            provider: provider("openai"),
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
            last_at: Some(at(9)),
            turn: words("1"),
            tool: words("Edit"),
            target: words("crates/brokkr-core/src/seat/row/said.rs"),
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
            provider: Some(Provider::Unnamed),
            started_at: Some(at(2)),
            ..asked("v1", Member::Seat)
        }]
    );
}

/// A member its effect's last start carries is a participant concluded
/// as carried, naming its source's attempt and that attempt's place among
/// its phase's starts, none past a start naming no effect; a member only
/// an earlier start carried is at work with its effect.
#[test]
fn a_carried_member_names_where_its_verdict_settled() {
    let carried = |member: &str, from: Option<(&str, usize)>| SeatFacts {
        outcome: Outcome::Carried,
        carried_from: from.map(|(attempt_id, ordinal)| CarriedFrom {
            attempt_id: attempt_id.to_string(),
            ordinal,
        }),
        ..asked("c1", named(member))
    };
    assert_eq!(
        facts_of(&journals::carried()),
        [
            SeatFacts {
                started_at: Some(at(2)),
                ..asked("x1", Member::Seat)
            },
            SeatFacts {
                outcome: Outcome::Indeterminate,
                started_at: Some(at(4)),
                ..asked("s1", Member::Seat)
            },
            SeatFacts {
                provider: provider("test"),
                outcome: Outcome::Indeterminate,
                ..asked("s1", named("a"))
            },
            SeatFacts {
                started_at: Some(at(7)),
                ..asked("c1", Member::Seat)
            },
            carried("a", Some(("two", 2))),
            asked("c1", named("c")),
            carried("b", None),
        ]
    );
}

/// A carried verdict's source is named over the whole fold: an attempt
/// first started after the carrying start, and a start counted in the
/// phase by a request after every start.
#[test]
fn a_carried_source_named_later_is_named() {
    let carried = |member: &str, attempt_id: &str, ordinal| SeatFacts {
        outcome: Outcome::Carried,
        carried_from: Some(CarriedFrom {
            attempt_id: attempt_id.to_string(),
            ordinal,
        }),
        ..asked("e1", named(member))
    };
    assert_eq!(
        facts_of(&journals::later()),
        [
            SeatFacts {
                outcome: Outcome::Indeterminate,
                started_at: Some(at(3)),
                ..asked("e1", Member::Seat)
            },
            carried("a", "a2", 3),
            carried("b", "a0", 1),
            SeatFacts {
                outcome: Outcome::Failed,
                started_at: Some(at(6)),
                ..asked("e2", Member::Seat)
            },
            asked("e0", Member::Seat),
        ]
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

/// An effect asked twice is one participant, as the view keys it by its
/// effect and member: read by the request it was first seen under, at
/// work while that request's attempt is, its cost summed in journal order
/// across both requests; a member first seen under the second request is
/// that request's, and no seat sums it.
#[test]
fn an_effect_asked_twice_is_one_participant_read_by_its_first_request() {
    use serde_json::json;
    let asked_in = |phase: &str| {
        let payload = json!({"effect_id": "e", "seat": "s", "phase": phase});
        (EventType::EffectRequested, payload)
    };
    let started = |attempt: &str| {
        let payload = json!({"effect_id": "e", "attempt_id": attempt});
        (EventType::EffectStarted, payload)
    };
    let session =
        |cost: f64| json!({"step": "claude-code-session-finished", "total_cost_usd": cost});
    let turn = json!({"member": "m", "step": "seat-turn", "turn": 2, "tool": "Read"});
    let done = json!({"effect_id": "e", "attempt_id": "b"});
    let events = journals::journal(vec![
        asked_in("work"),
        started("a"),
        journals::checkpoint("e", "a", session(1.0)),
        asked_in("check"),
        started("b"),
        journals::checkpoint("e", "b", session(0.5)),
        journals::checkpoint("e", "b", turn),
        (EventType::EffectSucceeded, done),
    ]);
    assert_eq!(
        facts_of(&events),
        [
            SeatFacts {
                cost: Some(1.5),
                last_attempt_cost: Some(0.5),
                started_at: Some(at(2)),
                last_at: Some(at(6)),
                ..asked("e", Member::Seat)
            },
            SeatFacts {
                turns: Some(2),
                outcome: Outcome::Succeeded,
                started_at: Some(at(7)),
                last_at: Some(at(7)),
                turn: words("2"),
                tool: words("Read"),
                ..asked("e", named("m"))
            },
        ]
    );
}

/// A member an effect asked again carries names its source counted in the
/// phase of the request it was first seen under, `x`, where one start is
/// counted, and not in the carrying request's, `y`, where none is.
#[test]
fn a_carried_source_counts_in_the_phase_its_member_was_first_seen_in() {
    let found = facts_of(&journals::carried_again());
    let member = found.iter().find(|facts| facts.member == named("m"));
    let carried = member.map(|member| (member.outcome, member.carried_from.clone()));
    let from = CarriedFrom {
        attempt_id: "b".to_string(),
        ordinal: 1,
    };
    assert_eq!(carried, Some((Outcome::Carried, Some(from))));
}

/// The request a participant was first seen under: the first of its
/// effect's requests holding a row of its member, though the effect was
/// asked before it, another effect's row of that member passed over.
#[test]
fn a_participant_s_home_is_its_effect_s_first_request_holding_its_member() {
    use serde_json::json;
    let asked_for = |effect: &str| (EventType::EffectRequested, json!({"effect_id": effect}));
    let started = |effect: &str| {
        let provenance = json!([{"member": "m", "provider": "p"}]);
        let payload = json!({"effect_id": effect, "attempt_id": "a", "provenance": provenance});
        (EventType::EffectStarted, payload)
    };
    let events = journals::journal(vec![
        asked_for("e0"),
        started("e0"),
        asked_for("e1"),
        asked_for("e1"),
        started("e1"),
        asked_for("e2"),
        asked_for("e1"),
    ]);
    let mut seats = Seats::default();
    for event in &events {
        seats.note(event);
    }
    assert_eq!(seats.home(4, &named("m")), 2);
    assert_eq!(seats.home(4, &Member::Seat), 1);
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
            last_at: Some(at(5)),
            turn: words("1"),
            tool: words("undefined"),
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

/// A member's row opened after its attempt started takes that start as
/// its last start, as its seat's row holds it.
#[test]
fn a_member_s_row_takes_its_attempt_s_last_start() {
    let account = Account::whole("run", &journals::panel());
    let since: Vec<_> = account
        .seats()
        .iter()
        .map(|row| (&row.member, row.since))
        .collect();
    assert_eq!(
        since,
        [
            (&Member::Seat, None),
            (&Member::Seat, Some(3)),
            (&named("a"), Some(3)),
            (&named("b"), Some(3)),
            (&Member::Seat, None),
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
            let kept = (carried.seats_kept(), whole.seats_kept());
            assert_eq!(kept.0, kept.1, "{name} split at {split}");
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

/// A start naming no attempt opens no site: its provider is its
/// participant row's, as it always was, and the site reading opens
/// nothing for it (decision 0089 ruling 7).
#[test]
fn a_start_naming_no_attempt_opens_no_site() {
    let run = |provenance: serde_json::Value| {
        let events = journals::journal(vec![
            (
                EventType::EffectRequested,
                serde_json::json!({"effect_id": "x", "seat": "s"}),
            ),
            (
                EventType::EffectStarted,
                serde_json::json!({"effect_id": "x", "provenance": provenance}),
            ),
        ]);
        Account::whole("run", &events)
    };
    let account = run(serde_json::json!([{"member": null, "provider": "anthropic"}]));
    assert_eq!(account.sites(), []);
    assert_eq!(account.seats().len(), 1);
    assert_eq!(
        account.seats()[0]
            .provider
            .as_ref()
            .map(|(_, provider)| provider),
        Some(&Provider::Named("anthropic".to_string()))
    );
    // A start naming neither an attempt nor a provider leaves the
    // participant's provider none, and opens no site either.
    let account = run(serde_json::json!([{"member": null}]));
    assert_eq!(account.sites(), []);
    assert_eq!(
        account.seats()[0]
            .provider
            .as_ref()
            .map(|(_, provider)| provider),
        Some(&Provider::Unnamed)
    );
}

/// A member's tag keys its site's row: the seat's own is none, a named
/// member's is its name.
#[test]
fn a_member_s_tag_keys_its_row() {
    assert_eq!(Member::Seat.tag(), None);
    assert_eq!(Member::Named("m".to_string()).tag(), Some("m"));
}

/// A site's row keeps the rule brokkr-core's reading gives its site, not
/// its own participant's turns: the row follows the reading as each
/// checkpoint lands (decision 0089 ruling 7).
#[test]
fn a_site_row_s_metering_is_the_site_reading_s() {
    let run = |steps: Vec<(EventType, serde_json::Value)>| {
        let mut events = vec![
            (
                EventType::EffectRequested,
                serde_json::json!({"effect_id": "x", "seat": "s"}),
            ),
            (
                EventType::EffectStarted,
                serde_json::json!({"effect_id": "x", "attempt_id": "a",
                    "provenance": [{"member": null, "provider": "anthropic"}]}),
            ),
        ];
        events.extend(steps);
        Account::whole("run", &journals::journal(events))
    };
    let turn = |record: serde_json::Value| {
        (
            EventType::EffectCheckpointed,
            serde_json::json!({"effect_id": "x", "attempt_id": "a", "checkpoint": record}),
        )
    };
    let account = run(vec![turn(
        serde_json::json!({"step": "seat-turn", "input_tokens": 10}),
    )]);
    let row = account
        .seats()
        .iter()
        .find(|row| row.site.is_some())
        .unwrap();
    assert_eq!(row.metering, Metering::Turns);
    let account = run(vec![
        turn(serde_json::json!({"step": "seat-turn", "input_tokens": 10})),
        turn(serde_json::json!({"step": "claude-session-finished", "output_tokens": 3})),
        turn(serde_json::json!({"step": "session-started", "session_id": "s"})),
    ]);
    let row = account
        .seats()
        .iter()
        .find(|row| row.site.is_some())
        .unwrap();
    assert_eq!(row.metering, Metering::Settled);
}

/// A start naming an attempt and member already started opens a second
/// site row: each keeps its own start's facts and the site reading's rule,
/// and the two rows are two sites, not one (decision 0089 ruling 7).
#[test]
fn a_reopened_attempt_opens_two_site_rows() {
    let started = || {
        (
            EventType::EffectStarted,
            serde_json::json!({"effect_id": "x", "attempt_id": "a",
                "provenance": [{"member": null, "provider": "anthropic"}]}),
        )
    };
    let turn = |record: serde_json::Value| {
        (
            EventType::EffectCheckpointed,
            serde_json::json!({"effect_id": "x", "attempt_id": "a", "checkpoint": record}),
        )
    };
    let events = journals::journal(vec![
        (
            EventType::EffectRequested,
            serde_json::json!({"effect_id": "x", "seat": "s"}),
        ),
        started(),
        turn(serde_json::json!({"step": "seat-turn"})),
        (
            EventType::EffectFailed,
            serde_json::json!({"effect_id": "x", "attempt_id": "a"}),
        ),
        started(),
        turn(serde_json::json!({"step": "seat-turn", "input_tokens": 10})),
    ]);
    let account = Account::whole("run", &events);
    let rows: Vec<&SeatRow> = account
        .seats()
        .iter()
        .filter(|row| row.site.is_some())
        .collect();
    assert_eq!(rows.len(), 2);
    assert_eq!(
        rows.iter().map(|row| row.site).collect::<Vec<_>>(),
        [Some(0), Some(1)]
    );
    assert_eq!(
        rows.iter().map(|row| row.metering).collect::<Vec<_>>(),
        [Metering::Session, Metering::Turns]
    );
}

/// A journal step: its event, payload and stamp.
type Step = (EventType, serde_json::Value, &'static str);

/// The run's `work` phase, entered at its first second.
fn work() -> Step {
    (
        EventType::PhaseEntered,
        json!({"phase": "work"}),
        "2026-01-01T00:00:00Z",
    )
}

/// Attempt `a` of `fx` started under `provenance` at `at`.
fn starts(provenance: serde_json::Value, at: &'static str) -> Step {
    (
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a", "provenance": provenance}),
        at,
    )
}

/// `fx` asked by the `worker` in the `work` phase at `at`.
fn asks(at: &'static str) -> Step {
    (
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
        at,
    )
}

/// A checkpoint of `fx`'s attempt `a` recording `record` at `at`.
fn records(record: serde_json::Value, at: &'static str) -> Step {
    (
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a", "checkpoint": record}),
        at,
    )
}

/// The first records of a run whose checkpoint lands before its effect's
/// request: `work` entered, attempt `a` started under member `m`'s
/// anthropic provider, `record` at the second second, and the request at
/// the third.
fn early_records(record: serde_json::Value) -> Vec<Step> {
    early_run(records(record, "2026-01-01T00:00:02Z"))
}

/// `record` before its effect's request, after the same `work` phase and
/// provider-bearing start, and the request at the third second.
fn early_run(record: Step) -> Vec<Step> {
    vec![
        work(),
        starts(
            json!([{"member": "m", "provider": "anthropic"}]),
            "2026-01-01T00:00:01Z",
        ),
        record,
        asks("2026-01-01T00:00:03Z"),
    ]
}

/// A site's facts as the kept rows and the whole reading state them: its
/// attempt and member, its start in milliseconds, its phase, provider,
/// model and metering rule.
type SiteKey = (
    String,
    Option<String>,
    i64,
    Option<String>,
    String,
    Option<String>,
    Metering,
);

/// The facts [`SiteKey`] names of `site`.
fn site_facts(site: &crate::summary::Site) -> SiteKey {
    (
        site.attempt_id.clone(),
        site.member.clone(),
        site.at,
        site.phase.clone(),
        site.provider.clone(),
        site.model.clone(),
        site.metering,
    )
}

/// `account`'s one site against the whole reading of `events`, and its
/// facts against `expected`.
fn assert_site(account: &Account, events: &[EventEnvelope], expected: SiteKey) {
    let read = crate::summary::sites(events);
    let kept = account.sites();
    assert_eq!(kept.len(), 1);
    assert_eq!(site_facts(&kept[0]), site_facts(&read[0]));
    assert_eq!(site_facts(&kept[0]), expected);
}

/// The one site row a request opened.
fn the_site(account: &Account) -> &SeatRow {
    account
        .seats()
        .iter()
        .find(|row| row.site.is_some())
        .expect("a request opens the site its start named")
}

/// The facts of `member` among `account`'s participants.
fn the_member(account: &Account, member: &str) -> SeatFacts {
    facts(account.seats())
        .into_iter()
        .find(|facts| facts.member == named(member))
        .expect("the member's facts")
}

/// A provider-bearing start that lands before its effect is asked is
/// reconciled when the request lands, as main's walk reads it: its site
/// row opens, with its own provider.
#[test]
fn a_start_before_its_request_keeps_its_site_row() {
    use crate::summary::test_support::journal;
    let events = journal(&[
        work(),
        starts(
            json!([{"member": "m", "provider": "anthropic"}]),
            "2026-01-01T00:00:01Z",
        ),
        asks("2026-01-01T00:00:02Z"),
        records(
            json!({"member": "m", "step": "seat-turn"}),
            "2026-01-01T00:00:03Z",
        ),
    ]);
    let account = Account::whole("run", &events);
    let site = the_site(&account);
    assert_eq!(site.member, Member::Named("m".to_string()));
    assert_eq!(
        site.provider,
        Some((2, Provider::Named("anthropic".to_string())))
    );
}

/// A checkpoint that lands after its attempt's start but before the
/// effect's request still dates the site the request opens: the site keeps
/// the model and metering rule its checkpoint read, and the participant its
/// tokens and turn (review 753s4s3, M1). The record's order no kept row
/// represents exactly, so the head is withheld and the run is read whole
/// (decision 0089 ruling 8).
#[test]
fn a_checkpoint_before_its_request_keeps_its_site_facts() {
    use crate::summary::test_support::journal;
    let events = journal(&early_records(
        json!({"member": "m", "step": "seat-turn", "model": "opus", "input_tokens": 10}),
    ));
    let account = Account::whole("run", &events);
    let site = the_site(&account);
    assert_eq!(site.site_model.as_deref(), Some("opus"));
    assert_eq!(site.metering, Metering::Turns);
    let member = the_member(&account, "m");
    assert_eq!(member.model.as_deref(), Some("opus"));
    assert_eq!(
        member.usage,
        Some(TokenUsage {
            input_tokens: Some(10),
            total_tokens: Some(10),
            ..TokenUsage::default()
        })
    );
    assert_eq!(member.provider, provider("anthropic"));
    assert!(
        !account.seats_kept(),
        "a record before its request keeps no seat head"
    );
}

/// A terminal of a start whose effect was never asked cannot make its rows
/// current: the site waits for the request that would replay both, so the
/// writer keeps no seat head and a reader falls back to the run whole
/// (review 753s4s3, M2). The request, once it lands, replays the terminal
/// too, so the site is represented and concluded, but the record's order
/// still keeps no head (decision 0089 ruling 8).
#[test]
fn a_terminated_unrequested_start_keeps_its_rows_unkept() {
    use crate::summary::test_support::journal;
    let start = starts(
        json!([{"member": "m", "provider": "anthropic"}]),
        "2026-01-01T00:00:01Z",
    );
    let failed = (
        EventType::EffectFailed,
        json!({"effect_id": "fx", "attempt_id": "a"}),
        "2026-01-01T00:00:02Z",
    );
    let before = journal(&[work(), start.clone(), failed.clone()]);
    let account = Account::whole("run", &before);
    assert_eq!(
        account
            .seats()
            .iter()
            .filter(|row| row.site.is_some())
            .count(),
        0
    );
    assert_eq!(crate::summary::sites(&before).len(), 1);
    assert!(
        !account.seats_kept(),
        "a site no request replayed is not represented as current"
    );
    let after = journal(&[work(), start, failed, asks("2026-01-01T00:00:03Z")]);
    let account = Account::whole("run", &after);
    assert!(
        !account.seats_kept(),
        "a terminal before its request keeps no seat head"
    );
    assert_eq!(the_member(&account, "m").outcome, Outcome::Failed);
}

/// A run whose checkpoint lands before its effect's request keeps the
/// site's facts — its start's milliseconds, its keys, its provider, model
/// and metering rule — but no seat head: the checkpoint's order is no
/// kept row's, so every participant readout reads the run whole (review
/// 753s4s3, M1; decision 0089 rulings 4, 7 and 8).
#[test]
fn a_checkpoint_before_and_after_its_request_keeps_site_facts_no_head() {
    use crate::summary::test_support::journal;
    let mut steps = early_records(
        json!({"member": "m", "step": "seat-turn", "turn": 1, "tool": "Read",
               "model": "opus", "input_tokens": 10}),
    );
    steps.push(records(
        json!({"member": "m", "step": "seat-turn", "turn": 2, "tool": "Write"}),
        "2026-01-01T00:00:04Z",
    ));
    let events = journal(&steps);
    let account = Account::whole("run", &events);
    assert!(
        !account.seats_kept(),
        "the checkpoint landed before its request"
    );
    assert_site(
        &account,
        &events,
        (
            "a".to_string(),
            Some("m".to_string()),
            1_767_225_601_000,
            Some("work".to_string()),
            "anthropic".to_string(),
            Some("opus".to_string()),
            Metering::Turns,
        ),
    );
    // The head is withheld for the checkpoint alone, with no start before
    // it: the condition is the record's landing, not the start's.
    let only_checkpoint = journal(&[
        work(),
        records(
            json!({"member": "m", "step": "seat-turn", "input_tokens": 10}),
            "2026-01-01T00:00:02Z",
        ),
        asks("2026-01-01T00:00:03Z"),
    ]);
    assert!(!Account::whole("run", &only_checkpoint).seats_kept());
    // A record that is no participant's — a transition, even one naming
    // the effect — keeps the head: only a start, checkpoint or terminal
    // withholds it.
    let other = journal(&[
        (
            EventType::TransitionDecided,
            json!({"effect_id": "fx"}),
            "2026-01-01T00:00:00Z",
        ),
        asks("2026-01-01T00:00:01Z"),
    ]);
    assert!(Account::whole("run", &other).seats_kept());
}

/// A run whose terminal lands before its effect's request keeps the site's
/// facts — its start's milliseconds, its keys and its metering rule — but
/// no seat head: the terminal's order is no kept row's, so every
/// participant readout reads the run whole (review 753s4s3, M2; decision
/// 0089 rulings 4, 7 and 8).
#[test]
fn a_terminal_before_its_request_keeps_site_facts_no_head() {
    use crate::summary::test_support::journal;
    let failed = || {
        (
            EventType::EffectFailed,
            json!({"effect_id": "fx", "attempt_id": "a"}),
            "2026-01-01T00:00:02Z",
        )
    };
    let events = journal(&early_run(failed()));
    let account = Account::whole("run", &events);
    assert!(
        !account.seats_kept(),
        "the terminal landed before its request"
    );
    assert_site(
        &account,
        &events,
        (
            "a".to_string(),
            Some("m".to_string()),
            1_767_225_601_000,
            Some("work".to_string()),
            "anthropic".to_string(),
            None,
            Metering::Session,
        ),
    );
    // The head is withheld for the terminal alone, with no start before it.
    let only_terminal = journal(&[work(), failed(), asks("2026-01-01T00:00:03Z")]);
    assert!(!Account::whole("run", &only_terminal).seats_kept());
}

/// A providerless start before its request still updates its participant:
/// the later start states the participant's provider as none, while the
/// earlier site keeps the provider key its own start set (review 753s4s3,
/// M3).
#[test]
fn a_providerless_early_restart_updates_the_participant() {
    use crate::summary::test_support::journal;
    let events = journal(&[
        work(),
        starts(
            json!([{"member": "m", "provider": "anthropic"}]),
            "2026-01-01T00:00:01Z",
        ),
        starts(
            json!([{"member": "m", "provider": null}]),
            "2026-01-01T00:00:02Z",
        ),
        asks("2026-01-01T00:00:03Z"),
    ]);
    let account = Account::whole("run", &events);
    assert_eq!(
        the_site(&account).site_provider.as_deref(),
        Some("anthropic")
    );
    assert_eq!(the_member(&account, "m").provider, Some(Provider::Unnamed));
}

/// A record that is not a start is never deferred as one, whatever its
/// payload names: the request that asks its effect opens no site for it.
#[test]
fn a_record_that_is_not_a_start_is_not_deferred() {
    use crate::summary::test_support::journal;
    let events = journal(&[
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "provenance": [{"member": "m", "provider": "anthropic"}],
                   "checkpoint": {"member": "m", "step": "seat-turn"}}),
            "2026-01-01T00:00:00Z",
        ),
        (
            EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
            "2026-01-01T00:00:01Z",
        ),
    ]);
    let account = Account::whole("run", &events);
    assert_eq!(
        account
            .seats()
            .iter()
            .filter(|row| row.site.is_some())
            .count(),
        0
    );
}

/// A start whose stamp does not parse states no provider on its rows, as
/// brokkr-core's site reading ignores it.
#[test]
fn an_unreadable_start_states_no_provider() {
    use crate::summary::test_support::journal;
    let events = journal(&[
        (
            EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
            "2026-01-01T00:00:00Z",
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "provenance": [{"member": "m", "provider": "anthropic"}]}),
            "not a stamp",
        ),
    ]);
    let read = facts(Account::whole("run", &events).seats());
    assert_eq!(read.len(), 1);
    assert_eq!(read[0].provider, None);
}

/// A re-opened participant's later session record moves the earlier site's
/// rule: the carried account's site and seat row read as the run read whole
/// (decision 0089 ruling 7).
#[test]
fn a_reopened_session_moves_the_earlier_site_s_rule() {
    use crate::summary::test_support::journal;
    let steps = journal(&[
        (
            EventType::PhaseEntered,
            json!({"phase": "work"}),
            "2026-01-01T00:00:00Z",
        ),
        (
            EventType::EffectRequested,
            json!({"effect_id": "fx", "seat": "worker", "phase": "work"}),
            "2026-01-01T00:00:01Z",
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "provenance": [{"member": "m", "provider": "anthropic"}]}),
            "2026-01-01T00:00:02Z",
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "checkpoint": {"member": "m", "step": "seat-turn", "input_tokens": 10}}),
            "2026-01-01T00:00:02Z",
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "checkpoint": {"member": "m", "step": "claude-session-finished",
                                  "output_tokens": 3}}),
            "2026-01-01T00:00:03Z",
        ),
        (
            EventType::EffectFailed,
            json!({"effect_id": "fx", "attempt_id": "a"}),
            "2026-01-01T00:00:04Z",
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "provenance": [{"member": "m", "provider": "anthropic"}]}),
            "2026-01-01T00:00:05Z",
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": "fx", "attempt_id": "a",
                   "checkpoint": {"member": "m", "step": "claude-session-finished",
                                  "output_tokens": 4}}),
            "2026-01-01T00:00:06Z",
        ),
    ]);
    let whole = Account::whole("run", &steps);
    let mut carried = Account::empty("run");
    for event in &steps {
        carried = carried.onto(std::slice::from_ref(event));
    }
    let rules = |account: &Account| -> Vec<Metering> {
        account.sites().iter().map(|site| site.metering).collect()
    };
    assert_eq!(rules(&carried), rules(&whole));
    assert_eq!(rules(&whole), [Metering::Turns, Metering::Session]);
    let rows = |account: &Account| -> Vec<(Option<u64>, Metering)> {
        account
            .seats()
            .iter()
            .filter(|row| row.site.is_some())
            .map(|row| (row.site, row.metering))
            .collect()
    };
    assert_eq!(rows(&carried), rows(&whole));
    assert_eq!(rows(&whole)[0].1, Metering::Turns);
}
