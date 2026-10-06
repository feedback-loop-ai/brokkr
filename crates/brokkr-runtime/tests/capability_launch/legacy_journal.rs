//! D9's native legacy boundary matrix (decision 0065 slice two, U4a–U4f;
//! CC1, CC3 and SC4): before any normalized emission, a legacy native
//! record travels the whole production path — compile with a realm that
//! grants `web-search` through `claude-native-search`, the shipped Claude
//! driver in its own process, the engine, the store's append fence,
//! export and offline verify — at every executable site shape, and
//! arrives exactly as the shipped lowering wrote it plus the engine's own
//! stamps, with no attribution group and no private observation. Each U4
//! merge reruns it.
//!
//! The driver is production's `adapters::serve` for the Claude kind, run
//! as this test binary re-entered through [`claude_driver_child`]; the
//! harness it launches is a deterministic shell `claude` that writes the
//! stream-json a native search call and a local read produce. So the
//! shipped telemetry lowering is what writes every seat row, and no
//! provider need be installed.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use brokkr_core::envelope::{EventEnvelope, EventType};
use brokkr_core::fold::Status;
use brokkr_core::realms::Boundary;
use brokkr_protocol::adapters::capability_calls::{Format, Observation, Tool, OBSERVATION_KEY};
use brokkr_protocol::adapters::AdapterKind;
use brokkr_runtime::capabilities::CapabilityContext;
use brokkr_runtime::{Bundle, Engine, World};
use brokkr_store::{SeatRecordError, Store, StoreError};
use serde_json::{json, Value};

use super::charters::{write_charter, write_role};
use super::{workspace, write};

/// Set only by a wrapper's own command line, never by this process: the
/// arguments the engine handed `driver claude` after its `--`, one a line.
const SERVE: &str = "BROKKR_LEGACY_JOURNAL_SERVE";

/// The fields SC4 reserves for v6's attribution group, and protocol's
/// private observation key, which a driver could leak before its consumer
/// exists. None may reach the journal at a preparation merge.
const UNRECORDED: [&str; 6] = [
    "capability",
    "dialect",
    "call_id",
    "call_state",
    "response_sha256",
    OBSERVATION_KEY,
];

const V6: &str = "contracts/seat-record.v6.schema.json";

/// The version the fake harness reports, and the fixture assessment
/// qualifies against.
const VERSION: &str = "2.1.266";

/// Not a test of its own: production's Claude driver, served when a
/// wrapper re-enters this binary with [`SERVE`] set. It exits at once so
/// nothing of libtest's follows the protocol on stdout.
#[test]
fn claude_driver_child() {
    let Ok(extra) = std::env::var(SERVE) else {
        return;
    };
    let extra = extra.lines().map(str::to_string).collect();
    let served = brokkr_protocol::adapters::serve(AdapterKind::Claude, extra);
    std::process::exit(if served.is_ok() { 0 } else { 70 });
}

/// Write an executable script beside its target and rename it in, so no
/// writer still holds the file a spawn executes.
fn script(path: &Path, body: &str) {
    use std::os::unix::fs::PermissionsExt;
    let staged = path.with_extension("staged");
    std::fs::write(&staged, body).unwrap();
    std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755)).unwrap();
    std::fs::rename(&staged, path).unwrap();
}

