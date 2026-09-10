//! The one local transcript derivation every surface consumes (#222,
//! proposed decision 0055).
//!
//! This module is pure: it selects and validates a supplied recorded
//! reference, admits a bounded supplied byte snapshot, classifies and
//! projects Claude, Codex and DSH retained content, associates proved
//! duplicates, applies the shared caps and constructs the informational
//! full-session hint and the ordered notices. It reads no environment
//! and touches no filesystem — decision 0013's separation is a compile
//! property, not a convention.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::Transcript;

/// The public document version the command serializes. Structural
/// changes to this unpublished document require a new identifier.
pub const TRANSCRIPT_SCHEMA: &str = "brokkr.transcript/v1";

/// Source bytes read from a selected file, plus one overflow probe byte.
pub const SOURCE_CAP: u64 = 33_554_432;

/// The existing Claude display budget, summed over emitted block texts.
pub const DISPLAY_CAP: usize = 4_000_000;

/// The bounded first-record budget for a DSH candidate header.
pub const DSH_HEADER_CAP: usize = 65_536;

/// Every examined entry in one lookup counts against this bound.
pub const DISCOVERY_LIMIT: usize = 10_000;

/// The closed transcript-kind vocabulary.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TranscriptKind {
    ClaudeSession,
    CodexThread,
    DshSession,
    None,
}

impl TranscriptKind {
    /// The kind a recorded `kind` string names, or `None` when the
    /// vocabulary does not include it. A present common reference with
    /// an unlisted kind is `unsupported-kind`, never a silent absence.
    pub fn parse(word: &str) -> Option<TranscriptKind> {
        match word {
            "claude-session" => Some(TranscriptKind::ClaudeSession),
            "codex-thread" => Some(TranscriptKind::CodexThread),
            "dsh-session" => Some(TranscriptKind::DshSession),
            "none" => Some(TranscriptKind::None),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            TranscriptKind::ClaudeSession => "claude-session",
            TranscriptKind::CodexThread => "codex-thread",
            TranscriptKind::DshSession => "dsh-session",
            TranscriptKind::None => "none",
        }
    }
}

/// The five closed block kinds. They serialize as the wire strings the
/// shipped Claude response already used, plus the three this change
/// introduces for the other two providers.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BlockKind {
    Text,
    Reasoning,
    Tool,
    ToolResult,
    Omitted,
}

impl BlockKind {
    pub fn as_str(self) -> &'static str {
        match self {
            BlockKind::Text => "text",
            BlockKind::Reasoning => "reasoning",
            BlockKind::Tool => "tool",
            BlockKind::ToolResult => "tool-result",
            BlockKind::Omitted => "omitted",
        }
    }
}

/// One block of a turn: prose, or an inert tool marker.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Block {
    pub kind: BlockKind,
    pub text: String,
}

impl Block {
    pub fn text(text: impl Into<String>) -> Block {
        Block {
            kind: BlockKind::Text,
            text: text.into(),
        }
    }

    pub fn reasoning(text: impl Into<String>) -> Block {
        Block {
            kind: BlockKind::Reasoning,
            text: text.into(),
        }
    }

    pub fn tool(text: impl Into<String>) -> Block {
        Block {
            kind: BlockKind::Tool,
            text: text.into(),
        }
    }

    pub fn tool_result(text: impl Into<String>) -> Block {
        Block {
            kind: BlockKind::ToolResult,
            text: text.into(),
        }
    }

    pub fn omitted(text: impl Into<String>) -> Block {
        Block {
            kind: BlockKind::Omitted,
            text: text.into(),
        }
    }
}

/// One transcript turn as every surface shows it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct Turn {
    pub role: String,
    pub ts: String,
    pub blocks: Vec<Block>,
}

/// The closed unavailability vocabulary the command delta fixes.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Unavailable {
    NoReference,
    None,
    UnsupportedKind,
    Unannounced,
    MissingHome,
    InvalidReference,
    UnsafePath,
    NotFound,
    AmbiguousSource,
    DiscoveryLimit,
    Unreadable,
    UnsupportedFormat,
    TurnNotRetained,
}

impl Unavailable {
    pub fn as_str(self) -> &'static str {
        match self {
            Unavailable::NoReference => "no-reference",
            Unavailable::None => "none",
            Unavailable::UnsupportedKind => "unsupported-kind",
            Unavailable::Unannounced => "unannounced",
            Unavailable::MissingHome => "missing-home",
            Unavailable::InvalidReference => "invalid-reference",
            Unavailable::UnsafePath => "unsafe-path",
            Unavailable::NotFound => "not-found",
            Unavailable::AmbiguousSource => "ambiguous-source",
            Unavailable::DiscoveryLimit => "discovery-limit",
            Unavailable::Unreadable => "unreadable",
            Unavailable::UnsupportedFormat => "unsupported-format",
            Unavailable::TurnNotRetained => "turn-not-retained",
        }
    }
}

/// A common reference that passed every validation gate: a supported
/// kind, a nonempty validated locator and an absolute validated home.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ValidReference {
    pub kind: TranscriptKind,
    pub locator: String,
    pub home: String,
}

/// How a legacy `session_id` may synthesize a Claude reference.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LegacyProvenance {
    Claude,
    LaneTally,
    Absent,
    /// Any other explicit provider provenance refuses synthesis.
    Other,
}

/// The outcome of selecting and validating the recorded reference.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Selection {
    /// The selected reference echoed unchanged (its recorded strings),
    /// or the synthesized legacy Claude reference, or null when there is
    /// no reference at all.
    pub reference: Option<Transcript>,
    /// True only for a synthesized legacy Claude reference.
    pub legacy: bool,
    /// The validated reference, or the refusal the selection owns.
    pub outcome: Result<ValidReference, Unavailable>,
}

impl Selection {
    pub fn kind(&self) -> Option<TranscriptKind> {
        self.outcome.as_ref().ok().map(|valid| valid.kind)
    }
}

/// True when every character is legal in a path input: no ASCII control
/// character and no NUL.
fn clean_path(text: &str) -> bool {
    !text.chars().any(|c| c.is_control() || c == '\0')
}

/// Claude: 1-64 ASCII hexadecimal-or-hyphen characters, first a
/// hexadecimal character. Leading hyphens are now invalid everywhere.
pub fn valid_claude_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) if first.is_ascii_hexdigit() => {}
        _ => return false,
    }
    id.len() <= 64 && chars.all(|c| c.is_ascii_hexdigit() || c == '-')
}

