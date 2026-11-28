//! Wire types and map-only object reader for Antigravity CLI's stdout stream
//! (`#1200`, `#1227`).
//!
//! Wherever the agy driver reads a JSON object into a typed struct from bytes
//! it did not write itself, [`Object`] requires a JSON map at every depth of
//! the struct so an array in the place of an object is never read by position.

use std::fmt;
use std::marker::PhantomData;

use serde::de::value::MapAccessDeserializer;
use serde::de::{IgnoredAny, MapAccess, Visitor};
use serde::{Deserialize, Deserializer};

use super::SIGNED_OUT;

/// Deserializes `T` strictly from a JSON object (`{...}`), rejecting JSON
/// arrays that serde's derived struct reader would otherwise accept by
/// position (`#1227`).
#[derive(Default)]
pub(in crate::adapters::agy) struct Object<T>(pub(in crate::adapters::agy) T);

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Object<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor<T>(PhantomData<T>);

        impl<'de, T: Deserialize<'de>> Visitor<'de> for ObjectVisitor<T> {
            type Value = Object<T>;

            fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str("a JSON object")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                T::deserialize(MapAccessDeserializer::new(map)).map(Object)
            }
        }

        deserializer.deserialize_map(ObjectVisitor(PhantomData))
    }
}

/// The rows of a turn: the one list of the events the fold knows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(in crate::adapters::agy) enum Event {
    Init,
    StepUpdate,
    Result,
}

impl Event {
    /// The row, as the driver's words name it.
    pub(super) fn row(self) -> &'static str {
        match self {
            Event::Init => "an init row",
            Event::StepUpdate => "a step_update row",
            Event::Result => "a result row",
        }
    }

    /// The row `text` holds as this event's, typed straight from the text,
    /// or none where it does not type: a key the fold reads written twice,
    /// at any depth, is no row, and an array in the place of an object is no
    /// object, at any depth (K1). On a cold turn `step_update.usage` is not
    /// read; on a rejoin it is the sole source of the turn's use and is typed.
    pub(super) fn typed(self, text: &str, rejoined: bool) -> Option<Row> {
        match self {
            Event::Init => Self::typed_init(text).map(Row::Init),
            Event::StepUpdate if !rejoined => serde_json::from_str::<Object<ColdStepRow>>(text)
                .ok()
                .map(|Object(r)| {
                    Row::Step(Step {
                        step_type: r.step_update.0.step_type,
                        state: r.step_update.0.state,
                        usage: Object::default(),
                    })
                }),
            Event::StepUpdate => serde_json::from_str::<Object<StepRow>>(text)
                .ok()
                .map(|Object(row)| Row::Step(row.step_update.0)),
            Event::Result => serde_json::from_str::<Object<ResultRow>>(text)
                .ok()
                .map(|Object(row)| Row::Result(row.result.0)),
        }
    }

    /// The `init` row `text` holds, typed through the same map-only reader
    /// [`Self::typed`] uses (K1).
    pub(super) fn typed_init(text: &str) -> Option<Init> {
        serde_json::from_str::<Object<InitRow>>(text)
            .ok()
            .map(|Object(row)| row.init.0)
    }
}

/// A line's top-level `event` value: one the fold knows, or any other.
#[derive(Deserialize)]
#[serde(untagged)]
enum Named {
    Known(Event),
    Other(IgnoredAny),
}

/// The `event`s a line's object names at its top level, read from its
/// text: the first the fold knows, and how many `event` keys it holds.
pub(super) struct Tags {
    pub(super) known: Option<Event>,
    pub(super) named: usize,
}

impl<'de> Deserialize<'de> for Tags {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Tags, D::Error> {
        deserializer.deserialize_map(TagsVisitor)
    }
}

struct TagsVisitor;

impl<'de> Visitor<'de> for TagsVisitor {
    type Value = Tags;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("an object naming an event")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Tags, A::Error> {
        let mut tags = Tags {
            known: None,
            named: 0,
        };
        while let Some(key) = map.next_key::<String>()? {
            if key != "event" {
                map.next_value::<IgnoredAny>()?;
                continue;
            }
            tags.named += 1;
            if let Named::Known(event) = map.next_value()? {
                tags.known.get_or_insert(event);
            }
        }
        Ok(tags)
    }
}

/// One known row of agy's stream, as far as the fold reads it.
pub(super) enum Row {
    Init(Init),
    Step(Step),
    Result(Ending),
}

/// What the turn's `init` row said of its conversation id (#1227 W5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::adapters::agy) enum Conversation {
    /// No `init` row arrived in this turn.
    None,
    /// Single lowercase-UUID `conversation_id`.
    Named(String),
    /// Absent, non-string, repeated, or non-UUID `conversation_id`.
    Invalid,
}

