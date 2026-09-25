//! Proposed decision 0069 at the renderer: the closed two-identifier
//! declaration, and the one discovery paragraph the result contract
//! carries when — and only when — the engine's carrier applies. Every
//! expected contract below is a literal written out by hand, never the
//! renderer's own output fed back to it.

use super::*;

/// The mandatory contract every model seat reads, up to the hands
/// paragraph, for `/result.json` and the single result `complete`.
const CONTRACT: &str = "## Result contract — MANDATORY\n\nWhen your work is finished, write a \
JSON object to exactly this file:\n\n    /result.json\n\nwith the shape:\n\n    {\"result\": \
\"<one of: complete>\",\n      \"inputs\": { ...optional typed facts for the phase machine... },\n      \
\"notes\": \"<short human summary of what you did and why>\"}\n\nThe file is the ONLY channel the \
engine reads. Printing the JSON instead of writing the file counts as producing no result. The \
object carries exactly these top-level keys — result, inputs, notes — and nothing else: a typed \
fact goes INSIDE inputs, and a record with any other top-level key is refused where it is sealed \
(decision 0034), which loses the whole attempt. You never decide the next phase — the engine's \
policy table rules on your typed result.";

/// Decision 0043's boxed paragraph, byte for byte as it shipped.
const BOXED: &str = "\n\nYour hands are boxed: the worktree, and this result file, are reachable \
ONLY through the `mcp__brokkr__workspace` tool. Your harness's own shell runs outside the box and \
cannot write here — a file written through it never reaches the engine. Write the result file with \
the workspace tool.";

/// The discovery paragraph for the shipped Codex declaration, exactly as
/// the specification's scenario quotes it.
const DISCOVERY: &str = "\n\nYour workspace tool is `mcp__brokkr__workspace`. If it is not \
listed, use `tool_search` to load it before doing workspace work. Native shell and apply_patch \
writes are refused by design; this is not a blocker. Use the workspace tool for all workspace \
writes, including the result file.";

fn codex_notice() -> Value {
    json!({"workspace_tool": "mcp__brokkr__workspace", "discovery_tool": "tool_search"})
}

fn input(extra: Value) -> Value {
    let mut input = json!({
        // A readable, empty charter: these tests author none (#372).
        "role_path": "/dev/null",
        "feature": "feature",
        "phase": "work",
        "workdir": "/work",
        "result_path": "/result.json",
        "context": {},
        "allowed_results": ["complete"],
    });
    for (key, value) in extra.as_object().unwrap() {
        input[key] = value.clone();
    }
    input
}

/// The engine-owned contract: everything from the result contract's
/// heading to the end. The heading is engine text; the tests here author
/// no charter, so its first occurrence is the contract's.
fn contract(prompt: &str) -> &str {
    &prompt[prompt
        .find("## Result contract")
        .expect("a result contract")..]
}

#[test]
fn a_hands_notice_reads_exactly_two_identifiers() {
    let notice = HandsNotice::parse(&codex_notice()).unwrap();
    assert_eq!(notice.workspace_tool(), "mcp__brokkr__workspace");
    assert_eq!(notice.discovery_tool(), "tool_search");
    assert_eq!(notice.to_value(), codex_notice());
    assert_eq!(HandsNotice::MEMBERS, ["workspace_tool", "discovery_tool"]);
}

