//! The JSON shapes a closed contract admits, read strictly: a record is an
//! object and nothing else (decision 0071 ruling 3).
//!
//! [`Object`] is the one home of that refusal. The host file's reader and
//! the driver protocol's header reader both read it through here, so the
//! rule is written once. This module is pure: it reads no I/O, clock or
//! environment (decision 0071 ruling 1).

use std::fmt;
use std::marker::PhantomData;

use serde::de::value::MapAccessDeserializer;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde::Deserialize;

/// A record read from a JSON object and nothing else.
///
/// serde's derived struct visitor also takes a sequence and reads its
/// fields by position, so `[10, 60]` would stand for a closed record the
/// contract says is an object (decision 0071 ruling 3). `deserialize_map`
/// admits only the map form, and the visitor's refusal names what it
/// expected: "a JSON object".
#[derive(Default)]
pub struct Object<T>(pub T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D>(deserializer: D) -> Result<Object<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct Fields<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for Fields<T> {
            type Value = T;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
                T::deserialize(MapAccessDeserializer::new(map))
            }
        }

        deserializer
            .deserialize_map(Fields(PhantomData))
            .map(Object)
    }
}

#[cfg(test)]
mod tests;
