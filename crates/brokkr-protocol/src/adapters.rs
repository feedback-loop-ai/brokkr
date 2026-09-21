//! Built-in driver adapters:
//! `brokkr driver <claude|lanetally|codex|dsh|exec>`.
//!
//! The Rust port of the retired Python adapters (decision 0009) — same
//! protocol behavior, same prompt composition, same result-file
//! contract, byte-for-byte compatible with the existing charters. The
//! seat writes its typed result to the file named in the input; a
//! missing file fails the attempt, an unparseable one is forwarded so
//! the ENGINE parks with raw evidence (decision 0001). Adapters never
//! repair anything.
//!
//! Env overrides for conformance shims: BROKKR_CLAUDE_BIN,
//! BROKKR_LANETALLY_BIN, BROKKR_CODEX_BIN, BROKKR_DSH_BIN,
//! BROKKR_EXEC_NAME. All five names answer to their old `FORGE_*`
//! spelling for one more release (decision 0019, `legacy`).

use std::io::{BufRead, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::{json, Map, Value};

mod composite;
mod route_overlay;
// Design D6 (b) seals the producer: the seams, the structured
// observation, its error and the one entry point. Every parser, hasher,
// serializer and injected helper stays private to `composite`, so no
// caller can supply an already-computed component or composite value.
pub use composite::{
    dsh_composite, dsh_composite_prepared, CompositeError, DshComposite, DshInvocation, DshNode,
    DshPrepared, DshSeams, DshSelection, DshUnprepared, DshUnselected,
};

use crate::dsh_sandbox;
use crate::hands::GitFacts;
use crate::secret;
use crate::transcript::{dsh_transcript_root_under, Kind as TranscriptKind, Transcript};
use crate::{Body, Message, ResultStatus};

const ADAPTER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const MODEL_NOT_REPORTED: &str = "not reported";
pub const MODEL_NOT_APPLICABLE: &str = "not applicable";

/// Decision 0035 ruling 3 reuses decision 0031's two sentinels for the
/// configured effort rather than inventing a second pair, and the
/// distinction between them is the one dsh makes visible: a control that
/// exists but goes unreported (`not reported`) is not a control that
/// does not exist (`not applicable`).
pub const EFFORT_NOT_REPORTED: &str = MODEL_NOT_REPORTED;
pub const EFFORT_NOT_APPLICABLE: &str = MODEL_NOT_APPLICABLE;

const USAGE_FIELDS: [&str; 5] = [
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "reasoning_output_tokens",
];

const RESULT_RECORD_FIELDS: [&str; 9] = [
    "num_turns",
    "input_tokens",
    "output_tokens",
    "cache_read_tokens",
    "cache_write_tokens",
    "reasoning_output_tokens",
    "total_cost_usd",
    "session_id",
    "transcript",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdapterKind {
    Claude,
    Lanetally,
    Codex,
    Dsh,
    Exec,
}

impl AdapterKind {
    pub fn parse(name: &str) -> Option<AdapterKind> {
        match name {
            "claude" => Some(AdapterKind::Claude),
            "lanetally" => Some(AdapterKind::Lanetally),
            "codex" => Some(AdapterKind::Codex),
            "dsh" => Some(AdapterKind::Dsh),
            "exec" => Some(AdapterKind::Exec),
            _ => None,
        }
    }

    /// What this adapter honours of the protocol's OPTIONAL vocabulary.
    ///
    /// `resume` is **offer receipt** and nothing more (proposed decision
    /// 0056 ruling 4): it says this adapter knows what a handle is, will
    /// correlate it and will decide it — including deciding to decline
    /// it and say why. It is NOT a claim that any offered session can be
    /// resumed; that question is the adapter's measured assessment, per
    /// named execution shape, and it is asked per invocation.
    ///
    /// All four model adapters declare receipt so that a claude or dsh
    /// retry can be told about its own session and answer honestly,
    /// which is issue #226's complaint. Exec declares none: it has no
    /// model turn and no provider to hold a session. A driver that does
    /// not declare receipt is never handed a handle, by any channel.
    fn supports(&self) -> Vec<String> {
        match self {
            AdapterKind::Exec => Vec::new(),
            _ => vec!["resume".to_string()],
        }
    }

    fn driver_name(&self) -> String {
        match self {
            AdapterKind::Claude => "claude-code".to_string(),
            // The Claude Code harness through LaneTally's session-capture
            // wrapper; the ledger discriminator is the checkpoint's
            // `capture` field, never this (forgeable) step-name stem.
            AdapterKind::Lanetally => "claude-lanetally".to_string(),
            AdapterKind::Codex => "codex".to_string(),
            AdapterKind::Dsh => "deepseek-harness".to_string(),
            AdapterKind::Exec => {
                adapter_binary("BROKKR_EXEC_NAME", Some("FORGE_EXEC_NAME"), "exec")
            }
        }
    }
}

/// The hands paragraph a model-backed seat reads inside its result
/// contract (decision 0043; decision 0046 rulings 3 and 4). Keyed off
/// the input's `hands` marker and `boundary` word:
///
/// - `hands: boxed` — today's words: the workspace tool is the only
///   writer, whatever the boxed word is;
/// - `boundary: harness` — the harness's own sandbox stands, no
///   workspace tool is served, and the result reaches the engine through
///   the door the input names: the one file the sandbox lets the seat
///   write, or the seat's final message, which the harness captures;
/// - `boundary: open` — nothing of Brokkr's stands; the seat writes the
///   file;
/// - neither — no paragraph, as before the box existed.
fn hands_paragraph(input: &Value) -> String {
    let word = input.get("boundary").and_then(Value::as_str);
    if input.get("hands").and_then(Value::as_str) == Some("boxed") {
        return "\n\nYour hands are boxed: the worktree, and this result file, are \
         reachable ONLY through the `mcp__brokkr__workspace` tool. Your \
         harness's own shell runs outside the box and cannot write here — a \
         file written through it never reaches the engine. Write the result \
         file with the workspace tool."
            .to_string();
    }
    match word {
        Some("harness") if last_message_door(input) => "\n\nYour hands stand under the \
         `harness` boundary: no workspace tool of Brokkr's is served, and you run under \
         your harness's own read-only sandbox. Your FINAL message must be exactly the \
         result object above and nothing else — the harness writes that message to the \
         result path, so you do not write the file yourself."
            .to_string(),
        Some("harness") => "\n\nYour hands stand under the `harness` boundary: no workspace \
         tool of Brokkr's is served, and you run under your harness's own sandbox. The \
         result path above is the one file that sandbox lets you write; write it yourself."
            .to_string(),
        Some("open") => "\n\nYour hands stand under the `open` boundary: nothing of \
         Brokkr's stands between you and the machine, and no workspace tool is served. \
         Write the result file yourself."
            .to_string(),
        _ => String::new(),
    }
}

/// Whether the seat's result reaches the engine through its harness's
/// capture of the final message rather than a file the seat writes
/// (decision 0046 ruling 4; design D23).
fn last_message_door(input: &Value) -> bool {
    input.get("result_delivery").and_then(Value::as_str) == Some("last-message")
}

/// Render the model-facing prompt from the three independently owned texts in
/// one engine input: charter, optional realm house, and site result contract.
///
/// `kind` is the driver about to read it (decision 0046; design DD21): the
/// hands paragraph is prose for a model, so it is rendered for the four
/// model kinds and never for `exec`, whose script reads the composed
/// environment and not a sentence. The rest of the prompt is the same
/// for every kind, which is what lets the shipped verify and ship
/// scripts keep reading the result path off it by line.
pub fn render_prompt(input: &Value, kind: AdapterKind) -> String {
    let get = |key: &str| input.get(key).and_then(Value::as_str).unwrap_or("");
    let role = input
        .get("role_path")
        .and_then(Value::as_str)
        .and_then(|p| std::fs::read_to_string(p).ok())
        .unwrap_or_default();
    let context = serde_json::to_string_pretty(input.get("context").unwrap_or(&json!({})))
        .unwrap_or_default();
    let allowed = input
        .get("allowed_results")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();
    let house = input
        .get("house_rules")
        .and_then(Value::as_str)
        .map(|text| format!("\n\n## House rules\n\n{}", text.trim()))
        .unwrap_or_default();
    let dialect = input
        .get("spec_dialect")
        .and_then(Value::as_str)
        .map(|text| format!("\n\n## Spec dialect\n\n{}", text.trim()))
        .unwrap_or_default();
    // Decision 0043: a boxed seat's harness keeps its own shell outside
    // the box, read-only or worse, and the first astra-judged gate wrote
    // its verdict through that shell twice and met no result contract.
    // The contract therefore names the one tool that can write — and,
    // since decision 0046, the boundary the seat stands under. An exec
    // driver reads no paragraph: its script reads the environment.
    // Decision 0065 ruling 5: beside its hands, a model seat is told by
    // capability name what it holds and what it does not, so it never
    // discovers a missing tool by failing to call it — from the same
    // record the launch was composed from.
    let hands = match kind {
        AdapterKind::Exec => String::new(),
        _ => format!(
            "{}{}",
            hands_paragraph(input),
            crate::native_controls::capabilities_paragraph(input)
        ),
    };
    // Under a `last-message` door (decision 0046 ruling 4) the contract's
    // own line says how the file comes to exist: the harness writes the
    // seat's final message to it. The path stays on its own line, where
    // every reader of this prompt finds it.
    let (asked, only) = if kind != AdapterKind::Exec && last_message_door(input) {
        (
            "your FINAL message must be exactly a JSON object, which your harness writes to \
             exactly this file",
            "The file is the ONLY channel the engine reads; your harness writes your final \
             message there, so a final message that is not the bare object counts as \
             producing no result.",
        )
    } else {
        (
            "write a JSON object to exactly this file",
            "The file is the ONLY channel the engine reads. Printing the JSON instead of \
             writing the file counts as producing no result.",
        )
    };
    format!(
        "{role}{house}{dialect}\n\n---\n## Task\n\nFeature: {feature}\nPhase: {phase} (you are this \
         phase's only seat)\nWorking directory: {workdir}\n\nRun context \
         (journal-derived, read-only):\n```json\n{context}\n```\n\n## Result contract \
         — MANDATORY\n\nWhen your work is finished, {asked}:\n\n    {result_path}\n\nwith the shape:\n\n    {{\"result\": \
         \"<one of: {allowed}>\",\n      \"inputs\": {{ ...optional typed facts for \
         the phase machine... }},\n      \"notes\": \"<short human summary of what \
         you did and why>\"}}

{only} The object carries exactly these top-level \
         keys — result, inputs, notes — and nothing else: a typed fact goes INSIDE \
         inputs, and a record with any other top-level key is refused where it is \
         sealed (decision 0034), which loses the whole attempt. You never decide the next phase — the engine's policy table rules \
         on your typed result.{hands}\n",
        role = role,
        house = house,
        dialect = dialect,
        feature = get("feature"),
        phase = get("phase"),
        workdir = get("workdir"),
        context = context,
        result_path = get("result_path"),
        allowed = allowed,
        hands = hands,
    )
}

struct Invocation {
    exit_code: i32,
    session_meta: Map<String, Value>,
    stdout: String,
    stderr: String,
    state: Option<String>,
    /// A provider refusal the harness recorded BEFORE its first turn
    /// (decision 0053). `Some(reason)` is a candidate refusal to start:
    /// no turn had opened when it was classified, so — IF no work began
    /// afterwards — no work exists for a different model to fail to
    /// inherit, and the attempt is reported with `result: failed` and NO
    /// `accepted` so the engine's structural fail-to-start predicate can
    /// advance the chain. `None` is every other shape, including a
    /// refusal that arrived after the first turn — that one stays a
    /// mid-session failure under decision 0016.
    ///
    /// The "if" is `run_seat`'s to settle, not the fold's: a harness may
    /// report an error and then go on to work, and a session that worked
    /// is not one that refused to start.
    refusal: Option<String>,
    /// How this invocation ended against the session it was launched for
    /// (proposed decision 0056 ruling 7, design D7). `run_seat` refuses
    /// to accept a successful seat from an unsettled rejoin even when the
    /// child exited clean and wrote a result file.
    launch: LaunchTerminal,
}

/// Provider-reported model ids are journal data, so admit only the
/// identifier alphabet providers use and clamp them before they cross
/// the driver boundary. In particular, no model output or diagnostic
/// prose can become a model label.
fn model_token(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.chars().count() > 80
        || !raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':' | '/'))
    {
        return None;
    }
    Some(raw.to_string())
}

/// The built-in harnesses have used each of these envelopes for usage
/// reports. This reads only a string in a named model field; it never
/// scans prose or substitutes the configured pin.
fn model_in_json(event: &Value) -> Option<String> {
    [
        "/model",
        "/message/model",
        "/usage/model",
        "/data/model",
        "/data/usage/model",
        "/data/message/source/model",
        "/message/source/model",
    ]
    .into_iter()
    .find_map(|pointer| event.pointer(pointer).and_then(Value::as_str))
    .and_then(model_token)
}

/// A configured effort as the journal records it: one bounded word of
/// the vocabulary a harness names its levels with. Clamped for the same
/// reason a model id is — the value crosses the driver boundary into an
/// append-only journal — and tighter, because an effort is a level and
/// never a path, an id, or a sentence.
///
/// The clamp is exactly `seat-record.v2`'s `effort` pattern, leading
/// character included: a level must START with an alphanumeric. A
/// harness echoing `_high` is refused HERE, where the cost is one turn
/// recording no effort, rather than journaled and refused later at
/// export — which would cost the whole run its export.
fn effort_token(raw: &str) -> Option<String> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.chars().count() > 40
        || !raw.starts_with(|c: char| c.is_ascii_alphanumeric())
        || !raw
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
    {
        return None;
    }
    Some(raw.to_string())
}

/// The harness's own echo of the effort it applied (decision 0035
/// ruling 3), as it rides a live STREAM: claude puts a top-level
/// `effort` beside every assistant record, and a codex release that
/// closes with a `result` nests the turn context it ran under.
///
/// It does NOT read the codex thread record — that is a different
/// envelope with its own reader ([`effort_in_thread`]), and the two are
/// kept apart so neither one's pointers can quietly stand in for the
/// other's. A single reader spanning both is how the thread record came
/// to be addressed as though a rollout nested each record under its own
/// name, which no rollout has ever done.
///
/// This reads only a string in a named effort field. It never scans
/// prose, and it never substitutes the configured pin: the pin is what a
/// bundle ASKED for, and the whole point of reading the echo is that it
/// is the value that survived every profile and plugin layer.
fn effort_in_json(event: &Value) -> Option<String> {
    ["/effort", "/turn_context/effort"]
        .into_iter()
        .find_map(|pointer| event.pointer(pointer).and_then(Value::as_str))
        .and_then(effort_token)
}

/// The codex thread record's own envelope, which is NOT the stream's:
/// every rollout line is `{"type": <record>, "payload": {…}}`, so the
/// fields a record names sit one level under `payload` and never under
/// a key spelled like the record. `turn_context` carries `model` and
/// `effort` there; the once-per-thread `thread_settings_applied` event
/// carries `model` and `reasoning_effort` under `thread_settings`.
///
/// Read as a pair because they come off one file read, and clamped by
/// the same two token rules everything crossing this boundary is: a
/// rollout is codex's file, not ours. Only the two shapes a rollout
/// actually writes are addressed — a pointer for a shape no codex has
/// ever produced is how the reader this replaces came to be wrong.
/// A rollout line worth PARSING, decided on its raw text before any
/// JSON is built: only the two record types above can name a model or an
/// effort, and each spells its own name in the line that carries it.
///
/// The filter is not an optimisation for its own sake. A rollout holds
/// every message payload of the whole thread — thousands of records,
/// megabytes — and the newest record naming either field can sit near
/// its TOP, because a release that applies its thread settings once
/// writes one such record and never another. Parsing each line to find
/// that out parsed essentially the whole file, once per turn, for the
/// whole life of a seat. A false positive here costs one parse the
/// pointers then refuse; a false negative is impossible, because a
/// record cannot be of a type it does not name.
fn names_thread_settings(line: &str) -> bool {
    line.contains("turn_context") || line.contains("thread_settings")
}

fn model_in_thread(line: &Value) -> Option<String> {
    ["/payload/model", "/payload/thread_settings/model"]
        .into_iter()
        .find_map(|pointer| line.pointer(pointer).and_then(Value::as_str))
        .and_then(model_token)
}

fn effort_in_thread(line: &Value) -> Option<String> {
    [
        "/payload/effort",
        "/payload/thread_settings/reasoning_effort",
    ]
    .into_iter()
    .find_map(|pointer| line.pointer(pointer).and_then(Value::as_str))
    .and_then(effort_token)
}

/// Codex releases which omit the model from JSONL name it in the
/// driver's own launch/usage header. The anchored `model:` field is the
/// only stderr text admitted as evidence.
fn model_in_header(stderr: &str) -> Option<String> {
    stderr
        .lines()
        .rev()
        .find_map(|line| line.trim().strip_prefix("model:").and_then(model_token))
}

fn positive_count(value: Option<&Value>) -> Option<u64> {
    value.and_then(Value::as_u64).filter(|value| *value > 0)
}

fn positive_cost(value: Option<&Value>) -> Option<f64> {
    value.and_then(Value::as_f64).filter(|value| *value > 0.0)
}

/// Claude reports cache reads and cache creations beside its uncached
/// input count. The seat record's input count is inclusive of reads,
/// while cache creations retain their own `cache_write_tokens` name.
fn claude_usage(usage: Option<&Value>) -> Map<String, Value> {
    let usage = usage.unwrap_or(&Value::Null);
    let input = positive_count(usage.get("input_tokens"));
    let cache_read = positive_count(usage.get("cache_read_input_tokens"));
    let mut record = Map::new();
    let inclusive = input
        .unwrap_or_default()
        .saturating_add(cache_read.unwrap_or_default());
    if inclusive > 0 {
        record.insert("input_tokens".into(), Value::from(inclusive));
    }
    for (source, target) in [
        ("output_tokens", "output_tokens"),
        ("cache_read_input_tokens", "cache_read_tokens"),
        ("cache_creation_input_tokens", "cache_write_tokens"),
    ] {
        if let Some(value) = positive_count(usage.get(source)) {
            record.insert(target.into(), Value::from(value));
        }
    }
    record
}

fn io_context<T>(result: std::io::Result<T>, context: &str) -> Result<T, String> {
    match result {
        Ok(value) => Ok(value),
        Err(error) => Err(format!("{context}: {error}")),
    }
}

/// One reader for every override, so the one-release fallback and its
/// one-time note are wired once rather than per variable. `legacy` is
/// the old `FORGE_*` spelling where decision 0019 renamed the variable,
/// and `None` where it did not.
fn adapter_binary(primary: &str, legacy: Option<&str>, fallback: &str) -> String {
    crate::legacy::env(primary, legacy).unwrap_or_else(|| fallback.to_string())
}

fn write_prompt(writer: &mut impl Write, payload: &str) -> Result<(), String> {
    io_context(
        writer.write_all(payload.as_bytes()),
        "could not write the prompt",
    )
}

fn run_cli(
    command: &[String],
    stdin_payload: Option<&str>,
    workdir: &str,
    bindings: &[secret::BoundSecret],
) -> Result<std::process::Output, String> {
    let (program, args) = command
        .split_first()
        .ok_or_else(|| "empty command".to_string())?;
    let mut invocation = Command::new(program);
    invocation
        .args(args)
        .current_dir(if workdir.is_empty() { "." } else { workdir })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    // Injection discipline (decision 0012, layer 3): values reach the
    // child ONLY through its environment, resolved at spawn time — never
    // argv (/proc/*/cmdline is world-readable), never the template. This
    // is the sole production call site of expose_for_spawn, CI-grep
    // pinned. A declared name overrides any pre-existing env entry: the
    // declaration is in the reviewed charter, so a collision is visible
    // at review time.
    for binding in bindings {
        let value = match std::str::from_utf8(binding.secret().expose_for_spawn()) {
            Ok(value) => value,
            Err(_) => return Err(format!("secret '{}' is not valid UTF-8", binding.name())),
        };
        invocation.env(binding.name(), value);
    }
    let mut child = io_context(invocation.spawn(), "could not invoke the agent CLI")?;
    if let Some(payload) = stdin_payload {
        let mut stdin = child.stdin.take().expect("piped");
        write_prompt(&mut stdin, payload)?;
    } else {
        drop(child.stdin.take());
    }
    io_context(child.wait_with_output(), "agent CLI did not conclude")
}

// ---------------------------------------------------------------------
// The launch lifecycle (proposed decision 0056 rulings 5, 6, 7 and 8).
//
//   plan -> child spawned -> exact root confirmed -> current work -> end
//
// A launch fact is published at the third arrow and nowhere else. A
// resume flag, a preassigned id, a known old handle, surviving edits,
// replayed transcript rows and an exit status of zero are none of them
// evidence that a rejoin happened, and this is the code that refuses to
// treat them as such.
// ---------------------------------------------------------------------

/// The provider-confirmed root a launch stood on, in seat-record v5's
/// closed vocabulary. Constructed only from an observation of the
/// harness naming its own session, never from what we asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct RootSession {
    kind: &'static str,
    id: String,
    harness_version: String,
    wrapper_digest: Option<String>,
    persistent: bool,
}

impl RootSession {
    fn value(&self) -> Value {
        let mut root = Map::new();
        root.insert("kind".into(), Value::String(self.kind.to_string()));
        root.insert("id".into(), Value::String(self.id.clone()));
        root.insert(
            "harness_version".into(),
            Value::String(self.harness_version.clone()),
        );
        if let Some(digest) = &self.wrapper_digest {
            root.insert("wrapper_digest".into(), Value::String(digest.clone()));
        }
        root.insert("persistent".into(), Value::Bool(self.persistent));
        Value::Object(root)
    }
}

/// The complete provider identifier grammar seat-record v5 admits:
/// 1–80 ASCII characters, alphanumeric first, then alphanumerics,
/// underscores and hyphens. Narrower than any bound a caller might
/// invent and wider than every provider's own spelling, so an id that
/// does not fit is REFUSED rather than truncated into a different
/// valid-looking identifier.
fn recordable_session_id(id: &str) -> bool {
    let mut characters = id.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphanumeric())
        && id.len() <= 80
        && characters.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// A harness version as v5 records it: 1–80 ASCII characters,
/// alphanumeric first, then alphanumerics, dots, underscores, plus and
/// hyphen. A version string that does not fit is not recorded, which
/// disables resume rather than recording a version nobody can compare.
fn recordable_version(version: &str) -> bool {
    let mut characters = version.chars();
    matches!(characters.next(), Some(first) if first.is_ascii_alphanumeric())
        && version.len() <= 80
        && characters.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '+' | '-'))
}

/// One invocation's launch plan, settled before the child spawns: the
/// argv, the root this launch intends to rejoin (`None` is a fresh
/// session), why an offer could not be taken, and the facts a confirmed
/// root will be recorded with.
struct LaunchPlan {
    command: Vec<String>,
    /// The exact root this launch was built to rejoin. A confirmation
    /// that names anything else is a mismatch, not a rename.
    rejoining: Option<String>,
    refusal: Option<&'static str>,
    /// Codex's re-imposed sandbox class, in v4's three-word vocabulary.
    sandbox: Option<String>,
    kind: &'static str,
    /// The version OBSERVED by this invocation's probe. `None` where no
    /// probe ran — an unmeasured or unsupported shape — under which no
    /// root is recorded and the next retry gets no offer.
    harness_version: Option<String>,
    wrapper_digest: Option<String>,
    /// Whether the shape this launch used persists its root at all.
    persistent: bool,
    /// Whether this harness's transcript locator IS its provider session
    /// identifier. True for claude and codex, which announce an id;
    /// false for dsh, whose locator is a retained directory — and a
    /// directory is not a provider handle without measured equivalence
    /// (proposed decision 0056 ruling 3).
    confirms_from_locator: bool,
    /// The applied effort this harness's launch row states, where the
    /// launch itself knows it. dsh seeds `not applicable` for a seat on
    /// an effortless route, so the row reads the standing from the first
    /// checkpoint (decision 0035 addendum 2026-09-11); the harnesses
    /// whose rows take it from the fold leave this `None`.
    effort: Option<String>,
}

impl LaunchPlan {
    /// A cold plan carrying only what a cold launch can honestly say.
    /// Its version is deliberately absent: this constructor serves
    /// ruling 8's replacement, and the child it replaces was refused
    /// before it named anything, so there is nothing observed to record.
    fn cold(command: Vec<String>, kind: &'static str, refusal: Option<&'static str>) -> LaunchPlan {
        LaunchPlan {
            command,
            rejoining: None,
            refusal,
            sandbox: None,
            kind,
            harness_version: None,
            wrapper_digest: None,
            persistent: true,
            confirms_from_locator: true,
            effort: None,
        }
    }
}

/// What an observed root did to a launch that was waiting for one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Confirmation {
    /// The provider named the exact root we offered: this is a rejoin.
    Resumed,
    /// A fresh session opened and named itself. Cold — even when the
    /// engine or adapter chose its identifier — and offerable next time.
    Fresh,
    /// The provider named a DIFFERENT root than the one we offered. No
    /// launch fact is published and no replacement is authorized: this
    /// attempt's outcome is whatever the invocation reports, and a
    /// different root is never relabelled as the requested session.
    Mismatch,
}

/// How an invocation ended with respect to the session it was meant to
/// rejoin. This is the fact `run_seat` needs and `Invocation` did not
/// carry: a rejoin that never confirmed — or that confirmed a DIFFERENT
/// root — must not be accepted as a successful seat merely because the
/// process exited clean and wrote a result file. The result came from a
/// session the engine did not offer, and D7 says a missing required
/// exact-root confirmation makes the attempt failed/indeterminate, never
/// a successful guessed rejoin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LaunchTerminal {
    /// No rejoin was attempted: an ordinary cold launch, including one
    /// whose fresh root the provider confirmed.
    Cold,
    /// A rejoin was attempted and provider evidence confirmed the exact
    /// offered root before first work.
    Resumed,
    /// A rejoin was attempted and the provider never named the offered
    /// root at all. The invocation's session is unknown.
    Unconfirmed,
    /// A rejoin was attempted and the provider named a different root.
    /// That root is never relabelled as the requested session.
    Mismatch,
}

impl LaunchTerminal {
    /// Whether the invocation's session is uncertain enough that no
    /// successful seat may be accepted from it.
    fn is_unsettled(self) -> bool {
        matches!(self, LaunchTerminal::Unconfirmed | LaunchTerminal::Mismatch)
    }
}

/// Holds a launch's candidate facts until the provider confirms a root,
/// and publishes exactly one launch row per executing model site.
///
/// The hold is the point of the whole type. Decision 0053 already
/// buffers the pre-session rows inside `run_seat`; this holds one level
/// further in, so that a row saying `resumed` never reaches that buffer
/// before the harness has said which session it is in.
struct LaunchHold {
    harness: &'static str,
    plan: LaunchPlan,
    published: bool,
    /// The root the provider actually named, once it has.
    confirmed: Option<RootSession>,
    /// The transcript address the confirmed root was opened at, for the
    /// launch row itself. A provider whose session identifier IS its
    /// locator (claude, codex) needs none: the root already carries the
    /// whole address. DSH needs both coordinates, so it supplies one.
    address: Option<Value>,
    outcome: Option<Confirmation>,
    /// Whether any row this invocation emitted proved the harness began
    /// work (decision 0053's own predicate). Ruling 8's replacement is
    /// only ever spent on an invocation for which this is false.
    began_work: bool,
}

impl LaunchHold {
    fn new(harness: &'static str, plan: LaunchPlan) -> LaunchHold {
        LaunchHold {
            harness,
            plan,
            published: false,
            confirmed: None,
            address: None,
            outcome: None,
            began_work: false,
        }
    }

    /// The address a root confirmed by this launch can be rejoined at:
    /// the exact transcript object the invocation admitted and recorded,
    /// handed over rather than composed a second time here.
    ///
    /// Design D6 wants the association ATOMIC. The engine reads a
    /// two-coordinate offer — the provider id, the persistence locator
    /// and the persistence home — off ONE checkpoint and never one field
    /// per row (`engine::resume::eligible_offer`), because two rows are
    /// two facts and a locator borrowed from a neighbour addresses a
    /// session nobody established. So the launch row that publishes
    /// `root_session` publishes the address beside it, and a launch that
    /// confirms nothing publishes neither.
    fn address(&mut self, transcript: Value) {
        self.address = Some(transcript);
    }

    /// The one door every emitted row passes through, so that the
    /// harness naming its own session is what publishes the launch —
    /// and publishes it BEFORE the first work row, where decision 0053's
    /// buffer will keep it in order.
    ///
    /// A provider names its session in exactly one place the driver
    /// already reads: decision 0032's transcript row, whose locator IS
    /// the session identifier for the two harnesses that report one. DSH
    /// records a retained DIRECTORY there, so its planner never sets a
    /// root kind that would let this door confirm anything.
    fn observe(&mut self, data: &Value, emit: &mut impl FnMut(&Value)) {
        emit(data);
        let step = data.get("step").and_then(Value::as_str).unwrap_or("");
        if begins_work(step) {
            self.began_work = true;
        }
        if self.outcome.is_some() || step != "transcript" || !self.plan.confirms_from_locator {
            return;
        }
        let id = data
            .pointer("/transcript/locator")
            .and_then(Value::as_str)
            .unwrap_or("");
        if !id.is_empty() {
            self.confirm(id, emit);
        }
    }

