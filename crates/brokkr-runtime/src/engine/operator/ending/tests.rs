//! The cause `stop` and `conclude` name (decision 0064 ruling 4, slice
//! E3), and the helpers the engine's other tests stop and conclude runs
//! with now that both verbs take one.

use super::*;
use brokkr_core::ending::ended;
use brokkr_core::envelope::EventType;
use brokkr_core::EventEnvelope;
use brokkr_core::RunState;
use std::path::Path;

use crate::engine::operator::{
    apply_fenced_operator_command, conclude, conclude_racing, operator_command,
};
use crate::engine::tests::{in_flight_store, looper, parked_store};

/// The cause a test names when it only needs a run stopped or concluded:
/// `operator-stopped`, citing nothing.
pub(in crate::engine) const STOPPED: Cause<'static> = Cause {
    ending: Ending::OperatorStopped,
    by_realm: None,
    by_run: None,
    by_ref: None,
};

/// The journal `store` writes, opened read-only, as the CLI opens the
/// workspace journal a citation naming no realm is read in.
pub(in crate::engine) fn workspace(store: &Store) -> Store {
    Store::open_read_only(store.path()).unwrap()
}

/// [`operator_stop`] for a test that needs the run stopping, under
/// [`STOPPED`].
pub(in crate::engine) fn stop_as(
    store: &mut Store,
    run_id: &str,
    operator: &str,
    reason: &str,
) -> Result<FencedCommandOutcome, EngineError> {
    stop_racing_as(store, run_id, (operator, reason), |_| {})
}

/// [`operator_stop_racing`] under [`STOPPED`], `between` its window.
pub(in crate::engine) fn stop_racing_as(
    store: &mut Store,
    run_id: &str,
    (operator, reason): (&str, &str),
    between: impl FnMut(&mut Store),
) -> Result<FencedCommandOutcome, EngineError> {
    let cited = workspace(store);
    let named = Named {
        cause: STOPPED,
        reason,
        operator,
    };
    operator_stop_racing(store, &cited, run_id, &named, between)
}

/// [`conclude`] for a test that needs the run concluded, under
/// [`STOPPED`].
pub(in crate::engine) fn conclude_as(
    store: &mut Store,
    run_id: &str,
    operator: &str,
    reason: &str,
) -> Result<RunState, EngineError> {
    conclude_racing_as(store, run_id, (operator, reason), |_| {})
}

/// [`conclude_racing`] under [`STOPPED`], `between` its windows.
pub(in crate::engine) fn conclude_racing_as(
    store: &mut Store,
    run_id: &str,
    (operator, reason): (&str, &str),
    between: impl FnMut(&mut Store),
) -> Result<RunState, EngineError> {
    let cited = workspace(store);
    let named = Named {
        cause: STOPPED,
        reason,
        operator,
    };
    conclude_racing(store, &cited, run_id, &named, between)
}

/// A journal as its serialized events, so two reads compare byte for byte.
pub(in crate::engine) fn bytes(events: &[EventEnvelope]) -> Vec<Value> {
    events.iter().map(|event| json!(event)).collect()
}

/// The published schema over `operator/commanded.args`.
pub(in crate::engine) fn contract() -> jsonschema::Validator {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("contracts/operator-ending.v1.schema.json");
    let schema: Value = serde_json::from_slice(
        &std::fs::read(&path).expect("the published payload schema is beside the contracts"),
    )
    .unwrap();
    jsonschema::draft7::new(&schema).unwrap()
}

