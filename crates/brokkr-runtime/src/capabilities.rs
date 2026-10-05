//! Capabilities are the realm's to grant (decision 0065, slice one).
//!
//! A capability is an ABSTRACTION an office is written against —
//! `web-search`, `web-fetch`, whatever an operator needs next — and four
//! kinds of operator data make it mean something, each read here and
//! nowhere else:
//!
//! - an abstract DEFINITION, `capabilities/<name>.json`: the name and its
//!   classes from the closed set `reads`, `writes`, `egress`. The class is
//!   the abstraction's, never an implementation's (ruling 1);
//! - a TOOL DIALECT, `dialects/tools/<name>.json`, published against
//!   `contracts/tool-dialect.v1.schema.json`: which capability it serves
//!   and the one implementation kind it binds it to (ruling 2);
//! - a realm GRANT, `realms.json`'s v6 `capabilities` map: the dialect, a
//!   tool subset, an office scope, and the dialect's own restriction keys
//!   (ruling 3). Only a realm grants;
//! - an adapter's NATIVE declaration: what a harness can already do, and
//!   how each such power is switched ON and OFF, or that it cannot be, or
//!   that nobody has measured it (ruling 4).
//!
//! From those, [`Authority::resolve`] derives one [`Outcome`] per
//! executable site and provider candidate: what it holds, what it does
//! not and why, the notices for wants it lost, and the native controls
//! its launch is composed with. A seat holds what its office asks for,
//! minus what the seat subtracts, intersected with what the realm grants
//! to that office (ruling 5) — and, independently of every ask, each
//! native power it does not hold is switched OFF or the compile refuses.
//!
//! Nothing here runs anything. Loading a dialect contacts no server, and
//! an `mcp` grant is refused by name until slice two builds its broker.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::{parse_strict, sha256_bytes, to_bytes};
use brokkr_core::realms::{is_name, CapabilityGrant};
use brokkr_protocol::native_controls::{
    self as launch, HeldPower, Identity, NativeExpectation, Segment,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};

mod binding;
mod dialect;
mod gates;
mod manifest;
use crate::bundle::SeatClass;
use binding::Unserved;
pub(crate) use binding::ABSENT_EGRESS_MINIMUM;
pub use binding::{Native, Retention, Unbound};
pub use dialect::{Connection, DialectKind, McpServer, Sends, ToolDialect};
use gates::Cause;
pub(crate) use manifest::Implementation;

/// Why a candidate holds no capability: a [`Cause`], which the ask's
/// strength settles, or SC4's unrepresentable identity, which refuses the
/// compile whatever the strength.
enum Unheld {
    Cause(Cause),
    Identity,
}

impl From<Cause> for Unheld {
    fn from(cause: Cause) -> Unheld {
        Unheld::Cause(cause)
    }
}

/// The realm name of a repository no map names.
pub const UNMAPPED: &str = "<unmapped>";

/// Where the operator's abstract definitions live, relative to the
/// operator's configuration directory.
pub const DEFINITIONS_DIR: &str = "capabilities";

/// Where the operator's tool dialects live, on the same terms.
pub const DIALECTS_DIR: &str = "dialects/tools";

/// The one typed slot a native restriction transport may carry: the
/// canonical JSON of the whole validated restriction object, substituted
/// as one argument value and never as shell text.
pub const RESTRICTIONS_SLOT: &str = "{restrictions_json}";

const DEFINITION_SCHEMA: &str = r#"{
  "type": "object",
  "required": ["name", "classes"],
  "additionalProperties": false,
  "properties": {
    "name": {"type": "string", "minLength": 1},
    "classes": {"type": "array", "minItems": 1, "uniqueItems": true,
                "items": {"enum": ["reads", "writes", "egress"]}}
  }
}"#;

const NATIVE_SCHEMA: &str = r##"{
  "definitions": {
    "reason": {"type": "string", "minLength": 1},
    "names": {"type": "array", "items": {"type": "string", "minLength": 1}},
    "list": {"type": "object", "required": ["flag", "separator"], "additionalProperties": false,
             "properties": {"flag": {"type": "string", "minLength": 1},
                            "separator": {"type": "string", "minLength": 1}}},
    "disposition": {
      "type": "object", "minProperties": 1, "maxProperties": 1, "additionalProperties": false,
      "properties": {
        "argv": {"type": "array", "minItems": 1, "items": {"type": "string", "minLength": 1}},
        "default": {"$ref": "#/definitions/reason"},
        "selection": {"type": "object", "required": ["include", "allow", "deny"],
                      "additionalProperties": false,
                      "properties": {"include": {"$ref": "#/definitions/names"},
                                     "allow": {"$ref": "#/definitions/names"},
                                     "deny": {"$ref": "#/definitions/names"}}},
        "unsupported": {"$ref": "#/definitions/reason"},
        "unmeasured": {"$ref": "#/definitions/reason"}
      }
    }
  },
  "type": "object", "minProperties": 1, "additionalProperties": false,
  "properties": {
    "unmeasured": {"$ref": "#/definitions/reason"},
    "known": {
      "type": "object",
      "propertyNames": {"pattern": "^[a-z0-9][a-z0-9._-]*$"},
      "additionalProperties": {
        "type": "object",
        "required": ["capability", "tools", "on", "off", "restrictions", "evidence"],
        "additionalProperties": false,
        "properties": {
          "capability": {"type": "string", "pattern": "^[a-z0-9][a-z0-9._-]*$"},
          "tools": {"type": "array", "minItems": 1, "uniqueItems": true,
                    "items": {"type": "string", "minLength": 1}},
          "on": {"$ref": "#/definitions/disposition"},
          "off": {"$ref": "#/definitions/disposition"},
          "restrictions": {
            "type": "object", "minProperties": 1, "maxProperties": 1,
            "additionalProperties": false,
            "properties": {
              "unsupported": {"$ref": "#/definitions/reason"},
              "argv": {"type": "array", "minItems": 1,
                       "items": {"type": "string", "minLength": 1}}
            }
          },
          "evidence": {
            "type": "object", "required": ["source", "scope", "limitations"],
            "additionalProperties": false,
            "properties": {"source": {"$ref": "#/definitions/reason"},
                           "scope": {"$ref": "#/definitions/reason"},
                           "limitations": {"$ref": "#/definitions/names"}}
          },
          "authored": {
            "type": "object", "additionalProperties": false,
            "properties": {"flags": {"$ref": "#/definitions/names"},
                           "config_flags": {"$ref": "#/definitions/names"},
                           "config_keys": {"$ref": "#/definitions/names"},
                           "feature_flags": {"$ref": "#/definitions/names"},
                           "features": {"$ref": "#/definitions/names"},
                           "list_flags": {"$ref": "#/definitions/names"},
                           "value_flags": {"$ref": "#/definitions/names"}}
          }
        }
      }
    },
    "selection": {
      "type": "object", "required": ["include", "allow", "deny"], "additionalProperties": false,
      "properties": {"include": {"$ref": "#/definitions/list"},
                     "allow": {"$ref": "#/definitions/list"},
                     "deny": {"$ref": "#/definitions/list"}}
    }
  },
  "oneOf": [
    {"required": ["unmeasured"], "maxProperties": 1},
    {"required": ["known"], "not": {"required": ["unmeasured"]}}
  ]
}"##;

/// The first violation of `schema` by `instance`: WHERE in the document,
/// and which clause of the schema it broke. Never the offending value —
/// a validator's own message quotes the instance, and an instance may be a
/// URL somebody put a credential in.
fn violation(schema: &Value, instance: &Value) -> Result<(), String> {
    let validator = jsonschema::draft7::new(schema).map_err(|error| error.to_string())?;
    match validator.validate(instance) {
        Ok(()) => Ok(()),
        Err(error) => Err(format!(
            "at '{}': it does not satisfy '{}'",
            error.instance_path(),
            error.schema_path()
        )),
    }
}

fn embedded(schema: &str) -> Value {
    serde_json::from_str(schema).expect("an embedded schema is valid JSON")
}

// ------------------------------------------------------------ requests

/// How much an office needs a capability (ruling 1): a `requires` the
/// realm does not grant refuses compilation; a `wants` is dropped and the
/// drop recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strength {
    Requires,
    Wants,
}

impl Strength {
    pub fn word(self) -> &'static str {
        match self {
            Strength::Requires => "requires",
            Strength::Wants => "wants",
        }
    }
}

/// What an office asks for, by abstract name.
pub type Requests = BTreeMap<String, Strength>;

/// Read a `capabilities` request map, as an agent or an inline site
/// writes it. The vocabulary is two words: anything else — a dialect, a
/// tool list, a class override, `true`, `null` — is refused rather than
/// read as optional, because a request is never a grant.
pub fn parse_requests(what: &str, raw: &Value) -> Result<Requests, String> {
    let Some(map) = raw.as_object() else {
        return Err(format!(
            "{what} 'capabilities' must be an object from capability name to \"requires\" or \
             \"wants\"; a request names no dialect, tool or grant, because only realms.json \
             grants (decision 0065 ruling 3)"
        ));
    };
    let mut requests = Requests::new();
    for (name, value) in map {
        if !is_name(name) {
            return Err(format!(
                "{what} requests a capability named '{name}'; a capability name is lowercase \
                 letters, digits, '.', '_' and '-', starting with a letter or digit"
            ));
        }
        let strength = match value.as_str() {
            Some("requires") => Strength::Requires,
            Some("wants") => Strength::Wants,
            _ => {
                return Err(format!(
                    "{what} requests capability '{name}' as {value}; a request is \"requires\" \
                     or \"wants\" and nothing else — a dialect, a tool list, a class or a grant \
                     belongs to realms.json and the operator's definitions (decision 0065 \
                     ruling 3)"
                ))
            }
        };
        requests.insert(name.clone(), strength);
    }
    Ok(requests)
}