    /// The harness named its session. This is the ONLY door through
    /// which a launch fact is published.
    fn confirm(&mut self, id: &str, emit: &mut impl FnMut(&Value)) -> Confirmation {
        if let Some(outcome) = self.outcome {
            // The root is latched: a later, different root is a
            // delegated child or a second session, never a correction of
            // the first. Last-write-wins on a session identity is how a
            // subagent's id ends up offered as a seat's own.
            return outcome;
        }
        let outcome = match &self.plan.rejoining {
            Some(offered) if offered == id => Confirmation::Resumed,
            Some(_) => Confirmation::Mismatch,
            None => Confirmation::Fresh,
        };
        self.outcome = Some(outcome);
        if outcome == Confirmation::Mismatch {
            // Nothing is published: we do not know which session this
            // invocation is in, and a guess is what ruling 7 forbids.
            return outcome;
        }
        // A root is recorded only when every field v5 requires is a fact
        // this invocation actually observed. A missing or unrecordable
        // version leaves the root absent — the launch is still reported,
        // and the next retry simply gets no offer.
        self.confirmed = match (&self.plan.harness_version, recordable_session_id(id)) {
            (Some(version), true) if recordable_version(version) => Some(RootSession {
                kind: self.plan.kind,
                id: id.to_string(),
                harness_version: version.clone(),
                wrapper_digest: self.plan.wrapper_digest.clone(),
                persistent: self.plan.persistent,
            }),
            _ => None,
        };
        self.publish(outcome == Confirmation::Resumed, emit);
        outcome
    }

    /// The invocation is ending and no root was ever named. A known
    /// fresh path still reports `cold` — that is a fact about what this
    /// adapter did, not a claim about the provider — and supplies no
    /// resumable root. A rejoin that was never confirmed publishes
    /// NOTHING: an unconfirmed resume is uncertain, and neither `cold`
    /// nor `resumed` would be true of it.
    fn finish(&mut self, emit: &mut impl FnMut(&Value)) {
        // Every rejoin has already said what it had to say: a confirmed
        // one published on confirmation, and one that was never
        // confirmed — or that named a different root — publishes
        // nothing, because neither word would be true of it. A cold plan
        // falls through, and `publish`'s own once-guard is what keeps a
        // cold launch that already confirmed its fresh root from
        // reporting itself twice.
        if self.plan.rejoining.is_some() {
            return;
        }
        self.publish(false, emit);
    }

    fn publish(&mut self, resumed: bool, emit: &mut impl FnMut(&Value)) {
        if self.published {
            return;
        }
        self.published = true;
        let mut row = Map::new();
        row.insert("step".into(), Value::String("harness-started".into()));
        row.insert("harness".into(), Value::String(self.harness.to_string()));
        row.insert(
            "launch".into(),
            Value::String(if resumed { "resumed" } else { "cold" }.into()),
        );
        if let Some(sandbox) = &self.plan.sandbox {
            row.insert("sandbox".into(), Value::String(sandbox.clone()));
        }
        // The pin travels in the manifest, not here: `model` on a seat's
        // checkpoints means what SERVED (decision 0031). `effort` states
        // the applied configuration where the launch already knows it.
        if let Some(effort) = &self.plan.effort {
            row.insert("effort".into(), Value::String(effort.clone()));
        }
        if let Some(root) = &self.confirmed {
            row.insert("root_session".into(), root.value());
            // The address travels with the root it addresses, on this
            // row, or not at all: a confirmed launch whose locator and
            // home sit on a neighbouring row offers a planner that needs
            // all three coordinates nothing it may use, and the next
            // attempt declines an offer this one earned (design D6).
            if let Some(address) = &self.address {
                row.insert("transcript".into(), address.clone());
            }
        }
        // A refusal reason names a declined OFFER, so it appears exactly
        // beside the cold launch that decline produced. No offer, no
        // reason; a resumed launch, no reason.
        if !resumed {
            if let Some(refusal) = self.plan.refusal {
                row.insert("resume_refusal".into(), Value::String(refusal.into()));
            }
        }
        emit(&Value::Object(row));
    }

    /// Whether this launch tried to rejoin and never got its
    /// confirmation — the one shape that may spend ruling 8's single
    /// pre-work cold replacement, and only on top of the separate
    /// machine evidence that no session opened.
    fn unconfirmed_rejoin(&self) -> bool {
        self.plan.rejoining.is_some() && self.outcome.is_none()
    }

    /// The invocation's session outcome once the child has ended. A
    /// rejoin with no outcome and a rejoin that named a different root
    /// are both unsettled; only an exact match is a confirmed resume.
    fn terminal(&self) -> LaunchTerminal {
        if self.plan.rejoining.is_none() {
            return LaunchTerminal::Cold;
        }
        match self.outcome {
            Some(Confirmation::Resumed) => LaunchTerminal::Resumed,
            Some(Confirmation::Mismatch) => LaunchTerminal::Mismatch,
            // `Fresh` is unreachable for a rejoining plan (a fresh root is
            // only ever observed beside `rejoining: None`); treating it as
            // unconfirmed is the fail-closed reading if that ever changes.
            Some(Confirmation::Fresh) | None => LaunchTerminal::Unconfirmed,
        }
    }
}

/// What the engine's private start context says about resuming one named
/// execution shape of this adapter.
enum ResumeGate {
    /// Measured, enabled, and qualified against this installed version.
    Enabled { applies_to: String },
    /// Not enabled, with the bounded v5 token that says why.
    Disabled(&'static str),
}

/// Read the selected adapter's assessment out of the private start
/// context and decide whether THIS invocation may rejoin (proposed
/// decision 0056 ruling 5).
///
/// Every failure here is fail-closed and named: an absent assessment, an
/// unnamed shape and a non-`supported` status are all
/// `unsupported-resume`, while a shape whose measured boundary or hands
/// mode is not the one standing at this site is
/// `restrictions-unavailable` — the restrictions were measured
/// somewhere else, so they are not measured here.
fn resume_gate(input: &Value, shape: &str) -> ResumeGate {
    let Some(entry) = input.pointer(&format!("/resume_context/assessment/{shape}")) else {
        return ResumeGate::Disabled("unsupported-resume");
    };
    if entry.get("status").and_then(Value::as_str) != Some("supported") {
        return ResumeGate::Disabled("unsupported-resume");
    }
    // The accounting rule, enforced where it can actually be enforced
    // (proposed decision 0056 ruling 9). A rejoined invocation counts
    // only ITS OWN new turns, tools, targets and usage; whether a
    // provider's resumed stream replays the old ones is a fact about
    // that provider, and the only honest way to hold the rule without
    // that fact is not to rejoin at all. So a shape whose current-work
    // boundary has never been measured cannot be enabled — which is why
    // a `supported` entry names its accounting evidence beside its
    // interface, restriction and root evidence, and why the loader
    // refuses one that does not.
    //
    // Checked again here rather than trusted from the loader, because
    // this is the point of USE: an assessment that reached this input by
    // some other road must not be able to buy a rejoin whose totals
    // nobody can attribute.
    if entry
        .pointer("/evidence/accounting")
        .and_then(Value::as_str)
        .is_none()
    {
        return ResumeGate::Disabled("unsupported-resume");
    }
    let names = |key: &str, wanted: &str| {
        entry
            .get(key)
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(wanted)))
    };
    // The site's own facts, as the engine wrote them into this input:
    // the boundary that stands here (decision 0046 ruling 3) and whether
    // Brokkr built the box (decision 0043). The engine now writes BOTH
    // for every registered site — `not applicable`/`none` for a resolved
    // no-hands site, the boundary word and `boxed`/`none` for a hands
    // site — so a missing or non-string marker is not the engine's
    // statement of `none`: it means this site's confinement is UNKNOWN,
    // and unknown declines (design D10 F1). Absence must never enable.
    let Some(boundary) = input.get("boundary").and_then(Value::as_str) else {
        return ResumeGate::Disabled("restrictions-unavailable");
    };
    let Some(hands) = input.get("hands").and_then(Value::as_str) else {
        return ResumeGate::Disabled("restrictions-unavailable");
    };
    if !names("boundaries", boundary) || entry.get("hands").and_then(Value::as_str) != Some(hands) {
        return ResumeGate::Disabled("restrictions-unavailable");
    }
    match entry
        .pointer("/identity/applies_to")
        .and_then(Value::as_str)
    {
        Some(version) => ResumeGate::Enabled {
            applies_to: version.to_string(),
        },
        // A supported entry without a measured identity cannot load —
        // the adapter loader refuses it — so this arm is reached only by
        // data that never came through that loader.
        None => ResumeGate::Disabled("unverified-harness"),
    }
}

/// Ask the selected executable which version it is, once, through the
/// interface that was measured for it (proposed decision 0056 ruling 5).
///
/// Bounded on purpose: one short-lived child, its first line, no shell.
/// It runs ONLY where resume is actually live — an offer is in hand, or
/// the shape is enabled and a confirmed root is worth recording — so a
/// cold invocation of an unmeasured shape spawns exactly what it spawned
/// before this ruling.
fn observed_version(command: &[String]) -> Option<String> {
    let output = run_cli(command, None, "", &[]).ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    // The version token, not the banner around it. All three installed
    // CLIs put the number in a different place on the line, and the
    // three shapes are measured rather than guessed
    // (`.forge/controller-host-provider-interface.json`, 2026-09-09):
    //
    //   codex  --version  →  "codex-cli 0.153.4"
    //   claude --version  →  "2.1.266 (Claude Code)"
    //   dsh    --version  →  "0.1.2-rc.1"
    //
    // So the rule is the first token of the first non-empty line that
    // BEGINS WITH A DIGIT and fits the record's own grammar. That skips
    // a product name and stops before a parenthesised suffix, and a line
    // with no such token is read as no version at all — which disables
    // resume with `unverified-harness` rather than recording a banner.
    text.lines()
        .map(str::trim)
        .find(|line| !line.is_empty())?
        .split_whitespace()
        .find(|token| token.starts_with(|c: char| c.is_ascii_digit()) && recordable_version(token))
        .map(str::to_string)
}

/// What this invocation observed about its own harness, and whether
/// that observation lets it rejoin.
struct Qualification {
    /// The version actually observed, recorded on a confirmed root so a
    /// later invocation has something to compare against. Absent where
    /// no probe ran or the probe answered nothing readable.
    observed: Option<String>,
    /// The bounded reason resume is off for this invocation, or `None`
    /// when it is on.
    refusal: Option<&'static str>,
}

/// Qualify one invocation against its assessment and the installed CLI.
///
/// The probe runs only where the gate is already open, which is what
/// keeps an unmeasured shape's cold invocation spawning exactly what it
/// spawned before this ruling. A version that is missing, unreadable or
/// different from the one the assessment was measured against disables
/// resume; so does a version different from the one the ORIGINATING root
/// was opened under, because a session opened by another CLI is not a
/// session this one measured (proposed decision 0056 ruling 5).
fn qualify(gate: &ResumeGate, probe: &[String], originating: Option<&str>) -> Qualification {
    let applies_to = match gate {
        ResumeGate::Disabled(reason) => {
            return Qualification {
                observed: None,
                refusal: Some(reason),
            }
        }
        ResumeGate::Enabled { applies_to } => applies_to,
    };
    let Some(observed) = observed_version(probe) else {
        return Qualification {
            observed: None,
            refusal: Some("unverified-harness"),
        };
    };
    let drifted =
        &observed != applies_to || originating.is_some_and(|opened_under| opened_under != observed);
    Qualification {
        observed: Some(observed),
        refusal: drifted.then_some("unverified-harness"),
    }
}

/// Whether this attempt's seat actually delivered a result file.
///
/// The fourth term of ruling 8's replacement predicate, and the one that
/// keeps decision 0053 ruling 7's precedence: where a harness's machine
/// fields and the seat's own delivery disagree, the delivered work wins.
/// A session that exited clean with its result written is never
/// discarded and re-run because something else about the invocation
/// looked like a refusal.
fn delivered_result(input: &Value) -> bool {
    input
        .get("result_path")
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .and_then(|path| std::fs::metadata(path).ok())
        .is_some_and(|meta| meta.len() > 0)
}

/// The harness version the offered root was opened under, as the engine
/// read it back off the row the offer came from.
fn originating_harness_version(input: &Value) -> Option<&str> {
    input
        .pointer("/resume_context/originating_harness_version")
        .and_then(Value::as_str)
}

/// The composite digest the offered root was opened under, as the engine
/// read it back off the same row the offer came from. A DSH root recorded
/// without one can never be re-offered (task 8.8(d)).
fn originating_wrapper_digest(input: &Value) -> Option<&str> {
    input
        .pointer("/resume_context/originating_wrapper_digest")
        .and_then(Value::as_str)
}

/// A declared composite digest in seat-record v5's grammar: exactly 64
/// lowercase hexadecimal characters. A `supported` DSH shape that declares
/// anything else is refused by the loader, and this is the same check at
/// the point of use.
fn recordable_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

/// One line of wire text, whitespace-collapsed and clamped: every field
/// a refusal reason quotes comes off the harness's stream, so it is
/// arbitrary-length and may carry newlines. The journal is append-only
/// and every readout renders what lands in it, so a reason is bounded
/// here rather than trusted — the same discipline as the ≤80-char tool
/// names and targets and the 4000-byte stderr tail. Control characters
/// go with the newlines: a terminal escape belongs in a record no more
/// than a line break does.
fn bounded_wire_line(text: &str, limit: usize) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(limit)
        .collect()
}

/// The token's own bound. A machine token is a short identifier
/// (`rate_limit`, `authentication_error`); the checkpoint bound for a
/// harness-supplied name governs it too.
const REFUSAL_TOKEN_LIMIT: usize = 80;

/// And the excerpt's, which may be a sentence.
const REFUSAL_TEXT_LIMIT: usize = 160;

/// Bound one refusal reason for the journal: a single line, clamped, and
/// never empty. The reason is the provider's own machine token when it
/// gave one, the HTTP status when it gave that, and a short excerpt of
/// its prose when it gave that — the same evidence a stderr tail already
/// carries on a failed attempt, and nothing executable is derived from it.
///
/// The token is bounded exactly like the excerpt. It is read from the
/// harness's stream, so "the provider's own token" is only ever a claim
/// about where the bytes came from: a harness emitting a 200KB
/// newline-bearing `error` must not put that in a durable record.
fn refusal_reason(token: &str, status: Option<i64>, text: Option<&str>) -> String {
    let token = bounded_wire_line(token, REFUSAL_TOKEN_LIMIT);
    // Never empty: a refusal with no token at all is still a refusal,
    // and an empty one would leave the line dangling after its colon.
    let token = if token.is_empty() {
        "api_error"
    } else {
        &token
    };
    let mut reason = format!("provider refused before the first turn: {token}");
    if let Some(status) = status {
        reason.push_str(&format!(" (HTTP {status})"));
    }
    if let Some(text) = text {
        let excerpt = bounded_wire_line(text, REFUSAL_TEXT_LIMIT);
        if !excerpt.is_empty() {
            reason.push_str(": ");
            reason.push_str(&excerpt);
        }
    }
    reason
}

/// The claude (and, byte-for-byte, lanetally) stream-json record of a
/// provider refusal. Three measured shapes, all meaning the request was
/// rejected before any inference (decision 0053):
///
/// - the synthetic assistant message #68816 records, carrying
///   `isApiErrorMessage: true` and an `error` token (`rate_limit`,
///   `authentication_error`, …) beside an ordinary terminal
///   `stop_reason: "stop_sequence"` — the shape the #219 run measured,
///   and the reason a `type` test alone cannot see it;
/// - the error `result` #79500 records, `is_error: true` with the prose
///   in `result` and no turn behind it;
/// - the `rate_limit_event` lifecycle message newer CLIs emit.
///
/// A record with none of those fields is an ordinary turn and is never
/// read here; nothing in this classifier looks at a model's prose.
fn claude_refusal(event: &Value) -> Option<String> {
    match event.get("type").and_then(Value::as_str) {
        Some("rate_limit_event") => Some(refusal_reason(
            "rate_limit",
            event.get("apiErrorStatus").and_then(Value::as_i64),
            event
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| event.get("error").and_then(Value::as_str)),
        )),
        Some("assistant") => {
            let api_error = event
                .get("isApiErrorMessage")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || event.get("error").and_then(Value::as_str).is_some();
            api_error.then(|| {
                refusal_reason(
                    event
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("api_error"),
                    event.get("apiErrorStatus").and_then(Value::as_i64),
                    event
                        .pointer("/message/content/0/text")
                        .and_then(Value::as_str),
                )
            })
        }
        Some("result") => {
            let is_error = event
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || event.get("error").and_then(Value::as_str).is_some();
            is_error.then(|| {
                refusal_reason(
                    event
                        .get("error")
                        .and_then(Value::as_str)
                        .unwrap_or("api_error"),
                    event.get("apiErrorStatus").and_then(Value::as_i64),
                    event.get("result").and_then(Value::as_str),
                )
            })
        }
        _ => None,
    }
}

/// The codex `exec --json` record of a pre-session refusal: the harness's
/// own machine-readable `error` event, or a `turn.failed` that arrived
/// before any `turn.started`. Codex puts the provider's message in
/// `message` on the one and under `error.message` on the other; neither
/// is model prose, and a record with any other type is never read here.
fn codex_refusal(event: &Value) -> Option<String> {
    match event.get("type").and_then(Value::as_str) {
        Some("error") => Some(refusal_reason(
            "api_error",
            None,
            event
                .get("message")
                .and_then(Value::as_str)
                .or_else(|| event.pointer("/error/message").and_then(Value::as_str)),
        )),
        Some("turn.failed") => Some(refusal_reason(
            "api_error",
            None,
            event
                .pointer("/error/message")
                .and_then(Value::as_str)
                .or_else(|| event.get("message").and_then(Value::as_str)),
        )),
        _ => None,
    }
}

/// One parsed claude stream-json line folded into the seat's telemetry:
/// `system`/`init` and `result` feed `session_meta`; each `tool_use`
/// block of an `assistant` message becomes one seat-turn checkpoint,
/// in block order, all carrying the message's turn number. The first
/// checkpoint for a message carries that message's usage; a text-only
/// message still emits one checkpoint, so usage and progress never
/// disappear merely because the model called no tool.
/// Privacy invariant (journal is evidence, not transcript): checkpoints
/// carry turn index, a ≤80-char tool name, and a ≤80-char target only —
/// never message text, thinking, or full tool inputs.
fn fold_stream_event(
    event: &Value,
    assistant_turns: &mut u64,
    session_meta: &mut Map<String, Value>,
    transcript: &mut Transcript,
    emit: &mut impl FnMut(&Value),
) -> Option<String> {
    // Decision 0053: a provider refusal is not a turn. It is classified
    // ONLY before the first real turn; once one has been counted the
    // session has opened and decision 0016's mid-session boundary holds
    // unchanged. A refusal record emits no checkpoint and no usage, so
    // the structural fail-to-start predicate can see it.
    if *assistant_turns == 0 {
        if let Some(reason) = claude_refusal(event) {
            return Some(reason);
        }
    }
    match event.get("type").and_then(Value::as_str) {
        Some("system") if event.get("subtype").and_then(Value::as_str) == Some("init") => {
            if let Some(session_id) = event.get("session_id").and_then(Value::as_str) {
                // Recorded NOW, not only at session end: the transcript
                // drilldowns can only locate — and live-stream — a
                // WORKING seat's prose if the id is known from the first
                // message. Decision 0032's shared module clamps and
                // shapes the one transcript row. Since decision 0053 the
                // row reaches the JOURNAL one beat later: `run_seat`
                // buffers it until a checkpoint proves work began,
                // because a checkpoint before that would put a refused
                // attempt on the mid-session side of decision 0016's
                // boundary. A refused attempt keeps the locator in its
                // failure reason instead.
                transcript.record(session_id, session_meta, emit);
            }
        }
        Some("assistant") => {
            *assistant_turns += 1;
            session_meta.insert("num_turns".into(), Value::from(*assistant_turns));
            let model = model_in_json(event);
            if let Some(model) = &model {
                session_meta.insert("model".into(), Value::String(model.clone()));
            }
            // The harness's own echo, beside this assistant record and
            // not from our pin (decision 0035 ruling 3). Claude writes it
            // per record, so a thread that changes effort mid-seat says
            // so turn by turn; the last one seen is the seat's.
            let effort = effort_in_json(event);
            if let Some(effort) = &effort {
                session_meta.insert("effort".into(), Value::String(effort.clone()));
            }
            let usage = claude_usage(event.pointer("/message/usage"));
            for (key, value) in &usage {
                add_usage(
                    session_meta,
                    key,
                    value.as_u64().expect("claude usage is a positive integer"),
                );
            }
            let mut base = Map::new();
            base.insert("step".into(), Value::String("seat-turn".into()));
            base.insert("turn".into(), Value::from(*assistant_turns));
            base.insert(
                "model".into(),
                Value::String(
                    model
                        .clone()
                        .unwrap_or_else(|| MODEL_NOT_REPORTED.to_string()),
                ),
            );
            base.insert(
                "effort".into(),
                Value::String(
                    effort
                        .clone()
                        .unwrap_or_else(|| EFFORT_NOT_REPORTED.to_string()),
                ),
            );
            // No `reasoning_output_tokens` here, and that absence is the
            // ruling: claude reports its thinking tokens ONLY in the
            // result (`output_tokens_details.thinking_tokens`), so a
            // per-turn figure would have to be invented. Decision 0035
            // ruling 4 says absent, never zero and never back-filled
            // from the run total.
            base.extend(usage);
            let blocks = event.pointer("/message/content").and_then(Value::as_array);
            let tool_uses = blocks
                .into_iter()
                .flatten()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("tool_use"));
            let mut emitted = false;
            for tool_use in tool_uses {
                let mut checkpoint = base.clone();
                if emitted {
                    for key in USAGE_FIELDS {
                        checkpoint.remove(key);
                    }
                }
                let tool = tool_use.get("name").and_then(Value::as_str).unwrap_or("");
                if !tool.is_empty() {
                    checkpoint.insert(
                        "tool".into(),
                        Value::String(tool.chars().take(80).collect()),
                    );
                }
                // file_path ONLY: commands and URLs can embed inline secrets,
                // and the journal is append-only — the verification review
                // hard-stopped on exactly this (run verify-…-917996f5). Full
                // detail belongs to the resumable transcript, not the record.
                let target = tool_use.pointer("/input/file_path").and_then(Value::as_str);
                if let Some(target) = target {
                    checkpoint.insert(
                        "target".into(),
                        Value::String(target.chars().take(80).collect()),
                    );
                }
                emit(&Value::Object(checkpoint));
                emitted = true;
            }
            if !emitted {
                emit(&Value::Object(base));
            }
        }
        Some("result") => {
            if let Some(model) = model_in_json(event) {
                session_meta.insert("model".into(), Value::String(model));
            }
            if let Some(effort) = effort_in_json(event) {
                session_meta.insert("effort".into(), Value::String(effort));
            }
            if let Some(session_id) = event.get("session_id").and_then(Value::as_str) {
                transcript.record(session_id, session_meta, emit);
            }
            if let Some(value) = positive_count(event.get("num_turns")) {
                session_meta.insert("num_turns".into(), Value::from(value));
            }
            if let Some(value) = positive_cost(event.get("total_cost_usd")) {
                session_meta.insert("total_cost_usd".into(), Value::from(value));
            }
            // Claude's result usage is the harness's session aggregate,
            // so it replaces the live sum rather than being added to it.
            for (key, value) in claude_usage(event.get("usage")) {
                session_meta.insert(key, value);
            }
            // The one place claude reports what it spent thinking. It is
            // a session figure, so it lands on the session record only —
            // the turns above stay silent about it rather than each
            // claiming a share nobody measured (decision 0035 ruling 4).
            if let Some(value) =
                positive_count(event.pointer("/usage/output_tokens_details/thinking_tokens"))
            {
                session_meta.insert("reasoning_output_tokens".into(), Value::from(value));
            }
        }
        _ => {}
    }
    None
}

/// How deep below `<codex home>/sessions` the thread record may sit.
/// Codex files a rollout under a dated `YYYY/MM/DD` path today; the
/// walk is bounded rather than pinned to that shape so a re-filing does
/// not silently stop reporting effort, and bounded rather than
/// unlimited so a large harness home cannot turn one turn into a full
/// filesystem scan.
const CODEX_THREAD_DEPTH: usize = 6;

/// The codex thread record, followed to read the one fact codex does
/// NOT put on the stream its adapter folds: the effort it applied.
///
/// Decision 0032 already retains the locator that reaches this file, so
/// decision 0035 needed no new mechanism — only for something to open
/// it. The file is found by the thread id codex itself announced, never
/// by "the newest file" under the home: a concurrent seat's thread must
/// not be able to lend this one its effort.
///
/// Same trust boundary as the dsh tail, and answered the same way: the
/// value taken is one clamped word, nothing executable is derived from
/// it, and a file that cannot be read or does not name an effort leaves
/// the record saying `not reported` rather than guessing.
#[derive(Default)]
struct CodexThreadEcho {
    /// The sessions root and the thread id codex announced, held
    /// together because neither is ever known without the other.
    announced: Option<(std::path::PathBuf, String)>,
    path: Option<std::path::PathBuf>,
}

impl CodexThreadEcho {
    /// Remember the thread codex announced, and try the walk now. The
    /// id is held whether or not the file is there yet, because
    /// announcing a thread and filing its rollout are two writes and
    /// codex does not promise the order.
    fn locate(&mut self, home: &std::path::Path, thread_id: &str) {
        // The same clamp the resume argv applies (`plain_thread_id`),
        // applied on this path too: the id arrives from the harness's
        // own stream and decides which file is read, so a spelling that
        // is not an id locates nothing rather than matching whatever it
        // happens to be a fragment of.
        if !plain_thread_id(thread_id) {
            return;
        }
        self.announced = Some((home.join("sessions"), thread_id.to_string()));
        self.resolve();
    }

    /// The walk, retried while it has not yet succeeded. Codex announces
    /// `thread.started` before it necessarily has the rollout on disk;
    /// resolving once and caching the miss would leave a codex seat
    /// saying `not reported` for its whole life — dsh's sentinel, on a
    /// harness that does echo its effort, which is exactly the
    /// distinction ruling 3 makes load-bearing.
    fn resolve(&mut self) {
        if self.path.is_some() {
            return;
        }
        let Some((sessions, thread_id)) = &self.announced else {
            return;
        };
        self.path = find_codex_thread(sessions, thread_id, CODEX_THREAD_DEPTH);
    }

    /// The model and effort the thread record LAST named, as one pair
    /// off one read. Codex writes a `turn_context` per turn, so
    /// re-reading here is what makes a thread that changed either of
    /// them mid-seat say so turn by turn.
    ///
    /// The model is here for the same reason the effort is: codex's
    /// `exec --json` stream does not carry it (0.153 puts only `usage`
    /// on `turn.completed`), and the value is sitting in the record
    /// decision 0032's locator already names. Reading it is not
    /// substituting our pin — this is codex's own echo of what served,
    /// which is exactly what decision 0031 ruling 1 asks the record to
    /// carry and what ruling 3 refuses to guess.
    fn echo(&mut self) -> (Option<String>, Option<String>) {
        self.resolve();
        let Some(body) = self
            .path
            .as_ref()
            .and_then(|path| std::fs::read_to_string(path).ok())
        else {
            return (None, None);
        };
        // The NEWEST record that names either names both: a rollout's
        // `turn_context` and its `thread_settings_applied` each carry
        // the model beside the effort, and nothing in between carries
        // one alone. Taking the pair off ONE record is what keeps the
        // two from being spliced together out of different turns, and
        // in every rollout codex actually writes that is what happens:
        // the walk stops at the first record it reaches.
        //
        // A field that record cannot ANSWER is the one exception, and it
        // has a single cause: the clamp above refused the value. That
        // clamp is ours, not codex's shape — `_high` is the case its own
        // doc names — so halting there would record `not reported` for a
        // level the thread spells plainly one record up, and would spell
        // it the same way as a harness that echoes nothing at all, which
        // is the distinction ruling 3 makes load-bearing. So a field the
        // anchor leaves empty keeps walking for the newest record that
        // can answer it. The pair still binds every field one record
        // answers; only a refusal splits it.
        let mut model = None;
        let mut effort = None;
        for line in body.lines().rev() {
            if model.is_some() && effort.is_some() {
                break;
            }
            if !names_thread_settings(line) {
                continue;
            }
            let Ok(line) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            if model.is_none() {
                model = model_in_thread(&line);
            }
            if effort.is_none() {
                effort = effort_in_thread(&line);
            }
        }
        (model, effort)
    }
}

