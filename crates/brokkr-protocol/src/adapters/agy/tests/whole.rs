//! Round three of #1200: an agy turn is read to its last byte and typed
//! once from the bytes agy wrote, nothing outranks a refusal it showed, its
//! words are masked before they are cut, and a root git cannot name refuses
//! the launch.

use super::judged::{bound, bound_input, driven, driven_with, sent, stream, strings, Driven};
use super::*;
use fold::Event;

/// The recorded tool-less turn's init, its finished response step, and its
/// `SUCCESS` result.
fn turn_rows() -> (&'static str, &'static str, &'static str) {
    let lines: Vec<&'static str> = TOOL_LESS.lines().collect();
    (lines[0], lines[1], lines[lines.len() - 1])
}

/// The words a verdict ends with, or its answer.
fn words(verdict: Verdict) -> String {
    match verdict {
        Verdict::Answer(response) => format!("answer {response:?}"),
        Verdict::Refused(refused) => refused.words().to_string(),
        Verdict::Failed(failed) => failed.to_string(),
    }
}

/// The verdict of `rows` pinned to [`FLASH`], agy exiting `exit` and its
/// stderr holding the signed-out words where `on_stderr`.
fn judged_on(rows: &[&str], exit: i32, on_stderr: bool) -> String {
    words(read(stream(rows), FLASH).fold.verdict(exit, on_stderr))
}

/// A line that is not UTF-8 fails the turn wherever it stands, and reading
/// goes on past it: a `SUCCESS`, such a line and a denial is no answer,
/// through the driver's whole path with a delivered result file (K1).
#[test]
fn a_line_that_is_not_utf8_fails_the_turn_and_the_reading_goes_on() {
    let (init, step, result) = turn_rows();
    let lone = [
        init.as_bytes(),
        step.as_bytes(),
        b"\xff\xfe",
        result.as_bytes(),
    ]
    .join(&b'\n');
    assert_eq!(
        read(&lone, FLASH).fold.verdict(0, false),
        failed(Failure::NotText, Some("SUCCESS"), false)
    );
    let denial = r#"{"event":"result","result":{"status":"SUCCESS","denied_actions":[{"action":"command"}]}}"#;
    let behind = [
        init.as_bytes(),
        step.as_bytes(),
        result.as_bytes(),
        b"READY \xc3\x28",
        denial.as_bytes(),
    ]
    .join(&b'\n');
    let failure = Failed {
        failure: Failure::NotText,
        seen: fold::Seen {
            status: Some("SUCCESS".into()),
            timed_out: false,
        },
    };
    assert_eq!(
        read(&behind, FLASH).fold.verdict(0, false),
        Verdict::Failed(failure.clone())
    );
    let turn = driven_with((&behind, "", 0), true, FLASH, &[]).turn;
    assert_eq!(turn.map(drop), Err(failure.to_string()));
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), &behind, "", true, 0);
    let messages = world.run(&bin, &["--model", FLASH], world.input(None), None);
    assert_eq!(ended(&messages), None);
    assert_eq!(finished(&messages), None);
}

