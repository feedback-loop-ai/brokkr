use super::*;
use crate::seat::row::tests::journals;
use crate::summary::Account;

/// Every row of every hand-built seat journal is read back from its data
/// as the row it was: a panel's, a carried member's, a retried seat's.
#[test]
fn every_hand_built_row_reads_back_from_its_data() {
    let mut read = 0;
    for (name, events) in journals::journals() {
        for row in Account::whole("run", &events).seats() {
            let data = row.data();
            assert_eq!(data.len(), SeatRow::COLUMNS.len(), "{name}");
            assert_eq!(SeatRow::of_data(data).as_ref(), Some(row), "{name}");
            read += 1;
        }
    }
    assert_eq!(read, 80);
}

#[test]
fn every_outcome_is_kept_by_its_word() {
    let outcomes = [
        Outcome::Working,
        Outcome::Succeeded,
        Outcome::Failed,
        Outcome::Indeterminate,
        Outcome::Carried,
    ];
    let words: Vec<&str> = outcomes.iter().map(|outcome| outcome.word()).collect();
    assert_eq!(
        words,
        ["working", "succeeded", "failed", "indeterminate", "carried"]
    );
    let concluded: Vec<Option<Outcome>> = words
        .iter()
        .chain(&["done"])
        .map(|word| Outcome::concluded(word))
        .collect();
    assert_eq!(
        concluded,
        [
            None,
            Some(Outcome::Succeeded),
            Some(Outcome::Failed),
            Some(Outcome::Indeterminate),
            None,
            None,
        ]
    );
}

/// A member's row every column of which a member's row may hold holds a
/// value, as a writer writes it: each count 1, each word its column's
/// name, each list that name as its one entry, a step's seq 1, each
/// boundary `namespace`, its pin `failed` and each price the bits 1; but
/// no end, as a member's row never holds one, its turns'
/// meter does not settle, as no turn's does, its session record is no
/// turn, as none is, and each meter's total is 2, its input and output
/// summed.
fn full() -> Vec<Datum> {
    SeatRow::COLUMNS
        .iter()
        .map(|(name, kind)| match (*name, *kind) {
            ("ended_seq" | "ended", _) => Datum::Null,
            ("pin", _) => text("failed"),
            ("boundary" | "stamp", _) => text("namespace"),
            ("metered_settles" | "session_turn", _) => Datum::Integer(0),
            ("metered_total_tokens" | "session_total_tokens", _) => Datum::Integer(2),
            ("steps", _) => text("1:15:steps"),
            ("models", _) => text("6:models"),
            ("early_tags", _) => text("1:110:early_tags"),
            ("early_lines", _) => text("6:+early1:-1:-1:-"),
            (_, "TEXT") => text(name),
            _ => Datum::Integer(1),
        })
        .collect()
}

/// A seat's own row, [`full`] as its seat's: no member, pin or carry,
/// and an end `failed`, then each of `columns` holding its value.
fn seat<'a>(columns: &[(&'a str, Datum)]) -> Vec<(&'a str, Datum)> {
    let mut seat = vec![
        ("member", Datum::Null),
        ("pin_seq", Datum::Null),
        ("pin", Datum::Null),
        ("finish_at", Datum::Null),
        ("finish_outcome", Datum::Null),
        ("carried_seq", Datum::Null),
        ("carried_attempt_id", Datum::Null),
        ("carried_ordinal", Datum::Null),
        ("ended_seq", Datum::Integer(1)),
        ("ended", text("failed")),
    ];
    seat.extend_from_slice(columns);
    seat
}

/// [`full`] with each named column holding its value.
fn with(columns: &[(&str, Datum)]) -> Vec<Datum> {
    let mut data = full();
    for (name, datum) in columns {
        let at = SeatRow::COLUMNS
            .iter()
            .position(|(column, _)| column == name);
        data[at.unwrap()] = datum.clone();
    }
    data
}

fn text(word: &str) -> Datum {
    Datum::Text(word.to_string())
}

/// Every token count of the meter whose columns start with `meter` held
/// as none.
fn untokened(meter: &str) -> Vec<(&'static str, Datum)> {
    let counts = SeatRow::COLUMNS.iter().filter(|(name, _)| {
        name.strip_prefix(meter)
            .is_some_and(|count| count.ends_with("_tokens"))
    });
    counts.map(|(name, _)| (*name, Datum::Null)).collect()
}

/// Each value a row may hold without its every part reads back as the
/// data it was read from: a provider named by none, a carried member's
/// source named by none, a member's finish pinning no outcome, a session
/// reporting no cost, a session record with no tokens and so settling
/// nothing, a meter with no input or output and so no total; and a
/// finish pinning `working`, a cost summed past `f64::MAX`, a total
/// saturated at `u64::MAX`, and a seat's own row, which ended.
#[test]
fn a_row_holding_each_part_it_may_omit_reads_back_as_its_data() {
    let mut unsettled = untokened("session_");
    unsettled.push(("session_settles", Datum::Integer(0)));
    let max = || Datum::Integer(u64::MAX as i64);
    let variants = [
        vec![
            ("metered_input_tokens", Datum::Null),
            ("metered_output_tokens", Datum::Null),
            ("metered_total_tokens", Datum::Null),
        ],
        vec![
            ("session_input_tokens", max()),
            ("session_total_tokens", max()),
        ],
        vec![],
        vec![("provider", Datum::Null)],
        vec![("carried_ordinal", Datum::Null)],
        vec![("pin", Datum::Null)],
        vec![("session_cost", Datum::Null)],
        unsettled,
        vec![("pin", text("working"))],
        vec![("spent", Datum::Integer(f64::INFINITY.to_bits() as i64))],
        seat(&[]),
        vec![("finish_outcome", Datum::Null)],
        vec![("session_id", Datum::Null)],
        vec![("seat_turn_target", Datum::Null)],
        vec![
            ("seat_turn_seq", Datum::Null),
            ("seat_turn", Datum::Null),
            ("seat_turn_tool", Datum::Null),
            ("seat_turn_target", Datum::Null),
        ],
        vec![
            ("steps", Datum::Null),
            ("tagged_seq", Datum::Null),
            ("models", Datum::Null),
            ("early_tags", Datum::Null),
            ("early_lines", Datum::Null),
        ],
        vec![("live", Datum::Integer(0))],
    ];
    for columns in variants {
        let data = with(&columns);
        let row = SeatRow::of_data(data.clone());
        assert_eq!(row.map(|row| row.data()), Some(data), "{columns:?}");
    }
}

