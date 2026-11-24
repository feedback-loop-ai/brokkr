//! Decision 0125's rule, judged over planted journals with the machine's
//! availability and each driver's use injected: never the machine's own
//! numbers, its journals or its realms map.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use brokkr_core::EventType;
use brokkr_store::Store;
use serde_json::{json, Value};

use super::*;
use crate::admission::capacity::tests::{ceilings, seated, Fixture, PLENTY, TEMP};
use crate::admission::capacity::Capacity;
use crate::admission::{pass_within, Judged, Reason};

const GIB: u64 = 1 << 30;

/// A machine of 16 GiB and 8 cores, with `memory` bytes and `idle`
/// millicores available.
fn available(memory: u64, idle: u64) -> Available {
    Available {
        memory_bytes: memory,
        memory_total: 16 * GIB,
        idle_millicores: idle,
        cores_millicores: 8000,
    }
}

/// One measured attempt's usage: its peak memory and load.
fn usage(memory: u64, load: u64) -> Value {
    json!({"peak_memory_bytes": memory, "mean_memory_bytes": 1, "peak_load_millicores": load,
           "mean_load_millicores": 1, "cpu_ms": 1, "wall_ms": 1})
}

/// Start `run` of `recipe` in `store`, seating alpha's one lane, and enter
/// each of `phases`, an attempt measured at its peaks ending each; `ended`
/// completes it.
fn run(store: &mut Store, run: &str, recipe: &str, phases: &[(&str, u64, u64)], ended: bool) {
    store
        .create_run(run, "f", recipe, &seated(&[("alpha", "a-1")], false))
        .unwrap();
    let mut append = |kind: EventType, payload: Value| {
        store.append_next(run, kind, payload, None, None).unwrap();
    };
    append(EventType::RunStarted, json!({"feature": "f"}));
    for (at, (phase, memory, load)) in phases.iter().enumerate() {
        append(EventType::PhaseEntered, json!({"phase": phase}));
        if *memory > 0 {
            let id = format!("effect-{at}");
            append(
                EventType::EffectRequested,
                json!({"effect_id": id, "phase": phase, "seat": phase}),
            );
            append(
                EventType::EffectStarted,
                json!({"effect_id": id, "attempt_id": id}),
            );
            append(
                EventType::EffectSucceeded,
                json!({"effect_id": id, "attempt_id": id, "result": {"result": "complete"},
                       "usage": usage(*memory, *load)}),
            );
            if let Some((next, _, _)) = phases.get(at + 1) {
                append(
                    EventType::TransitionDecided,
                    json!({"from": phase, "result": "complete", "next": next,
                           "rule_id": "FIXTURE", "severity": null, "inputs": {}, "problem": null}),
                );
            }
        }
    }
    if ended {
        append(EventType::RunCompleted, json!({}));
    }
}

/// A journal of its own under `dir`, holding what `fill` writes, closed.
fn journal(dir: &Path, name: &str, fill: impl FnOnce(&mut Store)) -> PathBuf {
    let path = dir.join(name);
    let mut store = Store::open(&path).unwrap();
    fill(&mut store);
    path
}

/// The pass over `fixture`'s queue on a v4 host of plenty, measured as
/// `machine` with each run's driver using what `uses` names; the journals
/// beside its own are `journals`.
fn judged(
    fixture: &Fixture,
    host: &Value,
    journals: &[PathBuf],
    measured: Measurement,
    uses: &BTreeMap<&str, Use>,
) -> (Vec<Judged>, Vec<Reason>) {
    let file = fixture.declare(host);
    let measure = |driven: &[Driven<'_>]| match &measured {
        Measurement::Measured { machine, .. } => Measurement::Measured {
            machine: *machine,
            runs: driven
                .iter()
                .map(|run| uses.get(run.run).copied())
                .collect(),
        },
        unmeasured => unmeasured.clone(),
    };
    let machine = Machine::with(journals, &measure);
    let probe = |_: &Path| Ok(PLENTY);
    let host = Host {
        file: &file,
        temp: TEMP.as_ref(),
        free: &probe,
        machine: Some(&machine),
    };
    pass_within(&fixture.store, &host).unwrap()
}

