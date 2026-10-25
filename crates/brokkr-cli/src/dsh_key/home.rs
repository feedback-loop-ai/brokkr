//! The served-home model (#717, the dsh key check's ruled follow-up): each
//! dsh seat of a folder's compile is served from the home brokkr-protocol's
//! own decision rules for it ([`dsh_served`]), by whether its launch carries
//! the engine's isolation intent, read by the predicate dispatch seals it
//! by ([`sealed_fence`]), the route its `--model` is served on and whether
//! it carries a `--patch` route overlay. The seats are grouped by route and
//! home, and each group is answered for the sources that home reads.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use brokkr_protocol::native_controls::SealedFence;
use brokkr_protocol::{dsh_served, holds_dotenv, DshServed};
use brokkr_runtime::agents::Adapter;
use brokkr_runtime::bundle::Bundle;
use brokkr_runtime::engine::sealed_fence;

use super::{keyed, reach, Line, Reach, Source, Sources};
use crate::render::Safe;

/// The provider every dsh seat is hired from.
const DSH: &str = "dsh";

/// One dsh seat of a compiled folder, as its launch is served: the site it
/// runs at, the model its `--model` pins, whether it carries a `--patch`,
/// and the fence of the isolation intent its launch carries, none where it
/// carries none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Seat {
    pub(crate) label: String,
    pub(crate) model: String,
    pub(crate) routed: bool,
    pub(crate) fence: Option<SealedFence>,
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

impl Home {
    /// The home line's words for `seats`.
    pub(super) fn words(&self, seats: &[String]) -> String {
        let seats = Safe::new(&format!("seats [{}]", seats.join(", ")))
            .as_str()
            .to_string();
        let engine = "its own engine-only home (decision 0065)";
        match self {
            Home::Operator => format!(
                "Brokkr serves {seats} on this route from the operator's own dsh home, which reads \
                 every source this check asks"
            ),
            Home::Engine => format!(
                "Brokkr serves {seats} on this route from {engine}, where dsh reads a key from the \
                 launching environment alone, never the operator's .env or patch layers"
            ),
            Home::Workdir(file) => format!(
                "Brokkr serves {seats} on this route from {engine}, and refuses each launch while \
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
}

/// Each dsh seat of `compiled`, one per site and candidate its chain hires
/// from dsh, read from the candidate's argv as the launch reads it. None
/// where a dsh seat's launch can't be read so: an inline dsh site, whose
/// argv the compile does not carry, or a candidate that pins no `--model`.
pub(crate) fn seats(compiled: &Bundle) -> Option<Vec<Seat>> {
    let mut seats = Vec::new();
    for (label, facts) in &compiled.sites {
        let serves = facts
            .capabilities
            .as_ref()
            .map_or(&[][..], |site| &site.outcomes);
        if facts.chain.is_empty() && serves.iter().any(|outcome| outcome.provider == DSH) {
            return None;
        }
        for candidate in facts.chain.iter().filter(|link| link.provider == DSH) {
            let argv = &candidate.argv;
            let pinned = argv.iter().position(|part| part == "--model");
            let model = pinned.and_then(|at| argv.get(at + 1))?;
            seats.push(Seat {
                label: label.clone(),
                model: model.clone(),
                routed: argv.iter().any(|part| part == "--patch"),
                fence: sealed_fence(Some(candidate), Some(facts)),
            });
        }
    }
    Some(seats)
}

/// Seats by the route they ride and the home that serves them: their
/// sites, and the variable the route reads, where one is named.
type Groups<'s> = BTreeMap<(&'s str, Home), (BTreeSet<&'s str>, Option<&'s str>)>;

/// `seats` grouped by the route each rides and the home that serves it,
/// in that order, each group's key answered for the sources its home reads
/// against `sources`, then its home line. A refused launch has no key line.
pub(super) fn homed(adapter: &Adapter, seats: &[Seat], sources: &Sources<'_>) -> Vec<Line> {
    let workdir = holds_dotenv(sources.repo).then(|| sources.repo.join(".env"));
    let mut groups: Groups<'_> = BTreeMap::new();
    for seat in seats {
        let (route, variable) = keyed(adapter, &seat.model);
        let home = match dsh_served(seat.fence, Some(route), seat.routed) {
            Ok(DshServed::Operator) => Home::Operator,
            Ok(DshServed::Engine) => workdir.clone().map_or(Home::Engine, Home::Workdir),
            Err(refusal) => Home::Refused(refusal.to_string()),
        };
        let (labels, read) = groups.entry((route, home)).or_default();
        labels.insert(&seat.label);
        *read = read.or(variable);
    }
    let mut lines = Vec::new();
    for ((route, home), (labels, variable)) in groups {
        let route = route.to_string();
        lines.extend(answer(&route, &home, variable, sources).map(Line::Key));
        let seats = labels.into_iter().map(str::to_string).collect();
        lines.push(Line::Home { route, seats, home });
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
