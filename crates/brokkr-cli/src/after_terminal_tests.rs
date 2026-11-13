//! The CLI's end of the store's terminal rule (#440): a run concluded
//! beneath its driver prints the driver's typed words and exits failed,
//! never with the retryable contention code.

use brokkr_core::envelope::EventType;
use brokkr_core::fold::Status;
use std::process::ExitCode;

use super::*;

/// A run concluded beneath its driver ends the driver with
/// `ConcludedBeneath`: the CLI prints the typed words on one line and
/// exits `failed`, and it is never contention's retryable code (#440).
#[test]
fn a_concluded_beneath_ending_prints_its_words_and_exits_failed() {
    let error: anyhow::Error = brokkr_runtime::EngineError::ConcludedBeneath {
        run_id: "r1".into(),
        status: Status::Stopped,
        event_type: EventType::EffectSucceeded,
        attempt_id: Some("attempt-1".into()),
    }
    .into();
    assert!(contention(&error).is_none(), "a refusal is not contention");
    assert_eq!(report(&error), ExitCode::from(Exit::Failed));
    assert_eq!(
        failure_line(&error),
        "error: run 'r1' was concluded (stopped) while attempt 'attempt-1' was still working, so \
         its effect/succeeded could not be appended and nothing was written; the driver stops \
         here, and the conclusion already closed the attempt as effect/indeterminate — the \
         report this driver was carrying is lost to the journal"
    );
}