/// The `claude` one wrapper's driver launches. It answers the version
/// probe and keeps every other command line it is given; a launch of
/// `claude-missing` writes a provider refusal before any turn, which is a
/// link that fails to start (decisions 0016 and 0053). Any other launch confirms
/// its root — the one `--resume` names, else a fresh one per invocation —
/// then makes a native search call; a `retried` seat's first invocation
/// fails there, mid-session; every other launch then reads a local file
/// and delivers the first result its prompt allows.
fn fake_claude(root: &Path, tag: &str, retried: bool) -> PathBuf {
    let path = root.join(format!("bin/claude-{tag}"));
    let count = root.join(format!("state/{tag}"));
    let fail_first = if retried {
        "[ \"$n\" = 0 ] && exit 1\n"
    } else {
        ""
    };
    script(
        &path,
        &format!(
            "#!/bin/sh\n\
             [ \"$1\" = --version ] && {{ printf '{VERSION} (Claude Code)\\n'; exit 0; }}\n\
             printf '%s\\n' \"$*\" >> '{count}.argv'\n\
             model= resume=\n\
             while [ $# -gt 0 ]; do\n\
             case \"$1\" in --model) model=$2; shift ;; --resume) resume=$2; shift ;; esac\n\
             shift\n\
             done\n\
             prompt=$(cat)\n\
             [ \"$model\" = claude-missing ] && {{ printf '{{\"type\":\"result\",\"is_error\":true,\"error\":\"not_found_error\"}}\\n'; exit 1; }}\n\
             n=$(cat '{count}' 2>/dev/null || echo 0)\n\
             printf '%s' $((n+1)) > '{count}'\n\
             session=${{resume:-{tag}-root-$n}}\n\
             printf '{{\"type\":\"system\",\"subtype\":\"init\",\"session_id\":\"%s\"}}\\n' \"$session\"\n\
             printf '{{\"type\":\"assistant\",\"message\":{{\"model\":\"%s\",\"usage\":{{\"input_tokens\":13,\"output_tokens\":2}},\"content\":[{{\"type\":\"tool_use\",\"id\":\"toolu_01\",\"name\":\"WebSearch\",\"input\":{{\"query\":\"brokkr\"}}}}]}}}}\\n' \"$model\"\n\
             {fail_first}\
             printf '{{\"type\":\"assistant\",\"message\":{{\"model\":\"%s\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"toolu_02\",\"name\":\"Read\",\"input\":{{\"file_path\":\"src/lib.rs\"}}}}]}}}}\\n' \"$model\"\n\
             result=$(printf '%s\\n' \"$prompt\" | grep -o '/[^ ]*/\\.forge/results/[^ ]*\\.json' | head -n 1)\n\
             word=$(printf '%s\\n' \"$prompt\" | grep -o 'one of: [a-z-]*' | head -n 1 | cut -d' ' -f3)\n\
             printf '{{\"result\":\"%s\",\"notes\":\"done\"}}' \"$word\" > \"$result\"\n\
             printf '{{\"type\":\"result\",\"subtype\":\"success\",\"session_id\":\"%s\",\"num_turns\":2}}\\n' \"$session\"\n",
            count = count.display(),
        ),
    );
    path
}

/// The engine token of one `driver claude` command: a wrapper that
/// drops `driver claude --`, hands the rest to [`claude_driver_child`]
/// with `tag`'s harness, and passes on only protocol lines, because
/// libtest owns the head of stdout. Where [`OBSERVING`] is written under
/// the root, those lines then pass through it.
fn wrapper(root: &Path, tag: &str, retried: bool) -> String {
    let claude = fake_claude(root, tag, retried);
    let path = root.join(format!("bin/driver-{tag}"));
    script(
        &path,
        &format!(
            "#!/bin/sh\n\
             shift 3\n\
             extra=$(printf '%s\\n' \"$@\")\n\
             serve() {{ BROKKR_CLAUDE_BIN='{claude}' HOME='{home}' {SERVE}=\"$extra\" exec '{exe}' \
             --exact legacy_journal::claude_driver_child --nocapture | grep --line-buffered '^{{'; }}\n\
             [ -e '{observing}' ] || {{ serve; exit; }}\n\
             serve | while IFS= read -r line; do printf '%s\\n' \"$line\" | sed -f '{observing}'; done\n",
            claude = claude.display(),
            home = root.join("home").display(),
            exe = std::env::current_exe().unwrap().display(),
            observing = root.join(OBSERVING).display(),
        ),
    );
    path.to_str().unwrap().to_string()
}

/// The sed script, under the root, that a wrapper passes each of the
/// shipped driver's protocol lines through, one at a time, when it is
/// written.
const OBSERVING: &str = "observing.sed";

