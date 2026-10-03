//! The one reader of what a turn lists under a key: its tools and its MCP
//! servers. Every listing reading goes through [`Listing`], so a stream
//! shape is read the same way for every fact built on it (#484). Pure,
//! like `measure`.

use serde_json::Value;

use super::{event_type, found_in, Lines, Stream, EXCERPT_CHARS};
use crate::probe::facts::Fact;

/// One entry a listing named, and where it was read: `the <type> event
/// on line <n> of <stream> at <pointer>`.
pub(super) struct Entry<T> {
    pub(super) value: T,
    pub(super) at: String,
}

/// Everything a turn listed under one key, read under one invariant:
///
/// - (a) every stream the turn produced is read: stdout's events, those
///   of every transcript the turn wrote and those on stderr, never one in
///   place of another;
/// - (b) every value under the key, at any depth of every event of every
///   stream, is visited. A value that is not an array, or an array entry
///   the reader cannot name, leaves the listing unread, named by stream,
///   event and pointer. Any line of any stream that is not UTF-8, or not
///   one JSON object naming each key once, leaves every listing of the turn
///   unread, named by stream and line, whatever the line holds, and so
///   does a stream that holds bytes but no event. Stderr alone is text: a
///   line of it that is UTF-8 and no event is read by `tools::said`, and
///   leaves every listing unread only when it names a listing, a server or
///   a tool the probe cannot read whole. Only a stream of no bytes is read
///   as nothing, and named. Nothing is skipped;
/// - (c) every entry named is kept whatever else went unread, so a reach
///   read anywhere stays in [`Listing::entries`] beside any unread value,
///   and an unread value never reads as nothing listed;
/// - (d) [`Listing::fact`] is measured only when every value was read.
pub(super) struct Listing<T> {
    key: &'static str,
    entries: Vec<Entry<T>>,
    listed: Vec<String>,
    unread: Vec<String>,
    /// Those of `unread` that are values rather than lines.
    values: Vec<String>,
    empty: Vec<String>,
}

impl<T: serde::Serialize + PartialEq> Listing<T> {
    /// Read every value under `key` in every stream of the turn, each
    /// array entry named by `name`.
    pub(super) fn read(
        streams: &[Stream],
        key: &'static str,
        name: impl Fn(&Value) -> Option<T>,
    ) -> Listing<T> {
        let mut listing = Listing {
            key,
            entries: Vec::new(),
            listed: Vec::new(),
            unread: Vec::new(),
            values: Vec::new(),
            empty: Vec::new(),
        };
        for stream in streams {
            listing.read_stream(stream, &name);
        }
        listing
    }

    /// Every line of one stream: each event's values under the key, and
    /// each line that is not one JSON object, whatever it holds.
    fn read_stream(&mut self, stream: &Stream, name: &impl Fn(&Value) -> Option<T>) {
        if stream.empty {
            self.empty.push(stream.source.clone());
            return;
        }
        for (line, fault) in &stream.unread {
            self.unread
                .push(format!("line {line} of {} {fault}", stream.source));
        }
        if stream.events.is_empty() && stream.lines == Lines::Events {
            self.unread
                .push(format!("{} holds no JSON event", stream.source));
        }
        for event in &stream.events {
            let at = format!(
                "the {} event on line {} of {}",
                event_type(&event.fields),
                event.line,
                stream.source
            );
            for found in found_in(&event.fields) {
                if found.key == self.key {
                    self.visit(&at, &found.pointer, found.value, name);
                }
            }
        }
    }

    /// One value under the key: an array whose entries are each named,
    /// or unread.
    fn visit(
        &mut self,
        event: &str,
        pointer: &str,
        value: &Value,
        name: &impl Fn(&Value) -> Option<T>,
    ) {
        let Value::Array(items) = value else {
            let json: String = value.to_string().chars().take(EXCERPT_CHARS).collect();
            self.unnamed(format!(
                "{event} holds at {pointer} a value that is not a list: {json}"
            ));
            return;
        };
        let before = self.unread.len();
        for (index, item) in items.iter().enumerate() {
            match name(item) {
                Some(value) => self.entries.push(Entry {
                    value,
                    at: format!("{event} at {pointer}/{index}"),
                }),
                None => self.unnamed(format!(
                    "{event} holds an entry at {pointer}/{index} the probe cannot name"
                )),
            }
        }
        if self.unread.len() == before {
            self.listed
                .push(format!("{event} listed {}: {}", self.key, items.len()));
        }
    }

    /// A value under the key that the reader could not name.
    fn unnamed(&mut self, what: String) {
        self.values.push(what.clone());
        self.unread.push(what);
    }

    /// Each value under the key that went unread, named by stream, event
    /// and pointer.
    pub(super) fn unread_values(self) -> Vec<String> {
        self.values
    }

    /// Every entry named, wherever it was read and whatever went unread.
    pub(super) fn entries(&self) -> &[Entry<T>] {
        &self.entries
    }

    /// What was not read, `None` when every value was.
    pub(super) fn unread(&self) -> Option<String> {
        (!self.unread.is_empty()).then(|| {
            format!(
                "the turn's {} could not be read whole: {}",
                self.key,
                self.unread.join("; ")
            )
        })
    }

    /// The union of every entry, measured only when every value was read
    /// and at least one listing was given, the empty streams named.
    pub(super) fn fact(self) -> Fact<Vec<T>> {
        if let Some(unread) = self.unread() {
            return Fact::unmeasured(unread);
        }
        if self.listed.is_empty() {
            return Fact::unmeasured(format!("no event of the turn listed its {}", self.key));
        }
        let mut evidence = self.listed.join(", and ");
        if !self.empty.is_empty() {
            evidence.push_str(&format!("; empty: {}", self.empty.join(", ")));
        }
        let mut union = Vec::new();
        for entry in self.entries {
            if !union.contains(&entry.value) {
                union.push(entry.value);
            }
        }
        Fact::measured(union, evidence)
    }
}
