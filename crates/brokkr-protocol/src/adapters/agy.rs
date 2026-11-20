//! `brokkr driver agy`: Antigravity CLI as a provisional harness at level 1
//! (#1200; decision 0075). The seat is unboxed and never resumed, every
//! model it serves is provisional, it runs under a HOME the engine owns, and
//! it never reads a turn agy stopped at a denial, or a model it did not
//! pin, as an answer. The facts come from the controller's hand
//! measurement of Antigravity CLI 1.5.13 (`docs/evidence/adapters/
//! agy-cli-1-5.md`).
//!
//! - The prompt goes on stdin as one `stream-json` user line, which closing
//!   stdin ends (`stdin-stream-json`); `-p` would put it in argv and takes
//!   the next argument as its value (`p-takes-next-flag`).
//! - Permissions ride one `--settings` value of `permissions.allow` rules
//!   (`allow-list-*`), and `--mode accept-edits` runs a write in the
//!   workspace (`denied-command-accept-edits`).
//! - The engine's HOME ([`home`]), its gate ([`gate`]) and the fold
//!   ([`fold`]) each have one home below.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};
use thiserror::Error;

use super::escapes::plain_evidence;
use super::mcp::{self, McpRefusal};
use super::spawn::spawn_fed_in;
use super::stream_turn::{streamed, Turn};
use super::{secret, Invocation, LaunchTerminal};
use crate::transcript::{Kind as TranscriptKind, Transcript};

mod fold;
mod gate;
mod home;

use crate::PreSessionRefusal;
use fold::{AgyFold, Failure, Verdict};
pub(super) use gate::{gate, gate_listed};
use home::{Home, Homes, Written};

/// What every agy launch runs before the seat's pin, effort and settings:
/// a stream turn read from stdin, accepting edits.
const HEAD: [&str; 6] = [
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--mode",
    "accept-edits",
];

/// The three controls a seat's argv may carry, each once and each with one
/// value: the adapter's model flag, its effort flag and its tool
/// permissions flag (`docs/examples/adapters/agy.json`). `--allow` is the
/// driver's own word, turned into the launch's one `--settings` value.
const CONTROLS: [&str; 3] = ["--model", "--effort", "--allow"];

/// The one flag every launch's rules ride.
const SETTINGS: &str = "--settings";

/// The rule forms the driver writes: agy ignores a rule it cannot parse
/// without a word (`settings-unparsable-rule`), so no other is written.
const RULE_FORMS: [&str; 2] = ["command(", "mcp("];

/// The one name of the child's environment the engine owns for an agy
/// seat: its HOME, which no seat binds (J1).
const HOME: &str = "HOME";

