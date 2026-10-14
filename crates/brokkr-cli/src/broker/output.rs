//! The broker's output edge (decision 0065 slice two, U6f1a; MB4, CR2,
//! SD3): what a served session lets out on its server's behalf, and the one
//! final guard every byte of it passes. A server's response is first read
//! strictly ([`checked`]): a key written twice in one object, or a number
//! serde_json would read as another value, is the server's protocol fault,
//! never repaired. Its result or error is then masked by the shared masker
//! at the positions that hold free text alone ([`REDACTABLE`]), a key
//! masking makes collide with another being a protocol fault too, and
//! serialized once by `canonical::to_bytes` as the one object that holds
//! exactly that result or error ([`prepared`]): the one buffer the harness
//! is answered with beside its own id ([`framed`]), and which U8b retains.
//! The broker's own replies are prepared the same way. Nothing leaves
//! unless the very bytes that leave pass the final guard, `secret::Guard`,
//! which every ledger record passes too: each frame whole, its id and
//! newline included, read as JSON within the session's deadline and
//! cancellation ([`watched`]); and, each read as text, each harness id as
//! it arrives, each tool name before it is recorded, and each line of the
//! server's standard error as it is kept. A check on any form before those
//! bytes proves nothing of them. A bound value that masking free text
//! cannot remove, as in a number, an id, a tool's name, a schema's type or
//! its instance data, cannot be safely masked, and nothing of it leaves.
//! `mask_json`'s own semantics are unchanged.

use std::collections::BTreeMap;

use brokkr_core::canonical;
use brokkr_protocol::secret::{self, BoundSecret, Guard};
use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::pieces::Pieces;
use super::{present, Fatal, Id, Reply, MASKING, PROTOCOL, TIMEOUT};
use crate::broker::session::stderr::Drain;
use crate::broker::session::{ensure, timely, Startup};

/// What a served session holds beside the harness: the server's bound
/// secrets, which every output is masked by; and its standard error,
/// drained beside its output since its startup, whose masked tail its
/// caller writes once the session ends.
pub(in crate::broker) struct Edge<'e, 'b, E> {
    pub(in crate::broker) bindings: &'b [BoundSecret],
    pub(in crate::broker) stderr: &'e mut Drain<'b, E>,
}

/// The response `frame` read exactly: every object in it names each key
/// once ([`canonical::parse_strict`]), and every number in it, as the final
/// guard reads them (`secret::json_numbers`), reads back as the very value
/// written. Anything else is the server's protocol fault.
pub(super) fn checked(frame: &[u8]) -> Result<(), Fatal> {
    let text = std::str::from_utf8(frame).or(Err(PROTOCOL))?;
    canonical::parse_strict(text).or(Err(PROTOCOL))?;
    let numbers = secret::json_numbers(frame);
    ensure(numbers.into_iter().all(exact), PROTOCOL)
}

/// Whether the number `written` reads, as serde_json reads and writes it,
/// as the same decimal value.
fn exact(written: &[u8]) -> bool {
    let read = serde_json::from_slice::<serde_json::Number>(written);
    read.is_ok_and(|read| decimal(written) == decimal(read.to_string().as_bytes()))
}

/// A decimal value: its sign, its significant digits and the power of ten
/// they are scaled by.
type Decimal = (bool, Vec<u8>, i64);

/// The decimal value the JSON number `text` writes, every zero written as
/// the one unsigned zero; none where its exponent is past an `i64`. This is
/// the precision check's reading of the server's own numbers; whether a
/// number is a bound value is `secret::Guard`'s alone, since only that
/// module reads a bound value (decision 0012).
fn decimal(text: &[u8]) -> Option<Decimal> {
    let negative = text.first() == Some(&b'-');
    let unsigned = &text[usize::from(negative)..];
    let scaled = unsigned
        .iter()
        .position(|byte| byte.eq_ignore_ascii_case(&b'e'));
    let (mantissa, power) = unsigned.split_at(scaled.unwrap_or(unsigned.len()));
    let power = std::str::from_utf8(power.get(1..).unwrap_or(b"0")).ok()?;
    let point = mantissa.iter().position(|byte| *byte == b'.');
    let (whole, fraction) = mantissa.split_at(point.unwrap_or(mantissa.len()));
    let fraction = fraction.get(1..).unwrap_or_default();
    let digits: Vec<u8> = whole.iter().chain(fraction).copied().collect();
    let Some(first) = digits.iter().position(|digit| *digit != b'0') else {
        return Some((false, Vec::new(), 0));
    };
    let last = digits.iter().rposition(|digit| *digit != b'0')?;
    let shift =
        i64::try_from(digits.len() - 1 - last).ok()? - i64::try_from(fraction.len()).ok()?;
    let power = power.parse::<i64>().ok()?.checked_add(shift)?;
    Some((negative, digits[first..=last].to_vec(), power))
}

