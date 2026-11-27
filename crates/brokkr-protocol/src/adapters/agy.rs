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
use super::spawn::{spawn_fed_in, Unstarted};
use super::stream_turn::{streamed_lines, Turn, Unconcluded};
use super::{secret, Emitted, Invocation, LaunchTerminal};

mod fold;
mod gate;
mod home;
mod keep;
mod rejoin;
mod restore;

use super::stream_turn::{plain, Observed, PLAIN_ID_LIMIT};
use super::LaunchHold;
use crate::native_controls::SealedBoundary;
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
/// admitted, made once in [`prepared_with`] from the launch's sealed
/// inputs (#1224).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Hands<'a> {
    /// No sealed boundary that boxes: level 1, the native tools on the
    /// gate's list, the seat's own grants, accepting edits.
    Native,
    /// The engine's set serves the box's hands, granted by these rules:
    /// `call_mcp_tool` alone on the gate's list, the rules alone in the
    /// settings, and no mode.
    Boxed(&'a [String]),
}

impl<'a> Hands<'a> {
    /// The hands of a launch whose sealed inputs stand under `boxing`, the
    /// boundary that boxes them ([`mcp::agy_boxing`]), and whose set grants
    /// `rules`: native where nothing boxes, boxed where the set serves the
    /// box's hands, and refused where a boundary boxes and they were not
    /// served, so a seat compiled as boxed never runs with agy's native
    /// tools.
    fn of(boxing: Option<SealedBoundary>, rules: &'a [String]) -> Result<Hands<'a>, LaunchError> {
        match boxing {
            None => Ok(Hands::Native),
            Some(boundary) if rules.is_empty() => Err(LaunchError::Unboxed(boundary)),
            Some(_) => Ok(Hands::Boxed(rules)),
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
/// which no seat binds: its HOME (J1), its `PWD`, the resolved working
/// directory agy walks up from (L2), and the switch that keeps a turn from
/// starting agy's self-updater (`updater-default`, `updater-disabled-true`;
/// decision 0124).
const HOME: &str = "HOME";
const PWD: &str = "PWD";
const AUTO_UPDATE: &str = "AGY_CLI_DISABLE_AUTO_UPDATE";
const OWNED: [&str; 3] = [HOME, PWD, AUTO_UPDATE];

/// The one value of [`AUTO_UPDATE`] measured to stop the updater; `1` does
/// not (`updater-disabled`).
const NO_UPDATE: &str = "true";

/// Why a seat is not launched. Each names what was written, never a value
/// (AS3).
#[derive(Debug, Error)]
pub(super) enum LaunchError {
    /// A seat binding one of [`OWNED`] would start agy under its own HOME,
    /// with the operator's servers, plugins and hooks (J1), walking up
    /// another `PWD` than the one the workspace check read (L2), or free to
    /// let agy replace the binary its facts were measured on.
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
    /// The pin is not a plain model id ([`plain`]), so the words that name
    /// it could escape it past the mask. They name no byte of it (#1178 Z1).
    #[error(
        "refusing to invoke the agent CLI: the agy model pin is not a plain model id (1 to {} \
         characters: an ASCII letter or digit, then ASCII letters, digits, '.', '_' and '-')",
        PLAIN_ID_LIMIT
    )]
    Unplain,
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
    /// A resolved working directory the child's `PWD` cannot name (L2).
    /// Nothing of the path is named: escaped, a bound value standing in it
    /// would no longer match the mask (#1200 U1).
    #[error(
        "refusing to invoke the agent CLI: the agy seat's working directory resolves to a path \
         that is not UTF-8, so the child's PWD cannot name it (#1200)"
    )]
    Untexted,
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
        "refusing to invoke the agent CLI: the engine's HOME for the agy seat was not made: \
         hands session: {0}"
    )]
    Unsessioned(#[source] std::io::Error),
    /// The engine HOME's session could not take its lock. Like every
    /// refusal below the temporary directory, it carries no path: one that
    /// is not UTF-8 renders lossily, and a bound value standing in it would
    /// no longer match the mask (#1200 V1).
    #[error(
        "refusing to invoke the agent CLI: the engine's HOME for the agy seat was not made: its \
         session's lock could not be taken: {0}"
    )]
    Unlocked(#[source] std::io::Error),
    /// The one parent of the engine's HOME directories, under the
    /// temporary directory, is not this user's alone (J5). It names no path
    /// (#1200 V1).
    #[error(
        "refusing to invoke the agent CLI: the parent of the engine's HOME directories under the \
         temporary directory is not a directory of this user's that nobody else may enter, so it \
         holds no engine HOME for the agy seat (#1200)"
    )]
    Unowned,
    /// A boxed seat granted a native tool: every native tool is switched
    /// off beside boxed hands, so the grant would do nothing, and it is
    /// refused rather than dropped (#1224). Its value is never named.
    #[error(
        "refusing to invoke the agent CLI: the seat's arguments carry '--allow', and an agy seat \
         whose hands are boxed takes no native grant: its gate switches off every native tool, \
         so the grant would do nothing (#1224)"
    )]
    BoxedGrant,
    /// The sealed inputs stand under a boundary that boxes the seat's
    /// hands, named by its word, and the box's hands were not served: the
    /// standing fence served the shape as before, or no hands were declared
    /// (#1224).
    #[error(
        "refusing to invoke the agent CLI: the agy seat stands under the {} boundary, which \
         boxes its hands, and the box's hands were not served, so agy would run with its native \
         tools; an agy seat under a boundary that boxes is boxed or refused (#1224)",
        .0.word()
    )]
    Unboxed(SealedBoundary),
    /// The sealed inputs stand under a boundary that boxes the seat's
    /// hands, and the launch's one MCP admission refused them, for the
    /// cause it states (#1224).
    #[error(
        "refusing to invoke the agent CLI: the agy seat stands under the {} boundary, which \
         boxes its hands, and the box's hands were not served: {}; an agy seat under a boundary \
         that boxes is boxed or refused (#1224)",
        .0.word(),
        .1
    )]
    Unserved(SealedBoundary, #[source] McpRefusal),
    /// The launch's one MCP admission refused it, in the words every
    /// driver states it with ([`McpRefusal::at_launch`]).
    #[error("{}", .0.at_launch())]
    Mcp(#[from] McpRefusal),
    /// `resume_context.declared_transcript_kind` was present with a value the
    /// driver cannot read as [`crate::DeclaredTranscriptKind`] (#1227 W6).
    #[error("refusing to invoke the agent CLI: {0}")]
    DeclaredTranscriptKind(#[source] crate::InvalidDeclaredTranscriptKind),
    /// `resume_context.declared_root_session_kind` was present with a value the
    /// driver cannot read as a declared root session kind (#1227).
    #[error("refusing to invoke the agent CLI: {0}")]
    DeclaredRootSessionKind(#[source] crate::declared_transcript::InvalidDeclaredRootSessionKind),
}

/// How a seat ended with no invocation, typed until [`drive`] renders it
/// once (#1178 T4): its launch refused, agy not started or not waited on,
/// its turn failed, a refusal behind work already done, or a shared
/// helper's own error, which arrives rendered at its own boundary and is
/// the one end that does (K10).
#[derive(Debug, Error)]
enum Ended {
    #[error(transparent)]
    Launch(#[from] LaunchError),
    #[error(transparent)]
    Unstarted(#[from] Unstarted),
    #[error(transparent)]
    Unconcluded(#[from] Unconcluded),
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
    if !plain(pin) {
        return Err(LaunchError::Unplain);
    }
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

/// How the caller started this agy seat: any offered conversation id and
/// whether this is a one-turn probe whose checkpoints are discarded (#1227 U4).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct StartMode<'a> {
    pub(super) session: Option<&'a str>,
    pub(super) probe: bool,
}

/// One agy seat, start to end. Every error the seat ends with, its launch's
/// and its turn's alike, is rendered here, once, and passes the seat's
/// bindings last: no shared path masks an error a driver returns.
pub(super) fn drive(
    pinned: super::Pinned<'_>,
    mode: StartMode<'_>,
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Emitted),
) -> Result<Invocation, String> {
    turn(pinned, mode, (prompt, input, bindings), emit).map_err(|ended| rendered(&ended, bindings))
}

/// How a seat `ended`, in its words, masked once through `bindings`: the
/// one render [`drive`] gives every end.
fn rendered(ended: &Ended, bindings: &[secret::BoundSecret]) -> String {
    secret::mask_text(ended.to_string().as_bytes(), bindings)
}

/// The seat's composed arguments, the launch [`prepared`] from them under
/// the driver's own temporary root, HOME and executable, and its turn,
/// before the one mask. A seat binding a name of [`OWNED`] is refused before
/// anything else.
fn turn(
    pinned: super::Pinned<'_>,
    mode: StartMode<'_>,
    (prompt, input, bindings): (&str, &Value, &[secret::BoundSecret]),
    emit: &mut impl FnMut(&Emitted),
) -> Result<Invocation, Ended> {
    turn_with(
        pinned,
        mode,
        (prompt, input, bindings),
        emit,
        &mut rejoin::noop_hook,
    )
}

fn turn_with(
    pinned: super::Pinned<'_>,
    mode: StartMode<'_>,
    (prompt, input, bindings): (&str, &Value, &[secret::BoundSecret]),
    emit: &mut impl FnMut(&Emitted),
    on_restore: &mut dyn FnMut(rejoin::CopyPoint<'_>),
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
    // Composed fresh before any kept conversation is touched (#1227 L7); the
    // engine HOME lives until the turn ends.
    let prepared = prepared(
        pinned.binary,
        &composed.extra,
        input,
        mode.session.is_some() && !mode.probe,
        homes,
        &brokkr,
    )?;
    invoke_with(&prepared, mode, input, prompt, bindings, emit, on_restore)
}

/// What one launch runs with: the engine's HOME, the launch, and the
/// resolved working directory, which the child runs in, its `PWD` names
/// (L2) and the workspace check reads directly before the spawn (M1).
struct Prepared {
    home: Home,
    operator_home: Option<PathBuf>,
    declared: bool,
    declared_root: bool,
    isolated_resumes: bool,
    launch: Launch,
    workdir: String,
}

fn prepared(
    bin: &str,
    extra: &[String],
    input: &Value,
    offered: bool,
    homes: Homes<'_>,
    brokkr: &Path,
) -> Result<Prepared, LaunchError> {
    prepared_with(bin, extra, input, offered, (homes, brokkr), home::made)
}

/// [`prepared`], the engine's HOME made by `made`.
fn prepared_with(
    bin: &str,
    extra: &[String],
    input: &Value,
    offered: bool,
    (homes, brokkr): (Homes<'_>, &Path),
    made: impl FnOnce(Homes<'_>, Written<'_>) -> Result<Home, LaunchError>,
) -> Result<Prepared, LaunchError> {
    let edge = mcp::Edge::new(input);
    let boxing = mcp::agy_boxing(&edge)?;
    let unserved = |refusal| match boxing {
        Some(boundary) => LaunchError::Unserved(boundary, refusal),
        None => LaunchError::Mcp(refusal),
    };
    let isolated = mcp::isolated("agy", &edge, extra, offered).map_err(unserved)?;
    let isolated_resumes = isolated.resumes("agy", super::AGY_SHAPE);
    let workdir = input.get("workdir").and_then(Value::as_str).unwrap_or("");
    let served = mcp::agy_set(&edge, &isolated, workdir).map_err(unserved)?;
    let hands = Hands::of(boxing, &served.rules)?;
    let launch = launch(bin, &isolated.argv, hands)?;
    let workdir = home::resolved(workdir)?;
    let hooks = gate::hooks(brokkr, hands)?;
    let written = Written {
        mcp: &served.document,
        hooks: &hooks,
    };
    let declared = crate::DeclaredTranscriptKind::read_from(input)
        .map_err(LaunchError::DeclaredTranscriptKind)?
        .is_some_and(|kind| kind.is_agy_conversation());
    let declared_root = crate::DeclaredTranscriptKind::read_root_session_from(input)
        .map_err(LaunchError::DeclaredRootSessionKind)?
        .is_some_and(|kind| kind.is_agy_conversation());
    let operator_home = homes.operator.map(Path::to_path_buf);
    let home = made(homes, written)?;
    Ok(Prepared {
        home,
        operator_home,
        declared,
        declared_root,
        isolated_resumes,
        launch,
        workdir,
    })
}

#[cfg(test)]
fn invoke(
    prepared: &Prepared,
    mode: StartMode<'_>,
    input: &Value,
    prompt: &str,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Emitted),
) -> Result<Invocation, Ended> {
    invoke_with(
        prepared,
        mode,
        input,
        prompt,
        bindings,
        emit,
        &mut rejoin::noop_hook,
    )
}

fn invoke_with(
    prepared: &Prepared,
    mode: StartMode<'_>,
    input: &Value,
    prompt: &str,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Emitted),
    on_restore: &mut dyn FnMut(rejoin::CopyPoint<'_>),
) -> Result<Invocation, Ended> {
    let owned = [
        (HOME, prepared.home.path()),
        (PWD, prepared.workdir.as_str()),
        (AUTO_UPDATE, NO_UPDATE),
    ];
    let line = stdin_line(prompt);
    home::unconfigured(&prepared.workdir)?;
    let planned = restore::plan_launch_with(prepared, mode, input, &owned, bindings, on_restore);
    // Held across restore, turn and post-turn keep; puts `rejoin/` back only
    // when the rejoin is never confirmed, otherwise cleans up the set-aside
    // copy on drop while still holding the exclusive lock.
    let mut lease = planned.lock;
    let command = planned.plan.command.clone();
    let mut hold = LaunchHold::new("agy", planned.plan);
    let mut fold = AgyFold::new(&prepared.launch.pin, bindings);
    // Workspace admission is the last check before spawn, after restore.
    home::unconfigured(&prepared.workdir)?;
    let fed = spawn_fed_in(&command, &line, (&prepared.workdir, &owned), bindings)?;
    let mut session_meta = Map::new();
    let ended = streamed_lines(fed, |line| {
        let step = fold.read(line, &mut session_meta, &mut hold, &mut *emit);
        if let Some(lease) = lease.as_mut() {
            if hold.terminal() == LaunchTerminal::Resumed {
                lease.confirm();
            }
        }
        step
    })?;
    let conversation = fold.take_conversation();
    let pin_confirmed = fold.pinned();
    let launch_terminal = hold.terminal();
    let Concluded {
        mut outcome,
        began_work,
        result_exists,
    } = concluded(fold, ended, session_meta, input, bindings, launch_terminal);
    if let Some(notice) = planned.restore_notice {
        keep::report_notice(&mut outcome, &notice);
    }
    let refused_to_start = outcome.as_ref().is_ok_and(|inv| {
        super::lost::refusal_to_start(
            inv.refusal.clone(),
            began_work,
            || result_exists,
            inv.exit_code,
            inv.launch,
        )
        .is_some()
    });
    rejoin::publish_turn(
        rejoin::TurnKeep {
            declared: rejoin::DeclaredKinds {
                transcript: prepared.declared,
                root: prepared.declared_root,
            },
            probe: mode.probe,
            refused_to_start,
            pin_confirmed,
            conversation: &conversation,
            engine_home: Path::new(prepared.home.path()),
            operator_home: prepared.operator_home.as_deref(),
            bindings,
        },
        &mut hold,
        &mut outcome,
        emit,
    );
    outcome
}

struct Concluded {
    outcome: Result<Invocation, Ended>,
    began_work: bool,
    result_exists: bool,
}

fn concluded(
    mut fold: AgyFold<'_>,
    ended: Turn,
    session_meta: Map<String, Value>,
    input: &Value,
    bindings: &[secret::BoundSecret],
    launch: LaunchTerminal,
) -> Concluded {
    let Turn {
        exit_code,
        clean,
        captured,
        unread,
    } = ended;
    fold.unreadable(unread);
    let signed_out = fold.signed_out() || Observed::of(captured.bytes()).said(fold::SIGNED_OUT);
    let result_file = super::lost::ResultFile::observe(input);
    let result_exists = result_file.exists();
    let began_work = fold.worked();
    let stderr = match signed_out {
        true => SIGN_IN_WITHHELD.to_string(),
        false => captured.evidence(|raw| plain_evidence(raw, bindings)),
    };
    if launch.is_unsettled() {
        return Concluded {
            outcome: Ok(Invocation {
                exit_code,
                session_meta: Map::new(),
                notes: String::new(),
                stderr,
                state: None,
                refusal: None,
                answer: None,
                launch,
                result_exists: Some(result_exists),
            }),
            began_work: false,
            result_exists,
        };
    }
    let verdict = fold.verdict(exit_code, signed_out);
    let delivered = clean && result_file.delivered();
    let (refusal, answer) = match judged(verdict, began_work || delivered, bindings) {
        Ok(judged) => judged,
        Err(ended) => {
            return Concluded {
                outcome: Err(ended),
                began_work,
                result_exists,
            };
        }
    };
    Concluded {
        outcome: Ok(Invocation {
            exit_code,
            session_meta,
            notes: String::new(),
            stderr,
            state: None,
            refusal,
            answer,
            launch,
            result_exists: Some(result_exists),
        }),
        began_work,
        result_exists,
    }
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
pub(super) mod tests;
