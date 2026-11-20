//! Round two of #1200: an agy seat's HOME is the engine's whatever it
//! binds, its turn is judged on the rows agy wrote, a denial once seen is
//! never lost, one launch is one turn, and what the driver refuses it names
//! without a value.

use std::os::unix::fs::PermissionsExt;

use super::*;
use fold::Event;

/// `value` bound to the seat as `name`, whatever its length: the store is
/// written by hand, so a value `brokkr secrets set` would refuse as short
/// is bound too.
pub(super) fn bound(name: &str, value: &str) -> secret::BoundSecret {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("secrets.env");
    std::fs::write(&store, format!("{name}={value}\n")).unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut bindings = secret::resolve_bindings(&store, &[name.to_string()]).unwrap();
    bindings.pop().unwrap()
}

/// What one turn of a fake `agy` left the driver with.
pub(super) struct Driven {
    pub(super) turn: Result<Invocation, String>,
    pub(super) rows: Vec<Value>,
    started: bool,
}

/// One turn of a fake `agy` printing `stdout` and `stderr` and exiting
/// `code`, run by [`drive`] for a seat pinned to `pin` under `bindings`.
pub(super) fn driven(
    (stdout, stderr, code): (&str, &str, i32),
    pin: &str,
    bindings: &[secret::BoundSecret],
) -> Driven {
    driven_with((stdout.as_bytes(), stderr, code), false, pin, bindings)
}

/// [`driven`] over `stdout`'s bytes, the seat's result written where
/// `delivers`.
pub(super) fn driven_with(
    (stdout, stderr, code): (&[u8], &str, i32),
    delivers: bool,
    pin: &str,
    bindings: &[secret::BoundSecret],
) -> Driven {
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), stdout, stderr, delivers, code);
    world.set(&bin);
    let input = world.input(None);
    let (binary, extra) = (bin.to_string_lossy(), argv(&["--model", pin]));
    let pinned = super::super::super::Pinned {
        binary: &binary,
        extra: &extra,
    };
    let mut rows = Vec::new();
    let turn = drive(pinned, false, "prompt", &input, bindings, &mut |row| {
        rows.push(row.clone())
    });
    assert!(world.homes_gone());
    Driven {
        turn,
        rows,
        started: world.started(),
    }
}

/// A seat that binds `HOME` is refused by the name before spawn; with the
/// refusal passed by, the child's HOME is still the engine's (J1).
#[test]
fn a_seat_that_binds_home_is_refused_and_its_child_home_is_the_engines() {
    let value = "/srv/the-operators-own-home";
    let bindings = [bound("HOME", value)];
    let refused = driven((TOOL_LESS, "", 0), FLASH, &bindings);
    let words = refused.turn.map(drop).unwrap_err();
    assert_eq!(words, LaunchError::HomeBound.to_string());
    assert!(
        words.contains("HOME") && !words.contains("operators-own"),
        "{words}"
    );
    assert!(!refused.started);
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), TOOL_LESS, "", false, 0);
    world.set(&bin);
    let input = world.input(None);
    let temp = std::env::temp_dir();
    let operator = world.path("operator");
    let homes = Homes {
        temp: &temp,
        operator: Some(&operator),
    };
    let (binary, extra) = (bin.to_string_lossy(), argv(&["--model", FLASH]));
    let (home, launch) = prepared(&binary, &extra, &input, homes, Path::new("/b")).unwrap();
    let invoked = invoke((&launch, &home), &input, "prompt", &bindings, &mut |_| {});
    assert!(invoked.is_ok(), "{:?}", invoked.map(drop));
    assert_eq!(world.recorded("home"), home.path());
}

/// Every string value of `value`, keys aside.
pub(super) fn strings(value: &Value) -> Vec<String> {
    match value {
        Value::String(text) => vec![text.clone()],
        Value::Array(items) => items.iter().flat_map(strings).collect(),
        Value::Object(map) => map.values().flat_map(strings).collect(),
        _ => Vec::new(),
    }
}

/// The seat's input in `world`, `value` bound to it as `AT` through a
/// store under the world's root, as the shared driver resolves a seat's
/// secrets.
pub(super) fn bound_input(world: &World, value: &str) -> Value {
    let store = world.path("secrets.env");
    std::fs::write(&store, format!("AT={value}\n")).unwrap();
    std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o600)).unwrap();
    let mut input = world.input(None);
    input["secrets"] = json!(["AT"]);
    input["secrets_file"] = json!(store);
    input
}

