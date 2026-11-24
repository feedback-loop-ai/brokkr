//! Fix round 0106b1r14, R1: a retry is admitted by one decision over all
//! its routes, in the transaction that journals its start, serialized with
//! every probe hold's write. A panel's wait reads its routes one by one, so
//! a peer that holds route X just after X was read clear, and lets route Y
//! go just before Y is read, leaves a re-read that found each route clear
//! though the two never were together. The start's own transaction reads
//! X held, and the retry waits.

use std::cell::RefCell;

use brokkr_store::{ProbeHold, ProbeHoldId, Store};

use super::held_route::{slicing, unended_holds};
use super::panel::{panel_engine, retried_two_route_panel, route_host, shared_log};
use super::*;

/// What a test runs before the wait reads a cooled provider and route.
type Act = Box<dyn FnMut(&str, &str)>;

thread_local! {
    /// What a test runs before the wait reads each cooled route.
    static BEFORE_ROUTE: RefCell<Option<Act>> = RefCell::new(None);
}

/// Run the act a test set before the wait's read of `provider`/`route`.
pub(in crate::engine::cooldown) fn before_route(provider: &str, route: &str) {
    BEFORE_ROUTE.with(|before| {
        if let Some(act) = before.borrow_mut().as_mut() {
            act(provider, route);
        }
    });
}

/// Set, or with `None` lift, this thread's act before each route's read.
fn before_route_here(act: Option<Act>) {
    BEFORE_ROUTE.with(|before| *before.borrow_mut() = act);
}

/// A peer run's probe holds on the panel's routes, written and ended on a
/// connection of their own: route Y's, then route X's, and the panel's log
/// as X's hold was ended.
struct Peer {
    store: Store,
    run: String,
    y: Option<ProbeHoldId>,
    x: Option<ProbeHoldId>,
    released: Option<Vec<String>>,
}

impl Peer {
    /// The peer's hold on `provider`/`route` at `now`.
    fn hold(&mut self, provider: &str, route: &str, now: OffsetDateTime) -> ProbeHoldId {
        // The journal keys a hold to a run it holds, so the peer's holds
        // name the panel's run under attempts no drive of it took: a hold
        // binds every reader but the attempt that holds it.
        let hold = ProbeHold {
            run_id: &self.run,
            attempt_id: route,
            site: "",
            provider,
            route,
            deadline: now + time::Duration::minutes(1),
        };
        self.store.hold_probe(&hold, now).unwrap()
    }

    /// How many of the panel run's own probe holds have ended.
    fn panel_ended(&self) -> i64 {
        let ended = "SELECT COUNT(*) FROM probe_holds WHERE run_id = ?1 AND ended_at IS NOT NULL";
        rusqlite::Connection::open(self.store.path())
            .unwrap()
            .query_row(ended, [&self.run], |row| row.get(0))
            .unwrap()
    }

    /// Before each route's read: once both of the panel's probes have
    /// answered, its retry's re-read finds X clear while the peer holds Y,
    /// then the peer holds X and lets Y go before Y is read.
    fn before(&mut self, read: (&str, &str), now: OffsetDateTime) {
        match (self.y, self.x, read) {
            (None, None, ("claude", "first")) if self.panel_ended() == 2 => {
                self.y = Some(self.hold("codex", "second", now));
            }
            (Some(y), None, ("codex", "second")) => {
                self.x = Some(self.hold("claude", "first", now));
                self.store.end_probe_hold(Some(y), now).unwrap();
            }
            _ => {}
        }
    }
}

/// The panel's two routes were never clear together at its retry's re-read,
/// though each was read clear: its start reads them in one snapshot, finds
/// X held, and journals nothing. The panel waits on X until the peer ends
/// its hold, probes both members again, and only then retries. Removal
/// control: journal the retry's start with no route to admit it over, as
/// the plain append, and the panel retries while the peer holds X.
#[test]
fn a_panels_retry_starts_only_once_both_its_routes_read_clear_in_one_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let body = retried_two_route_panel(dir.path(), Probed::Answer, Probed::Answer);
    let host = route_host(dir.path(), (2, 20), (3, 20));
    let (mut engine, now) = with_clock(panel_engine(dir.path(), body, 2, host));
    let journal = engine.store.path().to_path_buf();
    let peer = Arc::new(Mutex::new(Peer {
        store: Store::open(&journal).unwrap(),
        run: engine.run_id.clone(),
        y: None,
        x: None,
        released: None,
    }));
    let (read, before) = (Arc::clone(&now), Arc::clone(&peer));
    before_route_here(Some(Box::new(move |provider, route| {
        let at = *read.lock().unwrap();
        before.lock().unwrap().before((provider, route), at);
    })));
    let (log, sleeping, mut advance) = (dir.path().to_path_buf(), Arc::clone(&peer), slicing(&now));
    let (read, slept) = (Arc::clone(&now), Arc::clone(&now));
    engine.cooldown.clock = Clock {
        now: Box::new(move || *read.lock().unwrap()),
        sleep: Box::new(move |slice| {
            let mut peer = sleeping.lock().unwrap();
            if let (Some(x), None) = (peer.x, &peer.released) {
                peer.released = Some(shared_log(&log));
                let at = *slept.lock().unwrap();
                peer.store.end_probe_hold(Some(x), at).unwrap();
            }
            drop(peer);
            advance(slice);
        }),
    };
    engine.drive().unwrap();
    before_route_here(None);
    let peer = peer.lock().unwrap();
    assert!(peer.x.is_some(), "the peer never held X");
    let probed = ["a:attempt:refuse", "b:attempt:refuse", "a:probe", "b:probe"];
    assert_eq!(
        peer.released.as_deref(),
        Some(&probed.map(str::to_string)[..]),
        "the panel retried while the peer held X"
    );
    assert_eq!(
        shared_log(dir.path()),
        [
            "a:attempt:refuse",
            "b:attempt:refuse",
            "a:probe",
            "b:probe",
            "a:probe",
            "b:probe",
            "a:attempt:succeed",
            "b:attempt:succeed",
        ]
    );
    assert_eq!(unended_holds(&journal), 0);
}
