//! U8d (CC2, CR3, CR4): a single site's settled broker calls folded into
//! its journal through the barrier its driver's report passes before its
//! result is admitted ([`Engine::fold_with`]), on a leased engine whose
//! attempt's evidence stands in a canonical temporary root. Each ledger is
//! written by the protocol's own writer ([`Ledger`]), in memory, then laid
//! beside its plan; a fault is made by editing those bytes. While the MCP
//! compile fence stands no public engine can provision a plan, so these
//! proofs live here (operator ruling of 2026-10-20). The driver barrier's
//! own call is proved end to end by
//! `a_dispatched_driver_is_settled_by_its_broker_calls_before_its_result_is_admitted`.

use std::cell::{Cell, RefCell};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;
use std::rc::Rc;

use brokkr_core::canonical;
use brokkr_core::envelope::EventType;
use brokkr_protocol::broker::ledger::{Denial, Ledger};
use brokkr_protocol::broker::{Owner, Refusal};
use brokkr_protocol::native_controls::{SealedIsolation, SealedServing, SERVING_INPUTS};
use brokkr_protocol::process::Unsettled;
use brokkr_protocol::AttemptOutcome;
use serde_json::{json, Value};

use super::*;
use crate::engine::artifacts::tests::{sha, Provisioned};
use crate::engine::broker::admission::tests::evidence_leased;
use crate::engine::capability_calls::SettledEvent;
use crate::engine::tests::{checkpointing_command, engine, report, single_body};
use crate::engine::{DriverRun, Selection};

/// The attempt every fixture leases and folds.
const ATTEMPT: &str = "a1";

/// The effect it belongs to, as its plans name it.
const EFFECT: &str = "effect-1";

/// A leased engine whose attempt `a1` provisioned one plan, by the readers
/// and writers of [`Provisioned`]: its directories, the engine, the
/// attempt's evidence, the plan's digest, the spawn its site was sealed
/// with, and where the attempt began in the journal.
struct Site {
    _dirs: [tempfile::TempDir; 2],
    engine: Engine,
    attempt: Provisioned,
    digest: String,
    spawn: SiteSpawn,
    begun: Begun,
}

impl Site {
    /// One plan, retaining its responses where `retained`, and a spawn
    /// sealed with its broker, running `argv`.
    fn new(retained: bool, argv: Vec<String>) -> Site {
        let (dir, mut engine) = engine(single_body(Vec::new()));
        let (home, at) = evidence_leased(&mut engine, ATTEMPT);
        let repo = at.parent().unwrap().file_name().unwrap().to_str().unwrap();
        let attempt = Provisioned::new(&at, [repo, &engine.run_id, ATTEMPT]);
        let digest = attempt.plans(&[retained]).remove(0);
        let broker = SealedBroker {
            capability: "docs".into(),
            plan: attempt.dir().join("plan-0.json").to_str().unwrap().into(),
            plan_digest: digest.clone(),
            removals: Vec::new(),
        };
        let isolation = SealedIsolation {
            servers: SealedServers::Brokered(vec![broker]),
            ..SealedIsolation::default()
        };
        let serving = SealedServing {
            isolation,
            ..SealedServing::default()
        };
        let spawn = SiteSpawn {
            serving: Some(serving),
            ..SiteSpawn::inherit(argv)
        };
        let begun = engine.begun(&spawn).unwrap().unwrap();
        Site {
            _dirs: [dir, home],
            engine,
            attempt,
            digest,
            spawn,
            begun,
        }
    }

    /// A site whose spawn is never run.
    fn folding() -> Site {
        Site::new(false, vec!["must-not-run".into()])
    }

    /// The attempt begun again at the journal's head now.
    fn begin(&mut self) {
        self.begun = self.engine.begun(&self.spawn).unwrap().unwrap();
    }

    /// A second plan beside the first, serving `wiki` through `cap-wiki`,
    /// pinned last by the inventory, and its broker sealed after the
    /// first: the plan's digest.
    fn second(&mut self) -> String {
        let dir = self.attempt.dir();
        let mut plan: Value =
            serde_json::from_slice(&fs::read(dir.join("plan-0.json")).unwrap()).unwrap();
        (plan["server"], plan["capability"]) = (json!("cap-wiki"), json!("wiki"));
        let bytes = serde_json::to_vec(&plan).unwrap();
        let locator = dir.join("plan-1.json");
        made(&locator, &bytes);
        let digest = sha(&bytes);
        let at = dir.join("inventory.json");
        let mut inventory: Inventory = serde_json::from_slice(&fs::read(&at).unwrap()).unwrap();
        let pin = Pin {
            locator: locator.clone(),
            digest: digest.clone(),
        };
        inventory.plans.push(pin);
        fs::write(&at, serde_json::to_vec(&inventory).unwrap()).unwrap();
        let broker = SealedBroker {
            capability: "wiki".into(),
            plan: locator.to_str().unwrap().into(),
            plan_digest: digest.clone(),
            removals: Vec::new(),
        };
        let serving = self.spawn.serving.as_mut().unwrap();
        let SealedServers::Brokered(brokers) = &mut serving.isolation.servers else {
            unreachable!("the site is brokered")
        };
        brokers.push(broker);
        digest
    }