/// What U4f2's serializer will add beside each of the fake harness's two
/// calls, until then injected by the wrapper: the observation protocol
/// encodes, and on the search a forged call id, state and response digest.
const OBSERVATIONS: &str = concat!(
    r#"s/"tool":"WebSearch"/&,"call_id":"forged","call_state":"succeeded","#,
    r#""response_sha256":"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef","#,
    r#""observation":"#,
    r#"{"format":"claude","call":"toolu_01","tool":{"kind":"named","name":"WebSearch"}}/"#,
    "\n",
    r#"s/"tool":"Read"/&,"observation":"#,
    r#"{"format":"claude","call":"toolu_02","tool":{"kind":"named","name":"Read"}}/"#,
    "\n",
);

/// The shipped Claude adapter served through the `agent` wrapper, with a
/// model whose launch never starts, and a supported assessment for the
/// one Claude shape at this unboxed site: the shipped one is unmeasured,
/// so an eligible rejoin is reached only by declaring one.
fn adapters(root: &Path) {
    let shipped = workspace().join("adapters/claude.json");
    let mut claude: Value = serde_json::from_slice(&std::fs::read(shipped).unwrap()).unwrap();
    claude["driver"][0] = json!(wrapper(root, "agent", false));
    claude["models"]["missing"] = json!("claude-missing");
    claude["resume"] = json!({"boxed-workspace": {
        "status": "supported", "identity": {"version": VERSION, "applies_to": VERSION},
        "classes": ["work"], "boundaries": ["not applicable"], "hands": "none",
        "evidence": {"interface": "fixture", "restrictions": "fixture",
                     "root": "fixture", "accounting": "fixture"}}});
    write(root, "adapters/claude.json", &claude);
}

/// The world whose one realm, `private` over `work/`, grants `web-search`
/// through Claude's native dialect, with the shipped definitions it
/// resolves against.
fn world(root: &Path) -> World {
    for shipped in [
        "capabilities/web-search.json",
        "capabilities/web-fetch.json",
        "dialects/tools/claude-native-search.json",
    ] {
        let body = std::fs::read(workspace().join(shipped)).unwrap();
        write(root, shipped, &serde_json::from_slice(&body).unwrap());
    }
    write(
        root,
        "realms.json",
        &json!({"schema": "forge.realms/v6", "journal": "forge.db", "realms": [
            {"name": "private", "path": root.join("work"), "default_branch": "main",
             "capabilities": {"web-search": {"dialect": "claude-native-search"}}}]}),
    );
    World::load(&root.join("realms.json")).unwrap()
}

/// The seats, one per site shape, each stepping to the next: an
/// agent-backed single seat, an inline one, an agent whose primary never
/// starts, a panel and a sequence each with one inline site and one
/// agent site, two inline seats retried after a mid-session failure —
/// one whose root persists, one launched without persistence — and the
/// protected review gate every policy keeps.
const SEATS: [&str; 8] = [
    "ordinary", "inline", "fallback", "panel", "sequence", "resumed", "replaced", "review",
];

/// The results a seat declares; the harness answers with the first.
fn results_of(seat: &str) -> &'static [&'static str] {
    match seat {
        "panel" => &["pass", "fail"],
        "review" => &["clean"],
        _ => &["complete"],
    }
}

/// An inline Claude site on `tag`'s wrapper, holding `web-search`.
fn inline(root: &Path, tag: &str, extra: &[&str]) -> Value {
    let retried = matches!(tag, "resumed" | "replaced");
    let mut command = vec![wrapper(root, tag, retried)];
    command.extend(
        [
            "driver",
            "claude",
            "--",
            "--model",
            "claude-opus-5-5",
            "--effort",
            "high",
        ]
        .iter()
        .chain(extra)
        .map(|word| word.to_string()),
    );
    let mut site = json!({"role": "roles/role.md", "driver": {"command": command},
                          "capabilities": {"web-search": "wants"}});
    if retried {
        site["limits"] = json!({"max_attempts": 2});
    }
    site
}

