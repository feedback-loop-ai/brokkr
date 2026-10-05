//! A tool dialect, `dialects/tools/<name>.json`, read once into a type
//! (decision 0065 ruling 2; slice two, SC1). The frozen v1 contract stays
//! the source: the document is judged against it first, then parsed from
//! the same bound bytes into the kind it selects, with every executable
//! `mcp` fact kept — connection, version, secret names and the declared
//! retention — beside the tools, sends, egress class and restriction
//! schema every kind shares. Loading executes nothing, opens no store and
//! contacts nothing: an argv, a URL and a secret name are read as data.

use std::path::Path;

use brokkr_core::realms::{GrantRetention, GRANT_KEYS};
use jsonschema::{Draft, Registry};
use serde::Deserialize;
use serde_json::{json, Map, Value};

use super::{embedded, read_document, violation, DIALECTS_DIR};
use crate::agents::EgressClass;

pub(super) const TOOL_DIALECT_SCHEMA: &str = include_str!("../tool-dialect.v1.schema.json");

/// The grant key `forge.realms/v8` reserves beside the three every
/// grant-bearing version reserves (SC2).
const RETAIN: &str = "retain";

/// How an `mcp` server is reached: exactly one form, as the contract
/// admits it.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum Connection {
    /// A stdio launch, run directly and never through a shell.
    #[serde(rename = "argv")]
    Stdio(Vec<String>),
    /// A server URL without userinfo.
    #[serde(rename = "url")]
    Url(String),
}

/// What an `mcp` dialect declares about its server, kept whole (SC1).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct McpServer {
    pub connection: Connection,
    /// The `serverInfo` version the server's `initialize` must report.
    pub version: String,
    /// Decision 0012 binding NAMES, never values.
    pub secrets: Vec<String>,
    /// Whether the dialect opts its responses into retention (CR1).
    /// Omitted reads false; a realm may veto it and never require it.
    #[serde(default)]
    pub retained: bool,
}

/// The one implementation kind a tool dialect binds (ruling 2).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(tag = "kind")]
pub enum DialectKind {
    /// A capability a harness already has, addressed through its adapter.
    #[serde(rename = "provider-native")]
    Native {
        provider: String,
        adapter_key: String,
    },
    /// An MCP server, whole as data; refused as a grant until slice two
    /// enables its broker (U9b).
    #[serde(rename = "mcp")]
    Mcp(McpServer),
    /// Reserved: decision 0043's workspace tool, which does not move.
    #[serde(rename = "hands")]
    Hands,
}

impl DialectKind {
    pub fn word(&self) -> &'static str {
        match self {
            DialectKind::Native { .. } => "provider-native",
            DialectKind::Mcp(_) => "mcp",
            DialectKind::Hands => "hands",
        }
    }
}

/// What a dialect discloses: a fetch of a named URL and a free-text query
/// a model composes are different disclosures.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Sends {
    pub description: String,
    pub seat_composed: bool,
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
    pub egress: EgressClass,
    pub sends: Sends,
    /// The embedded schema a grant's restriction keys are judged against.
    pub restrictions: Value,
    pub source: String,
    pub sha256: String,
}

/// The typed reading of a document the contract has already admitted.
#[derive(Deserialize)]
struct Document {
    name: String,
    serves: String,
    tools: Vec<String>,
    classes: Option<Vec<String>>,
    egress: Option<String>,
    sends: Sends,
    restrictions: Option<Value>,
    #[serde(flatten)]
    kind: DialectKind,
}

/// A `forge.realms/v8` grant selected a dialect whose restriction schema
/// directly claims the reserved `retain` (SC2): an authority collision,
/// refused rather than reread.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("tool dialect '{dialect}' restriction schema redefines reserved grant key '{key}'", key = RETAIN)]
pub(super) struct RetainReserved {
    dialect: String,
}

/// The draft an embedded restriction schema is read as. One written
/// against any other is refused rather than judged by rules it did not
/// mean.
const DRAFT_07: &str = "http://json-schema.org/draft-07/schema";

/// The base a schema without a root `$id` is indexed from: jsonschema's
/// own (`compiler::DEFAULT_BASE_URI`, private to that crate), so the index
/// and the validator resolve every reference from one base.
const DEFAULT_BASE: &str = "json-schema:///";

/// The keywords whose value is DATA, not a schema: a `$ref` spelled inside
/// one is an example of a reference, not a reference.
const DATA_KEYWORDS: [&str; 4] = ["const", "default", "enum", "examples"];

/// What an embedded restriction schema is refused for (SC1, SC2): before
/// it is ever compiled, and by the compiler itself. No text repeats an
/// authored value — a schema, a reference or an `$id` may be where a
/// credential was pasted — only the keyword or the reserved key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum SchemaFault {
    #[error("declares a '$schema' other than draft-07 ('{DRAFT_07}#')")]
    Draft,
    #[error("has a '$ref' outside the dialect file; a restriction schema is never fetched")]
    External,
    #[error("has a '$ref' that names nothing in the dialect file")]
    Missing,
    #[error(
        "cannot be indexed by the draft-07 validator's resolver: one of its references leaves the \
         dialect file, which is never fetched, or one of its '$id's is not a URI"
    )]
    Unindexed,
    #[error("redefines '{key}', which is a key of the grant the engine owns")]
    Reserved { key: &'static str },
    #[error("is not valid draft-07: the validator does not compile it")]
    Uncompiled,
}

