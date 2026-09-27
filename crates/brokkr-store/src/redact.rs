//! Redaction for publication (#354): pure text processing over an
//! export and the manifest beside it. Nothing here touches the journal.

use serde_json::Value;

/// Sanitize a canonical export for publication: every absolute
/// filesystem path inside event payload string fields is rewritten to a
/// stable placeholder — `[path-1]`, `[path-2]`, … in first-appearance
/// order, the same original path always to the same placeholder,
/// distinct paths to distinct placeholders — so operator-machine detail
/// (and any username inside a path) never leaves the machine. Envelope
/// fields, seqs, structure, and the recorded hashes are untouched, which
/// means the output's event hashes no longer re-verify by construction:
/// a redacted export is evidence of shape, never of authorship. Hash
/// verification applies only to the verbatim export, and the caller must
/// mark the copy unmistakably (the CLI writes `.redacted.` filenames and
/// a marked manifest). The recognizer covers POSIX (`/`-rooted),
/// Windows drive-letter (`C:/`, `C:\\`), and UNC (`\\\\server`) paths,
/// quote-aware so a quoted path with spaces moves whole; scheme URLs
/// (`://`, `file:///…` included) survive verbatim as a declared bound.
/// A caller publishing more than the journal — the CLI also writes a
/// manifest — scrubs the whole set through one [`Redactor`] instead, so
/// the files it publishes agree.
pub fn redact_export(ndjson: &str) -> Result<String, serde_json::Error> {
    Redactor::learn(&[ndjson]).journal(ndjson)
}

/// One redaction, shared across every document published together.
///
/// A journal and the manifest beside it are one piece of evidence in two
/// files, so they must be scrubbed with ONE vocabulary: the same original
/// path always becomes the same `[path-N]` in both, and machine detail
/// that the journal hides cannot survive in the manifest. Two files
/// scrubbed independently would be worse than either alone — they would
/// disagree about what `[path-1]` means, and the un-scrubbed one would
/// hand back exactly what the other was published to withhold.
pub struct Redactor {
    users: Vec<String>,
    table: PathTable,
}

impl Redactor {
    /// Learn the machine detail in every document that will be scrubbed
    /// together, so the username numbering does not depend on which
    /// document happens to be scrubbed first.
    pub fn learn(sources: &[&str]) -> Redactor {
        // A username learned from a home-directory path is machine
        // detail wherever it reappears — the journal records the
        // operator by OS username — so it is scrubbed even outside
        // paths.
        Redactor {
            users: harvest_usernames(&sources.join("\n")),
            table: PathTable::default(),
        }
    }

    /// Scrub an NDJSON journal: payloads only, envelopes untouched.
    pub fn journal(&mut self, ndjson: &str) -> Result<String, serde_json::Error> {
        let mut out = String::new();
        for line in ndjson.lines().filter(|line| !line.trim().is_empty()) {
            let mut envelope: Value = serde_json::from_str(line)?;
            if let Some(payload) = envelope.get_mut("payload") {
                redact_value(payload, &mut self.table, &self.users);
            }
            out.push_str(&serde_json::to_string(&envelope)?);
            out.push('\n');
        }
        Ok(out)
    }

    /// Scrub a whole document — a run manifest, which is not an event
    /// and has no payload to descend into.
    pub fn document(&mut self, value: &Value) -> Value {
        let mut scrubbed = value.clone();
        redact_value(&mut scrubbed, &mut self.table, &self.users);
        scrubbed
    }
}

/// Every username the export leaks through a home-directory path, in
/// first-appearance order so `[user-N]` numbering is deterministic.
fn harvest_usernames(ndjson: &str) -> Vec<String> {
    let mut found: Vec<(usize, String)> = Vec::new();
    for marker in ["/home/", "/Users/", "\\Users\\", ":/Users/"] {
        for (at, _) in ndjson.match_indices(marker) {
            let user: String = ndjson[at + marker.len()..]
                .chars()
                .take_while(|c| is_username_char(*c))
                .collect();
            if !user.is_empty() {
                found.push((at, user));
            }
        }
    }
    found.sort();
    let mut users = Vec::new();
    for (_, user) in found {
        if !users.contains(&user) {
            users.push(user);
        }
    }
    users
}

/// Original path → placeholder, shared across the whole export so
/// distinctness is preserved journal-wide, not per event.
#[derive(Default)]
struct PathTable {
    placeholders: std::collections::HashMap<String, String>,
}

impl PathTable {
    fn placeholder(&mut self, path: &str) -> &str {
        let next = self.placeholders.len() + 1;
        self.placeholders
            .entry(path.to_string())
            .or_insert_with(|| format!("[path-{next}]"))
    }
}

