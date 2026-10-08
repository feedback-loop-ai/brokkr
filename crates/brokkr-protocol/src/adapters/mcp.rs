//! The private serving edge and the isolated MCP configuration every model
//! launch is served with (decision 0065 slice two, U1c; requirements SI2 and
//! MB1). The typed inputs the engine sealed beside a launch are read here,
//! once, into the final check's [`Serving`], and the box's hands transport
//! is bound here; beside them, the engine's typed isolation intent names the
//! launch's server set — empty or hands-only, the only sets this
//! no-broker plan has — and each invocation shape's measured assessment.
//!
//! Only the U0-qualified mechanisms are built. Claude's grammar, which
//! LaneTally's wrapper forwards, excludes ambient MCP under
//! `--strict-mcp-config` with an engine-written `--mcp-config` (U0 cells C03
//! to C06 and LT03 to LT10). dsh excludes it from an engine-only `DSH_HOME`
//! with the engine's server row in the seat's one `--patch` overlay (U0 D03
//! and D04, U0c K01 to K12r; U1c2), and only on the routes those cells
//! measured. Codex has no passing candidate, so an intent for it refuses
//! rather than borrowing another harness's result or falling back to
//! ambient configuration. Exec has no model MCP surface (SI2), so its launch
//! reads no intent.
//!
//! Dispatch seals the intent inside the serving inputs of every launch it
//! seals (U1g2), so a sealed launch cannot lose it, and a driver run by
//! hand, which carries none, is served as before. The server set and the
//! configuration built from it are checked whichever way the MCP compile
//! fence stands; a shape SI2 does not admit refuses only past the fence the
//! intent names, and until U9b lifts it is served as before (operator
//! ruling of 2026-10-07).
//!
//! [`Serving`]: crate::native_controls::Serving

use std::cell::OnceCell;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{claude_restriction_control, last_message_door, placed, ResumeGate, ServingShape};
use crate::native_controls::{
    SealedAssessment, SealedBoundary, SealedFence, SealedIsolation, SealedServers, SealedServing,
    Transport, SERVING_INPUTS,
};
use crate::transcript::{DshHome, DshHomeError};

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
/// ([`isolated`]) and the final check ([`served`]). Beside it, the isolated
/// configuration [`isolated`] built behind the launch's arguments, which
/// the final check rebuilds and requires.
pub(super) struct Edge<'a> {
    input: &'a Value,
    sealed: OnceCell<Result<SealedServing, String>>,
    built: OnceCell<Vec<String>>,
}

impl<'a> Edge<'a> {
    pub(super) fn new(input: &'a Value) -> Self {
        Edge {
            input,
            sealed: OnceCell::new(),
            built: OnceCell::new(),
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
/// Before the check, a sealed launch's MCP options must be exactly those of
/// the configuration its intent built and of its sealed server set
/// ([`final_set`]), or it refuses with SI2's exact cause; the check then
/// rebuilds that configuration behind the composition (U1g2).
///
/// [`check_final`]: crate::native_controls::check_final
/// [`Checked::into_argv`]: crate::native_controls::Checked::into_argv
pub(super) fn served(
    harness: &'static str,
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
    let hands = hands_transport(sealed, &brokkr, chosen.workdir);
    let built = edge.built.get().map_or(&[][..], Vec::as_slice);
    let server = hands.filter(|_| sealed_servers(sealed) == SealedServers::Hands);
    final_set(harness, &command, built, server.as_ref()).map_err(|refusal| refusal.at_launch())?;
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
            mcp: built,
            output: last_message_door(input)
                .then(|| input["result_path"].as_str().unwrap_or_default()),
            hands,
            ..chosen
        },
    )
    .map(crate::native_controls::Checked::into_argv)
    .map_err(|refusal| refusal.at_launch(input))
}

/// The shape a launch is about to be served as: cold, the cold replacement
/// of an offered rejoin, or a rejoin of the named resume shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Invocation<'a> {
    Cold,
    Replacement,
    Resume(&'a str),
}

/// Why a site's provider cannot be served strict MCP isolation for one
/// invocation shape: SI2's two exact site causes, which the compile pass
/// and the serving edge state alike. `reason` is the bounded measured
/// reason of validated adapter evidence.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StrictCause {
    #[error("provider '{provider}' cannot exclude ambient MCP configuration ({reason})")]
    Unsupported { provider: String, reason: String },
    #[error("provider '{provider}' has no measured strict MCP configuration for '{shape}'")]
    Unmeasured { provider: String, shape: String },
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
    #[error(transparent)]
    Strict(#[from] StrictCause),
    #[error("provider '{provider}' final MCP configuration is not the engine's sealed server set")]
    NotSealed { provider: &'static str },
    #[error(
        "provider '{provider}' arguments do not place the engine's MCP configuration as options \
         of their own: a terminator, a dangling value or an argument the grammar does not model \
         stands before it"
    )]
    Unplaced { provider: &'static str },
    #[error("provider 'dsh' has no measured strict MCP configuration on route '{route}'")]
    UnmeasuredRoute { route: String },
    #[error(
        "provider 'dsh' has no measured strict MCP configuration for a seat that pins no \
         `--model`: the route it would run is the profile's unnamed default"
    )]
    Unpinned,
    #[error("provider 'dsh' route '{route}' {}", row.mismatch())]
    RouteEntry { route: String, row: RouteRow },
}

