use super::meter::{once, sum_usage, Meter, Metering};
use super::*;
use serde_json::json;

/// A usage's six figures in its fields' order: input, output, cache
/// read, cache write, reasoning output, total.
type Figures = [Option<u64>; 6];

fn figures(usage: Option<TokenUsage>) -> Option<Figures> {
    usage.map(|usage| {
        [
            usage.input_tokens,
            usage.output_tokens,
            usage.cache_read_tokens,
            usage.cache_write_tokens,
            usage.reasoning_output_tokens,
            usage.total_tokens,
        ]
    })
}

/// A checkpoint of `step` carrying the token keys of `tokens`, read once.
fn checkpoint(step: &str, mut tokens: Value) -> Meter {
    tokens["step"] = json!(step);
    Meter::of(&tokens)
}

fn reading(by: Metering) -> &'static str {
    match by {
        Metering::Turns => "turns",
        Metering::Settled => "settled",
        Metering::Session => "session",
    }
}

#[test]
fn a_token_count_is_added_only_where_one_was_reported_and_saturates() {
    let mut total = None;
    add_token(&mut total, None);
    assert_eq!(total, None);
    add_token(&mut total, Some(7));
    add_token(&mut total, Some(5));
    assert_eq!(total, Some(12));
    add_token(&mut total, Some(u64::MAX));
    assert_eq!(total, Some(u64::MAX));
}

/// The total is input + output made again, never read; any one figure
/// alone keeps the record, and none at all is no record.
#[test]
fn a_finished_usage_totals_input_and_output_and_keeps_any_lone_figure() {
    let lone = |set: fn(&mut TokenUsage)| {
        let mut usage = TokenUsage::default();
        set(&mut usage);
        figures(finish_usage(usage))
    };
    assert_eq!(lone(|_| {}), None);
    assert_eq!(
        [
            lone(|usage| usage.input_tokens = Some(3)),
            lone(|usage| usage.output_tokens = Some(4)),
            lone(|usage| usage.cache_read_tokens = Some(2)),
            lone(|usage| usage.cache_write_tokens = Some(6)),
            lone(|usage| usage.reasoning_output_tokens = Some(1)),
            lone(|usage| usage.total_tokens = Some(99)),
        ],
        [
            Some([Some(3), None, None, None, None, Some(3)]),
            Some([None, Some(4), None, None, None, Some(4)]),
            Some([None, None, Some(2), None, None, None]),
            Some([None, None, None, Some(6), None, None]),
            Some([None, None, None, None, Some(1), None]),
            None,
        ]
    );
}

#[test]
fn token_usage_sums_each_key_its_records_carry_and_nothing_else() {
    let records = [
        json!({"input_tokens": 10, "output_tokens": 2, "cache_read_tokens": 8}),
        json!({"input_tokens": 5, "cache_write_tokens": 1,
               "reasoning_output_tokens": 1, "total_tokens": 1000}),
        json!({"step": "seat-turn", "input_tokens": "40"}),
    ];
    assert_eq!(
        figures(token_usage(records.iter())),
        Some([Some(15), Some(2), Some(8), Some(1), Some(1), Some(17)])
    );
    assert_eq!(figures(token_usage(records[2..].iter())), None);
    assert_eq!(figures(sum_usage(std::iter::empty())), None);
}

#[test]
fn only_an_item_completed_step_counts_an_item() {
    let mut items = None;
    count_item(&mut items, Some("seat-turn"));
    count_item(&mut items, None);
    assert_eq!(items, None);
    count_item(&mut items, Some("item-completed"));
    assert_eq!(items, Some(1));
    count_item(&mut items, Some("item-completed"));
    assert_eq!(items, Some(2));
}

/// A kind this reader does not know is still the reference recorded; a
/// reference lacking any of its three strings is none.
#[test]
fn a_transcript_reference_is_read_verbatim_whatever_its_kind() {
    let recorded = json!({"kind": "vendor-log", "locator": "runs/a.jsonl",
                          "home": "/srv/home", "pid": 7});
    assert_eq!(
        transcript_of(&recorded),
        Some(Transcript {
            kind: "vendor-log".to_string(),
            locator: "runs/a.jsonl".to_string(),
            home: "/srv/home".to_string(),
        })
    );
    let not_text = |key: &str| {
        let mut partial = recorded.clone();
        partial[key] = json!(1);
        transcript_of(&partial)
    };
    assert_eq!(
        [not_text("kind"), not_text("locator"), not_text("home")],
        [None, None, None]
    );
}

#[test]
fn a_records_cost_is_its_total_cost_usd_alone() {
    let cost = |record: Value| reported_cost(&record).map(f64::to_bits);
    assert_eq!(
        cost(json!({"total_cost_usd": 0.25, "cost_usd": 9.5})),
        Some(0.25_f64.to_bits())
    );
    assert_eq!(cost(json!({"total_cost_usd": 3})), Some(3.0_f64.to_bits()));
    assert_eq!(cost(json!({"cost_usd": 9.5})), None);
    assert_eq!(cost(json!({"total_cost_usd": "0.25"})), None);
}

