//! The host file's wire forms, read strictly: a closed object is an object
//! and a closed enum is its word, never serde's positional-array or
//! one-key-map alternatives (decision 0071 ruling 3). Every shape here is
//! one the published `forge.host/v1`, `forge.host/v2` or `forge.host/v3`
//! contract admits.

use std::collections::BTreeMap;
use std::fmt;
use std::marker::PhantomData;
use std::num::{NonZeroU32, NonZeroU64};

use serde::de::{self, IntoDeserializer, Unexpected, Visitor};
use serde::{Deserialize, Deserializer};
use serde_json::value::RawValue;

use super::{floor, RouteClass};

pub(super) use brokkr_core::shape::Object;

/// One provider as the file writes it, before the loader reads its routes
/// and seals their floors. Closed, as every host object is.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WrittenProvider {
    #[serde(deserialize_with = "positive_u32")]
    pub(super) ceiling: NonZeroU32,
    #[serde(default, deserialize_with = "strict_map")]
    pub(super) routes: BTreeMap<String, WrittenRoute>,
}

/// One written route: its ceiling, class, and the v3 floor kept as its
/// exact token until the file's version is known.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WrittenRoute {
    #[serde(deserialize_with = "positive_u32")]
    pub(super) ceiling: NonZeroU32,
    #[serde(deserialize_with = "word")]
    pub(super) class: RouteClass,
    #[serde(default, deserialize_with = "super::present")]
    pub(super) timeout_floor_seconds: Option<floor::Written>,
}

/// A number the contract calls an integer. JSON Schema's `integer` admits
/// any number with no fractional part, so `10.0`, `1e1` and `1.5e1` are the
/// same integers here as `10`; refusing them would refuse a file the
/// published schema admits (decision 0071 ruling 3). Each is read from its
/// exact token, never through `f64`, so no rounding can lower a floor or
/// raise a bound (the operator's ruling of 2026-11-04, option A).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Integral<T>(pub(super) T);

/// The largest positive exponent the adjusted mantissa may be shifted by.
/// No field's range needs a nonzero mantissa shifted further, so one past
/// this cannot fit; the bound also keeps a zero mantissa from making the
/// reader loop. A token like `1e999999999` is refused before the reader
/// builds it (the operator's ruling of 2026-11-04, option A). A negative
/// exponent is bounded by the mantissa's trailing zeros instead, so
/// `10^41e-41` reads as `1`.
const EXPONENT_LIMIT: i64 = 40;

/// Why a token is not an integer this reader can return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Refusal {
    /// A token that is not a JSON number at all: a string, object, array,
    /// boolean or null where the contract demands a number.
    NotANumber,
    /// A fraction the exponent does not shift away, or a negative exponent
    /// that would leave one: the number it names has a fractional part.
    NotAnInteger,
    /// The integer does not fit the field's type.
    OutOfRange,
    /// A positive exponent is past the reader's bound, or the exponent's
    /// digits overflow, so the number it would name is refused before the
    /// reader tries to build it.
    ExponentPastBound,
}

impl Refusal {
    fn unexpected(self) -> Unexpected<'static> {
        match self {
            Refusal::NotANumber => Unexpected::Other("a value that is not a number"),
            Refusal::NotAnInteger => Unexpected::Other("a number with a fractional part"),
            Refusal::OutOfRange => Unexpected::Other("a number out of range"),
            Refusal::ExponentPastBound => {
                Unexpected::Other("a number with an exponent too large to read")
            }
        }
    }
}

impl<'de, T> Deserialize<'de> for Integral<T>
where
    T: TryFrom<i128> + TryFrom<u128>,
{
    fn deserialize<D>(deserializer: D) -> Result<Integral<T>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let token = Box::<RawValue>::deserialize(deserializer)?;
        token
            .get()
            .parse::<Integral<T>>()
            .map_err(|refusal: Refusal| {
                <D::Error as de::Error>::invalid_value(refusal.unexpected(), &"an integer")
            })
    }
}