fn bundle(root: &Path, world: &World) -> Bundle {
    adapters(root);
    std::fs::create_dir_all(root.join("agents/charters")).unwrap();
    write_charter(&root.join("agents/charters/office.md"));
    for (agent, models) in [
        ("office", json!(["opus"])),
        ("chain", json!(["missing", "opus"])),
    ] {
        let efforts: serde_json::Map<String, Value> = models
            .as_array()
            .unwrap()
            .iter()
            .map(|model| (model.as_str().unwrap().to_string(), json!("high")))
            .collect();
        write(
            root,
            &format!("agents/{agent}.json"),
            &json!({"description": "a searching office", "charter": "charters/office.md",
                    "models": models, "efforts": efforts,
                    "capabilities": {"web-search": "wants"}}),
        );
    }
    write_role(&root.join("bundle"));
    let mut step = inline(root, "first", &[]);
    step["name"] = json!("first");
    step["results"] = json!(["complete"]);
    let mut seats = json!({
        "ordinary": {"agent": "office"},
        "inline": inline(root, "inline", &[]),
        "fallback": {"agent": "chain", "limits": {"max_attempts": 2}},
        "panel": {"aggregate": "unanimous-pass",
                  "panel": {"member": inline(root, "member", &[]), "agent": {"agent": "office"}}},
        "sequence": {"sequence": [step, {"name": "then", "agent": "office"}]},
        "resumed": inline(root, "resumed", &[]),
        "replaced": inline(root, "replaced", &["--no-session-persistence"]),
        "review": inline(root, "review", &[]),
    });
    let mut rules: Vec<Value> = Vec::new();
    for (at, seat) in SEATS.iter().enumerate() {
        let next = SEATS.get(at + 1).copied().unwrap_or("done");
        seats[seat]["results"] = json!(results_of(seat));
        for result in results_of(seat) {
            rules.push(json!({"id": format!("{seat}-{result}"), "from": seat,
                              "result": result, "next": next, "reason": "r"}));
        }
    }
    let mut phases: Vec<&str> = SEATS.to_vec();
    phases.push("done");
    write(
        root,
        "bundle/policy.json",
        &json!({"phases": phases, "initial": "ordinary", "terminal": ["done"], "rules": rules}),
    );
    write(
        root,
        "bundle/bundle.json",
        &json!({"name": "legacy-journal", "policy": "policy.json", "seats": seats}),
    );
    let context = CapabilityContext {
        realm: "private".into(),
        grants: world.map.realms[0].grants.clone(),
        root: root.to_path_buf(),
    };
    Bundle::compile_with_capabilities(
        &root.join("bundle"),
        &root.join("agents"),
        &root.join("adapters"),
        Some("private"),
        None,
        Boundary::Namespace,
        &context,
    )
    .unwrap_or_else(|refusal| panic!("the legacy matrix compiles: {refusal}"))
}

/// What a site wrote, keyed by its seat and the member or step that
/// wrote it (`""` for a single seat and for a sequence's own rows):
/// panel members run side by side, so only each site's order is fixed.
type Sites = BTreeMap<(String, String), Vec<Value>>;

