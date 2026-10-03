//! The probe against the streams the controller recorded from the real
//! CLIs on 2026-10-03 (`measure/streams/`, sanitised), read into facts
//! (#484): each decodes whole and yields what it shows, and the plan's
//! launches are held to claude's grammar, whose recorded failures show
//! a variadic flag taking the prompt.

use super::*;
use crate::probe::plan::{NO_SUCH_MODEL, PROMPT};

const CLAUDE_PLAIN: &str = include_str!("../measure/streams/claude-plain.stdout");
const CLAUDE_NOMODEL: &str = include_str!("../measure/streams/claude-nomodel.stdout");
const CLAUDE_NOMODEL_ERR: &str = include_str!("../measure/streams/claude-nomodel.stderr");
const CLAUDE_OFF_ERR: &str = include_str!("../measure/streams/claude-off.stderr");
const CLAUDE_BOXED_ERR: &str = include_str!("../measure/streams/claude-boxed.stderr");
const CODEX_PLAIN: &str = include_str!("../measure/streams/codex-plain.stdout");
const CODEX_PLAIN_ERR: &str = include_str!("../measure/streams/codex-plain.stderr");
const CODEX_NOMODEL: &str = include_str!("../measure/streams/codex-nomodel.stdout");
const CODEX_NOMODEL_ERR: &str = include_str!("../measure/streams/codex-nomodel.stderr");
const DSH_NOMODEL_ERR: &str = include_str!("../measure/streams/dsh-nomodel.stderr");

/// The facts of `plain` as every turn, and of `bad_model` as the launch
/// with the unknown model.
fn read(
    kind: AdapterKind,
    declared: &Declared,
    plain: Observation,
    bad_model: Observation,
) -> measure::Reading {
    let plan = plan::plan(kind, declared).unwrap();
    let observed = Observed {
        bad_model: Trial::Observed(bad_model),
        ..observed(plain)
    };
    measure::reading(&plan, &observed, &[])
}

/// A recorded refusal, whose excerpt is the line that classed it.
fn refused_on(line: &str) -> Fact<facts::Refusal> {
    let excerpt: String = line.chars().take(240).collect();
    Fact::measured(
        facts::Refusal {
            exit: Some(1),
            excerpt: excerpt.clone(),
        },
        format!("exit 1: {excerpt}"),
    )
}

#[test]
fn claude_s_recorded_turns_are_read_whole_into_the_facts_they_show() {
    let reading = read(
        AdapterKind::Claude,
        &claude_declared(),
        observation(Some(0), CLAUDE_PLAIN, ""),
        observation(Some(1), CLAUDE_NOMODEL, CLAUDE_NOMODEL_ERR),
    );
    assert_eq!(reading.unread, []);
    let facts = reading.facts;
    let types = [
        "system/hook_started",
        "system/hook_response",
        "system/init",
        "assistant",
        "rate_limit_event",
        "result/success",
    ];
    assert_eq!(
        facts.events.value().map(|events| &events.types),
        Some(&strings(&types))
    );
    let tools = facts.tools.value().unwrap();
    assert_eq!(
        (
            tools.len(),
            &tools[..2],
            tools.contains(&"WebSearch".to_string())
        ),
        (28, &strings(&["Task", "Bash"])[..], true)
    );
    assert_eq!(
        facts.user_mcp_unboxed,
        Fact::measured(
            true,
            "mcp__server__tool_28 of the MCP server server was listed by the system/init event \
             on line 3 of stdout at /tools/28"
        )
    );
    assert_eq!(
        facts
            .usage
            .value()
            .map(|usage| (&usage.locations, &usage.counters)),
        Some((
            &strings(&["assistant /message/usage", "result/success /usage"]),
            &strings(&[
                "cache_creation_input_tokens",
                "cache_read_input_tokens",
                "input_tokens",
                "output_tokens",
            ]),
        ))
    );
    assert_eq!(facts.refusals.config, refused_on(CLAUDE_NOMODEL_ERR.trim()));
}

#[test]
fn codex_s_recorded_turns_are_read_whole_into_the_facts_they_show() {
    let reading = read(
        AdapterKind::Codex,
        &codex_declared(),
        observation(Some(0), CODEX_PLAIN, CODEX_PLAIN_ERR),
        observation(Some(1), CODEX_NOMODEL, CODEX_NOMODEL_ERR),
    );
    assert_eq!(reading.unread, []);
    let facts = reading.facts;
    assert_eq!(
        facts
            .usage
            .value()
            .map(|usage| (&usage.locations, &usage.counters)),
        Some((
            &strings(&["turn.completed /usage"]),
            &strings(&[
                "cache_write_input_tokens",
                "cached_input_tokens",
                "input_tokens",
                "output_tokens",
                "reasoning_output_tokens",
            ]),
        ))
    );
    assert_eq!(
        facts
            .session
            .value()
            .map(|session| (&session.event, &session.key)),
        Some((&"thread.started".to_string(), &"thread_id".to_string()))
    );
    // The provider's refusal, not the warning before it, classes the
    // launch.
    let error = CODEX_NOMODEL.lines().nth(3).unwrap();
    assert_eq!(facts.refusals.config, refused_on(error));
}

