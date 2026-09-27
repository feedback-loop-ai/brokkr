//! The participant transcript routes (#352): the body route that serves
//! `brokkr transcript --json`'s document and the watch keyed like it, for
//! every kind. They share `tests.rs`'s journal and HTTP fixtures.

use super::tests::{
    claude_projects_home, exchange, participant_fixture, percent_encode, transcript_route,
    watch_request, BrokenWriter, FailAfterOneWrite,
};
use super::*;
use crate::tests::env_guard::EnvGuard;
use serde_json::json;
use std::io::Write;

/// The transcript drill (`/api/transcript/<run>/<key>`, #352) reads the
/// participant's own transcript through the command's one read path, or
/// says why it cannot: a refused read keeps its document under a 404. A
/// legacy Claude seat names only its flat id, which the route synthesizes
/// against the local projects root exactly as the command does.
#[test]
fn the_transcript_drill_reads_a_local_session_or_says_why_it_cannot() {
    let mut env = EnvGuard::lock();
    let drill = |id: &str| {
        let (_dir, db, key) = participant_fixture(None, Some("claude"), Some(id));
        transcript_route(&db, &key)
    };
    let unavailable = |response: Response| {
        assert_eq!(response.status, "404 Not Found", "{}", response.body);
        serde_json::from_str::<Value>(&response.body).unwrap()["unavailable"].clone()
    };

    // The id is validated STRICTLY, before any path is formed. The
    // journal's seat-record fence refuses an empty, traversing or
    // over-long id before it is recorded; one it records outside the
    // leading-hexadecimal language is refused here.
    assert_eq!(unavailable(drill("-abc")), "invalid-reference");

    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("home");
    let projects = home.join(".claude").join("projects");
    // Two project directories: the first does not hold the file, so the
    // scan must keep looking rather than conclude on the first miss.
    std::fs::create_dir_all(projects.join("empty-project")).unwrap();
    std::fs::create_dir_all(projects.join("real-project")).unwrap();

    let transcript = concat!(
        // Not JSON at all: skipped, never guessed at.
        "not json\n",
        // A record that is neither a user nor an assistant turn.
        "{\"type\":\"summary\"}\n",
        // A user turn with no message: no blocks, so no turn.
        "{\"type\":\"user\"}\n",
        // String content, with an explicit role and timestamp.
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",",
        "\"content\":\"the plain form\"},\"timestamp\":\"2026-01-01T00:00:00Z\"}\n",
        // Block content: prose, whitespace-only prose, a text block with
        // no text, a tool with a file target, a tool without one, and a
        // block kind this surface does not render.
        "{\"type\":\"user\",\"message\":{\"content\":[",
        "{\"type\":\"text\",\"text\":\"what happened\"},",
        "{\"type\":\"text\",\"text\":\"   \"},",
        "{\"type\":\"text\"},",
        "{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{\"file_path\":\"src/lib.rs\"}},",
        "{\"type\":\"tool_use\",\"name\":\"Bash\"},",
        "{\"type\":\"thinking\"}]}}\n",
    );
    std::fs::write(projects.join("real-project/abcd-1234.jsonl"), transcript).unwrap();

    // A file that is not valid UTF-8 is unreadable, not repaired.
    std::fs::write(projects.join("real-project/dead-beef.jsonl"), [0xff, 0xfe]).unwrap();

    // Past the size cap the response is truncated rather than unbounded.
    let mut oversized =
        String::from("{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"");
    oversized.push_str(&"x".repeat(4_000_001));
    oversized.push_str("\"}}\n");
    std::fs::write(projects.join("real-project/0000-1111.jsonl"), &oversized).unwrap();

    env.set("HOME", &home);

    let response = drill("abcd-1234");
    assert_eq!(response.status, "200 OK");
    let parsed: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(parsed["schema"], "brokkr.transcript/v1");
    assert_eq!(parsed["legacy"], true);
    assert_eq!(parsed["truncated"], false);
    let turns = parsed["turns"].as_array().unwrap();
    assert_eq!(turns.len(), 2, "two turns carried prose: {}", response.body);
    assert_eq!(turns[0]["role"], "assistant");
    assert_eq!(turns[0]["ts"], "2026-01-01T00:00:00Z");
    assert_eq!(turns[0]["blocks"][0]["text"], "the plain form");
    // The role falls back to the record type and the stamp to empty.
    assert_eq!(turns[1]["role"], "user");
    assert_eq!(turns[1]["ts"], "");
    let blocks = turns[1]["blocks"].as_array().unwrap();
    assert_eq!(blocks.len(), 3, "blank prose and unknown kinds drop out");
    assert_eq!(blocks[0]["kind"], "text");
    assert_eq!(blocks[0]["text"], "what happened");
    // Tool markers carry file targets only — never prose or commands.
    assert_eq!(blocks[1]["kind"], "tool");
    assert_eq!(blocks[1]["text"], "Read · src/lib.rs");
    assert_eq!(blocks[2]["text"], "Bash");

    let truncated = drill("0000-1111");
    let parsed: Value = serde_json::from_str(&truncated.body).unwrap();
    assert_eq!(parsed["truncated"], true, "the size cap holds");
    assert!(parsed["turns"].as_array().unwrap().is_empty());

    // Unreadable bytes are not a transcript, and a session that is not on
    // this machine is not found.
    assert_eq!(unavailable(drill("dead-beef")), "unreadable");
    assert_eq!(unavailable(drill("9999-9999")), "not-found");

    // No projects directory at all: the read says why, as the command's
    // does (decision 0073 ruling 4).
    env.set("HOME", dir.path().join("elsewhere"));
    assert_eq!(unavailable(drill("abcd-1234")), "not-found");

    // No HOME: there is nowhere to look, and nothing is invented.
    env.remove("HOME");
    assert_eq!(unavailable(drill("abcd-1234")), "no-reference");

    // A route that names no participant is refused before any read.
    let (_dir, db, _) = participant_fixture(None, None, None);
    for bad in ["r1", "r1/nobody", "r1/%zz", "nope/e1"] {
        let response = handle(&db, &format!("/api/transcript/{bad}"));
        assert_eq!(response.status, "404 Not Found", "{bad}");
        assert_eq!(response.body, "{\"error\":\"participant not found\"}");
    }
}

/// An unfoldable journal is fatal to its own verbs (`fleet.rs`), and the
/// drill is one of them: for a run whose journal loads and does not fold,
/// the body route, the watch and the presentation all answer the refusal
/// `brokkr transcript` exits with, in the fold's words, and serve no prose.
#[test]
fn an_unfoldable_journal_refuses_every_participant_route_as_the_command_does() {
    let mut env = EnvGuard::lock();
    let (home, projects) = claude_projects_home();
    std::fs::write(
        projects.join("seat/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"content\":\"the seat's prose\"}}\n",
    )
    .unwrap();
    env.set("HOME", home.path());
    let (_dir, db, key) = participant_fixture(None, Some("claude"), Some("abcd-1234"));
    assert_eq!(
        transcript_route(&db, &key).status,
        "200 OK",
        "readable first"
    );
    Store::open(&db)
        .unwrap()
        .append_next(
            "r1",
            brokkr_core::envelope::EventType::EffectStarted,
            json!({"effect_id": "never-requested", "attempt_id": "a2", "driver": "d"}),
            None,
            None,
        )
        .unwrap();

    let command = crate::run(crate::Cli {
        command: crate::Cmd::Transcript(crate::TranscriptArgs {
            run: "r1".into(),
            seat: key.clone(),
            turn: None,
            json: true,
            realms: None,
            db: Some(db.clone()),
        }),
    });
    let refusal = format!("{:#}", command.expect_err("the command refuses the run"));
    assert_eq!(
        refusal,
        "event 6: EffectStarted is impossible at cursor EffectInFlight { effect_id: \
         \"e1:seat\", attempt_id: \"a1\", seat: \"intake\", failed_attempts: 0 }"
    );
    let envelope = format!("\r\n\r\n{}", json!({"error": refusal}));
    let route = format!("r1/{}", percent_encode(&key));
    for path in ["/api/transcript/", "/sse/transcript/", "/api/presentation/"] {
        let request = format!("GET {path}{route} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
        let response = exchange(db.clone(), &request, Some(1));
        assert!(
            response.starts_with("HTTP/1.1 500 Internal Server Error"),
            "{path}: {response}"
        );
        assert!(response.ends_with(&envelope), "{path}: {response}");
    }
}

/// #380: the browser's session drill masks a bound value the model echoed
/// into its Claude session file against the store beside the journal, and
/// carries the reader's notices, which the page paints. A store emptied of
/// the name, an absent store and a store it cannot read mask nothing and
/// say so rather than passing silently.
#[test]
fn the_session_drill_masks_a_bound_value_and_says_what_it_covered() {
    let mut env = EnvGuard::lock();
    let (home, projects) = claude_projects_home();
    std::fs::write(
        projects.join("seat/abcd-1234.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\
         \"content\":\"echo says ghp-bound-7f3a9c\"}}\n",
    )
    .unwrap();
    env.set("HOME", home.path());
    let (_dir, db, key) = participant_fixture(None, Some("claude"), Some("abcd-1234"));
    let store = store_beside(&db);
    brokkr_protocol::secret::store_set(&store, "GH_TOKEN", "ghp-bound-7f3a9c").unwrap();
    let drill = || serde_json::from_str::<Value>(&transcript_route(&db, &key).body).unwrap();

    let response = transcript_route(&db, &key);
    assert_eq!(response.status, "200 OK");
    assert!(
        !response.body.contains("ghp-bound-7f3a9c"),
        "{}",
        response.body
    );
    let parsed: Value = serde_json::from_str(&response.body).unwrap();
    assert_eq!(
        parsed["turns"][0]["blocks"][0]["text"],
        "echo says [secret:GH_TOKEN]"
    );
    assert_eq!(
        parsed["notices"],
        json!([
            "secrets masked against the store's current values for GH_TOKEN; a value \
                rotated or removed since the run is not masked"
        ])
    );
    assert!(PAGE.contains("for (const notice of (view.body && view.body.notices) ?? [])"));

    // A store emptied of the name since the run, and a store that is not
    // there at all, mask nothing and say so, naming where they looked.
    let nothing_found = json!([format!(
        "secrets not masked: no values found in {}; a value bound from another store, or \
         removed since the run, is shown as written",
        store.display()
    )]);
    assert!(brokkr_protocol::secret::store_remove(&store, "GH_TOKEN").unwrap());
    let emptied = drill();
    assert_eq!(
        emptied["turns"][0]["blocks"][0]["text"],
        "echo says ghp-bound-7f3a9c"
    );
    assert_eq!(emptied["notices"], nothing_found);
    std::fs::remove_file(&store).unwrap();
    let absent = drill();
    assert_eq!(
        absent["turns"][0]["blocks"][0]["text"],
        "echo says ghp-bound-7f3a9c"
    );
    assert_eq!(absent["notices"], nothing_found);

    #[cfg(unix)]
    {
        brokkr_protocol::secret::store_set(&store, "GH_TOKEN", "ghp-bound-7f3a9c").unwrap();
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&store, std::fs::Permissions::from_mode(0o644)).unwrap();
        let parsed = drill();
        assert_eq!(
            parsed["turns"][0]["blocks"][0]["text"],
            "echo says ghp-bound-7f3a9c"
        );
        assert_eq!(
            parsed["notices"],
            json!([format!(
                "secrets not masked: refusing secrets store {}: permissions 644 are broader \
                 than 0600",
                store.display()
            )])
        );
    }
}

/// A drilled seat's prose lands BETWEEN journal checkpoints, so the
/// transcript gets a stream of its own, keyed by participant (#352): an
/// event when the file grew, a heartbeat when it did not, and — before a
/// single byte of stream — a clean 404 for a route that cannot name a
/// transcript, never a connection left open waiting for a file to appear.
#[test]
fn the_session_stream_fires_on_growth_and_says_nothing_otherwise() {
    let mut env = EnvGuard::lock();
    let (home, projects) = claude_projects_home();
    let file = projects.join("seat/abcd-1234.jsonl");
    let turn = "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\
                \"content\":\"a word\"}}\n";
    std::fs::write(&file, turn).unwrap();
    env.set("HOME", home.path());
    let (_dir, db, key) = participant_fixture(None, Some("claude"), Some("abcd-1234"));
    let (_gone, gone_db, gone_key) = participant_fixture(None, Some("claude"), Some("9999-9999"));
    let (_bad, bad_db, bad_key) = participant_fixture(None, Some("claude"), Some("-abc"));
    let route = format!("r1/{}", percent_encode(&key));

    // A malformed route, an unknown participant, an invalid reference, a
    // valid reference with no transcript behind it and a journal that is
    // not there all read the same way to the operator: nothing to watch.
    let missing_db = db.with_file_name("missing.db");
    for (db, route) in [
        (&db, "r1".to_string()),
        (&db, format!("{route}/extra")),
        (&db, "r1/nobody".to_string()),
        (&bad_db, format!("r1/{}", percent_encode(&bad_key))),
        (&gone_db, format!("r1/{}", percent_encode(&gone_key))),
        (&missing_db, route.clone()),
    ] {
        let response = exchange(db.clone(), &watch_request(&route), None);
        assert!(
            response.starts_with("HTTP/1.1 404 Not Found"),
            "{route}: {response}"
        );
        assert!(response.contains("transcript not found"), "{route}");
    }
    assert!(!missing_db.exists(), "a watch never creates a journal");

    // Three polls of a real stream, with the file mutated between them
    // by the writer itself so the timing is the test's, not the clock's.
    let mut request = std::io::Cursor::new(watch_request(&route).into_bytes());
    let mut watcher = GrowsThenVanishes {
        file: file.clone(),
        writes: 0,
        out: String::new(),
    };
    serve_io(&db, &mut request, &mut watcher, Some(3));
    let stream = watcher.out;
    assert!(stream.starts_with("HTTP/1.1 200 OK"), "{stream}");
    assert!(
        stream.contains("Content-Type: text/event-stream"),
        "{stream}"
    );
    assert_eq!(
        stream.matches("data: ").count(),
        1,
        "one append, one event — and none for the first look or the \
         poll that found the file gone: {stream}"
    );
    assert_eq!(
        stream.matches(": ping").count(),
        1,
        "the one poll that saw no new prose is a heartbeat; losing the \
         unique source closes the stream without another size: {stream}"
    );

    // A client that has already gone gets no stream: the header write
    // failing and the first message failing are both a clean return,
    // never a thread left polling a socket nobody is reading.
    std::fs::write(&file, turn).unwrap();
    let mut request = std::io::Cursor::new(watch_request(&route).into_bytes());
    serve_io(&db, &mut request, &mut BrokenWriter, Some(1));
    let mut request = std::io::Cursor::new(watch_request(&route).into_bytes());
    serve_io(
        &db,
        &mut request,
        &mut FailAfterOneWrite::default(),
        Some(1),
    );
}

/// The stream's clock, driven from the writer: the transcript gains a
/// turn while the first heartbeat is on the wire, and vanishes while the
/// growth event is. Both are things a live seat's file actually does.
struct GrowsThenVanishes {
    file: PathBuf,
    writes: usize,
    out: String,
}

impl Write for GrowsThenVanishes {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.out.push_str(&String::from_utf8_lossy(bytes));
        self.writes += 1;
        match self.writes {
            // 1 is the header. 2 is the first poll's heartbeat — new
            // prose lands before the next one.
            2 => {
                let mut file = std::fs::OpenOptions::new()
                    .append(true)
                    .open(&self.file)
                    .unwrap();
                file.write_all(b"{\"type\":\"user\",\"message\":{\"content\":\"more\"}}\n")
                    .unwrap();
            }
            // 3 is the growth event. Then the file goes away.
            3 => std::fs::remove_file(&self.file).unwrap(),
            _ => {}
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// The shared refusal mapping: every Claude lookup failure is a 404 on
/// both routes — the body route's document names the shared reason, and
/// the stream route writes the exact envelope before any stream header.
#[test]
fn lookup_refusals_are_the_exact_envelope_on_both_routes() {
    let mut env = EnvGuard::lock();
    let (home, projects) = claude_projects_home();
    // An unreadable body: the body route refuses it with the same envelope.
    std::fs::write(projects.join("seat/dead-beef.jsonl"), [0xff, 0xfe]).unwrap();
    // Two qualifying files: ambiguous-source.
    std::fs::write(
        projects.join("seat/aaaa-1111.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"content\":\"one\"}}\n",
    )
    .unwrap();
    std::fs::create_dir_all(projects.join("other")).unwrap();
    std::fs::write(
        projects.join("other/aaaa-1111.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"content\":\"two\"}}\n",
    )
    .unwrap();
    std::fs::write(
        projects.join("seat/a-bC09.jsonl"),
        "{\"type\":\"assistant\",\"message\":{\"content\":\"ok\"}}\n",
    )
    .unwrap();
    // A below-home symlink: unsafe-path.
    #[cfg(unix)]
    std::os::unix::fs::symlink(
        projects.join("seat/a-bC09.jsonl"),
        projects.join("other/baad-beef.jsonl"),
    )
    .unwrap();
    env.set("HOME", home.path());
    let seat = |id: &str| {
        let (dir, db, key) = participant_fixture(None, Some("claude"), Some(id));
        (dir, db, format!("r1/{}", percent_encode(&key)))
    };

    // The body route refuses every lookup and body failure with its reason.
    for (id, reason) in [
        ("dead-beef", "unreadable"),
        ("9999-9999", "not-found"),
        ("aaaa-1111", "ambiguous-source"),
        ("baad-beef", "unsafe-path"),
        ("-abc", "invalid-reference"),
    ] {
        let (_dir, db, route) = seat(id);
        let api = handle(&db, &format!("/api/transcript/{route}"));
        assert_eq!(api.status, "404 Not Found", "id {id}: {}", api.body);
        let document: Value = serde_json::from_str(&api.body).unwrap();
        assert_eq!(document["unavailable"], reason, "id {id}");
    }
    // The stream route refuses every lookup (it never reads a body) before
    // a stream header; an invalid id maps to the same transcript envelope.
    for id in ["9999-9999", "aaaa-1111", "baad-beef", "-abc"] {
        let (_dir, db, route) = seat(id);
        let sse = exchange(db, &watch_request(&route), None);
        assert!(sse.starts_with("HTTP/1.1 404 Not Found"), "id {id}: {sse}");
        assert!(
            sse.contains("{\"error\":\"transcript not found\"}"),
            "id {id}: {sse}"
        );
        assert!(
            !sse.contains("text/event-stream"),
            "a refusal writes no stream header: {sse}"
        );
    }

    // a-bC09 reads, and its stream opens.
    let (_dir, db, route) = seat("a-bC09");
    let api = handle(&db, &format!("/api/transcript/{route}"));
    assert_eq!(api.status, "200 OK", "{}", api.body);
    let parsed: Value = serde_json::from_str(&api.body).unwrap();
    assert_eq!(parsed["transcript"]["locator"], "a-bC09");
    assert_eq!(parsed["turns"][0]["blocks"][0]["text"], "ok");
    assert_eq!(parsed["truncated"], false);
    let sse = exchange(db, &watch_request(&route), Some(1));
    assert!(sse.starts_with("HTTP/1.1 200 OK"), "{sse}");
    assert!(sse.contains("Content-Type: text/event-stream"), "{sse}");
}

/// A body-stage refusal belongs to the body route (decision 0073 ruling 1,
/// the living spec's admission rule): for an admitted source whose bytes
/// are not UTF-8, the body route answers the command's `unreadable`
/// document under a 404, while the presentation, which reads no body,
/// keeps the source admitted and the watch, which sends sizes only, opens.
#[test]
fn a_body_stage_refusal_is_the_body_routes_alone() {
    let mut env = EnvGuard::lock();
    let (home, projects) = claude_projects_home();
    std::fs::write(projects.join("seat/dead-beef.jsonl"), [0xff, 0xfe]).unwrap();
    let rollouts = home.path().join("codex/sessions");
    std::fs::create_dir_all(&rollouts).unwrap();
    std::fs::write(rollouts.join("rollout-0199dead.jsonl"), [0xff, 0xfe]).unwrap();
    env.set("HOME", home.path());
    let codex = json!({"kind": "codex-thread", "locator": "0199dead",
                       "home": home.path().join("codex").to_str().unwrap()});
    for (reference, session) in [(None, Some("dead-beef")), (Some(codex), None)] {
        let (_dir, db, key) = participant_fixture(reference, session.map(|_| "claude"), session);
        let route = format!("r1/{}", percent_encode(&key));
        let body = handle(&db, &format!("/api/transcript/{route}"));
        assert_eq!(body.status, "404 Not Found", "{key}: {}", body.body);
        let document: Value = serde_json::from_str(&body.body).unwrap();
        assert_eq!(document["unavailable"], "unreadable", "{key}");
        let presentation: Value =
            serde_json::from_str(&handle(&db, &format!("/api/presentation/{route}")).body).unwrap();
        assert_eq!(
            (
                &presentation["admitted"],
                &presentation["reason"],
                &presentation["drill_eligible"]
            ),
            (&json!(true), &Value::Null, &json!(true)),
            "{key}: {presentation}"
        );
        let watch = exchange(db, &watch_request(&route), Some(1));
        assert!(watch.starts_with("HTTP/1.1 200 OK"), "{key}: {watch}");
        assert!(watch.ends_with("\r\n\r\n: ping\n\n"), "{key}: {watch}");
    }
}

/// A bounded growth watch reads the source and writes nothing: the
/// journal is untouched and the retained file keeps every byte.
#[test]
fn a_growth_watch_changes_no_retained_byte() {
    let mut env = EnvGuard::lock();
    let (home, projects) = claude_projects_home();
    let file = projects.join("seat/abcd-1234.jsonl");
    std::fs::write(
        &file,
        "{\"type\":\"assistant\",\"message\":{\"role\":\"assistant\",\"content\":\"a word\"}}\n",
    )
    .unwrap();
    env.set("HOME", home.path());
    let (_dir, db, key) = participant_fixture(None, Some("claude"), Some("abcd-1234"));
    let before = std::fs::read(&file).unwrap();
    let journal = std::fs::read(&db).unwrap();
    let route = format!("r1/{}", percent_encode(&key));
    let mut request = std::io::Cursor::new(watch_request(&route).into_bytes());
    let mut sink = Vec::new();
    serve_io(&db, &mut request, &mut sink, Some(2));
    assert!(String::from_utf8_lossy(&sink).starts_with("HTTP/1.1 200 OK"));
    assert_eq!(
        std::fs::read(&file).unwrap(),
        before,
        "a growth watch must read the source, never rewrite it"
    );
    assert_eq!(std::fs::read(&db).unwrap(), journal, "nor the journal");
}

/// #352's acceptance: the browser shows Codex and DSH transcripts. Over
/// the TCP shell, each kind's participant is admitted and eligible, its
/// body route answers with its turns, and its watch opens.
#[test]
fn the_browser_reads_codex_and_dsh_transcripts_over_http() {
    let _env = EnvGuard::lock();
    let homes = tempfile::tempdir().unwrap();
    let rollouts = homes.path().join("codex/sessions");
    std::fs::create_dir_all(&rollouts).unwrap();
    std::fs::write(
        rollouts.join("rollout-0199mine.jsonl"),
        "{\"type\":\"event_msg\",\"payload\":{\"type\":\"agent_message\",\"message\":\"codex says\"}}\n",
    )
    .unwrap();
    let seat = homes.path().join("dsh/sessions/one/project/seat");
    std::fs::create_dir_all(&seat).unwrap();
    std::fs::write(
        seat.join("session.jsonl"),
        "{\"type\":\"session\",\"version\":0}\n\
         {\"type\":\"user/message\",\"data\":{\"content\":\"dsh says\"},\"time\":1}\n",
    )
    .unwrap();
    for (kind, locator, text) in [
        ("codex-thread", "0199mine", "codex says"),
        ("dsh-session", "sessions/one", "dsh says"),
    ] {
        let home = homes.path().join(&kind[..kind.find('-').unwrap()]);
        let reference = json!({"kind": kind, "locator": locator, "home": home.to_str().unwrap()});
        let (_dir, db, key) = participant_fixture(Some(reference), None, None);
        let route = format!("r1/{}", percent_encode(&key));
        let get = |path: &str| {
            let request = format!("GET {path}{route} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n");
            exchange(db.clone(), &request, None)
        };
        let presentation = get("/api/presentation/");
        assert!(
            presentation.contains("\"drill_eligible\":true"),
            "{kind}: {presentation}"
        );
        let body = get("/api/transcript/");
        assert!(body.starts_with("HTTP/1.1 200 OK"), "{kind}: {body}");
        let document: Value = serde_json::from_str(body.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(document["turns"][0]["blocks"][0]["text"], text, "{kind}");
        let watch = exchange(db.clone(), &watch_request(&route), Some(1));
        assert!(
            watch.contains("Content-Type: text/event-stream"),
            "{kind}: {watch}"
        );
    }
}
