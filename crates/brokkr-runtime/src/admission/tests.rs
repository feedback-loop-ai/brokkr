//! Admission decides from the journal and the maps alone: a wait clears
//! when its condition holds and holds the entry when it never can, and a
//! realm whose governing facts moved since the entry was queued, either
//! way, holds it until the operator re-pins it.

use std::error::Error as _;
use std::ffi::OsString;
use std::path::Path;

use brokkr_core::realms::Boundary;
use brokkr_core::EventType;
use brokkr_store::{Attribution, NewEntry, QueueCommand};
use serde_json::{json, Value};

use super::*;
use crate::launch::{BundleSource, LaunchRequest, MapSource, NewRun, RunMap};
use crate::realms::World;

const BY: Attribution<'static> = Attribution {
    operator: "vy",
    reason: "operator's word",
};

/// A workspace that holds two library dialects and a house text.
fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let library = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dialects");
    for dialect in ["openspec", "speckit"] {
        let to = dir.path().join("dialects").join(dialect);
        std::fs::create_dir_all(&to).unwrap();
        let json = format!("{dialect}.json");
        std::fs::copy(library.join(&json), to.with_file_name(json)).unwrap();
        for text in std::fs::read_dir(library.join(dialect)).unwrap() {
            let text = text.unwrap().path();
            std::fs::copy(&text, to.join(text.file_name().unwrap())).unwrap();
        }
    }
    std::fs::write(dir.path().join("HOUSE.md"), "First rule.\n").unwrap();
    dir
}

/// Write the map: realm `b` at the workspace, with `facts` beside its
/// name, path and branch.
fn map(ws: &Path, facts: Value) {
    realms(ws, json!([realm("b", ".", facts)]));
}

/// The realm `name` at `path`, with `facts` beside its name, path and
/// branch.
fn realm(name: &str, path: &str, facts: Value) -> Value {
    let mut realm = json!({"name": name, "path": path, "default_branch": "main"});
    realm
        .as_object_mut()
        .unwrap()
        .extend(facts.as_object().unwrap().clone());
    realm
}

/// Write the map of `realms`.
fn realms(ws: &Path, realms: Value) {
    let map = json!({"schema": "forge.realms/v7", "journal": "forge.db", "realms": realms});
    std::fs::write(ws.join("realms.json"), map.to_string()).unwrap();
}

/// The launch `brokkr queue add` makes in `ws`, under its map or none.
fn launch(ws: &Path, mapped: bool) -> String {
    launch_in(ws, mapped, None)
}

/// The launch `brokkr queue add --repo <repo>` makes in `ws`.
fn launch_in(ws: &Path, mapped: bool, repo: Option<&str>) -> String {
    let map = match mapped {
        true => RunMap::Named(World::load(&ws.join("realms.json")).unwrap()),
        false => RunMap::Unmapped,
    };
    let request = LaunchRequest {
        workspace: ws.to_path_buf(),
        bundle: BundleSource::Dir("bundle".into()),
        journal: ws.join("forge.db"),
        repo: repo.map(Into::into),
        secrets: None,
        host_path: OsString::new(),
    };
    let run = NewRun {
        feature: "f".into(),
        map,
        dispatch: None,
    };
    QueuedLaunch::of(&request, &run).unwrap().encode().unwrap()
}

fn add(store: &mut Store, payload: &str, waits: &[Wait]) -> EntryId {
    let new = NewEntry {
        payload,
        priority: 0,
        waits,
    };
    store.queue_add(new, BY).unwrap()
}

/// Each entry the pass judged, by id, with its standing and reasons.
fn verdicts(store: &Store) -> Vec<(i64, Standing, Vec<Reason>)> {
    pass(store)
        .unwrap()
        .into_iter()
        .filter_map(|judged| {
            let verdict = judged.verdict?;
            Some((judged.entry.id.0, verdict.standing(), verdict.reasons))
        })
        .collect()
}

/// A journal in `ws`, holding one entry queued under the map as it is.
fn queued(ws: &Path) -> (Store, EntryId) {
    let mut store = Store::open(&ws.join("forge.db")).unwrap();
    let entry = add(&mut store, &launch(ws, true), &[]);
    (store, entry)
}

fn changed(differences: Vec<Difference>) -> Reason {
    Reason::RealmChanged {
        realm: "b".into(),
        differences,
    }
}

#[test]
fn a_realm_tightened_since_queued_holds_the_entry_naming_each_difference() {
    let ws = workspace();
    let grants = json!({"web-search": {"dialect": "web"}});
    map(
        ws.path(),
        json!({"boundary": "open", "capabilities": grants}),
    );
    let (store, entry) = queued(ws.path());
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Admissible, vec![])]
    );

    map(ws.path(), json!({"boundary": "harness"}));
    let held = changed(vec![
        Difference::GrantRemoved("web-search".into()),
        Difference::Boundary {
            was: Boundary::Open,
            now: Boundary::Harness,
        },
    ]);
    assert_eq!(
        held.to_string(),
        "realm b changed since queued: grant web-search removed; boundary open → harness"
    );
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Held, vec![held])]
    );
}

#[test]
fn a_realm_loosened_since_queued_holds_the_entry_too() {
    let ws = workspace();
    map(ws.path(), json!({}));
    let (store, entry) = queued(ws.path());
    let grants = json!({"web-fetch": {"dialect": "web"}});
    map(
        ws.path(),
        json!({"boundary": "open", "capabilities": grants}),
    );
    let held = changed(vec![
        Difference::GrantAdded("web-fetch".into()),
        Difference::Boundary {
            was: Boundary::Namespace,
            now: Boundary::Open,
        },
    ]);
    assert_eq!(
        held.to_string(),
        "realm b changed since queued: grant web-fetch added; boundary namespace → open"
    );
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Held, vec![held])]
    );
}