/// Every checkpoint the run journaled, by site, with the engine's two
/// digest stamps (proposed decision 0056 ruling 1) proved lowercase hex
/// and then named rather than spelled: one hashes this machine's command.
/// The temporary root a transcript address names is spelled `<root>`, and
/// the attempt that owns an attributed call id `<attempt>`.
fn journaled(events: &[EventEnvelope], root: &Path) -> Sites {
    let field =
        |event: &EventEnvelope, name: &str| event.payload[name].as_str().unwrap().to_string();
    let seats: BTreeMap<String, String> = events
        .iter()
        .filter(|event| event.event_type == EventType::EffectRequested)
        .map(|event| (field(event, "effect_id"), field(event, "seat")))
        .collect();
    let mut sites = Sites::new();
    for event in events {
        if event.event_type != EventType::EffectCheckpointed {
            continue;
        }
        let text = event.payload["checkpoint"].to_string();
        let text = text.replace(root.to_str().unwrap(), "<root>");
        let mut checkpoint: Value = serde_json::from_str(&text).unwrap();
        // The call id the design spells for the one search, owned by this
        // row's attempt and own site stamps; any other value stays to fail.
        if let Some(call_id) = checkpoint.get("call_id").and_then(Value::as_str) {
            let tuple = json!({"attempt": field(event, "attempt_id"), "provider": "claude",
                               "site": checkpoint["site_ref"], "call": "toolu_01",
                               "instance": checkpoint["instance_ref"]});
            let owned = format!("n-{}", brokkr_core::canonical::sha256_hex(&tuple));
            if call_id == owned {
                checkpoint["call_id"] = json!("<owned toolu_01>");
            }
        }
        for stamp in ["site_ref", "instance_ref"] {
            if let Some(digest) = checkpoint.get(stamp).and_then(Value::as_str) {
                let hex = digest
                    .bytes()
                    .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
                assert!(digest.len() == 64 && hex, "{stamp}: {digest}");
                checkpoint[stamp] = json!(stamp);
            }
        }
        let member = checkpoint["member"]
            .as_str()
            .unwrap_or_default()
            .to_string();
        let seat = seats[&field(event, "effect_id")].clone();
        sites.entry((seat, member)).or_default().push(checkpoint);
    }
    sites
}

const OPUS: &str = "claude-opus-5-5";

/// The rows one Claude invocation journals, written out here and not
/// derived from the harness's stream: the transcript address as the
/// session opens, the launch row with the root it confirmed, the native
/// search turn with its usage, then — if the invocation finishes — the
/// local read and the address again, and the session's own close. Every
/// row carries the site's boundary and both stamps; a member's or step's
/// tag goes beside.
fn session(member: &str, root: &str, launch: &str, persistent: bool, done: bool) -> Vec<Value> {
    let transcript = json!({"home": "<root>/home/.claude/projects", "kind": "claude-session",
                            "locator": root});
    let address = json!({"step": "transcript", "model": "not reported",
                         "transcript": transcript});
    let mut rows = vec![
        address.clone(),
        json!({"step": "harness-started", "harness": "claude", "launch": launch,
               "model": "not reported",
               "root_session": {"kind": "claude-session", "id": root,
                                "harness_version": VERSION, "persistent": persistent}}),
        json!({"step": "seat-turn", "turn": 1, "model": OPUS, "tool": "WebSearch",
               "input_tokens": 13, "output_tokens": 2}),
    ];
    if done {
        rows.push(
            json!({"step": "seat-turn", "turn": 2, "model": OPUS, "tool": "Read",
                         "target": "src/lib.rs"}),
        );
        rows.push(address);
    }
    let turns = if done { 2 } else { 1 };
    rows.push(
        json!({"step": "claude-code-session-finished", "model": OPUS,
                     "exit_code": i32::from(!done), "num_turns": turns,
                     "input_tokens": 13, "output_tokens": 2, "transcript": transcript}),
    );
    for row in &mut rows {
        row["effort"] = json!("not reported");
        row["boundary"] = json!("not applicable");
        row["site_ref"] = json!("site_ref");
        row["instance_ref"] = json!("instance_ref");
        if !member.is_empty() {
            row["member"] = json!(member);
        }
    }
    rows
}

