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
use serde::Deserialize;
use serde_json::{json, Map, Value};

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

const TOOL_DIALECT_SCHEMA: &str = include_str!("tool-dialect.v1.schema.json");

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
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SiteAsks {
    /// The execution label: `research`, `review:security`, `verify:checks`.
    pub label: String,
    /// The office: the agent's name, or an inline site's authoring label.
    /// A wrapper that moves the site does not move its office.
    pub office: String,
    pub asks: Requests,
    pub subtracted: Vec<String>,
}

impl SiteAsks {
    /// Office asks minus seat subtractions (ruling 5). An inline site's
    /// map IS its office's asks. A site naming an agent inherits the
    /// agent's asks when it writes no map; a map it does write is a subset
    /// with unchanged strengths, and what it leaves out is subtracted —
    /// `{}` subtracts everything. A seat never adds and never re-rates.
    pub fn of(
        label: &str,
        agent: Option<(&str, &Requests)>,
        site: Option<&Value>,
    ) -> Result<SiteAsks, String> {
        let what = format!("seat '{label}'");
        let written = site
            .map(|raw| parse_requests(&what, raw))
            .transpose()?;
        let Some((office, office_asks)) = agent else {
            return Ok(SiteAsks {
                label: label.to_string(),
                office: label.to_string(),
                asks: written.unwrap_or_default(),
                subtracted: Vec::new(),
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
        })
    }
}

// --------------------------------------------------- operator documents

/// Read one operator document: the file's raw bytes, their digest, and
/// the strict parse of those SAME bytes. The path is proven to stay
/// inside `root` after symlinks resolve. `Ok(None)` is a file that is
/// not there.
fn read_document(root: &Path, relative: &str) -> Result<Option<(Value, String)>, String> {
    let path = root.join(relative);
    let Ok(canonical) = path.canonicalize() else {
        return Ok(None);
    };
    let inside = root.canonicalize().is_ok_and(|root| canonical.starts_with(root));
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
            .filter(|path| path.extension().is_some_and(|extension| extension == "json"))
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
    pub fn require(&self, who: &str, name: &str) -> Result<&Definition, String> {
        self.get(name).ok_or_else(|| {
            format!(
                "{who}: capability '{name}' has no abstract definition at '{}' in the operator \
                 configuration; declare its classes before requesting it",
                Definition::source_of(name)
            )
        })
    }
}

/// The one implementation kind a tool dialect binds (ruling 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DialectKind {
    /// A capability a harness already has, addressed through its adapter.
    Native { provider: String, adapter_key: String },
    /// An MCP server. Whole as data; refused as a grant until slice two.
    Mcp,
    /// Reserved: decision 0043's workspace tool, which does not move.
    Hands,
}

impl DialectKind {
    pub fn word(&self) -> &'static str {
        match self {
            DialectKind::Native { .. } => "provider-native",
            DialectKind::Mcp => "mcp",
            DialectKind::Hands => "hands",
        }
    }
}

/// One tool dialect, as loaded from `dialects/tools/<name>.json`.
#[derive(Debug, Clone, PartialEq)]
pub struct ToolDialect {
    pub name: String,
    pub serves: String,
    pub kind: DialectKind,
    pub tools: Vec<String>,
    pub classes: Option<Vec<String>>,
    /// Decision 0036's class; absent reads `uncontracted`.
    pub egress: String,
    pub seat_composed: bool,
    /// The embedded schema a grant's restriction keys are judged against.
    pub restrictions: Value,
    pub source: String,
    pub sha256: String,
}

/// Every `$ref` in an embedded schema stays inside it, and no `properties`
/// or `required` anywhere names a key the engine owns.
fn embedded_schema_fault(schema: &Value) -> Option<String> {
    match schema {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                if !reference.starts_with('#') {
                    return Some(format!(
                        "references '{reference}', which is outside the dialect file; a \
                         restriction schema is never fetched"
                    ));
                }
            }
            let named = map
                .get("properties")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|properties| properties.keys().map(String::as_str))
                .chain(
                    map.get("required")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str),
                );
            for key in named {
                if brokkr_core::realms::GRANT_KEYS.contains(&key) {
                    return Some(format!(
                        "redefines '{key}', which is a key of the grant the engine owns"
                    ));
                }
            }
            map.values().find_map(embedded_schema_fault)
        }
        Value::Array(items) => items.iter().find_map(embedded_schema_fault),
        _ => None,
    }
}

