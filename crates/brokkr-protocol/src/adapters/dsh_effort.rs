//! A dsh seat's effort on the two channels the releases read it from,
//! and the level a request header says the seat ran at (decision 0035,
//! addenda 2026-09-05 and 2026-10-17).
//!
//! dsh 0.1 reads the level from the seat's settings document and ignores
//! it on the composition row; dsh 0.2 dropped the settings document's
//! fields and reads it on the composition row (measured 2026-10-17 on
//! 0.1.5-rc.1 and 0.2.1-rc.2, #728). The driver writes it on both.

use std::fmt;
use std::marker::PhantomData;

use serde::de::value::MapAccessDeserializer;
use serde::de::{Deserializer, MapAccess, Visitor};
use serde::Deserialize;
use serde_json::Value;

use super::{dsh_model_row, effort_token};

/// The pinned level as the one YAML scalar both channels write: clamped
/// to one bounded word by `effort_token`, and single-quoted with its
/// quotes doubled anyway, because both channels are YAML and the
/// discipline is the transcript root's.
pub(super) fn pinned_level(effort: &str) -> Result<String, String> {
    let Some(effort) = effort_token(effort) else {
        return Err("dsh driver: the pinned effort is not one bounded word".to_string());
    };
    Ok(format!("'{}'", effort.replace('\'', "''")))
}

/// The pinned-model row with the pinned level on it, where dsh 0.2's
/// `agent-default-model` reads it as a live composition field. 0.1
/// ignores the key without a word and reads the settings document
/// instead, so a seat pinned on either release reaches the wire.
pub(super) fn model_row(model: &str, effort: Option<&str>) -> Result<String, String> {
    let mut row = dsh_model_row(model)?;
    if let Some(effort) = effort {
        row.push_str(&format!("    reasoningEffort: {}\n", pinned_level(effort)?));
    }
    Ok(row)
}

/// A record read from a JSON object and nothing else. serde's derived
/// struct visitor also takes a sequence and reads its fields by position,
/// so `[{"reasoningEffort": "high"}, {}]` would stand for a header, and a
/// header the reader cannot read records nothing (#728).
#[derive(Default)]
struct Object<T>(T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer
            .deserialize_map(Fields(PhantomData))
            .map(Object)
    }
}

/// The visitor that hands a map, and only a map, to `T`'s derived reader.
struct Fields<T>(PhantomData<T>);

impl<'de, T: Deserialize<'de>> Visitor<'de> for Fields<T> {
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a JSON object")
    }

    fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<T, A::Error> {
        T::deserialize(MapAccessDeserializer::new(map))
    }
}

/// What the fold reads of a request header: the level in its `config`,
/// and whether the adapter's default supplied it. The header is an open
/// harness record, so every other key is left unread; it and both of the
/// records it holds are each an [`Object`].
#[derive(Deserialize)]
struct Header {
    #[serde(default)]
    config: Object<Config>,
    #[serde(default, rename = "adapterDefaults")]
    defaults: Object<Defaults>,
}

#[derive(Deserialize, Default)]
struct Config {
    #[serde(rename = "reasoningEffort")]
    effort: Option<String>,
}

#[derive(Deserialize, Default)]
struct Defaults {
    #[serde(default, rename = "reasoningEffort")]
    effort: bool,
}

/// The level a `request/header` event says this seat ran at, or `None`
/// when it says none the seat chose: no level, one that fails the clamp,
/// a header that does not read, or a level the adapter defaulted. dsh 0.2
/// replaces a pin it did not read with its adapter's default and marks
/// it `adapterDefaults.reasoningEffort: true`; that level is the
/// adapter's, never the seat's, so the seat's effort stands as it was.
pub(super) fn applied_level(event: &Value) -> Option<String> {
    let Object(header) = Object::<Header>::deserialize(event.pointer("/data/header")?).ok()?;
    if header.defaults.0.effort {
        return None;
    }
    header.config.0.effort.as_deref().and_then(effort_token)
}
