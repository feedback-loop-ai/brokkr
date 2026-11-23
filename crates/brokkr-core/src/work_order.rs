//! The typed work-order document and its Markdown form (decision 0116).
//! Text and tracker definitions arrive from the caller.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::{format_description::well_known::Iso8601, Date};
use url::{Host, Url};

use crate::{ending::Ending, shape::Object};

const HEADER_START: &str = "```json";
const HEADER_END: &str = "```";
const INTENT: &str = "## Intent";
const ACCEPTANCE: &str = "## Acceptance";

/// Brokkr's realm-local work-order identifier.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct WorkOrderId(String);

impl WorkOrderId {
    fn read(text: &str) -> Result<Self, Refusal> {
        let digits = text.strip_prefix("WO-").ok_or(Refusal::Id)?;
        let mut bytes = digits.bytes();
        if !matches!(bytes.next(), Some(b'1'..=b'9')) || !bytes.all(|byte| byte.is_ascii_digit()) {
            return Err(Refusal::Id);
        }
        Ok(Self(text.into()))
    }
}

impl FromStr for WorkOrderId {
    type Err = WorkOrderError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::read(text).map_err(WorkOrderError)
    }
}

impl fmt::Display for WorkOrderId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// The state written in a work order's header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkOrderState {
    Proposed,
    Ruled,
    Queued,
    InProgress,
    Shipped,
    Merged,
    Withdrawn,
    Superseded,
    DeliveredElsewhere,
}

/// One external item cited by a work order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reference {
    pub tracker: String,
    pub item: String,
    pub link: String,
}

/// A work order parsed from its version 1 Markdown form.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkOrder {
    header: Header<WorkOrderId, Condition>,
    intent: String,
    acceptance: Vec<String>,
}

impl WorkOrder {
    /// Read a work order with the realm's tracker vocabulary.
    pub fn parse(text: &str, trackers: &Trackers) -> Result<Self, WorkOrderError> {
        Self::read(text, trackers).map_err(WorkOrderError)
    }

    fn read(text: &str, trackers: &Trackers) -> Result<Self, Refusal> {
        let (json, body) = split_header(text)?;
        let header = Header::read(&json)?;
        for reference in &header.references {
            trackers.check(reference)?;
        }
        let (intent, acceptance) = read_body(body)?;
        Ok(Self {
            header,
            intent,
            acceptance,
        })
    }

    /// Write the canonical Markdown document, retaining the text of each field.
    pub fn render(&self) -> String {
        let header = serde_json::to_string_pretty(&self.header).expect("the header serializes");
        let intent = &self.intent;
        let acceptance = self.acceptance.join("\n- ");
        format!("{HEADER_START}\n{header}\n{HEADER_END}\n\n{INTENT}\n\n{intent}\n\n{ACCEPTANCE}\n\n- {acceptance}\n")
    }

    /// The id, once assigned.
    pub fn id(&self) -> Option<&WorkOrderId> {
        self.header.id.as_ref()
    }

    /// The document's title.
    pub fn title(&self) -> &str {
        &self.header.title
    }

    /// The state named in the header.
    pub fn state(&self) -> WorkOrderState {
        self.header.state
    }

    /// The cited external items, as written.
    pub fn references(&self) -> &[Reference] {
        &self.header.references
    }

    /// The distilled statement of the work.
    pub fn intent(&self) -> &str {
        &self.intent
    }

    /// The checkable statements, in document order.
    pub fn acceptance(&self) -> &[String] {
        &self.acceptance
    }
}

/// The header in contract order. The edge first reads id and condition
/// spellings, then checks them into their closed types.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Header<I, C> {
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    id: Option<I>,
    title: String,
    state: WorkOrderState,
    #[serde(deserialize_with = "objects")]
    rulings: Vec<Ruling>,
    #[serde(deserialize_with = "objects")]
    asks: Vec<Ask>,
    constraints: Vec<String>,
    caps: BTreeMap<String, Count>,
    #[serde(deserialize_with = "objects")]
    references: Vec<Reference>,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    parent: Option<I>,
    edges: EdgeList<I, C>,
}