/// One executable site's asks: the office they belong to, what remains
/// after the seat's subtraction, and what it subtracted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteAsks {
    /// The execution label: `research`, `review:security`, `verify:checks`.
    pub label: String,
    /// The office: the agent's name, or an inline site's authoring label.
    /// A wrapper that moves the site does not move its office.
    pub office: String,
    pub asks: Requests,
    pub subtracted: Vec<String>,
    /// The executable site's own canonical class, never a container's
    /// (GP1). Every construction names it: there is no default to inherit.
    pub(crate) class: SeatClass,
}

impl SiteAsks {
    /// Office asks minus seat subtractions (ruling 5), at the executable
    /// site's own canonical `class` (GP1). An inline site's map IS its
    /// office's asks. A site naming an agent inherits the agent's asks
    /// when it writes no map; a map it does write is a subset with
    /// unchanged strengths, and what it leaves out is subtracted — `{}`
    /// subtracts everything. A seat never adds and never re-rates.
    pub fn at(
        class: SeatClass,
        label: &str,
        agent: Option<(&str, &Requests)>,
        site: Option<&Value>,
    ) -> Result<SiteAsks, String> {
        let what = format!("seat '{label}'");
        let written = site.map(|raw| parse_requests(&what, raw)).transpose()?;
        let Some((office, office_asks)) = agent else {
            return Ok(SiteAsks {
                label: label.to_string(),
                office: label.to_string(),
                asks: written.unwrap_or_default(),
                subtracted: Vec::new(),
                class,
            });
        };
        let asks = match written {
            None => office_asks.clone(),
            Some(written) => {
                for (name, strength) in &written {
                    match office_asks.get(name) {
                        Some(asked) if asked == strength => {}
                        Some(asked) => {
                            return Err(format!(
                                "{what} (office '{office}') changes capability '{name}' from \
                                 {} to {}; a seat may subtract from its office's asks and never \
                                 change their strength",
                                asked.word(),
                                strength.word()
                            ))
                        }
                        None => {
                            return Err(format!(
                                "{what} (office '{office}') adds capability '{name}', which its \
                                 office does not ask for; a seat may subtract from its office's \
                                 asks and never add to them"
                            ))
                        }
                    }
                }
                written
            }
        };
        Ok(SiteAsks {
            label: label.to_string(),
            office: office.to_string(),
            subtracted: office_asks
                .keys()
                .filter(|name| !asks.contains_key(*name))
                .cloned()
                .collect(),
            asks,
            class,
        })
    }
}

// --------------------------------------------------- operator documents

/// Read one operator document: the file's raw bytes, their digest, and
/// the strict parse of those SAME bytes. The path is proven to stay
/// inside `root` after symlinks resolve. `Ok(None)` is a file that is
/// not there.
fn read_document(root: &Path, relative: &str) -> Result<Option<(Value, String)>, String> {
    // An EMPTY root is the directory the caller stands in — what the
    // default `agents` library root's parent is — and an empty path does
    // not canonicalize; an absolute root replaces the `.` it is joined to.
    let root = &Path::new(".").join(root);
    let path = root.join(relative);
    let Ok(canonical) = path.canonicalize() else {
        return Ok(None);
    };
    let inside = root
        .canonicalize()
        .is_ok_and(|root| canonical.starts_with(root));
    if !inside {
        return Err(format!(
            "'{relative}' resolves outside the operator configuration directory"
        ));
    }
    let bytes = std::fs::read(&canonical).map_err(|error| format!("'{relative}': {error}"))?;
    let text = String::from_utf8_lossy(&bytes);
    let value = parse_strict(&text).map_err(|error| format!("'{relative}': {error}"))?;
    Ok(Some((value, sha256_bytes(&bytes))))
}

fn sorted(classes: &[String]) -> Vec<String> {
    let mut classes = classes.to_vec();
    classes.sort();
    classes
}

/// One abstract capability: its name and its classes (ruling 1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub name: String,
    /// As the operator wrote them; compared as a SET.
    pub classes: Vec<String>,
    /// Relative to the operator's configuration directory.
    pub source: String,
    /// Over the file's raw bytes, so an authored change is a pinned one.
    pub sha256: String,
}

impl Definition {
    pub fn source_of(name: &str) -> String {
        format!("{DEFINITIONS_DIR}/{name}.json")
    }

    fn value(&self) -> Value {
        json!({"source": self.source, "sha256": self.sha256, "classes": self.classes})
    }
}

/// Every definition under the operator's `capabilities/`. A missing
/// directory is an empty set, never built-in or provider-derived
/// metadata: denying a harness's known native power needs no definition.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Definitions(BTreeMap<String, Definition>);

impl Definitions {
    pub fn load(root: &Path) -> Result<Definitions, String> {
        let Ok(entries) = std::fs::read_dir(root.join(DEFINITIONS_DIR)) else {
            return Ok(Definitions::default());
        };
        let mut stems: Vec<String> = entries
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|path| {
                path.extension()
                    .is_some_and(|extension| extension == "json")
            })
            .filter_map(|path| Some(path.file_stem()?.to_str()?.to_string()))
            .collect();
        stems.sort();
        let schema = embedded(DEFINITION_SCHEMA);
        let mut definitions = BTreeMap::new();
        for stem in stems {
            let source = Definition::source_of(&stem);
            let (value, sha256) = read_document(root, &source)?
                .ok_or_else(|| format!("'{source}' names nothing readable"))?;
            violation(&schema, &value).map_err(|problem| {
                format!(
                    "capability definition '{source}' {problem}; a definition is exactly a name \
                     and a non-empty set of classes from reads, writes and egress"
                )
            })?;
            let definition = Definition {
                name: value["name"].as_str().unwrap_or_default().to_string(),
                classes: serde_json::from_value(value["classes"].clone()).unwrap_or_default(),
                source: source.clone(),
                sha256,
            };
            if definition.name != stem || !is_name(&stem) {
                return Err(format!(
                    "capability definition '{source}' names '{}'; a definition's name is its \
                     file's stem, in the capability-name grammar",
                    definition.name
                ));
            }
            definitions.insert(stem, definition);
        }
        Ok(Definitions(definitions))
    }

    pub fn get(&self, name: &str) -> Option<&Definition> {
        self.0.get(name)
    }

    /// CQ2: a request for a capability nobody defined is an invalid
    /// declaration — before requires/wants, before any grant.
    pub fn require(&self, who: impl std::fmt::Display, name: &str) -> Result<&Definition, String> {
        self.get(name).ok_or_else(|| {
            format!(
                "{who}: capability '{name}' has no abstract definition at '{}' in the operator \
                 configuration; declare its classes before requesting it",
                Definition::source_of(name)
            )
        })
    }

    /// Semantic library lint: every capability every loaded agent asks for
    /// resolves to one of THESE definitions — the operator's, never the
    /// library's own directory and never a built-in catalogue. Parsing the
    /// request map is syntax and is not this; a name nobody defined is a
    /// problem whether the agent `wants` or `requires` it, and whichever
    /// seat might later subtract it. One line per missing definition, in
    /// library order, so a reader repairs the whole library at once.
    pub fn lint(&self, library: &crate::Library) -> Vec<String> {
        library
            .agents()
            .flat_map(|agent| {
                let who = format!("agent '{}'", agent.name);
                agent
                    .capabilities
                    .keys()
                    .filter_map(|name| self.require(&who, name).err())
                    .collect::<Vec<_>>()
            })
            .collect()
    }
}

// ------------------------------------------------- native declarations

/// How one native power is switched one way (ruling 4): adapter data in
/// the form `tool_permissions` uses, or the measured reason it cannot be,
/// or the reason nobody knows.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Disposition {
    /// Argv that switches it.
    Argv(Vec<String>),
    /// Nothing to write: this IS the harness's measured default.
    Default(String),
    /// Contributions to the harness's own tool lists.
    Selection(ToolLists),
    /// Measured: it cannot be switched this way.
    Unsupported(String),
    /// Nobody has measured whether it can.
    Unmeasured(String),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct ToolLists {
    pub include: Vec<String>,
    pub allow: Vec<String>,
    pub deny: Vec<String>,
}

/// How a grant's restriction object reaches the harness, if it can.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transport {
    Unsupported(String),
    Argv(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Evidence {
    pub source: String,
    pub scope: String,
    pub limitations: Vec<String>,
}

/// Which AUTHORED arguments would contend with the managed control.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct Authored {
    pub flags: Vec<String>,
    pub config_flags: Vec<String>,
    pub config_keys: Vec<String>,
    pub feature_flags: Vec<String>,
    pub features: Vec<String>,
    pub list_flags: Vec<String>,
    pub value_flags: Vec<String>,
}

