//! Reconciliation at each durable boundary of a start from the queue
//! (decision 0095 ruling 3): one entry is driven through its claim, the
//! engine's start under that claim, its link and its `spawn` row, by the
//! dispatcher that took the journal, and the pass ends at a boundary as a
//! crash ends it. The next dispatcher reconciles, and the queue's state,
//! its command rows and the runs that exist say what it did.

use std::path::Path;

use brokkr_store::{EntryId, EntryState, Generation, QueueEntry, QueueRefusal};
use uuid::Uuid;

use super::*;
use crate::dispatch::take_dispatch;
use crate::launch::start;
use crate::launch::tests::claimed::{claimed, Claimed};

/// The durable steps of a start from the queue, by the last one a pass
/// wrote before it ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Boundary {
    Claim,
    Start,
    Link,
    Spawn,
}

/// One entry of the journal at `root`, claimed by the dispatcher that took
/// it and driven by it through each step to `boundary`, the fault seam
/// that ends its pass there; the dispatcher, its lock held, the claim and
/// the run its start wrote, if it got that far.
fn driven_to(root: &Path, boundary: Boundary) -> (Dispatcher, Claim, Option<String>) {
    let Claimed {
        _dispatcher: dispatcher,
        generation,
        claim,
        request,
        run,
        ..
    } = claimed(root, None);
    if boundary == Boundary::Claim {
        return (dispatcher, claim, None);
    }
    let run = start(request, run, &mut |_| {}).unwrap().run_id;
    let mut store = journal(root);
    if boundary >= Boundary::Link {
        store
            .queue_link(claim, generation, &run, "started")
            .unwrap();
    }
    if boundary == Boundary::Spawn {
        store.queue_spawn(claim, generation, "spawning").unwrap();
    }
    (dispatcher, claim, Some(run))
}

/// The journal at `root`.
fn journal(root: &Path) -> Store {
    Store::open(&root.join("forge.db")).unwrap()
}

/// A pass ended at `boundary` by a crash: its dispatcher gone and its lock
/// with it, and the next dispatcher holding the journal, as generation 2.
fn crashed_at(root: &Path, boundary: Boundary) -> (Store, Dispatcher, Claim, Option<String>) {
    let (first, claim, run) = driven_to(root, boundary);
    drop(first);
    let mut store = journal(root);
    let next = take_dispatch(&mut store).unwrap().unwrap();
    assert_eq!(next.generation, Generation(2));
    (store, next, claim, run)
}

/// Every queue command the journal holds, in order: its seq, command,
/// operator and reason.
fn commands(store: &Store) -> Vec<(i64, String, String, String)> {
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    let query = "SELECT seq, command, operator, reason FROM queue_commands ORDER BY seq";
    let mut rows = conn.prepare(query).unwrap();
    let read = |row: &rusqlite::Row<'_>| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?));
    let rows = rows.query_map([], read).unwrap();
    rows.collect::<Result<_, _>>().unwrap()
}

/// A command row: `seq`, then `command`, by `operator`, for `reason`.
fn row(seq: i64, command: &str, operator: &str, reason: &str) -> (i64, String, String, String) {
    (seq, command.into(), operator.into(), reason.into())
}

/// The `link` row's reason for `run`, adopted under claim 2 of entry 1.
fn adoption(run: &str) -> String {
    format!(
        "reconciled: run '{run}' carries claim 2 of entry 1, so it is adopted, never \
         relaunched, stopped or concluded (decision 0068 ruling 7)"
    )
}

/// The rows a start writes up to its link, the entry added by the
/// operator and claimed by the dispatcher.
fn added_and_claimed() -> Vec<(i64, String, String, String)> {
    vec![
        row(1, "add", "vy", "queued"),
        row(2, "claim", "dispatch", "admitted"),
    ]
}

/// The run ids the journal holds.
fn runs(store: &Store) -> Vec<String> {
    let listed = store.list_runs().unwrap().into_iter();
    listed.map(|(run, ..)| run).collect()
}

/// What `dispatcher` settled reconciling `store`, refused by neither.
fn reconciled(store: &mut Store, dispatcher: &Dispatcher) -> Vec<Settled> {
    reconcile(store, dispatcher).unwrap().unwrap()
}

/// A claim row: its seq, entry, generation, digest and run.
type ClaimRow = (i64, i64, i64, String, Option<String>);

