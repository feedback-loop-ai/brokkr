//! An adapter's MCP facts (decision 0065 slice two, U1b; SI1, SI2, MB2):
//! what has been MEASURED about a provider carrying the engine's MCP
//! servers and keeping every other server and reader out, per invocation
//! shape.
//!
//! Four isolation axes are held apart because none implies another (design
//! D2): ambient MCP exclusion, native-tool write confinement, and read
//! isolation of the operator's store and of a child process's environment.
//! A read-only sandbox or a 0600 store proves write confinement at most;
//! neither is a read-isolation result, and nothing here derives one axis
//! from another. A shape no entry names is unmeasured: a measurement of
//! another shape, host, wrapper or harness supplies none, so each entry
//! names the harness and binary it was measured under, and both must be
//! this adapter's own. The harness version and the host are kept, because
//! a result applies only where and to what it was measured.
//!
//! The forms written before these facts — `"unsupported"` and
//! `{flag, servers}` — still load, and grant nothing: their server map is
//! checked as it always was and then discarded, so no server it names can
//! ever reach a seat. The loader in `load.rs` reads the file; this module
//! is handed the parsed value and reads nothing else.

use std::collections::BTreeMap;
use std::fmt;

use brokkr_protocol::adapters::AdapterKind;
use serde::Deserialize;
use serde_json::{Map, Value};
use thiserror::Error;

use super::load::RESUME_TEXT_LIMIT;
use super::{valid_name, HarnessHands, ResumeAssessment, NAME_GRAMMAR};

/// Which invocation a measurement covers. A replacement and each named
/// resume shape are measured on their own: none borrows another's result.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpInvocation {
    Cold,
    Replacement,
    /// A resume shape the adapter's own `resume` assessment declares.
    Resume(String),
}

/// Which hands were in force when the shape was measured: the box
/// (decision 0043), the harness's own sandbox (decision 0046), or none.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpHands {
    Boxed,
    Harness,
    #[serde(rename = "none")]
    NoHands,
}

impl McpHands {
    /// The word a `resume` shape's `hands` and the wire both write.
    fn word(self) -> &'static str {
        match self {
            McpHands::Boxed => "boxed",
            McpHands::Harness => "harness",
            McpHands::NoHands => "none",
        }
    }
}

/// The host a shape was measured on: the supported hosts (decision 0063).
/// A result from one host qualifies nothing on the other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum McpHost {
    Linux,
    Macos,
}

/// One invocation shape: an invocation under one hands mode on one host.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct McpShape {
    pub invocation: McpInvocation,
    pub hands: McpHands,
    pub host: McpHost,
}

impl fmt::Display for McpShape {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let hands = self.hands.word();
        let host = match self.host {
            McpHost::Linux => "linux",
            McpHost::Macos => "macos",
        };
        let invocation = match &self.invocation {
            McpInvocation::Cold => "cold".to_string(),
            McpInvocation::Replacement => "replacement".to_string(),
            McpInvocation::Resume(name) => format!("resume {name}"),
        };
        write!(
            formatter,
            "shape '{invocation}' with hands '{hands}' on {host}"
        )
    }
}

/// Why nothing is known: no entry, only the legacy form, or a declared
/// reason. Three causes, one outcome — none of them is evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpUnmeasured {
    Absent,
    Legacy,
    Declared(String),
}

/// One measured fact. `Measured` names the evidence that the isolation or
/// carriage holds; `Unsupported` names the measured reason it does not.
/// `Inapplicable` is only ever the exec harness's: no model MCP surface,
/// which is neither a measurement nor its absence.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(from = "AxisWire")]
pub enum McpAxis {
    Measured { evidence: String },
    Unsupported { reason: String },
    Unmeasured(McpUnmeasured),
    Inapplicable { reason: String },
}

/// One shape's four isolation axes, each its own measurement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpIsolation {
    pub ambient: McpAxis,
    pub native_write: McpAxis,
    pub store_read: McpAxis,
    pub process_read: McpAxis,
}

/// One measured shape: the harness version it was measured on and its
/// four axes. Its harness and binary are the adapter's own, checked at
/// load, so they are not kept twice.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct McpMeasurement {
    pub version: String,
    pub isolation: McpIsolation,
}

