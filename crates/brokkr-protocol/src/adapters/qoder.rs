//! `brokkr driver qoder`: Qoder CLI as a provisional harness at level 1
//! (#1178; decision 0075). The seat is unboxed and never resumed, every
//! model it serves is provisional, and the facts below come from the
//! controller's hand measurement of Qoder CLI 1.1.92
//! (`docs/evidence/adapters/qoder-cli-1-1.md`).
//!
//! Qoder's stream is Claude Code's shape, so its rows go through the one
//! Claude fold (decision 0071 ruling 5). Two things are read here first,
//! because Qoder's turn differs from Claude's in them:
//!
//! - the served model is init's `model` and nothing else: Qoder falls back
//!   to `Auto` for a model it cannot serve and exits 0, and its assistant
//!   rows name an internal code (`fast`, `qmodel_latest`), so a seat whose
//!   init does not name the pinned id exactly is refused before any row of
//!   it counts;
//! - its session is never persisted (`--no-session-persistence`), so its
//!   session id locates nothing and is dropped: the transcript is `none`.
//!
//! Its cost is credits, and every dollar and token figure it reports is 0;
//! the fold already reads a zero as not reported. Qoder's two system
//! subtypes Claude Code does not write, `artifacts_update` and
//! `hook_diagnostics`, are rows the fold passes over as it passes over any
//! row it does not read.

use std::path::Path;

use serde_json::{Map, Value};
use thiserror::Error;

use super::claude_fold::ClaudeFold;
use super::escapes::{plain_evidence, without_ansi};
use super::mcp::{self, McpRefusal};
use super::refusal::refusal_reason;
use super::stream_turn::{streamed, Turn};
use super::{fold_stream_event, secret, spawn_fed, Invocation, LaunchTerminal};
use crate::native_controls::mcp::qoder::isolation;
use crate::transcript::{Kind as TranscriptKind, Transcript};
use crate::PreSessionRefusal;

/// What every qoder seat runs before its own pin, effort and grants: a
/// print-mode stream turn that keeps no session, accepting edits.
const HEAD: [&str; 6] = [
    "-p",
    "--output-format",
    "stream-json",
    "--no-session-persistence",
    "--permission-mode",
    "accept_edits",
];

/// The native tools a level-1 seat holds. Without `--tools` the CLI keeps
/// its 34-tool default, `Agent`, `Workflow`, the cron tools and the web
/// tools among them (the record's Tools facts).
const TOOLS: [&str; 7] = ["--tools", "Read", "Edit", "Write", "Glob", "Grep", "Bash"];

/// The MCP meta-tools a seat the engine serves no server is denied:
/// `--tools ""` still leaves them, and only `--disallowed-tools` removes
/// them (the record's Tools facts). A seat served one keeps them, because
/// the engine's server is reached through them (Q14, Q15).
const META_DENIED: [&str; 2] = ["--disallowed-tools", "mcp_call,mcp_get,mcp_list"];

/// What the engine's MCP set gives one launch: the file the engine wrote
/// its document to, and the grant of each tool that document serves.
#[derive(Debug, Clone, Copy)]
pub(super) struct Engine<'a> {
    pub(super) file: &'a str,
    pub(super) grants: &'a [String],
}

/// The three controls a seat's argv may carry, each once and each with one
/// value: the adapter's model flag, its effort flag and its tool
/// permissions flag (`adapters/qoder.json`).
const CONTROLS: [&str; 3] = ["-m", "--reasoning-effort", "--allowed-tools"];

/// The stderr line Qoder writes when it serves another model than the one
/// asked for (the record's unknown-model turn).
const FALLBACK: &str = "is not available right now; using";

/// What a signed-out Qoder writes to stderr, and what its crash on an
/// option it refuses writes (the record's two refusals).
const SIGNED_OUT: &str = "Not logged in";
const CRASHED: &str = "unexpected critical error";

