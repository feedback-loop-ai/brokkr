use super::*;
use serde_json::json;

#[test]
fn thread_error_keeps_an_explicit_null() {
    let mut raw = json!({"type":"event_msg","payload":{"type":"task_complete","error":null}});
    let null_error = thread_error(Row::new(&raw, &[])).unwrap();
    assert!(!null_error.present(""));
    assert!(null_error.has(""));
    raw["payload"].as_object_mut().unwrap().remove("error");
    assert!(thread_error(Row::new(&raw, &[])).is_none());
}

#[test]
fn rollout_refusal_masks_before_clamping() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("bindings");
    let crossing = "crossing-the-clamp";
    let spaced = "private   words";
    crate::secret::store_set(&store, "CUT", crossing).unwrap();
    crate::secret::store_set(&store, "SPACE", spaced).unwrap();
    let bindings =
        crate::secret::resolve_bindings(&store, &["CUT".into(), "SPACE".into()]).unwrap();
    let padding = "x".repeat(130);
    let message = format!("{spaced} {padding} {crossing}");
    let raw = json!({"type":"error","status":429,"message":message});
    let refusal = codex_thread_refusal(&raw.to_string(), None, None, &bindings).unwrap();
    assert_eq!(refusal.class(), RefusalClass::RateLimit);
    assert_eq!(
        refusal.words(),
        format!("rate_limit (HTTP 429): [secret:SPACE] {padding} [secret:CUT]")
    );
    let guard = crate::secret::Guard::new(&bindings);
    assert!(guard.json(serde_json::to_string(&refusal).unwrap().as_bytes()));
}