/// The words the shared driver writes itself, whatever any adapter's
/// stream said: no value of a seat's.
const DRIVER_WORDS: [&str; 2] = ["agy-cli-session-finished", "not reported"];

/// The shared driver's own lead before a refusal to start's words.
const REFUSED: &str = "provider refused before the first turn: ";

/// Every string a run sent that is not the shared driver's own word: each
/// checkpoint's values, and the result's error, refusal and payload.
pub(super) fn sent(messages: &[crate::Body]) -> Vec<String> {
    let texts = messages.iter().flat_map(|body| match body {
        crate::Body::Checkpoint { data, .. } => strings(data),
        crate::Body::Result {
            error,
            refusal,
            result,
            ..
        } => {
            let error = error
                .as_deref()
                .map(|error| error.strip_prefix(REFUSED).unwrap_or(error));
            let mut texts: Vec<String> = error.iter().map(|error| error.to_string()).collect();
            texts.extend(refusal.iter().map(|r| r.words().to_string()));
            texts.extend(result.iter().flat_map(strings));
            texts
        }
        _ => Vec::new(),
    });
    texts
        .filter(|text| !DRIVER_WORDS.contains(&text.as_str()))
        .collect()
}

/// A bound secret spelled as a key, a status, the pin or one letter
/// changes no verdict, and reads in clear nowhere the driver's output goes
/// (J2): the refusal and every row leave masked once, by the shared driver
/// (K10), and the failure's words and the answer by this one.
#[test]
fn a_bound_secret_of_any_spelling_changes_no_verdict_and_leaves_masked() {
    for value in ["denied_actions", "status", "SUCCESS", "model", FLASH, "d"] {
        let bindings = [bound("AT", value)];
        let mask = |words: &str| secret::mask_text(words.as_bytes(), &bindings);
        let denied = driven((DENIED_ACCEPT, "", 0), FLASH, &bindings);
        let failure = Failed {
            failure: Failure::Denied(argv(&["command"])),
            seen: fold::Seen {
                status: Some("SUCCESS".into()),
                timed_out: false,
            },
        };
        let words = mask(&failure.to_string());
        assert_eq!(denied.turn.as_ref().map(drop), Err(&words), "{value}");
        assert!(!words.contains(value), "{value}");
        let answered = driven((TOOL_LESS, "", 0), FLASH, &bindings);
        let answer = answered.turn.as_ref().unwrap().answer.clone();
        assert_eq!(answer, Some(json!(mask("READY\n"))), "{value}");
        let substituted = {
            let mut world = World::new();
            let bin = fake_agy(&world.seat(), NEON, "", false, 0);
            let input = bound_input(&world, value);
            world.run(&bin, &["--model", "gemini-4-carbon"], input, None)
        };
        let words = "model_substituted: the seat pinned \"gemini-4-carbon\" and agy served \
                     \"gemini-4-neon\"";
        assert_eq!(
            ended(&substituted).and_then(|(_, _, refusal)| refusal),
            Some(mask(words)),
            "{value}"
        );
        let ran = {
            let mut world = World::new();
            let bin = fake_agy(&world.seat(), TOOL_LESS, "", false, 0);
            let input = bound_input(&world, value);
            world.run(&bin, &["--model", FLASH], input, None)
        };
        assert!(finished(&ran).is_some(), "{value}");
        for messages in [&substituted, &ran] {
            let left = sent(messages);
            assert!(
                left.iter().all(|text| !text.contains(value)),
                "{value}: {left:?}"
            );
        }
    }
}

/// A `SUCCESS` result's row, answering.
const SUCCESS: &str = r#"{"event":"result","result":{"status":"SUCCESS","response":"READY\n"}}"#;

/// `rows` joined as one stream.
pub(super) fn stream(rows: &[&str]) -> String {
    let lines: Vec<&str> = rows.iter().map(|row| row.trim_end()).collect();
    lines.join("\n")
}