/// Codex: 1-128 ASCII alphanumeric-or-dash characters, first an ASCII
/// alphanumeric. This is the engine's own resume/echo language.
pub fn valid_codex_id(id: &str) -> bool {
    let mut chars = id.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    id.len() <= 128 && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// DSH: a relative forward-slashed locator whose every component is
/// nonempty, and never `.`, `..`, a drive prefix or absolute.
pub fn valid_dsh_locator(locator: &str) -> bool {
    if locator.is_empty() || locator.starts_with('/') || locator.contains('\\') {
        return false;
    }
    if locator.contains(':') {
        return false;
    }
    locator.split('/').all(|component| {
        !component.is_empty()
            && component != "."
            && component != ".."
            && !component.chars().any(|c| c.is_control() || c == '\0')
    })
}

/// A home is valid when it is absolute on either supported platform,
/// carries no control character or NUL, and is not empty. Unix roots
/// begin with `/`; Windows homes are drive-absolute (`C:\` or `C:/`) or
/// UNC (`\\server\share`). Validating both spellings keeps a recorded
/// Windows home readable by the Windows-built reader without widening
/// Unix behavior into relative paths.
fn valid_home(home: &str) -> bool {
    if home.is_empty() || !clean_path(home) {
        return false;
    }
    if home.starts_with('/') {
        return true;
    }
    let bytes = home.as_bytes();
    if bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && (bytes[2] == b'\\' || bytes[2] == b'/')
    {
        return true;
    }
    (home.starts_with("\\\\") || home.starts_with("//")) && home.len() > 2
}

/// Validate one present common reference under the reading delta's
/// precedence: `none`, unsupported kind, empty locator, missing home,
/// then a malformed locator or home.
pub fn validate_common(reference: &Transcript) -> Result<ValidReference, Unavailable> {
    let kind = match TranscriptKind::parse(&reference.kind) {
        Some(TranscriptKind::None) => return Err(Unavailable::None),
        Some(kind) => kind,
        None => return Err(Unavailable::UnsupportedKind),
    };
    if reference.locator.is_empty() {
        return Err(Unavailable::Unannounced);
    }
    if reference.home.is_empty() {
        return Err(Unavailable::MissingHome);
    }
    if !valid_home(&reference.home) || !clean_path(&reference.locator) {
        return Err(Unavailable::InvalidReference);
    }
    let locator_ok = match kind {
        TranscriptKind::ClaudeSession => valid_claude_id(&reference.locator),
        TranscriptKind::CodexThread => valid_codex_id(&reference.locator),
        TranscriptKind::DshSession => valid_dsh_locator(&reference.locator),
        TranscriptKind::None => unreachable!("none returned above"),
    };
    if !locator_ok {
        return Err(Unavailable::InvalidReference);
    }
    Ok(ValidReference {
        kind,
        locator: reference.locator.clone(),
        home: reference.home.clone(),
    })
}

/// Select the effective reference before validating it. A present
/// common reference always wins, even when refused, and suppresses every
/// legacy fallback. Only an absent common reference with eligible
/// Claude/LaneTally/absent legacy provenance and a valid nonempty id may
/// synthesize a Claude reference from the supplied local projects root.
pub fn select_reference(
    common: Option<&Transcript>,
    provenance: LegacyProvenance,
    legacy_id: Option<&str>,
    local_projects: Option<&str>,
) -> Selection {
    if let Some(common) = common {
        let outcome = validate_common(common);
        return Selection {
            reference: Some(common.clone()),
            legacy: false,
            outcome,
        };
    }
    let eligible = matches!(
        provenance,
        LegacyProvenance::Claude | LegacyProvenance::LaneTally | LegacyProvenance::Absent
    );
    if !eligible {
        return Selection {
            reference: None,
            legacy: false,
            outcome: Err(Unavailable::NoReference),
        };
    }
    let Some(id) = legacy_id.filter(|id| !id.is_empty()) else {
        return Selection {
            reference: None,
            legacy: false,
            outcome: Err(Unavailable::NoReference),
        };
    };
    if !valid_claude_id(id) {
        return Selection {
            reference: None,
            legacy: false,
            outcome: Err(Unavailable::InvalidReference),
        };
    }
    let Some(home) = local_projects.filter(|home| valid_home(home)) else {
        // An eligible id with no usable local projects root cannot be
        // synthesized; that is an absent reference, not a refusal of a
        // recorded one.
        return Selection {
            reference: None,
            legacy: false,
            outcome: Err(Unavailable::NoReference),
        };
    };
    let reference = Transcript {
        kind: TranscriptKind::ClaudeSession.as_str().to_string(),
        locator: id.to_string(),
        home: home.to_string(),
    };
    Selection {
        outcome: Ok(ValidReference {
            kind: TranscriptKind::ClaudeSession,
            locator: id.to_string(),
            home: home.to_string(),
        }),
        reference: Some(reference),
        legacy: true,
    }
}

/// A JSON double-quoted string literal, exactly: quotation marks and
/// reverse solidus escaped, ASCII controls as the usual short escapes
/// where available and lowercase `\u00xx` otherwise, other Unicode
/// literal. No shell escaping is added.
pub fn json_string(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string())
}

/// The informational `full_session` value for one validated reference
/// and its confirmed path, or `None` where the table fixes null.
pub fn full_session(reference: &ValidReference, path: Option<&str>) -> Option<String> {
    match reference.kind {
        TranscriptKind::ClaudeSession => Some(format!(
            "full session: claude --resume {}",
            reference.locator
        )),
        TranscriptKind::CodexThread => Some(match path {
            Some(path) => format!(
                "full session: {}; codex exec resume {}; home: {}",
                json_string(path),
                reference.locator,
                json_string(&reference.home)
            ),
            None => format!(
                "full session: rollout unavailable; codex exec resume {}; home: {}",
                reference.locator,
                json_string(&reference.home)
            ),
        }),
        TranscriptKind::DshSession => {
            path.map(|path| format!("full session: {}", json_string(path)))
        }
        TranscriptKind::None => None,
    }
}

/// The three ordered notices, omitting each whose condition is false.
pub fn notices(truncated: bool, skipped_lines: u64, unrecognized_records: u64) -> Vec<String> {
    let mut out = Vec::new();
    if truncated {
        out.push("transcript truncated (size cap)".to_string());
    }
    if skipped_lines > 0 {
        out.push(format!(
            "malformed transcript lines skipped: {skipped_lines}"
        ));
    }
    if unrecognized_records > 0 {
        out.push(format!(
            "unrecognized transcript records: {unrecognized_records}"
        ));
    }
    out
}

/// One complete bounded local read result: the shape the command, the
/// TUI and the browser presentation all consume.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct TranscriptRead {
    pub reference: Option<Transcript>,
    pub legacy: bool,
    pub kind: Option<TranscriptKind>,
    pub path: Option<String>,
    pub turns: Vec<Turn>,
    pub truncated: bool,
    pub skipped_lines: u64,
    pub unrecognized_records: u64,
    pub notices: Vec<String>,
    pub unavailable: Option<Unavailable>,
    pub explanation: Option<String>,
    pub full_session: Option<String>,
}

impl TranscriptRead {
    pub fn is_readable(&self) -> bool {
        self.unavailable.is_none()
    }

    /// A readable projection, possibly zero-turn.
    #[allow(clippy::too_many_arguments)]
    pub fn readable(
        reference: Option<Transcript>,
        legacy: bool,
        kind: TranscriptKind,
        path: Option<String>,
        turns: Vec<Turn>,
        truncated: bool,
        skipped_lines: u64,
        unrecognized_records: u64,
    ) -> TranscriptRead {
        let hint = ValidReference {
            kind,
            locator: reference
                .as_ref()
                .map(|r| r.locator.clone())
                .unwrap_or_default(),
            home: reference
                .as_ref()
                .map(|r| r.home.clone())
                .unwrap_or_default(),
        };
        let full = full_session(&hint, path.as_deref());
        TranscriptRead {
            reference,
            legacy,
            kind: Some(kind),
            path,
            turns,
            truncated,
            skipped_lines,
            unrecognized_records,
            notices: notices(truncated, skipped_lines, unrecognized_records),
            unavailable: None,
            explanation: None,
            full_session: full,
        }
    }

    /// A refused result. `path`, counts, truncation and the hint are
    /// supplied only where the failure-stage matrix establishes them.
    #[allow(clippy::too_many_arguments)]
    pub fn refused(
        reference: Option<Transcript>,
        legacy: bool,
        reason: Unavailable,
        explanation: impl Into<String>,
        path: Option<String>,
        truncated: bool,
        skipped_lines: u64,
        unrecognized_records: u64,
        full_session: Option<String>,
    ) -> TranscriptRead {
        TranscriptRead {
            reference,
            legacy,
            kind: None,
            path,
            turns: Vec::new(),
            truncated,
            skipped_lines,
            unrecognized_records,
            notices: notices(truncated, skipped_lines, unrecognized_records),
            unavailable: Some(reason),
            explanation: Some(explanation.into()),
            full_session,
        }
    }

    /// The standard refusal owned by reference selection and validation.
    pub fn from_selection(selection: &Selection) -> TranscriptRead {
        match &selection.outcome {
            Ok(valid) => {
                let hint = full_session(valid, None);
                TranscriptRead::refused(
                    selection.reference.clone(),
                    selection.legacy,
                    Unavailable::NotFound,
                    "no retained transcript file was found for the selected reference",
                    None,
                    false,
                    0,
                    0,
                    hint,
                )
            }
            Err(reason) => TranscriptRead::refused(
                selection.reference.clone(),
                selection.legacy,
                *reason,
                explanation_for(*reason),
                None,
                false,
                0,
                0,
                None,
            ),
        }
    }
}

/// The sanitized explanation that accompanies each closed reason.
pub fn explanation_for(reason: Unavailable) -> String {
    match reason {
        Unavailable::NoReference => "no transcript reference was recorded".to_string(),
        Unavailable::None => "the recorded reference is explicitly none".to_string(),
        Unavailable::UnsupportedKind => "the recorded transcript kind is not supported".to_string(),
        Unavailable::Unannounced => "the recorded reference has no locator".to_string(),
        Unavailable::MissingHome => "the recorded reference has no home".to_string(),
        Unavailable::InvalidReference => {
            "the recorded reference is not valid for its kind".to_string()
        }
        Unavailable::UnsafePath => {
            "no safe unique transcript source exists below the recorded home".to_string()
        }
        Unavailable::NotFound => "no retained transcript file was found".to_string(),
        Unavailable::AmbiguousSource => {
            "more than one safe transcript source matches the reference".to_string()
        }
        Unavailable::DiscoveryLimit => "the transcript lookup exceeded its entry bound".to_string(),
        Unavailable::Unreadable => "the transcript source could not be read".to_string(),
        Unavailable::UnsupportedFormat => "DSH transcript format is not supported".to_string(),
        Unavailable::TurnNotRetained => {
            "the requested turn is outside the retained projection".to_string()
        }
    }
}

