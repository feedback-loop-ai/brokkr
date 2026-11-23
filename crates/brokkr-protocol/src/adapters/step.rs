//! Driver checkpoint identity, decided before its wire row is masked.

use serde_json::Value;

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
}

impl Emitted {
    pub(crate) fn new(step: Step, mut row: Value) -> Self {
        row["step"] = Value::String(step.word().into());
        Self { step, row }
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
