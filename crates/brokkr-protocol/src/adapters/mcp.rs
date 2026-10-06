//! The private serving edge and the isolated MCP configuration every model
//! launch is served with (decision 0065 slice two, U1c; requirements SI2 and
//! MB1). The typed inputs the engine sealed beside a launch are read here,
//! once, into the final check's [`Serving`], and the box's hands transport
//! is bound here; beside them, the engine's typed isolation intent names the
//! launch's server set — empty or hands-only, the only sets this
//! no-broker plan has — and each invocation shape's measured assessment.
//!
//! Only the U0-qualified mechanism is built: Claude's grammar, which
//! LaneTally's wrapper forwards, excludes ambient MCP under
//! `--strict-mcp-config` with an engine-written `--mcp-config` (U0 cells C03
//! to C06 and LT03 to LT10). Codex has no passing candidate and dsh's
//! engine-only home is not built here (U1c2), so an intent for either
//! refuses rather than borrowing Claude's result or falling back to ambient
//! configuration. Exec has no model MCP surface (SI2), so its launch reads
//! no intent. A launch whose input
//! carries no intent is served as before: mandatory strict admission
//! activates only at U1g.
//!
//! [`Serving`]: crate::native_controls::Serving

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

use super::{claude_restriction_control, last_message_door, placed, ServingShape};
use crate::native_controls::{SealedServing, Transport, SERVING_INPUTS};

/// The driver-input key the engine's isolation intent rides under.
const MCP_ISOLATION: &str = "mcp_isolation";

/// The MCP document of the empty server set, exactly as U0 measured it
/// (cell C03).
const EMPTY_DOCUMENT: &str = r#"{"mcpServers":{}}"#;

/// The longest measured reason an intent may carry, in scalar values: the
/// bound the adapter loader holds the evidence it comes from to.
const REASON_LIMIT: usize = 400;

/// The whole refusal of a launch that carries sealed inputs without the
/// ones they are sealed beside (rebuild unit 14).
pub(super) const UNPAIRED: &str =
    "refusing to invoke the agent CLI: the input carries a sealed launch record or sealed serving \
     inputs without the capability plan and the record they are sealed beside, so its final \
     command cannot be checked; a sealed launch is never served unchecked (rebuild unit 14; \
     design D6)";

/// The whole refusal of a launch that carries the engine's plan with neither
/// sealed input (rebuild unit 15-fix-b; SC15-R2-1): dispatch seals both
/// wherever it writes a plan, so a plan alone is a governed launch stripped
/// of what it is checked by.
const UNSEALED: &str =
    "refusing to invoke the agent CLI: the input carries the engine's capability plan without the \
     sealed launch record and sealed serving inputs it is served beside, so its final command \
     cannot be checked; a launch the engine governs is never served as an unsealed one (rebuild \
     unit 15; design D6)";

/// One launch's driver input as the private serving edge reads it: the
/// typed serving inputs sealed beside it are decoded at most once, on first
/// use, and that one value is shared by the isolation intent's comparison
/// ([`isolated`]) and the final check ([`served`]).
pub(super) struct Edge<'a> {
    input: &'a Value,
    sealed: OnceCell<Result<SealedServing, String>>,
}

impl<'a> Edge<'a> {
    pub(super) fn new(input: &'a Value) -> Self {
        Edge {
            input,
            sealed: OnceCell::new(),
        }
    }

    /// The sealed serving inputs, decoded, `None` where the input carries
    /// none.
    fn sealed(&self) -> Option<&Result<SealedServing, String>> {
        let inputs = self.input.get(SERVING_INPUTS)?;
        Some(
            self.sealed
                .get_or_init(|| SealedServing::decode(Some(inputs))),
        )
    }
}

/// The engine's own executable, which a harness spawns as `brokkr hands
/// serve`: the one place a launch reads it.
fn engine_executable() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

/// The box's hands bound to `brokkr` and `workdir`, where the sealed inputs
/// declare them; `None` where the site has none.
fn hands_transport<'a>(
    sealed: &'a SealedServing,
    brokkr: &'a Path,
    workdir: &'a str,
) -> Option<Transport<'a>> {
    sealed.spec.as_ref().map(|spec| Transport {
        brokkr,
        workdir: Path::new(workdir),
        spec,
    })
}

