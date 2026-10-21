//! U8e3 (MB5, CR2; operator ruling 2026-10-21, "A"): `brokkr operator
//! release`, settlement's one exception to positive proof. A run whose
//! engine was killed while its driver ran keeps its reservation, refusing
//! a conflicting run, once its attempt ended indeterminate; the operator's
//! release, journaled with its reason, releases it and the conflicting run
//! admits, nothing folded or replayed and the attempt left as it ended. A
//! running attempt, a recoverable one and a run with no reservation are
//! each refused by name, nothing journaled and nothing released.

use std::path::PathBuf;

use brokkr_protocol::Cleanup;

use super::*;
use crate::engine::broker::admission::tests::{
    conflicting, in_flight, unstaked_lease, Coordinated,
};
use crate::engine::broker::admission::{Conflict, Unadmitted, Unknown};
use crate::engine::broker::settlement::{ended, spawning, Refused, Unreleased};
use crate::engine::operator::{conclude, released_racing};
use crate::engine::tests::{engine, single_body};

/// The attempt the engine was killed in.
const ATTEMPT: &str = "a1";

/// What the operator attests.
const REASON: &str = "every process the attempt owned has ended";

/// A run whose engine was killed while its driver at `work` ran, in the
/// attempt `a1` its lease admitted: its directories, the engine, and the
/// run's directory in the coordination root.
struct Killed {
    dirs: [tempfile::TempDir; 2],
    engine: Engine,
    at: PathBuf,
}

impl Killed {
    /// The run driven to its attempt in flight, the driver's record written
    /// Settling by the door before it spawned, and sealed Settled by the
    /// barrier where `settled`; then the engine gone, its lease neither
    /// settled nor released.
    fn new(settled: bool) -> Killed {
        let (dir, mut engine) = engine(single_body(Vec::new()));
        let (home, at) = unstaked_lease(&mut engine, ATTEMPT);
        in_flight(&mut engine, ATTEMPT);
        let held = engine.held.as_ref().unwrap();
        assert_eq!(spawning(held, "work", None), Ok(()));
        if settled {
            ended(&mut engine.held, "work", &Cleanup::Settled);
        }
        engine.held = None;
        Killed {
            dirs: [dir, home],
            engine,
            at,
        }
    }

    /// The HOME its lease was admitted under.
    fn home(&self) -> PathBuf {
        self.dirs[1].path().join("home")
    }

    fn run(&self) -> String {
        self.engine.run_id.clone()
    }

    fn events(&self) -> Vec<EventEnvelope> {
        self.engine.store.load(&self.engine.run_id).unwrap()
    }

    /// The run's reservation as it stands: its standing attempts, and its
    /// settled ones.
    fn reservation(&self) -> (Vec<String>, Value) {
        let bytes = std::fs::read(self.at.join("reservation.json")).unwrap();
        let held: Value = serde_json::from_slice(&bytes).unwrap();
        let standing = held["attempts"].as_object().unwrap().keys().cloned();
        (standing.collect(), held["settled"].clone())
    }

    /// Whether the run's attempt still stands in its reservation.
    fn standing(&self) -> bool {
        self.reservation().0 == [ATTEMPT]
    }

    /// The run restarted in a fresh process, which ends the attempt
    /// indeterminate, its driver never proven settled, and drives on.
    fn restarted(&mut self) {
        let _stood = Coordinated::at(&self.home());
        assert!(self.engine.drive_once().unwrap().is_none());
        let last = self.events().pop().unwrap();
        assert_eq!(last.event_type, EventType::EffectIndeterminate);
    }

    /// The run concluded from its journal, which closes the attempt
    /// indeterminate.
    fn concluded(&mut self) {
        let run = self.run();
        conclude(&mut self.engine.store, &run, "operator", "its engine died").unwrap();
    }

    /// The operator's release, `between` run before each fenced append.
    fn released_with(
        &mut self,
        between: impl FnMut(&mut Store),
    ) -> Result<FencedCommandOutcome, EngineError> {
        let _stood = Coordinated::at(&self.home());
        let run = self.run();
        let store = &mut self.engine.store;
        released_racing(store, &run, ["operator", REASON], between)
    }

