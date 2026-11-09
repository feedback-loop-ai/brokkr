//! The window reads' tests (decision 0089 ruling 7, slice 4's unit U3b):
//! every read equals brokkr-core's whole-run reading on every fixture and
//! hand-built journal, each order is its cut key's order, and a window's
//! cost grows with what lies inside it and never with the journal.

use brokkr_core::envelope::{ChainError, EventEnvelope, EventType};
use brokkr_core::fold::concluded;
use brokkr_core::seat::row::{Datum, SeatRow};
use brokkr_core::stamp::parse_millis;
use brokkr_core::summary::{
    aggregate, checkpoints, sites, Account, Bounded, Checkpoint, MODEL_BYTES, NAME_BYTES,
};
use brokkr_core::EventEnvelope as Envelope;
use serde_json::{json, Value};

use super::*;
use crate::test_support::{fixture_events, import_fixtures, plant_older_writer};
use crate::tests::envelope_builder::EnvelopeBuilder;

/// A window's expected spend: its run, its run's stored realm key, its
/// checkpoint's time and stored phase, and its cost.
#[derive(Debug, PartialEq)]
struct Spend {
    run_id: String,
    realm: Option<String>,
    at_ms: i64,
    phase: Option<String>,
    usd: f64,
}

/// A window's expected site: the cut keys the writer keeps of it, its run's
/// aggregate columns, and the in-window facts brokkr-core's reading gives.
#[derive(Debug, PartialEq, Eq)]
struct Site {
    run_id: String,
    /// The site's row key in its run's seat rows: the writer's `opened`,
    /// counted before it, so a participant row ahead of it moves it.
    opened: u64,
    realm: Option<String>,
    recipe: Option<String>,
    open: bool,
    finished_ms: Option<i64>,
    phase: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    started_ms: i64,
    metering: Metering,
    reported: bool,
    usage: Option<TokenUsage>,
}

/// `text` cut where the writer keeps a name key.
fn cut(text: &str) -> String {
    Bounded::within(text, NAME_BYTES).cut
}

/// `text` cut where the writer keeps a model key.
fn model(text: &str) -> String {
    Bounded::within(text, MODEL_BYTES).cut
}

/// What brokkr-core reads of runs for a window: every spend whose
/// checkpoint falls in it, and every site that started or metered there,
/// with `reported` and the usage `attempts.rs` reads of it.
fn core_reads(
    runs: &[(String, Vec<EventEnvelope>)],
    from: i64,
    to: i64,
) -> (Vec<Spend>, Vec<Site>) {
    let mut spends = Vec::new();
    let mut held = Vec::new();
    for (run_id, events) in runs {
        let state = brokkr_core::fold(events).ok();
        let read = aggregate(events, state.as_ref());
        let account = Account::whole(run_id, events);
        let open = state
            .as_ref()
            .is_some_and(|state| concluded(state).is_none());
        let realm = read.realm.map(|realm| cut(&realm));
        for checkpoint in checkpoints(events) {
            let Some(usd) = checkpoint.usd else {
                continue;
            };
            if checkpoint.at >= from && checkpoint.at < to {
                spends.push(Spend {
                    run_id: run_id.clone(),
                    realm: realm.clone(),
                    at_ms: checkpoint.at,
                    phase: checkpoint.phase.as_deref().map(cut),
                    usd,
                });
            }
        }
        for (site_index, site) in sites(events).into_iter().enumerate() {
            let kept: Vec<&Checkpoint> = site
                .metered
                .iter()
                .filter(|checkpoint| checkpoint.at >= from && checkpoint.at < to)
                .collect();
            if !(site.at >= from && site.at < to) && kept.is_empty() {
                continue;
            }
            held.push(Site {
                run_id: run_id.clone(),
                opened: opened_of(&account, site_index),
                realm: realm.clone(),
                recipe: read.recipe.as_deref().map(cut),
                open,
                finished_ms: read.finished.as_ref().map(|finished| finished.at),
                phase: site.phase.as_deref().map(cut),
                provider: Some(cut(&site.provider)),
                model: site.model.as_deref().map(model),
                started_ms: site.at,
                metering: site.metering,
                reported: kept.iter().any(|checkpoint| checkpoint.reported),
                usage: sum_usage(kept.into_iter().filter_map(|checkpoint| {
                    checkpoint.meter.counted(site.metering, checkpoint.session)
                })),
            });
        }
    }
    (spends, sorted_sites(held))
}

/// The writer's `opened` key of the site the run's seats opened `site_index`
/// in: the row's place among all its seat rows, participant rows included,
/// which is where `seat_summary` writes it.
fn opened_of(account: &Account, site_index: usize) -> u64 {
    let at = SeatRow::COLUMNS
        .iter()
        .position(|(name, _)| *name == "site")
        .expect("the seat row declares its site key");
    account
        .seats()
        .iter()
        .position(|row| row.data().get(at) == Some(&Datum::Integer(site_index as i64)))
        .expect("the run keeps the site it opened") as u64
}

/// The sites in the one order tests compare them in, so a multiset equality
/// is a plain vector equality.
fn sorted_sites(mut sites: Vec<Site>) -> Vec<Site> {
    sites.sort_by_key(key);
    sites
}

/// A site's sort key: every field but its usage, which is compared as a
/// value.
fn key(
    site: &Site,
) -> (
    String,
    u64,
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    u8,
) {
    (
        site.run_id.clone(),
        site.opened,
        site.started_ms,
        site.phase.clone(),
        site.provider.clone(),
        site.model.clone(),
        match site.metering {
            Metering::Session => 0,
            Metering::Turns => 1,
            Metering::Settled => 2,
        },
    )
}

/// The spends in a comparable order, the cost last.
fn sorted_spends(mut spends: Vec<Spend>) -> Vec<Spend> {
    spends.sort_by(|left, right| {
        (
            left.run_id.clone(),
            left.at_ms,
            left.phase.clone(),
            left.realm.clone(),
        )
            .cmp(&(
                right.run_id.clone(),
                right.at_ms,
                right.phase.clone(),
                right.realm.clone(),
            ))
            .then(left.usd.total_cmp(&right.usd))
    });
    spends
}

/// The store's sites for a window, in `order`, made comparable.
fn read_sites(store: &Store, from: i64, to: i64, order: SiteOrder) -> Vec<Site> {
    let mut held = Vec::new();
    store
        .window_sites(from, to, None, order, |site| {
            held.push(Site {
                run_id: site.run_id.to_string(),
                opened: site.opened,
                realm: site.realm.map(str::to_string),
                recipe: site.recipe.map(str::to_string),
                open: site.open,
                finished_ms: site.finished_ms,
                phase: site.phase.map(str::to_string),
                provider: site.provider.map(str::to_string),
                model: site.model.map(str::to_string),
                started_ms: site.started_ms.unwrap_or_default(),
                metering: site.metering,
                reported: site.reported,
                usage: site.usage.clone(),
            });
        })
        .unwrap();
    sorted_sites(held)
}

/// The store's spends for a window, made comparable.
fn read_spends(store: &Store, from: i64, to: i64) -> Vec<Spend> {
    let mut held = Vec::new();
    store
        .window_spends(from, to, None, |spend| {
            held.push(Spend {
                run_id: spend.run_id.to_string(),
                realm: spend.realm.map(str::to_string),
                at_ms: spend.at_ms,
                phase: spend.phase.map(str::to_string),
                usd: spend.usd,
            });
        })
        .unwrap();
    sorted_spends(held)
}

/// Whether two reported costs are the same within ruling A5's relative
/// bound, `1e-9`.
fn close(left: f64, right: f64) -> bool {
    left == right || (left - right).abs() <= 1e-9 * left.abs().max(right.abs())
}