impl ToolDialect {
    pub fn source_of(name: &str) -> String {
        format!("{DIALECTS_DIR}/{name}.json")
    }

    /// Load one dialect by library name. Executes nothing and contacts
    /// nothing: an `mcp` dialect's argv and URL are read as data.
    pub fn load(root: &Path, name: &str) -> Result<ToolDialect, String> {
        let source = ToolDialect::source_of(name);
        let (value, sha256) = read_document(root, &source)?.ok_or_else(|| {
            format!("tool dialect '{name}' is not at '{source}' in the operator configuration")
        })?;
        // Judged in two steps so a refusal names the FIELD: first what
        // every dialect shares — which settles its kind — then the one
        // branch that kind selects. The branches are told apart by `kind`
        // alone, so this admits exactly what the whole contract admits,
        // without the contract's `oneOf` reducing every fault to "none of
        // the three".
        let contract = embedded(TOOL_DIALECT_SCHEMA);
        let outside = |problem: String| {
            format!("tool dialect '{source}' is outside brokkr.tool-dialect/v1 {problem}")
        };
        let mut shared = contract.clone();
        shared["oneOf"] = json!([{}]);
        violation(&shared, &value).map_err(outside)?;
        let branch = contract["oneOf"]
            .as_array()
            .into_iter()
            .flatten()
            .find(|branch| branch["properties"]["kind"]["const"] == value["kind"])
            .expect("the contract's kind enum and its branches name the same kinds");
        violation(branch, &value).map_err(outside)?;
        let text = |key: &str| value[key].as_str().unwrap_or_default().to_string();
        if text("name") != name {
            return Err(format!(
                "tool dialect '{source}' names itself '{}'; a dialect's name is its file's stem",
                text("name")
            ));
        }
        let restrictions = value
            .get("restrictions")
            .cloned()
            .unwrap_or_else(|| json!({"type": "object", "additionalProperties": false}));
        if let Some(fault) = embedded_schema_fault(&restrictions) {
            return Err(format!("tool dialect '{source}' restriction schema {fault}"));
        }
        jsonschema::draft7::new(&restrictions).map_err(|error| {
            format!("tool dialect '{source}' restriction schema is not valid draft-07: {error}")
        })?;
        let kind = match value["kind"].as_str() {
            Some("provider-native") => DialectKind::Native {
                provider: text("provider"),
                adapter_key: text("adapter_key"),
            },
            Some("mcp") => DialectKind::Mcp,
            _ => DialectKind::Hands,
        };
        let strings = |key: &str| -> Option<Vec<String>> {
            serde_json::from_value(value.get(key)?.clone()).ok()
        };
        Ok(ToolDialect {
            name: name.to_string(),
            serves: text("serves"),
            kind,
            tools: strings("tools").unwrap_or_default(),
            classes: strings("classes"),
            egress: value["egress"].as_str().unwrap_or("uncontracted").to_string(),
            seat_composed: value["sends"]["seat_composed"] == json!(true),
            restrictions,
            source,
            sha256,
        })
    }

    fn value(&self) -> Value {
        json!({"source": self.source, "sha256": self.sha256,
               "kind": self.kind.word(), "serves": self.serves})
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
}

/// What resolution reads of one provider: its name, its native
/// declaration, and the digest of the file that declaration came from.
#[derive(Debug, Clone, Copy)]
pub struct Serving<'a> {
    pub provider: &'a str,
    pub model: Option<&'a str>,
    /// `None` where no adapter answers for the provider at all.
    pub native: Option<(&'a NativeInventory, &'a str)>,
    /// The argv the seat or its agent AUTHORED, judged for contenders.
    pub authored: &'a [String],
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
}

