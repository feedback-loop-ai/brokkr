//! One line of a stream parsed so that nothing in it goes unread.
//! `serde_json` keeps the last value of a key an object repeats and drops
//! the others, and a dropped value is one no reader visits, so a line that
//! repeats a key is not read as an event at all. Pure, like `measure`.

use std::fmt;

use serde::de::{self, Deserialize, Deserializer, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};

/// `line` as one JSON object, when it is one and none of the objects it
/// holds names a key twice.
pub(super) fn object(line: &str) -> Option<Map<String, Value>> {
    match serde_json::from_str::<Strict>(line) {
        Ok(Strict(Value::Object(fields))) => Some(fields),
        _ => None,
    }
}

/// A JSON value read by [`Once`].
struct Strict(Value);

impl<'de> Deserialize<'de> for Strict {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Strict, D::Error> {
        deserializer.deserialize_any(Once).map(Strict)
    }
}

/// Builds a [`Value`], refusing an object that names a key twice.
struct Once;

impl<'de> Visitor<'de> for Once {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("JSON whose every object names each key once")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Value, E> {
        Ok(value.into())
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Value, E> {
        Ok(value.into())
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Value, E> {
        Ok(value.into())
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Value, E> {
        Ok(value.into())
    }

    fn visit_unit<E: de::Error>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let mut items = Vec::new();
        while let Some(Strict(item)) = seq.next_element()? {
            items.push(item);
        }
        Ok(Value::Array(items))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let mut fields = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let Strict(value) = map.next_value()?;
            if fields.insert(key, value).is_some() {
                return Err(de::Error::invalid_value(de::Unexpected::Map, &self));
            }
        }
        Ok(Value::Object(fields))
    }
}
