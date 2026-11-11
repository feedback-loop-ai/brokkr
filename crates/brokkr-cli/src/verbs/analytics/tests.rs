//! `brokkr analytics`' own part (#568): the hearths it hands over, the
//! headline it paints, and clap's refusal of a range in the view's words.

use clap::Parser;

use super::*;
use crate::Cli;

const NOW: &str = "2026-10-13T12:00:00Z";

#[test]
fn a_hearth_that_will_not_load_is_handed_over_quarantined_and_painted() {
    let dir = tempfile::tempdir().unwrap();
    let ghost = Hearth {
        realms: vec!["ghost".to_string()],
        journal: dir.path().join("ghost.db"),
    };
    let view = derive(&[ghost], Range::Day, None, NOW).unwrap();
    let refusal = view.quarantined[0].refusal.clone();
    assert!(
        !dir.path().join("ghost.db").exists(),
        "a read created a journal"
    );
    assert_eq!(
        paint(&view),
        format!(
            "2026-10-12T12:00:00Z → {NOW} · all realms\n\
             shipped        0 (no change vs previous 24h)\n\
             start → end    —\n\
             waiting on you 0.0h (nothing parked)\n\
             stopped by rule 0\n\
             ledger         0 of 0 shipped (—) · 0 excluded · 0 unclassified\n\
             reported cost  not reported\n\
             quarantined    ghost: {refusal}\n\
             not recorded yet: machine capacity over time · cost limits · costs for Codex and \
             dsh · reasons findings were dismissed\n"
        )
    );
}

#[test]
fn a_named_realm_heads_the_painting_and_a_bad_clock_is_refused() {
    let view = derive(&[], Range::Week, Some("alpha"), NOW).unwrap();
    assert!(paint(&view).starts_with("2026-10-06T12:00:00Z → 2026-10-13T12:00:00Z · alpha\n"));
    let refused = derive(&[], Range::Week, None, "noon").unwrap_err();
    assert_eq!(
        refused.to_string(),
        AnalyticsError::Clock("noon".to_string()).to_string(),
        "the view's words, which its own tests pin"
    );
}

#[test]
fn clap_refuses_a_range_outside_the_three_with_the_views_error() {
    let refused = Cli::try_parse_from(["brokkr", "analytics", "--range", "2d"])
        .err()
        .expect("2d is no range");
    assert_eq!(refused.kind(), clap::error::ErrorKind::ValueValidation);
    let source = std::error::Error::source(&refused)
        .and_then(|source| source.downcast_ref::<RangeRefused>())
        .map(|refused| &refused.0);
    assert_eq!(
        source,
        Some(&AnalyticsError::UnknownRange("2d".to_string()))
    );
}

// ------------------------------------------------------------------ rows

use brokkr_core::canonical::ZERO_HASH;
use brokkr_core::stamp::parse_millis;
use brokkr_core::summary::{Bounded, MODEL_BYTES, NAME_BYTES};
use brokkr_core::EventType;
use brokkr_store::test_support::{
    import_fixtures, lane, plant_broken_link, plant_older_writer, plant_summary_writer, unmigrate,
};
use serde_json::{json, Value};

use crate::tests::envelope_builder::EnvelopeBuilder;

const FIXTURE_NOW: &str = "2026-09-02T00:00:00Z";
const GENERATED_NOW: &str = "2026-10-13T12:00:00Z";

fn hearth(realms: Vec<String>, journal: &std::path::Path) -> Hearth {
    Hearth {
        realms,
        journal: journal.to_path_buf(),
    }
}

fn view(hearths: &[Hearth], range: Range, realm: Option<&str>, now: &str) -> Analytics {
    read_hearths(hearths, |grouped| {
        brokkr_view::analytics(&Query {
            hearths: grouped,
            range,
            realm,
            now,
        })
    })
    .unwrap()
}

/// Every figure from the rows equals today's over the same runs read
/// whole (decision 0089 ruling 9). `usd` is compared within 1e-9 relative
/// (ruling A5); every other value, printed text included, exactly.
/// Every range of `hearths`, from the rows and from the whole read, equal.
fn assert_hearths_are_their_whole_read(hearths: &[Hearth], context: &str) {
    for range in [Range::Day, Range::Week, Range::Month] {
        let rows = derive(hearths, range, None, GENERATED_NOW).unwrap();
        let whole = view(hearths, range, None, GENERATED_NOW);
        assert_same(&rows, &whole, &format!("{context} {range:?}"));
    }
}

/// The Day figures from `journal`'s rows and from the whole read, asserted
/// equal, and the rows' view for the caller's own assertions.
fn assert_rows_are_whole(journal: &std::path::Path, context: &str) -> Analytics {
    let hearths = [hearth(Vec::new(), journal)];
    let rows = derive(&hearths, Range::Day, None, GENERATED_NOW).unwrap();
    let whole = view(&hearths, Range::Day, None, GENERATED_NOW);
    assert_same(&rows, &whole, context);
    rows
}

