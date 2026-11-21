//! No journal event carries a byte of a driver's stderr (#1199, decision
//! 0012's addendum of 2026-11-20): a failed or indeterminate attempt names
//! it by content-free evidence, whose digest prefix is the SHA-256 of the
//! retained stderr, and a spawn failure, which has none, names none. The
//! masked tail is told to the engine's stderr only once the attempt's
//! terminal event is appended (the addendum's paragraph of 2026-11-21).

use std::sync::{Arc, Mutex};

use super::*;
use crate::engine::sequence::SequenceAt;

/// What the fake harness prints that the seat binds nowhere, so no driver
/// masks it.
const SENTINEL: &str = "UNBOUND-SENTINEL-1199";

/// The whole of the driver's stderr: the sentinel's line.
const SAID: &str = "the harness said UNBOUND-SENTINEL-1199\n";

/// What a driver that ended before its result is journaled as.
const LOST: &str =
    "driver exited after accepting, before a result — attempt cannot be established as complete";

/// [`SAID`] as the engine tells it under the attempt `event` names.
fn told(event: &EventEnvelope) -> String {
    let attempt = event.payload["attempt_id"].as_str().unwrap();
    format!("{attempt}: driver stderr: the harness said UNBOUND-SENTINEL-1199\n")
}

/// `command`, a `sh -c` driver, printing [`SAID`] on its stderr first.
fn saying(mut command: Vec<String>) -> Vec<String> {
    command[2] = format!("printf '%s' '{SAID}' >&2; {}", command[2]);
    command
}

/// [`SAID`] as the journal names it.
fn evidence() -> String {
    let digest = &brokkr_core::canonical::sha256_bytes(SAID.as_bytes())[..16];
    format!("driver stderr: 39 bytes (first byte: letter; sha256: {digest})")
}

fn failing() -> Vec<String> {
    driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Failed {
            error: "boom".into(),
        },
    )
}

/// The engine's stderr as a test watches it: what was told, and the
/// event types the run's journal already held when its first byte came.
#[derive(Clone)]
pub(in crate::engine) struct Watch {
    db: std::path::PathBuf,
    run_id: String,
    held_at_first_byte: Arc<Mutex<Option<Vec<EventType>>>>,
    told: Arc<Mutex<Vec<u8>>>,
}

impl std::io::Write for Watch {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        let mut held = self.held_at_first_byte.lock().unwrap();
        if held.is_none() {
            let events = Store::open(&self.db).unwrap().load(&self.run_id).unwrap();
            *held = Some(events.iter().map(|event| event.event_type).collect());
        }
        self.told.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Watch {
    /// Hand `engine` a watch for its stderr, journaled under `dir`.
    pub(in crate::engine) fn on(dir: impl AsRef<std::path::Path>, engine: &mut Engine) -> Watch {
        let watch = Watch {
            db: dir.as_ref().join("forge.db"),
            run_id: engine.run_id.clone(),
            held_at_first_byte: Arc::default(),
            told: Arc::default(),
        };
        engine.told = Box::new(watch.clone());
        watch
    }

    /// What was told, and whether `terminal` was journaled before it.
    fn told_after(&self, terminal: EventType) -> (String, bool) {
        let (told, held) = self.told_and_held();
        (told, held.contains(&terminal))
    }

    /// What was told, and the event types journaled before its first
    /// byte: none where nothing was told.
    pub(in crate::engine) fn told_and_held(&self) -> (String, Vec<EventType>) {
        let told = String::from_utf8(self.told.lock().unwrap().clone()).unwrap();
        let held = self.held_at_first_byte.lock().unwrap().clone();
        (told, held.unwrap_or_default())
    }
}

/// A writer that refuses every byte, as a closed stderr does.
struct Closed;

impl std::io::Write for Closed {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
}

/// Every envelope of the run's journal, none of which holds the sentinel.
fn assert_sealed(engine: &Engine) -> Vec<EventEnvelope> {
    let events = engine.store.load(&engine.run_id).unwrap();
    for event in &events {
        let written = serde_json::to_string(event).unwrap();
        assert!(!written.contains(SENTINEL), "journaled: {written}");
    }
    events
}

/// The run's one `event_type` event.
fn terminal(events: &[EventEnvelope], event_type: EventType) -> &EventEnvelope {
    events.iter().find(|e| e.event_type == event_type).unwrap()
}

/// The `field` text of the run's one `event_type` event.
fn terminal_text(events: &[EventEnvelope], event_type: EventType, field: &str) -> String {
    terminal(events, event_type).payload[field]
        .as_str()
        .unwrap()
        .to_string()
}

fn executed(command: Vec<String>) -> (tempfile::TempDir, Engine, Watch) {
    let (dir, mut driven) = engine(single_body(command));
    let watch = Watch::on(&dir, &mut driven);
    let requested = requested(&driven, "effect");
    execute_work(&mut driven, &[requested]).unwrap();
    (dir, driven, watch)
}

#[test]
fn a_failed_attempt_journals_its_stderr_by_evidence_alone() {
    assert_eq!(SAID.len(), 39);
    let (_dir, driven, watch) = executed(saying(failing()));
    let events = assert_sealed(&driven);
    assert_eq!(
        terminal_text(&events, EventType::EffectFailed, "error"),
        format!("boom; {}", evidence())
    );
    let failed = terminal(&events, EventType::EffectFailed);
    assert_eq!(
        watch.told_after(EventType::EffectFailed),
        (told(failed), true)
    );
}

#[test]
fn an_indeterminate_attempt_journals_its_stderr_by_evidence_alone() {
    let lost = driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Indeterminate {
            reason: "lost".into(),
        },
    );
    let (_dir, driven, watch) = executed(saying(lost));
    let events = assert_sealed(&driven);
    assert_eq!(
        terminal_text(&events, EventType::EffectIndeterminate, "reason"),
        format!("{LOST}; {}", evidence())
    );
    let parked = terminal(&events, EventType::EffectIndeterminate);
    assert_eq!(
        watch.told_after(EventType::EffectIndeterminate),
        (told(parked), true)
    );
}

