//! The I/O boundary of the agent library: two directories become a
//! `Library` and an `Adapters`. Everything that touches the filesystem
//! lives here, on purpose, so `agents.rs` — the resolver — can be a pure
//! function of its arguments and be proven so by inspection.
//!
//! Parsing is strict in both trees: unknown keys are rejected outright
//! (a typo must not be a silently ignored capability), names obey
//! `^[a-z][a-z0-9-]*$`, charter paths are canonicalised and must be
//! CONTAINED within the library root, and a `secrets.env` anywhere under
//! either root is refused exactly as `manifest_for` refuses one inside a
//! bundle (decision 0012: digests carry names, never values).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::canonical::{sha256_bytes, sha256_hex};
use serde_json::{Map, Value};
use thiserror::Error;

use super::{
    valid_name, Adapter, Agent, EgressClass, McpNeed, McpSupport, ToolPermissions, TrustTier,
    NAME_GRAMMAR,
};
use crate::bundle::Limits;

#[derive(Debug, Error)]
pub enum LibraryError {
    #[error("{0}")]
    Invalid(String),
    #[error("agent library io: {0}")]
    Io(#[from] std::io::Error),
}

fn invalid<T>(message: String) -> Result<T, LibraryError> {
    Err(LibraryError::Invalid(message))
}

/// The `.json` files directly under `root`, sorted, with their parsed
/// bodies — plus the `secrets.env` refusal over the whole tree.
fn definition_files(root: &Path, kind: &str) -> Result<Vec<(String, PathBuf)>, LibraryError> {
    refuse_secret_stores(root, kind)?;
    let mut found = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let path = entry?.path();
        let is_definition =
            path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("json");
        if !is_definition {
            continue;
        }
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default()
            .to_string();
        found.push((stem, path));
    }
    found.sort();
    Ok(found)
}

/// A secrets store under the library or adapter tree would ride into a
/// digest exactly as one inside a bundle would — an offline-guessing
/// oracle (decision 0012, layer 2).
fn refuse_secret_stores(root: &Path, kind: &str) -> Result<(), LibraryError> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.file_name().and_then(|n| n.to_str()) == Some("secrets.env") {
                return invalid(format!(
                    "the {kind} tree contains a secrets store '{}'; the store must \
                     live outside it (e.g. .forge/secrets.env) so digests carry \
                     names only",
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn read_json(path: &Path) -> Result<Value, LibraryError> {
    let bytes = std::fs::read(path)?;
    serde_json::from_slice(&bytes)
        .map_err(|e| LibraryError::Invalid(format!("{}: {e}", path.display())))
}

fn object<'a>(value: &'a Value, what: &str) -> Result<&'a Map<String, Value>, LibraryError> {
    match value.as_object() {
        Some(map) => Ok(map),
        None => invalid(format!("{what} must be a JSON object")),
    }
}

/// Unknown keys are rejected outright: a misspelled `tool_permisions`
/// must not read as "this provider declares nothing here".
fn only_keys(map: &Map<String, Value>, allowed: &[&str], what: &str) -> Result<(), LibraryError> {
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            return invalid(format!(
                "{what} has unknown key '{key}'; known keys: {}",
                allowed.join(", ")
            ));
        }
    }
    Ok(())
}

fn string(map: &Map<String, Value>, key: &str, what: &str) -> Result<String, LibraryError> {
    match map.get(key).and_then(Value::as_str) {
        Some(text) if !text.is_empty() => Ok(text.to_string()),
        _ => invalid(format!("{what} needs a non-empty string '{key}'")),
    }
}

fn string_array(
    map: &Map<String, Value>,
    key: &str,
    what: &str,
) -> Result<Vec<String>, LibraryError> {
    let Some(items) = map.get(key).and_then(Value::as_array) else {
        return invalid(format!("{what} needs '{key}' as an array of strings"));
    };
    let mut out = Vec::with_capacity(items.len());
    for item in items {
        match item.as_str() {
            Some(text) => out.push(text.to_string()),
            None => return invalid(format!("{what} '{key}' must hold strings only")),
        }
    }
    Ok(out)
}