/// `name` carries `thread_id` as a WHOLE token — bounded at both ends by
/// a non-alphanumeric or by the ends of the name — rather than merely as
/// a substring. A bare `contains` would let a short or degenerate id
/// match a longer id's file, which is the "never by the newest file"
/// rule lost by another route: a fragment is not an announcement.
fn names_codex_thread(name: &str, thread_id: &str) -> bool {
    name.match_indices(thread_id).any(|(at, _)| {
        let before = name[..at].chars().next_back();
        let after = name[at + thread_id.len()..].chars().next();
        !before.is_some_and(|c| c.is_ascii_alphanumeric())
            && !after.is_some_and(|c| c.is_ascii_alphanumeric())
    })
}

/// The thread record whose file name carries `thread_id`, anywhere in a
/// bounded walk below `root`.
fn find_codex_thread(
    root: &std::path::Path,
    thread_id: &str,
    depth: usize,
) -> Option<std::path::PathBuf> {
    let mut directories = Vec::new();
    for entry in std::fs::read_dir(root).ok()?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            directories.push(path);
            continue;
        }
        let named = path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.ends_with(".jsonl") && names_codex_thread(name, thread_id));
        if named {
            return Some(path);
        }
    }
    if depth == 0 {
        return None;
    }
    // Deterministic order: two seats under one home must not disagree
    // about which file a shared prefix names.
    directories.sort();
    directories
        .into_iter()
        .find_map(|dir| find_codex_thread(&dir, thread_id, depth - 1))
}

/// Fold Codex's stable `exec --json` JSONL into bounded live telemetry.
/// Commands, item bodies, model output, reasoning, and filesystem paths are
/// intentionally ignored; the journal records progress and usage, not a
/// transcript.
fn fold_codex_event(
    event: &Value,
    turn: &mut u64,
    session_meta: &mut Map<String, Value>,
    transcript: &mut Transcript,
    echo: &mut CodexThreadEcho,
    emit: &mut impl FnMut(&Value),
) -> Option<String> {
    // Decision 0053: an error codex reported before its first `turn.started`
    // is a refusal to start, not a turn. After the first turn the boundary
    // is decision 0016's, unchanged.
    if *turn == 0 {
        if let Some(reason) = codex_refusal(event) {
            return Some(reason);
        }
    }
    match event.get("type").and_then(Value::as_str) {
        Some("thread.started") => {
            if let Some(thread_id) = event.get("thread_id").and_then(Value::as_str) {
                echo.locate(transcript.home(), thread_id);
                // Recorded NOW, as the claude fold records its own —
                // and for a second reason here: the thread id is what a
                // retry resumes (decision 0030), and an attempt killed
                // on its deadline never reaches the session-finished
                // checkpoint. Captured at `thread.started`, the id is in
                // hand from the harness's first message. It reaches the
                // JOURNAL with the attempt's first work checkpoint,
                // which `run_seat` flushes it ahead of: a pre-session
                // row would put a refused attempt on decision 0016's
                // mid-session side. So a kill AFTER the first turn still
                // hands its thread to the retry that follows, and a kill
                // before it has none to hand — decision 0053 ruling 8,
                // which names that window and its cost.
                transcript.record(thread_id, session_meta, emit);
            }
        }
        Some("turn.started") => {
            *turn += 1;
            emit(&json!({"step":"turn-started", "turn": *turn, "harness":"codex"}));
        }
        Some(kind @ ("item.started" | "item.completed")) => {
            let item_type = event
                .pointer("/item/type")
                .and_then(Value::as_str)
                .unwrap_or("unknown");
            emit(&json!({
                "step": if kind == "item.started" { "item-started" } else { "item-completed" },
                "turn": *turn,
                "tool": item_type.chars().take(80).collect::<String>(),
                "harness":"codex",
            }));
        }
        Some("turn.completed") => {
            let usage = event.get("usage").unwrap_or(&Value::Null);
            // Neither fact is reliably on this stream, and neither is
            // ever taken from our pin: both are read from the thread
            // record codex writes them into (decision 0035 ruling 3),
            // through decision 0032's retained locator. The stream is
            // still asked first for the model — a release that does put
            // it on `turn.completed` is the more direct report.
            let (thread_model, effort) = echo.echo();
            let model = model_in_json(event).or(thread_model);
            if let Some(model) = &model {
                session_meta.insert("model".into(), Value::String(model.clone()));
            }
            if let Some(effort) = &effort {
                session_meta.insert("effort".into(), Value::String(effort.clone()));
            }
            let mut checkpoint = Map::new();
            checkpoint.insert("step".into(), Value::String("turn-completed".into()));
            checkpoint.insert("turn".into(), Value::from(*turn));
            checkpoint.insert("harness".into(), Value::String("codex".into()));
            checkpoint.insert(
                "model".into(),
                Value::String(model.unwrap_or_else(|| MODEL_NOT_REPORTED.to_string())),
            );
            checkpoint.insert(
                "effort".into(),
                Value::String(effort.unwrap_or_else(|| EFFORT_NOT_REPORTED.to_string())),
            );
            for (source, target) in [
                ("input_tokens", "input_tokens"),
                ("cached_input_tokens", "cache_read_tokens"),
                ("output_tokens", "output_tokens"),
                // Received all along and dropped until decision 0035
                // ruling 4 asked for it; `cache_write_tokens` is a v1
                // field no codex record has ever filled.
                ("cache_write_input_tokens", "cache_write_tokens"),
                // Codex is the one built-in that meters its reasoning
                // per turn. It is a SUBSET of output_tokens above, so it
                // rides its own key and is never added to a total again.
                ("reasoning_output_tokens", "reasoning_output_tokens"),
            ] {
                if let Some(value) = positive_count(usage.get(source)) {
                    checkpoint.insert(target.into(), Value::from(value));
                    add_usage(session_meta, target, value);
                }
            }
            // The turn count is journaled HERE, not only from `result`.
            // Verified against codex-cli 0.148.0: a real `codex exec
            // --json` run ends at `turn.completed` and emits no `result`
            // event at all, so a codex seat's num_turns never reached
            // `brokkr costs` — the aggregator saw per-turn token keys it
            // does not sum and nothing else. Cost in USD is reported
            // nowhere in that stream (`usage` is token counts only), so
            // total_cost_usd stays absent rather than invented.
            session_meta.insert("num_turns".into(), Value::from(*turn));
            emit(&Value::Object(checkpoint));
        }
        // Conformance shims and older clients may provide only final metadata.
        Some("result") => {
            if let Some(model) = model_in_json(event) {
                session_meta.insert("model".into(), Value::String(model));
            }
            if let Some(effort) = effort_in_json(event) {
                session_meta.insert("effort".into(), Value::String(effort));
            }
            if let Some(session_id) = event.get("session_id").and_then(Value::as_str) {
                transcript.record(session_id, session_meta, emit);
            }
            if let Some(value) = positive_count(event.get("num_turns")) {
                session_meta.insert("num_turns".into(), Value::from(value));
            }
            if let Some(value) = positive_cost(event.get("total_cost_usd")) {
                session_meta.insert("total_cost_usd".into(), Value::from(value));
            }
        }
        _ => {}
    }
    None
}

/// One harness-reported token count folded into the session totals.
/// Usage ACCUMULATES: a plain insert left a multi-turn session's meta
/// holding only its LAST turn's counts, which is the number every cost
/// surface then read as the session's.
fn add_usage(session_meta: &mut Map<String, Value>, key: &str, value: u64) {
    let total = session_meta
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(0)
        .saturating_add(value);
    session_meta.insert(key.to_string(), Value::from(total));
}

/// The dsh usage keys, as `@deepseek-ai/dsh-llm`'s `TokenUsage` spells
/// them, paired with the journal's own names.
///
/// dsh's own counts are DISJOINT: a cache read is NOT part of
/// `inputTokens`. Verified by arithmetic against 0.1.0-rc.6 — a step
/// whose provider reported `prompt_tokens: 101` with `cached_tokens: 7`
/// was written to the transcript as `inputTokens: 94, cacheReadTokens:
/// 7`, and the next step's 103/21 as 82/21.
///
/// The journal's `input_tokens` is INCLUSIVE, because that is what the
/// key already meant everywhere else: codex reports `input_tokens:
/// 14830` beside `cached_input_tokens: 11264` (codex-cli 0.148.0, one
/// real `codex exec --json` turn), and `brokkr-view::session_tokens`
/// sums `input_tokens` and `output_tokens` *only*, documenting that a
/// cache read arrives inside the input count and adding it again would
/// double-count. So `DSH_INPUT_CACHE_READ` is folded back into the
/// input count below rather than leaving one journal key meaning one
/// thing for codex and another for dsh.
///
/// dsh 0.1.2-rc.1 (re-measured 2026-09-04, one real headless session)
/// adds `totalTokens`, `reasoningTokens` and a zero `cacheReadTokens` to
/// the same object. The total is a sum the journal never stores twice;
/// the reasoning count is decision 0035's `reasoning_output_tokens`, a
/// reported subset of `output_tokens`, and `positive_count` keeps a
/// reported zero out of the record as the contract requires.
const DSH_USAGE: [(&str, &str); 4] = [
    ("inputTokens", "input_tokens"),
    ("outputTokens", "output_tokens"),
    (DSH_INPUT_CACHE_READ, "cache_read_tokens"),
    ("reasoningTokens", "reasoning_output_tokens"),
];

/// The dsh count that is a subset of the journal's `input_tokens` but a
/// sibling of dsh's own `inputTokens`.
const DSH_INPUT_CACHE_READ: &str = "cacheReadTokens";

/// The level the finishing record carries for a non-exec seat.
/// Harness-written values cross the boundary clamp exactly as before —
/// every fold inserts them token-checked, so an echo that fails the
/// shape still reads `not reported`. The one value that never crossed
/// a boundary is the driver's own seed: a source-literal constant
/// (decision 0035 addendum 2026-09-11), matched literally rather than
/// clamped, because the clamp's alphabet has no space and the sentinel
/// does. Matching it by value cannot admit a harness string no fold
/// would have written.
fn applied_harness_effort(session_meta: &Map<String, Value>) -> String {
    match session_meta.get("effort").and_then(Value::as_str) {
        Some(effort) if effort == EFFORT_NOT_APPLICABLE => EFFORT_NOT_APPLICABLE.to_string(),
        Some(effort) => effort_token(effort).unwrap_or_else(|| EFFORT_NOT_REPORTED.to_string()),
        None => EFFORT_NOT_REPORTED.to_string(),
    }
}

/// The level dsh's request header last echoed for this seat, or the
/// sentinel for a row written before any header did.
fn dsh_echoed_effort(session_meta: &Map<String, Value>) -> String {
    session_meta
        .get("effort")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| EFFORT_NOT_REPORTED.to_string())
}

/// One dsh session-log line folded into bounded live telemetry.
///
/// dsh 0.1.0-rc.6's headless profile offers no machine-readable stdout
/// stream (unchanged in 0.1.2-rc.1, re-measured 2026-09-04: the same
/// launcher flags, and headless gained only "stream reasoning to
/// stderr", which `redact_dsh_reasoning` keeps out of the journal) —
/// verified against the installed binary: `dsh --help` lists
/// `-V/--profile/--patch/--dump-config/--dump-default-config` and the
/// `web`/`plugin` commands, none of them an output format, and `dsh
/// --profile headless --help` lists `-h` alone under "Answer one task,
/// print the final assistant message, and exit". The live signal is
/// therefore the JSONL transcript
/// `@deepseek-ai/dsh-session-persistence-jsonl` appends as the session
/// runs (observed growing 14 → 45 → 71 → 89 lines while the child was
/// still alive).
///
/// The log's first line is the immutable session header, which names the
/// session; every later line is one `SessionEvent`. One real session,
/// driven end to end through 0.1.0-rc.6 with a tool call in it, wrote:
///
/// ```text
/// session · permission/preset · sandbox/mode · approval/policy
/// agent/inbox/spliced · turn/start · step/start · user/message
/// request/header · request/context · assistant/chunk × 9
/// assistant/message · tool/call · tool/result · step/end
/// step/start · assistant/chunk × 5 · assistant/message · step/end
/// turn/end
/// ```
///
/// So an `assistant/message` is one assembled assistant *step*, not one
/// dsh turn: dsh authors its own coarser `data.turn`, and the two
/// messages above both carry `turn: 1` with `step: 1` and `step: 2`. A
/// headless seat answers one task, so dsh's own turn index never leaves
/// 1 and would report no progress at all. The step is the unit that
/// advances, and it is the same unit the claude fold counts — one
/// assembled assistant message per model response. It carries that
/// step's `usage` when the adapter reported accounting; a `tool/call`
/// names the tool the step asked for.
///
/// Privacy invariant, same as `fold_stream_event`: a turn index, a
/// ≤80-char tool name and token counts only — never message text,
/// reasoning, or tool arguments. dsh's `arguments` is the model's own
/// unparsed JSON string, so no target is derived from it at all.
fn fold_dsh_event(
    event: &Value,
    first_seq: Option<u64>,
    turns: &mut u64,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
) {
    // Current-only accounting (design D8; task 9.6): a resumed session's
    // file holds its restored history beside the new work, and only
    // events past the owned pre-followup sequence are this invocation's.
    // The boundary exists only when a root was rejoined. A cold launch
    // carries none, so its file is folded from its first event: dsh's
    // log index IS the sequence and starts at 0, and a cold boundary of
    // 0 would silently drop that first event on the shipped route.
    if let Some(first_seq) = first_seq {
        if event
            .get("seq")
            .and_then(Value::as_u64)
            .is_some_and(|seq| seq <= first_seq)
        {
            return;
        }
    }
    match event.get("type").and_then(Value::as_str) {
        Some("session") => {
            // The dsh locator is the retained seat root, known and
            // journaled before spawn. Its internal session id is not a
            // second locator shape (decision 0032).
        }
        Some("request/header") => {
            // The harness's own echo of the level it applied: the
            // request it is about to send, after every profile, plugin
            // and settings layer (decision 0035 ruling 3, for the lane
            // its addendum brought in). Measured on 0.1.2-rc.1: the
            // header's `config` carries `reasoningEffort` when a level
            // applies and omits it when none does, so an absent field
            // leaves the seat saying `not reported`, honestly — for a
            // seat that pinned one. A seat that arrived with no pin
            // carries invoke_dsh_with's seed (`not applicable`, by the
            // compile law that only effortless routes compile pin-less),
            // which an absent field leaves standing.
            if let Some(effort) = event
                .pointer("/data/header/config/reasoningEffort")
                .and_then(Value::as_str)
                .and_then(effort_token)
            {
                session_meta.insert("effort".into(), Value::String(effort));
            }
        }
        Some("assistant/message") => {
            *turns += 1;
            session_meta.insert("num_turns".into(), Value::from(*turns));
            // The served model, from the harness's own record of the
            // response (`data.message.source.model`), never from the pin
            // (decision 0031): the last reported one becomes the seat's
            // served model, and a step that names none says so.
            let model = model_in_json(event);
            if let Some(model) = &model {
                session_meta.insert("model".into(), Value::String(model.clone()));
            }
            let mut checkpoint = Map::new();
            checkpoint.insert("step".into(), Value::String("seat-turn".into()));
            checkpoint.insert("turn".into(), Value::from(*turns));
            checkpoint.insert("harness".into(), Value::String("deepseek".into()));
            checkpoint.insert(
                "model".into(),
                Value::String(model.unwrap_or_else(|| MODEL_NOT_REPORTED.to_string())),
            );
            // The last level the header echoed, on this step as on
            // every other row (decision 0035 ruling 3).
            checkpoint.insert(
                "effort".into(),
                Value::String(dsh_echoed_effort(session_meta)),
            );
            let usage = event.pointer("/data/usage").unwrap_or(&Value::Null);
            let cache_read = usage
                .get(DSH_INPUT_CACHE_READ)
                .and_then(Value::as_u64)
                .unwrap_or(0);
            for (source, target) in DSH_USAGE {
                if let Some(mut value) = positive_count(usage.get(source)) {
                    // The one place dsh's disjoint accounting is
                    // reconciled with the journal's inclusive
                    // `input_tokens` — see DSH_USAGE.
                    if source == "inputTokens" {
                        value = value.saturating_add(cache_read);
                    }
                    checkpoint.insert(target.into(), Value::from(value));
                    add_usage(session_meta, target, value);
                }
            }
            emit(&Value::Object(checkpoint));
        }
        Some("tool/call") => {
            let Some(tool) = event
                .pointer("/data/name")
                .and_then(Value::as_str)
                .filter(|tool| !tool.is_empty())
            else {
                return;
            };
            if *turns == 0 {
                return;
            }
            let model = session_meta
                .get("model")
                .and_then(Value::as_str)
                .unwrap_or(MODEL_NOT_REPORTED);
            emit(&json!({
                "step":"seat-turn",
                "turn": *turns,
                "harness":"deepseek",
                "model": model,
                "effort": dsh_echoed_effort(session_meta),
                "tool": tool.chars().take(80).collect::<String>(),
            }));
        }
        _ => {}
    }
}

/// The fixed transcript filename the selected JSONL backend writes inside
/// each session-owned directory
/// (`<root>/--<cwd>--/<id>/session.v3.jsonl`).
///
/// The selected `0.1.5-rc.1` core sets `SESSION_FORMAT_VERSION = 3`, so
/// `sessionFormatLogFilename(3)` is `session.v3.jsonl`; the seat overlay
/// disables compression (`compression: none`), which changes only the
/// suffix, so the plaintext artifact keeps that generation name (design
/// D6; task 8.8(d)).
const DSH_TRANSCRIPT: &str = "session.v3.jsonl";

/// The generation name the previously shipped, plugin-free core wrote.
///
/// Admitting version three does NOT retire it. A shipped cold launch —
/// one the qualification gate disabled, or one whose composite did not
/// match the declared identity — still runs whatever core the host has
/// installed, and that core may be the one that writes this name. AS1
/// requires the shipped cold route to keep the telemetry it already had,
/// so discovery reads both generations while the strict warm admission
/// below keeps naming `DSH_TRANSCRIPT` alone.
const DSH_TRANSCRIPT_SHIPPED: &str = "session.jsonl";

/// Both generations, newest first: a session directory holds one of them,
/// and the selected core's name is the likelier answer, so it is asked
/// first. This is a fixed two-name list and never a directory scan.
const DSH_TRANSCRIPT_NAMES: [&str; 2] = [DSH_TRANSCRIPT, DSH_TRANSCRIPT_SHIPPED];

/// The existing transcript locator bound (Rust `chars`, not UTF-8 bytes),
/// repeated here so the DSH planner can validate an OFFERED locator and the
/// locator it plans before the shared `Transcript::record` clamp can turn
/// one address into another (design D6; task 8.8(d)).
const DSH_LOCATOR_LIMIT: usize = 80;

/// Bounded pre-spawn admission IO: one session header line, one stored
/// session file and one retained-root enumeration each have a finite
/// DSH-local budget, so a malformed store cannot force an unbounded
/// allocation before the provider starts (design D6; task 8.8(d)). A
/// truncated or over-budget boundary refuses the offer rather than
/// degrading to a zero or a partial maximum.
const DSH_HEADER_LIMIT: u64 = 4096;
const DSH_SESSION_FILE_LIMIT: u64 = 64 * 1024 * 1024;
const DSH_DIRECTORY_ENTRIES: usize = 4096;

/// The seat's own transcript under a per-seat root. NEVER "the newest
/// file": the root is fresh and belongs to this invocation alone, so no
/// directory scan can lose a race against a concurrent seat.
///
/// One root is not the same claim as one session, though. dsh's session
/// header carries a `delegationDepth`, so a session this seat delegates
/// writes a SECOND transcript under the same root, and `read_dir` yields
/// entries in whatever order the filesystem likes. The seat's own
/// session is therefore chosen by what its header says — depth 0 — and
/// not by which entry happened to come back first.
fn find_dsh_transcript(root: &std::path::Path) -> Option<std::path::PathBuf> {
    for project in std::fs::read_dir(root).ok()?.flatten() {
        for session in std::fs::read_dir(project.path())
            .into_iter()
            .flatten()
            .flatten()
        {
            for name in DSH_TRANSCRIPT_NAMES {
                let candidate = session.path().join(name);
                if names_the_seats_own_session(&candidate) {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Whether a transcript's first line is the header of the session this
/// seat booted, rather than one it delegated.
///
/// A file whose header has not landed yet — dsh creates it before its
/// first append, and a half-written line is not JSON — is not the answer
/// *yet*: the poll loop simply asks again on its next pass, which is the
/// same discipline as a root that does not exist at boot.
fn names_the_seats_own_session(candidate: &std::path::Path) -> bool {
    let Ok(file) = std::fs::File::open(candidate) else {
        return false;
    };
    let mut header = String::new();
    if std::io::BufReader::new(file)
        .read_line(&mut header)
        .is_err()
    {
        return false;
    }
    let Ok(event) = serde_json::from_str::<Value>(&header) else {
        return false;
    };
    event.get("type").and_then(Value::as_str) == Some("session")
        && event
            .get("delegationDepth")
            .and_then(Value::as_u64)
            .unwrap_or(0)
            == 0
}

/// What the driver has read of the growing transcript so far: the open
/// handle and the bytes after its last complete line.
#[derive(Default)]
struct DshTail {
    file: Option<std::fs::File>,
    pending: Vec<u8>,
}

/// Fold whatever the child has appended since the last pass. Called
/// while the process still runs, so a partly-written last line stays in
/// `pending` until its newline arrives; a line that is not JSON is
/// noise, dropped, never repaired (decision 0001).
///
/// TRUST BOUNDARY, and it is weaker than claude's. Claude's stream-json
/// arrives on a pipe only the child holds; this is an ordinary file in a
/// directory the seat's own agent can write to, and the driver publishes
/// its relative path in the shared transcript locator. An agent with
/// filesystem tools can therefore append lines this fold will believe —
/// a forged session header can make it follow the wrong file below that
/// seat-owned root, and forged `assistant/message` lines can inflate the
/// turn and token counts. That is accepted, not overlooked: such an
/// agent is already trusted with the working tree, the fold clamps every
/// value it takes (≤80-char tool names and u64 counts) and derives
/// nothing executable from any of them, so the worst case is wrong
/// telemetry in the journal, never an injection or a control-flow
/// decision. A driver whose harness offers a real stdout stream should
/// use it rather than inherit this.
fn drain_dsh_transcript(
    tail: &mut DshTail,
    root: &std::path::Path,
    first_seq: Option<u64>,
    turns: &mut u64,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
) {
    if tail.file.is_none() {
        tail.file = find_dsh_transcript(root).and_then(|path| std::fs::File::open(path).ok());
    }
    let Some(file) = tail.file.as_mut() else {
        return;
    };
    let mut chunk = Vec::new();
    let _ = file.read_to_end(&mut chunk);
    tail.pending.extend_from_slice(&chunk);
    while let Some(index) = tail.pending.iter().position(|byte| *byte == b'\n') {
        let line: Vec<u8> = tail.pending.drain(..=index).collect();
        let Ok(event) = serde_json::from_slice::<Value>(&line) else {
            continue;
        };
        fold_dsh_event(&event, first_seq, turns, session_meta, emit);
    }
}

/// How long the poll loop idles between passes over a running seat's
/// transcript.
const DSH_POLL_IDLE: std::time::Duration = std::time::Duration::from_millis(25);

/// Carry a running seat to its exit code: sample the child's state, then
/// drain whatever it has appended. Exit is sampled BEFORE the drain, so
/// the pass that follows the last one reads a settled file — nothing the
/// child wrote can be missed by the driver losing a race with its own
/// child's exit.
///
/// A `wait` that ERRORS is terminal, never "not finished yet". Folding
/// that error back into the loop is exactly how a seat goes silent
/// forever: a persistent `waitpid` failure — ECHILD, if anything in the
/// process reaps children out from under this one — would spin here at
/// `DSH_POLL_IDLE` with no result, no error and no exit, which is the
/// invisible seat this whole driver change exists to end. The buffered
/// path this loop replaced surfaced such a failure as `agent CLI did not
/// conclude`; so does this one.
fn poll_until_exit(
    mut wait: impl FnMut() -> std::io::Result<Option<i32>>,
    mut drain: impl FnMut(),
) -> Result<i32, String> {
    loop {
        let finished = io_context(wait(), "agent CLI did not conclude")?;
        drain();
        if let Some(code) = finished {
            return Ok(code);
        }
        std::thread::sleep(DSH_POLL_IDLE);
    }
}

fn stage_prompt_with<Create, WritePrompt>(
    prompt: &str,
    create: Create,
    write_prompt: WritePrompt,
) -> Result<tempfile::NamedTempFile, String>
where
    Create: FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
    WritePrompt: FnOnce(&mut tempfile::NamedTempFile, &[u8]) -> std::io::Result<()>,
{
    let mut file = io_context(create(), "could not stage the prompt")?;
    io_context(
        write_prompt(&mut file, prompt.as_bytes()),
        "could not stage the prompt",
    )?;
    Ok(file)
}

fn stage_prompt(prompt: &str) -> Result<tempfile::NamedTempFile, String> {
    stage_prompt_with(
        prompt,
        || {
            tempfile::Builder::new()
                .prefix("forge-prompt-")
                .suffix(".md")
                .tempfile()
        },
        |file, bytes| file.write_all(bytes),
    )
}

/// The claude stream-json invocation, parameterized ONLY by the harness
/// binary (never an `AdapterKind` — the claude and lanetally arms must
/// stay byte-identical in behavior, so kind-specific drift is
/// unrepresentable here). Three invariants, each load-bearing:
/// stdout is folded LIVE line-by-line (checkpoints are streamed
/// telemetry — converging on `wait_with_output` buffering is a rejected
/// design); stderr drains on its own thread so a chatty session — or a
/// chatty wrapper layer — cannot deadlock the stdout fold; unparseable
/// stream lines are noise, never repaired (decision 0001), which is
/// what makes wrapper-interleaved non-JSON output safe.
fn invoke_stream_json(
    command: &[String],
    prompt: &str,
    workdir: &str,
    hold: &mut LaunchHold,
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    let mut transcript = Transcript::resolve(TranscriptKind::ClaudeSession)?;
    let (program, args) = (&command[0], &command[1..]);
    let child = Command::new(program)
        .args(args)
        .current_dir(if workdir.is_empty() { "." } else { workdir })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = io_context(child, "could not invoke the agent CLI")?;
    {
        let mut stdin = child.stdin.take().expect("piped");
        io_context(
            stdin.write_all(prompt.as_bytes()),
            "could not write the prompt",
        )?;
    }
    // stderr drains on its own thread so a chatty session cannot
    // deadlock the stdout stream we are folding live.
    let stderr_pipe = child.stderr.take().expect("piped");
    let stderr_thread = std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut pipe = stderr_pipe;
        let _ = pipe.read_to_end(&mut captured);
        String::from_utf8_lossy(&captured).into_owned()
    });
    let stdout = child.stdout.take().expect("piped");
    let mut session_meta = Map::new();
    let mut assistant_turns = 0u64;
    // Decision 0053: the FIRST pre-turn refusal is the one reported, and
    // the fold keeps reading the whole stream after it. A latched
    // refusal is not a decision to stop listening: a harness that
    // retried and then worked emits turns behind it, and those turns are
    // the seat's served model, usage, cost and resumable session id
    // (decision 0030). Whether the refusal ends the attempt is settled
    // by whether any work began — `run_seat` reads that from the
    // checkpoints, not from where the classifier stopped.
    let mut refusal: Option<String> = None;
    let status = {
        let mut watch = |data: &Value| hold.observe(data, emit);
        for line in std::io::BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            // Unparseable stream lines are noise, never repaired
            // (decision 0001).
            let Ok(event) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let classified = fold_stream_event(
                &event,
                &mut assistant_turns,
                &mut session_meta,
                &mut transcript,
                &mut watch,
            );
            refusal = refusal.or(classified);
        }
        let status = io_context(child.wait(), "agent CLI did not conclude")?;
        transcript.finish(&mut session_meta, &mut watch);
        status
    };
    Ok(Invocation {
        exit_code: status.code().unwrap_or(-1),
        session_meta,
        stdout: String::new(),
        stderr: stderr_thread.join().unwrap_or_default(),
        state: None,
        refusal,
        launch: LaunchTerminal::Cold,
    })
}

/// The sandbox classes `codex exec -s|--sandbox` takes — read-only,
/// workspace-write, danger-full-access (verified against the installed
/// codex-cli 0.148.0, `codex exec --help`). A resume re-expresses a
/// class this list names, or it does not happen: an unknown spelling is
/// never translated on a guess (decision 0030 ruling 2).
const CODEX_SANDBOX_CLASSES: [&str; 3] = ["read-only", "workspace-write", "danger-full-access"];

/// The named execution shape each built-in declares its measurements
/// under (proposed decision 0056 ruling 5). One shape per adapter today
/// because one shape is what each of them actually runs; a second shape
/// is a second measurement, not a second guess.
const CODEX_SHAPE: &str = "work-site";
const CLAUDE_SHAPE: &str = "boxed-workspace";
const LANETALLY_SHAPE: &str = "wrapper-work-site";
const DSH_SHAPE: &str = "headless-work";

/// `codex exec` takes no effort FLAG: the level is a configuration key
/// (`model_reasoning_effort`, verified against codex-cli 0.153.0, whose
/// own error names the vocabulary), and `-c key=value` is the channel
/// for one. So the adapter data says `effort_flag: "--effort"` — the
/// pinning grammar every provider shares, and decision 0035's own
/// spelling — and this driver is where `--effort <level>` becomes the
/// override codex actually reads. The pair is the analogue of the dsh
/// arm's `--model` → overlay translation, for the same reason.
fn codex_effort_config(effort: &str) -> String {
    format!("model_reasoning_effort=\"{effort}\"")
}

/// The seat's pinned effort, taken out of its own passthrough. Shared,
/// because `--effort <level>` is the pinning grammar every provider's
/// adapter data declares (decision 0035 ruling 5) while only one built-in
/// takes it as a flag under that name: `claude --effort <level>` is
/// native (verified against the installed CLI, which names its levels
/// low, medium, high, xhigh, max), so the claude arm passes the pin
/// straight through and never calls this. The arms whose harness spells
/// it differently split it out here first.
///
/// A flag with nothing after it, or a level that is not one bounded
/// word, stays in the argv, so the harness refuses it loudly rather
/// than this adapter dropping a pin in silence.
fn split_effort(extra: &[String]) -> (Option<String>, Vec<String>) {
    let mut effort = None;
    let mut passthrough = Vec::with_capacity(extra.len());
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        // `--effort <level>` and `--effort=<level>`: the value is
        // whichever spelling carried it, and a bare `--effort` at the
        // end of the argv carries none.
        let level = match part.strip_prefix("--effort=") {
            Some(value) => Some(value.to_string()),
            None if part == "--effort" => parts.next().cloned(),
            None => None,
        };
        match level.as_deref().and_then(effort_token) {
            Some(level) if effort.is_none() => effort = Some(level),
            _ => {
                passthrough.push(part.clone());
                // Only a value that arrived as its OWN argv part goes
                // back as one; an `--effort=…` spelling already carries
                // its level inside the part just pushed.
                if part == "--effort" {
                    if let Some(level) = level {
                        passthrough.push(level);
                    }
                }
            }
        }
    }
    (effort, passthrough)
}

/// The cold argv: `codex exec --json -C <workdir>`, the pinned effort as
/// the config override codex reads, the seat's own passthrough, and LAST
/// the engine-managed native controls (decision 0065 ruling 4) — the OFF
/// switch for a native capability the seat does not hold. They arrive
/// from the driver input and never from `extra`: an authored pair that
/// spells the same switch was refused before this builder ran.
fn codex_cold(bin: &str, extra: &[String], workdir: &str, managed: &[String]) -> Vec<String> {
    let (effort, passthrough) = split_effort(extra);
    let mut command = vec![
        bin.to_string(),
        "exec".into(),
        "--json".into(),
        "-C".into(),
        workdir.to_string(),
    ];
    if let Some(effort) = &effort {
        command.push("-c".into());
        command.push(codex_effort_config(effort));
    }
    command.extend(passthrough);
    command.extend(managed.iter().cloned());
    command
}

/// The engine-managed native controls of one codex launch. Read again
/// where a rejected rejoin is replaced cold: `codex_launch` already
/// refused a missing authority, so what is read here is the same plan.
fn codex_managed(input: &Value) -> Vec<String> {
    crate::native_controls::managed(input)
        .ok()
        .flatten()
        .map(|controls| controls.argv)
        .unwrap_or_default()
}

/// The seat's declared sandbox class, taken out of its own passthrough.
/// `codex exec resume` accepts neither `-C/--cd` nor `-s/--sandbox`
/// (verified: `codex exec resume --help`, codex-cli 0.148.0), so the
/// class has to leave the argv here and go back in as
/// `-c sandbox_mode="<class>"`. A flag with nothing after it declares
/// nothing, and the resume refuses itself rather than inventing a class.
fn split_codex_sandbox(extra: &[String]) -> (Option<String>, Vec<String>) {
    let mut class = None;
    let mut passthrough = Vec::with_capacity(extra.len());
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        if let Some(value) = part.strip_prefix("--sandbox=") {
            class = Some(value.to_string());
        } else if part == "--sandbox" || part == "-s" {
            match parts.next() {
                Some(value) => class = Some(value.clone()),
                None => passthrough.push(part.clone()),
            }
        } else {
            passthrough.push(part.clone());
        }
    }
    (class, passthrough)
}