/// The sequence engine whose one step runs `command`, its stderr watched.
fn sequenced(command: Vec<String>) -> (tempfile::TempDir, Engine, Watch) {
    let (dir, mut driven) = engine(single_body(vec!["driver".into()]));
    let watch = Watch::on(&dir, &mut driven);
    run_sequence(
        &mut driven,
        &SequenceAt::new("effect", "attempt", "work"),
        &[step("failing", command)],
        &json!({"role_path": "role.md", "inputs": {}}),
        5,
        &Selection::new(),
    )
    .unwrap();
    (dir, driven, watch)
}

#[test]
fn a_sequence_step_failure_journals_its_stderr_by_evidence_alone() {
    let (_dir, driven, watch) = sequenced(saying(failing()));
    let events = assert_sealed(&driven);
    assert_eq!(
        terminal_text(&events, EventType::EffectFailed, "error"),
        format!("sequence step 'failing': boom; {}", evidence())
    );
    let failed = terminal(&events, EventType::EffectFailed);
    assert_eq!(failed.payload["attempt_id"], "attempt");
    assert_eq!(
        watch.told_after(EventType::EffectFailed),
        (told(failed), true)
    );
}

#[test]
fn a_sequence_step_indeterminate_journals_its_stderr_before_it_is_told() {
    let lost = driver_command(
        "effect",
        "attempt",
        AttemptOutcome::Indeterminate {
            reason: "lost".into(),
        },
    );
    let (_dir, driven, watch) = sequenced(saying(lost));
    let events = assert_sealed(&driven);
    assert_eq!(
        terminal_text(&events, EventType::EffectIndeterminate, "reason"),
        format!("sequence step 'failing': {LOST}; {}", evidence())
    );
    let parked = terminal(&events, EventType::EffectIndeterminate);
    assert_eq!(
        watch.told_after(EventType::EffectIndeterminate),
        (told(parked), true)
    );
}

/// A closed engine stderr neither panics the tell nor keeps the attempt
/// from settling: its terminal event is journaled whole.
#[test]
fn a_tell_to_a_closed_stderr_still_settles_the_attempt() {
    let (_dir, mut driven) = engine(single_body(saying(failing())));
    driven.told = Box::new(Closed);
    let requested = requested(&driven, "effect");
    execute_work(&mut driven, &[requested]).unwrap();
    assert_eq!(failed_error(&driven), format!("boom; {}", evidence()));
}

#[test]
fn an_empty_stderr_is_named_in_fixed_words_and_tells_nothing() {
    let (_dir, driven, watch) = executed(failing());
    assert_eq!(failed_error(&driven), "boom; driver stderr: empty");
    assert_eq!(
        watch.told_after(EventType::EffectFailed),
        (String::new(), false)
    );
}

