//! Reading the observations into facts. Pure: nothing here runs, reads
//! or writes anything; `observe` did, and handed over masked text.
//!
//! A CLI's output is the edge (decision 0071 ruling 3). Its lines are
//! parsed into JSON here, walked for the keys every harness so far names
//! its facts under, and left behind as the typed facts of `facts`. What
//! the walk does not recognise is `unmeasured`, never a default.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{Map, Value};

use super::facts::{Counting, Events, Fact, Facts, Headless, Refusal, Refusals, Session, Usage};
use super::observe::{Observation, Trial, SCRATCH_PREFIX};
use super::plan::{Plan, UserConfig, NO_SUCH_EFFORT, USER_SCOPE_SERVER};
use crate::hands::SERVER_NAME;

/// How much of a refusal's line a report keeps.
const EXCERPT_CHARS: usize = 240;

/// The keys a session identifier is announced under.
const SESSION_KEYS: [&str; 2] = ["session_id", "thread_id"];

/// The keys a cost is reported under.
const COST_KEYS: [&str; 2] = ["total_cost_usd", "cost_usd"];

/// What a native egress tool's name holds, once folded to lowercase
/// letters and digits: web search, fetch, browsing and grounding.
const EGRESS_WORDS: [&str; 4] = ["web", "fetch", "browse", "grounding"];

/// What refusals list the accepted levels after: clap's, commander's and
/// serde's wording.
const LEVEL_MARKERS: [&str; 3] = ["possible values:", "Allowed choices are", "expected one of"];

/// Every launch of one probe run, observed or passed on.
pub(crate) struct Observed {
    pub(crate) version: Observation,
    pub(crate) turn: Observation,
    pub(crate) no_credentials: Trial,
    pub(crate) bad_model: Trial,
    pub(crate) bad_effort: Trial,
    pub(crate) boxed: Trial,
}

/// A turn's events, and where they were read from.
struct Stream {
    source: String,
    events: Vec<Map<String, Value>>,
    non_json: usize,
}

/// A turn as the facts see it: its events, why they could not be read,
/// or the CLI's refusal of the launch itself.
enum Turn {
    Read(Stream),
    Unread(String),
    Refused(String),
}

impl<T: serde::Serialize> Fact<T> {
    /// The same reading with its value mapped; evidence and reasons kept.
    pub(crate) fn map<U>(self, change: impl FnOnce(T) -> U) -> Fact<U> {
        match self {
            Fact::Measured { value, evidence } => Fact::Measured {
                value: change(value),
                evidence,
            },
            Fact::Unmeasured { why } => Fact::Unmeasured { why },
            Fact::Unsupported { evidence } => Fact::Unsupported { evidence },
        }
    }
}

/// A fact read from a turn: measured by `read` when the turn was read,
/// unmeasured when it was not, unsupported when the CLI refused it.
fn on_turn<T: serde::Serialize>(turn: &Turn, read: impl FnOnce(&Stream) -> Fact<T>) -> Fact<T> {
    match turn {
        Turn::Read(stream) => read(stream),
        Turn::Unread(why) => Fact::unmeasured(why.clone()),
        Turn::Refused(evidence) => Fact::Unsupported {
            evidence: evidence.clone(),
        },
    }
}

fn exit_text(exit: Option<i32>) -> String {
    match exit {
        Some(code) => format!("exit {code}"),
        None => "no exit code (a signal, or the probe's deadline)".to_string(),
    }
}

/// The line that says what happened: stderr's first, else stdout's last.
pub(crate) fn excerpt(observation: &Observation) -> String {
    let stderr = observation
        .stderr
        .lines()
        .find(|line| !line.trim().is_empty());
    let stdout = observation
        .stdout
        .lines()
        .rfind(|line| !line.trim().is_empty());
    let line = stderr.or(stdout).unwrap_or("(no output)").trim();
    line.chars().take(EXCERPT_CHARS).collect()
}

fn exit_and_excerpt(observation: &Observation) -> String {
    format!("{}: {}", exit_text(observation.exit), excerpt(observation))
}

