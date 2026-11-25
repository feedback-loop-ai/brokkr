//! The unbound Claude input shared by the adapter and refusal suites.

use super::*;

pub(super) fn claude_refusal(event: &Value) -> Option<PreSessionRefusal> {
    super::claude_refusal(&Row::new(event, &[]))
}
