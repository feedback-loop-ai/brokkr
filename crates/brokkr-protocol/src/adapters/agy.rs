//! `brokkr driver agy`: Antigravity CLI as a provisional harness at level 1
//! (#1200; decision 0075) and, where its site boxes its hands, at level 3
//! (#1224). The seat is never resumed, every
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
//! - A boxed launch ([`Hands::Boxed`]) is left one tool, the engine's: no
//!   mode, the engine's rules alone, a gate that lists `call_mcp_tool`
//!   alone, and no `--allow` (the record's fourth measurement).
//! - The engine's HOME ([`home`]), its gate ([`gate`]) and the fold
//!   ([`fold`]) each have one home below.

use std::path::{Path, PathBuf};

use serde::Serialize;
use serde_json::{Map, Value};
use thiserror::Error;

use super::escapes::plain_evidence;
use super::mcp::{self, McpRefusal};
use super::spawn::spawn_fed_in;
use super::stream_turn::{streamed_lines, Turn};
use super::{secret, Invocation, LaunchTerminal};
use crate::transcript::{Kind as TranscriptKind, Transcript};

mod fold;
mod gate;
mod home;

use super::qoder::Observed;
use super::stream_turn::Line;
use crate::PreSessionRefusal;
use fold::{AgyFold, Failed, Verdict};
pub(super) use gate::{gate, gate_listed};
use home::{Home, Homes, Written};

/// What every agy launch runs first: a stream turn read from stdin.
const HEAD: [&str; 4] = [
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
];

/// The mode an unboxed launch runs in after [`HEAD`], accepting edits. A
/// boxed launch carries no mode, as every boxed turn of the record ran.
const ACCEPT_EDITS: [&str; 2] = ["--mode", "accept-edits"];

/// Whether a launch's hands are boxed: the one value that decides the
/// launch's mode, the gate's list and whether the seat's `--allow` is
/// admitted, made once in [`prepared_with`] from the engine's set (#1224).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hands<'a> {
    /// No engine server: level 1, the native tools on the gate's list, the
    /// seat's own grants, accepting edits.
    Native,
    /// The engine's set serves the box's hands, granted by these rules:
    /// `call_mcp_tool` alone on the gate's list, the rules alone in the
    /// settings, and no mode.
    Boxed(&'a [String]),
}

impl<'a> Hands<'a> {
    /// The hands the engine's set gives a launch granted `rules`: boxed
    /// where they grant anything, since no rule is no server.
    fn of(rules: &'a [String]) -> Hands<'a> {
        match rules.is_empty() {
            true => Hands::Native,
            false => Hands::Boxed(rules),
        }
    }

    /// The mode the launch runs in, after [`HEAD`].
    fn mode(self) -> &'static [&'static str] {
        match self {
            Hands::Native => &ACCEPT_EDITS,
            Hands::Boxed(_) => &[],
        }
    }
}

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

/// The names of the child's environment the engine owns for an agy seat,
/// which no seat binds: its HOME (J1), and its `PWD`, the resolved working
/// directory agy walks up from (L2).
const HOME: &str = "HOME";
const PWD: &str = "PWD";
const OWNED: [&str; 2] = [HOME, PWD];

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
    /// A seat binding one of [`OWNED`] would start agy under its own HOME,
    /// with the operator's servers, plugins and hooks (J1), or walking up
    /// another `PWD` than the one the workspace check read (L2).
    #[error(
        "refusing to invoke the agent CLI: the seat binds a secret named {0}, which the engine \
         owns for an agy seat (#1200)"
    )]
    Owned(&'static str),
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
    /// A working directory that cannot be canonicalized, named as the seat
    /// gave it (K4).
    #[error(
        "refusing to invoke the agent CLI: the agy seat's working directory {0} cannot be \
         resolved: {1}"
    )]
    Unrooted(String, #[source] std::io::Error),
    /// A resolved working directory the child's `PWD` cannot name, named
    /// escaped (L2).
    #[error(
        "refusing to invoke the agent CLI: the agy seat's working directory resolves to {0}, \
         which is not UTF-8, so the child's PWD cannot name it (#1200)"
    )]
    Untexted(String),
    /// A workspace that carries agy's own configuration (G6), in the
    /// working directory or a directory above it (L1, M6).
    #[error(
        "refusing to invoke the agent CLI: {} stands in the agy seat's working directory or a \
         directory above it, and brokkr driver agy refuses a seat there; remove it, or run the \
         seat elsewhere (#1200)",
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
    /// A boxed seat granted a native tool: every native tool is switched
    /// off beside boxed hands, so the grant would do nothing, and it is
    /// refused rather than dropped (#1224). Its value is never named.
    #[error(
        "refusing to invoke the agent CLI: the seat's arguments carry '--allow', and an agy seat \
         whose hands are boxed takes no native grant: its gate switches off every native tool, \
         so the grant would do nothing (#1224)"
    )]
    BoxedGrant,
    /// The launch's one MCP admission refused it, in the words every
    /// driver states it with ([`McpRefusal::at_launch`]).
    #[error("{}", .0.at_launch())]
    Mcp(#[from] McpRefusal),
}

