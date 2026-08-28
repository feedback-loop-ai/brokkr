use super::*;
use serde_json::json;

fn wire(body: Body) -> String {
    serde_json::to_string(&Message::new(body)).unwrap()
}

fn start(seat: &str) -> Body {
    Body::Start {
        effect_id: "effect".into(),
        attempt_id: "attempt".into(),
        seat: seat.into(),
        input: json!({}),
    }
}

fn session(script: &Value, input: &str, hang: impl FnMut()) -> String {
    let dir = tempfile::tempdir().unwrap();
    let mut output = Vec::new();
    run_fake_session(script, dir.path(), input.as_bytes(), &mut output, hang).unwrap();
    String::from_utf8(output).unwrap()
}

#[test]
fn fake_session_covers_noise_handshake_cancel_shutdown_and_eof() {
    let hello = wire(Body::Hello {
        engine_version: "test".into(),
    });
    let cancel = wire(Body::Cancel {
        effect_id: "effect".into(),
    });
    let output = session(
        &json!({}),
        &format!("\nnot-json\n{hello}\n{cancel}\n"),
        || {},
    );
    assert!(output.contains("capabilities"));
    assert!(output.contains("cancelled"));

    let shutdown = wire(Body::Shutdown);
    assert!(session(&json!({}), &shutdown, || {}).is_empty());
    assert!(session(&json!({}), "", || {}).is_empty());
    assert!(session(&json!({}), "{\"type\":\"unknown\"}\n", || {}).is_empty());
    let ignored = wire(Body::Capabilities {
        driver: "peer".into(),
        version: "1".into(),
        supports: vec![],
    });
    assert!(session(&json!({}), &ignored, || {}).is_empty());
}

#[test]
fn fake_session_executes_every_scripted_behavior_without_real_sleep() {
    let script = json!({"seats": {
        "success": [{"behavior": "succeed", "result": {"result": "complete"}}],
        "failure": [{"behavior": "fail"}],
        "garbage": [{"behavior": "garbage"}],
        "hang": [{"behavior": "hang"}],
        "vanish": [{"behavior": "vanish"}],
    }});
    let mut hung = false;
    for (seat, expected) in [
        ("success", "\"status\":\"succeeded\""),
        ("failure", "scripted failure"),
        ("garbage", "this is not a protocol message"),
        ("hang", "\"type\":\"accepted\""),
        ("vanish", "\"type\":\"accepted\""),
        ("missing", "\"type\":\"accepted\""),
    ] {
        let input = wire(start(seat));
        let output = session(&script, &input, || hung = true);
        assert!(output.contains(expected), "{seat}: {output}");
    }
    assert!(hung);
}

#[test]
fn attempt_file_name_is_portable_and_collision_resistant() {
    let first = attempt_file_name("review:security");
    let second = attempt_file_name("review/security");
    assert!(!first.contains(':'));
    assert!(!first.contains('/'));
    assert_ne!(first, second);
}