/// One misplaced or missing citation: the cause, its three citation flags
/// (`--by-run`, `--by-realm`, `--by-ref`), the variant it is refused as,
/// and its exact text, `{verb}` standing for the verb that refused it.
pub(in crate::engine) type CitationCase = (
    &'static str,
    [Option<&'static str>; 3],
    fn(&CauseRefusal) -> bool,
    &'static str,
);

/// Ruling 4 and choice 7: a citation belongs to its cause and nowhere
/// else, on every verb that names one. Each case's text is pinned here
/// once, for all three verbs.
pub(in crate::engine) const CITATION_CASES: [CitationCase; 9] = [
    (
        "superseded",
        [None, None, None],
        |refused| matches!(refused, CauseRefusal::NoReplacement { .. }),
        "{verb} --cause superseded needs --by-run: the run that replaced this one",
    ),
    (
        "superseded",
        [Some("later"), None, Some("abc123")],
        |refused| matches!(refused, CauseRefusal::RefOnSuperseded { .. }),
        "{verb} --cause superseded takes no --by-ref; name the run that replaced this one with \
         --by-run",
    ),
    (
        "delivered-elsewhere",
        [None, None, None],
        |refused| matches!(refused, CauseRefusal::NoRef { .. }),
        "{verb} --cause delivered-elsewhere needs --by-ref: the commit or pull request it landed \
         by",
    ),
    (
        "delivered-elsewhere",
        [Some("later"), None, Some("abc123")],
        |refused| {
            matches!(
                refused,
                CauseRefusal::Misplaced {
                    flag: Flag::ByRun,
                    ..
                }
            )
        },
        "{verb} --cause delivered-elsewhere takes no --by-run; --by-run belongs to 'superseded'",
    ),
    (
        "delivered-elsewhere",
        [None, Some("neighbour"), Some("abc123")],
        |refused| {
            matches!(
                refused,
                CauseRefusal::Misplaced {
                    flag: Flag::ByRealm,
                    ..
                }
            )
        },
        "{verb} --cause delivered-elsewhere takes no --by-realm; --by-realm belongs to 'the realm \
         --by-run is read in'",
    ),
    (
        "withdrawn",
        [Some("later"), None, None],
        |refused| {
            matches!(
                refused,
                CauseRefusal::Misplaced {
                    flag: Flag::ByRun,
                    ..
                }
            )
        },
        "{verb} --cause withdrawn takes no --by-run; --by-run belongs to 'superseded'",
    ),
    (
        "withdrawn",
        [None, Some("neighbour"), None],
        |refused| {
            matches!(
                refused,
                CauseRefusal::Misplaced {
                    flag: Flag::ByRealm,
                    ..
                }
            )
        },
        "{verb} --cause withdrawn takes no --by-realm; --by-realm belongs to 'superseded'",
    ),
    (
        "withdrawn",
        [None, None, Some("abc123")],
        |refused| {
            matches!(
                refused,
                CauseRefusal::Misplaced {
                    flag: Flag::ByRef,
                    ..
                }
            )
        },
        "{verb} --cause withdrawn takes no --by-ref; --by-ref belongs to 'delivered-elsewhere'",
    ),
    (
        "infrastructure",
        [None, None, Some("abc123")],
        |refused| {
            matches!(
                refused,
                CauseRefusal::Misplaced {
                    flag: Flag::ByRef,
                    ..
                }
            )
        },
        "{verb} --cause infrastructure takes no --by-ref; --by-ref belongs to \
         'delivered-elsewhere'",
    ),
];

/// The cause one [`CitationCase`] names.
pub(in crate::engine) fn case_cause(
    (word, [by_run, by_realm, by_ref], _, _): &CitationCase,
) -> Cause<'static> {
    Cause {
        ending: Ending::named(word).unwrap(),
        by_realm: *by_realm,
        by_run: *by_run,
        by_ref: *by_ref,
    }
}

/// The cause refusal `error` carries, its verb's own, with the prefix
/// every verb's refusal prints.
pub(in crate::engine) fn cause_refusal(error: EngineError, verb: Verb) -> CauseRefusal {
    let printed = error.to_string();
    match error {
        EngineError::CauseRefused(refused) => {
            assert_eq!(refused.verb(), verb);
            assert_eq!(
                printed,
                format!("{verb} refused, nothing was written: {refused}")
            );
            *refused
        }
        other => panic!("{verb} refused with another error: {other}"),
    }
}

