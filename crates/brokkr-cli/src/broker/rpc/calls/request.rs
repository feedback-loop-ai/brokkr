//! What the harness asks (decision 0065 slice two, U6e; MB3, SC4): each
//! frame decoded whole into one closed request before anything is
//! recorded, or refused with its id where it has one. Moved out of
//! [`super`] whole, unchanged, to make room for U8b's retention.

use std::collections::BTreeMap;

use serde::de::IgnoredAny;
use serde::Deserialize;
use serde_json::{Map, Value};

use super::super::{present, shallow, Id, Keyed, Refused, Two};
use crate::broker::session::ensure;

/// The longest tool name a call carries, in bytes (SC4).
const TOOL_MAX: usize = 256;

/// One frame from the harness: the members JSON-RPC 2.0 gives a request or
/// a notification, and no other. An id, where present, is a number or a
/// string, never null.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Asked {
    #[serde(rename = "jsonrpc")]
    _version: Two,
    #[serde(default, deserialize_with = "present")]
    id: Option<Id>,
    method: String,
    params: Option<Value>,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// A harness's initialize parameters: the version it asks for, which must
/// be a string, whatever it names, beside whatever else it says. MB3's
/// version answers any (U9b1c), as MCP's version negotiation has it.
#[derive(Deserialize)]
struct Greeting {
    #[serde(rename = "protocolVersion")]
    _version: String,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// A harness's cancellation: the request it cancels, beside its reason.
#[derive(Deserialize)]
struct Cancelled {
    #[serde(rename = "requestId")]
    request: Id,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// A tools/call's parameters, MCP's members and no other: the tool, its
/// arguments, which are the server's data and never the broker's, and its
/// metadata, which is not passed on. Each that is present is an object,
/// never null.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Called {
    pub(super) name: String,
    #[serde(default, deserialize_with = "present")]
    pub(super) arguments: Option<Map<String, Value>>,
    #[serde(rename = "_meta", default, deserialize_with = "present")]
    _meta: Option<BTreeMap<String, IgnoredAny>>,
    #[serde(flatten)]
    _keyed: Keyed,
}

/// A harness request, checked whole.
pub(super) enum Request {
    Initialize(Id),
    Ping(Id),
    List(Id),
    Call(Id, Called),
    Cancel(Id),
    Told,
    Other(Id),
}

impl Request {
    /// The id the harness would be answered by, none for a notification.
    pub(super) fn id(&self) -> Option<&Id> {
        match self {
            Request::Initialize(id) | Request::Ping(id) | Request::List(id) => Some(id),
            Request::Call(id, _) | Request::Other(id) => Some(id),
            Request::Cancel(_) | Request::Told => None,
        }
    }
}

/// The request the harness's frame `bytes` holds, or why it is refused
/// before anything is recorded, with its id where it has one. A
/// notification needs no answer: one the broker does not take is told
/// nothing.
pub(super) fn decoded(bytes: &[u8]) -> Result<Request, (Option<Id>, Refused)> {
    ensure(shallow(bytes), (None, Refused::Limit))?;
    let asked: Asked = serde_json::from_slice(bytes).or(Err((None, Refused::Invalid)))?;
    let params = asked.params.unwrap_or(Value::Null);
    match (asked.method.as_str(), asked.id) {
        ("notifications/cancelled", None) => {
            let cancelled = Cancelled::deserialize(params);
            Ok(cancelled.map_or(Request::Told, |cancelled| {
                Request::Cancel(cancelled.request)
            }))
        }
        (_, None) => Ok(Request::Told),
        ("initialize", Some(id)) => match Greeting::deserialize(params) {
            Ok(_) => Ok(Request::Initialize(id)),
            Err(_) => Err((Some(id), Refused::Invalid)),
        },
        ("ping", Some(id)) => Ok(Request::Ping(id)),
        ("tools/list", Some(id)) => Ok(Request::List(id)),
        ("tools/call", Some(id)) => match called(params) {
            Ok(called) => Ok(Request::Call(id, called)),
            Err(refused) => Err((Some(id), refused)),
        },
        (_, Some(id)) => Ok(Request::Other(id)),
    }
}

/// A tools/call's parameters: MCP's shape, or invalid; then a name within
/// SC4's byte bound, or the request limit; then in its vocabulary, or
/// invalid. None is recorded, nor its name echoed.
fn called(params: Value) -> Result<Called, Refused> {
    let called = Called::deserialize(params).or(Err(Refused::Invalid))?;
    ensure(called.name.len() <= TOOL_MAX, Refused::Limit)?;
    ensure(vocabulary(&called.name), Refused::Invalid)?;
    Ok(called)
}

/// Whether `tool` is in seat-record's tool vocabulary (SC4): an ASCII
/// letter or digit, then letters, digits and `._:/-`.
fn vocabulary(tool: &str) -> bool {
    let mut bytes = tool.bytes();
    let head = bytes
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric());
    head & bytes.all(|byte| byte.is_ascii_alphanumeric() | b"._:/-".contains(&byte))
}