/// Each judged entry's reasons, as kinds and words.
fn said(judged: &[Judged]) -> Vec<Vec<(&'static str, String)>> {
    (judged.iter())
        .filter_map(|judged| judged.verdict.as_ref())
        .map(|verdict| {
            (verdict.reasons.iter())
                .map(|reason| (reason.kind(), reason.to_string()))
                .collect()
        })
        .collect()
}

fn plenty() -> Value {
    ceilings(9, 9, json!({}))
}

fn declared(memory: u64, load: u64) -> Value {
    let mut host = plenty();
    host["schema"] = json!("forge.host/v5");
    host["demand"] = json!({"recipes": {"fixture": {
        "peak_memory_bytes": memory, "peak_load_millicores": load
    }}});
    host
}

fn measured(memory: u64, idle: u64) -> Measurement {
    Measurement::Measured {
        machine: available(memory, idle),
        runs: Vec::new(),
    }
}

/// A registry of one journal beside the pass's own, holding `fill`'s runs.
struct Registry {
    _dir: tempfile::TempDir,
    journals: Vec<PathBuf>,
}

fn registry(fill: impl FnOnce(&mut Store)) -> Registry {
    let dir = tempfile::tempdir().unwrap();
    let journals = vec![journal(dir.path(), "history.db", fill)];
    Registry {
        _dir: dir,
        journals,
    }
}

/// The fixture's recipe, measured twice in another journal of the
/// registry: implement at 2 and 4 GiB and 1000 and 3000 millicores, so
/// 3 GiB and 2000 on average, and 3.75 GiB and 2500 with the margin.
fn measured_fixture() -> (Fixture, Registry) {
    let registry = registry(|store| {
        for (name, memory, load) in [("h1", 2 * GIB, 1000), ("h2", 4 * GIB, 3000)] {
            run(store, name, "fixture", &[("implement", memory, load)], true);
        }
    });
    (Fixture::new(), registry)
}

#[test]
fn an_entry_whose_memory_demand_does_not_fit_waits_naming_the_numbers_and_one_that_fits_starts() {
    let (mut fixture, registry) = measured_fixture();
    let registry = &registry.journals;
    fixture.queue();
    let none = BTreeMap::new();
    let short = judged(
        &fixture,
        &plenty(),
        registry,
        measured(3 * GIB, 8000),
        &none,
    )
    .0;
    assert_eq!(
        said(&short),
        vec![vec![(
            "memory_short",
            "memory is short: recipe fixture needs 4026531840 bytes (3221225472 average over \
             the average of its last 2 measured runs + 805306368 margin), 3221225472 bytes \
             available, 0 bytes reserved by 0 running runs"
                .to_string()
        )]]
    );
    let fits = judged(
        &fixture,
        &plenty(),
        registry,
        measured(4 * GIB, 8000),
        &none,
    )
    .0;
    assert_eq!(said(&fits), vec![vec![]]);
    assert!(fits[0].room.is_some());
    assert_eq!(
        fits[0].demand.as_ref().map(|sized| sized.memory_bytes),
        Some(Need {
            average: 3 * GIB,
            margin: 768 << 20,
            demand: 3 * GIB + (768 << 20)
        })
    );
}

#[test]
fn an_entry_whose_load_does_not_fit_the_idle_cores_waits_as_cpu_short() {
    let (mut fixture, registry) = measured_fixture();
    let registry = &registry.journals;
    fixture.queue();
    let none = BTreeMap::new();
    let short = judged(
        &fixture,
        &plenty(),
        registry,
        measured(8 * GIB, 2499),
        &none,
    )
    .0;
    assert_eq!(
        said(&short),
        vec![vec![(
            "cpu_short",
            "cpu is short: recipe fixture needs 2500 millicores (2000 average over the average \
             of its last 2 measured runs + 500 margin), 2499 millicores available, 0 \
             millicores reserved by 0 running runs"
                .to_string()
        )]]
    );
    let fits = judged(
        &fixture,
        &plenty(),
        registry,
        measured(8 * GIB, 2500),
        &none,
    )
    .0;
    assert_eq!(said(&fits), vec![vec![]]);
}