/// dsh 0.1.5-rc.1 has no `--model`: the recorded refusal is of the flag,
/// a control, which refuses the controls that name it and is not the
/// model's refusal.
#[test]
fn dsh_s_recorded_refusal_of_its_model_flag_is_a_refused_control_not_a_configuration_one() {
    let declared = Declared {
        native: Native::Known {
            powers: Vec::new(),
            off: OffControl::Argv(strings(&["--model", NO_SUCH_MODEL])),
        },
        ..dsh_declared()
    };
    let refusal = observation(Some(1), "", DSH_NOMODEL_ERR);
    let plan = plan::plan(AdapterKind::Dsh, &declared).unwrap();
    let observed = Observed {
        bad_model: Trial::Observed(refusal.clone()),
        native_off: Trial::Observed(refusal),
        ..observed(observation(Some(0), "PROBE-OK", ""))
    };
    let facts = measure::reading(&plan, &observed, &[]).facts;
    let line = "error: unknown option '--model'";
    assert_eq!(
        (facts.user_mcp_off, facts.refusals.config),
        (
            Fact::Unsupported {
                evidence: format!("the CLI refused the declared OFF controls: exit 1: {line}"),
            },
            Fact::unmeasured(format!(
                "no line of the refusal is a configuration refusal of the model {NO_SUCH_MODEL}, \
                 so it is not shown to be the configuration's: exit 1: {line}"
            )),
        )
    );
}

/// claude's variadic flags, `<values...>` in its `--help`: each takes
/// every argument after it up to the next flag.
const VARIADIC: [&str; 4] = [
    "--tools",
    "--allowedTools",
    "--disallowedTools",
    "--mcp-config",
];

/// claude's flags that take one value.
const VALUED: [&str; 4] = [
    "--output-format",
    "--permission-mode",
    "--model",
    "--effort",
];

/// What commander leaves claude as its prompt of `argv`: the arguments
/// no flag takes.
fn prompt_of(argv: &[String]) -> Vec<&str> {
    let mut takes = (false, false);
    let mut prompt = Vec::new();
    for arg in &argv[1..] {
        if arg.starts_with('-') {
            takes = (
                VALUED.contains(&arg.as_str()),
                VARIADIC.contains(&arg.as_str()),
            );
            continue;
        }
        match takes {
            (true, _) => takes = (false, false),
            (false, true) => {}
            (false, false) => prompt.push(arg.as_str()),
        }
    }
    prompt
}

/// The plan's turn under claude's variadic flags (#484). Recorded on
/// 2026-10-03, a prompt after `--disallowedTools` left the turn with none,
/// and one after `--mcp-config` was read as a config file's path, as the
/// grammar says: so every launch the plan composes gives its prompt
/// before any flag that could take it.
#[test]
fn no_claude_launch_gives_its_prompt_where_a_variadic_flag_takes_it() {
    let head = ["{cli}", "-p", "--output-format", "stream-json", "--verbose"];
    let recorded_off = [
        &head[..],
        &["--disallowedTools", "WebFetch,WebSearch", "{prompt}"],
    ];
    let recorded_boxed = [
        &head[..],
        &[
            "--tools",
            "",
            "--strict-mcp-config",
            "--mcp-config",
            r#"{"mcpServers":{}}"#,
            "{prompt}",
        ],
    ];
    let recorded = [recorded_off.concat(), recorded_boxed.concat()].map(|argv| strings(&argv));
    assert_eq!(
        recorded.each_ref().map(|argv| prompt_of(argv)),
        [Vec::<&str>::new(), Vec::new()]
    );
    assert_eq!(
        (
            CLAUDE_OFF_ERR.lines().next(),
            CLAUDE_BOXED_ERR.lines().nth(1)
        ),
        (
            Some(
                "Error: Input must be provided either through stdin or as a prompt argument when \
                 using --print"
            ),
            Some(format!("MCP config file not found: /work/{PROMPT}").as_str()),
        )
    );
    let plan = plan::plan(AdapterKind::Claude, &claude_declared()).unwrap();
    let launches = [plan.bad_model, plan.bad_effort, plan.boxed, plan.native_off];
    let mut argvs = vec![plan.turn];
    argvs.extend(launches.into_iter().map(|step| match step {
        plan::Step::Launch(argv) => argv,
        plan::Step::Untried(why) => panic!("{why}"),
    }));
    assert_eq!(
        argvs.iter().map(|argv| prompt_of(argv)).collect::<Vec<_>>(),
        vec![vec!["{prompt}"]; 5]
    );
}