/// Read `hearths`'s one migrated journal from its rows, with `before` run
/// between the fleet read and the store's open, and hand the query and the
/// source to `inspect` while both are live.
fn with_rows<T>(
    hearths: &[Hearth],
    range: Range,
    realm: Option<&str>,
    now: &str,
    before: impl FnOnce(),
    inspect: impl FnOnce(&Query<'_>, super::rows::HearthSource) -> T,
) -> T {
    read_hearths(hearths, |grouped| {
        before();
        let query = Query {
            hearths: grouped,
            range,
            realm,
            now,
        };
        let (window, before_window) = brokkr_view::windows(&query).unwrap();
        let store = brokkr_store::Store::open_read_only(&hearths[0].journal).unwrap();
        let source = super::rows::HearthSource::read(
            &grouped[0],
            range,
            realm,
            now,
            window,
            before_window,
            Some(&store),
        );
        inspect(&query, source)
    })
}

/// The same, for one journal at a time.
fn assert_journals_are_their_whole_read(journals: &[&std::path::Path]) {
    for journal in journals {
        assert_hearths_are_their_whole_read(
            &[hearth(Vec::new(), journal)],
            &format!("{journal:?}"),
        );
    }
}

fn assert_same(rows: &Analytics, whole: &Analytics, context: &str) {
    assert_eq!(
        rows.headline.median, whole.headline.median,
        "{context}: the median is the window's"
    );
    assert_eq!(
        rows.headline.shipped.previous, whole.headline.shipped.previous,
        "{context}: previous is the window before"
    );
    same(
        &serde_json::to_value(rows).unwrap(),
        &serde_json::to_value(whole).unwrap(),
        "",
        context,
    );
}

fn same(left: &Value, right: &Value, path: &str, context: &str) {
    match (left, right) {
        (Value::Object(left), Value::Object(right)) => {
            assert_eq!(
                left.keys().collect::<Vec<_>>(),
                right.keys().collect::<Vec<_>>(),
                "{context}: keys at {path}"
            );
            for (key, value) in left {
                same(value, &right[key], &format!("{path}/{key}"), context);
            }
        }
        (Value::Array(left), Value::Array(right)) => {
            assert_eq!(left.len(), right.len(), "{context}: length at {path}");
            for (index, (left, right)) in left.iter().zip(right).enumerate() {
                same(left, right, &format!("{path}/{index}"), context);
            }
        }
        (Value::Number(left), Value::Number(right)) if path.ends_with("usd") => {
            let left = left.as_f64().unwrap();
            let right = right.as_f64().unwrap();
            assert!(
                (left - right).abs() <= 1e-9 * right.abs().max(1.0),
                "{context}: usd at {path}: {left} vs {right}"
            );
        }
        _ => assert_eq!(left, right, "{context}: value at {path}"),
    }
}

#[test]
fn every_figure_is_its_whole_read_on_the_fixture_journals() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("fixtures.db");
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    import_fixtures(&mut store);
    drop(store);
    let hearths = [hearth(Vec::new(), &journal)];
    for range in [Range::Day, Range::Week, Range::Month] {
        let rows = derive(&hearths, range, None, FIXTURE_NOW).unwrap();
        let whole = view(&hearths, range, None, FIXTURE_NOW);
        assert_same(&rows, &whole, &format!("fixtures {range:?}"));
    }
}

/// The clock `minutes` from `base`, as the stamp a fixture writes.
fn stamp(base: i64, minutes: i64) -> String {
    let at = time::OffsetDateTime::from_unix_timestamp(base / 1000).unwrap()
        + time::Duration::minutes(minutes);
    at.format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

type Steps = Vec<(EventType, Value, String)>;

/// A run's start, pinned to `realm` and named `recipe` when each is given.
fn started(realm: Option<&str>, recipe: &str, at: String) -> Steps {
    let manifest = match realm {
        Some(realm) => json!({
            "realms": {"source": "/w/realms.json", "realm": realm,
                       "map": {"realms": [{"name": realm, "path": realm}]}},
            "bundle_name": recipe,
        }),
        None => json!({"bundle_name": recipe}),
    };
    vec![(
        EventType::RunStarted,
        json!({"feature": "generated", "manifest": manifest}),
        at,
    )]
}

/// One effect's request, start, checkpoint and success at `at`, reporting
/// `usd` for `model` on `provider`.
fn seated(effect: &str, provider: &str, model: &str, usd: f64, at: &str) -> Steps {
    vec![
        (
            EventType::EffectRequested,
            json!({"effect_id": effect, "seat": "builder"}),
            at.to_string(),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": effect, "attempt_id": "a",
                   "provenance": [{"agent": "a", "provider": provider, "model": model}]}),
            at.to_string(),
        ),
        (
            EventType::EffectCheckpointed,
            json!({"effect_id": effect, "attempt_id": "a",
                   "checkpoint": {"step": "claude-session-finished", "model": model,
                                  "total_cost_usd": usd}}),
            at.to_string(),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": effect, "result": {"result": "complete"}}),
            at.to_string(),
        ),
    ]
}

/// The rule a hard stop's last ruling names: it may omit `rule_id`, state
/// it null, or name a rule.
enum LastRuling<'a> {
    Absent,
    Null,
    Named(&'a str),
}

/// One stopped run's realm, recipe, rule, seat and finish.
struct RunCase<'a> {
    realm: Option<&'a str>,
    recipe: &'a str,
    rule: LastRuling<'a>,
    provider: &'a str,
    model: &'a str,
    usd: f64,
    /// Its finish, in minutes from the journal's clock.
    finish: i64,
}

/// A run stopped by its hard stop whose one seat reported its cost: its
/// last ruling names `rule`, or names none.
fn stopped_run(base: i64, case: RunCase<'_>) -> Steps {
    let RunCase {
        realm,
        recipe,
        rule,
        provider,
        model,
        usd,
        finish,
    } = case;
    let mut steps = started(realm, recipe, stamp(base, -30 * 60));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "review"}),
        stamp(base, -30 * 60),
    ));
    steps.extend(seated("fx", provider, model, usd, &stamp(base, -20 * 60)));
    let mut decided = json!({"from": "review", "result": "ruled", "next": "stop"});
    match rule {
        LastRuling::Absent => {}
        LastRuling::Null => decided["rule_id"] = Value::Null,
        LastRuling::Named(rule) => decided["rule_id"] = json!(rule),
    }
    steps.push((EventType::TransitionDecided, decided, stamp(base, finish)));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "stop"}),
        stamp(base, finish),
    ));
    steps.push((EventType::RunStopped, json!({}), stamp(base, finish)));
    steps
}

/// A run stopped before it entered a phase: its row's status is `stopped`
/// and its phase NULL, which is no hard stop (L1).
fn stopped_before_any_phase(base: i64) -> Steps {
    let mut steps = started(Some("world"), "label-recipe", stamp(base, -30 * 60));
    steps.push((EventType::RunStopped, json!({}), stamp(base, -19 * 60)));
    steps
}

/// A run parked in `phase` from `at` to `ended`, its first operator command
/// at `answered`, all in minutes from the journal's clock.
fn parked_in_phase(base: i64, phase: &str, at: i64, answered: i64, ended: i64) -> Steps {
    let mut steps = started(Some("parked-realm"), "parked-recipe", stamp(base, at - 60));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": phase}),
        stamp(base, at - 60),
    ));
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "parked", "seat": "builder"}),
        stamp(base, at - 60),
    ));
    steps.push((
        EventType::RunParked,
        json!({"reason": "waiting on the operator"}),
        stamp(base, at),
    ));
    steps.push((
        EventType::OperatorCommanded,
        json!({"command_id": "answer", "command": "retry"}),
        stamp(base, answered),
    ));
    steps.push((
        EventType::OperatorAccepted,
        json!({"command_id": "answer"}),
        stamp(base, answered),
    ));
    steps.push((EventType::RunCompleted, json!({}), stamp(base, ended)));
    steps
}