/// What the launch is composed with.
#[derive(Debug, Clone, PartialEq)]
pub enum NativePlan {
    Known {
        declaration: String,
        on: Vec<String>,
        off: Vec<String>,
        /// Known powers whose OFF control nobody has measured: not held,
        /// and not claimed denied.
        unmeasured: Vec<String>,
        /// The driver input's `native_controls`.
        controls: Value,
    },
    Unmeasured {
        declaration: Option<String>,
        reason: String,
    },
}

/// The sealed outcome for one site and one provider candidate. Only
/// [`Authority::resolve`] constructs one; the manifest record, the driver
/// input and the prompt paragraph are projections of it.
#[derive(Debug, Clone, PartialEq)]
pub struct Outcome {
    pub provider: String,
    pub model: Option<String>,
    pub held: BTreeMap<String, Holding>,
    pub not_held: BTreeMap<String, String>,
    /// One per wanted capability this candidate lost: the capability, and
    /// the complete sentence saying who lost it, where and why.
    pub notices: Vec<(String, String)>,
    pub native: NativePlan,
}

impl Outcome {
    /// The per-candidate record of `run-manifest/v11`.
    pub fn manifest(&self) -> Value {
        let held: Map<String, Value> = self
            .held
            .iter()
            .map(|(name, holding)| {
                (
                    name.clone(),
                    json!({
                        "classes": holding.classes,
                        "dialect": holding.dialect,
                        "dialect_sha256": holding.dialect_sha256,
                        "definition_sha256": holding.definition_sha256,
                        "tools": holding.tools,
                        "restrictions": holding.restrictions,
                    }),
                )
            })
            .collect();
        let native = match &self.native {
            NativePlan::Known {
                declaration,
                on,
                off,
                unmeasured,
                ..
            } => {
                let mut native = json!({"inventory": "known", "declaration": declaration,
                                        "on": on, "off": off});
                if !unmeasured.is_empty() {
                    native["unmeasured"] = json!(unmeasured);
                }
                native
            }
            NativePlan::Unmeasured {
                declaration,
                reason,
            } => {
                let mut native = json!({"inventory": "unmeasured", "reason": reason});
                if let Some(declaration) = declaration {
                    native["declaration"] = json!(declaration);
                }
                native
            }
        };
        let mut record = json!({
            "provider": self.provider,
            "held": held,
            "not_held": self.not_held,
            "notices": self.notices.iter().map(|(_, message)| message).collect::<Vec<_>>(),
            "native": native,
        });
        if let Some(model) = &self.model {
            record["model"] = json!(model);
        }
        record
    }

