//! A shipped adapter re-pointed at another driver (decision 0065 slice two,
//! U1d). Each shape of a typed `mcp` declaration was measured under the
//! adapter's own driver harness, and the loader refuses that evidence under
//! any other harness (SI1), so a fixture that re-points the driver drops it
//! to the bare form, which grants and measures nothing.

use serde_json::{json, Value};

/// `adapter` run by `driver`, carrying no MCP evidence measured under the
/// harness it replaced.
pub(crate) fn to(adapter: &mut Value, driver: Value) {
    adapter["driver"] = driver;
    adapter["mcp"] = json!("unsupported");
}