/// Rebuild units 14 and 15 (operator ruling 2 of 2026-09-23; design D6):
/// the command a launch spawns, cold, rejoining or a rejected rejoin's cold
/// replacement, once [`check_final`] has proved that it expresses exactly
/// its sealed plan. Where the engine sealed the launch, the record and the
/// typed serving inputs sealed beside it (rebuild unit 14a2) are decoded,
/// the record's whole ordered segments must reassemble `handed`, the
/// arguments the driver was handed, and the check is handed them with the
/// engine's serving choices:
/// `chosen`'s executable, workdir, the session it rejoins (`None` cold)
/// and, for DSH, overlay, stream reading and prompt; the recipe's words by
/// their recorded origin; the result path where the door is the capture;
/// and the box's hands bound to this executable and workdir. Nothing is
/// read back from the argv. The spawn is handed [`Checked::into_argv`] and
/// nothing else.
///
/// A launch with no plan, no record and no serving inputs was not sealed: a
/// driver run by hand, which this check gives no guarantee, is served as
/// composed, as the inline judgment serves it (rebuild unit 5d-fix-c2). A
/// plan with neither sealed input refuses (rebuild unit 15-fix-b), and so
/// does one half of the sealed pair without the other, or without its plan.
///
/// [`check_final`]: crate::native_controls::check_final
/// [`Checked::into_argv`]: crate::native_controls::Checked::into_argv
pub(super) fn served(
    harness: &str,
    command: Vec<String>,
    handed: &[String],
    edge: &Edge<'_>,
    chosen: crate::native_controls::Serving<'_>,
) -> Result<Vec<String>, String> {
    use crate::native_controls::{check_final, managed, reassemble, Dialect, LaunchRecord, Origin};
    let input = edge.input;
    let (record, sealed) = match (input.get("launch_record"), edge.sealed()) {
        (None, None) if input.get("native_controls").is_none() => return Ok(command),
        (None, None) => return Err(UNSEALED.to_string()),
        (Some(record), Some(sealed)) => (record, sealed),
        _ => return Err(UNPAIRED.to_string()),
    };
    let Some(controls) = managed(input)? else {
        return Err(UNPAIRED.to_string());
    };
    let record = LaunchRecord::decode(Some(record))?;
    let sealed = sealed.as_ref().map_err(Clone::clone)?;
    // The whole ordered record, not only its authored segments, must
    // reassemble the arguments this driver was handed: a record emptied,
    // reordered or grown around them has no origin to read the recipe's
    // words by, and is refused (NCC; tasks 15.1 and 15.2).
    reassemble(&record.segments, handed)?;
    let authored: Vec<String> = record
        .segments
        .iter()
        .filter(|segment| segment.origin == Origin::Authored)
        .flat_map(|segment| segment.argv.iter().cloned())
        .collect();
    let brokkr = engine_executable();
    let dialect = &sealed.dialect;
    check_final(
        harness,
        command,
        &controls,
        &record.expected,
        Dialect {
            permissions: dialect.permissions.as_ref(),
            sandbox: &dialect.sandbox,
            hands: &dialect.hands,
            boundary: &dialect.boundary,
            stands: dialect.stands,
        },
        crate::native_controls::Serving {
            authored: &authored,
            pins: &sealed.pins,
            output: last_message_door(input)
                .then(|| input["result_path"].as_str().unwrap_or_default()),
            hands: hands_transport(sealed, &brokkr, chosen.workdir),
            ..chosen
        },
    )
    .map(crate::native_controls::Checked::into_argv)
    .map_err(|refusal| refusal.at_launch(input))
}

/// The server set the engine composed for one launch: the current no-broker
/// plan has no other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "kebab-case")]
enum ServerSet {
    Empty,
    Hands,
}

/// One invocation shape's isolation, as the engine read it from validated
/// adapter evidence: measured, measured unsupported with its reason, or
/// never measured.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
enum Assessment {
    Measured,
    Unsupported(String),
    Unmeasured,
}

