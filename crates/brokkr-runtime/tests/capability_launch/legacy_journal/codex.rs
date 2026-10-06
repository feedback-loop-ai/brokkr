//! U4f2's Codex leg of D9's native matrix, and its refusals (decision
//! 0065 slice two; CC1, CC2, CC3 and SC4): production's Codex and Claude
//! drivers, each in its own process, through compile, the engine, the
//! store's append fence, export and offline verify, in a realm that
//! grants `web-search` through `codex-native-search`.
//!
//! The Codex driver emits an observation on an item's start and on its
//! completion; the engine counts the call once and keeps a second item
//! its own call. A Claude fallback that replaced a refused Codex primary
//! holds nothing, so its search fails the attempt as unheld rather than
//! borrowing the primary's group, and a held call with no harness id fails
//! as unattributable rather than taking a guessed one.

use super::{
    adapters, compiled, drive, exports_and_verifies, nothing_private, rooted, script, serve_child,
    workspace, write, AdapterKind, Bundle, Engine, EventEnvelope, EventType, Path, PathBuf, Status,
    World,
};
use serde_json::{json, Value};

/// Set only by a wrapper's own command line: the arguments the engine
/// handed `driver codex` after its `--`, one a line.
const SERVE: &str = "BROKKR_CODEX_JOURNAL_SERVE";

/// The version the fake Codex reports.
const VERSION: &str = "0.154.0";

/// Not a test of its own: production's Codex driver, served when a
/// wrapper re-enters this binary with [`SERVE`] set.
#[test]
fn codex_driver_child() {
    serve_child(SERVE, AdapterKind::Codex);
}