/// Why a seat's argv is not launched. Each names what was written, never a
/// value (AS3).
#[derive(Debug, Error)]
pub(super) enum LaunchError {
    /// Qoder keeps no session (`--no-session-persistence`), so a start
    /// offered a rejoin has nothing to rejoin and is refused (U4).
    #[error(
        "refusing to invoke the agent CLI: the start offers a resume, and the qoder adapter \
         declares no resume (#1178)"
    )]
    Resumed,
    #[error(
        "refusing to invoke the agent CLI: the seat's arguments carry '{0}', which brokkr \
         driver qoder does not admit; a qoder seat's arguments are -m, --reasoning-effort \
         and --allowed-tools, each once (#1178)"
    )]
    Unadmitted(String),
    #[error(
        "refusing to invoke the agent CLI: the seat's arguments carry a value with no option \
         before it, which brokkr driver qoder does not admit (#1178)"
    )]
    Positional,
    #[error("refusing to invoke the agent CLI: the seat's arguments carry '{0}' more than once")]
    Doubled(&'static str),
    #[error("refusing to invoke the agent CLI: the seat's arguments carry '{0}' with no value")]
    Valueless(&'static str),
    #[error(
        "refusing to invoke the agent CLI: the seat pins no model with -m, and Qoder serves a \
         model of its own choosing when none is named; a provisional seat pins its id (#1178)"
    )]
    Unpinned,
    #[error(
        "refusing to invoke the agent CLI: the engine's MCP file for the qoder seat has a path \
         that is not UTF-8, so no argument names it exactly (#1178)"
    )]
    Unnamed,
    /// The engine's MCP file could not be written.
    #[error(
        "refusing to invoke the agent CLI: the engine's MCP file for the qoder seat was not \
         written: {0}"
    )]
    Unstaged(#[source] std::io::Error),
    /// The launch's one MCP admission refused it, in the words every
    /// driver states it with ([`McpRefusal::at_launch`]).
    #[error("{}", .0.at_launch())]
    Mcp(#[from] McpRefusal),
}

/// One qoder launch: the command spawned and the id it pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Launch {
    pub(super) command: Vec<String>,
    pub(super) pin: String,
}

/// The launch for `extra`, the arguments behind the driver's `--`, served
/// `engine`'s set: the head, the seat's own controls in the order it wrote
/// them, the engine's grants joined to its one `--allowed-tools` value,
/// the MCP isolation in its one home, and the tool lists last. Nothing
/// else in `extra` is launched: a token this driver does not admit
/// refuses before any provider work.
fn launch(bin: &str, extra: &[String], engine: Engine<'_>) -> Result<Launch, LaunchError> {
    let mut pin = None;
    let mut seen: Vec<&'static str> = Vec::new();
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        let Some(control) = CONTROLS.iter().copied().find(|control| control == part) else {
            if part.starts_with('-') {
                return Err(LaunchError::Unadmitted(part.clone()));
            }
            return Err(LaunchError::Positional);
        };
        if seen.contains(&control) {
            return Err(LaunchError::Doubled(control));
        }
        seen.push(control);
        let value = parts
            .next()
            .filter(|value| !value.is_empty() && !value.starts_with('-'))
            .ok_or(LaunchError::Valueless(control))?;
        if control == CONTROLS[0] {
            pin = Some(value.clone());
        }
    }
    let pin = pin.ok_or(LaunchError::Unpinned)?;
    let mut command = vec![bin.to_string()];
    command.extend(HEAD.iter().map(|part| part.to_string()));
    command.extend(granted(extra, engine.grants));
    command.extend(isolation(engine.file));
    command.extend(TOOLS.iter().map(|part| part.to_string()));
    if engine.grants.is_empty() {
        command.extend(META_DENIED.iter().map(|part| part.to_string()));
    }
    Ok(Launch { command, pin })
}