/// The ask a test varies the cause of.
fn named(cause: Cause<'static>) -> Named<'static> {
    Named {
        cause,
        reason: "the operator named the ending",
        operator: "operator",
    }
}

/// Run `id` in `db`, parked by the engine, and the run `later` a
/// `superseded` citation resolves to, still going (ruling 4 lets a
/// re-commission be cited before it finishes).
fn parked_beside_later(db: &Path, id: &str) -> Store {
    let store = parked_store(db, id);
    if Store::open_read_only(db).unwrap().load("later").is_err() {
        in_flight_store(db, "later");
    }
    store
}

/// What each verb refuses, by its verb, with the journal it left: the
/// table of citations, a cause outside the six, and a cited run its
/// journal cannot show.
fn refusals(
    verb: Verb,
    mut ask: impl FnMut(&mut Store, &Store, &Named) -> Result<(), EngineError>,
) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("refusals.db");
    let mut store = parked_beside_later(&db, "held");
    let cited = workspace(&store);
    let before = bytes(&store.load("held").unwrap());
    let mut refused = |cause: Cause<'static>| {
        let refusal = cause_refusal(ask(&mut store, &cited, &named(cause)).unwrap_err(), verb);
        assert_eq!(
            bytes(&store.load("held").unwrap()),
            before,
            "{verb} wrote nothing"
        );
        refusal
    };
    for case in CITATION_CASES {
        let refusal = refused(case_cause(&case));
        let (word, _, variant, text) = case;
        assert!(variant(&refusal), "{verb} {word}: {refusal:?}");
        assert_eq!(
            refusal.to_string(),
            text.replace("{verb}", &verb.to_string())
        );
    }
    let shipped = refused(Cause {
        ending: Ending::Shipped,
        ..STOPPED
    });
    assert!(matches!(
        shipped,
        CauseRefusal::NotOperatorOwned {
            ending: Ending::Shipped,
            ..
        }
    ));
    assert_eq!(
        shipped.to_string(),
        "'shipped' is not an operator-owned ending; an operator names one of exhausted, \
         superseded, infrastructure, delivered-elsewhere, withdrawn, operator-stopped"
    );
    let absent = refused(Cause {
        ending: Ending::Superseded,
        by_run: Some("no-such-run"),
        ..STOPPED
    });
    assert!(matches!(absent, CauseRefusal::Absent { .. }));
    assert_eq!(
        absent.to_string(),
        "run 'no-such-run' in the workspace journal: run 'no-such-run' not found"
    );
}

/// Ruling 4: every misplaced or missing citation, a cause outside the six
/// and a cited run that is not there are refused on `stop`, by exact text,
/// before anything is written.
#[test]
fn stop_refuses_every_misfit_cause_and_writes_nothing() {
    refusals(Verb::Stop, |store, cited, named| {
        operator_stop(store, cited, "held", named).map(drop)
    });
}

/// The same refusals on `conclude`, by its own verb, writing nothing.
#[test]
fn conclude_refuses_every_misfit_cause_and_writes_nothing() {
    refusals(Verb::Conclude, |store, cited, named| {
        conclude(store, cited, "held", named).map(drop)
    });
}

/// Ruling 4: `--cause` is required. A missing one is refused where it was
/// typed, by a usage text naming the six; an unknown word by the
/// vocabulary's own text; and a stop asked for without a cause through
/// the command path writes nothing.
#[test]
fn a_missing_or_unknown_cause_is_refused_and_a_causeless_stop_writes_nothing() {
    for verb in [Verb::Stop, Verb::Conclude, Verb::Classify] {
        let missing = named_cause(verb, None).unwrap_err();
        assert!(matches!(missing, CauseRefusal::Missing { verb: v } if v == verb));
        assert_eq!(
            missing.to_string(),
            format!(
                "{verb} needs --cause: one of exhausted, superseded, infrastructure, \
                 delivered-elsewhere, withdrawn, operator-stopped"
            )
        );
        let unknown = named_cause(verb, Some("not-a-word")).unwrap_err();
        assert!(matches!(unknown, CauseRefusal::Unknown { verb: v, .. } if v == verb));
        assert_eq!(
            unknown.to_string(),
            "'not-a-word' is not an ending; an ending is one of shipped, shipped-flagged, \
             refused, failed, blocked, exhausted, superseded, infrastructure, \
             delivered-elsewhere, withdrawn, operator-stopped, unclassified"
        );
        assert_eq!(
            named_cause(verb, Some("withdrawn")).unwrap(),
            Ending::Withdrawn
        );
    }
    let dir = tempfile::tempdir().unwrap();
    let mut store = parked_store(&dir.path().join("causeless.db"), "held");
    let before = bytes(&store.load("held").unwrap());
    let stop = OperatorCommand::Stop;
    let refused = operator_command(&mut store, "held", stop, "operator", "enough").unwrap_err();
    assert!(matches!(
        cause_refusal(refused, Verb::Stop),
        CauseRefusal::Missing { verb: Verb::Stop }
    ));
    assert_eq!(
        bytes(&store.load("held").unwrap()),
        before,
        "nothing written"
    );
}