/// An adapter's whole `mcp` declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpSupport {
    /// `"unsupported"` (no flag) or `{flag, servers}`: grants nothing and
    /// measures nothing. The flag is kept as written for the status page;
    /// no seat is passed it, and the server map is not kept at all.
    Legacy { flag: Option<String> },
    /// The exec harness's declaration that it has no model MCP surface.
    Inapplicable { reason: String },
    /// Typed facts: whether the harness can carry an engine server, and
    /// each measured shape's version and isolation.
    Declared {
        carriage: McpAxis,
        shapes: BTreeMap<McpShape, McpMeasurement>,
    },
}

impl McpSupport {
    /// Whether this provider can load an engine-written MCP server. Read
    /// by tests until a broker consumes it.
    #[cfg(test)]
    fn carriage(&self) -> McpAxis {
        match self {
            McpSupport::Legacy { .. } => McpAxis::Unmeasured(McpUnmeasured::Legacy),
            McpSupport::Inapplicable { reason } => McpAxis::Inapplicable {
                reason: reason.clone(),
            },
            McpSupport::Declared { carriage, .. } => carriage.clone(),
        }
    }

    /// What is known of one shape. A shape no entry names reads
    /// `Unmeasured(Absent)` on every axis: never another shape's result.
    /// Read by tests until a broker consumes it.
    #[cfg(test)]
    fn isolation(&self, shape: &McpShape) -> McpIsolation {
        let every = |axis: McpAxis| McpIsolation {
            ambient: axis.clone(),
            native_write: axis.clone(),
            store_read: axis.clone(),
            process_read: axis,
        };
        match self {
            McpSupport::Legacy { .. } | McpSupport::Inapplicable { .. } => every(self.carriage()),
            McpSupport::Declared { shapes, .. } => match shapes.get(shape) {
                Some(measurement) => measurement.isolation.clone(),
                None => every(McpAxis::Unmeasured(McpUnmeasured::Absent)),
            },
        }
    }

    /// Closed decoding of the value `load.rs` read for `mcp`, where the
    /// key has always been checked: `None` is the bare `"unsupported"`,
    /// and the loader has already refused every other non-object. Each
    /// shape is admitted against the rest of the adapter by
    /// [`McpDecoded::admit`] once that rest is read.
    pub(super) fn decode(
        what: &str,
        declared: Option<&Value>,
        driver: &[String],
    ) -> Result<McpDecoded, McpRefusal> {
        decode_form(declared, driver).map_err(|problem| McpRefusal::new(what, problem))
    }
}

/// A decoded declaration, each shape already measured under the driver's
/// harness, and the binary each was measured on, not yet checked against
/// the adapter's binary, hands and resume shapes.
pub(super) struct McpDecoded {
    support: McpSupport,
    measured_on: Vec<(McpShape, String)>,
}

impl McpDecoded {
    pub(super) fn admit(self, context: &McpContext<'_>) -> Result<McpSupport, McpRefusal> {
        for (shape, binary) in &self.measured_on {
            admit(shape, binary, context)
                .map_err(|problem| McpRefusal::new(context.what, problem))?;
        }
        Ok(self.support)
    }
}

/// What the rest of the adapter says that each shape is checked against.
pub(super) struct McpContext<'a> {
    pub what: &'a str,
    pub binary: &'a str,
    pub workspace: bool,
    pub harness: &'a HarnessHands,
    pub resume: &'a ResumeAssessment,
}

/// An `mcp` refusal, naming the adapter file it is in.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("{what} 'mcp' {problem}")]
pub struct McpRefusal {
    pub what: String,
    pub problem: McpError,
}

impl McpRefusal {
    fn new(what: &str, problem: McpError) -> McpRefusal {
        McpRefusal {
            what: what.to_string(),
            problem,
        }
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum McpError {
    #[error("has unknown key '{0}'; known keys: flag, servers, carriage, shapes, inapplicable")]
    UnknownKey(String),
    #[error(
        "mixes forms; write the legacy 'flag' and 'servers', the typed 'carriage' and \
         'shapes', or 'inapplicable' alone"
    )]
    MixedForms,
    #[error("does not decode: {0}")]
    Decode(String),
    #[error("needs a non-empty string 'flag'")]
    LegacyFlag,
    #[error("needs 'servers' as an object of strings")]
    LegacyServers,
    #[error("'servers' names '{0}', which does not match {NAME_GRAMMAR}")]
    LegacyServer(String),
    #[error("'servers.{0}' must be a non-empty string")]
    LegacyServerPath(String),
    #[error(
        "is 'inapplicable', but harness '{0}' serves a model; only the exec harness has no \
         model MCP surface"
    )]
    ModelHarness(String),
    #[error("declares {0} twice")]
    DuplicateShape(McpShape),
    #[error(
        "{shape} was measured under harness '{measured}', not this adapter's driver harness \
         '{harness}'; another harness's or wrapper's evidence qualifies nothing here"
    )]
    BorrowedHarness {
        shape: McpShape,
        measured: String,
        harness: String,
    },
    #[error(
        "{shape} was measured on '{measured}', not on this adapter's binary '{binary}'; \
         another harness's or wrapper's evidence qualifies nothing here"
    )]
    BorrowedBinary {
        shape: McpShape,
        measured: String,
        binary: String,
    },
    #[error("{0} names hands this adapter does not declare; a measurement supplies no hands")]
    UndeclaredHands(McpShape),
    #[error("{0} names a resume shape this adapter's 'resume' does not declare")]
    UndeclaredResume(McpShape),
    #[error(
        "{shape} names a resume shape this adapter's 'resume' declares under hands \
         '{declared}'; a measurement under other hands qualifies nothing"
    )]
    ResumeHands { shape: McpShape, declared: String },
}