/// What a scan of the edge reads between its numbers (MB4, MB3): `scan`'s
/// deadline, the one the frame it would make has, and the attempt's
/// cancellation. That deadline past, or the attempt cancelled, ends the
/// scan with the timeout cause, which is the session's for each.
pub(super) fn watched(scan: Startup<'_>) -> impl Fn() -> Result<(), Fatal> + '_ {
    move || ensure(timely(scan), TIMEOUT)
}

/// Whether a request id is safe to answer by (MB4): as every frame writes
/// it, as JSON, and as the value it holds, as text, each through the final
/// `guard`. One that is not cannot be masked without losing what it
/// correlates, so the session ends with the masking cause and the id is
/// copied nowhere.
pub(super) fn correlated(id: &Id, guard: &Guard<'_>) -> Result<(), Fatal> {
    let written = serde_json::to_vec(id).unwrap_or_default();
    let held = match id {
        Id::Number(_) => written.clone(),
        Id::Text(text) => text.clone().into_bytes(),
    };
    ensure(guard.json(&written) & guard.text(&held), MASKING)
}

/// What a tool name is recorded as where neither it nor its masked text is
/// safe: the whole name masked.
const UNNAMED: &str = "[secret]";

/// What a harness's call may be recorded, and refused, as (MB4): its tool's
/// own name where the final guard finds it safe as text; else a masked
/// name, the one name a record or refusal ever holds of it.
#[derive(Debug, PartialEq)]
pub(super) enum Named {
    Safe,
    Masked(String),
}

/// What a call to `tool` may be recorded as ([`Named`]): masked by
/// `bindings` where `guard` finds that safe, and else wholly ([`UNNAMED`]),
/// so the raw name reaches no record.
pub(super) fn named(tool: &str, bindings: &[BoundSecret], guard: &Guard<'_>) -> Named {
    let masked = secret::mask_text(tool.as_bytes(), bindings);
    match (guard.text(tool.as_bytes()), guard.text(masked.as_bytes())) {
        (true, _) => Named::Safe,
        (false, true) => Named::Masked(masked),
        (false, false) => Named::Masked(UNNAMED.to_owned()),
    }
}

/// A tools/call result in MCP's shape, as far as the broker reads it: its
/// content blocks, each an object; its word on whether the tool failed;
/// and its structured content and metadata, each an object where present.
#[derive(Deserialize)]
pub(super) struct Answered {
    #[serde(rename = "content")]
    _content: Vec<Map<String, Value>>,
    #[serde(rename = "isError", default, deserialize_with = "present")]
    pub(super) error: Option<bool>,
    #[serde(rename = "structuredContent", default, deserialize_with = "present")]
    _structured: Option<Map<String, Value>>,
    #[serde(rename = "_meta", default, deserialize_with = "present")]
    _meta: Option<BTreeMap<String, IgnoredAny>>,
}

/// Which reply a result is, as masking reads it: a tools/call result, the
/// filtered catalog's listing, or a local answer of the broker's own.
#[derive(Debug, Clone, Copy)]
pub(super) enum Kind {
    Call,
    Listing,
    Local,
}

/// Where a member of a result sits, as masking reads it: a position that
/// holds free text masking may redact ([`At::Text`], and a schema's
/// [`At::Described`] where it is a string), server data whose keys and
/// values it may redact alike ([`At::Free`]), a position that leads to
/// those, schemas by name ([`At::Schemas`]), or one masking never touches
/// ([`At::Fixed`]).
#[derive(Debug, Clone, Copy, PartialEq)]
enum At {
    Called,
    Blocks,
    Block,
    TextBlock,
    Listed,
    Tools,
    Tool,
    Annotations,
    Schema,
    Schemas,
    Described,
    Text,
    Free,
    Fixed,
}

/// The only positions masking may change (MB4, CR2): a text block's text,
/// the title and description of a block, a tool, its annotations and any
/// schema it carries, and structured content, keys and values. A member a
/// position does not list is fixed, and inside a schema is schema again,
/// but for the members [`SCHEMA`] names. Every fixed position comes out of
/// masking byte-identical by construction, so one holding a bound value is
/// refused by the final guard: a tool's name, a schema's type, property
/// names, required, a schema's instance data, a block's type, mimeType or
/// uri, annotations beside a title, `isError` and `_meta`. A tool's name is
/// never masked, so the catalog offers the very tools the grant offered and
/// is looked up exactly; a check of that after masking would be
/// unreachable, and is left out:
/// `a_bound_value_in_a_fixed_position_is_refused_and_never_rewritten` is
/// its removal control.
const REDACTABLE: [(At, &str, At); 16] = [
    (At::Called, "content", At::Blocks),
    (At::Called, "structuredContent", At::Free),
    (At::Block, "title", At::Text),
    (At::Block, "description", At::Text),
    (At::TextBlock, "text", At::Text),
    (At::TextBlock, "title", At::Text),
    (At::TextBlock, "description", At::Text),
    (At::Listed, "tools", At::Tools),
    (At::Tool, "title", At::Text),
    (At::Tool, "description", At::Text),
    (At::Tool, "inputSchema", At::Schema),
    (At::Tool, "outputSchema", At::Schema),
    (At::Tool, "annotations", At::Annotations),
    (At::Annotations, "title", At::Text),
    (At::Schema, "title", At::Described),
    (At::Schema, "description", At::Described),
];

/// The members of a schema that are no schema themselves: JSON Schema's
/// instance data, which `enum`, `const`, `default` and `examples` hold,
/// every item of it fixed, so a title or description written inside it is
/// data and never masked; and its schemas by name, each member of which is
/// a schema whatever the name, so a property named `enum` or `title` is
/// read as the schema it is.
const SCHEMA: [(&str, At); 9] = [
    ("enum", At::Fixed),
    ("const", At::Fixed),
    ("default", At::Fixed),
    ("examples", At::Fixed),
    ("properties", At::Schemas),
    ("patternProperties", At::Schemas),
    ("dependentSchemas", At::Schemas),
    ("$defs", At::Schemas),
    ("definitions", At::Schemas),
];

impl Kind {
    /// Where a result of this kind sits.
    fn at(self) -> At {
        match self {
            Kind::Call => At::Called,
            Kind::Listing => At::Listed,
            Kind::Local => At::Fixed,
        }
    }
}

impl At {
    /// Where the member `key` of an object here sits.
    fn member(self, key: &str) -> At {
        let listed = REDACTABLE
            .iter()
            .find(|(within, name, _)| (*within == self) & (*name == key))
            .map(|(_, _, at)| *at);
        let schema = SCHEMA
            .iter()
            .find(|(name, _)| (self == At::Schema) & (*name == key))
            .map(|(_, at)| *at);
        listed.or(schema).unwrap_or(self.unlisted())
    }

    /// Where a member neither [`REDACTABLE`] nor [`SCHEMA`] lists sits.
    fn unlisted(self) -> At {
        match self {
            At::Schema | At::Schemas => At::Schema,
            At::Called
            | At::Blocks
            | At::Block
            | At::TextBlock
            | At::Listed
            | At::Tools
            | At::Tool
            | At::Annotations
            | At::Described
            | At::Text
            | At::Free
            | At::Fixed => At::Fixed,
        }
    }

    /// Where each item of an array here sits.
    fn item(self) -> At {
        match self {
            At::Blocks => At::Block,
            At::Tools => At::Tool,
            At::Schema => At::Schema,
            At::Called
            | At::Block
            | At::TextBlock
            | At::Listed
            | At::Tool
            | At::Annotations
            | At::Schemas
            | At::Described
            | At::Text
            | At::Free
            | At::Fixed => At::Fixed,
        }
    }

    /// Where the object `map` here sits: a content block whose type is
    /// text holds text.
    fn object(self, map: &Map<String, Value>) -> At {
        let text = map.get("type").and_then(Value::as_str) == Some("text");
        match (self, text) {
            (At::Block, true) => At::TextBlock,
            (at, _) => at,
        }
    }
}

/// `value`, sitting `at`, masked by `bindings` where that is free text or
/// free data and nowhere else; a key masking makes collide with another is
/// the server's protocol fault.
fn redact(value: &mut Value, at: At, bindings: &[BoundSecret]) -> Result<(), Fatal> {
    match (at, value) {
        (At::Fixed, _) => Ok(()),
        (At::Free, value) => freely(value, bindings),
        (At::Text | At::Described, Value::String(text)) => {
            *text = secret::mask_text(text.as_bytes(), bindings);
            Ok(())
        }
        (At::Described, value) => redact(value, At::Schema, bindings),
        (at, Value::Object(map)) => {
            let at = at.object(map);
            map.iter_mut()
                .try_for_each(|(key, member)| redact(member, at.member(key), bindings))
        }
        (at, Value::Array(items)) => items
            .iter_mut()
            .try_for_each(|item| redact(item, at.item(), bindings)),
        (_, Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)) => Ok(()),
    }
}

