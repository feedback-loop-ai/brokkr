//! The one reader of what a turn lists: its tools and its MCP servers.
//! Every listing reading goes through [`Listing`], so a stream shape is
//! read the same way for every fact built on it (#484). Pure, like
//! `measure`.

use super::read::{Said, Server};
use super::{Lines, Stream};
use crate::probe::facts::Fact;

/// One entry a listing named, and where it was read: `the <type> event
/// on line <n> of <stream> at <pointer>`.
pub(super) struct Entry<T> {
    pub(super) value: T,
    pub(super) at: String,
}

/// The listing a typed reader said an event gives, when it gives the
/// one read: its pointer and its entries.
type Pick<T> = fn(&Said) -> Option<(&'static str, &[T])>;

/// The CLI's tools, which the report names `tools`.
pub(super) const TOOLS: (&str, Pick<String>) = ("tools", |said| {
    let Said::Tools { at, names } = said else {
        return None;
    };
    Some((at, names))
});

/// The MCP servers, which the report names `mcp_servers`.
pub(super) const SERVERS: (&str, Pick<Server>) = ("mcp_servers", |said| {
    let Said::Servers { at, servers } = said else {
        return None;
    };
    Some((at, servers))
});

/// Everything a turn listed under one key, read under one invariant:
///
/// - (a) every stream the turn produced is read: stdout's events, those
///   of every transcript the turn wrote and those on stderr, never one in
///   place of another;
/// - (b) every listing an event of every stream gives, as its typed
///   reader decoded it, is read. Any line of any stream the reader did not
///   read whole, whatever it holds, leaves every listing of the turn
///   unread, named by stream and line, and so does an event stream that
///   holds bytes but no event. Only a stream of no bytes is read as
///   nothing, and named. Nothing is skipped;
/// - (c) every entry named is kept whatever else went unread, so a reach
///   read anywhere stays in [`Listing::entries`] beside any unread line;
/// - (d) [`Listing::fact`] is measured only when every line was read.
pub(super) struct Listing<T> {
    key: &'static str,
    pick: Pick<T>,
    entries: Vec<Entry<T>>,
    listed: Vec<String>,
    unread: Vec<String>,
    empty: Vec<String>,
}

impl<T: Clone + serde::Serialize + PartialEq> Listing<T> {
    /// Read the listing `(key, pick)`, [`TOOLS`] or [`SERVERS`], in every
    /// stream of the turn.
    pub(super) fn read(streams: &[Stream], (key, pick): (&'static str, Pick<T>)) -> Listing<T> {
        let mut listing = Listing {
            key,
            pick,
            entries: Vec::new(),
            listed: Vec::new(),
            unread: Vec::new(),
            empty: Vec::new(),
        };
        for stream in streams {
            listing.read_stream(stream);
        }
        listing
    }

    /// Every line of one stream: each event's listings, and each line no
    /// reader read, whatever it holds.
    fn read_stream(&mut self, stream: &Stream) {
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
                event.label, event.line, stream.source
            );
            for (pointer, items) in event.said.iter().filter_map(self.pick) {
                self.listed
                    .push(format!("{at} listed {}: {}", self.key, items.len()));
                let named = items.iter().enumerate().map(|(index, value)| Entry {
                    value: value.clone(),
                    at: format!("{at} at {pointer}/{index}"),
                });
                self.entries.extend(named);
            }
        }
    }

    /// Every entry named, wherever it was read and whatever went unread.
    pub(super) fn entries(&self) -> &[Entry<T>] {
        &self.entries
    }

    /// What was not read, `None` when every line was.
    pub(super) fn unread(&self) -> Option<String> {
        (!self.unread.is_empty()).then(|| {
            format!(
                "the turn's {} could not be read whole: {}",
                self.key,
                self.unread.join("; ")
            )
        })
    }

    /// The union of every entry, measured only when every line was read
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
