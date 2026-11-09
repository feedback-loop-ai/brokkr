//! Decision 0103 slice 4c2a's coverage round: the probe form a sequence seat
//! reaches before 4c2b builds that slice's own sequence probe. A wait the
//! journal records at a sequence body is refused by name, never probed as a
//! single seat.

use super::*;
use crate::bundle::{SeatBody, SeatClass};

/// A wait recorded at a sequence seat reaches `ProbeSeat::of`'s sequence
/// form and parks naming the body, invoking no driver.
#[test]
fn a_wait_recorded_at_a_sequence_seat_refuses_the_probe_form_by_name() {
    let dir = tempfile::tempdir().unwrap();
    let body = SeatBody::Sequence {
        steps: vec![uncompiled_sequence_step! {
            name: "only".into(),
            class: SeatClass::Work,
            results: vec!["complete".into()],
            body: crate::engine::tests::step_body(vec!["unused".into()]),
        }],
    };
    let mut engine = started(dir.path(), body, work(2));
    let compiled = engine.bundle.clone();
    append_failed_payload(
        &mut engine,
        json!({"effect_id": "effect", "attempt_id": "a1", "error": "refused",
               "start_failure": true, "start_failure_sites": [null],
               "waits": wait_value(base() + time::Duration::seconds(1), 1)}),
    );
    let run = engine.run_id.clone();
    drop(engine);
    let mut resumed = resume_engine(dir.path(), compiled, &run);
    resumed.cooldown.clock = Clock::advancing(base() + time::Duration::seconds(2)).0;
    resumed.drive().unwrap();
    let reason = park_reason(&resumed);
    assert!(
        reason.contains("a wait is recorded at a body that is not a single seat"),
        "{reason}"
    );
    assert!(logs(dir.path(), "work").is_empty(), "the sequence ran");
}