/// A denial is never lost: a `SUCCESS` behind it fails naming it, in the
/// fold and through the driver's whole path with a result delivered (J3).
#[test]
fn a_success_behind_a_denial_fails_naming_the_denial() {
    let behind = stream(&[DENIED_ACCEPT, SUCCESS]);
    let denial = Failed {
        failure: Failure::Denied(argv(&["command"])),
        seen: fold::Seen {
            status: Some("SUCCESS".into()),
            timed_out: false,
        },
    };
    let read = read(&behind, FLASH);
    assert_eq!(read.fold.verdict(0, false), Verdict::Failed(denial.clone()));
    {
        let mut world = World::new();
        let bin = fake_agy(&world.seat(), &behind, "", true, 0);
        let messages = world.run(&bin, &["--model", FLASH], world.input(None), None);
        // A failure behind work is no result: the attempt sends none.
        assert_eq!(ended(&messages), None);
    }
    let bindings = [bound("AT", "tok9z")];
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), &behind, "", true, 0);
    world.set(&bin);
    let input = world.input(None);
    let (binary, extra) = (bin.to_string_lossy(), argv(&["--model", FLASH]));
    let pinned = super::super::super::Pinned {
        binary: &binary,
        extra: &extra,
    };
    let turn = drive(pinned, false, "p", &input, &bindings, &mut |_| {}).map(drop);
    assert_eq!(turn, Err(denial.to_string()));
}

/// One launch is one turn, and a known row that does not type fails it; an
/// event the fold does not know, and a row that is no object, change
/// nothing (J3).
#[test]
fn a_turn_out_of_its_order_or_a_known_row_that_does_not_type_fails() {
    let lines: Vec<&str> = TOOL_LESS.lines().collect();
    let (init, step, result) = (lines[0], lines[1], lines[lines.len() - 1]);
    let verdict = |rows: &[&str]| read(stream(rows), FLASH).fold.verdict(0, false);
    // Each failure, and the status of a result folded before it.
    let ok = Some("SUCCESS");
    let cases: [(&[&str], Failure, Option<&str>); 8] = [
        (&[init, init, step, result], Failure::Reinitialized, None),
        (&[TOOL_LESS, step], Failure::Behind(Event::StepUpdate), ok),
        (&[TOOL_LESS, result], Failure::Behind(Event::Result), ok),
        (&[result, init], Failure::Behind(Event::Init), ok),
        (
            &[
                init,
                r#"{"event":"result","result":{"response":"READY\n"}}"#,
            ],
            Failure::Unread(Event::Result),
            None,
        ),
        (
            &[
                init,
                r#"{"event":"result","result":{"status":"SUCCESS","response":"x","denied_actions":"command"}}"#,
            ],
            Failure::Unread(Event::Result),
            None,
        ),
        (
            &[r#"{"event":"init","init":5}"#, TOOL_LESS],
            Failure::Unread(Event::Init),
            None,
        ),
        (
            &[init, r#"{"event":"step_update"}"#, step, result],
            Failure::Unread(Event::StepUpdate),
            None,
        ),
    ];
    for (rows, failure, status) in cases {
        let words = failure.to_string();
        assert_eq!(verdict(rows), failed(failure, status, false), "{words}");
    }
    let passed = [
        r#"{"event":"user","message":{}}"#,
        r#"{"event":7}"#,
        r#"{"no_event":"result"}"#,
        r#"{"event":{"init":1},"result":5}"#,
        r#"{"event":"result""#,
        r#"{"event""#,
        r#"{"x":"#,
        "{",
        "[1]",
        "\"result\"",
        "",
    ];
    let mut rows = vec![init];
    rows.extend(passed);
    rows.extend(&lines[1..]);
    assert_eq!(
        read(stream(&rows), FLASH).fold.verdict(0, false),
        Verdict::Answer("READY\n".into())
    );
    // A line that is not JSON at all, through the driver.
    let noisy = format!("not json\n{TOOL_LESS}");
    let answered = driven((&noisy, "", 0), FLASH, &[]);
    assert_eq!(answered.turn.unwrap().answer, Some(json!("READY\n")));
}

/// A signed-out turn's stderr is its sign-in block, which is no evidence:
/// none of it, and no piece of its URL, reaches anything the driver sends
/// or prints (J4).
#[test]
fn a_signed_out_turn_keeps_none_of_its_sign_in_block() {
    let signed_out = driven((SIGNED_OUT, SIGNED_OUT_STDERR, 1), FLASH, &[]);
    let invocation = signed_out.turn.unwrap();
    assert_eq!(invocation.stderr, SIGN_IN_WITHHELD);
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), SIGNED_OUT, SIGNED_OUT_STDERR, false, 1);
    let messages = world.run(&bin, &["--model", FLASH], world.input(None), None);
    let sent = format!("{messages:?}");
    let url = SIGNED_OUT_STDERR
        .split_whitespace()
        .find(|word| word.starts_with("https://"))
        .unwrap();
    // The attempt's own: its client, challenge and state, and its hosts.
    let pieces = url.split(|c: char| !c.is_ascii_alphanumeric());
    for piece in pieces.filter(|piece| piece.len() > 8) {
        assert!(!invocation.stderr.contains(piece), "{piece}");
        assert!(!sent.contains(piece), "{piece}");
    }
    for line in lines_of(SIGNED_OUT_STDERR) {
        assert!(!invocation.stderr.contains(&line), "{line}");
    }
}

