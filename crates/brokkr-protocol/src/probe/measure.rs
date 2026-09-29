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
use super::observe::{Observation, Transcript, Trial, Written, SCRATCH_PREFIX};
use super::plan::{Plan, NO_SUCH_EFFORT};

mod listing;
mod strict;
mod tools;

/// How much of a refusal's line, or of a value it cannot read, a report
/// keeps.
const EXCERPT_CHARS: usize = 240;

/// The keys a session identifier is announced under.
const SESSION_KEYS: [&str; 2] = ["session_id", "thread_id"];

/// The keys a cost is reported under.
const COST_KEYS: [&str; 2] = ["total_cost_usd", "cost_usd"];

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

/// One JSON object a stream carried, and its line, counted from 1.
struct Event {
    line: usize,
    fields: Map<String, Value>,
}

/// One stream a turn produced, stdout or a transcript: its events, where
/// they were read from, the number of each line that was not one JSON
/// object, and whether it held no bytes at all.
struct Stream {
    source: String,
    events: Vec<Event>,
    unparsed: Vec<usize>,
    empty: bool,
}

/// Every stream a turn produced, stdout first, and the one its events,
/// session, usage and cost are read from.
struct Streams {
    all: Vec<Stream>,
    primary: usize,
}

impl Streams {
    fn primary(&self) -> &Stream {
        &self.all[self.primary]
    }
}