/// What is wrong with an embedded restriction schema before it is ever
/// compiled: it is written against another draft, a `$ref` anywhere in it
/// leaves the dialect file or names nothing in it, the validator's resolver
/// cannot index it without fetching, or it directly claims a key every
/// grant-bearing version reserves.
pub(super) fn embedded_schema_fault(schema: &Value) -> Option<SchemaFault> {
    if let Some(draft) = schema.get("$schema") {
        if draft.as_str().map(|uri| uri.trim_end_matches('#')) != Some(DRAFT_07) {
            return Some(SchemaFault::Draft);
        }
    }
    if let Some(fault) = reference_fault(schema, schema) {
        return Some(fault);
    }
    if indexed(schema).is_none() {
        return Some(SchemaFault::Unindexed);
    }
    claimed(schema, &GRANT_KEYS).map(|key| SchemaFault::Reserved { key })
}

/// Every `$ref` stays inside the dialect file and resolves in it as a
/// plain JSON pointer. Walked through the whole document, data keywords
/// aside: a reference is never fetched, wherever it hides, and one the
/// resolver could answer without a fetch is still not the file's own.
fn reference_fault(root: &Value, node: &Value) -> Option<SchemaFault> {
    match node {
        Value::Object(map) => {
            if let Some(reference) = map.get("$ref").and_then(Value::as_str) {
                let Some(pointer) = reference.strip_prefix('#') else {
                    return Some(SchemaFault::External);
                };
                if root.pointer(pointer).is_none() {
                    return Some(SchemaFault::Missing);
                }
            }
            map.iter()
                .filter(|(keyword, _)| !DATA_KEYWORDS.contains(&keyword.as_str()))
                .find_map(|(_, value)| reference_fault(root, value))
        }
        Value::Array(items) => items.iter().find_map(|item| reference_fault(root, item)),
        _ => None,
    }
}

/// The schema indexed exactly as `jsonschema::draft7::new` indexes it — by
/// its root `$id`, or jsonschema's default base — with a retriever that
/// fetches nothing.
fn indexed(schema: &Value) -> Option<Registry<'_>> {
    let resource = Draft::Draft7.create_resource_ref(schema);
    let base = jsonschema::uri::from_str(resource.id().unwrap_or(DEFAULT_BASE)).ok()?;
    Registry::new()
        .draft(Draft::Draft7)
        .add(base.as_str(), resource)
        .ok()?
        .prepare()
        .ok()
}

/// The first of `reserved` the schema claims DIRECTLY at the grant's own
/// top level (operator ruling, 2026-10-04): a key of its root `properties`
/// or `dependencies`, or an entry of its root `required` or of a list in
/// its root `dependencies`. A claim made only through a reference,
/// composition, a conditional or a dependency schema is not searched for:
/// it is inert, because the grant's reserved keys are stripped before its
/// restrictions are validated, so no schema decides one
/// (`a_reserved_key_never_reaches_restriction_validation`).
fn claimed(schema: &Value, reserved: &[&'static str]) -> Option<&'static str> {
    let named = ["properties", "dependencies"]
        .into_iter()
        .filter_map(|keyword| schema.get(keyword)?.as_object())
        .flat_map(|named| named.keys().map(String::as_str));
    let listed = schema
        .get("required")
        .into_iter()
        .chain(
            schema
                .get("dependencies")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(Map::values),
        )
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(Value::as_str);
    named
        .chain(listed)
        .find_map(|key| reserved.iter().copied().find(|word| *word == key))
}

/// An `mcp` launch argument naming a credential other than by a declared
/// decision 0012 binding (SC1). Neither text repeats the argument, which
/// may be where somebody pasted a credential by hand.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum Undeclared {
    #[error(
        "tool dialect '{file}' connection argv[{index}] carries a malformed secret reference; a \
         reference is {{{{secret:NAME}}}} with NAME matching [A-Z][A-Z0-9_]*"
    )]
    Malformed { file: String, index: usize },
    #[error(
        "tool dialect '{file}' connection argv[{index}] references secret '{name}', which its \
         'secrets' does not declare; a server reaches only the bindings its dialect names \
         (decision 0012)"
    )]
    Secret {
        file: String,
        index: usize,
        name: String,
    },
}

/// A document outside the frozen contract, at the field and clause the
/// validator names (SC1).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("tool dialect '{file}' is outside brokkr.tool-dialect/v1 {problem}")]
pub(super) struct Outside {
    pub(super) file: String,
    pub(super) problem: String,
}