/// The journal D9 expects at a preparation merge: the shipped lowering's
/// legacy rows exactly, at every site, with the engine's own panel and
/// sequence rows; the persistent root rejoined, the other one replaced.
/// The fallback's first link was refused before its first turn, so it
/// journals nothing and its served link opens the next `agent` root.
fn wanted() -> Sites {
    let mut sites = Sites::new();
    let mut site = |seat: &str, member: &str, rows: Vec<Value>| {
        sites.insert((seat.to_string(), member.to_string()), rows);
    };
    let whole = |member: &str, root: &str| session(member, root, "cold", true, true);
    site("ordinary", "", whole("", "agent-root-0"));
    site("inline", "", whole("", "inline-root-0"));
    site("fallback", "", whole("", "agent-root-1"));
    for (member, root) in [("member", "member-root-0"), ("agent", "agent-root-2")] {
        let mut rows = whole(member, root);
        rows.push(
            json!({"step": "panel-member-finished", "boundary": "not applicable",
                         "inner_checkpoints": 6, "member": member, "model": OPUS,
                         "outcome": "succeeded", "session_ref": null}),
        );
        site("panel", member, rows);
    }
    site("sequence", "first", whole("first", "first-root-0"));
    site("sequence", "then", whole("then", "agent-root-3"));
    let result = json!({"result": "complete", "notes": "done", "model": OPUS,
                        "effort": "not reported", "boundary": "not applicable",
                        "input_tokens": 13, "output_tokens": 2, "num_turns": 2,
                        "transcript": {"home": "<root>/home/.claude/projects",
                                       "kind": "claude-session", "locator": "first-root-0"}});
    let finished = json!({"step": "sequence-step-finished", "step_name": "first",
                          "boundary": "not applicable", "model": OPUS, "result": result});
    site("sequence", "", vec![finished]);
    let mut resumed = session("", "resumed-root-0", "cold", true, false);
    resumed.extend(session("", "resumed-root-0", "resumed", true, true));
    site("resumed", "", resumed);
    let mut replaced = session("", "replaced-root-0", "cold", false, false);
    replaced.extend(session("", "replaced-root-1", "cold", false, true));
    site("replaced", "", replaced);
    site("review", "", whole("", "review-root-0"));
    sites
}

/// Every command line each harness was launched with, by wrapper.
fn launches(root: &Path) -> BTreeMap<String, Vec<String>> {
    let tags = [
        "agent", "inline", "member", "first", "resumed", "replaced", "review",
    ];
    tags.iter()
        .map(|tag| {
            let lines = std::fs::read_to_string(root.join(format!("state/{tag}.argv"))).unwrap();
            (tag.to_string(), lines.lines().map(str::to_string).collect())
        })
        .collect()
}

/// Each launch as the shipped Claude driver composes it for a seat that
/// holds `web-search` and not `web-fetch`: the search allowed, the fetch
/// denied. The agent sites go through the adapter's own permission mode;
/// the fallback's refused `missing` link is the second agent launch; the
/// resumed seat's retry rejoins its root, and the unpersisted one cannot.
fn wanted_launches() -> BTreeMap<String, Vec<String>> {
    let line = |mode: &str, model: &str, after: &str| {
        format!(
            "-p --output-format stream-json --verbose {mode}--model {model} --effort high \
             {after}--allowedTools WebSearch --disallowedTools WebFetch"
        )
    };
    let agent = line("--permission-mode acceptEdits ", OPUS, "");
    let inline = line("", OPUS, "");
    let unpersisted = line("", OPUS, "--no-session-persistence ");
    let mut launches = BTreeMap::from([
        (
            "agent",
            vec![
                agent.clone(),
                line("--permission-mode acceptEdits ", "claude-missing", ""),
                agent.clone(),
                agent.clone(),
                agent,
            ],
        ),
        (
            "resumed",
            vec![inline.clone(), format!("{inline} --resume resumed-root-0")],
        ),
        ("replaced", vec![unpersisted.clone(), unpersisted]),
    ]);
    for tag in ["inline", "member", "first", "review"] {
        launches.insert(tag, vec![inline.clone()]);
    }
    launches
        .into_iter()
        .map(|(tag, lines)| (tag.to_string(), lines))
        .collect()
}

/// The observation of the native search call the fake harness makes, as
/// protocol types it.
fn web_search() -> Observation {
    Observation {
        format: Format::Claude,
        call: Some("toolu_01".into()),
        tool: Tool::Named {
            name: "WebSearch".into(),
        },
    }
}