/// Two entries and room for one: the first admitted reserves its demand
/// before the second is judged, so the second waits.
#[test]
fn a_second_entry_in_one_pass_sees_the_first_ones_reservation() {
    let (mut fixture, registry) = measured_fixture();
    fixture.queue();
    fixture.queue();
    let none = BTreeMap::new();
    let judged = judged(
        &fixture,
        &plenty(),
        &registry.journals,
        measured(6 * GIB, 8000),
        &none,
    )
    .0;
    let kinds: Vec<Vec<&str>> = said(&judged)
        .into_iter()
        .map(|reasons| reasons.into_iter().map(|(kind, _)| kind).collect())
        .collect();
    assert_eq!(kinds, vec![vec![], vec!["memory_short"]]);
    let reserved = judged[1].demand.as_ref().unwrap().reserved;
    assert_eq!(
        reserved,
        Reserved {
            memory_bytes: 3 * GIB + (768 << 20),
            load_millicores: 2500,
            runs: 0,
            whole: 0,
            admitted: 1
        }
    );
    assert!(judged[0].room.is_some() && judged[1].room.is_none());
}

/// A run in implement still holds its verify build: it reserves verify's
/// peak less what its tree uses now; one whose tree cannot be read
/// reserves the whole peak, and the reason says so.
#[test]
fn a_running_run_in_implement_reserves_its_verify_peak_beyond_its_measured_use() {
    let mut fixture = Fixture::new();
    let phases = [("implement", GIB, 1000), ("verify", 8 * GIB, 6000)];
    let registry = registry(|store| run(store, "h1", "fixture", &phases, true));
    let registry = &registry.journals;
    run(
        &mut fixture.store,
        "live",
        "fixture",
        &[("implement", 0, 0)],
        false,
    );
    fixture.queue();
    let using = Use {
        memory_bytes: GIB,
        load_millicores: 1000,
    };
    let uses = BTreeMap::from([("live", using)]);
    let (judged, _) = judged(
        &fixture,
        &plenty(),
        registry,
        measured(12 * GIB, 8000),
        &uses,
    );
    let reserved = judged[0].demand.as_ref().unwrap().reserved;
    assert_eq!(
        (
            reserved.memory_bytes,
            reserved.load_millicores,
            reserved.runs,
            reserved.whole
        ),
        (7 * GIB, 5000, 1, 0)
    );
    assert_eq!(said(&judged)[0][0].0, "memory_short");
    let (unread, _) = self::judged(
        &fixture,
        &plenty(),
        registry,
        measured(16 * GIB, 8000),
        &BTreeMap::new(),
    );
    let reserved = unread[0].demand.as_ref().unwrap().reserved;
    assert_eq!((reserved.memory_bytes, reserved.whole), (8 * GIB, 1));
    assert!(
        said(&unread)[0][0]
            .1
            .ends_with("reserved by 1 running runs, 1 of them unmeasured and reserved whole"),
        "{:?}",
        said(&unread)
    );
}