fn named(names: &[String], key: &str, what: &str) -> Result<(), LibraryError> {
    for name in names {
        if !valid_name(name) {
            return invalid(format!(
                "{what} '{key}' names '{name}', which does not match {NAME_GRAMMAR}"
            ));
        }
    }
    Ok(())
}

/// A string map whose keys obey the name grammar and whose values are
/// non-empty strings — `models`, `tool_permissions.names`, `mcp.servers`.
fn name_map(
    map: &Map<String, Value>,
    key: &str,
    what: &str,
) -> Result<BTreeMap<String, String>, LibraryError> {
    let Some(entries) = map.get(key).and_then(Value::as_object) else {
        return invalid(format!("{what} needs '{key}' as an object of strings"));
    };
    let mut out = BTreeMap::new();
    for (name, value) in entries {
        if !valid_name(name) {
            return invalid(format!(
                "{what} '{key}' names '{name}', which does not match {NAME_GRAMMAR}"
            ));
        }
        match value.as_str() {
            Some(text) if !text.is_empty() => {
                out.insert(name.clone(), text.to_string());
            }
            _ => return invalid(format!("{what} '{key}.{name}' must be a non-empty string")),
        }
    }
    Ok(out)
}

/// A capability that is either declared as data or declared ABSENT with
/// the explicit string `"unsupported"`. The explicitness is the point:
/// an empty map is ambiguous between "cannot" and "not filled in yet",
/// and this decision exists to refuse that ambiguity.
fn capability<'a>(
    map: &'a Map<String, Value>,
    key: &str,
    what: &str,
) -> Result<Option<&'a Value>, LibraryError> {
    match map.get(key) {
        Some(Value::String(text)) if text == "unsupported" => Ok(None),
        Some(value @ Value::Object(_)) => Ok(Some(value)),
        Some(Value::String(text)) => invalid(format!(
            "{what} '{key}' is the string '{text}'; the only legal string here is \
             \"unsupported\" — a provider's missing capability is declared, never \
             inferred"
        )),
        _ => invalid(format!(
            "{what} needs '{key}': either its mapping, or the explicit string \
             \"unsupported\""
        )),
    }
}

/// A provider's operator-granted trust tier (decision 0021 ruling 2).
/// Absent is `Untrusted` — fail-closed, and NOT an error: a newcomer
/// adapter that declares nothing is exactly ruling 7's symmetric
/// starting position. A tier the vocabulary does not name IS an error,
/// at load time, in the style decision 0004 established for the phase
/// machine: a misspelled `"trusetd"` must never read as untrusted by
/// accident, because then a promotion would silently not have happened.
fn trust_tier(map: &Map<String, Value>, what: &str) -> Result<TrustTier, LibraryError> {
    let Some(declared) = map.get("trust_tier") else {
        return Ok(TrustTier::Untrusted);
    };
    match declared.as_str().and_then(TrustTier::parse) {
        Some(tier) => Ok(tier),
        None => invalid(format!(
            "{what} 'trust_tier' is {declared}; the vocabulary is closed — \
             \"trusted\" or \"untrusted\" — and an absent tier is untrusted"
        )),
    }
}

/// One written egress class, in the closed vocabulary of decision 0036
/// ruling 1. Refused at load time in the style ruling 1 borrows from the
/// tier: a misspelled `"lokal"` must never read as `uncontracted` by
/// accident, because then a route the operator believes they placed
/// would silently not have been placed.
fn egress_class(declared: &Value, key: &str, what: &str) -> Result<EgressClass, LibraryError> {
    match declared.as_str().and_then(EgressClass::parse) {
        Some(class) => Ok(class),
        None => invalid(format!(
            "{what} '{key}' is {declared}; the egress vocabulary is closed — {} \
             — and an absent class is uncontracted (decision 0036 ruling 1)",
            EgressClass::VOCABULARY
        )),
    }
}