    /// The driver input's `native_controls`: the engine-owned plan the
    /// protocol composes the final argv from.
    pub fn controls(&self) -> Value {
        match &self.native {
            NativePlan::Known { controls, .. } => controls.clone(),
            NativePlan::Unmeasured { reason, .. } => {
                json!({"inventory": "unmeasured", "reason": reason})
            }
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
    /// The per-site record of `run-manifest/v11`.
    pub fn manifest(&self) -> Value {
        let asks: Map<String, Value> = self
            .asks
            .asks
            .iter()
            .map(|(name, strength)| (name.clone(), json!(strength.word())))
            .collect();
        json!({
            "office": self.asks.office,
            "asks": asks,
            "subtracted": self.asks.subtracted,
            "candidates": self.outcomes.iter().map(Outcome::manifest).collect::<Vec<_>>(),
        })
    }

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

/// Why an ask is not held. `through` names the dialect once a grant was
/// found; `denied` says whether a native OFF stands behind the loss.
struct Cause {
    through: Option<String>,
    but: String,
    denied: bool,
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
    /// Capability to the `(provider, adapter key)` its dialect binds.
    /// Every grant has one: a grant of any other kind was refused.
    bindings: BTreeMap<String, (String, String)>,
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
            let dialect = ToolDialect::load(&context.root, &grant.dialect)
                .map_err(|problem| format!("realm '{realm}': capability '{capability}': {problem}"))?;
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
        let mut bindings = BTreeMap::new();
        for (capability, dialect) in &dialects {
            match &dialect.kind {
                DialectKind::Native {
                    provider,
                    adapter_key,
                } => {
                    bindings.insert(
                        capability.clone(),
                        (provider.clone(), adapter_key.clone()),
                    );
                }
                DialectKind::Mcp => {
                    return Err(format!(
                        "realm '{realm}' grants capability '{capability}' through dialect '{}' \
                         of kind 'mcp', whose broker support is not implemented until decision \
                         0065 slice two",
                        dialect.name
                    ))
                }
                DialectKind::Hands => {
                    return Err(format!(
                        "realm '{realm}' grants capability '{capability}' through dialect '{}' \
                         of kind 'hands', which is reserved: the workspace tool stays governed \
                         by decisions 0043 and 0046 and is not a realm grant",
                        dialect.name
                    ))
                }
            }
        }
        Ok(Authority {
            context,
            definitions,
            dialects,
            bindings,
        })
    }

    /// The manifest's realm-wide half: the grants as written, and every
    /// definition and dialect the compile consulted, used or not.
    pub fn manifest(&self, consulted: &[String]) -> Value {
        let grants: Map<String, Value> = self
            .context
            .grants
            .iter()
            .map(|(capability, grant)| (capability.clone(), grant.value()))
            .collect();
        let definitions: Map<String, Value> = self
            .context
            .grants
            .keys()
            .chain(consulted)
            .filter_map(|name| Some((name.clone(), self.definitions.get(name)?.value())))
            .collect();
        let dialects: Map<String, Value> = self
            .dialects
            .values()
            .map(|dialect| (dialect.name.clone(), dialect.value()))
            .collect();
        json!({
            "realm": self.context.realm,
            "grants": grants,
            "definitions": definitions,
            "dialects": dialects,
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
            bindings: BTreeMap::new(),
        }
    }

    /// The `(provider, adapter key)` a granted capability is bound to —
    /// what `brokkr doctor` compares an installed harness's native
    /// declarations against. A same-name grant bound to another provider
    /// covers nothing of this one's.
    pub fn binding(&self, capability: &str) -> Option<(&str, &str)> {
        self.bindings
            .get(capability)
            .map(|(provider, key)| (provider.as_str(), key.as_str()))
    }

    fn who(&self, site: &SiteAsks) -> String {
        format!(
            "seat '{}' (office '{}') in realm '{}'",
            site.label, site.office, self.context.realm
        )
    }

    /// Why this candidate cannot hold `capability`, or the holding and
    /// the native key that serves it.
    fn holding(
        &self,
        site: &SiteAsks,
        capability: &str,
        serving: &Serving<'_>,
    ) -> Result<(Holding, String), Cause> {
        let bare = |but: String| Cause {
            through: None,
            but,
            denied: true,
        };
        let Some(grant) = self.context.grants.get(capability) else {
            return Err(bare("the realm does not grant it to this office".into()));
        };
        if !grant.reaches(&site.office) {
            let scope = grant.offices.as_deref().unwrap_or_default();
            return Err(bare(match scope.is_empty() {
                true => "the realm grants it to no office".to_string(),
                false => format!(
                    "the realm grants it only to offices [{}], not to this office",
                    scope.join(", ")
                ),
            }));
        }
        let dialect = &self.dialects[capability];
        let through = |but: String, denied: bool| Cause {
            through: Some(dialect.name.clone()),
            but,
            denied,
        };
        let tools = grant.tools.clone().unwrap_or_else(|| dialect.tools.clone());
        if tools.is_empty() {
            return Err(through("the realm's grant admits no tool".into(), true));
        }
        let provider = serving.provider;
        let (bound, adapter_key) = &self.bindings[capability];
        if bound != provider {
            return Err(through(
                format!(
                    "provider '{provider}' cannot carry a binding to provider '{bound}'"
                ),
                true,
            ));
        }
        let known = match serving.native {
            Some((NativeInventory::Known { known, .. }, _)) => known,
            Some((NativeInventory::Unmeasured(reason), _)) => {
                return Err(through(
                    format!(
                        "provider '{provider}' declares its native capabilities unmeasured \
                         ({reason})"
                    ),
                    false,
                ))
            }
            None => {
                return Err(through(
                    format!("no adapter declares provider '{provider}'"),
                    false,
                ))
            }
        };
        let Some(native) = known
            .get(adapter_key)
            .filter(|native| native.capability == capability)
        else {
            return Err(through(
                format!(
                    "provider '{provider}' declares no native capability '{adapter_key}' \
                     serving it"
                ),
                true,
            ));
        };
        if let Some(tool) = tools.iter().find(|tool| !native.tools.contains(tool)) {
            return Err(through(
                format!("provider '{provider}' native '{adapter_key}' has no tool '{tool}'"),
                true,
            ));
        }
        match &native.on {
            Disposition::Unsupported(reason) => {
                return Err(through(
                    format!("provider '{provider}' cannot switch it on ({reason})"),
                    true,
                ))
            }
            Disposition::Unmeasured(reason) => {
                return Err(through(
                    format!(
                        "provider '{provider}' declares its ON control unmeasured ({reason})"
                    ),
                    true,
                ))
            }
            // A whole-set switch cannot admit a proper subset: it would
            // enable a tool the realm excluded.
            Disposition::Argv(_) | Disposition::Default(_) if tools.len() < native.tools.len() => {
                return Err(through(
                    format!(
                        "provider '{provider}' switches native '{adapter_key}' on as a whole \
                         and cannot admit only [{}] of its tools [{}]",
                        tools.join(", "),
                        native.tools.join(", ")
                    ),
                    true,
                ))
            }
            _ => {}
        }
        if !grant.restrictions.is_empty() {
            if let Transport::Unsupported(_) = native.restrictions {
                let names = restriction_names("", &grant.restrictions);
                return Err(through(
                    format!(
                        "provider '{provider}' cannot express restriction '{}'",
                        names.join("', '")
                    ),
                    true,
                ));
            }
        }
        let definition = &self.definitions.0[capability];
        Ok((
            Holding {
                classes: definition.classes.clone(),
                dialect: dialect.name.clone(),
                dialect_sha256: dialect.sha256.clone(),
                definition_sha256: definition.sha256.clone(),
                tools,
                restrictions: grant.restrictions.clone(),
            },
            adapter_key.clone(),
        ))
    }

    /// Resolve one site against one provider candidate (design D4 steps
    /// 3 to 5). `Err` is a complete compile refusal.
    pub fn resolve(&self, site: &SiteAsks, serving: &Serving<'_>) -> Result<Outcome, String> {
        let who = self.who(site);
        for name in site.asks.keys().chain(&site.subtracted) {
            self.definitions.require(&who, name)?;
        }
        let mut held = BTreeMap::new();
        let mut keys: BTreeMap<String, String> = BTreeMap::new();
        let mut not_held = BTreeMap::new();
        let mut notices = Vec::new();
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
                Err(cause) => {
                    let through = cause
                        .through
                        .as_ref()
                        .map(|dialect| format!(" through dialect '{dialect}'"))
                        .unwrap_or_default();
                    if *strength == Strength::Requires {
                        return Err(match cause.through {
                            None => format!(
                                "{who}: requires capability '{capability}' but {}",
                                cause.but
                            ),
                            Some(_) => format!(
                                "{who}: requires capability '{capability}'{through}, but {}; \
                                 the capability cannot be held under this grant",
                                cause.but
                            ),
                        });
                    }
                    let tail = match (&cause.through, cause.denied) {
                        (None, _) => "",
                        (Some(_), true) => "; native capability remains OFF",
                        (Some(_), false) => "; no native denial is claimed",
                    };
                    notices.push((
                        capability.clone(),
                        format!(
                            "{who}: dropped wanted capability '{capability}'{through} because \
                             {}{tail}",
                            cause.but
                        ),
                    ));
                    not_held.insert(capability.clone(), cause.but);
                }
            }
        }
        let native = self.native_plan(&who, serving, &held, &keys, &mut not_held)?;
        Ok(Outcome {
            provider: serving.provider.to_string(),
            model: serving.model.map(str::to_string),
            held,
            not_held,
            notices,
            native,
        })
    }

    /// Independently of every ask (design D4 step 4): each native power
    /// this candidate's harness is known to have is switched ON if held
    /// through it and OFF otherwise — and one that cannot be switched off
    /// refuses the seat (ruling 4).
    fn native_plan(
        &self,
        who: &str,
        serving: &Serving<'_>,
        held: &BTreeMap<String, Holding>,
        keys: &BTreeMap<String, String>,
        not_held: &mut BTreeMap<String, String>,
    ) -> Result<NativePlan, String> {
        let provider = serving.provider;
        let (known, selection, declaration) = match serving.native {
            Some((NativeInventory::Known { known, selection }, digest)) => {
                (known, selection, digest)
            }
            Some((NativeInventory::Unmeasured(reason), digest)) => {
                return Ok(NativePlan::Unmeasured {
                    declaration: Some(digest.to_string()),
                    reason: reason.clone(),
                })
            }
            None => {
                return Ok(NativePlan::Unmeasured {
                    declaration: None,
                    reason: format!("no adapter declares provider '{provider}'"),
                })
            }
        };
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
        if let Some((written, capability)) =
            brokkr_protocol::native_controls::authored_conflict(serving.authored, &guards)
        {
            return Err(format!(
                "{who}: its arguments carry '{written}', which controls native capability \
                 '{capability}' of provider '{provider}'. Only the realm grants a capability, \
                 and the engine composes the one control the grant resolves to; request \
                 '{capability}' by name under 'capabilities' instead (decision 0065 rulings 3 \
                 and 4)"
            ));
        }
        let (mut argv, mut lists) = (Vec::new(), ToolLists::default());
        let (mut on, mut off, mut unmeasured) = (Vec::new(), Vec::new(), Vec::new());
        for (key, native) in known {
            let holding = keys.get(key).map(|capability| &held[capability]);
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
                (Disposition::Unsupported(reason), _) => {
                    return Err(format!(
                        "{who}: provider '{provider}' cannot switch off its native capability \
                         '{}', which this seat does not hold ({reason}); an ungranted native \
                         capability that cannot be disabled cannot be seated in this realm \
                         (decision 0065 ruling 4)",
                        native.capability
                    ))
                }
                (Disposition::Unmeasured(reason), _) => {
                    unmeasured.push(key.clone());
                    not_held.entry(native.capability.clone()).or_insert(format!(
                        "provider '{provider}' has it natively and its OFF control is \
                         unmeasured ({reason}); it is not granted and no denial is claimed"
                    ));
                    continue;
                }
                (Disposition::Default(_), _) => {}
            }
            match holding {
                Some(holding) => {
                    if let Transport::Argv(template) = &native.restrictions {
                        if !holding.restrictions.is_empty() {
                            let encoded = String::from_utf8_lossy(&to_bytes(&Value::Object(
                                holding.restrictions.clone(),
                            )))
                            .into_owned();
                            argv.extend(
                                template
                                    .iter()
                                    .map(|part| part.replace(RESTRICTIONS_SLOT, &encoded)),
                            );
                        }
                    }
                    on.push(key.clone());
                }
                None => {
                    off.push(key.clone());
                    not_held.entry(native.capability.clone()).or_insert(format!(
                        "provider '{provider}' has it natively, the realm does not grant it to \
                         this seat, and it is switched off"
                    ));
                }
            }
        }
        let mut controls = json!({
            "inventory": "known",
            "argv": argv,
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
        Ok(NativePlan::Known {
            declaration: declaration.to_string(),
            on,
            off,
            unmeasured,
            controls,
        })
    }
}

#[cfg(test)]
mod tests;