/// How a qualified dsh route reached its engine-only home when U0 or U0c
/// measured it: on `dsh-base`'s shipped row alone, or with its validated
/// `llm-pi-ai` provider entry, which production carries as the seat's one
/// bound route overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum RouteRow {
    Shipped,
    Overlay,
}

impl RouteRow {
    /// Why a seat's overlay is not the measured configuration of a route
    /// whose row is `self`.
    fn mismatch(self) -> &'static str {
        match self {
            RouteRow::Shipped => {
                "was measured on dsh-base's shipped row alone, and the seat's overlay carries a \
                 provider entry for it"
            }
            RouteRow::Overlay => {
                "was measured with its validated provider entry, and the seat carries no route \
                 overlay"
            }
        }
    }
}

/// The routes, by the provider segment of the seat's model pin, on which
/// an engine-only dsh home was measured to exclude ambient MCP and serve
/// the engine's server: `spark-glm` by U0 (D03 and D04), the four keyed
/// families by U0c, and nothing else. U0c's `spark` cells are partial, no
/// tool call observed, so `spark` is unmeasured (operator ruling
/// 2026-10-07).
const DSH_ROUTES: [(&str, RouteRow); 5] = [
    ("spark-glm", RouteRow::Overlay),
    ("deepseek-official", RouteRow::Shipped),
    ("dashscope", RouteRow::Overlay),
    ("meta", RouteRow::Overlay),
    ("meta-contributor", RouteRow::Overlay),
];

impl McpRefusal {
    /// The refusal as a driver states it, before any provider work.
    pub(super) fn at_launch(&self) -> String {
        format!("refusing to invoke the agent CLI: {self}")
    }
}

/// A launch's composed arguments with its isolated configuration in them,
/// and the intent they were built from, `None` where the input carried none
/// or the standing fence serves its shape as before.
#[derive(Debug)]
pub(super) struct Isolated {
    pub(super) argv: Vec<String>,
    isolation: Option<SealedIsolation>,
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

/// The serving inputs sealed beside `edge`'s launch and the engine's intent
/// sealed in them, `None` where the input carries none: a driver run by
/// hand. Inputs that cannot be read, an intent absent from them among
/// their faults, carry no intent to serve by, and a measured reason must be
/// one bounded line.
fn intent<'e>(
    edge: &'e Edge<'_>,
) -> Result<Option<(&'e SealedServing, &'e SealedIsolation)>, McpRefusal> {
    let Some(sealed) = edge.sealed() else {
        return Ok(None);
    };
    let sealed = sealed.as_ref().map_err(|_| McpRefusal::Unreadable)?;
    let isolation = &sealed.isolation;
    let bounded = |reason: &str| {
        !reason.is_empty()
            && reason.chars().count() <= REASON_LIMIT
            && !reason.chars().any(char::is_control)
    };
    let readable = [&isolation.cold, &isolation.replacement]
        .into_iter()
        .chain(isolation.resume.values())
        .all(|assessment| match assessment {
            SealedAssessment::Unsupported(reason) => bounded(reason),
            SealedAssessment::Measured | SealedAssessment::Unmeasured => true,
        });
    match readable {
        true => Ok(Some((sealed, isolation))),
        false => Err(McpRefusal::UnboundedReason),
    }
}

impl SealedBoundary {
    /// Whether Brokkr builds this boundary's box, as core's
    /// `Boundary::is_boxed` rules it: this crate reads no core type, so the
    /// copy is held to it by the runtime's
    /// `a_sealed_boundary_boxes_exactly_where_core_rules_it`.
    pub fn boxes(self) -> bool {
        use SealedBoundary::{Container, Harness, Namespace, Open, Seatbelt};
        match self {
            Namespace | Seatbelt | Container => true,
            Harness | Open => false,
        }
    }
}

impl SealedFence {
    /// `refusal` past the lifted fence; nothing while it stands.
    fn refuses(self, refusal: McpRefusal) -> Result<(), McpRefusal> {
        match self {
            SealedFence::Standing => Ok(()),
            SealedFence::Lifted => Err(refusal),
        }
    }
}