// ------------------------------------------------------------ snapshot

/// A bounded byte snapshot handed to the pure reader: at most
/// [`SOURCE_CAP`] bytes plus the one-byte overflow probe fact.
pub struct Snapshot<'a> {
    pub bytes: &'a [u8],
    /// The probe byte was present: the source is larger than the cap.
    pub overflow: bool,
    /// The read reached true end of file.
    pub eof: bool,
}

/// The admitted snapshot: validated UTF-8 and the rows that participate.
pub struct Admitted<'a> {
    pub text: &'a str,
    /// Only newline-terminated rows participate when the source is over
    /// the cap.
    pub overflow: bool,
}

/// Admit a supplied snapshot: validate consumed UTF-8 before any
/// semantic work. A code point cut solely by the source cap is a
/// boundary fragment; any other invalid byte is `unreadable`.
pub fn admit<'a>(snapshot: &Snapshot<'a>) -> Result<Admitted<'a>, Unavailable> {
    match std::str::from_utf8(snapshot.bytes) {
        Ok(text) => Ok(Admitted {
            text,
            overflow: snapshot.overflow,
        }),
        Err(error) => {
            let valid = error.valid_up_to();
            // `error_len() == None` means an incomplete multibyte
            // sequence at the very end of the supplied bytes: exactly a
            // source-cap boundary fragment.
            let boundary_fragment = error.error_len().is_none();
            if snapshot.overflow && boundary_fragment {
                // Safe: the prefix validated above.
                let text = std::str::from_utf8(&snapshot.bytes[..valid]).unwrap();
                Ok(Admitted {
                    text,
                    overflow: true,
                })
            } else {
                Err(Unavailable::Unreadable)
            }
        }
    }
}

/// One complete physical row of an admitted snapshot.
pub struct Row {
    pub text: String,
    /// True when the row was terminated by a newline. A final
    /// newline-less row is only complete when it parses.
    pub newline_terminated: bool,
    /// True when this is the final newline-less row of a true-EOF
    /// snapshot (candidate provisional content).
    pub final_fragment: bool,
}

/// The complete physical rows that participate in classification.
pub fn rows(admitted: &Admitted<'_>) -> Vec<Row> {
    let text = admitted.text;
    let mut out = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for (index, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            out.push(Row {
                text: text[start..index].to_string(),
                newline_terminated: true,
                final_fragment: false,
            });
            start = index + 1;
        }
    }
    if start < text.len() {
        // A newline-less tail: above the cap it never participates; at
        // true EOF it is provisional until it parses.
        if !admitted.overflow {
            out.push(Row {
                text: text[start..].to_string(),
                newline_terminated: false,
                final_fragment: true,
            });
        }
    }
    out
}

/// One bounded, classified local snapshot before any turn selection.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Projection {
    pub turns: Vec<Turn>,
    pub skipped_lines: u64,
    pub unrecognized_records: u64,
    pub truncated: bool,
    pub unavailable: Option<Unavailable>,
}

impl Projection {
    /// A source-failure projection keeps zero counts but preserves any
    /// truncation the bounded read independently established.
    fn source_failure(truncated: bool) -> Projection {
        Projection {
            truncated,
            unavailable: Some(Unavailable::Unreadable),
            ..Projection::default()
        }
    }
}

/// Parse the complete physical rows of an admitted snapshot one at a
/// time, handing each decoded value to `visit` and releasing it before
/// the next row is parsed. Neither the row text nor its decoded JSON is
/// retained for the whole file (design D4). A newline-less final fragment
/// that does not parse is provisional and uncounted; any other complete
/// row that does not parse is malformed and counted. Returns the
/// malformed count.
fn for_each_parsed_row<F>(admitted: &Admitted<'_>, mut visit: F) -> u64
where
    F: FnMut(usize, Value),
{
    let text = admitted.text;
    let bytes = text.as_bytes();
    let mut skipped = 0u64;
    let mut start = 0usize;
    let mut index = 0usize;
    for (end, byte) in bytes.iter().enumerate() {
        if *byte != b'\n' {
            continue;
        }
        match serde_json::from_str::<Value>(&text[start..end]) {
            Ok(value) => visit(index, value),
            Err(_) => skipped += 1,
        }
        index += 1;
        start = end + 1;
    }
    if start < text.len() && !admitted.overflow {
        // A newline-less tail at true EOF is provisional: it participates
        // only when it parses, and an unparseable tail is never counted.
        if let Ok(value) = serde_json::from_str::<Value>(&text[start..]) {
            visit(index, value);
        }
    }
    skipped
}

/// Apply the 4,000,000-byte sum-of-block-texts budget after association
/// and classification: retain whole turns in source order, stop before
/// the first turn that would exceed it, never skip forward.
fn display_cap(turns: Vec<Turn>, truncated: &mut bool) -> Vec<Turn> {
    let mut out = Vec::new();
    let mut budget = DISPLAY_CAP;
    for turn in turns {
        let cost: usize = turn.blocks.iter().map(|block| block.text.len()).sum();
        if cost > budget {
            *truncated = true;
            break;
        }
        budget -= cost;
        out.push(turn);
    }
    out
}

/// Project one admitted snapshot of the given kind. A source I/O or
/// UTF-8 failure is `unreadable`; a DSH semantic refusal is
/// `unsupported-format` with whole-prefix counts.
pub fn project(kind: TranscriptKind, snapshot: &Snapshot<'_>) -> Projection {
    let admitted = match admit(snapshot) {
        Ok(admitted) => admitted,
        // A UTF-8 refusal keeps only the overflow fact the bounded read
        // already established, with zero counts.
        Err(_) => return Projection::source_failure(snapshot.overflow),
    };
    let mut projection = Projection {
        truncated: admitted.overflow,
        ..Projection::default()
    };
    match kind {
        TranscriptKind::ClaudeSession => project_claude(&admitted, &mut projection),
        TranscriptKind::CodexThread => project_codex(&admitted, &mut projection),
        TranscriptKind::DshSession => project_dsh(&admitted, &mut projection),
        TranscriptKind::None => {}
    }
    projection
}

// -------------------------------------------------------------- Claude

fn project_claude(admitted: &Admitted<'_>, projection: &mut Projection) {
    let mut turns = Vec::new();
    projection.skipped_lines = for_each_parsed_row(admitted, |_, value| {
        let (blocks, role, ts, unrecognized) = claude_row(&value);
        if unrecognized {
            projection.unrecognized_records += 1;
        }
        if !blocks.is_empty() {
            turns.push(Turn { role, ts, blocks });
        }
    });
    projection.turns = display_cap(turns, &mut projection.truncated);
}

/// Claude's closed classification table (proposed 0055 ruling 3).
fn claude_row(value: &Value) -> (Vec<Block>, String, String, bool) {
    let Some(object) = value.as_object() else {
        return (Vec::new(), String::new(), String::new(), true);
    };
    let Some(kind) = object.get("type").and_then(Value::as_str) else {
        return (Vec::new(), String::new(), String::new(), true);
    };
    if matches!(
        kind,
        "summary" | "system" | "progress" | "file-history-snapshot" | "queue-operation"
    ) {
        return (Vec::new(), String::new(), String::new(), false);
    }
    if kind != "user" && kind != "assistant" {
        return (Vec::new(), String::new(), String::new(), true);
    }
    let ts = object
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let message = object.get("message");
    let role = message
        .and_then(|message| message.get("role"))
        .and_then(Value::as_str)
        .unwrap_or(kind)
        .to_string();
    let mut blocks = Vec::new();
    let mut unrecognized = false;
    match message {
        None | Some(Value::Null) => {}
        Some(Value::Object(message)) => match message.get("content") {
            None | Some(Value::Null) => {}
            Some(Value::String(text)) => blocks.push(Block::text(text.clone())),
            Some(Value::Array(parts)) => {
                for part in parts {
                    match part {
                        Value::Object(block) => match block.get("type").and_then(Value::as_str) {
                            Some("text") => match block.get("text") {
                                Some(Value::String(text)) => {
                                    if !text.trim().is_empty() {
                                        blocks.push(Block::text(text.clone()));
                                    }
                                }
                                None | Some(Value::Null) => {}
                                Some(_) => unrecognized = true,
                            },
                            Some("tool_use") => {
                                let name = block.get("name").and_then(Value::as_str).unwrap_or("?");
                                let target = block
                                    .get("input")
                                    .and_then(|input| input.get("file_path"))
                                    .and_then(Value::as_str)
                                    .unwrap_or("");
                                let text = if target.is_empty() {
                                    name.to_string()
                                } else {
                                    format!("{name} · {target}")
                                };
                                blocks.push(Block::tool(text));
                            }
                            Some(
                                "thinking" | "redacted_thinking" | "tool_result" | "image"
                                | "document",
                            ) => {}
                            _ => unrecognized = true,
                        },
                        _ => unrecognized = true,
                    }
                }
            }
            Some(_) => unrecognized = true,
        },
        Some(_) => unrecognized = true,
    }
    (blocks, role, ts, unrecognized)
}

// --------------------------------------------------------------- Codex

/// Top-level Codex rollout record types that are recognized quiet
/// metadata (design D5's enumeration).
fn codex_quiet_top(kind: &str) -> bool {
    matches!(
        kind,
        "session_meta"
            | "turn_context"
            | "compacted"
            | "token_usage_record"
            | "world_state"
            | "security_risk_score"
    )
}

/// Recognized event envelopes that carry no projected prose.
fn codex_quiet_event(kind: &str) -> bool {
    matches!(
        kind,
        "token_count"
            | "thread_goal_updated"
            | "thread_rolled_back"
            | "turn_aborted"
            | "task_started"
            | "turn_started"
            | "task_complete"
            | "turn_complete"
            | "thread_settings_applied"
            | "session_configured"
            | "context_compacted"
            | "item_started"
            | "agent_reasoning_raw_content"
            | "agent_reasoning_section_break"
            | "reasoning_raw_content_delta"
    )
}

/// A mirrored Codex fact family. Identity alone is not evidence: a call
/// and its output are two facts, and message, reasoning and tool facts
/// never associate across families.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum CodexFact {
    Message,
    Reasoning,
    Call,
    Result,
}