/// A journal's entries, claims, commands and runs, row for row.
type Written = (
    Vec<QueueEntry>,
    Vec<ClaimRow>,
    Vec<(i64, String, String, String)>,
    Vec<String>,
);

/// What a reconciliation may write, as the journal at `store`'s path holds
/// it, row for row: its entries, its claims, its commands and its runs.
fn written(store: &Store) -> Written {
    let conn = rusqlite::Connection::open(store.path()).unwrap();
    let query = "SELECT seq, entry_id, generation, digest, run_id FROM queue_claims ORDER BY seq";
    let mut rows = conn.prepare(query).unwrap();
    let read = |row: &rusqlite::Row<'_>| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
        ))
    };
    let claims = rows.query_map([], read).unwrap();
    let claims = claims.collect::<Result<_, _>>().unwrap();
    let queue = store.queue_list().unwrap();
    (queue, claims, commands(store), runs(store))
}

/// Assert `refused` refuses `dispatcher`, which took the journal `took`,
/// the store of journal `named`, by its variant and its text.
fn refused_elsewhere(refused: DispatcherError, took: Option<Uuid>, named: Option<Uuid>) {
    let id = |journal: Option<Uuid>| journal.map_or("(no id)".into(), |id| id.to_string());
    let text = format!(
        "dispatch: this dispatcher took journal {}, and the store it was handed holds journal \
         {}; a dispatcher acts in the journal it took alone, so nothing was written (decision \
         0095)",
        id(took),
        id(named)
    );
    assert_eq!(refused.to_string(), text);
    assert_eq!(refused, DispatcherError::OtherJournal(took, named));
}

/// After the claim, no run carries it: the entry goes back to its place
/// by a `reclaim` naming the claim it ends, no run is written, and its
/// next claim is a new seq.
#[test]
fn a_pass_ended_after_its_claim_is_reclaimed_to_its_place_and_claimed_anew() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, next, claim, _) = crashed_at(dir.path(), Boundary::Claim);

    let settled = reconciled(&mut store, &next);
    assert_eq!(settled, vec![Settled::Reclaimed(claim)]);
    let entry = store.queue_entry(EntryId(1)).unwrap();
    assert_eq!(
        (entry.state.clone(), entry.position),
        (EntryState::Queued, Some(1))
    );
    let reason = "reconciled: no run carries claim 2 of entry 1, so the claim ends and the entry \
                  returns to its place";
    let mut rows = added_and_claimed();
    rows.push(row(3, "reclaim", "dispatch", reason));
    assert_eq!(commands(&store), rows);
    assert_eq!(runs(&store), Vec::<String>::new());

    let judged = store.queue_claim_with(&entry, 0, "d", next.generation, "again", |_, _, _| {
        Ok::<(), ()>(())
    });
    assert_eq!(judged.unwrap().unwrap().parts(), (EntryId(1), 4));
}

/// After the start, its run carries the claim: the entry is linked to it,
/// adopted by a `link` that says so, to be spawned, and no second run is
/// written. No reclaim ends a claim whose run exists.
#[test]
fn a_pass_ended_after_its_start_is_adopted_and_linked_with_no_second_run() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, next, claim, run) = crashed_at(dir.path(), Boundary::Start);
    let run = run.unwrap();

    let settled = reconciled(&mut store, &next);
    assert_eq!(settled, vec![Settled::ToSpawn(claim, run.clone())]);
    let state = store.queue_entry(EntryId(1)).unwrap().state;
    assert_eq!(
        state,
        EntryState::Claimed { run: run.clone() },
        "no reclaim ends a claim whose run exists"
    );
    let mut rows = added_and_claimed();
    rows.push(row(3, "link", "dispatch", &adoption(&run)));
    assert_eq!(commands(&store), rows);
    assert_eq!(runs(&store), vec![run]);
}

/// After the link, its run started no driver: the entry is to be spawned,
/// with no second run and no second link, and the next dispatcher's own
/// `spawn` row lands under the claim the first took.
#[test]
fn a_pass_ended_after_its_link_is_to_be_spawned_with_no_second_run_or_link() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, next, claim, run) = crashed_at(dir.path(), Boundary::Link);
    let run = run.unwrap();
    let mut rows = added_and_claimed();
    rows.push(row(3, "link", "dispatch", "started"));
    assert_eq!(commands(&store), rows);

    let settled = reconciled(&mut store, &next);
    assert_eq!(settled, vec![Settled::ToSpawn(claim, run.clone())]);
    assert_eq!(commands(&store), rows);
    assert_eq!(runs(&store), vec![run]);

    store
        .queue_spawn(claim, next.generation, "spawning")
        .unwrap();
    rows.push(row(4, "spawn", "dispatch", "spawning"));
    assert_eq!(commands(&store), rows);
}

