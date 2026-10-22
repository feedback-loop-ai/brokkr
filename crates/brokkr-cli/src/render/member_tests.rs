use super::*;

#[test]
fn a_carried_verdict_uses_the_completed_tone() {
    assert!(matches!(tone("carried"), Tone::Good));
    assert_eq!(status_code("carried"), GREEN);
}
