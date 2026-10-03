//! The launches the probe tries: the harness's headless launch grammar,
//! the adapter's own passthrough, and one deliberate mistake per refusal
//! class. Paths and the prompt stay as `{cli}`, `{workdir}` and
//! `{prompt}` here, so the argv the report shows is the same on every
//! host; `observe` fills them in at spawn.

use super::{Declared, Native, OffControl, ProbeError};
use crate::adapters::{codex_effort_config, AdapterKind};

/// The reply the one turn asks for, spelled once for [`REPLY`] and
/// [`PROMPT`].
macro_rules! reply {
    () => {
        "PROBE-OK"
    };
}

/// The reply the one turn asks for, which a stream may carry back.
pub(crate) const REPLY: &str = reply!();

/// The one turn the probe asks for. It asks for no work, so a turn costs
/// as little as the CLI allows.
pub(crate) const PROMPT: &str = concat!(
    "Reply with exactly ",
    reply!(),
    " and nothing else. Use no tool."
);

/// A model id no provider serves: the configuration refusal's trigger.
pub(crate) const NO_SUCH_MODEL: &str = "brokkr-probe-no-such-model";

/// An effort no CLI accepts: its refusal lists the ones it does.
pub(crate) const NO_SUCH_EFFORT: &str = "brokkr-probe-no-such-effort";

/// The MCP server planted in the scratch HOME's user-scope configuration.
/// A turn whose MCP servers name it has read the operator's own settings.
pub(crate) const USER_SCOPE_SERVER: &str = "brokkr-probe-user-scope";

/// One launch the probe tries, or why it cannot try it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Step {
    Launch(Vec<String>),
    Untried(String),
}

/// A user-scope configuration file, relative to HOME, that names
/// [`USER_SCOPE_SERVER`].
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UserConfig {
    Planted {
        path: &'static str,
        contents: &'static str,
    },
    Unknown(&'static str),
}

/// Every launch of one probe run.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Plan {
    pub(crate) version: Vec<String>,
    pub(crate) turn: Vec<String>,
    pub(crate) bad_model: Step,
    pub(crate) bad_effort: Step,
    pub(crate) boxed: Step,
    /// The adapter's hands argv, placeholders intact, empty when it
    /// declares none: what a refusal of the boxed turn must name.
    pub(crate) hands: Vec<String>,
    /// The plain turn under the argv that switches every declared native
    /// power off (decision 0065 ruling 4).
    pub(crate) native_off: Step,
    /// That argv, placeholders intact: what a power the turn under it no
    /// longer lists was measured switched off by, and what a refusal of
    /// that turn must name.
    pub(crate) off: Vec<String>,
    pub(crate) native: Native,
    pub(crate) user_config: UserConfig,
    /// How `turn` departs from the launch the adapter's driver composes.
    pub(crate) unlike_driver: &'static str,
}