/// The six causes, each with the citation its word takes, and the `args`
/// `operator-ending.v1` writes for it.
fn every_shape() -> [(Cause<'static>, Value); 7] {
    let cause = |word: &str| Cause {
        ending: Ending::named(word).unwrap(),
        ..STOPPED
    };
    let by = |realm: Option<&'static str>| Cause {
        by_run: Some("later"),
        by_realm: realm,
        ..cause("superseded")
    };
    [
        (
            by(None),
            json!({"cause": "superseded", "by": {"realm": null, "run_id": "later"}}),
        ),
        (
            by(Some("next-door")),
            json!({"cause": "superseded", "by": {"realm": "next-door", "run_id": "later"}}),
        ),
        (
            Cause {
                by_ref: Some("abc123"),
                ..cause("delivered-elsewhere")
            },
            json!({"cause": "delivered-elsewhere", "ref": "abc123"}),
        ),
        (cause("infrastructure"), json!({"cause": "infrastructure"})),
        (cause("withdrawn"), json!({"cause": "withdrawn"})),
        // Addendum ruling 3 as amended (choice 3 (c)): nothing guards it.
        (cause("exhausted"), json!({"cause": "exhausted"})),
        (
            cause("operator-stopped"),
            json!({"cause": "operator-stopped"}),
        ),
    ]
}

/// The event types `events` holds past its first `from`.
fn appended(events: &[EventEnvelope], from: usize) -> Vec<EventType> {
    events[from..]
        .iter()
        .map(|event| event.event_type)
        .collect()
}

/// Ruling 4: a stop writes exactly one `operator/commanded`, its `args`
/// the cause's shape under `operator-ending.v1`, for every cause; the
/// sentence stays on the acceptance, and nothing else moves.
#[test]
fn stop_writes_its_cause_in_one_commanded_under_operator_ending_v1() {
    let contract = contract();
    each_shape(|store, cited, run_id, (cause, args), before| {
        let outcome = operator_stop(store, cited, run_id, &named(cause)).unwrap();
        assert!(
            matches!(outcome, FencedCommandOutcome::Accepted { .. }),
            "{args}"
        );
        let events = store.load(run_id).unwrap();
        assert_eq!(
            appended(&events, before),
            [EventType::OperatorCommanded, EventType::OperatorAccepted],
            "{args}"
        );
        let (commanded, acceptance) = (&events[before], &events[before + 1]);
        assert_eq!(commanded.payload["command"], "stop");
        assert_eq!(commanded.payload["args"], args);
        assert!(contract.is_valid(&commanded.payload["args"]), "{args}");
        assert_eq!(
            acceptance.payload["reason"],
            "the operator named the ending"
        );
    });
}

