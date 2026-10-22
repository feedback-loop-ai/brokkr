//! Doctor's whole-plan MCP lines (decision 0065 slice two, U9a2): each
//! valid `mcp` grant's facts read past the realm-wide refusal that stands
//! until U9b, its egress by the compile's own comparison, and each seat's
//! whole plan as the compile's own seat pass judges it in a survey of the
//! bundle the report reads. Facts to print, never a bundle, a spawn or an
//! authority any compile accepts.

use brokkr_core::realms::{Boundary, CapabilityGrant};
use brokkr_protocol::adapters::StrictCause;
use brokkr_protocol::native_controls::bounded_line;
use brokkr_runtime::agents::EgressClass;
use brokkr_runtime::bundle::{CompileError, Survey};
use brokkr_runtime::capabilities::{
    admitted, Authority, Connection, DialectKind, McpServer, Outcome, Retention, SiteAsks,
    ToolDialect,
};
use brokkr_runtime::launch::LaunchError;

use super::{restriction_words, safe, scope_words, Report};

/// The binding minimum an `mcp` grant's egress is compared against, as
/// the survey of the bundle the report reads parsed it, and whose it is.
pub(super) enum Minimum {
    /// The bundle the report reads: the one it declares, or the default
    /// a bundle that declares none gets.
    Bundle(EgressClass),
    /// No bundle was given or found: the default compile uses.
    Default(EgressClass),
    /// The bundle could not be surveyed: no comparison is made, and no
    /// default is guessed in its place.
    Unread(String),
}

impl Minimum {
    /// The minimum `surveyed` read, the bundle's where one was `given`.
    pub(super) fn of(given: bool, surveyed: &Result<Survey, LaunchError>) -> Minimum {
        match (given, surveyed) {
            (true, Ok(survey)) => Minimum::Bundle(survey.minimum),
            (false, Ok(survey)) => Minimum::Default(survey.minimum),
            (_, Err(problem)) => Minimum::Unread(bounded_line(&problem.to_string())),
        }
    }
}

/// The host an MCP broker's namespace box would stand on (decision 0063):
/// what a static line can say of it, never a measurement.
#[cfg(not(target_os = "macos"))]
const HOST_BOX: &str = "on this Linux host a namespace box is what would serve it; whether this \
                        host can build one is not measured by this line, and no box was started";
#[cfg(target_os = "macos")]
const HOST_BOX: &str = "on macOS no namespace box exists, so it cannot be served on this host \
                        (decision 0077)";

/// MB3's installation layout, judged without running the server: static
/// requirements, kept apart from the host line's unmeasured facts.
const INSTALLATION: &str = "installation the broker requires (MB3), judged without running the \
                            server: argv[0] resolves once, from an absolute path or \
                            /usr/local/bin:/usr/bin:/bin, never the working directory or PATH; \
                            a system entry sits directly in /usr/bin, /usr/sbin, /usr/local/bin \
                            or /usr/local/sbin, its program tree is that one singly linked file, \
                            and what it loads comes only from the fixed system set; any other \
                            file is a package entry, whose tree is its canonical parent, or that \
                            parent's parent where the parent is bin or sbin, every file of it \
                            singly linked, and never /, the host HOME or an ancestor of it; the \
                            tree stands outside every seat's reach";

