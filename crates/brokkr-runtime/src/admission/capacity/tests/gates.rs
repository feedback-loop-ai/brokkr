//! The production gates over another connection's journal (fix round
//! 0106b1r5, M1): one queued entry seating claude/first, judged by the real
//! pass ([`pass_within_at`]) and claimed through the real claim
//! ([`Room::claim_at`]) at the instant a test names. A probe's tests drive
//! the engine's own journal and ask both gates of the route, so neither
//! leg is a predicate fed a shared read.

use std::path::{Path, PathBuf};

use brokkr_store::Generation;
use serde_json::json;

use super::{host, route, Fixture, PLENTY, TEMP};
use crate::admission::capacity::{Capacity, Room, Unclaimed};
use crate::admission::{pass_within_at, Host, Judged, Reason};

/// The pass's and the claim's reading of claude/first at one instant.
pub(crate) type Legs = [Result<(), Capacity>; 2];

/// One entry queued in another journal, its host file, and the room a pass
/// found for it while the route stood open.
pub(crate) struct Gates {
    fixture: Fixture,
    file: PathBuf,
    room: Room,
}

impl Gates {
    /// Queue an entry seating claude/first in the journal at `journal`,
    /// through a connection of its own, and take the room the real pass
    /// finds for it at `open`, which no cool-down holds.
    pub(crate) fn over(journal: &Path, open: time::OffsetDateTime) -> Gates {
        let mut fixture = Fixture::opened(tempfile::tempdir().unwrap(), journal.to_path_buf());
        fixture.write(
            "adapters/claude.json",
            json!({"provider": "claude", "binary": "claude-cli", "driver": ["claude-cli", "run"],
                   "models": {"c-first": "first/c-1"}, "model_flag": "-m", "efforts": [],
                   "effort_flag": "unsupported", "tool_permissions": "unsupported",
                   "mcp": "unsupported", "native_capabilities": {"known": {}}}),
        );
        fixture.write(
            "agents/firsty.json",
            json!({"description": "firsty", "charter": "charters/work.md",
                   "models": ["c-first"]}),
        );
        fixture.seat("firsty", "firsty");
        fixture.queue();
        let routes = json!({"first": route(9, "cloud")});
        let file = fixture.declare(&host(json!({"claude": {"ceiling": 9, "routes": routes}})));
        let judged = judged(&fixture, &file, open);
        let room =
            (judged.room).unwrap_or_else(|| panic!("no room at {open}: {:?}", judged.verdict));
        Gates {
            fixture,
            file,
            room,
        }
    }

    /// What the real pass and the real claim read of claude/first at `at`.
    /// A claim that finds the route open claims the entry, so a test asks
    /// for an open route last.
    pub(crate) fn legs(&mut self, at: time::OffsetDateTime) -> Legs {
        let judged = judged(&self.fixture, &self.file, at);
        let reasons = judged.verdict.map(|verdict| verdict.reasons);
        let pass = match (judged.room, reasons.as_deref()) {
            (Some(_), _) => Ok(()),
            (None, Some([Reason::Capacity(unmet)])) => Err(unmet.clone()),
            (None, other) => panic!("the pass stopped the entry otherwise: {other:?}"),
        };
        let claimed = self.room.claim_at(
            &mut self.fixture.store,
            Generation(1),
            "admitted",
            &mut || true,
            at,
        );
        let claim = match claimed.unwrap() {
            Ok(_) => Ok(()),
            Err(Unclaimed::Refused(Reason::Capacity(unmet))) => Err(unmet),
            Err(other) => panic!("the claim refused otherwise: {other:?}"),
        };
        [pass, claim]
    }
}

/// The one entry as the real pass judges it at `now`.
fn judged(fixture: &Fixture, file: &Path, now: time::OffsetDateTime) -> Judged {
    let probe = |_: &Path| Ok(PLENTY);
    let host = Host {
        file,
        temp: TEMP.as_ref(),
        free: &probe,
        machine: None,
    };
    pass_within_at(&fixture.store, &host, now)
        .unwrap()
        .0
        .remove(0)
}