/// A known row that writes a key the fold reads twice, at any depth it
/// reads, does not type and fails the turn: serde_json would keep the last
/// (K1). Each line is written as text, since serde will not write one.
#[test]
fn a_known_row_that_repeats_a_key_the_fold_reads_fails() {
    let (init, step, result) = turn_rows();
    let results = [
        r#"{"event":"result","result":{"status":"SUCCESS","response":"READY\n","denied_actions":[{"action":"command"}],"denied_actions":[]}}"#,
        r#"{"event":"result","result":{"status":"ERROR","status":"SUCCESS","response":"READY\n"}}"#,
        r#"{"event":"result","result":{"status":"SUCCESS","response":"","response":"READY\n"}}"#,
        r#"{"event":"result","result":{"status":"SUCCESS","response":"READY\n","denied_actions":[{"action":"command","action":"x"}]}}"#,
        r#"{"event":"result","result":{"status":"SUCCESS","response":"READY\n","usage":{"input_tokens":1,"input_tokens":2}}}"#,
        r#"{"event":"result","result":{"status":"ERROR"},"result":{"status":"SUCCESS","response":"READY\n"}}"#,
        r#"{"event":"user","event":"result","result":{"status":"SUCCESS","response":"READY\n"}}"#,
    ];
    for row in results {
        assert_eq!(
            read(stream(&[init, step, row]), FLASH)
                .fold
                .verdict(0, false),
            failed(Failure::Unread(Event::Result), None, false),
            "{row}"
        );
    }
    let model_twice =
        format!(r#"{{"event":"init","init":{{"model":"gemini-4-neon","model":"{FLASH}"}}}}"#);
    let step_twice = r#"{"event":"step_update","step_update":{"step_type":"tool","step_type":"agent_response","state":"DONE"}}"#;
    let cases = [
        ([model_twice.as_str(), step, result], Event::Init),
        ([init, step_twice, result], Event::StepUpdate),
    ];
    for (rows, event) in cases {
        assert_eq!(
            read(stream(&rows), FLASH).fold.verdict(0, false),
            failed(Failure::Unread(event), Some("SUCCESS"), false),
            "{rows:?}"
        );
    }
}

/// The signed-out words in a pinned `SUCCESS` that holds a response, or on
/// stderr beside a clean answer, are the signed-out refusal: in the fold,
/// before work, and behind a delivered result file, where its words alone
/// leave (K2, K8).
#[test]
fn a_signed_out_marker_outranks_a_success_with_a_response() {
    let (init, _, _) = turn_rows();
    let marked = r#"{"event":"result","result":{"status":"SUCCESS","response":"READY\n","error":"authentication failed or timed out"}}"#;
    let signed_out = signed_out_words("SUCCESS");
    assert_eq!(judged_on(&[init, marked], 0, false), signed_out);
    assert_eq!(
        judged_on(&TOOL_LESS.lines().collect::<Vec<_>>(), 0, true),
        signed_out
    );
    // Before any work: a refusal, its stderr withheld.
    let early = driven((&stream(&[init, marked]), SIGNED_OUT_STDERR, 0), FLASH, &[]);
    let early = early.turn.unwrap();
    let refusal = early.refusal.map(|refusal| refusal.words().to_string());
    assert_eq!(refusal.as_deref(), Some(signed_out.as_str()));
    assert_eq!(early.stderr, SIGN_IN_WITHHELD);
    // Behind a delivered file, or behind work with the marker on stderr.
    let delivered = stream(&[init, marked]);
    let late = driven_with((delivered.as_bytes(), "", 0), true, FLASH, &[]).turn;
    assert_eq!(late.map(drop), Err(signed_out.clone()));
    let worked = driven((TOOL_LESS, SIGNED_OUT_STDERR, 0), FLASH, &[]);
    assert_eq!(worked.turn.map(drop), Err(signed_out));
}

/// K2's order: for each pair of adjacent ranks a turn showing both, and the
/// higher rank's verdict. Where two ranks cannot both hold (exit 3 and
/// another exit; a status and no result; no result and an empty
/// response), the pair is the higher rank and the next that can.
#[test]
fn the_verdict_is_the_first_rank_that_applies() {
    let (init, step, _) = turn_rows();
    let row = |status: &str, response: &str, extra: &str| {
        format!(
            r#"{{"event":"result","result":{{"status":"{status}","response":"{response}"{extra}}}}}"#
        )
    };
    let signed = r#","error":"authentication failed or timed out""#;
    let denied = r#","denied_actions":[{"action":"command"}]"#;
    let denial = "agy's result had status SUCCESS; agy stopped the turn at a denial of command: \
                  a turn agy stopped at a denial is not an answer (#1200)";
    let both = format!("{denied}{signed}");
    let cases: [(&str, Vec<String>, i32, bool, String); 9] = [
        (
            "1 over 2",
            vec![row("SUCCESS", "x", &both)],
            0,
            true,
            denial.into(),
        ),
        (
            "1 over 3",
            vec![row("SUCCESS", "x", denied)],
            3,
            false,
            format!("{denial}; agy then exited 3 at its own print timeout"),
        ),
        (
            "2 over 3",
            vec![row("ERROR", "", signed)],
            3,
            false,
            signed_out_words("ERROR"),
        ),
        (
            "3 over 4",
            vec![row("ERROR", "", "")],
            3,
            false,
            status_first(PRINT_TIMEOUT_WORDS, "ERROR"),
        ),
        (
            "4 over 5",
            vec![row("WAITING", "x", "")],
            5,
            false,
            "agy's result had status WAITING; agy exited 5, so the turn gave no answer (#1200)"
                .into(),
        ),
        (
            "5 over 7",
            vec![row("WAITING", "", "")],
            0,
            false,
            WAITING_WORDS.into(),
        ),
        (
            "6",
            Vec::new(),
            0,
            false,
            "agy's turn ended with no result, so it gave no answer (#1200)".into(),
        ),
        (
            "7 over 8",
            vec![row("SUCCESS", " ", "")],
            0,
            false,
            EMPTY_WORDS.into(),
        ),
        (
            "8",
            vec![row("SUCCESS", "x", "")],
            0,
            false,
            "answer \"x\"".into(),
        ),
    ];
    for (ranks, tail, exit, on_stderr, expected) in cases {
        let mut rows = vec![init, step];
        rows.extend(tail.iter().map(String::as_str));
        assert_eq!(judged_on(&rows, exit, on_stderr), expected, "{ranks}");
    }
}

/// A bound value of 44 characters.
const LONG: &str = "Zq7Wv2Lk9Rt4Hy6Np3Bx8Mc5Dg1Fs0Ja7Ke2Uo9Ip4Q3";

/// Every text one driven turn left the driver with.
fn left_by(driven: &Driven) -> Vec<String> {
    let mut left: Vec<String> = driven.rows.iter().flat_map(strings).collect();
    match &driven.turn {
        Err(words) => left.push(words.clone()),
        Ok(invocation) => {
            left.extend(strings(&Value::Object(invocation.session_meta.clone())));
            left.extend(invocation.answer.iter().flat_map(strings));
            left.extend(invocation.refusal.iter().map(|r| r.words().to_string()));
            left.push(invocation.stderr.clone());
        }
    }
    left
}

/// A bound value agy echoed whole as a status, a denied action or the
/// served model is masked before any word is cut: no 8 characters of it in
/// a row, its first 40 among them, leave the driver or the shared one (K3).
#[test]
fn a_bound_value_agy_echoed_whole_is_masked_before_any_word_is_cut() {
    assert_eq!(LONG.chars().count(), 44);
    let (init, step, _) = turn_rows();
    let status = format!(r#"{{"event":"result","result":{{"status":"{LONG}"}}}}"#);
    let action = format!(
        r#"{{"event":"result","result":{{"status":"SUCCESS","denied_actions":[{{"action":"{LONG}"}}]}}}}"#
    );
    let model = format!(r#"{{"event":"init","init":{{"model":"{LONG}"}}}}"#);
    let turns = [
        stream(&[init, step, &status]),
        stream(&[init, step, &action]),
        model,
    ];
    let bindings = [bound("AT", LONG)];
    let runs: Vec<&str> = (0..=LONG.len() - 8).map(|at| &LONG[at..at + 8]).collect();
    for stdout in turns {
        let mut left = left_by(&driven((&stdout, "", 0), FLASH, &bindings));
        let mut world = World::new();
        let bin = fake_agy(&world.seat(), &stdout, "", false, 0);
        let input = bound_input(&world, LONG);
        left.extend(sent(&world.run(&bin, &["--model", FLASH], input, None)));
        for run in &runs {
            assert!(
                left.iter().all(|text| !text.contains(run)),
                "{run}: {left:?}"
            );
        }
        assert!(
            left.iter().any(|text| text.contains("[secret:AT]")),
            "{left:?}"
        );
    }
}

/// The failure of a turn that ended `WAITING` and exited 0.
const WAITING_WORDS: &str = "agy's result had status WAITING; agy's turn ended with a status \
                             that is not SUCCESS, which is not an answer (#1200)";

/// A result's status is named at any exit, through the driver (K5).
#[test]
fn a_result_names_its_status_at_any_exit() {
    let (init, _, _) = turn_rows();
    let cases = [
        (
            "ERROR",
            1,
            "agy's result had status ERROR; agy exited 1, so the turn gave no answer (#1200)",
        ),
        ("WAITING", 0, WAITING_WORDS),
        (
            "WAITING",
            5,
            "agy's result had status WAITING; agy exited 5, so the turn gave no answer (#1200)",
        ),
    ];
    for (status, code, words) in cases {
        let row = format!(r#"{{"event":"result","result":{{"status":"{status}"}}}}"#);
        let turn = driven((&stream(&[init, &row]), "", code), FLASH, &[]).turn;
        assert_eq!(turn.map(drop), Err(words.to_string()), "{status} {code}");
    }
}

/// A line that is passed over is no row: exit 2 behind it is still a
/// refused option (K6).
#[test]
fn a_passed_over_line_before_a_bad_flag_exit_leaves_the_bad_flag_refusal() {
    let bad_flag = "launch_refused: agy exited 2 on an option it does not define";
    let passed = "not json\n{\"event\":\"user\"}\n";
    assert_eq!(words(read(passed, FLASH).fold.verdict(2, false)), bad_flag);
    let turn = driven((passed, BAD_FLAG_STDERR, 2), FLASH, &[])
        .turn
        .unwrap();
    assert_eq!(
        turn.refusal.map(|r| r.words().to_string()),
        Some(bad_flag.into())
    );
}

/// An operator HOME that holds `.owner.lock` at its top is linked like any
/// other entry: the child's HOME is the session's `home`, and the session's
/// own lock stands beside it (K7).
#[test]
fn an_operator_home_holding_an_owner_lock_is_linked_and_the_launch_runs() {
    let (_root, operator, temp) = home_root();
    let lock = operator.join(".owner.lock");
    std::fs::write(&lock, "the operator's").unwrap();
    let operator = Some(operator.as_path());
    let home = home::made(
        Homes {
            temp: &temp,
            operator,
        },
        written(&empty_set()),
    )
    .unwrap();
    let path = Path::new(home.path());
    assert_eq!(std::fs::read_link(path.join(".owner.lock")).unwrap(), lock);
    let session = path.parent().unwrap();
    assert_eq!(listed(session), [".owner.lock", "home"]);
    assert!(!session.join(".owner.lock").is_symlink());
    drop(home);
    let mut world = World::new();
    std::fs::write(world.path("operator/.owner.lock"), "").unwrap();
    world.answers();
    assert!(world.homes_gone());
}

/// Each of agy's workspace files, at the working directory and in the
/// directory above it, refuses through the no-spawn route: the fake, which
/// records its start, never runs (K9).
#[test]
fn each_workspace_configuration_at_the_workdir_or_above_never_starts_agy() {
    for relative in CONFIGURATION {
        for at_root in [false, true] {
            let mut world = World::new();
            let repo = world.seat().canonicalize().unwrap();
            let workdir = sub(&repo);
            let bin = fake_agy(&world.seat(), TOOL_LESS, "", true, 0);
            let planted = plant(if at_root { &repo } else { &workdir }, relative);
            let mut input = world.input(None);
            input["workdir"] = json!(workdir);
            let messages = world.run(&bin, &["--model", FLASH], input, None);
            let error = ended(&messages).and_then(|(_, error, _)| error);
            assert_eq!(
                error,
                Some(LaunchError::Configured(planted).to_string()),
                "{relative} {at_root}"
            );
            assert!(!world.started(), "{relative} {at_root}");
        }
    }
}

/// A turn that answered and wrote no result file sends its turn and its
/// finish, and no result: the start ends the driver (K9).
#[test]
fn a_turn_that_answered_and_delivered_no_result_file_sends_no_result() {
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", false, 0);
    world.set(&bin);
    let start = json!({"attempt_id": "attempt", "effect_id": "effect", "input": world.input(None)});
    let mut messages = Vec::new();
    let answered = super::super::super::run_seat(
        AdapterKind::Agy,
        &argv(&["--model", FLASH]),
        &start,
        None,
        &mut |body| messages.push(body),
    );
    assert!(!answered);
    let sent: Vec<String> = messages
        .iter()
        .map(|body| match body {
            crate::Body::Accepted { .. } => "accepted".to_string(),
            crate::Body::Checkpoint { data, .. } => data["step"].as_str().unwrap().to_string(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(
        sent,
        [
            "accepted",
            "seat-turn",
            "transcript",
            "agy-cli-session-finished"
        ]
    );
    assert!(world.homes_gone());
}
