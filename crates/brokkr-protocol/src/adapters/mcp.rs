//! The private serving edge and the isolated MCP configuration every model
//! launch is served with (decision 0065 slice two, U1c; requirements SI2 and
//! MB1). The typed inputs the engine sealed beside a launch are read here,
//! once, into the final check's [`Serving`], and the box's hands transport
//! is bound here; beside them, the engine's typed isolation intent names the
//! launch's server set — empty, the box's hands, or one broker per sealed
//! holding beside those hands (U7d) — and each invocation shape's measured
//! assessment. Only Claude's grammar carries brokers; Codex and dsh have
//! no measured broker carrier, so a brokered set refuses for them.
//!
//! Only the U0-qualified mechanisms are built. Claude's grammar, which
//! LaneTally's wrapper forwards, excludes ambient MCP under
//! `--strict-mcp-config` with an engine-written `--mcp-config` (U0 cells C03
//! to C06 and LT03 to LT10). dsh excludes it from an engine-only `DSH_HOME`
//! with the engine's server row in the seat's one `--patch` overlay (U0 D03
//! and D04, U0c K01 to K12r; U1c2), and only on the routes those cells
//! measured. Codex is served its measured isolation options and its
//! store-denying profile (U9c2), and its declared verdicts are read as the
//! others' are. Exec has no model MCP surface (SI2), so its launch
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

use std::cell::{OnceCell, RefCell};
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::{claude_restriction_control, last_message_door, placed, ResumeGate, ServingShape};
use crate::hands::{Sealed, SEALED_REACH};
use crate::native_controls::{
    SealedAssessment, SealedBoundary, SealedBroker, SealedFence, SealedIsolation, SealedServers,
    SealedServing, Transport, ARTIFACT_ROOT, SERVING_INPUTS,
};
use crate::served_home::{dsh_served, DshCause, DshRefusal, DshServed, RouteRefusal};
use crate::transcript::{DshHome, DshHomeError};

mod admit;

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

thread_local! {
    /// The names every harness spawn of the attempt in progress removes
    /// (U7d; MB4), set from that attempt's own sealed serving input by
    /// [`Removing::attempt`] before anything spawns, and empty outside an
    /// attempt. `spawn_harness`, every harness's one spawn door, runs on
    /// the thread that serves the attempt, as `lost`'s spawn fact does, so
    /// a version probe or qualification spawn removes the same names as the
    /// model command, whose final check derives them from the same input.
    static REMOVED: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
}

/// One attempt's removals in force: from [`Removing::attempt`] until the
/// value drops, on return or unwind, when the names in force before it are
/// put back, so no attempt's list outlives it or reaches the next one.
pub(super) struct Removing {
    before: Vec<String>,
}

impl Removing {
    /// Bring `input`'s removals into force for the attempt it starts: its
    /// sealed set's ([`SealedServers::removals`]), the list its final check
    /// is handed. Sealed inputs that do not decode bring none; every launch
    /// reading them refuses as unreadable before any spawn ([`intent`]).
    pub(super) fn attempt(input: &Value) -> Removing {
        let edge = Edge::new(input);
        let names = match edge.sealed() {
            Some(Ok(sealed)) => sealed.isolation.servers.removals(),
            Some(Err(_)) | None => Vec::new(),
        };
        Removing {
            before: REMOVED.replace(names),
        }
    }
}

impl Drop for Removing {
    fn drop(&mut self) {
        REMOVED.set(std::mem::take(&mut self.before));
    }
}

/// `builder` with every name the attempt in progress removes taken out of
/// the harness's environment, inherited, set or bound alike, so no broker
/// the harness spawns inherits one (U7d; MB4).
pub(super) fn unbind(builder: &mut std::process::Command) {
    REMOVED.with_borrow(|names| {
        for name in names {
            builder.env_remove(name);
        }
    });
}

/// The engine's own executable, which a harness spawns as `brokkr hands
/// serve`: the one place a launch reads it.
fn engine_executable() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

/// The box's hands bound to `brokkr`, `workdir`, the artifact root the
/// engine wrote beside the sealed inputs (U8a) and the `reach` it sealed
/// ([`sealed_reach`]; U8a2c), where those declare them; `None` where the
/// site has none. A root the input lacks binds none, matching no server.
fn hands_transport<'a>(
    sealed: &'a SealedServing,
    (brokkr, workdir): (&'a Path, &'a str),
    input: &'a Value,
    reach: &'a Sealed,
) -> Option<Transport<'a>> {
    let artifacts = input.get(ARTIFACT_ROOT).and_then(Value::as_str);
    sealed.spec.as_ref().map(|spec| Transport {
        brokkr,
        workdir: Path::new(workdir),
        artifacts: Path::new(artifacts.unwrap_or_default()),
        sealed: reach,
        spec,
    })
}