/// A provider's own destination class (decision 0036 ruling 2), and the
/// one migration decision 0036 ruling 4 rules: `binding_grant` is
/// superseded, a `true` grant READS as `contracted` and a `false` or
/// absent one as `uncontracted`, so no adapter file on disk is forced to
/// change and every one of them keeps the clearance it has. The old key
/// stays readable for one release; declaring BOTH is refused rather than
/// silently resolved, because the two could then disagree and only one
/// of them could win.
fn adapter_egress(map: &Map<String, Value>, what: &str) -> Result<EgressClass, LibraryError> {
    match (map.get("egress"), map.get("binding_grant")) {
        (Some(_), Some(_)) => invalid(format!(
            "{what} declares both 'egress' and the superseded 'binding_grant'; \
             decision 0036 ruling 4 reads a true grant as \"contracted\" and a \
             false or absent grant as \"uncontracted\", so keep one of them"
        )),
        (Some(declared), None) => egress_class(declared, "egress", what),
        (None, Some(declared)) => match declared.as_bool() {
            Some(true) => Ok(EgressClass::Contracted),
            Some(false) => Ok(EgressClass::Uncontracted),
            None => invalid(format!(
                "{what} 'binding_grant' is {declared}; the grant is a boolean, and \
                 an absent grant is none"
            )),
        },
        (None, None) => Ok(EgressClass::Uncontracted),
    }
}

/// A provider's declared routes (decision 0036 ruling 2): route name →
/// class. Absent is no routes at all, which is the shape of an adapter
/// that fronts a single destination.
fn routes(
    map: &Map<String, Value>,
    what: &str,
) -> Result<BTreeMap<String, EgressClass>, LibraryError> {
    let Some(declared) = map.get("routes") else {
        return Ok(BTreeMap::new());
    };
    let Some(entries) = declared.as_object() else {
        return invalid(format!(
            "{what} 'routes' must be an object of route name → egress class"
        ));
    };
    let mut out = BTreeMap::new();
    for (route, value) in entries {
        if !route_name(route) {
            return invalid(format!(
                "{what} 'routes' names '{route}', which does not match {ROUTE_GRAMMAR}"
            ));
        }
        out.insert(
            route.clone(),
            egress_class(value, &format!("routes.{route}"), what)?,
        );
    }
    Ok(out)
}

/// A provider's declared credential variables (decision 0036 ruling 5):
/// route name → the environment variable that route's endpoint needs.
/// A NAME, never a value — the same rule the whole tree obeys, enforced
/// beside it by `refuse_secret_stores`.
fn credentials(
    map: &Map<String, Value>,
    what: &str,
) -> Result<BTreeMap<String, String>, LibraryError> {
    let Some(declared) = map.get("credentials") else {
        return Ok(BTreeMap::new());
    };
    let Some(entries) = declared.as_object() else {
        return invalid(format!(
            "{what} 'credentials' must be an object of route name → environment \
             variable name"
        ));
    };
    let mut out = BTreeMap::new();
    for (route, value) in entries {
        if !route_name(route) {
            return invalid(format!(
                "{what} 'credentials' names '{route}', which does not match \
                 {ROUTE_GRAMMAR}"
            ));
        }
        match value.as_str().filter(|name| secret_name(name)) {
            Some(variable) => out.insert(route.clone(), variable.to_string()),
            None => {
                return invalid(format!(
                    "{what} 'credentials.{route}' is {value}; a credential is the \
                     NAME of an environment variable ({SECRET_NAME_GRAMMAR}), \
                     never a value"
                ))
            }
        };
    }
    Ok(out)
}