/// Each of [`every_shape`]'s causes, named on a parked run of its own
/// beside `later`, its citation read in the journal it names: `act` names
/// it, given the run's journal length before it did.
fn each_shape(mut act: impl FnMut(&mut Store, &Store, &str, (Cause<'static>, Value), usize)) {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("shapes.db");
    let next_door = dir.path().join("next-door.db");
    in_flight_store(&next_door, "later");
    for (at, shape) in every_shape().into_iter().enumerate() {
        let run_id = format!("held-{at}");
        let mut store = parked_beside_later(&db, &run_id);
        let cited = match shape.0.by_realm {
            Some(_) => Store::open_read_only(&next_door).unwrap(),
            None => workspace(&store),
        };
        let before = store.load(&run_id).unwrap().len();
        act(&mut store, &cited, &run_id, shape, before);
    }
}

/// Choice 4 (a), the first case: `conclude` commands the stop itself, the
/// cause rides in that stop's `args`, and no `classify` follows, for every
/// cause, `exhausted` among them with no guard.
#[test]
fn conclude_commands_a_stop_carrying_the_cause_and_writes_no_classify() {
    each_shape(|store, cited, run_id, (cause, args), before| {
        let state = conclude(store, cited, run_id, &named(cause)).unwrap();
        let events = store.load(run_id).unwrap();
        assert_eq!(
            appended(&events, before),
            [
                EventType::OperatorCommanded,
                EventType::OperatorAccepted,
                EventType::RunStopped
            ],
            "{args}: no classify"
        );
        assert_eq!(events[before].payload["args"], args);
        let ending = checked(Verb::Conclude, &cause, cited).unwrap().ended();
        assert_eq!(ended(&events, &state), Some(ending), "{args}");
    });
}

/// A run with a stop in force: `stop` accepted under `cause`.
fn stopping(db: &Path, run_id: &str, cause: Cause<'static>) -> Store {
    let mut store = parked_beside_later(db, run_id);
    let cited = workspace(&store);
    let stopped = operator_stop(&mut store, &cited, run_id, &named(cause)).unwrap();
    assert!(matches!(stopped, FencedCommandOutcome::Accepted { .. }));
    store
}

/// Choice 4 (a), the second case: a stop in force carrying the cause
/// `conclude` names gets nothing more than its conclusion, and the journal
/// it found stands byte for byte beneath `run/stopped`.
#[test]
fn conclude_on_a_stop_carrying_the_same_cause_writes_nothing_more() {
    let dir = tempfile::tempdir().unwrap();
    let infrastructure = Cause {
        ending: Ending::Infrastructure,
        ..STOPPED
    };
    let mut store = stopping(&dir.path().join("same.db"), "held", infrastructure);
    let cited = workspace(&store);
    let before = bytes(&store.load("held").unwrap());
    let state = conclude(&mut store, &cited, "held", &named(infrastructure)).unwrap();
    let events = store.load("held").unwrap();
    assert_eq!(bytes(&events[..before.len()]), before);
    assert_eq!(appended(&events, before.len()), [EventType::RunStopped]);
    assert_eq!(
        ended(&events, &state).map(|ended| ended.ending),
        Some(Ending::Infrastructure)
    );
}

/// The one `classify` a conclusion appended after `run/stopped`: its
/// events past `from` are exactly `run/stopped` and it, its `args` are
/// `expected` with the operator's sentence, and the reading returns it.
fn assert_classified_after_stop(store: &Store, from: usize, state: &RunState, expected: Value) {
    let events = store.load("held").unwrap();
    assert_eq!(
        appended(&events, from),
        [EventType::RunStopped, EventType::OperatorCommanded]
    );
    let classify = events.last().unwrap();
    assert_eq!(classify.payload["command"], "classify");
    assert_eq!(classify.payload["operator"], "operator");
    let mut args = expected;
    args["reason"] = json!("the operator named the ending");
    assert_eq!(classify.payload["args"], args);
    assert!(contract().is_valid(&classify.payload["args"]));
    assert_eq!(
        classify.causation_id.as_deref(),
        Some(events[from].event_id.as_str()),
        "the classify is fenced on the run/stopped it follows"
    );
    assert_eq!(
        ended(&events, state).map(|ended| ended.ending),
        Some(Ending::Withdrawn),
        "the classify stands over the stop's cause"
    );
}

/// Choice 4 (a), the third case: a stop in force carrying another cause,
/// or none at all (an older journal's, or the bridge's), gets exactly one
/// `classify` after `run/stopped`, which the reading returns.
#[test]
fn conclude_on_a_stop_carrying_another_cause_or_none_appends_one_classify() {
    let dir = tempfile::tempdir().unwrap();
    let withdrawn = Cause {
        ending: Ending::Withdrawn,
        ..STOPPED
    };
    let infrastructure = Cause {
        ending: Ending::Infrastructure,
        ..STOPPED
    };
    let mut other = stopping(&dir.path().join("other.db"), "held", infrastructure);
    let cited = workspace(&other);
    let from = other.load("held").unwrap().len();
    let state = conclude(&mut other, &cited, "held", &named(withdrawn)).unwrap();
    assert_classified_after_stop(&other, from, &state, json!({"cause": "withdrawn"}));

    let mut none = parked_store(&dir.path().join("none.db"), "held");
    let (seq, hash) = none.head_hash("held").unwrap();
    let wire = looper("bridge-stop", "stop", seq, &hash);
    let bridged = apply_fenced_operator_command(&mut none, "held", &wire).unwrap();
    assert!(matches!(bridged, FencedCommandOutcome::Accepted { .. }));
    let cited = workspace(&none);
    let from = none.load("held").unwrap().len();
    let state = conclude(&mut none, &cited, "held", &named(withdrawn)).unwrap();
    assert_classified_after_stop(&none, from, &state, json!({"cause": "withdrawn"}));
}
