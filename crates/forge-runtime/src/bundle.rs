//! Declarative bundles: policy + one seat per phase (decision 0005).
//! `compile` validates before anything runs and produces the pinned
//! content-addressed manifest. Rejections here are the executable slice
//! of the constitutional lint: a bundle that could reach ship around the
//! protected review phase, name a result no rule covers, or reference a
//! missing role never loads at all.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use forge_core::canonical::sha256_bytes;
use forge_core::policy::Machine;
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

#[derive(Debug, Clone)]
pub struct Seat {
    pub role_path: PathBuf,
    pub results: Vec<String>,
    pub command: Vec<String>,
    pub limits: Limits,
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
            let role_rel = raw
                .get("role")
                .and_then(Value::as_str)
                .ok_or_else(|| CompileError::Invalid(format!("seat '{phase}' missing 'role'")))?;
            let role_path = dir.join(role_rel);
            if !role_path.is_file() {
                return Err(CompileError::Invalid(format!(
                    "seat '{phase}' role file '{role_rel}' does not exist"
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
            let command = raw
                .get("driver")
                .and_then(|d| d.get("command"))
                .and_then(Value::as_array)
                .map(|a| {
                    a.iter()
                        .filter_map(Value::as_str)
                        .map(|part| {
                            // "./"-prefixed entries are bundle-relative.
                            match part.strip_prefix("./") {
                                Some(rel) => dir.join(rel).to_string_lossy().into_owned(),
                                None => part.to_string(),
                            }
                        })
                        .collect::<Vec<_>>()
                })
                .filter(|c| !c.is_empty())
                .ok_or_else(|| {
                    CompileError::Invalid(format!("seat '{phase}' needs driver.command"))
                })?;
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
            seats.insert(
                phase.clone(),
                Seat {
                    role_path,
                    results,
                    command,
                    limits,
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
