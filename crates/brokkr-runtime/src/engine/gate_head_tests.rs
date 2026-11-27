use super::tests::{engine, event, git_commit, member, single_body, step_body};
use super::verdict::tests::driver;
use super::*;
use crate::bundle::{GateAssessment, Limits, SeatClass, StepBody};
use brokkr_core::gate_moved_head::MovedHead;

fn marked(start: Option<&str>, end: Option<&str>) -> String {
    gate_moved_head::encode(&MovedHead {
        head_at_end: end.map(str::to_string),
        head_at_start: start.map(str::to_string),
    })
}

fn evidence(reasons: &[&str]) -> String {
    let events: Vec<_> = reasons
        .iter()
        .map(|reason| {
            event(
                EventType::EffectIndeterminate,
                json!({"effect_id": "e1", "attempt_id": "a1", "reason": reason}),
            )
        })
        .collect();
    gate_evidence(&events, gate_moved_head::decode).to_string()
}

/// The evidence a parked run carries is the newest marked indeterminate's
/// that reads as JSON, in the bytes the reason packed: both heads, a `null`
/// head that could not be read, or whatever JSON the marker carries, as the
/// pre-codec reader echoed it. Evidence that does not read is passed over,
/// so a run with none left parks with `{}`.
#[test]
fn a_parked_run_carries_the_newest_marked_evidence_byte_for_byte() {
    let both = marked(Some("aaa"), Some("bbb"));
    let start_unread = marked(None, Some("bbb"));
    let older = marked(Some("old"), Some("aaa"));
    let extra_key = r#"GATE-MOVED-HEAD {"head_at_end":"b","head_at_start":"a","x":1}"#;
    for (reasons, bytes) in [
        (
            vec![both.as_str()],
            r#"{"head_at_end":"bbb","head_at_start":"aaa"}"#,
        ),
        (
            vec![start_unread.as_str()],
            r#"{"head_at_end":"bbb","head_at_start":null}"#,
        ),
        (
            vec![older.as_str(), both.as_str()],
            r#"{"head_at_end":"bbb","head_at_start":"aaa"}"#,
        ),
        (
            vec![older.as_str(), extra_key],
            r#"{"head_at_end":"b","head_at_start":"a","x":1}"#,
        ),
        (vec!["GATE-MOVED-HEAD 42"], "42"),
        (
            vec![older.as_str(), "GATE-MOVED-HEAD not json"],
            r#"{"head_at_end":"aaa","head_at_start":"old"}"#,
        ),
        (vec!["GATE-MOVED-HEAD not json"], "{}"),
        (vec!["effect e1 indeterminate: timed out"], "{}"),
    ] {
        assert_eq!(evidence(&reasons), bytes, "{reasons:?}");
    }
}

fn editing(commands: &str, result: &str) -> Vec<String> {
    let mut driver = driver(&json!({"result": result}));
    driver[2] = format!("set -e\n{commands}\n{}", driver[2]);
    driver
}

fn start(body: SeatBody, gate: GateAssessment) -> (tempfile::TempDir, Engine) {
    let (dir, mut engine) = engine(body);
    let seat = engine.bundle.seats.get_mut("work").unwrap();
    seat.gate = gate;
    seat.limits = Limits::default();
    let work = engine
        .bundle
        .machine
        .rules
        .iter_mut()
        .find(|rule| rule.id == "WORK")
        .unwrap();
    work.next = Some("done".into());
    std::fs::write(engine.repo.join(".gitignore"), "ignored\n/.forge/\n").unwrap();
    git_commit(&engine.repo, "base");
    (dir, engine)
}

fn start_gate(commands: &str) -> (tempfile::TempDir, Engine) {
    start(
        single_body(editing(commands, "complete")),
        GateAssessment::AllGate,
    )
}

fn drive_tree_park(engine: &mut Engine, evidence: Value) -> String {
    let end = engine.drive().unwrap();
    assert_eq!(
        end.state.run.park_reason(),
        Some(&ParkReason::GateChangedTree)
    );
    let events = engine.store.load(&engine.run_id).unwrap();
    let parked = events
        .iter()
        .find(|event| event.event_type == EventType::RunParked)
        .unwrap();
    assert_eq!(parked.payload["reason"], "GATE-CHANGED-TREE");
    assert_eq!(parked.payload["cause"]["kind"], "ruled");
    assert_eq!(parked.payload["evidence"], evidence);
    assert_eq!(
        events
            .iter()
            .filter(|event| event.event_type == EventType::EffectSucceeded)
            .count(),
        0
    );
    let terminal = events
        .iter()
        .find(|event| event.event_type == EventType::EffectIndeterminate)
        .unwrap();
    terminal.payload["reason"].as_str().unwrap().to_owned()
}

#[test]
fn a_gate_that_leaves_a_tracked_edit_parks_with_paths_and_named_stderr() {
    let (_dir, mut engine) = start_gate("printf 'edited' > base.txt\nprintf diagnostic >&2");
    let before = git_head(&engine.repo).unwrap();
    let reason = drive_tree_park(&mut engine, json!({"omitted": 0, "paths": ["base.txt"]}));
    assert_eq!(git_head(&engine.repo), Some(before));
    let digest = brokkr_core::canonical::sha256_bytes(b"diagnostic");
    assert_eq!(
        reason,
        format!(
            "GATE-CHANGED-TREE {{\"omitted\":0,\"paths\":[\"base.txt\"]}}; driver stderr: 10 bytes (first byte: letter; sha256: {})",
            &digest[..16],
        )
    );
}

