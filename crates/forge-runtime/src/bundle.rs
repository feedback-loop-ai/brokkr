//! Declarative bundles: policy + one seat per phase (decision 0005).
//! `compile` validates before anything runs and produces the pinned
//! content-addressed manifest. Rejections here are the executable slice
//! of the constitutional lint: a bundle that could reach ship around the
//! protected review phase, name a result no rule covers, or reference a
//! missing role never loads at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use forge_core::canonical::sha256_bytes;
use forge_core::policy::{Machine, BOOLEAN_INPUTS, SEVERITY_INPUTS};
use serde_json::{json, Map, Value};
use thiserror::Error;

pub const ENGINE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const EVENT_SCHEMA: u32 = 1;
pub const DRIVER_PROTOCOL: u32 = 1;

#[derive(Debug, Error)]
pub enum CompileError {
    #[error("bundle: {0}")]
    Invalid(String),
    #[error("bundle io: {0}")]
    Io(#[from] std::io::Error),
    #[error("bundle json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("bundle policy: {0}")]
    Policy(#[from] forge_core::PolicyError),
}

/// Inputs the engine owns. A seat may never supply or declare these:
/// journal-computed truth is never accepted from a caller (README law 2).
pub const ENGINE_OWNED_INPUTS: [&str; 4] = [
    "consecutive_failures",
    "drift_detected",
    "dirty_worktrees",
    "reviewed_heads",
];

#[derive(Debug, Clone)]
pub struct Seat {
    pub results: Vec<String>,
    pub limits: Limits,
    /// Typed facts this seat may supply (decision 0007). Anything else a
    /// seat sends is dropped before evaluation and never enters the
    /// journal record. Defaults to the non-engine-owned inputs the
    /// phase's own rules reference.
    pub inputs: Vec<String>,
    pub body: SeatBody,
}

/// Optional container confinement for a driver (the policy-confined
/// trust class): the command runs inside a pinned image with only the
/// declared mounts, network off unless granted. Absence = trusted
/// native process. Data, like everything else about a seat.
#[derive(Debug, Clone)]
pub struct Confine {
    pub image: String,
    pub network: bool,
    /// Extra read-only mounts beyond the workdir and bundle dir.
    pub mounts: Vec<String>,
}

/// One agent session, or a parallel panel joined by a declared
/// deterministic rule (decision 0002's two sanctioned forms: this is
/// concurrency INSIDE the executor — one effect, one typed result at
/// the boundary; members are journaled as checkpoint evidence).
#[derive(Debug, Clone)]
pub enum SeatBody {
    Single {
        role_path: PathBuf,
        command: Vec<String>,
        confine: Option<Confine>,
    },
    Panel {
        members: Vec<PanelMember>,
        aggregate: Aggregate,
    },
}

#[derive(Debug, Clone)]
pub struct PanelMember {
    pub name: String,
    pub role_path: PathBuf,
    pub command: Vec<String>,
    pub confine: Option<Confine>,
}

/// Deterministic, order-independent aggregation rules — a closed
/// vocabulary, like conditions: named in data, implemented in the
/// engine, never arbitrary code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Aggregate {
    /// "pass" only when every member reports "pass"; otherwise "fail".
    UnanimousPass,
    /// Worst-member-wins over clean < residual < security-hold; severity
    /// is the max, security and fixes flags are OR-ed.
    ReviewPanel,
}

impl Aggregate {
    fn parse(name: &str) -> Option<Aggregate> {
        match name {
            "unanimous-pass" => Some(Aggregate::UnanimousPass),
            "review-panel" => Some(Aggregate::ReviewPanel),
            _ => None,
        }
    }
    fn required_results(&self) -> &'static [&'static str] {
        match self {
            Aggregate::UnanimousPass => &["pass", "fail"],
            Aggregate::ReviewPanel => &["clean", "residual", "security-hold"],
        }
    }
}

/// Per-seat autonomy limits (decision 0006). Defaults keep the old
/// behavior: one attempt, one-hour deadline.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_attempts: u64,
    pub timeout_seconds: u64,
}