/// One projected Codex block and the optional association fact it shares
/// with a canonical `response_item` counterpart.
struct CodexBlock {
    block: Block,
    fact: Option<(CodexFact, String)>,
}

impl CodexBlock {
    fn plain(block: Block) -> CodexBlock {
        CodexBlock { block, fact: None }
    }

    fn identified(block: Block, fact: CodexFact, id: Option<&str>) -> CodexBlock {
        CodexBlock {
            block,
            fact: id
                .filter(|id| !id.is_empty())
                .map(|id| (fact, id.to_string())),
        }
    }
}

/// One classified Codex record before the association pass.
struct CodexRecord {
    blocks: Vec<CodexBlock>,
    role: String,
    ts: String,
    unrecognized: bool,
    canonical: bool,
}

/// A Codex media part stays visible as an omission that names its class,
/// never the bytes.
fn media_omission(kind: &str) -> &'static str {
    if kind.ends_with("image") {
        "[image omitted]"
    } else {
        "[audio omitted]"
    }
}

/// A tool payload rendered as deterministic JSON when it is not a
/// string, or its recorded string when it is.
fn payload_text(value: Option<&Value>) -> Option<String> {
    match value {
        None | Some(Value::Null) => None,
        Some(Value::String(text)) => Some(text.clone()),
        Some(value) => Some(value.to_string()),
    }
}

fn project_codex(admitted: &Admitted<'_>, projection: &mut Projection) {
    let mut records: Vec<CodexRecord> = Vec::new();
    projection.skipped_lines = for_each_parsed_row(admitted, |_, value| {
        records.push(codex_row(&value));
    });
    for record in &records {
        if record.unrecognized {
            projection.unrecognized_records += 1;
        }
    }
    // Association is a separate, explicit pass over the complete bounded
    // prefix: only recorded identity plus direction and compatible family
    // can remove an event fallback, and only a unique canonical
    // counterpart can do so.
    associate_codex(&mut records);
    let mut turns = Vec::new();
    for record in records {
        if record.blocks.is_empty() {
            continue;
        }
        turns.push(Turn {
            role: record.role,
            ts: record.ts,
            blocks: record.blocks.into_iter().map(|block| block.block).collect(),
        });
    }
    projection.turns = display_cap(turns, &mut projection.truncated);
}

/// Remove the blocks an event fallback provably mirrors. A canonical
/// record never deduplicates another canonical record, a colliding
/// identity keeps both records, and only the blocks the canonical record
/// actually covers disappear.
fn associate_codex(records: &mut [CodexRecord]) {
    use std::collections::{HashMap, HashSet};
    let mut canonical: HashMap<(CodexFact, String), usize> = HashMap::new();
    let mut fallback: HashMap<(CodexFact, String), usize> = HashMap::new();
    for record in records.iter() {
        let mut seen: HashSet<(CodexFact, String)> = HashSet::new();
        for block in &record.blocks {
            let Some((fact, id)) = &block.fact else {
                continue;
            };
            let key = (*fact, id.clone());
            if !seen.insert(key.clone()) {
                continue;
            }
            if record.canonical {
                *canonical.entry(key).or_default() += 1;
            } else {
                *fallback.entry(key).or_default() += 1;
            }
        }
    }
    for record in records.iter_mut() {
        if record.canonical {
            continue;
        }
        record.blocks.retain(|block| match &block.fact {
            Some((fact, id)) => {
                let key = (*fact, id.clone());
                canonical.get(&key) != Some(&1) || fallback.get(&key) != Some(&1)
            }
            None => true,
        });
    }
}

/// Project one Codex record into its blocks and recorded metadata.
fn codex_row(value: &Value) -> CodexRecord {
    let Some(object) = value.as_object() else {
        return CodexRecord {
            blocks: Vec::new(),
            role: String::new(),
            ts: String::new(),
            unrecognized: true,
            canonical: false,
        };
    };
    let ts = object
        .get("timestamp")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let quiet = |ts: String, unrecognized: bool| CodexRecord {
        blocks: Vec::new(),
        role: String::new(),
        ts,
        unrecognized,
        canonical: false,
    };
    let Some(kind) = object.get("type").and_then(Value::as_str) else {
        return quiet(ts, true);
    };
    let Some(payload) = object.get("payload") else {
        return quiet(ts, true);
    };
    match kind {
        "response_item" => {
            let (blocks, role, unrecognized) = codex_response_item(payload);
            CodexRecord {
                blocks,
                role,
                ts,
                unrecognized,
                canonical: true,
            }
        }
        "event_msg" => {
            let (blocks, role, unrecognized) = codex_event_msg(payload);
            CodexRecord {
                blocks,
                role,
                ts,
                unrecognized,
                canonical: false,
            }
        }
        _ if codex_quiet_top(kind) => quiet(ts, false),
        _ => quiet(ts, true),
    }
}