/// The first line `--version` printed, when it exited clean.
pub(crate) fn version(observation: &Observation) -> Fact<String> {
    let first = observation.stdout.lines().next().unwrap_or("").trim();
    if observation.exit == Some(0) && !first.is_empty() {
        Fact::measured(first.to_string(), "the first line `--version` printed")
    } else {
        Fact::unmeasured(format!(
            "`--version` printed no version: {}",
            exit_and_excerpt(observation)
        ))
    }
}

fn parse_lines(text: &str, source: &str) -> Stream {
    let mut events = Vec::new();
    let mut non_json = 0;
    for line in text.lines().filter(|line| !line.trim().is_empty()) {
        match serde_json::from_str::<Value>(line) {
            Ok(Value::Object(event)) => events.push(event),
            _ => non_json += 1,
        }
    }
    Stream {
        source: source.to_string(),
        events,
        non_json,
    }
}

/// The turn's events: stdout's, or, when stdout carried none, the first
/// transcript the turn wrote that holds some.
fn read_stream(observation: &Observation) -> Turn {
    std::iter::once(parse_lines(&observation.stdout, "stdout"))
        .chain(
            observation
                .transcripts
                .iter()
                .map(|transcript| parse_lines(&transcript.text, &transcript.path)),
        )
        .find(|stream| !stream.events.is_empty())
        .map_or_else(
            || {
                Turn::Unread(
                    "the turn printed no JSON event and wrote no .jsonl transcript under \
                     the scratch HOME"
                        .to_string(),
                )
            },
            Turn::Read,
        )
}

/// The plain turn: read when it exited clean, otherwise nothing it shows
/// is a measurement.
fn read_base(observation: &Observation) -> Turn {
    if observation.exit == Some(0) {
        read_stream(observation)
    } else {
        Turn::Unread(format!(
            "the headless turn did not succeed: {}",
            exit_and_excerpt(observation)
        ))
    }
}

/// The turn under the adapter's hands argv: a non-zero exit is the CLI
/// refusing that argv, which is itself the measurement.
fn read_boxed(trial: &Trial) -> Turn {
    match trial {
        Trial::Untried(why) => Turn::Unread(why.clone()),
        Trial::Observed(observation) if observation.exit != Some(0) => Turn::Refused(format!(
            "the CLI refused the adapter's hands argv: {}",
            exit_and_excerpt(observation)
        )),
        Trial::Observed(observation) => read_stream(observation),
    }
}

fn event_type(event: &Map<String, Value>) -> String {
    let kind = event
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("(untyped)");
    match event.get("subtype").and_then(Value::as_str) {
        Some(subtype) => format!("{kind}/{subtype}"),
        None => kind.to_string(),
    }
}

/// One key found anywhere in an event: its JSON pointer, its value, and
/// the object that holds it.
struct Found<'a> {
    pointer: String,
    key: &'a str,
    value: &'a Value,
    parent: &'a Map<String, Value>,
}

/// Every key in `object` and the objects beneath it, depth first.
fn walk<'a>(object: &'a Map<String, Value>, pointer: &str, found: &mut Vec<Found<'a>>) {
    for (key, value) in object {
        let here = format!("{pointer}/{key}");
        if let Value::Object(inner) = value {
            walk(inner, &here, found);
        }
        found.push(Found {
            pointer: here,
            key,
            value,
            parent: object,
        });
    }
}

fn found_in(event: &Map<String, Value>) -> Vec<Found<'_>> {
    let mut found = Vec::new();
    walk(event, "", &mut found);
    found
}

fn push_unique(list: &mut Vec<String>, item: String) {
    if !list.contains(&item) {
        list.push(item);
    }
}

fn events(stream: &Stream, source: String) -> Fact<Events> {
    let mut types = Vec::new();
    for event in &stream.events {
        push_unique(&mut types, event_type(event));
    }
    let evidence = format!("{} events read from {source}", stream.events.len());
    Fact::measured(
        Events {
            source,
            format: "ndjson".to_string(),
            non_json_lines: stream.non_json,
            types,
        },
        evidence,
    )
}

/// The first event naming a session, the key it used, and the id.
fn session_id(stream: &Stream) -> Option<(String, &'static str, String)> {
    stream.events.iter().find_map(|event| {
        SESSION_KEYS.iter().find_map(|key| {
            let id = event.get(*key)?.as_str()?;
            Some((event_type(event), *key, id.to_string()))
        })
    })
}

