use super::Object;
use serde::Deserialize;

/// A closed record whose fields a positional array could line up with.
#[derive(Debug, Deserialize, PartialEq)]
struct Record {
    level: String,
    count: u32,
}

#[test]
fn an_object_reads_into_its_record() {
    let read: Object<Record> =
        serde_json::from_str(r#"{"level":"high","count":2}"#).expect("an object reads");
    assert_eq!(
        read.0,
        Record {
            level: "high".to_string(),
            count: 2,
        }
    );
}

#[test]
fn an_array_of_the_same_fields_is_refused_as_not_an_object() {
    let Err(error) = serde_json::from_str::<Object<Record>>(r#"["high",2]"#) else {
        panic!("an array of the record's fields was not refused");
    };
    assert_eq!(
        error.to_string(),
        "invalid type: sequence, expected a JSON object at line 1 column 0"
    );
}

#[test]
fn a_scalar_is_refused_as_not_an_object() {
    let Err(error) = serde_json::from_str::<Object<Record>>("42") else {
        panic!("a scalar was not refused");
    };
    assert_eq!(
        error.to_string(),
        "invalid type: integer `42`, expected a JSON object at line 1 column 2"
    );
}