/// One native power of a harness, under the adapter's own key.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct NativeCapability {
    /// The abstract capability this power realises.
    pub capability: String,
    pub tools: Vec<String>,
    pub on: Disposition,
    pub off: Disposition,
    pub restrictions: Transport,
    pub evidence: Evidence,
    #[serde(default)]
    pub authored: Authored,
}

/// What an OFF disposition declares of a known native power (ruling 4),
/// read off that disposition and nothing else: the one step of a plan's
/// resolution that decides between switching the power off and refusing
/// the seat. It is never a finding that the power is delivered OFF: only
/// the whole plan, composed by the launch's own composer, says that
/// ([`Authority::resolve`]; operator ruling 4 of 2026-09-23).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Denial<'a> {
    /// The adapter declares a control that switches it off.
    Delivered,
    /// Measured: it cannot be switched off. The seat is refused.
    Impossible(&'a str),
    /// Nobody measured its OFF control. No denial is claimed, and the seat
    /// is refused rather than launched on a guess.
    Unmeasured(&'a str),
}

impl NativeCapability {
    /// What the OFF disposition SAYS, with no question asked of the
    /// provider that would have to deliver it. A declared `Argv` or
    /// `Selection` is a denial only once the whole plan it joins composes
    /// for the serving provider, which is [`Authority::resolve`]'s to say.
    pub fn declared_denial(&self) -> Denial<'_> {
        match &self.off {
            Disposition::Unsupported(reason) => Denial::Impossible(reason),
            Disposition::Unmeasured(reason) => Denial::Unmeasured(reason),
            Disposition::Argv(_) | Disposition::Default(_) | Disposition::Selection(_) => {
                Denial::Delivered
            }
        }
    }
}

/// The driver KIND an adapter's own invocation dispatches, which is the
/// harness whose grammar and launch a seat on that provider consumes.
/// [`OPAQUE_HARNESS`] for a command that dispatches no built-in driver.
pub fn harness_of(driver: &[String]) -> &str {
    match driver {
        [_, marker, name, ..] if marker == "driver" => name,
        _ => OPAQUE_HARNESS,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ListFlag {
    pub flag: String,
    pub separator: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SelectionFlags {
    pub include: ListFlag,
    pub allow: ListFlag,
    pub deny: ListFlag,
}

/// What an adapter says of its harness's own powers. `Unmeasured` is not
/// an empty inventory: it grants nothing, denies nothing, and says so.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeInventory {
    Known {
        known: BTreeMap<String, NativeCapability>,
        selection: Option<SelectionFlags>,
    },
    Unmeasured(String),
}

/// The reading of an adapter written before the ruling.
pub const ABSENT_ASSESSMENT: &str = "the adapter declares no native_capabilities assessment";

impl NativeInventory {
    /// The native capability a concrete tool name belongs to, where this
    /// inventory is known and declares one. An unmeasured inventory owns
    /// no name it can show, so it answers for none.
    pub fn capability_of(&self, tool: &str) -> Option<&str> {
        match self {
            NativeInventory::Known { known, .. } => known
                .values()
                .find(|native| native.tools.iter().any(|name| name == tool))
                .map(|native| native.capability.as_str()),
            NativeInventory::Unmeasured(_) => None,
        }
    }

    /// Read an adapter's `native_capabilities`. Absent is unmeasured with
    /// the absence as its reason — never a verified empty inventory.
    pub fn parse(what: &str, raw: Option<&Value>) -> Result<NativeInventory, String> {
        let Some(raw) = raw else {
            return Ok(NativeInventory::Unmeasured(ABSENT_ASSESSMENT.to_string()));
        };
        violation(&embedded(NATIVE_SCHEMA), raw)
            .map_err(|problem| format!("{what} 'native_capabilities' {problem}"))?;
        if let Some(reason) = raw.get("unmeasured").and_then(Value::as_str) {
            return Ok(NativeInventory::Unmeasured(reason.to_string()));
        }
        let known: BTreeMap<String, NativeCapability> =
            serde_json::from_value(raw["known"].clone()).expect("validated above");
        let selection: Option<SelectionFlags> = raw
            .get("selection")
            .map(|flags| serde_json::from_value(flags.clone()).expect("validated above"));
        for (key, native) in &known {
            let selects = [&native.on, &native.off]
                .iter()
                .any(|disposition| matches!(disposition, Disposition::Selection(_)));
            if selects && selection.is_none() {
                return Err(format!(
                    "{what} 'native_capabilities' key '{key}' uses a selection control, but the \
                     adapter declares no 'selection' list flags to express it with"
                ));
            }
            if let Transport::Argv(template) = &native.restrictions {
                let slots = template
                    .iter()
                    .filter(|part| part.contains(RESTRICTIONS_SLOT))
                    .count();
                if slots != 1 {
                    return Err(format!(
                        "{what} 'native_capabilities' key '{key}' restriction transport carries \
                         {slots} '{RESTRICTIONS_SLOT}' slots; it carries exactly one"
                    ));
                }
            }
        }
        Ok(NativeInventory::Known { known, selection })
    }

    /// Parse every control this declaration states under the grammar of
    /// the harness its adapter dispatches, where the adapter loads and
    /// whichever half a realm will use (rebuild unit 11; operator ruling 2;
    /// design D6 and D11): each ON and OFF argv, the selection lists'
    /// flags, separators and entries, and a restriction transport with the
    /// empty restriction, the only one slice one carries, in its slot. A
    /// harness brokkr models no grammar for (`exec`, an opaque custom
    /// driver) has no final command the engine reads, so nothing is asked
    /// of it here. No refusal echoes a declared token.
    #[expect(
        clippy::excessive_nesting,
        reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
    )]
    pub fn check_declared(&self, what: &str, harness: &str) -> Result<(), String> {
        use launch::grammar::{self, ListKind};
        let NativeInventory::Known { known, selection } = self else {
            return Ok(());
        };
        let Some(table) = grammar::grammar(harness) else {
            return Ok(());
        };
        let what = format!("{what} 'native_capabilities'");
        if let Some(flags) = selection {
            for (slot, kind, list) in [
                ("include", ListKind::Include, &flags.include),
                ("allow", ListKind::Allow, &flags.allow),
                ("deny", ListKind::Deny, &flags.deny),
            ] {
                if grammar::list_of(harness, &list.flag) != Some(kind) {
                    return Err(format!(
                        "{what} selection '{slot}' flag {} is not what the '{harness}' grammar \
                         reads as the '{slot}' tool list",
                        table.label(&list.flag)
                    ));
                }
                grammar::managed_separator(&list.separator)
                    .map_err(|cause| format!("{what} selection '{slot}' separator {cause}"))?;
            }
        }
        let empty = String::from_utf8_lossy(&to_bytes(&json!({}))).into_owned();
        for (key, native) in known {
            for (half, disposition) in [("ON", &native.on), ("OFF", &native.off)] {
                match disposition {
                    Disposition::Argv(argv) => declared_argv(harness, argv)
                        .map_err(|cause| format!("{what} key '{key}' {half} argv {cause}"))?,
                    Disposition::Selection(lists) => {
                        for (slot, entries) in [
                            ("include", &lists.include),
                            ("allow", &lists.allow),
                            ("deny", &lists.deny),
                        ] {
                            for (index, entry) in entries.iter().enumerate() {
                                let cause = match grammar::managed_patterns(entry) {
                                    Ok(patterns) if patterns.len() == 1 => continue,
                                    Ok(_) => {
                                        "joins more than one pattern; a selection entry is one \
                                         managed tool pattern"
                                    }
                                    Err(cause) => cause,
                                };
                                return Err(format!(
                                    "{what} key '{key}' {half} selection '{slot}' entry {} {cause}",
                                    index + 1
                                ));
                            }
                        }
                    }
                    Disposition::Default(_)
                    | Disposition::Unsupported(_)
                    | Disposition::Unmeasured(_) => {}
                }
            }
            if let Transport::Argv(template) = &native.restrictions {
                let argv: Vec<String> = template
                    .iter()
                    .map(|part| part.replace(RESTRICTIONS_SLOT, &empty))
                    .collect();
                declared_argv(harness, &argv).map_err(|cause| {
                    format!(
                        "{what} key '{key}' restriction transport, with the empty restriction \
                         in its slot, {cause}"
                    )
                })?;
            }
        }
        Ok(())
    }
}

/// One declared argv under its modelled harness's grammar: every token is
/// placed, every option it carries has a classified effect, and every
/// managed list and bounded control carries a value the engine can read.
fn declared_argv(harness: &str, argv: &[String]) -> Result<(), String> {
    let command = launch::parse_origin(harness, argv, false)
        .map_err(|refusal| refusal.cause)?
        .expect("a modelled harness has a grammar");
    for node in &command.nodes {
        node.bears_capability()
            .map_err(str::to_string)
            .and_then(|_| launch::declared_values(harness, node))
            .map_err(|cause| format!("cannot be composed: '{}' {cause}", node.name()))?;
    }
    Ok(())
}

/// The harness of a command that dispatches no built-in driver: opaque to
/// the engine, which never sees its final command.
pub const OPAQUE_HARNESS: &str = "<custom>";

/// The seat an adapter-level plan is resolved for ([`Authority::assess`]):
/// a label naming the hypothesis, never a seat a bundle declares.
pub const ADAPTER_SEAT: &str = "adapter-plan";

