//! `brokkr driver qoder`: Qoder CLI as a provisional harness (#1178;
//! decision 0075). The seat is never resumed, every model it serves is
//! provisional, and the facts below come from the controller's hand
//! measurements of Qoder CLI 1.1.92 and 1.1.95
//! (`docs/evidence/adapters/qoder-cli-1-1.md`). It is unboxed at level 1,
//! and boxed where the engine's set serves the box's hands (#1235): the
//! one value that decides which, and what a boxed turn must show, are in
//! [`hands`].
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
//! Decision 0075's 2026-11-21 addendum rules the rest: the pin is a plain
//! model id ([`ModelPin`]), every row is decided on as Qoder wrote it and
//! only a masked copy of it reaches the Claude fold, a stdout line no
//! measured stream carries refuses the seat, a provider's refusal is named
//! by its class alone, a fallback notice wins every stop, and every word
//! names only plain ids.
//!
//! Its cost is credits, and every dollar and token figure it reports is 0;
//! the fold already reads a zero as not reported. Qoder's two system
//! subtypes Claude Code does not write, `artifacts_update` and
//! `hook_diagnostics`, are rows the fold passes over as it passes over any
//! row it does not read.

use std::io::BufRead;
use std::path::Path;

use serde_json::{Map, Value};
use thiserror::Error;

use super::claude_fold::ClaudeFold;
use super::mcp::{self, McpRefusal};
use super::{fold_stream_event, io_context, lost, secret, spawn_fed, Invocation, LaunchTerminal};
use crate::native_controls::mcp::qoder::isolation;
use crate::transcript::{Kind as TranscriptKind, Transcript};
use crate::{PreSessionRefusal, RefusalClass};

mod hands;
use hands::{Breach, Hands};

/// What every qoder seat runs before its permission mode, its own pin,
/// effort and grants: a print-mode stream turn that keeps no session.
const HEAD: [&str; 4] = [
    "-p",
    "--output-format",
    "stream-json",
    "--no-session-persistence",
];

/// What the engine's MCP set gives one launch: the file the engine wrote
/// its document to, and the hands that set makes the seat's ([`Hands::of`]).
#[derive(Debug, Clone, Copy)]
struct Engine<'a> {
    file: &'a str,
    hands: &'a Hands,
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
    /// A boxed seat's own tool grant: its launch holds no tool of Qoder's
    /// own, so the grant names a tool that does not exist, and it is
    /// refused rather than dropped (C4). The words name the control, never
    /// its value.
    #[error(
        "refusing to invoke the agent CLI: the seat's arguments carry '--allowed-tools', and a \
         qoder seat whose hands are boxed holds no tool of Qoder's own to grant: its one grant \
         is the engine's workspace tool (#1235)"
    )]
    BoxedGrant,
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

/// The most characters a plain model id holds.
const PLAIN_ID_LIMIT: usize = 80;

/// Whether `text` is a plain model id: 1 to [`PLAIN_ID_LIMIT`] characters,
/// the first an ASCII letter or a digit, so no plain id reads as a flag
/// (H6), and each other an ASCII letter, a digit, `.`, `_` or `-`. No such
/// character is one JSON or Rust escapes, and none is one of the mask's
/// replacement text (`[secret:NAME]`), so a plain id has no escaped form
/// and no mask turns another value into one.
fn plain(text: &str) -> bool {
    let character = |byte: u8| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-');
    let first = text
        .bytes()
        .next()
        .is_some_and(|byte| byte.is_ascii_alphanumeric());
    first && text.len() <= PLAIN_ID_LIMIT && text.bytes().all(character)
}

/// A qoder seat's `-m` value, built once at launch and only from a
/// [`plain`] id (Z1). Every later use, the stream's check and every word
/// that names it, takes this type, and none outside this module (H6).
#[derive(Debug, Clone, PartialEq, Eq)]
struct ModelPin(String);

