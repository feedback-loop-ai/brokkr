//! The served-home model (#717, the dsh key check's ruled follow-up): each
//! dsh seat of a folder's compile is served from the home brokkr-protocol's
//! own decision rules for it ([`dsh_served`]), by the isolation intent its
//! launch carries, the one dispatch seals beside it ([`sealed_isolation`]),
//! read through the admission the driver applies to its cold launch and to
//! its resumed one, the replacement a rejoin offer launches, the route its
//! `--model` is served on and whether it carries a `--patch` route overlay.
//! The seats are grouped by route and the homes of both launches, and each
//! group is answered for the sources its engine-only home reads where
//! either launch has one, fail-closed, and its one home's otherwise. A
//! seat whose launch the compile does not carry is named by its site, and
//! leaves every other seat's answer standing.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use brokkr_protocol::native_controls::SealedIsolation;
use brokkr_protocol::{dsh_served, holds_dotenv, DshServed};
use brokkr_runtime::agents::Adapter;
use brokkr_runtime::bundle::Bundle;
use brokkr_runtime::engine::sealed_isolation;

use super::{keyed, reach, reaches, with_caveats, Line, Reach, Source, Sources};
use crate::render::Safe;

/// The provider every dsh seat is hired from.
const DSH: &str = "dsh";

/// One dsh seat of a compiled folder, as its launch is served: the site it
/// runs at, the model its `--model` pins, whether it carries a `--patch`,
/// and the isolation intent its launch carries, none where it carries none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Seat {
    pub(crate) label: String,
    pub(crate) model: String,
    pub(crate) routed: bool,
    pub(crate) isolation: Option<SealedIsolation>,
}

/// The home a group of seats is served from on one route.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Home {
    /// The operator's own dsh home, which reads every source asked.
    Operator,
    /// An engine-only home, which reads the launching environment alone.
    Engine,
    /// An engine-only home, which refuses each launch while the `.env` at
    /// this path stands in the repository the seats run in.
    Workdir(PathBuf),
    /// Refused by the served-home decision, in its words.
    Refused(String),
}

/// The engine-only home, as a line names it.
const ENGINE: &str = "its own engine-only home (decision 0065)";

/// Where the operator's home serves a launch from, as a line says it.
const FROM_OPERATOR: &str =
    "from the operator's own dsh home, which reads every source this check asks";

/// What an engine-only home reads, as a line says it after [`ENGINE`].
const ENVIRONMENT_ALONE: &str = "where dsh reads a key from the launching environment alone, \
                                 never the operator's .env or patch layers";

impl Home {
    /// The home line's words for `seats`, whose cold launch `self` serves
    /// and whose resumed launch `resumed` does: one sentence where both
    /// launches are served from one home, and where they are not, what
    /// Brokkr does with each ([`Home::clause`]).
    pub(super) fn words(&self, resumed: &Home, seats: &[String]) -> String {
        let seats = named(seats);
        if self != resumed {
            let cold = self.clause(&format!("a cold launch of {seats} on this route"), "cold");
            let resumed = resumed.clause("a resumed launch", "replacement");
            return format!("Brokkr {cold}, and {resumed}");
        }
        match self {
            Home::Operator => format!("Brokkr serves {seats} on this route {FROM_OPERATOR}"),
            Home::Engine => {
                format!("Brokkr serves {seats} on this route from {ENGINE}, {ENVIRONMENT_ALONE}")
            }
            Home::Workdir(file) => format!(
                "Brokkr serves {seats} on this route from {ENGINE}, and refuses each launch while \
                 {} is present, unread, since dsh would read it for a key; export the key where \
                 brokkr runs and remove the file",
                file.display()
            ),
            Home::Refused(refusal) => format!(
                "Brokkr refuses to launch {seats} on this route: {}",
                Safe::new(refusal).as_str()
            ),
        }
    }

    /// Whether this home refuses the launches it is asked to serve.
    pub(crate) fn refuses(&self) -> bool {
        matches!(self, Home::Workdir(_) | Home::Refused(_))
    }

    /// What Brokkr does with `launch`, one offer state of seats whose other
    /// state another home serves, its invocation `shape`: a launch served
    /// from the operator's home is so served because the driver does not
    /// admit that shape while the fence stands ([`dsh_served`]).
    fn clause(&self, launch: &str, shape: &str) -> String {
        match self {
            Home::Operator => format!(
                "serves {launch} {FROM_OPERATOR}, while the driver does not admit its {shape} shape"
            ),
            Home::Engine => format!("serves {launch} from {ENGINE}, {ENVIRONMENT_ALONE}"),
            Home::Workdir(file) => format!(
                "refuses {launch} from {ENGINE} while {} is present, unread, since dsh would \
                 read it for a key (export the key where brokkr runs and remove the file)",
                file.display()
            ),
            Home::Refused(refusal) => {
                format!("refuses {launch} ({})", Safe::new(refusal).as_str())
            }
        }
    }

    /// Whether this home is an engine-only one, which reads a key from the
    /// launching environment alone or refuses its launch beside a `.env`.
    fn engine_only(&self) -> bool {
        matches!(self, Home::Engine | Home::Workdir(_))
    }
}

/// The dsh seats of a compiled folder: each whose launch is read, and the
/// sites of each whose launch can't be read so.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct Seats {
    pub(crate) read: Vec<Seat>,
    pub(crate) unread: BTreeSet<String>,
}

