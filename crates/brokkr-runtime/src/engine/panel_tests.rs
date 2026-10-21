//! The unanimous panel invocation shared by terminal and storage tests.
use super::*;

pub(super) fn unanimous(
    engine: &mut Engine,
    at: PanelAt<'_>,
    members: &[PanelMember],
    input: &Value,
    seconds: u64,
) -> Result<(), EngineError> {
    engine.execute_panel(
        &at,
        members,
        Aggregate::UnanimousPass,
        input,
        std::time::Duration::from_secs(seconds),
        &Selection::new(),
    )
}