#[test]
fn every_other_notice_shape_is_refused_with_the_rule_it_broke() {
    let shape = "must be an object with exactly 'workspace_tool' and 'discovery_tool'";
    let grammar = "must match ^[A-Za-z_][A-Za-z0-9_]*$";
    let cases: Vec<(Value, String)> =
        vec![
        (Value::Null, shape.into()),
        (json!(false), shape.into()),
        (json!("mcp__brokkr__workspace"), shape.into()),
        (json!(["mcp__brokkr__workspace", "tool_search"]), shape.into()),
        (
            json!({"workspace_tool": "w", "discovery_tool": "d", "prose": "hi"}),
            "has unknown member 'prose'; only 'workspace_tool' and 'discovery_tool' are allowed"
                .into(),
        ),
        (
            json!({"discovery_tool": "d"}),
            "is missing 'workspace_tool'".into(),
        ),
        (
            json!({"workspace_tool": "w"}),
            "is missing 'discovery_tool'".into(),
        ),
        (
            json!({"workspace_tool": 7, "discovery_tool": "d"}),
            "'workspace_tool' must be a string".into(),
        ),
        (
            json!({"workspace_tool": "w", "discovery_tool": null}),
            "'discovery_tool' must be a string".into(),
        ),
        (
            json!({"workspace_tool": "", "discovery_tool": "d"}),
            "'workspace_tool' must be 1 to 128 ASCII bytes; it is 0 bytes".into(),
        ),
        (
            json!({"workspace_tool": "w", "discovery_tool": ""}),
            "'discovery_tool' must be 1 to 128 ASCII bytes; it is 0 bytes".into(),
        ),
        (
            json!({"workspace_tool": "two words", "discovery_tool": "d"}),
            format!("'workspace_tool' {grammar}"),
        ),
        (
            json!({"workspace_tool": "w", "discovery_tool": "tool\nsearch"}),
            format!("'discovery_tool' {grammar}"),
        ),
        (
            json!({"workspace_tool": "mcp.brokkr", "discovery_tool": "d"}),
            format!("'workspace_tool' {grammar}"),
        ),
        (
            json!({"workspace_tool": "w", "discovery_tool": "tool-search"}),
            format!("'discovery_tool' {grammar}"),
        ),
        (
            json!({"workspace_tool": "9lives", "discovery_tool": "d"}),
            format!("'workspace_tool' {grammar}"),
        ),
        (
            json!({"workspace_tool": "wörkspace", "discovery_tool": "d"}),
            format!("'workspace_tool' {grammar}"),
        ),
        (
            json!({"workspace_tool": "w", "discovery_tool": "`tool_search`"}),
            format!("'discovery_tool' {grammar}"),
        ),
    ];
    for (value, expected) in cases {
        assert_eq!(
            HandsNotice::parse(&value),
            Err(expected),
            "the declaration {value}"
        );
    }
}

#[test]
fn identifier_limits_are_literal() {
    let longest = format!("w{}", "_".repeat(127));
    assert_eq!(longest.len(), 128);
    for (workspace, discovery) in [("w", "_"), (longest.as_str(), longest.as_str())] {
        let notice =
            HandsNotice::parse(&json!({"workspace_tool": workspace, "discovery_tool": discovery}))
                .unwrap();
        assert_eq!(notice.workspace_tool(), workspace);
        assert_eq!(notice.discovery_tool(), discovery);
    }
    let over = format!("{longest}_");
    assert_eq!(
        HandsNotice::parse(&json!({"workspace_tool": over, "discovery_tool": "d"})),
        Err("'workspace_tool' must be 1 to 128 ASCII bytes; it is 129 bytes".into())
    );
    assert_eq!(
        HandsNotice::parse(&json!({"workspace_tool": "w", "discovery_tool": over})),
        Err("'discovery_tool' must be 1 to 128 ASCII bytes; it is 129 bytes".into())
    );
}

#[test]
fn a_boxed_seat_with_the_codex_carrier_reads_exactly_one_discovery_paragraph() {
    for kind in [
        AdapterKind::Codex,
        AdapterKind::Claude,
        AdapterKind::Dsh,
        AdapterKind::Lanetally,
    ] {
        let prompt = render_prompt(
            &input(
                json!({"hands": "boxed", "boundary": "namespace", "hands_notice": codex_notice()}),
            ),
            kind,
        )
        .unwrap();
        assert_eq!(
            contract(&prompt),
            format!("{CONTRACT}{BOXED}{DISCOVERY}\n"),
            "{kind:?}"
        );
        assert_eq!(prompt.matches("Your workspace tool is").count(), 1);
    }
}

#[test]
fn without_an_applicable_carrier_the_boxed_contract_is_todays() {
    // No carrier, and every carrier the renderer cannot read: the boxed
    // contract is decision 0043's, byte for byte.
    for carrier in [
        None,
        Some(Value::Null),
        Some(json!(false)),
        Some(json!("mcp__brokkr__workspace")),
        Some(json!({"workspace_tool": "two words", "discovery_tool": "tool_search"})),
        Some(json!({"workspace_tool": "mcp__brokkr__workspace"})),
        Some(json!({"workspace_tool": "w", "discovery_tool": "d", "prose": "x"})),
    ] {
        let mut extra = json!({"hands": "boxed", "boundary": "namespace"});
        if let Some(carrier) = carrier.clone() {
            extra["hands_notice"] = carrier;
        }
        let prompt = render_prompt(&input(extra), AdapterKind::Codex).unwrap();
        assert_eq!(
            contract(&prompt),
            format!("{CONTRACT}{BOXED}\n"),
            "{carrier:?}"
        );
    }
}