impl ModelPin {
    fn parse(value: &str) -> Option<ModelPin> {
        plain(value).then(|| ModelPin(value.to_string()))
    }
}

impl std::fmt::Display for ModelPin {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

/// One qoder launch: the command spawned, the id it pins, and the hands
/// its turn is checked by.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Launch {
    command: Vec<String>,
    pin: ModelPin,
    hands: Hands,
}

/// The launch for `extra`, the arguments behind the driver's `--`, served
/// `engine`'s set: the head, the permission mode its hands rule, the seat's
/// own controls in the order it wrote them, the engine's grants as the one
/// `--allowed-tools` value of a boxed seat, the MCP isolation in its one
/// home, and the tool lists last. Nothing else in `extra` is launched: a
/// token this driver does not admit, and a boxed seat's own grant, refuse
/// before any provider work.
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
        if control == CONTROLS[2] && !engine.hands.admits_seat_grant() {
            return Err(LaunchError::BoxedGrant);
        }
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
    let hands = engine.hands;
    let mut command = vec![bin.to_string()];
    command.extend(HEAD.iter().map(|part| part.to_string()));
    command.extend(hands.mode());
    command.extend(extra.iter().cloned());
    command.extend(hands.granted());
    command.extend(isolation(engine.file));
    command.extend(hands.tools());
    Ok(Launch {
        command,
        pin,
        hands: hands.clone(),
    })
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
/// the seat's hands made from that set, once ([`Hands::of`]), and the
/// launch. No launch is offered a rejoin, so the cold assessment is the
/// one admitted.
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
    let hands = Hands::of(served.grants);
    let engine = Engine {
        file: file.to_str().ok_or(LaunchError::Unnamed)?,
        hands: &hands,
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
        .and_then(|dir| {
            std::fs::write(dir.path().join(MCP_FILE), document.to_string()).map(|()| dir)
        });
    written.map_err(LaunchError::Unstaged)
}

/// Qoder's rows read for the model they served, before the Claude fold
/// reads them for everything else. Every rule of Qoder's own is decided on
/// a row as Qoder wrote it, and the Claude fold reads only a copy the
/// seat's bindings masked, so its classifier, its session record and every
/// row it emits read masked text alone, as every other adapter's do (H1).
struct QoderFold {
    pin: ModelPin,
    /// Whether an init row has been read: every measured stream carries
    /// exactly one, and a second refuses the seat (Y1).
    initialised: bool,
    /// Whether init named the pin. An init that names no model confirms
    /// nothing and leaves it unset (Y1).
    confirmed: bool,
    /// The launch's hands, which decide whether each row is read as a
    /// boxed turn's ([`Hands::check`]).
    hands: Hands,
    claude: ClaudeFold,
}

/// Why the fold stopped a stream at a row.
enum Stop {
    /// The row itself is refused: a line that is no JSON object, a second
    /// init, or an init naming another model than the pin.
    Refused(PreSessionRefusal),
    /// A turn arrived that no init confirmed. What it is refused as waits
    /// on the stderr captured by then ([`QoderFold::ended`], Y3).
    Unconfirmed,
    /// A boxed turn's row shows the box does not hold (C5).
    Breached(Breach),
}

impl QoderFold {
    fn new(pin: &ModelPin, hands: &Hands) -> QoderFold {
        QoderFold {
            pin: pin.clone(),
            initialised: false,
            confirmed: false,
            hands: hands.clone(),
            claude: ClaudeFold::default(),
        }
    }

    /// Whether a row has been folded as a turn of the seat's.
    fn worked(&self) -> bool {
        self.claude.turns > 0
    }

