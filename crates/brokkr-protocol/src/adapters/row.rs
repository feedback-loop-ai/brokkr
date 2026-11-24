//! Raw harness structure and masked text (decision 0114).

use std::ops::Range;

use serde_json::Value;

use super::capability_calls::{Tool, MCP_PREFIX, MCP_SEPARATOR};
use super::plain_thread_id;
use super::qoder::ModelPin;
use super::refusal::{
    bounded_wire_line, parse_nanos, stated_reset, LocalTime, REFUSAL_TOKEN_LIMIT,
};
use super::stream_turn::plain;
use crate::secret::{self, BoundSecret, MaskedSplit};
use crate::PreSessionRefusal;

pub(super) struct Row<'a> {
    raw: &'a Value,
    bindings: &'a [BoundSecret],
    fallback: Option<&'a Value>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) enum Pin {
    Pinned,
    Other { plain: bool },
    Absent,
}

impl<'a> Row<'a> {
    pub(super) fn new(raw: &'a Value, bindings: &'a [BoundSecret]) -> Self {
        Self {
            raw,
            bindings,
            fallback: None,
        }
    }

    /// A Codex machine field falls back to the nested error only when
    /// the outer field is absent. An explicit null takes precedence.
    pub(super) fn with_fallback(&self, path: &str) -> Self {
        Self {
            fallback: self.raw.pointer(path),
            ..*self
        }
    }

    fn value(&self, path: &str) -> Option<&'a Value> {
        self.raw
            .pointer(path)
            .or_else(|| self.fallback?.pointer(path))
    }

    pub(super) fn kind<T: Copy>(&self, path: &str, table: &[(&str, T)]) -> Option<T> {
        let word = self.string(path)?;
        table
            .iter()
            .find_map(|&(key, kind)| (word == key).then_some(kind))
    }

    pub(super) fn is(&self, path: &str, word: &str) -> bool {
        self.equals(path, word)
    }

    pub(super) fn equals(&self, path: &str, expected: &str) -> bool {
        self.string(path) == Some(expected)
    }

    pub(super) fn since(&self, path: &str, boundary: i128) -> Option<bool> {
        parse_nanos(self.string(path)?).map(|time| time >= boundary)
    }

    pub(super) fn present(&self, path: &str) -> bool {
        self.value(path).is_some_and(|value| !value.is_null())
    }

    pub(super) fn is_object(&self, path: &str) -> bool {
        self.value(path).is_some_and(Value::is_object)
    }

    pub(super) fn has(&self, path: &str) -> bool {
        self.value(path).is_some()
    }

    pub(super) fn is_init(&self) -> bool {
        self.is("/type", "system") && self.is("/subtype", "init")
    }

    pub(super) fn pin(&self, path: &str, pin: &ModelPin) -> Pin {
        match self.string(path) {
            Some(model) if model == pin.as_str() => Pin::Pinned,
            Some(model) => Pin::Other {
                plain: plain(model),
            },
            None => Pin::Absent,
        }
    }

    pub(super) fn flag(&self, path: &str) -> Option<bool> {
        self.value(path)?.as_bool()
    }

    pub(super) fn count(&self, path: &str) -> Option<i64> {
        self.value(path)?.as_i64()
    }

    pub(super) fn unsigned_count(&self, path: &str) -> Option<u64> {
        self.value(path)?.as_u64()
    }

    pub(super) fn number(&self, path: &str) -> Option<f64> {
        self.value(path)?.as_f64()
    }

    /// A stated time can leave the row only when its parsed span is clear.
    pub(super) fn reset(&self, path: &str) -> Option<LocalTime> {
        let raw = self.string(path)?;
        let (reset, span) = stated_reset(raw)?;
        (!secret::mask_intersects(raw, self.bindings, span)).then_some(reset)
    }

    pub(super) fn token<T: Copy>(&self, path: &str, table: &[(&str, T)]) -> Option<T> {
        let word = bounded_wire_line(self.string(path)?, REFUSAL_TOKEN_LIMIT);
        table
            .iter()
            .find_map(|&(key, kind)| (word == key).then_some(kind))
    }

    pub(super) fn ident(&self, path: &str) -> Option<Ident> {
        Some(Ident(self.string(path)?.to_owned()))
    }

    pub(super) fn text(&self, path: &str) -> Option<String> {
        self.string(path).map(|text| self.masked(text))
    }

    pub(super) fn tool(&self, path: &str) -> Tool {
        let Some(name) = self.string(path) else {
            return Tool::Missing;
        };
        let Some(rest) = name.strip_prefix(MCP_PREFIX) else {
            return Tool::Named {
                name: self.masked(name),
            };
        };
        let Some(separator) = mcp_separator(rest) else {
            return Tool::McpUnidentified {
                reported: self.masked(name),
            };
        };
        let prefix = MCP_PREFIX.len();
        let separator = prefix + separator.start..prefix + separator.end;
        match secret::mask_split(name, self.bindings, prefix, separator) {
            MaskedSplit::Parts { left, right } => Tool::Mcp {
                server: left,
                tool: right,
            },
            MaskedSplit::Whole(reported) => Tool::McpUnidentified { reported },
        }
    }

    fn masked(&self, text: &str) -> String {
        secret::mask_text(text.as_bytes(), self.bindings)
    }

    pub(super) fn at(&self, path: &str) -> Option<Row<'a>> {
        Some(Self::new(self.value(path)?, self.bindings))
    }

    pub(super) fn items(&self, path: &str) -> Option<impl Iterator<Item = Row<'a>>> {
        let bindings = self.bindings;
        Some(
            self.value(path)?
                .as_array()?
                .iter()
                .map(move |item| Row::new(item, bindings)),
        )
    }

    fn string(&self, path: &str) -> Option<&'a str> {
        self.value(path)?.as_str()
    }
}