/// The passthrough flags that may travel to a resume, each verified
/// present in `codex exec resume --help` (codex-cli 0.148.0) and unable
/// to express a sandbox or to say WHICH session is rejoined. The first
/// list takes a value, the second stands alone.
///
/// An allow-list, and deliberately not a deny-list of the flags that
/// raise a sandbox: the seat's argv is open-ended, `codex exec resume`
/// takes a strict subset of what `codex exec` takes, and the class has
/// to be re-imposed through `-c sandbox_mode=…` — a lower-precedence
/// mechanism than the `--sandbox` flag the cold path uses. Any list of
/// the ways to outrank it is a list that goes stale the next time codex
/// grows a flag; a list of what is known safe fails closed instead.
const CODEX_RESUME_VALUE_FLAGS: [&str; 7] = [
    "-m",
    "--model",
    "-i",
    "--image",
    "-o",
    "--output-last-message",
    "--output-schema",
];
const CODEX_RESUME_BARE_FLAGS: [&str; 4] = [
    "--json",
    "--strict-config",
    "--skip-git-repo-check",
    "--ephemeral",
];

/// The first part of the seat's passthrough that may not travel to a
/// resume, if there is one. Three kinds of part are refused here and
/// every one of them is a real hazard, not a formality:
///
/// - anything that can reach the sandbox — `-c`, `--enable`/`--disable`,
///   `--dangerously-bypass-approvals-and-sandbox`, `--ignore-rules`;
/// - `--last` and `--all`, which choose which session is rejoined: the
///   seat's argv never redirects an offer the engine made (ruling 4);
/// - a bare word, which `codex exec resume [SESSION_ID] [PROMPT]` would
///   read positionally as the session id — ahead of the one appended
///   here;
///
/// and with them everything `codex exec` takes that a resume does not:
/// `--profile`, `--add-dir`, `--approve-for-me` and `-C` are each
/// rejected outright by `codex exec resume` (verified, 0.148.0), so
/// passing them on would buy a usage error dressed up as a refusal.
fn codex_resume_blocker(passthrough: &[String]) -> Option<String> {
    let mut parts = passthrough.iter();
    while let Some(part) = parts.next() {
        if CODEX_RESUME_BARE_FLAGS.contains(&part.as_str()) {
            continue;
        }
        if CODEX_RESUME_VALUE_FLAGS.contains(&part.as_str()) {
            // The value travels with its flag and is never classified on
            // its own — a model name is not a flag, whatever it spells.
            if parts.next().is_some() {
                continue;
            }
            return Some(part.clone());
        }
        match part.split_once('=') {
            Some((name, _)) if CODEX_RESUME_VALUE_FLAGS.contains(&name) => continue,
            _ => return Some(part.clone()),
        }
    }
    None
}

/// The one part of a codex seat's argv that selects a session on the
/// COLD path, refused there as `claude_selector_conflict` and
/// `dsh_control_conflict` refuse theirs (proposed decision 0056 ruling
/// 4). A bare `resume` is the subcommand word: `codex exec --json -C
/// <dir> resume <id>` parses as `codex exec resume`, so a
/// bundle-authored `resume <id>` after the engine's own flags would turn
/// a cold spawn into a rejoin the engine never offered. `--last` and
/// `--all` select nothing on a cold `exec` — they belong to the resume
/// subcommand alone — so they stay `codex_resume_blocker`'s concern on
/// the warm path and travel unchanged on the cold one, as they always
/// have.
///
/// The word is refused wherever it appears, value positions included: a
/// model, image or output file literally named `resume` is not worth a
/// grammar that has to track which codex flags take a value, and such a
/// grammar goes stale the next time codex grows one. Bundles are
/// operator-trusted, so this is a refusal that names its part, never a
/// silent drop.
const CODEX_SELECTOR: &str = "resume";

fn codex_selector_conflict(extra: &[String]) -> Option<&'static str> {
    extra
        .iter()
        .any(|part| part == CODEX_SELECTOR)
        .then_some(CODEX_SELECTOR)
}

/// A thread id as codex writes it and the journal displays it: one
/// plain identifier of ASCII alphanumerics and dashes, not leading with
/// one. The id reaches argv positionally, so a spelling that could be
/// read as a flag — or as anything but an id — is refused rather than
/// passed on, and the attempt spawns cold.
fn plain_thread_id(id: &str) -> bool {
    !id.starts_with('-')
        && !id.is_empty()
        && id.len() <= 128
        && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

/// How codex is launched for this attempt. A session on offer is taken
/// ONLY when this shape is measured and enabled against the installed
/// CLI, and when the seat's declared sandbox class can travel with it:
/// the resume drops the class it was opened under (measured — a thread
/// opened `-s read-only` writes files on a bare resume), so a class that
/// cannot be re-expressed is a cold spawn with the reason journaled,
/// never a quiet escalation (decision 0030 ruling 2).
///
/// The gate is read FIRST on an offer, then the cheaper local checks,
/// and the version probe runs LAST: a shape that is not enabled at this
/// site declines with the token that says so (`unsupported-resume`,
/// `restrictions-unavailable`) rather than with whatever the seat's own
/// argv happened to lack, and an offer that was already going to be
/// declined never spends a child process to find out which version
/// declined it.
///
/// Refuses outright — before any provider work — when the seat's argv
/// carries a session selector (`codex_selector_conflict`), on the cold
/// path as on the warm one.
fn codex_launch(
    bin: &str,
    extra: &[String],
    workdir: &str,
    session: Option<&str>,
    input: &Value,
) -> Result<LaunchPlan, String> {
    if let Some(conflict) = codex_selector_conflict(extra) {
        return Err(format!(
            "refusing to invoke the agent CLI: the seat's arguments carry '{conflict}', which \
             selects a codex session to rejoin. The engine decides which session an attempt \
             rejoins (proposed decision 0056 ruling 4); an argument that decides it instead is \
             refused before any provider work rather than dropped in silence"
        ));
    }
    // Decision 0065 rulings 4 and 5: the native controls this launch is
    // composed with are the engine's, read from the driver input. A site
    // the engine computed no authority for is refused, and so is an
    // authored argument that reaches the same capability — `--search`, a
    // `web_search` config assignment, even the OFF pair itself — because
    // ordering two controls against each other is not a ruling.
    let controls = crate::native_controls::managed(input)?.unwrap_or_default();
    if let Some(conflict) = crate::native_controls::authored_conflict(extra, &controls.guards) {
        return Err(crate::native_controls::conflict_refusal(&conflict));
    }
    let managed = controls.argv;
    let gate = resume_gate(input, CODEX_SHAPE);
    let probe = vec![bin.to_string(), "--version".to_string()];
    let cold = |refusal: Option<&'static str>, version: Option<String>| LaunchPlan {
        command: codex_cold(bin, extra, workdir, &managed),
        rejoining: None,
        refusal,
        sandbox: None,
        kind: "codex-thread",
        harness_version: version,
        wrapper_digest: None,
        // A codex thread is a rollout on disk: it outlives its
        // invocation, which is what made decision 0030 possible at all.
        persistent: true,
        confirms_from_locator: true,
        effort: None,
    };
    // No offer: a cold spawn with no refusal to report, and a version
    // probe only where the shape is enabled and a confirmed root would
    // therefore be worth recording for the next retry.
    let Some(session) = session else {
        let qualification = qualify(&gate, &probe, None);
        return Ok(cold(None, qualification.observed));
    };
    // A closed gate names ITS reason: under an unmeasured shape the
    // offer is declined because the shape is unmeasured, whatever the
    // seat's argv did or did not declare.
    if let ResumeGate::Disabled(reason) = &gate {
        return Ok(cold(Some(reason), None));
    }
    if !plain_thread_id(session) {
        return Ok(cold(Some("invalid-session-id"), None));
    }
    // The effort pin leaves the argv FIRST, for the same reason the
    // sandbox class does: `codex exec resume` takes neither as a flag,
    // and both go back in as `-c key=value`. Splitting it here also
    // keeps it out of the allow-list check below, which would otherwise
    // read a pin the engine placed as an incompatible argv and drop the
    // whole session over it.
    let (effort, remainder) = split_effort(extra);
    let (class, passthrough) = split_codex_sandbox(&remainder);
    let Some(class) = class else {
        return Ok(cold(Some("sandbox-unavailable"), None));
    };
    if !CODEX_SANDBOX_CLASSES.contains(&class.as_str()) {
        return Ok(cold(Some("unsupported-sandbox"), None));
    }
    // The rest of the seat's argv has to be safe to carry across, part
    // by part: a second sandbox expression could outrank the one
    // re-imposed here, and last-write-wins is not a thing to gamble a
    // restriction on.
    if codex_resume_blocker(&passthrough).is_some() {
        return Ok(cold(Some("incompatible-argv"), None));
    }
    let qualification = qualify(&gate, &probe, originating_harness_version(input));
    if let Some(refusal) = qualification.refusal {
        return Ok(cold(Some(refusal), qualification.observed));
    }
    let mut command = vec![
        bin.to_string(),
        "exec".into(),
        "resume".into(),
        "--json".into(),
        "-c".into(),
        format!("sandbox_mode=\"{class}\""),
    ];
    // The rejoined thread carries the effort it was opened under only
    // because it is re-expressed here: a resume drops what it was given,
    // exactly as it drops the sandbox class.
    if let Some(effort) = &effort {
        command.push("-c".into());
        command.push(codex_effort_config(effort));
    }
    command.extend(passthrough);
    // The managed native controls ride the rejoin exactly as they ride a
    // cold spawn (decision 0065 ruling 4): a resume drops what it was
    // given, so a search switched off cold must be switched off again
    // here. They never passed through `codex_resume_blocker` — that
    // allow-list judges what the SEAT wrote, and an authored `-c` still
    // turns the rejoin cold — and they sit before the two positionals.
    command.extend(managed.iter().cloned());
    command.push(session.to_string());
    // The prompt still arrives on stdin, which `codex exec resume` reads
    // only when the prompt positional is `-` (verified against 0.148.0).
    command.push("-".into());
    Ok(LaunchPlan {
        command,
        rejoining: Some(session.to_string()),
        refusal: None,
        sandbox: Some(class),
        kind: "codex-thread",
        harness_version: qualification.observed,
        wrapper_digest: None,
        persistent: true,
        confirms_from_locator: true,
        effort: None,
    })
}

/// Every claude flag that selects, copies or relocates a conversation.
/// The first list takes a value, the second stands alone; both are read
/// from the installed CLI's own help (2.1.266).
///
/// This is a list of what may NOT appear rather than an allow-list,
/// because unlike `codex exec resume` — which takes a strict subset and
/// therefore fails loudly on anything else — `claude` accepts these
/// beside `--resume` and resolves the conflict itself. `--bg` with
/// `--resume` "starts a copy and says so when the session is already
/// running"; `--fork-session` creates a new id. A copy is not a rejoin,
/// and neither is a fork.
const CLAUDE_SELECTORS_WITH_VALUE: [&str; 5] =
    ["-r", "--resume", "--session-id", "--from-pr", "--teleport"];
const CLAUDE_SELECTORS_BARE: [&str; 8] = [
    "-c",
    "--continue",
    "--fork-session",
    "--bg",
    "--background",
    "--cloud",
    "-w",
    "--worktree",
];

/// The NAME of the first part of the seat's argv that selects a
/// conversation, if there is one. The name alone: a joined spelling's
/// value is a session id, a PR number or a URL, and the refusal that
/// names it reaches `Result.error` and the journal, where no argv value
/// may be copied (AS3).
///
/// Checked on the COLD and gate paths as much as on the resume path
/// (proposed decision 0056 ruling 6): an ambient `--continue` left in a
/// seat's passthrough would silently rejoin whatever conversation the
/// working directory last held, which is worse on a cold path than on a
/// warm one — nothing chose it. A cold-inadmissible setting refuses
/// before any provider work rather than being dropped in silence.
fn claude_selector_conflict(extra: &[String]) -> Option<&'static str> {
    extra.iter().find_map(|part| {
        let name = part.split_once('=').map_or(part.as_str(), |(name, _)| name);
        CLAUDE_SELECTORS_WITH_VALUE
            .iter()
            .chain(CLAUDE_SELECTORS_BARE.iter())
            .find(|selector| **selector == name)
            .copied()
    })
}

/// One authoritative claude restriction control, canonicalized across
/// the spellings the installed 2.1.266 help gives it. The boolean says
/// whether the measured grammar requires a value: `--strict-mcp-config`
/// stands alone, while `--permission-mode`, `--model`, `--effort`,
/// `--tools`, `--mcp-config` and the allowed/disallowed tool lists each
/// take at least one. `--allowedTools`/`--allowed-tools` are ONE control
/// under two names, so a second spelling is a duplicate rather than a
/// companion.
fn claude_restriction_control(part: &str) -> Option<(&'static str, bool)> {
    let name = part.split_once('=').map_or(part, |(name, _)| name);
    Some(match name {
        "--permission-mode" => ("--permission-mode", true),
        "--tools" => ("--tools", true),
        "--strict-mcp-config" => ("--strict-mcp-config", false),
        "--mcp-config" => ("--mcp-config", true),
        "--allowedTools" | "--allowed-tools" => ("--allowedTools", true),
        "--disallowedTools" | "--disallowed-tools" => ("--disallowedTools", true),
        "--model" => ("--model", true),
        "--effort" => ("--effort", true),
        _ => return None,
    })
}

/// The first authoritative claude restriction that is duplicated or
/// malformed, if there is one (AS3; proposed decision 0056 ruling 6).
///
/// The controls are the ones the engine composes the current restriction
/// plan from. A last-wins CLI resolves a second copy in the seat's own
/// favour, which is how a permission mode, tool list, MCP document,
/// allowed-tools list, model or effort could silently replace the plan
/// the operator believes applies — so a duplicate is refused before any
/// provider work rather than appended and gambled on. The same walk
/// enforces the measured arity: a value-taking control with nothing after
/// it, or with the next flag where its value belongs, is malformed.
///
/// `false` for a control that stands alone, `true` for one that takes a
/// value. Only the flag's NAME decides; the value is never classified.
fn claude_restriction_conflict(extra: &[String]) -> Option<String> {
    let mut seen: Vec<&'static str> = Vec::new();
    let mut index = 0;
    while index < extra.len() {
        let part = &extra[index];
        if let Some((control, takes_value)) = claude_restriction_control(part) {
            if seen.contains(&control) {
                return Some(format!(
                    "refusing to invoke the agent CLI: the seat's arguments carry '{control}' more \
                     than once, and the CLI resolves a duplicate last-wins against the current \
                     restriction plan the engine composed (proposed decision 0056 ruling 6)"
                ));
            }
            seen.push(control);
            if takes_value && !part.contains('=') {
                match extra.get(index + 1) {
                    // A value that starts with `-` is the next flag, not
                    // this control's value; the empty string `--tools ""`
                    // is the one admitted empty value and does not.
                    Some(value) if !value.starts_with('-') => index += 1,
                    _ => {
                        return Some(format!(
                            "refusing to invoke the agent CLI: the seat's arguments carry \
                             '{control}' with no value, which the measured grammar requires"
                        ))
                    }
                }
            }
        }
        index += 1;
    }
    None
}

/// The cold argv for the claude stream: the print/stream-json shape this
/// driver has always used, then the seat's own composed passthrough —
/// the permission mode, the model and effort, `--tools ""`, the strict
/// MCP config and the boxed workspace fragment the engine put there.
fn claude_cold(bin: &str, extra: &[String]) -> Vec<String> {
    let mut command = vec![
        bin.to_string(),
        "-p".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--verbose".into(),
    ];
    command.extend(extra.iter().cloned());
    command
}

/// A claude session identifier as the CLI mints and takes one: the
/// installed help calls `--session-id` a UUID, and `--resume` takes that
/// same identifier. The id reaches argv as `--resume`'s value, so a
/// spelling that could be read as a flag — or as a picker search term,
/// which is what a bare `-r` opens — is refused rather than passed on.
fn plain_claude_session(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 80
        && id.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

/// How claude — and LaneTally's wrapper over the same harness — is
/// launched for this attempt.
///
/// The resume path is exactly `--resume <owned-id>` appended to the cold
/// argv, so the current restriction plan the engine composed is the plan
/// that applies: the permission mode, the model and effort, `--tools ""`,
/// the strict MCP config, the current MCP document and the allowed
/// workspace tool where boxed all travel unchanged. Nothing is added on
/// the resume path that the cold path does not also carry, which is what
/// makes "the class is re-imposed, never inherited" checkable by reading
/// one argv rather than reasoning about precedence.
///
/// Deliberately NOT done here, each for a measured reason from the
/// installed 2.1.266 help: no bare `-r`/`--resume` (the value is
/// optional and omitting it opens an interactive picker); no
/// `-c/--continue` (it picks by directory, not by identity); no
/// `--fork-session` and no user `--session-id` beside a resume (both
/// produce a different session); no background, cloud, teleport or
/// worktree selector (`--bg` with `--resume` may start a COPY); and no
/// reliance on new `--system-prompt` text overriding what the
/// conversation recorded, because `--system-prompt-snapshot` defaults to
/// `on` and says the recorded prompt is replayed verbatim on every
/// resume. The current task and result instruction therefore stay on the
/// current user-input path, which is stdin, exactly as on a cold launch.
fn claude_launch(
    bin: &str,
    extra: &[String],
    session: Option<&str>,
    input: &Value,
    shape: &str,
    wrapper_digest: Option<String>,
) -> Result<LaunchPlan, String> {
    // Decision 0065 rulings 4 and 5, as on the codex path: the engine's
    // plan or a refusal, and no authored list that admits a native tool
    // the realm did not grant. The held tools are then folded into the
    // seat's OWN lists, once, so the hands fragment's empty tool list
    // gains exactly what is held, `mcp__brokkr__workspace` stays allowed
    // and strict MCP configuration stays — and the duplicate and arity
    // refusals below judge the argv that will actually run.
    let controls = crate::native_controls::managed(input)?.unwrap_or_default();
    if let Some(conflict) = crate::native_controls::authored_conflict(extra, &controls.guards) {
        return Err(crate::native_controls::conflict_refusal(&conflict));
    }
    let composed = crate::native_controls::apply_selection(extra, &controls.selection);
    let extra = composed.as_slice();
    if let Some(conflict) = claude_selector_conflict(extra) {
        return Err(format!(
            "refusing to invoke the agent CLI: the seat's arguments carry '{conflict}', which \
             selects, copies or relocates a conversation. The engine decides which session an \
             attempt rejoins (proposed decision 0056 ruling 4); an argument that decides it \
             instead is refused before any provider work rather than dropped in silence"
        ));
    }
    if let Some(conflict) = claude_restriction_conflict(extra) {
        return Err(conflict);
    }
    // `--no-session-persistence` is admitted — it is a legitimate thing
    // for a seat to want — and it makes the shape nonresumable, which is
    // a fact the launch row reports rather than a setting to strip.
    let persistent = !extra.iter().any(|part| part == "--no-session-persistence");
    let gate = resume_gate(input, shape);
    let probe = vec![bin.to_string(), "--version".to_string()];
    let plan = |rejoining: Option<String>,
                refusal: Option<&'static str>,
                version: Option<String>| LaunchPlan {
        command: match &rejoining {
            None => claude_cold(bin, extra),
            Some(id) => {
                let mut command = claude_cold(bin, extra);
                command.push("--resume".into());
                command.push(id.clone());
                command
            }
        },
        rejoining,
        refusal,
        sandbox: None,
        kind: "claude-session",
        harness_version: version,
        wrapper_digest: wrapper_digest.clone(),
        persistent,
        confirms_from_locator: true,
        effort: None,
    };
    let Some(session) = session else {
        let qualification = qualify(&gate, &probe, None);
        return Ok(plan(None, None, qualification.observed));
    };
    // The gate first, as on the codex and dsh paths: a closed gate names
    // its own reason, whatever the offered id or the seat's argv looks
    // like.
    if let ResumeGate::Disabled(reason) = &gate {
        return Ok(plan(None, Some(reason), None));
    }
    if !plain_claude_session(session) {
        return Ok(plan(None, Some("invalid-session-id"), None));
    }
    if !persistent {
        return Ok(plan(None, Some("nonpersistent-session"), None));
    }
    let qualification = qualify(&gate, &probe, originating_harness_version(input));
    match qualification.refusal {
        Some(refusal) => Ok(plan(None, Some(refusal), qualification.observed)),
        None => Ok(plan(
            Some(session.to_string()),
            None,
            qualification.observed,
        )),
    }
}

/// Spawn codex, write the prompt to its stdin, and fold its stable
/// `exec --json` JSONL live. Shared by the cold and resume argvs so the
/// two paths differ in exactly one thing: the command line.
fn invoke_codex(
    command: &[String],
    prompt: &str,
    workdir: &str,
    hold: &mut LaunchHold,
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    let mut transcript = Transcript::resolve(TranscriptKind::CodexThread)?;
    let (program, args) = (&command[0], &command[1..]);
    let child = Command::new(program)
        .args(args)
        .current_dir(if workdir.is_empty() { "." } else { workdir })
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();
    let mut child = io_context(child, "could not invoke the agent CLI")?;
    let mut stdin = child.stdin.take().expect("piped");
    io_context(
        stdin.write_all(prompt.as_bytes()),
        "could not write the prompt",
    )?;
    drop(stdin);
    let stderr_pipe = child.stderr.take().expect("piped");
    let stderr_thread = std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut pipe = stderr_pipe;
        let _ = pipe.read_to_end(&mut captured);
        String::from_utf8_lossy(&captured).into_owned()
    });
    let mut session_meta = Map::new();
    let mut turn = 0;
    let mut echo = CodexThreadEcho::default();
    // Decision 0053: codex's wire protocol DOES carry the distinction —
    // a machine-readable `error`/`turn.failed` before the first
    // `turn.started` is a refusal to start. After a turn it is 0016's
    // mid-session failure, so the fold classifies only at `turn == 0`,
    // and — as in the claude arm — it keeps folding the rest of the
    // stream so a session that recovered keeps its thread id and totals.
    let mut refusal: Option<String> = None;
    let status = {
        let mut watch = |data: &Value| hold.observe(data, emit);
        for line in std::io::BufReader::new(child.stdout.take().expect("piped")).lines() {
            let Ok(line) = line else { break };
            if let Ok(event) = serde_json::from_str::<Value>(&line) {
                let classified = fold_codex_event(
                    &event,
                    &mut turn,
                    &mut session_meta,
                    &mut transcript,
                    &mut echo,
                    &mut watch,
                );
                refusal = refusal.or(classified);
            }
        }
        io_context(child.wait(), "agent CLI did not conclude")?
    };
    let stderr = stderr_thread.join().unwrap_or_default();
    // An attempt that concluded without ever reaching a `turn.completed`
    // — a release that folds none, or a codex that exits before its
    // first turn finishes — has read the thread record not at all. Ask
    // it once more here, on the way out, before falling back to the
    // launch header.
    //
    // NOT the deadline case, though it reads like one: a deadline is
    // enforced one process up, where the watchdog SIGKILLs this whole
    // driver (`process.rs::kill_driver`), so nothing after `child.wait`
    // runs for a seat that parks. What a parked codex ran under reaches
    // the journal from the other direction — the `turn-completed`
    // checkpoint each folded turn already emitted names its model and
    // effort as it goes, and a seat killed before its first turn has no
    // fact to carry.
    //
    // Asked unconditionally
    // and inserted only where nothing stands: a turn that already read
    // the record wrote the same pair, and `or_insert` is what keeps a
    // stream's more direct report of the model from being overwritten.
    let (model, effort) = echo.echo();
    if let Some(model) = model {
        session_meta
            .entry("model")
            .or_insert_with(|| Value::String(model));
    }
    if let Some(effort) = effort {
        session_meta
            .entry("effort")
            .or_insert_with(|| Value::String(effort));
    }
    if !session_meta.contains_key("model") {
        if let Some(model) = model_in_header(&stderr) {
            session_meta.insert("model".into(), Value::String(model));
        }
    }
    transcript.finish(&mut session_meta, &mut |data: &Value| {
        hold.observe(data, emit)
    });
    Ok(Invocation {
        exit_code: status.code().unwrap_or(-1),
        session_meta,
        stdout: String::new(),
        stderr,
        state: None,
        refusal,
        launch: LaunchTerminal::Cold,
    })
}