#[test]
fn a_queued_run_before_its_first_phase_reserves_its_whole_peak() {
    let mut fixture = Fixture::new();
    let registry = registry(|store| {
        run(store, "starting", "fixture", &[], false);
        run(store, "live", "fixture", &[("implement", 0, 0)], false);
    });
    fixture.queue();
    let uses = BTreeMap::from([
        (
            "starting",
            Use {
                memory_bytes: 6 * GIB,
                load_millicores: 2000,
            },
        ),
        (
            "live",
            Use {
                memory_bytes: 3 * GIB,
                load_millicores: 500,
            },
        ),
    ]);
    let (entries, _) = judged(
        &fixture,
        &declared(4 * GIB, 1000),
        &registry.journals,
        measured(9 * GIB, 8000),
        &uses,
    );
    assert_eq!(
        entries[0].demand.as_ref().unwrap().reserved,
        Reserved {
            memory_bytes: 5 * GIB,
            load_millicores: 1500,
            runs: 2,
            whole: 1,
            admitted: 0,
        }
    );
    assert_eq!(said(&entries)[0][0].0, "memory_short");
}

/// History read from two journals of the registry and the pass's own,
/// newest first; a third that cannot be read is named once and sizes
/// nothing; one the map lists that is not there is passed over silently.
#[test]
fn history_is_read_across_the_registry_journals_and_an_unreadable_one_is_named_and_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let first = journal(dir.path(), "first.db", |store| {
        run(store, "a", "fixture", &[("implement", 2 * GIB, 1000)], true);
    });
    let second = journal(dir.path(), "second.db", |store| {
        run(store, "b", "fixture", &[("implement", 6 * GIB, 3000)], true);
    });
    let broken = dir.path().join("broken.db");
    std::fs::write(&broken, "not a journal").unwrap();
    let registry = [first, second, broken.clone(), dir.path().join("absent.db")];
    let mut fixture = Fixture::new();
    fixture.queue();
    let (judged, named) = judged(
        &fixture,
        &plenty(),
        &registry,
        measured(16 * GIB, 8000),
        &BTreeMap::new(),
    );
    let sized = judged[0].demand.as_ref().unwrap();
    assert_eq!(sized.source, Source::Measured { runs: 2 });
    assert_eq!(sized.memory_bytes.average, 4 * GIB);
    let named: Vec<(&str, String)> = named
        .iter()
        .map(|reason| (reason.kind(), reason.to_string()))
        .collect();
    assert_eq!(named.len(), 1, "{named:?}");
    assert_eq!(named[0].0, "journal_unreadable");
    assert!(
        named[0].1.starts_with(&format!(
            "the journal {} the realms map lists cannot be read",
            broken.display()
        )),
        "{named:?}"
    );
    assert_eq!(said(&judged), vec![vec![]]);
}

/// Ruling C: an entry whose recipe has no measured run and no declaration
/// starts on an idle machine, and waits beside a running run, naming it.
#[test]
fn an_entry_of_unknown_demand_starts_on_an_idle_machine_and_waits_beside_a_running_run() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let none = BTreeMap::new();
    let idle = judged(&fixture, &plenty(), &[], measured(GIB, 0), &none).0;
    assert_eq!(said(&idle), vec![vec![]]);
    assert_eq!(idle[0].demand.as_ref().unwrap().source, Source::Unknown);
    let dir = tempfile::tempdir().unwrap();
    let other = journal(dir.path(), "other.db", |store| {
        run(store, "elsewhere", "verify", &[("build", 0, 0)], false);
    });
    let beside = judged(
        &fixture,
        &plenty(),
        &[other],
        measured(16 * GIB, 8000),
        &none,
    )
    .0;
    assert_eq!(
        said(&beside),
        vec![vec![(
            "demand_unknown",
            "recipe fixture has no measured or declared demand, so it starts only when \
             nothing else runs, and 1 runs are running, 0 entries were admitted before it in \
             this pass and 0 journals of the realms map cannot be read"
                .to_string()
        )]]
    );
}

