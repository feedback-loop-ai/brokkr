//! The engine's complete MCP server set at the final check (decision 0065
//! slice two, U7a; MB1, SI2): the empty, hands-only and three-server sets
//! check exactly, each read from the final command independently of the
//! builder that wrote it, and a missing, extra, duplicate, changed or
//! counterfeit server, a lost isolation, a name that is not exactly its
//! holding's and a removal the sealed set does not name each refuse with
//! the final-set cause before any spawn. The mutation oracle's effect
//! matcher, [`unmatched`], moved here from `native_controls/tests.rs`, and
//! reads the server set first, as the check does.

use super::*;

/// The final-set refusal at `harness`, written out by hand: this module's
/// one pin of the cause.
pub(super) fn unsealed(harness: &str) -> Refusal {
    Refusal {
        authored: false,
        cause: format!(
            "provider '{harness}' final MCP configuration is not the engine's sealed server set"
        ),
    }
}

/// Whether `effect` is an MCP option at `harness`, as the oracle reads it:
/// strict MCP or a document for Claude's grammar, an `mcp_servers`
/// assignment for Codex.
fn is_mcp(harness: &str, effect: &Expressed) -> bool {
    match (harness, effect) {
        ("codex", Expressed::Control(name, values)) => {
            *name == "--config" && values.iter().any(|value| value.starts_with("mcp_servers."))
        }
        (_, Expressed::Control(name, _)) => {
            matches!(*name, "--strict-mcp-config" | "--mcp-config")
        }
        _ => false,
    }
}

/// The refusal a readable mutation earns after delivery and before any
/// departure where its own state no longer carries exactly the effects of
/// the checked command (rebuild unit 21-fix-a, R1): the final-set refusal
/// where its MCP options differ, counted as often as each stands (U7a);
/// else the first effect of the checked command it lacks, each counted as
/// often as it stands, else the first it adds, each named without its
/// value.
pub(super) fn unmatched(harness: &str, expected: &[String], command: &[String]) -> Option<Refusal> {
    let effects = |command: &[String]| -> Option<Vec<Expressed>> {
        let parsed = grammar::parse_final(harness, &command[1..])?.ok()?;
        let bound = grammar::Bound::default();
        read_state(&parsed.command, bound)
            .ok()
            .map(|state| state.effects)
    };
    let (checked, mut left) = (effects(expected)?, effects(command)?);
    let servers = |effects: &[Expressed]| -> Vec<Expressed> {
        let mcp = effects.iter().filter(|effect| is_mcp(harness, effect));
        mcp.cloned().collect()
    };
    let (sealed, carried) = (servers(&checked), servers(&left));
    let count = |effect: &Expressed, of: &[Expressed]| of.iter().filter(|e| *e == effect).count();
    let moved = (sealed.iter().chain(&carried)).any(|e| count(e, &sealed) != count(e, &carried));
    if moved {
        return Some(unsealed(harness));
    }
    let named = |effect: &Expressed| match effect {
        Expressed::List(ListKind::Include, _) => "an include list".to_string(),
        Expressed::List(ListKind::Allow, _) => "an allow list".to_string(),
        Expressed::List(ListKind::Deny, _) => "a deny list".to_string(),
        Expressed::Class(word) => format!("the '{word}' sandbox class"),
        Expressed::Off(capability) => {
            format!("the measured OFF for native capability '{capability}'")
        }
        Expressed::Control(name, _) => format!("'{name}'"),
        Expressed::Isolation(Isolation::UserConfig) => {
            "the isolation option '--ignore-user-config'".to_string()
        }
        Expressed::Isolation(Isolation::Apps) => "the isolation option '--disable apps'".into(),
        Expressed::Isolation(Isolation::Plugins) => {
            "the isolation option '--disable plugins'".to_string()
        }
    };
    for effect in &checked {
        match left.iter().position(|other| other == effect) {
            Some(at) => drop(left.remove(at)),
            None => return Some(uncarried(harness, &named(effect))),
        }
    }
    left.first().map(|extra| uncomposed(harness, &named(extra)))
}

