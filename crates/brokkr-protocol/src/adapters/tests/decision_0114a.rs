use super::*;
use crate::RefusalClass;

struct Stream {
    dir: tempfile::TempDir,
    shim: PathBuf,
    start: Value,
}

impl Stream {
    fn script(script: &str, secrets: &[(&str, &str)]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let shim = executable(dir.path(), "harness", script);
        let store = dir.path().join("secrets.env");
        for (name, value) in secrets {
            secret::store_set(&store, name, value).unwrap();
        }
        let start = json!({
            "effect_id":"effect", "attempt_id":"attempt",
            "input":{
                "workdir":dir.path(), "result_path":dir.path().join("result.json"),
                "allowed_results":["complete"], "feature":"f", "phase":"work",
                "role_path":"/dev/null", "secrets_file":store,
                "secrets":secrets.iter().map(|(name, _)| *name).collect::<Vec<_>>()
            },
        });
        Self { dir, shim, start }
    }

    fn rows(rows: &[Value], secrets: &[(&str, &str)]) -> Self {
        let rows = rows
            .iter()
            .map(Value::to_string)
            .collect::<Vec<_>>()
            .join("\n");
        Self::script(
            &format!("#!/bin/sh\ncat >/dev/null\ncat <<'ROWS'\n{rows}\nROWS\n"),
            secrets,
        )
    }

    fn run(&self, kind: AdapterKind, extra: &[String]) -> (Vec<Step>, Vec<Body>) {
        let mut bodies = Vec::new();
        let mut steps = Vec::new();
        run_seat_with(
            kind,
            Ok(()),
            &self.start,
            &mut |body| bodies.push(body),
            |prompt, input, bindings, emit| {
                let pinned = Pinned {
                    binary: self.shim.to_str().unwrap(),
                    extra,
                };
                invoke_with_stager(
                    kind,
                    pinned,
                    prompt,
                    input,
                    None,
                    bindings,
                    false,
                    &mut |item: &Emitted| {
                        steps.push(item.step);
                        emit(item);
                    },
                    stage_prompt,
                )
            },
        );
        (steps, bodies)
    }
}

fn wire_rows(bodies: &[Body]) -> impl Iterator<Item = &Value> {
    bodies.iter().filter_map(|body| match body {
        Body::Checkpoint { data, .. } => Some(data),
        _ => None,
    })
}

fn assert_refused(bodies: &[Body], words: &str) {
    let [Body::Result {
        status: ResultStatus::Failed,
        refusal: Some(refusal),
        result: None,
        ..
    }] = bodies
    else {
        panic!("expected one classified result: {bodies:?}");
    };
    assert_eq!(
        (
            refusal.class(),
            refusal.words(),
            refusal.retry_after_seconds()
        ),
        (RefusalClass::RateLimit, words, None),
    );
}