/// What resolution reads of one provider: its name, its native
/// declaration, and the digest of the file that declaration came from.
#[derive(Debug, Clone, Copy)]
pub struct Serving<'a> {
    pub provider: &'a str,
    /// The driver KIND the provider dispatches — `codex`, `claude`,
    /// `lanetally`, `dsh`, `exec` — or [`OPAQUE_HARNESS`] for a command that
    /// dispatches no built-in driver. Adapters are data, so a provider may
    /// run the codex harness under any name: what a launch consumes and the
    /// powers it is known to carry follow the harness, never the name.
    pub harness: &'a str,
    pub model: Option<&'a str>,
    /// `None` where no adapter answers for the provider at all.
    pub native: Option<(&'a NativeInventory, &'a str)>,
    /// Why no adapter answers, where the adapter data could not be LOADED
    /// at all — the loader's own words. A load that fails is not a
    /// provider with nothing to declare (decision 0066 ruling 1).
    pub unloaded: Option<&'a str>,
    /// The argv before the engine's hands: an inline site's command, or an
    /// agent's composed driver template, pins and local permissions. It is
    /// what the native plan composes with; under an opaque harness it is
    /// also judged for a guarded control, since nothing parses it.
    pub authored: &'a [String],
    /// The fragment the ENGINE appended for the boundary — the adapter's
    /// workspace hands. Composed with, never judged as authored.
    pub fragment: &'a [String],
    /// What of that argv the engine composed from the site's own typed
    /// declarations — how much of `fragment` is the box's hands, and the
    /// local permissions its typed allow lowered to — carried from where
    /// the candidate was composed into the plan (rebuild unit 12-fix-c).
    pub provenance: &'a launch::Provenance,
    /// What the RECIPE itself wrote, by origin (design D5.7): an inline
    /// site's whole command; nothing for an agent candidate, whose
    /// composition is the adapter's and the engine's alone.
    /// Under a harness brokkr drives it carries no capability-bearing
    /// option (operator ruling 1 of 2026-09-23; rebuild unit 12).
    pub written: &'a [String],
}

// ------------------------------------------------------------ authority

/// The operator context one compile authorises against (design D2): the
/// operated realm, what it grants, and the directory its definitions and
/// tool dialects live in. Roots locate operator data; they grant nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityContext {
    pub realm: String,
    pub grants: BTreeMap<String, CapabilityGrant>,
    pub root: PathBuf,
}

impl CapabilityContext {
    /// The explicit no-grant context: what every realm under v1 through
    /// v5 means, and what a repository no map names means.
    pub fn no_grants(realm: &str, root: &Path) -> CapabilityContext {
        CapabilityContext {
            realm: realm.to_string(),
            grants: BTreeMap::new(),
            root: root.to_path_buf(),
        }
    }
}

/// One held capability, fully attributable (ruling 8).
#[derive(Debug, Clone, PartialEq)]
pub struct Holding {
    pub classes: Vec<String>,
    pub dialect: String,
    pub dialect_sha256: String,
    pub definition_sha256: String,
    pub tools: Vec<String>,
    /// The realm's restriction object, exactly as written.
    pub restrictions: Map<String, Value>,
    /// What carries it (SC3), from the dialect's kind.
    pub(crate) implementation: Implementation,
    /// The dialect's declaration beside the grant's veto (CR1).
    pub retention: Retention,
}

/// The native contribution as typed data, before `controls` renders it
/// (decision 0065 slice one, design D5.7): the abstract capabilities it
/// switches ON and OFF, the raw argv — each power's ON or OFF switch and
/// its substituted restriction transport, in key order — and the tool
/// selection still pending its lowering, WITH the adapter's own list flags
/// and separators, so two inventories that differ only in their mappings
/// never yield one contribution. Everything carries native origin from
/// construction; nothing is recovered from the rendered JSON or from a
/// command's bytes.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NativeContribution {
    pub held: Vec<String>,
    pub denied: Vec<String>,
    pub argv: Vec<String>,
    pub selection: launch::Selection,
}

impl NativeContribution {
    /// The contribution as one native segment for the candidate's own
    /// provider and harness, materialized by the launch's own lowering
    /// ([`launch::native_segment`]) from this typed data alone. A
    /// representation that launch cannot consume refuses exactly as the
    /// launch would, so a pending selection is never claimed as argv. What
    /// each holding admits is the sealed expectation's, taken from the
    /// holdings when the plan was resolved (rebuild unit 12-fix).
    pub fn segment(
        &self,
        provider: &str,
        harness: &str,
        expected: &NativeExpectation,
    ) -> Result<Segment, launch::Refusal> {
        launch::native_segment(
            harness,
            &launch::Controls {
                provider: provider.to_string(),
                harness: harness.to_string(),
                inventory: launch::Inventory::Known,
                held: self.held.clone(),
                denied: self.denied.clone(),
                admits: expected.admits(),
                argv: self.argv.clone(),
                selection: self.selection.clone(),
                guards: Vec::new(),
                provenance: launch::Provenance::default(),
            },
        )
    }
}

/// What the launch is composed with.
#[derive(Debug, Clone, PartialEq)]
pub enum NativePlan {
    Known {
        declaration: String,
        on: Vec<String>,
        off: Vec<String>,
        /// The driver input's `native_controls`.
        controls: Value,
        /// The same controls as typed data, before rendering; boxed, as
        /// it carries the whole selection with its mappings.
        contribution: Box<NativeContribution>,
        /// What the plan answers for, sealed from the inventory, the
        /// key-to-holding relation and the typed holdings — never read
        /// back from `controls` or `contribution`.
        expected: NativeExpectation,
    },
    Unmeasured {
        declaration: Option<String>,
        reason: String,
        /// What the engine typed of the site's argv, carried to the driver
        /// as a known plan carries it (operator ruling of 2026-09-29, R5).
        /// Never a lowered allow: that refuses at compile.
        provenance: launch::Provenance,
    },
}

impl NativePlan {
    /// The independent native expectation: a known plan's sealed value, or
    /// an unmeasured inventory with its exact reason — never a known empty
    /// one.
    pub fn expected(&self) -> NativeExpectation {
        match self {
            NativePlan::Known { expected, .. } => expected.clone(),
            NativePlan::Unmeasured { reason, .. } => NativeExpectation::Unmeasured(reason.clone()),
        }
    }

    /// The driver input's `native_controls` for `provider`: the engine-owned
    /// plan the protocol composes the final argv from. An unmeasured
    /// inventory names its provider too, so the driver can tell a provider
    /// with nothing declared from one KNOWN to carry a power it must answer
    /// for (decision 0066 ruling 1).
    pub fn controls(&self, provider: &str, harness: &str) -> Value {
        match self {
            NativePlan::Known { controls, .. } => controls.clone(),
            NativePlan::Unmeasured {
                reason, provenance, ..
            } => {
                let mut controls = json!({
                    "inventory": "unmeasured", "provider": provider, "harness": harness,
                    "reason": reason,
                });
                // A member is written where it types something; absent, the
                // driver reads it as typing nothing, as a known plan's.
                if provenance.hands > 0 {
                    controls["hands"] = json!(provenance.hands);
                }
                if !provenance.local.is_empty() {
                    controls["local"] = json!(provenance.local);
                }
                controls
            }
        }
    }
}

/// The sealed outcome for one site and one provider candidate. Only
/// [`Authority::resolve`] constructs one; the manifest record, the driver
/// input and the prompt paragraph are projections of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub provider: String,
    /// The driver kind that provider dispatches ([`Serving::harness`]).
    pub harness: String,
    pub model: Option<String>,
    pub held: BTreeMap<String, Holding>,
    pub not_held: BTreeMap<String, String>,
    /// One per wanted capability this candidate lost: the capability, and
    /// the complete sentence saying who lost it, where and why.
    pub notices: Vec<(String, String)>,
    pub native: NativePlan,
}

impl Outcome {
    /// The driver input's `native_controls`: the engine-owned plan the
    /// protocol composes the final argv from.
    pub fn controls(&self) -> Value {
        self.native.controls(&self.provider, &self.harness)
    }

    /// Whom this candidate's plan was resolved for (design D5.7): its own
    /// provider, harness and model, never a primary's.
    pub fn identity(&self) -> Identity {
        Identity {
            provider: self.provider.clone(),
            harness: self.harness.clone(),
            model: self.model.clone(),
        }
    }

    /// The driver input's `capabilities`: what the prompt tells the seat.
    pub fn prompt(&self) -> Value {
        let held: Map<String, Value> = self
            .held
            .iter()
            .map(|(name, holding)| (name.clone(), json!({"tools": holding.tools})))
            .collect();
        let mut told = json!({"held": held, "not_held": self.not_held});
        if let NativePlan::Unmeasured { reason, .. } = &self.native {
            told["native"] = json!(format!(
                "Provider '{}' declares its native capabilities unmeasured ({reason}); nothing \
                 is claimed about what it can reach on its own",
                self.provider
            ));
        }
        told
    }
}

/// One executable site's sealed capability facts: what it asks, and one
/// outcome per provider candidate — never their union.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteCapabilities {
    pub asks: SiteAsks,
    pub outcomes: Vec<Outcome>,
}

