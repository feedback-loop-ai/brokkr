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
use brokkr_core::EventType;
use brokkr_store::test_support::{
    import_fixtures, plant_broken_link, plant_older_writer, plant_summary_writer, unmigrate,
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

/// One stopped run's realm, recipe, rule, seat and finish.
struct RunCase<'a> {
    realm: Option<&'a str>,
    recipe: &'a str,
    rule: &'a str,
    provider: &'a str,
    model: &'a str,
    usd: f64,
    /// Its finish, in minutes from the journal's clock.
    finish: i64,
}

/// A run stopped by its rule whose one seat reported its cost.
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
    steps.push((
        EventType::TransitionDecided,
        json!({"rule_id": rule, "from": "review", "result": "ruled", "next": "stop"}),
        stamp(base, finish),
    ));
    steps.push((
        EventType::PhaseEntered,
        json!({"phase": "stop"}),
        stamp(base, finish),
    ));
    steps.push((EventType::RunStopped, json!({}), stamp(base, finish)));
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

/// A journal with every shape the row read must fold as the whole read
/// does: seventy stopped runs with a realm, recipe, rule and model each
/// (past 64 groups of four cuts, one of each past its bound), seventy
/// phases of one run's parks, a park answered inside the window and ended
/// before it, and a run whose stored realm is NULL.
fn generated_journal(dir: &std::path::Path) -> std::path::PathBuf {
    let journal = dir.join("generated.db");
    let mut store = brokkr_store::Store::open(&journal).unwrap();
    let base = parse_millis(GENERATED_NOW).unwrap();
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
                rule: &rule,
                provider,
                model: &format!("model-{index:03}"),
                usd: 0.01 * f64::from(index + 1),
                finish: -18 * 60 + i64::from(index) * 10,
            },
        );
        plant(&mut store, &run_id, &steps);
    }
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
    assert_eq!(day.models_more, 70 - 64);
    assert_eq!(day.realms.len(), 64);
    // The hearth's label, seventy pinned realms, a parked realm, a clock
    // realm: seventy-three names, the first sixty-four kept.
    assert_eq!(day.realms_more, 74 - 64);
    assert_eq!(day.recipes.len(), 64);
    assert_eq!(day.recipes_more, 73 - 64);
    assert_eq!(day.waiting_by_step.len(), 64);
    assert_eq!(day.waiting_by_step_more, 71 - 64);
    assert_eq!(day.headline.stopped_by_rule.rules_more, 70 - 64);
    assert!(
        day.headline.waiting.waiting_millis >= 60 * 60 * 1000,
        "the clock-stepped-back park's waiting time is kept: {:?}",
        day.headline.waiting
    );
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
                rule: &format!("{name}-rule-{index}"),
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
            rule: "behind-rule",
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