impl Default for Limits {
    fn default() -> Limits {
        Limits {
            max_attempts: 1,
            timeout_seconds: 3600,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Bundle {
    pub name: String,
    pub dir: PathBuf,
    pub machine: Machine,
    pub seats: BTreeMap<String, Seat>,
    pub manifest: Value,
    /// The phase every path to a non-stop terminal must traverse.
    pub protected_phase: String,
}

impl Bundle {
    pub fn compile(dir: &Path) -> Result<Bundle, CompileError> {
        let dir = dir
            .canonicalize()
            .map_err(|e| CompileError::Invalid(format!("bundle dir {}: {e}", dir.display())))?;
        let config: Value =
            serde_json::from_str(&std::fs::read_to_string(dir.join("bundle.json"))?)?;
        let name = config
            .get("name")
            .and_then(Value::as_str)
            .ok_or_else(|| CompileError::Invalid("bundle.json missing 'name'".into()))?
            .to_string();
        let policy_rel = config
            .get("policy")
            .and_then(Value::as_str)
            .ok_or_else(|| CompileError::Invalid("bundle.json missing 'policy'".into()))?;
        let table: Value = serde_json::from_str(&std::fs::read_to_string(dir.join(policy_rel))?)?;
        let machine = Machine::from_table(&table)?;

        let protected_phase = config
            .get("protected_phase")
            .and_then(Value::as_str)
            .unwrap_or("review")
            .to_string();
        if !machine.phases.contains(&protected_phase) {
            return Err(CompileError::Invalid(format!(
                "policy has no '{protected_phase}' phase; the protected review \
                 gate is non-removable (extension model, layer 1)"
            )));
        }
        assert_phase_unavoidable(&machine, &table, &protected_phase)?;

        let raw_seats = config
            .get("seats")
            .and_then(Value::as_object)
            .ok_or_else(|| CompileError::Invalid("bundle.json missing 'seats'".into()))?;
        let mut seats = BTreeMap::new();
        for (phase, raw) in raw_seats {
            if !machine.phases.contains(phase) {
                return Err(CompileError::Invalid(format!(
                    "seat '{phase}' names a phase the policy does not have"
                )));
            }
            let results = raw
                .get("results")
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>()
                })
                .filter(|r| !r.is_empty())
                .ok_or_else(|| {
                    CompileError::Invalid(format!("seat '{phase}' needs non-empty 'results'"))
                })?;
            for result in &results {
                let covered = machine
                    .rules
                    .iter()
                    .any(|rule| rule.from == *phase && rule.result == *result);
                if !covered {
                    return Err(CompileError::Invalid(format!(
                        "seat '{phase}' may emit '{result}' but no rule covers it; \
                         result variants without an outer rule are rejected"
                    )));
                }
            }
            let body = if let Some(panel) = raw.get("panel") {
                let members_raw = panel.as_object().ok_or_else(|| {
                    CompileError::Invalid(format!("seat '{phase}' panel must be an object"))
                })?;
                if members_raw.len() < 2 {
                    return Err(CompileError::Invalid(format!(
                        "seat '{phase}' panel needs at least two members; \
                         a one-member panel is a single seat"
                    )));
                }
                let aggregate_name = raw
                    .get("aggregate")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        CompileError::Invalid(format!("seat '{phase}' panel needs 'aggregate'"))
                    })?;
                let aggregate = Aggregate::parse(aggregate_name).ok_or_else(|| {
                    CompileError::Invalid(format!(
                        "seat '{phase}' unknown aggregate '{aggregate_name}'; known: \
                         unanimous-pass, review-panel"
                    ))
                })?;
                for required in aggregate.required_results() {
                    if !results.iter().any(|r| r == required) {
                        return Err(CompileError::Invalid(format!(
                            "seat '{phase}' aggregate '{aggregate_name}' can emit \
                             '{required}' but the seat does not declare it"
                        )));
                    }
                }
                let mut members = Vec::with_capacity(members_raw.len());
                for (name, member_raw) in members_raw {
                    let role_path = parse_role(&dir, &format!("{phase}:{name}"), member_raw)?;
                    let command = parse_command(&dir, &format!("{phase}:{name}"), member_raw)?;
                    members.push(PanelMember {
                        name: name.clone(),
                        role_path,
                        command,
                        confine: parse_confine(&format!("{phase}:{name}"), member_raw)?,
                    });
                }
                SeatBody::Panel { members, aggregate }
            } else {
                SeatBody::Single {
                    role_path: parse_role(&dir, phase, raw)?,
                    command: parse_command(&dir, phase, raw)?,
                    confine: parse_confine(phase, raw)?,
                }
            };
            let limits = match raw.get("limits") {
                None => Limits::default(),
                Some(raw_limits) => {
                    let object = raw_limits.as_object().ok_or_else(|| {
                        CompileError::Invalid(format!("seat '{phase}' limits must be an object"))
                    })?;
                    let mut limits = Limits::default();
                    for (key, value) in object {
                        let number = value.as_u64().filter(|n| *n >= 1).ok_or_else(|| {
                            CompileError::Invalid(format!(
                                "seat '{phase}' limits.{key} must be an integer >= 1"
                            ))
                        })?;
                        match key.as_str() {
                            "max_attempts" => limits.max_attempts = number,
                            "timeout_seconds" => limits.timeout_seconds = number,
                            other => {
                                return Err(CompileError::Invalid(format!(
                                    "seat '{phase}' has unknown limit '{other}'"
                                )))
                            }
                        }
                    }
                    limits
                }
            };
            // Input provenance (decision 0007): every input the phase's
            // rules reference must be engine-computed or supplied by
            // this seat's declaration; a declaration may only name known,
            // non-engine-owned evaluator inputs.
            let referenced = referenced_seat_inputs(&table, phase);
            let inputs = match raw.get("inputs") {
                None => referenced.clone(),
                Some(declared) => {
                    let declared = declared
                        .as_array()
                        .map(|a| {
                            a.iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .ok_or_else(|| {
                            CompileError::Invalid(format!(
                                "seat '{phase}' inputs must be an array of strings"
                            ))
                        })?;
                    for name in &declared {
                        if ENGINE_OWNED_INPUTS.contains(&name.as_str()) {
                            return Err(CompileError::Invalid(format!(
                                "seat '{phase}' declares engine-owned input '{name}'; \
                                 journal-computed truth is never accepted from a seat"
                            )));
                        }
                        if !declarable_input(name) {
                            return Err(CompileError::Invalid(format!(
                                "seat '{phase}' declares unknown input '{name}'; known: \
                                 the evaluator's closed vocabulary minus engine-owned"
                            )));
                        }
                    }
                    for needed in &referenced {
                        if !declared.contains(needed) {
                            return Err(CompileError::Invalid(format!(
                                "phase '{phase}' rules reference input '{needed}' but \
                                 seat '{phase}' does not declare it; the rule could \
                                 never fire from seat data"
                            )));
                        }
                    }
                    declared
                }
            };
            seats.insert(
                phase.clone(),
                Seat {
                    results,
                    limits,
                    inputs,
                    body,
                },
            );
        }

        for phase in &machine.phases {
            if machine.terminal.contains(phase) {
                continue;
            }
            if !seats.contains_key(phase) {
                return Err(CompileError::Invalid(format!(
                    "non-terminal phase '{phase}' has no seat (no executor can run it)"
                )));
            }
        }

        let manifest = manifest_for(&dir, &name)?;
        Ok(Bundle {
            name,
            dir,
            machine,
            seats,
            manifest,
            protected_phase,
        })
    }

    pub fn manifest_digest(&self) -> String {
        forge_core::canonical::sha256_hex(&self.manifest)
    }
}

/// Removing the protected phase must disconnect every non-stop terminal
/// from the initial phase — no table path ships around review.
fn assert_phase_unavoidable(
    machine: &Machine,
    table: &Value,
    protected: &str,
) -> Result<(), CompileError> {
    let rules = table["rules"].as_array().expect("validated by Machine::from_table");
    let mut reachable = vec![machine.initial.clone()];
    let mut frontier = vec![machine.initial.clone()];
    while let Some(node) = frontier.pop() {
        for rule in rules {
            let from = rule["from"].as_str().unwrap_or_default();
            let next = rule["next"].as_str().unwrap_or_default();
            if from == node && from != protected && next != protected {
                let next = next.to_string();
                if !reachable.contains(&next) {
                    reachable.push(next.clone());
                    frontier.push(next);
                }
            }
        }
    }
    for terminal in &machine.terminal {
        if terminal != "stop" && reachable.contains(terminal) {
            return Err(CompileError::Invalid(format!(
                "policy reaches terminal '{terminal}' without passing '{protected}'; \
                 a path to shipping that bypasses the protected review gate is \
                 constitutionally rejected"
            )));
        }
    }
    Ok(())
}

fn parse_role(dir: &Path, what: &str, raw: &Value) -> Result<PathBuf, CompileError> {
    let role_rel = raw
        .get("role")
        .and_then(Value::as_str)
        .ok_or_else(|| CompileError::Invalid(format!("seat '{what}' missing 'role'")))?;
    let role_path = dir.join(role_rel);
    if !role_path.is_file() {
        return Err(CompileError::Invalid(format!(
            "seat '{what}' role file '{role_rel}' does not exist"
        )));
    }
    Ok(role_path)
}

fn parse_command(dir: &Path, what: &str, raw: &Value) -> Result<Vec<String>, CompileError> {
    raw.get("driver")
        .and_then(|d| d.get("command"))
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(|part| {
                    // "{forge}" is this engine's own executable (built-in
                    // drivers); "./"-prefixed entries are bundle-relative.
                    if part == "{forge}" {
                        return std::env::current_exe()
                            .map(|p| p.to_string_lossy().into_owned())
                            .unwrap_or_else(|_| "forge".to_string());
                    }
                    match part.strip_prefix("./") {
                        Some(rel) => dir.join(rel).to_string_lossy().into_owned(),
                        None => part.to_string(),
                    }
                })
                .collect::<Vec<_>>()
        })
        .filter(|c: &Vec<String>| !c.is_empty())
        .ok_or_else(|| CompileError::Invalid(format!("seat '{what}' needs driver.command")))
}