fn redact_value(value: &mut Value, table: &mut PathTable, users: &[String]) {
    match value {
        Value::String(text) => {
            let replaced = redact_usernames(&redact_string(text, table), users);
            *text = replaced;
        }
        Value::Array(items) => {
            for item in items {
                redact_value(item, table, users);
            }
        }
        Value::Object(fields) => {
            for field in fields.values_mut() {
                redact_value(field, table, users);
            }
        }
        _ => {}
    }
}

/// Bare occurrences of a harvested username become `[user-N]`. Only
/// whole tokens move: a username embedded in a longer path-ish word
/// (`carolyn`, `alice.txt`) is somebody else's text.
fn redact_usernames(input: &str, users: &[String]) -> String {
    let mut text = input.to_string();
    for (index, user) in users.iter().enumerate() {
        let placeholder = format!("[user-{}]", index + 1);
        text = replace_bounded(&text, user, &placeholder);
    }
    text
}

fn replace_bounded(text: &str, needle: &str, placeholder: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(at) = rest.find(needle) {
        let end = at + needle.len();
        let before_ok =
            at == 0 || !is_username_char(rest[..at].chars().next_back().expect("at > 0"));
        let after_ok =
            end == rest.len() || !is_username_char(rest[end..].chars().next().expect("end < len"));
        out.push_str(&rest[..at]);
        if before_ok && after_ok {
            out.push_str(placeholder);
        } else {
            out.push_str(&rest[at..end]);
        }
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn redact_string(input: &str, table: &mut PathTable) -> String {
    let mut out = String::with_capacity(input.len());
    let mut from = 0;
    while let Some((start, end)) = next_absolute_path(input, from) {
        out.push_str(&input[from..start]);
        out.push_str(table.placeholder(&input[start..end]));
        from = end;
    }
    out.push_str(&input[from..]);
    out
}

/// Find the next absolute path: POSIX (`/…`), drive-letter (`C:/…`,
/// `C:\…`), or UNC (`\\server\…`). A path opened right after a quote
/// runs to the matching quote, so spaces inside quoted paths survive as
/// path text; an unquoted one runs to the next separator. `://` never
/// opens a path, so scheme URLs (`file:///…` included) pass verbatim —
/// a declared bound of the scheme, recorded in the manifest marking.
fn next_absolute_path(text: &str, from: usize) -> Option<(usize, usize)> {
    for (offset, _) in text[from..].char_indices() {
        let start = from + offset;
        if is_path_start(text, start) {
            let quoted_by = text[..start]
                .chars()
                .next_back()
                .filter(|character| matches!(character, '\'' | '"' | '`'));
            // A drive-letter path owns the colon at its second
            // character; every other colon ends a path, which is what
            // splits `PATH=/usr/bin:/home/x` into two redactions.
            let drive = text.as_bytes()[start..].get(1) == Some(&b':');
            let end = text[start..]
                .char_indices()
                .skip(1)
                .find_map(|(offset, character)| {
                    let ends = match quoted_by {
                        Some(quote) => character == quote,
                        None => {
                            path_end(character) || (character == ':' && !(drive && offset == 1))
                        }
                    };
                    ends.then_some(start + offset)
                })
                .unwrap_or(text.len());
            // A separator alone (`/`, `end /`) is punctuation, not a
            // path: something must follow the opening marker.
            if text[start..end].chars().count() >= 2 {
                return Some((start, end));
            }
        }
    }
    None
}

fn is_path_start(text: &str, start: usize) -> bool {
    let bytes = &text.as_bytes()[start..];
    let previous = text[..start].chars().next_back();
    let boundary = previous.is_none_or(path_boundary)
        || (previous == Some(':') && bytes.get(1) != Some(&b'/'));
    if !boundary {
        return false;
    }

    bytes.first() == Some(&b'/')
        || (bytes.len() >= 3
            && bytes[0].is_ascii_alphabetic()
            && bytes[1] == b':'
            && matches!(bytes[2], b'/' | b'\\'))
        || bytes.starts_with(b"\\\\")
}

fn path_boundary(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '\'' | '"' | '`' | '=' | '(' | '[' | '{' | ',' | ';' | '|' | '&' | '<' | '>'
        )
}

fn path_end(character: char) -> bool {
    character.is_whitespace()
        || matches!(
            character,
            '\'' | '"' | '`' | ',' | ';' | '|' | '&' | '<' | '>' | ')' | ']' | '}'
        )
}

/// A username token's characters: any alphabetic script counts, so
/// `józef` bounds the same way `alice` does.
fn is_username_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '.' | '_' | '-')
}
