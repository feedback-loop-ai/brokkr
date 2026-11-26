use super::*;

const ADAPTER: &str = "adapter 'test-web'";
const KEY: &str = "fetch";

fn fetch_inventory(on: launch::Overlay, off: launch::Overlay) -> NativeInventory {
    NativeInventory::parse(
        ADAPTER,
        Some(&json!({
            "known": {KEY: {
                "capability": "web-fetch",
                "tools": ["web_fetch"],
                "on": {"overlay": on},
                "off": {"overlay": off},
                "restrictions": {"unsupported": "whole tool switch"},
                "evidence": {"source": "fixture", "scope": "fixture", "limitations": []}
            }}
        })),
    )
    .unwrap()
}

fn assert_mismatch(native: NativeInventory, harness: &str, half: &str) {
    let expected = format!(
        "{ADAPTER} 'native_capabilities' key '{KEY}' {half} overlay does not match the DSH \
         capability and state it declares"
    );
    assert_eq!(native.check_declared(ADAPTER, harness), Err(expected));
}

#[test]
fn a_dsh_overlay_under_another_harness_is_refused() {
    use launch::Overlay::DshWebFetch;
    let native = fetch_inventory(DshWebFetch(true), DshWebFetch(false));
    assert_mismatch(native, "claude", "ON");
}

#[test]
fn a_dsh_overlay_switching_another_capability_is_refused() {
    use launch::Overlay::{DshWebFetch, DshWebSearch};
    let native = fetch_inventory(DshWebSearch(true), DshWebFetch(false));
    assert_mismatch(native, "dsh", "ON");
}

#[test]
fn a_dsh_overlay_with_the_other_halfs_polarity_is_refused() {
    use launch::Overlay::DshWebFetch;
    let native = fetch_inventory(DshWebFetch(true), DshWebFetch(true));
    assert_mismatch(native, "dsh", "OFF");
}
