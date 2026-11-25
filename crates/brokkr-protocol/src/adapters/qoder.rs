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
//!   rows name an internal code (`fast`, `qmodel_latest`), so a stream
//!   confirms its model by exactly one init row naming the pinned id, and
//!   a seat whose stream does not is refused before any row of it counts;
//! - its session is never persisted (`--no-session-persistence`), so its
//!   session id locates nothing and is dropped: the transcript is `none`.
//!
//! Decision 0075's 2026-11-21 addendum also rules that an unmeasured
//! stdout line refuses the seat, a provider refusal is named by its
//! class alone, and a fallback notice wins every stop. Decision 0114
//! reads structure through [`Row`] and masks each text projection.
//!
//! The pin is a plain model id ([`ModelPin`]).
//! Its cost is credits, and every dollar and token figure it reports is 0;
//! the fold already reads a zero as not reported. Qoder's two system
//! subtypes Claude Code does not write, `artifacts_update` and
//! `hook_diagnostics`, are rows the fold passes over as it passes over any
//! row it does not read.

use std::io::BufRead;
use std::path::Path;

use serde_json::{Map, Value};
use thiserror::Error;

use super::claude_fold::{ClaudeFold, Source};
use super::escapes::plain_evidence;
use super::mcp::{self, McpRefusal};
use super::row::{Pin, Row};
use super::stream_turn::{plain, Observed, STOPPED_AT_REFUSAL, UNCONCLUDED};
use super::{
    fold_stream_event, io_context, lost, model_token, secret, spawn_fed, Emitted, Invocation,
    LaunchTerminal,
};
use crate::native_controls::mcp::qoder::isolation;
use crate::transcript::{Kind as TranscriptKind, Transcript};
use crate::{PreSessionRefusal, RefusalClass};

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
    /// The pin is not a [`ModelPin`]. The words name no byte of it (Z1).
    #[error(
        "refusing to invoke the agent CLI: the qoder model pin is not a plain model id (an \
         ASCII letter or digit, then ASCII letters, digits, '.', '_' and '-')"
    )]
    Unplain,
    #[error(
        "refusing to invoke the agent CLI: the engine's MCP file for the qoder seat has a path \
         that is not UTF-8, so no argument names it exactly (#1178)"
    )]
    Unnamed,
    /// The engine's MCP file could not be written. Its words are fixed and
    /// the error's kind: the error's own text names the temporary path,
    /// which comes from the environment (Y2). The error stays its source.
    #[error(
        "refusing to invoke the agent CLI: the engine's MCP file for the qoder seat was not \
         written: {}",
        .0.kind()
    )]
    Unstaged(#[source] std::io::Error),
    /// The launch's one MCP admission refused it, in the words every
    /// driver states it with ([`McpRefusal::at_launch`]).
    #[error("{}", .0.at_launch())]
    Mcp(#[from] McpRefusal),
}

/// A qoder seat's `-m` value, built once at launch and only from a
/// [`plain`] id (Z1), compared in [`Row::pin`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ModelPin(String);

impl ModelPin {
    pub(super) fn parse(value: &str) -> Option<ModelPin> {
        plain(value).then(|| ModelPin(value.to_string()))
    }

    pub(super) fn as_str(&self) -> &str {
        &self.0
    }

    fn reported(&self, bindings: &[secret::BoundSecret]) -> Option<String> {
        model_token(&secret::mask_text(self.0.as_bytes(), bindings))
    }
}

impl std::fmt::Display for ModelPin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One qoder launch: the command spawned and the id it pins.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Launch {
    command: Vec<String>,
    pin: ModelPin,
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
            pin = Some(ModelPin::parse(value).ok_or(LaunchError::Unplain)?);
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
/// The launch's errors are rendered here, once, and every word the seat
/// ends with, launch, staging, admission, composition, the resume refusal
/// and the turn's own error alike, and the refusal its invocation carries,
/// passes the seat's bindings once, last (V4, Z4), through the mask for
/// re-escaped text, so a value a rendering escaped is masked too (Y2).
pub(super) fn drive(
    bin: &str,
    extra: &[String],
    offered: bool,
    prompt: &str,
    input: &Value,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Emitted),
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
    match driven() {
        Ok(invocation) => Ok(words_masked(invocation, bindings)),
        Err(error) => Err(secret::mask_projected(&error, bindings)),
    }
}

/// `invocation` with its refusal's words masked by `bindings`, as
/// [`drive`]'s errors are: once, last (Z4).
fn words_masked(mut invocation: Invocation, bindings: &[secret::BoundSecret]) -> Invocation {
    invocation.refusal = invocation.refusal.map(|refusal| {
        let words = secret::mask_projected(refusal.words(), bindings);
        refusal.with_words(words)
    });
    invocation
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

/// Qoder's pin admission, before the shared fold reads each row.
struct QoderFold {
    pin: ModelPin,
    /// Every measured stream carries exactly one init (Y1).
    initialised: bool,
    confirmed: Option<ConfirmedPin>,
    claude: ClaudeFold,
}

struct ConfirmedPin {
    /// A bound pin still confirms the launch and has no reportable label.
    model: Option<String>,
}

/// Why the fold stopped a stream at a row.
enum Stop {
    /// The row itself is refused: a line that is no JSON object, a second
    /// init, or an init naming another model than the pin.
    Refused(PreSessionRefusal),
    /// A turn arrived that no init confirmed. What it is refused as waits
    /// on the stderr captured by then ([`QoderFold::ended`], Y3).
    Unconfirmed,
}

impl QoderFold {
    fn new(pin: &ModelPin) -> QoderFold {
        QoderFold {
            pin: pin.clone(),
            initialised: false,
            confirmed: None,
            claude: ClaudeFold::default(),
        }
    }

    /// Whether a row has been folded as a turn of the seat's.
    fn worked(&self) -> bool {
        self.claude.turns > 0
    }

    /// Decide Qoder's admission before folding the row. A provider
    /// refusal is named by its class alone (H1 and H2).
    fn read(
        &mut self,
        line: &[u8],
        bindings: &[secret::BoundSecret],
        session_meta: &mut Map<String, Value>,
        transcript: &mut Transcript,
        emit: &mut impl FnMut(&Emitted),
    ) -> Result<Option<PreSessionRefusal>, Stop> {
        let Ok(object) = serde_json::from_slice::<Map<String, Value>>(line) else {
            return Err(Stop::Refused(unmeasured_line()));
        };
        let event = Value::Object(object);
        let row = Row::new(&event, bindings);
        self.confirm(&row, bindings, session_meta)?;
        let model = self.confirmed.as_ref().and_then(|pin| pin.model.as_deref());
        let classified = fold_stream_event(
            &row,
            Source::Qoder { model },
            &mut self.claude,
            session_meta,
            transcript,
            emit,
        );
        Ok(classified.map(provider_refused))
    }

    /// A second init, a different model or work before confirmation stops
    /// the stream (Y1). An init that names no model confirms nothing.
    fn confirm(
        &mut self,
        row: &Row<'_>,
        bindings: &[secret::BoundSecret],
        meta: &mut Map<String, Value>,
    ) -> Result<(), Stop> {
        if row.is_init() {
            return self.confirm_init(row, bindings, meta);
        }
        let work = &[("assistant", ()), ("user", ()), ("result", ())];
        if self.confirmed.is_none() && row.kind("/type", work).is_some() {
            return Err(Stop::Unconfirmed);
        }
        Ok(())
    }

    fn confirm_init(
        &mut self,
        row: &Row<'_>,
        bindings: &[secret::BoundSecret],
        meta: &mut Map<String, Value>,
    ) -> Result<(), Stop> {
        if std::mem::replace(&mut self.initialised, true) {
            return Err(Stop::Refused(second_init()));
        }
        match row.pin("/model", &self.pin) {
            Pin::Pinned => {
                let model = self.pin.reported(bindings);
                if let Some(label) = &model {
                    meta.insert("model".into(), Value::String(label.clone()));
                }
                self.confirmed = Some(ConfirmedPin { model });
                Ok(())
            }
            Pin::Other { plain } => {
                let served = if plain {
                    Substitution::Plain(row.text("/model").unwrap_or_default())
                } else {
                    Substitution::Unplain
                };
                Err(Stop::Refused(substituted(&self.pin, served)))
            }
            Pin::Absent => Ok(()),
        }
    }

    /// The refusal the turn ends with, decided once (H5): where Qoder's
    /// fallback line is on the stderr `observed` by then, every stop and
    /// every end refuses as substituted, in the reported-on-stderr words;
    /// otherwise a stop keeps its own words, and a clean exit no init row
    /// confirmed is unconfirmed (Y1, Y3).
    ///
    /// `observed` is Qoder's stderr NOT masked, so no binding can erase a
    /// marker it overlaps (U2). It is only matched here: no byte of it
    /// enters the refusal's words.
    fn ended(
        &self,
        stop: Option<Stop>,
        observed: &Observed,
        exit_code: i32,
    ) -> Option<PreSessionRefusal> {
        if observed.fell_back() {
            return Some(substituted(&self.pin, Substitution::Stderr));
        }
        match stop {
            Some(Stop::Refused(refused)) => Some(refused),
            Some(Stop::Unconfirmed) => Some(unconfirmed(&self.pin)),
            None => (exit_code == 0 && self.confirmed.is_none()).then(|| unconfirmed(&self.pin)),
        }
    }

    /// The refusal a CLI that exited non-zero states on stderr: signed out,
    /// or crashed on an option it refuses. `observed` is read as in
    /// [`QoderFold::ended`], and only matched: the words are fixed.
    fn failed(observed: &Observed, exit_code: i32) -> Option<PreSessionRefusal> {
        let said = |marker: &str| observed.said(marker);
        match (exit_code, said(SIGNED_OUT), said(CRASHED)) {
            (0, _, _) => None,
            (_, true, _) => Some(own(
                "not_logged_in",
                "Qoder CLI is signed out: run `qodercli login`, or set \
                 QODER_PERSONAL_ACCESS_TOKEN under a private QODER_CONFIG_DIR"
                    .to_string(),
            )),
            (code, false, true) => Some(own(
                "launch_refused",
                format!("Qoder exited {code} on an option it refuses"),
            )),
            (_, false, false) => None,
        }
    }
}

enum Substitution {
    Plain(String),
    Unplain,
    Stderr,
}

/// Name the substituted model only when init reported a plain id.
/// Row has already masked that id whole (decision 0114).
fn substituted(pin: &ModelPin, served: Substitution) -> PreSessionRefusal {
    let words = match served {
        Substitution::Plain(served) => {
            format!("the seat pinned '{pin}' and Qoder served '{served}'")
        }
        Substitution::Unplain => {
            format!("the seat pinned '{pin}' and Qoder served a model that is not a plain id")
        }
        Substitution::Stderr => format!(
            "the seat pinned '{pin}' and Qoder reported on stderr that it fell back to another \
             model"
        ),
    };
    own("model_substituted", words)
}

/// The refusal of a stdout line that is not UTF-8 or not a JSON object
/// (Z3). No measured stream carries one, an empty line included.
fn unmeasured_line() -> PreSessionRefusal {
    own(
        "stream_unmeasured",
        "Qoder wrote a stdout line that is not a JSON object, which no measured Qoder stream \
         carries"
            .to_string(),
    )
}

/// The refusal of a stream that sent a second init row (Y1).
fn second_init() -> PreSessionRefusal {
    own(
        "model_unconfirmed",
        "Qoder sent a second init row, which no measured Qoder stream carries".to_string(),
    )
}

/// The refusal of a seat no init row confirmed the model of.
fn unconfirmed(pin: &ModelPin) -> PreSessionRefusal {
    own(
        "model_unconfirmed",
        format!("the seat pinned '{pin}' and no init row named the model Qoder served"),
    )
}

/// One refusal in the driver's own words, `token: words`, built directly:
/// no clipping step reads them, so a plain id they name is named whole
/// (H2). Every one is of the class `other`.
fn own(token: &str, words: String) -> PreSessionRefusal {
    PreSessionRefusal::from_parts(RefusalClass::Other, format!("{token}: {words}"), None, None)
}

/// A refusal the Claude fold classified, named by its class alone in fixed
/// words (H2): none of the provider's text, no excerpt and no other value
/// of the row reaches them. The HTTP status and the retry-after it parsed
/// stay as the refusal's fields (decision 0103).
fn provider_refused(classified: PreSessionRefusal) -> PreSessionRefusal {
    let words = match classified.class() {
        RefusalClass::RateLimit => {
            "rate_limit: Qoder's provider refused the turn at its rate limit"
        }
        RefusalClass::Other => "api_error: Qoder's provider refused the turn",
    };
    let (status, retry_after) = (classified.status(), classified.retry_after_seconds());
    PreSessionRefusal::from_parts(classified.class(), words.to_string(), status, retry_after)
}

impl Observed {
    /// Whether Qoder reported it fell back to another model: the one scan
    /// of the fallback marker, whose answer every identity check reads (Z7).
    fn fell_back(&self) -> bool {
        self.said(FALLBACK)
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
    emit: &mut impl FnMut(&Emitted),
) -> Result<Invocation, String> {
    let path = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or("");
    let (workdir, result_path) = (path("workdir"), path("result_path"));
    let mut transcript = Transcript::resolve(TranscriptKind::None)?;
    let mut fold = QoderFold::new(&launch.pin);
    let (mut child, stderr_thread) = spawn_fed(&launch.command, prompt, workdir, bindings)?;
    let stdout = child.stdout.take().expect("piped");
    let mut session_meta = Map::new();
    let mut refusal = None;
    let mut stopped = None;
    for line in std::io::BufReader::new(stdout).split(b'\n') {
        // Every line is read, as bytes: one that cannot be read is no JSON
        // object, so it stops the stream as one that is not (Z3).
        let line = line.unwrap_or_default();
        match fold.read(&line, bindings, &mut session_meta, &mut transcript, emit) {
            Ok(classified) => refusal = refusal.or(classified),
            Err(stop) => {
                stopped = Some(stop);
                break;
            }
        }
    }
    let status = match stopped {
        Some(_) => super::conclude_harness(&mut child)?,
        None => io_context(child.wait(), UNCONCLUDED)?,
    };
    transcript.finish(&mut session_meta, emit);
    let captured = stderr_thread.take(stopped.is_some());
    let exit_code = match stopped {
        Some(_) => STOPPED_AT_REFUSAL,
        None => lost::exit_code(status),
    };
    // Read unmasked so no binding erases a marker (U2), and only matched:
    // no refusal, error or late message below carries a byte of it.
    let observed = Observed::of(captured.bytes());
    let ended = fold.ended(stopped, &observed, exit_code);
    // A seat refused behind a turn already folded, or a result already
    // written, is no refusal to start, whether the stream stopped at a row
    // or the model was refused at its end (Z5), and the answer of a model
    // nobody confirmed is never accepted: the turn ends lost, with no
    // result of its own, its words the error `lost::invocation_failed`
    // prints (H4, decision 0006). They pass the seat's bindings in `drive`,
    // once and last (V4). Delivery is read from the child's real status,
    // never the stop's reported code: it exited 0 on its own and its result
    // file exists (H3).
    let delivered = status.success() && Path::new(result_path).exists();
    if let Some(late) = ended.as_ref().filter(|_| fold.worked() || delivered) {
        return Err(late.words().to_string());
    }
    let refusal = (ended.or(refusal)).or_else(|| QoderFold::failed(&observed, exit_code));
    Ok(Invocation {
        exit_code,
        session_meta,
        notes: String::new(),
        // Stripped first, so a value an escape split is whole when the mask
        // reads it (U3). It is evidence, no word of the driver's: the shared
        // driver tells it on its own stderr as it tells every adapter's, and
        // the journal names it by evidence alone (#1199).
        stderr: captured.evidence(|raw| plain_evidence(raw, bindings)),
        state: None,
        refusal,
        answer: None,
        launch: LaunchTerminal::Cold,
        result_exists: None,
    })
}

#[cfg(test)]
pub(super) mod tests;