fn codex_response_item(payload: &Value) -> (Vec<CodexBlock>, String, bool) {
    let Some(kind) = payload.get("type").and_then(Value::as_str) else {
        return (Vec::new(), String::new(), true);
    };
    match kind {
        "message" => {
            let role = payload
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("assistant")
                .to_string();
            let id = if role == "assistant" {
                payload.get("id").and_then(Value::as_str)
            } else {
                None
            };
            let mut blocks = Vec::new();
            let mut unrecognized = false;
            match payload.get("content") {
                Some(Value::Array(parts)) => {
                    for part in parts {
                        match part.get("type").and_then(Value::as_str) {
                            Some("input_text" | "output_text") => {
                                if let Some(text) = part.get("text").and_then(Value::as_str) {
                                    blocks.push(CodexBlock::identified(
                                        Block::text(text),
                                        CodexFact::Message,
                                        id,
                                    ));
                                }
                            }
                            Some(kind @ ("input_image" | "input_audio" | "image" | "audio")) => {
                                blocks.push(CodexBlock::identified(
                                    Block::omitted(media_omission(kind)),
                                    CodexFact::Message,
                                    id,
                                ));
                            }
                            _ => unrecognized = true,
                        }
                    }
                }
                None | Some(Value::Null) => {}
                Some(_) => unrecognized = true,
            }
            (blocks, role, unrecognized)
        }
        "reasoning" => {
            let id = payload.get("id").and_then(Value::as_str);
            let mut blocks = Vec::new();
            let mut unrecognized = false;
            match payload.get("summary") {
                Some(Value::Array(parts)) => {
                    for part in parts {
                        match part.get("type").and_then(Value::as_str) {
                            Some("summary_text") => {
                                if let Some(text) = part.get("text").and_then(Value::as_str) {
                                    blocks.push(CodexBlock::identified(
                                        Block::reasoning(text),
                                        CodexFact::Reasoning,
                                        id,
                                    ));
                                }
                            }
                            _ => unrecognized = true,
                        }
                    }
                }
                None | Some(Value::Null) => {}
                Some(_) => unrecognized = true,
            }
            (blocks, "assistant".to_string(), unrecognized)
        }
        "function_call" | "custom_tool_call" => {
            let name = payload.get("name").and_then(Value::as_str).unwrap_or("?");
            let arguments = payload_text(payload.get("arguments").or_else(|| payload.get("input")));
            let text = match arguments {
                Some(arguments) => format!("{name} {arguments}"),
                None => name.to_string(),
            };
            let id = payload.get("call_id").and_then(Value::as_str);
            (
                vec![CodexBlock::identified(
                    Block::tool(text),
                    CodexFact::Call,
                    id,
                )],
                "assistant".to_string(),
                false,
            )
        }
        "function_call_output" | "custom_tool_call_output" => {
            let output = payload_text(payload.get("output")).unwrap_or_default();
            let id = payload.get("call_id").and_then(Value::as_str);
            (
                vec![CodexBlock::identified(
                    Block::tool_result(output),
                    CodexFact::Result,
                    id,
                )],
                "tool".to_string(),
                false,
            )
        }
        "local_shell_call" | "web_search_call" => {
            let action = payload_text(payload.get("action")).unwrap_or_default();
            let id = payload
                .get("call_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str);
            (
                vec![CodexBlock::identified(
                    Block::tool(action),
                    CodexFact::Call,
                    id,
                )],
                "assistant".to_string(),
                false,
            )
        }
        "tool_search_call" => {
            let arguments = payload_text(payload.get("arguments")).unwrap_or_default();
            let id = payload
                .get("call_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str);
            (
                vec![CodexBlock::identified(
                    Block::tool(format!("tool_search {arguments}")),
                    CodexFact::Call,
                    id,
                )],
                "assistant".to_string(),
                false,
            )
        }
        "tool_search_output" => {
            let tools = payload_text(payload.get("tools")).unwrap_or_default();
            let id = payload
                .get("call_id")
                .or_else(|| payload.get("id"))
                .and_then(Value::as_str);
            (
                vec![CodexBlock::identified(
                    Block::tool_result(tools),
                    CodexFact::Result,
                    id,
                )],
                "tool".to_string(),
                false,
            )
        }
        "additional_tools" | "compaction" | "compaction_summary" | "context_compaction"
        | "compaction_trigger" => (Vec::new(), String::new(), false),
        _ => (Vec::new(), String::new(), true),
    }
}

fn codex_event_msg(payload: &Value) -> (Vec<CodexBlock>, String, bool) {
    let Some(kind) = payload.get("type").and_then(Value::as_str) else {
        return (Vec::new(), String::new(), true);
    };
    match kind {
        "item_completed" => codex_completed_item(payload.get("item").unwrap_or(&Value::Null)),
        "user_message" => {
            let text = payload
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                (Vec::new(), "user".to_string(), false)
            } else {
                (
                    vec![CodexBlock::plain(Block::text(text))],
                    "user".to_string(),
                    false,
                )
            }
        }
        "agent_message" => {
            let text = payload
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                (Vec::new(), "assistant".to_string(), false)
            } else {
                (
                    vec![CodexBlock::plain(Block::text(text))],
                    "assistant".to_string(),
                    false,
                )
            }
        }
        "agent_reasoning" => {
            let text = payload
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                (Vec::new(), "assistant".to_string(), false)
            } else {
                (
                    vec![CodexBlock::plain(Block::reasoning(text))],
                    "assistant".to_string(),
                    false,
                )
            }
        }
        "exec_command_begin" | "exec_command_end" => {
            let command = payload_text(payload.get("command")).unwrap_or_default();
            let id = payload.get("call_id").and_then(Value::as_str);
            if kind == "exec_command_begin" {
                (
                    vec![CodexBlock::identified(
                        Block::tool(command),
                        CodexFact::Call,
                        id,
                    )],
                    "assistant".to_string(),
                    false,
                )
            } else {
                (
                    vec![CodexBlock::identified(
                        Block::tool_result(codex_command_output(payload)),
                        CodexFact::Result,
                        id,
                    )],
                    "tool".to_string(),
                    false,
                )
            }
        }
        "mcp_tool_call_begin" | "mcp_tool_call_end" => {
            let text = if kind == "mcp_tool_call_begin" {
                format!(
                    "{} {}",
                    payload
                        .pointer("/invocation/server")
                        .and_then(Value::as_str)
                        .unwrap_or("mcp"),
                    payload
                        .pointer("/invocation/tool")
                        .and_then(Value::as_str)
                        .unwrap_or("?")
                )
            } else {
                payload_text(payload.pointer("/result/Ok/content"))
                    .or_else(|| payload_text(payload.pointer("/result/Err")))
                    .unwrap_or_default()
            };
            let id = payload.get("call_id").and_then(Value::as_str);
            if kind == "mcp_tool_call_begin" {
                (
                    vec![CodexBlock::identified(
                        Block::tool(text),
                        CodexFact::Call,
                        id,
                    )],
                    "assistant".to_string(),
                    false,
                )
            } else {
                (
                    vec![CodexBlock::identified(
                        Block::tool_result(text),
                        CodexFact::Result,
                        id,
                    )],
                    "tool".to_string(),
                    false,
                )
            }
        }
        "dynamic_tool_call_request" | "dynamic_tool_call_response" => {
            let text = payload_text(payload.get("arguments")).unwrap_or_default();
            let id = if kind == "dynamic_tool_call_request" {
                payload.get("callId").and_then(Value::as_str)
            } else {
                payload.get("call_id").and_then(Value::as_str)
            };
            if kind == "dynamic_tool_call_request" {
                (
                    vec![CodexBlock::identified(
                        Block::tool(text),
                        CodexFact::Call,
                        id,
                    )],
                    "assistant".to_string(),
                    false,
                )
            } else {
                (
                    vec![CodexBlock::identified(
                        Block::tool_result(text),
                        CodexFact::Result,
                        id,
                    )],
                    "tool".to_string(),
                    false,
                )
            }
        }
        _ if codex_quiet_event(kind) => (Vec::new(), String::new(), false),
        _ => (Vec::new(), String::new(), true),
    }
}

fn codex_command_output(payload: &Value) -> String {
    let stdout = payload.get("stdout").and_then(Value::as_str).unwrap_or("");
    let stderr = payload.get("stderr").and_then(Value::as_str).unwrap_or("");
    if !stdout.is_empty() || !stderr.is_empty() {
        return format!("{stdout}{stderr}");
    }
    if let Some(aggregate) = payload
        .get("aggregated_output")
        .and_then(Value::as_str)
        .filter(|text| !text.is_empty())
    {
        return aggregate.to_string();
    }
    payload
        .get("formatted_output")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string()
}

fn codex_completed_item(item: &Value) -> (Vec<CodexBlock>, String, bool) {
    let Some(kind) = item.get("type").and_then(Value::as_str) else {
        return (Vec::new(), String::new(), true);
    };
    match kind {
        "UserMessage" | "AgentMessage" => {
            let assistant = kind == "AgentMessage";
            let role = if assistant { "assistant" } else { "user" };
            let id = if assistant {
                item.get("id").and_then(Value::as_str)
            } else {
                None
            };
            let mut blocks = Vec::new();
            let mut unrecognized = false;
            match item.get("content") {
                Some(Value::Array(parts)) => {
                    for part in parts {
                        match part.get("type").and_then(Value::as_str) {
                            Some("input_text" | "output_text" | "text") => {
                                if let Some(text) = part.get("text").and_then(Value::as_str) {
                                    blocks.push(CodexBlock::identified(
                                        Block::text(text),
                                        CodexFact::Message,
                                        id,
                                    ));
                                }
                            }
                            Some(kind @ ("input_image" | "input_audio" | "image" | "audio")) => {
                                blocks.push(CodexBlock::identified(
                                    Block::omitted(media_omission(kind)),
                                    CodexFact::Message,
                                    id,
                                ));
                            }
                            _ => unrecognized = true,
                        }
                    }
                }
                Some(Value::String(text)) => blocks.push(CodexBlock::identified(
                    Block::text(text.clone()),
                    CodexFact::Message,
                    id,
                )),
                None | Some(Value::Null) => {}
                Some(_) => unrecognized = true,
            }
            (blocks, role.to_string(), unrecognized)
        }
        "Reasoning" => {
            let id = item.get("id").and_then(Value::as_str);
            let mut blocks = Vec::new();
            match item.get("summary_text") {
                Some(Value::Array(parts)) => {
                    for part in parts {
                        if let Some(text) = part.as_str() {
                            blocks.push(CodexBlock::identified(
                                Block::reasoning(text),
                                CodexFact::Reasoning,
                                id,
                            ));
                        }
                    }
                }
                None | Some(Value::Null) => {}
                Some(_) => {}
            }
            (blocks, "assistant".to_string(), false)
        }
        "FunctionCallOutput" => {
            let output = payload_text(item.get("output")).unwrap_or_default();
            let id = item.get("call_id").and_then(Value::as_str);
            (
                vec![CodexBlock::identified(
                    Block::tool_result(output),
                    CodexFact::Result,
                    id,
                )],
                "tool".to_string(),
                false,
            )
        }
        "CommandExecution" => {
            let command = payload_text(item.get("command")).unwrap_or_default();
            let output = codex_command_output(item);
            let id = item.get("id").and_then(Value::as_str);
            (
                vec![
                    CodexBlock::identified(Block::tool(command), CodexFact::Call, id),
                    CodexBlock::identified(Block::tool_result(output), CodexFact::Result, id),
                ],
                "assistant".to_string(),
                false,
            )
        }
        "DynamicToolCall" | "McpToolCall" => {
            let name = item
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string();
            let arguments = payload_text(item.get("arguments")).unwrap_or_default();
            let output = payload_text(item.get("content_items"))
                .or_else(|| payload_text(item.pointer("/result/content")))
                .or_else(|| payload_text(item.pointer("/error/message")))
                .unwrap_or_default();
            let id = item.get("id").and_then(Value::as_str);
            (
                vec![
                    CodexBlock::identified(
                        Block::tool(format!("{name} {arguments}")),
                        CodexFact::Call,
                        id,
                    ),
                    CodexBlock::identified(Block::tool_result(output), CodexFact::Result, id),
                ],
                "assistant".to_string(),
                false,
            )
        }
        "Plan" => {
            let text = item
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if text.is_empty() {
                (Vec::new(), "assistant".to_string(), false)
            } else {
                (
                    vec![CodexBlock::plain(Block::text(text))],
                    "assistant".to_string(),
                    false,
                )
            }
        }
        _ => (Vec::new(), String::new(), true),
    }
}