/// The two holdings of MB1's scenario, each sealed with its own plan,
/// digest and secret name.
fn brokers() -> Vec<SealedBroker> {
    [
        ("library-docs", "a", "LIBRARY_DOCS_TOKEN"),
        ("issue-tracker", "b", "ISSUE_TRACKER_TOKEN"),
    ]
    .map(|(capability, digit, secret)| SealedBroker {
        capability: capability.into(),
        plan: format!("/state/plans/{capability}.json"),
        plan_digest: digit.repeat(64),
        removals: argv(&[secret]),
    })
    .to_vec()
}

/// The names [`brokers`] seals for removal, in the order a spawn may list
/// them.
fn removals() -> Vec<String> {
    argv(&["ISSUE_TRACKER_TOKEN", "LIBRARY_DOCS_TOKEN"])
}

/// The hands server's entry the fixtures bind, written out by hand.
const HANDS_ENTRY: &str = r#"{"args":["hands","serve","--workdir","/w","--artifacts","/a","--reach","{\"home\":\"\",\"worktree\":{\"path\":\"\",\"identity\":null},\"git\":null,\"binds\":[],\"store\":null}","--spec","{\"binds\":[],\"kind\":\"workspace\",\"network\":false}"],"command":"/bin/brokkr"}"#;

/// A broker's entry, written out by hand: `command` running `broker serve`
/// on the plan at `plan` sealed at `digest`.
fn broker_entry(command: &str, plan: &str, digest: &str) -> String {
    format!(
        r#"{{"args":["broker","serve","--plan","{plan}","--plan-digest","{digest}"],"command":"{command}"}}"#
    )
}

/// The library-docs broker's own entry, its own plan and digest.
fn docs_entry(command: &str) -> String {
    broker_entry(command, "/state/plans/library-docs.json", &"a".repeat(64))
}

