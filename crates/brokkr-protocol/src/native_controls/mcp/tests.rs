//! The independent readers of the MCP server set on their own (decision
//! 0065 slice two, U7a): what a Codex TOML basic string and string array
//! decode to, which the final check's seam shows only as a set carried or
//! refused, and the words a Claude server table that is not one object is
//! refused with, which that seam never shows.

use super::*;

/// Every escape TOML defines reads back to its one character, `\u` in
/// four digits and `\U` in eight, and a tab may also stand raw.
#[test]
fn a_toml_basic_string_reads_back_every_escape() {
    assert_eq!(
        toml_string("\"\\b\\t\\n\\f\\r\\\"\\\\\\u002F\\U0001F600\tx\""),
        Some("\u{8}\t\n\u{c}\r\"\\/\u{1F600}\tx".into())
    );
}

/// A basic string the engine never writes is refused, never read as the
/// nearest value: an escape TOML does not define, a bare quote inside, a
/// raw control character or DEL, an escape of too few digits, of a sign or
/// of a surrogate, a dangling backslash and an unquoted value.
#[test]
fn a_toml_basic_string_the_engine_never_writes_is_refused() {
    for (what, text) in [
        ("an unknown escape", "\"/b\\in\""),
        ("a bare quote", "\"/b\"in\""),
        ("a control character", "\"/b\u{1}in\""),
        ("a raw newline", "\"/b\nin\""),
        ("DEL", "\"/b\u{7f}in\""),
        ("too few digits", "\"\\u2F\""),
        ("too few long digits", "\"\\U002F\""),
        ("a sign", "\"\\u+02F\""),
        ("a surrogate", "\"\\uD800\""),
        ("a dangling backslash", "\"/bin\\\""),
        ("no quotes", "/bin"),
    ] {
        assert_eq!(toml_string(text), None, "{what}");
    }
}

/// A TOML array reads back the strings it lists, blanks around each comma
/// allowed; a trailing comma, a missing comma and an unclosed string are
/// each refused.
#[test]
fn a_toml_string_array_reads_only_its_comma_separated_strings() {
    assert_eq!(
        toml_strings("[ \"a\" ,\t\"b\\\"c\" ]"),
        Some(vec!["a".to_string(), "b\"c".to_string()])
    );
    assert_eq!(toml_strings("[]"), Some(Vec::new()));
    for (what, text) in [
        ("a trailing comma", "[\"a\",]"),
        ("no comma", "[\"a\" \"b\"]"),
        ("an unclosed string", "[\"a]"),
    ] {
        assert_eq!(toml_strings(text), None, "{what}");
    }
}

/// A `--config` control carrying two assignments is no one reading, and a
/// server whose three parts are complete under a quoted name is never read
/// as a server of that name, quotes and all.
#[test]
fn a_config_control_of_two_values_or_a_quoted_name_is_no_reading() {
    let values = vec![
        "mcp_servers.brokkr.command=\"/bin/brokkr\"".to_string(),
        "mcp_servers.brokkr.default_tools_approval_mode=\"approve\"".to_string(),
    ];
    assert_eq!(codex_entries(&[("--config", &values)]), None);
    let quoted = [
        "command=\"/bin/brokkr\"",
        "args=[]",
        "default_tools_approval_mode=\"approve\"",
    ]
    .map(|part| vec![format!("mcp_servers.\"brokkr\".{part}")]);
    let controls: Vec<(&str, &Vec<String>)> = quoted.iter().map(|v| ("--config", v)).collect();
    assert_eq!(codex_entries(&controls), None);
}

