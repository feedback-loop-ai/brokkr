//! A shipped adapter re-pointed at another driver (decision 0065 slice two,
//! U1d). Each shape of a typed `mcp` declaration was measured under the
//! adapter's own driver harness, and the loader refuses that evidence under
//! any other harness (SI1), so a fixture that re-points the driver drops
//! that evidence.
//!
//! Past the MCP compile fence, lifted on Linux (U9b1), SI2 refuses a model
//! serving with no measured strict shape, an opaque command's included. A
//! re-pointed adapter carries synthetic plumbing facts measured under its
//! new driver's harness instead (`synthetic_mcp`), never U0's; a seat these
//! suites only compile, and that needs no model surface, is driven by the
//! engine's own exec driver ([`exec`]).

use serde_json::{json, Value};

#[path = "../../../../tests/support/synthetic_mcp.rs"]
mod synthetic_mcp;

/// `adapter` run by `driver`, carrying no MCP evidence measured under the
/// harness it replaced, only synthetic facts under the new one's.
pub(crate) fn to(adapter: &mut Value, driver: Value) {
    adapter["driver"] = driver;
    *adapter = synthetic_mcp::measured(adapter.take());
}

/// `adapter` declaring the `harness` hands `fragment`, which no U0 cell
/// measured, with synthetic facts for each shape served under them.
pub(crate) fn harness(adapter: &mut Value, fragment: Value) {
    adapter["hands"]["harness"] = fragment;
    let facts = synthetic_mcp::facts(adapter);
    let harnessed = facts["shapes"].as_array().unwrap().iter();
    let harnessed = harnessed
        .filter(|shape| shape["hands"] == "harness")
        .cloned();
    adapter["mcp"]["shapes"]
        .as_array_mut()
        .unwrap()
        .extend(harnessed);
}

/// A command with no model surface: the exec driver's, which SI2 does
/// not judge (`McpIntent::NoModelSurface`).
pub(crate) fn exec() -> Value {
    json!(["{brokkr}", "driver", "exec", "--", "true"])
}