#[test]
fn bound_driver_step_keeps_the_refusal() {
    let mut env = EnvGuard::lock();
    let codex_home = tempfile::tempdir().unwrap();
    env.set("CODEX_HOME", codex_home.path());
    let codex = json!({"type":"turn.failed","error":{"codex_error_info":"usage_limit_exceeded","message":"quota exhausted"}});
    let claude =
        json!({"type":"rate_limit_event","apiErrorStatus":429,"message":"quota exhausted"});
    let cases = [
        (
            AdapterKind::Codex,
            vec![codex.clone()],
            "star",
            "rate_limit: quota exhausted",
        ),
        (
            AdapterKind::Claude,
            vec![claude],
            "star",
            "rate_limit (HTTP 429): quota exhausted",
        ),
        (
            AdapterKind::Codex,
            vec![
                json!({"type":"thread.started","thread_id":"thread-1234"}),
                codex,
            ],
            "scri",
            "rate_limit: quota exhausted",
        ),
    ];
    for (kind, rows, value, words) in cases {
        let (plain_steps, plain) = Stream::rows(&rows, &[]).run(kind, &[]);
        let (bound_steps, bound) = Stream::rows(&rows, &[("S", value)]).run(kind, &[]);
        assert_eq!(plain_steps, [Step::Transcript, Step::HarnessStarted]);
        assert_eq!(bound_steps, plain_steps);
        assert_refused(&plain, words);
        assert_refused(&bound, words);
    }

    let work = [
        json!({"type":"system","subtype":"init","session_id":"session-1234"}),
        json!({"type":"assistant","message":{"id":"m","content":[]}}),
    ];
    let (plain, _) = Stream::rows(&work, &[]).run(AdapterKind::Claude, &[]);
    assert_eq!(
        plain,
        [Step::Transcript, Step::HarnessStarted, Step::SeatTurn]
    );
    for (value, transcript, launch) in [
        ("star", "transcript", "harness-[secret:S]ted"),
        ("scri", "tran[secret:S]pt", "harness-started"),
    ] {
        let (bound, bodies) = Stream::rows(&work, &[("S", value)]).run(AdapterKind::Claude, &[]);
        assert_eq!(bound, plain);
        assert!(matches!(bodies.first(), Some(Body::Accepted { .. })));
        let mut rows = wire_rows(&bodies);
        assert_eq!(rows.next().unwrap()["step"], transcript);
        assert_eq!(rows.next().unwrap()["step"], launch);
        assert_eq!(rows.next().unwrap()["step"], "seat-turn");
    }
}

fn hold(stream: &Stream, kind: &'static str, offer: Option<&str>) -> LaunchHold {
    LaunchHold::new(
        kind,
        LaunchPlan {
            command: vec![stream.shim.to_string_lossy().into_owned()],
            rejoining: offer.map(str::to_string),
            refusal: None,
            sandbox: None,
            kind,
            harness_version: Some("1.2.3".into()),
            wrapper_digest: None,
            persistent: true,
            confirms_from_locator: true,
            effort: None,
        },
    )
}

fn bindings(stream: &Stream) -> Vec<secret::BoundSecret> {
    let names: Vec<String> =
        serde_json::from_value(stream.start["input"]["secrets"].clone()).unwrap();
    secret::resolve_bindings(&stream.dir.path().join("secrets.env"), &names).unwrap()
}

fn claude_id(event: Value) {
    for bound in [false, true] {
        let values = [("S", "1234")];
        let stream = Stream::rows(
            std::slice::from_ref(&event),
            if bound { &values } else { &[] },
        );
        let mut hold = hold(&stream, "claude-session", Some("session-1234"));
        let mut wire = Vec::new();
        let command = hold.plan.command.clone();
        let bindings = bindings(&stream);
        invoke_stream_json(
            &command,
            "prompt",
            stream.dir.path().to_str().unwrap(),
            &bindings,
            &mut hold,
            &mut ProbeAnswer::new(false),
            &mut |item| wire.push(seat_row(AdapterKind::Claude, &bindings, &item.row)),
        )
        .unwrap();
        assert_eq!(hold.terminal(), LaunchTerminal::Resumed);
        assert_eq!(
            wire[0]["transcript"]["locator"],
            if bound {
                "session-[secret:S]"
            } else {
                "session-1234"
            }
        );
        assert_eq!(wire[1]["launch"], "resumed");
        assert_eq!(
            wire[1].pointer("/root_session/id"),
            (!bound).then(|| &event["session_id"])
        );
    }
}