    /// One stdout line, as Qoder wrote it. `Err` stops the stream there: a
    /// line that is no JSON object (Z3), a second init row, a first one
    /// naming another model than the pin, or a turn no init confirmed
    /// (Y1). Nothing of a stopping row is folded. Those rules read the row
    /// unmasked; the Claude fold reads only its copy masked by `bindings`,
    /// once the pin is written in, and a refusal it classifies is named by
    /// its class alone ([`provider_refused`], H1 and H2).
    fn read(
        &mut self,
        line: &[u8],
        bindings: &[secret::BoundSecret],
        session_meta: &mut Map<String, Value>,
        transcript: &mut Transcript,
        emit: &mut impl FnMut(&Value),
    ) -> Result<Option<PreSessionRefusal>, Stop> {
        let Ok(mut row) = serde_json::from_slice::<Map<String, Value>>(line) else {
            return Err(Stop::Refused(unmeasured_line()));
        };
        self.confirm(&row)?;
        let assistant = row.get("type").and_then(Value::as_str) == Some("assistant");
        // Nothing persists the session, so its id locates nothing.
        row.remove("session_id");
        // An assistant row names Qoder's internal code; init named the
        // model, and an assistant row reaches here only once it has.
        let message = row.get_mut("message").and_then(Value::as_object_mut);
        if let (true, Some(message)) = (assistant, message) {
            message.insert("model".into(), Value::String(self.pin.to_string()));
        }
        // The one place a row is masked: no row reaches the fold unmasked.
        let mut event = Value::Object(row);
        secret::mask_json(&mut event, bindings);
        let classified =
            fold_stream_event(&event, &mut self.claude, session_meta, transcript, emit);
        Ok(classified.map(provider_refused))
    }

    /// `row` read for the model it confirms, as Qoder wrote it: an init
    /// row names the pin, or nothing; a second one, or a turn before init
    /// named the pin, stops the stream (Y1). Then, for a boxed seat, for
    /// what its hands must show: a row that shows the box does not hold
    /// stops the stream before any of it counts (C5).
    fn confirm(&mut self, row: &Map<String, Value>) -> Result<(), Stop> {
        self.identify(row)?;
        self.hands.check(row).map_err(Stop::Breached)
    }