/// The integer a JSON number token names exactly: an optional `-`, digits,
/// an optional fraction, and an optional exponent. The fraction's digits are
/// folded into the mantissa and its length subtracted from the exponent, so
/// `1.5e1` and `15e0` name the same integer; the value is computed with
/// integer arithmetic only, and a negative exponent may only remove trailing
/// zeros.
impl<T> std::str::FromStr for Integral<T>
where
    T: TryFrom<i128> + TryFrom<u128>,
{
    type Err = Refusal;

    fn from_str(token: &str) -> Result<Integral<T>, Refusal> {
        let bytes = token.as_bytes();
        let Some(first) = bytes.first() else {
            return Err(Refusal::NotANumber);
        };
        if !first.is_ascii_digit() && *first != b'-' {
            return Err(Refusal::NotANumber);
        }
        let (negative, mut at) = match *first {
            b'-' => (true, 1),
            _ => (false, 0),
        };
        let int_start = at;
        at = digits_end(bytes, int_start);
        let int_digits = &bytes[int_start..at];
        if int_digits.is_empty() {
            return Err(Refusal::NotAnInteger);
        }
        let mut fraction = &[][..];
        if bytes.get(at) == Some(&b'.') {
            let fraction_start = at + 1;
            at = digits_end(bytes, fraction_start);
            fraction = &bytes[fraction_start..at];
            if fraction.is_empty() {
                return Err(Refusal::NotAnInteger);
            }
        }
        let (written, used) = exponent(&bytes[at..])?;
        if at + used != bytes.len() {
            return Err(Refusal::NotAnInteger);
        }
        // The fraction's digits join the mantissa, one decimal place each,
        // so its length debits the exponent. Only the adjusted shift has to
        // fit the bound: a fraction can carry a large written exponent back
        // into range, and the mantissa's own length is bounded by the token.
        // `usize` fits `i128` on every supported host, so the length is a
        // widening cast; the value arithmetic below stays checked.
        let adjusted = i128::from(written) - fraction.len() as i128;
        if adjusted > i128::from(EXPONENT_LIMIT) {
            return Err(Refusal::ExponentPastBound);
        }
        let combined;
        let digits: &[u8] = if fraction.is_empty() {
            int_digits
        } else {
            combined = [int_digits, fraction].concat();
            &combined
        };
        let magnitude = magnitude(digits, adjusted)?;
        fits::<T>(negative, magnitude).map(Integral)
    }
}

/// Where the run of ASCII digits in `bytes` that starts at `at` ends.
fn digits_end(bytes: &[u8], mut at: usize) -> usize {
    while bytes.get(at).is_some_and(u8::is_ascii_digit) {
        at += 1;
    }
    at
}

/// The exponent written after the fraction, and how many bytes of `tail`
/// it took: `(value, bytes)`. The empty tail is no exponent.
fn exponent(tail: &[u8]) -> Result<(i64, usize), Refusal> {
    let Some((&marker, _)) = tail.split_first() else {
        return Ok((0, 0));
    };
    if marker != b'e' && marker != b'E' {
        return Ok((0, 0));
    }
    let mut at = 1;
    let negative = match tail.get(at) {
        Some(b'-') => {
            at += 1;
            true
        }
        Some(b'+') => {
            at += 1;
            false
        }
        _ => false,
    };
    let start = at;
    let mut value = 0i64;
    while let Some(digit) = tail.get(at) {
        if !digit.is_ascii_digit() {
            break;
        }
        value = value
            .checked_mul(10)
            .and_then(|value| value.checked_add(i64::from(digit - b'0')))
            .ok_or(Refusal::ExponentPastBound)?;
        at += 1;
    }
    if at == start {
        return Err(Refusal::NotAnInteger);
    }
    Ok((if negative { -value } else { value }, at))
}

/// `digits` shifted by `exponent`: a positive exponent appends zeros, and
/// a negative one removes only trailing zeros. A positive exponent is
/// bounded by [`EXPONENT_LIMIT`] before this is called, so the loop is
/// bounded by the token's own grammar.
fn magnitude(digits: &[u8], exponent: i128) -> Result<u128, Refusal> {
    if exponent >= 0 {
        let mut value = digits_value(digits)?;
        for _ in 0..exponent {
            value = value.checked_mul(10).ok_or(Refusal::OutOfRange)?;
        }
        return Ok(value);
    }
    let mut end = digits.len();
    let mut remove = exponent.unsigned_abs();
    while remove > 0 && end > 0 && digits[end - 1] == b'0' {
        end -= 1;
        remove -= 1;
    }
    if remove > 0 && digits[..end].iter().any(|digit| *digit != b'0') {
        return Err(Refusal::NotAnInteger);
    }
    digits_value(&digits[..end])
}

/// The unsigned integer a run of decimal digits names.
fn digits_value(digits: &[u8]) -> Result<u128, Refusal> {
    digits.iter().try_fold(0u128, |value, digit| {
        value
            .checked_mul(10)
            .and_then(|value| value.checked_add(u128::from(digit - b'0')))
            .ok_or(Refusal::OutOfRange)
    })
}

/// `magnitude` as `T`, keeping the sign: the field's exact range, never a
/// rounded one.
fn fits<T>(negative: bool, magnitude: u128) -> Result<T, Refusal>
where
    T: TryFrom<i128> + TryFrom<u128>,
{
    if negative {
        let signed = match magnitude {
            0 => 0,
            value if value == i128::MIN.unsigned_abs() => i128::MIN,
            value => -i128::try_from(value).map_err(|_| Refusal::OutOfRange)?,
        };
        T::try_from(signed).map_err(|_| Refusal::OutOfRange)
    } else {
        T::try_from(magnitude).map_err(|_| Refusal::OutOfRange)
    }
}

/// The number serde names when a positive integer is refused, without
/// narrowing it to a type serde's `Unexpected` holds; one outside both
/// ranges keeps its own words instead of wrapping.
fn number(value: i128) -> Unexpected<'static> {
    if let Ok(value) = u64::try_from(value) {
        Unexpected::Unsigned(value)
    } else if let Ok(value) = i64::try_from(value) {
        Unexpected::Signed(value)
    } else {
        Unexpected::Other("a number out of range")
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