fn dsh_id(bound: bool) {
    let dir = tempfile::tempdir().unwrap();
    let locator = "seat-1234";
    let root = dir.path().join(locator);
    let id = "session-1234";
    plant_dsh_session(dir.path(), locator, "--w--", id, 27);
    let launch = DshLaunch {
        locator: locator.into(),
        ..dsh_stream_launch(Path::new("/dev/null"), &root, Some(id), Some(27))
    };
    let mut watch = DshRootWatch::new(&launch, Transcript::dsh(&launch.home));
    let mut hold = LaunchHold::new("deepseek", launch.plan());
    let values = [("S", "1234")];
    let stream = Stream::rows(&[], if bound { &values } else { &[] });
    let bindings = bindings(&stream);
    let mut wire = Vec::new();
    let mut meta = Map::new();
    let mut emit = |item: &Emitted| wire.push(seat_row(AdapterKind::Dsh, &bindings, &item.row));
    let init = json!({"type":"system","subtype":"init","session_id":id});
    assert_eq!(
        fold_dsh_line(
            init.to_string(),
            &mut watch,
            &mut hold,
            &mut meta,
            &bindings,
            &mut emit
        ),
        DshStreamLine::Event
    );
    plant_dsh_session(dir.path(), locator, "--w--", id, 28);
    watch.settle(
        DshStreamLine::End,
        &mut hold,
        &mut meta,
        &mut emit,
        &mut |_| {},
    );
    assert_eq!(hold.terminal(), LaunchTerminal::Resumed);
    assert_eq!(
        meta["session_id"],
        if bound { "session-[secret:S]" } else { id }
    );
    assert_eq!(
        wire[0]["transcript"]["locator"],
        if bound { "seat-[secret:S]" } else { locator }
    );
    assert_eq!(wire[1]["launch"], "resumed");
    assert_eq!(
        wire[1].pointer("/root_session/id").and_then(Value::as_str),
        (!bound).then_some(id)
    );
}

fn codex_id(home: &Path, bound: bool) {
    let sessions = home.join("sessions");
    std::fs::create_dir_all(&sessions).unwrap();
    for (file, model) in [
        ("rollout-first-thread-12345.jsonl", "wrong-thread"),
        ("rollout-second-thread-1234.jsonl", "from-rollout"),
    ] {
        let line = json!({"type":"turn_context","payload":{"model":model}});
        std::fs::write(sessions.join(file), format!("{line}\n")).unwrap();
    }
    let id = "thread-1234";
    let values = [("S", "1234")];
    let stream = Stream::rows(
        &[json!({"type":"thread.started","thread_id":id})],
        if bound { &values } else { &[] },
    );
    let bindings = bindings(&stream);
    let mut hold = hold(&stream, "codex-thread", None);
    let mut rows = Vec::new();
    let done = invoke_codex(
        None,
        "prompt",
        stream.dir.path().to_str().unwrap(),
        &bindings,
        &mut hold,
        &mut ProbeAnswer::new(false),
        &mut |item| rows.push(seat_row(AdapterKind::Codex, &bindings, &item.row)),
    )
    .unwrap();
    assert_eq!(done.session_meta["model"], "from-rollout");
    assert_eq!(
        rows[0]["transcript"]["locator"],
        if bound { "thread-[secret:S]" } else { id }
    );
    assert_eq!(rows[1]["launch"], "cold");
    assert_eq!(
        rows[1].pointer("/root_session/id").and_then(Value::as_str),
        (!bound).then_some(id)
    );
}

#[test]
fn bound_id_part_confirms_by_raw_ident() {
    let mut env = EnvGuard::lock();
    let home = tempfile::tempdir().unwrap();
    env.set("HOME", home.path());
    env.set("DSH_HOME", home.path());
    env.set("CODEX_HOME", home.path());
    claude_id(json!({"type":"system","subtype":"init","session_id":"session-1234"}));
    claude_id(json!({"type":"result","session_id":"session-1234"}));
    for bound in [false, true] {
        dsh_id(bound);
        codex_id(home.path(), bound);
    }
}

#[test]
fn bound_exec_stdout_leaves_only_masked_notes() {
    let mut stream = Stream::script(
        "#!/bin/sh\nprintf 'before %s after\\n' \"$S\"\n",
        &[("S", "private-value")],
    );
    stream.start["input"]["dialect_exec"] = json!({"success_result":"complete"});
    let (steps, bodies) = stream.run(
        AdapterKind::Exec,
        &[stream.shim.to_str().unwrap().to_owned()],
    );
    let Some(Body::Result {
        status: ResultStatus::Succeeded,
        result: Some(result),
        ..
    }) = bodies.last()
    else {
        panic!("expected delivered exec notes: {bodies:?}");
    };
    assert_eq!(steps, [Step::Transcript, Step::ExecStarted]);
    assert_eq!(result["notes"], "before [secret:S] after");
}