/// A grant is compared whole: two grants that differ only in a key this
/// module never names, as the retention veto realms v8 reserves (#531)
/// will be, are two grants, and the entry is held.
#[test]
fn a_grant_that_differs_in_a_key_no_field_list_names_holds_the_entry() {
    let ws = workspace();
    let kept = json!({"dialect": "web", "offices": ["implementer"]});
    let grants = |retain: bool| json!({"web": {"dialect": "web", "retain": retain}, "kept": kept});
    map(ws.path(), json!({"capabilities": grants(true)}));
    let (store, entry) = queued(ws.path());
    map(ws.path(), json!({"capabilities": grants(false)}));
    let held = changed(vec![Difference::GrantChanged("web".into())]);
    assert_eq!(
        held.to_string(),
        "realm b changed since queued: grant web changed"
    );
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Held, vec![held])]
    );
}

#[test]
fn a_realm_renamed_or_with_new_house_rules_or_dialect_holds_the_entry() {
    let ws = workspace();
    map(
        ws.path(),
        json!({"house": "HOUSE.md", "dialect": "openspec"}),
    );
    let (mut store, entry) = queued(ws.path());
    // The same texts, read again from a rewritten map, are the same facts.
    map(
        ws.path(),
        json!({"dialect": "openspec", "house": "./HOUSE.md"}),
    );
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Admissible, vec![])]
    );

    std::fs::write(ws.path().join("HOUSE.md"), "Second rule.\n").unwrap();
    map(
        ws.path(),
        json!({"name": "c", "house": "HOUSE.md", "dialect": "speckit"}),
    );
    let held = changed(vec![
        Difference::Realm {
            was: Some("b".into()),
            now: Some("c".into()),
        },
        Difference::House,
        Difference::Dialect,
    ]);
    assert_eq!(
        held.to_string(),
        "realm b changed since queued: the repository's realm b → c; house rules changed; \
         dialect changed"
    );
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Held, vec![held])]
    );

    // Queued where the map named no realm for it, and named since.
    store.queue_command(entry, QueueCommand::Drop, BY).unwrap();
    map(ws.path(), json!({"path": "elsewhere"}));
    let unnamed = add(&mut store, &launch(ws.path(), true), &[]);
    map(ws.path(), json!({}));
    let named = Reason::RealmChanged {
        realm: "b".into(),
        differences: vec![Difference::Realm {
            was: None,
            now: Some("b".into()),
        }],
    };
    assert_eq!(
        named.to_string(),
        "realm b changed since queued: the repository's realm none → b"
    );
    assert_eq!(
        verdicts(&store),
        vec![(unnamed.0, Standing::Held, vec![named])]
    );
}

/// An entry queued under no map reads the map `brokkr run` would find in
/// its workspace now, so it is held when that cannot be read too (the
/// operator's ruling of 2026-10-04), and admitted when there is none.
#[test]
fn a_map_that_cannot_be_read_now_holds_the_entry_queued_under_it_or_under_none() {
    let ws = workspace();
    map(ws.path(), json!({"house": "HOUSE.md"}));
    let (mut store, entry) = queued(ws.path());
    let unmapped = add(&mut store, &launch(ws.path(), false), &[]);
    std::fs::remove_file(ws.path().join("HOUSE.md")).unwrap();
    let house = ws.path().join("./HOUSE.md");
    let unreadable = |detail: String| {
        let reason = Reason::MapUnreadable(detail.clone());
        let said = format!("the realms map it was queued under cannot be read now: {detail}");
        assert_eq!(reason.to_string(), said);
        reason
    };
    let gone = unreadable(format!(
        "realm 'b' names house at {}, but it is not a readable file: No such file or directory \
         (os error 2)",
        house.display()
    ));
    let free = (unmapped.0, Standing::Admissible, vec![]);
    assert_eq!(
        verdicts(&store),
        vec![
            (entry.0, Standing::Held, vec![gone.clone()]),
            (unmapped.0, Standing::Held, vec![gone])
        ]
    );
    std::fs::remove_file(ws.path().join("realms.json")).unwrap();
    let missing = unreadable(format!(
        "no realms map at {}",
        ws.path().join("realms.json").display()
    ));
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Held, vec![missing]), free]
    );
}