/// A run left open at an effect, so its row has no finish.
fn open_run(base: i64) -> Steps {
    let mut steps = started(Some("parked-realm"), "parked-recipe", stamp(base, -23 * 60));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "open"}),
        stamp(base, -22 * 60),
    ));
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "open", "seat": "builder"}),
        stamp(base, -22 * 60),
    ));
    steps.push((
        EventType::EffectStarted,
        json!({"effect_id": "open", "attempt_id": "a"}),
        stamp(base, -22 * 60),
    ));
    steps
}

/// A run a writer from before the summary left behind, with a park the
/// fallback reads whole.
fn behind_parked_run(base: i64) -> Steps {
    let mut steps = started(
        Some("behind-parked-realm"),
        "behind-parked-recipe",
        stamp(base, -30 * 60),
    );
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "phase-000"}),
        stamp(base, -30 * 60),
    ));
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "parked", "seat": "builder"}),
        stamp(base, -20 * 60),
    ));
    steps.push((
        EventType::RunParked,
        json!({"reason": "behind"}),
        stamp(base, -20 * 60),
    ));
    steps.push((
        EventType::OperatorCommanded,
        json!({"command_id": "answer", "command": "retry"}),
        stamp(base, -19 * 60),
    ));
    steps.push((
        EventType::OperatorAccepted,
        json!({"command_id": "answer"}),
        stamp(base, -19 * 60),
    ));
    steps.push((EventType::RunCompleted, json!({}), stamp(base, -18 * 60)));
    steps
}

/// A run that completed, its finish `finish_minutes` from `base`.
fn completed_run(base: i64, finish_minutes: i64) -> Steps {
    let mut steps = started(
        Some("previous-realm"),
        "previous-recipe",
        stamp(base, finish_minutes - 60),
    );
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "review"}),
        stamp(base, finish_minutes - 60),
    ));
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "builder"}),
        stamp(base, finish_minutes - 30),
    ));
    steps.push((
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a"}),
        stamp(base, finish_minutes - 30),
    ));
    steps.push((
        EventType::EffectSucceeded,
        json!({"effect_id": "fx", "result": {"result": "complete"}}),
        stamp(base, finish_minutes - 30),
    ));
    steps.push((
        EventType::TransitionDecided,
        json!({"rule_id": "PREV", "from": "review", "result": "complete", "next": "done"}),
        stamp(base, finish_minutes - 1),
    ));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "done"}),
        stamp(base, finish_minutes - 1),
    ));
    steps.push((
        EventType::RunCompleted,
        json!({}),
        stamp(base, finish_minutes),
    ));
    steps
}

/// One run with a park in a phase of its own.
fn parked_run(index: u32, base: i64) -> Steps {
    let phase = match index {
        6 => format!("phase-{}", "p".repeat(100)),
        _ => format!("phase-{index:03}"),
    };
    let mut steps = started(Some("parked-realm"), "parked-recipe", stamp(base, -23 * 60));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": phase}),
        stamp(base, -22 * 60),
    ));
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "parked", "seat": "builder"}),
        stamp(base, -22 * 60),
    ));
    steps.push((
        EventType::RunParked,
        json!({"reason": "waiting on the operator"}),
        stamp(base, -22 * 60),
    ));
    steps.push((
        EventType::OperatorCommanded,
        json!({"command_id": "answer", "command": "retry"}),
        stamp(base, -22 * 60 + 1),
    ));
    steps.push((
        EventType::OperatorAccepted,
        json!({"command_id": "answer"}),
        stamp(base, -22 * 60 + 1),
    ));
    steps.push((EventType::RunCompleted, json!({}), stamp(base, -21 * 60)));
    steps
}

/// A park that starts before the window and ends before it, answered
/// inside it: the command's stamp is later than the run's next event's,
/// across a clock stepped back (decision 0089 ruling 9).
fn clock_park_run(base: i64) -> Steps {
    let mut steps = started(Some("clock-realm"), "clock-recipe", stamp(base, -30 * 60));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "clock"}),
        stamp(base, -30 * 60),
    ));
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "clock", "seat": "builder"}),
        stamp(base, -30 * 60),
    ));
    steps.push((
        EventType::RunParked,
        json!({"reason": "clock"}),
        stamp(base, -30 * 60),
    ));
    steps.push((
        EventType::OperatorCommanded,
        json!({"command_id": "answer", "command": "retry"}),
        stamp(base, -23 * 60),
    ));
    steps.push((
        EventType::OperatorAccepted,
        json!({"command_id": "answer"}),
        stamp(base, -23 * 60),
    ));
    // The next event ends the park, before the window starts though its
    // stamp is earlier than the command's.
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "after", "seat": "builder"}),
        stamp(base, -25 * 60),
    ));
    steps.push((EventType::RunCompleted, json!({}), stamp(base, -22 * 60)));
    steps
}

/// A run whose start pins no realm: the hearth's label is its realm.
fn unlabelled_run(base: i64) -> Steps {
    let mut steps = started(None, "label-recipe", stamp(base, -30 * 60));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "review"}),
        stamp(base, -30 * 60),
    ));
    steps.push((EventType::RunCompleted, json!({}), stamp(base, -18 * 60)));
    steps
}

/// Seal `steps` into `run_id`'s chain.
fn sealed(run_id: &str, steps: &Steps) -> Vec<brokkr_core::EventEnvelope> {
    let mut previous = ZERO_HASH.to_string();
    let mut events = Vec::new();
    for (index, (event_type, payload, at)) in steps.iter().enumerate() {
        let seq = index as u64 + 1;
        let event = EnvelopeBuilder::new(*event_type, payload.clone())
            .run(run_id)
            .seq(seq)
            .event_id(format!("{run_id}-{seq}"))
            .at(at.clone())
            .previous(previous.clone())
            .sealed();
        previous = event.event_hash.clone();
        events.push(event);
    }
    events
}

