use serde_json::{json, Value};

pub(super) fn adapter(tier: &str, contracted: bool) -> Value {
    let mut dsh = super::adapter("dsh", Some(tier), Some(contracted));
    dsh["model_flag"] = json!("--model");
    dsh["efforts"] = json!(["low", "medium", "high", "xhigh"]);
    dsh["effort_flag"] = json!("--effort");
    crate::dsh_adapter::with_measured_web(dsh)
}