#[test]
fn a_recipes_first_running_attempt_does_not_turn_into_history() {
    let mut fixture = Fixture::new();
    run(
        &mut fixture.store,
        "first",
        "fixture",
        &[("intake", GIB, 1000), ("implement", 0, 0)],
        false,
    );
    fixture.queue();
    let pass = |fixture: &Fixture| {
        judged(
            fixture,
            &plenty(),
            &[],
            measured(16 * GIB, 8000),
            &BTreeMap::new(),
        )
        .0
    };
    let waiting = pass(&fixture);
    assert_eq!(waiting[0].demand.as_ref().unwrap().source, Source::Unknown);
    assert_eq!(said(&waiting)[0][0].0, "demand_unknown");

    fixture
        .store
        .append_next("first", EventType::RunCompleted, json!({}), None, None)
        .unwrap();
    let after = pass(&fixture);
    assert_eq!(
        after[0].demand.as_ref().unwrap().source,
        Source::Measured { runs: 1 }
    );
    assert_eq!(said(&after), vec![vec![]]);
}

#[test]
fn stopped_only_history_leaves_demand_unknown_beside_a_running_run() {
    let mut fixture = Fixture::new();
    run(
        &mut fixture.store,
        "stopped-in-intake",
        "fixture",
        &[("intake", 300 << 20, 1000)],
        false,
    );
    fixture
        .store
        .append_next(
            "stopped-in-intake",
            EventType::RunStopped,
            json!({}),
            None,
            None,
        )
        .unwrap();
    run(
        &mut fixture.store,
        "live",
        "fixture",
        &[("implement", 0, 0)],
        false,
    );
    fixture.queue();
    let mut host = declared(GIB, 1000);
    let declared_demand = host.as_object_mut().unwrap().remove("demand").unwrap();
    host.as_object_mut().unwrap().remove("runs");
    let pass = |host: &Value| {
        judged(
            &fixture,
            host,
            &[],
            measured(16 * GIB, 8000),
            &BTreeMap::new(),
        )
        .0
    };
    let unknown = pass(&host);
    assert_eq!(unknown[0].demand.as_ref().unwrap().source, Source::Unknown);
    assert_eq!(
        unknown[0].verdict.as_ref().unwrap().reasons,
        vec![Reason::Capacity(Capacity::Demand(Unfit(Short::Unknown {
            recipe: "fixture".to_string(),
            beside: Beside {
                running: 1,
                admitted: 0,
                unreadable: 0,
            },
        })))]
    );

    host["demand"] = declared_demand;
    let declared = pass(&host);
    assert_eq!(
        declared[0].demand.as_ref().unwrap().source,
        Source::Declared
    );
    assert_eq!(said(&declared), vec![vec![]]);
}

#[test]
fn unmeasured_remaining_phases_reserve_the_recipes_full_expected_peak() {
    let mut fixture = Fixture::new();
    run(
        &mut fixture.store,
        "completed-without-implement-usage",
        "fixture",
        &[("intake", 4 * GIB, 1000), ("implement", 0, 0)],
        true,
    );
    run(
        &mut fixture.store,
        "live",
        "fixture",
        &[("implement", 0, 0)],
        false,
    );
    fixture.queue();
    let (entries, _) = judged(
        &fixture,
        &plenty(),
        &[],
        measured(8 * GIB, 8000),
        &BTreeMap::new(),
    );
    assert_eq!(
        entries[0].demand.as_ref().unwrap().reserved,
        Reserved {
            memory_bytes: 4 * GIB,
            load_millicores: 1000,
            runs: 1,
            whole: 1,
            admitted: 0,
        }
    );
    assert_eq!(said(&entries)[0][0].0, "memory_short");
}