/// D9's handoff, from the engine's crate: protocol's one encoding of an
/// observation decodes here into protocol's one type, and an encoding
/// that adds a field, or names a tool kind the type does not, is refused
/// with serde's exact cause rather than read around.
#[test]
fn the_shared_observation_decodes_in_the_engine_crate_and_refuses_extras() {
    let wire = json!({"format": "claude", "call": "toolu_01",
                      "tool": {"kind": "named", "name": "WebSearch"}});
    let decoded: Observation = serde_json::from_value(wire.clone()).unwrap();
    assert_eq!(decoded, web_search());
    let mut forged = wire.clone();
    forged["capability"] = json!("web-search");
    let mut stamped = wire.clone();
    stamped["tool"]["dialect"] = json!("claude-native-search");
    let mut unnamed = wire;
    unnamed["tool"] = json!({"kind": "native", "name": "WebSearch"});
    let refusals = [
        (
            forged,
            "unknown field `capability`, expected one of `format`, `call`, `tool`",
        ),
        (stamped, "unknown field `dialect`, expected `name`"),
        (
            unnamed,
            "unknown variant `native`, expected one of `named`, `mcp`, \
             `mcp_unidentified`, `missing`",
        ),
    ];
    for (wire, cause) in refusals {
        let refused = serde_json::from_value::<Observation>(wire).unwrap_err();
        assert_eq!(refused.to_string(), cause);
    }
}

/// A direct append of a partial attribution group or a private
/// observation, each beside a legacy native row, is refused with the
/// exact v6 violation at the seq it would have taken, and the journal
/// stands still. A whole engine-owned group is v6's to admit (SC4), and
/// the export sweep passes the journal that takes it: append and export
/// read one contract. (Verify also folds, and a completed run takes no
/// further event, so store's own suite proves verify on a live run.)
fn fences_the_attribution_group(store: &mut Store, run_id: &str) {
    let head = store.head_hash(run_id).unwrap();
    let base = json!({"step": "seat-turn", "turn": 3, "model": "claude-opus-5-5",
                      "tool": "WebSearch"});
    let group = json!({"capability": "web-search", "dialect": "claude-native-search",
                       "call_id": "attempt-1:toolu_01", "call_state": "observed"});
    let mut whole = base.clone();
    let fields = group.as_object().unwrap().clone();
    whole.as_object_mut().unwrap().extend(fields);
    let mut partial = base.clone();
    partial["capability"] = json!("web-search");
    let mut private = base;
    private[OBSERVATION_KEY] = serde_json::to_value(web_search()).unwrap();
    for checkpoint in [partial, private] {
        let payload = json!({"effect_id": "fx", "checkpoint": checkpoint});
        let error = store
            .append_next(run_id, EventType::EffectCheckpointed, payload, None, None)
            .unwrap_err();
        let StoreError::SeatRecord(refusal) = error else {
            panic!("{error}");
        };
        let seq = head.0 + 1;
        let want = SeatRecordError {
            seq,
            path: "/".into(),
            contract: V6,
        };
        assert_eq!(refusal, want, "{checkpoint}");
        assert_eq!(store.head_hash(run_id).unwrap(), head);
    }
    let payload = json!({"effect_id": "fx", "checkpoint": whole});
    let landed = store
        .append_next(run_id, EventType::EffectCheckpointed, payload, None, None)
        .unwrap();
    assert_eq!(landed.seq, head.0 + 1);
    let exported = store.export_ndjson(run_id).unwrap();
    assert_eq!(exported.lines().count() as u64, landed.seq);
}

/// D9's preparation proof at this merge: every site shape journals the
/// rows the shipped Claude lowering wrote for a held native search and a
/// local read exactly, the run exports and the export verifies offline,
/// no record carries an attribution field or a private observation, and
/// a direct append of either is refused.
#[test]
fn every_site_shape_journals_its_legacy_native_rows_through_export_and_verify() {
    let (_dir, root, mut engine) = driven(None);
    let run_id = engine.run_id.clone();
    let events = engine.store.load(&run_id).unwrap();
    assert_eq!(launches(&root), wanted_launches());
    assert_eq!(journaled(&events, &root), wanted());
    // Nothing SC4 reserves, and no private observation, on any record.
    for event in &events {
        for record in [&event.payload["checkpoint"], &event.payload["result"]] {
            for field in UNRECORDED {
                assert_eq!(record.get(field), None, "{field} at seq {}", event.seq);
            }
        }
    }
    exports_and_verifies(&engine.store, &run_id, events.len());
    fences_the_attribution_group(&mut engine.store, &run_id);
}

