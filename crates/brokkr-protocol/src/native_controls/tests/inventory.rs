use super::*;

pub(super) fn unmeasured_lanetally() -> Sealed {
    let mut sealed = claude_local();
    sealed.harness = "lanetally";
    sealed.controls = Controls {
        provider: "lanetally".into(),
        harness: "lanetally".into(),
        inventory: Inventory::Unmeasured("not measured".into()),
        held: Vec::new(),
        denied: Vec::new(),
        admits: admits(&[]),
        selection: Selection {
            flags: claude_flags(),
            ..Selection::default()
        },
        guards: Vec::new(),
        ..sealed.controls
    };
    sealed.expected.identity = Identity {
        provider: "lanetally".into(),
        harness: "lanetally".into(),
        model: None,
    };
    sealed.expected.native = NativeExpectation::Unmeasured("not measured".into());
    sealed
}

#[test]
fn sealed_unmeasured_inventory_refuses_another_reason() {
    let mut sealed = unmeasured_lanetally();
    let command = sealed.served(&CLAUDE_LEAD);
    sealed.expected.native = NativeExpectation::Unmeasured("another assessment".into());
    assert_eq!(
        sealed.cold(command),
        Err(final_refusal(
            "lanetally",
            "was planned under another native inventory than its sealed record expects"
        ))
    );
}