/// For an entry queued under no map, only nothing at the map's path is
/// no map: a link that loops, a link that dangles and a directory there
/// are a map that cannot be read, and hold it unlatched until the path
/// is cleared, each beside that control (#430).
#[test]
fn a_map_path_that_cannot_be_resolved_holds_the_entry_queued_under_none() {
    let ws = workspace();
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let entry = add(&mut store, &launch(ws.path(), false), &[]);
    let path = ws.path().join("realms.json");
    let held = vec![(
        entry.0,
        Standing::Held,
        vec![Reason::MapUnreadable(format!(
            "no realms map at {}",
            path.display()
        ))],
    )];
    let free = vec![(entry.0, Standing::Admissible, vec![])];
    assert_eq!(verdicts(&store), free);
    /// What is planted at the map's path, how, and how it is cleared.
    type Plant = (&'static str, fn(&Path), fn(&Path));
    let link = |path: &Path| std::fs::remove_file(path).unwrap();
    let plants: [Plant; 3] = [
        (
            "a looping link",
            |path| std::os::unix::fs::symlink(path, path).unwrap(),
            link,
        ),
        (
            "a dangling link",
            |path| std::os::unix::fs::symlink(path.with_file_name("gone.json"), path).unwrap(),
            link,
        ),
        (
            "a directory",
            |path| std::fs::create_dir(path).unwrap(),
            |path| std::fs::remove_dir(path).unwrap(),
        ),
    ];
    for (plant, at, clear) in plants {
        at(&path);
        judge(&mut store, BY).unwrap();
        assert_eq!(verdicts(&store), held, "{plant}");
        assert_eq!(store.queue_list().unwrap()[0].latch, None, "{plant}");
        clear(&path);
        assert_eq!(verdicts(&store), free, "{plant} cleared");
    }
}

/// For an entry queued under no map, a map path that cannot even be
/// looked up (its workspace is no longer a directory) is no absence
/// either: only nothing at the path is no map, so the entry is held,
/// beside its control (#430).
#[test]
fn a_map_path_that_cannot_be_looked_up_holds_the_entry_queued_under_none() {
    let ws = workspace();
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let inner = ws.path().join("w");
    std::fs::create_dir(&inner).unwrap();
    let entry = add(&mut store, &launch(&inner, false), &[]);
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Admissible, vec![])]
    );
    std::fs::remove_dir(&inner).unwrap();
    std::fs::write(&inner, "").unwrap();
    assert_eq!(
        verdicts(&store),
        vec![(
            entry.0,
            Standing::Held,
            vec![Reason::MapUnreadable(format!(
                "no realms map at {}",
                inner.join("realms.json").display()
            ))],
        )]
    );
}

/// The operator's release of a drifted entry: once judged and latched,
/// re-pinned to the map on disk, it is admitted, and it starts under that
/// map.
#[test]
fn the_operators_repin_releases_a_drifted_entry_under_the_map_on_disk() {
    let ws = workspace();
    map(ws.path(), json!({"boundary": "open"}));
    let (mut store, entry) = queued(ws.path());
    let unmapped = add(&mut store, &launch(ws.path(), false), &[]);
    map(ws.path(), json!({"boundary": "harness"}));
    assert_eq!(verdicts(&store)[0].1, Standing::Held);

    // A drift that only a look has seen is no latch to release.
    let unlatched = release(&mut store, entry, BY).unwrap_err();
    assert!(matches!(unlatched, AdmissionError::NothingLatched(at) if at == entry));
    assert_eq!(
        unlatched.to_string(),
        "queue entry 1 holds no latched realm drift to release; `brokkr queue judge` latches \
         what it finds"
    );
    judge(&mut store, BY).unwrap();
    let released = release(&mut store, entry, BY).unwrap();
    let boundary = |was| Difference::Boundary {
        was,
        now: Boundary::Harness,
    };
    assert_eq!(released, Released(vec![boundary(Boundary::Open)]));
    assert_eq!(released.to_string(), "boundary open → harness");
    let judged = pass(&store).unwrap();
    assert_eq!(judged[0].verdict, Some(Verdict { reasons: vec![] }));
    let (_, run) = judged
        .into_iter()
        .next()
        .unwrap()
        .launch
        .rebuild(ws.path().join("forge.db"), OsString::new())
        .unwrap();
    let on_disk = World::load(&ws.path().join("realms.json")).unwrap();
    let RunMap::Named(world) = run.map else {
        panic!("not the named map")
    };
    assert_eq!(world.sha256, on_disk.sha256);

    // Queued under no map, it is released to the map that names its
    // repository now, as `brokkr run` would adopt it there.
    let released = release(&mut store, unmapped, BY).unwrap();
    assert_eq!(
        released.to_string(),
        "the repository's realm none → b; boundary namespace → harness"
    );
    let pinned = QueuedLaunch::decode(&store.queue_entry(unmapped).unwrap().payload).unwrap();
    assert!(matches!(pinned.map, MapSource::Ambient(_)));

    // A map that is gone is refused, not taken.
    map(ws.path(), json!({"boundary": "open"}));
    judge(&mut store, BY).unwrap();
    std::fs::remove_file(ws.path().join("realms.json")).unwrap();
    let error = release(&mut store, entry, BY).unwrap_err();
    let AdmissionError::Entry { entry: at, source } = &error else {
        panic!("not the entry's refusal: {error:?}")
    };
    assert_eq!(*at, entry);
    assert!(matches!(source, LaunchError::World(WorldError::Missing(_))));
}