/// The dsh headless invocation. It does NOT converge on `run_cli`'s
/// buffered `wait_with_output`: that is what left a dsh seat silent from
/// `harness-started` to process exit — the wager challenger run
/// scaffold-tool-grants-per-stack-b-58476f86 sat at journal seq 5 for an
/// entire implement seat with no evidence it was alive but a process
/// table. Instead the child's own session transcript is followed as it
/// grows, so each assistant turn advances the journal while the seat
/// works, the way the claude fold does with stream-json.
///
/// stdin and stdout are `/dev/null`: the headless runner reads no stdin
/// and prints only its final answer, which this driver has never
/// journaled. stderr is piped and drained on its own thread so a chatty
/// session cannot deadlock the poll loop.
fn invoke_dsh(
    extra: &[String],
    prompt: &str,
    workdir: &str,
    input: &Value,
    session: Option<&str>,
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    invoke_dsh_with(extra, prompt, workdir, input, session, emit, |child| {
        child
            .try_wait()
            .map(|status| status.map(|status| status.code().unwrap_or(-1)))
    })
}

/// The DSH plugin's own value-taking selectors and the launcher's control
/// spellings. The engine decides which session a seat runs (proposed
/// decision 0056 ruling 4) and composes every restriction the seat applies
/// (ruling 6); a seat's argv carrying one of these would decide or
/// override that instead, and a last-wins plugin resolves the conflict in
/// the seat's favour. So each is refused before any provider work rather
/// than forwarded. `--patch` is validated separately (AS3): `split_dsh_patch`
/// admits exactly the one authorized route overlay.
const DSH_VALUE_SELECTORS: [&str; 7] = [
    "-o",
    "--output-format",
    "-s",
    "--session",
    "-w",
    "--workdir",
    "--json-schema",
];
const DSH_BARE_SELECTORS: [&str; 11] = [
    "-n",
    "--new",
    "-r",
    "--resume",
    "-l",
    "--list",
    "-h",
    "--help",
    "--profile",
    "--dump-config",
    "--dump-default-config",
];

/// The first residual argument in the seat's argv after the engine's own
/// model, effort and single route overlay have been extracted, as a fixed
/// category. The engine decides which session a seat runs (proposed
/// decision 0056 ruling 4) and composes every restriction the seat applies
/// (ruling 6); any residual argument — a value selector, a profile or
/// settings override, extra positional text, the option terminator, a
/// joined/clustered spelling, `--from-default-profile` or an unverified
/// `--verbose` — is refused before any provider work rather than
/// forwarded. The returned category never echoes the token, its joined
/// value, a model, an ID or a path (AS3; tasks 8.8(d)/8.10).
fn dsh_control_conflict(extra: &[String]) -> Option<&'static str> {
    let part = extra.first()?;
    let name = part.split_once('=').map_or(part.as_str(), |(name, _)| name);
    if DSH_VALUE_SELECTORS.contains(&name) || DSH_BARE_SELECTORS.contains(&name) {
        Some("a session or restriction control the engine owns")
    } else if name == "--from-default-profile" || name == "--verbose" {
        Some("an unverified launcher control")
    } else if part == "--" {
        Some("the option terminator")
    } else if part.starts_with('-') {
        Some("an unrecognized or joined option")
    } else {
        Some("extra positional text")
    }
}

/// A DSH session id in the grammar the pinned plugin mints
/// (`session-<uuid>`) and seat-record v5 admits. Anything else is refused
/// rather than passed as a `--session` value that could read as a flag.
fn plain_dsh_session_id(id: &str) -> bool {
    recordable_session_id(id)
}

/// What one stored `session.v3.jsonl`'s first line states. `None` is unsafe
/// evidence — absent, unreadable, truncated, over budget, non-JSON, not a
/// session header at all, or a depth-zero header with no string id — and
/// declines the whole admission rather than being skipped in favour of a
/// convenient match (design D6; task 8.8(d)).
enum DshStoredSession {
    /// A complete header a delegated child wrote under the owned root.
    Delegated,
    /// A complete depth-zero header naming its own session id.
    DepthZero(String),
}

/// Classify a transcript's first line. The line must be complete within a
/// finite budget: a prefix cut at the budget, or a valid JSON prefix
/// padded to the budget and followed by further bytes, is not a header
/// (task 8.8(d)).
fn dsh_stored_session(candidate: &std::path::Path) -> Option<DshStoredSession> {
    dsh_stored_session_with(candidate, DSH_HEADER_LIMIT)
}

/// `dsh_stored_session` over an injected line budget, so the admission
/// bound is reachable from a test without a 4 KiB file.
fn dsh_stored_session_with(candidate: &std::path::Path, budget: u64) -> Option<DshStoredSession> {
    let file = std::fs::File::open(candidate).ok()?;
    let mut reader = std::io::BufReader::new(file);
    let mut header = Vec::new();
    let read = reader
        .by_ref()
        .take(budget)
        .read_until(b'\n', &mut header)
        .ok()?;
    // A newline within the budget is the only complete line: `read_until`
    // stops at the budget without it, so a longer or padded row cannot
    // present its prefix as the whole header.
    if read == 0 || header.last() != Some(&b'\n') {
        return None;
    }
    let event = serde_json::from_slice::<Value>(&header).ok()?;
    if event.get("type").and_then(Value::as_str) != Some("session") {
        return None;
    }
    // The selected core's header requires a non-negative safe-integer
    // `delegationDepth`. `as_u64` refuses a string, null, negative,
    // fractional or missing value instead of defaulting malformed storage
    // to the seat's own depth and admitting a partial/mismatched boundary
    // (design D6; task 8.8(d)).
    let depth = event.get("delegationDepth").and_then(Value::as_u64)?;
    if depth != 0 {
        return Some(DshStoredSession::Delegated);
    }
    event
        .get("id")
        .and_then(Value::as_str)
        .map(|id| DshStoredSession::DepthZero(id.to_string()))
}

/// The one stored depth-zero session file under a retained root whose
/// header names `expected`, or a refusal. Exactly one candidate is
/// required: a missing or ambiguous set is unreadable rather than resolved
/// to a substitute. Each project/session directory and the selected
/// `session.v3.jsonl` is canonicalized and required to stay inside the
/// retained root, and the whole walk is charged against a finite
/// enumeration budget. A directory, file or header that cannot be admitted
/// fails the whole selection rather than being skipped for a convenient
/// match: an escaping symlink, an unreadable candidate or an
/// invalid/truncated header cannot supply the one valid depth-zero header
/// (design D6; task 8.8(d)).
fn dsh_session_file(root: &std::path::Path, expected: &str) -> Result<std::path::PathBuf, String> {
    dsh_session_file_with(root, expected, DSH_DIRECTORY_ENTRIES)
}

/// One directory's entries as `std::fs::read_dir` yields them, in a shape a
/// test can substitute a failing iterator for. The real reader stays the
/// production read.
type SessionDirEntries = Box<dyn Iterator<Item = std::io::Result<std::fs::DirEntry>>>;

fn read_session_dir(path: &std::path::Path) -> std::io::Result<SessionDirEntries> {
    Ok(Box::new(std::fs::read_dir(path)?))
}

/// `dsh_session_file` over an injected enumeration budget, so the finite
/// bound is reachable from a test with a handful of entries.
fn dsh_session_file_with(
    root: &std::path::Path,
    expected: &str,
    budget: usize,
) -> Result<std::path::PathBuf, String> {
    dsh_session_file_reading(root, expected, budget, &read_session_dir)
}

/// `dsh_session_file_with` over an injected directory reader, so the
/// per-entry iterator error arms are reachable from a test without a
/// filesystem that fails mid-enumeration.
fn dsh_session_file_reading(
    root: &std::path::Path,
    expected: &str,
    budget: usize,
    read_dir: &dyn Fn(&std::path::Path) -> std::io::Result<SessionDirEntries>,
) -> Result<std::path::PathBuf, String> {
    let mut matches: Vec<std::path::PathBuf> =
        dsh_depth_zero_sessions_reading(root, budget, read_dir)?
            .into_iter()
            .filter(|(id, _)| id == expected)
            .map(|(_, candidate)| candidate)
            .collect();
    match matches.len() {
        0 => Err("dsh driver: no stored depth-zero session names the offered id".to_string()),
        1 => Ok(matches.remove(0)),
        _ => Err("dsh driver: more than one stored session names the offered id".to_string()),
    }
}

/// Every stored depth-zero session under a retained root, as `(id, file)`
/// pairs in enumeration order.
///
/// This is the one walk: `dsh_session_file_reading` filters it for the
/// offered id, and Pass C's launch confirmation counts its `(id, file)`
/// occurrences against the ones the store held before the child spawned,
/// so a plugin that opened a FRESH sibling session instead of rejoining
/// the offered one is visible as an entry that was not there before —
/// even when it reuses an id the store already held at another address
/// (task 8.8(d); design D6/D7). Every occurrence is reported, repeated ids
/// and repeated canonical addresses included. Every containment, budget and header rule below is the
/// admission's own: a directory, file or header that cannot be admitted
/// fails the whole walk rather than being skipped for a convenient match.
fn dsh_depth_zero_sessions_reading(
    root: &std::path::Path,
    budget: usize,
    read_dir: &dyn Fn(&std::path::Path) -> std::io::Result<SessionDirEntries>,
) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    let root = std::fs::canonicalize(root)
        .map_err(|_| "dsh driver: the retained root is unreadable".to_string())?;
    let mut found = Vec::new();
    let mut visited = 0usize;
    let charge = |visited: &mut usize| -> Result<(), String> {
        *visited += 1;
        if *visited > budget {
            return Err("dsh driver: the retained root exceeds its enumeration budget".to_string());
        }
        Ok(())
    };
    let projects =
        read_dir(&root).map_err(|_| "dsh driver: the retained root is unreadable".to_string())?;
    for project in projects {
        let project =
            project.map_err(|_| "dsh driver: the retained root is unreadable".to_string())?;
        charge(&mut visited)?;
        let project_dir = std::fs::canonicalize(project.path())
            .map_err(|_| "dsh driver: a retained project path is unreadable".to_string())?;
        if !project_dir.starts_with(&root) {
            return Err("dsh driver: a retained project path escapes the owned root".to_string());
        }
        if !project_dir.is_dir() {
            return Err("dsh driver: a retained project path is not a directory".to_string());
        }
        let sessions = read_dir(&project_dir)
            .map_err(|_| "dsh driver: the retained root is unreadable".to_string())?;
        for session in sessions {
            let session =
                session.map_err(|_| "dsh driver: the retained root is unreadable".to_string())?;
            charge(&mut visited)?;
            let session_dir = std::fs::canonicalize(session.path())
                .map_err(|_| "dsh driver: a retained session path is unreadable".to_string())?;
            if !session_dir.starts_with(&root) {
                return Err(
                    "dsh driver: a retained session path escapes the owned root".to_string()
                );
            }
            if !session_dir.is_dir() {
                return Err("dsh driver: a retained session path is not a directory".to_string());
            }
            let candidate = std::fs::canonicalize(session_dir.join(DSH_TRANSCRIPT))
                .map_err(|_| "dsh driver: the stored session file is unreadable".to_string())?;
            if !candidate.starts_with(&root) {
                return Err(
                    "dsh driver: the stored session file escapes the owned root".to_string()
                );
            }
            if !candidate.is_file() {
                return Err("dsh driver: the stored session file is not a regular file".to_string());
            }
            match dsh_stored_session(&candidate) {
                None => {
                    return Err("dsh driver: a retained session header is unreadable".to_string())
                }
                Some(DshStoredSession::Delegated) => {}
                Some(DshStoredSession::DepthZero(id)) => found.push((id, candidate)),
            }
        }
    }
    Ok(found)
}

/// Every stored depth-zero session under a retained root, read through
/// production's own directory reader and enumeration budget.
fn dsh_depth_zero_sessions(
    root: &std::path::Path,
) -> Result<Vec<(String, std::path::PathBuf)>, String> {
    dsh_depth_zero_sessions_reading(root, DSH_DIRECTORY_ENTRIES, &read_session_dir)
}

/// The highest sequence number stored in a session file — the owned
/// pre-followup boundary a warm invocation folds past — or `None` when the
/// file cannot be read inside the finite admission budget. An unreadable,
/// over-budget, truncated or malformed row refuses the offer rather than
/// guessing a baseline; a truncated boundary is never a zero or a
/// partial-prefix maximum (design D8; task 8.8(d)).
///
/// The first row is the header, read inside the header budget; every later
/// row is one whole serialized provider event, so it gets the file-sized
/// event budget instead. A normal user message or tool result serializes
/// into ONE row and easily exceeds a header-sized line while staying far
/// below the file budget, so the event rows must not be cut at the header
/// bound (design D6; task 8.8(d)).
fn dsh_session_last_seq(path: &std::path::Path) -> Option<u64> {
    dsh_session_last_seq_with(path, DSH_SESSION_FILE_LIMIT)
}

/// `dsh_session_last_seq` over an injected event budget, so an over-budget
/// event row is reachable from a test without a 64 MiB file.
fn dsh_session_last_seq_with(path: &std::path::Path, event_budget: u64) -> Option<u64> {
    let file = std::fs::File::open(path).ok()?;
    if file.metadata().ok()?.len() > DSH_SESSION_FILE_LIMIT {
        return None;
    }
    let mut reader = std::io::BufReader::new(file);
    let mut last = 0u64;
    let mut line = Vec::new();
    // The header is the first row and stays inside the header budget;
    // every row after it is a whole event bounded by the file budget.
    let mut budget = DSH_HEADER_LIMIT;
    let mut first = true;
    loop {
        line.clear();
        let read = reader
            .by_ref()
            .take(budget)
            .read_until(b'\n', &mut line)
            .ok()?;
        if read == 0 {
            break;
        }
        // Complete, newline-terminated rows only: a row cut at the budget
        // or a half-written final row is truncated evidence, not a row to
        // skip past while reporting a lower maximum.
        if line.last() != Some(&b'\n') {
            return None;
        }
        let event = serde_json::from_slice::<Value>(&line).ok()?;
        if first {
            // The header names the session and carries no `seq`; it is not
            // part of the current-work boundary.
            if event.get("type").and_then(Value::as_str) != Some("session") {
                return None;
            }
            first = false;
            budget = event_budget;
            continue;
        }
        // A complete row that is not an event carrying a non-negative
        // integer sequence is malformed storage, not a row to skip:
        // skipping it would report a lower maximum as if the boundary were
        // complete (design D6; task 8.8(d)).
        let seq = event.get("seq").and_then(Value::as_u64)?;
        last = last.max(seq);
    }
    Some(last)
}

/// Resolve one owned DSH persistence locator beneath the admitted home.
/// The locator is a bounded relative path; an absolute path, a `..`
/// component, a symlink escape or a locator over the 80-character bound is
/// refused, and the resolved directory must hold exactly one depth-zero
/// session header naming the offered id (task 8.8(d)).
///
/// The returned root keeps the caller's admitted-home spelling (rather
/// than the canonical target) so the planned locator can be recomputed
/// from the same address without a lossy conversion: a symlinked spelling
/// of the same canonical home is equivalent, and the canonical containment
/// check is what rejects an escape.
/// The address this launch will RECORD, bounded before anything is
/// staged (task 8.8(d); design D6).
///
/// Held apart from the planner on purpose. In the planner the two paths
/// that reach it cannot breach the bound — an offered root was already
/// bounded by `resolve_dsh_root`, and a fresh root is short and
/// separator-free by construction — so the refusal is a restatement
/// there and no commission could exercise it. The rule is still a rule:
/// what `Transcript::record` will clamp is the RESOLVED root's address,
/// not the offered string, and those are the same only while every
/// producer of a root keeps them so. Stating it once, here, keeps the
/// requirement enforced at the one place the planner reads a locator and
/// leaves it answerable to a test.
fn planned_dsh_locator(transcript: &Transcript, root: &std::path::Path) -> Result<String, String> {
    let locator = transcript.locator_under_home(root)?;
    if locator.is_empty() || locator.chars().count() > DSH_LOCATOR_LIMIT {
        return Err(
            "dsh driver: the planned dsh locator is outside the admitted bound".to_string(),
        );
    }
    Ok(locator)
}

fn resolve_dsh_root(
    home: &std::path::Path,
    locator: &str,
    expected: &str,
) -> Result<std::path::PathBuf, String> {
    if locator.is_empty() {
        return Err("dsh driver: the owned target carries no persistence locator".into());
    }
    if locator.chars().count() > DSH_LOCATOR_LIMIT {
        return Err("dsh driver: the owned persistence locator exceeds the admitted bound".into());
    }
    let relative = std::path::Path::new(locator);
    if relative.is_absolute() {
        return Err(
            "dsh driver: the owned persistence locator is not a bounded relative path".into(),
        );
    }
    for component in relative.components() {
        // `Transcript::locator_under_home` rewrites `\` to `/` for every
        // transcript producer; on Unix a literal backslash is an ordinary
        // filename byte, so a locator whose component carries one would be
        // *recorded* as a different address than the one admitted here.
        // Refuse it rather than let the shared clamp turn one owned address
        // into another (task 8.8(d); design D6).
        match component {
            std::path::Component::Normal(name) if !name.to_string_lossy().contains('\\') => {}
            _ => {
                return Err(
                    "dsh driver: the owned persistence locator is not a bounded, round-tripping \
                     relative path"
                        .into(),
                )
            }
        }
    }
    let canonical_home = std::fs::canonicalize(home)
        .map_err(|_| "dsh driver: the admitted dsh home is unreadable".to_string())?;
    let resolved = std::fs::canonicalize(canonical_home.join(relative))
        .map_err(|_| "dsh driver: the owned persistence locator does not resolve".to_string())?;
    if !resolved.starts_with(&canonical_home) {
        return Err("dsh driver: the owned persistence locator escapes the dsh home".into());
    }
    if !resolved.is_dir() {
        return Err("dsh driver: the owned persistence locator is not a directory".into());
    }
    dsh_session_file(&resolved, expected)?;
    Ok(home.join(relative))
}

/// One DSH invocation's settled plan: the argv (minus the prompt), the
/// root it intends to rejoin, the bounded reason an offer was declined,
/// the version and composite OBSERVED on the enabled path, and the staged
/// seat overlay that must outlive the child.
struct DshLaunch {
    command: Vec<String>,
    rejoining: Option<String>,
    refusal: Option<&'static str>,
    observed: Option<String>,
    wrapper_digest: Option<String>,
    /// Whether this invocation uses the plugin's `--output-format
    /// stream-json` exchange. False on the shipped cold route, where the
    /// driver folds the retained transcript instead.
    stream_json: bool,
    /// Fold only transcript events past this sequence: the owned root's
    /// stored boundary when rejoining, and `None` on a fresh root, whose
    /// file is this invocation's from its first event.
    first_seq: Option<u64>,
    locator: String,
    /// The absolute retained root the transcript fold follows.
    root: std::path::PathBuf,
    /// Held for the child's lifetime; dropping it removes the staged file.
    #[allow(dead_code)]
    overlay: DshSeatOverlay,
    /// True when the seat pinned no `--effort`, so the absence is the
    /// standing and every row reads `not applicable` (decision 0035
    /// addendum 2026-09-11).
    effortless: bool,
    /// The host's git identity and directories, resolved before the seat
    /// can edit anything (decision 0054).
    facts: GitFacts,
    /// The private git store a linked-worktree seat commits into, held
    /// for the seat's whole life and handed to the promotion afterwards.
    staged: Option<(String, dsh_sandbox::GitScope, dsh_sandbox::SeatGitStore)>,
}

impl DshLaunch {
    fn plan(&self) -> LaunchPlan {
        LaunchPlan {
            command: self.command.clone(),
            rejoining: self.rejoining.clone(),
            refusal: self.refusal,
            sandbox: None,
            // The row predates any header echo, so a pinned seat reads
            // `not reported` here and the echo lands on the rows that
            // follow. A seat on an effortless route has no level to
            // echo at all, and says so from this first row.
            effort: Some(if self.effortless {
                EFFORT_NOT_APPLICABLE.to_string()
            } else {
                EFFORT_NOT_REPORTED.to_string()
            }),
            kind: "dsh-session",
            harness_version: self.observed.clone(),
            wrapper_digest: self.wrapper_digest.clone(),
            persistent: true,
            // Confirmation is the stream-json init event, never the
            // retained locator: a directory is not a provider handle
            // (proposed decision 0056 ruling 3). The invocation drives
            // `LaunchHold::confirm` itself on that event.
            confirms_from_locator: false,
        }
    }
}

/// How a DSH seat is launched for this attempt (task 8.8(d)).
///
/// The gate closes before any probe: while the shape is `unmeasured` (or
/// otherwise disabled) the seat runs the shipped cold invocation unchanged,
/// with no version probe, no composite recompute and no `--new`/`--session`.
/// Where the gate is open the driver probes the core version, recomputes
/// the canonical composite through the one Rust producer, and compares both
/// against the declared identity and — on an offer — against the
/// originating root's recorded values. Any mismatch, a `supported` shape
/// without the member, or an originating root with no recorded digest
/// declines as `unverified-harness`, ships the cold route and records no
/// offerable root.
fn dsh_launch(
    bin: &str,
    extra: &[String],
    workdir: &str,
    session: Option<&str>,
    input: &Value,
) -> Result<DshLaunch, String> {
    dsh_launch_resolving(bin, extra, workdir, session, input, DshSeams::resolve)
}

/// `dsh_launch` over an injected seam resolver, so the unreadable-seams
/// refusal is a plain test without an environment that has no DSH home.
/// The real resolver stays production's only path into a launch.
fn dsh_launch_resolving(
    bin: &str,
    extra: &[String],
    workdir: &str,
    session: Option<&str>,
    input: &Value,
    resolve: impl FnOnce() -> Result<DshSeams, CompositeError>,
) -> Result<DshLaunch, String> {
    dsh_launch_with(bin, extra, workdir, session, input, || {
        let seams = resolve()
            .map_err(|error| format!("dsh driver: the dsh seams are unreadable: {error}"))?;
        dsh_composite(&seams)
            .map_err(|error| format!("dsh driver: the composite identity is unreadable: {error}"))
    })
}

/// `dsh_launch` over an injected composite producer, so every drift and
/// mismatch case is a plain test over synthetic homes.
fn dsh_launch_with(
    bin: &str,
    extra: &[String],
    workdir: &str,
    session: Option<&str>,
    input: &Value,
    composite: impl FnOnce() -> Result<DshComposite, String>,
) -> Result<DshLaunch, String> {
    // Original adjacency first: the three extractions below are
    // sequential, so a control standing in another control's value slot
    // would vanish before that slot is read (see `dsh_input_boundaries`).
    dsh_input_boundaries(extra)?;
    let (model, passthrough) = split_dsh_model(extra)?;
    let (effort, passthrough) = split_effort(&passthrough);
    let (route_arg, passthrough) = split_dsh_patch(&passthrough)?;
    // The inherited selector-only deny-list is not the admission rule:
    // after the engine's own model, effort and single route overlay are
    // extracted, EVERY residual argument is refused before any route read,
    // version probe, composite call or staging. The fixed category never
    // echoes the token (AS3; tasks 8.8(d)/8.10).
    if let Some(category) = dsh_control_conflict(&passthrough) {
        return Err(format!(
            "refusing to invoke the agent CLI: the seat's arguments carry {category}, which the \
             engine owns or does not recognize (proposed decision 0056 rulings 4 and 6); it is \
             refused before any provider work rather than resolved last-wins by the harness"
        ));
    }
    // Validate the extracted model and the effort/model relationship before
    // any provider observation, so a malformed pin refuses on the cold,
    // offered and disabled paths alike (task 8.8(d)).
    if let Some(model) = model.as_deref() {
        parse_dsh_model(model)?;
    }
    if effort.is_some() && model.is_none() {
        return Err(
            "dsh driver: `--effort` needs a `--model` beside it: the level rides the seat's \
             default-model selection, which names its provider and model, and this driver does \
             not read the profile's default back to restate it"
                .to_string(),
        );
    }
    let route = route_overlay::claim(input, workdir, model.as_deref(), route_arg.as_deref())?;
    let transcript = Transcript::resolve(TranscriptKind::DshSession)?;
    let home = transcript.home().to_path_buf();
    let gate = resume_gate(input, DSH_SHAPE);

    let mut observed: Option<String> = None;
    let mut digest: Option<String> = None;
    let mut qualified = false;
    let mut refusal: Option<&'static str> = None;
    match &gate {
        ResumeGate::Disabled(reason) => {
            // A closed gate reports its reason only beside an OFFER it
            // declined; a cold seat that never offered a session carries
            // no refusal token at all.
            if session.is_some() {
                refusal = Some(reason);
            }
        }
        ResumeGate::Enabled { applies_to } => {
            let declared = input
                .pointer("/resume_context/assessment/headless-work/identity/wrapper_digest")
                .and_then(Value::as_str)
                .filter(|value| recordable_digest(value));
            if let Some(declared) = declared {
                if let Some(version) = observed_version(&[bin.to_string(), "--version".to_string()])
                {
                    observed = Some(version.clone());
                    if &version == applies_to {
                        if let Ok(value) = composite() {
                            if value.canonical() == declared {
                                digest = Some(value.canonical().to_string());
                                qualified = true;
                            }
                        }
                    }
                }
            }
            // An offer must be opened under the same version AND the same
            // composite it was recorded with; a root with no recorded
            // digest is not offerable.
            if qualified && session.is_some() {
                qualified = originating_harness_version(input) == observed.as_deref()
                    && originating_wrapper_digest(input) == digest.as_deref();
            }
            if !qualified && session.is_some() {
                refusal = Some("unverified-harness");
            }
        }
    }

    let mut stream_json = qualified;
    let mut rejoining: Option<String> = None;
    let mut first_seq = None;
    let fresh = |home: &std::path::Path| -> Result<std::path::PathBuf, String> {
        dsh_transcript_root_in(|| dsh_transcript_root_under(Some(home.to_path_buf())))
    };
    let root = if qualified {
        match session {
            Some(id) => match owned_dsh_root(&home, input, id, &dsh_session_file) {
                Ok((root, boundary)) => {
                    rejoining = Some(id.to_string());
                    first_seq = Some(boundary);
                    root
                }
                // The token is the check's own: an id outside the grammar
                // is `invalid-session-id`, a recorded home that is not the
                // admitted one is `instance-changed`, and a store the
                // driver cannot verify is `unverified-harness` (LE2). The
                // bounded reason stays out of the row.
                Err((token, _)) => {
                    refusal = Some(token);
                    stream_json = false;
                    fresh(&home)?
                }
            },
            None => fresh(&home)?,
        }
    } else {
        fresh(&home)?
    };

    // The planned locator is validated against the same 80-character
    // bound before the shared `Transcript::record` clamp can shorten it
    // into a different address, and before any overlay is staged
    // (task 8.8(d); design D6). The offered locator was already required
    // to round-trip losslessly by `resolve_dsh_root`; the fresh root is
    // short and separator-free by construction, so this checks its output
    // too.
    let locator = planned_dsh_locator(&transcript, &root)?;
    // The dsh harness sandbox confines writes to the session workspace
    // (decision 0054). A linked worktree's git metadata lives outside it,
    // so the driver resolves the two git directories through Git NOW —
    // before the seat can edit anything — and, when the seat's mode
    // confines writes, points dsh's sandbox provider at the scoped
    // runner that can reach them. A seat that cannot commit refuses
    // here, before it spends an implementation.
    let facts = if workdir.is_empty() {
        GitFacts::default()
    } else {
        crate::hands::git_facts(Path::new(workdir))
    };
    let mode = std::env::var("DSH_PERMISSION_MODE").unwrap_or_default();
    // The private git store is named in every command's runner argv, so
    // it is held for the seat's whole life and handed to the promotion,
    // which is the only way anything the seat committed reaches the
    // shared repository.
    let staged = dsh_sandbox_row_for(workdir, &facts, &mode)?;
    let sandbox_row = staged.as_ref().map(|(row, _, _)| row.clone());
    let overlay = dsh_seat_overlay_with(
        model.as_deref(),
        effort.as_deref(),
        &root,
        route.as_deref(),
        sandbox_row.as_deref(),
    )?;
    let mut command = vec![
        bin.to_string(),
        "--profile".into(),
        "headless".into(),
        "--patch".into(),
        overlay.path().to_string_lossy().into_owned(),
    ];
    if stream_json {
        command.push("--output-format".into());
        command.push("stream-json".into());
        match &rejoining {
            Some(id) => {
                command.push("--session".into());
                command.push(id.clone());
            }
            None => command.push("--new".into()),
        }
    }
    // Nothing of the seat's own argv follows: `dsh_control_conflict`
    // above refused every residual part, so the argv is exactly what the
    // engine composed.
    Ok(DshLaunch {
        command,
        rejoining,
        refusal,
        observed,
        wrapper_digest: digest,
        stream_json,
        first_seq,
        locator,
        root,
        overlay,
        effortless: effort.is_none(),
        facts,
        staged,
    })
}

