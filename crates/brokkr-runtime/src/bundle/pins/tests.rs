//! An inline site's pins, read off its command as the compile reads them:
//! its model and fallback pins, each on the route its driver serves it on,
//! unreadable where either cannot be read as one concrete id, and none for
//! a driver that takes no model; and an agent candidate no adapter maps.

use serde_json::json;

use super::*;

const UNLOADED: CapabilityAdapters<'static> = CapabilityAdapters {
    adapters: None,
    unloaded: None,
};

fn read(command: &[&str]) -> Vec<ModelPins> {
    let raw = json!({"driver": {"command": command}});
    inline(&raw, UNLOADED)
}

#[test]
fn an_inline_site_pins_what_its_built_in_model_driver_is_told() {
    let driver = |kind: &str, pins: &[&str]| {
        let mut command = vec!["{brokkr}", "driver", kind, "--"];
        command.extend(pins);
        read(&command)
    };
    let pinned = |served: &[(&str, Option<&str>)]| {
        let served = (served.iter()).map(|(id, route)| Served {
            id: id.to_string(),
            route: route.map(str::to_string),
        });
        vec![ModelPins::Read(served.collect())]
    };
    assert_eq!(
        driver("dsh", &["--model", "spark-glm/glm"]),
        pinned(&[("spark-glm/glm", Some("spark-glm"))])
    );
    assert_eq!(
        driver("dsh", &["--model", "deepseek-flash"]),
        pinned(&[("deepseek-flash", Some("deepseek-official"))])
    );
    let fallback = ["--model", "opus-5", "--fallback-model", "cloud/sonnet"];
    assert_eq!(
        driver("claude", &fallback),
        pinned(&[("opus-5", None), ("cloud/sonnet", Some("cloud"))])
    );
    assert_eq!(
        driver("claude", &["--model", "opus-5", "--fallback-model", "--x"]),
        vec![ModelPins::Unreadable(vec!["--fallback-model".into()])]
    );
    assert_eq!(driver("exec", &["true", "--model", "x"]), exec());
    assert_eq!(read(&["./run.sh", "--model", "x"]), exec());
}

#[test]
fn an_agent_candidate_no_adapter_maps_is_unreadable_never_no_model() {
    assert_eq!(
        agent(UNLOADED, "dsh", "flash"),
        ModelPins::Unreadable(Vec::new())
    );
}
