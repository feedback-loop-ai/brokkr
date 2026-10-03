//! Reading the observations into facts. Pure: nothing here runs, reads
//! or writes anything; `observe` did, and handed over masked text.
//!
//! A CLI's output is the edge (decision 0071 ruling 3). Its lines are
//! parsed into JSON here, walked for the keys every harness so far names
//! its facts under, and left behind as the typed facts of `facts`. What
//! the walk does not recognise is `unmeasured`, never a default.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::{Map, Value};

use super::facts::{Counting, Events, Fact, Facts, Headless, Refusals, Session, Usage};
use super::observe::{Captured, Observation, Transcript, Trial, Written, SCRATCH_PREFIX};
use super::plan::Plan;

mod listing;
mod refusals;
mod strict;
mod text;
mod tools;

/// How much of a refusal's line, or of a value it cannot read, a report
/// keeps.
const EXCERPT_CHARS: usize = 240;

/// The keys a session identifier is announced under.
const SESSION_KEYS: [&str; 2] = ["session_id", "thread_id"];

/// The keys a cost is reported under.
const COST_KEYS: [&str; 2] = ["total_cost_usd", "cost_usd"];

/// Every launch of one probe run, observed or passed on.
pub(crate) struct Observed {
    pub(crate) version: Observation,
    pub(crate) turn: Observation,
    pub(crate) no_credentials: Trial,
    pub(crate) bad_model: Trial,
    pub(crate) bad_effort: Trial,
    pub(crate) boxed: Trial,
    pub(crate) native_off: Trial,
}

/// One JSON object a stream carried, and its line, counted from 1.
struct Event {
    line: usize,
    fields: Map<String, Value>,
}

/// Why a line of a stream went unread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Fault {
    /// It decoded, but is not one JSON object naming each key once: a
    /// line cut short, or joined to another, among them.
    NotOneObject,
    /// Its bytes are not UTF-8, so it was never decoded, nor repaired.
    NotUtf8,
    /// It is text naming a listing, an MCP server or a tool that the
    /// probe cannot read whole.
    Unrecognised,
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Fault::NotOneObject => "is not one JSON object naming each key once",
            Fault::NotUtf8 => "is not UTF-8",
            Fault::Unrecognised => {
                "names a listing, an MCP server or a tool the probe cannot read whole"
            }
        })
    }
}

/// How a stream's lines are read (#484). Stdout and a transcript are
/// event streams, where a line that is not one JSON object is unread.
/// Stderr is text, read line by line, since a CLI prints its ordinary
/// warnings there:
///
/// - (i) a line that is not UTF-8 is unread, named by stream and line,
///   and refuses any admitting verdict;
/// - (ii) a line that is one JSON object is read as an event, like a
///   line of stdout;
/// - (iii) any other line is text, its whitespace runs folded, read by
///   `text::said`: one naming the planted server, a tool
///   `mcp__<server>__…` or an `MCP server <name>` of a server other than
///   the hands server is a reach;
/// - (iv) one that starts JSON it does not hold whole, or mentions MCP or
///   a tool in any case, or names a tool the plain turn listed or the
///   adapter declares, and shows no reach, is unread like a line of (i)
///   unless it is one of the forms `text::said` reads to its end;
/// - (v) any other line of text is read: it leaves no listing unmeasured.
#[derive(Clone, Copy, PartialEq)]
enum Lines {
    Events,
    Text,
}

/// One stream a turn produced, stdout, a transcript or stderr: how its
/// lines are read, its events, where they were read from, each reach a
/// line of its text showed, each line no reader read and why, and whether
/// it held no bytes at all.
struct Stream {
    source: String,
    lines: Lines,
    events: Vec<Event>,
    reaches: Vec<(usize, String)>,
    unread: Vec<(usize, Fault)>,
    empty: bool,
}

/// The three turns whose streams are read as events: the plain turn, the
/// boxed one and the one under the declared OFF controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TurnName {
    Plain,
    Boxed,
    Off,
}

/// Something a turn captured that no reader read, which leaves the
/// probe's evidence incomplete (#484): a line of one of its streams, or a
/// value one of its listings holds.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Unread {
    Line {
        turn: TurnName,
        source: String,
        line: usize,
        fault: Fault,
    },
    /// `what` names the value by its stream, event and pointer.
    Value { turn: TurnName, what: String },
}