/// The seat's admitted controls with `grants` joined after the value of
/// its one `--allowed-tools`, comma-separated as the adapter's separator,
/// or written as that control where the seat grants nothing of its own: so
/// the control appears once. No grant changes no byte.
fn granted(extra: &[String], grants: &[String]) -> Vec<String> {
    let mut seat = extra.to_vec();
    if grants.is_empty() {
        return seat;
    }
    let allowed = CONTROLS[2];
    // Every value was read as no option, so the control is found by itself.
    match seat.iter().position(|part| part == allowed) {
        Some(at) => seat[at + 1] = [&seat[at + 1..at + 2], grants].concat().join(","),
        None => seat.extend([allowed.to_string(), grants.join(",")]),
    }
    seat
}

/// One qoder seat, start to end: its composed arguments, the launch
/// [`prepared`] from them, and its turn. `offered` is whether the start
/// offered a rejoin: the qoder adapter declares no resume, so an offer is
/// refused before any provider work, and every launch is admitted cold.
/// The launch's errors are rendered here, once, and every error the seat
/// ends with, launch, staging, admission, composition, the resume refusal
/// and the turn's own alike, passes the seat's bindings once, last (V4).
pub(super) fn drive(
    bin: &str,
    extra: &[String],
    offered: bool,
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    // The seat before its one mask: its errors rendered, not masked.
    let mut driven = || {
        if offered {
            return Err(LaunchError::Resumed.to_string());
        }
        let composed = super::composed_launch("qoder", extra, input)?;
        let prepared = prepared(bin, &composed.extra, input, &std::env::temp_dir());
        // The file lives until the turn ends.
        let (_staged, launch) = prepared.map_err(|error| error.to_string())?;
        invoke(&launch, prompt, input, bindings, emit)
    };
    driven().map_err(|error| secret::mask_text(error.as_bytes(), bindings))
}

/// The launch of `extra` before any provider work: SI2's one admission
/// ([`mcp::isolated`]), as every built-in harness is admitted, then the
/// engine's set read from the intent it checked ([`mcp::qoder_set`]), its
/// document written to a private file under `root` that the launch names,
/// and the launch. No launch is offered a rejoin, so the cold assessment is
/// the one admitted.
fn prepared(
    bin: &str,
    extra: &[String],
    input: &Value,
    root: &Path,
) -> Result<(tempfile::TempDir, Launch), LaunchError> {
    let edge = mcp::Edge::new(input);
    let isolated = mcp::isolated("qoder", &edge, extra, false)?;
    let workdir = input.get("workdir").and_then(Value::as_str).unwrap_or("");
    let served = mcp::qoder_set(&edge, &isolated, workdir)?;
    let staged = staged(&served.document, root)?;
    let file = staged.path().join(MCP_FILE);
    let engine = Engine {
        file: file.to_str().ok_or(LaunchError::Unnamed)?,
        grants: &served.grants,
    };
    let launch = launch(bin, &isolated.argv, engine)?;
    Ok((staged, launch))
}

/// The name of the engine's MCP document in its private directory.
const MCP_FILE: &str = "mcp.json";

/// `document` written as [`MCP_FILE`] in a private directory of its own
/// under `root`, which goes with the value.
fn staged(document: &Value, root: &Path) -> Result<tempfile::TempDir, LaunchError> {
    let written = (tempfile::Builder::new().prefix("brokkr-qoder-mcp-"))
        .tempdir_in(root)
        .and_then(|dir| mcp::written(&dir.path().join(MCP_FILE), document).map(|()| dir));
    written.map_err(LaunchError::Unstaged)
}

/// Qoder's rows read for the model they served, before the Claude fold
/// reads them for everything else.
pub(super) struct QoderFold {
    pin: String,
    /// What init named, once it has: by then it equals the pin. An init
    /// that names no model leaves it unset (V5).
    served: Option<String>,
    claude: ClaudeFold,
}