    /// The operator's release, as `brokkr operator` asks it.
    fn released(&mut self) -> Result<FencedCommandOutcome, EngineError> {
        let _stood = Coordinated::at(&self.home());
        let run = self.run();
        let release = OperatorCommand::Release;
        operator_command(&mut self.engine.store, &run, release, "operator", REASON)
    }

    /// A conflicting run's admission on the worktree.
    fn conflicting(&self) -> Result<(), Unadmitted> {
        conflicting(&self.home(), &self.engine).map(drop)
    }

    /// The refusal of a conflicting run while this one holds the worktree.
    fn held(&self) -> Result<(), Unadmitted> {
        Err(Unadmitted::Evidence(Conflict::Held(self.run())))
    }
}

/// The settlement refusal `released` failed with.
fn unreleased(released: Result<FencedCommandOutcome, EngineError>) -> Unreleased {
    match released {
        Err(EngineError::Unreleased(unreleased)) => unreleased,
        other => panic!("not a settlement refusal: {other:?}"),
    }
}

/// MB5, CR2: an engine killed while its driver ran leaves its reservation
/// standing once its attempt ended indeterminate, by a restart (the run
/// drives on) or a conclusion (it is stopped), refusing a conflicting run.
/// The operator's release journals its command with its reason, and on the
/// live run the acceptance that disposes of it; only then is the
/// reservation released, and the conflicting run admits. Nothing else is
/// journaled, so nothing is folded or replayed, and the run and its
/// attempt stand as they ended.
#[test]
fn the_operators_release_lets_a_conflicting_run_admit_and_moves_nothing() {
    for concluded in [false, true] {
        let mut killed = Killed::new(false);
        assert_eq!(killed.conflicting(), killed.held(), "{concluded}");
        match concluded {
            true => killed.concluded(),
            false => killed.restarted(),
        }
        assert_eq!(killed.conflicting(), killed.held(), "{concluded}");
        let before = killed.events();
        let released = killed.released().unwrap();
        let events = killed.events();
        let head = events.last().unwrap();
        let at_head = FencedCommandOutcome::Accepted {
            head_seq: head.seq,
            head_hash: head.event_hash.clone(),
        };
        assert_eq!(released, at_head, "{concluded}");
        let tail = &events[before.len()..];
        let commanded = &tail[0];
        assert_eq!(commanded.event_type, EventType::OperatorCommanded);
        let command_id = commanded.payload["command_id"].clone();
        let asked = json!({"command_id": command_id, "command": "release",
                           "args": {"reason": REASON}, "operator": "operator"});
        assert_eq!(commanded.payload, asked, "{concluded}");
        let accepted = json!({"command_id": command_id, "operator": "operator", "reason": REASON});
        let accepted = (EventType::OperatorAccepted, accepted);
        let disposed: Vec<_> = (tail[1..].iter())
            .map(|event| (event.event_type, event.payload.clone()))
            .collect();
        let expected = if concluded { vec![] } else { vec![accepted] };
        assert_eq!(disposed, expected, "{concluded}");
        let (was, now) = (fold(&before).unwrap(), fold(&events).unwrap());
        assert_eq!((now.run, now.phase), (was.run, was.phase), "{concluded}");
        let released = (Vec::<String>::new(), json!([ATTEMPT]));
        assert_eq!(killed.reservation(), released, "{concluded}");
        assert_eq!(killed.conflicting(), Ok(()), "{concluded}");
    }
}