/// After the `spawn` row, the entry is settled: reconciliation leaves it
/// alone, writes nothing and asks for no second spawn.
#[test]
fn a_pass_ended_after_its_spawn_row_is_left_alone_and_never_spawned_again() {
    let dir = tempfile::tempdir().unwrap();
    let (mut store, next, _, run) = crashed_at(dir.path(), Boundary::Spawn);
    let mut rows = added_and_claimed();
    rows.push(row(3, "link", "dispatch", "started"));
    rows.push(row(4, "spawn", "dispatch", "spawning"));
    assert_eq!(commands(&store), rows);

    assert_eq!(reconciled(&mut store, &next), vec![]);
    assert_eq!(commands(&store), rows);
    assert_eq!(runs(&store), vec![run.unwrap()]);
}

/// A standing dispatcher whose link, or spawn, was `Contended` past its
/// patience wrote nothing for it, and reconciles by the same rules under
/// its own generation: the run its start wrote is adopted, and a linked
/// entry is to be spawned.
#[test]
fn a_standing_dispatcher_reconciles_its_own_unlinked_or_unspawned_entry() {
    let dir = tempfile::tempdir().unwrap();
    let (standing, claim, run) = driven_to(dir.path(), Boundary::Start);
    let (mut store, run) = (journal(dir.path()), run.unwrap());
    let settled = reconciled(&mut store, &standing);
    assert_eq!(settled, vec![Settled::ToSpawn(claim, run.clone())]);
    let link = commands(&store).pop().unwrap();
    assert_eq!(link, row(3, "link", "dispatch", &adoption(&run)));

    let dir = tempfile::tempdir().unwrap();
    let (standing, claim, run) = driven_to(dir.path(), Boundary::Link);
    let mut store = journal(dir.path());
    let before = commands(&store);
    let settled = reconciled(&mut store, &standing);
    assert_eq!(settled, vec![Settled::ToSpawn(claim, run.unwrap())]);
    assert_eq!(commands(&store), before);
}

/// A dispatcher a newer one fenced out reconciles nothing, whether its
/// entry was left launching or linked: refused as superseded, with no row
/// written and none named to spawn.
#[test]
fn a_superseded_dispatchers_reconciliation_is_refused_and_writes_nothing() {
    for (boundary, left) in [(Boundary::Claim, true), (Boundary::Link, false)] {
        let dir = tempfile::tempdir().unwrap();
        let (first, _, _) = driven_to(dir.path(), boundary);
        let mut store = journal(dir.path());
        // Its lock file removed under it, and a second dispatcher in.
        assert_eq!(store.dispatch_hold(1).unwrap(), Generation(2));
        let before = commands(&store);

        let error = reconcile(&mut store, &first).unwrap_err();
        assert_eq!(
            error.to_string(),
            "dispatcher generation 1 is not the journal's latest holder; it writes nothing"
        );
        assert!(
            matches!(
                error,
                StoreError::Queue(QueueRefusal::Superseded(Generation(1)))
            ),
            "{error:?}"
        );
        assert_eq!(commands(&store), before);
        let state = store.queue_entry(EntryId(1)).unwrap().state;
        assert_eq!(state == EntryState::Launching, left, "{boundary:?}");
    }
}

/// The journal at `root`'s own id.
fn id_of(root: &Path) -> Option<Uuid> {
    journal(root).journal().unwrap()
}