/// Every read of the window equals brokkr-core's whole-run reading of it:
/// the sites in every order, and the spends within ruling A5's relative
/// bound.
fn assert_reads_as_core(store: &Store, runs: &[(String, Vec<EventEnvelope>)], from: i64, to: i64) {
    let (spends, sites) = core_reads(runs, from, to);
    let read = read_spends(store, from, to);
    assert_eq!(read.len(), spends.len(), "spends {from}..{to}");
    for (found, wanted) in read.iter().zip(&spends) {
        assert_eq!(
            (&found.run_id, &found.realm, found.at_ms, &found.phase,),
            (&wanted.run_id, &wanted.realm, wanted.at_ms, &wanted.phase,),
            "spends {from}..{to}"
        );
        assert!(
            close(found.usd, wanted.usd),
            "{} at {}: usd {}, read {}",
            wanted.run_id,
            wanted.at_ms,
            found.usd,
            wanted.usd
        );
    }
    for order in [
        SiteOrder::Model,
        SiteOrder::Provider,
        SiteOrder::RealmProvider,
        SiteOrder::RecipeProvider,
    ] {
        assert_eq!(
            read_sites(store, from, to, order),
            sites,
            "{order:?} sites {from}..{to}"
        );
    }
}

/// Every window a set of runs' events can be cut through: before all of
/// them, after all of them, and at each of their stamps and midpoints.
fn windows_of(runs: &[(String, Vec<EventEnvelope>)]) -> Vec<(i64, i64)> {
    let mut stamps: Vec<i64> = runs
        .iter()
        .flat_map(|(_, events)| events.iter())
        .filter_map(|event| parse_millis(&event.recorded_at))
        .collect();
    stamps.sort_unstable();
    stamps.dedup();
    let Some(&first) = stamps.first() else {
        return Vec::new();
    };
    let last = *stamps.last().unwrap();
    let mut windows = vec![
        (first - 1, first),
        (first, last + 1),
        (first + 1, last),
        (last, last + 1),
    ];
    for pair in stamps.windows(2) {
        let (one, two) = (pair[0], pair[1]);
        windows.push((one, two));
        windows.push((one + 1, two + 1));
        windows.push((one, (one + two) / 2 + 1));
    }
    windows.sort_unstable();
    windows.dedup();
    windows
}

/// Every exported journal, imported and refolded, reads in every window as
/// brokkr-core reads it whole.
#[test]
fn every_fixture_reads_as_its_fold() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    import_fixtures(&mut Store::open(&path).unwrap());
    let store = Store::open(&path).unwrap();
    let mut runs = Vec::new();
    for (_, _, events) in fixture_events() {
        let run_id = events[0].run_id.clone();
        match store.load(&run_id) {
            Ok(_) => runs.push((run_id, events)),
            Err(error) => {
                // The one fixture the journal carries and the chain refuses:
                // its event 11 hash was rewritten after it was sealed.
                assert_eq!(run_id, "conclude-broken-chain-hand-built");
                assert!(
                    matches!(error, StoreError::Chain(ChainError::BadHash { seq: 11 })),
                    "{run_id}: {error:?}"
                );
            }
        }
    }
    for (from, to) in windows_of(&runs) {
        assert_reads_as_core(&store, &runs, from, to);
    }
}

/// A run's events, chained in the order they are pushed, each at the stamp
/// the case states.
struct Journal {
    run_id: String,
    previous: String,
    events: Vec<Envelope>,
}

impl Journal {
    fn new(run_id: &str) -> Journal {
        Journal {
            run_id: run_id.to_string(),
            previous: brokkr_core::canonical::ZERO_HASH.to_string(),
            events: Vec::new(),
        }
    }

    fn push(&mut self, event_type: EventType, payload: Value, at: &str) {
        let seq = self.events.len() as u64 + 1;
        let event = EnvelopeBuilder::new(event_type, payload)
            .run(&self.run_id)
            .seq(seq)
            .event_id(format!("{}-{seq}", self.run_id))
            .at(at)
            .previous(self.previous.clone())
            .sealed();
        self.previous = event.event_hash.clone();
        self.events.push(event);
    }

    fn started(&mut self, at: &str, realm: Option<&str>) {
        self.started_recipe(at, realm, None);
    }

    fn started_recipe(&mut self, at: &str, realm: Option<&str>, recipe: Option<&str>) {
        let mut manifest = match realm {
            Some(realm) => json!({"realms": {
                "source": "soil", "realm": realm,
                "map": {"realms": []}
            }}),
            None => json!({}),
        };
        if let (Some(recipe), Some(object)) = (recipe, manifest.as_object_mut()) {
            object.insert("bundle_name".into(), json!(recipe));
        }
        self.push(
            EventType::RunStarted,
            json!({"feature": "a feature", "manifest": manifest}),
            at,
        );
    }

    fn phase(&mut self, at: &str, phase: &str) {
        self.push(EventType::PhaseEntered, json!({"phase": phase}), at);
    }

    fn effect(&mut self, at: &str, effect: &str, attempt: &str, provider: &str) {
        self.effect_with(at, effect, attempt, &[(None, provider)]);
    }

    /// The effect is requested, with no start yet: the request its
    /// checkpoint or terminal would otherwise precede.
    fn requested(&mut self, at: &str, effect: &str) {
        self.push(
            EventType::EffectRequested,
            json!({"effect_id": effect, "seat": "implementer"}),
            at,
        );
    }

    /// One attempt starts with `members`' provenance entries, each naming
    /// its member and provider: a panel's two sites under one attempt.
    fn effect_with(
        &mut self,
        at: &str,
        effect: &str,
        attempt: &str,
        members: &[(Option<&str>, &str)],
    ) {
        self.push(
            EventType::EffectRequested,
            json!({"effect_id": effect, "seat": "implementer"}),
            at,
        );
        let provenance: Vec<Value> = members
            .iter()
            .map(|(member, provider)| json!({"member": member, "provider": provider}))
            .collect();
        self.push(
            EventType::EffectStarted,
            json!({"effect_id": effect, "attempt_id": attempt,
                   "provenance": provenance}),
            at,
        );
    }

    fn checkpoint(&mut self, at: &str, effect: &str, attempt: &str, record: Value) {
        self.push(
            EventType::EffectCheckpointed,
            json!({"effect_id": effect, "attempt_id": attempt, "checkpoint": record}),
            at,
        );
    }

    /// The effect succeeds: a control event that moves the run's head
    /// without writing a checkpoint fact.
    fn succeeded(&mut self, at: &str, effect: &str, attempt: &str) {
        self.push(
            EventType::EffectSucceeded,
            json!({"effect_id": effect, "attempt_id": attempt, "result": {}}),
            at,
        );
    }

    /// The effect fails: a control event that returns the effect to the
    /// executable position, so a later attempt may start.
    fn failed(&mut self, at: &str, effect: &str, attempt: &str) {
        self.push(
            EventType::EffectFailed,
            json!({"effect_id": effect, "attempt_id": attempt}),
            at,
        );
    }

    /// The operator's run completes: the event that finishes it, at a stamp
    /// after its last fact.
    fn completed(&mut self, at: &str) {
        self.push(EventType::RunCompleted, json!({}), at);
    }

    /// Plant the journal and open it read-write, so every projection is
    /// the writer's, and the events are returned for the oracle.
    fn plant(self) -> (tempfile::TempDir, Store, Vec<Envelope>) {
        let (dir, store) = crate::tests::store();
        plant_older_writer(&store, &self.run_id, &self.events);
        drop(store);
        let store = refold(&dir);
        (dir, store, self.events)
    }
}

/// The milliseconds of an RFC3339 stamp the cases state.
fn ms(stamp: &str) -> i64 {
    parse_millis(stamp).unwrap()
}

/// One hand-built journal's case: its run and events, and the windows that
/// cut through its sites.
struct Case {
    runs: Vec<(String, Vec<Envelope>)>,
    windows: Vec<(i64, i64)>,
}