/// Seal `steps` into `run_id`'s chain and import it as a writer does.
fn plant(store: &mut brokkr_store::Store, run_id: &str, steps: &Steps) {
    let mut ndjson = String::new();
    for event in sealed(run_id, steps) {
        ndjson.push_str(&serde_json::to_string(&event).unwrap());
        ndjson.push('\n');
    }
    let origin = std::path::PathBuf::from(format!("{run_id}.ndjson"));
    store.import_run(&ndjson, &json!({}), &origin).unwrap();
}

/// Seventy stopped runs whose realm, recipe and rule each cross 64 groups
/// and, for one run apiece, the 64-byte cut; each also names a model and a
/// provider.
fn generated_stopped_runs(store: &mut brokkr_store::Store, base: i64) {
    for index in 0..70u32 {
        let run_id = format!("gen-{index:03}");
        let realm = if index == 3 {
            format!("realm-{}", "r".repeat(100))
        } else {
            format!("realm-{index:03}")
        };
        let recipe = if index == 4 {
            format!("recipe-{}", "c".repeat(100))
        } else {
            format!("recipe-{index:03}")
        };
        let rule = if index == 5 {
            format!("RULE-{}", "x".repeat(80))
        } else {
            format!("RULE-{index:03}")
        };
        let provider = if index % 2 == 0 { "claude" } else { "codex" };
        let steps = stopped_run(
            base,
            RunCase {
                realm: Some(&realm),
                recipe: &recipe,
                rule: LastRuling::Named(&rule),
                provider,
                model: &format!("model-{index:03}"),
                usd: 0.01 * f64::from(index + 1),
                finish: -18 * 60 + i64::from(index) * 10,
            },
        );
        plant(store, &run_id, &steps);
    }
}

/// A hard stop naming a rule and one naming none, and a run the store
/// would never import, so a model past 80 bytes and a provider past 64
/// reach the rows and the whole read alike.
fn generated_hard_stops(store: &mut brokkr_store::Store, base: i64) {
    plant(
        store,
        "gen-stop-norule",
        &stopped_run(
            base,
            RunCase {
                realm: Some("stop-realm"),
                recipe: "stop-recipe",
                rule: LastRuling::Absent,
                provider: "claude",
                model: "stop-model",
                usd: 0.5,
                finish: -19 * 60 + 5,
            },
        ),
    );
    plant(
        store,
        "gen-stop-rule",
        &stopped_run(
            base,
            RunCase {
                realm: Some("stop-realm"),
                recipe: "stop-recipe",
                rule: LastRuling::Named("STOP-RULE"),
                provider: "claude",
                model: "stop-model",
                usd: 0.5,
                finish: -19 * 60 + 15,
            },
        ),
    );
    let long_provider = format!("provider-{}", "p".repeat(80));
    let long_model = format!("model-{}", "m".repeat(90));
    plant_older_writer(
        store,
        "gen-bound",
        &sealed(
            "gen-bound",
            &stopped_run(
                base,
                RunCase {
                    realm: Some("stop-realm"),
                    recipe: "stop-recipe",
                    rule: LastRuling::Named("STOP-RULE"),
                    provider: &long_provider,
                    model: &long_model,
                    usd: 0.5,
                    finish: -19 * 60 + 30,
                },
            ),
        ),
    );
}

/// A journal with every shape the row read must fold as the whole read
/// does: seventy stopped runs with a realm, recipe, rule and model each
/// (past 64 groups of four cuts, a realm, recipe, rule, model and provider
/// each past its bound), a hard stop naming a rule and one naming none, a
/// run stopped before it entered any phase, seventy phases of one run's
/// parks, a park answered inside the window and ended before it, and a run
/// whose stored realm is NULL.
fn generated_journal(dir: &std::path::Path) -> std::path::PathBuf {
    let journal = dir.join("generated.db");
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    let base = parse_millis(GENERATED_NOW).unwrap();
    generated_stopped_runs(&mut store, base);
    generated_hard_stops(&mut store, base);
    plant(&mut store, "gen-no-phase", &stopped_before_any_phase(base));
    for index in 0..70u32 {
        plant(
            &mut store,
            &format!("gen-parked-{index:03}"),
            &parked_run(index, base),
        );
    }
    plant(&mut store, "gen-clock", &clock_park_run(base));
    plant(&mut store, "gen-unlabelled", &unlabelled_run(base));
    // Shipped runs in each range's preceding window: 24h's is a day back,
    // 7d's a week back, and 30d's a month back.
    for (name, finish) in [
        ("prev-day", -30 * 60),
        ("prev-week", -8 * 24 * 60),
        ("prev-month", -40 * 24 * 60),
    ] {
        for index in 0..3 {
            plant(
                &mut store,
                &format!("{name}-{index}"),
                &completed_run(base, finish),
            );
        }
    }
    let store = brokkr_store::test_support::refolded(store);
    drop(store);
    journal
}

