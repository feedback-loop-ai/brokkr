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
    let hands = Transport {
        brokkr: std::path::Path::new("/bin/brokkr"),
        workdir: std::path::Path::new("/w"),
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