/// A word outside the six, or an entry with no `member` key, is dropped;
/// a `member` that is not a string is kept as no tag.
#[test]
fn boundary_entries_keep_the_six_words_of_entries_that_name_a_member() {
    let entries = |payload: Value| {
        boundary_entries(&payload)
            .into_iter()
            .map(|entry| (entry.member, entry.word, entry.gate))
            .collect::<Vec<_>>()
    };
    let payload = json!({"boundary": [
        {"member": "implement", "boundary": "namespace", "gate": true},
        {"member": null, "boundary": "not applicable"},
        {"member": "verify", "boundary": "boxed"},
        {"boundary": "open"},
        {"member": "review", "boundary": "open", "gate": "yes"},
    ]});
    assert_eq!(
        entries(payload),
        vec![
            (Some("implement".to_string()), "namespace".to_string(), true),
            (None, "not applicable".to_string(), false),
            (Some("review".to_string()), "open".to_string(), false),
        ]
    );
    assert_eq!(entries(json!({"boundary": "namespace"})), vec![]);
    assert_eq!(entries(json!({})), vec![]);
}

#[test]
fn a_seat_records_stamp_is_its_boundary_word_within_the_six() {
    let stamp = |record: Value| stamp_of(&record);
    assert_eq!(
        stamp(json!({"boundary": "seatbelt", "model": "m"})),
        Some("seatbelt".to_string())
    );
    assert_eq!(
        stamp(json!({"boundary": "not applicable"})),
        Some("not applicable".to_string())
    );
    assert_eq!(stamp(json!({"boundary": "boxed"})), None);
    assert_eq!(stamp(json!({"model": "m"})), None);
}

/// A seat is read by its turns when one carries tokens, settled by a
/// Claude session record that reports output, and by its session alone
/// when no turn carries any.
#[test]
fn a_seat_is_read_by_its_turns_settled_by_a_claude_session_or_by_its_session() {
    let turn = checkpoint(
        "seat-turn",
        json!({"input_tokens": 100, "output_tokens": 2}),
    );
    let bare = checkpoint("turn-completed", json!({}));
    let item = checkpoint("item-completed", json!({"input_tokens": 9}));
    let claude = checkpoint(
        "claude-code-session-finished",
        json!({"input_tokens": 300, "output_tokens": 40}),
    );
    let quiet = checkpoint(
        "claude-lanetally-session-finished",
        json!({"input_tokens": 300}),
    );
    let codex = checkpoint("codex-session-finished", json!({"output_tokens": 40}));
    let by = |meters: &[&Meter], session: Option<&Meter>| {
        reading(Metering::of(meters.iter().copied(), session))
    };
    assert_eq!(
        [
            by(&[&turn], Some(&claude)),
            by(&[&turn], Some(&quiet)),
            by(&[&turn], Some(&codex)),
            by(&[&turn], None),
            by(&[&bare, &item], Some(&claude)),
            by(&[], None),
        ],
        ["settled", "turns", "turns", "turns", "session", "session"]
    );
}

/// Settled, a turn counts all but its output and reasoning, which the
/// session record alone counts; the total a checkpoint carries is the one
/// it was read with, made again when summed.
#[test]
fn a_checkpoint_counts_what_its_seats_reading_takes_from_it() {
    let turn = checkpoint(
        "seat-turn",
        json!({"input_tokens": 100, "output_tokens": 2,
               "cache_read_tokens": 80, "reasoning_output_tokens": 1}),
    );
    let session = checkpoint(
        "claude-code-session-finished",
        json!({"input_tokens": 300, "output_tokens": 40, "reasoning_output_tokens": 30}),
    );
    let whole_turn = [Some(100), Some(2), Some(80), None, Some(1), Some(102)];
    let whole_session = [Some(300), Some(40), None, None, Some(30), Some(340)];
    assert_eq!(
        [
            figures(turn.counted(Metering::Turns, false)),
            figures(session.counted(Metering::Turns, true)),
            figures(turn.counted(Metering::Session, false)),
            figures(session.counted(Metering::Session, true)),
            figures(turn.counted(Metering::Settled, false)),
            figures(session.counted(Metering::Settled, true)),
            figures(session.counted(Metering::Settled, false)),
        ],
        [
            Some(whole_turn),
            None,
            None,
            Some(whole_session),
            Some([Some(100), None, Some(80), None, None, Some(102)]),
            Some([None, Some(40), None, None, Some(30), None]),
            None,
        ]
    );
}

/// A turn repeating the usage the last metered turn carried is that
/// message again and meters nothing; a turn carrying none is passed over,
/// and a record that is not a turn is left as it is.
#[test]
fn once_meters_each_message_of_a_seats_turns_once() {
    let usage = json!({"input_tokens": 100, "output_tokens": 2});
    let mut meters = [
        checkpoint("seat-turn", usage.clone()),
        checkpoint("seat-turn", json!({})),
        checkpoint("turn-completed", usage.clone()),
        checkpoint("claude-code-session-finished", usage.clone()),
        checkpoint(
            "seat-turn",
            json!({"input_tokens": 140, "output_tokens": 9}),
        ),
        checkpoint("seat-turn", usage),
    ];
    once(meters.iter_mut());
    let input = |meter: &Meter, session: bool| {
        let by = if session {
            Metering::Session
        } else {
            Metering::Turns
        };
        meter
            .counted(by, session)
            .and_then(|usage| usage.input_tokens)
    };
    assert_eq!(
        [
            input(&meters[0], false),
            input(&meters[1], false),
            input(&meters[2], false),
            input(&meters[3], true),
            input(&meters[4], false),
            input(&meters[5], false),
        ],
        [Some(100), None, None, Some(100), Some(140), Some(100)]
    );
}