/// The operator's ruling of 2026-10-04 (#430's H1 and L2): a drift once
/// judged LATCHES. The entry stays held when the map is put back as it
/// was queued (A → B → A), across a reopen, and until the operator
/// re-pins it; and the re-pin takes only the map the latch found.
#[test]
fn a_judged_drift_stays_latched_when_the_map_returns_and_a_repin_takes_only_the_map_judged() {
    let ws = workspace();
    map(ws.path(), json!({"boundary": "open"}));
    let (mut store, entry) = queued(ws.path());
    map(ws.path(), json!({"boundary": "harness"}));
    let drift = vec![Difference::Boundary {
        was: Boundary::Open,
        now: Boundary::Harness,
    }];
    let latched = |differences: &[Difference], moved| Reason::RealmLatched {
        realm: "b".into(),
        differences: differences.to_vec(),
        moved,
    };
    let held = |reasons| vec![(entry.0, Standing::Held, reasons)];
    let judged: Vec<Option<Verdict>> = judge(&mut store, BY)
        .unwrap()
        .into_iter()
        .map(|judged| judged.verdict)
        .collect();
    let reasons = vec![latched(&drift, false)];
    assert_eq!(judged, vec![Some(Verdict { reasons })]);
    let latch = "realm b changed since queued, latched until the operator re-pins, re-queues or \
                 drops it";
    assert_eq!(
        latched(&drift, false).to_string(),
        format!("{latch}: boundary open → harness")
    );

    // A: the map put back as it was queued holds the entry still.
    map(ws.path(), json!({"boundary": "open"}));
    assert_eq!(verdicts(&store), held(vec![latched(&drift, true)]));
    assert_eq!(
        latched(&drift, true).to_string(),
        format!(
            "{latch}: boundary open → harness; the map on disk has changed since, and `brokkr \
             queue judge` latches what it finds now"
        )
    );
    // The re-pin refuses a map it was not shown.
    map_moved(release(&mut store, entry, BY).unwrap_err(), entry);
    // A map that cannot be read beside the latch: both hold.
    std::fs::write(ws.path().join("realms.json"), "{").unwrap();
    let detail = World::load(&ws.path().join("realms.json"))
        .unwrap_err()
        .to_string();
    let unreadable = Reason::MapUnreadable(detail);
    assert_eq!(
        verdicts(&store),
        held(vec![latched(&drift, false), unreadable])
    );
    judge(&mut store, BY).unwrap();
    map(ws.path(), json!({"boundary": "open"}));

    // Judged again, the latch finds no difference now, and still holds,
    // across a reopen.
    judge(&mut store, BY).unwrap();
    drop(store);
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    assert_eq!(verdicts(&store), held(vec![latched(&[], false)]));
    assert_eq!(
        latched(&[], false).to_string(),
        format!("{latch}: no difference now")
    );
    let released = release(&mut store, entry, BY).unwrap();
    assert_eq!(
        released.to_string(),
        "no difference: the realm is as it was queued"
    );
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Admissible, vec![])]
    );
}

/// #430's H2, the chief's reproduction in one process: a peer's `queue
/// judge` between a re-pin's compare and its write is never cleared
/// unseen. Queued under A, judged under B, the re-pin compares B; a peer
/// judges C and the map goes back to B; the write is refused, naming C's
/// differences, and the entry stays latched on C. A peer's re-pin in
/// between, or a latch since that cannot be read, refuses it too.
#[test]
fn a_repin_refuses_a_latch_a_peer_wrote_after_it_compared() {
    let ws = workspace();
    map(ws.path(), json!({"boundary": "open"}));
    let (mut store, entry) = queued(ws.path());
    let queued_with = store.queue_entry(entry).unwrap().payload;
    map(ws.path(), json!({"boundary": "harness"}));
    judge(&mut store, BY).unwrap();
    let compared = shown(&store, entry).unwrap();
    map(ws.path(), json!({"boundary": "namespace"}));
    judge(&mut store, BY).unwrap();
    map(ws.path(), json!({"boundary": "harness"}));
    let error = repin(&mut store, entry, compared, BY).unwrap_err();
    let namespace = vec![Difference::Boundary {
        was: Boundary::Open,
        now: Boundary::Namespace,
    }];
    let AdmissionError::LatchMoved {
        entry: at,
        differences,
    } = &error
    else {
        panic!("not the moved latch's refusal: {error:?}")
    };
    assert_eq!((*at, differences), (entry, &namespace));
    assert_eq!(
        error.to_string(),
        "queue entry 1's latched hold changed before the re-pin was written, and nothing was \
         re-pinned: it now records boundary open → namespace; `brokkr queue repin` compares it \
         afresh"
    );
    let latched = Reason::RealmLatched {
        realm: "b".into(),
        differences: namespace,
        moved: true,
    };
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Held, vec![latched])]
    );
    assert_eq!(store.queue_entry(entry).unwrap().payload, queued_with);

    // A peer's re-pin between them: no latch stands to take.
    map(ws.path(), json!({"boundary": "namespace"}));
    let compared = shown(&store, entry).unwrap();
    release(&mut store, entry, BY).unwrap();
    let peers = store.queue_entry(entry).unwrap().payload;
    let error = repin(&mut store, entry, compared, BY).unwrap_err();
    assert!(matches!(error, AdmissionError::NothingLatched(at) if at == entry));
    assert_eq!(store.queue_entry(entry).unwrap().payload, peers);

    // A peer's latch this brokkr cannot read.
    map(ws.path(), json!({"boundary": "harness"}));
    judge(&mut store, BY).unwrap();
    let compared = shown(&store, entry).unwrap();
    let read = store.queue_entry(entry).unwrap().seen();
    store.queue_latch(entry, "{}", read, BY).unwrap();
    let error = repin(&mut store, entry, compared, BY).unwrap_err();
    assert!(matches!(error, AdmissionError::Latch { entry: at, .. } if at == entry));
    assert_eq!(store.queue_entry(entry).unwrap().payload, peers);
}