#[test]
fn an_unboxed_or_handless_seat_ignores_even_a_valid_carrier() {
    let harness = "\n\nYour hands stand under the `harness` boundary: no workspace tool of \
Brokkr's is served, and you run under your harness's own sandbox. The result path above is the \
one file that sandbox lets you write; write it yourself.";
    let open = "\n\nYour hands stand under the `open` boundary: nothing of Brokkr's stands \
between you and the machine, and no workspace tool is served. Write the result file yourself.";
    for (extra, tail) in [
        (json!({"hands": "none", "boundary": "harness"}), harness),
        (json!({"hands": "none", "boundary": "open"}), open),
        (json!({"hands": "none", "boundary": "not applicable"}), ""),
        (json!({}), ""),
    ] {
        let mut with = extra.clone();
        with["hands_notice"] = codex_notice();
        let prompt = render_prompt(&input(with), AdapterKind::Codex).unwrap();
        assert_eq!(contract(&prompt), format!("{CONTRACT}{tail}\n"), "{extra}");
    }

    // A harness gate's last-message door keeps its own instructions.
    let door = render_prompt(
        &input(json!({
            "hands": "none", "boundary": "harness", "result_delivery": "last-message",
            "hands_notice": codex_notice(),
        })),
        AdapterKind::Codex,
    )
    .unwrap();
    assert_eq!(
        contract(&door),
        "## Result contract — MANDATORY\n\nWhen your work is finished, your FINAL message \
must be exactly a JSON object, which your harness writes to exactly this file:\n\n    \
/result.json\n\nwith the shape:\n\n    {\"result\": \"<one of: complete>\",\n      \"inputs\": \
{ ...optional typed facts for the phase machine... },\n      \"notes\": \"<short human summary \
of what you did and why>\"}\n\nThe file is the ONLY channel the engine reads; your harness \
writes your final message there, so a final message that is not the bare object counts as \
producing no result. The object carries exactly these top-level keys — result, inputs, notes — \
and nothing else: a typed fact goes INSIDE inputs, and a record with any other top-level key is \
refused where it is sealed (decision 0034), which loses the whole attempt. You never decide the \
next phase — the engine's policy table rules on your typed result.\n\nYour hands stand under the \
`harness` boundary: no workspace tool of Brokkr's is served, and you run under your harness's \
own read-only sandbox. Your FINAL message must be exactly the result object above and nothing \
else — the harness writes that message to the result path, so you do not write the file \
yourself.\n"
    );
}

#[test]
fn an_exec_script_reads_no_discovery_paragraph_even_beside_a_carrier() {
    let prompt = render_prompt(
        &input(json!({"hands": "boxed", "boundary": "namespace", "hands_notice": codex_notice()})),
        AdapterKind::Exec,
    )
    .unwrap();
    assert_eq!(contract(&prompt), format!("{CONTRACT}\n"));
}

#[test]
fn a_declared_custom_workspace_is_the_one_name_the_contract_uses() {
    let prompt = render_prompt(
        &input(json!({
            "hands": "boxed", "boundary": "namespace",
            "hands_notice": {"workspace_tool": "fixture_workspace", "discovery_tool": "fixture_search"},
        })),
        AdapterKind::Codex,
    )
    .unwrap();
    assert_eq!(
        contract(&prompt),
        format!(
            "{CONTRACT}\n\nYour hands are boxed: the worktree, and this result file, are \
reachable ONLY through the `fixture_workspace` tool. Your harness's own shell runs outside the box \
and cannot write here — a file written through it never reaches the engine. Write the result file \
with the workspace tool.\n\nYour workspace tool is `fixture_workspace`. If it is not listed, use \
`fixture_search` to load it before doing workspace work. Native shell and apply_patch writes are \
refused by design; this is not a blocker. Use the workspace tool for all workspace writes, \
including the result file.\n"
        )
    );
    assert!(!prompt.contains("mcp__brokkr__workspace"), "{prompt}");
}
