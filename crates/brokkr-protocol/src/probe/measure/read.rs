//! What a typed reader takes one line of a harness's streams to say
//! (#484). Each harness has one reader: it decodes an event by its `type`,
//! and `subtype` where the harness uses one, into a closed struct, and a
//! line of text against a closed table of forms. A key no struct names,
//! a type it does not know, a value of another JSON type, or a string no
//! reader consumes leaves the line unread; nothing is scanned for meaning
//! outside the reader that consumes it. Pure, like `measure`.

use std::fmt;

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer};
use serde_json::{Map, Value};

use super::forms::Origin;
use super::{claude, claude_log, codex, codex_log, dsh, Fault, Lines};
use crate::probe::plan::PROMPT;

/// The harnesses whose streams the probe has a typed reader for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Harness {
    Claude,
    Codex,
    Dsh,
}

impl fmt::Display for Harness {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Harness::Claude => "claude",
            Harness::Codex => "codex",
            Harness::Dsh => "dsh",
        })
    }
}

impl Harness {
    /// How the harness's stdout is read: claude and codex print events,
    /// and dsh's headless profile prints its final message as text, the
    /// turn's answer.
    pub(super) fn stdout(self) -> Lines {
        match self {
            Harness::Claude | Harness::Codex => Lines::Events,
            Harness::Dsh => Lines::Text(Origin::Answer),
        }
    }

    /// One JSON object, decoded by the harness's events.
    pub(super) fn event(self, fields: Map<String, Value>) -> Result<Decoded, Fault> {
        match self {
            Harness::Claude => claude::event(fields),
            Harness::Codex => codex::event(fields),
            Harness::Dsh => dsh::event(fields),
        }
    }

    /// One JSON object of a transcript, decoded by the harness's rows:
    /// claude's project log, codex's rollout, and dsh's session log, whose
    /// rows are its events.
    pub(super) fn row(self, fields: Map<String, Value>) -> Result<Decoded, Fault> {
        match self {
            Harness::Claude => claude_log::row(fields),
            Harness::Codex => codex_log::row(fields),
            Harness::Dsh => dsh::event(fields),
        }
    }

    /// One line of text, not the turn's answer, read whole against the
    /// harness's forms; `None` when it is none of them.
    pub(super) fn text(self, line: &str) -> Option<Vec<Said>> {
        self.text_in(line, Origin::Aside, &mut Block::Plain)
    }

    /// One line of text from `origin`, in a stream whose earlier lines
    /// left it in `block`: only dsh opens one, its reasoning, whose line
    /// is read as the model's thinking unless a form reads it whole. Only
    /// dsh prints its answer as text.
    pub(super) fn text_in(
        self,
        line: &str,
        origin: Origin,
        block: &mut Block,
    ) -> Option<Vec<Said>> {
        match self {
            Harness::Claude => claude::text(line),
            Harness::Codex => codex::text(line, Origin::Aside),
            Harness::Dsh => dsh::text(line, origin, block),
        }
    }

    /// What one line says, whether it is an event or text; nothing when
    /// the harness's reader does not read it.
    pub(super) fn said(self, line: &str) -> Vec<Said> {
        let said = match super::strict::object(line) {
            Some(fields) => self.event(fields).ok().map(|decoded| decoded.said),
            None => self.text(line),
        };
        said.unwrap_or_default()
    }
}

/// Where a stream of text stands: in no block, or in dsh's reasoning,
/// which holds the one line after its header.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Block {
    Plain,
    Reasoning,
}

/// An event a reader decoded: its type, `type/subtype` where the harness
/// uses a subtype, and what it says.
pub(super) struct Decoded {
    pub(super) label: String,
    pub(super) said: Vec<Said>,
}

/// One thing a line was read to say.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Said {
    /// The CLI's tools, listed at the pointer `at`.
    Tools {
        at: &'static str,
        names: Vec<String>,
    },
    /// The MCP servers the turn started, listed at the pointer `at`.
    Servers {
        at: &'static str,
        servers: Vec<Server>,
    },
    /// The session the turn runs in, announced under `key`.
    Session {
        key: &'static str,
        id: String,
    },
    Usage(Counted),
    /// A cost, reported at the pointer `at`.
    Cost {
        at: &'static str,
    },
    /// The reply the probe's prompt asks for.
    Reply,
    Refusal(Refused),
    /// An effort refusal: the level refused, and the levels it lists.
    Levels(Levels),
    /// The turn failed, as the event states at the pointer `at`: an error
    /// flag or status, a rejected rate limit, or no turn taken (#484).
    Failed {
        at: &'static str,
    },
    /// A tool ran, as a count or a stop at the pointer `at` states:
    /// `tool` names it where the count does, and is `None` where only a
    /// tool use was stated (#484).
    Ran {
        tool: Option<&'static str>,
        at: &'static str,
    },
    /// The model the turn ran, as the event names it.
    Model(String),
}