/// A measured line: non-empty, bounded and free of control characters, as
/// every other adapter-side reason is bounded.
#[derive(Debug, Deserialize)]
#[serde(try_from = "String")]
struct Text(String);

#[derive(Debug, Error)]
#[error(
    "a measured fact must be a bounded non-empty line of at most {RESUME_TEXT_LIMIT} characters"
)]
struct Unbounded;

impl TryFrom<String> for Text {
    type Error = Unbounded;

    fn try_from(text: String) -> Result<Text, Unbounded> {
        if text.is_empty()
            || text.chars().count() > RESUME_TEXT_LIMIT
            || text.chars().any(char::is_control)
        {
            return Err(Unbounded);
        }
        Ok(Text(text))
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum AxisWire {
    Measured(Text),
    Unsupported(Text),
    Unmeasured(Text),
}

impl From<AxisWire> for McpAxis {
    fn from(wire: AxisWire) -> McpAxis {
        match wire {
            AxisWire::Measured(Text(evidence)) => McpAxis::Measured { evidence },
            AxisWire::Unsupported(Text(reason)) => McpAxis::Unsupported { reason },
            AxisWire::Unmeasured(Text(reason)) => {
                McpAxis::Unmeasured(McpUnmeasured::Declared(reason))
            }
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InapplicableWire {
    inapplicable: Text,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FactsWire {
    carriage: McpAxis,
    shapes: Vec<ShapeWire>,
}

/// What a shape was measured on (SI1): the driver's harness, its binary
/// and version, and the host.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MeasuredOn {
    harness: Text,
    binary: Text,
    version: Text,
    host: McpHost,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ShapeWire {
    invocation: McpInvocation,
    hands: McpHands,
    measured_on: MeasuredOn,
    ambient: McpAxis,
    native_write: McpAxis,
    store_read: McpAxis,
    process_read: McpAxis,
}

const LEGACY_KEYS: [&str; 2] = ["flag", "servers"];
const FACTS_KEYS: [&str; 2] = ["carriage", "shapes"];
const INAPPLICABLE_KEY: &str = "inapplicable";

/// One of three forms, chosen by the keys written and never mixed.
fn decode_form(declared: Option<&Value>, driver: &[String]) -> Result<McpDecoded, McpError> {
    let bare = |support: McpSupport| McpDecoded {
        support,
        measured_on: Vec::new(),
    };
    // The loader hands over `None` for `"unsupported"` and an object
    // otherwise; both legacy readings grant nothing.
    let Some(value @ Value::Object(map)) = declared else {
        return Ok(bare(McpSupport::Legacy { flag: None }));
    };
    if let Some(key) = map.keys().find(|key| {
        !LEGACY_KEYS.contains(&key.as_str())
            && !FACTS_KEYS.contains(&key.as_str())
            && key.as_str() != INAPPLICABLE_KEY
    }) {
        return Err(McpError::UnknownKey(key.clone()));
    }
    let names = |keys: &[&str]| map.keys().any(|key| keys.contains(&key.as_str()));
    match (
        names(&LEGACY_KEYS),
        names(&FACTS_KEYS),
        map.contains_key(INAPPLICABLE_KEY),
    ) {
        (true, false, false) => legacy(map).map(bare),
        (false, false, true) => inapplicable(wire(value)?, driver).map(bare),
        (false, _, false) => facts(wire(value)?, driver),
        (true, _, _) | (false, true, true) => Err(McpError::MixedForms),
    }
}

fn wire<T: serde::de::DeserializeOwned>(value: &Value) -> Result<T, McpError> {
    serde_json::from_value(value.clone()).map_err(|problem| McpError::Decode(problem.to_string()))
}

/// The legacy map is checked as it always was, in the words it always
/// used and with no new bound on its values, then discarded.
fn legacy(map: &Map<String, Value>) -> Result<McpSupport, McpError> {
    let flag = match map.get("flag").and_then(Value::as_str) {
        Some(flag) if !flag.is_empty() => flag.to_string(),
        _ => return Err(McpError::LegacyFlag),
    };
    let Some(servers) = map.get("servers").and_then(Value::as_object) else {
        return Err(McpError::LegacyServers);
    };
    for (name, path) in servers {
        if !valid_name(name) {
            return Err(McpError::LegacyServer(name.clone()));
        }
        if !path.as_str().is_some_and(|path| !path.is_empty()) {
            return Err(McpError::LegacyServerPath(name.clone()));
        }
    }
    Ok(McpSupport::Legacy { flag: Some(flag) })
}

/// Only the exec harness may say it has no model MCP surface: a model
/// harness writing it would excuse itself from every isolation check.
fn inapplicable(wire: InapplicableWire, driver: &[String]) -> Result<McpSupport, McpError> {
    let harness = crate::capabilities::harness_of(driver);
    match AdapterKind::parse(harness) {
        Some(AdapterKind::Exec) => Ok(McpSupport::Inapplicable {
            reason: wire.inapplicable.0,
        }),
        Some(
            AdapterKind::Claude | AdapterKind::Lanetally | AdapterKind::Codex | AdapterKind::Dsh,
        )
        | None => Err(McpError::ModelHarness(harness.to_string())),
    }
}

/// Each shape is checked here against the driver's harness, which is
/// known before the rest of the adapter is read.
fn facts(wire: FactsWire, driver: &[String]) -> Result<McpDecoded, McpError> {
    let driver_harness = crate::capabilities::harness_of(driver);
    let mut shapes = BTreeMap::new();
    let mut measured_on = Vec::new();
    for entry in wire.shapes {
        let MeasuredOn {
            harness,
            binary,
            version,
            host,
        } = entry.measured_on;
        let shape = McpShape {
            invocation: entry.invocation,
            hands: entry.hands,
            host,
        };
        if harness.0 != driver_harness {
            return Err(McpError::BorrowedHarness {
                shape,
                measured: harness.0,
                harness: driver_harness.to_string(),
            });
        }
        measured_on.push((shape.clone(), binary.0));
        let measurement = McpMeasurement {
            version: version.0,
            isolation: McpIsolation {
                ambient: entry.ambient,
                native_write: entry.native_write,
                store_read: entry.store_read,
                process_read: entry.process_read,
            },
        };
        if shapes.insert(shape.clone(), measurement).is_some() {
            return Err(McpError::DuplicateShape(shape));
        }
    }
    Ok(McpDecoded {
        support: McpSupport::Declared {
            carriage: wire.carriage,
            shapes,
        },
        measured_on,
    })
}

/// A shape is this adapter's own: measured on its binary, under hands it
/// declares, and for a resume shape one its `resume` assessment names
/// under the same hands.
fn admit(shape: &McpShape, measured_on: &str, context: &McpContext<'_>) -> Result<(), McpError> {
    if measured_on != context.binary {
        return Err(McpError::BorrowedBinary {
            shape: shape.clone(),
            measured: measured_on.to_string(),
            binary: context.binary.to_string(),
        });
    }
    let hands = match shape.hands {
        McpHands::Boxed => context.workspace,
        McpHands::Harness => context.harness.work.is_some() || context.harness.gate.is_some(),
        McpHands::NoHands => true,
    };
    if !hands {
        return Err(McpError::UndeclaredHands(shape.clone()));
    }
    admit_resume(shape, context.resume)
}

fn admit_resume(shape: &McpShape, resume: &ResumeAssessment) -> Result<(), McpError> {
    match &shape.invocation {
        McpInvocation::Cold | McpInvocation::Replacement => Ok(()),
        McpInvocation::Resume(name) => match resume.shape(name) {
            None => Err(McpError::UndeclaredResume(shape.clone())),
            Some(declared) if declared.hands != shape.hands.word() => Err(McpError::ResumeHands {
                shape: shape.clone(),
                declared: declared.hands.clone(),
            }),
            Some(_) => Ok(()),
        },
    }
}

#[cfg(test)]
mod tests;