impl Header<String, String> {
    fn read(json: &str) -> Result<Header<WorkOrderId, Condition>, Refusal> {
        let Object(header): Object<Self> =
            serde_json::from_str(json).map_err(|error| Refusal::HeaderJson(error.to_string()))?;
        header.checked()
    }

    fn checked(self) -> Result<Header<WorkOrderId, Condition>, Refusal> {
        if self.title.trim().is_empty() || self.title.contains(['\r', '\n']) {
            return Err(Refusal::Title);
        }
        for ruling in &self.rulings {
            ruling.check()?;
        }
        for ask in &self.asks {
            ask.check(self.rulings.len())?;
        }
        if !self.caps.keys().all(|key| crate_name(key)) {
            return Err(Refusal::CapCrate);
        }
        Ok(Header {
            id: self.id.as_deref().map(WorkOrderId::read).transpose()?,
            title: self.title,
            state: self.state,
            rulings: self.rulings,
            asks: self.asks,
            constraints: self.constraints,
            caps: self.caps,
            references: self.references,
            parent: self.parent.as_deref().map(WorkOrderId::read).transpose()?,
            edges: EdgeList(
                self.edges
                    .0
                    .iter()
                    .map(Edge::checked)
                    .collect::<Result<_, _>>()?,
            ),
        })
    }
}

fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}

fn objects<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Vec<T>, D::Error> {
    let items = Vec::<Object<T>>::deserialize(deserializer)?;
    Ok(items.into_iter().map(|Object(item)| item).collect())
}

// A JSON Schema integer can have a decimal or exponent spelling. The
// contract limits counts to the range that fits in a u32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct Count(u32);

impl<'de> Deserialize<'de> for Count {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::read(value).ok_or_else(|| {
            serde::de::Error::invalid_value(
                serde::de::Unexpected::Float(value),
                &"an unsigned 32-bit whole number",
            )
        })
    }
}

impl Count {
    fn read(value: f64) -> Option<Self> {
        ((0.0..=f64::from(u32::MAX)).contains(&value) && value.fract() == 0.0)
            .then_some(Self(value as u32))
    }
}

fn crate_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ruling {
    words: String,
    date: String,
    cite: String,
}

