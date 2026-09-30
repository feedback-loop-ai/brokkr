//! The one rule at the verdict (#484): an admitting verdict stands only
//! when every line of both turns' streams was read and every fact it
//! rests on was measured. The chief's four shapes on 891c3c9f, each run
//! through the probe against a Claude-like fake, and the clean streams
//! that keep their admitting verdicts.

use serde_json::{json, Value};

use super::*;

/// A clean plain turn: its own tool is `Bash`, and no MCP server reached
/// it.
const PLAIN_BASH: &str = r#"tools='"Bash"'"#;

/// The session transcript both turns write, and its path in evidence.
const SESSION: &str = "~/.claude/projects/{workdir}/{session}.jsonl";

const BOXED: &str = "boxed offices";
const GRANT: &str = "a seat in a realm that grants its capabilities";

/// One stream shape: the fake's shell on its plain turn and under the
/// hands argv, and the eligibility the probe must derive.
struct Row {
    shape: &'static str,
    plain: String,
    boxed: String,
    eligibility: Value,
}

fn refused(admits: &str, gaps: &[&str]) -> Value {
    json!({
        "verdict": "refused",
        "reason": format!("the evidence for {admits} is not complete: {}", gaps.join("; ")),
    })
}

const NOT_ONE_OBJECT: &str = "is not one JSON object naming each key once";

/// The gaps of a boxed turn whose line `line` of `source`, unread as
/// `fault` says, left its listings unread and so Bash's off switch
/// unmeasured.
fn bash_off_unread(line: usize, source: &str, fault: &str) -> [String; 2] {
    [
        format!("line {line} of the boxed turn's {source} {fault}"),
        format!(
            "Bash's off switch is unmeasured: the boxed turn was not read: the turn's tools \
             could not be read whole: line {line} of {source} {fault}"
        ),
    ]
}

/// The chief's four shapes on 891c3c9f.
fn found_by_the_chief_on_891c3c9f() -> Vec<Row> {
    let whitespace = bash_off_unread(2, "stdout", NOT_ONE_OBJECT);
    // The box's whole append is the joined line, so it holds no event.
    let [joined, joined_off] = bash_off_unread(2, SESSION, NOT_ONE_OBJECT);
    let joined_off = format!("{joined_off}; {SESSION} holds no JSON event");
    let not_utf8 = bash_off_unread(1, "stdout", "is not UTF-8");
    vec![
        Row {
            shape: "a plain init with no tools key beside a clean box",
            plain: r#"printf '{"type":"system","subtype":"init","session_id":"%s","mcp_servers":[]}\n' "$sid"; exit 0"#.to_string(),
            boxed: BOXED_CLEAN.to_string(),
            eligibility: refused(
                BOXED,
                &["tools is unmeasured: no event of the turn listed its tools"],
            ),
        },
        Row {
            shape: "a whitespace line after the boxed init, after a clean plain turn",
            plain: PLAIN_BASH.to_string(),
            boxed: format!("{BOXED_CLEAN}; later=' '"),
            eligibility: refused(GRANT, &[&whitespace[0], &whitespace[1]]),
        },
        Row {
            shape: "a boxed event joined to the plain turn's unfinished line",
            plain: format!(r#"{PLAIN_BASH}; printf '{{}}' >> "$dir/$sid.jsonl""#),
            boxed: BOXED_CLEAN.to_string(),
            eligibility: refused(GRANT, &[&joined, &joined_off]),
        },
        Row {
            shape: "a raw 0xE9 byte inside a string of the boxed init",
            plain: PLAIN_BASH.to_string(),
            boxed: r#"tools='"mcp__brokkr__workspace"'; servers=$(printf '{"name":"brokkr","status":"connected","note":"\351"}')"#.to_string(),
            eligibility: refused(GRANT, &[&not_utf8[0], &not_utf8[1]]),
        },
    ]
}

/// A clean plain turn beside a boxed turn that appends a whitespace line
/// to the session transcript, and the clean streams, read whole and
/// measured, that keep their admitting verdicts.
fn clean_plain_turns() -> Vec<Row> {
    let appended = bash_off_unread(3, SESSION, NOT_ONE_OBJECT);
    vec![
        Row {
            shape: "a whitespace line the box appends to the session, after a clean plain turn",
            plain: PLAIN_BASH.to_string(),
            boxed: format!(r#"{BOXED_CLEAN}; printf '%s\n' ' ' >> "$dir/$sid.jsonl""#),
            eligibility: refused(GRANT, &[&appended[0], &appended[1]]),
        },
        Row {
            shape: "a clean stream, read whole and measured",
            plain: PLAIN_BASH.to_string(),
            boxed: BOXED_CLEAN.to_string(),
            eligibility: json!({
                "verdict": "boxed",
                "reason": "its own tools switch off and the hands MCP server connects",
            }),
        },
        Row {
            shape: "a box that keeps Bash, its refusal read whole",
            plain: PLAIN_BASH.to_string(),
            boxed: r#"tools='"Bash","mcp__brokkr__workspace"'; servers='{"name":"brokkr","status":"connected"}'"#.to_string(),
            eligibility: json!({
                "verdict": "granting-realms-only",
                "reason": format!("no off switch exists for its native capabilities Bash, {GRANTING_REALMS}"),
            }),
        },
    ]
}

#[test]
fn an_admitting_verdict_needs_every_line_of_both_turns_read_and_every_fact_it_rests_on_measured() {
    if !in_its_own_engine(
        "evidence::an_admitting_verdict_needs_every_line_of_both_turns_read_and_every_fact_it_rests_on_measured",
    ) {
        return;
    }
    let world = world();
    let rows = [found_by_the_chief_on_891c3c9f(), clean_plain_turns()];
    for (index, row) in rows.into_iter().flatten().enumerate() {
        let cli = world.fake(
            &format!("claude-{index}"),
            &claude_with("9.9.9", &row.plain, &row.boxed),
        );
        let report = probe(AdapterKind::Claude, &cli, &claude_declared(), &world);
        assert_eq!(
            (row.shape, &report["eligibility"]),
            (row.shape, &row.eligibility)
        );
    }
}