/// #430's H3, the chief's reproduction in one process: a `queue judge`
/// never latches a finding measured against a pin the operator has since
/// replaced. Queued under A (open, web-search) and judged under B
/// (harness, no grant), a judge measures the map STALE against the pin A;
/// the operator re-pins to B; with the map at STALE again the measured
/// finding is refused, nothing is latched, and judged afresh the latch
/// and the re-pin say what the pin B differs by. STALE is C (namespace,
/// web-search), then A itself, whose stale finding records no difference.
#[test]
fn a_judge_never_latches_a_finding_measured_against_a_pin_since_replaced() {
    let grant = json!({"web-search": {"dialect": "web"}});
    let a = json!({"boundary": "open", "capabilities": grant});
    let b = json!({"boundary": "harness"});
    let c = json!({"boundary": "namespace", "capabilities": grant});
    let boundary = |was, now| Difference::Boundary { was, now };
    let variants = [
        (
            c,
            vec![boundary(Boundary::Open, Boundary::Namespace)],
            "grant web-search added; boundary harness → namespace",
        ),
        (
            a.clone(),
            vec![],
            "grant web-search added; boundary harness → open",
        ),
    ];
    for (stale, measured_as, afresh) in variants {
        let ws = workspace();
        map(ws.path(), a.clone());
        let (mut store, entry) = queued(ws.path());
        map(ws.path(), b.clone());
        judge(&mut store, BY).unwrap();
        map(ws.path(), stale.clone());
        let measured = read(&store).unwrap();
        let finding = measured[0].1.as_ref().unwrap();
        assert_eq!(finding.differences, measured_as);
        map(ws.path(), b.clone());
        let accepted = release(&mut store, entry, BY).unwrap();
        assert_eq!(
            accepted.to_string(),
            "grant web-search removed; boundary open → harness"
        );
        let peers = store.queue_entry(entry).unwrap();
        map(ws.path(), stale);
        let error = latch(&mut store, measured, BY).unwrap_err();
        assert!(matches!(error, AdmissionError::Unmeasured(at) if at == entry));
        assert_eq!(
            error.to_string(),
            "queue entry 1 was re-pinned or latched while `brokkr queue judge` measured it, and \
             what it found was not latched; `brokkr queue judge` measures it afresh"
        );
        assert_eq!(store.queue_entry(entry).unwrap(), peers);
        judge(&mut store, BY).unwrap();
        let released = release(&mut store, entry, BY).unwrap();
        assert_eq!(released.to_string(), afresh);
    }

    // Measured before any latch, and a peer judged and re-pinned since:
    // only the pin moved, and the finding is refused all the same.
    let ws = workspace();
    map(ws.path(), a);
    let (mut store, entry) = queued(ws.path());
    map(ws.path(), b);
    let measured = read(&store).unwrap();
    judge(&mut store, BY).unwrap();
    release(&mut store, entry, BY).unwrap();
    let peers = store.queue_entry(entry).unwrap();
    let error = latch(&mut store, measured, BY).unwrap_err();
    assert!(matches!(error, AdmissionError::Unmeasured(at) if at == entry));
    assert_eq!(store.queue_entry(entry).unwrap(), peers);
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Admissible, vec![])]
    );

    // Measured, and dropped by a peer before the latch: the store's own
    // refusal stands.
    map(ws.path(), json!({"boundary": "open"}));
    let measured = read(&store).unwrap();
    store.queue_command(entry, QueueCommand::Drop, BY).unwrap();
    let error = latch(&mut store, measured, BY).unwrap_err();
    assert!(matches!(
        error,
        AdmissionError::Store(StoreError::Queue(QueueRefusal::Dropped(at))) if at == entry
    ));
}

/// The operator's ruling of 2026-10-04 (#430's M1): an entry queued under
/// no map is compared like any other, and a map that names its repository
/// since is a change of the facts that govern it.
#[test]
fn an_entry_queued_under_no_map_is_held_when_a_map_names_its_repository() {
    let ws = workspace();
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let entry = add(&mut store, &launch(ws.path(), false), &[]);
    let admissible = vec![(entry.0, Standing::Admissible, vec![])];
    assert_eq!(verdicts(&store), admissible);
    let grants = json!({"web-search": {"dialect": "web"}});
    map(
        ws.path(),
        json!({"boundary": "open", "capabilities": grants, "house": "HOUSE.md"}),
    );
    let held = changed(vec![
        Difference::Realm {
            was: None,
            now: Some("b".into()),
        },
        Difference::GrantAdded("web-search".into()),
        Difference::Boundary {
            was: Boundary::Namespace,
            now: Boundary::Open,
        },
        Difference::House,
    ]);
    assert_eq!(
        held.to_string(),
        "realm b changed since queued: the repository's realm none → b; grant web-search added; \
         boundary namespace → open; house rules changed"
    );
    assert_eq!(
        verdicts(&store),
        vec![(entry.0, Standing::Held, vec![held])]
    );
}

/// An entry queued under no map, held and latched because a map came to
/// name its repository, is not re-pinned once that map is gone again: the
/// re-pin would take no map, which is not the map the latch found (#430).
#[test]
fn an_unmapped_entry_is_not_repinned_once_the_map_that_held_it_is_gone() {
    let ws = workspace();
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let entry = add(&mut store, &launch(ws.path(), false), &[]);
    map(ws.path(), json!({"boundary": "open"}));
    judge(&mut store, BY).unwrap();
    std::fs::remove_file(ws.path().join("realms.json")).unwrap();
    map_moved(release(&mut store, entry, BY).unwrap_err(), entry);
}

/// `moved` is the re-pin refused because the map on disk is not the one
/// `entry`'s latch found, in the operator's words.
fn map_moved(moved: AdmissionError, entry: EntryId) {
    assert!(matches!(moved, AdmissionError::MapMoved(at) if at == entry));
    assert_eq!(
        moved.to_string(),
        format!(
            "the realms map on disk is not the one queue entry {}'s latched hold found; `brokkr \
             queue judge` shows and latches what differs now",
            entry.0
        )
    );
}