/// Read `conversation_id` from an `init` line beside the fold (#1227 W5).
/// Absent, non-string, repeated, or non-UUID values yield `Invalid`.
pub(in crate::adapters::agy) fn read_conversation_id(text: &str) -> Conversation {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum RawValue {
        Text(String),
        Other(IgnoredAny),
    }
    struct IdVisitor;
    impl<'de> Visitor<'de> for IdVisitor {
        type Value = Conversation;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("an init JSON object")
        }

        fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Conversation, A::Error> {
            let (mut count, mut first) = (0usize, None);
            while let Some(key) = map.next_key::<String>()? {
                if key != "conversation_id" {
                    map.next_value::<IgnoredAny>()?;
                    continue;
                }
                count += 1;
                let value = map.next_value::<RawValue>()?;
                if count == 1 {
                    first = Some(value);
                }
            }
            Ok(match (count, first) {
                (1, Some(RawValue::Text(id))) if brokkr_core::seat::is_agy_uuid(&id) => {
                    Conversation::Named(id)
                }
                _ => Conversation::Invalid,
            })
        }
    }
    struct Wrapper(Conversation);
    impl<'de> Deserialize<'de> for Wrapper {
        fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
            d.deserialize_map(IdVisitor).map(Wrapper)
        }
    }
    serde_json::from_str::<Wrapper>(text).map_or(Conversation::Invalid, |w| w.0)
}

#[derive(Deserialize)]
struct InitRow {
    init: Object<Init>,
}

#[derive(Deserialize)]
struct ColdStepRow {
    step_update: Object<ColdStep>,
}

#[derive(Deserialize)]
struct ColdStep {
    step_type: StepType,
    state: StepState,
}

#[derive(Deserialize)]
struct StepRow {
    step_update: Object<Step>,
}

#[derive(Deserialize)]
struct ResultRow {
    result: Object<Ending>,
}

#[derive(Deserialize)]
pub(super) struct Init {
    pub(super) model: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct Step {
    pub(super) step_type: StepType,
    pub(super) state: StepState,
    #[serde(default)]
    pub(super) usage: Object<Usage>,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum StepType {
    AgentResponse,
    #[serde(other)]
    Other,
}

#[derive(Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum StepState {
    Done,
    #[serde(other)]
    Other,
}

/// A turn's `result`. Its `status` is named; `denied_actions`, where
/// present, is a list, and where absent is no denial, as every recorded
/// success shows.
#[derive(Deserialize)]
pub(super) struct Ending {
    pub(super) status: Status,
    #[serde(default)]
    pub(super) response: String,
    #[serde(default)]
    pub(super) usage: Object<Usage>,
    pub(super) num_turns: Option<u64>,
    #[serde(default)]
    pub(super) denied_actions: Vec<Object<Denied>>,
    pub(super) error: Option<String>,
}

impl Ending {
    pub(super) fn signed_out(&self) -> bool {
        (self.error.as_deref()).is_some_and(|error| error.contains(SIGNED_OUT))
    }
}

/// A result's status: `SUCCESS`, or any other word, which is named.
#[derive(Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub(super) enum Status {
    Success,
    #[serde(untagged)]
    Other(String),
}

impl Status {
    /// The status as agy wrote it.
    pub(super) fn word(&self) -> &str {
        match self {
            Status::Success => "SUCCESS",
            Status::Other(word) => word,
        }
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub(super) struct Usage {
    pub(super) input_tokens: u64,
    pub(super) output_tokens: u64,
    pub(super) thinking_tokens: u64,
    pub(super) cache_read_tokens: u64,
}

impl Usage {
    pub(super) fn add(&mut self, other: &Usage) {
        self.input_tokens = self.input_tokens.saturating_add(other.input_tokens);
        self.output_tokens = self.output_tokens.saturating_add(other.output_tokens);
        self.thinking_tokens = self.thinking_tokens.saturating_add(other.thinking_tokens);
        self.cache_read_tokens = self
            .cache_read_tokens
            .saturating_add(other.cache_read_tokens);
    }
}

#[derive(Deserialize)]
pub(super) struct Denied {
    pub(super) action: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outer_row_arrays_with_object_payloads_are_rejected() {
        let usage =
            r#"{"input_tokens":1,"output_tokens":2,"thinking_tokens":3,"cache_read_tokens":4}"#;
        let init_row_arr = r#"[{"model":"gemini-3-flash"}]"#;
        assert!(
            Event::typed_init(init_row_arr).is_none(),
            "init-row-arr (typed_init)"
        );

        let cold_step_row_arr = r#"[{"step_type":"agent_response","state":"DONE"}]"#;
        let step_row_arr =
            format!(r#"[{{"step_type":"agent_response","state":"DONE","usage":{usage}}}]"#);
        let result_row_arr = format!(
            r#"[{{"status":"SUCCESS","response":"OK","usage":{usage},"num_turns":1,"denied_actions":[{{"action":"run_command"}}],"error":null}}]"#
        );

        let cases: [(&str, Event, bool, &str); 5] = [
            ("init-row-arr", Event::Init, false, init_row_arr),
            (
                "cold-step-row-arr",
                Event::StepUpdate,
                false,
                cold_step_row_arr,
            ),
            ("step-row-arr", Event::StepUpdate, true, &step_row_arr),
            ("result-row-arr-cold", Event::Result, false, &result_row_arr),
            (
                "result-row-arr-rejoin",
                Event::Result,
                true,
                &result_row_arr,
            ),
        ];
        for (label, event, rejoined, text) in cases {
            assert!(event.typed(text, rejoined).is_none(), "{label}");
        }
    }
}