impl SiteCapabilities {
    /// The outcome that serves one attempt: the selected link's own — by
    /// provider and model, so a fallback never borrows its primary's
    /// holdings — or an inline site's single one.
    pub fn serving(&self, link: Option<(&str, &str)>) -> Option<&Outcome> {
        match link {
            None => self.outcomes.first(),
            Some((provider, model)) => self.outcomes.iter().find(|outcome| {
                outcome.provider == provider && outcome.model.as_deref() == Some(model)
            }),
        }
    }
}

/// Why a known native power that no ask of the seat reached is not held
/// (operator ruling of 2026-09-29, rebuild unit 21-fix-a, R3): the realm
/// grants it to the seat's office and nothing requests it, or the realm
/// does not grant it. Two causes, never one generic text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Unasked {
    Granted,
    Ungranted,
}

impl Unasked {
    /// The exact reason the prompt and the manifest give.
    fn reason(self) -> &'static str {
        match self {
            Unasked::Granted => "granted, but this seat does not request it",
            Unasked::Ungranted => "the realm does not grant it to this seat",
        }
    }
}

/// The dotted paths of a restriction object's leaves: `allow.hosts`.
pub fn restriction_names(prefix: &str, restrictions: &Map<String, Value>) -> Vec<String> {
    let mut names = Vec::new();
    for (key, value) in restrictions {
        let path = match prefix.is_empty() {
            true => key.clone(),
            false => format!("{prefix}.{key}"),
        };
        match value.as_object() {
            Some(nested) if !nested.is_empty() => names.extend(restriction_names(&path, nested)),
            _ => names.push(path),
        }
    }
    names
}

/// Everything a compile authorises against, loaded and validated ONCE:
/// the context, every definition, and the dialect each grant selected.
#[derive(Debug, Clone, PartialEq)]
pub struct Authority {
    pub context: CapabilityContext,
    pub definitions: Definitions,
    /// Capability to the dialect its grant selected.
    pub dialects: BTreeMap<String, ToolDialect>,
    /// The bundle's binding minimum an `mcp` dialect's egress meets (MB4).
    minimum: crate::agents::EgressClass,
    /// Private, so only [`Authority::load`] and [`Authority::nothing`]
    /// construct one: a struct literal elsewhere skips load's checks.
    loaded: Loaded,
}

/// The mark only this module's loaders can write; `#[non_exhaustive]`
/// would fence other crates and leave this one's other modules open.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Loaded;

/// The serving provider's own native capability `adapter_key`, where its
/// inventory is known and that entry serves `capability`; else why not.
fn native_serving<'a>(
    serving: &Serving<'a>,
    adapter_key: &str,
    capability: &str,
) -> Result<&'a NativeCapability, String> {
    let provider = serving.provider;
    let known = match serving.native {
        Some((NativeInventory::Known { known, .. }, _)) => known,
        Some((NativeInventory::Unmeasured(reason), _)) => {
            return Err(format!(
                "provider '{provider}' declares its native capabilities unmeasured ({reason})"
            ))
        }
        None => return Err(format!("no adapter declares provider '{provider}'")),
    };
    known
        .get(adapter_key)
        .filter(|native| native.capability == capability)
        .ok_or_else(|| {
            format!(
                "provider '{provider}' declares no native capability '{adapter_key}' serving it"
            )
        })
}

impl Authority {
    /// Validate every grant of the operated realm BEFORE any seat is
    /// looked at (design D4 steps 1 and 2): the definition exists, the
    /// dialect loads, serves the capability and agrees on its classes, the
    /// tool subset is a subset, the restriction keys pass the dialect's
    /// schema. Then the two realm-wide refusals, which hold whether or not
    /// any seat asks: an `mcp` grant, and a `hands` grant. None of these
    /// can become an optional drop.
    pub fn load(context: CapabilityContext) -> Result<Authority, String> {
        let realm = &context.realm;
        let definitions = Definitions::load(&context.root)?;
        let mut dialects = BTreeMap::new();
        for (capability, grant) in &context.grants {
            let definition = definitions.get(capability).ok_or_else(|| {
                format!(
                    "realm '{realm}': capability '{capability}' has no abstract definition at \
                     '{}' in the operator configuration; declare its classes before granting it",
                    Definition::source_of(capability)
                )
            })?;
            // Which keys the grant reserves is its map version's (SC2).
            let dialect = ToolDialect::load(&context.root, &grant.dialect)
                .and_then(|dialect| {
                    let reserved = dialect.reserves(grant.retention);
                    reserved
                        .map(|()| dialect)
                        .map_err(|collision| collision.to_string())
                })
                .map_err(|problem| {
                    format!("realm '{realm}': capability '{capability}': {problem}")
                })?;
            if dialect.serves != *capability {
                return Err(format!(
                    "realm '{realm}' grants capability '{capability}' through dialect '{}', \
                     which serves '{}'",
                    dialect.name, dialect.serves
                ));
            }
            if let Some(classes) = &dialect.classes {
                if sorted(classes) != sorted(&definition.classes) {
                    return Err(format!(
                        "realm '{realm}': capability '{capability}' in dialect '{}' declares \
                         classes [{}], conflicting with abstract definition '{}' classes [{}]",
                        dialect.name,
                        classes.join(", "),
                        definition.source,
                        definition.classes.join(", ")
                    ));
                }
            }
            for tool in grant.tools.iter().flatten() {
                if !dialect.tools.contains(tool) {
                    return Err(format!(
                        "realm '{realm}' grants capability '{capability}' tool '{tool}', which \
                         dialect '{}' does not name; its tools are [{}]",
                        dialect.name,
                        dialect.tools.join(", ")
                    ));
                }
            }
            violation(
                &dialect.restrictions,
                &Value::Object(grant.restrictions.clone()),
            )
            .map_err(|problem| {
                format!(
                    "realm '{realm}' grants capability '{capability}' through dialect '{}' with \
                     an invalid restriction {problem}",
                    dialect.name
                )
            })?;
            dialects.insert(capability.clone(), dialect);
        }
        // The compile fence (SC5): until slice two's enabling unit, every
        // grant binds a native provider, used by a seat or not.
        for (capability, dialect) in &dialects {
            binding::native(realm, capability, dialect).map_err(|unbound| unbound.to_string())?;
        }
        Ok(Authority {
            context,
            definitions,
            dialects,
            minimum: ABSENT_EGRESS_MINIMUM,
            loaded: Loaded,
        })
    }

    /// An authority that grants and defines nothing: what a readout reads
    /// a realm as when its declared grants did not validate, so the lines
    /// that do not depend on them can still be printed. Never what a
    /// compile uses — a compile refuses instead.
    pub fn nothing(realm: &str, root: &Path) -> Authority {
        Authority {
            context: CapabilityContext::no_grants(realm, root),
            definitions: Definitions::default(),
            dialects: BTreeMap::new(),
            minimum: ABSENT_EGRESS_MINIMUM,
            loaded: Loaded,
        }
    }

    /// The site every capability refusal and notice opens with, typed so
    /// that no author's label is ever spelled raw (rebuild unit 12-fix-f).
    fn who<'a>(&'a self, site: &'a SiteAsks) -> brokkr_protocol::native_controls::Site<'a> {
        brokkr_protocol::native_controls::Site {
            seat: &site.label,
            office: &site.office,
            realm: &self.context.realm,
        }
    }

    /// The realm's grant of `capability` that reaches the site's office
    /// (design D4), or why none does: no dialect is chosen before it.
    fn reaching(&self, site: &SiteAsks, capability: &str) -> Result<&CapabilityGrant, Cause> {
        let Some(grant) = self.context.grants.get(capability) else {
            return Err(Cause::Ungranted(
                "the realm does not grant it to this office".into(),
            ));
        };
        if grant.reaches(&site.office) {
            return Ok(grant);
        }
        let scope = grant.offices.as_deref().unwrap_or_default();
        Err(Cause::Ungranted(match scope.is_empty() {
            true => "the realm grants it to no office".to_string(),
            false => format!(
                "the realm grants it only to offices [{}], not to this office",
                scope.join(", ")
            ),
        }))
    }