/// Each dsh seat of `compiled`, one per site and candidate its chain hires
/// from dsh, read from the candidate's argv as the launch reads it. A dsh
/// seat whose launch can't be read so, an inline dsh site, whose argv the
/// compile does not carry, or a candidate that pins no `--model`, is
/// named by its site alone, and every other seat is still read.
pub(crate) fn seats(compiled: &Bundle) -> Seats {
    let mut seats = Seats::default();
    for (label, facts) in &compiled.sites {
        let serves = facts
            .capabilities
            .as_ref()
            .map_or(&[][..], |site| &site.outcomes);
        if facts.chain.is_empty() && serves.iter().any(|outcome| outcome.provider == DSH) {
            seats.unread.insert(label.clone());
        }
        for candidate in facts.chain.iter().filter(|link| link.provider == DSH) {
            let argv = &candidate.argv;
            let pinned = argv.iter().position(|part| part == "--model");
            let Some(model) = pinned.and_then(|at| argv.get(at + 1)) else {
                seats.unread.insert(label.clone());
                continue;
            };
            seats.read.push(Seat {
                label: label.clone(),
                model: model.clone(),
                routed: argv.iter().any(|part| part == "--patch"),
                isolation: sealed_isolation(Some(candidate), Some(facts)),
            });
        }
    }
    seats
}

/// The read seats' lines ([`homed`]), then, where any seat's launch can't
/// be read, the line naming those seats' sites and, with its caveats
/// ([`with_caveats`]), each route the adapter pins that no read seat rides.
pub(super) fn modelled(adapter: &Adapter, seats: &Seats, sources: &Sources<'_>) -> Vec<Line> {
    let mut lines = homed(adapter, &seats.read, sources);
    if seats.unread.is_empty() {
        return lines;
    }
    lines.push(Line::Unmodelled(seats.unread.iter().cloned().collect()));
    let ridden: BTreeSet<&str> = seats
        .read
        .iter()
        .map(|seat| keyed(adapter, &seat.model).0)
        .collect();
    let mut answers = reaches(adapter, sources);
    answers.retain(|(reach, _)| !ridden.contains(reach.route()));
    lines.extend(with_caveats(answers, sources.repo));
    lines
}

/// The words of the line naming `seats`, the sites whose dsh launch can't
/// be read from the compile.
pub(super) fn unmodelled(seats: &[String]) -> String {
    format!(
        "Brokkr can't yet tell which home serves {}, since the compile carries no argv for \
         an inline dsh site's launch and no model for a candidate that pins no --model; each \
         route no other seat rides is asked of every source the operator's home reads, with its \
         caveat",
        named(seats)
    )
}

/// `seats`, as a line names them.
fn named(seats: &[String]) -> String {
    Safe::new(&format!("seats [{}]", seats.join(", ")))
        .as_str()
        .to_string()
}

/// Seats by the route they ride and the homes that serve their cold and
/// resumed launches: their sites, and the variable the route reads, where
/// one is named.
type Groups<'s> = BTreeMap<(&'s str, Home, Home), (BTreeSet<&'s str>, Option<&'s str>)>;

/// `seats` grouped by the route each rides and the homes that serve it, in
/// that order: its cold launch, offered no rejoin, and its resumed one, the
/// cold replacement a rejoin offer launches, each as the one served-home
/// decision rules it. Each group's key is answered against `sources` for
/// the sources the engine-only home of either launch reads, which is
/// fail-closed, and otherwise its cold launch's home, then its home line. A
/// refused launch has no key line.
pub(super) fn homed(adapter: &Adapter, seats: &[Seat], sources: &Sources<'_>) -> Vec<Line> {
    let workdir = holds_dotenv(sources.repo).then(|| sources.repo.join(".env"));
    let mut groups: Groups<'_> = BTreeMap::new();
    for seat in seats {
        let (route, variable) = keyed(adapter, &seat.model);
        let served = |offered| {
            let isolation = seat.isolation.as_ref();
            match dsh_served(isolation, offered, Some(route), seat.routed) {
                Ok(DshServed::Operator) => Home::Operator,
                Ok(DshServed::Engine) => workdir.clone().map_or(Home::Engine, Home::Workdir),
                Err(refusal) => Home::Refused(refusal.to_string()),
            }
        };
        let entry = groups.entry((route, served(false), served(true)));
        let (labels, read) = entry.or_default();
        labels.insert(&seat.label);
        *read = read.or(variable);
    }
    let mut lines = Vec::new();
    for ((route, cold, resumed), (labels, variable)) in groups {
        let route = route.to_string();
        let keyed = match resumed.engine_only() && !cold.engine_only() {
            true => &resumed,
            false => &cold,
        };
        lines.extend(answer(&route, keyed, variable, sources).map(Line::Key));
        let seats = labels.into_iter().map(str::to_string).collect();
        lines.push(Line::Home {
            route,
            seats,
            cold,
            resumed,
        });
    }
    lines
}

/// The key line of seats `home` serves on `route`, reading `variable`:
/// every source for the operator's home, the launching environment alone
/// for an engine-only one, and none for a launch that is refused.
fn answer(
    route: &str,
    home: &Home,
    variable: Option<&str>,
    sources: &Sources<'_>,
) -> Option<Reach> {
    let route = route.to_string();
    let Some(variable) = variable.map(str::to_string) else {
        return matches!(home, Home::Operator | Home::Engine)
            .then_some(Reach::Undeclared { route });
    };
    match home {
        Home::Operator => Some(reach(route, variable, sources).0),
        Home::Engine if (sources.set)(&variable) => Some(Reach::Reached {
            route,
            variable,
            source: Source::Environment,
        }),
        Home::Engine => Some(Reach::Unset { route, variable }),
        Home::Workdir(_) | Home::Refused(_) => None,
    }
}
