//! Which dsh home serves a launch (decision 0065 slice two, U1c2), decided
//! in one place for the driver that serves the launch and for doctor, which
//! models each dsh seat's home from its capability plan (#717): the
//! operator's own home where the launch carries no isolation intent, an
//! engine-only home where the driver's admission admits the launch's shape
//! on a route U0 or U0c measured one on, and past the lifted fence a
//! refusal of any other. Beside it, the one check of a
//! working directory's `.env` an engine-only home refuses a launch on. Both
//! read no file but the `.env`'s name, and stage nothing.

use std::path::Path;

use crate::adapters::StrictCause;
use crate::native_controls::{SealedFence, SealedIsolation};

/// How a qualified dsh route reached its engine-only home when U0 or U0c
/// measured it: on `dsh-base`'s shipped row alone, or with its validated
/// `llm-pi-ai` provider entry, which production carries as the seat's one
/// bound route overlay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RouteRow {
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

/// Why a dsh launch's route is not one an engine-only home was measured
/// on. Each renders as the cause after the driver's prefix.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum RouteRefusal {
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

/// The home one dsh launch is served from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DshServed {
    /// The operator's own dsh home, as a launch with no intent always was.
    Operator,
    /// An engine-only home staged for the seat, which reads a key from the
    /// launching environment alone.
    Engine,
}

/// Why a dsh launch past the lifted fence is refused before any route is
/// claimed or home staged; its text is the driver's cause.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error(transparent)]
pub struct DshRefusal(pub(crate) DshCause);

/// What refuses a dsh launch past the lifted fence: the admission of its
/// shape ([`SealedIsolation::admits`]) or its route, each in its own words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DshCause {
    #[error(transparent)]
    Admission(StrictCause),
    #[error(transparent)]
    Route(RouteRefusal),
}

/// The home a dsh launch is served from, by `isolation`, the isolation
/// intent its launch carries, none where it carries none; whether it was
/// `offered` a rejoin; the route its model pin is served on, none where the
/// seat pins none; and whether it carries a route overlay. A launch with no
/// intent is served from the operator's home. One with an intent is served
/// from an engine-only home where the driver's one admission admits its
/// shape, its cold launch or an offer's cold replacement
/// (`SealedIsolation::admits`), on a route `DSH_ROUTES` measured with that
/// overlay; any other refuses past the lifted fence, and while the fence
/// stands is served from the operator's home, as the driver serves a shape
/// it does not admit.
pub fn dsh_served(
    isolation: Option<&SealedIsolation>,
    offered: bool,
    route: Option<&str>,
    routed: bool,
) -> Result<DshServed, DshRefusal> {
    let Some(isolation) = isolation else {
        return Ok(DshServed::Operator);
    };
    let admitted = isolation
        .admits("dsh", offered)
        .map_err(DshCause::Admission);
    let served = admitted.and_then(|()| measured(route, routed).map_err(DshCause::Route));
    match (served, isolation.fence) {
        (Ok(()), _) => Ok(DshServed::Engine),
        (Err(_), SealedFence::Standing) => Ok(DshServed::Operator),
        (Err(refusal), SealedFence::Lifted) => Err(DshRefusal(refusal)),
    }
}

/// Whether `route`, with or without a route overlay, is one U0 or U0c
/// measured an engine-only home on.
fn measured(route: Option<&str>, routed: bool) -> Result<(), RouteRefusal> {
    let route = route.ok_or(RouteRefusal::Unpinned)?;
    let Some((_, row)) = DSH_ROUTES.iter().find(|(name, _)| *name == route) else {
        return Err(RouteRefusal::UnmeasuredRoute {
            route: route.to_string(),
        });
    };
    match routed == (*row == RouteRow::Overlay) {
        true => Ok(()),
        false => Err(RouteRefusal::RouteEntry {
            route: route.to_string(),
            row: *row,
        }),
    }
}

/// Whether `dir` holds a `.env` in any form: its name is looked up and the
/// file is never opened, and a name that cannot be looked up counts as
/// held. dsh reads a `<cwd>/.env` as a credential layer after the process
/// environment (dsh-credentials-local 0.1.5-rc.2, U0c), the one layer an
/// engine-only home does not close, so a launch's working directory that
/// holds one refuses to stage it.
pub fn holds_dotenv(dir: &Path) -> bool {
    !matches!(
        std::fs::symlink_metadata(dir.join(".env")),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound
    )
}
