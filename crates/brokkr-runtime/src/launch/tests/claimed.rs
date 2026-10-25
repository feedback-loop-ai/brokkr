//! A dispatcher's start under its queue claim (decision 0095 rulings 4, 5
//! and 8), driven as the dispatcher will drive it: the journal taken by
//! its lock, the entry queued as `brokkr queue add` writes it and claimed
//! by the dispatcher's own step for the bundle the entry compiles, then
//! rebuilt and started through the engine, the claim set on the run it
//! rebuilds.

use std::num::NonZeroU32;

use brokkr_store::{Attribution, Claim, EntryId, Generation, NewEntry, QueueRefusal};

use super::*;
use crate::dispatch::{take_dispatch, Dispatcher};

/// One entry claimed: the dispatcher that took its journal, holding the
/// lock, and its generation; the entry and its claim; and the request and
/// run the entry rebuilds as, the claim set on that run.
pub(crate) struct Claimed {
    pub(crate) _dispatcher: Dispatcher,
    pub(crate) generation: Generation,
    pub(crate) entry: EntryId,
    pub(crate) claim: Claim,
    pub(crate) request: LaunchRequest,
    pub(crate) run: NewRun,
}

/// A journal at `root` holding one entry, the unmapped launch of
/// [`bundle_at`], claimed by the dispatcher that took the journal for the
/// bundle of manifest digest `measured`, else the one the entry compiles,
/// as a pass measures it; the entry rebuilt, which gives no claim, and the
/// claim set on the run it rebuilds as.
pub(crate) fn claimed(root: &Path, measured: Option<&str>) -> Claimed {
    let asked = request(root, &bundle_at(root, false), root, root);
    let queued = QueuedLaunch::of(&asked, &new_run(None)).unwrap();
    let mut store = Store::open(&asked.journal).unwrap();
    let payload = queued.encode().unwrap();
    let new = NewEntry {
        payload: &payload,
        priority: 0,
        waits: &[],
    };
    let by = Attribution {
        operator: "vy",
        reason: "queued",
    };
    let entry = store.queue_add(new, by).unwrap();
    let dispatcher = take_dispatch(&mut store).unwrap().unwrap();
    let generation = dispatcher.generation;
    let compiled = queued.compiled().unwrap().manifest_digest();
    let digest = measured.unwrap_or(&compiled);
    let read = store.queue_entry(entry).unwrap();
    let judged = store.queue_claim_with(&read, 0, digest, generation, "admitted", |_, _, _| {
        Ok::<(), ()>(())
    });
    let claim = judged.unwrap().unwrap();
    let (request, mut run) = queued.rebuild(asked.journal, asked.host_path).unwrap();
    assert!(run.claim.is_none(), "a rebuilt entry holds no claim");
    run.claim = Some((claim, generation));
    Claimed {
        _dispatcher: dispatcher,
        generation,
        entry,
        claim,
        request,
        run,
    }
}

/// The published run-manifest contract of `version`.
fn manifest_contract(version: u8) -> jsonschema::Validator {
    let path = workspace().join(format!("contracts/run-manifest.v{version}.schema.json"));
    let schema: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    jsonschema::draft7::new(&schema).unwrap()
}

/// The manifest the journal at `root` pinned for `run`, which its
/// `run/started` carries too.
fn pinned(root: &Path, run: &str) -> Value {
    let store = Store::open(&root.join("forge.db")).unwrap();
    let manifest = store.manifest(run).unwrap();
    assert_eq!(store.load(run).unwrap()[0].payload["manifest"], manifest);
    manifest
}

