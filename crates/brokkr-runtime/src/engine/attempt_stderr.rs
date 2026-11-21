//! Name failed and indeterminate attempts' stderr at the journal edge,
//! and tell their masked tails only after the turn's appends (#1199).

use brokkr_core::envelope::EventType;
use brokkr_protocol::process::DriverStderr;
use serde_json::Value;

use super::{Engine, EngineError};

/// One driver is settled at a time. Its retained stderr is either still
/// with the active settlement, carried by a held outcome, or ready to tell.
#[derive(Default)]
pub(super) struct AttemptStderr {
    pub(super) active: Option<String>,
    ready: Vec<(String, String)>,
}

impl AttemptStderr {
    pub(super) fn name(&self, event: EventType, payload: &mut Value) {
        let Some(stderr) = &self.active else { return };
        let field = match event {
            EventType::EffectFailed => "error",
            EventType::EffectIndeterminate => "reason",
            _ => return,
        };
        let reason = payload[field]
            .as_str()
            .expect("terminal payload has reason text");
        payload[field] = DriverStderr::of(stderr).named(reason).into();
    }

    pub(super) fn landed(&mut self, event: EventType, attempt_id: Option<&str>) {
        if let (EventType::EffectFailed | EventType::EffectIndeterminate, Some(id), Some(stderr)) =
            (event, attempt_id, self.active.take())
        {
            if !stderr.is_empty() {
                self.ready.push((id.to_owned(), stderr));
            }
        }
    }

    pub(super) fn tell(&mut self, writer: &mut impl std::io::Write) {
        for (attempt_id, stderr) in self.ready.drain(..) {
            DriverStderr::of(&stderr).tell(&attempt_id, writer);
        }
    }
}

impl Engine {
    pub(super) fn finish_turn(&mut self) -> Result<(), EngineError> {
        if !self.stderr.ready.is_empty() {
            self.park_after_terminal()?;
            self.stderr.tell(&mut self.told);
        }
        Ok(())
    }
}