/// `journal` planted and refolded as one case over `windows`.
fn case(journal: Journal, windows: Vec<(i64, i64)>) -> Case {
    let run_id = journal.run_id.clone();
    let (_, store, events) = journal.plant();
    drop(store);
    Case {
        runs: vec![(run_id, events)],
        windows,
    }
}

/// `run_id`'s journal through its first attempt's first checkpoint: it
/// starts in `hearth-a`, enters `implement`, attempt `a` starts for
/// `alpha` at :02, and `record` lands at :03.
fn journal_through_first_checkpoint(run_id: &str, record: Value) -> Journal {
    let mut journal = Journal::new(run_id);
    journal.started("2026-01-01T00:00:00Z", Some("hearth-a"));
    journal.phase("2026-01-01T00:00:01Z", "implement");
    journal.effect("2026-01-01T00:00:02Z", "fx", "a", "alpha");
    journal.checkpoint("2026-01-01T00:00:03Z", "fx", "a", record);
    journal
}

/// A re-opened attempt's two sites: the same attempt and member opened
/// twice, each site with a checkpoint after its own start.
fn reopened_case() -> Case {
    let mut journal = journal_through_first_checkpoint(
        "reopened",
        json!({"step": "seat-turn", "model": "m1", "input_tokens": 10}),
    );
    journal.effect("2026-01-01T00:00:04Z", "fx", "a", "beta");
    journal.checkpoint(
        "2026-01-01T00:00:05Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "model": "m2", "input_tokens": 20,
               "total_cost_usd": 0.25}),
    );
    case(
        journal,
        vec![
            (ms("2026-01-01T00:00:02Z"), ms("2026-01-01T00:00:03Z")),
            (ms("2026-01-01T00:00:02Z"), ms("2026-01-01T00:00:06Z")),
            (ms("2026-01-01T00:00:04Z"), ms("2026-01-01T00:00:06Z")),
        ],
    )
}

/// A site started before the window that meters inside it, and a site
/// started inside whose checkpoint falls after it.
fn straddles_case() -> Case {
    let mut journal = Journal::new("straddles");
    journal.started("2026-01-01T00:00:00Z", Some("hearth-a"));
    journal.phase("2026-01-01T00:00:01Z", "implement");
    journal.effect("2026-01-01T00:00:02Z", "fx", "a", "alpha");
    journal.checkpoint(
        "2026-01-01T00:00:10Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "input_tokens": 7, "output_tokens": 3}),
    );
    journal.effect("2026-01-01T00:00:20Z", "fx", "b", "beta");
    journal.checkpoint(
        "2026-01-01T00:00:30Z",
        "fx",
        "b",
        json!({"step": "seat-turn", "input_tokens": 5}),
    );
    case(
        journal,
        vec![
            (ms("2026-01-01T00:00:09Z"), ms("2026-01-01T00:00:11Z")),
            (ms("2026-01-01T00:00:19Z"), ms("2026-01-01T00:00:21Z")),
            (ms("2026-01-01T00:00:19Z"), ms("2026-01-01T00:00:31Z")),
        ],
    )
}

/// A panel: one attempt's two named members, each its own site, with
/// interleaved checkpoints. Only one reports a cost, so a lookup that
/// ignores the member folds both members' tokens into the last site.
fn panel_case() -> Case {
    let mut journal = Journal::new("panel");
    journal.started("2026-01-01T00:00:00Z", Some("hearth-a"));
    journal.phase("2026-01-01T00:00:01Z", "implement");
    journal.effect_with(
        "2026-01-01T00:00:03Z",
        "fx",
        "a",
        &[(Some("alpha"), "alpha"), (Some("beta"), "beta")],
    );
    journal.checkpoint(
        "2026-01-01T00:00:04Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "member": "alpha", "input_tokens": 10}),
    );
    journal.checkpoint(
        "2026-01-01T00:00:05Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "member": "beta", "input_tokens": 7,
               "total_cost_usd": 0.25}),
    );
    journal.checkpoint(
        "2026-01-01T00:00:06Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "member": "alpha", "input_tokens": 20}),
    );
    case(
        journal,
        vec![(ms("2026-01-01T00:00:04Z"), ms("2026-01-01T00:00:07Z"))],
    )
}

/// A checkpoint before any start is in no site's facts, and its cost
/// counts as a spend. Its effect is requested before it, so the run's seat
/// head stands with no site row: the checkpoint is the run's whole fact,
/// and the spend must be served by the indexed read, not recovered by a
/// fallback.
fn before_any_start_case() -> Case {
    let mut journal = Journal::new("before-any-start");
    journal.started("2026-01-01T00:00:00Z", None);
    journal.requested("2026-01-01T00:00:00Z", "fx");
    journal.checkpoint(
        "2026-01-01T00:00:01Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "total_cost_usd": 0.5, "input_tokens": 4}),
    );
    case(
        journal,
        vec![(ms("2026-01-01T00:00:00Z"), ms("2026-01-01T00:00:02Z"))],
    )
}

/// The three metering rules: a turn (one repeated, which meters nothing), a
/// session record alone, and a settling Claude session record whose turns
/// count only their input.
fn metering_case() -> Case {
    let mut journal = Journal::new("metering");
    journal.started("2026-01-01T00:00:00Z", Some("hearth-b"));
    journal.phase("2026-01-01T00:00:01Z", "implement");
    journal.effect("2026-01-01T00:00:02Z", "turns", "t", "alpha");
    journal.checkpoint(
        "2026-01-01T00:00:03Z",
        "turns",
        "t",
        json!({"step": "seat-turn", "input_tokens": 100, "output_tokens": 2,
               "total_cost_usd": 0.5}),
    );
    journal.checkpoint(
        "2026-01-01T00:00:04Z",
        "turns",
        "t",
        json!({"step": "seat-turn", "input_tokens": 100, "output_tokens": 2}),
    );
    journal.effect("2026-01-01T00:00:05Z", "session", "s", "beta");
    journal.checkpoint(
        "2026-01-01T00:00:06Z",
        "session",
        "s",
        json!({"step": "codex-session-finished", "input_tokens": 300, "output_tokens": 40}),
    );
    journal.effect("2026-01-01T00:00:07Z", "settled", "c", "gamma");
    journal.checkpoint(
        "2026-01-01T00:00:08Z",
        "settled",
        "c",
        json!({"step": "seat-turn", "input_tokens": 100, "output_tokens": 2,
               "cache_read_tokens": 80, "reasoning_output_tokens": 1}),
    );
    journal.checkpoint(
        "2026-01-01T00:00:09Z",
        "settled",
        "c",
        json!({"step": "claude-code-session-finished", "output_tokens": 40,
               "reasoning_output_tokens": 30}),
    );
    case(
        journal,
        vec![(ms("2026-01-01T00:00:00Z"), ms("2026-01-01T00:00:10Z"))],
    )
}

/// An older attempt's checkpoint lands after a newer attempt of the same
/// effect starts: the row names the older attempt, so it belongs to the
/// site that attempt opened, never the newer attempt's site that opened
/// later.
fn older_attempt_case() -> Case {
    let mut journal = journal_through_first_checkpoint(
        "older-attempt",
        json!({"step": "seat-turn", "input_tokens": 1}),
    );
    journal.failed("2026-01-01T00:00:04Z", "fx", "a");
    journal.push(
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "b",
               "provenance": [{"member": null, "provider": "beta"}]}),
        "2026-01-01T00:00:05Z",
    );
    journal.checkpoint(
        "2026-01-01T00:00:07Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "input_tokens": 10, "total_cost_usd": 0.5}),
    );
    case(
        journal,
        vec![(ms("2026-01-01T00:00:06Z"), ms("2026-01-01T00:00:08Z"))],
    )
}