/// U8e3's refusals, each by its own cause, nothing journaled and nothing
/// released: an attempt not yet ended, its driver possibly running; one
/// whose every driver is sealed Settled, which a restart recovers; a run
/// whose reservation holds no attempt standing, once released or never
/// reserved; and a host with no protected coordination root.
#[test]
fn a_release_is_refused_by_name_and_releases_nothing() {
    let mut running = Killed::new(false);
    let (refused, length) = (running.released(), running.events().len());
    let not_ended = Unreleased::refused(Refused::Running(ATTEMPT.into()));
    assert_eq!(unreleased(refused), not_ended);
    assert_eq!(
        not_ended.to_string(),
        "the operator's release was refused, and the run's writer lease stands: attempt 'a1' \
         has not ended; release only an attempt that ended, once every process it owned has"
    );
    assert_eq!((running.events().len(), running.standing()), (length, true));
    assert_eq!(running.conflicting(), running.held());

    let mut recoverable = Killed::new(true);
    let refused = unreleased(recoverable.released());
    let recovers = Refused::Recoverable(ATTEMPT.into());
    assert_eq!(refused, Unreleased::refused(recovers));
    assert_eq!(recoverable.events().len(), length);
    assert!(recoverable.standing());

    let mut released = Killed::new(false);
    released.restarted();
    released.released().unwrap();
    let length = released.events().len();
    let unreserved = Refused::Unreserved(released.run());
    assert_eq!(
        unreleased(released.released()),
        Unreleased::refused(unreserved.clone())
    );
    assert_eq!(released.events().len(), length);
    let (_dir, mut never) = engine(single_body(Vec::new()));
    let (run, home) = (never.run_id.clone(), released.home());
    let _stood = Coordinated::at(&home);
    let release = OperatorCommand::Release;
    let refused = operator_command(&mut never.store, &run, release, "operator", REASON);
    let unreserved = Unreleased::refused(Refused::Unreserved(run.clone()));
    assert_eq!(unreleased(refused), unreserved);

    let _stood = Coordinated::unrooted(&home);
    let refused = operator_command(&mut never.store, &run, release, "operator", REASON);
    let unrooted = Unreleased::standing(Unadmitted::Identity(Unknown::Root));
    assert_eq!(unreleased(refused), unrooted);
}

/// Decision 0029, U8e3: the release is journaled on the head it was judged
/// on, and released only once journaled. A peer that appends before the
/// command takes its head, and nothing of the release is written; one
/// that appends between the command and its acceptance leaves the command
/// disposed of as a run that moved. Either way the release is refused and
/// the reservation stands, refusing a conflicting run.
#[test]
fn a_peer_that_moves_the_journal_refuses_the_release() {
    for after_the_command in [false, true] {
        let mut killed = Killed::new(false);
        killed.restarted();
        let (length, run) = (killed.events().len(), killed.run());
        let mut turns = 0;
        let peer = move |store: &mut Store| {
            turns += 1;
            if turns == 1 + usize::from(after_the_command) {
                let command = json!({"command_id": "peer", "command": "stop", "args": {},
                                     "operator": "peer"});
                journal(store, &run, EventType::OperatorCommanded, command);
            }
        };
        let refused = unreleased(killed.released_with(peer));
        assert_eq!(refused, Unreleased::refused(Refused::Moved));
        let tail: Vec<_> = killed.events()[length..]
            .iter()
            .map(|event| (event.event_type, event.payload["operator"].clone()))
            .collect();
        let (commanded, rejected) = (EventType::OperatorCommanded, EventType::OperatorRejected);
        let expected = match after_the_command {
            false => vec![(commanded, json!("peer"))],
            true => vec![
                (commanded, json!("operator")),
                (commanded, json!("peer")),
                (rejected, json!("operator")),
            ],
        };
        assert_eq!(tail, expected, "{after_the_command}");
        let last = killed.events().pop().unwrap();
        if after_the_command {
            assert_eq!(last.payload["reason"], Refusal::LostFence.word());
        }
        assert!(killed.standing(), "{after_the_command}");
        assert_eq!(killed.conflicting(), killed.held());
    }
}

/// Nothing but the operator's console may release: the Looper bridge
/// refuses `release` as a command it does not take, journaling the word
/// it was sent.
#[test]
fn the_bridge_refuses_a_release() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = super::super::tests::parked_store(&dir.path().join("bridge.db"), "bridge");
    let (seq, hash) = store.head_hash("bridge").unwrap();
    let wire = super::super::tests::looper("looper-release", "release", seq, &hash);
    let refused = apply_fenced_operator_command(&mut store, "bridge", &wire).unwrap();
    let rejected = store.load("bridge").unwrap().pop().unwrap();
    let refused_by_name = FencedCommandOutcome::Rejected {
        reason: Refusal::CommandNotAllowed.word().into(),
        head_seq: rejected.seq,
        head_hash: rejected.event_hash,
    };
    assert_eq!(refused, refused_by_name);
}