/// The grammar decision 0040 ruling 5 gives a route name, quoted
/// verbatim in both refusals above: a route is the PREFIX
/// `resolve_route` splits off a concrete model id, so it must be able to
/// name every prefix that split can produce — the id's own alphabet
/// (ASCII letters of either case, digits, `-`, `_`, `.` and `:`) minus
/// the `/` that separates the prefix from the rest.
///
/// The agent-name grammar cannot do that job: it is lower case and
/// hyphens only, so `us.east` and `openai_compat` were routes no
/// operator could write, and therefore routes that resolved
/// `Uncontracted` forever with no data able to say otherwise. Ruling 1
/// of decision 0036 makes class assignment operator DATA, and data that
/// cannot be written is not data. [`NAME_GRAMMAR`] stays exactly what it
/// is for agents, adapters and abstract model names.
pub const ROUTE_GRAMMAR: &str = "^[A-Za-z0-9._:-]+$";

/// `true` when `name` matches [`ROUTE_GRAMMAR`].
fn route_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
}

/// The grammar decision 0012 gives a bindable name, quoted verbatim in
/// the refusal above: a credential a route names is the same variable a
/// binding would carry.
pub const SECRET_NAME_GRAMMAR: &str = "^[A-Z][A-Z0-9_]*$";

fn secret_name(name: &str) -> bool {
    let mut characters = name.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_uppercase())
        && characters.all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Decision 0036 ruling 2's route resolver, and the whole of it. Three
/// cases, and the asymmetry between the last two is the whole point:
///
/// - a concrete model id of the form `<route>/<model>` whose route this
///   adapter DECLARES resolves to that route's declared class;
/// - an id whose prefix this adapter does NOT name resolves to
///   `Uncontracted`, because ruling 1 holds that uncontracted is
///   everything else and is the value of an absent declaration — and
///   this file is silent about that route;
/// - an id with NO prefix resolves to the adapter's own declared class
///   and NO BETTER, because an unprefixed id reaches whatever default
///   the harness profile resolves, and the adapter's own class is the
///   operator's word about exactly that destination.
///
/// An unprefixed id genuinely arrives at the adapter's own destination.
/// A prefixed one names a destination the machine positively knows is
/// NOT it. Reading the adapter's class onto a route the file never
/// mentions is the fail-open the decision's first rejected alternative
/// exists to prevent: ruling one provider acceptable for the endpoint it
/// declares would clear every other endpoint the same binary can reach,
/// which is precisely "granting `dsh` the binding grant clears the
/// Alibaba and DeepSeek routes at the same stroke". Silence about a
/// route is not a promotion, and it is not an inheritance either.
///
/// Returns the route it resolved through, so a refusal and a `doctor`
/// line can name it.
pub fn resolve_route<'a>(adapter: &Adapter, model_id: &'a str) -> (Option<&'a str>, EgressClass) {
    match model_id.split_once('/') {
        Some((route, _)) => (
            Some(route),
            adapter
                .routes
                .get(route)
                .copied()
                .unwrap_or(EgressClass::Uncontracted),
        ),
        None => (None, adapter.egress),
    }
}

/// A provider's pinning flag: a flag string, or `"unsupported"`. One
/// reader for both axes a pin travels on — `model_flag` since decision
/// 0016, `effort_flag` since decision 0035 — so a provider that cannot
/// be told one of them declares that in the same words.
fn pin_flag(
    map: &Map<String, Value>,
    key: &str,
    what: &str,
) -> Result<Option<String>, LibraryError> {
    match map.get(key).and_then(Value::as_str) {
        Some("unsupported") => Ok(None),
        Some(flag) if !flag.is_empty() => Ok(Some(flag.to_string())),
        _ => invalid(format!(
            "{what} needs '{key}': the flag this provider takes, or the \
             explicit string \"unsupported\""
        )),
    }
}

// ------------------------------------------------------------- library

/// The agent library: one definition per file, so one broken file is one
/// warning rather than a fatal error for the whole listing.
#[derive(Debug, Clone)]
pub struct Library {
    agents: BTreeMap<String, Agent>,
}