/// Every hand-built journal's case.
fn hand_built() -> Vec<Case> {
    vec![
        reopened_case(),
        straddles_case(),
        before_any_start_case(),
        panel_case(),
        metering_case(),
        older_attempt_case(),
    ]
}

/// `case`'s runs planted and folded in a fresh journal.
fn plant_case(case: &Case) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("forge.db");
    let store = Store::open(&path).unwrap();
    for (run_id, events) in &case.runs {
        plant_older_writer(&store, run_id, events);
    }
    drop(store);
    (dir, Store::open(&path).unwrap())
}

/// Every hand-built journal reads in its windows as brokkr-core reads it
/// whole.
#[test]
fn every_hand_built_journal_reads_as_its_fold() {
    for case in hand_built() {
        let (_dir, store) = plant_case(&case);
        for (from, to) in case.windows {
            assert_reads_as_core(&store, &case.runs, from, to);
        }
    }
}

/// One attempt's two named members keep their own sites: each member's
/// in-window checkpoints fold into the site its provenance opened, never
/// into the other's, so a member's tokens and reported cost stay its own.
#[test]
fn each_named_member_folds_only_its_own_checkpoints() {
    let case = panel_case();
    let (_dir, store) = plant_case(&case);
    let mut read = Vec::new();
    store
        .window_sites(
            ms("2026-01-01T00:00:04Z"),
            ms("2026-01-01T00:00:07Z"),
            None,
            SiteOrder::Model,
            |site| {
                read.push((
                    site.provider.map(str::to_string),
                    site.reported,
                    site.usage.as_ref().and_then(|usage| usage.input_tokens),
                ));
            },
        )
        .unwrap();
    assert_eq!(
        read,
        [
            (Some("alpha".to_string()), false, Some(30)),
            (Some("beta".to_string()), true, Some(7)),
        ]
    );
}

/// An older attempt's checkpoint after a newer attempt starts is the older
/// attempt's site's fact: the site lookup reads the checkpoint's attempt,
/// not the newest site that opened before its seq, so the newer attempt's
/// site is never served in a window it did not run in.
#[test]
fn chief_older_attempt_checkpoint_after_a_newer_start() {
    let case = older_attempt_case();
    let (_dir, store) = plant_case(&case);
    let from = ms("2026-01-01T00:00:06Z");
    let to = ms("2026-01-01T00:00:08Z");
    for order in [
        SiteOrder::Model,
        SiteOrder::Provider,
        SiteOrder::RealmProvider,
        SiteOrder::RecipeProvider,
    ] {
        let sites = read_sites(&store, from, to, order);
        assert_eq!(sites.len(), 1, "{order:?}: {sites:?}");
        let site = &sites[0];
        assert_eq!(site.opened, 1, "{order:?}: {sites:?}");
        assert_eq!(
            site.provider.as_deref(),
            Some("alpha"),
            "{order:?}: {sites:?}"
        );
        assert_eq!(site.metering, Metering::Turns, "{order:?}: {sites:?}");
        assert!(site.reported, "{order:?}: {sites:?}");
        assert_eq!(
            site.usage.as_ref().and_then(|usage| usage.input_tokens),
            Some(10),
            "{order:?}: {sites:?}"
        );
    }
}

/// Plant `journals` and open them read-write, so every projection is the
/// writer's.
fn plant_runs(journals: Vec<Journal>) -> (tempfile::TempDir, Store) {
    let (dir, store) = crate::tests::store();
    for journal in journals {
        plant_older_writer(&store, &journal.run_id, &journal.events);
    }
    drop(store);
    let store = refold(&dir);
    (dir, store)
}

/// `dir`'s journal opened read-write, so every projection is the writer's,
/// and held to its fold.
fn refold(dir: &tempfile::TempDir) -> Store {
    let store = Store::open(&dir.path().join("forge.db")).unwrap();
    assert_eq!(
        crate::test_support::every_run_reads_as_its_fold(&store),
        Vec::<String>::new()
    );
    store
}

/// The runs the order test folds: two sites of one run in one realm and
/// recipe, a second run in that realm with another recipe, one whose run
/// named no realm, and one in a third realm with another recipe. The
/// realms, recipes, providers and models are chosen so every order is a
/// different sequence, and so removing any one key from its order changes
/// it.
fn ordered_journal() -> Vec<Journal> {
    let mut one = Journal::new("r1");
    one.started_recipe("2026-01-01T00:00:00Z", Some("zeta"), Some("bbb"));
    one.phase("2026-01-01T00:00:01Z", "implement");
    one.effect("2026-01-01T00:00:02Z", "f1", "a1", "gamma");
    one.checkpoint(
        "2026-01-01T00:00:03Z",
        "f1",
        "a1",
        json!({"step": "seat-turn", "model": "m-2", "input_tokens": 1,
               "total_cost_usd": 0.1}),
    );
    one.effect("2026-01-01T00:00:04Z", "f2", "a2", "alpha");
    one.checkpoint(
        "2026-01-01T00:00:05Z",
        "f2",
        "a2",
        json!({"step": "seat-turn", "model": "m-4", "input_tokens": 2,
               "total_cost_usd": 0.2}),
    );
    let mut two = Journal::new("r2");
    two.started_recipe("2026-01-01T00:00:00Z", Some("zeta"), Some("aaa"));
    two.phase("2026-01-01T00:00:01Z", "implement");
    two.effect("2026-01-01T00:00:06Z", "f3", "a3", "zeta");
    two.checkpoint(
        "2026-01-01T00:00:07Z",
        "f3",
        "a3",
        json!({"step": "seat-turn", "model": "m-1", "input_tokens": 3,
               "total_cost_usd": 0.3}),
    );
    let mut none = Journal::new("r3");
    none.started_recipe("2026-01-01T00:00:00Z", None, None);
    none.phase("2026-01-01T00:00:01Z", "implement");
    none.effect("2026-01-01T00:00:08Z", "f4", "a4", "null-provider");
    none.checkpoint(
        "2026-01-01T00:00:09Z",
        "f4",
        "a4",
        json!({"step": "seat-turn", "model": "m-3", "input_tokens": 4,
               "total_cost_usd": 0.4}),
    );
    let mut other = Journal::new("r5");
    other.started_recipe("2026-01-01T00:00:00Z", Some("aaa"), Some("zzz"));
    other.phase("2026-01-01T00:00:01Z", "implement");
    other.effect("2026-01-01T00:00:10Z", "f5", "a5", "omega");
    other.checkpoint(
        "2026-01-01T00:00:11Z",
        "f5",
        "a5",
        json!({"step": "seat-turn", "model": "m-5", "input_tokens": 5,
               "total_cost_usd": 0.5}),
    );
    vec![one, two, none, other]
}

/// Each order is its cut key's order, then the run and the site; a site
/// that both started and metered in the window is yielded once. A NULL realm
/// sorts before every named one, as the stored key's order does. Every key
/// each order names moves the sequence it is read in.
#[test]
fn each_order_is_its_key_order_and_each_site_appears_once() {
    let (_dir, store) = plant_runs(ordered_journal());
    let from = ms("2026-01-01T00:00:00Z");
    let to = ms("2026-01-01T00:00:12Z");
    let expected = [
        (
            SiteOrder::Model,
            [
                ("r2", "zeta"),
                ("r1", "gamma"),
                ("r3", "null-provider"),
                ("r1", "alpha"),
                ("r5", "omega"),
            ],
        ),
        (
            SiteOrder::Provider,
            [
                ("r1", "alpha"),
                ("r1", "gamma"),
                ("r3", "null-provider"),
                ("r5", "omega"),
                ("r2", "zeta"),
            ],
        ),
        (
            SiteOrder::RealmProvider,
            [
                ("r3", "null-provider"),
                ("r5", "omega"),
                ("r1", "alpha"),
                ("r1", "gamma"),
                ("r2", "zeta"),
            ],
        ),
        (
            SiteOrder::RecipeProvider,
            [
                ("r3", "null-provider"),
                ("r5", "omega"),
                ("r2", "zeta"),
                ("r1", "alpha"),
                ("r1", "gamma"),
            ],
        ),
    ];
    for (order, sequence) in expected {
        let mut read = Vec::new();
        store
            .window_sites(from, to, None, order, |site| {
                read.push((site.run_id.to_string(), site.provider.unwrap().to_string()));
            })
            .unwrap();
        let expected: Vec<(String, String)> = sequence
            .into_iter()
            .map(|(run, provider)| (run.to_string(), provider.to_string()))
            .collect();
        assert_eq!(read, expected, "{order:?}");
    }
}