/// An effort refusal a recognised form states: the level it refused,
/// and the levels it lists as accepted.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Levels {
    pub(super) refused: String,
    pub(super) accepted: Vec<String>,
}

/// An MCP server a listing names, and the status it gives it.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub(super) struct Server {
    pub(super) name: String,
    pub(super) status: &'static str,
}

/// Usage reported at the pointer `at`, the message it counts when the
/// event names one, and each counter it gives.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Counted {
    pub(super) at: &'static str,
    pub(super) message: Option<String>,
    pub(super) counts: Vec<(&'static str, u64)>,
}

impl Counted {
    /// The usage at `at` for `message`, of each counter `counts` gives.
    pub(super) fn of<const N: usize>(
        at: &'static str,
        message: Option<String>,
        counts: [(&'static str, Option<u64>); N],
    ) -> Said {
        let given = counts
            .into_iter()
            .filter_map(|(name, count)| Some((name, count?)));
        Said::Usage(Counted {
            at,
            message,
            counts: given.collect(),
        })
    }
}

/// A refusal a recognised form states: its class, and what it refuses.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Refused {
    pub(super) class: Class,
    pub(super) object: String,
}

/// The classes of refusal the probe reads by their forms (#484).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Class {
    /// A model the configuration names, refused.
    Config,
    /// The credentials, refused.
    Auth,
    /// A flag or config key, refused.
    Control,
}

/// `fields` decoded as `T`, or the line is unread as not one of the
/// harness's events.
pub(super) fn decode<T: DeserializeOwned>(
    harness: Harness,
    fields: Map<String, Value>,
) -> Result<T, Fault> {
    serde_json::from_value(Value::Object(fields)).map_err(|_| Fault::Undecoded(harness))
}

/// The keys of `envelope` taken out of `fields` and decoded as `E`: the
/// keys every row of a transcript may carry, which its type's struct
/// then does not name again. The rest stay in `fields`.
pub(super) fn envelope<E: DeserializeOwned>(
    harness: Harness,
    fields: &mut Map<String, Value>,
    envelope: &[&str],
) -> Result<E, Fault> {
    let taken = envelope
        .iter()
        .filter_map(|key| fields.remove_entry(*key))
        .collect();
    decode(harness, taken)
}

/// A key that may be absent, but holds a `T` when it is given: `null` is
/// not one, so a listing given as `null` is not read as no listing. A
/// field of this type is `#[serde(default)]`, by its own attribute or its
/// struct's. A key the recordings show `null` is an `Option` instead.
#[derive(Debug)]
pub(super) struct Given<T>(pub(super) Option<T>);

impl<T> Default for Given<T> {
    fn default() -> Given<T> {
        Given(None)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Given<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Given<T>, D::Error> {
        T::deserialize(deserializer).map(|value| Given(Some(value)))
    }
}

/// A value the recordings only ever showed `null`: any other is not one
/// the reader names.
pub(super) type Null = Option<()>;

/// A list the recordings only ever showed empty: an entry is not one the
/// reader names.
pub(super) type Empty = Given<Vec<Never>>;

/// What no value deserializes to.
#[derive(Debug, Deserialize)]
pub(super) enum Never {}

/// The probe's own prompt, as a transcript echoes it: any other string
/// is not one, so an echo never carries a prompt the probe did not give.
#[derive(Debug)]
pub(super) struct Prompt;

impl<'de> Deserialize<'de> for Prompt {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Prompt, D::Error> {
        let text = String::deserialize(deserializer)?;
        if text == PROMPT {
            Ok(Prompt)
        } else {
            Err(serde::de::Error::custom("not the probe's prompt"))
        }
    }
}

/// A UUID in its canonical form: a session, an event or a hook as a
/// harness names it. Any other string under its key is not one.
#[derive(Debug)]
pub(super) struct Uuid(pub(super) String);

impl<'de> Deserialize<'de> for Uuid {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Uuid, D::Error> {
        let text = String::deserialize(deserializer)?;
        let groups: Vec<&str> = text.split('-').collect();
        let canonical = groups.iter().map(|group| group.len()).eq([8, 4, 4, 4, 12])
            && groups
                .iter()
                .all(|group| group.bytes().all(|byte| byte.is_ascii_hexdigit()));
        if canonical {
            Ok(Uuid(text))
        } else {
            Err(serde::de::Error::custom("not a UUID"))
        }
    }
}
