//! Driver checkpoint identity, decided before its wire row is masked.

use serde_json::{json, Value};

use super::row::CapturedId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Step {
    Transcript,
    HarnessStarted,
    ExecStarted,
    SeatTurn,
    TurnStarted,
    ItemStarted,
    ItemCompleted,
    TurnCompleted,
}

impl Step {
    pub(super) fn word(self) -> &'static str {
        match self {
            Self::Transcript => "transcript",
            Self::HarnessStarted => "harness-started",
            Self::ExecStarted => "exec-started",
            Self::SeatTurn => "seat-turn",
            Self::TurnStarted => "turn-started",
            Self::ItemStarted => "item-started",
            Self::ItemCompleted => "item-completed",
            Self::TurnCompleted => "turn-completed",
        }
    }
}

/// A checkpoint proves work only beyond the two pre-session steps.
pub(super) fn begins_work(step: Step) -> bool {
    match step {
        Step::Transcript | Step::HarnessStarted => false,
        Step::ExecStarted
        | Step::SeatTurn
        | Step::TurnStarted
        | Step::ItemStarted
        | Step::ItemCompleted
        | Step::TurnCompleted => true,
    }
}

pub(crate) struct Emitted {
    pub(super) step: Step,
    pub(crate) row: Value,
    pub(super) identity: Option<CapturedId>,
}

impl Emitted {
    pub(crate) fn new(step: Step, mut row: Value) -> Self {
        row["step"] = Value::String(step.word().into());
        Self {
            step,
            row,
            identity: None,
        }
    }

    pub(crate) fn transcript(address: &Value, identity: Option<CapturedId>) -> Self {
        let mut emitted = Self::new(Step::Transcript, json!({"transcript": address}));
        emitted.identity = identity;
        emitted
    }
}

impl super::LaunchHold {
    /// The one door every emitted row passes through, so that the
    /// harness naming its own session is what publishes the launch —
    /// and publishes it BEFORE the first work row, where decision 0053's
    /// buffer will keep it in order.
    ///
    /// Claude and Codex attach the raw identity to their transcript
    /// checkpoint. Dsh confirms its session separately from its retained
    /// directory locator.
    pub(super) fn observe(&mut self, data: &Emitted, emit: &mut impl FnMut(&Emitted)) {
        emit(data);
        if begins_work(data.step) {
            self.began_work = true;
        }
        if self.outcome.is_some()
            || data.step != Step::Transcript
            || !self.plan.confirms_from_locator
        {
            return;
        }
        if let Some(identity) = data.identity.as_ref().filter(|id| !id.ident.is_offered("")) {
            self.confirm(&identity.ident, &identity.display, emit);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_checkpoint_word_comes_from_its_typed_step() {
        for (step, word) in [
            (Step::Transcript, "transcript"),
            (Step::HarnessStarted, "harness-started"),
            (Step::ExecStarted, "exec-started"),
            (Step::SeatTurn, "seat-turn"),
            (Step::TurnStarted, "turn-started"),
            (Step::ItemStarted, "item-started"),
            (Step::ItemCompleted, "item-completed"),
            (Step::TurnCompleted, "turn-completed"),
        ] {
            let emitted = Emitted::new(step, json!({"step":"wrong","turn":2}));
            assert_eq!(emitted.step, step);
            assert_eq!(emitted.row, json!({"step":word,"turn":2}));
        }
    }
}