/// Ruling 5: a run the dispatcher starts pins the claim it was started
/// under, its entry and its seq, into its manifest, which is then a
/// run-manifest/v14 one and no v13 one; a direct run pins none, and its
/// manifest is both.
#[test]
fn a_dispatched_start_pins_its_claim_and_a_direct_start_pins_none() {
    let dir = tempfile::tempdir().unwrap();
    let claimed = claimed(dir.path(), None);
    let direct = start(claimed.request.clone(), new_run(None), &mut silent).unwrap();
    let dispatched = start(claimed.request, claimed.run, &mut silent).unwrap();
    let [v13, v14] = [13, 14].map(manifest_contract);

    let manifest = pinned(dir.path(), &dispatched.run_id);
    // The journal's first entry, claimed by its second command.
    assert_eq!(claimed.claim.parts(), (claimed.entry, 2));
    let pin = json!({"entry": 1, "claim": 2});
    assert_eq!(manifest["queue_entry"], pin);
    assert!(
        v14.is_valid(&manifest) && !v13.is_valid(&manifest),
        "{manifest}"
    );

    let manifest = pinned(dir.path(), &direct.run_id);
    assert_eq!(manifest.get("queue_entry"), None);
    assert!(
        v14.is_valid(&manifest) && v13.is_valid(&manifest),
        "{manifest}"
    );
}

/// Ruling 5: the claim is pinned at start, never compiled into a bundle,
/// so a dispatched run resumes with no drift and keeps its pin.
#[test]
fn a_dispatched_run_resumes_with_no_drift_and_keeps_its_claim() {
    let dir = tempfile::tempdir().unwrap();
    let claimed = claimed(dir.path(), None);
    let request = claimed.request.clone();
    let started = start(request, claimed.run, &mut silent).unwrap();
    let run = started.run_id.clone();
    drop(started);
    let resumed = resume(claimed.request, &run).unwrap();
    assert_eq!(resumed.run_id, run);
    let claim = json!(claimed.claim.parts().1);
    assert_eq!(pinned(dir.path(), &run)["queue_entry"]["claim"], claim);
}

/// A dispatched start of the entry `claimed` holds, refused: by the
/// queue's refusal, whose text is the launch's, with no run written.
fn refused(root: &Path, claimed: Claimed) -> QueueRefusal {
    let error = start(claimed.request, claimed.run, &mut silent);
    let error = error.err().expect("refused");
    assert_eq!(rows(&root.join("forge.db")), vec![]);
    let text = error.to_string();
    let LaunchError::Engine(EngineError::Store(StoreError::Queue(refusal))) = error else {
        panic!("{error:?}")
    };
    assert_eq!(refusal.to_string(), text);
    refusal
}

/// Rulings 4 and 7: a dispatched start whose bundle is not the one its
/// claim's pass measured, or whose claim was reclaimed, is refused by the
/// queue's fence inside the start's one transaction and writes no run.
#[test]
fn a_dispatched_start_whose_claim_does_not_stand_writes_no_run() {
    let dir = tempfile::tempdir().unwrap();
    let moved = claimed(dir.path(), Some("measured"));
    let refusal = QueueRefusal::BundleMoved(moved.claim, "measured".into());
    assert_eq!(refused(dir.path(), moved), refusal);

    let dir = tempfile::tempdir().unwrap();
    let reclaimed = claimed(dir.path(), None);
    let mut store = Store::open(&dir.path().join("forge.db")).unwrap();
    let (claim, generation) = (reclaimed.claim, reclaimed.generation);
    let bound = NonZeroU32::MIN;
    (store.queue_reclaim(claim, generation, None, bound, "reclaimed")).unwrap();
    assert_eq!(refused(dir.path(), reclaimed), QueueRefusal::Stale(claim));
}

/// A claim beside a Looper dispatch is refused before the envelope is
/// read or a run written: its run-manifest/v2 carries no `queue_entry`.
#[test]
fn a_claim_beside_a_dispatch_is_refused_and_writes_no_run() {
    let dir = tempfile::tempdir().unwrap();
    let claimed = claimed(dir.path(), None);
    let run = NewRun {
        dispatch: Some(dir.path().join("never-read.json")),
        ..claimed.run
    };
    let error = start(claimed.request, run, &mut silent)
        .err()
        .expect("refused");
    assert!(matches!(error, LaunchError::ClaimWithDispatch), "{error:?}");
    assert_eq!(
        error.to_string(),
        "a queue claim cannot start a run with --dispatch: its run-manifest/v2 carries no \
         queue_entry, so the queue holds such an entry (decision 0095 ruling 4); nothing was \
         started"
    );
    assert_eq!(rows(&dir.path().join("forge.db")), vec![]);
}