// ----------------------------------------------------------------- DSH

/// DSH's recognized quiet operational/context event vocabulary
/// (design D6's captured catalog minus the five content kinds).
fn dsh_quiet_event(kind: &str) -> bool {
    matches!(
        kind,
        "agent-preset/selected"
            | "agent/inbox/spliced"
            | "approval/asked"
            | "approval/decided"
            | "approval/policy"
            | "command/done"
            | "command/run"
            | "compaction/end"
            | "compaction/prune"
            | "compaction/start"
            | "compaction/summary"
            | "feedback/record"
            | "goal/change"
            | "hook/invoked"
            | "hook/result"
            | "llm/retry"
            | "llm/retry-started"
            | "model/selection"
            | "permission/preset"
            | "plan/mode"
            | "request/context"
            | "request/header"
            | "sandbox/mode"
            | "schedule/change"
            | "session-log-deepseek/delivery-accepted"
            | "session/end-seed"
            | "session/title"
            | "session/title-llm-request"
            | "step/end"
            | "step/start"
            | "subagent/descriptor"
            | "subagent/model-selection-policy"
            | "team/member"
            | "team/message/delivered"
            | "team/message/queued"
            | "team/task"
            | "todo/write"
            | "tool-workflow/agent-end"
            | "tool-workflow/agent-start"
            | "tool-workflow/run-end"
            | "tool-workflow/run-start"
            | "tool/code-dispatch"
            | "tool/code-dispatch-start"
            | "turn/end"
            | "turn/start"
            | "web/deepseek-search-llm-request"
    )
}

const DSH_SAFE_MAX: i64 = 9_007_199_254_740_991;

fn dsh_numeric_zero(value: &Value) -> bool {
    value.as_f64() == Some(0.0)
}

/// Admit the opening DSH ownership header. Ownership was established by
/// discovery; this validates the version and reports the projection
/// refusal when it is not numeric zero.
fn dsh_header(value: &Value) -> bool {
    let Some(object) = value.as_object() else {
        return false;
    };
    if object.get("type").and_then(Value::as_str) != Some("session") {
        return false;
    }
    match object.get("delegationDepth") {
        None => {}
        Some(depth) if depth.as_u64() != Some(0) => return false,
        Some(_) => {}
    }
    object.get("version").is_some_and(dsh_numeric_zero)
}

fn project_dsh(admitted: &Admitted<'_>, projection: &mut Projection) {
    let mut header: Option<Value> = None;
    let mut opening_is_header = false;
    let mut events: Vec<DshEvent> = Vec::new();
    // Every observed logical identity counts once, including quiet
    // omissions, so a duplicate sequence stays ambiguous for citation
    // uniqueness.
    let mut observed: Vec<i64> = Vec::new();
    let mut refused = false;
    let mut unrecognized = 0u64;
    let skipped = for_each_parsed_row(admitted, |index, value| {
        // The opening physical row must itself be the admitted session
        // header: a malformed or absent first row cannot borrow a later one.
        if index == 0 {
            opening_is_header = true;
            header = Some(value);
            return;
        }
        match dsh_row(&value) {
            DshRow::Events(mut rows, row_unrecognized) => {
                if row_unrecognized {
                    unrecognized += 1;
                }
                for row in &rows {
                    if let Some(seq) = row.seq {
                        observed.push(seq);
                    }
                }
                events.append(&mut rows);
            }
            DshRow::Quiet => {
                if let Some(seq) = dsh_seq(&value) {
                    observed.push(seq);
                }
            }
            DshRow::Unrecognized => {
                unrecognized += 1;
                if let Some(seq) = dsh_seq(&value) {
                    observed.push(seq);
                }
                if !dsh_ignorable(&value) {
                    refused = true;
                }
            }
            DshRow::Omission => {
                // A recognized envelope with an unsupported nested variant
                // keeps no content and counts once without refusing.
                unrecognized += 1;
                if let Some(seq) = dsh_seq(&value) {
                    observed.push(seq);
                }
            }
            DshRow::Refused => {
                unrecognized += 1;
                refused = true;
            }
        }
    });
    if !opening_is_header {
        // No usable opening row: the discovery stage owns this refusal.
        projection.skipped_lines = skipped;
        projection.unavailable = Some(Unavailable::UnsupportedFormat);
        return;
    }
    let header = header.expect("a first parsed row stored the header");
    if !dsh_header(&header) {
        // Rejected version or depth: zero counts, no event classification.
        projection.skipped_lines = 0;
        projection.unrecognized_records = 0;
        projection.unavailable = Some(Unavailable::UnsupportedFormat);
        return;
    }
    projection.skipped_lines = skipped;
    projection.unrecognized_records = unrecognized;
    if refused {
        projection.turns = Vec::new();
        projection.unavailable = Some(Unavailable::UnsupportedFormat);
        return;
    }
    let mut seq_counts: std::collections::HashMap<i64, usize> = std::collections::HashMap::new();
    for seq in observed {
        *seq_counts.entry(seq).or_default() += 1;
    }
    // Citation suppression: an assembly with readable projected blocks
    // suppresses uniquely identified, earlier readable chunks it cites in
    // its own recorded turn/step. The chunk index by `(turn, step)` bounds
    // the work by the actual retained members instead of rescanning every
    // event for every assembly.
    let mut chunk_index: std::collections::HashMap<(i64, i64), Vec<usize>> =
        std::collections::HashMap::new();
    for (index, event) in events.iter().enumerate() {
        if !event.chunk {
            continue;
        }
        if let (Some(turn), Some(step)) = (event.turn, event.step) {
            chunk_index.entry((turn, step)).or_default().push(index);
        }
    }
    let mut suppressed = std::collections::HashSet::new();
    for (index, event) in events.iter().enumerate() {
        if !event.assembly || event.cited.is_empty() || event.blocks.is_empty() {
            continue;
        }
        let (Some(turn), Some(step)) = (event.turn, event.step) else {
            continue;
        };
        let Some(candidates) = chunk_index.get(&(turn, step)) else {
            continue;
        };
        for &candidate_index in candidates {
            if candidate_index >= index {
                continue;
            }
            let candidate = &events[candidate_index];
            let Some(seq) = candidate.seq else { continue };
            if seq_counts.get(&seq) != Some(&1) {
                continue;
            }
            if event
                .cited
                .iter()
                .any(|(start, end)| seq >= *start && seq <= *end)
            {
                suppressed.insert(candidate_index);
            }
        }
    }
    // Dedicated call/result events own matching embedded tool blocks at
    // their own source positions; the owning message keeps every other
    // block in recorded order.
    associate_dsh_tools(&mut events);
    let mut turns = Vec::new();
    for (index, event) in events.into_iter().enumerate() {
        if suppressed.contains(&index) {
            continue;
        }
        let blocks: Vec<Block> = event.blocks.into_iter().map(|block| block.block).collect();
        if blocks.is_empty() {
            continue;
        }
        turns.push(Turn {
            role: event.role,
            ts: event.ts,
            blocks,
        });
    }
    projection.turns = display_cap(turns, &mut projection.truncated);
}