/// A spawn failure's terminal event is the one main journals, byte for
/// byte: no stderr is named, and nothing is told.
#[test]
fn a_spawn_failure_names_no_stderr() {
    let (_dir, driven, watch) = executed(vec!["/nonexistent/brokkr-1199-driver".into()]);
    let events = driven.store.load(&driven.run_id).unwrap();
    let started = terminal(&events, EventType::EffectStarted);
    let attempt = started.payload["attempt_id"].as_str().unwrap();
    let failed = terminal(&events, EventType::EffectFailed);
    assert_eq!(
        serde_json::to_string(&failed.payload).unwrap(),
        format!(
            "{{\"attempt_id\":\"{attempt}\",\"effect_id\":\"effect\",\"error\":\
             \"driver did not spawn: failed to spawn driver /nonexistent/brokkr-1199-driver: \
             No such file or directory (os error 2)\"}}"
        )
    );
    assert_eq!(
        watch.told_after(EventType::EffectFailed),
        (String::new(), false)
    );
    // A sequence step's driver that never spawned retained no stderr.
    let (_dir, driven, watch) = sequenced(vec!["/nonexistent/brokkr-1199-driver".into()]);
    assert_eq!(
        failed_error(&driven),
        "sequence step 'failing': driver did not spawn: failed to spawn driver \
         /nonexistent/brokkr-1199-driver: No such file or directory (os error 2)"
    );
    assert_eq!(
        watch.told_after(EventType::EffectFailed),
        (String::new(), false)
    );
}

/// Set only on a wrapper's own command line: the validator argv the
/// engine handed `driver exec` after its `--`, one a line.
#[cfg(unix)]
const SERVE_EXEC: &str = "BROKKR_DRIVER_STDERR_SERVE_EXEC";

/// Not a test of its own: production's exec driver, served when the
/// wrapper re-enters this binary with [`SERVE_EXEC`] set.
#[cfg(unix)]
#[test]
fn exec_driver_child() {
    let Ok(extra) = std::env::var(SERVE_EXEC) else {
        return;
    };
    let extra = extra.lines().map(str::to_string).collect();
    let kind = brokkr_protocol::adapters::AdapterKind::Exec;
    let served = brokkr_protocol::adapters::serve(kind, extra, || Ok(()));
    std::process::exit(if served.is_ok() { 0 } else { 70 });
}

/// The engine's own executable for a dialect step: a wrapper that drops
/// `driver exec --` and serves the rest through [`exec_driver_child`],
/// passing on only protocol lines. Copied in by a child and renamed,
/// never exec'd from a file this process wrote: a sibling test's fork
/// that still holds the writer would make it `Text file busy` (ETXTBSY).
#[cfg(unix)]
fn serving_exec(dir: &std::path::Path) -> std::path::PathBuf {
    use crate::engine::capability_tests::provision_tests::survey_tests::installed;
    let body = format!(
        "#!/bin/sh\nshift 3\n{SERVE_EXEC}=\"$(printf '%s\\n' \"$@\")\" \
         exec '{exe}' --exact engine::tests::driver_stderr::exec_driver_child \
         --nocapture --test-threads=1 -q | grep --line-buffered '^{{'\n",
        exe = std::env::current_exe().unwrap().display(),
    );
    let source = dir.join("serve-exec.sh");
    std::fs::write(&source, body).unwrap();
    installed(dir, "serve-exec", &source)
}

/// A dialect step whose validator prints `stdout` and the sentinel on its
/// stderr, then exits `code`.
#[cfg(unix)]
fn validator(stdout: &str, code: i32) -> SequenceStep {
    let script = format!("printf '{stdout}'; printf %s {SENTINEL} >&2; exit {code}");
    uncompiled_sequence_step! {
        name: "validate".into(),
        class: SeatClass::Gate,
        results: vec!["drafted".into(), "fail".into()],
        body: StepBody::Dialect {
            execution: crate::bundle::DialectExecution {
                argv: vec!["sh".into(), "-c".into(), script],
                state: None,
            },
        },
    }
}

/// The sequence engine over `steps`, its executable the exec driver, its
/// stderr watched; run to its end.
#[cfg(unix)]
fn dialect_run(steps: &[SequenceStep]) -> (tempfile::TempDir, Engine, Watch) {
    let (dir, mut driven) = engine(single_body(vec!["driver".into()]));
    driven.bundle.executable = crate::argv::Executable(Some(serving_exec(dir.path())));
    let watch = Watch::on(&dir, &mut driven);
    let at = SequenceAt::new("effect", "attempt", "work");
    let input = json!({"role_path": "role.md", "inputs": {}});
    run_sequence(&mut driven, &at, steps, &input, 30, &Selection::new()).unwrap();
    (dir, driven, watch)
}