/// `realm` narrows both reads to one stored realm key, and none narrows
/// neither.
#[test]
fn a_realm_narrows_both_reads() {
    let mut alpha = Journal::new("alpha-run");
    alpha.started_recipe("2026-01-01T00:00:00Z", Some("alpha"), Some("one"));
    alpha.phase("2026-01-01T00:00:01Z", "implement");
    alpha.effect("2026-01-01T00:00:02Z", "f1", "a1", "gamma");
    alpha.checkpoint(
        "2026-01-01T00:00:03Z",
        "f1",
        "a1",
        json!({"step": "seat-turn", "input_tokens": 1, "total_cost_usd": 0.1}),
    );
    let mut beta = Journal::new("beta-run");
    beta.started_recipe("2026-01-01T00:00:00Z", Some("beta"), Some("two"));
    beta.phase("2026-01-01T00:00:01Z", "implement");
    beta.effect("2026-01-01T00:00:02Z", "f2", "a2", "beta");
    beta.checkpoint(
        "2026-01-01T00:00:03Z",
        "f2",
        "a2",
        json!({"step": "seat-turn", "input_tokens": 2, "total_cost_usd": 0.2}),
    );
    let (_dir, store) = plant_runs(vec![alpha, beta]);
    let from = ms("2026-01-01T00:00:00Z");
    let to = ms("2026-01-01T00:00:04Z");

    let runs = |store: &Store, realm: Option<&str>, spends: bool| -> Vec<String> {
        let mut found = Vec::new();
        if spends {
            store
                .window_spends(from, to, realm, |spend| {
                    found.push(spend.run_id.to_string())
                })
                .unwrap();
        } else {
            store
                .window_sites(from, to, realm, SiteOrder::Model, |site| {
                    found.push(site.run_id.to_string())
                })
                .unwrap();
        }
        found.sort();
        found
    };
    for spends in [false, true] {
        assert_eq!(
            runs(&store, None, spends),
            ["alpha-run".to_string(), "beta-run".to_string()],
            "no realm narrows neither, spends {spends}"
        );
        assert_eq!(
            runs(&store, Some("alpha"), spends),
            ["alpha-run".to_string()],
            "alpha alone, spends {spends}"
        );
        assert_eq!(
            runs(&store, Some("nowhere"), spends),
            Vec::<String>::new(),
            "a realm no run named, spends {spends}"
        );
    }
}

/// A lagging run is left to the whole-run fallback: while its summary row
/// sits behind its head, neither indexed read emits one of its sites or
/// spends, so a caller that reads that run whole counts them once
/// (decision 0089 ruling 8).
#[test]
fn a_lagging_run_is_left_to_the_whole_run_fallback() {
    let journal = site_and_checkpoint_journal("lagging");
    let (dir, mut store, _) = journal.plant();
    store
        .conn
        .execute(
            "UPDATE run_summary SET aggregated_seq = 0 WHERE run_id = 'lagging'",
            [],
        )
        .unwrap();
    assert_eq!(
        crate::test_support::lane(&store, "lagging").as_deref(),
        Some("behind")
    );
    // A writable open refolds the behind lane; a read-only one preserves
    // the lag the caller must fall back on.
    crate::test_support::lags(&mut store, &["lagging"]);
    let path = dir.path().join("forge.db");
    drop(store);
    let reader = Store::open_read_only(&path).unwrap();
    let from = ms("2026-01-01T00:00:00Z");
    let to = ms("2026-01-01T00:01:00Z");

    assert_window_reads_empty(&reader, from, to);
}

/// Every read's plan names its index and scans no projection whole.
#[test]
fn the_plans_name_the_indexes_and_scan_no_projection() {
    let (_dir, store) = plant_runs(ordered_journal());
    // The detector's own control: a whole scan under the statements' aliases
    // is what the negative assertions must be able to see.
    let whole = crate::test_support::plan(
        &store,
        "SELECT * FROM event_index AS e",
        rusqlite::params![],
    )
    .unwrap();
    assert!(scans(&whole, "event_index"), "{whole:?}");
    let whole = crate::test_support::plan(
        &store,
        "SELECT * FROM seat_summary AS s",
        rusqlite::params![],
    )
    .unwrap();
    assert!(scans(&whole, "seat_summary"), "{whole:?}");
    let spends = crate::test_support::plan(
        &store,
        &SPENDS,
        rusqlite::params![FROM, TO, None::<&str>, crate::seat_summary::SHAPE.as_str()],
    )
    .unwrap();
    assert!(
        spends
            .iter()
            .any(|step| step.contains("event_index_checkpoint")),
        "{spends:?}"
    );
    assert!(
        !scans(&spends, "event_index") && !scans(&spends, "seat_summary"),
        "{spends:?}"
    );
    let falls = crate::test_support::plan(
        &store,
        &FALLBACKS,
        rusqlite::params![None::<&str>, crate::seat_summary::SHAPE.as_str()],
    )
    .unwrap();
    assert!(
        falls
            .iter()
            .any(|step| step.contains("event_index_checkpoint_stale")),
        "{falls:?}"
    );
    // The fallback scans the journal only through the stale index, never
    // the table, and no projection whole.
    assert!(
        falls.iter().all(|step| {
            !(step.starts_with("SCAN e ") || step == "SCAN e")
                || step.contains("event_index_checkpoint_stale")
        }) && !scans(&falls, "seat_summary"),
        "{falls:?}"
    );
    for order in [
        SiteOrder::Model,
        SiteOrder::Provider,
        SiteOrder::RealmProvider,
        SiteOrder::RecipeProvider,
    ] {
        let sites = crate::test_support::plan(
            &store,
            sites_sql(order),
            rusqlite::params![FROM, TO, None::<&str>, crate::seat_summary::SHAPE.as_str()],
        )
        .unwrap();
        for index in [
            "event_index_checkpoint",
            "seat_summary_started",
            "seat_summary_site",
        ] {
            assert!(
                sites.iter().any(|step| step.contains(index)),
                "{order:?}: {index} in {sites:?}"
            );
        }
        assert!(
            !scans(&sites, "event_index") && !scans(&sites, "seat_summary"),
            "{order:?}: {sites:?}"
        );
    }
}

/// Whether `plan` scans `table` whole, by the table's name or the alias the
/// reads give it (the statements alias both projections).
fn scans(plan: &[String], table: &str) -> bool {
    let aliases: &[&str] = match table {
        "event_index" => &["e"],
        "seat_summary" => &["s", "s2"],
        _ => &[],
    };
    plan.iter().any(|step| {
        let Some(rest) = step.strip_prefix("SCAN ") else {
            return false;
        };
        let scanned = rest.split_whitespace().next().unwrap_or_default();
        scanned == table || aliases.contains(&scanned)
    })
}

/// The milliseconds in a day, and the year of facts a window does not read.
const DAY: i64 = 24 * 60 * 60 * 1_000;

/// The window the cost test holds fixed.
const FROM: i64 = 1_767_225_600_000;
const TO: i64 = FROM + 60_000;

