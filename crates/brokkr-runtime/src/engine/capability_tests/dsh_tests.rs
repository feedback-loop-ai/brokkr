use super::*;

pub(super) fn dsh_inventory() -> NativeInventory {
    let adapter = crate::dsh_adapter::with_measured_web(json!({"provider": "dsh"}));
    NativeInventory::parse("adapter 'dsh'", adapter.get("native_capabilities")).unwrap()
}

pub(super) fn assert_web_off(input: &Value) {
    let controls = &input["native_controls"];
    assert_eq!(controls["inventory"], "known");
    assert_eq!(controls["provider"], "dsh");
    assert_eq!(controls["harness"], "dsh");
    assert_eq!(controls["on"], json!([]));
    assert_eq!(controls["off"], json!(["web-fetch", "web-search"]));
    assert_eq!(
        controls["overlay"],
        json!([{"dsh-web-fetch": false}, {"dsh-web-search": false}])
    );
    assert_eq!(input["capabilities"]["held"], json!({}));
    assert_eq!(input["capabilities"].get("native"), None);
}

#[test]
fn an_unmeasured_dsh_inventory_refuses_the_seat() {
    let authority = Authority::nothing("private", Path::new(""));
    let asks = SiteAsks::of("work", None, None).unwrap();
    let native = NativeInventory::Unmeasured("never probed".into());
    let serving = Serving {
        provider: "dsh",
        harness: "dsh",
        model: Some("flash"),
        ..crate::capabilities::tests::serving(&native)
    };
    assert_eq!(
        authority.resolve(&asks, &serving).unwrap_err(),
        "seat 'work' (office 'work') in realm 'private': provider 'dsh' is known to carry native \
         capability 'web-search', which this seat does not hold, and no valid control denies it: \
         its adapter declares its native capabilities unmeasured (never probed). A known native \
         power is launched only with a delivered denial, never on what absence implies; repair \
         the adapter data (decision 0066 ruling 1)"
    );
}