    /// [`QoderFold::confirm`]'s reading of the model, level 1's.
    fn identify(&mut self, row: &Map<String, Value>) -> Result<(), Stop> {
        let text = |key: &str| row.get(key).and_then(Value::as_str);
        let kind = text("type");
        if kind == Some("system") && text("subtype") == Some("init") {
            if std::mem::replace(&mut self.initialised, true) {
                return Err(Stop::Refused(second_init()));
            }
            match text("model") {
                Some(served) if served == self.pin.0 => self.confirmed = true,
                Some(served) => return Err(Stop::Refused(substituted(&self.pin, Some(served)))),
                // An init that names nothing confirms nothing (Y1).
                None => {}
            }
        } else if !self.confirmed && matches!(kind, Some("assistant" | "user" | "result")) {
            return Err(Stop::Unconfirmed);
        }
        Ok(())
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
            return Some(substituted(&self.pin, None));
        }
        match stop {
            Some(Stop::Refused(refused)) => Some(refused),
            Some(Stop::Unconfirmed) => Some(unconfirmed(&self.pin)),
            Some(Stop::Breached(breach)) => Some(breach.refusal()),
            None => (exit_code == 0 && !self.confirmed).then(|| unconfirmed(&self.pin)),
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

/// The refusal of a seat Qoder served another model than its pin: the one
/// init named, as Qoder wrote it, or, where init named none other, the
/// fallback Qoder reported on stderr, in words of the driver's own (U1).
/// The words name the pin and, only where it is a plain id, the served
/// model, each by `Display` in single quotes and whole (Z4, H2).
fn substituted(pin: &ModelPin, served: Option<&str>) -> PreSessionRefusal {
    let words = match served {
        Some(served) if plain(served) => {
            format!("the seat pinned '{pin}' and Qoder served '{served}'")
        }
        Some(_) => {
            format!("the seat pinned '{pin}' and Qoder served a model that is not a plain id")
        }
        None => format!(
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

/// Qoder's stderr as its checks read it: the raw text, lossy UTF-8, and
/// the same text [`without_ansi`], neither masked. A marker found in either
/// is said (V3): a false refusal costs a refused seat, a missed one accepts
/// a model nobody pinned. Both stay here and are only matched (U2): neither
/// is returned, emitted, logged or put in any error, so it has no `Debug`.
struct Observed {
    raw: String,
    stripped: String,
}

impl Observed {
    fn of(bytes: &[u8]) -> Observed {
        let raw = String::from_utf8_lossy(bytes).into_owned();
        let stripped = without_ansi(&raw);
        Observed { raw, stripped }
    }

    fn said(&self, marker: &str) -> bool {
        self.raw.contains(marker) || self.stripped.contains(marker)
    }

    /// Whether Qoder reported it fell back to another model: the one scan
    /// of the fallback marker, whose answer every identity check reads (Z7).
    fn fell_back(&self) -> bool {
        self.said(FALLBACK)
    }
}

const ESC: char = '\u{1b}';
const BEL: char = '\u{7}';

/// The characters an escape sequence is read from.
type Chars<'a> = std::iter::Peekable<std::str::Chars<'a>>;

/// `text` without its escape sequences, each ended where ECMA-48 ends it
/// (V2): a CSI (`ESC [`) at its final byte, an OSC (`ESC ]`) at BEL or ST
/// (`ESC \`), and any other `ESC` followed by a byte 0x40 to 0x5F as a
/// two-byte sequence. An `ESC` before any other byte goes alone, and that
/// byte is kept. Qoder colours its refusals. This is the one stripper of
/// the checks and of the evidence.
fn without_ansi(text: &str) -> String {
    let mut kept = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != ESC {
            kept.push(c);
            continue;
        }
        match chars.next_if(|next| ('\u{40}'..='\u{5f}').contains(next)) {
            Some('[') => control_sequence(&mut chars),
            Some(']') => operating_system_command(&mut chars),
            Some(_) | None => {}
        }
    }
    kept
}

/// A CSI after its `ESC [`: parameter bytes 0x30 to 0x3F, intermediate
/// bytes 0x20 to 0x2F, then one final byte 0x40 to 0x7E, consumed. Any
/// other byte ends a malformed sequence and is not consumed.
fn control_sequence(chars: &mut Chars<'_>) {
    while chars
        .next_if(|c| ('\u{30}'..='\u{3f}').contains(c))
        .is_some()
    {}
    while chars
        .next_if(|c| ('\u{20}'..='\u{2f}').contains(c))
        .is_some()
    {}
    chars.next_if(|c| ('\u{40}'..='\u{7e}').contains(c));
}

/// An OSC after its `ESC ]`: everything up to BEL or ST, consumed with it.
fn operating_system_command(chars: &mut Chars<'_>) {
    while let Some(c) = chars.next() {
        if c == BEL || (c == ESC && chars.next_if_eq(&'\\').is_some()) {
            return;
        }
    }
}

/// The exit a refused seat reads when the driver stopped its harness at
/// the refusing row: the harness never answered, so the stop is the
/// driver's refusal and not a signal the harness died of. The refusal
/// path journals no exit code; `lost::refusal_to_start` reads this one as
/// a code of the command's own, which it is not, so it is named here.
const STOPPED_AT_REFUSAL: i32 = 1;

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
    let mut fold = QoderFold::new(&launch.pin, &launch.hands);
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
        None => io_context(child.wait(), "agent CLI did not conclude")?,
    };
    // The session record holds only what masked rows wrote (H1).
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
        stderr: captured.evidence(|raw| {
            let stripped = without_ansi(&String::from_utf8_lossy(raw));
            secret::mask_text(stripped.as_bytes(), bindings)
        }),
        state: None,
        refusal,
        answer: None,
        launch: LaunchTerminal::Cold,
    })
}

#[cfg(test)]
pub(super) mod tests;