fn session(announced: Option<&(String, &'static str, String)>) -> Fact<Session> {
    match announced {
        Some((event, key, id)) => Fact::measured(
            Session {
                event: event.clone(),
                key: key.to_string(),
            },
            format!("the {event} event announced {key} {id}"),
        ),
        None => Fact::unmeasured("no event named a session_id or thread_id"),
    }
}

/// Where usage sat, which counters it named, and whether one message's
/// usage was restated on several events (#402).
fn usage(stream: &Stream) -> Fact<Usage> {
    let mut locations = Vec::new();
    let mut counters = BTreeSet::new();
    let mut messages: BTreeMap<String, usize> = BTreeMap::new();
    for event in &stream.events {
        for found in found_in(event)
            .into_iter()
            .filter(|found| found.key == "usage")
        {
            let Value::Object(counts) = found.value else {
                continue;
            };
            push_unique(
                &mut locations,
                format!("{} {}", event_type(event), found.pointer),
            );
            counters.extend(counts.keys().cloned());
            if let Some(id) = found.parent.get("id").and_then(Value::as_str) {
                *messages.entry(id.to_string()).or_default() += 1;
            }
        }
    }
    if locations.is_empty() {
        return Fact::unmeasured("no event of the turn carried a usage object");
    }
    let (counting, evidence) = match messages.iter().find(|(_, events)| **events > 1) {
        Some((id, events)) => (
            Counting::RepeatedPerMessage,
            format!("message {id} carried its usage on {events} events; count each message once"),
        ),
        None => (
            Counting::PerEvent,
            "no two usage-bearing events named the same message".to_string(),
        ),
    };
    let counters = counters.into_iter().collect();
    Fact::measured(
        Usage {
            locations,
            counters,
            counting,
        },
        evidence,
    )
}

fn cost(stream: &Stream) -> Fact<Vec<String>> {
    let mut locations = Vec::new();
    for event in &stream.events {
        for found in found_in(event) {
            if COST_KEYS.contains(&found.key) && found.value.is_number() {
                push_unique(
                    &mut locations,
                    format!("{} {}", event_type(event), found.pointer),
                );
            }
        }
    }
    let evidence = if locations.is_empty() {
        "no event carried total_cost_usd or cost_usd".to_string()
    } else {
        format!("the turn reported its cost at {}", locations.join(", "))
    };
    Fact::measured(locations, evidence)
}

/// The first array any event holds under `key`, with that event's type.
fn first_list<'a>(stream: &'a Stream, key: &str) -> Option<(String, &'a Vec<Value>)> {
    stream.events.iter().find_map(|event| {
        let list = found_in(event)
            .into_iter()
            .find_map(|found| found.value.as_array().filter(|_| found.key == key))?;
        Some((event_type(event), list))
    })
}

/// A list's entries read by `entry`, refusing the whole list when one
/// entry is not what the reader recognises.
fn listed<T: serde::Serialize>(
    stream: &Stream,
    key: &str,
    entry: impl Fn(&Value) -> Option<T>,
) -> Fact<Vec<T>> {
    let Some((event, items)) = first_list(stream, key) else {
        return Fact::unmeasured(format!("no event of the turn listed its {key}"));
    };
    match items.iter().map(entry).collect::<Option<Vec<T>>>() {
        Some(entries) => {
            let evidence = format!("the {event} event listed {key}: {}", entries.len());
            Fact::measured(entries, evidence)
        }
        None => Fact::unmeasured(format!(
            "the {event} event's {key} held an entry it does not name"
        )),
    }
}

fn tool_name(item: &Value) -> Option<String> {
    match item {
        Value::String(name) => Some(name.clone()),
        other => other.get("name")?.as_str().map(str::to_string),
    }
}

fn server_entry(item: &Value) -> Option<(String, String)> {
    let name = item.get("name")?.as_str()?;
    let status = item.get("status")?.as_str()?;
    Some((name.to_string(), status.to_string()))
}