/// The offered root resolved beneath the admitted home, plus the sequence
/// boundary a warm fold starts after. The complete owned target is
/// required: a string provider ID equal to the negotiated ID, a
/// non-empty persistence locator and the recorded persistence home, all
/// read off the SAME confirmed checkpoint. The recorded home is
/// canonicalized and required to equal the current admitted home before
/// any retained store is read, so two homes holding the identical
/// ID/locator never redirect the offer (task 8.8(d); design D6).
///
/// A refusal carries the v5 token the launch row will name beside its
/// bounded reason: `invalid-session-id` for an id outside the grammar,
/// `instance-changed` for a recorded home that is not the admitted one —
/// the one detectable client drift this check can see — and
/// `unverified-harness` for everything the retained store cannot verify.
///
/// The matching depth-zero file is selected twice: once inside
/// `resolve_dsh_root`, which validates containment, and once here, which
/// re-selects before reading the boundary. A store that changes between
/// the two reads must refuse rather than fold a file the first selection
/// never admitted, so the second selection is a real guard, not a
/// formality. `select_file` is that second selection; the production
/// caller supplies `dsh_session_file`, and a test supplies a selector
/// that reproduces drift deterministically (design D10).
fn owned_dsh_root(
    home: &std::path::Path,
    input: &Value,
    id: &str,
    select_file: &dyn Fn(&std::path::Path, &str) -> Result<std::path::PathBuf, String>,
) -> Result<(std::path::PathBuf, u64), (&'static str, String)> {
    let unverified = |why: &str| ("unverified-harness", format!("dsh driver: {why}"));
    if !plain_dsh_session_id(id) {
        return Err((
            "invalid-session-id",
            "dsh driver: the offered session id is outside the admitted grammar".into(),
        ));
    }
    let provider = input
        .pointer("/resume_context/owned_target/provider_id")
        .and_then(Value::as_str)
        .ok_or_else(|| unverified("the owned target carries no provider id"))?;
    if provider != id {
        return Err(unverified("the owned target names a different root"));
    }
    let locator = input
        .pointer("/resume_context/owned_target/persistence_locator")
        .and_then(Value::as_str)
        .filter(|locator| !locator.is_empty())
        .ok_or_else(|| unverified("the owned target carries no persistence locator"))?;
    let recorded_home = input
        .pointer("/resume_context/owned_target/persistence_home")
        .and_then(Value::as_str)
        .filter(|home| !home.is_empty())
        .ok_or_else(|| unverified("the owned target carries no persistence home"))?;
    let canonical_home = std::fs::canonicalize(home)
        .map_err(|_| unverified("the admitted dsh home is unreadable"))?;
    let canonical_recorded = std::fs::canonicalize(recorded_home)
        .map_err(|_| unverified("the recorded persistence home is unreadable"))?;
    if canonical_home != canonical_recorded {
        return Err((
            "instance-changed",
            "dsh driver: the owned target names a different persistence home".into(),
        ));
    }
    let root =
        resolve_dsh_root(home, locator, id).map_err(|error| ("unverified-harness", error))?;
    let file = select_file(&root, id).map_err(|error| ("unverified-harness", error))?;
    let boundary = dsh_session_last_seq(&file)
        .ok_or_else(|| unverified("the stored session sequence is unreadable"))?;
    Ok((root, boundary))
}

/// The same invocation with the one question the OS answers — "is the
/// child still running?" — injectable, the way `stage_prompt_with` and
/// `dsh_transcript_root_in` make their own syscalls injectable. A real
/// `waitpid` failure cannot be provoked from a test, and the arm that
/// handles it is the difference between a seat that reports a refusal
/// and a seat that spins in silence forever, so it is reachable here.
#[allow(clippy::too_many_arguments)]
fn invoke_dsh_with(
    extra: &[String],
    prompt: &str,
    workdir: &str,
    input: &Value,
    session: Option<&str>,
    emit: &mut impl FnMut(&Value),
    wait: impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
) -> Result<Invocation, String> {
    let bin = adapter_binary("BROKKR_DSH_BIN", Some("FORGE_DSH_BIN"), "dsh");
    let launch = dsh_launch(&bin, extra, workdir, session, input)?;
    invoke_dsh_launch(launch, prompt, workdir, emit, wait)
}

/// `invoke_dsh_with` over an already-settled launch, so the qualified
/// stream-json arm is reachable from a test without a real composite
/// install and its node probe. Production reaches it only through
/// `dsh_launch`, which still performs every qualification check.
fn invoke_dsh_launch(
    launch: DshLaunch,
    prompt: &str,
    workdir: &str,
    emit: &mut impl FnMut(&Value),
    wait: impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
) -> Result<Invocation, String> {
    invoke_dsh_launch_observed(launch, prompt, workdir, emit, wait, &mut |_| {})
}

/// `invoke_dsh_launch` with this ONE invocation's observer of the
/// confirmation's completed observations. Production watches nothing; a
/// test reads here exactly what the watcher consumed — never a second
/// walk of its own — and only once the watcher has ruled on it.
fn invoke_dsh_launch_observed(
    mut launch: DshLaunch,
    prompt: &str,
    workdir: &str,
    emit: &mut impl FnMut(&Value),
    wait: impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
    observer: &mut impl FnMut(&DshObservation),
) -> Result<Invocation, String> {
    let transcript = Transcript::resolve(TranscriptKind::DshSession)?;
    let staged = launch.staged.take();
    let mut session_meta = Map::new();
    // Decision 0035 addendum 2026-09-11: a dsh seat with no `--effort`
    // pin compiles only on an effortless route, so the absence IS the
    // standing — no level to forward, none for any header to echo. It
    // is seeded so every fold row and the finishing record read `not
    // applicable` from the first row; a header that echoes a real level
    // overwrites the seed, which is the applied configuration ruling 3
    // asks the fold to read.
    if launch.effortless {
        session_meta.insert(
            "effort".into(),
            Value::String(EFFORT_NOT_APPLICABLE.to_string()),
        );
    }
    // The store is censused BEFORE anything spawns, so a session the
    // plugin opens instead of rejoining the offered one is visible as a
    // session that was not there (task 8.8(d), Pass C).
    let mut watch = DshRootWatch::new(&launch, transcript);
    let mut hold = LaunchHold::new("deepseek", launch.plan());
    // The retained locator is published before anything spawns, exactly as
    // before; `run_seat` holds it until a turn begins (decision 0053). A
    // REJOIN holds it instead: an attempt that never confirms the offered
    // root publishes no locator either, because the row would address a
    // session this driver cannot say it was in (design D7).
    if !watch.confirming() {
        watch.record_locator(&mut hold, &mut session_meta, emit);
    }
    // The shipped cold route confirms nothing, so its one launch row is
    // published before the spawn — exactly where it always was, and the
    // only reason a spawn failure still flushes the held rows. The
    // stream-json route waits for the init event instead.
    if !launch.stream_json {
        hold.finish(emit);
    }
    let mut command = launch.command.clone();
    command.push(prompt.to_string());
    // Everything the seat committed lives in the private store and
    // nowhere else until the promotion below moves it. Returning early
    // would drop the store — and the seat's work with it — behind an
    // error naming neither (decision 0054 ruling 5).
    let attempt = if launch.stream_json {
        invoke_dsh_stream_json(
            &command,
            &launch,
            workdir,
            &mut watch,
            &mut hold,
            &mut session_meta,
            emit,
            observer,
        )
    } else {
        invoke_dsh_shipped(&command, workdir, &launch, wait, &mut session_meta, emit)
    };
    let mut invocation = match attempt {
        Ok(invocation) => invocation,
        Err(problem) => return Err(dsh_failure_before_promotion(problem, staged)),
    };
    // The seat wrote its objects and moved its branch inside the private
    // common directory the driver staged; the shared repository was
    // read-only to it throughout. This is where the ONE ref the worktree
    // owns crosses over, outside every box (decision 0054 ruling 5). A
    // promotion that cannot happen is a driver failure with the store's
    // path in it, never a silent loss of the seat's commits.
    if let Some((_, scope, store)) = staged {
        if let Some(promotion) = dsh_sandbox::promote_seat_commits(store, &scope)? {
            invocation.stderr.push_str(&promotion.summary());
            invocation.stderr.push('\n');
        }
    }
    // The qualified route's launch row was held until the harness named
    // its root, or until the invocation ended without one (`finish` is a
    // no-op once a confirmation has already published).
    if launch.stream_json {
        hold.finish(emit);
    }
    invocation.launch = hold.terminal();
    Ok(invocation)
}

/// Spawn one dsh child with the given stdout disposition, and drain its
/// stderr on a thread so a chatty session cannot deadlock the fold.
fn spawn_dsh(
    command: &[String],
    workdir: &str,
    stdout: Stdio,
    facts: &GitFacts,
) -> Result<(std::process::Child, std::thread::JoinHandle<String>), String> {
    let mut builder = Command::new(&command[0]);
    builder
        .args(&command[1..])
        .current_dir(if workdir.is_empty() { "." } else { workdir })
        // Seat commits are unsigned (CONTRIBUTING): the host's own
        // `commit.gpgsign` is outranked for every git call this seat
        // makes, and the signing wrapper and its key stay outside the
        // harness's sandbox.
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "commit.gpgsign")
        .env("GIT_CONFIG_VALUE_0", "false")
        .stdin(Stdio::null())
        .stdout(stdout)
        .stderr(Stdio::piped());
    // The seat commits under the host's identity, resolved outside the
    // sandbox the way the namespace box resolves it (decision 0043
    // ruling 6).
    for (key, value) in &facts.identity {
        builder.env(key, value);
    }
    let child = builder.spawn();
    let mut child = io_context(child, "could not invoke the agent CLI")?;
    let stderr_pipe = child.stderr.take().expect("piped");
    let stderr_thread = std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut pipe = stderr_pipe;
        let _ = pipe.read_to_end(&mut captured);
        String::from_utf8_lossy(&captured).into_owned()
    });
    Ok((child, stderr_thread))
}

/// The shipped cold route: stdout is `/dev/null` (the headless runner
/// prints only its final answer, which this driver has never journaled),
/// and the surviving signal is the JSONL transcript the persistence
/// plugin appends as the session runs.
fn invoke_dsh_shipped(
    command: &[String],
    workdir: &str,
    launch: &DshLaunch,
    mut wait: impl FnMut(&mut std::process::Child) -> std::io::Result<Option<i32>>,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    let (mut child, stderr_thread) = spawn_dsh(command, workdir, Stdio::null(), &launch.facts)?;
    let mut turns = 0u64;
    let mut tail = DshTail::default();
    let exit_code = poll_until_exit(
        || wait(&mut child),
        || {
            drain_dsh_transcript(
                &mut tail,
                &launch.root,
                launch.first_seq,
                &mut turns,
                session_meta,
                emit,
            )
        },
    )?;
    finish_dsh(session_meta, exit_code, stderr_thread)
}

/// The qualified `--output-format stream-json` exchange (task 8.8(d)):
/// stdout is the pinned plugin's two-event envelope, whose
/// post-`await agents.resume` init event is the ONLY root confirmation this
/// driver accepts. The retained transcript is folded alongside it, only
/// past the offered root's own sequence boundary, so a warm session never
/// re-counts its restored history.
#[allow(clippy::too_many_arguments)]
fn invoke_dsh_stream_json(
    command: &[String],
    launch: &DshLaunch,
    workdir: &str,
    watch: &mut DshRootWatch,
    hold: &mut LaunchHold,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
    observer: &mut impl FnMut(&DshObservation),
) -> Result<Invocation, String> {
    let (mut child, stderr_thread) = spawn_dsh(command, workdir, Stdio::piped(), &launch.facts)?;
    let stdout = child.stdout.take().expect("piped");
    let mut turns = 0u64;
    let mut tail = DshTail::default();
    for line in std::io::BufReader::new(stdout).lines() {
        // EVERY exit from a line has a disposition (design D7). A line is
        // decoded before the store behind it is observed, so a valid init
        // event is heard first: the session's current activity may already
        // be stored by the time this driver consumes the init that
        // preceded it. A line that could not be read ends the stream, and
        // one that is not JSON is skipped, exactly as before — but on a
        // rejoin neither leaves without the watcher's ruling on it, and
        // the settle behind the child's exit cannot rescue either.
        let Ok(line) = line else {
            watch.unread(DshStreamLine::Unreadable);
            watch.settle(
                DshStreamLine::Unreadable,
                hold,
                session_meta,
                emit,
                observer,
            );
            break;
        };
        let read = match serde_json::from_str::<Value>(&line) {
            Ok(event) => {
                fold_dsh_stream_event(&event, watch, hold, session_meta, emit);
                DshStreamLine::Event
            }
            Err(_) => {
                watch.unread(DshStreamLine::Malformed);
                DshStreamLine::Malformed
            }
        };
        // Every drain is preceded by the settle that could release the
        // hold, and NO drain happens while the hold is still closed. That
        // is both halves of design D7's order: a confirmed rejoin's
        // locator and launch rows reach the journal before the first work
        // row (task 7.4), and an unconfirmed one publishes no work row at
        // all — a fold running ahead of the confirmation would address a
        // session this driver cannot yet name, and an append landing
        // between the settle and the drain would put it there anyway.
        watch.settle(read, hold, session_meta, emit, observer);
        if read == DshStreamLine::Event && watch.may_fold() {
            drain_dsh_transcript(
                &mut tail,
                &launch.root,
                launch.first_seq,
                &mut turns,
                session_meta,
                emit,
            );
        }
    }
    let status = io_context(child.wait(), "agent CLI did not conclude")?;
    // A clean end of stream adds no confirmation fact and clears nothing:
    // this settle reads the store only while the confirmation is pending.
    watch.settle(DshStreamLine::End, hold, session_meta, emit, observer);
    if watch.may_fold() {
        drain_dsh_transcript(
            &mut tail,
            &launch.root,
            launch.first_seq,
            &mut turns,
            session_meta,
            emit,
        );
    }
    finish_dsh(session_meta, status.code().unwrap_or(-1), stderr_thread)
}

/// One line of the plugin's stream-json envelope. The init event names the
/// session the plugin actually opened after `agents.resume`; that is one
/// of the four facts the launch hold waits for, and on its own it is
/// nothing but a value the request asked for. The result envelope carries
/// no per-message boundary, so the transcript fold — not this event —
/// owns usage.
fn fold_dsh_stream_event(
    event: &Value,
    watch: &mut DshRootWatch,
    hold: &mut LaunchHold,
    session_meta: &mut Map<String, Value>,
    emit: &mut impl FnMut(&Value),
) {
    let is_init = event.get("type").and_then(Value::as_str) == Some("system")
        && event.get("subtype").and_then(Value::as_str) == Some("init");
    if !is_init {
        return;
    }
    let Some(id) = event
        .get("session_id")
        .and_then(Value::as_str)
        .filter(|id| !id.is_empty())
    else {
        return;
    };
    session_meta.insert("session_id".into(), Value::String(id.to_string()));
    watch.named(id, hold, emit);
}

/// Which reading of the stream-json child an observation sits behind. The
/// watcher treats a line it could not decode differently on either side of
/// the init event, so the kind travels with the observation it produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DshStreamLine {
    /// A line that decoded as JSON, init event or not.
    Event,
    /// A line that is not JSON.
    Malformed,
    /// A line the reader could not return at all (non-UTF-8 stdout).
    Unreadable,
    /// The settle behind the child's exit.
    End,
}

/// Where one observation left the confirmation (design D7's state table).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DshDisposition {
    /// A cold launch: no offer, so nothing to confirm or to contradict.
    Cold,
    /// Readable, consistent evidence that is not yet all four facts.
    Pending,
    /// Permanently refused: this invocation can no longer confirm.
    Refused,
    /// The child named a different root.
    Mismatched,
    /// All four facts agreed and the hold released.
    Confirmed,
}

/// One COMPLETED production observation: the exact census and boundary the
/// watcher consumed, and the disposition it reached from them. The
/// invocation's observer is handed it only after the decision and every
/// read of that snapshot are done, on every outcome, so a child that waits
/// on the observer cannot repair a store the watcher is still reading. A
/// reading the watcher did not take — it had already settled, or the line
/// itself refused — reports no census.
#[derive(Debug, Clone, PartialEq)]
struct DshObservation {
    line: DshStreamLine,
    census: Option<Vec<(String, std::path::PathBuf)>>,
    last_seq: Option<u64>,
    disposition: DshDisposition,
}

/// What the store the plan settled on held BEFORE the child spawned. It is
/// read once and never refreshed: a later census cannot supply history the
/// launch did not start with, and never becomes a new baseline.
struct DshBaseline {
    /// The boundary the prior depth-zero header produced (fact 1).
    first_seq: u64,
    /// The canonical address of the ONE header that named the offer.
    offered_file: std::path::PathBuf,
    /// Every admitted `(header id, canonical file)` occurrence, repeated
    /// ids and repeated addresses included.
    entries: Vec<(String, std::path::PathBuf)>,
}

/// The launch-root confirmation a DSH rejoin has to pass before ANYTHING
/// about its session is published (task 8.8(d), Pass C; design D6/D7).
///
/// The plugin's `session_id` is request-derived: it is the value this
/// driver asked for, echoed back, so by itself it confirms nothing. Four
/// mechanically observable facts have to agree before the hold releases:
///
/// 1. a valid prior depth-zero header retained at the resolved locator for
///    the offered id — read BEFORE the spawn (it is what produced
///    `first_seq`) and required to still be exactly one header, at that
///    same address;
/// 2. the pinned plugin's post-`await agents.resume` init event, read from
///    the stream-json child, naming that same root;
/// 3. no fresh depth-zero storage entry in the retained store — an entry
///    the store did not hold before the spawn is the plugin opening a NEW
///    session instead of rejoining the offered one. Entries are counted by
///    `(header id, canonical file)` occurrence and never by distinct id: a
///    second `session-9` at a new address is a new session, whatever it
///    calls itself;
/// 4. new sequence activity past the recorded `firstSeq` in that same
///    root.
///
/// Same-root nonce continuity is 10.7's probe-only model-recall device
/// (design D6): it is never planted in a prompt and never read here.
///
/// Confirmation is a LATCH, not a predicate that may be asked again until
/// it answers yes. Evidence that is readable and consistent but incomplete
/// waits. Evidence that contradicts the offered root — or a required
/// reading that could not be taken — refuses the invocation for good, on
/// either side of the init event: no later store, init event, clean exit
/// or delivered result file reopens it.
///
/// Until all four agree this publishes nothing at all — no transcript
/// locator, no launch row, no `root_session` AND no folded work row — so
/// a missing or different root followed by a clean exit, or by an
/// otherwise valid delivered result file, stays failed or indeterminate
/// under D7 and authorizes no cold replacement by itself.
///
/// The work row is the half the ordering turns on. The fold reads the
/// retained store, which the child is writing as the stream is read, so
/// leaving it ungated would let a child drain its work into the journal
/// while the hold was still closed and then emit the init event behind
/// it — publishing the locator and launch row AFTER their own work rows,
/// and returning a confirmed rejoin built on work that was never
/// confirmed. `may_fold` withholds the fold until the hold releases.
struct DshRootWatch {
    /// The exact root this launch was built to rejoin. `None` is a cold
    /// launch, which has no offer to confirm and publishes as it always
    /// did.
    offered: Option<String>,
    /// The retained root the plan settled on, and the only store this
    /// confirmation reads.
    root: std::path::PathBuf,
    /// The pre-spawn store. Its absence beside an offer — no boundary, a
    /// store this driver could not census, or anything but exactly one
    /// header naming the offer — is missing fact 1, which no later reading
    /// can supply: that launch is refused before it spawns.
    baseline: Option<DshBaseline>,
    /// The locator row, held until confirmation on a rejoin.
    locator: String,
    transcript: Transcript,
    /// Whether the child's init event named the offered root.
    named_the_offer: bool,
    /// Latched once the outcome is settled: a confirmation publishes once,
    /// and a refusal or a mismatch is not revisited by a later reading.
    /// Nothing clears it.
    settled: bool,
    /// Latched only by the settle that released the hold, so `released`
    /// implies `settled`. A rejoin folds its transcript from here and
    /// never before: until this is true the driver cannot say which
    /// session a work row would belong to.
    released: bool,
    /// Whether the settled outcome is the hold's mismatch, which keeps its
    /// own terminal reason.
    mismatched: bool,
}

impl DshRootWatch {
    /// Open the watch over a settled launch, censusing the retained store
    /// BEFORE the child can touch it. A census is taken only where there
    /// is an offer to confirm; a cold launch reads nothing.
    fn new(launch: &DshLaunch, transcript: Transcript) -> DshRootWatch {
        let offered = launch
            .stream_json
            .then(|| launch.rejoining.clone())
            .flatten();
        let baseline = offered.as_deref().and_then(|offered| {
            let first_seq = launch.first_seq?;
            let entries = dsh_depth_zero_sessions(&launch.root).ok()?;
            let offered_file = dsh_only_header(&entries, offered)?.clone();
            Some(DshBaseline {
                first_seq,
                offered_file,
                entries,
            })
        });
        let mut watch = DshRootWatch {
            offered,
            root: launch.root.clone(),
            baseline,
            locator: launch.locator.clone(),
            transcript,
            named_the_offer: false,
            settled: false,
            released: false,
            mismatched: false,
        };
        if watch.confirming() && watch.baseline.is_none() {
            watch.refuse();
        }
        watch
    }

    /// Whether this launch is holding its locator and launch rows for a
    /// confirmation. A cold launch is not, and publishes before the spawn
    /// exactly as it always has.
    fn confirming(&self) -> bool {
        self.offered.is_some()
    }

    /// Whether the retained transcript may be folded into the journal yet.
    /// A cold launch always may. A rejoin may only once its confirmation
    /// released the hold: work published before that would name a session
    /// this driver has not established it is in, and would reach the
    /// journal ahead of the locator and launch rows D7 orders in front of
    /// it.
    fn may_fold(&self) -> bool {
        !self.confirming() || self.released
    }

    /// The ONE absorbing refusal: settled, never released. The hold keeps
    /// no outcome, so the attempt ends `Unconfirmed` — no locator, no
    /// launch row, no `root_session`, and no cold replacement authorized.
    /// Every contradiction and every required reading that could not be
    /// taken arrives here, and nothing leaves.
    fn refuse(&mut self) {
        self.settled = true;
    }

    /// Where the confirmation stands, for the observation that reports it.
    fn disposition(&self) -> DshDisposition {
        match (self.confirming(), self.settled, self.released) {
            (false, ..) => DshDisposition::Cold,
            (true, false, _) => DshDisposition::Pending,
            (true, true, true) => DshDisposition::Confirmed,
            (true, true, false) if self.mismatched => DshDisposition::Mismatched,
            (true, true, false) => DshDisposition::Refused,
        }
    }

    /// Publish the retained locator row, and hand the launch hold the
    /// exact address it recorded. DSH is the one built-in provider whose
    /// session identifier is not its own locator: a root is rejoinable
    /// only through the retained directory it was opened in, so the
    /// launch row that confirms the root carries that address on the same
    /// row (design D6).
    fn record_locator(
        &mut self,
        hold: &mut LaunchHold,
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
    ) {
        let locator = self.locator.clone();
        let address = self.transcript.record(&locator, session_meta, emit);
        hold.address(address);
    }

    /// The child named a session. A DIFFERENT one settles at once —
    /// there is nothing left to wait for and nothing to publish, and the
    /// hold latches the mismatch — while the offered one still has to
    /// survive the retained-store reads below. A settled watch hears
    /// nothing: an init event cannot reset a refusal or a mismatch.
    fn named(&mut self, id: &str, hold: &mut LaunchHold, emit: &mut impl FnMut(&Value)) {
        if self.settled {
            return;
        }
        if self.offered.as_deref() != Some(id) {
            // A cold plan's own fresh root, or a rejoin's mismatch. The
            // hold decides which, and a mismatch publishes nothing.
            self.mismatched = hold.confirm(id, emit) == Confirmation::Mismatch;
            self.settled = true;
            return;
        }
        self.named_the_offer = true;
    }

    /// One stream line the driver could not decode, or could not read at
    /// all.
    ///
    /// Before the init event such a line is output this driver could not
    /// read ahead of the confirmation. It may have been anything, and an
    /// unread line is never a satisfied one: it refuses, whether or not
    /// the store has visibly moved, because a store that has not moved YET
    /// says nothing about what the child does behind the line. A line the
    /// reader could not return at all refuses on either side of the init
    /// event — the stream ends there, and the settle behind the child's
    /// exit must not rescue what was never read. After the init event a
    /// malformed line is noise that supplies no fact, and the settle
    /// behind it still reads the store.
    ///
    /// A cold launch has no offer to contradict and keeps its noise
    /// handling exactly as it was.
    fn unread(&mut self, line: DshStreamLine) {
        if self.settled || !self.confirming() {
            return;
        }
        if line == DshStreamLine::Unreadable || !self.named_the_offer {
            self.refuse();
        }
    }

    /// Take one required-store observation, and release the hold if — and
    /// only if — all four facts agree. Called before every transcript
    /// drain, so the locator and launch rows reach the journal ahead of
    /// the first work row (design D7's order). The observer is told what
    /// the reading consumed once it is complete, on every outcome.
    fn settle(
        &mut self,
        line: DshStreamLine,
        hold: &mut LaunchHold,
        session_meta: &mut Map<String, Value>,
        emit: &mut impl FnMut(&Value),
        observer: &mut impl FnMut(&DshObservation),
    ) {
        let mut observation = DshObservation {
            line,
            census: None,
            last_seq: None,
            disposition: self.disposition(),
        };
        let reading = match (&self.offered, &self.baseline) {
            (Some(offered), Some(baseline)) if !self.settled => Some((
                offered.clone(),
                dsh_read_offered_store(
                    &self.root,
                    offered,
                    baseline,
                    self.named_the_offer,
                    &mut observation,
                ),
            )),
            _ => None,
        };
        if let Some((offered, reading)) = reading {
            match reading {
                DshDisposition::Pending => {}
                DshDisposition::Confirmed => {
                    self.settled = true;
                    self.released = true;
                    // D7's order: the held location fact, then the launch
                    // row, then the first work checkpoint the drain behind
                    // this call emits. The launch row carries that same
                    // address, so the root and the two coordinates a
                    // rejoin needs are one checkpoint (design D6).
                    self.record_locator(hold, session_meta, emit);
                    hold.confirm(&offered, emit);
                }
                _ => self.refuse(),
            }
            observation.disposition = self.disposition();
        }
        observer(&observation);
    }
}

/// The one required-store rule, on either side of the init event.
///
/// Each step is a reading that has to be TAKEN and has to AGREE. One that
/// cannot be taken — a store this driver cannot census, a boundary the
/// reader refuses whole rather than reporting a lower maximum — refuses
/// exactly as a contradiction does. Dropping it would let the child finish
/// the row, put the store back and present a snapshot that agrees, when the
/// snapshot that could have refused is the one already taken.
///
/// 1. The complete admitted census.
/// 2. Exactly one current header naming the offer, at the address the
///    baseline retained for it. Cardinality comes first, so an ambiguous
///    offer is refused as ambiguous and never resolved to its first match.
/// 3. Every current `(id, file)` occurrence consumes one baseline
///    occurrence of its own. A new address, a new identity at an old
///    address or one occurrence too many refuses; an unrelated old sibling
///    that is gone does not, because the rule is containment and not
///    whole-store equality.
/// 4. The offered sequence, read once. Before the init event, activity
///    past the boundary is work no confirmed rejoin produced — the pinned
///    plugin emits its init event immediately after `await agents.resume`,
///    ahead of the session's first current turn — and the init event
///    behind it cannot adopt it. After the init event it is fact 4.
fn dsh_read_offered_store(
    root: &std::path::Path,
    offered: &str,
    baseline: &DshBaseline,
    named_the_offer: bool,
    observation: &mut DshObservation,
) -> DshDisposition {
    let Ok(sessions) = dsh_depth_zero_sessions(root) else {
        return DshDisposition::Refused;
    };
    observation.census = Some(sessions.clone());
    let Some(file) = dsh_only_header(&sessions, offered) else {
        return DshDisposition::Refused;
    };
    if *file != baseline.offered_file {
        return DshDisposition::Refused;
    }
    if !dsh_census_within(&sessions, &baseline.entries) {
        return DshDisposition::Refused;
    }
    observation.last_seq = dsh_session_last_seq(file);
    let Some(last) = observation.last_seq else {
        return DshDisposition::Refused;
    };
    match (last > baseline.first_seq, named_the_offer) {
        (false, _) => DshDisposition::Pending,
        (true, true) => DshDisposition::Confirmed,
        (true, false) => DshDisposition::Refused,
    }
}

/// The address of the ONE header in a census that names `id`, or `None`
/// when there is none or more than one. An ambiguous id is never resolved
/// to whichever occurrence the enumeration happened to reach first.
fn dsh_only_header<'a>(
    census: &'a [(String, std::path::PathBuf)],
    id: &str,
) -> Option<&'a std::path::PathBuf> {
    let mut named = census.iter().filter(|(header, _)| header == id);
    match (named.next(), named.next()) {
        (Some((_, file)), None) => Some(file),
        _ => None,
    }
}