/// The matrix compiled and driven to completion under a canonicalised
/// temporary root, with `observing` written as the wrappers' filter.
fn driven(observing: Option<&str>) -> (tempfile::TempDir, PathBuf, Engine) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap();
    for sub in ["bin", "state", "home", "work"] {
        std::fs::create_dir_all(root.join(sub)).unwrap();
    }
    if let Some(program) = observing {
        std::fs::write(root.join(OBSERVING), program).unwrap();
    }
    let store = Store::open(&root.join("forge.db")).unwrap();
    let world = world(&root);
    let bundle = bundle(&root, &world);
    let repo = Some(root.join("work"));
    let mut engine =
        Engine::start_in_world(store, bundle, "Feature: legacy", repo, Some(world)).unwrap();
    let end = engine.drive().unwrap();
    let status = end.state.status;
    assert_eq!(status, Status::Completed, "{:?}", end.state.park_reason);
    (dir, root, engine)
}

/// The run exports all `count` of its events and the export verifies
/// offline to the completed run.
fn exports_and_verifies(store: &Store, run_id: &str, count: usize) {
    let exported = store.export_ndjson(run_id).unwrap();
    assert_eq!(exported.lines().count(), count);
    let verified = brokkr_store::verify_export(&exported).unwrap();
    let journal = (Status::Completed, count as u64);
    assert_eq!((verified.status, verified.seq), journal);
}

/// U4e's consumer at every site shape (CC1, CC3, SC4): the shipped Claude
/// driver's rows, each call carrying the observation U4f2 will emit and
/// the search a forged call id and state, journal exactly the legacy rows
/// with the engine's own whole group on the held search — its call id
/// the digest of its attempt, site stamps, provider and harness id — and the
/// local read ordinary. The resumed seat's rejoined root replays its
/// `toolu_01`, which is history and stays ordinary (U4f); the replaced
/// seat's root was never offered, so its second search is new. No
/// observation or forged value reaches the store, and the run exports and
/// verifies.
#[test]
fn every_site_shape_journals_the_engines_group_on_an_observed_held_call() {
    let (_dir, root, engine) = driven(Some(OBSERVATIONS));
    let run_id = engine.run_id.clone();
    let events = engine.store.load(&run_id).unwrap();
    let mut wanted = wanted();
    for ((seat, _), rows) in wanted.iter_mut() {
        let searches = rows.iter_mut().filter(|row| row["tool"] == "WebSearch");
        for row in searches.take(if seat == "resumed" { 1 } else { 2 }) {
            let group = json!({"capability": "web-search", "dialect": "claude-native-search",
                               "call_id": "<owned toolu_01>", "call_state": "observed"});
            row.as_object_mut()
                .unwrap()
                .extend(group.as_object().unwrap().clone());
        }
    }
    assert_eq!(journaled(&events, &root), wanted);
    let attributed: BTreeSet<&str> = events
        .iter()
        .filter_map(|event| event.payload["checkpoint"].get("call_id"))
        .filter_map(Value::as_str)
        .collect();
    // One search per invocation, each its own call: eight seats, the
    // panel's and the sequence's second site, and the replaced seat's
    // second launch; the resumed one's replay is no new use.
    assert_eq!(attributed.len(), 11);
    for event in &events {
        let checkpoint = &event.payload["checkpoint"];
        for field in ["response_sha256", OBSERVATION_KEY] {
            assert_eq!(checkpoint.get(field), None, "{field} at seq {}", event.seq);
        }
    }
    exports_and_verifies(&engine.store, &run_id, events.len());
}