#[test]
fn a_known_demand_waits_while_an_unknown_demand_holds_the_machine() {
    let (mut fixture, history) = measured_fixture();
    let other = fixture.root.join("other");
    std::fs::create_dir_all(other.join("roles")).unwrap();
    let bundle = fixture.root.join("bundle");
    for file in ["policy.json", "roles/work.md"] {
        std::fs::copy(bundle.join(file), other.join(file)).unwrap();
    }
    let mut manifest: Value =
        serde_json::from_slice(&std::fs::read(bundle.join("bundle.json")).unwrap()).unwrap();
    manifest["name"] = json!("other");
    std::fs::write(other.join("bundle.json"), manifest.to_string()).unwrap();
    let first = fixture.queue_bundle("other");
    fixture.queue();
    let pass = |fixture: &Fixture| {
        judged(
            fixture,
            &plenty(),
            &history.journals,
            measured(16 * GIB, 8000),
            &BTreeMap::new(),
        )
        .0
    };
    let held = |holder| {
        (
            "demand_unknown",
            format!("{holder} holds the whole machine until it ends"),
        )
    };
    let from_queue = pass(&fixture);
    assert_eq!(
        said(&from_queue),
        vec![
            vec![],
            vec![held(format!("entry {first} of recipe other, admitted before it in this pass with its demand unknown,"))],
        ]
    );

    run(
        &mut fixture.store,
        "live",
        "other",
        &[("intake", 0, 0)],
        false,
    );
    let from_journal = pass(&fixture);
    assert_eq!(
        said(&from_journal)[1],
        vec![held(
            "running run 'live' of recipe other, whose demand is unknown,".to_string()
        )]
    );
}

#[test]
fn demand_larger_than_the_whole_machine_starts_only_when_idle() {
    let mut fixture = Fixture::new();
    fixture.queue();
    let registry = registry(|store| {
        run(store, "live", "fixture", &[("implement", 0, 0)], false);
    });
    for (resource, unit, memory, load, total, free) in [
        ("memory", "bytes", 16 * GIB, 1000, 16 * GIB, 8 * GIB),
        ("cpu", "millicores", GIB, 8000, 8000, 4000),
    ] {
        let host = declared(memory, load);
        let use_now = Use {
            memory_bytes: memory,
            load_millicores: load,
        };
        let pass = |journals| {
            judged(
                &fixture,
                &host,
                journals,
                measured(8 * GIB, 4000),
                &BTreeMap::from([("live", use_now)]),
            )
            .0
        };
        assert_eq!(said(&pass(&[])), vec![vec![]], "{resource}");
        assert_eq!(
            said(&pass(&registry.journals)),
            vec![vec![(
                if resource == "memory" {
                    "memory_short"
                } else {
                    "cpu_short"
                },
                format!(
                    "{resource} is short: recipe fixture needs {} {unit} ({total} average over \
                     its declared demand + {} margin), {free} {unit} available, 0 {unit} \
                     reserved by 1 running runs; it needs more than the machine's {total} \
                     {unit}, so it starts only when nothing else runs",
                    total + total / 4,
                    total / 4,
                ),
            )]]
        );
    }
}

#[test]
fn a_declared_demand_applies_until_the_recipe_has_measured_history() {
    let (mut fixture, history) = measured_fixture();
    fixture.queue();
    let host = declared(2 * GIB, 1000);
    let pass = |journals| {
        judged(
            &fixture,
            &host,
            journals,
            measured(3 * GIB, 8000),
            &BTreeMap::new(),
        )
        .0
    };
    let declared = pass(&[]);
    let sized = declared[0].demand.as_ref().unwrap();
    assert_eq!(sized.source, Source::Declared);
    assert_eq!(sized.memory_bytes, Need::of(2 * GIB, 25));
    assert_eq!(sized.load_millicores, Need::of(1000, 25));
    assert_eq!(said(&declared), vec![vec![]]);
    let learned = pass(&history.journals);
    let sized = learned[0].demand.as_ref().unwrap();
    assert_eq!(sized.source, Source::Measured { runs: 2 });
    assert_eq!(sized.memory_bytes, Need::of(3 * GIB, 25));
    assert_eq!(sized.load_millicores, Need::of(2000, 25));
    assert_eq!(said(&learned)[0][0].0, "memory_short");
}