/// How a seat ended with no invocation, typed until [`drive`] renders it
/// once (#1178 T4): its launch refused, its turn failed, a refusal behind
/// work already done, or a shared helper's own error, which arrives
/// rendered at its own boundary and is the one end that does (K10).
#[derive(Debug, Error)]
enum Ended {
    #[error(transparent)]
    Launch(#[from] LaunchError),
    #[error(transparent)]
    Failed(#[from] Failed),
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

/// The launch for `extra`, the arguments behind the driver's `--`, with
/// `hands`: the head, the mode, the pin, the effort where the seat names
/// one, and the one `--settings` value, which holds the seat's grants on a
/// native launch and the engine's rules alone on a boxed one, where a
/// seat's `--allow` refuses.
fn launch(bin: &str, extra: &[String], hands: Hands<'_>) -> Result<Launch, LaunchError> {
    let [pin, effort, allow] = admitted(extra)?;
    let pin = pin.ok_or(LaunchError::Unpinned)?;
    let allowed: Vec<&str> = match (hands, allow) {
        (Hands::Boxed(_), Some(_)) => return Err(LaunchError::BoxedGrant),
        (Hands::Boxed(rules), None) => rules.iter().map(String::as_str).collect(),
        (Hands::Native, allow) => allow.map_or_else(Vec::new, |allow| allow.split(',').collect()),
    };
    for rule in &allowed {
        targeted(target(rule).ok_or(LaunchError::Unruled)?)?;
    }
    let mut command = vec![bin.to_string()];
    command.extend(HEAD.iter().map(|part| part.to_string()));
    command.extend(hands.mode().iter().map(|part| part.to_string()));
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
/// last: no shared path masks an error a driver returns. A word of agy's
/// inside it was masked once already, whole, before it was cut (K3); this
/// second pass is the only one over the seat's own words and the shared
/// helpers' errors, and over what joining words could rejoin (K10).
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
/// before the one mask. A seat that binds a name of [`OWNED`] is refused
/// by the name, before anything else. Every launch is admitted cold.
fn turn(
    pinned: super::Pinned<'_>,
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, Ended> {
    let bound = |name: &&str| bindings.iter().any(|binding| binding.name() == *name);
    if let Some(name) = OWNED.into_iter().find(bound) {
        return Err(LaunchError::Owned(name).into());
    }
    let composed = super::composed_launch("agy", pinned.extra, input)?;
    let (temp, operator) = (std::env::temp_dir(), std::env::var_os("HOME"));
    let homes = Homes {
        temp: &temp,
        operator: operator.as_deref().map(Path::new),
    };
    let brokkr = mcp::engine_executable();
    // The HOME lives until the turn ends.
    let prepared = prepared(pinned.binary, &composed.extra, input, homes, &brokkr)?;
    invoke(&prepared, input, prompt, bindings, emit)
}

/// What one launch runs with: the engine's HOME, the launch, and the
/// resolved working directory, which the child runs in, its `PWD` names
/// (L2) and the workspace check reads directly before the spawn (M1).
struct Prepared {
    home: Home,
    launch: Launch,
    workdir: String,
}

/// The launch of `extra` before any provider work: SI2's one admission
/// ([`mcp::isolated`]), the engine's set read from the intent it checked
/// ([`mcp::agy_set`]), the launch, the working directory resolved, and the
/// engine's HOME made under `homes` with the set's document and the gate
/// run by `brokkr`. Whether the hands are boxed is read here, once, from
/// the set ([`Hands::of`]), and decides the launch and the gate alike. No
/// launch is offered a rejoin, so the cold assessment is the one admitted.
fn prepared(
    bin: &str,
    extra: &[String],
    input: &Value,
    homes: Homes<'_>,
    brokkr: &Path,
) -> Result<Prepared, LaunchError> {
    prepared_with(bin, extra, input, (homes, brokkr), home::made)
}

/// [`prepared`], the engine's HOME made by `made`.
fn prepared_with(
    bin: &str,
    extra: &[String],
    input: &Value,
    (homes, brokkr): (Homes<'_>, &Path),
    made: impl FnOnce(Homes<'_>, Written<'_>) -> Result<Home, LaunchError>,
) -> Result<Prepared, LaunchError> {
    let edge = mcp::Edge::new(input);
    let isolated = mcp::isolated("agy", &edge, extra, false)?;
    let workdir = input.get("workdir").and_then(Value::as_str).unwrap_or("");
    let served = mcp::agy_set(&edge, &isolated, workdir)?;
    let hands = Hands::of(&served.rules);
    let launch = launch(bin, &isolated.argv, hands)?;
    let workdir = home::resolved(workdir)?;
    let hooks = gate::hooks(brokkr, hands)?;
    let written = Written {
        mcp: &served.document,
        hooks: &hooks,
    };
    let home = made(homes, written)?;
    Ok(Prepared {
        home,
        launch,
        workdir,
    })
}

/// Run one agy turn under `home`: the workspace checked directly before
/// agy starts ([`home::unconfigured`]), the prompt on stdin, every line of its
/// stdout through [`AgyFold`] as agy wrote its bytes, to its end, agy
/// stopped at a refusing row, and the turn's verdict read at its end in
/// one order (K2). Each value is masked once (K10): the shared driver masks
/// every emitted row, every value of the record and the refusal, so they
/// leave here as agy's fold made them, and this driver masks what no shared
/// path does, the answer and the stderr. A failure, and a refusal behind a
/// folded turn or a delivered result, ends the turn with no result of its
/// own and with its words alone, as qoder's late path does: they pass the
/// bindings in [`drive`], and carry no stderr (K8).
fn invoke(
    prepared: &Prepared,
    input: &Value,
    prompt: &str,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, Ended> {
    let Prepared {
        home,
        launch,
        workdir,
    } = prepared;
    let mut fold = AgyFold::new(&launch.pin, bindings);
    let mut transcript = Transcript::resolve(TranscriptKind::None)?;
    // The engine's HOME and PWD, set on the child last, after the bindings
    // (J1, L2); the child runs in the directory its PWD names.
    let owned = [(HOME, home.path()), (PWD, workdir.as_str())];
    let line = stdin_line(prompt);
    // The workspace check, once, last before agy starts (M1): it narrows
    // the time a writer has to plant agy's configuration, and does not close
    // it (#1217).
    home::unconfigured(workdir)?;
    let fed = spawn_fed_in(&launch.command, &line, (workdir, &owned), bindings)?;
    let mut session_meta = Map::new();
    let ended = streamed_lines(fed, |line| {
        match fold.read(line, &mut session_meta, &mut *emit) {
            Ok(()) => Line::Read,
            Err(refused) => Line::Stopped(refused),
        }
    })?;
    transcript.finish(&mut session_meta, &mut *emit);
    concluded(fold, ended, session_meta, input, bindings)
}

/// The invocation of a turn read to `ended`, judged by `fold`: a stdout
/// that could not be read to its end is the turn's fault (L3), and the
/// verdict is read once.
fn concluded(
    mut fold: AgyFold<'_>,
    ended: Turn,
    session_meta: Map<String, Value>,
    input: &Value,
    bindings: &[secret::BoundSecret],
) -> Result<Invocation, Ended> {
    let Turn {
        exit_code,
        captured,
        unread,
        ..
    } = ended;
    fold.unreadable(unread);
    // Read unmasked so no binding erases the marker, and only matched (J4).
    let signed_out = fold.signed_out() || Observed::of(captured.bytes()).said(fold::SIGNED_OUT);
    let verdict = fold.verdict(exit_code, signed_out);
    let delivered = exit_code == 0 && super::delivered_result(input);
    let (refusal, answer) = judged(verdict, fold.worked() || delivered, bindings)?;
    // Evidence: agy's stderr, stripped and then masked, which the shared
    // driver's tail journals as it does every adapter's until #1199; a
    // signed-out agy's is its sign-in block, which is not evidence (J4).
    let stderr = match signed_out {
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

/// The turn's refusal and answer from its `verdict`, or how it ended with
/// neither: a failure, or a refusal behind work the seat already did. The
/// refusal is masked where the shared driver masks every adapter's
/// ([`super::masked_refusal`]); the answer, which no shared path reads,
/// passes `bindings` here, and rides as the claude and qoder folds' would,
/// where nothing journals it; the result file is the seat's result (J7).
fn judged(
    verdict: Verdict,
    worked: bool,
    bindings: &[secret::BoundSecret],
) -> Result<(Option<PreSessionRefusal>, Option<Value>), Ended> {
    match verdict {
        Verdict::Failed(failed) => Err(Ended::Failed(failed)),
        Verdict::Refused(late) if worked => Err(Ended::Late(late)),
        Verdict::Refused(refused) => Ok((Some(refused), None)),
        Verdict::Answer(response) => {
            let response = secret::mask_text(response.as_bytes(), bindings);
            Ok((None, Some(Value::String(response))))
        }
    }
}

#[cfg(test)]
mod tests;