#[test]
fn every_figure_is_its_whole_read_on_the_generated_journal() {
    let dir = tempfile::tempdir().unwrap();
    let journal = generated_journal(dir.path());
    let hearths = [hearth(Vec::new(), &journal)];
    for range in [Range::Day, Range::Week, Range::Month] {
        let rows = derive(&hearths, range, None, GENERATED_NOW).unwrap();
        let whole = view(&hearths, range, None, GENERATED_NOW);
        assert_same(&rows, &whole, &format!("generated {range:?}"));
    }
    let day = derive(&hearths, Range::Day, None, GENERATED_NOW).unwrap();
    assert_eq!(day.models.len(), 64);
    assert_eq!(day.models_more, 72 - 64);
    assert_eq!(day.realms.len(), 64);
    // The hearth's label, seventy pinned realms, a parked realm, a clock
    // realm, a stop realm: seventy-five names, the first sixty-four kept.
    assert_eq!(day.realms_more, 75 - 64);
    assert_eq!(day.recipes.len(), 64);
    assert_eq!(day.recipes_more, 74 - 64);
    assert_eq!(day.waiting_by_step.len(), 64);
    assert_eq!(day.waiting_by_step_more, 71 - 64);
    // Seventy named rules, STOP-RULE and the stop that names none.
    assert_eq!(day.headline.stopped_by_rule.rules_more, 72 - 64);
    assert!(
        day.headline.waiting.waiting_millis >= 60 * 60 * 1000,
        "the clock-stepped-back park's waiting time is kept: {:?}",
        day.headline.waiting
    );
    // C2: the long model and provider a past writer planted reach the row
    // read, and no run in this journal is left whole, so the cuts above
    // were read from rows rather than the whole fallback.
    let long_provider = format!("provider-{}", "p".repeat(80));
    let long_model = format!("model-{}", "m".repeat(90));
    read_hearths(&hearths, |grouped| {
        let query = Query {
            hearths: grouped,
            range: Range::Day,
            realm: None,
            now: GENERATED_NOW,
        };
        let (window, _) = brokkr_view::windows(&query).unwrap();
        let store = brokkr_store::Store::open_read_only(&journal).unwrap();
        let mut whole = Vec::new();
        store
            .behind_runs(|run_id| whole.push(run_id.to_string()))
            .unwrap();
        store
            .window_fallbacks(0, 0, None, |run| whole.push(run.run_id.to_string()))
            .unwrap();
        assert!(whole.is_empty(), "no run is read whole: {whole:?}");
        let mut bound = None;
        store
            .window_sites(
                window.from,
                window.to,
                None,
                brokkr_store::metered::SiteOrder::Model,
                |site| {
                    if site.run_id == "gen-bound" {
                        bound = Some((
                            site.provider.map(str::to_string),
                            site.model.map(str::to_string),
                        ));
                    }
                },
            )
            .unwrap();
        assert_eq!(
            bound,
            Some((
                Some(Bounded::within(&long_provider, NAME_BYTES).cut),
                Some(Bounded::within(&long_model, MODEL_BYTES).cut),
            )),
            "the long provider and model reach the row read at their bounds"
        );
    });
}

/// A journal of `count` stopped runs, each a realm and recipe of its own.
fn small_journal(dir: &std::path::Path, name: &str, count: u32) -> std::path::PathBuf {
    let journal = dir.join(format!("{name}.db"));
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    let base = parse_millis(GENERATED_NOW).unwrap();
    for index in 0..count {
        let steps = stopped_run(
            base,
            RunCase {
                realm: Some(&format!("{name}-realm-{index}")),
                recipe: &format!("{name}-recipe-{index}"),
                rule: LastRuling::Named(&format!("{name}-rule-{index}")),
                provider: "claude",
                model: &format!("{name}-model-{index}"),
                usd: 0.25,
                finish: -19 * 60,
            },
        );
        plant(&mut store, &format!("{name}-{index}"), &steps);
    }
    drop(store);
    journal
}

#[test]
fn a_realm_filter_is_its_whole_read() {
    let dir = tempfile::tempdir().unwrap();
    let journal = generated_journal(dir.path());
    let hearths = [hearth(Vec::new(), &journal)];
    let long = format!("realm-{}", "r".repeat(100));
    for realm in [
        None,
        Some("world"),
        Some("clock-realm"),
        Some(long.as_str()),
        Some("no-such-realm"),
    ] {
        for range in [Range::Day, Range::Week, Range::Month] {
            let rows = derive(&hearths, range, realm, GENERATED_NOW).unwrap();
            let whole = view(&hearths, range, realm, GENERATED_NOW);
            assert_same(&rows, &whole, &format!("generated {range:?} {realm:?}"));
        }
    }
}

#[test]
fn an_unmigrated_hearth_and_an_unfinished_backfill_are_read_whole() {
    let dir = tempfile::tempdir().unwrap();
    let plain = small_journal(dir.path(), "plain", 3);

    let unmigrated = dir.path().join("unmigrated.db");
    std::fs::copy(&plain, &unmigrated).unwrap();
    let conn = rusqlite::Connection::open(&unmigrated).unwrap();
    unmigrate(&conn).unwrap();
    drop(conn);

    let unfinished = dir.path().join("unfinished.db");
    std::fs::copy(&plain, &unfinished).unwrap();
    let conn = rusqlite::Connection::open(&unfinished).unwrap();
    conn.execute("DELETE FROM meta WHERE key = 'run_summary_backfill'", [])
        .unwrap();
    drop(conn);

    assert_journals_are_their_whole_read(&[&unmigrated, &unfinished]);
}

#[test]
fn a_behind_run_and_a_stale_seat_run_are_each_read_whole_alone() {
    let dir = tempfile::tempdir().unwrap();
    let base = parse_millis(GENERATED_NOW).unwrap();
    let journal = small_journal(dir.path(), "migrated", 2);

    // The run a writer from before the summary left behind its head.
    let behind = stopped_run(
        base,
        RunCase {
            realm: Some("behind-realm"),
            recipe: "behind-recipe",
            rule: LastRuling::Named("behind-rule"),
            provider: "claude",
            model: "behind-model",
            usd: 0.5,
            finish: -19 * 60,
        },
    );
    let behind = sealed("behind-run", &behind);
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    plant(&mut store, "open-run", &open_run(base));
    drop(store);
    let store = brokkr_store::Store::open(&journal).unwrap();
    plant_older_writer(&store, "behind-run", &behind);
    let parked = sealed("behind-parked", &behind_parked_run(base));
    plant_older_writer(&store, "behind-parked", &parked);
    // A run from before the summary whose chain does not verify: the
    // fallback's one-run opening refuses it, as today's whole read
    // quarantines it and no figure reads it.
    let broken = sealed("broken-run", &completed_run(base, -19 * 60));
    plant_older_writer(&store, "broken-run", &broken);
    drop(store);
    let conn = rusqlite::Connection::open(&journal).unwrap();
    plant_broken_link(&conn, &broken[0], 99).unwrap();
    drop(conn);

    // A run whose seat rows stand at the head before a later checkpoint,
    // which a writer from between the projections leaves stale.
    let stale = dir.path().join("stale.db");
    let mut store = brokkr_store::Store::open(&stale).unwrap();
    let mut steps = started(Some("stale-realm"), "stale-recipe", stamp(base, -30 * 60));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "work"}),
        stamp(base, -30 * 60),
    ));
    steps.push((
        EventType::EffectRequested,
        json!({"effect_id": "fx", "seat": "builder"}),
        stamp(base, -20 * 60),
    ));
    steps.push((
        EventType::EffectStarted,
        json!({"effect_id": "fx", "attempt_id": "a",
               "provenance": [{"agent": "a", "provider": "claude", "model": "stale-model"}]}),
        stamp(base, -20 * 60),
    ));
    steps.push((
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "claude-session-finished", "model": "stale-model",
                              "total_cost_usd": 0.25}}),
        stamp(base, -20 * 60),
    ));
    plant(&mut store, "stale-run", &steps);
    let events = store.load("stale-run").unwrap();
    let last = events.last().unwrap();
    let extra = EnvelopeBuilder::new(
        EventType::EffectCheckpointed,
        json!({"effect_id": "fx", "attempt_id": "a",
               "checkpoint": {"step": "claude-session-finished", "model": "stale-model",
                              "total_cost_usd": 0.5}}),
    )
    .run("stale-run")
    .seq(last.seq + 1)
    .event_id("stale-run-extra")
    .at(stamp(base, -19 * 60))
    .previous(last.event_hash.clone())
    .sealed();
    drop(store);
    // A fresh open carries no writer's fold, so its drop check does not
    // judge the seat rows this writer leaves stale.
    let store = brokkr_store::Store::open(&stale).unwrap();
    plant_summary_writer(&store, "stale-run", &[extra]);
    drop(store);

    assert_journals_are_their_whole_read(&[&journal, &stale]);
}