fn parse_confine(what: &str, raw: &Value) -> Result<Option<Confine>, CompileError> {
    let Some(raw_confine) = raw.get("driver").and_then(|d| d.get("confine")) else {
        return Ok(None);
    };
    let object = raw_confine.as_object().ok_or_else(|| {
        CompileError::Invalid(format!("seat '{what}' driver.confine must be an object"))
    })?;
    let image = object
        .get("image")
        .and_then(Value::as_str)
        .filter(|i| !i.is_empty())
        .ok_or_else(|| {
            CompileError::Invalid(format!("seat '{what}' confine needs a non-empty image"))
        })?
        .to_string();
    let network = object.get("network").and_then(Value::as_bool).unwrap_or(false);
    let mounts = object
        .get("mounts")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
        .unwrap_or_default();
    for key in object.keys() {
        if !["image", "network", "mounts"].contains(&key.as_str()) {
            return Err(CompileError::Invalid(format!(
                "seat '{what}' confine has unknown key '{key}'"
            )));
        }
    }
    Ok(Some(Confine { image, network, mounts }))
}

fn declarable_input(name: &str) -> bool {
    !ENGINE_OWNED_INPUTS.contains(&name)
        && (BOOLEAN_INPUTS.contains(&name) || SEVERITY_INPUTS.contains(&name))
}

