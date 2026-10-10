//! An inline site's pins, read off its command as the compile reads them:
//! its model and fallback pins, unreadable where either cannot be read as
//! one concrete id, and none for a driver that takes no model.

use serde_json::json;

use super::*;

fn read(command: &[&str]) -> Vec<ModelPins> {
    let raw = json!({"driver": {"command": command}});
    let adapters = CapabilityAdapters {
        adapters: None,
        unloaded: None,
    };
    inline(&raw, adapters)
}

#[test]
fn an_inline_site_pins_what_its_built_in_model_driver_is_told() {
    let driver = |kind: &str, pins: &[&str]| {
        let mut command = vec!["{brokkr}", "driver", kind, "--"];
        command.extend(pins);
        read(&command)
    };
    let pinned = |ids: &[&str]| {
        vec![ModelPins::Read(
            ids.iter().map(|id| id.to_string()).collect(),
        )]
    };
    assert_eq!(
        driver("dsh", &["--model", "spark-glm/glm"]),
        pinned(&["spark-glm/glm"])
    );
    let fallback = ["--model", "opus-5", "--fallback-model", "cloud/sonnet"];
    assert_eq!(
        driver("claude", &fallback),
        pinned(&["opus-5", "cloud/sonnet"])
    );
    assert_eq!(
        driver("claude", &["--model", "opus-5", "--fallback-model", "--x"]),
        vec![ModelPins::Unreadable(vec!["--fallback-model".into()])]
    );
    assert_eq!(driver("exec", &["true", "--model", "x"]), exec());
    assert_eq!(read(&["./run.sh", "--model", "x"]), exec());
}