#[test]
fn a_world_of_two_hearths_one_migrated_and_one_not_is_its_whole_read() {
    let dir = tempfile::tempdir().unwrap();
    let migrated = small_journal(dir.path(), "migrated", 3);
    let unmigrated = dir.path().join("unmigrated.db");
    std::fs::copy(small_journal(dir.path(), "copied", 2), &unmigrated).unwrap();
    let conn = rusqlite::Connection::open(&unmigrated).unwrap();
    unmigrate(&conn).unwrap();
    drop(conn);

    let hearths = [
        hearth(vec!["alpha".to_string()], &migrated),
        hearth(vec!["beta".to_string()], &unmigrated),
    ];
    for range in [Range::Day, Range::Week, Range::Month] {
        let rows = derive(&hearths, range, None, GENERATED_NOW).unwrap();
        let whole = view(&hearths, range, None, GENERATED_NOW);
        assert_same(&rows, &whole, &format!("two hearths {range:?}"));
    }
}

#[test]
fn a_hearth_the_fleet_refuses_contributes_no_rows() {
    let dir = tempfile::tempdir().unwrap();
    let journal = small_journal(dir.path(), "refused", 2);
    let conn = rusqlite::Connection::open(&journal).unwrap();
    conn.execute("UPDATE runs SET created_at = x'00'", [])
        .unwrap();
    drop(conn);
    let rows = assert_rows_are_whole(&journal, "a refused hearth");
    assert_eq!(rows.quarantined.len(), 1, "the refusal is reported");
    assert_eq!(rows.quarantined[0].realm, "world");
    assert!(
        rows.quarantined[0].refusal.contains("created_at"),
        "the refusal names the column it could not read: {}",
        rows.quarantined[0].refusal
    );
    assert!(rows.realms.is_empty(), "no realm is listed");
    assert!(
        rows.headline.median.millis.is_none(),
        "no run's duration is counted"
    );
    assert_eq!(rows.headline.stopped_by_rule.count, 0);
    assert!(
        rows.per_day.iter().all(|day| day.stopped == 0),
        "no day counts a stop"
    );
    assert!(rows.models.is_empty(), "no model is counted");
    assert!(rows.recipes.is_empty(), "no recipe is counted");
}

#[test]
fn a_hard_stop_that_names_no_rule_keeps_its_stop() {
    let dir = tempfile::tempdir().unwrap();
    let journal = dir.path().join("stops.db");
    let base = parse_millis(GENERATED_NOW).unwrap();
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    plant(
        &mut store,
        "stop-norule",
        &stopped_run(
            base,
            RunCase {
                realm: Some("stop-realm"),
                recipe: "stop-recipe",
                rule: LastRuling::Absent,
                provider: "claude",
                model: "stop-model",
                usd: 0.5,
                finish: -19 * 60,
            },
        ),
    );
    plant(
        &mut store,
        "stop-rule",
        &stopped_run(
            base,
            RunCase {
                realm: Some("stop-realm"),
                recipe: "stop-recipe",
                rule: LastRuling::Named("STOP-RULE"),
                provider: "claude",
                model: "stop-model",
                usd: 0.5,
                finish: -18 * 60,
            },
        ),
    );
    drop(store);
    let rows = assert_rows_are_whole(&journal, "a hard stop with no rule");
    assert_eq!(rows.headline.stopped_by_rule.count, 2);
    assert_eq!(
        serde_json::to_value(&rows.headline.stopped_by_rule).unwrap(),
        json!({
            "count": 2,
            "top": {"rule_id": null, "count": 1},
            "text": "1 by —",
        }),
        "the stop that names no rule is the lesser id on the tie"
    );
}

#[test]
fn a_single_hard_stop_that_names_no_rule_is_counted() {
    for (index, rule) in [LastRuling::Absent, LastRuling::Null]
        .into_iter()
        .enumerate()
    {
        let dir = tempfile::tempdir().unwrap();
        let journal = dir.path().join(format!("stop-{index}.db"));
        let base = parse_millis(GENERATED_NOW).unwrap();
        let mut store = brokkr_store::Store::open(&journal).unwrap();
        plant(
            &mut store,
            "only",
            &stopped_run(
                base,
                RunCase {
                    realm: Some("stop-realm"),
                    recipe: "stop-recipe",
                    rule,
                    provider: "claude",
                    model: "stop-model",
                    usd: 0.5,
                    finish: -19 * 60,
                },
            ),
        );
        drop(store);
        let rows = assert_rows_are_whole(&journal, &format!("a hard stop with no rule {index}"));
        assert_eq!(rows.headline.stopped_by_rule.count, 1);
        assert_eq!(
            serde_json::to_value(&rows.headline.stopped_by_rule).unwrap(),
            json!({
                "count": 1,
                "top": {"rule_id": null, "count": 1},
                "text": "1 by —",
            }),
            "a hard stop whose ruling omits or nulls its rule is still a stop"
        );
    }
}

