//! Unbound inputs for refusal and launch tests.

use super::*;

pub(super) fn claude_refusal(event: &Value) -> Option<PreSessionRefusal> {
    super::claude_refusal(&Row::new(event, &[]))
}

impl CodexThreadEcho<'_> {
    pub(super) fn locate_unbound(&mut self, home: &Path, thread_id: &str) {
        self.locate(home, Ident::from_offered(thread_id));
    }
}

impl LaunchHold {
    pub(super) fn confirm_unbound(
        &mut self,
        id: &str,
        emit: &mut impl FnMut(&Emitted),
    ) -> Confirmation {
        self.confirm(&Ident::from_offered(id), id, emit)
    }
}