/// The engine's isolation intent for one launch: its server set and the
/// assessment of each shape it can be served as. The cold replacement of a
/// declined or rejected rejoin is measured on its own (U1b's
/// `McpInvocation::Replacement`) and judged by `replacement`, never by
/// `cold` or `resume`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Isolation {
    servers: ServerSet,
    cold: Assessment,
    replacement: Assessment,
    resume: Assessment,
}

/// The shape a launch is about to be served as: cold, the cold replacement
/// of an offered rejoin, or a rejoin of the named resume shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Invocation<'a> {
    Cold,
    Replacement,
    Resume(&'a str),
}

/// Why no isolated configuration is served (requirement SI2). Each renders
/// as the cause after the driver's prefix ([`McpRefusal::at_launch`]).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(super) enum McpRefusal {
    #[error(
        "the engine's MCP isolation intent cannot be read, so no strict MCP configuration is built"
    )]
    Unreadable,
    #[error("the engine's MCP isolation intent carries a reason that is not one bounded line")]
    UnboundedReason,
    #[error("provider '{provider}' cannot exclude ambient MCP configuration ({reason})")]
    Unsupported {
        provider: &'static str,
        reason: String,
    },
    #[error("provider '{provider}' has no measured strict MCP configuration for '{shape}'")]
    Unmeasured {
        provider: &'static str,
        shape: String,
    },
    #[error("provider '{provider}' final MCP configuration is not the engine's sealed server set")]
    NotSealed { provider: &'static str },
    #[error(
        "provider '{provider}' arguments do not place the engine's MCP configuration as options \
         of their own: a terminator, a dangling value or an argument the grammar does not model \
         stands before it"
    )]
    Unplaced { provider: &'static str },
}

impl McpRefusal {
    /// The refusal as a driver states it, before any provider work.
    pub(super) fn at_launch(&self) -> String {
        format!("refusing to invoke the agent CLI: {self}")
    }
}

/// A launch's composed arguments with its isolated configuration in them,
/// and the intent they were built from, `None` where the input carried none.
#[derive(Debug)]
pub(super) struct Isolated {
    pub(super) argv: Vec<String>,
    isolation: Option<Isolation>,
}

impl Isolated {
    /// Whether a rejoin of `shape` keeps the isolation the launch was built
    /// with. A launch with no intent rejoins as before; one whose resume
    /// shape is not measured for `provider` declines the offer and is
    /// served as the cold replacement [`isolated`] already admitted.
    pub(super) fn resumes(&self, provider: &'static str, shape: &str) -> bool {
        self.isolation
            .as_ref()
            .is_none_or(|isolation| isolation.admit(provider, Invocation::Resume(shape)).is_ok())
    }
}

impl Isolation {
    /// The engine's intent in `input`, `None` where it carries none, read
    /// closed: an unknown key or word, an absent member and a reason that is
    /// not one bounded line each refuse.
    fn read(input: &Value) -> Result<Option<Isolation>, McpRefusal> {
        let Some(value) = input.get(MCP_ISOLATION) else {
            return Ok(None);
        };
        let isolation = Isolation::deserialize(value).map_err(|_| McpRefusal::Unreadable)?;
        let bounded = |reason: &str| {
            !reason.is_empty()
                && reason.chars().count() <= REASON_LIMIT
                && !reason.chars().any(char::is_control)
        };
        let readable = [&isolation.cold, &isolation.replacement, &isolation.resume]
            .into_iter()
            .all(|assessment| match assessment {
                Assessment::Unsupported(reason) => bounded(reason),
                Assessment::Measured | Assessment::Unmeasured => true,
            });
        match readable {
            true => Ok(Some(isolation)),
            false => Err(McpRefusal::UnboundedReason),
        }
    }