    /// Why this candidate cannot hold `capability`, or the holding and
    /// the native key that serves it.
    fn holding(
        &self,
        site: &SiteAsks,
        capability: &str,
        serving: &Serving<'_>,
    ) -> Result<(Holding, String), Unheld> {
        let grant = self.reaching(site, capability)?;
        let dialect = self
            .dialect(capability)
            .map_err(|missing| Cause::incompatible(&grant.dialect, missing.to_string()))?;
        let through = |but| Unheld::Cause(Cause::incompatible(&dialect.name, but));
        let tools = grant.tools.clone().unwrap_or_else(|| dialect.tools.clone());
        if tools.is_empty() {
            return Err(through("the realm's grant admits no tool".into()));
        }
        // GP1, before any provider carries it (design D3 steps 5 and 6).
        let definition = &self.definitions.0[capability];
        gates::check(site, definition, grant)
            .map_err(|refusal| Cause::gate(&dialect.name, refusal))?;
        let provider = serving.provider;
        let (bound, adapter_key) = self
            .carried(capability, dialect, &tools)
            .map_err(|unserved| unserved.unheld(dialect))?;
        if bound != provider {
            return Err(through(format!(
                "provider '{provider}' cannot carry a binding to provider '{bound}'"
            )));
        }
        let native = native_serving(serving, adapter_key, capability).map_err(through)?;
        if let Some(tool) = tools.iter().find(|tool| !native.tools.contains(tool)) {
            return Err(through(format!(
                "provider '{provider}' native '{adapter_key}' has no tool '{tool}'"
            )));
        }
        match &native.on {
            Disposition::Unsupported(reason) => {
                return Err(through(format!(
                    "provider '{provider}' cannot switch it on ({reason})"
                )))
            }
            Disposition::Unmeasured(reason) => {
                return Err(through(format!(
                    "provider '{provider}' declares its ON control unmeasured ({reason})"
                )))
            }
            // A whole-set switch cannot admit a proper subset: it would
            // enable a tool the realm excluded.
            Disposition::Argv(_) | Disposition::Default(_) if tools.len() < native.tools.len() => {
                return Err(through(format!(
                    "provider '{provider}' switches native '{adapter_key}' on as a whole and \
                     cannot admit only [{}] of its tools [{}]",
                    tools.join(", "),
                    native.tools.join(", ")
                )))
            }
            _ => {}
        }
        // Slice one carries only the empty restriction (operator ruling of
        // 2026-09-25; design D11): a nonempty one is inexpressible on every
        // candidate, a declared transport included, so it reaches CQ1's
        // outcomes alone and is never composed.
        if !grant.restrictions.is_empty() {
            let names = restriction_names("", &grant.restrictions);
            let deferred = match native.restrictions {
                Transport::Unsupported(_) => "",
                Transport::Argv(_) => {
                    " through its declared transport, which carries only the empty restriction \
                     until a provider restriction transport is measured (operator ruling of \
                     2026-09-25)"
                }
            };
            return Err(through(format!(
                "provider '{provider}' cannot express restriction '{}'{deferred}",
                names.join("', '")
            )));
        }
        Ok((
            Holding {
                classes: definition.classes.clone(),
                dialect: dialect.name.clone(),
                dialect_sha256: dialect.sha256.clone(),
                definition_sha256: definition.sha256.clone(),
                tools,
                restrictions: grant.restrictions.clone(),
                implementation: Implementation::of(capability, &dialect.kind)
                    .expect("a carried binding is never the reserved hands kind"),
                retention: Retention::of(dialect, grant),
            },
            adapter_key.to_string(),
        ))
    }

    /// Resolve one site against one provider candidate (design D4 steps
    /// 3 to 5). `Err` is a complete compile refusal.
    pub fn resolve(&self, site: &SiteAsks, serving: &Serving<'_>) -> Result<Outcome, String> {
        let who = self.who(site);
        for name in site.asks.keys().chain(&site.subtracted) {
            self.definitions.require(who, name)?;
        }
        let mut held = BTreeMap::new();
        let mut keys: BTreeMap<String, String> = BTreeMap::new();
        let mut not_held = BTreeMap::new();
        let mut dropped: Vec<(String, String, String)> = Vec::new();
        for name in &site.subtracted {
            not_held.insert(
                name.clone(),
                "this seat subtracted it from its office's asks".to_string(),
            );
        }
        for (capability, strength) in &site.asks {
            match self.holding(site, capability, serving) {
                Ok((holding, key)) => {
                    keys.insert(key, capability.clone());
                    held.insert(capability.clone(), holding);
                }
                Err(Unheld::Identity) => return Err(format!("{who}: {}", Unserved::Identity)),
                Err(Unheld::Cause(cause)) if *strength == Strength::Requires => {
                    return Err(cause.required(who, capability));
                }
                Err(Unheld::Cause(cause)) => {
                    // `through` is empty where no grant was found at all.
                    dropped.push((capability.clone(), cause.through(), cause.but()));
                    not_held.insert(capability.clone(), cause.but());
                }
            }
        }
        // Operator ruling 1 of 2026-09-23 (rebuild unit 12): what the
        // recipe wrote carries no capability-bearing option, whatever the
        // realm grants and whatever the option's value; nothing authored is
        // merged into what the engine composes below.
        brokkr_protocol::native_controls::authored_refusal(serving.harness, serving.written)
            .map_err(|refusal| refusal.at_compile(&who))?;
        // An explicit include limit that excludes a held tool (design D6;
        // CQ1): a wanted capability drops, its native control switched OFF,
        // and the plan is resolved again without it; a required one refuses
        // the whole conflict. Each pass holds one capability fewer.
        let native = loop {
            let native = self.native_plan(who, serving, &held, &keys, &mut not_held)?;
            let exclusion = match admit(serving, &native) {
                Ok(()) => break native,
                Err(launch::Failure::Excluded(exclusion))
                    if site.asks.get(&exclusion.capability) == Some(&Strength::Wants) =>
                {
                    exclusion
                }
                Err(failure) => return Err(failure.refusal().at_compile(&who)),
            };
            let capability = exclusion.capability;
            let holding = held
                .remove(&capability)
                .expect("an excluded capability is held");
            keys.retain(|_, name| *name != capability);
            dropped.push((
                capability.clone(),
                format!(" through dialect '{}'", holding.dialect),
                exclusion.clause.clone(),
            ));
            not_held.insert(capability, exclusion.clause);
        };
        // A notice claims a native OFF only where THIS candidate's plan
        // composed one for a native power serving the capability. A
        // provider whose inventory or OFF control is unmeasured denies
        // nothing it can show, whatever else lost it the capability — a
        // binding to another provider included (ruling 4; NC5).
        let switched_off = |capability: &str| match (&native, serving.native) {
            (NativePlan::Known { off, .. }, Some((NativeInventory::Known { known, .. }, _))) => {
                off.iter().any(|key| known[key].capability == capability)
            }
            _ => false,
        };
        // The seat is told the same thing the manifest is: its own reason
        // for not holding the capability carries the tail, so a prompt
        // never leaves a dropped native power's state unsaid.
        let mut notices = Vec::with_capacity(dropped.len());
        for (capability, through, but) in dropped {
            let tail = match (through.is_empty(), switched_off(&capability)) {
                (true, _) => "",
                (false, true) => "; native capability remains OFF",
                (false, false) => "; no native denial is claimed",
            };
            not_held.insert(capability.clone(), format!("{but}{tail}"));
            let message = format!(
                "{who}: dropped wanted capability '{capability}'{through} because {but}{tail}"
            );
            notices.push((capability, message));
        }
        Ok(Outcome {
            provider: serving.provider.to_string(),
            harness: serving.harness.to_string(),
            model: serving.model.map(str::to_string),
            held,
            not_held,
            notices,
            native,
        })
    }

    /// Design D8 and operator ruling 4 of 2026-09-23: the complete
    /// adapter-level plan `brokkr doctor` submits where it has resolved no
    /// seat. A seat labelled [`ADAPTER_SEAT`] of `office` asks `asks` of
    /// the adapter's provider, served by the adapter's own template and
    /// nothing else — no recipe's words, model pins, typed tools or hands —
    /// and the whole plan goes through [`Authority::resolve`]: every known
    /// power ON or OFF together, the realm's grants and restrictions, and
    /// the admission by the composer the launch's final check recomposes
    /// with. An admitted plan is then served as launch serves it
    /// ([`final_validation`]), so a complete command that fails the final
    /// check refuses here with launch's own cause. What it returns is that
    /// admission or that refusal, never an answer for one capability alone.
    ///
    /// A refusal leaves through the protocol's one refusal sink
    /// ([`launch::bounded_line`]), as a compile's and a launch's do: one
    /// line of at most 512 scalar values, whichever step refused (design
    /// D6; review return SC2 of rebuild unit 22).
    pub fn assess(
        &self,
        adapter: &crate::agents::Adapter,
        office: &str,
        asks: Requests,
    ) -> Result<Outcome, String> {
        self.assessed(adapter, office, asks)
            .map_err(|cause| launch::bounded_line(&cause))
    }

    /// [`Authority::assess`] before its refusal is bounded.
    fn assessed(
        &self,
        adapter: &crate::agents::Adapter,
        office: &str,
        asks: Requests,
    ) -> Result<Outcome, String> {
        // A seatless hypothesis has no class to read: judged as work, it
        // answers no gate's holding (GP1) and authorizes no launch.
        let site = SiteAsks {
            label: ADAPTER_SEAT.to_string(),
            office: office.to_string(),
            asks,
            subtracted: Vec::new(),
            class: SeatClass::Work,
        };
        let outcome = self.resolve(
            &site,
            &Serving {
                provider: &adapter.provider,
                harness: harness_of(&adapter.driver),
                model: None,
                native: Some((&adapter.native, &adapter.digest)),
                unloaded: None,
                authored: &adapter.driver,
                fragment: &[],
                provenance: &launch::Provenance::default(),
                written: &[],
            },
        )?;
        final_validation(adapter, &outcome)
            .map_err(|cause| format!("{}: {cause}", self.who(&site)))?;
        Ok(outcome)
    }