/// Whether a harness takes a model on its own command line, by the
/// adapter's declared `model_flag`.
enum Model {
    Flag,
    Unreachable(&'static str),
}

/// How a harness is told an effort on its own command line.
enum Effort {
    /// The adapter's declared `effort_flag`, followed by the level.
    Flag,
    /// Codex's config override, as its driver writes it.
    CodexConfig,
    Unreachable(&'static str),
}

/// A harness's headless launch: what follows the binary before the
/// adapter's passthrough, how it takes an effort, and where its
/// user-scope MCP servers live.
struct Grammar {
    head: &'static [&'static str],
    model: Model,
    effort: Effort,
    user_config: UserConfig,
    unlike_driver: &'static str,
}

/// How the probe's turn departs from the claude and codex drivers.
const PROMPT_AS_ARGUMENT: &str = "the prompt is the last argument and stdin is closed, where \
                                  the adapter's driver writes the prompt to stdin";

const CLAUDE_USER_CONFIG: &str =
    r#"{"mcpServers":{"brokkr-probe-user-scope":{"type":"stdio","command":"true","args":[]}}}"#;

const CODEX_USER_CONFIG: &str = "[mcp_servers.brokkr-probe-user-scope]\ncommand = \"true\"\n";

const DSH_PATCH_ONLY: &str = "dsh takes a model and an effort only through the profile patch \
                              its driver composes, which the probe does not compose";

/// The launch grammar of each harness the probe knows: the head of the
/// argv each built-in driver launches (`claude_cold`, `codex_cold` and
/// `dsh_launch` in `adapters`), less the seat's composition, and where
/// the probe's turn departs from it, which the report names beside the
/// argv. Decision 0075 ruling 1 moves the grammar into the adapter's spec.
fn grammar(kind: AdapterKind, adapter: &str) -> Result<Grammar, ProbeError> {
    match kind {
        AdapterKind::Claude => Ok(Grammar {
            head: &["-p", "--output-format", "stream-json", "--verbose"],
            model: Model::Flag,
            effort: Effort::Flag,
            user_config: UserConfig::Planted {
                path: ".claude.json",
                contents: CLAUDE_USER_CONFIG,
            },
            unlike_driver: PROMPT_AS_ARGUMENT,
        }),
        AdapterKind::Codex => Ok(Grammar {
            head: &["exec", "--json", "-C", "{workdir}"],
            model: Model::Flag,
            effort: Effort::CodexConfig,
            user_config: UserConfig::Planted {
                path: ".codex/config.toml",
                contents: CODEX_USER_CONFIG,
            },
            unlike_driver: PROMPT_AS_ARGUMENT,
        }),
        AdapterKind::Dsh => Ok(Grammar {
            head: &["--profile", "headless"],
            model: Model::Unreachable(DSH_PATCH_ONLY),
            effort: Effort::Unreachable(DSH_PATCH_ONLY),
            user_config: UserConfig::Unknown(
                "the probe knows no user-scope MCP configuration file for dsh",
            ),
            unlike_driver: "the prompt is the last argument, stdin is closed and no --patch \
                            overlay is given, where the adapter's driver writes the prompt to \
                            stdin and always composes a --patch profile overlay",
        }),
        AdapterKind::Lanetally | AdapterKind::Exec => Err(ProbeError::NotAHarness {
            adapter: adapter.to_string(),
        }),
    }
}

/// Every launch the probe tries against `declared`'s harness.
pub(crate) fn plan(kind: AdapterKind, declared: &Declared) -> Result<Plan, ProbeError> {
    let grammar = grammar(kind, &declared.adapter)?;
    let turn_with = |extra: &[String]| -> Vec<String> {
        let mut argv = vec!["{cli}".to_string()];
        argv.extend(grammar.head.iter().map(|part| part.to_string()));
        argv.extend(declared.passthrough.iter().cloned());
        argv.extend(extra.iter().cloned());
        argv.push("{prompt}".to_string());
        argv
    };
    let bad_model = match (&grammar.model, &declared.model_flag) {
        (Model::Flag, Some(flag)) => {
            Step::Launch(turn_with(&[flag.clone(), NO_SUCH_MODEL.to_string()]))
        }
        (Model::Flag, None) => Step::Untried("the adapter declares no model flag".to_string()),
        (Model::Unreachable(why), _) => Step::Untried(why.to_string()),
    };
    let bad_effort = match (&grammar.effort, &declared.effort_flag) {
        (Effort::Flag, Some(flag)) => {
            Step::Launch(turn_with(&[flag.clone(), NO_SUCH_EFFORT.to_string()]))
        }
        (Effort::Flag, None) => Step::Untried("the adapter declares no effort flag".to_string()),
        (Effort::CodexConfig, _) => Step::Launch(turn_with(&[
            "-c".to_string(),
            codex_effort_config(NO_SUCH_EFFORT),
        ])),
        (Effort::Unreachable(why), _) => Step::Untried(why.to_string()),
    };
    let boxed = match &declared.hands {
        Some(fragment) => Step::Launch(turn_with(fragment)),
        None => Step::Untried(format!(
            "the adapter declares no hands argv that switches the CLI's own tools off ({})",
            declared
                .hands_gap
                .as_deref()
                .unwrap_or("no reason recorded")
        )),
    };
    let (native_off, off) = match &declared.native {
        Native::Known {
            off: OffControl::Argv(argv),
            ..
        } => (Step::Launch(turn_with(argv)), argv.clone()),
        Native::Known {
            off: OffControl::Refused(why),
            ..
        } => (
            Step::Untried(format!(
                "the engine composes no OFF control for a seat granted nothing: {why}"
            )),
            Vec::new(),
        ),
        Native::Unmeasured(why) => (
            Step::Untried(format!(
                "the adapter declares its native capabilities unmeasured: {why}"
            )),
            Vec::new(),
        ),
    };
    Ok(Plan {
        version: vec!["{cli}".to_string(), "--version".to_string()],
        turn: turn_with(&[]),
        bad_model,
        bad_effort,
        boxed,
        hands: declared.hands.clone().unwrap_or_default(),
        native_off,
        off,
        native: declared.native.clone(),
        user_config: grammar.user_config,
        unlike_driver: grammar.unlike_driver,
    })
}