/// The CLI's own tools: every listed tool but an MCP server's.
fn native_tools(stream: &Stream) -> Fact<Vec<String>> {
    listed(stream, "tools", tool_name).map(|tools| {
        tools
            .into_iter()
            .filter(|tool| !tool.starts_with("mcp__"))
            .collect()
    })
}

fn is_egress(tool: &str) -> bool {
    let folded: String = tool
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase();
    EGRESS_WORDS.iter().any(|word| folded.contains(word))
}

fn mcp_server(stream: &Stream) -> Fact<String> {
    listed(stream, "mcp_servers", server_entry).map(|servers| {
        servers
            .into_iter()
            .find(|(name, _)| name == SERVER_NAME)
            .map_or_else(|| "not listed".to_string(), |(_, status)| status)
    })
}

fn user_mcp(turn: &Turn, config: &UserConfig) -> Fact<bool> {
    match config {
        UserConfig::Unknown(why) => Fact::unmeasured(*why),
        UserConfig::Planted { .. } => on_turn(turn, |stream| {
            listed(stream, "mcp_servers", server_entry)
                .map(|servers| servers.iter().any(|(name, _)| name == USER_SCOPE_SERVER))
        }),
    }
}

/// Whether the hands argv left any native egress tool behind.
fn egress_off(native_egress: &Fact<Vec<String>>, boxed_tools: &Fact<Vec<String>>) -> Fact<bool> {
    let Some(egress) = native_egress.value() else {
        return Fact::unmeasured("the plain turn's tools were not read");
    };
    if egress.is_empty() {
        return Fact::measured(true, "the plain turn listed no native egress tool");
    }
    match boxed_tools {
        Fact::Measured { value: left, .. } => {
            let kept: Vec<&String> = left.iter().filter(|tool| egress.contains(tool)).collect();
            let evidence = if kept.is_empty() {
                format!("the hands argv removed {}", egress.join(", "))
            } else {
                format!(
                    "the hands argv left {}",
                    kept.iter()
                        .map(|t| t.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            Fact::measured(kept.is_empty(), evidence)
        }
        Fact::Unmeasured { why } => Fact::unmeasured(format!("the boxed turn was not read: {why}")),
        Fact::Unsupported { evidence } => Fact::Unsupported {
            evidence: evidence.clone(),
        },
    }
}

/// How one deliberate mistake was refused.
fn refusal(trial: &Trial) -> Fact<Refusal> {
    match trial {
        Trial::Untried(why) => Fact::unmeasured(why.clone()),
        Trial::Observed(observation) if observation.exit == Some(0) => {
            Fact::unmeasured("the CLI exited 0, so there was no refusal to read")
        }
        Trial::Observed(observation) => Fact::measured(
            Refusal {
                exit: observation.exit,
                excerpt: excerpt(observation),
            },
            exit_and_excerpt(observation),
        ),
    }
}

/// The accepted levels a refusal lists after one of its markers.
fn accepted_levels(text: &str) -> Option<Vec<String>> {
    let rest = text.lines().find_map(|line| {
        LEVEL_MARKERS
            .iter()
            .find_map(|marker| line.split_once(marker).map(|(_, rest)| rest))
    })?;
    let levels: Vec<String> = rest
        .replace(" or ", ",")
        .split(',')
        .filter_map(|item| item.split_whitespace().next())
        .map(|word| {
            word.trim_matches(|c: char| !c.is_ascii_alphanumeric())
                .to_string()
        })
        .filter(|level| !level.is_empty())
        .collect();
    Some(levels).filter(|levels| !levels.is_empty())
}

fn efforts(trial: &Trial) -> Fact<Vec<String>> {
    let observation = match trial {
        Trial::Untried(why) => return Fact::unmeasured(why.clone()),
        Trial::Observed(observation) => observation,
    };
    if observation.exit == Some(0) {
        return Fact::unmeasured(format!(
            "the CLI accepted the unknown effort '{NO_SUCH_EFFORT}' and exited 0, so no \
             refusal lists its levels"
        ));
    }
    let text = format!("{}\n{}", observation.stderr, observation.stdout);
    match accepted_levels(&text) {
        Some(levels) => Fact::measured(levels, exit_and_excerpt(observation)),
        None => Fact::unmeasured(format!(
            "the refusal names no accepted levels: {}",
            exit_and_excerpt(observation)
        )),
    }
}

/// A transcript path with this run's variable parts named: the session
/// id, the component naming the scratch repository, and every digit run.
pub(crate) fn normalise(path: &str, session: Option<&str>) -> String {
    let path = match session {
        Some(id) => path.replace(id, "{session}"),
        None => path.to_string(),
    };
    path.split('/')
        .map(|part| {
            if part.contains(SCRATCH_PREFIX) {
                "{workdir}".to_string()
            } else {
                digit_runs(part)
            }
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn digit_runs(part: &str) -> String {
    let mut out = String::new();
    let mut in_run = false;
    for c in part.chars() {
        let digit = c.is_ascii_digit();
        if digit && !in_run {
            out.push_str("{n}");
        } else if !digit {
            out.push(c);
        }
        in_run = digit;
    }
    out
}

fn transcripts(observation: &Observation, session: Option<&str>) -> Fact<Vec<String>> {
    let paths: Vec<String> = observation
        .transcripts
        .iter()
        .map(|transcript| normalise(&transcript.path, session))
        .collect();
    if paths.is_empty() {
        Fact::unmeasured("the turn wrote no .jsonl file under the scratch HOME")
    } else {
        Fact::measured(
            paths,
            "the .jsonl files the turn created under the scratch HOME",
        )
    }
}

/// The facts one turn's stream shows.
fn stream_facts(
    base: &Turn,
    turn: &Observation,
) -> (Fact<Events>, Fact<Session>, Fact<Vec<String>>) {
    let announced = match base {
        Turn::Read(stream) => session_id(stream),
        Turn::Unread(_) | Turn::Refused(_) => None,
    };
    let id = announced.as_ref().map(|(_, _, id)| id.as_str());
    let events = on_turn(base, |stream| events(stream, normalise(&stream.source, id)));
    let session = on_turn(base, |_| session(announced.as_ref()));
    let transcripts = on_turn(base, |_| transcripts(turn, id));
    (events, session, transcripts)
}

/// Every fact, from every observation.
pub(crate) fn facts(plan: &Plan, observed: &Observed, bound: &[&str]) -> Facts {
    let base = read_base(&observed.turn);
    let boxed = read_boxed(&observed.boxed);
    let (events, session, transcripts) = stream_facts(&base, &observed.turn);
    let tools = on_turn(&base, native_tools);
    let boxed_tools = on_turn(&boxed, native_tools);
    let native_egress = tools
        .clone()
        .map(|tools| tools.into_iter().filter(|tool| is_egress(tool)).collect());
    let egress_off = egress_off(&native_egress, &boxed_tools);
    let config_isolation = on_turn(&base, |_| {
        Fact::measured(
            true,
            format!(
                "a turn ran under a scratch HOME with only these credentials bound: [{}]",
                bound.join(", ")
            ),
        )
    });
    Facts {
        headless: Fact::measured(
            Headless {
                argv: plan.turn.clone(),
                exit: observed.turn.exit,
            },
            format!(
                "one turn ran with stdin closed: {}",
                exit_text(observed.turn.exit)
            ),
        ),
        events,
        session,
        usage: on_turn(&base, usage),
        cost: on_turn(&base, cost),
        refusals: Refusals {
            auth: refusal(&observed.no_credentials),
            config: refusal(&observed.bad_model),
            rate_limit: Fact::unmeasured(
                "not provoked: a rate limit spends quota and risks the account",
            ),
            outage: Fact::unmeasured("not provoked: a provider outage cannot be caused safely"),
        },
        efforts: efforts(&observed.bad_effort),
        tools,
        boxed_tools,
        mcp_server: on_turn(&boxed, mcp_server),
        native_egress,
        egress_off,
        config_isolation,
        user_mcp_unboxed: user_mcp(&base, &plan.user_config),
        user_mcp_boxed: user_mcp(&boxed, &plan.user_config),
        transcripts,
        resume: Fact::unmeasured(
            "the probe does not drive a resume turn yet; decision 0056's per-shape \
             assessment stays the adapter's (#226)",
        ),
    }
}