impl Library {
    /// Load every definition, collecting per-file problems instead of
    /// aborting — `brokkr agents list` warns and keeps listing, mirroring
    /// `brokkr recipes list`.
    pub fn scan(root: &Path) -> Result<(Library, Vec<String>), LibraryError> {
        let root = root
            .canonicalize()
            .map_err(|e| LibraryError::Invalid(format!("agent library {}: {e}", root.display())))?;
        let mut agents = BTreeMap::new();
        let mut problems = Vec::new();
        for (name, path) in definition_files(&root, "agent library")? {
            match parse_agent(&root, &name, &path) {
                Ok(agent) => {
                    agents.insert(name, agent);
                }
                Err(LibraryError::Invalid(problem)) => problems.push(problem),
                Err(other) => return Err(other),
            }
        }
        Ok((Library { agents }, problems))
    }

    /// The compiler's load: any problem in any definition is a compile
    /// error, because a bundle pins the whole library it resolved
    /// against.
    pub fn load(root: &Path) -> Result<Library, LibraryError> {
        let (library, problems) = Library::scan(root)?;
        match problems.first() {
            Some(problem) => invalid(problem.clone()),
            None => Ok(library),
        }
    }

    pub fn agent(&self, name: &str) -> Option<&Agent> {
        self.agents.get(name)
    }

    pub fn names(&self) -> Vec<String> {
        self.agents.keys().cloned().collect()
    }

    pub fn agents(&self) -> impl Iterator<Item = &Agent> {
        self.agents.values()
    }
}

fn parse_agent(root: &Path, name: &str, path: &Path) -> Result<Agent, LibraryError> {
    let what = format!("agent '{name}' ({})", path.display());
    if !valid_name(name) {
        return invalid(format!("{what}: the file name must match {NAME_GRAMMAR}"));
    }
    let source = read_json(path)?;
    let map = object(&source, &what)?;
    only_keys(
        map,
        &[
            "description",
            "charter",
            "models",
            "efforts",
            "tools",
            "hands",
            "limits",
            "inputs",
        ],
        &what,
    )?;
    let description = string(map, "description", &what)?;
    let charter_rel = string(map, "charter", &what)?;
    let charter = contained(root, &charter_rel, &what)?;
    let charter_digest = sha256_bytes(&std::fs::read(&charter)?);
    let models = string_array(map, "models", &what)?;
    if models.is_empty() {
        return invalid(format!(
            "{what} 'models' is empty; an agent names at least one model"
        ));
    }
    named(&models, "models", &what)?;
    // A charter names the effort it hires exactly as it names the model
    // (decision 0035 ruling 5), keyed by the candidate it belongs to
    // rather than positionally: a chain reordered in review must not
    // silently re-hire every seat at a different level. Whether a
    // candidate NEEDS one is not knowable here — it depends on the
    // adapter that ends up serving it — so this only refuses an effort
    // named for a candidate that is not in the chain at all.
    let efforts = match map.get("efforts") {
        None => BTreeMap::new(),
        Some(_) => name_map(map, "efforts", &what)?,
    };
    for candidate in efforts.keys() {
        if !models.contains(candidate) {
            return invalid(format!(
                "{what} 'efforts' names an effort for '{candidate}', which is not \
                 in its 'models' chain [{}]",
                models.join(", ")
            ));
        }
    }
    let (allow, mcp) = parse_tools(map, &what)?;
    // Decision 0043: one boxed tool instead of a list. Refused at the
    // same place a malformed tool list is, naming the agent.
    let hands = match map.get("hands") {
        None => None,
        Some(raw) => Some(
            brokkr_protocol::hands::HandsSpec::parse(raw)
                .map_err(|problem| LibraryError::Invalid(format!("{what} 'hands': {problem}")))?,
        ),
    };
    let limits = parse_limits(map, &what)?;
    let inputs = match map.get("inputs") {
        None => None,
        Some(_) => Some(string_array(map, "inputs", &what)?),
    };
    Ok(Agent {
        name: name.to_string(),
        description,
        charter,
        charter_digest,
        models,
        efforts,
        allow,
        mcp,
        hands,
        limits,
        inputs,
        digest: sha256_hex(&source),
        source,
    })
}

