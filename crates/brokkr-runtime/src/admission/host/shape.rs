//! The host file's wire forms, read strictly: a closed object is an object
//! and a closed enum is its word, never serde's positional-array or
//! one-key-map alternatives (decision 0071 ruling 3). Every shape here is
//! one the published `forge.host/v1` or `forge.host/v2` contract admits.

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::num::{NonZeroU32, NonZeroU64};

use serde::de::value::MapAccessDeserializer;
use serde::de::{self, IntoDeserializer, MapAccess, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};

/// A number the contract calls an integer. JSON Schema's `integer` admits
/// any number with no fractional part, so `10.0` and `6e1` are the same
/// integers here as `10` and `60`; refusing them would refuse a file the
/// published schema admits (decision 0071 ruling 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Integral<T>(pub(super) T);

impl<'de, T> Deserialize<'de> for Integral<T>
where
    T: TryFrom<u64> + TryFrom<i64>,
{
    fn deserialize<D>(deserializer: D) -> Result<Integral<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct IntegralVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for IntegralVisitor<T>
        where
            T: TryFrom<u64> + TryFrom<i64>,
        {
            type Value = Integral<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("an integer")
            }

            fn visit_u64<E: de::Error>(self, value: u64) -> Result<Integral<T>, E> {
                T::try_from(value)
                    .map(Integral)
                    .map_err(|_| E::invalid_value(Unexpected::Unsigned(value), &self))
            }

            fn visit_i64<E: de::Error>(self, value: i64) -> Result<Integral<T>, E> {
                T::try_from(value)
                    .map(Integral)
                    .map_err(|_| E::invalid_value(Unexpected::Signed(value), &self))
            }

            fn visit_f64<E: de::Error>(self, value: f64) -> Result<Integral<T>, E> {
                if value.fract() == 0.0 && (0.0..u64::MAX as f64).contains(&value) {
                    T::try_from(value as u64)
                        .map(Integral)
                        .map_err(|_| E::invalid_value(Unexpected::Float(value), &self))
                } else {
                    Err(E::invalid_value(Unexpected::Float(value), &self))
                }
            }
        }

        deserializer.deserialize_any(IntegralVisitor(PhantomData))
    }
}

/// The number serde names when a positive integer is refused.
fn number(value: i128) -> Unexpected<'static> {
    if value < 0 {
        Unexpected::Signed(value as i64)
    } else {
        Unexpected::Unsigned(value as u64)
    }
}

/// A positive integer, the contract's ceiling or `floor_bytes`.
pub(super) fn positive_u32<'de, D>(deserializer: D) -> Result<NonZeroU32, D::Error>
where
    D: Deserializer<'de>,
{
    let Integral(value) = Integral::<i128>::deserialize(deserializer)?;
    u32::try_from(value)
        .ok()
        .and_then(NonZeroU32::new)
        .ok_or_else(|| <D::Error as de::Error>::invalid_value(number(value), &"a nonzero u32"))
}

/// A positive integer, the contract's `floor_bytes`.
pub(super) fn positive_u64<'de, D>(deserializer: D) -> Result<NonZeroU64, D::Error>
where
    D: Deserializer<'de>,
{
    let Integral(value) = Integral::<i128>::deserialize(deserializer)?;
    u64::try_from(value)
        .ok()
        .and_then(NonZeroU64::new)
        .ok_or_else(|| <D::Error as de::Error>::invalid_value(number(value), &"a nonzero u64"))
}

/// A record read from a JSON object and nothing else. serde's derived
/// struct reader also takes a sequence and reads its fields by position,
/// which the closed host contract does not admit (decision 0071 ruling 3).
pub(super) struct Object<T>(pub(super) T);

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

/// A record from an object, for a field read through `deserialize_with`.
pub(super) fn object<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Object::<T>::deserialize(deserializer).map(|Object(value)| value)
}

/// A written key that must be an object, or be absent: `null` is not the
/// object's absence (decision 0068 ruling 5's `present`).
pub(super) fn present_object<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    object(deserializer).map(Some)
}

/// A map whose every value is an object, read strictly.
pub(super) fn strict_map<'de, D, T>(deserializer: D) -> Result<BTreeMap<String, T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    BTreeMap::<String, Object<T>>::deserialize(deserializer).map(|map| {
        map.into_iter()
            .map(|(key, Object(value))| (key, value))
            .collect()
    })
}

/// A closed enum's word and nothing else. serde's derived enum reader also
/// takes the one-key map form, which the closed host contract does not
/// admit (decision 0071 ruling 3, as decision 0084's severity).
pub(super) fn word<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    struct Words<T>(PhantomData<T>);

    impl<'de, T: Deserialize<'de>> Visitor<'de> for Words<T> {
        type Value = T;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E: de::Error>(self, word: &str) -> Result<T, E> {
            T::deserialize(word.into_deserializer())
        }
    }

    deserializer.deserialize_str(Words(PhantomData))
}

/// A written key that must be a word, or be absent.
pub(super) fn present_word<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    word(deserializer).map(Some)
}