/// U7d2 (MB1): a measured fragment expanded beside sealed brokers carries
/// the whole set's document, each broker on its own plan, written out by
/// hand here; with none it is the hands-only expansion, and a broker whose
/// capability is no bare name expands nothing.
#[test]
fn a_fragment_expanded_beside_brokers_carries_the_whole_set() {
    let spec = crate::hands::HandsSpec {
        network: false,
        binds: Vec::new(),
    };
    let reach = crate::hands::Sealed::default();
    let hands = Transport {
        brokkr: std::path::Path::new("/bin/brokkr"),
        workdir: std::path::Path::new("/w"),
        artifacts: std::path::Path::new("/a"),
        sealed: &reach,
        spec: &spec,
    };
    let broker = |capability: &str| SealedBroker {
        capability: capability.into(),
        plan: "/p/plan-0.json".into(),
        plan_digest: "a".repeat(64),
        removals: vec!["DOCS_TOKEN".into()],
    };
    let fragment = ["--mcp-config".to_string(), "{hands_mcp_json}".to_string()];
    let served = "{\"command\":\"/bin/brokkr\",\"args\":[\"hands\",\"serve\",\"--workdir\",\"/w\",\
                  \"--artifacts\",\"/a\",\"--reach\",\"{\\\"home\\\":\\\"\\\",\\\"worktree\\\":\
                  {\\\"path\\\":\\\"\\\",\\\"identity\\\":null},\\\"git\\\":null,\\\"binds\\\":[],\\\"store\\\":null}\",\
                  \"--spec\",\"{\\\"binds\\\":[],\\\"kind\\\":\\\"workspace\\\",\\\"network\\\":\
                  false}\"]}";
    let docs = format!(
        "{{\"command\":\"/bin/brokkr\",\"args\":[\"broker\",\"serve\",\"--plan\",\
         \"/p/plan-0.json\",\"--plan-digest\",\"{}\"]}}",
        "a".repeat(64)
    );
    let parsed = |argv: Option<Vec<String>>| {
        let argv = argv.unwrap();
        assert_eq!(argv[0], "--mcp-config");
        serde_json::from_str::<Value>(&argv[1]).unwrap()
    };
    let whole = format!("{{\"mcpServers\":{{\"brokkr\":{served},\"cap-library-docs\":{docs}}}}}");
    assert_eq!(
        parsed(hands.expand_beside(&fragment, &[broker("library-docs")])),
        serde_json::from_str::<Value>(&whole).unwrap()
    );
    let alone = format!("{{\"mcpServers\":{{\"brokkr\":{served}}}}}");
    assert_eq!(
        parsed(hands.expand(&fragment)),
        serde_json::from_str::<Value>(&alone).unwrap()
    );
    assert_eq!(hands.expand_beside(&fragment, &[]), hands.expand(&fragment));
    assert_eq!(hands.expand_beside(&fragment, &[broker("docs.v2")]), None);
}