/// A valid `mcp` grant's own lines (U9a2), read through its authority
/// `surveyed` ALONE: past the compile's realm-wide refusal, which the
/// grant's line above keeps, and never compiled past it. Nothing for a
/// grant of another kind, or one that did not validate, whose line above
/// already fails. Each line is a warning, since no seat can hold it until
/// U9b; a minimum that could not be read fails its own.
pub(super) fn report_mcp(
    report: &mut Report,
    what: &str,
    (capability, grant): (&str, &CapabilityGrant),
    surveyed: Option<&Authority>,
    (minimum, boundary): (&Minimum, Boundary),
) {
    let found = surveyed.and_then(|authority| authority.dialects.get(capability));
    let Some(dialect) = found else {
        return;
    };
    let server = match &dialect.kind {
        DialectKind::Mcp(server) => server,
        DialectKind::Native { .. } | DialectKind::Hands => return,
    };
    let line = format!("{what} '{capability}'");
    let facts = format!(
        "dialect '{}' (mcp) · tools [{}] · {} · restrictions {} · retention {} · server {} · \
         secret bindings [{}], whose values doctor never reads · no seat holds it until decision \
         0065 slice two lifts the compile fence, whose refusal is the line above",
        dialect.name,
        grant.tools.as_ref().unwrap_or(&dialect.tools).join(", "),
        scope_words(grant),
        restriction_words(grant),
        Retention::of(dialect, grant),
        server_words(server),
        server.secrets.join(", "),
    );
    report.warn(&format!("{line} mcp"), safe(&facts));
    report_egress(report, &line, dialect, minimum);
    report.warn(&format!("{line} box"), box_words(boundary));
    report.warn(
        &format!("{line} host"),
        safe(&format!(
            "UNMEASURED: whether the server {} is installed or reachable here, where it \
             resolves, whether its program tree meets the installation rule above, and whether \
             it starts and reports version '{}'; doctor launches no broker, box or server, reads \
             no secret value and claims no readiness",
            server_words(server),
            server.version
        )),
    );
}

/// The server an `mcp` dialect connects to, as its declaration names it.
fn server_words(server: &McpServer) -> String {
    match &server.connection {
        Connection::Stdio(argv) => format!("'{}' (stdio)", argv.join(" ")),
        Connection::Url(url) => format!("at URL '{url}'"),
    }
}

/// MB4 as U5a2 judges it at compile: `dialect`'s egress against the
/// binding `minimum`, named with whose it is, by the compile's own
/// comparison ([`admitted`]) and in its own words. What a seat that
/// requires or wants it is planned is that seat's own line.
fn report_egress(report: &mut Report, line: &str, dialect: &ToolDialect, minimum: &Minimum) {
    let line = format!("{line} egress");
    let egress = dialect.egress.name();
    let (minimum, whose) = match minimum {
        Minimum::Bundle(minimum) => (*minimum, "the bundle's, as compile parses it"),
        Minimum::Default(minimum) => (
            *minimum,
            "the default compile uses, as no bundle was given or found",
        ),
        Minimum::Unread(problem) => {
            let problem = format!(
                "egress '{egress}'; the bundle's binding minimum could not be read, so no \
                 comparison is made and no default is assumed: {problem}"
            );
            return report.missing(&line, problem);
        }
    };
    let judged = match admitted(dialect, minimum) {
        Ok(_) => "meets it".to_string(),
        Err(below) => format!("is below it: {below}"),
    };
    let detail = format!(
        "egress '{egress}' against binding minimum '{}' ({whose}) {judged}, by the compile's own \
         comparison; what each seat that asks for it is planned is that seat's line",
        minimum.name(),
    );
    report.warn(&line, safe(&detail));
}

/// MB2's static eligibility under a realm's `boundary`, the host this
/// build serves, and the installation rule the broker holds a server to:
/// static facts, kept apart from the host line's unmeasured ones.
fn box_words(boundary: Boundary) -> String {
    let reach = match boundary {
        Boundary::Namespace => {
            "a seat here is eligible only with boxed workspace hands and network off, on a model \
             harness whose strictness is measured and whose adapter carries MCP"
        }
        Boundary::Seatbelt | Boundary::Container | Boundary::Harness | Boundary::Open => {
            "so no seat in this realm can hold it"
        }
    };
    format!(
        "static eligibility (MB2): an MCP capability is served only through the engine's broker, \
         in a namespace box of its own beside a seat's boxed workspace hands (decision 0077) · \
         this realm's boundary is '{boundary}': {reach} · {HOST_BOX} · {INSTALLATION}"
    )
}