impl SealedIsolation {
    /// Whether `provider` may be served as `invocation`: its assessment
    /// must be measured, and the engine must build a qualified mechanism
    /// for its harness. A measured claim for a harness with none is missing
    /// evidence, never a fallback. dsh's route is judged separately
    /// ([`Isolated::dsh`]), once its model pin is read.
    fn admit(&self, provider: &'static str, invocation: Invocation<'_>) -> Result<(), McpRefusal> {
        let (assessment, shape) = match invocation {
            Invocation::Cold => (&self.cold, "cold"),
            Invocation::Replacement => (&self.replacement, "replacement"),
            Invocation::Resume(shape) => (
                self.resume
                    .get(shape)
                    .unwrap_or(&SealedAssessment::Unmeasured),
                shape,
            ),
        };
        let unmeasured = || {
            McpRefusal::Strict(StrictCause::Unmeasured {
                provider: provider.into(),
                shape: shape.to_string(),
            })
        };
        match assessment {
            SealedAssessment::Unsupported(reason) => {
                return Err(McpRefusal::Strict(StrictCause::Unsupported {
                    provider: provider.into(),
                    reason: reason.clone(),
                }))
            }
            SealedAssessment::Unmeasured => return Err(unmeasured()),
            SealedAssessment::Measured => {}
        }
        match ServingShape::of(provider) {
            Some(ServingShape::Claude | ServingShape::Dsh) => Ok(()),
            Some(ServingShape::Codex) | None => Err(unmeasured()),
        }
    }
}

/// How one dsh launch is served: from the operator's home where the input
/// carries no intent, as before, or from an engine-only home, rejoining an
/// offered session only where the intent's resume shape is admitted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DshIsolation {
    Operator,
    Engine { rejoins: bool },
}

impl Isolated {
    /// dsh's serving under this launch's intent, once the seat's model pin
    /// is read to the provider segment it names, its route, and whether
    /// the seat carries a route overlay: a route U0 or U0c did not measure,
    /// a seat that pins none and an overlay that is not the measured
    /// route's each refuse before any route is claimed or home staged past
    /// the fence, and while it stands are served from the operator's home.
    pub(super) fn dsh(
        &self,
        route: Option<&str>,
        routed: bool,
    ) -> Result<DshIsolation, McpRefusal> {
        let Some(isolation) = &self.isolation else {
            return Ok(DshIsolation::Operator);
        };
        match Isolated::route(route, routed) {
            Ok(()) => Ok(DshIsolation::Engine {
                rejoins: self.resumes("dsh", super::DSH_SHAPE),
            }),
            Err(refusal) => (isolation.fence.refuses(refusal)).map(|()| DshIsolation::Operator),
        }
    }

    /// Whether `route`, with or without a route overlay, is one U0 or U0c
    /// measured an engine-only home on.
    fn route(route: Option<&str>, routed: bool) -> Result<(), McpRefusal> {
        let route = route.ok_or(McpRefusal::Unpinned)?;
        let Some((_, row)) = DSH_ROUTES.iter().find(|(name, _)| *name == route) else {
            return Err(McpRefusal::UnmeasuredRoute {
                route: route.to_string(),
            });
        };
        match routed == (*row == RouteRow::Overlay) {
            true => Ok(()),
            false => Err(McpRefusal::RouteEntry {
                route: route.to_string(),
                row: *row,
            }),
        }
    }
}

impl DshIsolation {
    /// The adapter's resume gate under this serving: a gate the adapter
    /// closed keeps its own reason, and an open one closes as
    /// `restrictions-unavailable` where the engine-only home may not
    /// rejoin, so the offer is declined and the launch is its cold
    /// replacement, never recorded as resume support.
    pub(super) fn gate(self, gate: ResumeGate) -> ResumeGate {
        match (self, gate) {
            (DshIsolation::Engine { rejoins: false }, ResumeGate::Enabled { .. }) => {
                ResumeGate::Disabled("restrictions-unavailable")
            }
            (DshIsolation::Engine { rejoins: true } | DshIsolation::Operator, gate)
            | (DshIsolation::Engine { rejoins: false }, gate @ ResumeGate::Disabled(_)) => gate,
        }
    }

    /// The home this serving reads, staged now for the engine.
    pub(super) fn home(self, workdir: &str) -> Result<DshHome, DshHomeError> {
        match self {
            DshIsolation::Operator => DshHome::operator(),
            DshIsolation::Engine { .. } => DshHome::stage(workdir),
        }
    }
}

/// The server set `sealed` inputs declare (U1g2): the box's hands server
/// where they seal typed hands under a boundary that boxes them, and the
/// empty set otherwise. Under `harness` the harness's own sandbox carries an
/// office's hands and under `open` nothing does, so hands sealed under
/// either are served the empty set, as the compile intended it.
fn sealed_servers(sealed: &SealedServing) -> SealedServers {
    let boxed = sealed.dialect.stands.is_some_and(SealedBoundary::boxes);
    match sealed.spec.is_some() && boxed {
        true => SealedServers::Hands,
        false => SealedServers::Empty,
    }
}