/// A host file with no run ceiling: measured, no entry waits as
/// `runs_undeclared`; unmeasured, exactly main's verdict, the run ceiling
/// required.
#[test]
fn a_host_file_without_a_run_ceiling_is_judged_by_demand_only_where_the_machine_is_measured() {
    let (mut fixture, registry) = measured_fixture();
    let registry = &registry.journals;
    fixture.queue();
    let mut v1 = plenty();
    v1["schema"] = json!("forge.host/v1");
    v1.as_object_mut().unwrap().remove("runs");
    let none = BTreeMap::new();
    let measured_pass = judged(&fixture, &v1, registry, measured(16 * GIB, 8000), &none).0;
    assert_eq!(said(&measured_pass), vec![vec![]]);
    let unmeasured = Measurement::Unmeasured(Unmeasured::Platform);
    let unmeasured_pass = judged(&fixture, &v1, registry, unmeasured, &none).0;
    let main =
        crate::admission::capacity::tests::passed(&fixture.store, &fixture.declare(&v1)).unwrap();
    assert_eq!(said(&unmeasured_pass), said(&main));
    assert_eq!(said(&main)[0][0].0, "runs_undeclared");
    assert_eq!(unmeasured_pass[0].demand, None);
}

/// The count checks come first, and keep their reasons: a provider at its
/// ceiling waits as `provider_full` however much room there is.
#[test]
fn a_provider_ceiling_is_judged_before_demand_and_keeps_its_reason() {
    let (mut fixture, registry) = measured_fixture();
    run(
        &mut fixture.store,
        "live",
        "fixture",
        &[("implement", 0, 0)],
        false,
    );
    fixture.queue();
    let none = BTreeMap::new();
    let judged = judged(
        &fixture,
        &ceilings(1, 1, json!({})),
        &registry.journals,
        measured(16 * GIB, 8000),
        &none,
    )
    .0;
    assert_eq!(said(&judged)[0][0].0, "provider_full");
    assert_eq!(judged[0].demand, None);
}

/// Every file of `dir`, by name, with its bytes.
fn files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let name = entry.file_name().to_string_lossy().into_owned();
            (name, std::fs::read(entry.path()).unwrap())
        })
        .collect()
}

/// The registry's journals are read and never written: each directory is
/// byte for byte as it was, with no `-wal`, `-shm` or lock left behind,
/// a running run's included.
#[test]
fn reading_the_registry_journals_leaves_each_directory_byte_identical() {
    let dirs = [tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap()];
    let registry = [
        journal(dirs[0].path(), "a.db", |store| {
            run(store, "done", "fixture", &[("implement", GIB, 1000)], true);
        }),
        journal(dirs[1].path(), "b.db", |store| {
            run(store, "going", "fixture", &[("implement", 0, 0)], false);
        }),
    ];
    let before: Vec<_> = dirs.iter().map(|dir| files(dir.path())).collect();
    assert!(
        before.iter().all(|files| files.len() == 1),
        "{:?}",
        before.iter().map(BTreeMap::len).collect::<Vec<_>>()
    );
    let mut fixture = Fixture::new();
    fixture.queue();
    let judged = judged(
        &fixture,
        &plenty(),
        &registry,
        measured(16 * GIB, 8000),
        &BTreeMap::new(),
    )
    .0;
    assert_eq!(judged[0].demand.as_ref().unwrap().reserved.runs, 1);
    let after: Vec<_> = dirs.iter().map(|dir| files(dir.path())).collect();
    assert_eq!(after, before);
}

/// Every demand reason has its kind, and none is the operator's to clear.
#[test]
fn every_demand_reason_waits_and_has_its_kind() {
    let holder = Short::Held(Holder::Run {
        run: "r".into(),
        recipe: "x".into(),
    });
    let unknown = Short::Unknown {
        recipe: "x".into(),
        beside: Beside::default(),
    };
    for (short, kind) in [(holder, "demand_unknown"), (unknown, "demand_unknown")] {
        let reason = Reason::Capacity(Capacity::Demand(Unfit(short)));
        assert_eq!((reason.kind(), reason.holds()), (kind, false));
    }
}