impl QoderFold {
    pub(super) fn new(pin: &str) -> QoderFold {
        QoderFold {
            pin: pin.to_string(),
            served: None,
            claude: ClaudeFold::default(),
        }
    }

    /// Whether a row has been folded as a turn of the seat's.
    pub(super) fn worked(&self) -> bool {
        self.claude.turns > 0
    }

    /// One row. `Err` is the seat's refusal: init named another model than
    /// the pin, or a turn arrived before any init named one. Nothing of a
    /// refused row is folded.
    pub(super) fn read(
        &mut self,
        mut event: Value,
        session_meta: &mut Map<String, Value>,
        transcript: &mut Transcript,
        emit: &mut impl FnMut(&Value),
    ) -> Result<Option<PreSessionRefusal>, PreSessionRefusal> {
        let kind = event.get("type").and_then(Value::as_str);
        let assistant = kind == Some("assistant");
        if kind == Some("system") && event["subtype"] == "init" {
            match event.get("model").and_then(Value::as_str) {
                Some(served) if served == self.pin => self.served = Some(served.to_string()),
                Some(served) => return Err(substituted(&self.pin, Some(served))),
                // An init that names nothing names nothing (V5).
                None => {}
            }
        } else if self.served.is_none() && matches!(kind, Some("assistant" | "user" | "result")) {
            return Err(unconfirmed(&self.pin));
        }
        let Some(row) = event.as_object_mut() else {
            return Ok(None);
        };
        // Nothing persists the session, so its id locates nothing.
        row.remove("session_id");
        // An assistant row names Qoder's internal code; init named the model.
        let message = row.get_mut("message").and_then(Value::as_object_mut);
        if let (true, Some(served), Some(message)) = (assistant, &self.served, message) {
            message.insert("model".into(), Value::String(served.clone()));
        }
        Ok(fold_stream_event(
            &event,
            &mut self.claude,
            session_meta,
            transcript,
            emit,
        ))
    }

    /// The refusal of the model a finished turn was served by, where the
    /// stream left one to state: Qoder's fallback line on stderr, which
    /// refuses the seat whatever init said and with no init at all, or a
    /// clean exit no init row confirmed.
    ///
    /// `observed` is Qoder's stderr NOT masked, so no binding can erase a
    /// marker it overlaps (U2). It is only matched here: no byte of it
    /// enters the refusal's words.
    pub(super) fn identity(
        &self,
        observed: &Observed,
        exit_code: i32,
    ) -> Option<PreSessionRefusal> {
        if observed.said(FALLBACK) {
            return Some(substituted(&self.pin, None));
        }
        (exit_code == 0 && self.served.is_none()).then(|| unconfirmed(&self.pin))
    }

    /// The refusal a CLI that exited non-zero states on stderr: signed out,
    /// or crashed on an option it refuses. `observed` is read as in
    /// [`QoderFold::identity`], and only matched: the words are fixed.
    pub(super) fn failed(observed: &Observed, exit_code: i32) -> Option<PreSessionRefusal> {
        let said = |marker: &str| observed.said(marker);
        match (exit_code, said(SIGNED_OUT), said(CRASHED)) {
            (0, _, _) => None,
            (_, true, _) => Some(refusal_reason(
                "not_logged_in",
                None,
                Some(
                    "Qoder CLI is signed out: run `qodercli login`, or set \
                     QODER_PERSONAL_ACCESS_TOKEN under a private QODER_CONFIG_DIR",
                ),
            )),
            (code, false, true) => Some(refusal_reason(
                "launch_refused",
                None,
                Some(&format!("Qoder exited {code} on an option it refuses")),
            )),
            (_, false, false) => None,
        }
    }
}