    /// Independently of every ask (design D4 step 4): each native power
    /// this candidate's harness is known to have is switched ON if held
    /// through it and OFF otherwise — and one that cannot be switched off
    /// refuses the seat (ruling 4).
    #[expect(
        clippy::too_many_lines,
        reason = "baseline 2026-09-29, decision 0065 slice one merged with main; split after #319"
    )]
    fn native_plan(
        &self,
        who: brokkr_protocol::native_controls::Site<'_>,
        serving: &Serving<'_>,
        held: &BTreeMap<String, Holding>,
        keys: &BTreeMap<String, String>,
        not_held: &mut BTreeMap<String, String>,
    ) -> Result<NativePlan, String> {
        let provider = serving.provider;
        // Decision 0066 ruling 1 (finding H1): a denial is something the
        // launch proves, never something absence implies. A provider known
        // to carry a native power is seated only where its adapter data
        // declares a control for that power; data that is missing, legacy,
        // emptied, unreadable or silent about it REFUSES the seat, with
        // the cause named, instead of compiling a plan that denies
        // nothing. The floor supplies no switch — the refusal is all it
        // can do. No seat can hold the power on such data either, so
        // "does not hold" is true of every site that reaches this.
        let floor = brokkr_protocol::native_controls::known_powers(serving.harness);
        let undeniable = |capability: &str, cause: String| {
            format!(
                "{who}: provider '{provider}' is known to carry native capability \
                 '{capability}', which this seat does not hold, and no valid control denies \
                 it: {cause}. A known native power is launched only with a delivered denial, \
                 never on what absence implies; repair the adapter data (decision 0066 ruling \
                 1)"
            )
        };
        // Operator ruling of 2026-09-29 (R5; design D5.3): a typed tools
        // restriction meeting an unmeasured plan refuses here, with that
        // plan's own cause. It is never lowered onto a plan that measures
        // nothing, nor left for the launch to refuse: compile and launch
        // agree. The plan otherwise carries the provenance it was served.
        // The cause is stated whole right after the site, in the engine's
        // own words. The adapter's reason, which may be long adapter data,
        // follows as the line's tail, so the bound cuts only that.
        // The tail is written whatever declared it (unit 26c): an undeclared
        // provider types no local allow, as a list lowers only through its
        // provider's loaded adapter, so no arm for it stands here, where the
        // exact-coverage gate would count it unreachable. The removal
        // control is the bundle suite's
        // `an_inline_claude_or_lanetally_site_lowers_its_allow_and_every_other_shape_refuses`.
        let unmeasured = |declaration: Option<String>, reason: String, cause: &str| {
            if !serving.provenance.local.is_empty() {
                return Err(format!(
                    "{who}: its typed 'tools.allow' refuses at compile, as harness '{}' of \
                     provider '{provider}' has native controls {cause} (ruling R5 of 2026-09-29; \
                     design D5.3): {reason}",
                    serving.harness
                ));
            }
            Ok(NativePlan::Unmeasured {
                declaration,
                reason,
                provenance: serving.provenance.clone(),
            })
        };
        let (known, selection, declaration) = match (serving.native, floor.first()) {
            (Some((NativeInventory::Known { known, selection }, digest)), _) => {
                (known, selection, digest)
            }
            (Some((NativeInventory::Unmeasured(reason), _)), Some(capability)) => {
                return Err(undeniable(
                    capability,
                    format!("its adapter declares its native capabilities unmeasured ({reason})"),
                ))
            }
            (None, Some(capability)) => {
                return Err(undeniable(
                    capability,
                    match serving.unloaded {
                        Some(problem) => {
                            format!("the adapter data could not be loaded ({problem})")
                        }
                        None => format!("no adapter declares provider '{provider}'"),
                    },
                ))
            }
            (Some((NativeInventory::Unmeasured(reason), digest)), None) => {
                return unmeasured(
                    Some(digest.to_string()),
                    reason.clone(),
                    "its adapter declares unmeasured",
                )
            }
            (None, None) => {
                return unmeasured(
                    None,
                    format!("no adapter declares provider '{provider}'"),
                    "no adapter declares",
                )
            }
        };
        if let Some(capability) = floor
            .iter()
            .find(|name| !known.values().any(|native| native.capability == **name))
        {
            return Err(undeniable(
                capability,
                format!("its adapter declares no native capability serving '{capability}'"),
            ));
        }
        let guards: Vec<brokkr_protocol::native_controls::Guard> = known
            .values()
            .map(|native| brokkr_protocol::native_controls::Guard {
                capability: native.capability.clone(),
                flags: native.authored.flags.clone(),
                config_flags: native.authored.config_flags.clone(),
                config_keys: native.authored.config_keys.clone(),
                feature_flags: native.authored.feature_flags.clone(),
                features: native.authored.features.clone(),
                list_flags: native.authored.list_flags.clone(),
                tools: native.tools.clone(),
                value_flags: native.authored.value_flags.clone(),
            })
            .collect();
        // A command nothing parses — an opaque custom driver — is judged
        // CONSERVATIVELY by name for a guarded control (design D6a). A
        // harness brokkr drives was judged by origin in `resolve`, and its
        // values are never read for admission (rebuild unit 12). The opaque
        // judgment places nothing in a grammar, so it has no refusal of its
        // own to convert (unit 26c).
        let opaque = brokkr_protocol::native_controls::grammar::grammar(serving.harness).is_none();
        if let Some((written, capability)) = match opaque {
            true => brokkr_protocol::native_controls::opaque_conflict(serving.authored, &guards),
            false => None,
        } {
            return Err(format!(
                "{who}: its arguments carry '{written}', which controls native capability \
                 '{capability}' of provider '{provider}'. Only the realm grants a capability, \
                 and the engine composes the one control the grant resolves to; request \
                 '{capability}' by name under 'capabilities' instead (decision 0065 rulings 3 \
                 and 4)"
            ));
        }
        // What the plan answers for, sealed from typed inputs BEFORE any
        // control is rendered (design D5.7): each known power is held where
        // the realm's holding reaches its key, with that holding's admitted
        // tools and restriction object as written, and denied otherwise. A
        // measured default ON is held here though it emits no argument.
        let expected = NativeExpectation::Known {
            held: known
                .iter()
                .filter_map(|(key, native)| {
                    keys.get(key).map(|capability| HeldPower {
                        capability: native.capability.clone(),
                        tools: held[capability].tools.clone(),
                        restrictions: held[capability].restrictions.clone(),
                    })
                })
                .collect(),
            denied: known
                .iter()
                .filter(|(key, _)| !keys.contains_key(*key))
                .map(|(_, native)| native.capability.clone())
                .collect(),
        };
        let (mut argv, mut lists) = (Vec::new(), ToolLists::default());
        let (mut on, mut off) = (Vec::new(), Vec::new());
        for (key, native) in known {
            let holding = keys.get(key).map(|capability| &held[capability]);
            // What becomes of a seat that does not hold the power
            // ([`Denial`]): a declared control composes below, and the
            // other two refuse. Whether the composition can actually
            // deliver it is settled by `admit` over the whole plan, which
            // is where the provider-aware answer belongs and what `brokkr
            // doctor` reads through [`Authority::assess`]; here the
            // declaration is what decides ON versus OFF.
            match (holding, native.declared_denial()) {
                (Some(_), _) | (None, Denial::Delivered) => {}
                // The refusal carries its own warrant: who measured that
                // the power cannot be removed, and over what — so an
                // operator can tell a finding about one CLI version from a
                // law about the provider.
                (None, Denial::Impossible(reason)) => {
                    return Err(format!(
                        "{who}: provider '{provider}' cannot switch off its native capability \
                         '{}', which this seat does not hold ({reason}; evidence: {}, scope: {}); \
                         an ungranted native capability that cannot be disabled cannot be seated \
                         in this realm (decision 0065 ruling 4)",
                        native.capability, native.evidence.source, native.evidence.scope
                    ))
                }
                // Nobody measured the OFF control: no denial is claimed,
                // and the seat is refused rather than launched on a guess
                // (decision 0066 ruling 1) — for every provider that
                // declares the power, floor or not.
                (None, Denial::Unmeasured(reason)) => {
                    return Err(undeniable(
                        &native.capability,
                        format!("its OFF control is unmeasured ({reason})"),
                    ))
                }
            }
            let disposition = match holding {
                Some(_) => &native.on,
                None => &native.off,
            };
            match (disposition, holding) {
                (Disposition::Argv(switch), _) => argv.extend(switch.iter().cloned()),
                (Disposition::Selection(selected), Some(holding)) => {
                    let admitted = |names: &[String]| -> Vec<String> {
                        names
                            .iter()
                            .filter(|tool| holding.tools.contains(tool))
                            .cloned()
                            .collect()
                    };
                    lists.include.extend(admitted(&selected.include));
                    lists.allow.extend(admitted(&selected.allow));
                    lists.deny.extend(selected.deny.iter().cloned());
                    lists.deny.extend(
                        native
                            .tools
                            .iter()
                            .filter(|tool| !holding.tools.contains(tool))
                            .cloned(),
                    );
                }
                (Disposition::Selection(selected), None) => {
                    lists.include.extend(selected.include.iter().cloned());
                    lists.allow.extend(selected.allow.iter().cloned());
                    lists.deny.extend(selected.deny.iter().cloned());
                }
                // A measured default needs nothing written. The other two
                // never reach this match: an ON that cannot be switched lost
                // the holding in `holding`, and an OFF that cannot be was
                // refused just above.
                (
                    Disposition::Default(_)
                    | Disposition::Unsupported(_)
                    | Disposition::Unmeasured(_),
                    _,
                ) => {}
            }
            match holding {
                // A holding's restriction is the empty one (design D11), so
                // no transport argument is ever composed for it.
                Some(_) => on.push(key.clone()),
                // An unselected entry of a capability the seat holds through
                // another entry loses the seat nothing: its reason is owed
                // per capability, never per entry (review R2 of run
                // 0065-rebuild-unit-21-see-the-uni-014db2ff).
                None if held.contains_key(&native.capability) => off.push(key.clone()),
                None => {
                    off.push(key.clone());
                    let unasked = match self.context.grants.get(&native.capability) {
                        Some(grant) if grant.reaches(who.office) => Unasked::Granted,
                        _ => Unasked::Ungranted,
                    };
                    not_held.entry(native.capability.clone()).or_insert(format!(
                        "provider '{provider}' has it natively, {}, and it is switched off",
                        unasked.reason()
                    ));
                }
            }
        }
        // The plan says whom it was resolved for and which abstract
        // capabilities it answers for, ON and OFF, so the driver can prove
        // at the last boundary that every known power of THIS provider was
        // ruled on (decision 0066 rulings 1 and 2) — a fallback link's own
        // plan, never its primary's, and never an empty one.
        let capabilities = |keys: &[String]| -> Vec<&str> {
            keys.iter()
                .map(|key| known[key].capability.as_str())
                .collect()
        };
        // What each holding admits (rebuild unit 12-fix; design D6): the
        // tools of the ONE entry the realm's holding binds by its adapter
        // key, narrowed by the grant — the sealed expectation's own, built
        // from the holdings above and never from the entries that merely
        // share a capability's name.
        let mut controls = json!({
            "inventory": "known",
            "provider": provider,
            "harness": serving.harness,
            "on": capabilities(&on),
            "off": capabilities(&off),
            "admits": expected.admits(),
            "argv": argv,
            // The provenance of what admits a tool without a holding
            // (rebuild unit 12-fix-c), carried to the driver as typed data.
            "hands": serving.provenance.hands,
            "local": serving.provenance.local,
            "guards": guards.iter().map(|guard| json!({
                "capability": guard.capability,
                "flags": guard.flags,
                "config_flags": guard.config_flags,
                "config_keys": guard.config_keys,
                "feature_flags": guard.feature_flags,
                "features": guard.features,
                "list_flags": guard.list_flags,
                "tools": guard.tools,
                "value_flags": guard.value_flags,
            })).collect::<Vec<_>>(),
        });
        if let Some(flags) = selection {
            let list = |list: &ListFlag| json!({"flag": list.flag, "separator": list.separator});
            controls["selection"] = json!({
                "include": lists.include,
                "allow": lists.allow,
                "deny": lists.deny,
                "flags": {"include": list(&flags.include), "allow": list(&flags.allow),
                          "deny": list(&flags.deny)},
            });
        }
        // The same selection as typed data, its list flags and separators
        // taken from the adapter's mapping as declared, not from `controls`.
        let typed = |list: &ListFlag| launch::ListFlag {
            flag: list.flag.clone(),
            separator: list.separator.clone(),
        };
        let contribution = NativeContribution {
            held: capabilities(&on).into_iter().map(str::to_string).collect(),
            denied: capabilities(&off).into_iter().map(str::to_string).collect(),
            argv,
            selection: launch::Selection {
                include: lists.include,
                allow: lists.allow,
                deny: lists.deny,
                flags: selection.as_ref().map(|flags| {
                    [
                        typed(&flags.include),
                        typed(&flags.allow),
                        typed(&flags.deny),
                    ]
                }),
            },
        };
        Ok(NativePlan::Known {
            declaration: declaration.to_string(),
            on,
            off,
            controls,
            contribution: Box::new(contribution),
            expected,
        })
    }
}