/// The recorded identifier and direction a DSH tool block carries.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum DshDirection {
    Call,
    Result,
}

struct DshTool {
    id: String,
    direction: DshDirection,
}

/// One projected DSH block and the optional tool fact it stores.
struct DshBlock {
    block: Block,
    tool: Option<DshTool>,
}

impl DshBlock {
    fn plain(block: Block) -> DshBlock {
        DshBlock { block, tool: None }
    }
}

/// Associate dedicated `tool/call` and `tool/result` events with the tool
/// blocks embedded in messages. A dedicated event owns an embedded block
/// only when the recorded call id and the owning `(turn, step)` pair both
/// match exactly once; dedicated events never suppress one another and an
/// absent, colliding or disagreeing identity keeps both copies.
fn associate_dsh_tools(events: &mut [DshEvent]) {
    use std::collections::{HashMap, HashSet};
    let mut dedicated: HashMap<(String, DshDirection, i64, i64), usize> = HashMap::new();
    for event in events.iter() {
        if !event.dedicated {
            continue;
        }
        let (Some(turn), Some(step)) = (event.turn, event.step) else {
            continue;
        };
        let mut seen: HashSet<(String, DshDirection, i64, i64)> = HashSet::new();
        for block in &event.blocks {
            let Some(tool) = &block.tool else { continue };
            let key = (tool.id.clone(), tool.direction, turn, step);
            if seen.insert(key.clone()) {
                *dedicated.entry(key).or_default() += 1;
            }
        }
    }
    for event in events.iter_mut() {
        if event.dedicated {
            continue;
        }
        let (Some(turn), Some(step)) = (event.turn, event.step) else {
            continue;
        };
        event.blocks.retain(|block| match &block.tool {
            Some(tool) => dedicated.get(&(tool.id.clone(), tool.direction, turn, step)) != Some(&1),
            None => true,
        });
    }
}

fn dsh_ignorable(value: &Value) -> bool {
    value
        .as_object()
        .and_then(|object| object.get("ignorable"))
        .and_then(Value::as_bool)
        == Some(true)
}

struct DshEvent {
    blocks: Vec<DshBlock>,
    role: String,
    ts: String,
    seq: Option<i64>,
    turn: Option<i64>,
    step: Option<i64>,
    chunk: bool,
    assembly: bool,
    cited: Vec<(i64, i64)>,
    /// A dedicated `tool/call` or `tool/result` event.
    dedicated: bool,
}

enum DshRow {
    Events(Vec<DshEvent>, bool),
    Quiet,
    /// A recognized envelope with an unsupported or absent nested
    /// variant: counted once, no content, never a whole-read refusal.
    Omission,
    Unrecognized,
    Refused,
}

/// A recorded DSH millisecond number, including the floating-point `-0`
/// spelling, which is still zero milliseconds. Non-integer fractions are
/// not a millisecond count.
fn dsh_millis(value: &Value) -> Option<i64> {
    if let Some(integer) = value.as_i64() {
        return Some(integer);
    }
    if value.as_f64() == Some(0.0) {
        return Some(0);
    }
    None
}

/// The signed epoch-millisecond string DSH turns carry, or empty.
fn dsh_time(value: Option<&Value>) -> String {
    match value.and_then(dsh_millis) {
        Some(millis) if millis.unsigned_abs() <= DSH_SAFE_MAX as u64 => millis.to_string(),
        _ => String::new(),
    }
}

/// An ordinary (unpacked) DSH message row's blocks, each retaining the
/// recorded tool identifier it stores.
fn dsh_message_blocks(data: &Value) -> (Vec<DshBlock>, bool) {
    let mut blocks = Vec::new();
    let mut unrecognized = false;
    let content = data.get("content").or_else(|| data.get("blocks"));
    match content {
        Some(Value::Array(parts)) => {
            for part in parts {
                match part.get("type").and_then(Value::as_str) {
                    Some("text") => {
                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                            blocks.push(DshBlock::plain(Block::text(text)));
                        }
                    }
                    Some("reasoning") => {
                        if let Some(text) = part.get("text").and_then(Value::as_str) {
                            blocks.push(DshBlock::plain(Block::reasoning(text)));
                        }
                    }
                    Some("tool-call") => {
                        let name = part.get("name").and_then(Value::as_str).unwrap_or("?");
                        let arguments = payload_text(part.get("arguments")).unwrap_or_default();
                        let id = part
                            .get("id")
                            .and_then(Value::as_str)
                            .filter(|id| !id.is_empty());
                        blocks.push(DshBlock {
                            block: Block::tool(format!("{name} {arguments}")),
                            tool: id.map(|id| DshTool {
                                id: id.to_string(),
                                direction: DshDirection::Call,
                            }),
                        });
                    }
                    Some("tool-result") => {
                        let output = payload_text(part.get("content"))
                            .or_else(|| payload_text(part.get("text")))
                            .unwrap_or_default();
                        let id = part
                            .get("toolCallId")
                            .and_then(Value::as_str)
                            .filter(|id| !id.is_empty());
                        blocks.push(DshBlock {
                            block: Block::tool_result(output),
                            tool: id.map(|id| DshTool {
                                id: id.to_string(),
                                direction: DshDirection::Result,
                            }),
                        });
                    }
                    Some("image") => {
                        blocks.push(DshBlock::plain(Block::omitted("[image omitted]")))
                    }
                    _ => unrecognized = true,
                }
            }
        }
        Some(Value::String(text)) => blocks.push(DshBlock::plain(Block::text(text.clone()))),
        None | Some(Value::Null) => {}
        Some(_) => unrecognized = true,
    }
    (blocks, unrecognized)
}

/// Validate and extract `sourceEventSeqs` under the reading delta.
fn dsh_citations(value: &Value, owning_seq: Option<i64>) -> Result<Vec<(i64, i64)>, ()> {
    let Some(field) = value
        .as_object()
        .and_then(|object| object.get("sourceEventSeqs"))
    else {
        return Ok(Vec::new());
    };
    let Some(list) = field.as_array() else {
        return Err(());
    };
    if list.is_empty() {
        return Ok(Vec::new());
    }
    let owning = owning_seq
        .filter(|seq| (0..=DSH_SAFE_MAX).contains(seq))
        .ok_or(())?;
    let mut out = Vec::new();
    for entry in list {
        match entry {
            Value::Number(_) => {
                let seq = entry.as_i64().ok_or(())?;
                if !(0..=DSH_SAFE_MAX).contains(&seq) || seq >= owning {
                    return Err(());
                }
                out.push((seq, seq));
            }
            Value::Array(range) if range.len() == 2 => {
                let start = range[0].as_i64().ok_or(())?;
                let end = range[1].as_i64().ok_or(())?;
                if !(0..=DSH_SAFE_MAX).contains(&start)
                    || !(0..=DSH_SAFE_MAX).contains(&end)
                    || start > end
                    || end >= owning
                {
                    return Err(());
                }
                out.push((start, end));
            }
            _ => return Err(()),
        }
    }
    Ok(out)
}

/// The logical sequence a DSH event row records.
fn dsh_seq(value: &Value) -> Option<i64> {
    value
        .as_object()
        .and_then(|object| object.get("seq"))
        .and_then(Value::as_i64)
}