/// Each seat's whole capability plan as the survey of the bundle the
/// report reads judged it, by the compile's own seat pass, one line per
/// serving: an agent's candidates each on their own, never their union,
/// and an inline site's one. Then each site that pass refuses, with the
/// compile's refusal and U1g1's verdict on each serving its planner
/// reached, and what stopped the pass where no site owns it. Nothing where
/// no bundle was surveyed.
pub(super) fn report_seats(report: &mut Report, surveyed: Option<&Survey>) {
    let Some(survey) = surveyed else {
        return;
    };
    for (label, capabilities) in &survey.sites {
        // Every planned site passed through the planner, which recorded
        // its verdicts beside its plan.
        let servings = capabilities.outcomes.iter().zip(&survey.strict[label]);
        for (outcome, refusal) in servings {
            report_serving(report, label, &capabilities.asks, outcome, refusal.as_ref());
        }
    }
    for (label, refusal) in &survey.refused {
        report_refused(report, label, refusal, survey.strict.get(label));
    }
    if let Some(failed) = &survey.failed {
        let stopped = format!(
            "the compile's seat pass stopped before every site was read, and what it read is \
             above: {failed}"
        );
        report.warn("capabilities survey", safe(&stopped));
    }
}

/// A site whose capability record refuses, in the compile's words: what a
/// compile of it would refuse with once no realm-wide refusal came first,
/// and U1g1's verdict on each serving its planner reached, read apart from
/// that refusal; `strict` is `None` where the planner was never reached.
fn report_refused(
    report: &mut Report,
    label: &str,
    refusal: &CompileError,
    strict: Option<&Vec<Option<StrictCause>>>,
) {
    let verdicts = match strict {
        None => "never reached, as the site was refused before any serving was planned".into(),
        Some(strict) => listed(strict.iter().enumerate().map(|(index, verdict)| {
            format!("serving {} {}", index + 1, verdict_words(verdict.as_ref()))
        })),
    };
    report.warn(
        &safe(&format!("capabilities seat '{label}'")),
        safe(&format!(
            "the compile's seat pass refuses it: {refusal} · {STRICT} {verdicts}"
        )),
    );
}

/// How U1g1's strict MCP isolation is named on a seat's line.
const STRICT: &str = "U1g1's strict MCP isolation, which refuses only once decision 0065 slice \
                      two lifts the compile fence,";

/// One serving's verdict under U1g1's admission.
fn verdict_words(refusal: Option<&StrictCause>) -> String {
    match refusal {
        None => "admits every shape it can be served as".to_string(),
        Some(cause) => format!("refuses it: {cause}"),
    }
}

/// One serving of one seat: what its office asks, what it holds and why
/// it does not hold the rest, in the compile's words, and U1g1's
/// strict-isolation verdict, which refuses only once U9b lifts the fence.
fn report_serving(
    report: &mut Report,
    label: &str,
    asks: &SiteAsks,
    outcome: &Outcome,
    refusal: Option<&StrictCause>,
) {
    let line = safe(&format!(
        "capabilities seat '{label}' {} model {}",
        outcome.provider,
        outcome.model.as_deref().unwrap_or("none")
    ));
    let asked = asks
        .asks
        .iter()
        .map(|(capability, strength)| format!("{} '{capability}'", strength.word()));
    let held = outcome.held.iter().map(|(capability, holding)| {
        format!(
            "'{capability}' through dialect '{}' with tools [{}]",
            holding.dialect,
            holding.tools.join(", ")
        )
    });
    let unheld =
        (outcome.not_held.iter()).map(|(capability, why)| format!("'{capability}' ({why})"));
    let detail = safe(&format!(
        "office '{}' asks: {} · holds: {} · not held: {} · {STRICT} {}",
        asks.office,
        listed(asked),
        listed(held),
        listed(unheld),
        verdict_words(refusal),
    ));
    match refusal {
        None => report.ok(&line, detail),
        Some(_) => report.warn(&line, detail),
    }
}

/// Items joined by `, `, or `nothing`.
fn listed(items: impl Iterator<Item = String>) -> String {
    let items: Vec<String> = items.collect();
    match items.is_empty() {
        true => "nothing".to_string(),
        false => items.join(", "),
    }
}