    /// Whether `provider` may be served as `invocation`: its assessment
    /// must be measured, and the engine must build a qualified mechanism
    /// for its harness. A measured claim for a harness with none is missing
    /// evidence, never a fallback. dsh's every assessment is missing
    /// evidence until its engine-only home is built (U1c2; operator ruling
    /// of 2026-10-06), whatever limitation the intent records for it.
    fn admit(&self, provider: &'static str, invocation: Invocation<'_>) -> Result<(), McpRefusal> {
        let (assessment, shape) = match invocation {
            Invocation::Cold => (&self.cold, "cold"),
            Invocation::Replacement => (&self.replacement, "replacement"),
            Invocation::Resume(shape) => (&self.resume, shape),
        };
        let unmeasured = || McpRefusal::Unmeasured {
            provider,
            shape: shape.to_string(),
        };
        if ServingShape::of(provider) == Some(ServingShape::Dsh) {
            return Err(unmeasured());
        }
        match assessment {
            Assessment::Unsupported(reason) => {
                return Err(McpRefusal::Unsupported {
                    provider,
                    reason: reason.clone(),
                })
            }
            Assessment::Unmeasured => return Err(unmeasured()),
            Assessment::Measured => {}
        }
        match ServingShape::of(provider) {
            Some(ServingShape::Claude) => Ok(()),
            Some(ServingShape::Codex | ServingShape::Dsh) | None => Err(unmeasured()),
        }
    }
}

/// Whether the typed inputs sealed beside `input` declare the box's hands.
/// Absent inputs declare none; inputs that cannot be read declare nothing
/// the intent can be compared with.
fn sealed_hands(edge: &Edge<'_>) -> Option<bool> {
    match edge.sealed() {
        None => Some(false),
        Some(sealed) => sealed.as_ref().ok().map(|sealed| sealed.spec.is_some()),
    }
}

/// `argv` with the empty set's strict flag and document appended, once
/// `provider`'s grammar places the whole of it as options. The grammar
/// refuses a token it cannot place and a split value that reads as an
/// option, so a whole parse places the appended three as the flag and
/// `--mcp-config` with the document; an option terminator, a dangling value
/// or an argument the grammar does not model before them refuses instead.
fn with_empty_set(
    provider: &'static str,
    mut argv: Vec<String>,
) -> Result<Vec<String>, McpRefusal> {
    argv.extend(["--strict-mcp-config", "--mcp-config", EMPTY_DOCUMENT].map(String::from));
    match placed(provider, &argv) {
        Some(_) => Ok(argv),
        None => Err(McpRefusal::Unplaced { provider }),
    }
}

/// `extra`, a launch's composed arguments for `provider`, with the isolated
/// configuration of the engine's intent in them, or the refusal that stops
/// the launch before any provider work. The intent's server set must be the
/// one the sealed inputs declare: the hands set rides the adapter's measured
/// workspace fragment, whose strict flag and document the final check
/// proves, and the empty set gains the strict flag and the empty document
/// here, beside no other MCP configuration. A launch offered no rejoin is
/// admitted cold; one `offered` a rejoin can always end as its cold
/// replacement, so that shape is admitted for it, and the rejoin itself
/// separately ([`Isolated::resumes`]).
pub(super) fn isolated(
    provider: &'static str,
    edge: &Edge<'_>,
    extra: &[String],
    offered: bool,
) -> Result<Isolated, McpRefusal> {
    let Some(isolation) = Isolation::read(edge.input)? else {
        return Ok(Isolated {
            argv: extra.to_vec(),
            isolation: None,
        });
    };
    isolation.admit(
        provider,
        match offered {
            true => Invocation::Replacement,
            false => Invocation::Cold,
        },
    )?;
    let not_sealed = McpRefusal::NotSealed { provider };
    let hands = isolation.servers == ServerSet::Hands;
    if sealed_hands(edge) != Some(hands) {
        return Err(not_sealed);
    }
    let argv = match hands {
        true => extra.to_vec(),
        false => {
            let configured = extra.iter().any(|part| {
                claude_restriction_control(part).is_some_and(|(control, _)| {
                    matches!(control, "--strict-mcp-config" | "--mcp-config")
                })
            });
            if configured {
                return Err(not_sealed);
            }
            with_empty_set(provider, extra.to_vec())?
        }
    };
    Ok(Isolated {
        argv,
        isolation: Some(isolation),
    })
}

#[cfg(test)]
mod tests;