/// The facts of a realm open with web-search.
fn web() -> Value {
    json!({"boundary": "open", "capabilities": {"web-search": {"dialect": "web"}}})
}

/// The trees `a` and `b` in `ws`.
fn trees(ws: &Path) {
    for tree in ["a", "b"] {
        std::fs::create_dir(ws.join(tree)).unwrap();
    }
}

/// Point the symlink `name` in `ws` at `to`, which need not exist, as an
/// operator retargets a `current`-style alias.
fn alias(ws: &Path, name: &str, to: &str) {
    let link = ws.join(name);
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(ws.join(to), link).unwrap();
}

/// Write the map of `ra` at `a`, with `facts`, and `rb` at `b`, open with
/// web-search.
fn two(ws: &Path, facts: Value) {
    realms(
        ws,
        json!([realm("ra", "a", facts), realm("rb", "b", web())]),
    );
}

/// A workspace whose map holds `ra` at `a` under `harness` with no grant,
/// beside `rb`, and one entry queued with `--repo selected`, an alias of
/// `a`: queued in `ra`, and admissible.
fn aliased() -> (tempfile::TempDir, Store, EntryId) {
    let ws = workspace();
    trees(ws.path());
    two(ws.path(), json!({"boundary": "harness"}));
    alias(ws.path(), "selected", "a");
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let entry = add(
        &mut store,
        &launch_in(ws.path(), true, Some("selected")),
        &[],
    );
    let admissible = vec![(entry.0, Standing::Admissible, vec![])];
    assert_eq!(verdicts(&store), admissible);
    (ws, store, entry)
}

/// The drift `pass` shows on `entry`, the queue's one waiting entry, in
/// `realm`: `judge` latches it as found, and the operator's re-pin accepts
/// it, saying `said`, and admits the entry.
fn drifts(store: &mut Store, entry: EntryId, realm: &str, found: Vec<Difference>, said: &str) {
    let changed = Reason::RealmChanged {
        realm: realm.into(),
        differences: found.clone(),
    };
    let text = format!("realm {realm} changed since queued: {said}");
    assert_eq!(changed.to_string(), text);
    assert_eq!(
        verdicts(store),
        vec![(entry.0, Standing::Held, vec![changed])]
    );
    judge(store, BY).unwrap();
    let latched = Reason::RealmLatched {
        realm: realm.into(),
        differences: found.clone(),
        moved: false,
    };
    assert_eq!(
        verdicts(store),
        vec![(entry.0, Standing::Held, vec![latched])]
    );
    let released = release(store, entry, BY).unwrap();
    assert_eq!(
        (released.to_string(), released.0),
        (said.to_string(), found)
    );
    assert_eq!(
        verdicts(store),
        vec![(entry.0, Standing::Admissible, vec![])]
    );
}