impl McpServer {
    /// An `mcp` launch names a credential only by decision 0012's
    /// `{{secret:NAME}}`, and only a NAME the dialect declares under
    /// `secrets`. Judged as text: no store is opened. The contract's
    /// pattern already keeps userinfo out of a `url`.
    pub(super) fn undeclared(&self, source: &str) -> Result<(), Undeclared> {
        let argv = match &self.connection {
            Connection::Stdio(argv) => argv.as_slice(),
            Connection::Url(_) => &[],
        };
        for (index, part) in argv.iter().enumerate() {
            let names = brokkr_protocol::secret::scan_secret_refs(part).map_err(|_| {
                Undeclared::Malformed {
                    file: source.into(),
                    index,
                }
            })?;
            if let Some(name) = names.iter().find(|name| !self.secrets.contains(name)) {
                return Err(Undeclared::Secret {
                    file: source.into(),
                    index,
                    name: name.clone(),
                });
            }
        }
        Ok(())
    }
}

/// The document against the contract, judged in two steps so a refusal
/// names the FIELD: first what every dialect shares — which settles its
/// kind — then the one branch that kind selects. The branches are told
/// apart by `kind` alone, so this admits exactly what the whole contract
/// admits, without the contract's `oneOf` reducing every fault to "none of
/// the three".
pub(super) fn conforms(source: &str, value: &Value) -> Result<(), Outside> {
    let contract = embedded(TOOL_DIALECT_SCHEMA);
    let outside = |problem: String| Outside {
        file: source.into(),
        problem,
    };
    let mut shared = contract.clone();
    shared["oneOf"] = json!([{}]);
    violation(&shared, value).map_err(outside)?;
    let branch = contract["oneOf"]
        .as_array()
        .into_iter()
        .flatten()
        .find(|branch| branch["properties"]["kind"]["const"] == value["kind"])
        .expect("the contract's kind enum and its branches name the same kinds");
    violation(branch, value).map_err(outside)
}

impl ToolDialect {
    pub fn source_of(name: &str) -> String {
        format!("{DIALECTS_DIR}/{name}.json")
    }

    /// Load one dialect by library name: the contract, then its name, then
    /// the kind's own facts — an `mcp` launch's secret references — then
    /// the restriction schema (SC1's order). Executes nothing and contacts
    /// nothing.
    pub fn load(root: &Path, name: &str) -> Result<ToolDialect, String> {
        let source = ToolDialect::source_of(name);
        let (value, sha256) = read_document(root, &source)?.ok_or_else(|| {
            format!("tool dialect '{name}' is not at '{source}' in the operator configuration")
        })?;
        conforms(&source, &value).map_err(|outside| outside.to_string())?;
        let document = Document::deserialize(&value)
            .expect("a document the contract admits reads as its typed kind");
        if document.name != name {
            return Err(format!(
                "tool dialect '{source}' names itself '{}'; a dialect's name is its file's stem",
                document.name
            ));
        }
        match &document.kind {
            DialectKind::Mcp(server) => server
                .undeclared(&source)
                .map_err(|undeclared| undeclared.to_string())?,
            DialectKind::Native { .. } | DialectKind::Hands => {}
        }
        let restrictions = document
            .restrictions
            .unwrap_or_else(|| json!({"type": "object", "additionalProperties": false}));
        // The whole schema is still compiled, but the compiler's own words
        // may repeat an authored pointer or value, so its refusal is named
        // by the typed fault alone.
        let compiled = || {
            jsonschema::draft7::new(&restrictions)
                .err()
                .map(|_| SchemaFault::Uncompiled)
        };
        if let Some(fault) = embedded_schema_fault(&restrictions).or_else(compiled) {
            return Err(format!(
                "tool dialect '{source}' restriction schema {fault}"
            ));
        }
        let egress = document
            .egress
            .as_deref()
            .map_or(Some(EgressClass::Uncontracted), EgressClass::parse)
            .expect("the contract's egress enum is decision 0036's vocabulary");
        Ok(ToolDialect {
            name: document.name,
            serves: document.serves,
            kind: document.kind,
            tools: document.tools,
            classes: document.classes,
            egress,
            sends: document.sends,
            restrictions,
            source,
            sha256,
        })
    }

    /// Whether a grant written in a map version that reserves `retain`
    /// (`forge.realms/v8`) may select this dialect: its restriction schema
    /// must not directly claim `retain`, judged exactly as for the three
    /// keys every version reserves (SC2). A v6 or v7 grant reserves no
    /// `retain`, so there a written one stays the dialect's restriction.
    pub(super) fn reserves(&self, retention: GrantRetention) -> Result<(), RetainReserved> {
        let claimed = match retention {
            GrantRetention::Unreserved => None,
            GrantRetention::Inherit | GrantRetention::Veto => {
                claimed(&self.restrictions, &[RETAIN])
            }
        };
        match claimed {
            None => Ok(()),
            Some(_) => Err(RetainReserved {
                dialect: self.name.clone(),
            }),
        }
    }

    pub(super) fn value(&self) -> Value {
        json!({"source": self.source, "sha256": self.sha256,
               "kind": self.kind.word(), "serves": self.serves})
    }
}