/// The reach the engine sealed for the box's hands, read from the input as
/// its type (U8a2c); one absent or unread seals nothing, which no hands
/// server the engine composes carries and no box is built from.
fn sealed_reach(input: &Value) -> Sealed {
    let carried = input.get(SEALED_REACH).cloned().map(serde_json::from_value);
    carried.and_then(Result::ok).unwrap_or_default()
}

/// The box's hands a dsh seat is served through its one overlay (#896):
/// the engine's executable and the `hands serve` arguments the final check
/// binds the box's hands to, where the sealed inputs declare hands a
/// boundary boxes ([`sealed_servers`]). `None` where the input carries no
/// sealed inputs, a driver run by hand, or they declare no boxed hands;
/// inputs that cannot be read refuse, and so do hands no overlay value can
/// bind exactly.
pub(super) fn dsh_hands(
    input: &Value,
    workdir: &str,
) -> Result<Option<(String, Vec<String>)>, McpRefusal> {
    dsh_hands_at(input, workdir, &engine_executable())
}

/// [`dsh_hands`] for the engine's executable at `brokkr`, so a path no
/// overlay value represents is a plain test.
fn dsh_hands_at(
    input: &Value,
    workdir: &str,
    brokkr: &Path,
) -> Result<Option<(String, Vec<String>)>, McpRefusal> {
    let edge = Edge::new(input);
    let Some(sealed) = edge.sealed() else {
        return Ok(None);
    };
    let sealed = sealed.as_ref().map_err(|_| McpRefusal::Unreadable)?;
    if sealed_servers(sealed) != SealedServers::Hands {
        return Ok(None);
    }
    let reach = sealed_reach(input);
    // The set's own check of what can be served exactly, every path UTF-8,
    // which a set of the hands has whenever its spec is sealed.
    let hands = hands_transport(sealed, (brokkr, workdir), input, &reach)
        .filter(|hands| hands.expand(&[]).is_some())
        .ok_or(McpRefusal::Unbindable)?;
    let args = crate::hands::serve_args(hands.workdir, hands.artifacts, hands.sealed, hands.spec);
    Ok(Some((brokkr.to_string_lossy().into_owned(), args)))
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
/// rebuilds that configuration behind the composition (U1g2). The check is
/// handed the sealed brokers beside the box's hands and the names the spawn
/// removes, the attempt's own ([`Removing::attempt`]; U7d).
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
    let (brokkr, reach) = (engine_executable(), sealed_reach(input));
    let hands = hands_transport(sealed, (&brokkr, chosen.workdir), input, &reach);
    let built = edge.built.get().map_or(&[][..], Vec::as_slice);
    let server = hands.filter(|_| sealed_servers(sealed) == SealedServers::Hands);
    let servers = &sealed.isolation.servers;
    let (brokers, removed) = (servers.brokers(), servers.removals());
    final_set(harness, &command, built, server.as_ref(), brokers)
        .map_err(|refusal| refusal.at_launch())?;
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
            brokers,
            removed: &removed,
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
    /// The `mcp__cap-` servers a launch's allow list names, each list
    /// rendered in order, are not its sealed brokers' (U9b1d; MB1).
    #[error(
        "provider '{provider}' final allow list names the brokered MCP servers [{allowed}], but \
         the engine seals [{sealed}]: each sealed broker is allowed once, by its served name, in \
         sealed order, and no other"
    )]
    Unallowed {
        provider: &'static str,
        allowed: String,
        sealed: String,
    },
    #[error(
        "provider '{provider}' arguments do not place the engine's MCP configuration as options \
         of their own: a terminator, a dangling value or an argument the grammar does not model \
         stands before it"
    )]
    Unplaced { provider: &'static str },
    /// A dsh route no engine-only home was measured on, as the one
    /// served-home decision refuses it ([`dsh_served`]).
    #[error(transparent)]
    Route(#[from] RouteRefusal),
    #[error(
        "the box's hands cannot be bound to a dsh seat exactly: a path they name is not UTF-8, \
         and no overlay value represents it (#896)"
    )]
    Unbindable,
}