/// The first separator after the MCP prefix, with a name on either side.
pub(super) fn mcp_separator(rest: &str) -> Option<Range<usize>> {
    let start = rest.find(MCP_SEPARATOR)?;
    let end = start + MCP_SEPARATOR.len();
    (start != 0 && end < rest.len()).then_some(start..end)
}

/// A raw session id is only compared. Its spelling never reaches the wire.
#[derive(Clone, PartialEq, Eq)]
pub(super) struct Ident(String);

impl Ident {
    pub(super) fn from_offered(handle: &str) -> Self {
        Self(handle.to_owned())
    }

    pub(super) fn is_offered(&self, handle: &str) -> bool {
        self.0 == handle
    }

    /// Admit only a plain thread to the rollout walk, independently of
    /// whether its file has appeared yet.
    pub(super) fn for_rollout(id: Self) -> Option<Self> {
        plain_thread_id(&id.0).then_some(id)
    }

    pub(super) fn names_rollout(&self, name: &str) -> bool {
        plain_thread_id(&self.0)
            && name.match_indices(&self.0).any(|(at, _)| {
                let before = name[..at].chars().next_back();
                let after = name[at + self.0.len()..].chars().next();
                !before.is_some_and(|c| c.is_ascii_alphanumeric())
                    && !after.is_some_and(|c| c.is_ascii_alphanumeric())
            })
    }
}

/// One announced id paired with the only spelling a checkpoint can show.
pub(crate) struct CapturedId {
    pub(super) ident: Ident,
    pub(crate) display: String,
}

impl CapturedId {
    fn read(row: &Row<'_>, path: &str) -> Option<Self> {
        Some(Self {
            ident: row.ident(path)?,
            display: row.text(path)?,
        })
    }

    pub(super) fn init(row: &Row<'_>) -> Option<Self> {
        row.is_init()
            .then(|| Self::read(row, "/session_id"))
            .flatten()
    }

    pub(super) fn claude(row: &Row<'_>) -> Option<Self> {
        if row.is("/type", "result") {
            Self::read(row, "/session_id")
        } else {
            Self::init(row)
        }
    }

    pub(super) fn codex(row: &Row<'_>) -> Option<Self> {
        if row.is("/type", "thread.started") {
            Self::read(row, "/thread_id")
        } else if row.is("/type", "result") {
            Self::read(row, "/session_id")
        } else {
            None
        }
    }
}

/// The input and prior decisions for one family fold.
pub(super) struct FoldInput<R> {
    pub(super) row: R,
    pub(super) refusal: Option<PreSessionRefusal>,
    pub(super) identity: Option<CapturedId>,
}

impl<R> FoldInput<R> {
    /// Run the family fold unless the raw row was already
    /// classified as a refusal to start: that refusal ends the row first,
    /// and the fold never reads it. One home for both families' folds.
    pub(super) fn fold(
        self,
        fold: impl FnOnce(R, Option<CapturedId>) -> Option<PreSessionRefusal>,
    ) -> Option<PreSessionRefusal> {
        match self.refusal {
            Some(refusal) => Some(refusal),
            None => fold(self.row, self.identity),
        }
    }
}

#[cfg(test)]
mod tests;
