use super::*;

pub(super) fn refuses_unmeasured_inventory(
    operator: &Operator,
    context: &CapabilityContext,
    refusal: impl FnOnce(&str, &str, &str, &str) -> String,
) {
    one_inline_seat(
        operator,
        &[
            "{brokkr}",
            "driver",
            "dsh",
            "--",
            "--model",
            "deepseek-v4-flash",
            "--effort",
            "high",
        ],
    );
    let adapters = copied_adapters();
    edit_adapter(adapters.path(), "dsh", |adapter| {
        adapter["native_capabilities"] = json!({"unmeasured": "never probed"});
    });
    assert_eq!(
        solo(operator, adapters.path(), context),
        refusal(
            &context.realm,
            "dsh",
            "web-search",
            "its adapter declares its native capabilities unmeasured (never probed)"
        )
    );
}
