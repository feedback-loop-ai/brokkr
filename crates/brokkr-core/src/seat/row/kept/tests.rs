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
    assert_eq!(read, 48);
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
    for outcome in outcomes {
        assert_eq!(Outcome::of_word(outcome.word()), Some(outcome));
    }
    assert_eq!(Outcome::of_word("done"), None);
}

/// A row every column of which holds a value: each count 1, each word its
/// column's name, each outcome `failed` and each price a half.
fn full() -> Vec<Datum> {
    SeatRow::COLUMNS
        .iter()
        .map(|(name, kind)| match (*name, *kind) {
            ("ended" | "pin", _) => Datum::Text("failed".to_string()),
            (_, "TEXT") => Datum::Text(name.to_string()),
            (_, "REAL") => Datum::Real(0.5),
            _ => Datum::Integer(1),
        })
        .collect()
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

/// Each value a row may hold without its every part reads back as the
/// data it was read from: a provider named by none, a carried member's
/// source named by none, a member's finish pinning no outcome, a session
/// reporting no cost, a meter with no tokens.
#[test]
fn a_row_holding_each_part_it_may_omit_reads_back_as_its_data() {
    let usage = [
        "metered_input_tokens",
        "metered_output_tokens",
        "metered_cache_read_tokens",
        "metered_cache_write_tokens",
        "metered_reasoning_output_tokens",
        "metered_total_tokens",
    ];
    let untokened: Vec<(&str, Datum)> = usage.iter().map(|name| (*name, Datum::Null)).collect();
    let variants = [
        vec![],
        vec![("provider", Datum::Null)],
        vec![("carried_ordinal", Datum::Null)],
        vec![("pin", Datum::Null)],
        vec![("session_cost", Datum::Null)],
        vec![("metered_settles", Datum::Integer(0))],
        untokened,
    ];
    for columns in variants {
        let data = with(&columns);
        let row = SeatRow::of_data(data.clone());
        assert_eq!(row.map(|row| row.data()), Some(data), "{columns:?}");
    }
}

/// Data no row writes is no row: a value of another type than its
/// column's, a flag that is neither, a word no outcome is, a part of a
/// value without the rest, and more or fewer values than columns.
#[test]
fn data_no_row_writes_reads_as_none() {
    let none = || Datum::Null;
    let session_unmetered: Vec<(&str, Datum)> = SeatRow::COLUMNS
        .iter()
        .filter(|(name, _)| name.starts_with("session_") && *name != "session_cost")
        .map(|(name, _)| (*name, Datum::Null))
        .collect();
    let metered_untagged = [("metered_turn", none()), ("metered_settles", none())];
    let refused: Vec<Vec<(&str, Datum)>> = vec![
        vec![("request_seq", none())],
        vec![("effect_id", none())],
        vec![("since", text("1"))],
        vec![("seat", Datum::Integer(1))],
        vec![("spent", text("0.5"))],
        vec![("metered_turn", Datum::Integer(2))],
        vec![("started_at", none())],
        vec![("model_seq", none())],
        vec![("provider_seq", none())],
        vec![("ended", text("done"))],
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
    ];
    for columns in refused {
        assert_eq!(SeatRow::of_data(with(&columns)), None, "{columns:?}");
    }
    let mut fewer = full();
    fewer.pop();
    assert_eq!(SeatRow::of_data(fewer), None);
    let mut more = full();
    more.push(Datum::Null);
    assert_eq!(SeatRow::of_data(more), None);
}