impl From<DshRefusal> for McpRefusal {
    /// The served-home decision's refusal as the driver states it: the
    /// admission's cause or the route's, each in its own words.
    fn from(DshRefusal(refused): DshRefusal) -> McpRefusal {
        match refused {
            DshCause::Admission(cause) => McpRefusal::Strict(cause),
            DshCause::Route(refusal) => McpRefusal::Route(refusal),
        }
    }
}

impl McpRefusal {
    /// The refusal as a driver states it, before any provider work.
    pub(super) fn at_launch(&self) -> String {
        format!("refusing to invoke the agent CLI: {self}")
    }
}

/// A launch's composed arguments with its isolated configuration in them,
/// the intent they were built from, `None` where the input carried none
/// or the standing fence serves its shape as before, and whether the
/// launch was offered a rejoin.
#[derive(Debug)]
pub(super) struct Isolated {
    pub(super) argv: Vec<String>,
    isolation: Option<SealedIsolation>,
    offered: bool,
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
    /// the seat carries a route overlay, as the one served-home decision
    /// rules it ([`dsh_served`]); an engine-only home rejoins an offered
    /// session only where the intent's resume shape is admitted.
    pub(super) fn dsh(
        &self,
        route: Option<&str>,
        routed: bool,
    ) -> Result<DshIsolation, McpRefusal> {
        let isolation = self.isolation.as_ref();
        match dsh_served(isolation, self.offered, route, routed)? {
            DshServed::Operator => Ok(DshIsolation::Operator),
            DshServed::Engine => Ok(DshIsolation::Engine {
                rejoins: self.resumes("dsh", super::DSH_SHAPE),
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

/// Whether the intent's server set is the one `sealed` inputs declare
/// ([`sealed_servers`]), or at least one broker beside the box's hands
/// where they declare those hands (U7d; MB1, MB2): brokers beside no
/// hands, or an empty broker list, which is the hands set spelt again,
/// never are.
fn declared(sealed: &SealedServing) -> bool {
    match (&sealed.isolation.servers, sealed_servers(sealed)) {
        (SealedServers::Brokered(brokers), SealedServers::Hands) => !brokers.is_empty(),
        (intended, declared) => *intended == declared,
    }
}

/// SI2's final configuration check (U1g2): the MCP options a sealed
/// Claude-grammar command carries must be exactly those of the
/// configuration its intent `built` and, where its sealed set holds it, of
/// the box's hands `server` with its sealed `brokers` beside it, the whole
/// set's one document (U7d). A strict flag removed, another document
/// appended or the built one changed refuses before any server or model
/// starts, and so does a set its builder cannot name. A command that
/// cannot be read whole is the final check's own refusal. Codex builds no
/// isolated configuration, its hands' bindings being the final check's,
/// and dsh carries its set in its engine-only home, not its argv.
fn final_set(
    provider: &'static str,
    command: &[String],
    built: &[String],
    server: Option<&Transport<'_>>,
    brokers: &[SealedBroker],
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
    let not_sealed = McpRefusal::NotSealed { provider };
    let mut owed = placed(provider, built).map_or_else(Vec::new, |built| options(&built));
    if let Some(server) = server {
        let document = server.beside(brokers).ok_or(not_sealed.clone())?;
        owed.push(("--strict-mcp-config".into(), Vec::new()));
        owed.push(("--mcp-config".into(), vec![document.to_string()]));
        owed.sort();
    }
    match options(&carried.command) == owed {
        true => allowed(provider, &carried.command, brokers),
        false => Err(not_sealed),
    }
}

/// The final check's binding of a Claude-grammar `command`'s allowance to
/// its sealed `brokers` (U9b1d; MB1): every value its allow lists name
/// under `mcp__cap-`, read by the grammar, is exactly each broker's served
/// name once, in sealed order ([`SealedBroker::allowance`]). A server the
/// set does not seal, one it seals left out, a name written twice or one
/// tool of a server named refuses before any spawn. A recipe's own `mcp__`
/// allow never reaches here (`authored_server_conflict`).
fn allowed(
    provider: &'static str,
    command: &crate::native_controls::grammar::Command,
    brokers: &[SealedBroker],
) -> Result<(), McpRefusal> {
    use crate::native_controls::grammar::{node_patterns, tool_name, ListKind};
    let allowed: Vec<&str> = (command.lists(ListKind::Allow))
        .flat_map(node_patterns)
        .map(tool_name)
        .filter(|tool| tool.starts_with("mcp__cap-"))
        .collect();
    let sealed = SealedBroker::allowance(brokers).ok_or(McpRefusal::NotSealed { provider })?;
    if allowed == sealed {
        return Ok(());
    }
    Err(McpRefusal::Unallowed {
        provider,
        allowed: allowed.join(", "),
        sealed: sealed.join(", "),
    })
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
/// and dsh's hands ride that overlay (#896), beside which no engine-only
/// home is measured, so a hands set is never admitted for it. A brokered
/// set rides the same fragment, whose document the engine expanded to the
/// whole set (U7d); Codex's fragment
/// and dsh's home carry no broker, so a brokered set refuses for either
/// whichever way the fence stands, never served without the brokers it
/// seals. A launch offered no rejoin is admitted cold; one
/// `offered` a rejoin can always end as its cold replacement, so that shape
/// is admitted for it ([`SealedIsolation::admits`]), and the rejoin itself
/// separately ([`Isolated::resumes`]). A shape not admitted refuses past the
/// fence and, while it stands, is served as before.
pub(super) fn isolated(
    provider: &'static str,
    edge: &Edge<'_>,
    extra: &[String],
    offered: bool,
) -> Result<Isolated, McpRefusal> {
    let shape = ServingShape::of(provider);
    let isolating = super::codex::isolating(shape, edge.input);
    let unbuilt = Isolated {
        argv: [extra, &isolating].concat(),
        isolation: None,
        offered,
    };
    let Some((sealed, isolation)) = intent(edge)? else {
        return Ok(unbuilt);
    };
    let not_sealed = McpRefusal::NotSealed { provider };
    if !declared(sealed) {
        return Err(not_sealed);
    }
    let hands = isolation.servers != SealedServers::Empty;
    if !isolation.servers.brokers().is_empty() && shape != Some(ServingShape::Claude) {
        return Err(McpRefusal::Strict(StrictCause::Unmeasured {
            provider: provider.into(),
            shape: "brokers".into(),
        }));
    }
    let empty = !hands && shape == Some(ServingShape::Claude);
    let configured = extra.iter().any(|part| {
        claude_restriction_control(part)
            .is_some_and(|(control, _)| matches!(control, "--strict-mcp-config" | "--mcp-config"))
    });
    if empty && configured {
        return Err(not_sealed);
    }
    if let Err(refusal) = isolation.admits(provider, offered) {
        isolation.fence.refuses(refusal.into())?;
        edge.built.get_or_init(|| isolating);
        return Ok(unbuilt);
    }
    let argv = match empty {
        true => with_empty_set(provider, extra.to_vec())?,
        false => unbuilt.argv,
    };
    edge.built.get_or_init(|| argv[extra.len()..].to_vec());
    Ok(Isolated {
        argv,
        isolation: Some(isolation.clone()),
        offered,
    })
}

/// An adapter's `harness_auth` as written: the environment variable names
/// its harness reads to authenticate, beside the measurement that found
/// them, and nothing else (MB4).
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct DeclaredAuth {
    names: Vec<String>,
    measured: String,
}

/// Why an adapter's `harness_auth` declaration is refused.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HarnessAuthError {
    /// Not an object of exactly `names` and a non-empty `measured`.
    #[error("must be an object of exactly 'names' and a non-empty 'measured' ({0})")]
    Shape(String),
    /// A name outside decision 0012's grammar.
    #[error("names '{0}', which does not match [A-Z][A-Z0-9_]*")]
    Name(String),
    #[error("names '{0}' twice")]
    Repeated(String),
}

/// The names an adapter's `harness_auth` declaration measured (MB4), read
/// once: each holds decision 0012's name grammar and none is written twice.
/// The measurement is evidence the adapter digest pins; nothing reads it.
pub fn harness_auth(value: &Value) -> Result<Vec<String>, HarnessAuthError> {
    // serde reads an array into a struct by position; only an object is a
    // declaration.
    if !value.is_object() {
        return Err(HarnessAuthError::Shape("it is not an object".into()));
    }
    let declared = <DeclaredAuth as serde::Deserialize>::deserialize(value)
        .map_err(|error| HarnessAuthError::Shape(error.to_string()))?;
    if declared.measured.is_empty() {
        return Err(HarnessAuthError::Shape("'measured' is empty".into()));
    }
    let mut names: Vec<String> = Vec::with_capacity(declared.names.len());
    for name in declared.names {
        if !crate::secret::valid_name(&name) {
            return Err(HarnessAuthError::Name(name));
        }
        if names.contains(&name) {
            return Err(HarnessAuthError::Repeated(name));
        }
        names.push(name);
    }
    Ok(names)
}

#[cfg(test)]
pub(super) mod tests;