/// A journal holding one run parked in phase `same` for two hours.
fn same_phase_park(
    dir: &std::path::Path,
    name: &str,
    run_id: &str,
    base: i64,
) -> std::path::PathBuf {
    let journal = dir.join(format!("{name}.db"));
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    plant(
        &mut store,
        run_id,
        &parked_in_phase(base, "same", -220, -160, -100),
    );
    drop(store);
    journal
}

#[test]
fn a_run_id_in_two_hearths_is_two_runs_in_waiting_by_step() {
    let dir = tempfile::tempdir().unwrap();
    let base = parse_millis(GENERATED_NOW).unwrap();
    let same = [
        hearth(
            vec!["left".to_string()],
            &same_phase_park(dir.path(), "same-left", "r1", base),
        ),
        hearth(
            vec!["right".to_string()],
            &same_phase_park(dir.path(), "same-right", "r1", base),
        ),
    ];
    let rows = derive(&same, Range::Day, None, GENERATED_NOW).unwrap();
    let whole = view(&same, Range::Day, None, GENERATED_NOW);
    assert_same(&rows, &whole, "the same run id in two hearths");
    // Main's figure, pinned as a literal: a park's run identity is its
    // place in the scoped run list, so two hearths count two runs.
    assert_eq!(
        serde_json::to_value(&rows.waiting_by_step).unwrap(),
        json!([{
            "phase": "same",
            "waiting_millis": 7_200_000,
            "runs": 2,
            "text": "2.0h · 2",
        }])
    );

    let distinct = [
        hearth(
            vec!["left".to_string()],
            &same_phase_park(dir.path(), "distinct-left", "r1", base),
        ),
        hearth(
            vec!["right".to_string()],
            &same_phase_park(dir.path(), "distinct-right", "r2", base),
        ),
    ];
    let rows = derive(&distinct, Range::Day, None, GENERATED_NOW).unwrap();
    let whole = view(&distinct, Range::Day, None, GENERATED_NOW);
    assert_same(&rows, &whole, "two run ids in two hearths");
    assert_eq!(
        serde_json::to_value(&rows.waiting_by_step).unwrap(),
        json!([{
            "phase": "same",
            "waiting_millis": 7_200_000,
            "runs": 2,
            "text": "2.0h · 2",
        }]),
        "the control: two run ids count two runs on both sides"
    );
}

#[test]
fn a_row_read_that_fails_reads_its_hearth_whole() {
    let dir = tempfile::tempdir().unwrap();
    let journal = small_journal(dir.path(), "row-error", 2);
    let conn = rusqlite::Connection::open(&journal).unwrap();
    conn.execute(
        "UPDATE run_summary SET ending = 'no-such-ending' WHERE run_id = 'row-error-0'",
        [],
    )
    .unwrap();
    drop(conn);
    let hearths = [hearth(Vec::new(), &journal)];
    let rows = derive(&hearths, Range::Day, None, GENERATED_NOW).unwrap();
    let whole = view(&hearths, Range::Day, None, GENERATED_NOW);
    assert_same(&rows, &whole, "a row read that fails");
    assert_eq!(
        rows.headline.shipped.count, whole.headline.shipped.count,
        "the whole read still paints every shipped run"
    );
    assert!(
        rows.quarantined.is_empty(),
        "a row that will not parse is no hearth refusal"
    );
}

/// A run the whole read cannot load is counted from no row (S2, security).
/// Its summary row stands at its head with current projections, so neither
/// `behind_runs` nor `window_fallbacks` names it; the rows must leave it
/// out exactly as the whole read does, while still counting it unreadable.
#[test]
fn a_run_the_whole_read_cannot_load_is_counted_from_no_row() {
    let dir = tempfile::tempdir().unwrap();
    let journal = small_journal(dir.path(), "corrupt", 2);
    // Corrupt corrupt-0's first envelope in place, leaving its `event_hash`,
    // the head stamps and the seat heads untouched: the row still stands at
    // its head, while the fleet read can no longer load the run.
    let conn = rusqlite::Connection::open(&journal).unwrap();
    conn.execute("DROP TRIGGER events_append_only_update", [])
        .unwrap();
    let rewritten = conn
        .execute(
            "UPDATE events SET envelope = replace(envelope, 'generated', 'generatedX')
              WHERE run_id = 'corrupt-0' AND seq = 1",
            [],
        )
        .unwrap();
    assert_eq!(rewritten, 1, "the first envelope is rewritten");
    drop(conn);

    let store = brokkr_store::Store::open_read_only(&journal).unwrap();
    assert!(store.aggregates_ready().unwrap(), "the rows are ready");
    assert_eq!(lane(&store, "corrupt-0").as_deref(), Some("finished"));
    let mut behind = Vec::new();
    store
        .behind_runs(|run_id| behind.push(run_id.to_string()))
        .unwrap();
    assert!(
        behind.is_empty(),
        "the row is not behind its head: {behind:?}"
    );
    let mut fallback = Vec::new();
    store
        .window_fallbacks(0, 0, None, |run| fallback.push(run.run_id.to_string()))
        .unwrap();
    assert!(
        fallback.is_empty(),
        "the projections are current, so no fallback names it: {fallback:?}"
    );
    drop(store);

    let rows = assert_rows_are_whole(&journal, "a run the whole read cannot load");
    assert_eq!(
        serde_json::to_value(&rows.headline.stopped_by_rule).unwrap(),
        json!({
            "count": 1,
            "top": {"rule_id": "corrupt-rule-1", "count": 1},
            "text": "1 by corrupt-rule-1",
        }),
        "the unreadable run's stop is counted from no row"
    );
    assert_eq!(rows.headline.cost.text, "$0.25");
    assert_eq!(rows.models.len(), 1, "one model, from the readable run");
    let world = rows
        .realms
        .iter()
        .find(|realm| realm.realm == "world")
        .expect("the unreadable run is named by the hearth's label");
    assert_eq!(world.unreadable_runs, 1);
}