/// Why a seat is not launched. Each names what was written, never a value
/// (AS3).
#[derive(Debug, Error)]
pub(super) enum LaunchError {
    /// The engine's HOME does not outlive a turn, so a start offered a
    /// rejoin has nothing to rejoin and is refused (G11).
    #[error(
        "refusing to invoke the agent CLI: the start offers a resume, and the agy adapter \
         declares no resume (#1200)"
    )]
    Resumed,
    /// A seat binding the name would start agy under its own HOME, with
    /// the operator's servers, plugins and hooks (J1).
    #[error(
        "refusing to invoke the agent CLI: the seat binds a secret named {HOME}, which the \
         engine owns for an agy seat (#1200)"
    )]
    HomeBound,
    /// An option the driver does not admit, by its name up to its first
    /// `=` and never what follows (J6).
    #[error(
        "refusing to invoke the agent CLI: the seat's arguments carry '{0}', which brokkr \
         driver agy does not admit; an agy seat's arguments are --model, --effort and --allow, \
         each once (#1200)"
    )]
    Unadmitted(String),
    #[error(
        "refusing to invoke the agent CLI: the seat's arguments carry a value with no option \
         before it, which brokkr driver agy does not admit (#1200)"
    )]
    Positional,
    #[error("refusing to invoke the agent CLI: the seat's arguments carry '{0}' more than once")]
    Doubled(&'static str),
    #[error("refusing to invoke the agent CLI: the seat's arguments carry '{0}' with no value")]
    Valueless(&'static str),
    #[error(
        "refusing to invoke the agent CLI: the seat pins no model with --model; a provisional \
         seat pins its id (#1200)"
    )]
    Unpinned,
    #[error(
        "refusing to invoke the agent CLI: the seat's --allow carries a rule that is neither \
         command(…) nor mcp(…), which agy would ignore without a word (#1200)"
    )]
    Unruled,
    /// A rule's target agy could read otherwise than the seat meant (J12).
    #[error(
        "refusing to invoke the agent CLI: the seat's --allow carries a rule whose target is \
         blank, has a blank at an end, or holds a control character or a parenthesis (#1200)"
    )]
    Untargeted,
    #[error(
        "refusing to invoke the agent CLI: the engine's HOME or executable for the agy seat has \
         a path that is not UTF-8, so nothing names it exactly (#1200)"
    )]
    Unnamed,
    #[error(
        "refusing to invoke the agent CLI: the agy seat's working directory cannot be resolved: \
         {0}"
    )]
    Unrooted(#[source] std::io::Error),
    /// A workspace that carries agy's own configuration (G6).
    #[error(
        "refusing to invoke the agent CLI: the agy seat's workspace carries {}, which agy would \
         start or run in a turn nobody trusted; remove it, or run the seat elsewhere (#1200)",
        .0.display()
    )]
    Configured(PathBuf),
    /// A path of the walk that cannot be read is no absence (J8).
    #[error(
        "refusing to invoke the agent CLI: the agy seat's workspace path {} cannot be read, so \
         whether it carries agy's configuration is unknown: {} (#1200)",
        .0.display(),
        .1
    )]
    Unread(PathBuf, #[source] std::io::Error),
    /// Git, asked once where the walk ends, could not be run.
    #[error(
        "refusing to invoke the agent CLI: git could not be asked for the agy seat's \
         repository root: {0} (#1200)"
    )]
    Unasked(#[source] std::io::Error),
    #[error(
        "refusing to invoke the agent CLI: Antigravity CLI is signed out: run `agy` once and \
         sign in; the login is ~/.gemini/antigravity-cli/antigravity-oauth-token (#1200)"
    )]
    SignedOut,
    /// The engine's HOME could not be made.
    #[error(
        "refusing to invoke the agent CLI: the engine's HOME for the agy seat was not made: {0}"
    )]
    Unhomed(#[source] std::io::Error),
    /// The engine HOME's session, which records its owner, was not made.
    #[error(
        "refusing to invoke the agent CLI: the engine's HOME for the agy seat was not made: {0}"
    )]
    Unsessioned(#[source] crate::hands::SessionError),
    /// The one parent of the engine's HOME directories is not this user's
    /// alone (J5).
    #[error(
        "refusing to invoke the agent CLI: {} is not a directory of this user's that nobody else \
         may enter, so it holds no engine HOME for the agy seat (#1200)",
        .0.display()
    )]
    Unowned(PathBuf),
    /// The launch's one MCP admission refused it, in the words every
    /// driver states it with ([`McpRefusal::at_launch`]).
    #[error("{}", .0.at_launch())]
    Mcp(#[from] McpRefusal),
}

/// How a seat ended with no invocation, typed until [`drive`] renders it
/// once (#1178 T4): its launch refused, its turn failed, a refusal behind
/// work already done, or the shared driver's own words.
#[derive(Debug, Error)]
enum Ended {
    #[error(transparent)]
    Launch(#[from] LaunchError),
    #[error(transparent)]
    Failed(#[from] Failure),
    #[error("{}", .0.words())]
    Late(PreSessionRefusal),
    #[error("{0}")]
    Driver(String),
}

/// The shared driver's helpers end in their own words.
impl From<String> for Ended {
    fn from(words: String) -> Ended {
        Ended::Driver(words)
    }
}

/// The words the invocation's stderr holds in place of a signed-out agy's:
/// its sign-in block is a prompt to the operator holding a URL made for one
/// attempt, and is not evidence (J4).
const SIGN_IN_WITHHELD: &str =
    "agy wrote its sign-in request on stderr, which brokkr does not keep: it holds a link made \
     for one attempt (#1200)\n";

/// One agy launch: the command spawned and the id it pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Launch {
    pub(super) command: Vec<String>,
    pub(super) pin: String,
}

/// The value of each of [`CONTROLS`] `extra` writes, in their order. A
/// token that is none of them refuses by its name; a value is never named.
fn admitted(extra: &[String]) -> Result<[Option<&String>; 3], LaunchError> {
    let mut values: [Option<&String>; 3] = [None; 3];
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        let Some(slot) = CONTROLS.iter().position(|control| control == part) else {
            let name = part.split('=').next().unwrap_or_default();
            return Err(match part.starts_with('-') {
                true => LaunchError::Unadmitted(name.to_string()),
                false => LaunchError::Positional,
            });
        };
        let control = CONTROLS[slot];
        if values[slot].is_some() {
            return Err(LaunchError::Doubled(control));
        }
        let value = parts
            .next()
            .filter(|value| !value.is_empty() && !value.starts_with('-'));
        values[slot] = Some(value.ok_or(LaunchError::Valueless(control))?);
    }
    Ok(values)
}

