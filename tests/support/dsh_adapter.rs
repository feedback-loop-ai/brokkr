//! The measured web declaration shared by synthetic DSH adapters.

use serde_json::Value;

pub(crate) fn with_measured_web(mut adapter: Value) -> Value {
    let shipped: Value = serde_json::from_str(include_str!("../../adapters/dsh.json")).unwrap();
    adapter["native_capabilities"] = shipped["native_capabilities"].clone();
    adapter
}