impl Ruling {
    fn check(&self) -> Result<(), Refusal> {
        if !Date::parse(&self.date, &Iso8601::DATE).is_ok_and(|date| date.to_string() == self.date)
        {
            return Err(Refusal::RulingDate);
        }
        if https_uri_host(&self.cite).is_none() {
            return Err(Refusal::RulingCite);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Ask {
    number: Count,
    question: String,
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    ruling: Option<Count>,
}

impl Ask {
    fn check(&self, rulings: usize) -> Result<(), Refusal> {
        if self.number.0 == 0 {
            return Err(Refusal::AskNumber);
        }
        if self
            .ruling
            .is_some_and(|index| index.0 == 0 || index.0 as usize > rulings)
        {
            return Err(Refusal::AskRuling);
        }
        Ok(())
    }
}

// The wrapper lets Header derive its bounds while admitting only object-shaped edges.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
struct EdgeList<I, C>(Vec<Edge<I, C>>);

impl<'de, I: Deserialize<'de>, C: Deserialize<'de>> Deserialize<'de> for EdgeList<I, C> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        objects(deserializer).map(Self)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Edge<I, C> {
    awaits: I,
    condition: C,
}

impl Edge<String, String> {
    fn checked(&self) -> Result<Edge<WorkOrderId, Condition>, Refusal> {
        Ok(Edge {
            awaits: WorkOrderId::read(&self.awaits)?,
            condition: Condition::read(&self.condition)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Condition {
    Completed,
    Ended,
    Merged,
    Ending(Ending),
}

impl Condition {
    fn read(word: &str) -> Result<Self, Refusal> {
        match word {
            "completed" => Ok(Self::Completed),
            "ended" => Ok(Self::Ended),
            "merged" => Ok(Self::Merged),
            ending => {
                let ending = Ending::named(ending).map_err(|_| Refusal::Condition)?;
                if ending == Ending::Unclassified {
                    return Err(Refusal::Unclassified);
                }
                Ok(Self::Ending(ending))
            }
        }
    }

    fn word(self) -> &'static str {
        match self {
            Self::Completed => "completed",
            Self::Ended => "ended",
            Self::Merged => "merged",
            Self::Ending(ending) => ending.word(),
        }
    }
}

impl Serialize for Condition {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.word())
    }
}

/// A caller-supplied vocabulary of tracker words and permitted link hosts.
#[derive(Debug)]
pub struct Trackers(BTreeMap<String, Vec<HostPattern>>);

impl Trackers {
    /// Read one JSON definition per tracker. A host is an exact DNS name
    /// or a leading wildcard matching its subdomains.
    pub fn from_definitions<'a>(
        definitions: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, WorkOrderError> {
        let mut trackers = Self(BTreeMap::new());
        for definition in definitions {
            trackers.add(definition).map_err(WorkOrderError)?;
        }
        Ok(trackers)
    }

    fn add(&mut self, definition: &str) -> Result<(), Refusal> {
        let (word, hosts) = TrackerDefinition::read(definition)?;
        if self.0.insert(word, hosts).is_some() {
            return Err(Refusal::DuplicateTracker);
        }
        Ok(())
    }

    fn check(&self, reference: &Reference) -> Result<(), Refusal> {
        let hosts = self
            .0
            .get(&reference.tracker)
            .ok_or_else(|| Refusal::UnknownTracker(reference.tracker.clone()))?;
        let written_host = https_uri_host(&reference.link).ok_or(Refusal::ReferenceLink)?;
        // Check the host a client would use, including URL host normalization.
        let url = Url::parse(&reference.link).ok();
        let host = url.as_ref().and_then(Url::host_str);
        if host.is_some_and(|host| hosts.iter().any(|pattern| pattern.matches(host))) {
            return Ok(());
        }
        Err(Refusal::ReferenceHost {
            tracker: reference.tracker.clone(),
            host: host.unwrap_or(written_host).into(),
        })
    }
}

// Validate the written HTTPS URI before any client URL parser can repair it.
// The contract's httpsLink uses the same RFC 3986 syntax and port range.
fn https_uri_host(text: &str) -> Option<&str> {
    let (scheme, rest) = text.split_once("://")?;
    if !scheme.eq_ignore_ascii_case("https") {
        return None;
    }
    let (authority, tail) = rest.split_at(rest.find(['/', '?', '#']).unwrap_or(rest.len()));
    let host_port = match authority.split_once('@') {
        Some((userinfo, host_port)) if uri_chars(userinfo, b":") => host_port,
        Some(_) => return None,
        None => authority,
    };
    if !tail.splitn(2, '#').all(|part| uri_chars(part, b":@/?")) {
        return None;
    }
    host_and_port(host_port)
}

fn host_and_port(authority: &str) -> Option<&str> {
    let (host, port) = if authority.starts_with('[') {
        let (host, _) = authority.split_once(']')?;
        let (host, port) = authority.split_at(host.len() + 1);
        if !matches!(Host::parse(host), Ok(Host::Ipv6(_))) {
            return None;
        }
        (host, port)
    } else {
        let (host, port) = authority.split_at(authority.find(':').unwrap_or(authority.len()));
        if host.is_empty() || !uri_chars(host, b"") {
            return None;
        }
        (host, port)
    };
    if port.is_empty() {
        return Some(host);
    }
    let digits = port.strip_prefix(':')?;
    (digits.is_empty()
        || (digits.bytes().all(|byte| byte.is_ascii_digit()) && digits.parse::<u16>().is_ok()))
    .then_some(host)
}

fn uri_chars(text: &str, delimiters: &[u8]) -> bool {
    let bytes = text.as_bytes();
    bytes.iter().enumerate().all(|(index, &byte)| {
        if byte == b'%' {
            return bytes
                .get(index + 1..index + 3)
                .is_some_and(|escape| escape.iter().all(u8::is_ascii_hexdigit));
        }
        byte.is_ascii_alphanumeric()
            || b"-._~!$&'()*+,;=".contains(&byte)
            || delimiters.contains(&byte)
    })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackerDefinition {
    tracker: String,
    hosts: Vec<String>,
    #[serde(default, rename = "description")]
    _description: String,
}

impl TrackerDefinition {
    fn read(text: &str) -> Result<(String, Vec<HostPattern>), Refusal> {
        let Object(definition): Object<Self> =
            serde_json::from_str(text).map_err(|error| Refusal::TrackerJson(error.to_string()))?;
        let mut bytes = definition.tracker.bytes();
        if !matches!(bytes.next(), Some(b'a'..=b'z'))
            || !bytes.all(|byte| matches!(byte, b'a'..=b'z' | b'0'..=b'9' | b'-'))
        {
            return Err(Refusal::TrackerWord);
        }
        if definition.hosts.is_empty() {
            return Err(Refusal::TrackerHosts);
        }
        let hosts = definition
            .hosts
            .iter()
            .map(|host| HostPattern::read(host))
            .collect::<Result<_, _>>()?;
        Ok((definition.tracker, hosts))
    }
}

#[derive(Debug)]
enum HostPattern {
    Exact(String),
    Subdomains(String),
}

impl HostPattern {
    fn read(text: &str) -> Result<Self, Refusal> {
        let (domain, pattern) = text
            .strip_prefix("*.")
            .map_or((text, Self::Exact as fn(String) -> Self), |domain| {
                (domain, Self::Subdomains)
            });
        if !domain.split('.').all(host_label) {
            return Err(Refusal::TrackerHostPattern);
        }
        Ok(pattern(domain.to_ascii_lowercase()))
    }

    fn matches(&self, host: &str) -> bool {
        match self {
            Self::Exact(domain) => host == domain,
            Self::Subdomains(domain) => host
                .strip_suffix(domain)
                .and_then(|prefix| prefix.strip_suffix('.'))
                .is_some_and(|prefix| prefix.split('.').all(host_label)),
        }
    }
}

fn host_label(label: &str) -> bool {
    !label.is_empty()
        && !label.starts_with('-')
        && !label.ends_with('-')
        && label
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
}

fn split_header(text: &str) -> Result<(String, std::str::Lines<'_>), Refusal> {
    let mut lines = text.lines();
    if lines.next() != Some(HEADER_START) {
        return Err(Refusal::HeaderStart);
    }
    let mut json = Vec::new();
    for line in lines.by_ref() {
        if line == HEADER_END {
            return Ok((json.join("\n"), lines));
        }
        json.push(line);
    }
    Err(Refusal::HeaderEnd)
}

enum Section {
    Intent,
    Acceptance,
    Other,
}

fn section(line: &str) -> Option<Section> {
    let line = line.trim();
    match line {
        INTENT => Some(Section::Intent),
        ACCEPTANCE => Some(Section::Acceptance),
        _ => {
            let after = line.strip_prefix("##")?;
            (after.is_empty() || after.starts_with([' ', '\t'])).then_some(Section::Other)
        }
    }
}

fn read_body(mut lines: std::str::Lines<'_>) -> Result<(String, Vec<String>), Refusal> {
    match lines.find(|line| !line.trim().is_empty()).and_then(section) {
        Some(Section::Intent) => {}
        Some(Section::Other) => return Err(Refusal::OtherSection),
        Some(Section::Acceptance) | None => return Err(Refusal::MissingIntent),
    }
    let mut intent = Vec::new();
    for line in lines.by_ref() {
        match section(line) {
            Some(Section::Intent) => return Err(Refusal::RepeatedIntent),
            Some(Section::Other) => return Err(Refusal::OtherSection),
            Some(Section::Acceptance) => {
                let intent = intent.join("\n");
                let intent = intent.trim();
                if intent.is_empty() {
                    return Err(Refusal::EmptyIntent);
                }
                return Ok((intent.to_owned(), read_acceptance(lines)?));
            }
            // A CR left at the end after lines() would become part of
            // the next read's CRLF separator once the intent is rendered.
            None => intent.push(line.trim_end_matches('\r')),
        }
    }
    Err(Refusal::MissingAcceptance)
}

fn read_acceptance(lines: std::str::Lines<'_>) -> Result<Vec<String>, Refusal> {
    let acceptance: Vec<String> = lines
        .filter(|line| !line.trim().is_empty())
        .map(acceptance_item)
        .collect::<Result<_, _>>()?;
    if acceptance.is_empty() {
        return Err(Refusal::EmptyAcceptance);
    }
    Ok(acceptance)
}

fn acceptance_item(line: &str) -> Result<String, Refusal> {
    match section(line) {
        Some(Section::Intent) => return Err(Refusal::RepeatedIntent),
        Some(Section::Acceptance) => return Err(Refusal::RepeatedAcceptance),
        Some(Section::Other) => return Err(Refusal::OtherSection),
        None => {}
    }
    let (marker, statement) = line
        .split_once([' ', '\t'])
        .ok_or(Refusal::AcceptanceItem)?;
    let statement = statement.trim();
    if !list_marker(marker) || statement.is_empty() {
        return Err(Refusal::AcceptanceItem);
    }
    Ok(statement.into())
}

fn list_marker(marker: &str) -> bool {
    matches!(marker, "-" | "+" | "*")
        || marker.strip_suffix(['.', ')']).is_some_and(|digits| {
            (1..=9).contains(&digits.len()) && digits.bytes().all(|byte| byte.is_ascii_digit())
        })
}

/// An operator-readable refusal at a work order's parsing boundary.
#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct WorkOrderError(Refusal);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
enum Refusal {
    #[error("a work order must start with a fenced JSON header")]
    HeaderStart,
    #[error("the work order's JSON header needs a closing fence")]
    HeaderEnd,
    #[error("the work order's JSON header is invalid: {0}")]
    HeaderJson(String),
    #[error("a work order id must be WO- followed by a positive integer without leading zeroes")]
    Id,
    #[error("the work order's title must occupy one non-empty line")]
    Title,
    #[error("a work order ruling date must be a calendar date in YYYY-MM-DD form")]
    RulingDate,
    #[error("a work order ruling citation must be an absolute HTTPS link")]
    RulingCite,
    #[error("a work order ask number must be positive")]
    AskNumber,
    #[error("a work order ask must name an existing ruling by its one-based position")]
    AskRuling,
    #[error("each public-API cap must be keyed by a crate name")]
    CapCrate,
    #[error("a work order edge condition must name completed, ended, merged or an ending")]
    Condition,
    #[error("unclassified cannot be named as a work order edge condition")]
    Unclassified,
    #[error("the tracker definition is invalid: {0}")]
    TrackerJson(String),
    #[error("a tracker word must start with a lowercase letter and use lowercase letters, digits or hyphens")]
    TrackerWord,
    #[error("a tracker definition must list at least one link host")]
    TrackerHosts,
    #[error("a tracker link host pattern must be a DNS name or a DNS suffix starting with '*.'")]
    TrackerHostPattern,
    #[error("each tracker word must have exactly one definition")]
    DuplicateTracker,
    #[error("the work order cites unknown tracker '{0}'")]
    UnknownTracker(String),
    #[error("a work order reference must carry an absolute HTTPS link")]
    ReferenceLink,
    #[error("host '{host}' is outside the link hosts for tracker '{tracker}'")]
    ReferenceHost { tracker: String, host: String },
    #[error("the work order needs an '## Intent' section after its header")]
    MissingIntent,
    #[error("the work order needs an '## Acceptance' section after its intent")]
    MissingAcceptance,
    #[error("the work order repeats its '## Intent' section")]
    RepeatedIntent,
    #[error("the work order repeats its '## Acceptance' section")]
    RepeatedAcceptance,
    #[error("a work order has only '## Intent' and '## Acceptance' sections")]
    OtherSection,
    #[error("the work order's intent must contain prose")]
    EmptyIntent,
    #[error("the work order's acceptance must contain at least one list item")]
    EmptyAcceptance,
    #[error("each work order acceptance item must be a non-empty list item on one line")]
    AcceptanceItem,
}

#[cfg(test)]
mod tests;