/// A run stopped before it entered any phase is a valid row: status
/// `stopped` and a NULL phase (L1). The flag must stay false, and the
/// hearth must be read from its rows rather than disabled by a row error.
#[test]
fn a_stopped_run_with_no_phase_is_read_from_its_row() {
    let dir = tempfile::tempdir().unwrap();
    let journal = small_journal(dir.path(), "no-phase", 0);
    let base = parse_millis(GENERATED_NOW).unwrap();
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    plant(&mut store, "no-phase-run", &stopped_before_any_phase(base));
    drop(store);

    let rows = assert_rows_are_whole(&journal, "a stopped run with no phase");
    assert_eq!(
        rows.headline.stopped_by_rule.count, 0,
        "a stop that entered no phase is no hard stop"
    );

    let hearths = [hearth(Vec::new(), &journal)];
    with_rows(
        &hearths,
        Range::Day,
        None,
        GENERATED_NOW,
        || {},
        |_, source| {
            assert!(
                source.reads_rows(),
                "the no-phase stopped row loads, so its hearth reads from rows"
            );
        },
    );
}

#[test]
fn a_fallback_run_that_will_not_open_reads_its_hearth_whole() {
    let dir = tempfile::tempdir().unwrap();
    let journal = small_journal(dir.path(), "fallback", 2);
    let base = parse_millis(GENERATED_NOW).unwrap();
    let events = sealed(
        "behind-run",
        &stopped_run(
            base,
            RunCase {
                realm: Some("behind-realm"),
                recipe: "behind-recipe",
                rule: LastRuling::Named("behind-rule"),
                provider: "claude",
                model: "behind-model",
                usd: 0.5,
                finish: -19 * 60,
            },
        ),
    );
    let store = brokkr_store::Store::open(&journal).unwrap();
    plant_older_writer(&store, "behind-run", &events);
    drop(store);

    let hearths = [hearth(Vec::new(), &journal)];
    with_rows(
        &hearths,
        Range::Day,
        None,
        GENERATED_NOW,
        || {
            // The fleet read loaded the run; break its chain behind its
            // back, so only the fallback's one-run opening fails.
            let conn = rusqlite::Connection::open(&journal).unwrap();
            plant_broken_link(&conn, &events[0], 99).unwrap();
            drop(conn);
        },
        |query, source| {
            let source = brokkr_view::FactUnion::new(vec![Box::new(source)]);
            let rows = brokkr_view::analytics_with(&source, query).unwrap();
            let whole = brokkr_view::analytics(query).unwrap();
            assert_same(&rows, &whole, "a fallback run that will not open");
            assert_eq!(
                rows.headline.shipped.count, whole.headline.shipped.count,
                "the run the fleet read still paints"
            );
        },
    );
}

/// A day in milliseconds: the span a Day readout reads its parks over.
const DAY_MILLIS: i64 = 24 * 60 * 60 * 1000;

/// `journal` opened read-only with its rows ready, so a test that plants
/// one broken shape is not passed by a hearth read whole for another.
fn ready(journal: &std::path::Path) -> brokkr_store::Store {
    let store = brokkr_store::Store::open_read_only(journal).unwrap();
    assert!(store.aggregates_ready().unwrap(), "the rows are ready");
    store
}

/// `journal`'s Day readout through the row path, which reads its rows when
/// `reads_rows` says so and the whole read otherwise, asserted equal to
/// the whole read and handed back.
fn day_through_rows(journal: &std::path::Path, reads_rows: bool, context: &str) -> Analytics {
    let hearths = [hearth(Vec::new(), journal)];
    with_rows(
        &hearths,
        Range::Day,
        None,
        GENERATED_NOW,
        || {},
        |query, source| {
            assert_eq!(source.reads_rows(), reads_rows, "{context}: the rows read");
            let source = brokkr_view::FactUnion::new(vec![Box::new(source)]);
            let rows = brokkr_view::analytics_with(&source, query).unwrap();
            assert_same(&rows, &brokkr_view::analytics(query).unwrap(), context);
            rows
        },
    )
}

/// A run the store leaves whole keeps its park out of the rows. Here its
/// row stands behind its head while its park's dated facts still stand,
/// so the store's park read still yields the park; the whole fallback
/// counts it, and the rows must not count it again.
#[test]
fn a_park_of_a_run_read_whole_is_counted_once() {
    let dir = tempfile::tempdir().unwrap();
    let base = parse_millis(GENERATED_NOW).unwrap();
    let journal = same_phase_park(dir.path(), "whole-park", "whole-park-run", base);
    let conn = rusqlite::Connection::open(&journal).unwrap();
    let behind = conn
        .execute(
            "UPDATE run_summary SET folded_seq = folded_seq - 1 WHERE run_id = 'whole-park-run'",
            [],
        )
        .unwrap();
    assert_eq!(behind, 1, "the row is set behind its head");
    drop(conn);

    let store = ready(&journal);
    assert_eq!(lane(&store, "whole-park-run").as_deref(), Some("behind"));
    let mut parks = Vec::new();
    store
        .window_parks(base - DAY_MILLIS, base, base, None, |park| {
            parks.push(park.run_id.to_string());
        })
        .unwrap();
    assert_eq!(parks, ["whole-park-run"], "the park's row still stands");
    drop(store);

    let rows = day_through_rows(&journal, true, "a park of a run read whole");
    assert_eq!(
        serde_json::to_value(&rows.waiting_by_step).unwrap(),
        json!([{
            "phase": "same",
            "waiting_millis": 3_600_000,
            "runs": 1,
            "text": "1.0h · 1",
        }]),
        "the park is counted once"
    );
}

/// A park row that will not read fails the row read at its parks, after
/// its spends and runs have read: the hearth is read whole.
#[test]
fn a_park_row_that_will_not_read_reads_its_hearth_whole() {
    let dir = tempfile::tempdir().unwrap();
    let base = parse_millis(GENERATED_NOW).unwrap();
    let journal = same_phase_park(dir.path(), "park-error", "park-error-run", base);
    let conn = rusqlite::Connection::open(&journal).unwrap();
    let broken = conn
        .execute(
            "UPDATE event_index SET phase = X'00'
              WHERE run_id = 'park-error-run' AND type = ?1",
            [EventType::RunParked.as_str()],
        )
        .unwrap();
    assert_eq!(broken, 1, "the park's phase is no longer text");
    drop(conn);

    let refused = ready(&journal).window_parks(base - DAY_MILLIS, base, base, None, |_| {});
    assert!(refused.is_err(), "the park row will not read");
    day_through_rows(&journal, false, "a park row that will not read");
}
