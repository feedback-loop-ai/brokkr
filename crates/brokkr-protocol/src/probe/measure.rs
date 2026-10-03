//! Reading the observations into facts. Pure: nothing here runs, reads
//! or writes anything; `observe` did, and handed over masked text.
//!
//! A CLI's output is the edge (decision 0071 ruling 3). Each line is
//! decoded once, by the harness's typed reader (`read`), into what it
//! says, and left behind as the typed facts of `facts`. A line the reader
//! does not decode whole is unread, and what no line says is
//! `unmeasured`, never a default.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::facts::{Counting, Events, Fact, Facts, Headless, Refusals, Session, Usage};
use super::observe::{Captured, Observation, Transcript, Trial, Written, SCRATCH_PREFIX};
use super::plan::Plan;
use read::{Block, Counted, Harness, Said};

mod answer;
mod claude;
mod claude_log;
mod claude_message;
mod codex;
mod codex_log;
mod dsh;
mod forms;
mod listing;
pub(crate) mod read;
mod refusals;
mod strict;
mod tools;

/// How much of a refusal's line a report keeps.
const EXCERPT_CHARS: usize = 240;

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

/// One event a stream carried, decoded: its line, counted from 1, its
/// type, and what it says.
struct Event {
    line: usize,
    label: String,
    said: Vec<Said>,
}

/// Why a line of a stream went unread.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Fault {
    /// It decoded, but is not one JSON object naming each key once: a
    /// line cut short, or joined to another, among them.
    NotOneObject,
    /// Its bytes are not UTF-8, so it was never decoded, nor repaired.
    NotUtf8,
    /// It is one JSON object, but not an event of this harness's reader:
    /// its type, a key or a value's JSON type is not one the reader names.
    Undecoded(Harness),
    /// It holds text, or an event holds a string, that is not one of the
    /// forms the harness's reader recognises whole.
    Unrecognised,
}

impl fmt::Display for Fault {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Fault::NotOneObject => {
                formatter.write_str("is not one JSON object naming each key once")
            }
            Fault::NotUtf8 => formatter.write_str("is not UTF-8"),
            Fault::Undecoded(harness) => write!(
                formatter,
                "is not a {harness} event the probe decodes whole: its type, a key or a value's \
                 type is not one the reader names"
            ),
            Fault::Unrecognised => formatter.write_str(
                "holds text no reader consumes in a form the probe does not recognise whole",
            ),
        }
    }
}

/// How a stream's lines are read (#484), closed-world, by the harness's
/// typed reader: an event stream, stdout or a transcript, holds one
/// event per line, decoded by the harness's events or its transcript's
/// rows, and a line that is not one JSON object is unread. Text, stderr
/// and dsh's stdout, is read line by line, since a CLI prints its
/// ordinary warnings there:
///
/// - (i) a line that is not UTF-8 is unread, named by stream and line,
///   and refuses any admitting verdict;
/// - (ii) a line that is one JSON object is decoded as an event, like a
///   line of stdout, so wrapping text in JSON changes nothing;
/// - (iii) any other line is unread unless it is blank, the reply the
///   probe asks for, or one of the forms the reader recognises whole,
///   and what that form says is kept.
#[derive(Clone, Copy, PartialEq)]
enum Lines {
    Events,
    Transcript,
    Text,
}

/// The name stderr is read under.
const STDERR: &str = "stderr";

/// One stream a turn produced, stdout, a transcript or stderr: how its
/// lines are read, its events, what its lines of text said, where they
/// were read from, each line no reader read and why, and whether it held
/// no bytes at all.
struct Stream {
    source: String,
    lines: Lines,
    events: Vec<Event>,
    text: Vec<(usize, Said)>,
    unread: Vec<(usize, Fault)>,
    empty: bool,
}

/// Everything `stream`'s lines said, each beside its line: its events'
/// and its text's.
fn said_in(stream: &Stream) -> impl Iterator<Item = (usize, &Said)> {
    let events = stream.events.iter().flat_map(|event| {
        let line = event.line;
        event.said.iter().map(move |said| (line, said))
    });
    events.chain(stream.text.iter().map(|(line, said)| (*line, said)))
}