fn dsh_row(value: &Value) -> DshRow {
    let Some(object) = value.as_object() else {
        return DshRow::Unrecognized;
    };
    let Some(kind) = object.get("type").and_then(Value::as_str) else {
        return DshRow::Unrecognized;
    };
    let data = object.get("data").unwrap_or(&Value::Null);
    match kind {
        "user/message" | "assistant/message" => {
            let message = if kind == "user/message" {
                data
            } else {
                data.get("message").unwrap_or(&Value::Null)
            };
            let (blocks, unrecognized) = dsh_message_blocks(message);
            let owning = dsh_seq(value);
            let cited = match dsh_citations(value, owning) {
                Ok(cited) => cited,
                Err(()) => return DshRow::Refused,
            };
            let ts = dsh_time(object.get("time"));
            let role = if kind == "user/message" {
                "user"
            } else {
                "assistant"
            };
            let event = DshEvent {
                blocks,
                role: role.to_string(),
                ts,
                seq: owning,
                turn: data.get("turn").and_then(Value::as_i64),
                step: data.get("step").and_then(Value::as_i64),
                chunk: false,
                assembly: kind == "assistant/message",
                cited,
                dedicated: false,
            };
            if unrecognized {
                // A recognized envelope with an unsupported nested
                // variant keeps its supported siblings and counts once.
                return DshRow::Events(vec![event], true);
            }
            DshRow::Events(vec![event], unrecognized)
        }
        "tool/call" => {
            let name = data.get("name").and_then(Value::as_str).unwrap_or("?");
            let arguments = payload_text(data.get("arguments")).unwrap_or_default();
            let id = data
                .get("callId")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty());
            DshRow::Events(
                vec![DshEvent {
                    blocks: vec![DshBlock {
                        block: Block::tool(format!("{name} {arguments}")),
                        tool: id.map(|id| DshTool {
                            id: id.to_string(),
                            direction: DshDirection::Call,
                        }),
                    }],
                    role: "assistant".to_string(),
                    ts: dsh_time(object.get("time")),
                    seq: dsh_seq(value),
                    turn: data.get("turn").and_then(Value::as_i64),
                    step: data.get("step").and_then(Value::as_i64),
                    chunk: false,
                    assembly: false,
                    cited: Vec::new(),
                    dedicated: true,
                }],
                false,
            )
        }
        "tool/result" => {
            let message = data.get("message").unwrap_or(&Value::Null);
            let (blocks, unrecognized) = dsh_message_blocks(message);
            let owning = dsh_seq(value);
            let cited = match dsh_citations(value, owning) {
                Ok(cited) => cited,
                Err(()) => return DshRow::Refused,
            };
            DshRow::Events(
                vec![DshEvent {
                    blocks,
                    role: "tool".to_string(),
                    ts: dsh_time(object.get("time")),
                    seq: owning,
                    turn: data.get("turn").and_then(Value::as_i64),
                    step: data.get("step").and_then(Value::as_i64),
                    chunk: false,
                    assembly: false,
                    cited,
                    dedicated: true,
                }],
                unrecognized,
            )
        }
        "assistant/chunk" => {
            let chunk = data.get("chunk").unwrap_or(&Value::Null);
            let Some(chunk_kind) = chunk.get("type").and_then(Value::as_str) else {
                // A recognized chunk envelope with an absent nested
                // variant is a counted omission, not a whole-read refusal.
                return DshRow::Omission;
            };
            match chunk_kind {
                "text-delta" => {
                    let text = chunk.get("text").and_then(Value::as_str).unwrap_or("");
                    if text.is_empty() {
                        return DshRow::Quiet;
                    }
                    DshRow::Events(
                        vec![DshEvent {
                            blocks: vec![DshBlock::plain(Block::text(text))],
                            role: "assistant".to_string(),
                            ts: dsh_time(object.get("time")),
                            seq: dsh_seq(value),
                            turn: data.get("turn").and_then(Value::as_i64),
                            step: data.get("step").and_then(Value::as_i64),
                            chunk: true,
                            assembly: false,
                            cited: Vec::new(),
                            dedicated: false,
                        }],
                        false,
                    )
                }
                "reasoning-delta" => {
                    let text = chunk.get("text").and_then(Value::as_str).unwrap_or("");
                    if text.is_empty() {
                        return DshRow::Quiet;
                    }
                    DshRow::Events(
                        vec![DshEvent {
                            blocks: vec![DshBlock::plain(Block::reasoning(text))],
                            role: "assistant".to_string(),
                            ts: dsh_time(object.get("time")),
                            seq: dsh_seq(value),
                            turn: data.get("turn").and_then(Value::as_i64),
                            step: data.get("step").and_then(Value::as_i64),
                            chunk: true,
                            assembly: false,
                            cited: Vec::new(),
                            dedicated: false,
                        }],
                        false,
                    )
                }
                "tool-call-delta" | "block-start" | "block-end" | "usage" | "finish" => {
                    DshRow::Quiet
                }
                // Any other nested chunk variant counts once without
                // refusing the whole source.
                _ => DshRow::Omission,
            }
        }
        "text-chunks" | "reasoning-chunks" | "tool-call-chunks" => dsh_packed(kind, object, data),
        _ if dsh_quiet_event(kind) => DshRow::Quiet,
        _ => DshRow::Unrecognized,
    }
}

/// True when `object`'s key set is exactly the required keys plus at
/// most the optional ones: no missing required key and no key outside
/// the declared storage shape.
fn exact_keys(
    object: &serde_json::Map<String, Value>,
    required: &[&str],
    optional: &[&str],
) -> bool {
    if object.len() < required.len() || object.len() > required.len() + optional.len() {
        return false;
    }
    object
        .keys()
        .all(|key| required.contains(&key.as_str()) || optional.contains(&key.as_str()))
        && required.iter().all(|key| object.contains_key(*key))
}

/// Decode one packed DSH storage row. Any structural violation refuses
/// the whole read and counts the physical row once. The envelope and
/// data shapes are validated exactly before any member can supply
/// content or association.
fn dsh_packed(kind: &str, object: &serde_json::Map<String, Value>, data: &Value) -> DshRow {
    if !exact_keys(object, &["type", "seq0", "time0", "data"], &[]) {
        return DshRow::Refused;
    }
    let Some(data) = data.as_object() else {
        return DshRow::Refused;
    };
    let tool = kind == "tool-call-chunks";
    let shape_ok = if tool {
        exact_keys(
            data,
            &["turn", "step", "index", "dt", "args", "id"],
            &["name"],
        )
    } else {
        exact_keys(data, &["turn", "step", "index", "dt", "texts"], &[])
    };
    if !shape_ok {
        return DshRow::Refused;
    }
    let Some(turn) = data.get("turn").and_then(Value::as_i64) else {
        return DshRow::Refused;
    };
    let Some(step) = data.get("step").and_then(Value::as_i64) else {
        return DshRow::Refused;
    };
    let Some(index) = data.get("index").and_then(Value::as_i64) else {
        return DshRow::Refused;
    };
    let Some(seq0) = object.get("seq0").and_then(Value::as_i64) else {
        // A negative-zero spelling parses as a float and is excluded here.
        return DshRow::Refused;
    };
    if !(0..=DSH_SAFE_MAX).contains(&seq0) {
        return DshRow::Refused;
    }
    let Some(time0) = object.get("time0").and_then(dsh_millis) else {
        return DshRow::Refused;
    };
    if time0.unsigned_abs() > DSH_SAFE_MAX as u64 {
        return DshRow::Refused;
    }
    if tool {
        // The tool id is a required string; a present name must be one.
        if data.get("id").and_then(Value::as_str).is_none() {
            return DshRow::Refused;
        }
        if data.get("name").is_some_and(|name| name.as_str().is_none()) {
            return DshRow::Refused;
        }
    }
    let Some(dt) = data.get("dt").and_then(Value::as_array) else {
        return DshRow::Refused;
    };
    let members = match data.get(if tool { "args" } else { "texts" }) {
        Some(Value::Array(members)) if !members.is_empty() => members,
        _ => return DshRow::Refused,
    };
    if dt.len() + 1 != members.len() {
        return DshRow::Refused;
    }
    if !members.iter().all(|member| member.is_string()) {
        return DshRow::Refused;
    }
    let mut gaps = Vec::new();
    for gap in dt {
        let Some(gap) = dsh_millis(gap) else {
            return DshRow::Refused;
        };
        if gap.unsigned_abs() > DSH_SAFE_MAX as u64 {
            return DshRow::Refused;
        }
        gaps.push(gap);
    }
    let mut events = Vec::new();
    let mut time = time0;
    for (member_index, member) in members.iter().enumerate() {
        if member_index > 0 {
            time = match time.checked_add(gaps[member_index - 1]) {
                Some(time) if time.unsigned_abs() <= DSH_SAFE_MAX as u64 => time,
                _ => return DshRow::Refused,
            };
        }
        let seq = match seq0.checked_add(member_index as i64) {
            Some(seq) if (0..=DSH_SAFE_MAX).contains(&seq) => seq,
            _ => return DshRow::Refused,
        };
        let text = member.as_str().unwrap_or("");
        let blocks = match kind {
            "text-chunks" if !text.is_empty() => vec![DshBlock::plain(Block::text(text))],
            "reasoning-chunks" if !text.is_empty() => {
                vec![DshBlock::plain(Block::reasoning(text))]
            }
            // Argument fragments are recognized quiet omissions.
            _ => Vec::new(),
        };
        events.push(DshEvent {
            blocks,
            role: "assistant".to_string(),
            ts: time.to_string(),
            seq: Some(seq),
            turn: Some(turn),
            step: Some(step),
            chunk: kind != "tool-call-chunks",
            assembly: false,
            cited: Vec::new(),
            dedicated: false,
        });
    }
    let _ = index;
    DshRow::Events(events, false)
}

#[cfg(test)]
mod tests;
