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

use forge_core::canonical::{sha256_bytes, sha256_hex};
use serde_json::{Map, Value};
use thiserror::Error;

use super::{valid_name, Adapter, Agent, McpNeed, McpSupport, ToolPermissions, NAME_GRAMMAR};
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

/// A provider's model flag: a flag string, or `"unsupported"`.
fn model_flag(map: &Map<String, Value>, what: &str) -> Result<Option<String>, LibraryError> {
    match map.get("model_flag").and_then(Value::as_str) {
        Some("unsupported") => Ok(None),
        Some(flag) if !flag.is_empty() => Ok(Some(flag.to_string())),
        _ => invalid(format!(
            "{what} needs 'model_flag': the flag this provider takes, or the \
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
    /// aborting — `forge agents list` warns and keeps listing, mirroring
    /// `forge recipes list`.
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
            "tools",
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
    let (allow, mcp) = parse_tools(map, &what)?;
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
        allow,
        mcp,
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
            "binary",
            "hint",
            "driver",
            "models",
            "model_flag",
            "tool_permissions",
            "mcp",
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
    let tool_permissions = match capability(map, "tool_permissions", &what)? {
        None => None,
        Some(value) => {
            let raw = object(value, &format!("{what} 'tool_permissions'"))?;
            only_keys(
                raw,
                &["flag", "separator", "names"],
                &format!("{what} 'tool_permissions'"),
            )?;
            Some(ToolPermissions {
                flag: string(raw, "flag", &format!("{what} 'tool_permissions'"))?,
                separator: string(raw, "separator", &format!("{what} 'tool_permissions'"))?,
                names: name_map(raw, "names", &format!("{what} 'tool_permissions'"))?,
            })
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
    Ok(Adapter {
        provider,
        binary,
        hint,
        driver,
        models,
        model_flag: model_flag(map, &what)?,
        tool_permissions,
        mcp,
        digest: sha256_bytes(&std::fs::read(path)?),
    })
}