impl fmt::Display for Unread {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let named = |turn: &TurnName| match turn {
            TurnName::Plain => "the plain turn",
            TurnName::Boxed => "the boxed turn",
            TurnName::Off => "the OFF turn",
        };
        match self {
            Unread::Line {
                turn,
                source,
                line,
                fault,
            } => write!(
                formatter,
                "line {line} of {}'s {source} {fault}",
                named(turn)
            ),
            Unread::Value { turn, what } => write!(formatter, "in {}, {what}", named(turn)),
        }
    }
}

/// Every fact, and everything the three turns captured that no reader
/// read.
pub(crate) struct Reading {
    pub(crate) facts: Facts,
    pub(crate) unread: Vec<Unread>,
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
/// or the CLI's refusal of the launch itself. A turn that failed keeps
/// every stream it captured beside the reason, since each of their lines
/// is read whatever the exit (#484).
enum Turn {
    Read(Streams),
    Unread(String, Vec<Stream>),
    Refused(String, Vec<Stream>),
}

impl Turn {
    /// Every stream the turn captured, however it ended.
    fn streams(&self) -> &[Stream] {
        match self {
            Turn::Read(streams) => &streams.all,
            Turn::Unread(_, streams) | Turn::Refused(_, streams) => streams,
        }
    }
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
        Turn::Unread(why, _) => Fact::unmeasured(why.clone()),
        Turn::Refused(evidence, _) => Fact::Unsupported {
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
        .text
        .lines()
        .find(|line| !line.trim().is_empty());
    let stdout = observation
        .stdout
        .text
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
    let first = observation.stdout.text.lines().next().unwrap_or("").trim();
    if observation.exit == Some(0) && !first.is_empty() {
        Fact::measured(first.to_string(), "the first line `--version` printed")
    } else {
        Fact::unmeasured(format!(
            "`--version` printed no version: {}",
            exit_and_excerpt(observation)
        ))
    }
}

/// `captured` read as the stream `source`, its first line being line
/// `first` of what it came from: a line that did not decode is unread,
/// and never parsed, and one that is no event is unread or text, as
/// `lines` says, text checked for the names in `tools`.
fn parse_lines(
    captured: &Captured,
    source: &str,
    first: usize,
    lines: Lines,
    tools: &[String],
) -> Stream {
    let mut events = Vec::new();
    let mut reaches = Vec::new();
    let mut unread: Vec<(usize, Fault)> = captured
        .not_utf8
        .iter()
        .map(|line| (first - 1 + line, Fault::NotUtf8))
        .collect();
    for (index, line) in captured.text.lines().enumerate() {
        if captured.not_utf8.contains(&(index + 1)) {
            continue;
        }
        match (strict::object(line), lines) {
            (Some(fields), _) => events.push(Event {
                line: first + index,
                fields,
            }),
            (None, Lines::Events) => unread.push((first + index, Fault::NotOneObject)),
            (None, Lines::Text) => match text::said(line, tools) {
                text::Said::Reach(reach) => reaches.push((first + index, reach)),
                text::Said::Unread => unread.push((first + index, Fault::Unrecognised)),
                text::Said::Nothing => {}
            },
        }
    }
    unread.sort_unstable_by_key(|(line, _)| *line);
    Stream {
        source: source.to_string(),
        lines,
        events,
        reaches,
        unread,
        empty: captured.text.is_empty() && captured.not_utf8.is_empty(),
    }
}

/// What a launch wrote to a transcript, as a stream: lines it appended
/// keep their numbers in the file, and a file it rewrote is named so.
fn transcript_stream(transcript: &Transcript) -> Stream {
    let (source, first) = match transcript.written {
        Written::Created => (transcript.path.clone(), 1),
        Written::Appended { from_line } => (transcript.path.clone(), from_line),
        Written::Rewritten => (format!("{}, which the turn rewrote,", transcript.path), 1),
    };
    parse_lines(&transcript.text, &source, first, Lines::Events, &[])
}

/// Every stream a launch captured, read whatever its exit (#484):
/// stdout's and each transcript's, each named with this run's variable
/// parts, then stderr's when it holds any byte, its text checked for
/// `tools` and for every tool those streams list. With them, the one the
/// turn's own events are read from: stdout, or, when stdout carried none,
/// the first transcript that holds some. Stderr is never that one.
fn captured(observation: &Observation, tools: &[String]) -> (Vec<Stream>, Option<usize>) {
    let stdout = parse_lines(&observation.stdout, "stdout", 1, Lines::Events, &[]);
    let mut all: Vec<Stream> = std::iter::once(stdout)
        .chain(observation.transcripts.iter().map(transcript_stream))
        .collect();
    let primary = all.iter().position(|stream| !stream.events.is_empty());
    let id = primary.and_then(|primary| session_id(&all[primary]).map(|(_, _, id)| id));
    for stream in &mut all {
        stream.source = normalise(&stream.source, id.as_deref());
    }
    let mut names = tools.to_vec();
    names.extend(tools::listed_tools(&all));
    let stderr = parse_lines(&observation.stderr, "stderr", 1, Lines::Text, &names);
    if !stderr.empty {
        all.push(stderr);
    }
    (all, primary)
}

/// A turn that exited clean: read when any of its streams holds an event.
fn read_stream((all, primary): (Vec<Stream>, Option<usize>)) -> Turn {
    match primary {
        Some(primary) => Turn::Read(Streams { all, primary }),
        None => Turn::Unread(
            "the turn printed no JSON event and wrote no .jsonl transcript under the scratch \
             HOME"
                .to_string(),
            all,
        ),
    }
}

/// The plain turn: read when it exited clean, otherwise nothing it shows
/// is a measurement, though every line it captured is still read.
fn read_base(observation: &Observation, tools: &[String]) -> Turn {
    let streams = captured(observation, tools);
    if observation.exit == Some(0) {
        read_stream(streams)
    } else {
        let why = format!(
            "the headless turn did not succeed: {}",
            exit_and_excerpt(observation)
        );
        Turn::Unread(why, streams.0)
    }
}

/// A turn launched under an argv the adapter declares, by how the report
/// names the turn and the argv.
struct Under {
    turn: &'static str,
    argv: &'static str,
}

const HANDS: Under = Under {
    turn: "the boxed turn",
    argv: "the adapter's hands argv",
};

const OFF: Under = Under {
    turn: "the turn under the declared OFF controls",
    argv: "the declared OFF controls",
};

/// The flags and `-c` keys of `controls`, by which a refusal names them.
fn control_words(controls: &[String]) -> Vec<&str> {
    let words = controls
        .iter()
        .filter_map(|part| match part.strip_prefix('-') {
            Some(_) => Some(part.as_str()),
            None => part.split_once('=').map(|(key, _)| key),
        });
    words.collect()
}

/// Whether what a launch printed names one of `controls`' flags or keys.
fn names_a_control(observation: &Observation, controls: &[String]) -> bool {
    let named = control_words(controls);
    [&observation.stderr.text, &observation.stdout.text]
        .into_iter()
        .flat_map(|text| text.split(|c: char| !(c.is_ascii_alphanumeric() || "-_.".contains(c))))
        .any(|word| named.contains(&word))
}

/// The turn under `controls`, an argv the adapter declares: a non-zero
/// exit whose text names one of their flags or keys is the CLI refusing
/// them, which is itself the measurement; any other non-zero exit, like
/// a launch that ended with no exit code, by a signal or the deadline,
/// refused nothing the probe can name (#484). However it ended, every
/// line it captured is read, its stderr checked for `tools`.
fn read_under(trial: &Trial, under: Under, controls: &[String], tools: &[String]) -> Turn {
    let observation = match trial {
        Trial::Untried(why) => return Turn::Unread(why.clone(), Vec::new()),
        Trial::Observed(observation) => observation,
    };
    let streams = captured(observation, tools);
    let ended = exit_and_excerpt(observation);
    let how = match observation.exit {
        Some(0) => return read_stream(streams),
        None => "did not finish".to_string(),
        Some(_) if !names_a_control(observation, controls) => {
            format!("failed, naming no flag or key of {}", under.argv)
        }
        Some(_) => {
            let refused = format!("the CLI refused {}: {ended}", under.argv);
            return Turn::Refused(refused, streams.0);
        }
    };
    Turn::Unread(format!("{} {how}: {ended}", under.turn), streams.0)
}

/// Everything `turn` captured that no reader read: each such line of
/// each of its streams, and each value its listings hold that the
/// listing reader could not name.
fn unread_in(turn: &Turn, name: TurnName) -> Vec<Unread> {
    let lines = turn.streams().iter().flat_map(|stream| {
        stream.unread.iter().map(|(line, fault)| Unread::Line {
            turn: name,
            source: stream.source.clone(),
            line: *line,
            fault: *fault,
        })
    });
    let values = tools::unread_values(turn)
        .into_iter()
        .map(|what| Unread::Value { turn: name, what });
    lines.chain(values).collect()
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
            non_json_lines: stream.unread.len(),
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
        Turn::Unread(..) | Turn::Refused(..) => None,
    };
    let id = announced.as_ref().map(|(_, _, id)| id.as_str());
    let events = on_turn(base, |streams| events(streams.primary()));
    let session = on_turn(base, |_| session(announced.as_ref()));
    let transcripts = on_turn(base, |_| transcripts(turn, id));
    (events, session, transcripts)
}

/// Every fact, from every observation, and everything the three turns
/// captured that no reader read. The stderr of each is checked for the
/// tools the adapter declares and those the plain turn listed.
pub(crate) fn reading(plan: &Plan, observed: &Observed, bound: &[&str]) -> Reading {
    let mut named = tools::declared_tools(plan);
    let base = read_base(&observed.turn, &named);
    named.extend(tools::listed_tools(base.streams()));
    let boxed = read_under(&observed.boxed, HANDS, &plan.hands, &named);
    let off = read_under(&observed.native_off, OFF, &plan.off, &named);
    let mut unread = unread_in(&base, TurnName::Plain);
    unread.extend(unread_in(&boxed, TurnName::Boxed));
    unread.extend(unread_in(&off, TurnName::Off));
    Reading {
        facts: facts(plan, observed, bound, &base, &boxed, &off),
        unread,
    }
}

/// Every fact, from every observation and the three turns as read.
fn facts(
    plan: &Plan,
    observed: &Observed,
    bound: &[&str],
    base: &Turn,
    boxed: &Turn,
    off: &Turn,
) -> Facts {
    let (events, session, transcripts) = stream_facts(base, &observed.turn);
    let tools = on_turn(base, tools::native_tools);
    let boxed_tools = on_turn(boxed, tools::native_tools);
    let off_tools = on_turn(off, tools::native_tools);
    let native_egress = tools::native_egress(&tools);
    let egress_off = tools::egress_off(&native_egress, &off_tools);
    let capabilities = tools::capabilities(plan, &tools, &off_tools);
    let user_mcp_unboxed = tools::user_mcp(base, &plan.user_config);
    let user_mcp_boxed = tools::user_mcp(boxed, &plan.user_config);
    let user_mcp_off = tools::user_mcp(off, &plan.user_config);
    let config_isolation = on_turn(base, |_| {
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
        usage: on_turn(base, |streams| usage(streams.primary())),
        cost: on_turn(base, |streams| cost(streams.primary())),
        refusals: Refusals {
            auth: refusals::refusal(&observed.no_credentials),
            config: refusals::config_refusal(&observed.bad_model),
            rate_limit: Fact::unmeasured(
                "not provoked: a rate limit spends quota and risks the account",
            ),
            outage: Fact::unmeasured("not provoked: a provider outage cannot be caused safely"),
        },
        efforts: refusals::efforts(&observed.bad_effort),
        tools,
        boxed_tools,
        mcp_server: on_turn(boxed, tools::mcp_server),
        native_egress,
        egress_off,
        capabilities,
        config_isolation,
        user_mcp_unboxed,
        user_mcp_boxed,
        user_mcp_off,
        transcripts,
        resume: Fact::unmeasured(
            "the probe does not drive a resume turn yet; decision 0056's per-shape \
             assessment stays the adapter's (#226)",
        ),
    }
}
