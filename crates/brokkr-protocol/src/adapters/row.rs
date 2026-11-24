//! Raw harness structure and masked text (decision 0114).

use serde_json::Value;

use super::plain_thread_id;
use super::refusal::{bounded_wire_line, REFUSAL_TOKEN_LIMIT};
use crate::secret::{self, BoundSecret};
use crate::PreSessionRefusal;

pub(super) struct Row<'a> {
    raw: &'a Value,
    bindings: &'a [BoundSecret],
    fallback: Option<&'a Value>,
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
        self.string(path) == Some(word)
    }

    pub(super) fn contains(&self, path: &str, token: &str) -> bool {
        self.string(path).is_some_and(|word| word.contains(token))
    }

    pub(super) fn flag(&self, path: &str) -> Option<bool> {
        self.value(path)?.as_bool()
    }

    pub(super) fn count(&self, path: &str) -> Option<i64> {
        self.value(path)?.as_i64()
    }

    pub(super) fn number(&self, path: &str) -> Option<f64> {
        self.value(path)?.as_f64()
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
        Some(secret::mask_text(
            self.string(path)?.as_bytes(),
            self.bindings,
        ))
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
        (row.is("/type", "system") && row.is("/subtype", "init"))
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

/// Shared reads captured before the family fold takes its masked copy.
pub(super) struct FoldInput<'a> {
    pub(super) masked: &'a Value,
    pub(super) refusal: Option<PreSessionRefusal>,
    pub(super) identity: Option<CapturedId>,
}

impl<'a> FoldInput<'a> {
    /// The family fold over the masked row, unless the raw row was already
    /// classified as a refusal to start: that refusal ends the row first,
    /// and the fold never reads it. One home for both families' folds.
    pub(super) fn fold(
        self,
        fold: impl FnOnce(&'a Value, Option<CapturedId>) -> Option<PreSessionRefusal>,
    ) -> Option<PreSessionRefusal> {
        match self.refusal {
            Some(refusal) => Some(refusal),
            None => fold(self.masked, self.identity),
        }
    }
}

#[cfg(test)]
mod tests;