/// A `run_summary` row in the running lane, its aggregates and stamps
/// current, as the writer leaves it: every stamp equals the folded seq, and
/// its seat head names this build's shape, so the lane and the seat rule
/// admit the row to the indexed reads.
fn plant_run(conn: &rusqlite::Connection, run_id: &str, realm: &str) {
    conn.execute(
        "INSERT INTO run_summary
            (run_id, head_seq, head_hash, folded_seq, folded_hash, status, realm, recipe,
             aggregated_seq, stop_folded_seq, ending_folded_seq)
         VALUES (?1, 0, 'h', 0, 'h', 'running', ?2, 'one', 0, 0, 0)",
        rusqlite::params![run_id, realm],
    )
    .unwrap();
    conn.execute(
        "INSERT INTO seat_head (run_id, folded_seq, folded_hash, shape)
         VALUES (?1, 0, 'h', ?2)",
        rusqlite::params![run_id, crate::seat_summary::SHAPE.as_str()],
    )
    .unwrap();
}

/// One site row of `run_id`, opened at `opened`, started at `started_ms`.
fn plant_site(conn: &rusqlite::Connection, run_id: &str, opened: i64, started_ms: i64) {
    conn.execute(
        "INSERT INTO seat_summary
            (run_id, opened, attempt_id, member, started_seq, started_ms,
             phase_key, provider_key, model_key, metering, site, digest)
         VALUES (?1, ?2, 'a', NULL, 1, ?3, 'implement', 'alpha', 'm', 1, ?2, 'd')",
        rusqlite::params![run_id, opened, started_ms],
    )
    .unwrap();
}

/// One checkpoint row of `run_id`'s attempt `a`, at `seq` and `at_ms`, its
/// facts current: the insert's own trigger marks a checkpoint's facts
/// behind until its writer computes them, so the test writes the stamp the
/// writer would, keeping the row current for the currentness rules.
fn plant_checkpoint(conn: &rusqlite::Connection, run_id: &str, seq: i64, at_ms: i64) {
    conn.execute(
        "INSERT INTO event_index
            (run_id, seq, type, attempt_id, member, at_ms, reported, turn, session,
             repeat, input_tokens, output_tokens, usd, facts, facts_seq)
         VALUES (?1, ?2, 'effect/checkpointed', 'a', NULL, ?3, 1, 1, 0, 0, 1, 0, 0.5, 1, ?2)",
        rusqlite::params![run_id, seq, at_ms],
    )
    .unwrap();
    conn.execute(
        "UPDATE event_index SET facts_seq = ?2 WHERE run_id = ?1 AND seq = ?2",
        rusqlite::params![run_id, seq],
    )
    .unwrap();
}

/// A current run with one site at `FROM + 10` and one checkpoint at
/// `FROM + 20`: the one site and spend the fixed window serves.
fn plant_window_run(conn: &rusqlite::Connection, run_id: &str) {
    plant_run(conn, run_id, "alpha");
    plant_site(conn, run_id, 0, FROM + 10);
    plant_checkpoint(conn, run_id, 2, FROM + 20);
}

/// The year of sites and checkpoints a fixed window does not read.
fn plant_far(conn: &rusqlite::Connection, before: bool) {
    for index in 0..3i64 {
        let at = if before {
            FROM - (index + 1) * 120 * DAY
        } else {
            TO + (index + 1) * 30 * DAY
        };
        let run_id = format!("{}-{index}", if before { "earlier" } else { "later" });
        plant_run(conn, &run_id, "alpha");
        plant_site(conn, &run_id, 0, at);
        plant_checkpoint(conn, &run_id, 2, at + 1);
    }
}

/// A window visits only the sites and spends near it: a year of earlier
/// and a year of later facts leave each read's VM-step count equal, k
/// checkpoints inside the window each add one visit, and a year of earlier
/// and later sites adds none.
///
/// One site and checkpoint stand just past the window, so the baseline
/// already pays the step that meets the range's upper bound: adding a year
/// of later facts then measures what the read visits, not how SQLite
/// reaches its bound.
#[test]
fn a_window_visits_only_the_sites_near_it() {
    let (_dir, store) = crate::tests::store();
    plant_window_run(&store.conn, "window");
    let beyond = TO + 1;
    plant_run(&store.conn, "beyond", "alpha");
    plant_site(&store.conn, "beyond", 0, beyond);
    plant_checkpoint(&store.conn, "beyond", 2, beyond);

    let spends = store.window_spends_steps(FROM, TO, None).unwrap();
    let sites: Vec<i64> = [
        SiteOrder::Model,
        SiteOrder::Provider,
        SiteOrder::RealmProvider,
        SiteOrder::RecipeProvider,
    ]
    .into_iter()
    .map(|order| store.window_sites_steps(FROM, TO, None, order).unwrap())
    .collect();
    assert!(spends > 0 && sites.iter().all(|steps| *steps > 0));

    plant_far(&store.conn, true);
    plant_far(&store.conn, false);
    assert_eq!(
        store.window_spends_steps(FROM, TO, None).unwrap(),
        spends,
        "a year of earlier and later spends is in no range"
    );
    for (order, step) in [
        SiteOrder::Model,
        SiteOrder::Provider,
        SiteOrder::RealmProvider,
        SiteOrder::RecipeProvider,
    ]
    .into_iter()
    .zip(sites.iter())
    {
        assert_eq!(
            store.window_sites_steps(FROM, TO, None, order).unwrap(),
            *step,
            "{order:?}: a year of earlier and later sites is in no range"
        );
    }

    // k checkpoints inside the window, each a row of the one site already
    // there: each read grows by exactly k visits.
    let k = 10i64;
    let before_spends = store.window_spends_steps(FROM, TO, None).unwrap();
    let before_sites = store
        .window_sites_steps(FROM, TO, None, SiteOrder::Model)
        .unwrap();
    plant_checkpoint(&store.conn, "window", 3, FROM + 21);
    let one_spend = store.window_spends_steps(FROM, TO, None).unwrap() - before_spends;
    let one_site = store
        .window_sites_steps(FROM, TO, None, SiteOrder::Model)
        .unwrap()
        - before_sites;
    assert!(one_spend > 0 && one_site > 0);
    for index in 1..k {
        plant_checkpoint(&store.conn, "window", 3 + index, FROM + 21 + index);
    }
    assert_eq!(
        store.window_spends_steps(FROM, TO, None).unwrap() - before_spends,
        one_spend * k,
        "k window spends each cost one visit"
    );
    assert_eq!(
        store
            .window_sites_steps(FROM, TO, None, SiteOrder::Model)
            .unwrap()
            - before_sites,
        one_site * k,
        "k window checkpoints each cost one visit"
    );
}

/// A run with a site at :10 and a checkpoint at :20: the five events a
/// summary writer leaves folded before it appends its sixth.
fn site_and_checkpoint_journal(run_id: &str) -> Journal {
    let mut journal = Journal::new(run_id);
    journal.started("2026-01-01T00:00:00Z", Some("alpha"));
    journal.phase("2026-01-01T00:00:01Z", "implement");
    journal.effect("2026-01-01T00:00:10Z", "fx", "a", "alpha");
    journal.checkpoint(
        "2026-01-01T00:00:20Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "input_tokens": 10, "total_cost_usd": 0.5}),
    );
    journal
}

/// `journal`'s first five events planted and folded, then its sixth
/// appended by a writer from between the summary and the seat summary, and
/// the journal reopened read-only so the lag stands.
fn plant_stale_projection(journal: &Journal) -> (tempfile::TempDir, Store) {
    let (dir, store) = crate::tests::store();
    plant_older_writer(&store, &journal.run_id, &journal.events[..5]);
    drop(store);
    let path = dir.path().join("forge.db");
    let store = Store::open(&path).unwrap();
    crate::test_support::plant_summary_writer(&store, &journal.run_id, &journal.events[5..]);
    drop(store);
    (dir, Store::open_read_only(&path).unwrap())
}