/// Whether every `(header id, canonical file)` occurrence in `current`
/// consumes an occurrence of its own in `baseline`: sub-multiset
/// containment. A set of ids cannot see a repeated id at a new address,
/// and a set of pairs cannot see one pair admitted twice, so each baseline
/// occurrence is spent at most once.
fn dsh_census_within(
    current: &[(String, std::path::PathBuf)],
    baseline: &[(String, std::path::PathBuf)],
) -> bool {
    let mut unspent: Vec<&(String, std::path::PathBuf)> = baseline.iter().collect();
    current.iter().all(|entry| {
        let held = unspent.iter().position(|held| *held == entry);
        held.map(|at| unspent.swap_remove(at)).is_some()
    })
}

/// The common tail of both dsh invocations: the harness and profile the
/// lane always is, and the redacted stderr. No pin lands in `session_meta`:
/// `model` there is only ever what the transcript said served (decision
/// 0031).
fn finish_dsh(
    session_meta: &mut Map<String, Value>,
    exit_code: i32,
    stderr_thread: std::thread::JoinHandle<String>,
) -> Result<Invocation, String> {
    session_meta.insert("harness".into(), Value::String("deepseek".into()));
    session_meta.insert("profile".into(), Value::String("headless".into()));
    // Decision 0053: dsh's headless profile makes no machine-readable
    // pre-session refusal available. Its stdout is the final answer or
    // nothing, and a provider rejection reaches this driver only as
    // stderr prose plus a non-zero exit — sniffing that prose to make a
    // control decision is exactly the repair decision 0001 forbids. So
    // dsh classifies nothing here and a refusal before its first turn
    // follows decision 0006 unchanged; the guide says so beside claude
    // and codex.
    let stderr = redact_dsh_reasoning(&stderr_thread.join().unwrap_or_default());
    // The promotion that moves the seat's commits out of the private
    // store happens in `invoke_dsh_with`, which owns the store for the
    // seat's whole life and is the one place both routes return through.
    Ok(Invocation {
        exit_code,
        session_meta: session_meta.clone(),
        stdout: String::new(),
        stderr,
        state: None,
        refusal: None,
        launch: LaunchTerminal::Cold,
    })
}

/// A driver failure that reaches a seat BEFORE its promotion keeps the
/// private store and names it, exactly as a promotion that cannot happen
/// does: the store holds the only copy of the seat's commits until the
/// promotion moves them. A seat with no scoped store has nothing to lose,
/// so its failure travels unchanged.
fn dsh_failure_before_promotion(
    problem: String,
    staged: Option<(String, dsh_sandbox::GitScope, dsh_sandbox::SeatGitStore)>,
) -> String {
    match staged {
        Some((_, _, staged)) => dsh_sandbox::keep_store(staged, problem),
        None => problem,
    }
}

/// The one dsh stderr stream the journal may not quote.
///
/// dsh 0.1.2-rc.1's headless profile streams the model's reasoning to
/// stderr under a `dsh: reasoning:` line (measured 2026-09-04: one
/// header, then the raw thinking text, until the harness's next `dsh: `
/// line or the end of the stream). The driver's stderr tail is what a
/// parked seat quotes into the journal, and a journal admits no
/// reasoning text (decisions 0032 and 0034). So a reasoning block is
/// replaced by one line that says it was there, and every harness line
/// survives, because those are what a park needs to be read.
fn redact_dsh_reasoning(stderr: &str) -> String {
    const HEADER: &str = "dsh: reasoning:";
    const REDACTED: &str = "dsh: reasoning: [not journaled — decision 0034]";
    let mut kept = Vec::new();
    let mut inside = false;
    for line in stderr.lines() {
        if line.trim_end() == HEADER {
            inside = true;
            kept.push(REDACTED);
        } else if line.starts_with("dsh: ") {
            inside = false;
            kept.push(line);
        } else if !inside {
            kept.push(line);
        }
    }
    let mut text = kept.join("\n");
    if stderr.ends_with('\n') {
        text.push('\n');
    }
    text
}

fn invoke(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
    session: Option<&str>,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
) -> Result<Invocation, String> {
    invoke_with_stager(
        kind,
        extra,
        prompt,
        input,
        session,
        bindings,
        emit,
        stage_prompt,
    )
}

#[allow(clippy::too_many_arguments)]
fn invoke_with_stager(
    kind: AdapterKind,
    extra: &[String],
    prompt: &str,
    input: &Value,
    session: Option<&str>,
    bindings: &[secret::BoundSecret],
    emit: &mut impl FnMut(&Value),
    mut stage: impl FnMut(&str) -> Result<tempfile::NamedTempFile, String>,
) -> Result<Invocation, String> {
    let workdir = input
        .get("workdir")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    match kind {
        AdapterKind::Claude => {
            let bin = adapter_binary("BROKKR_CLAUDE_BIN", Some("FORGE_CLAUDE_BIN"), "claude");
            let plan = claude_launch(&bin, extra, session, input, CLAUDE_SHAPE, None)?;
            let command = plan.command.clone();
            let mut hold = LaunchHold::new("claude", plan);
            let mut invocation = invoke_stream_json(&command, prompt, &workdir, &mut hold, emit)?;
            hold.finish(emit);
            invocation.launch = hold.terminal();
            Ok(invocation)
        }
        // Same harness, same stream: LaneTally's wrapper is
        // argv-compatible with claude (including stream-json), so the
        // only difference IS the binary. No spawn-time fallback to plain
        // `claude` when the wrapper is missing — that would silently
        // un-capture sessions; doctor is the advisory surface, and
        // substituting plain claude to make a resume work would be the
        // same silent un-capture one ruling later (proposed decision
        // 0056 ruling 5: a wrapper is qualified on its own wrapper).
        AdapterKind::Lanetally => {
            let bin = adapter_binary(
                "BROKKR_LANETALLY_BIN",
                Some("FORGE_LANETALLY_BIN"),
                "claude-lanetally",
            );
            let plan = claude_launch(&bin, extra, session, input, LANETALLY_SHAPE, None)?;
            let command = plan.command.clone();
            let mut hold = LaunchHold::new("claude", plan);
            let mut invocation = invoke_stream_json(&command, prompt, &workdir, &mut hold, emit)?;
            hold.finish(emit);
            invocation.launch = hold.terminal();
            Ok(invocation)
        }
        AdapterKind::Codex => {
            let bin = adapter_binary("BROKKR_CODEX_BIN", Some("FORGE_CODEX_BIN"), "codex");
            let plan = codex_launch(&bin, extra, &workdir, session, input)?;
            let command = plan.command.clone();
            let mut hold = LaunchHold::new("codex", plan);
            let mut invocation = invoke_codex(&command, prompt, &workdir, &mut hold, emit)?;
            hold.finish(emit);
            invocation.launch = hold.terminal();
            // Ruling 8's ONE pre-work replacement, and only on evidence
            // that no session opened at all: the rejoin was never
            // confirmed, no row proved the harness began work, the
            // process ended non-zero, and no result file was delivered.
            // A generic nonzero exit, stderr prose, missing telemetry or
            // elapsed time proves none of that on its own, which is why
            // all four terms are here.
            //
            // "Never confirmed" is load-bearing and measured, not
            // assumed: a successful `codex exec resume --json` emits
            // `thread.started` carrying the SAME thread id it was
            // handed, before any turn begins. So a rejoin that actually
            // ran can never reach this arm, whatever it exits with, and
            // the seat is never charged twice for one attempt.
            if hold.unconfirmed_rejoin()
                && !hold.began_work
                && invocation.exit_code != 0
                && !delivered_result(input)
            {
                // The rejected child's candidates go with it: a
                // replacement inherits none of its launch, root,
                // locator or accounting. The cold argv reuses `extra`
                // unchanged because `codex_launch` already validated
                // exactly these immutable arguments — the selector and
                // incompatible-argv guards ran on them before the first
                // spawn — so a second builder guard here would repeat a
                // check that cannot have become false.
                let cold = LaunchPlan::cold(
                    codex_cold(&bin, extra, &workdir, &codex_managed(input)),
                    "codex-thread",
                    Some("harness-refused"),
                );
                let command = cold.command.clone();
                let mut replacement = LaunchHold::new("codex", cold);
                let mut outcome = invoke_codex(&command, prompt, &workdir, &mut replacement, emit)?;
                replacement.finish(emit);
                outcome.launch = replacement.terminal();
                // No recursion: a failed replacement reports its own
                // outcome, whatever that is.
                return Ok(outcome);
            }
            Ok(invocation)
        }
        AdapterKind::Dsh => invoke_dsh(extra, prompt, &workdir, input, session, emit),
        AdapterKind::Exec => {
            if extra.is_empty() {
                return Err("exec driver needs a command template after '--'".to_string());
            }
            // Exec has no model turn and therefore no tool target in the
            // seat record. Its unresolved command remains pinned in the
            // bundle manifest; no command-shaped prose enters a checkpoint.
            let mut session_meta = Map::new();
            Transcript::resolve(TranscriptKind::None)?.finish(&mut session_meta, emit);
            emit(&json!({"step": "exec-started"}));
            let mut prompt_file: Option<tempfile::NamedTempFile> = None;
            if extra.iter().any(|part| part.contains("{prompt_file}")) {
                prompt_file = Some(stage(prompt)?);
            }
            let prompt_path = prompt_file
                .as_ref()
                .map(|f| f.path().to_string_lossy().into_owned())
                .unwrap_or_default();
            let command: Vec<String> = extra
                .iter()
                .map(|part| resolve_exec_part(part, &workdir, &prompt_path, bindings))
                .collect();
            let stdin_payload = if prompt_file.is_none() {
                Some(prompt)
            } else {
                None
            };
            let out = run_cli(&command, stdin_payload, &workdir, bindings)?;
            // Known-plaintext masking choke point (decision 0012, layer
            // 5), on RAW captured bytes before any string conversion.
            // stdout is captured-and-dropped today; stderr is re-emitted
            // by run_seat and journaled as the stderr tail on failure —
            // the path that fires exactly when a credentialed command
            // prints the offending header.
            let stderr = secret::mask_bytes(&out.stderr, bindings);
            let stdout = secret::mask_bytes(&out.stdout, bindings);
            let state = input
                .pointer("/dialect_exec/state")
                .and_then(Value::as_array)
                .filter(|argv| !argv.is_empty())
                .map(|argv| {
                    let argv = argv
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect::<Vec<_>>();
                    run_cli(&argv, None, &workdir, bindings).map(|output| {
                        let bytes = secret::mask_bytes(&output.stdout, bindings);
                        String::from_utf8_lossy(&bytes).into_owned()
                    })
                })
                .transpose()?;
            Ok(Invocation {
                exit_code: out.status.code().unwrap_or(-1),
                session_meta,
                stdout: String::from_utf8_lossy(&stdout).into_owned(),
                stderr: String::from_utf8_lossy(&stderr).into_owned(),
                state,
                // Decision 0053: exec has no model turn and no provider
                // to refuse it. Its failures are the script's own, and
                // an inline exec site carries an empty chain anyway, so
                // there is no fallback for a refusal to reach.
                refusal: None,
                launch: LaunchTerminal::Cold,
            })
        }
    }
}

/// The provider row dsh's headless profile boots its agent on when the
/// pinned model names none. A patch overlay replaces the targeted row's
/// WHOLE config (dsh-base's own words), so the overlay that pins a
/// model must restate the provider or the boot loses it.
const DSH_PROVIDER: &str = "deepseek-official";

/// One pinned model, as dsh addresses it: a provider route in the
/// profile tree and a model id that route serves. `<id>` alone is the
/// official DeepSeek route; `<provider>/<id>` names another route the
/// profile declares — `dashscope/qwen3.8-max` for Model Studio. The
/// split is on the FIRST slash, exactly as decision 0036 ruling 2 reads
/// a concrete id: the route is the first segment and everything after
/// it is the id the route serves. That id may carry slashes of its own,
/// because an aggregator's catalogue names models as `<vendor>/<name>`
/// — `meta-contributor/meta/muse-spark-1.3-contributor` is the
/// `meta-contributor` route serving OpenRouter's `meta/muse-spark-…` —
/// and the driver has no business reading a vendor's naming as a
/// second route. Every segment is one plain identifier and none is
/// empty, so nothing that reaches the YAML overlay can open a row.
struct DshModel<'a> {
    provider: &'a str,
    model: &'a str,
}

fn parse_dsh_model(pinned: &str) -> Result<DshModel<'_>, String> {
    let plain = |part: &str| {
        !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':'))
    };
    let (provider, model) = match pinned.split_once('/') {
        Some((provider, model)) => (provider, model),
        None => (DSH_PROVIDER, pinned),
    };
    if !plain(provider) || !model.split('/').all(plain) {
        return Err(
            "dsh driver: the pinned model is not `<id>` or `<provider>/<id>` of plain \
             identifiers (the id may carry slashes between plain segments)"
                .to_string(),
        );
    }
    Ok(DshModel { provider, model })
}

/// The dsh launcher takes no model flag: the model is one row of the
/// composed profile tree (`agent-default-model`), and the launcher's
/// only override channel is `--patch <overlay.yml>`, applied last. So
/// the adapter data says `model_flag: "--model"` — the pinning grammar
/// every provider shares — and this driver is where `--model <id>`
/// becomes the overlay dsh actually reads. Everything after `--` that
/// is not that pair passes through to the launcher unchanged.
fn split_dsh_model(extra: &[String]) -> Result<(Option<String>, Vec<String>), String> {
    let mut model = None;
    let mut passthrough = Vec::with_capacity(extra.len());
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        if part == "--model" {
            let id = parts
                .next()
                .ok_or_else(|| "dsh driver: --model needs a model id after it".to_string())?;
            if id.is_empty() || id.starts_with('-') {
                return Err("dsh driver: --model needs a model id after it".to_string());
            }
            if model.replace(id.clone()).is_some() {
                return Err("dsh driver: --model given twice".to_string());
            }
        } else if part.starts_with("--model") {
            // Only the separate `--model <id>` spelling is admitted; a
            // joined spelling is refused before any provider observation.
            return Err("dsh driver: --model needs a model id after it".to_string());
        } else {
            passthrough.push(part.clone());
        }
    }
    Ok((model, passthrough))
}

/// Each authorized DSH control's value slot, read in the argv the SEAT
/// actually wrote. The three extractions below run in sequence, and each
/// removes its own flag with its value before the next one looks; so a
/// control that stood in a LATER control's value slot disappears by the
/// time that later control is split, and what was positional text behind
/// it slides into the emptied slot. `--effort --model <id> high` would
/// then read as a valid level, and `--patch --model <id> <path>` or
/// `--patch --effort[=]<level> <path>` as a valid overlay path — three
/// arguments admitted from an argv that never offered any of them.
///
/// So DSH-local admission checks the original adjacency first: a value
/// slot occupied by a flag-shaped token is refused by the field that
/// owns the slot, before any extraction, route read, version probe,
/// composite call or staging. The shared effort splitter is untouched
/// and keeps its behaviour for every other adapter; this pass only
/// refuses argv the DSH arm must never admit. The fixed field never
/// echoes the occupying token, its joined value, a model or a path
/// (AS3; tasks 8.8(d)/8.10).
///
/// A slot with nothing in it at all is not this pass's business: a bare
/// `--model` or `--patch` is refused by arity in its own splitter, and a
/// bare `--effort` stays in the argv as the residual the shared splitter
/// declines to drop in silence.
fn dsh_input_boundaries(extra: &[String]) -> Result<(), String> {
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        let refusal = match part.as_str() {
            "--model" => "dsh driver: --model needs a model id after it",
            "--patch" => "dsh driver: --patch needs an overlay path after it",
            "--effort" => "dsh driver: --effort needs a level after it",
            // Every other spelling — a joined control, a residual, a
            // value — claims no separate slot, so the walk moves on by
            // one and the splitters and the residual rule judge it.
            _ => continue,
        };
        // The one part after the flag is its value. It is consumed here
        // either way: a control's value is never re-read as a control.
        if parts.next().is_some_and(|value| value.starts_with('-')) {
            return Err(refusal.to_string());
        }
    }
    Ok(())
}

/// Split the seat's single `--patch` value out of the passthrough. One
/// exact `--patch` with a non-empty, non-flag value is the only admitted
/// shape; a second `--patch`, a bare `--patch` and any other `--patch…`
/// spelling are refused by arity before staging, never forwarded and never
/// dropped to make the launch admissible (AS3; design D6 mechanism 1).
fn split_dsh_patch(extra: &[String]) -> Result<(Option<String>, Vec<String>), String> {
    let mut route = None;
    let mut passthrough = Vec::with_capacity(extra.len());
    let mut parts = extra.iter();
    while let Some(part) = parts.next() {
        if part == "--patch" {
            let value = parts
                .next()
                .ok_or_else(|| "dsh driver: --patch needs an overlay path after it".to_string())?;
            // A flag-shaped value is refused HERE, by arity, exactly as
            // `split_dsh_model` refuses one: any leading `-`, not only a
            // long option's `--`. A single-dash launcher spelling taken
            // as the overlay path would otherwise be carried past the
            // admission rule into the route read (AS3; task 8.8(d)).
            if value.is_empty() || value.starts_with('-') {
                return Err("dsh driver: --patch needs an overlay path after it".to_string());
            }
            if route.replace(value.clone()).is_some() {
                return Err("dsh driver: --patch given twice".to_string());
            }
        } else if part.starts_with("--patch") {
            return Err(
                "dsh driver: only the one separate `--patch <overlay>` spelling is admitted"
                    .to_string(),
            );
        } else {
            passthrough.push(part.clone());
        }
    }
    Ok((route, passthrough))
}

/// The pinned-model row of the seat overlay, in the loader-patch
/// grammar dsh composes after every bundle and profile layer, staged
/// over an injected file so the two ways staging can fail — no file, a
/// file that takes no bytes — are reachable from a test without a full
/// disk. The id is written into YAML verbatim, so it is confined to the
/// characters a model id is made of — a model name is data the operator
/// pinned, never a place to smuggle a second row into the tree.
/// The standalone model overlay stage, used by the staging-failure tests:
/// production composes the model row into the one per-seat overlay beside
/// the route rows.
#[cfg(test)]
fn dsh_model_overlay_in(
    model: &str,
    create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
) -> Result<tempfile::NamedTempFile, String> {
    let body = dsh_model_row(model)?;
    let mut file = io_context(create(), "could not stage the dsh model overlay")?;
    io_context(
        file.write_all(body.as_bytes()),
        "could not write the dsh model overlay",
    )?;
    Ok(file)
}

/// The pinned-model row alone, so the one per-seat overlay can place the
/// validated route rows ahead of it (design D6 mechanism 1; AS3).
fn dsh_model_row(model: &str) -> Result<String, String> {
    let DshModel { provider, model } = parse_dsh_model(model)?;
    Ok(format!(
        "# Written by `brokkr driver dsh` for one seat: the pinned model, as the\n\
         # overlay dsh's launcher composes last. A patch replaces the targeted\n\
         # row's whole config, so the provider is restated beside the model.\n\
         - id: agent-default-model\n\
         \x20 config:\n\
         \x20   provider: {provider}\n\
         \x20   model: {model}\n"
    ))
}

/// A DSH storage refusal: its own fixed category, and the errno text the
/// HOST words the failure with — never the path that was tried.
///
/// The shared `io_context` interpolates the whole `io::Error`, and every
/// place this driver allocates storage allocates it beneath the operator's
/// own layout: the seat overlay and its settings document under `TMPDIR`,
/// the retained transcript root under the admitted DSH home. `tempfile`
/// reports a failed allocation as `<errno> at path "<directory>"`, so
/// passing that error through published the operator's temporary root or
/// harness home into a `Result.error` the seat reads back (AS3;
/// tasks 8.8(d)/8.10).
///
/// The errno arrives from the host's own `std::io::Error` rather than as a
/// literal number: the raw OS error where the host gave one, else the
/// kind's own words, which is what a wrapped error keeps. The categories
/// are unchanged — this replaces only what follows them.
fn dsh_storage_context<T>(result: std::io::Result<T>, context: &str) -> Result<T, String> {
    result.map_err(|error| {
        let errno = match error.raw_os_error() {
            Some(code) => std::io::Error::from_raw_os_error(code),
            None => std::io::Error::from(error.kind()),
        };
        format!("{context}: {errno}")
    })
}

/// The root over an injected creator, so the one way staging can fail
/// is reachable from a test without a full disk — and without a test
/// moving the harness home out from under every other test in the
/// process.
fn dsh_transcript_root_in(
    create: impl FnOnce() -> std::io::Result<std::path::PathBuf>,
) -> Result<std::path::PathBuf, String> {
    dsh_storage_context(create(), "could not stage the dsh session transcript root")
}

/// The overlay row that points dsh's session persistence at this seat's
/// own root, in the encoding an external line reader can follow. The
/// path is written as a single-quoted YAML scalar with its quotes
/// doubled, and a path that could open a line of its own is refused
/// rather than written — same discipline as the model id.
///
/// Accepted by the installed 0.1.0-rc.6 and re-measured on 0.1.2-rc.1
/// (2026-09-04: without `compression: none` that release writes
/// `session.jsonl.zstd`; with this row, the generation-addressed plain
/// file below): `dsh --profile headless --dump-config` composes this row
/// over the `session-persistence-jsonl` id that `@deepseek-ai/dsh-base`
/// already contributes (whose only default is `root:
/// dshHomePath('sessions')`), and a real headless session run with it
/// wrote a plain-text `<root>/--<cwd>--/session-<uuid>/session.v3.jsonl`
/// on the selected `0.1.5-rc.1` core — the layout `find_dsh_transcript`
/// walks and `DSH_TRANSCRIPT` names.
fn dsh_transcript_row(root: &std::path::Path) -> Result<String, String> {
    let root = root.to_string_lossy();
    if root.contains('\n') || root.contains('\r') {
        // The field, never the path: this root is composed beneath the
        // admitted DSH home, so echoing it hands the seat the operator's
        // home name and harness layout in a Result.error it can read
        // (AS3; tasks 8.8(d)/8.10).
        return Err(
            "dsh driver: the seat's transcript root spans more than one line, so it cannot be \
             written as one overlay scalar"
                .to_string(),
        );
    }
    Ok(format!(
        "# Written by `brokkr driver dsh` for one seat: this seat's session\n\
         # transcript, raw and line-readable, under a root only this seat\n\
         # writes — so the driver follows the right session by construction\n\
         # and never by scanning a shared directory for the newest file.\n\
         - id: session-persistence-jsonl\n\
         \x20 config:\n\
         \x20   root: '{}'\n\
         \x20   compression: none\n\
         \x20   packChunks: false\n",
        root.replace('\'', "''")
    ))
}

/// One overlay per seat carrying every row this driver pins: the
/// transcript root always, the model when one is pinned, and — when an
/// effort is pinned beside it — the settings row, pointed at a settings
/// document that is this seat's own. It is ONE patch file because
/// `--patch` is the launcher's only override channel and the seat's
/// argv stays as narrow as it was; the settings document is a second
/// file only because dsh reads a level from its settings layer and from
/// nowhere else.
#[derive(Debug)]
struct DshSeatOverlay {
    patch: tempfile::NamedTempFile,
    /// Held for the child's lifetime, never read back by this driver:
    /// dsh reads the document live, and a path that vanished mid-seat
    /// would be a level that vanished with it.
    #[allow(dead_code)]
    settings: Option<tempfile::NamedTempFile>,
}

impl DshSeatOverlay {
    fn path(&self) -> &std::path::Path {
        self.patch.path()
    }
}

/// The git metadata a dsh `workspace-write` seat cannot reach, resolved
/// by the trusted driver before the seat starts (decision 0054). `None`
/// when the workspace's git directory already sits inside the writable
/// root, when the mode needs no writes, or when the workspace is not a
/// repository. The session cwd — not the repository toplevel — is the
/// provider's writable root, so a git directory outside it is the defect
/// whether the seat's cwd is a linked worktree or a subdirectory.
///
/// A scope this returns is not automatically served: `dsh_sandbox_row_for`
/// asks `scope_refusal` first, because only a linked worktree Git itself
/// created can be bound without widening the boundary (decision 0054
/// ruling 3).
fn dsh_git_runner_scope(
    workdir: &str,
    facts: &GitFacts,
    mode: &str,
) -> Option<dsh_sandbox::GitScope> {
    let common = facts.common_dir.as_ref()?;
    // The git paths are absolute; a relative `--repo .` resolves against
    // the driver's cwd, which is the workspace itself. Both sides are
    // canonicalized so a symlinked or nested spelling agrees with the
    // workspace dsh itself canonicalizes for its profile.
    let workspace = std::path::absolute(workdir).unwrap_or_else(|_| PathBuf::from(workdir));
    let workspace = std::fs::canonicalize(&workspace).unwrap_or(workspace);
    let common = std::fs::canonicalize(common).unwrap_or_else(|_| common.clone());
    if common.starts_with(&workspace) {
        return None;
    }
    if mode == "read-only" || mode == "danger-full-access" {
        return None;
    }
    let git_dir = facts.git_dir.clone().unwrap_or_else(|| common.clone());
    Some(dsh_sandbox::GitScope {
        workspace,
        git_dir: std::fs::canonicalize(&git_dir).unwrap_or(git_dir),
        common_dir: common,
    })
}

/// The scoped-runner row for one seat, the scope it was written for and
/// the host state it names, or `None` when the seat's git metadata
/// already sits inside the writable workspace. A seat whose linked
/// worktree cannot reach its git directory refuses here, before it spends
/// an implementation.
///
/// The caller must hold the staged store for the seat's whole life:
/// dropping it unlinks the private common directory every command in the
/// seat is writing into, and the empty file they mount over `config` and
/// `config.worktree`. It is handed to `promote_seat_commits` afterwards,
/// which is how the seat's branch reaches the shared repository.
fn dsh_sandbox_row_for(
    workdir: &str,
    facts: &GitFacts,
    mode: &str,
) -> Result<Option<(String, dsh_sandbox::GitScope, dsh_sandbox::SeatGitStore)>, String> {
    let Some(scope) = dsh_git_runner_scope(workdir, facts, mode) else {
        return Ok(None);
    };
    // A layout the scoped runner will not serve refuses here, before the
    // seat spends an implementation, rather than at the seat's first
    // commit (decision 0054).
    if let Some(problem) = dsh_sandbox::scope_refusal(&scope) {
        return Err(format!("dsh driver: {problem}"));
    }
    let bwrap = dsh_bwrap()?;
    let program = dsh_runner_program();
    let staged = dsh_sandbox::stage_seat_store(&scope)?;
    let row = dsh_sandbox::sandbox_row(&program, &bwrap, &staged, &scope)?;
    Ok(Some((row, scope, staged)))
}

/// The bwrap binary the scoped runner needs, or the refusal that names
/// why the seat cannot start. A present-but-unusable bubblewrap is not
/// support: dsh would have fallen back to its Landlock rung, which
/// cannot express an extra writable root either. The path is absolute,
/// so the runner executes exactly this binary rather than searching
/// `PATH` again inside the seat.
#[cfg(target_os = "linux")]
fn dsh_bwrap_on(path: &std::ffi::OsStr) -> Result<PathBuf, String> {
    let bwrap = crate::hands::bwrap_on(path).map_err(|problem| {
        format!(
            "dsh driver: {problem}; a linked worktree's git metadata lies outside the seat's \
             writable workspace and this driver will not run the seat without a scoped runner"
        )
    })?;
    let bwrap = std::fs::canonicalize(&bwrap).unwrap_or(bwrap);
    dsh_sandbox::require_usable_bwrap(&bwrap)?;
    Ok(bwrap)
}

#[cfg(target_os = "linux")]
fn dsh_bwrap() -> Result<PathBuf, String> {
    dsh_bwrap_on(&std::env::var_os("PATH").unwrap_or_default())
}

#[cfg(not(target_os = "linux"))]
fn dsh_bwrap() -> Result<PathBuf, String> {
    Err(format!(
        "dsh driver: this host ({}) has no bubblewrap-compatible runner for a linked \
         worktree's git metadata; use a standalone checkout, or a realm boundary that can \
         write the git directory",
        std::env::consts::OS
    ))
}

/// The runner program the sandbox row names: the override a test or a
/// non-`PATH` installation sets, else this binary, else the name a
/// `PATH` lookup resolves. The `dsh-sandbox-runner` verb is appended by
/// the row, never stored here. The executable arrives as the host's own
/// answer rather than as a rendered error, so the one way it can fail
/// needs no translation step of its own.
fn dsh_runner_program_from(
    override_value: Option<String>,
    executable: std::io::Result<PathBuf>,
) -> String {
    if let Some(program) = override_value {
        return program;
    }
    match executable {
        Ok(path) => path.to_string_lossy().into_owned(),
        Err(_) => "brokkr".to_string(),
    }
}

fn dsh_runner_program() -> String {
    dsh_runner_program_from(
        crate::legacy::env("BROKKR_DSH_RUNNER", None),
        std::env::current_exe(),
    )
}