/// A change to a workspace, the drift it makes and how the operator
/// reads it.
type Variant = (fn(&Path), Vec<Difference>, &'static str);

/// #430's H4, the chief's reproduction: the realm an entry was queued in
/// is the one its pin selected, never the one its repository's path
/// resolves to now. Queued with `--repo selected`, an alias of `a`, in
/// `ra` (harness, no grant), the alias retargeted to `b`, or left
/// dangling, under a map that has not changed holds the entry as the
/// control does: the map giving `ra` what `rb` has, the alias untouched.
#[test]
fn a_repository_alias_retargeted_or_dangling_since_queued_holds_the_entry() {
    let from_harness = |now| Difference::Boundary {
        was: Boundary::Harness,
        now,
    };
    let moved = |now: Option<&str>| Difference::Realm {
        was: Some("ra".into()),
        now: now.map(Into::into),
    };
    let added = Difference::GrantAdded("web-search".into());
    let variants: [Variant; 3] = [
        (
            |ws| alias(ws, "selected", "b"),
            vec![
                moved(Some("rb")),
                added.clone(),
                from_harness(Boundary::Open),
            ],
            "the repository's realm ra → rb; grant web-search added; boundary harness → open",
        ),
        (
            |ws| alias(ws, "selected", "gone"),
            vec![moved(None), from_harness(Boundary::Namespace)],
            "the repository's realm ra → none; boundary harness → namespace",
        ),
        (
            |ws| two(ws, web()),
            vec![added, from_harness(Boundary::Open)],
            "grant web-search added; boundary harness → open",
        ),
    ];
    for (change, found, said) in variants {
        let (ws, mut store, entry) = aliased();
        change(ws.path());
        drifts(&mut store, entry, "ra", found, said);
    }
}

/// #430's H4, the chief's second variant, with no alias in `--repo`: the
/// map's own realm `live` is at `current`, an alias of `a`, so an entry
/// queued with `--repo b` is in no realm; `current` retargeted to `b`
/// under a map that has not changed names the entry's repository `live`,
/// open with web-search, and holds it.
#[test]
fn a_realm_path_retargeted_to_the_repository_an_entry_operates_holds_it() {
    let ws = workspace();
    trees(ws.path());
    alias(ws.path(), "current", "a");
    realms(ws.path(), json!([realm("live", "current", web())]));
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let entry = add(&mut store, &launch_in(ws.path(), true, Some("b")), &[]);
    let admissible = vec![(entry.0, Standing::Admissible, vec![])];
    assert_eq!(verdicts(&store), admissible);
    alias(ws.path(), "current", "b");
    let found = vec![
        Difference::Realm {
            was: None,
            now: Some("live".into()),
        },
        Difference::GrantAdded("web-search".into()),
        Difference::Boundary {
            was: Boundary::Namespace,
            now: Boundary::Open,
        },
    ];
    let said = "the repository's realm none → live; grant web-search added; boundary namespace → \
                open";
    drifts(&mut store, entry, "live", found, said);
}

/// #430's H6: the realm the map on disk names an entry's repository is
/// resolved once, and the digest a re-pin would write and the differences
/// are both read off it. Queued in `ra` through `selected`, the alias
/// retargeted to `b`, and put back to `a` the instant the resolution has
/// read it: the finding is `rb`'s, digest and differences alike, never
/// `rb`'s digest beside no difference; and the re-pin under the map as it
/// stands then is refused.
#[test]
fn a_repository_retargeted_mid_judgement_is_judged_on_one_selection() {
    let (ws, mut store, entry) = aliased();
    alias(ws.path(), "selected", "b");
    let queued = store.queue_entry(entry).unwrap();
    let launch = QueuedLaunch::decode(&queued.payload).unwrap();
    let in_b = launch.clone().repinned().unwrap().map.digest();
    let worlds = launch
        .held_and_now_with(|world, repo| {
            let selected = world.realm_for(repo);
            alias(ws.path(), "selected", "a");
            selected
        })
        .unwrap();
    let finding = seen(worlds).unwrap();
    let expected = Finding {
        encoding: FindingEncoding::V1,
        realm: "ra".into(),
        differences: vec![
            Difference::Realm {
                was: Some("ra".into()),
                now: Some("rb".into()),
            },
            Difference::GrantAdded("web-search".into()),
            Difference::Boundary {
                was: Boundary::Harness,
                now: Boundary::Open,
            },
        ],
        on_disk: in_b,
    };
    assert_eq!(finding, expected);
    let finding = json!(finding).to_string();
    store
        .queue_latch(entry, &finding, queued.seen(), BY)
        .unwrap();
    let refused = release(&mut store, entry, BY).unwrap_err();
    assert!(matches!(refused, AdmissionError::MapMoved(at) if at == entry));
}

/// A held pin is read for the realm it selected, so one whose selection
/// cannot be read, or names a realm its map does not hold, is refused.
#[test]
fn a_held_pin_selecting_no_realm_its_map_holds_is_refused() {
    let ws = workspace();
    map(ws.path(), json!({}));
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let payload = launch(ws.path(), true);
    let forgeries = [
        ("\"ghost\"", "ghost is not a realm its map holds"),
        (
            "7",
            "cannot be read: invalid type: integer `7`, expected a string",
        ),
    ];
    for (selects, said) in forgeries {
        let forged = payload.replace("\"realm\":\"b\"", &format!("\"realm\":{selects}"));
        let forged = add(&mut store, &forged, &[]);
        let error = pass(&store).unwrap_err();
        assert!(matches!(error, AdmissionError::Entry { entry, .. } if entry == forged));
        assert_eq!(
            error.source().unwrap().to_string(),
            format!("this run's pinned realms map is unreadable: its selected realm {said}")
        );
        store.queue_command(forged, QueueCommand::Drop, BY).unwrap();
    }
}

/// A run in the journal that reads `events` past its start.
fn run(store: &mut Store, name: &str, events: &[(EventType, Value)]) {
    let manifest = json!({"schema": "run-manifest/v1"});
    store.create_run(name, "f", "b", &manifest).unwrap();
    let started = (EventType::RunStarted, json!({"feature": "f"}));
    for (event, payload) in std::iter::once(&started).chain(events) {
        store
            .append_next(name, *event, payload.clone(), None, None)
            .unwrap();
    }
}

#[test]
fn a_wait_clears_when_its_condition_holds_and_holds_the_entry_when_it_never_can() {
    let ws = workspace();
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let unmapped = launch(ws.path(), false);
    // Entries 1 to 6: unstarted, dropped, running, parked, completed and
    // stopped.
    let awaited: Vec<EntryId> = (0..6).map(|_| add(&mut store, &unmapped, &[])).collect();
    store
        .queue_command(awaited[1], QueueCommand::Drop, BY)
        .unwrap();
    let parked = (EventType::RunParked, json!({"reason": "lost"}));
    let ends = [
        ("r3", vec![]),
        ("r4", vec![parked]),
        ("r5", vec![(EventType::RunCompleted, json!({}))]),
        ("r6", vec![(EventType::RunStopped, json!({}))]),
    ];
    for ((name, events), entry) in ends.into_iter().zip(&awaited[2..]) {
        run(&mut store, name, &events);
        store.queue_claim(*entry, name).unwrap();
    }
    let mut dependents = Vec::new();
    for entry in &awaited {
        for on in WaitOn::ALL {
            let wait = Wait { entry: *entry, on };
            dependents.push((add(&mut store, &unmapped, &[wait]), wait));
        }
    }
    let (first, _) = dependents[0];
    store.queue_command(first, QueueCommand::Hold, BY).unwrap();

    let judged: Vec<(Standing, Vec<Reason>)> = verdicts(&store)
        .into_iter()
        .filter(|(id, ..)| *id > 6)
        .map(|(_, standing, reasons)| (standing, reasons))
        .collect();
    let wait = |n: usize| dependents[n].1;
    let unended = |n: usize, run: &str| Reason::Unended {
        wait: wait(n),
        run: run.into(),
    };
    let stopped = Reason::Stopped {
        wait: wait(10),
        run: "r6".into(),
    };
    let expected = vec![
        vec![Reason::OperatorHold, Reason::NotStarted(wait(0))],
        vec![Reason::NotStarted(wait(1))],
        vec![Reason::Dropped(wait(2))],
        vec![Reason::Dropped(wait(3))],
        vec![unended(4, "r3")],
        vec![unended(5, "r3")],
        vec![unended(6, "r4")],
        vec![unended(7, "r4")],
        vec![],
        vec![],
        vec![stopped],
        vec![],
    ];
    let reasons: Vec<&Vec<Reason>> = judged.iter().map(|(_, reasons)| reasons).collect();
    assert_eq!(reasons, expected.iter().collect::<Vec<_>>());
    let standings = [1, 2, 8].map(|n| judged[n].0);
    assert_eq!(
        standings,
        [Standing::Waiting, Standing::Held, Standing::Admissible]
    );
}

#[test]
fn every_reason_and_standing_has_its_word() {
    let wait = |entry, on| Wait {
        entry: EntryId(entry),
        on,
    };
    let reasons = [
        Reason::OperatorHold,
        Reason::NotStarted(wait(1, WaitOn::Completed)),
        Reason::Dropped(wait(2, WaitOn::Ended)),
        Reason::Unended {
            wait: wait(3, WaitOn::Completed),
            run: "r3".into(),
        },
        Reason::Stopped {
            wait: wait(6, WaitOn::Completed),
            run: "r6".into(),
        },
        changed(vec![]),
        Reason::RealmLatched {
            realm: "b".into(),
            differences: vec![Difference::House],
            moved: false,
        },
        Reason::MapUnreadable("x".into()),
    ];
    let said = reasons.map(|reason| (reason.to_string(), reason.kind(), reason.holds()));
    let never = "which can never hold";
    let words = [
        ("held by the operator", "operator_hold", true),
        (
            "waits on 1:completed: entry 1 has not started a run",
            "not_started",
            false,
        ),
        (
            &format!("waits on 2:ended, {never}: entry 2 was dropped"),
            "dropped",
            true,
        ),
        (
            "waits on 3:completed: its run 'r3' has not ended",
            "unended",
            false,
        ),
        (
            &format!("waits on 6:completed, {never}: its run 'r6' stopped"),
            "stopped",
            true,
        ),
        ("realm b changed since queued: ", "realm_changed", true),
        (
            "realm b changed since queued, latched until the operator re-pins, re-queues or \
             drops it: house rules changed",
            "realm_latched",
            true,
        ),
        (
            "the realms map it was queued under cannot be read now: x",
            "map_unreadable",
            true,
        ),
    ];
    assert_eq!(
        said,
        words.map(|(says, kind, holds)| (says.to_string(), kind, holds))
    );
    assert_eq!(
        [Standing::Admissible, Standing::Waiting, Standing::Held].map(Standing::word),
        ["admissible", "waiting", "held"]
    );
}

#[test]
fn a_queue_the_pass_cannot_read_admits_nothing_and_says_why() {
    let ws = workspace();
    let mut store = Store::open(&ws.path().join("forge.db")).unwrap();
    let foreign = add(&mut store, "{}", &[]);
    let error = pass(&store).unwrap_err();
    assert!(matches!(error, AdmissionError::Entry { entry, .. } if entry == foreign));
    let chain = (error.to_string(), error.source().unwrap().to_string());
    assert_eq!(
        chain,
        ("queue entry 1".into(), "reading a queued launch".into())
    );
    store
        .queue_command(foreign, QueueCommand::Drop, BY)
        .unwrap();

    // A held world that does not answer for itself.
    map(ws.path(), json!({}));
    let payload = launch(ws.path(), true);
    let digest = World::load(&ws.path().join("realms.json")).unwrap().sha256;
    let forged = add(&mut store, &payload.replace(&digest, &"0".repeat(64)), &[]);
    let error = pass(&store).unwrap_err();
    assert!(matches!(error, AdmissionError::Entry { entry, .. } if entry == forged));
    assert!(error
        .source()
        .unwrap()
        .to_string()
        .starts_with("this run's pinned realms map is unreadable: the embedded map hashes to "));
    store.queue_command(forged, QueueCommand::Drop, BY).unwrap();

    // A latch this brokkr cannot read, which no release takes either.
    let latched = add(&mut store, &payload, &[]);
    store
        .queue_latch(latched, "{}", Seen::default(), BY)
        .unwrap();
    let unread = |error: AdmissionError| {
        assert!(matches!(error, AdmissionError::Latch { entry, .. } if entry == latched));
        let chain = (error.to_string(), error.source().unwrap().to_string());
        assert_eq!(
            chain,
            (
                "queue entry 3's latched hold cannot be read".into(),
                "missing field `encoding` at line 1 column 2".into()
            )
        );
    };
    unread(pass(&store).unwrap_err());
    unread(release(&mut store, latched, BY).unwrap_err());
    store
        .queue_command(latched, QueueCommand::Drop, BY)
        .unwrap();

    // An awaited run whose journal does not fold.
    let unmapped = launch(ws.path(), false);
    let awaited = add(&mut store, &unmapped, &[]);
    store
        .create_run("r0", "f", "b", &json!({"schema": "run-manifest/v1"}))
        .unwrap();
    store.queue_claim(awaited, "r0").unwrap();
    let wait = Wait {
        entry: awaited,
        on: WaitOn::Ended,
    };
    add(&mut store, &unmapped, &[wait]);
    let error = pass(&store).unwrap_err();
    assert!(matches!(&error, AdmissionError::Fold { run, .. } if run == "r0"));
    assert_eq!(error.to_string(), "folding run 'r0'");
}