/// The refusal of a seat Qoder served another model than its pin: the one
/// init named, already masked with its row, or, where init named none
/// other, the fallback Qoder reported on stderr, in words of the driver's
/// own (U1).
fn substituted(pin: &str, served: Option<&str>) -> PreSessionRefusal {
    let words = match served {
        Some(served) => format!("the seat pinned {pin:?} and Qoder served {served:?}"),
        None => format!(
            "the seat pinned {pin:?} and Qoder reported on stderr that it fell back to another \
             model"
        ),
    };
    refusal_reason("model_substituted", None, Some(&words))
}

/// The refusal of a seat no init row confirmed the model of.
fn unconfirmed(pin: &str) -> PreSessionRefusal {
    refusal_reason(
        "model_unconfirmed",
        None,
        Some(&format!(
            "the seat pinned {pin:?} and no init row named the model Qoder served"
        )),
    )
}

/// Qoder's stderr as its checks read it: the raw text, lossy UTF-8, and
/// the same text [`without_ansi`], neither masked. A marker found in either
/// is said (V3): a false refusal costs a refused seat, a missed one accepts
/// a model nobody pinned. Both stay here and are only matched (U2): neither
/// is returned, emitted, logged or put in any error, so it has no `Debug`.
pub(super) struct Observed {
    raw: String,
    stripped: String,
}

impl Observed {
    pub(super) fn of(bytes: &[u8]) -> Observed {
        let raw = String::from_utf8_lossy(bytes).into_owned();
        let stripped = without_ansi(&raw);
        Observed { raw, stripped }
    }

    pub(super) fn said(&self, marker: &str) -> bool {
        self.raw.contains(marker) || self.stripped.contains(marker)
    }
}

/// Run one qoder turn: the prompt on stdin, every row through
/// [`QoderFold`], the harness stopped at a refusing row, and the turn's
/// end read for a refusal its stream did not state. `input` names the
/// workdir and the result path, as it does for every driver.
fn invoke(
    launch: &Launch,
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    let path = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or("");
    let (workdir, result_path) = (path("workdir"), path("result_path"));
    let mut transcript = Transcript::resolve(TranscriptKind::None)?;
    let mut fold = QoderFold::new(&launch.pin);
    let fed = spawn_fed(&launch.command, prompt, workdir, bindings)?;
    let mut session_meta = Map::new();
    let mut refusal = None;
    let turn = streamed(fed, |mut event| {
        secret::mask_json(&mut event, bindings);
        let classified = fold.read(event, &mut session_meta, &mut transcript, emit)?;
        refusal = refusal.take().or(classified);
        Ok(())
    })?;
    transcript.finish(&mut session_meta, emit);
    let Turn {
        exit_code,
        stopped,
        captured,
        ..
    } = turn;
    // Read unmasked so no binding erases a marker (U2), and only matched:
    // no refusal, error or late message below carries a byte of it.
    let observed = Observed::of(captured.bytes());
    let identity = match stopped {
        Some(_) => None,
        None => fold.identity(&observed, exit_code),
    };
    // A model refused behind a turn already folded, or a result already
    // written, is no refusal to start, and the answer of a model nobody
    // confirmed is never accepted: the turn ends with no result of its own.
    // Its words pass the seat's bindings in `drive`, once and last (V4).
    let delivered = exit_code == 0 && std::path::Path::new(result_path).exists();
    if let Some(late) = identity.as_ref().filter(|_| fold.worked() || delivered) {
        return Err(late.words().to_string());
    }
    let refusal =
        (stopped.or(identity).or(refusal)).or_else(|| QoderFold::failed(&observed, exit_code));
    Ok(Invocation {
        exit_code,
        session_meta,
        stdout: String::new(),
        // Stripped first, so a value an escape split is whole when the mask
        // reads it (U3). It is evidence, no word of the driver's: the shared
        // driver's stderr tail carries it into the journal as it carries
        // every adapter's, and #1199 closes that path for all of them (V1).
        stderr: captured.evidence(|raw| plain_evidence(raw, bindings)),
        state: None,
        refusal,
        answer: None,
        launch: LaunchTerminal::Cold,
    })
}

#[cfg(test)]
mod tests;