/// The seat overlay with the scoped sandbox row a linked-worktree seat
/// needs (decision 0054). It is one patch file because `--patch` is the
/// launcher's only override channel.
fn dsh_seat_overlay_with(
    model: Option<&str>,
    effort: Option<&str>,
    root: &std::path::Path,
    route: Option<&[u8]>,
    sandbox: Option<&str>,
) -> Result<DshSeatOverlay, String> {
    dsh_seat_overlay_in(model, effort, root, route, sandbox, || {
        tempfile::Builder::new()
            .prefix("brokkr-dsh-seat-")
            .suffix(".yml")
            .tempfile()
    })
}

// Test-only observation that `dsh_seat_overlay_in` was entered, so a
// refusing planner path can prove that staging never happened: an error
// or an absent retained directory does not establish that fact (task
// 8.10; answer U's R3). The planner is synchronous and the counter is
// thread-local, so parallel tests never observe one another's staging;
// a process-global counter would.
#[cfg(test)]
thread_local! {
    static DSH_STAGING_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn dsh_staging_calls() -> usize {
    DSH_STAGING_CALLS.with(std::cell::Cell::get)
}

#[cfg(test)]
fn reset_dsh_staging_calls() {
    DSH_STAGING_CALLS.with(|calls| calls.set(0));
}

/// The seat overlay over an injected file, so the ways staging can fail
/// are reachable from a test without a full disk. The injected creator
/// stages the patch; the settings document, when one is needed, is
/// staged beside it under its own prefix. Every row after the model's
/// is composed first and written once, so the one way the write can
/// fail is the one way a test can make it fail.
///
/// The validated route rows, when the seat carries one, are folded AHEAD
/// of the transcript, model and settings rows, so the launcher receives
/// exactly one `--patch` and Brokkr's own rows apply last (AS3; design D6
/// mechanism 1). The route bytes are already grammar-checked and their
/// provenance is the bound manifest digest the bundle digest covers.
fn dsh_seat_overlay_in(
    model: Option<&str>,
    effort: Option<&str>,
    root: &std::path::Path,
    route: Option<&[u8]>,
    sandbox: Option<&str>,
    create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
) -> Result<DshSeatOverlay, String> {
    #[cfg(test)]
    DSH_STAGING_CALLS.with(|calls| calls.set(calls.get() + 1));
    let mut rows = dsh_transcript_row(root)?;
    let settings = match (model, effort) {
        (Some(model), Some(effort)) => {
            let settings = dsh_effort_settings_in(model, effort, || {
                tempfile::Builder::new()
                    .prefix("brokkr-dsh-seat-settings-")
                    .suffix(".yaml")
                    .tempfile()
            })?;
            rows.push_str(&dsh_settings_row(settings.path())?);
            Some(settings)
        }
        (None, Some(_)) => {
            return Err(
                "dsh driver: `--effort` needs a `--model` beside it: the level rides the \
                 seat's default-model selection, which names its provider and model, and this \
                 driver does not read the profile's default back to restate it"
                    .to_string(),
            );
        }
        _ => None,
    };
    // The scoped sandbox row a linked-worktree seat needs (decision 0054)
    // rides with Brokkr's own rows, after the route's and before the
    // write: the route may not reach the provider the sandbox names.
    if let Some(sandbox) = sandbox {
        rows.push_str(sandbox);
    }
    let mut body = String::new();
    if let Some(route) = route {
        let text = std::str::from_utf8(route)
            .map_err(|_| "dsh driver: validated route overlay is not UTF-8".to_string())?;
        body.push_str(text);
        if !text.ends_with('\n') {
            body.push('\n');
        }
    }
    if let Some(model) = model {
        body.push_str(&dsh_model_row(model)?);
    }
    body.push_str(&rows);
    let mut patch = dsh_storage_context(create(), "could not stage the dsh seat overlay")?;
    // The write carries a path too: `tempfile` wraps a failed write on its
    // own handle with the file it holds, so this half is as path-bearing
    // as the allocation above.
    dsh_storage_context(
        patch.write_all(body.as_bytes()),
        "could not write the dsh seat overlay",
    )?;
    Ok(DshSeatOverlay { patch, settings })
}

/// This seat's settings document: the pinned model restated as a
/// complete default-model selection with the pinned level on it. dsh's
/// `agent-default-model` package keeps `reasoningEffort` out of its
/// composition config on purpose — a later selection that clears an
/// effort must stay cleared rather than re-inherit one — and reads it
/// from the settings section instead, where a complete selection wins
/// over the composition row (measured 2026-09-05 on 0.1.2-rc.1: the
/// same key written into the composition row was ignored without a
/// word; written here, the request header echoed it). A level is one
/// bounded word by `effort_token`'s clamp and is quoted anyway, because
/// the document is YAML and the discipline is the transcript root's.
fn dsh_effort_settings_in(
    model: &str,
    effort: &str,
    create: impl FnOnce() -> std::io::Result<tempfile::NamedTempFile>,
) -> Result<tempfile::NamedTempFile, String> {
    let DshModel { provider, model } = parse_dsh_model(model)?;
    let Some(effort) = effort_token(effort) else {
        return Err("dsh driver: the pinned effort is not one bounded word".to_string());
    };
    let mut file = dsh_storage_context(create(), "could not stage the dsh seat settings")?;
    let body = format!(
        "# Written by `brokkr driver dsh` for one seat: the effort pin, in the\n\
         # settings section dsh reads over its composition. The composition\n\
         # row that pins the model cannot carry a level by its own design, so\n\
         # the selection is restated here whole, with the level on it.\n\
         agent-default-model:\n\
         \x20 provider: {provider}\n\
         \x20 model: {model}\n\
         \x20 reasoningEffort: '{}'\n",
        effort.replace('\'', "''")
    );
    dsh_storage_context(
        file.write_all(body.as_bytes()),
        "could not write the dsh seat settings",
    )?;
    Ok(file)
}

/// The overlay row that points dsh's settings provider at this seat's
/// own document. The row's whole config is its path (measured on
/// 0.1.2-rc.1: `@deepseek-ai/dsh-settings-file` takes `path`, defaulting
/// to `<harness home>/settings.yaml`), so replacing it restates nothing
/// else. It does mean the seat reads no operator settings document,
/// and that is the point: a seat's level is the seat's, not whatever
/// the operator's own document last said.
fn dsh_settings_row(path: &std::path::Path) -> Result<String, String> {
    let path = path.to_string_lossy();
    if path.contains('\n') || path.contains('\r') {
        // The field, never the path — the transcript row's rule, for the
        // same reason: this document is staged beneath the operator's own
        // temporary root, so echoing its path hands the seat that root's
        // name in a `Result.error` it can read (AS3; tasks 8.8(d)/8.10).
        return Err(
            "dsh driver: the seat's settings path spans more than one line, so it cannot be \
             written as one overlay scalar"
                .to_string(),
        );
    }
    Ok(format!(
        "# Written by `brokkr driver dsh` for one seat: the settings document\n\
         # carrying this seat's effort, as the settings row's whole config.\n\
         - id: settings\n\
         \x20 config:\n\
         \x20   path: '{}'\n",
        path.replace('\'', "''")
    ))
}

/// Resolve one exec template part: `{workdir}`, `{prompt_file}`, and
/// each declared `{{secret:NAME}}` → the literal shell env reference
/// `$NAME` — never the value. `$NAME` expands only when the template
/// itself invokes a shell (`bash -c '…'`); env injection is the
/// mechanism that always works, and no `sh -c` wrapping is added (it
/// would change quoting semantics for every existing exec bundle).
fn resolve_exec_part(
    part: &str,
    workdir: &str,
    prompt_path: &str,
    bindings: &[secret::BoundSecret],
) -> String {
    let mut part = part
        .replace("{workdir}", workdir)
        .replace("{prompt_file}", prompt_path);
    for binding in bindings {
        part = part.replace(
            &format!("{{{{secret:{}}}}}", binding.name()),
            &format!("${}", binding.name()),
        );
    }
    part
}

fn stderr_tail_start(stderr: &str) -> usize {
    let mut start = stderr.len().saturating_sub(4000);
    while !stderr.is_char_boundary(start) {
        start -= 1;
    }
    start
}

/// Whether a driver checkpoint proves the harness began work (decision
/// 0053). Every row a driver writes before the session opens is one of
/// two pre-session shapes — the shared `transcript` locator, and the
/// `harness-started` launch row codex and dsh write — and the first row
/// that is neither is the one that flips the attempt across decision
/// 0016's boundary. Exec has no model turn: its `exec-started` row IS the
/// start.
fn begins_work(step: &str) -> bool {
    !matches!(step, "transcript" | "harness-started")
}

/// The refused attempt's pointer at its own prose: `<kind>/<locator>`
/// off the transcript row decision 0032 already shapes, or nothing when
/// the harness never announced a session. Neither half is read for
/// anything — it is an address a person types into a drilldown — but the
/// locator is a harness-supplied field like every other one this reason
/// quotes, and decision 0032's own clamp bounds its LENGTH only. It is
/// put through the same wire bound as the token here, so the composed
/// reason is one line whatever a harness announced its session as.
fn transcript_locator(session_meta: &Map<String, Value>) -> Option<String> {
    let transcript = session_meta.get("transcript")?;
    let locator = transcript.get("locator").and_then(Value::as_str)?;
    let locator = bounded_wire_line(locator, REFUSAL_TOKEN_LIMIT);
    if locator.is_empty() {
        return None;
    }
    let kind = bounded_wire_line(
        transcript
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        REFUSAL_TOKEN_LIMIT,
    );
    Some(format!("{kind}/{locator}"))
}

/// `session` is the prior session the engine handed back for this seat
/// (decision 0030), or `None` for the cold start every attempt was
/// before it. Only the codex arm knows what to do with one; the rest
/// ignore it exactly as they ignored the message that carried it.
fn run_seat(
    kind: AdapterKind,
    extra: &[String],
    start: &Value,
    session: Option<&str>,
    send: &mut impl FnMut(Body),
) {
    run_seat_with(kind, start, send, |prompt, input, bindings, mut emit| {
        invoke(kind, extra, prompt, input, session, bindings, &mut emit)
    });
}

/// `run_seat` over the ONE thing it delegates: the invocation itself.
/// Everything else a seat's terminal result turns on lives in this body —
/// the prompt and secret bindings, decision 0053's checkpoint buffer, the
/// delivered-file fact and the unsettled-launch guard — so a test that
/// drives a real synthetic child through here meets production's own
/// terminal rule, not a copy of it.
fn run_seat_with(
    kind: AdapterKind,
    start: &Value,
    send: &mut impl FnMut(Body),
    invoke: impl FnOnce(
        &str,
        &Value,
        &[secret::BoundSecret],
        &mut dyn FnMut(&Value),
    ) -> Result<Invocation, String>,
) {
    let input = start.get("input").cloned().unwrap_or(json!({}));
    let effect_id = start["effect_id"].as_str().unwrap_or("").to_string();
    let attempt_id = start["attempt_id"].as_str().unwrap_or("").to_string();
    // Decision 0053: `accepted` is withheld until a checkpoint proves a
    // turn began. A provider that refuses before its first turn sends
    // `result: failed` with NO `accepted` and NO checkpoint — exactly the
    // engine's structural fail-to-start predicate, so the chain advances
    // as it does for a driver the machine cannot reach at all. Every
    // other path sends `accepted` exactly as it did before, so nothing
    // widens but the refusal decision 0016 already calls a failure to
    // start; a refusal after the first turn is still a mid-session
    // failure under 0016.
    let mut accepted_sent = false;
    // A checkpoint is withheld until one proves work began. The rows a
    // driver writes before the session opens — the transcript locator and
    // the codex/dsh launch row — are buffered, flushed once the attempt
    // crosses the boundary, and dropped when it refused to start.
    let mut began_work = false;
    let mut buffered: Vec<Value> = Vec::new();
    let result_path = input
        .get("result_path")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    if let Some(parent) = std::path::Path::new(&result_path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    // Sealed secret bindings (decision 0012): every DECLARED name is
    // resolved before spawn — template references are only the optional
    // argv-side spelling. A missing name (or unreadable store) refuses
    // the attempt determinately, naming the secret and the store path,
    // never an empty-string injection.
    let declared: Vec<String> = input
        .get("secrets")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default();
    let bindings = if declared.is_empty() {
        Vec::new()
    } else {
        let store = input
            .get("secrets_file")
            .and_then(Value::as_str)
            .unwrap_or("");
        match secret::resolve_bindings(std::path::Path::new(store), &declared) {
            Ok(bindings) => bindings,
            Err(error) => {
                // A missing secret is a determinate configuration refusal,
                // not a provider's. It keeps the attempt's `accepted`
                // exactly as it always was, so the fail-to-start boundary
                // moves for a provider refusal and for nothing else. The
                // driver has not spawned, so no turn can have begun.
                send(Body::Accepted {
                    effect_id: effect_id.clone(),
                    attempt_id: attempt_id.clone(),
                    session_ref: None,
                });
                send(Body::Result {
                    effect_id,
                    attempt_id,
                    status: ResultStatus::Failed,
                    result: None,
                    error: Some(error),
                });
                return;
            }
        }
    };

    let prompt = render_prompt(&input, kind);
    // Streamed telemetry: each seat-turn the claude arm folds out of
    // stream-json becomes a live protocol checkpoint on this attempt.
    let invocation = match invoke(&prompt, &input, &bindings, &mut |data: &Value| {
        let mut data = data.clone();
        // Every fold emits a JSON object and the seat record (0034)
        // is defined on objects, so there is no non-object
        // checkpoint to branch on.
        let checkpoint = data
            .as_object_mut()
            .expect("driver checkpoints are JSON objects");
        checkpoint.entry("model").or_insert_with(|| {
            Value::String(
                if kind == AdapterKind::Exec {
                    MODEL_NOT_APPLICABLE
                } else {
                    MODEL_NOT_REPORTED
                }
                .to_string(),
            )
        });
        // The same default, one field over (decision 0035 ruling 3).
        // A fold that read its harness's echo has already written
        // the level; this is what every other row says — `not
        // applicable` for exec, which has no model turn, and `not
        // reported` for a row its harness echoed no level for: the
        // rows before the first request goes out, and every row of
        // a lane that echoes nothing at all.
        checkpoint.entry("effort").or_insert_with(|| {
            Value::String(
                if kind == AdapterKind::Exec {
                    EFFORT_NOT_APPLICABLE
                } else {
                    EFFORT_NOT_REPORTED
                }
                .to_string(),
            )
        });
        let starts = data
            .get("step")
            .and_then(Value::as_str)
            .is_some_and(begins_work);
        if began_work || starts {
            began_work = true;
            if !accepted_sent {
                accepted_sent = true;
                send(Body::Accepted {
                    effect_id: effect_id.clone(),
                    attempt_id: attempt_id.clone(),
                    session_ref: None,
                });
            }
            for row in buffered.drain(..) {
                send(Body::Checkpoint {
                    effect_id: effect_id.clone(),
                    attempt_id: attempt_id.clone(),
                    data: row,
                });
            }
            send(Body::Checkpoint {
                effect_id: effect_id.clone(),
                attempt_id: attempt_id.clone(),
                data,
            });
        } else {
            buffered.push(data);
        }
    }) {
        Ok(invocation) => invocation,
        Err(error) => {
            // A driver that could not be invoked at all never opened a
            // turn, but it is not a CLASSIFIED provider refusal: keep
            // `accepted` exactly as it always was, so only the refusal
            // moves the fail-to-start boundary.
            if !accepted_sent {
                send(Body::Accepted {
                    effect_id: effect_id.clone(),
                    attempt_id: attempt_id.clone(),
                    session_ref: None,
                });
                for row in buffered.drain(..) {
                    send(Body::Checkpoint {
                        effect_id: effect_id.clone(),
                        attempt_id: attempt_id.clone(),
                        data: row,
                    });
                }
            }
            send(Body::Result {
                effect_id,
                attempt_id,
                status: ResultStatus::Failed,
                result: None,
                error: Some(error),
            });
            return;
        }
    };
    let Invocation {
        exit_code,
        session_meta,
        stdout,
        stderr,
        state,
        refusal,
        launch,
    } = invocation;
    // A provider refusal before the first turn is a determinate failure
    // to start (decision 0053): `result: failed`, no `accepted`, no
    // checkpoint, and the reason is the result's error so the journal
    // keeps the refused attempt and why the chain descends. The buffered
    // pre-session rows are dropped with it — a checkpoint would put the
    // attempt on 0016's other side — so the reason carries the transcript
    // locator instead. That pointer is the evidence #219 was diagnosed
    // from, and a refused attempt that named nowhere to look would make
    // the next such diagnosis start from scratch.
    //
    // Two facts have to agree before an attempt is thrown away as a
    // refusal to start: no work checkpoint, and no delivery. A classifier
    // reads its harness's machine fields, not the seat's own contract, so
    // if a record it read as a refusal is overtaken by a clean exit with
    // the result file written, the delivered work is the fact and the
    // refusal was a notice. No measured CLI both refuses and delivers —
    // `began_work` catches every shape #219 saw — so this is the second
    // wall, and it is the one that keeps the single asserted shape
    // (`rate_limit_event`, which a newer CLI may yet emit as an advisory
    // rather than a rejection) from ever discarding a seat's work.
    let delivered = exit_code == 0 && std::fs::metadata(&result_path).is_ok();
    if let Some(reason) = refusal {
        if !began_work && !delivered {
            let stderr_tail_start = stderr_tail_start(&stderr);
            eprint!("{}", &stderr[stderr_tail_start..]);
            send(Body::Result {
                effect_id,
                attempt_id,
                status: ResultStatus::Failed,
                result: None,
                error: Some(match transcript_locator(&session_meta) {
                    Some(locator) => format!("{reason} [transcript {locator}]"),
                    None => reason,
                }),
            });
            return;
        }
    }
    // Every non-refusal path reports `accepted` exactly as it always
    // did, flushing the pre-session rows buffered while the session was
    // opening.
    if !accepted_sent {
        send(Body::Accepted {
            effect_id: effect_id.clone(),
            attempt_id: attempt_id.clone(),
            session_ref: None,
        });
        for row in buffered.drain(..) {
            send(Body::Checkpoint {
                effect_id: effect_id.clone(),
                attempt_id: attempt_id.clone(),
                data: row,
            });
        }
    }
    // A rejoin that never confirmed the exact offered root — or that
    // named a DIFFERENT one — has an unknown session, whatever the
    // process exited with and whatever it wrote. Design D7: a missing
    // required exact-root confirmation makes the attempt failed, never a
    // successful guessed rejoin, and the adapter spends no cold
    // replacement on it. The delivered file, if any, is left on disk for
    // diagnosis; it is not this attempt's accepted work.
    if launch.is_unsettled() {
        let reason = match launch {
            LaunchTerminal::Mismatch => {
                "provider named a different session than the offered root; \
                 refusing to accept the invocation"
            }
            _ => {
                "provider never confirmed the offered session; \
                 refusing to accept the invocation"
            }
        };
        send(Body::Result {
            effect_id,
            attempt_id,
            status: ResultStatus::Failed,
            result: None,
            error: Some(reason.to_string()),
        });
        return;
    }
    let served_model = if kind == AdapterKind::Exec {
        MODEL_NOT_APPLICABLE.to_string()
    } else {
        session_meta
            .get("model")
            .and_then(Value::as_str)
            .and_then(model_token)
            .unwrap_or_else(|| MODEL_NOT_REPORTED.to_string())
    };
    // The effort the harness echoed, on the same terms and with the same
    // sentinels — the last level a fold read for this seat, or the
    // absence spelled out. Never the pin: a bundle's `--effort` says what
    // was ASKED for, and this field says what a layer of profiles,
    // plugins and provider routes actually applied (decision 0035
    // rulings 3 and 6, which keep the two apart deliberately).
    let applied_effort = if kind == AdapterKind::Exec {
        EFFORT_NOT_APPLICABLE.to_string()
    } else {
        applied_harness_effort(&session_meta)
    };
    let stderr_tail_start = stderr_tail_start(&stderr);
    eprint!("{}", &stderr[stderr_tail_start..]);
    let mut checkpoint = Map::new();
    checkpoint.insert(
        "step".into(),
        Value::String(format!("{}-session-finished", kind.driver_name())),
    );
    checkpoint.insert("exit_code".into(), Value::from(exit_code));
    let result_record = session_meta.clone();
    checkpoint.extend(session_meta);
    // Inserted after driver metadata: this field is always the normalized
    // provider report, never a configured default supplied by the seat.
    checkpoint.insert("model".into(), Value::String(served_model.clone()));
    checkpoint.insert("effort".into(), Value::String(applied_effort.clone()));
    // Ledger-capture marker: a source-literal CONSTANT, never
    // data-derived, inserted AFTER the session_meta extend so
    // last-write-wins guarantees no stream-derived key can ever shadow
    // it. "Priceable in the LaneTally ledger", not "priced" —
    // total_cost_usd above stays the harness-reported list price.
    if kind == AdapterKind::Lanetally {
        checkpoint.insert("capture".into(), Value::String("lanetally".into()));
    }
    send(Body::Checkpoint {
        effect_id: effect_id.clone(),
        attempt_id: attempt_id.clone(),
        data: Value::Object(checkpoint),
    });

    if let Some(dialect) = input.get("dialect_exec") {
        let success = dialect
            .get("success_result")
            .and_then(Value::as_str)
            .unwrap_or("drafted");
        let failure = dialect
            .get("failure_result")
            .and_then(Value::as_str)
            .unwrap_or("fail");
        let notes = [stdout.trim(), stderr.trim()]
            .into_iter()
            .filter(|part| !part.is_empty())
            .collect::<Vec<_>>()
            .join("\n");
        let mut result = json!({
            "result": if exit_code == 0 { success } else { failure },
            "inputs": {"change": dialect.get("change").cloned().unwrap_or(Value::Null)},
            "notes": notes,
            "model": served_model,
            "effort": applied_effort,
        });
        if let Some(state) = state {
            result["state"] = Value::String(state);
        }
        send(Body::Result {
            effect_id,
            attempt_id,
            status: ResultStatus::Succeeded,
            result: Some(result),
            error: None,
        });
        return;
    }

    if exit_code != 0 {
        send(Body::Result {
            effect_id,
            attempt_id,
            status: ResultStatus::Failed,
            result: None,
            error: Some(format!("agent CLI exited {exit_code}")),
        });
        return;
    }
    let Ok(raw) = std::fs::read(&result_path) else {
        send(Body::Result {
            effect_id,
            attempt_id,
            status: ResultStatus::Failed,
            result: None,
            error: Some("seat wrote no result file (the result contract was not met)".into()),
        });
        return;
    };
    // Masking choke point, third surface (decision 0012, layer 5): the
    // child-written result payload rides Body::Result into the
    // append-only journal via EffectSucceeded — a child that echoes
    // $TOKEN into its notes must not put plaintext there. Raw bytes
    // first, string conversion second.
    let raw = secret::mask_bytes(&raw, &bindings);
    let raw = String::from_utf8_lossy(&raw);
    // Typed-invalid on purpose when unparseable: the engine parks with
    // raw evidence (decision 0001); adapters repair nothing.
    let mut seat_result = match serde_json::from_str::<Value>(&raw) {
        Ok(result) => result,
        Err(error) => json!({"__unparseable_result_file__": error.to_string()}),
    };
    // Result contracts are objects. Enrich the driver's copy with the
    // provider report after masking/parsing so seat-authored content can
    // neither forge nor suppress the model in the final result.
    if let Some(result) = seat_result.as_object_mut() {
        result.insert("model".into(), Value::String(served_model));
        result.insert("effort".into(), Value::String(applied_effort));
        for key in RESULT_RECORD_FIELDS {
            if let Some(value) = result_record.get(key) {
                result.insert(key.into(), value.clone());
            }
        }
    }
    send(Body::Result {
        effect_id,
        attempt_id,
        status: ResultStatus::Succeeded,
        result: Some(seat_result),
        error: None,
    });
}

/// The longest session handle this adapter will hold. Every provider's
/// own grammar is far narrower — a UUID, a `session-`-prefixed UUID, a
/// codex thread id — and the seat record's own field stops at 80. The
/// bound is here so a malformed offer cannot make the adapter carry an
/// unbounded string before anything has validated it.
const OFFER_LIMIT: usize = 80;

/// One offer, with the attempt it belongs to (proposed decision 0056
/// ruling 4). The correlation is not decoration: `Body::Resume` already
/// carries both ids, and without comparing them a second offer silently
/// overwrites the first, and an offer for one attempt reaches another.
struct PendingOffer {
    effect_id: String,
    attempt_id: String,
    handle: String,
}

/// What the exchange knows about an offer at any moment. `Poisoned` is
/// the arm that matters: an exchange that saw a malformed, duplicated or
/// mismatched offer never launches a provider and is NEVER repaired into
/// a cold execution — a cold run here would be work the engine did not
/// ask for, charged to an attempt whose instructions were already
/// confused.
enum OfferState {
    None,
    Pending(PendingOffer),
    Poisoned(&'static str),
}

/// The adapter main loop over stdio: hello/capabilities, an optional
/// correlated resume offer, start→seat, cancel/shutdown. `extra` are the
/// args after `--` in the bundle's driver command.
///
/// `resume` arrives BEFORE the `start` it belongs to and carries no seat
/// and no input — it is the session handle for the attempt the next
/// `start` describes, and nothing else. It is consumed by that one
/// attempt, ONCE, and only when its effect and attempt match: a second
/// start with no resume in front of it is a cold start, and an offer
/// whose correlation does not match the next start reaches no provider
/// and survives to no later start.
fn serve_io(
    kind: AdapterKind,
    extra: &[String],
    input: impl BufRead,
    mut output: impl Write,
) -> std::io::Result<()> {
    let mut send = |body: Body| {
        let line = serde_json::to_string(&Message::new(body))
            .expect("the closed protocol message vocabulary serializes");
        let _ = output.write_all(line.as_bytes());
        let _ = output.write_all(b"\n");
        let _ = output.flush();
    };
    let mut negotiated = false;
    let mut offer = OfferState::None;
    for line in input.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(message) = serde_json::from_str::<Value>(&line) else {
            continue; // the engine speaks the protocol; ignore noise
        };
        let field = |key: &str| {
            message
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string()
        };
        match message.get("type").and_then(Value::as_str) {
            Some("hello") => {
                negotiated = true;
                send(Body::Capabilities {
                    driver: kind.driver_name(),
                    version: ADAPTER_VERSION.to_string(),
                    supports: kind.supports(),
                })
            }
            Some("resume") => {
                offer = match &offer {
                    // A second offer before one start is ambiguous: two
                    // handles claim the same attempt and nothing on the
                    // wire says which the engine meant. Neither is used.
                    OfferState::Pending(_) => OfferState::Poisoned("duplicate resume offer"),
                    OfferState::Poisoned(reason) => OfferState::Poisoned(reason),
                    OfferState::None if !negotiated => {
                        OfferState::Poisoned("resume before capabilities were negotiated")
                    }
                    OfferState::None => {
                        let (effect_id, attempt_id) = (field("effect_id"), field("attempt_id"));
                        let handle = message.get("session_ref").and_then(Value::as_str);
                        match handle {
                            Some(handle)
                                if !effect_id.is_empty()
                                    && !attempt_id.is_empty()
                                    && !handle.is_empty()
                                    && handle.len() <= OFFER_LIMIT =>
                            {
                                OfferState::Pending(PendingOffer {
                                    effect_id,
                                    attempt_id,
                                    handle: handle.to_string(),
                                })
                            }
                            _ => OfferState::Poisoned("malformed resume envelope"),
                        }
                    }
                };
            }
            Some("start") => {
                let (effect_id, attempt_id) = (field("effect_id"), field("attempt_id"));
                // The offer is consumed here whatever happens to it, so
                // a handle can never reach a later start.
                let session = match std::mem::replace(&mut offer, OfferState::None) {
                    OfferState::None => Ok(None),
                    OfferState::Poisoned(reason) => Err(reason),
                    OfferState::Pending(pending) => {
                        if pending.effect_id == effect_id && pending.attempt_id == attempt_id {
                            Ok(Some(pending.handle))
                        } else {
                            Err("resume offer named a different effect or attempt")
                        }
                    }
                };
                match session {
                    Ok(session) => run_seat(kind, extra, &message, session.as_deref(), &mut send),
                    // A poisoned exchange launches no provider. The
                    // failure is bounded and structural — no provider
                    // prose, no invented refusal token — and it is sent
                    // only because the START named the correlation, so
                    // the engine can attribute it. A driver defect is a
                    // determinate failure under decision 0006.
                    Err(reason) => send(Body::Result {
                        effect_id,
                        attempt_id,
                        status: ResultStatus::Failed,
                        result: None,
                        error: Some(format!("resume exchange refused: {reason}")),
                    }),
                }
            }
            Some("cancel") => {
                send(Body::Cancelled {
                    effect_id: field("effect_id"),
                });
                return Ok(());
            }
            Some("shutdown") => return Ok(()),
            _ => {}
        }
    }
    Ok(())
}

pub fn serve(kind: AdapterKind, extra: Vec<String>) -> std::io::Result<()> {
    serve_io(kind, &extra, std::io::stdin().lock(), std::io::stdout())
}

#[cfg(test)]
mod tests;