/// The server's data `value` masked whole, keys included, every key it had
/// kept: one masking makes collide with another is a protocol fault.
fn freely(value: &mut Value, bindings: &[BoundSecret]) -> Result<(), Fatal> {
    let before = members(value);
    secret::mask_json(value, bindings);
    ensure(members(value) == before, PROTOCOL)
}

/// How many members every object within `value` holds in all.
fn members(value: &Value) -> usize {
    match value {
        Value::Object(map) => map.len() + map.values().map(members).sum::<usize>(),
        Value::Array(items) => items.iter().map(members).sum(),
        _ => 0,
    }
}

/// `reply` prepared for the harness, a result read as `kind`: masked at the
/// positions [`REDACTABLE`] lists, an error at its message and its data,
/// and serialized once as its canonical bytes, the one buffer every frame
/// answering it holds. The frame is guarded whole ([`framed`]); the buffer
/// is a run of its bytes.
pub(super) fn prepared(
    mut reply: Reply,
    kind: Kind,
    bindings: &[BoundSecret],
) -> Result<Vec<u8>, Fatal> {
    match &mut reply {
        Reply::Result(result) => redact(result, kind.at(), bindings)?,
        Reply::Error(fault) => {
            fault.message = secret::mask_text(fault.message.as_bytes(), bindings);
            let data = fault.data.as_mut();
            data.map_or(Ok(()), |data| freely(data, bindings))?;
        }
    }
    Ok(canonical::to_bytes(
        &serde_json::to_value(&reply).unwrap_or_default(),
    ))
}

/// The frame answering the harness's request `id`, or none, with the
/// prepared object `body`: JSON-RPC's version and that id, then the very
/// bytes of its one member, and its newline, cut at its safe points into
/// the pieces it is written in ([`Pieces`]); or MB4's cause where the final
/// `guard` finds that whole line unsafe, or where it cannot be cut safely,
/// or `watch`'s, where it ends the guard's scan first. Every frame the
/// harness is told is made here.
pub(super) fn framed(
    id: Option<&Id>,
    body: &[u8],
    guard: &Guard<'_>,
    watch: impl FnMut() -> Result<(), Fatal>,
) -> Result<Pieces, Fatal> {
    let named = serde_json::to_vec(&id).unwrap_or_default();
    let head = [&b"{\"jsonrpc\":\"2.0\",\"id\":"[..], &named, b","].concat();
    let line = [&head[..], &body[1..], b"\n"].concat();
    let passed = guard.json_within(&line, watch)?;
    ensure(passed, MASKING)?;
    Pieces::new(line).ok_or(MASKING)
}