/// `journal`'s events planted and its summary row written by a writer from
/// between the summary and the seat summary, with no seat head and no seat
/// rows at all, and the journal reopened read-only so the absence stands.
fn plant_unprojected(journal: &Journal) -> (tempfile::TempDir, Store) {
    let (dir, store) = crate::tests::store();
    crate::test_support::plant_run(&store.conn, &journal.run_id, "{}").expect("a run plants");
    crate::test_support::plant_summary_writer(&store, &journal.run_id, &journal.events);
    drop(store);
    let path = dir.path().join("forge.db");
    (dir, Store::open_read_only(&path).unwrap())
}

/// Neither window read emits a row for `from..to`, in every order.
fn assert_window_reads_empty(store: &Store, from: i64, to: i64) {
    assert!(
        read_spends(store, from, to).is_empty(),
        "the stale run's spend is whole-read"
    );
    for order in [
        SiteOrder::Model,
        SiteOrder::Provider,
        SiteOrder::RealmProvider,
        SiteOrder::RecipeProvider,
    ] {
        assert!(
            read_sites(store, from, to, order).is_empty(),
            "{order:?}: the stale run's site is whole-read"
        );
    }
}

/// Core's reading of `runs` in `from..to` holds one site, opened 1, that
/// `reader` serves by no indexed read: it names `run_id` whole instead.
fn assert_one_site_left_whole(
    reader: &Store,
    runs: &[(String, Vec<Envelope>)],
    from: i64,
    to: i64,
    run_id: &str,
) {
    let (_, sites) = core_reads(runs, from, to);
    assert_eq!(sites.len(), 1, "{sites:?}");
    assert_eq!(sites[0].opened, 1, "{sites:?}");
    assert_window_reads_empty(reader, from, to);
    assert_eq!(fallbacks(reader, from, to), [run_id.to_string()]);
}

/// The runs `window_fallbacks` names for `from..to`, in its order.
fn fallbacks(store: &Store, from: i64, to: i64) -> Vec<String> {
    let mut named = Vec::new();
    store
        .window_fallbacks(from, to, None, |run| named.push(run.run_id.to_string()))
        .unwrap();
    named
}

/// A run whose checkpoint facts lag its summary head, while its lane still
/// reads `running`, is left to the whole-run fallback: the window reads
/// emit none of its rows, `window_fallbacks` names it, and the whole read
/// the fallback makes holds the spend and the site's in-window facts
/// (decision 0089 ruling 8).
#[test]
fn a_stale_checkpoint_run_is_left_to_the_whole_run_fallback() {
    let mut journal = site_and_checkpoint_journal("stale-checkpoint");
    journal.checkpoint(
        "2026-01-01T00:00:30Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "input_tokens": 99, "total_cost_usd": 0.7}),
    );
    let (_dir, reader) = plant_stale_projection(&journal);
    assert_eq!(
        crate::test_support::lane(&reader, &journal.run_id).as_deref(),
        Some("running"),
        "the lane alone would admit it"
    );

    let runs = vec![(journal.run_id.clone(), journal.events.clone())];
    let from = ms("2026-01-01T00:00:25Z");
    let to = ms("2026-01-01T00:00:40Z");
    let (spends, sites) = core_reads(&runs, from, to);
    assert!(
        spends
            .iter()
            .any(|spend| spend.run_id == "stale-checkpoint" && close(spend.usd, 0.7)),
        "{spends:?}"
    );
    let folded = sites
        .iter()
        .find(|site| site.run_id == "stale-checkpoint")
        .expect("the checkpoint's site is in the window");
    assert!(folded.reported);
    assert_eq!(
        folded.usage.as_ref().and_then(|usage| usage.input_tokens),
        Some(99)
    );

    assert_window_reads_empty(&reader, from, to);
    // The :20 spend and the site at :10 are current rows of the same run,
    // and are left whole too: the run's checkpoint facts lag elsewhere.
    assert_window_reads_empty(&reader, ms("2026-01-01T00:00:00Z"), to);
    assert_eq!(
        fallbacks(&reader, from, to),
        ["stale-checkpoint".to_string()]
    );
}

/// A run whose seat rows lag its summary head is left whole even when its
/// checkpoint facts are current: a site that started inside the full window
/// is not emitted by the indexed read, and `window_fallbacks` names the run
/// (decision 0089 ruling 8).
#[test]
fn a_stale_seat_run_is_left_to_the_whole_run_fallback() {
    let mut journal = site_and_checkpoint_journal("stale-seat");
    // A control event: no checkpoint fact can be stale here, only the seat
    // head can lag.
    journal.succeeded("2026-01-01T00:00:30Z", "fx", "a");
    let (_dir, reader) = plant_stale_projection(&journal);
    assert_eq!(
        crate::test_support::lane(&reader, &journal.run_id).as_deref(),
        Some("running"),
        "the lane alone would admit it"
    );

    let from = ms("2026-01-01T00:00:00Z");
    let to = ms("2026-01-01T00:00:40Z");
    let runs = vec![(journal.run_id.clone(), journal.events.clone())];
    let (_, sites) = core_reads(&runs, from, to);
    assert!(
        sites.iter().any(|site| site.run_id == "stale-seat"),
        "{sites:?}"
    );
    assert_window_reads_empty(&reader, from, to);
    assert_eq!(fallbacks(&reader, from, to), ["stale-seat".to_string()]);
}

/// A run that finished two minutes after its facts were folded is named
/// whole by the fallback: the window is not the run's finish, so the
/// indexed reads serve none of its rows and the whole read recovers the
/// site and its 0.5 once (decision 0089 ruling 8).
#[test]
fn chief_closed_run_spent_before_its_finish() {
    let mut journal = site_and_checkpoint_journal("chief-closed-stale");
    journal.completed("2026-01-01T00:02:00Z");
    let (_dir, reader) = plant_stale_projection(&journal);
    assert_eq!(
        crate::test_support::lane(&reader, &journal.run_id).as_deref(),
        Some("finished"),
        "the run finished two minutes after its facts"
    );

    let from = ms("2026-01-01T00:00:00Z");
    let to = ms("2026-01-01T00:01:00Z");
    let runs = vec![(journal.run_id.clone(), journal.events.clone())];
    let (spends, sites) = core_reads(&runs, from, to);
    assert_eq!(spends.len(), 1, "{spends:?}");
    assert!(close(spends[0].usd, 0.5), "{spends:?}");
    assert_eq!(sites.len(), 1, "{sites:?}");
    assert_eq!(sites[0].opened, 1, "{sites:?}");
    assert!(sites[0].reported, "{sites:?}");

    assert_window_reads_empty(&reader, from, to);
    assert_eq!(
        fallbacks(&reader, from, to),
        ["chief-closed-stale".to_string()],
        "the whole read recovers the site and the spend once"
    );
}

/// A run whose seat head names another build's shape has stale seat rows
/// even though the head is the summary's own: no indexed read serves it, and
/// the fallback names it whole (decision 0089 ruling 8).
#[test]
fn chief_old_projection_shape_is_left_whole() {
    let journal = site_and_checkpoint_journal("chief-old-shape");
    let (dir, store, events) = journal.plant();
    store
        .conn
        .execute(
            "UPDATE seat_head SET shape = 'old' WHERE run_id = 'chief-old-shape'",
            [],
        )
        .unwrap();
    drop(store);
    let reader = Store::open_read_only(&dir.path().join("forge.db")).unwrap();

    let from = ms("2026-01-01T00:00:00Z");
    let to = ms("2026-01-01T00:01:00Z");
    let runs = vec![("chief-old-shape".to_string(), events)];
    let (spends, _) = core_reads(&runs, from, to);
    assert_eq!(spends.len(), 1, "{spends:?}");
    assert_one_site_left_whole(&reader, &runs, from, to, "chief-old-shape");
}

