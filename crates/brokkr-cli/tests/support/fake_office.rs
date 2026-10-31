//! The scripted fake driver hired as an office (decision 0065 slice two,
//! U9b1). Past the MCP compile fence, lifted on Linux, SI2 refuses a model
//! serving whose adapter measures no strict shape, an inline opaque
//! command's included, and the fake driver speaks the driver protocol, so
//! the exec driver cannot carry it. A fixture's seat hires this office
//! instead: an adapter whose driver is the fake driver, carrying synthetic
//! plumbing facts measured under its own harness (`synthetic_mcp`), never
//! U0's.

use std::path::Path;

use serde_json::{json, Value};

#[path = "../../../../tests/support/hired_office.rs"]
mod hired_office;
#[path = "../../../../tests/support/synthetic_mcp.rs"]
mod synthetic_mcp;
pub(crate) use synthetic_mcp::measured;

/// The office every fixture seat of [`hire`] hires.
pub(crate) const OFFICE: &str = "fake-office";

/// The fake driver's argv, reading `script` and keeping `state`.
pub(crate) fn fake_driver(script: &Path, state: &Path) -> Value {
    json!([
        env!("CARGO_BIN_EXE_brokkr"),
        "fake-driver",
        "--script",
        script.to_string_lossy(),
        "--state",
        state.to_string_lossy()
    ])
}

/// Writes, under `root`, the adapter `fake` whose driver is the fake
/// driver reading `script` and keeping `state`, and the office
/// [`OFFICE`] hiring its one model.
pub(crate) fn hire(root: &Path, script: &Path, state: &Path) {
    hire_driver(root, [OFFICE, "fake"], fake_driver(script, state));
}

/// Writes, under `root`, the adapter `provider` whose driver is `driver`,
/// with [`measured`] facts, and the office `office` hiring its one model,
/// named as the office is.
pub(crate) fn hire_driver(root: &Path, names: [&str; 2], driver: Value) {
    hire_limited(root, names, driver, None);
}

/// [`hire_driver`], the office holding `limits` where given.
pub(crate) fn hire_limited(
    root: &Path,
    [office, provider]: [&str; 2],
    driver: Value,
    limits: Option<Value>,
) {
    let adapter = json!({
        "provider": provider,
        "binary": driver[0],
        "driver": driver,
        "models": {office: format!("{provider}/{office}")},
        "model_flag": "--model",
        "efforts": ["low", "medium", "high"],
        "effort_flag": "--effort",
        "tool_permissions": "unsupported",
    });
    hired_office::write(root, office, &measured(adapter), limits);
}

/// The adapter an inline seat of [`inline`] dispatches by name.
pub(crate) const INLINE: &str = "inline-fake";

/// An inline seat's driver command running the fake driver reading
/// `script` and keeping `state`, under `root`: dispatched as the operator
/// adapter [`INLINE`] (`<wrapper> driver inline-fake …`), whose wrapper
/// drops those two words and runs the fake driver, so the seat stays
/// inline, references no office, and its adapter carries [`measured`]
/// facts under the harness it dispatches.
pub(crate) fn inline(root: &Path, script: &Path, state: &Path) -> Value {
    use std::os::unix::fs::PermissionsExt;
    let wrapper = root.join(INLINE);
    let body = format!(
        "#!/bin/sh\nshift 2\nexec '{}' fake-driver \"$@\"\n",
        env!("CARGO_BIN_EXE_brokkr")
    );
    std::fs::write(&wrapper, body).unwrap();
    std::fs::set_permissions(&wrapper, std::fs::Permissions::from_mode(0o755)).unwrap();
    let adapter = json!({
        "provider": INLINE,
        "binary": wrapper,
        "driver": [wrapper, "driver", INLINE],
        "models": {INLINE: format!("{INLINE}/scripted")},
        "model_flag": "--model",
        "efforts": [],
        "effort_flag": "unsupported",
        "tool_permissions": "unsupported",
    });
    std::fs::create_dir_all(root.join("adapters")).unwrap();
    let path = root.join(format!("adapters/{INLINE}.json"));
    std::fs::write(path, serde_json::to_vec_pretty(&measured(adapter)).unwrap()).unwrap();
    let fake = fake_driver(script, state);
    let mut command = vec![json!(wrapper), json!("driver"), json!(INLINE)];
    command.extend(fake.as_array().unwrap()[2..].iter().cloned());
    json!({"command": command})
}

/// Every adapter a fixture wrote under `root`'s `adapters/` with no typed
/// `mcp` facts, rewritten with [`measured`] ones so it compiles past the
/// lifted fence; one that does not parse is left for its test to judge.
pub(crate) fn measure_all(root: &Path) {
    for entry in std::fs::read_dir(root.join("adapters")).unwrap() {
        let path = entry.unwrap().path();
        let read = serde_json::from_slice::<Value>(&std::fs::read(&path).unwrap());
        let Ok(adapter) = read.map_err(drop) else {
            continue;
        };
        if !adapter["mcp"].is_object() {
            let bytes = serde_json::to_vec_pretty(&measured(adapter)).unwrap();
            std::fs::write(&path, bytes).unwrap();
        }
    }
}

/// A seat hiring [`OFFICE`], ending in one of `results`.
pub(crate) fn seat(results: &[&str]) -> Value {
    json!({"results": results, "agent": OFFICE})
}

/// A panel member, sequence step or select case hiring [`OFFICE`].
pub(crate) fn body() -> Value {
    json!({"agent": OFFICE})
}