/// The non-engine-owned inputs the phase's rules reference: the default
/// (and minimum) seat declaration.
fn referenced_seat_inputs(table: &Value, phase: &str) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    let Some(rules) = table.get("rules").and_then(Value::as_array) else {
        return names;
    };
    for rule in rules {
        if rule.get("from").and_then(Value::as_str) != Some(phase) {
            continue;
        }
        let Some(when) = rule.get("when").and_then(Value::as_object) else {
            continue;
        };
        for key in when.keys() {
            let name = key
                .strip_suffix("_gte")
                .or_else(|| key.strip_suffix("_above"))
                .unwrap_or(key)
                .to_string();
            if !ENGINE_OWNED_INPUTS.contains(&name.as_str()) && !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names.sort();
    names
}

fn manifest_for(dir: &Path, bundle_name: &str) -> Result<Value, CompileError> {
    let mut files = Map::new();
    let mut stack = vec![dir.to_path_buf()];
    let mut paths = Vec::new();
    while let Some(current) = stack.pop() {
        for entry in std::fs::read_dir(&current)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                paths.push(path);
            }
        }
    }
    paths.sort();
    for path in paths {
        let rel = path
            .strip_prefix(dir)
            .expect("walked under dir")
            .to_string_lossy()
            .into_owned();
        files.insert(rel, Value::String(sha256_bytes(&std::fs::read(&path)?)));
    }
    Ok(json!({
        "engine": ENGINE_VERSION,
        "event_schema": EVENT_SCHEMA,
        "database_schema": forge_store::DATABASE_SCHEMA,
        "driver_protocol": DRIVER_PROTOCOL,
        "bundle_name": bundle_name,
        "files": Value::Object(files),
    }))
}