/// SI2's final configuration check (U1g2): the MCP options a sealed
/// Claude-grammar command carries must be exactly those of the
/// configuration its intent `built` and, where its sealed set holds it, of
/// the box's hands `server`. A strict flag removed, another document
/// appended or the built one changed refuses before any server or model
/// starts. A command that cannot be read whole is the final check's own
/// refusal. Codex builds no isolated configuration, its hands' bindings
/// being the final check's, and dsh carries its set in its engine-only
/// home, not its argv.
fn final_set(
    provider: &'static str,
    command: &[String],
    built: &[String],
    server: Option<&Transport<'_>>,
) -> Result<(), McpRefusal> {
    use crate::native_controls::grammar::{parse_final, Command};
    if ServingShape::of(provider) != Some(ServingShape::Claude) {
        return Ok(());
    }
    let Some(Ok(carried)) = command
        .get(1..)
        .and_then(|argv| parse_final(provider, argv))
    else {
        return Ok(());
    };
    let options = |command: &Command| {
        let mut options: Vec<(String, Vec<String>)> = (command.nodes.iter())
            .filter(|node| matches!(node.name(), "--strict-mcp-config" | "--mcp-config"))
            .map(|node| (node.name().to_string(), node.values.clone()))
            .collect();
        options.sort();
        options
    };
    let mut owed = placed(provider, built).map_or_else(Vec::new, |built| options(&built));
    if let Some(server) = server {
        let document = crate::hands::mcp_config(server.brokkr, server.workdir, server.spec);
        owed.push(("--strict-mcp-config".into(), Vec::new()));
        owed.push(("--mcp-config".into(), vec![document.to_string()]));
        owed.sort();
    }
    match options(&carried.command) == owed {
        true => Ok(()),
        false => Err(McpRefusal::NotSealed { provider }),
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
/// the launch before any provider work. Whichever way the fence stands, the
/// intent's server set must be the one the sealed inputs declare, and the
/// empty set is served beside no other MCP configuration: the hands set
/// rides the adapter's measured workspace fragment, whose strict flag and
/// document the final check proves, and the empty set gains the strict
/// flag and the empty document here. dsh's empty set is its engine-only
/// home with no server row in its overlay, so its arguments are unchanged,
/// and dsh holds no hands (its adapter's measured fact), so a hands set is
/// never measured for it. A launch offered no rejoin is admitted cold; one
/// `offered` a rejoin can always end as its cold replacement, so that shape
/// is admitted for it, and the rejoin itself separately
/// ([`Isolated::resumes`]). A shape not admitted refuses past the fence and,
/// while it stands, is served as before.
pub(super) fn isolated(
    provider: &'static str,
    edge: &Edge<'_>,
    extra: &[String],
    offered: bool,
) -> Result<Isolated, McpRefusal> {
    let unbuilt = Isolated {
        argv: extra.to_vec(),
        isolation: None,
    };
    let Some((sealed, isolation)) = intent(edge)? else {
        return Ok(unbuilt);
    };
    let not_sealed = McpRefusal::NotSealed { provider };
    if sealed_servers(sealed) != isolation.servers {
        return Err(not_sealed);
    }
    let hands = isolation.servers == SealedServers::Hands;
    let shape = ServingShape::of(provider);
    let empty = !hands && shape == Some(ServingShape::Claude);
    let configured = extra.iter().any(|part| {
        claude_restriction_control(part)
            .is_some_and(|(control, _)| matches!(control, "--strict-mcp-config" | "--mcp-config"))
    });
    if empty && configured {
        return Err(not_sealed);
    }
    let invocation = match offered {
        true => Invocation::Replacement,
        false => Invocation::Cold,
    };
    let admitted = isolation.admit(provider, invocation).and_then(|()| {
        match hands && shape == Some(ServingShape::Dsh) {
            true => Err(McpRefusal::Strict(StrictCause::Unmeasured {
                provider: provider.into(),
                shape: "hands".into(),
            })),
            false => Ok(()),
        }
    });
    if let Err(refusal) = admitted {
        return isolation.fence.refuses(refusal).map(|()| unbuilt);
    }
    let argv = match empty {
        true => with_empty_set(provider, extra.to_vec())?,
        false => extra.to_vec(),
    };
    edge.built.get_or_init(|| argv[extra.len()..].to_vec());
    Ok(Isolated {
        argv,
        isolation: Some(isolation.clone()),
    })
}

#[cfg(test)]
pub(super) mod tests;