/// A turn as the facts see it: its streams, why they could not be read,
/// or the CLI's refusal of the launch itself.
enum Turn {
    Read(Streams),
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
fn on_turn<T: serde::Serialize>(turn: &Turn, read: impl FnOnce(&Streams) -> Fact<T>) -> Fact<T> {
    match turn {
        Turn::Read(streams) => read(streams),
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

/// `text` read as the stream `source`, its first line being line `first`
/// of what it came from.
fn parse_lines(text: &str, source: &str, first: usize) -> Stream {
    let mut events = Vec::new();
    let mut unparsed = Vec::new();
    for (index, line) in text
        .lines()
        .enumerate()
        .filter(|(_, line)| !line.trim().is_empty())
    {
        match strict::object(line) {
            Some(fields) => events.push(Event {
                line: first + index,
                fields,
            }),
            None => unparsed.push(first + index),
        }
    }
    Stream {
        source: source.to_string(),
        events,
        unparsed,
        empty: text.is_empty(),
    }
}

/// What a launch wrote to a transcript, as a stream: lines it appended
/// keep their numbers in the file, and a file it rewrote is named so.
fn transcript_stream(transcript: &Transcript) -> Stream {
    match transcript.written {
        Written::Created => parse_lines(&transcript.text, &transcript.path, 1),
        Written::Appended { from_line } => {
            parse_lines(&transcript.text, &transcript.path, from_line)
        }
        Written::Rewritten => parse_lines(
            &transcript.text,
            &format!("{}, which the turn rewrote,", transcript.path),
            1,
        ),
    }
}

/// Every stream the turn produced, stdout's and each transcript's, each
/// named with this run's variable parts; its own events are stdout's, or,
/// when stdout carried none, the first transcript's that holds some.
fn read_stream(observation: &Observation) -> Turn {
    let mut all: Vec<Stream> = std::iter::once(parse_lines(&observation.stdout, "stdout", 1))
        .chain(observation.transcripts.iter().map(transcript_stream))
        .collect();
    let Some(primary) = all.iter().position(|stream| !stream.events.is_empty()) else {
        return Turn::Unread(
            "the turn printed no JSON event and wrote no .jsonl transcript under the scratch \
             HOME"
                .to_string(),
        );
    };
    let id = session_id(&all[primary]).map(|(_, _, id)| id);
    for stream in &mut all {
        stream.source = normalise(&stream.source, id.as_deref());
    }
    Turn::Read(Streams { all, primary })
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
/// refusing that argv, which is itself the measurement. A launch that
/// ended with no exit code, by a signal or the deadline, refused nothing.
fn read_boxed(trial: &Trial) -> Turn {
    match trial {
        Trial::Untried(why) => Turn::Unread(why.clone()),
        Trial::Observed(observation) if observation.exit.is_none() => Turn::Unread(format!(
            "the boxed turn did not finish: {}",
            exit_and_excerpt(observation)
        )),
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

/// Every key in `object` and in the objects and arrays beneath it, depth
/// first.
fn walk<'a>(object: &'a Map<String, Value>, pointer: &str, found: &mut Vec<Found<'a>>) {
    for (key, value) in object {
        let here = format!("{pointer}/{key}");
        beneath(value, &here, found);
        found.push(Found {
            pointer: here,
            key,
            value,
            parent: object,
        });
    }
}

/// Every key inside `value`, an array's entries included.
fn beneath<'a>(value: &'a Value, pointer: &str, found: &mut Vec<Found<'a>>) {
    match value {
        Value::Object(inner) => walk(inner, pointer, found),
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                beneath(item, &format!("{pointer}/{index}"), found);
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
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

fn events(stream: &Stream) -> Fact<Events> {
    let mut types = Vec::new();
    for event in &stream.events {
        push_unique(&mut types, event_type(&event.fields));
    }
    let evidence = format!("{} events read from {}", stream.events.len(), stream.source);
    Fact::measured(
        Events {
            source: stream.source.clone(),
            format: "ndjson".to_string(),
            non_json_lines: stream.unparsed.len(),
            types,
        },
        evidence,
    )
}

/// The first event naming a session, the key it used, and the id.
fn session_id(stream: &Stream) -> Option<(String, &'static str, String)> {
    stream.events.iter().find_map(|event| {
        SESSION_KEYS.iter().find_map(|key| {
            let id = event.fields.get(*key)?.as_str()?;
            Some((event_type(&event.fields), *key, id.to_string()))
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
    let mut messages: BTreeMap<&str, Vec<&Map<String, Value>>> = BTreeMap::new();
    let mut unnamed = Vec::new();
    for event in stream.events.iter().map(|event| &event.fields) {
        for found in found_in(event)
            .into_iter()
            .filter(|found| found.key == "usage")
        {
            let Value::Object(counts) = found.value else {
                continue;
            };
            let location = format!("{} {}", event_type(event), found.pointer);
            counters.extend(counts.keys().cloned());
            match found.parent.get("id").and_then(Value::as_str) {
                Some(id) => messages.entry(id).or_default().push(counts),
                None => push_unique(&mut unnamed, location.clone()),
            }
            push_unique(&mut locations, location);
        }
    }
    if locations.is_empty() {
        return Fact::unmeasured("no event of the turn carried a usage object");
    }
    let evidence = format!("the turn reported its usage at {}", locations.join(", "));
    Fact::measured(
        Usage {
            locations,
            counters: counters.into_iter().collect(),
            counting: counting(&messages, &unnamed),
        },
        evidence,
    )
}

/// How usage counts: repeated per message when one message carried the
/// same usage on several events. A message seen on one event, or usage
/// that names no message, reads the same under either counting, so
/// anything else is unmeasured, and the unnamed usage is listed.
fn counting(
    messages: &BTreeMap<&str, Vec<&Map<String, Value>>>,
    unnamed: &[String],
) -> Fact<Counting> {
    let outside = if unnamed.is_empty() {
        String::new()
    } else {
        format!("; the usage at {} named no message", unnamed.join(", "))
    };
    let repeated = messages
        .iter()
        .find(|(_, usages)| usages.len() > 1 && usages.windows(2).all(|pair| pair[0] == pair[1]));
    match repeated {
        Some((id, usages)) => Fact::measured(
            Counting::RepeatedPerMessage,
            format!(
                "message {id} carried the same usage on {} events; count each message once{outside}",
                usages.len()
            ),
        ),
        None => Fact::unmeasured(format!(
            "no message carried the same usage on several events, so nothing showed how usage \
             counts{outside}"
        )),
    }
}

fn cost(stream: &Stream) -> Fact<Vec<String>> {
    let mut locations = Vec::new();
    for event in stream.events.iter().map(|event| &event.fields) {
        for found in found_in(event) {
            if COST_KEYS.contains(&found.key) && found.value.is_number() {
                push_unique(
                    &mut locations,
                    format!("{} {}", event_type(event), found.pointer),
                );
            }
        }
    }
    if locations.is_empty() {
        return Fact::unmeasured("no event carried total_cost_usd or cost_usd");
    }
    let evidence = format!("the turn reported its cost at {}", locations.join(", "));
    Fact::measured(locations, evidence)
}

/// The launch that refused a deliberate mistake, or why there is no
/// refusal to read: `accepted` when it exited 0. A launch that ended with
/// no exit code, by a signal or the probe's deadline, refused nothing.
fn refused<T: serde::Serialize>(trial: &Trial, accepted: String) -> Result<&Observation, Fact<T>> {
    let observation = match trial {
        Trial::Untried(why) => return Err(Fact::unmeasured(why.clone())),
        Trial::Observed(observation) => observation,
    };
    match observation.exit {
        Some(0) => Err(Fact::unmeasured(accepted)),
        None => Err(Fact::unmeasured(format!(
            "no refusal was read: {}",
            exit_and_excerpt(observation)
        ))),
        Some(_) => Ok(observation),
    }
}

/// How one deliberate mistake was refused.
fn refusal(trial: &Trial) -> Fact<Refusal> {
    let accepted = "the CLI exited 0, so there was no refusal to read".to_string();
    match refused(trial, accepted) {
        Ok(observation) => Fact::measured(
            Refusal {
                exit: observation.exit,
                excerpt: excerpt(observation),
            },
            exit_and_excerpt(observation),
        ),
        Err(fact) => fact,
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
    let accepted = format!(
        "the CLI accepted the unknown effort '{NO_SUCH_EFFORT}' and exited 0, so no refusal \
         lists its levels"
    );
    let observation = match refused(trial, accepted) {
        Ok(observation) => observation,
        Err(fact) => return fact,
    };
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
            "the .jsonl files the turn wrote under the scratch HOME",
        )
    }
}

/// The facts one turn's stream shows.
fn stream_facts(
    base: &Turn,
    turn: &Observation,
) -> (Fact<Events>, Fact<Session>, Fact<Vec<String>>) {
    let announced = match base {
        Turn::Read(streams) => session_id(streams.primary()),
        Turn::Unread(_) | Turn::Refused(_) => None,
    };
    let id = announced.as_ref().map(|(_, _, id)| id.as_str());
    let events = on_turn(base, |streams| events(streams.primary()));
    let session = on_turn(base, |_| session(announced.as_ref()));
    let transcripts = on_turn(base, |_| transcripts(turn, id));
    (events, session, transcripts)
}

/// Every fact, from every observation.
pub(crate) fn facts(plan: &Plan, observed: &Observed, bound: &[&str]) -> Facts {
    let base = read_base(&observed.turn);
    let boxed = read_boxed(&observed.boxed);
    let (events, session, transcripts) = stream_facts(&base, &observed.turn);
    let tools = on_turn(&base, tools::native_tools);
    let boxed_tools = on_turn(&boxed, tools::native_tools);
    let native_egress = tools::native_egress(&tools);
    let egress_off = tools::egress_off(&native_egress, &boxed_tools);
    let capabilities = tools::capabilities(&tools, &boxed_tools, plan);
    let user_mcp_unboxed = tools::user_mcp(&base, &plan.user_config);
    let user_mcp_boxed = tools::user_mcp(&boxed, &plan.user_config);
    let config_isolation = on_turn(&base, |_| {
        tools::config_isolation(&user_mcp_unboxed, &user_mcp_boxed)
    });
    Facts {
        headless: Fact::measured(
            Headless {
                argv: plan.turn.clone(),
                exit: observed.turn.exit,
            },
            format!(
                "one turn ran under a scratch HOME with only these credentials bound: [{}]; \
                 {}: {}",
                bound.join(", "),
                plan.unlike_driver,
                exit_text(observed.turn.exit)
            ),
        ),
        events,
        session,
        usage: on_turn(&base, |streams| usage(streams.primary())),
        cost: on_turn(&base, |streams| cost(streams.primary())),
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
        mcp_server: on_turn(&boxed, tools::mcp_server),
        native_egress,
        egress_off,
        capabilities,
        config_isolation,
        user_mcp_unboxed,
        user_mcp_boxed,
        transcripts,
        resume: Fact::unmeasured(
            "the probe does not drive a resume turn yet; decision 0056's per-shape \
             assessment stays the adapter's (#226)",
        ),
    }
}