/// An MCP document naming `entries` in order, written out by hand.
fn document(entries: &[(&str, String)]) -> String {
    let named: Vec<String> = (entries.iter())
        .map(|(name, entry)| format!("\"{name}\":{entry}"))
        .collect();
    format!(r#"{{"mcpServers":{{{}}}}}"#, named.join(","))
}

/// MB1's three servers, in the order the engine writes them.
fn three() -> Vec<(&'static str, String)> {
    let tracker = broker_entry(
        "/bin/brokkr",
        "/state/plans/issue-tracker.json",
        &"b".repeat(64),
    );
    vec![
        ("brokkr", HANDS_ENTRY.to_string()),
        ("cap-issue-tracker", tracker),
        ("cap-library-docs", docs_entry("/bin/brokkr")),
    ]
}

/// [`claude_cold`] carrying the document of `entries` in place of the hands'.
fn carrying(entries: &[(&str, String)]) -> Vec<String> {
    let mut cold = claude_cold();
    cold[11] = document(entries);
    cold
}

/// [`brokers`]' served names in sealed order, as a brokered launch's allow
/// list grants them (U9b1d).
const SEALED: [&str; 2] = ["cap-library-docs", "cap-issue-tracker"];

/// `cold` with its allow list also naming `servers` after the hands tool,
/// as the set's builder writes a brokered set's allowance (U9b1d).
fn allowing(mut cold: Vec<String>, servers: &[&str]) -> Vec<String> {
    let named: String = servers.iter().map(|name| format!(",mcp__{name}")).collect();
    cold[13] = format!("mcp__brokkr__workspace{named},WebSearch");
    cold
}

/// The engine's serving choices beside the plan: the sealed `brokers` and
/// the names its spawn `removed`.
fn serving<'a>(brokers: &'a [SealedBroker], removed: &'a [String]) -> Serving<'a> {
    Serving {
        brokers,
        removed,
        ..Serving::default()
    }
}

/// MB1's scenario: the box's hands and the library-docs and issue-tracker
/// brokers check exactly, each broker the engine's executable running
/// `broker serve` on its own sealed plan beside the hands profile's `hands
/// serve`, and the plan's native OFF stands beside them. The removals are a
/// set, in any order.
#[test]
fn hands_and_two_brokers_check_exactly_beside_native_off() {
    let (boxed, brokers, removed) = (claude_final(), brokers(), removals());
    let serving = serving(&brokers, &removed);
    let cold = allowing(carrying(&three()), &SEALED);
    assert_eq!(
        boxed.check(cold.clone(), serving).map(Checked::into_argv),
        Ok(cold.clone())
    );
    assert_eq!(cold[9..11], argv(&["--strict-mcp-config", "--mcp-config"]));
    assert_eq!(cold[14..], argv(&["--disallowedTools", "WebFetch"]));
    let reversed: Vec<String> = removed.iter().rev().cloned().collect();
    let reordered = Serving {
        removed: &reversed,
        ..serving
    };
    assert_eq!(
        boxed.check(cold.clone(), reordered).map(Checked::into_argv),
        Ok(cold)
    );
}

/// Each departure from MB1's three servers in the final command refuses
/// with the final-set cause, before any later guard: a server missing,
/// extra or named twice; the right name running a dialect's executable,
/// another executable, the hands profile, another plan or an altered
/// digest, or carrying an option beside its two; strict isolation dropped
/// or the document replaced by a file; and a spawn removing other names.
#[test]
fn each_departure_from_the_three_server_set_refuses_before_spawn() {
    let (boxed, brokers, removed) = (claude_final(), brokers(), removals());
    let (serving, three) = (serving(&brokers, &removed), three());
    let carrying = |entries: &[(&str, String)]| allowing(carrying(entries), &SEALED);
    let replaced =
        |entry: String| [three[..2].to_vec(), vec![("cap-library-docs", entry)]].concat();
    let docs = "/state/plans/library-docs.json";
    let with_env = docs_entry("/bin/brokkr").replace("\"}", "\",\"env\":{}}");
    let unisolated = {
        let mut cold = carrying(&three);
        cold.remove(9);
        cold
    };
    let filed = {
        let mut cold = carrying(&three);
        cold[11] = "/w/mcp.json".into();
        cold
    };
    let cases = [
        ("missing", carrying(&three[..2])),
        ("hands alone", claude_cold()),
        (
            "extra",
            carrying(
                &[
                    three.clone(),
                    vec![("cap-other", docs_entry("/bin/brokkr"))],
                ]
                .concat(),
            ),
        ),
        (
            "duplicate",
            carrying(&[three.clone(), vec![three[2].clone()]].concat()),
        ),
        (
            "dialect executable",
            carrying(&replaced(r#"{"args":["docs-mcp"],"command":"npx"}"#.into())),
        ),
        (
            "another executable",
            carrying(&replaced(docs_entry("/usr/bin/brokkr"))),
        ),
        ("hands profile", carrying(&replaced(HANDS_ENTRY.into()))),
        (
            "another plan",
            carrying(&replaced(broker_entry(
                "/bin/brokkr",
                "/state/plans/issue-tracker.json",
                &"a".repeat(64),
            ))),
        ),
        (
            "altered digest",
            carrying(&replaced(broker_entry(
                "/bin/brokkr",
                docs,
                &"c".repeat(64),
            ))),
        ),
        ("closed entry", carrying(&replaced(with_env))),
        ("isolation dropped", unisolated),
        ("a file for the document", filed),
    ];
    for (what, command) in cases {
        assert_eq!(
            boxed.check(command, serving),
            Err(unsealed("claude")),
            "{what}"
        );
    }
    let cold = carrying(&three);
    for (what, removed) in [
        ("a removal missing", argv(&["LIBRARY_DOCS_TOKEN"])),
        (
            "a removal beside them",
            [removals(), argv(&["HOME"])].concat(),
        ),
        (
            "a removal twice",
            [removals(), argv(&["LIBRARY_DOCS_TOKEN"])].concat(),
        ),
    ] {
        let serving = Serving {
            removed: &removed,
            ..serving
        };
        assert_eq!(
            boxed.check(cold.clone(), serving),
            Err(unsealed("claude")),
            "{what}"
        );
    }
}

/// The empty set checks served as composed and with the empty set's strict
/// flag and document, and the hands-only set with the transport's document
/// alone; a counterfeit hands server where none is sealed, brokers where no
/// hands are, a broker document where the hands alone are sealed, and a
/// removal neither set names each refuse.
#[test]
fn the_empty_and_hands_only_sets_check_exactly_and_refuse_a_counterfeit() {
    let boxed = claude_final();
    assert_eq!(document(&[("brokkr", HANDS_ENTRY.into())]), MCP_DOCUMENT);
    assert_eq!(
        boxed.cold(claude_cold()).map(Checked::into_argv),
        Ok(claude_cold())
    );
    assert_eq!(boxed.cold(carrying(&three())), Err(unsealed("claude")));
    let removed = argv(&["LIBRARY_DOCS_TOKEN"]);
    let removing = Serving {
        removed: &removed,
        ..Serving::default()
    };
    assert_eq!(
        boxed.check(claude_cold(), removing),
        Err(unsealed("claude"))
    );

    let local = claude_local();
    let composed = local.composed().unwrap();
    let served = |mcp: &[String]| {
        let (extra, managed) = (composed.extra.clone(), composed.managed.clone());
        [argv(&CLAUDE_LEAD), extra, mcp.to_vec(), managed].concat()
    };
    let empty = argv(&[
        "--strict-mcp-config",
        "--mcp-config",
        r#"{"mcpServers":{}}"#,
    ]);
    let built = Serving {
        mcp: &empty,
        ..Serving::default()
    };
    assert_eq!(
        local.check(served(&empty), built).map(Checked::into_argv),
        Ok(served(&empty))
    );
    assert_eq!(
        local.cold(served(&[])).map(Checked::into_argv),
        Ok(served(&[]))
    );
    let hands = argv(&["--strict-mcp-config", "--mcp-config", MCP_DOCUMENT]);
    let counterfeit = Serving {
        mcp: &hands,
        ..Serving::default()
    };
    assert_eq!(
        local.check(served(&hands), counterfeit),
        Err(unsealed("claude"))
    );
    let (brokers, removed) = (brokers(), removals());
    let unhanded = Serving {
        brokers: &brokers,
        removed: &removed,
        ..built
    };
    assert_eq!(
        local.check(served(&empty), unhanded),
        Err(unsealed("claude"))
    );
}

/// The document's server table is one object: an array, a string or a
/// number in its place refuses with the final-set cause, and so does a
/// command that carries no MCP option at all where the box's hands are
/// sealed.
#[test]
fn a_server_table_that_is_not_one_object_refuses() {
    let boxed = claude_final();
    for table in ["[]", "\"brokkr\"", "1"] {
        let mut cold = claude_cold();
        cold[11] = format!(r#"{{"mcpServers":{table}}}"#);
        assert_eq!(boxed.cold(cold), Err(unsealed("claude")), "{table}");
    }
    let mut unserved = claude_cold();
    unserved.drain(9..12);
    assert_eq!(boxed.cold(unserved), Err(unsealed("claude")));
}

/// A broker is named exactly `cap-<capability>`: a capability whose name is
/// one bare name with `_` checks as itself, while one that is not one bare
/// name, an empty one and two holdings under one name refuse, even where
/// the command carries the very name the engine would write; no name is
/// shortened, normalized or suffixed into another holding.
#[test]
fn a_server_name_is_never_repaired_into_another_holding() {
    let boxed = claude_final();
    let removed = argv(&["LIBRARY_DOCS_TOKEN"]);
    let broker = |capability: &str| SealedBroker {
        capability: capability.into(),
        ..brokers()[0].clone()
    };
    let underscored = [broker("library_docs")];
    let cold = carrying(&[
        ("brokkr", HANDS_ENTRY.into()),
        ("cap-library_docs", docs_entry("/bin/brokkr")),
    ]);
    let cold = allowing(cold, &["cap-library_docs"]);
    assert_eq!(
        (boxed.check(cold.clone(), serving(&underscored, &removed))).map(Checked::into_argv),
        Ok(cold)
    );
    for (what, brokers, name) in [
        ("dotted", vec![broker("library.docs")], "cap-library.docs"),
        ("spaced", vec![broker("library docs")], "cap-library docs"),
        ("empty", vec![broker("")], "cap-"),
        (
            "colliding",
            vec![broker("library-docs"); 2],
            "cap-library-docs",
        ),
    ] {
        let cold = carrying(&[
            ("brokkr", HANDS_ENTRY.into()),
            (name, docs_entry("/bin/brokkr")),
        ]);
        assert_eq!(
            boxed.check(cold, serving(&brokers, &removed)),
            Err(unsealed("claude")),
            "{what}"
        );
    }
}

/// Codex carries the box's hands in its adapter's measured fragment, whose
/// bindings name `brokkr` alone, so a sealed broker set is not carried and
/// refuses as hands that are not the engine's; the hands alone still check.
#[test]
fn codex_does_not_carry_a_broker_set() {
    let boxed = codex_hands(&CODEX_HANDS);
    let cold = boxed.served(&CODEX_LEAD);
    assert_eq!(
        boxed.cold(cold.clone()).map(Checked::into_argv),
        Ok(cold.clone())
    );
    let (brokers, removed) = (brokers(), removals());
    assert_eq!(
        boxed.check(cold, serving(&brokers, &removed)),
        Err(final_refusal(
            "codex",
            "is sealed with hands that are not the engine's workspace hands"
        ))
    );
}

/// Codex's servers are read from exactly `mcp_servers.<name>.<part>`
/// assignments, each part once, through bare names and TOML basic strings
/// the reader decodes on its own: a key of two or four parts, a quoted
/// server name, a part assigned twice even to its own value, an escape TOML
/// does not define, and an argument array with a trailing comma or without
/// its commas each refuse with the final-set cause; and a measured hands
/// fragment carrying a control beside its bindings is not the engine's
/// workspace hands.
#[test]
fn a_codex_assignment_outside_its_one_reading_refuses() {
    let boxed = codex_hands(&CODEX_HANDS);
    let cold = boxed.served(&CODEX_LEAD);
    let replaced = |at: usize, value: String| {
        let mut served = cold.clone();
        served[at] = value;
        served
    };
    let beside = |value: &str| {
        let mut served = cold.clone();
        served.splice(17..17, argv(&["-c", value]));
        served
    };
    let args = |array: String| format!("mcp_servers.brokkr.args={array}");
    let cases = [
        ("two key parts", beside("mcp_servers.brokkr={}")),
        (
            "four key parts",
            beside("mcp_servers.brokkr.env.HOME=\"/h\""),
        ),
        (
            "a quoted server name",
            replaced(12, "mcp_servers.\"brokkr\".command=\"/bin/brokkr\"".into()),
        ),
        (
            "a part twice",
            beside("mcp_servers.brokkr.command=\"/bin/brokkr\""),
        ),
        (
            "an unknown escape",
            replaced(12, "mcp_servers.brokkr.command=\"/b\\in/brokkr\"".into()),
        ),
        (
            "a trailing comma",
            replaced(14, args(SERVER_ARGUMENTS.replace("}\"]", "}\",]"))),
        ),
        (
            "no commas",
            replaced(14, args(SERVER_ARGUMENTS.replace("\",\"", "\" \""))),
        ),
    ];
    for (what, command) in cases {
        assert_eq!(boxed.cold(command), Err(unsealed("codex")), "{what}");
    }
    let controlled = [&CODEX_HANDS[..], &["--add-dir", "/x"]].concat();
    assert_eq!(
        codex_hands(&controlled).cold(cold),
        Err(final_refusal(
            "codex",
            "is sealed with hands that are not the engine's workspace hands"
        ))
    );
}

/// The sealed broker set's closed form: written as the engine seals it and
/// read back exactly, and an unknown member or a missing one refused.
#[test]
fn a_sealed_broker_set_is_read_closed() {
    let servers = SealedServers::Brokered(brokers());
    let written = serde_json::to_value(&servers).unwrap();
    assert_eq!(
        written,
        json!({"brokered": [
            {"capability": "library-docs", "plan": "/state/plans/library-docs.json",
             "plan_digest": "a".repeat(64), "removals": ["LIBRARY_DOCS_TOKEN"]},
            {"capability": "issue-tracker", "plan": "/state/plans/issue-tracker.json",
             "plan_digest": "b".repeat(64), "removals": ["ISSUE_TRACKER_TOKEN"]}]})
    );
    let read = |value: Value| serde_json::from_value::<SealedServers>(value).ok();
    assert_eq!(read(written.clone()), Some(servers));
    let mut extra = written.clone();
    extra["brokered"][0]["command"] = json!("npx");
    let mut missing = written;
    missing["brokered"][1]
        .as_object_mut()
        .unwrap()
        .remove("removals");
    assert_eq!((read(extra), read(missing)), (None, None));
}