/// A dispatcher acts in the journal it took alone (decision 0095 rulings
/// 2 and 3), whatever its generation: journals A and B each hold their
/// entry 1 left launching under claim 2 by their own live dispatcher, both
/// of generation 1. A's dispatcher handed B's store is refused by
/// `OtherJournal`, naming both journals, and B's entries, claims, commands
/// and runs are unchanged, row for row; B's own dispatcher reconciles it.
#[test]
fn a_dispatcher_reconciles_no_journal_but_the_one_it_took() {
    let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let (of_a, _, _) = driven_to(a.path(), Boundary::Claim);
    let (of_b, claim, _) = driven_to(b.path(), Boundary::Claim);
    assert_eq!(
        (of_a.generation, of_b.generation),
        (Generation(1), Generation(1))
    );
    let mut store = journal(b.path());
    let before = written(&store);

    let refused = reconcile(&mut store, &of_a).unwrap().unwrap_err();
    refused_elsewhere(refused, id_of(a.path()), id_of(b.path()));
    assert_eq!(written(&store), before);
    assert_eq!(
        reconciled(&mut store, &of_b),
        vec![Settled::Reclaimed(claim)]
    );
}

/// A dispatcher's journal is the id it names itself by, not a path, as
/// slice 3a's claim reads it (#430, 2026-10-24). With A's dispatcher live,
/// a store opened through a symlink to B's file, the link retargeted to
/// A's since, or opened at A's path after B's file was renamed into it,
/// holds B: it is refused by `OtherJournal` with B's rows unchanged. A
/// byte copy of A carries its id, so it is the same journal, and the
/// dispatcher reconciles the copy, A's own rows unmoved.
#[test]
fn a_dispatcher_is_bound_to_its_journals_id_not_a_path() {
    for renamed in [false, true] {
        let (a, b) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let (of_a, _, _) = driven_to(a.path(), Boundary::Claim);
        let (_of_b, _, _) = driven_to(b.path(), Boundary::Claim);
        let (took, named) = (id_of(a.path()), id_of(b.path()));
        let (to_a, to_b) = (a.path().join("forge.db"), b.path().join("forge.db"));
        let mut store = match renamed {
            false => {
                let link = a.path().join("current.db");
                std::os::unix::fs::symlink(&to_b, &link).unwrap();
                let through = Store::open(&link).unwrap();
                std::fs::remove_file(&link).unwrap();
                std::os::unix::fs::symlink(&to_a, &link).unwrap();
                through
            }
            true => {
                std::fs::rename(&to_b, &to_a).unwrap();
                Store::open(&to_a).unwrap()
            }
        };
        let before = written(&store);
        let refused = reconcile(&mut store, &of_a).unwrap().unwrap_err();
        refused_elsewhere(refused, took, named);
        assert_eq!(written(&store), before, "renamed: {renamed}");
    }

    let dir = tempfile::tempdir().unwrap();
    let (of_a, claim, _) = driven_to(dir.path(), Boundary::Claim);
    let copy = dir.path().join("copy.db");
    std::fs::copy(dir.path().join("forge.db"), &copy).unwrap();
    let before = written(&journal(dir.path()));
    let mut copied = Store::open(&copy).unwrap();
    assert_eq!(copied.journal().unwrap(), id_of(dir.path()));
    assert_eq!(
        reconciled(&mut copied, &of_a),
        vec![Settled::Reclaimed(claim)]
    );
    assert_eq!(written(&journal(dir.path())), before);
}

/// A journal with no id is none a dispatcher took: one taken while its
/// journal had no id reconciles nothing, not even in that journal, with
/// no id still or named since, and a dispatcher of a named journal
/// reconciles nothing in it once it has no id. Each is refused by
/// `OtherJournal`, with nothing written.
#[test]
fn a_journal_with_no_id_is_reconciled_by_no_dispatcher() {
    let dir = tempfile::tempdir().unwrap();
    let (first, _, _) = driven_to(dir.path(), Boundary::Claim);
    drop(first);
    let path = dir.path().join("forge.db");
    let unnamed = || {
        let conn = rusqlite::Connection::open(&path).unwrap();
        brokkr_store::test_support::unname(&conn).unwrap();
    };
    let mut store = journal(dir.path());
    unnamed();
    let next = take_dispatch(&mut store).unwrap().unwrap();
    let before = written(&store);
    let refused = reconcile(&mut store, &next).unwrap().unwrap_err();
    refused_elsewhere(refused, None, None);
    let named = id_of(dir.path());
    let refused = reconcile(&mut store, &next).unwrap().unwrap_err();
    refused_elsewhere(refused, None, named);
    assert_eq!(written(&store), before);
    drop(next);

    let next = take_dispatch(&mut store).unwrap().unwrap();
    unnamed();
    let refused = reconcile(&mut store, &next).unwrap().unwrap_err();
    refused_elsewhere(refused, named, None);
    assert_eq!(written(&store), before);
}
