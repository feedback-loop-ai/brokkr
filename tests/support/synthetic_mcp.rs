//! Synthetic MCP facts for the scripted and re-driven adapters the suites
//! write (decision 0065 slice two, U9b1). Past the MCP compile fence,
//! lifted on Linux, SI2 refuses a model serving whose adapter measures no
//! strict shape, an opaque command's included. These facts are fixture
//! plumbing that lets such an adapter compile, never U0's qualification of
//! a harness, which only the shipped declarations record.

use serde_json::{json, Value};

/// The facts `adapter` carries so it compiles past the fence: measured
/// under the harness its own driver dispatches (`<custom>` for an opaque
/// command) on its own binary, as the loader admits only (SI1), for each
/// invocation a seat is served as, under each hands it declares, its
/// ambient exclusion measured and every other axis unprobed.
pub(crate) fn facts(adapter: &Value) -> Value {
    let driver = adapter["driver"].as_array().expect("a driver argv");
    let harness = match (driver.get(1).and_then(Value::as_str), driver.get(2)) {
        (Some("driver"), Some(kind)) => kind.clone(),
        _ => json!("<custom>"),
    };
    let host = match cfg!(target_os = "macos") {
        true => "macos",
        false => "linux",
    };
    let declared = |key: &str| adapter["hands"].get(key).is_some();
    let hands = [
        ("none", true),
        ("boxed", declared("workspace")),
        ("harness", declared("harness")),
    ];
    let unprobed = json!({"unmeasured": "synthetic fixture: not probed"});
    let measured = json!({"measured": "synthetic fixture plumbing, not U0"});
    let on = json!({"harness": harness, "binary": adapter["binary"], "version": "synthetic", "host": host});
    let shapes: Vec<Value> = ["cold", "replacement"]
        .into_iter()
        .flat_map(|invocation| hands.map(|(hands, held)| held.then_some((invocation, hands))))
        .flatten()
        .map(|(invocation, hands)| {
            json!({"invocation": invocation, "hands": hands, "measured_on": on,
                   "ambient": measured, "native_write": unprobed, "store_read": unprobed,
                   "process_read": unprobed})
        })
        .collect();
    json!({"carriage": measured, "shapes": shapes})
}

/// `adapter` carrying [`facts`].
pub(crate) fn measured(mut adapter: Value) -> Value {
    adapter["mcp"] = facts(&adapter);
    adapter
}
