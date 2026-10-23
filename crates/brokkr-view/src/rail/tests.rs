use super::*;
use serde_json::json;

#[test]
fn the_recorded_order_reads_whole_or_uses_the_legacy_fallback() {
    for payload in [
        json!({}),
        json!({"phases":"verify"}),
        json!({"phases":["verify",1]}),
    ] {
        assert_eq!(recorded_order(&payload), None);
    }
    assert_eq!(
        recorded_order(&json!({"phases":["verify","review"]})),
        Some(vec!["verify".into(), "review".into()])
    );
}