/// The JSON events a Codex item writes on its start and its completion.
fn item(id: Option<&str>, kind: &str) -> String {
    let id = id.map_or(String::new(), |id| format!(r#""id":"{id}","#));
    ["item.started", "item.completed"]
        .map(|event| format!(r#"{{"type":"{event}","item":{{{id}"type":"{kind}"}}}}"#))
        .join("\n")
}

/// The `codex` one wrapper's driver launches. It answers the version
/// probe; a launch of `gpt-lost` writes the stream's rate-limit error
/// before any turn, a link that fails to start. Any other launch
/// announces a fresh thread, opens its turn, writes `items`, delivers the
/// first result its prompt allows and closes the turn.
fn fake_codex(root: &Path, tag: &str, items: &str) -> PathBuf {
    let path = root.join(format!("bin/codex-{tag}"));
    let count = root.join(format!("state/{tag}"));
    let items: String = items
        .lines()
        .map(|line| format!("printf '%s\\n' '{line}'\n"))
        .collect();
    script(
        &path,
        &format!(
            "#!/bin/sh\n\
             [ \"$1\" = --version ] && {{ printf 'codex-cli {VERSION}\\n'; exit 0; }}\n\
             printf '%s\\n' \"$*\" >> '{count}.argv'\n\
             model=\n\
             while [ $# -gt 0 ]; do\n\
             case \"$1\" in --model) model=$2; shift ;; esac\n\
             shift\n\
             done\n\
             prompt=$(cat)\n\
             [ \"$model\" = gpt-lost ] && {{ printf '{{\"type\":\"error\",\"message\":\"stream error: rate limit\"}}\\n'; exit 1; }}\n\
             n=$(cat '{count}' 2>/dev/null || echo 0)\n\
             printf '%s' $((n+1)) > '{count}'\n\
             printf '{{\"type\":\"thread.started\",\"thread_id\":\"{tag}-thread-%s\"}}\\n' \"$n\"\n\
             printf '{{\"type\":\"turn.started\"}}\\n'\n\
             {items}\
             result=$(printf '%s\\n' \"$prompt\" | grep -o '/[^ ]*/\\.forge/results/[^ ]*\\.json' | head -n 1)\n\
             word=$(printf '%s\\n' \"$prompt\" | grep -o 'one of: [a-z-]*' | head -n 1 | cut -d' ' -f3)\n\
             printf '{{\"result\":\"%s\",\"notes\":\"done\"}}' \"$word\" > \"$result\"\n\
             printf '{{\"type\":\"turn.completed\",\"usage\":{{\"input_tokens\":21,\"output_tokens\":5}}}}\\n'\n",
            count = count.display(),
        ),
    );
    path
}

/// The engine token of one `driver codex` command: a wrapper that drops
/// `driver codex --`, hands the rest to [`codex_driver_child`] with
/// `tag`'s harness, and passes on only protocol lines.
fn wrapper(root: &Path, tag: &str, items: &str) -> String {
    let codex = fake_codex(root, tag, items);
    let path = root.join(format!("bin/driver-codex-{tag}"));
    script(
        &path,
        &format!(
            "#!/bin/sh\n\
             shift 3\n\
             extra=$(printf '%s\\n' \"$@\")\n\
             BROKKR_CODEX_BIN='{codex}' HOME='{home}' CODEX_HOME='{home}/.codex' {SERVE}=\"$extra\" \
             exec '{exe}' --exact legacy_journal::codex::codex_driver_child --nocapture \
             | grep --line-buffered '^{{'\n",
            codex = codex.display(),
            home = root.join("home").display(),
            exe = std::env::current_exe().unwrap().display(),
        ),
    );
    path.to_str().unwrap().to_string()
}

/// An inline Codex site on `tag`'s wrapper, wanting `web-search`, whose
/// harness writes `items`.
fn inline(root: &Path, tag: &str, items: &str) -> Value {
    let command = [
        "driver",
        "codex",
        "--",
        "--model",
        "gpt-6-astra",
        "--effort",
        "high",
    ];
    let mut argv = vec![wrapper(root, tag, items)];
    argv.extend(command.map(str::to_string));
    json!({"role": "roles/role.md", "driver": {"command": argv},
           "capabilities": {"web-search": "wants"}})
}

/// The shipped Codex adapter, served through the `primary` wrapper with
/// no item of its own, and a model `lost` whose launch never starts;
/// beside it the shipped Claude adapter on the `agent` wrapper, whose
/// harness searches.
fn codex_adapter(root: &Path) {
    adapters(root);
    let shipped = workspace().join("adapters/codex.json");
    let mut codex: Value = serde_json::from_slice(&std::fs::read(shipped).unwrap()).unwrap();
    codex["driver"][0] = json!(wrapper(root, "primary", ""));
    codex["models"]["lost"] = json!("gpt-lost");
    write(root, "adapters/codex.json", &codex);
}

/// A run of `seats`, each stepping to the next and each answering
/// `complete`, then the protected review gate on its own quiet Codex.
/// The `switched` agent's Codex primary never starts, and its Claude
/// fallback serves.
fn bundle(root: &Path, world: &World, seats: &[(&str, Value)]) -> Bundle {
    codex_adapter(root);
    std::fs::create_dir_all(root.join("agents/charters")).unwrap();
    std::fs::write(
        root.join("agents/charters/office.md"),
        crate::charters::titled("office"),
    )
    .unwrap();
    write(
        root,
        "agents/switched.json",
        &json!({"description": "a searching office", "charter": "charters/office.md",
                "models": ["lost", "opus"], "efforts": {"lost": "high", "opus": "high"},
                "capabilities": {"web-search": "wants"}}),
    );
    std::fs::create_dir_all(root.join("bundle/roles")).unwrap();
    crate::charters::write_role(&root.join("bundle"));
    let mut body = serde_json::Map::new();
    let mut rules = Vec::new();
    let mut phases: Vec<&str> = seats.iter().map(|(seat, _)| *seat).collect();
    phases.extend(["review", "done"]);
    for (at, (seat, site)) in seats.iter().enumerate() {
        let mut site = site.clone();
        site["results"] = json!(["complete"]);
        body.insert(seat.to_string(), site);
        rules.push(json!({"id": format!("{seat}-complete"), "from": seat,
                          "result": "complete", "next": phases[at + 1], "reason": "r"}));
    }
    let mut review = inline(root, "review", "");
    review["results"] = json!(["clean"]);
    body.insert("review".into(), review);
    rules.push(
        json!({"id": "review-clean", "from": "review", "result": "clean",
                      "next": "done", "reason": "r"}),
    );
    write(
        root,
        "bundle/policy.json",
        &json!({"phases": phases, "initial": phases[0], "terminal": ["done"], "rules": rules}),
    );
    write(
        root,
        "bundle/bundle.json",
        &json!({"name": "codex-journal", "policy": "policy.json", "seats": body}),
    );
    compiled(root, world)
}

/// `seats` compiled in a realm that grants `web-search` through Codex's
/// native dialect and driven until the run completes or parks, under a
/// canonicalised temporary root.
fn driven(
    seats: impl FnOnce(&Path) -> Vec<(&'static str, Value)>,
) -> (tempfile::TempDir, PathBuf, Engine, Status) {
    let (dir, root) = rooted();
    let world = super::world(&root, "codex-native-search");
    let seats = seats(&root);
    let bundle = bundle(&root, &world, &seats);
    let (engine, state) = drive(&root, world, bundle, "Feature: codex");
    (dir, root, engine, state.status)
}

/// The checkpoints `seat`'s effect journaled, in order, with the
/// temporary root spelled `<root>` and both digest stamps proved
/// lowercase hex and then named. A call id that is the digest the design
/// spells for one of Codex's `calls`, owned by the row's own attempt and
/// site stamps, is spelled `<owned call>`; any other value stays to fail.
fn journaled(events: &[EventEnvelope], root: &Path, seat: &str, calls: &[&str]) -> Vec<Value> {
    let field = |event: &EventEnvelope, name: &str| event.payload[name].clone();
    let effect = events
        .iter()
        .find(|event| {
            event.event_type == EventType::EffectRequested && event.payload["seat"] == seat
        })
        .map(|event| field(event, "effect_id"));
    let rows = events.iter().filter(|event| {
        event.event_type == EventType::EffectCheckpointed
            && Some(field(event, "effect_id")) == effect
    });
    rows.map(|event| {
        let text = event.payload["checkpoint"].to_string();
        let mut row: Value =
            serde_json::from_str(&text.replace(root.to_str().unwrap(), "<root>")).unwrap();
        for call in calls {
            let tuple = json!({"attempt": field(event, "attempt_id"), "provider": "codex",
                               "site": row["site_ref"], "instance": row["instance_ref"],
                               "call": call});
            if row["call_id"] == format!("n-{}", brokkr_core::canonical::sha256_hex(&tuple)) {
                row["call_id"] = json!(format!("<owned {call}>"));
            }
        }
        for stamp in ["site_ref", "instance_ref"] {
            let digest = row[stamp].as_str().unwrap();
            let hex = digest
                .bytes()
                .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
            assert!(digest.len() == 64 && hex, "{stamp}: {digest}");
            row[stamp] = json!(stamp);
        }
        row
    })
    .collect()
}

/// `fields` beside the boundary, sentinels and named stamps every row of
/// these sites carries.
fn row(fields: Value) -> Value {
    let mut row = json!({"boundary": "not applicable", "effort": "not reported",
                         "model": "not reported", "site_ref": "site_ref",
                         "instance_ref": "instance_ref"});
    row.as_object_mut()
        .unwrap()
        .extend(fields.as_object().unwrap().clone());
    row
}

/// The rows a session of `harness` opens with on root `id`: its transcript
/// address under `home`, then its cold launch with the root confirmed.
fn opened(harness: &str, home: &str, kind: &str, id: &str, version: &str) -> Vec<Value> {
    let transcript = json!({"home": format!("<root>/home/{home}"), "kind": kind, "locator": id});
    let root = json!({"kind": kind, "id": id, "harness_version": version, "persistent": true});
    vec![
        row(json!({"step": "transcript", "transcript": transcript})),
        row(
            json!({"step": "harness-started", "harness": harness, "launch": "cold",
                   "root_session": root}),
        ),
    ]
}

/// The Codex thread `id` opening its first turn.
fn codex_turn(id: &str) -> Vec<Value> {
    let mut rows = opened("codex", ".codex", "codex-thread", id, VERSION);
    rows.push(row(
        json!({"step": "turn-started", "turn": 1, "harness": "codex"}),
    ));
    rows
}

/// The row of an item's `step` running `tool`, with the engine's group
/// where it observed `owned`.
fn item_row(step: &str, tool: &str, owned: Option<&str>) -> Value {
    let mut fields = json!({"step": step, "turn": 1, "tool": tool, "harness": "codex"});
    if let Some(call) = owned {
        fields["capability"] = json!("web-search");
        fields["dialect"] = json!("codex-native-search");
        fields["call_id"] = json!(format!("<owned {call}>"));
        fields["call_state"] = json!("observed");
    }
    row(fields)
}

/// No record of the run carries a response digest or an observation, its
/// whole journal exports, and the export verifies offline to `status`.
fn settles(engine: &Engine, events: &[EventEnvelope], status: Status) {
    nothing_private(events);
    exports_and_verifies(&engine.store, &engine.run_id, events.len(), status);
}

/// U4f2 through the shipped Codex driver (CC1, CC3, SC4): it observes a
/// held `web_search` item on its start and on its completion, and the
/// engine attributes the call once, to `codex-native-search` under the
/// item's own attempt-owned id; a second item is its own call, and a
/// local command stays ordinary on both its rows. Exactly two calls are
/// attributed, and the run exports and verifies.
#[test]
fn a_codex_items_start_and_completion_count_once_and_a_new_item_is_its_own_call() {
    let items = [
        item(Some("item_1"), "web_search"),
        item(Some("item_2"), "web_search"),
        item(Some("item_3"), "command_execution"),
    ]
    .join("\n");
    let (_dir, root, engine, status) =
        driven(|root| vec![("searching", inline(root, "searching", &items))]);
    assert_eq!(status, Status::Completed);
    let events = engine.store.load(&engine.run_id).unwrap();
    let mut wanted = codex_turn("searching-thread-0");
    for (call, tool, owned) in [
        ("item_1", "web_search", true),
        ("item_2", "web_search", true),
        ("item_3", "command_execution", false),
    ] {
        wanted.push(item_row("item-started", tool, owned.then_some(call)));
        wanted.push(item_row("item-completed", tool, None));
    }
    let usage = json!({"input_tokens": 21, "output_tokens": 5});
    let mut turn = row(json!({"step": "turn-completed", "turn": 1, "harness": "codex"}));
    turn.as_object_mut()
        .unwrap()
        .extend(usage.as_object().unwrap().clone());
    let mut finished = row(json!({"step": "codex-session-finished", "exit_code": 0,
                                  "num_turns": 1, "transcript": wanted[0]["transcript"]}));
    finished
        .as_object_mut()
        .unwrap()
        .extend(usage.as_object().unwrap().clone());
    wanted.extend([turn, finished]);
    let calls = ["item_1", "item_2", "item_3"];
    assert_eq!(journaled(&events, &root, "searching", &calls), wanted);
    let attributed: std::collections::BTreeSet<&str> = events
        .iter()
        .filter_map(|event| event.payload["checkpoint"]["call_id"].as_str())
        .collect();
    assert_eq!(attributed.len(), 2);
    settles(&engine, &events, Status::Completed);
}

/// CC1's and CC2's refusals through the shipped drivers. A Claude fallback
/// serving after its Codex primary failed to start holds nothing in a
/// realm that grants only Codex's dialect, so its search fails the
/// attempt as unheld: the primary's group is never borrowed. A held Codex
/// item with no harness id fails as unattributable, and no id is guessed.
/// Either way the refused call is never journaled, nothing is attributed,
/// and the parked run exports and verifies.
#[test]
fn an_unheld_or_unattributable_native_call_fails_its_attempt_through_the_shipped_drivers() {
    let site = |root: &Path, seat: &str| match seat {
        "switched" => json!({"agent": "switched", "limits": {"max_attempts": 2}}),
        _ => inline(root, seat, &item(None, "web_search")),
    };
    let claude = opened(
        "claude",
        ".claude/projects",
        "claude-session",
        "agent-root-0",
        super::VERSION,
    );
    let cases = [
        (
            "switched",
            claude,
            "observed capability tool 'WebSearch' is not held by this attempt",
        ),
        (
            "searching",
            codex_turn("searching-thread-0"),
            "capability telemetry cannot be attributed",
        ),
    ];
    for (seat, wanted, cause) in cases {
        let (_dir, root, engine, status) = driven(|root| vec![(seat, site(root, seat))]);
        assert_eq!(status, Status::AwaitingOperator, "{cause}");
        let events = engine.store.load(&engine.run_id).unwrap();
        assert_eq!(journaled(&events, &root, seat, &[]), wanted, "{cause}");
        let failed = events
            .iter()
            .rfind(|event| event.event_type == EventType::EffectFailed)
            .unwrap();
        assert_eq!(failed.payload["error"], format!("{cause}; stderr tail: "));
        settles(&engine, &events, Status::AwaitingOperator);
    }
}