/// The launch for `extra`, the arguments behind the driver's `--`, granted
/// the engine's `rules` beside the seat's own: the head, the pin, the
/// effort where the seat names one, and the one `--settings` value.
fn launch(bin: &str, extra: &[String], rules: &[String]) -> Result<Launch, LaunchError> {
    let [pin, effort, allow] = admitted(extra)?;
    let pin = pin.ok_or(LaunchError::Unpinned)?;
    let mut allowed: Vec<&str> = allow.map_or_else(Vec::new, |allow| allow.split(',').collect());
    allowed.extend(rules.iter().map(String::as_str));
    for rule in &allowed {
        targeted(target(rule).ok_or(LaunchError::Unruled)?)?;
    }
    let mut command = vec![bin.to_string()];
    command.extend(HEAD.iter().map(|part| part.to_string()));
    command.extend([CONTROLS[0].to_string(), pin.clone()]);
    if let Some(effort) = effort {
        command.extend([CONTROLS[1].to_string(), effort.clone()]);
    }
    command.extend([SETTINGS.to_string(), settings(&allowed)]);
    Ok(Launch {
        command,
        pin: pin.clone(),
    })
}

/// The target of `rule` where it is one of [`RULE_FORMS`].
fn target(rule: &str) -> Option<&str> {
    RULE_FORMS.iter().find_map(|form| {
        rule.strip_prefix(form)
            .and_then(|rest| rest.strip_suffix(')'))
    })
}

/// A rule's target checked whole (J12): not empty and not blank, with no
/// blank at either end, no control character and no parenthesis. Several
/// words are a prefix of a command's (`rule-multi-word-prefix`).
fn targeted(target: &str) -> Result<(), LaunchError> {
    let odd = |c: char| c.is_control() || c == '(' || c == ')';
    let ends = [target.chars().next(), target.chars().last()];
    let blank_end = ends.iter().any(|end| end.is_none_or(char::is_whitespace));
    match blank_end || target.chars().any(odd) {
        true => Err(LaunchError::Untargeted),
        false => Ok(()),
    }
}

#[derive(Serialize)]
struct Settings<'a> {
    permissions: Permissions<'a>,
}

#[derive(Serialize)]
struct Permissions<'a> {
    allow: &'a [&'a str],
}

/// The launch's one `--settings` value: the session's allow rules. Its one
/// home, so no launch carries a second.
fn settings(allow: &[&str]) -> String {
    let permissions = Permissions { allow };
    serde_json::to_string(&Settings { permissions }).expect("the settings serialize")
}

#[derive(Serialize)]
struct UserLine<'a> {
    event: &'static str,
    message: Message<'a>,
}

#[derive(Serialize)]
struct Message<'a> {
    content: &'a str,
}

/// `prompt` as the one stdin line agy reads a turn from.
fn stdin_line(prompt: &str) -> String {
    let line = UserLine {
        event: "user",
        message: Message { content: prompt },
    };
    serde_json::to_string(&line).expect("a user line serializes") + "\n"
}

/// One agy seat, start to end: the start refused where it `offered` a
/// rejoin, because the agy adapter declares no resume, and otherwise the
/// seat's [`turn`]. Every error the seat ends with, its launch's and its
/// turn's alike, is rendered here, once, and passes the seat's bindings
/// once, last.
pub(super) fn drive(
    pinned: super::Pinned<'_>,
    offered: bool,
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    let ended = match offered {
        true => Err(Ended::Launch(LaunchError::Resumed)),
        false => turn(pinned, prompt, input, bindings, emit),
    };
    ended.map_err(|error| secret::mask_text(error.to_string().as_bytes(), bindings))
}