/// HIGH A of #1199's round two: a dialect check's notes are its stdout,
/// its stderr named only by evidence, through production's exec driver,
/// as a non-final step's checkpoint and as the attempt's result, whether
/// the validator passed or failed.
#[cfg(unix)]
#[test]
fn a_dialect_checks_stderr_reaches_no_journal_event() {
    let digest = brokkr_core::canonical::sha256_bytes(SENTINEL.as_bytes());
    let named = format!(
        "validator stderr: 21 bytes (first byte: letter; sha256: {})",
        &digest[..16]
    );
    let then = step(
        "then",
        driver_command(
            "effect",
            "attempt",
            AttemptOutcome::Succeeded {
                result: json!({"result": "complete"}),
            },
        ),
    );
    let checkpoint = EventType::EffectCheckpointed;
    let succeeded = EventType::EffectSucceeded;
    for (steps, journaled, verdict, notes) in [
        (
            vec![validator("", 1), then.clone()],
            checkpoint,
            "fail",
            named.clone(),
        ),
        (vec![validator("", 1)], succeeded, "fail", named.clone()),
        (
            vec![validator("diag", 0), then],
            checkpoint,
            "drafted",
            format!("diag\n{named}"),
        ),
        (
            vec![validator("diag", 0)],
            succeeded,
            "drafted",
            format!("diag\n{named}"),
        ),
    ] {
        let (_dir, driven, _watch) = dialect_run(&steps);
        let events = assert_sealed(&driven);
        let (event_type, result) = events
            .iter()
            .find_map(|event| match event.event_type {
                EventType::EffectCheckpointed => {
                    let checkpoint = &event.payload["checkpoint"];
                    (checkpoint["step"] == "sequence-step-finished")
                        .then(|| (event.event_type, checkpoint["result"].clone()))
                }
                EventType::EffectSucceeded => {
                    Some((event.event_type, event.payload["result"].clone()))
                }
                _ => None,
            })
            .unwrap_or_else(|| panic!("the validator's result is journaled: {events:#?}"));
        assert_eq!(
            (event_type, &result["result"], &result["notes"]),
            (journaled, &json!(verdict), &json!(notes)),
            "{result}"
        );
    }
}

/// A validator a signal kills leaves no result; its masked tail is still
/// told, once the step's indeterminate is journaled.
#[cfg(unix)]
#[test]
fn a_killed_validators_tail_is_told_after_its_step_settles() {
    let mut killed = validator("", 0);
    let StepBody::Dialect { execution } = &mut killed.body else {
        unreachable!("a dialect step")
    };
    execution.argv[2] = format!("printf '%s\\n' {SENTINEL} >&2; kill -KILL $$");
    let (_dir, driven, watch) = dialect_run(&[killed]);
    let events = assert_sealed(&driven);
    let parked = terminal(&events, EventType::EffectIndeterminate);
    let attempt = parked.payload["attempt_id"].as_str().unwrap();
    assert_eq!(
        watch.told_after(EventType::EffectIndeterminate),
        (
            format!(
                "{attempt}: driver stderr: {SENTINEL}\n\
                 {attempt}: driver stderr: agent CLI ended by a signal, so no result is sent\n"
            ),
            true
        )
    );
}

/// A step's result the seat-record fence refuses fails the attempt there,
/// and the step's tail is told once that failure is journaled: at the
/// final result, and at a non-final step's checkpoint.
#[test]
fn a_fenced_steps_tail_is_told_after_its_failure() {
    let refused = |result: Value| {
        let succeeded = AttemptOutcome::Succeeded { result };
        step(
            "fenced",
            saying(driver_command("effect", "attempt", succeeded)),
        )
    };
    let last = refused(json!({"result": "complete", "commits": ["see the branch"]}));
    let first = refused(json!({"result": "complete", "model": "a configured guess"}));
    let never = step("never", vec!["missing-driver".into()]);
    for (steps, input) in [
        (vec![last], step_input()),
        (vec![first, never], two_step_input()),
    ] {
        let (dir, mut driven) = engine(single_body(vec!["driver".into()]));
        let watch = Watch::on(&dir, &mut driven);
        let at = SequenceAt::new("effect", "attempt", "work");
        run_sequence(&mut driven, &at, &steps, &input, 5, &Selection::new()).unwrap();
        let events = assert_sealed(&driven);
        let failed = terminal(&events, EventType::EffectFailed);
        assert_eq!(
            watch.told_after(EventType::EffectFailed),
            (told(failed), true)
        );
    }
}