/// Compile admission (decision 0066 rulings 3 and 4): the plan one
/// candidate resolved to is composed by the SAME function the driver
/// composes it with, over the same two parts of the argv — what was
/// authored, and the fragment the engine appended. A capability server in
/// the authored part, or a control the provider's launch does not consume,
/// refuses here, naming the site, rather than first failing after a spawn
/// or, worse, being recorded and dropped. A held capability an explicit
/// limit excludes is told apart, for [`Authority::resolve`] to rule on.
fn admit(serving: &Serving<'_>, plan: &NativePlan) -> Result<(), launch::Failure> {
    let plan = plan.controls(serving.provider, serving.harness);
    let decoded = launch::managed(&json!({"native_controls": plan}))
        .expect("the plan this module wrote is one the driver reads")
        .expect("the key is present");
    launch::compose_or_exclude(
        serving.harness,
        serving.authored,
        serving.fragment,
        &decoded,
    )
    .map(drop)
}

/// The workdir an adapter-level plan's cold command names: a hypothesis
/// has none of its own, and nothing is spawned in it.
const ADAPTER_WORKDIR: &str = "/";

/// The overlay and prompt an adapter-level DSH command names: every DSH
/// command carries both, and the final check reads each only as the
/// argument it is, so neither is staged or written.
const ADAPTER_OVERLAY: &str = "/adapter-plan/overlay.json";
const ADAPTER_PROMPT: &str = "adapter-plan";

/// Design D8 and review return SC1 of rebuild unit 22: the complete cold
/// command an admitted adapter-level plan ([`Authority::assess`]) serves,
/// handed to the built-in driver's own launch exactly as the engine hands a
/// sealed launch over — the plan, a launch record sealed from its typed
/// facts (the adapter's template and nothing authored, no local
/// declaration, no hands) and empty serving inputs (no pins, no dialect
/// fragment) — so the driver composes, builds and checks it with
/// [`launch::check_final`], and its refusal is launch's own. The record
/// and the launch arguments are the engine's own: a spawn of the adapter's
/// template, sealed and projected by [`SiteSpawn::seal`] and
/// [`SiteSpawn::launch_arguments`] (review return L1), so the template
/// agreement and the local sandbox check a dispatch makes are made here.
/// Nothing is spawned: a cold launch offers no session, so no version is
/// probed.
///
/// Each harness is answered as its launch serves it (review return M1). A
/// driver the engine does not dispatch is opaque, and `exec` consumes no
/// native control and checks no final command: the composition
/// [`Authority::resolve`] admitted is all their launch judges, and the plan
/// rides as data. DSH and LaneTally commands are judged by the readings
/// their launches' own final validation exports (operator ruling of
/// 2026-09-30, unit 22-fix-b): a DSH command names an overlay and a prompt
/// it never stages. A driver name no built-in driver answers to, which
/// launch refuses before anything is composed, is refused here too.
///
/// [`SiteSpawn::seal`]: crate::engine::SiteSpawn::seal
/// [`SiteSpawn::launch_arguments`]: crate::engine::SiteSpawn::launch_arguments
fn final_validation(adapter: &crate::agents::Adapter, outcome: &Outcome) -> Result<(), String> {
    use brokkr_protocol::adapters::{
        claude_command, codex_command, dsh_cold_command, lanetally_command,
    };
    use launch::{
        AllowIntent, Application, Expected, HandsIntent, LocalExpectation, Origin, SandboxIntent,
        SealedServing, SERVING_INPUTS,
    };
    let harness = harness_of(&adapter.driver);
    match harness {
        OPAQUE_HARNESS | "exec" => return Ok(()),
        "codex" | "claude" | "dsh" | "lanetally" => {}
        _ => {
            return Err(format!(
                "no adapter-level cold command of harness '{harness}' is checked here, because \
                 no built-in driver of that name is launched, so its plan is not reported as \
                 admitted (design D8)"
            ))
        }
    }
    let segments = vec![Segment::new(Origin::Template, &adapter.driver)];
    let mut spawn = crate::engine::SiteSpawn {
        argv: adapter.driver.clone(),
        env: brokkr_protocol::process::SpawnEnv::Inherit,
        rewalk: None,
        refusal: None,
        segments,
        record: None,
        serving: None,
        class: None,
        charter: None,
    };
    spawn.seal(Expected {
        identity: outcome.identity(),
        native: outcome.native.expected(),
        local: LocalExpectation {
            allow: AllowIntent::Unspecified,
            sandbox: SandboxIntent::Unspecified,
            application: Application::Unrestricted,
        },
        hands: HandsIntent::None,
        template: crate::agents::declared_template(&adapter.driver),
    })?;
    let extras = launch::harness_arguments(&adapter.driver).to_vec();
    let input = json!({
        "workdir": ADAPTER_WORKDIR,
        "seat": ADAPTER_SEAT,
        "native_controls": outcome.controls(),
        "launch_record": spawn.launch_record(),
        SERVING_INPUTS: SealedServing::default().value(),
        "launch_arguments": spawn.launch_arguments(),
    });
    match harness {
        "codex" => codex_command(harness, &extras, ADAPTER_WORKDIR, None, &input),
        "dsh" => dsh_cold_command(
            harness,
            &extras,
            ADAPTER_WORKDIR,
            ADAPTER_OVERLAY,
            ADAPTER_PROMPT,
            &input,
        ),
        "lanetally" => lanetally_command(harness, &extras, None, &input),
        _ => claude_command(harness, &extras, None, &input),
    }
    .map(drop)
}

#[cfg(test)]
mod tests;