/// Resolve a library-relative path and prove it stays inside the root.
/// `parse_role`'s bare `dir.join(rel)` is tolerable inside an operator's
/// own bundle; it is not tolerable for a shared library joined on behalf
/// of every recipe.
fn contained(root: &Path, relative: &str, what: &str) -> Result<PathBuf, LibraryError> {
    let joined = root.join(relative);
    let canonical = joined
        .canonicalize()
        .map_err(|e| LibraryError::Invalid(format!("{what} charter '{relative}': {e}")))?;
    if !canonical.starts_with(root) {
        return invalid(format!(
            "{what} charter '{relative}' resolves to {} , which is outside the \
             library root {}",
            canonical.display(),
            root.display()
        ));
    }
    Ok(canonical)
}

fn parse_tools(
    map: &Map<String, Value>,
    what: &str,
) -> Result<(Option<Vec<String>>, Vec<McpNeed>), LibraryError> {
    let Some(raw) = map.get("tools") else {
        return Ok((None, Vec::new()));
    };
    let tools = object(raw, &format!("{what} 'tools'"))?;
    only_keys(tools, &["allow", "mcp"], &format!("{what} 'tools'"))?;
    let allow = match tools.get("allow") {
        // Absent declares NO restriction. `[]` is rejected as ambiguous
        // between "no restriction" and "restrict to nothing".
        None => None,
        Some(_) => {
            let names = string_array(tools, "allow", &format!("{what} 'tools'"))?;
            if names.is_empty() {
                return invalid(format!(
                    "{what} 'tools.allow' is empty, which is ambiguous between \
                     'no restriction' and 'restrict to nothing'; omit the key to \
                     declare no restriction"
                ));
            }
            named(&names, "tools.allow", what)?;
            Some(names)
        }
    };
    let mut mcp = Vec::new();
    if let Some(entries) = tools.get("mcp") {
        let Some(entries) = entries.as_array() else {
            return invalid(format!("{what} 'tools.mcp' must be an array"));
        };
        for entry in entries {
            let need = object(entry, &format!("{what} 'tools.mcp' entry"))?;
            only_keys(
                need,
                &["server", "optional"],
                &format!("{what} 'tools.mcp' entry"),
            )?;
            let server = string(need, "server", &format!("{what} 'tools.mcp' entry"))?;
            named(std::slice::from_ref(&server), "tools.mcp.server", what)?;
            mcp.push(McpNeed {
                server,
                optional: need
                    .get("optional")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
        }
    }
    Ok((allow, mcp))
}

fn parse_limits(map: &Map<String, Value>, what: &str) -> Result<Option<Limits>, LibraryError> {
    let Some(raw) = map.get("limits") else {
        return Ok(None);
    };
    let raw = object(raw, &format!("{what} 'limits'"))?;
    only_keys(
        raw,
        &["max_attempts", "timeout_seconds"],
        &format!("{what} 'limits'"),
    )?;
    let mut limits = Limits::default();
    for (key, value) in raw {
        let Some(number) = value.as_u64().filter(|n| *n >= 1) else {
            return invalid(format!("{what} 'limits.{key}' must be an integer >= 1"));
        };
        match key.as_str() {
            "max_attempts" => limits.max_attempts = number,
            _ => limits.timeout_seconds = number,
        }
    }
    Ok(Some(limits))
}

// ------------------------------------------------------------ adapters

/// The provider adapters, indexed by the abstract model names they
/// serve. One model name maps to exactly one provider: resolution is
/// then unambiguous by construction, with no provider-preference
/// tiebreak to reason about, and agents stay provider-free.
#[derive(Debug, Clone)]
pub struct Adapters {
    adapters: BTreeMap<String, Adapter>,
    by_model: BTreeMap<String, String>,
    files: Vec<String>,
}

impl Adapters {
    pub fn load(root: &Path) -> Result<Adapters, LibraryError> {
        let root = root
            .canonicalize()
            .map_err(|e| LibraryError::Invalid(format!("adapters {}: {e}", root.display())))?;
        let mut adapters: BTreeMap<String, Adapter> = BTreeMap::new();
        let mut by_model: BTreeMap<String, String> = BTreeMap::new();
        let mut sources: BTreeMap<String, String> = BTreeMap::new();
        let mut files = Vec::new();
        for (name, path) in definition_files(&root, "adapters")? {
            let adapter = parse_adapter(&name, &path)?;
            files.push(path.display().to_string());
            for model in adapter.models.keys() {
                if let Some(other) = by_model.get(model) {
                    return invalid(format!(
                        "abstract model '{model}' is mapped by two adapters, \
                         {} and {}; one model name maps to exactly one provider \
                         so resolution has no tiebreak to guess",
                        sources[other],
                        path.display()
                    ));
                }
                by_model.insert(model.clone(), name.clone());
            }
            sources.insert(name.clone(), path.display().to_string());
            adapters.insert(name, adapter);
        }
        Ok(Adapters {
            adapters,
            by_model,
            files,
        })
    }

    /// The adapter serving `model`, with the concrete provider model id.
    pub fn serving(&self, model: &str) -> Option<(&Adapter, &str)> {
        let provider = self.by_model.get(model)?;
        let adapter = &self.adapters[provider];
        Some((adapter, adapter.models[model].as_str()))
    }

    pub fn digest(&self, provider: &str) -> Option<&str> {
        self.adapters.get(provider).map(|a| a.digest.as_str())
    }

    /// One provider's adapter by name. `None` for a driver no adapter
    /// declares, which decision 0021's refusals read as "declares
    /// nothing": untrusted, ungranted.
    pub fn adapter(&self, provider: &str) -> Option<&Adapter> {
        self.adapters.get(provider)
    }

    /// The adapter files consulted, named in every unmapped-model error
    /// so a reader knows exactly where to add the mapping.
    pub fn files(&self) -> &[String] {
        &self.files
    }

    pub fn providers(&self) -> impl Iterator<Item = &Adapter> {
        self.adapters.values()
    }
}

fn parse_adapter(name: &str, path: &Path) -> Result<Adapter, LibraryError> {
    let what = format!("adapter '{name}' ({})", path.display());
    if !valid_name(name) {
        return invalid(format!("{what}: the file name must match {NAME_GRAMMAR}"));
    }
    let source = read_json(path)?;
    let map = object(&source, &what)?;
    only_keys(
        map,
        &[
            "provider",
            "trust_tier",
            "binding_grant",
            "egress",
            "routes",
            "credentials",
            "binary",
            "hint",
            "driver",
            "models",
            "model_flag",
            "efforts",
            "effort_flag",
            "tool_permissions",
            "mcp",
            "hands",
        ],
        &what,
    )?;
    let provider = string(map, "provider", &what)?;
    if provider != name {
        return invalid(format!(
            "{what} declares provider '{provider}' but the file is named \
             '{name}.json'; the file name is the provider name"
        ));
    }
    let binary = string(map, "binary", &what)?;
    let driver = string_array(map, "driver", &what)?;
    if driver.is_empty() {
        return invalid(format!("{what} 'driver' is empty; it is the invocation"));
    }
    let models = name_map(map, "models", &what)?;
    // Three legal shapes, two outcomes. `"unsupported"` and a declared
    // gap BOTH yield `None` — the refusal in `compose` is identical, so
    // no seat gains power by the reason existing. What the reason buys
    // is that a future reader learns the gap was MEASURED against the
    // provider's CLI, not defaulted by whoever wrote the file first.
    let (tool_permissions, tool_permissions_gap) = match capability(map, "tool_permissions", &what)?
    {
        None => (None, None),
        Some(value) => {
            let raw = object(value, &format!("{what} 'tool_permissions'"))?;
            if raw.contains_key("unsupported") {
                only_keys(raw, &["unsupported"], &format!("{what} 'tool_permissions'"))?;
                (
                    None,
                    Some(string(
                        raw,
                        "unsupported",
                        &format!("{what} 'tool_permissions'"),
                    )?),
                )
            } else {
                only_keys(
                    raw,
                    &["flag", "separator", "names"],
                    &format!("{what} 'tool_permissions'"),
                )?;
                (
                    Some(ToolPermissions {
                        flag: string(raw, "flag", &format!("{what} 'tool_permissions'"))?,
                        separator: string(raw, "separator", &format!("{what} 'tool_permissions'"))?,
                        names: name_map(raw, "names", &format!("{what} 'tool_permissions'"))?,
                    }),
                    None,
                )
            }
        }
    };
    let mcp = match capability(map, "mcp", &what)? {
        None => None,
        Some(value) => {
            let raw = object(value, &format!("{what} 'mcp'"))?;
            only_keys(raw, &["flag", "servers"], &format!("{what} 'mcp'"))?;
            Some(McpSupport {
                flag: string(raw, "flag", &format!("{what} 'mcp'"))?,
                servers: name_map(raw, "servers", &format!("{what} 'mcp'"))?,
            })
        }
    };
    let hint = match map.get("hint") {
        None => None,
        Some(_) => Some(string(map, "hint", &what)?),
    };
    let model_flag = pin_flag(map, "model_flag", &what)?;
    // The effort vocabulary this provider declares (decision 0035
    // ruling 5), read beside the flag that expresses it. Required, and
    // required even when empty, for the reason `model_flag` is: an
    // absent declaration would read as "effortless" and quietly excuse
    // every seat this provider serves from the pin — which is the
    // implicit default ruling 1 exists to refuse.
    let efforts = string_array(map, "efforts", &what)?;
    named(&efforts, "efforts", &what)?;
    // Decision 0043: how the provider puts its hands in the box, or the
    // measured reason it cannot. The same three legal shapes as
    // `tool_permissions`, for the same reason.
    // Absent is legal here, unlike `tool_permissions`: the key arrived
    // with decision 0043, and an adapter written before it — including
    // every `brokkr init` scaffold in the field — must keep compiling.
    // Absent reads as unsupported with no reason, fail-closed.
    let (hands, hands_gap) = match map.get("hands").map(|_| capability(map, "hands", &what)) {
        None | Some(Ok(None)) => (None, None),
        Some(Err(error)) => return Err(error),
        Some(Ok(Some(value))) => {
            let raw = object(value, &format!("{what} 'hands'"))?;
            if raw.contains_key("unsupported") {
                only_keys(raw, &["unsupported"], &format!("{what} 'hands'"))?;
                (
                    None,
                    Some(string(raw, "unsupported", &format!("{what} 'hands'"))?),
                )
            } else {
                only_keys(raw, &["workspace"], &format!("{what} 'hands'"))?;
                (
                    Some(string_array(raw, "workspace", &format!("{what} 'hands'"))?),
                    None,
                )
            }
        }
    };
    Ok(Adapter {
        provider,
        trust_tier: trust_tier(map, &what)?,
        egress: adapter_egress(map, &what)?,
        routes: routes(map, &what)?,
        credentials: credentials(map, &what)?,
        binary,
        hint,
        driver,
        models,
        model_flag,
        efforts,
        effort_flag: pin_flag(map, "effort_flag", &what)?,
        tool_permissions,
        tool_permissions_gap,
        hands,
        hands_gap,
        mcp,
        digest: sha256_bytes(&std::fs::read(path)?),
    })
}