/// A server table that is not one object is refused in the reader's own
/// words.
#[test]
fn a_server_table_that_is_not_one_object_is_refused_in_its_words() {
    let refusal =
        |text: &str| (serde_json::from_str::<Document>(text).err()).map(|e| e.to_string());
    assert_eq!(
        refusal(r#"{"mcpServers":[]}"#),
        Some("invalid type: sequence, expected an object naming each server once at line 1 column 14".into())
    );
}

/// U8a: the box's hands server is read field by field, the artifact root
/// the engine carries among them, so an entry naming another root or none
/// is not the engine's hands, whatever else it carries.
#[test]
fn the_hands_entry_is_read_with_its_artifact_root() {
    let path = std::path::Path::new;
    let (brokkr, workdir, artifacts) = (path("/bin/brokkr"), path("/w"), path("/a"));
    let spec = &crate::hands::HandsSpec::default();
    let sealed = &crate::hands::seal(spec, path("/h"), workdir, &Default::default(), None);
    let owed = Owed::Hands(Transport {
        brokkr,
        workdir,
        artifacts,
        sealed,
        spec,
    });
    let entry = |args: Vec<String>| Entry {
        command: "/bin/brokkr".into(),
        args,
    };
    let args = owed.args();
    assert!(owed.is("/bin/brokkr", &entry(args.clone())));
    let mut moved = args.clone();
    moved[5] = "/elsewhere".into();
    let rootless = [&args[..4], &args[6..]].concat();
    // U8a2c: and with the reach sealed for it, read as its type; another
    // reach, one that does not read, or none, is not the engine's hands.
    let other = crate::hands::seal(spec, path("/elsewhere"), workdir, &Default::default(), None);
    let (mut resealed, mut unread) = (args.clone(), args.clone());
    resealed[7] = serde_json::to_string(&other).unwrap();
    unread[7] = "{".into();
    let reachless = [&args[..6], &args[8..]].concat();
    for args in [moved, rootless, resealed, unread, reachless] {
        assert!(!owed.is("/bin/brokkr", &entry(args.clone())), "{args:?}");
    }
}

/// #896: dsh carries the box's hands alone in its staged overlay, so R1
/// reads them carried by a command with no MCP option, and not with a
/// broker beside them, an MCP option in the command or no hands at all.
#[test]
fn dsh_carries_the_hands_alone_in_its_overlay() {
    let path = std::path::Path::new;
    let spec = crate::hands::HandsSpec::default();
    let reach = crate::hands::Sealed::default();
    let hands = Transport {
        brokkr: path("/bin/brokkr"),
        workdir: path("/w"),
        artifacts: path("/a"),
        sealed: &reach,
        spec: &spec,
    };
    let broker = [SealedBroker {
        capability: "docs".into(),
        plan: "/p/plan.json".into(),
        plan_digest: "a".repeat(64),
        removals: Vec::new(),
    }];
    let document = vec!["{}".to_string()];
    let option = [("--mcp-config", &document)];
    let alone = Servers {
        hands: Some(hands),
        brokers: &[],
    };
    let brokered = Servers {
        hands: Some(hands),
        brokers: &broker,
    };
    assert!(alone.carried("dsh", &[]));
    assert!(!brokered.carried("dsh", &[]));
    assert!(!alone.carried("dsh", &option));
    assert!(!Servers::default().carried("dsh", &[]));
}

/// A harness whose grammar carries no MCP options reads no server table,
/// as the reader's own contract says, so dsh's overlay-borne set never
/// borrows Claude's or Codex's reading.
#[test]
fn a_harness_with_no_mcp_carrier_reads_no_servers() {
    let document = vec!["{}".to_string()];
    let controls = [("--mcp-config", &document)];
    assert_eq!(read("dsh", &controls), None);
    assert_eq!(read("exec", &controls), None);
}

/// U9b1d (MB1): beside sealed brokers, the shipped Claude fragment's allow
/// list names each broker by its served name after the hands tool, once,
/// in sealed order, written out by hand; with none the fragment keeps every
/// byte but its document, and a fragment writing no allow list, as Codex's
/// and dsh's write none, gains none.
#[test]
fn a_fragment_allows_each_broker_by_its_served_name_after_the_hands_tool() {
    let path = std::path::Path::new;
    let (spec, reach) = (crate::hands::HandsSpec::default(), Default::default());
    let hands = Transport {
        brokkr: path("/bin/brokkr"),
        workdir: path("/w"),
        artifacts: path("/a"),
        sealed: &reach,
        spec: &spec,
    };
    let broker = |capability: &str| SealedBroker {
        capability: capability.into(),
        plan: format!("/p/{capability}.json"),
        plan_digest: "a".repeat(64),
        removals: Vec::new(),
    };
    let fragment: Vec<String> = ["--tools", "", "--strict-mcp-config", "--mcp-config"]
        .into_iter()
        .chain([
            "{hands_mcp_json}",
            "--allowedTools",
            "mcp__brokkr__workspace",
        ])
        .map(String::from)
        .collect();
    let expanded = |brokers: &[SealedBroker]| hands.expand_beside(&fragment, brokers).unwrap();
    let (docs, tracker) = (broker("library-docs"), broker("issue-tracker"));
    assert_eq!(
        expanded(std::slice::from_ref(&docs))[6],
        "mcp__brokkr__workspace,mcp__cap-library-docs"
    );
    assert_eq!(
        expanded(&[docs.clone(), tracker])[6],
        "mcp__brokkr__workspace,mcp__cap-library-docs,mcp__cap-issue-tracker"
    );
    let alone = expanded(&[]);
    assert_eq!((&alone[..4], &alone[5..]), (&fragment[..4], &fragment[5..]));
    let toml = ["-c", "mcp_servers.brokkr.command={brokkr}"].map(String::from);
    let served = ["-c", "mcp_servers.brokkr.command=/bin/brokkr"].map(String::from);
    assert_eq!(hands.expand_beside(&toml, &[docs]), Some(served.to_vec()));
}

/// R4's one reading: W learns from a hands document only the `cap-` servers
/// it names, each once, in one closed table; anything else serves none.
#[test]
fn only_a_documents_own_cap_servers_are_brokered() {
    let entry = r#"{"command":"/bin/brokkr","args":[]}"#;
    let table = |names: &[&str]| {
        let entries: Vec<String> = (names.iter())
            .map(|name| format!(r#""{name}":{entry}"#))
            .collect();
        format!(r#"{{"mcpServers":{{{}}}}}"#, entries.join(","))
    };
    assert_eq!(
        brokered(&table(&[
            "brokkr",
            "cap-web",
            "cap-library-docs",
            "capx",
            "cap-"
        ])),
        ["mcp__cap-library-docs", "mcp__cap-web"]
    );
    for unread in [
        table(&["brokkr", "cap-web", "cap-web"]),
        format!(r#"{{"mcpServers":{{"cap-web":{entry}}},"more":1}}"#),
        r#"{"mcpServers":{"cap-web":{"command":"x","args":[],"env":{}}}}"#.to_string(),
        "{hands_mcp_json}".to_string(),
    ] {
        assert_eq!(brokered(&unread), Vec::<String>::new(), "{unread}");
    }
}