/// A run whose only projected row is its summary, with no seat head and no
/// seat rows, is not current: its site started in the window though no
/// projection row holds it, so the indexed reads serve none of it and the
/// fallback names it whole (decision 0089 ruling 8).
#[test]
fn chief_started_site_with_no_projection_is_left_whole() {
    let mut journal = Journal::new("chief-unprojected");
    journal.started("2026-01-01T00:00:00Z", Some("alpha"));
    journal.phase("2026-01-01T00:00:01Z", "implement");
    journal.effect("2026-01-01T00:00:10Z", "fx", "a", "alpha");
    let (_dir, reader) = plant_unprojected(&journal);
    assert_eq!(
        crate::test_support::lane(&reader, &journal.run_id).as_deref(),
        Some("running"),
        "the summary alone reads current"
    );

    let from = ms("2026-01-01T00:00:00Z");
    let to = ms("2026-01-01T00:01:00Z");
    let runs = vec![(journal.run_id.clone(), journal.events.clone())];
    assert_one_site_left_whole(&reader, &runs, from, to, "chief-unprojected");
}

/// A stale run is named store-wide: its facts fall outside the window it is
/// asked about, and it is named all the same, while the indexed reads serve
/// none of its rows in its own window (decision 0089 ruling 8).
#[test]
fn a_stale_run_is_named_store_wide() {
    let mut journal = site_and_checkpoint_journal("outside-window");
    journal.checkpoint(
        "2026-01-01T00:00:30Z",
        "fx",
        "a",
        json!({"step": "seat-turn", "input_tokens": 99, "total_cost_usd": 0.7}),
    );
    let (_dir, reader) = plant_stale_projection(&journal);
    let early = (ms("2026-01-01T00:00:00Z"), ms("2026-01-01T00:00:05Z"));
    assert_window_reads_empty(&reader, early.0, early.1);
    assert_eq!(
        fallbacks(&reader, early.0, early.1),
        ["outside-window".to_string()],
        "no fact of the stale run falls in the window, and it is named"
    );

    let own = (ms("2026-01-01T00:00:00Z"), ms("2026-01-01T00:00:40Z"));
    assert_window_reads_empty(&reader, own.0, own.1);
    assert_eq!(
        fallbacks(&reader, own.0, own.1),
        ["outside-window".to_string()]
    );
}

/// A wholly current run is named by no fallback: the indexed reads serve its
/// one site in every order and its one spend, and `window_fallbacks` is
/// empty, so U4 never reads it whole as well (decision 0089 ruling 8).
///
/// The negative control: the fallback's selectivity is "named iff not
/// current", not "every run left out of the indexed reads". A seat arm that
/// names every non-behind run passes the cases above but fails here.
#[test]
fn chief_current_run_is_not_also_named_for_whole_read() {
    let (_dir, store) = crate::tests::store();
    plant_window_run(&store.conn, "current-window");

    let spends = read_spends(&store, FROM, TO);
    assert_eq!(spends.len(), 1, "{spends:?}");
    assert_eq!(spends[0].run_id, "current-window", "{spends:?}");
    for order in [
        SiteOrder::Model,
        SiteOrder::Provider,
        SiteOrder::RealmProvider,
        SiteOrder::RecipeProvider,
    ] {
        let sites = read_sites(&store, FROM, TO, order);
        assert_eq!(sites.len(), 1, "{order:?}: {sites:?}");
        assert_eq!(sites[0].run_id, "current-window", "{order:?}: {sites:?}");
    }
    assert!(
        fallbacks(&store, FROM, TO).is_empty(),
        "a current run is served by the indexed reads, never named whole"
    );
}

/// The fallback's realm filter narrows both arms: a run is named under the
/// realm its summary row names, NULL names every realm, and a realm no run
/// named names none. The seat arm and the checkpoint arm each keep their
/// own run's realm, so one arm cannot name the other's.
#[test]
fn a_realm_narrows_the_whole_run_fallback() {
    let (_dir, store) = crate::tests::store();
    for (run_id, realm) in [
        ("facts-alpha", "alpha"),
        ("seat-alpha", "alpha"),
        ("facts-beta", "beta"),
        ("seat-beta", "beta"),
    ] {
        plant_run(&store.conn, run_id, realm);
    }
    for run_id in ["facts-alpha", "facts-beta"] {
        plant_checkpoint(&store.conn, run_id, 2, FROM + 20);
        store
            .conn
            .execute(
                "UPDATE event_index SET facts_seq = NULL WHERE run_id = ?1 AND seq = 2",
                rusqlite::params![run_id],
            )
            .unwrap();
    }
    for run_id in ["seat-alpha", "seat-beta"] {
        store
            .conn
            .execute(
                "DELETE FROM seat_head WHERE run_id = ?1",
                rusqlite::params![run_id],
            )
            .unwrap();
    }

    let read = |realm: Option<&str>| {
        let mut named = Vec::new();
        store
            .window_fallbacks(FROM, TO, realm, |run| named.push(run.run_id.to_string()))
            .unwrap();
        named
    };
    assert_eq!(
        read(Some("alpha")),
        ["facts-alpha".to_string(), "seat-alpha".to_string()]
    );
    assert_eq!(
        read(Some("beta")),
        ["facts-beta".to_string(), "seat-beta".to_string()]
    );
    assert_eq!(
        read(None),
        [
            "facts-alpha".to_string(),
            "facts-beta".to_string(),
            "seat-alpha".to_string(),
            "seat-beta".to_string()
        ]
    );
    assert_eq!(read(Some("nowhere")), Vec::<String>::new());
}

/// The fallback's cost grows with the stale runs alone: a year of current
/// facts for a run adds no visit, while each run named grows it, so the
/// read never walks the journal's current rows (decision 0089 ruling 8).
#[test]
fn the_fallback_visits_only_the_stale_runs() {
    let (_dir, store) = crate::tests::store();
    plant_window_run(&store.conn, "window");
    let base = store.window_fallbacks_steps(None).unwrap();

    for index in 0..3i64 {
        plant_checkpoint(&store.conn, "window", 10 + index, FROM - (index + 1) * DAY);
    }
    assert_eq!(
        store.window_fallbacks_steps(None).unwrap(),
        base,
        "current facts add no visit"
    );

    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE run_id = 'window' AND seq = 2",
            [],
        )
        .unwrap();
    let one = store.window_fallbacks_steps(None).unwrap();
    assert!(one > base, "a stale run adds visits: {one} > {base}");

    plant_run(&store.conn, "stale", "alpha");
    plant_checkpoint(&store.conn, "stale", 2, FROM + 20);
    store
        .conn
        .execute(
            "UPDATE event_index SET facts_seq = NULL WHERE run_id = 'stale' AND seq = 2",
            [],
        )
        .unwrap();
    let two = store.window_fallbacks_steps(None).unwrap();
    assert!(two > one, "each stale run adds visits: {two} > {one}");
}

/// A metering rule no writer words is refused, never guessed: the site's
/// row is unreadable, so the caller reads the run whole (ruling 8).
#[test]
fn a_metering_rule_no_writer_words_is_refused() {
    let (_dir, store) = crate::tests::store();
    plant_run(&store.conn, "bad", "alpha");
    plant_site(&store.conn, "bad", 0, FROM + 10);
    store
        .conn
        .execute(
            "UPDATE seat_summary SET metering = 7 WHERE run_id = 'bad'",
            [],
        )
        .unwrap();
    let error = store
        .window_sites(FROM, TO, None, SiteOrder::Model, |_| {})
        .unwrap_err();
    assert!(
        matches!(
            error,
            StoreError::Sqlite(rusqlite::Error::IntegralValueOutOfRange(10, 7))
        ),
        "{error:?}"
    );
}
