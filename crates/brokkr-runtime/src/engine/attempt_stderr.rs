//! Name failed and indeterminate attempts' stderr at the journal edge,
//! and tell their masked tails after the terminal event and any park due
//! (#1199). A drive that returns with a tail queued tells it after
//! releasing the run (#1248).
//! An attempt whose non-terminal append contends keeps its stderr for
//! the indeterminate the lawful end settles it with.

use brokkr_core::envelope::EventType;
use brokkr_protocol::process::DriverStderr;
use serde_json::Value;

use super::{Engine, EngineError};

/// One driver is settled at a time. Its retained stderr is either still
/// with the active settlement, kept for the lawful end, carried by a held
/// outcome, or ready to tell.
#[derive(Default)]
pub(super) struct AttemptStderr {
    pub(super) active: Option<String>,
    contended: Option<(String, String)>,
    ready: Vec<(String, String)>,
}

impl AttemptStderr {
    pub(super) fn keep_contended(&mut self, attempt_id: Option<String>) {
        if let Some(attempt_id) = attempt_id {
            self.contended = self.active.take().map(|stderr| (attempt_id, stderr));
        }
    }

    pub(super) fn take_contended(&mut self) -> Option<(String, String)> {
        self.contended.take()
    }

    pub(super) fn restore_contended(&mut self, kept: Option<(String, String)>, attempt_id: &str) {
        self.active = kept
            .filter(|(id, _)| id == attempt_id)
            .map(|(_, stderr)| stderr);
    }

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
    /// Tell queued tails after the turn's appends, including any park it owes.
    /// An outcome landed by the lawful end gets any park it owes at the next
    /// turn's cursor. With no park due, [`Engine::execute_effect`] tells the
    /// tail before a wait or retry. A drive returning with a tail queued
    /// tells it after releasing the run.
    pub(super) fn finish_turn(&mut self) -> Result<(), EngineError> {
        if !self.stderr.ready.is_empty() {
            self.park_after_terminal()?;
            self.stderr.tell(&mut self.told);
        }
        Ok(())
    }
}