/// The three turns whose streams are read as events: the plain turn, the
/// boxed one and the one under the declared OFF controls.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum TurnName {
    Plain,
    Boxed,
    Off,
}

/// A line of one of a turn's streams that no reader read, which leaves
/// the probe's evidence incomplete (#484).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Unread {
    pub(crate) turn: TurnName,
    pub(crate) source: String,
    pub(crate) line: usize,
    pub(crate) fault: Fault,
}

impl fmt::Display for Unread {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let turn = match self.turn {
            TurnName::Plain => "the plain turn",
            TurnName::Boxed => "the boxed turn",
            TurnName::Off => "the OFF turn",
        };
        write!(
            formatter,
            "line {} of {turn}'s {} {}",
            self.line, self.source, self.fault
        )
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

/// How a stream is read: by which harness's reader, its lines as
/// events or as text.
#[derive(Clone, Copy)]
struct Reader {
    harness: Harness,
    lines: Lines,
}

/// What one line was read to be: an event, or text and what it says.
enum Line {
    Event(read::Decoded),
    Text(Vec<Said>),
}

/// One UTF-8 line, read as `reader` says, `block` carrying what the
/// stream's earlier lines of text opened: one JSON object is decoded as
/// an event or a transcript's row, and any other line is unread, or read
/// as text.
fn read_line(line: &str, reader: Reader, block: &mut Block) -> Result<Line, Fault> {
    match (strict::object(line), reader.lines) {
        (Some(fields), Lines::Transcript) => reader.harness.row(fields).map(Line::Event),
        (Some(fields), Lines::Events | Lines::Text) => {
            reader.harness.event(fields).map(Line::Event)
        }
        (None, Lines::Events | Lines::Transcript) => Err(Fault::NotOneObject),
        (None, Lines::Text) => reader
            .harness
            .text_in(line, block)
            .map(Line::Text)
            .ok_or(Fault::Unrecognised),
    }
}

/// `captured` read as the stream `source`, its first line being line
/// `first` of what it came from: a line that is not UTF-8 is unread, and
/// never parsed; and any other is read by [`read_line`].
fn parse_lines(captured: &Captured, source: &str, first: usize, reader: Reader) -> Stream {
    let mut events = Vec::new();
    let mut text = Vec::new();
    let mut block = Block::Plain;
    let mut unread: Vec<(usize, Fault)> = captured
        .not_utf8
        .iter()
        .map(|line| (first - 1 + line, Fault::NotUtf8))
        .collect();
    for (index, line) in captured.text.lines().enumerate() {
        if captured.not_utf8.contains(&(index + 1)) {
            continue;
        }
        let at = first + index;
        match read_line(line, reader, &mut block) {
            Ok(Line::Event(decoded)) => events.push(Event {
                line: at,
                label: decoded.label,
                said: decoded.said,
            }),
            Ok(Line::Text(said)) => text.extend(said.into_iter().map(|said| (at, said))),
            Err(fault) => unread.push((at, fault)),
        }
    }
    unread.sort_unstable_by_key(|(line, _)| *line);
    Stream {
        source: source.to_string(),
        lines: reader.lines,
        events,
        text,
        unread,
        empty: captured.text.is_empty() && captured.not_utf8.is_empty(),
    }
}

/// What a launch wrote to a transcript, as a stream: lines it appended
/// keep their numbers in the file, and a file it rewrote is named so.
fn transcript_stream(transcript: &Transcript, harness: Harness) -> Stream {
    let (source, first) = match transcript.written {
        Written::Created => (transcript.path.clone(), 1),
        Written::Appended { from_line } => (transcript.path.clone(), from_line),
        Written::Rewritten => (format!("{}, which the turn rewrote,", transcript.path), 1),
    };
    let reader = Reader {
        harness,
        lines: Lines::Transcript,
    };
    parse_lines(&transcript.text, &source, first, reader)
}

/// Every stream a launch captured, read whatever its exit by `harness`'s
/// reader (#484): stdout's and each transcript's, each named with this
/// run's variable parts, then stderr's when it holds any byte. With them,
/// the one the turn's own events are read from: stdout, or, when stdout
/// carried none, the first transcript that holds some. Stderr is never
/// that one.
fn captured(observation: &Observation, harness: Harness) -> (Vec<Stream>, Option<usize>) {
    let reader = |lines| Reader { harness, lines };
    let stdout = parse_lines(&observation.stdout, "stdout", 1, reader(harness.stdout()));
    let mut all: Vec<Stream> = std::iter::once(stdout)
        .chain(
            observation
                .transcripts
                .iter()
                .map(|transcript| transcript_stream(transcript, harness)),
        )
        .collect();
    let primary = all.iter().position(|stream| !stream.events.is_empty());
    let id = primary.and_then(|primary| session_id(&all[primary]).map(|(_, _, id)| id));
    for stream in &mut all {
        stream.source = normalise(&stream.source, id.as_deref());
    }
    let stderr = parse_lines(&observation.stderr, STDERR, 1, reader(Lines::Text));
    if !stderr.empty {
        all.push(stderr);
    }
    (all, primary)
}

/// A turn that exited clean, launched under `controls`: unread when none
/// of its streams holds an event; refused when a line refuses one of
/// `controls`; unread when a line states any other refusal or a failure,
/// or none is the reply, since a clean exit is not the turn answering
/// (#484); and otherwise read.
fn read_stream(
    (all, primary): (Vec<Stream>, Option<usize>),
    under: &Under,
    controls: &[String],
    observation: &Observation,
) -> Turn {
    let Some(primary) = primary else {
        let why = "the turn printed no JSON event and wrote no .jsonl transcript under the \
                   scratch HOME";
        return Turn::Unread(why.to_string(), all);
    };
    let ended = exit_and_excerpt(observation);
    if refusals::refuses_a_control(&all, controls) {
        return Turn::Refused(format!("the CLI refused {}: {ended}", under.argv), all);
    }
    match answer::unanswered(&all) {
        Some(why) => Turn::Unread(format!("{} exited 0, but {why}: {ended}", under.turn), all),
        None => Turn::Read(Streams { all, primary }),
    }
}

/// The plain turn: read as [`read_stream`] says when it exited clean,
/// otherwise nothing it shows is a measurement, though every line it
/// captured is still read.
fn read_base(observation: &Observation, harness: Harness) -> Turn {
    let streams = captured(observation, harness);
    if observation.exit == Some(0) {
        read_stream(streams, &PLAIN, &[], observation)
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

const PLAIN: Under = Under {
    turn: "the headless turn",
    argv: "the headless argv",
};

const HANDS: Under = Under {
    turn: "the boxed turn",
    argv: "the adapter's hands argv",
};

const OFF: Under = Under {
    turn: "the turn under the declared OFF controls",
    argv: "the declared OFF controls",
};

/// The turn under `controls`, an argv the adapter declares: a non-zero
/// exit one of whose lines is the CLI's refusal of one of their flags or
/// keys, a form `harness`'s reader recognises as one, is the CLI refusing
/// them, which is itself the measurement; any other non-zero exit, like a
/// launch that ended with no exit code, by a signal or the deadline,
/// refused nothing the probe can name (#484); and a clean exit is read as
/// [`read_stream`] says. However it ended, every line it captured is
/// read.
fn read_under(trial: &Trial, under: Under, controls: &[String], harness: Harness) -> Turn {
    let observation = match trial {
        Trial::Untried(why) => return Turn::Unread(why.clone(), Vec::new()),
        Trial::Observed(observation) => observation,
    };
    let streams = captured(observation, harness);
    let ended = exit_and_excerpt(observation);
    let how = match observation.exit {
        Some(0) => return read_stream(streams, &under, controls, observation),
        None => "did not finish".to_string(),
        Some(_) if !refusals::refuses_a_control(&streams.0, controls) => format!(
            "failed, and no line it printed is the CLI's refusal of a flag or key of {}",
            under.argv
        ),
        Some(_) => {
            let refused = format!("the CLI refused {}: {ended}", under.argv);
            return Turn::Refused(refused, streams.0);
        }
    };
    Turn::Unread(format!("{} {how}: {ended}", under.turn), streams.0)
}

/// Every line of each of `turn`'s streams that no reader read.
fn unread_in(turn: &Turn, name: TurnName) -> Vec<Unread> {
    let lines = turn.streams().iter().flat_map(|stream| {
        stream.unread.iter().map(|(line, fault)| Unread {
            turn: name,
            source: stream.source.clone(),
            line: *line,
            fault: *fault,
        })
    });
    lines.collect()
}

fn push_unique(list: &mut Vec<String>, item: String) {
    if !list.contains(&item) {
        list.push(item);
    }
}

/// Each thing `stream`'s events said that `pick` takes, beside the event
/// that said it.
fn each_said<'a, T: 'a>(
    stream: &'a Stream,
    pick: fn(&'a Said) -> Option<T>,
) -> impl Iterator<Item = (&'a Event, T)> + 'a {
    stream.events.iter().flat_map(move |event| {
        let picked = event.said.iter().filter_map(pick);
        picked.map(move |item| (event, item))
    })
}

fn events(stream: &Stream) -> Fact<Events> {
    let mut types = Vec::new();
    for event in &stream.events {
        push_unique(&mut types, event.label.clone());
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
    let mut sessions = each_said(stream, |said| {
        let Said::Session { key, id } = said else {
            return None;
        };
        Some((*key, id))
    });
    let (event, (key, id)) = sessions.next()?;
    Some((event.label.clone(), key, id.clone()))
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
        None => Fact::unmeasured("no event of the turn named its session"),
    }
}

/// Where usage sat, which counters it named, and whether one message's
/// usage was restated on several events (#402).
fn usage(stream: &Stream) -> Fact<Usage> {
    let mut locations = Vec::new();
    let mut counters = BTreeSet::new();
    let mut messages: BTreeMap<&str, Vec<&Counted>> = BTreeMap::new();
    let mut unnamed = Vec::new();
    let counted = each_said(stream, |said| {
        let Said::Usage(counted) = said else {
            return None;
        };
        Some(counted)
    });
    for (event, counted) in counted {
        let location = format!("{} {}", event.label, counted.at);
        counters.extend(counted.counts.iter().map(|(name, _)| name.to_string()));
        match &counted.message {
            Some(id) => messages.entry(id).or_default().push(counted),
            None => push_unique(&mut unnamed, location.clone()),
        }
        push_unique(&mut locations, location);
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
fn counting(messages: &BTreeMap<&str, Vec<&Counted>>, unnamed: &[String]) -> Fact<Counting> {
    let outside = if unnamed.is_empty() {
        String::new()
    } else {
        format!("; the usage at {} named no message", unnamed.join(", "))
    };
    let repeated = messages.iter().find(|(_, usages)| {
        usages.len() > 1
            && usages
                .windows(2)
                .all(|pair| pair[0].counts == pair[1].counts)
    });
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
    let costs = each_said(stream, |said| {
        let Said::Cost { at } = said else {
            return None;
        };
        Some(*at)
    });
    for (event, at) in costs {
        push_unique(&mut locations, format!("{} {at}", event.label));
    }
    if locations.is_empty() {
        return Fact::unmeasured("no event of the turn reported a cost");
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
/// captured that no reader read.
pub(crate) fn reading(plan: &Plan, observed: &Observed, bound: &[&str]) -> Reading {
    let base = read_base(&observed.turn, plan.reader);
    let boxed = read_under(&observed.boxed, HANDS, &plan.hands, plan.reader);
    let off = read_under(&observed.native_off, OFF, &plan.off, plan.reader);
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
                 {}: {}{}",
                bound.join(", "),
                plan.unlike_driver,
                exit_text(observed.turn.exit),
                answer::models(base)
            ),
        ),
        events,
        session,
        usage: on_turn(base, |streams| usage(streams.primary())),
        cost: on_turn(base, |streams| cost(streams.primary())),
        refusals: Refusals {
            auth: refusals::auth_refusal(&observed.no_credentials, plan.reader),
            config: refusals::config_refusal(&observed.bad_model, plan.reader),
            rate_limit: Fact::unmeasured(
                "not provoked: a rate limit spends quota and risks the account",
            ),
            outage: Fact::unmeasured("not provoked: a provider outage cannot be caused safely"),
        },
        efforts: refusals::efforts(&observed.bad_effort, plan.reader),
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

#[cfg(test)]
mod tests;