/// The seat's composed arguments, the launch [`prepared`] from them under
/// the driver's own temporary root, HOME and executable, and its turn,
/// before the one mask. A seat that binds [`HOME`] is refused by the
/// name, before anything else. Every launch is admitted cold.
fn turn(
    pinned: super::Pinned<'_>,
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, Ended> {
    if bindings.iter().any(|binding| binding.name() == HOME) {
        return Err(LaunchError::HomeBound.into());
    }
    let composed = super::composed_launch("agy", pinned.extra, input)?;
    let (temp, operator) = (std::env::temp_dir(), std::env::var_os("HOME"));
    let homes = Homes {
        temp: &temp,
        operator: operator.as_deref().map(Path::new),
    };
    let brokkr = mcp::engine_executable();
    let prepared = prepared(pinned.binary, &composed.extra, input, homes, &brokkr);
    // The HOME lives until the turn ends.
    let (home, launch) = prepared?;
    invoke((&launch, &home), input, prompt, bindings, emit)
}

/// The launch of `extra` before any provider work: SI2's one admission
/// ([`mcp::isolated`]), the engine's set read from the intent it checked
/// ([`mcp::agy_set`]), the launch, the workspace check, and the engine's
/// HOME made under `homes` with the set's document and the gate run by
/// `brokkr`. No launch is offered a rejoin, so the cold assessment is the
/// one admitted.
fn prepared(
    bin: &str,
    extra: &[String],
    input: &Value,
    homes: Homes<'_>,
    brokkr: &Path,
) -> Result<(Home, Launch), LaunchError> {
    let edge = mcp::Edge::new(input);
    let isolated = mcp::isolated("agy", &edge, extra, false)?;
    let workdir = input.get("workdir").and_then(Value::as_str).unwrap_or("");
    let served = mcp::agy_set(&edge, &isolated, workdir)?;
    let launch = launch(bin, &isolated.argv, &served.rules)?;
    home::unconfigured(workdir)?;
    let hooks = gate::hooks(brokkr, !served.rules.is_empty())?;
    let written = Written {
        mcp: &served.document,
        hooks: &hooks,
    };
    Ok((home::made(homes, written)?, launch))
}

/// Run one agy turn under `home`: the prompt on stdin, every row through
/// [`AgyFold`], agy stopped at a refusing row, and the turn's verdict read
/// at its end. Each row is judged as agy wrote it, and what leaves the
/// driver passes the seat's bindings: every emitted row, every value of
/// the record, the refusal, the answer and the stderr (J2). A failure, and
/// a refusal behind a folded turn or a delivered result, ends the turn with
/// no result of its own: its words pass the bindings in [`drive`].
fn invoke(
    (launch, home): (&Launch, &Home),
    input: &Value,
    prompt: &str,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, Ended> {
    let mut fold = AgyFold::new(&launch.pin);
    let workdir = input.get("workdir").and_then(Value::as_str).unwrap_or("");
    let mut transcript = Transcript::resolve(TranscriptKind::None)?;
    // Each row the fold emits is the driver's own: its keys are the
    // checkpoint's vocabulary, and its values pass the bindings.
    let mut masked = |row: &Value| {
        let mut row = row.clone();
        let values = row
            .as_object_mut()
            .into_iter()
            .flat_map(|row| row.values_mut());
        values.for_each(|value| secret::mask_json(value, bindings));
        emit(&row);
    };
    // The engine's HOME, set on the child last, after the bindings (J1).
    let owned = [(HOME, home.path())];
    let line = stdin_line(prompt);
    let fed = spawn_fed_in(&launch.command, &line, (workdir, &owned), bindings)?;
    let mut session_meta = Map::new();
    let ended = streamed(fed, |event| {
        fold.read(&event, &mut session_meta, &mut masked)
    })?;
    transcript.finish(&mut session_meta, &mut masked);
    for value in session_meta.values_mut() {
        secret::mask_json(value, bindings);
    }
    let Turn {
        exit_code,
        stopped,
        captured,
    } = ended;
    let verdict = match stopped {
        Some(refused) => Verdict::Refused(refused),
        None => fold.verdict(exit_code),
    };
    let delivered = exit_code == 0 && super::delivered_result(input);
    let (refusal, answer) = judged(verdict, fold.worked() || delivered, bindings)?;
    // Evidence: agy's stderr, stripped and then masked, which the shared
    // driver's tail journals as it does every adapter's until #1199; a
    // signed-out agy's is its sign-in block, which is not evidence (J4).
    let stderr = match fold.signed_out() {
        true => SIGN_IN_WITHHELD.to_string(),
        false => captured.evidence(|raw| plain_evidence(raw, bindings)),
    };
    Ok(Invocation {
        exit_code,
        session_meta,
        stdout: String::new(),
        stderr,
        state: None,
        refusal,
        answer,
        launch: LaunchTerminal::Cold,
    })
}

/// The turn's refusal and answer from its `verdict`, each masked through
/// `bindings`, or how it ended with neither: a failure, or a refusal behind
/// work the seat already did. The answer rides as the claude and qoder
/// folds' would, where nothing journals it; the result file is the seat's
/// result (J7).
fn judged(
    verdict: Verdict,
    worked: bool,
    bindings: &[secret::BoundSecret],
) -> Result<(Option<PreSessionRefusal>, Option<Value>), Ended> {
    match verdict {
        Verdict::Failed(failure) => Err(Ended::Failed(failure)),
        Verdict::Refused(late) if worked => Err(Ended::Late(late)),
        Verdict::Refused(refused) => Ok((super::masked_refusal(Some(refused), bindings), None)),
        Verdict::Answer(response) => {
            let response = secret::mask_text(response.as_bytes(), bindings);
            Ok((None, Some(Value::String(response))))
        }
        Verdict::Exited => Ok((None, None)),
    }
}

#[cfg(test)]
mod tests;