/// What a record says, kept in part, which no row writes (#753, slice
/// 2b): a finish with no time, or a time or an outcome with no finish; a
/// session's id with no session record; a turn's seq, turn or tool
/// without the rest, and a target without its turn.
fn unsaid() -> Vec<Vec<(&'static str, Datum)>> {
    let none = || Datum::Null;
    let unfinished = || vec![("pin_seq", none()), ("pin", none())];
    let with_unfinished = |column: &'static str| {
        let mut columns = unfinished();
        columns.push((column, text(column)));
        columns.push((
            if column == "finish_at" {
                "finish_outcome"
            } else {
                "finish_at"
            },
            none(),
        ));
        columns
    };
    let unturned = ["seat_turn_seq", "seat_turn", "seat_turn_tool"];
    let mut sessionless: Vec<(&str, Datum)> = SeatRow::COLUMNS
        .iter()
        .filter(|(name, _)| name.starts_with("session_") && *name != "session_id")
        .map(|(name, _)| (*name, none()))
        .collect();
    sessionless.push(("session_id", text("s")));
    let mut found = vec![
        vec![("finish_at", none())],
        with_unfinished("finish_at"),
        with_unfinished("finish_outcome"),
        sessionless,
    ];
    found.extend(unturned.map(|column| vec![(column, none())]));
    let targeted = unturned.map(|column| (column, none()));
    found.push(targeted.to_vec());
    found
}

/// Data no row writes is no row: a value of another type than its
/// column's, a flag that is neither, a word no outcome is or one its
/// column's writer never writes, a cost no record states or a sum no
/// costs reach, a turns' meter no turn writes, a session meter that is a
/// turn's or settles no output, a meter's total its input and output do
/// not sum to, a part of a value without the rest, a list its writer does
/// not write, and more or fewer values than columns.
#[test]
fn data_no_row_writes_reads_as_none() {
    let none = || Datum::Null;
    let bits = |cost: f64| Datum::Integer(cost.to_bits() as i64);
    let session_unmetered: Vec<(&str, Datum)> = SeatRow::COLUMNS
        .iter()
        .filter(|(name, _)| name.starts_with("session_") && *name != "session_cost")
        .map(|(name, _)| (*name, Datum::Null))
        .collect();
    let metered_untagged = [("metered_turn", none()), ("metered_settles", none())];
    let refused: Vec<Vec<(&str, Datum)>> = vec![
        seat(&[("ended", text("carried"))]),
        seat(&[("ended", text("working"))]),
        vec![("pin", text("carried"))],
        vec![("request_seq", none())],
        vec![("effect_id", none())],
        vec![("since", text("1"))],
        vec![("seat", Datum::Integer(1))],
        vec![("spent", text("0.5"))],
        vec![("spent", bits(f64::NAN))],
        vec![("session_cost", bits(f64::NAN))],
        vec![("session_cost", bits(f64::INFINITY))],
        vec![("session_cost", bits(f64::NEG_INFINITY))],
        vec![("metered_turn", Datum::Integer(2))],
        vec![("metered_turn", Datum::Integer(0))],
        vec![("metered_settles", Datum::Integer(1))],
        untokened("metered_"),
        vec![("session_turn", Datum::Integer(1))],
        vec![("session_output_tokens", none())],
        untokened("session_"),
        vec![("metered_total_tokens", Datum::Integer(999))],
        vec![("session_total_tokens", Datum::Integer(999))],
        vec![("session_total_tokens", none())],
        vec![
            ("metered_input_tokens", none()),
            ("metered_output_tokens", none()),
        ],
        vec![("started_at", none())],
        vec![("model_seq", none())],
        vec![("provider_seq", none())],
        seat(&[("ended", text("done"))]),
        vec![("pin_seq", none())],
        vec![("carried_seq", none())],
        vec![("carried_seq", none()), ("carried_attempt_id", none())],
        metered_untagged.to_vec(),
        vec![("metered_settles", none())],
        vec![("session_seq", none())],
        session_unmetered,
        vec![("transcript_seq", none())],
        vec![("transcript_locator", none())],
        vec![("transcript_home", none())],
        vec![("steps", text("1:16:steps"))],
        vec![("steps", Datum::Integer(1))],
        vec![("tagged_seq", text("1"))],
        vec![("models", text(""))],
        vec![("early_lines", text("1:-"))],
        vec![("early_lines", Datum::Integer(1))],
        vec![("live", none())],
        vec![("live", Datum::Integer(2))],
    ];
    for columns in refused.into_iter().chain(unsaid()) {
        assert_eq!(SeatRow::of_data(with(&columns)), None, "{columns:?}");
    }
    let mut fewer = full();
    fewer.pop();
    assert_eq!(SeatRow::of_data(fewer), None);
    let mut more = full();
    more.push(Datum::Null);
    assert_eq!(SeatRow::of_data(more), None);
}