#[test]
fn a_gate_that_commits_and_edits_still_parks_on_moved_head() {
    let (_dir, mut engine) =
        start_gate("git commit -q --allow-empty -m gate-commit\nprintf 'edited' > base.txt");
    let end = engine.drive().unwrap();
    assert_eq!(
        end.state.run.park_reason(),
        Some(&ParkReason::GateMovedHead)
    );
    assert_eq!(
        std::fs::read(engine.repo.join("base.txt")).unwrap(),
        b"edited"
    );
}

#[test]
fn a_gate_uses_the_dirty_content_it_started_with() {
    let (_dir, mut engine) = start_gate(":");
    assert_eq!(engine.drive().unwrap().state.run, Run::Completed);

    let (_dir, mut engine) = start_gate(":");
    std::fs::write(engine.repo.join("base.txt"), "dirty").unwrap();
    assert_eq!(engine.drive().unwrap().state.run, Run::Completed);

    for command in [
        "printf 'changed further' > base.txt",
        "git restore base.txt",
    ] {
        let (_dir, mut engine) = start_gate(command);
        std::fs::write(engine.repo.join("base.txt"), "dirty").unwrap();
        std::fs::write(
            engine.repo.join(".gitignore"),
            "ignored\n/.forge/\n# dirty too\n",
        )
        .unwrap();
        drive_tree_park(&mut engine, json!({"omitted": 0, "paths": ["base.txt"]}));
    }
}

#[test]
fn a_gate_can_write_untracked_ignored_and_result_files() {
    let (_dir, mut engine) =
        start_gate("mkdir -p .forge/results\nprintf result > .forge/results/gate.json\nprintf scratch > ignored\nprintf new > untracked");
    assert_eq!(engine.drive().unwrap().state.run, Run::Completed);
    for (path, bytes) in [
        (".forge/results/gate.json", "result"),
        ("ignored", "scratch"),
        ("untracked", "new"),
    ] {
        assert_eq!(
            std::fs::read_to_string(engine.repo.join(path)).unwrap(),
            bytes
        );
    }
}

#[test]
fn tree_evidence_names_staged_binary_mode_deletion_and_type_changes() {
    let (_dir, mut engine) = start_gate(
        "printf staged-change > staged.txt\n git add staged.txt\nprintf '\\000changed' > binary.txt\nchmod +x mode.txt\nrm gone.txt type.txt\nln -s nowhere type.txt",
    );
    for path in ["binary.txt", "gone.txt", "mode.txt", "type.txt"] {
        std::fs::write(engine.repo.join(path), b"initial\0").unwrap();
    }
    git_commit(&engine.repo, "staged");
    drive_tree_park(
        &mut engine,
        json!({"omitted": 0, "paths": ["binary.txt", "gone.txt", "mode.txt", "staged.txt", "type.txt"]}),
    );
}

fn panel_then_chief(chief: &str) -> SeatBody {
    let panel = uncompiled_sequence_step! {
        name: "positions".into(),
        class: SeatClass::Work,
        results: vec!["pass".into(), "fail".into()],
        body: StepBody::Invocation(Invocation::Panel {
            members: vec![member("position", editing("printf 'panel edit' > base.txt", "pass"))],
            aggregate: Aggregate::UnanimousPass,
        }),
    };
    let chief = uncompiled_sequence_step! {
        name: "chief".into(),
        class: SeatClass::Gate,
        results: vec!["complete".into()],
        body: step_body(editing(chief, "complete")),
    };
    SeatBody::Sequence {
        steps: vec![panel, chief],
    }
}

#[test]
fn the_work_panel_may_edit_and_its_gate_chief_is_checked() {
    let make = |chief| {
        start(
            panel_then_chief(chief),
            GateAssessment::ContainsWork(crate::bundle::WorkSite::seat()),
        )
    };
    let (_dir, mut engine) = make(":");
    assert_eq!(engine.drive().unwrap().state.run, Run::Completed);
    assert_eq!(
        std::fs::read(engine.repo.join("base.txt")).unwrap(),
        b"panel edit"
    );

    let (_dir, mut engine) =
        make("test \"$(cat base.txt)\" = 'panel edit'\nprintf 'chief edit' > base.txt");
    drive_tree_park(&mut engine, json!({"omitted": 0, "paths": ["base.txt"]}));
    assert_eq!(
        std::fs::read(engine.repo.join("base.txt")).unwrap(),
        b"chief edit"
    );
}

#[test]
fn unreadable_tree_readings_cannot_pass_a_gate() {
    let (_dir, mut engine) = start_gate("printf broken > .git/index");
    drive_tree_park(
        &mut engine,
        json!({"end_unreadable": true, "start_unreadable": false}),
    );

    for (start_unreadable, end_unreadable) in [(false, true), (true, false), (true, true)] {
        let (_dir, mut engine) = start_gate(":");
        engine.trees = heads::TreeReads::scripted(vec![
            (!start_unreadable).then(Default::default),
            (!end_unreadable).then(Default::default),
        ]);
        drive_tree_park(
            &mut engine,
            json!({"end_unreadable": end_unreadable, "start_unreadable": start_unreadable}),
        );
    }
}

#[test]
fn tree_evidence_bounds_and_escapes_the_changed_paths() {
    let (_dir, mut engine) = start_gate(":");
    let mut paths = (0..51)
        .map(|index| (format!("path-{index:02}").into_bytes(), "digest".to_owned()))
        .collect::<BTreeMap<_, _>>();
    paths.insert(vec![b'a', 0xff], "digest".into());
    engine.trees = heads::TreeReads::scripted(vec![Some(Default::default()), Some(paths)]);
    let expected = std::iter::once("a\\xff".into())
        .chain((0..49).map(|index| format!("path-{index:02}")))
        .collect::<Vec<_>>();
    drive_tree_park(&mut engine, json!({"omitted": 2, "paths": expected}));
}