/// Exit 3 is the print timeout whatever came before it, through the
/// driver's whole path: the recorded turn's finished response step makes it
/// a refusal behind work, which ends the seat with no result (J11).
#[test]
fn a_print_timeout_ends_as_its_refusal_whatever_rows_came_before() {
    let timeout = refusal_of(PRINT_TIMEOUT, 3);
    assert!(read(PRINT_TIMEOUT, FLASH).fold.worked());
    let after_answer = refusal_of(TOOL_LESS, 3);
    assert_eq!(
        after_answer.words(),
        format!("{PRINT_TIMEOUT_WORDS}; agy's result had status SUCCESS")
    );
    let timed_out = driven((PRINT_TIMEOUT, PRINT_TIMEOUT_STDERR, 3), FLASH, &[]);
    assert_eq!(timed_out.turn.map(drop), Err(timeout.words().to_string()));
    let mut world = World::new();
    let bin = fake_agy(&world.seat(), PRINT_TIMEOUT, PRINT_TIMEOUT_STDERR, false, 3);
    let messages = world.run(&bin, &["--model", FLASH], world.input(None), None);
    assert_eq!(ended(&messages), None, "no `agent CLI exited 3`");
}

/// A result that is not `SUCCESS` is named by its status, each the docs
/// list and any other word, bounded; what is no word is not named.
#[test]
fn a_status_that_is_not_success_is_named_by_it() {
    let init = TOOL_LESS.lines().next().unwrap();
    let long = "A".repeat(60);
    let statuses = [
        ("ERROR", "ERROR"),
        ("CANCELED", "CANCELED"),
        ("INTERRUPTED", "INTERRUPTED"),
        ("INVALID", "INVALID"),
        ("WAITING", "WAITING"),
        ("RUNNING", "RUNNING"),
        ("SOMETHING_NEW", "SOMETHING_NEW"),
        (&long, &long[..40]),
        ("rm -rf", "rm -rf"),
        ("a\tb\u{1b}", "a\\tb\\u{1b}"),
    ];
    for (status, name) in statuses {
        let result = json!({"event": "result", "result": {"status": status}});
        let verdict = read(stream(&[init, &result.to_string()]), FLASH)
            .fold
            .verdict(0, false);
        assert_eq!(
            verdict,
            failed(Failure::Status(name.into()), None, false),
            "{status}"
        );
    }
}

/// An option written with a value is named up to its `=` (J6).
#[test]
fn a_refused_option_is_named_and_never_quoted() {
    for (part, name) in [
        ("--model=x", "--model"),
        ("--no-such=secretvalue", "--no-such"),
    ] {
        let error = launch("agy", &argv(&["--model", FLASH, part]), &[]).unwrap_err();
        assert!(
            matches!(&error, LaunchError::Unadmitted(said) if said == name),
            "{error:?}"
        );
        assert!(!error.to_string().contains(&part[name.len()..]), "{error}");
    }
}

/// A rule's target is checked whole: blank, a blank at an end, a control
/// character or a parenthesis refuses, naming the control alone; several
/// words are admitted (`rule-multi-word-prefix`, J12).
#[test]
fn a_rule_is_checked_whole_and_may_hold_several_words() {
    let refused = [
        "command( )",
        "command( git)",
        "command(git )",
        "command(git\tstatus)",
        "command(git\u{7f})",
        "command(\u{0})",
        "command(a(b)",
        "mcp(a)b)",
    ];
    for rule in refused {
        let error = launch("agy", &argv(&["--model", FLASH, "--allow", rule]), &[]).unwrap_err();
        assert!(matches!(error, LaunchError::Untargeted), "{rule:?}");
        assert!(error.to_string().contains("--allow"));
    }
    let admitted = launch(
        "agy",
        &argv(&["--model", FLASH, "--allow", "command(git status)"]),
        &[],
    );
    assert_eq!(
        admitted.unwrap().command.last().unwrap(),
        r#"{"permissions":{"allow":["command(git status)"]}}"#
    );
}
