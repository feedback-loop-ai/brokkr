//! Raw harness structure and masked text (decision 0114).

use serde_json::Value;

use super::refusal::{bounded_wire_line, REFUSAL_TOKEN_LIMIT};
use crate::secret::{self, BoundSecret};

pub(super) struct Row<'a> {
    raw: &'a Value,
    bindings: &'a [BoundSecret],
}

impl<'a> Row<'a> {
    pub(super) fn new(raw: &'a Value, bindings: &'a [BoundSecret]) -> Self {
        Self { raw, bindings }
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
        self.raw.pointer(path)?.as_bool()
    }

    pub(super) fn count(&self, path: &str) -> Option<i64> {
        self.raw.pointer(path)?.as_i64()
    }

    pub(super) fn number(&self, path: &str) -> Option<f64> {
        self.raw.pointer(path)?.as_f64()
    }

    pub(super) fn token<T: Copy>(&self, path: &str, table: &[(&str, T)]) -> Option<T> {
        let word = bounded_wire_line(self.string(path)?, REFUSAL_TOKEN_LIMIT);
        table
            .iter()
            .find_map(|&(key, kind)| (word == key).then_some(kind))
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
            self.raw
                .pointer(path)?
                .as_array()?
                .iter()
                .map(move |item| Row::new(item, bindings)),
        )
    }

    fn string(&self, path: &str) -> Option<&'a str> {
        self.raw.pointer(path)?.as_str()
    }
}

#[cfg(test)]
mod tests;
