use serde_json::json;

use super::*;
use crate::adapters::{
    dsh_launch_with, dsh_served,
    tests::{engine_input, sealed_pair, Seal},
};
use crate::env_guard::EnvGuard;
use crate::native_controls::dsh::fixtures::{plan, OFF_ROW};

#[test]
fn the_staged_dsh_web_row_answers_for_both_tools() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    let workdir = root.to_str().unwrap();
    let mut env = EnvGuard::lock();
    env.set("DSH_HOME", &root);
    let bin = "/nonexistent/dsh";
    for (fetch, search, expected) in [
        (false, false, OFF_ROW),
        (
            true,
            false,
            "- id: tool-web\n  config:\n    fetch: true\n    search: false\n",
        ),
        (
            false,
            true,
            "- id: tool-web\n  config:\n    fetch: false\n    search: true\n",
        ),
        (
            true,
            true,
            "- id: tool-web\n  config:\n    fetch: true\n    search: true\n",
        ),
    ] {
        let input = engine_input(json!({"workdir": workdir}), plan(fetch, search), &[], 0);
        let input = sealed_pair(input, &[], Seal::authored(0));
        let launch = dsh_launch_with(bin, &[], workdir, None, &input, |_| {
            panic!("a cold launch with resume disabled never probes the composite")
        })
        .unwrap();
        let command = dsh_served(bin, &launch, &[], "prompt", workdir, &input).unwrap();
        let expected_patch = dsh_transcript_row(&launch.root).unwrap() + expected;
        assert_eq!(
            std::fs::read_to_string(&command[4]).unwrap(),
            expected_patch
        );
    }
}

#[test]
fn boxed_dsh_keeps_web_disabled() {
    use crate::adapters::dsh_hands::{Mode, Release};
    use crate::native_controls::{compose_launch, managed};

    let sandbox = Selection::Hands {
        rows: String::new(),
        mode: Mode::ReadOnly,
        release: Release::V0_2_1,
        observed: String::new(),
    };
    let web = |fetch| {
        let controls = managed(&json!({"native_controls": plan(fetch, false)}))
            .unwrap()
            .unwrap();
        compose_launch("dsh", &[], &[], &controls)
            .unwrap()
            .1
            .unwrap()
    };
    assert_eq!(web_for(web(false), &sandbox), Ok(None));
    assert_eq!(web_for(web(true), &sandbox), Err(StageRefusal::Boxed));
}