    /// The plan's ledger.
    fn ledger(&self) -> PathBuf {
        self.attempt.dir().join("plan-0.json.ledger")
    }

    /// `bytes` laid down as the plan's ledger, as its broker leaves it.
    fn laid(&self, bytes: &[u8]) {
        made(&self.ledger(), bytes);
    }

    /// The ledger `serve` writes through the protocol's own writer for
    /// `owner`, in memory, sealed at the plan's digest.
    fn written(&self, owner: &Owner, serve: impl FnOnce(Ledger<'static>)) -> Vec<u8> {
        written((owner, &self.digest), serve)
    }

    /// [`Site::written`] for the plan's own owner, laid down.
    fn serve(&self, serve: impl FnOnce(Ledger<'static>)) {
        self.laid(&self.written(&self.attempt.owner(), serve));
    }

    /// The driver's report of `outcome` settled by the barrier.
    fn fold(&mut self, outcome: AttemptOutcome) -> AttemptReport {
        self.fold_between(outcome, |_| {})
    }

    /// [`Site::fold`], `between` run before each append is tried.
    fn fold_between(
        &mut self,
        outcome: AttemptOutcome,
        between: impl FnMut(&mut Store),
    ) -> AttemptReport {
        let reported = report(outcome, "");
        let (engine, spawn, begun) = (&mut self.engine, &self.spawn, &self.begun);
        let folded = engine.fold_with([EFFECT, ATTEMPT], spawn, begun, reported, between);
        folded.unwrap()
    }

    /// Every checkpoint the journal holds for the attempt, in order.
    fn rows(&self) -> Vec<Value> {
        let events = self.engine.store.load(&self.engine.run_id).unwrap();
        let rows = events.into_iter().filter(|event| {
            let checkpointed = event.event_type == EventType::EffectCheckpointed;
            checkpointed & (event.attempt_id.as_deref() == Some(ATTEMPT))
        });
        rows.map(|event| event.payload["checkpoint"].clone())
            .collect()
    }
}

/// The ledger `serve` writes through the protocol's own writer for `owner`,
/// in memory, sealed at `digest`.
fn written((owner, digest): (&Owner, &str), serve: impl FnOnce(Ledger<'static>)) -> Vec<u8> {
    let bytes = Rc::new(RefCell::new(Vec::new()));
    let sink = Rc::clone(&bytes);
    let append = move |line: &[u8]| {
        sink.borrow_mut().extend_from_slice(line);
        Ok(())
    };
    serve(Ledger::open_with(append, (owner, digest), &[]).unwrap());
    bytes.take()
}

/// `bytes` written as the new owner-only file `path`.
fn made(path: &std::path::Path, bytes: &[u8]) {
    let mut options = OpenOptions::new();
    let mut file = options
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .unwrap();
    file.write_all(bytes).unwrap();
}

/// The engine's id of call `call` of the plan serving `capability`,
/// derived here from CC2's tuple: the plan's sealed attempt, site and
/// instance, its server and the call's sequence.
fn call_id(capability: &str, call: u64) -> String {
    let server = format!("cap-{capability}");
    let tuple = json!({
        "attempt": ATTEMPT, "site": "work", "instance": "work#0", "server": server, "call": call,
    });
    format!("m-{}", canonical::sha256_hex(&tuple))
}

/// The settled checkpoint of call `call` to `tool` of the plan serving
/// `capability`, in `state`.
fn served(capability: &str, call: u64, tool: &str, state: &str) -> Value {
    json!({
        "step": "capability-call-settled", "tool": tool, "capability": capability,
        "dialect": "docs-mcp", "call_id": call_id(capability, call), "call_state": state,
    })
}

/// The settled checkpoint of call `call` of the first plan to `tool`, in
/// `state`.
fn row(call: u64, tool: &str, state: &str) -> Value {
    served("docs", call, tool, state)
}

/// The driver's success.
fn succeeded() -> AttemptOutcome {
    AttemptOutcome::Succeeded {
        result: json!({"result": "complete"}),
    }
}

/// A failure of `error`.
fn failed(error: &str) -> AttemptOutcome {
    let error = error.to_string();
    AttemptOutcome::Failed { error }
}

/// One accepted call to `tool`, ended `outcome`.
fn call(ledger: &mut Ledger<'_>, tool: &str, outcome: Outcome) {
    let begun = ledger.room().unwrap().begin(tool).unwrap();
    ledger.end(begun, outcome).unwrap();
}

/// `count` calls to `lookup` that succeeded.
fn answered(ledger: &mut Ledger<'_>, count: u64) {
    (0..count).for_each(|_| call(ledger, "lookup", Outcome::Succeeded));
}

/// CC2, CR3: a clean session's every accepted call settles as one
/// checkpoint in call order, in the state its Terminal records — a child's
/// error failed, and the three local denials, an ungranted tool, a call
/// begun while another was active and the spent budget, each one refused —
/// and the harness's success stands, ordinary tool errors and denials
/// latching nothing.
#[test]
fn a_clean_session_settles_one_checkpoint_per_accepted_call_and_its_result_stands() {
    let mut site = Site::folding();
    site.serve(|mut ledger| {
        call(&mut ledger, "lookup", Outcome::Succeeded);
        call(&mut ledger, "lookup", Outcome::Failed);
        call(&mut ledger, "admin", Outcome::Refused(Denial::Ungranted));
        let active = ledger.room().unwrap().begin("lookup").unwrap();
        let second = ledger.room().unwrap().begin("lookup").unwrap();
        ledger
            .end(second, Outcome::Refused(Denial::Active))
            .unwrap();
        ledger.end(active, Outcome::Succeeded).unwrap();
        call(&mut ledger, "lookup", Outcome::Refused(Denial::Budget));
        assert_eq!(ledger.close(), Ok(Disposition::Clean));
    });
    let settled = site.fold(succeeded());
    assert_eq!(settled.settled_outcome(), succeeded());
    let rows = [
        row(1, "lookup", "succeeded"),
        row(2, "lookup", "failed"),
        row(3, "admin", "refused"),
        row(4, "lookup", "succeeded"),
        row(5, "lookup", "refused"),
        row(6, "lookup", "refused"),
    ];
    assert_eq!(site.rows(), rows);
}

/// CR4: a healthy session that accepted no call settles nothing and lets
/// the harness's success stand.
#[test]
fn a_healthy_zero_call_session_settles_nothing_and_succeeds() {
    let mut site = Site::folding();
    site.serve(|ledger| assert_eq!(ledger.close(), Ok(Disposition::Clean)));
    let settled = site.fold(succeeded());
    assert_eq!(
        (settled.settled_outcome(), site.rows()),
        (succeeded(), vec![])
    );
}

/// Every fatal cause a session latches, MB3 and MB4's box, source, store,
/// readiness, handoff and exec causes among them.
fn fatal() -> Vec<Failure> {
    let refused = [
        Refusal::Unbound,
        Refusal::Name("binding 'TOKEN' is not declared".into()),
        Refusal::StartupInputs,
        Refusal::ProgramTree,
        Refusal::LaunchInReach,
        Refusal::BindOverlapsReach,
        Refusal::Linked,
        Refusal::Identity,
        Refusal::Unavailable,
        Refusal::StoreReachable,
        Refusal::StoreInBox,
        Refusal::Establishment,
        Refusal::Handoff,
        Refusal::Exec,
        Refusal::Response,
        Refusal::Timeout,
        Refusal::Masking,
    ];
    let served = [
        Failure::Version,
        Failure::Protocol,
        Failure::Response,
        Failure::Timeout,
        Failure::Masking,
        Failure::Limit,
        Failure::Undelivered,
    ];
    refused
        .into_iter()
        .map(Failure::Refused)
        .chain(served)
        .collect()
}

/// CR3, CR4: a verified session that closed failed, with no call accepted,
/// overrides the harness's success with its exact cause, whatever cause it
/// latched, and invents no checkpoint for it; a harness that failed on its
/// own keeps its error beside the cause.
#[test]
fn a_failed_zero_call_session_fails_a_successful_harness_with_its_exact_cause() {
    for failure in fatal() {
        let mut site = Site::folding();
        let latched = failure.clone();
        site.serve(|mut ledger| {
            ledger.fail(latched.clone());
            assert_eq!(ledger.close(), Ok(Disposition::Failed(latched)));
        });
        let cause = failure.to_string();
        let settled = site.fold(succeeded());
        assert_eq!(settled.settled_outcome(), failed(&cause), "{cause}");
        assert_eq!(site.rows(), Vec::<Value>::new(), "{cause}");
    }
    let mut site = Site::folding();
    site.serve(|mut ledger| {
        ledger.fail(Failure::Refused(Refusal::Exec));
        ledger.close().unwrap();
    });
    let settled = site.fold(failed("the seat gave up"));
    let both =
        "MCP server could not start inside its box; the driver then failed: the seat gave up";
    assert_eq!(settled.settled_outcome(), failed(both));
}

/// CR4's harness that ignores its broker: a pinned-version mismatch before
/// any call, a fatal protocol failure after one completed call, and the
/// ledger's limit at exactly 4,096 accepted calls, each with a complete
/// call set, keep their exact causes against the harness's success, with
/// exactly 0, 1 and 4,096 checkpoints, and no 4,097th.
#[test]
fn version_protocol_and_capacity_endings_keep_their_causes_despite_complete_calls() {
    let cases: [(u64, Failure, &str); 3] = [
        (
            0,
            Failure::Version,
            "MCP server version does not match the dialect's pinned version",
        ),
        (1, Failure::Protocol, "MCP server protocol is invalid"),
        (
            4096,
            Failure::Limit,
            "broker ledger exceeds the attempt limit",
        ),
    ];
    for (calls, failure, cause) in cases {
        let mut site = Site::folding();
        site.serve(|mut ledger| {
            answered(&mut ledger, calls);
            match failure {
                // The 4,097th request waits unread: the limit latches.
                Failure::Limit => assert_eq!(ledger.room().err(), Some(Failure::Limit)),
                _ => ledger.fail(failure),
            }
            ledger.close().unwrap();
        });
        let settled = site.fold(succeeded());
        assert_eq!(settled.settled_outcome(), failed(cause));
        let rows = site.rows();
        let expected: Vec<Value> = (1..=calls).map(|n| row(n, "lookup", "succeeded")).collect();
        assert_eq!(rows.len() as u64, calls, "{cause}");
        assert_eq!(rows, expected, "{cause}");
    }
}

/// The ledger `lines` make, each with its newline.
fn lines(lines: &[&[u8]]) -> Vec<u8> {
    lines
        .iter()
        .flat_map(|line| [*line, b"\n"].concat())
        .collect()
}

/// The ledger's lines but the one at `index`, each with its newline.
fn without(bytes: &[u8], index: usize) -> Vec<u8> {
    let kept = bytes.split_inclusive(|byte| *byte == b'\n').enumerate();
    let kept = kept.filter(|(at, _)| *at != index).map(|(_, line)| line);
    kept.flatten().copied().collect()
}

/// CR3, CR4: a ledger that does not verify keeps the calls of its verified
/// prefix, a begun call without its Terminal interrupted and citing no
/// digest, fails the harness's success with the protocol's exact cause,
/// and settles no call past where verification stopped: a partial final
/// record, a malformed middle record, a gap, another owner, a Terminal
/// without its Started, a second Terminal for one call, and no closure.
#[test]
fn a_ledger_that_does_not_verify_keeps_its_verified_calls_with_its_exact_cause() {
    let first = || vec![row(1, "lookup", "succeeded")];
    let interrupted = || {
        vec![
            row(1, "lookup", "succeeded"),
            row(2, "lookup", "interrupted"),
        ]
    };
    let cases: [(Fault, Vec<Value>, &str); 7] = [
        (
            Fault::Partial,
            interrupted(),
            "broker ledger ends in a partial record",
        ),
        (
            Fault::Malformed,
            first(),
            "broker ledger record is malformed",
        ),
        (
            Fault::Gap,
            first(),
            "broker ledger sequence is not contiguous",
        ),
        (
            Fault::Owner,
            vec![],
            "broker ledger owner does not match this attempt",
        ),
        (Fault::Order, vec![], "broker ledger lifecycle is invalid"),
        (
            Fault::Conflict,
            first(),
            "broker ledger contains a conflicting duplicate",
        ),
        (
            Fault::Unclosed,
            first(),
            "broker ledger lifecycle is invalid",
        ),
    ];
    for (fault, rows, cause) in cases {
        let mut site = Site::folding();
        site.laid(&fault.ledger(&site));
        let settled = site.fold(succeeded());
        assert_eq!(settled.settled_outcome(), failed(cause), "{fault:?}");
        assert_eq!(site.rows(), rows, "{fault:?}");
    }
}

/// How a settled ledger fails to verify.
#[derive(Debug, Clone, Copy)]
enum Fault {
    /// A call begun, its Terminal cut short by the broker's death.
    Partial,
    /// A record that is not one, after the first call.
    Malformed,
    /// The second call's Started missing.
    Gap,
    /// Opened for another attempt.
    Owner,
    /// A Terminal with no Started.
    Order,
    /// A second Terminal for the first call.
    Conflict,
    /// The first call ended, and no closure.
    Unclosed,
}

impl Fault {
    /// The plan's ledger at `site`, as this fault leaves it.
    fn ledger(self, site: &Site) -> Vec<u8> {
        let owner = site.attempt.owner();
        let one = || site.written(&owner, |mut ledger| answered(&mut ledger, 1));
        let two = || {
            site.written(&owner, |mut ledger| {
                answered(&mut ledger, 2);
                ledger.close().unwrap();
            })
        };
        match self {
            Fault::Partial => {
                let begun = site.written(&owner, |mut ledger| {
                    answered(&mut ledger, 1);
                    // Begun, and never ended.
                    let _unresolved = ledger.room().unwrap().begin("lookup").unwrap();
                });
                [&begun[..], br#"{"record":"terminal","seq":5"#].concat()
            }
            Fault::Malformed => [one(), b"{\"record\":\n".to_vec(), without(&two(), 0)].concat(),
            Fault::Gap => without(&two(), 3),
            Fault::Owner => {
                let attempt = "a2".to_string();
                let other = Owner { attempt, ..owner };
                site.written(&other, |ledger| drop(ledger.close()))
            }
            Fault::Order => {
                let terminal = r#"{"record":"terminal","seq":2,"call":1,"tool":"lookup","outcome":"succeeded"}"#;
                [site.written(&owner, drop), lines(&[terminal.as_bytes()])].concat()
            }
            Fault::Conflict => {
                let again =
                    r#"{"record":"terminal","seq":4,"call":1,"tool":"lookup","outcome":"failed"}"#;
                [one(), lines(&[again.as_bytes()])].concat()
            }
            Fault::Unclosed => one(),
        }
    }
}

/// CR3, CR4: a plan whose ledger is missing fails the harness's success
/// and invents no call; a ledger no plan of the inventory pins fails it
/// too, while the verified calls of the plan's own ledger still settle.
#[test]
fn a_missing_or_unexpected_ledger_fails_the_attempt_without_inventing_a_call() {
    let mut missing = Site::folding();
    let settled = missing.fold(succeeded());
    let cause = "broker ledger is missing for this attempt";
    assert_eq!(
        (settled.settled_outcome(), missing.rows()),
        (failed(cause), vec![])
    );

    let mut unexpected = Site::folding();
    unexpected.serve(|mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    made(&unexpected.attempt.dir().join("plan-7.json.ledger"), b"");
    let settled = unexpected.fold(succeeded());
    let cause = "broker ledger is not expected for this attempt";
    assert_eq!(settled.settled_outcome(), failed(cause));
    assert_eq!(unexpected.rows(), [row(1, "lookup", "succeeded")]);
}

/// No broker forwards a call to a tool its plan does not grant: a ledger
/// claiming one answered fails the attempt, and settles the calls before
/// it and none from it on, so it never reads as a grant (CC2).
#[test]
fn a_ledger_claiming_an_ungranted_tool_answered_settles_nothing_from_it() {
    let mut site = Site::folding();
    site.serve(|mut ledger| {
        call(&mut ledger, "lookup", Outcome::Succeeded);
        call(&mut ledger, "admin", Outcome::Succeeded);
        call(&mut ledger, "lookup", Outcome::Succeeded);
        ledger.close().unwrap();
    });
    let settled = site.fold(succeeded());
    let cause = "broker ledger claims an ungranted tool";
    assert_eq!(settled.settled_outcome(), failed(cause));
    assert_eq!(site.rows(), [row(1, "lookup", "succeeded")]);
}

/// A peer that holds the journal's write lock, until dropped.
fn write_lock_on(db: &std::path::Path) -> rusqlite::Connection {
    let holder = rusqlite::Connection::open(db).unwrap();
    holder.execute_batch("BEGIN IMMEDIATE").unwrap();
    brokkr_store::test_support::plant_run(&holder, "peer", "{}").unwrap();
    holder
}

/// A site whose ledger holds two answered calls, closed clean, its store
/// waiting a short patience; and the journal's path.
fn two_answered() -> (Site, PathBuf) {
    let mut site = Site::folding();
    site.serve(|mut ledger| {
        answered(&mut ledger, 2);
        ledger.close().unwrap();
    });
    let patience = std::time::Duration::from_millis(25);
    site.engine.store.set_patience(patience).unwrap();
    let db = site.engine.store.path().to_path_buf();
    (site, db)
}

/// CR4: a peer's lock across repeated tries delays the calls, never loses
/// or doubles one: tried under the settlement's patiences, each lands
/// exactly once after the lock lets go, and the harness's success stands.
#[test]
fn a_lock_across_repeated_tries_commits_each_call_once_after_it_lets_go() {
    let (mut site, db) = two_answered();
    let (tries, mut peer) = (Cell::new(0), Some(write_lock_on(&db)));
    let settled = site.fold_between(succeeded(), |_| {
        tries.set(tries.get() + 1);
        if tries.get() == 3 {
            peer.take();
        }
    });
    assert_eq!(tries.get(), 3, "two tries met the lock");
    assert_eq!(settled.settled_outcome(), succeeded());
    let rows = [row(1, "lookup", "succeeded"), row(2, "lookup", "succeeded")];
    assert_eq!(site.rows(), rows);
}

/// CR4: a lock that outlasts every patience of the settlement strands the
/// calls: nothing was written, the attempt is indeterminate with the count
/// named, never its harness's success, and the ledger stays on disk as it
/// was, its evidence recoverable.
#[test]
fn a_lock_outlasting_the_settlement_strands_the_calls_and_keeps_their_evidence() {
    let (mut site, db) = two_answered();
    let before = fs::read(site.ledger()).unwrap();
    let (tries, _peer) = (Cell::new(0), write_lock_on(&db));
    let settled = site.fold_between(succeeded(), |_| tries.set(tries.get() + 1));
    assert_eq!((tries.get(), settled.settled_outcome()), (3, stranded(2)));
    assert_eq!(site.rows(), Vec::<Value>::new());
    assert_eq!(fs::read(site.ledger()).unwrap(), before);
}

/// The indeterminate outcome of `count` calls a lock outlasting the
/// settlement's three patiences stranded.
fn stranded(count: usize) -> AttemptOutcome {
    let reason = format!(
        "{count} checkpoint(s) were not journaled: a peer still held the journal's write lock \
         3 patiences after the seat stopped"
    );
    AttemptOutcome::Indeterminate { reason }
}

/// The settled checkpoints `rows` of the attempt appended to `store`'s
/// journal of `run`, as a commit whose acknowledgement never came back.
fn committed(store: &mut Store, run: &str, rows: &[Value]) {
    for checkpoint in rows {
        let payload = json!({"effect_id": EFFECT, "attempt_id": ATTEMPT, "checkpoint": checkpoint});
        let attempt = Some(ATTEMPT.to_string());
        let event = EventType::EffectCheckpointed;
        store
            .append_next(run, event, payload, None, attempt)
            .unwrap();
    }
}

/// CR4: a commit whose acknowledgement was lost is found in the journal
/// on the next try, by each call's id and whole payload, and nothing is
/// appended twice; settling the same ledger again, as a restarted engine
/// would, appends nothing either, and replays nothing.
#[test]
fn a_commit_whose_acknowledgement_was_lost_is_found_and_never_appended_twice() {
    let (mut site, db) = two_answered();
    let run = site.engine.run_id.clone();
    let rows = [row(1, "lookup", "succeeded"), row(2, "lookup", "succeeded")];
    let (tries, mut peer) = (Cell::new(0), Some(write_lock_on(&db)));
    let settled = site.fold_between(succeeded(), |store| {
        tries.set(tries.get() + 1);
        // The first try met the lock; the second finds what a try before
        // it committed, as a commit whose answer was lost leaves it.
        if tries.get() == 2 {
            peer.take();
            committed(store, &run, &rows);
        }
    });
    assert_eq!((tries.get(), settled.settled_outcome()), (2, succeeded()));
    assert_eq!(site.rows(), rows);
    let ledger = fs::read(site.ledger()).unwrap();
    let again = site.fold(succeeded());
    assert_eq!(
        (again.settled_outcome(), site.rows()),
        (succeeded(), rows.to_vec())
    );
    assert_eq!(fs::read(site.ledger()).unwrap(), ledger);
}

/// The conflicting duplicate's cause (CR3).
const CONFLICT: &str = "broker ledger contains a conflicting duplicate";

/// `row` with its digest `sha256`.
fn citing(mut row: Value, sha256: &str) -> Value {
    row["response_sha256"] = json!(sha256);
    row
}

/// CR3, CR4 (review H1): no payload the journal holds under a call id the
/// engine derived is passed over unseen. A first call journaled failed
/// then succeeded, or succeeded then failed, beside a ledger that answered
/// it; journaled twice whole; journaled with a digest its Terminal does
/// not cite; and journaled with a field seat-record v6 admits but the
/// engine never writes on a settled call, a turn: each is
/// the conflicting duplicate, the journal is left exactly as it was, and
/// nothing is appended, the ledger's second call included.
#[test]
fn every_journaled_payload_of_a_derived_call_is_compared_whole() {
    let answer = |call| row(call, "lookup", "succeeded");
    let mut turned = answer(1);
    turned["turn"] = json!(1);
    let cases: [Vec<Value>; 5] = [
        vec![row(1, "lookup", "failed"), answer(1)],
        vec![answer(1), row(1, "lookup", "failed")],
        vec![answer(1), answer(1)],
        vec![citing(answer(1), &"0".repeat(64))],
        vec![turned],
    ];
    for journaled in cases {
        let (mut site, _db) = two_answered();
        let run = site.engine.run_id.clone();
        committed(&mut site.engine.store, &run, &journaled);
        let settled = site.fold(succeeded());
        assert_eq!(settled.settled_outcome(), failed(CONFLICT), "{journaled:?}");
        assert_eq!(site.rows(), journaled, "{journaled:?}");
    }
}

/// CR3 (review H1): two rows the engine derived for one call id are never
/// both appended, nor one indexed over the other: the same call settled
/// twice, as itself or in another state, is the conflicting duplicate.
#[test]
fn two_derived_rows_of_one_call_are_a_conflict() {
    let event = |state| {
        let payload = json!({"effect_id": EFFECT, "attempt_id": ATTEMPT, "checkpoint": row(1, "lookup", state)});
        serde_json::from_value::<SettledEvent>(payload).unwrap()
    };
    for twice in [
        [event("succeeded"), event("failed")],
        [event("succeeded"), event("succeeded")],
    ] {
        let pending = capability_calls::unjournaled(&[], ATTEMPT, &twice);
        assert_eq!(pending, Err(Invalid::Conflict), "{twice:?}");
    }
}

/// CR4 (review M1): what is read before each try is the journal since the
/// attempt began, never the run whole: a row standing before the attempt
/// began, which a whole read would find under the first call's id, is
/// never seen, so the ledger's calls are appended behind it.
#[test]
fn the_journal_is_read_only_since_the_attempt_began() {
    let (mut site, _db) = two_answered();
    let run = site.engine.run_id.clone();
    let before = [row(1, "lookup", "failed")];
    committed(&mut site.engine.store, &run, &before);
    site.begin();
    let settled = site.fold(succeeded());
    assert_eq!(settled.settled_outcome(), succeeded());
    let rows = [
        row(1, "lookup", "failed"),
        row(1, "lookup", "succeeded"),
        row(2, "lookup", "succeeded"),
    ];
    assert_eq!(site.rows(), rows);
}

/// Where `sha256` is published below `site`'s worktree's artifact root.
fn published(site: &Site, sha256: &str) -> PathBuf {
    let work = fs::canonicalize(site.engine.workdir()).unwrap();
    work.join(".forge/artifacts/sha256").join(sha256)
}

/// The bytes published at `sha256` for `site`, if any.
fn artifact(site: &Site, sha256: &str) -> Option<Vec<u8>> {
    fs::read(published(site, sha256)).ok()
}

/// The one response every retaining site's call answered with.
const ANSWER: &[u8] = br#"{"result":{"content":[]}}"#;

/// A site whose one call succeeded with `ANSWER` retained and cited, and
/// that digest.
fn retaining() -> (Site, String) {
    let site = Site::new(true, vec!["must-not-run".into()]);
    let cited = sha(ANSWER);
    let answers = [(Some((ANSWER, cited.as_str())), Outcome::Succeeded)];
    site.attempt.served((0, &site.digest), true, &answers);
    (site, cited)
}

/// CR2, CC2 (review L2): a retained response is published at its digest
/// before the checkpoint that cites it is first tried, and the checkpoint
/// carries that digest beside its state.
#[test]
fn a_retained_response_is_published_before_the_checkpoint_citing_it() {
    let (mut site, cited) = retaining();
    let (at, tried) = (published(&site, &cited), RefCell::new(Vec::new()));
    let settled = site.fold_between(succeeded(), |_| tried.borrow_mut().push(fs::read(&at).ok()));
    assert_eq!(
        tried.take(),
        [Some(ANSWER.to_vec())],
        "published when tried"
    );
    assert_eq!(settled.settled_outcome(), succeeded());
    assert_eq!(site.rows(), [citing(row(1, "lookup", "succeeded"), &cited)]);
}

/// CR2: a cited stage that is not the bytes its Terminal cites is never
/// published, and no call of the attempt is appended: its success fails
/// with the publication's cause.
#[test]
fn an_unpublished_response_appends_no_call() {
    let (mut site, cited) = retaining();
    let stage = site.attempt.dir().join("plan-0.json.1.response");
    fs::write(stage, br#"{"result":{"content":[1]}}"#).unwrap();
    let settled = site.fold(succeeded());
    let cause = "broker response digest mismatch";
    assert_eq!(settled.settled_outcome(), failed(cause));
    assert_eq!((site.rows(), artifact(&site, &cited)), (vec![], None));
}

/// Evidence whose run directory is gone, so no attempt directory stands,
/// or is a file, so none can be opened, is not the attempt's: nothing is
/// read or appended, and the harness's success does not stand.
#[test]
fn evidence_without_its_run_directory_is_unbound() {
    for file in [false, true] {
        let mut site = Site::folding();
        site.serve(|mut ledger| {
            answered(&mut ledger, 1);
            ledger.close().unwrap();
        });
        let run = site.attempt.dir().parent().unwrap().to_path_buf();
        fs::remove_dir_all(&run).unwrap();
        if file {
            made(&run, b"");
        }
        let settled = site.fold(succeeded());
        let cause = "broker evidence is not bound to this attempt";
        assert_eq!(
            (settled.settled_outcome(), site.rows()),
            (failed(cause), vec![]),
            "a file: {file}"
        );
    }
}

/// A broker sealed with a plan its attempt's inventory does not pin is not
/// the attempt's: nothing is appended, and the harness's success does not
/// stand.
#[test]
fn a_broker_its_inventory_does_not_pin_is_unbound() {
    let mut site = Site::folding();
    site.serve(|mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    let serving = site.spawn.serving.as_mut().unwrap();
    let SealedServers::Brokered(brokers) = &mut serving.isolation.servers else {
        unreachable!("the site is brokered")
    };
    brokers[0].plan_digest = "f".repeat(64);
    let settled = site.fold(succeeded());
    let cause = "broker evidence is not bound to this attempt";
    assert_eq!(
        (settled.settled_outcome(), site.rows()),
        (failed(cause), vec![])
    );
}

/// CR4: the head an attempt began at is the anchor every read verifies:
/// one the journal does not hold is a storage failure handed up, and
/// nothing is read past it or appended.
#[test]
fn a_head_the_journal_does_not_hold_is_handed_up_and_appends_nothing() {
    let mut site = Site::folding();
    site.serve(|mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    let begun = Begun((1, "0".repeat(64)));
    let reported = report(succeeded(), "");
    let folded = site
        .engine
        .fold_with([EFFECT, ATTEMPT], &site.spawn, &begun, reported, |_| {});
    let unknown = matches!(
        folded,
        Err(EngineError::Store(brokkr_store::StoreError::UnknownHead {
            seq: 1
        }))
    );
    assert!(unknown, "{folded:?}");
    assert_eq!(site.rows(), Vec::<Value>::new());
}

/// CR4 (review M1): brokers are folded one at a time, each one's calls
/// appended before the next one's ledger is read: the second plan's ledger
/// is laid down only while the first's calls are tried, and is found.
#[test]
fn each_broker_is_appended_before_the_next_ledger_is_read() {
    let mut site = Site::folding();
    let digest = site.second();
    site.serve(|mut ledger| {
        answered(&mut ledger, 2);
        ledger.close().unwrap();
    });
    let wiki = written((&site.attempt.owner(), &digest), |mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    let (second, run) = (
        site.attempt.dir().join("plan-1.json.ledger"),
        site.engine.run_id.clone(),
    );
    let journaled = RefCell::new(Vec::new());
    let settled = site.fold_between(succeeded(), |store| {
        journaled.borrow_mut().push(store.load(&run).unwrap().len());
        if !second.exists() {
            made(&second, &wiki);
        }
    });
    assert_eq!(settled.settled_outcome(), succeeded());
    let rows = [
        row(1, "lookup", "succeeded"),
        row(2, "lookup", "succeeded"),
        served("wiki", 1, "lookup", "succeeded"),
    ];
    assert_eq!(site.rows(), rows);
    let [first, then] = journaled.take()[..] else {
        panic!("one try for each broker")
    };
    assert_eq!(then - first, 2, "the first broker's calls landed first");
}

/// CR4: once a lock outlasting the settlement strands one broker's calls,
/// the next broker's are counted with them and never tried, since the
/// same lock outlasts them: the attempt is indeterminate with every one
/// named.
#[test]
fn a_lock_stranding_one_broker_strands_the_next_without_trying_it() {
    let (mut site, db) = two_answered();
    let digest = site.second();
    let wiki = written((&site.attempt.owner(), &digest), |mut ledger| {
        answered(&mut ledger, 1);
        ledger.close().unwrap();
    });
    made(&site.attempt.dir().join("plan-1.json.ledger"), &wiki);
    let (tries, _peer) = (Cell::new(0), write_lock_on(&db));
    let settled = site.fold_between(succeeded(), |_| tries.set(tries.get() + 1));
    assert_eq!((tries.get(), settled.settled_outcome()), (3, stranded(3)));
    assert_eq!(site.rows(), Vec::<Value>::new());
}

/// Seat-record v6's fence refuses a settled checkpoint whose tool is not
/// in its vocabulary: the calls before it are journaled, none from it on,
/// and the attempt fails with the fence's refusal.
#[test]
fn a_checkpoint_the_fence_refuses_fails_the_attempt_after_the_calls_before_it() {
    let mut site = Site::folding();
    site.serve(|mut ledger| {
        call(&mut ledger, "lookup", Outcome::Succeeded);
        call(
            &mut ledger,
            "not a tool",
            Outcome::Refused(Denial::Ungranted),
        );
        call(&mut ledger, "lookup", Outcome::Succeeded);
        ledger.close().unwrap();
    });
    let settled = site.fold(succeeded());
    let cause = "seat record at journal seq 3 violates contracts/seat-record.v6.schema.json at /";
    assert_eq!(settled.settled_outcome(), failed(cause));
    assert_eq!(site.rows(), [row(1, "lookup", "succeeded")]);
}

/// U8e's boundary: an attempt whose processes are not proven settled is
/// not read, even where its ledger is missing, since a live broker may
/// still be writing it: the report is as it was, and nothing is appended.
#[test]
fn an_attempt_not_proven_settled_is_not_read() {
    let mut site = Site::folding();
    let reason = Unsettled::Stdout;
    let unsettled = AttemptReport {
        cleanup: Cleanup::Unresolved { reason },
        ..report(succeeded(), "")
    };
    let (spawn, begun) = (site.spawn.clone(), &site.begun);
    let folded = site
        .engine
        .fold_with([EFFECT, ATTEMPT], &spawn, begun, unsettled, |_| {});
    let folded = folded.unwrap();
    assert_eq!((folded.refused, site.rows()), (None, vec![]));
}

/// A site sealed with a broker whose engine holds no lease reads no
/// evidence: it is unbound, and the harness's success does not stand.
#[test]
fn a_brokered_site_without_its_lease_is_unbound() {
    let mut site = Site::folding();
    site.serve(|ledger| drop(ledger.close()));
    site.engine.held = None;
    let settled = site.fold(succeeded());
    let cause = "broker evidence is not bound to this attempt";
    assert_eq!(
        (settled.settled_outcome(), site.rows()),
        (failed(cause), vec![])
    );
}

/// The driver barrier, end to end (CC2, CR4): a dispatched driver that
/// reports success after its broker closed failed has its attempt
/// journaled failed with the exact cause and no call; one whose broker
/// answered one call, and whose harness also reported that call as MCP
/// telemetry, journals exactly one attributed checkpoint for it beside the
/// ordinary one, and its success.
#[test]
fn a_dispatched_driver_is_settled_by_its_broker_calls_before_its_result_is_admitted() {
    let telemetry = json!({
        "step": "tool", "tool": "lookup", "turn": 1,
        "observation": {
            "format": "claude", "call": "toolu_1",
            "tool": {"kind": "mcp", "server": "cap-docs", "tool": "lookup"},
        },
    });
    let argv = checkpointing_command(EFFECT, ATTEMPT, &[telemetry], succeeded());
    let cases: [(u64, Option<Failure>, EventType, usize); 2] = [
        (
            0,
            Some(Failure::Refused(Refusal::Exec)),
            EventType::EffectFailed,
            0,
        ),
        (1, None, EventType::EffectSucceeded, 1),
    ];
    for (calls, failure, ended, attributed) in cases {
        let mut site = Site::new(false, argv.clone());
        site.serve(|mut ledger| {
            answered(&mut ledger, calls);
            failure.into_iter().for_each(|failure| ledger.fail(failure));
            ledger.close().unwrap();
        });
        let spawn = site.spawn.clone();
        let mut input = json!({});
        input[SERVING_INPUTS] = spawn.serving.as_ref().unwrap().value();
        let deadline = std::time::Duration::from_secs(10);
        let engine = &mut site.engine;
        let run = engine.run_driver(EFFECT, ATTEMPT, "work", &spawn, input, deadline, None, None);
        let run = run.unwrap();
        assert!(
            matches!(run, DriverRun::Ran(_)),
            "{ended:?}: the driver ran"
        );
        let selection = Selection::new();
        engine
            .conclude_single(EFFECT, ATTEMPT, run, &selection, None)
            .unwrap();
        let events = engine.store.load(&engine.run_id).unwrap();
        let last = events.last().unwrap();
        assert_eq!(last.event_type, ended);
        let rows = site.rows();
        let attributed_rows = rows.iter().filter(|row| row.get("capability").is_some());
        assert_eq!(attributed_rows.count(), attributed, "{ended:?}");
        assert_eq!(rows.len(), attributed + 1, "{ended:?}: the telemetry row");
        if ended == EventType::EffectFailed {
            let error = last.payload["error"].as_str().unwrap();
            let cause = "MCP server could not start inside its box; stderr tail: ";
            assert!(error.starts_with(cause), "{error}");
        }
    }
}
