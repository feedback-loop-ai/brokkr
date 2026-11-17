//! The row mappings `brokkr analytics` folds: a run's row to its facts and a
//! site's row to its fact, including the rows a whole read opens no run or
//! site from.

use brokkr_core::ending::Ending;
use brokkr_core::seat::meter::Metering;
use brokkr_store::aggregate::WindowReviews;

use super::*;

fn row<'a>(run_id: &'a str) -> WindowRun<'a> {
    WindowRun {
        run_id,
        realm: Some("alpha"),
        recipe: Some("recipe"),
        ending_rule: Some("RULE"),
        hard_stop: true,
        started_ms: Some(1),
        finished_ms: Some(10),
        ending: Some(Ending::Shipped),
        implement_visits: 2,
        reviews: WindowReviews {
            first: true,
            second: false,
            back: true,
        },
        open: false,
    }
}

/// A run's row maps to its facts with no costs: the recipe order's stream
/// reads them by the run's key.
#[test]
fn a_run_row_maps_its_facts() {
    let facts = run_facts(&row("r"), "label", None).unwrap();
    assert_eq!(
        (
            facts.hearth.as_str(),
            facts.run.as_str(),
            facts.realm.as_str(),
            facts.recipe.as_deref(),
            facts.started,
            facts.open,
            facts.implement_visits,
            facts.reviews.clone(),
            facts.costs.clone(),
        ),
        (
            "label",
            "r",
            "alpha",
            Some("recipe"),
            Some(1),
            false,
            2,
            Reviews {
                first: true,
                second: false,
                back: true
            },
            vec![],
        )
    );
    assert_eq!(
        facts.finished,
        Some(Finished {
            at: 10,
            ending: Ending::Shipped,
            hard_stop: Some(HardStop {
                rule: Some("RULE".to_string())
            }),
        })
    );
}

#[test]
fn a_run_out_of_scope_is_no_row() {
    assert!(
        run_facts(&row("r"), "label", Some("beta")).is_none(),
        "another realm's run"
    );
}

fn park(run_id: &str) -> WindowPark<'_> {
    WindowPark {
        run_id,
        realm: Some("alpha"),
        phase: Some("verify"),
        at: 3,
        answered: None,
        ended: 9,
        open: false,
    }
}

#[test]
fn a_park_row_maps_its_fact_and_an_unanswered_park_waits_to_its_end() {
    assert_eq!(
        park_fact(&park("r"), "label", None),
        Some(ParkFact {
            realm: "alpha".to_string(),
            hearth: "label".to_string(),
            run: "r".to_string(),
            phase: Some("verify".to_string()),
            at: 3,
            answered: 9,
            ended: 9,
        })
    );
}

#[test]
fn a_park_out_of_scope_is_no_fact() {
    assert!(
        park_fact(&park("r"), "label", Some("beta")).is_none(),
        "another realm's park"
    );
}

#[test]
fn a_hard_stop_without_a_rule_is_still_a_stop() {
    let mut row = row("r");
    row.ending_rule = None;
    assert_eq!(
        finished(&row).unwrap().hard_stop,
        Some(HardStop { rule: None })
    );
    let mut completed = row;
    completed.hard_stop = false;
    assert_eq!(finished(&completed).unwrap().hard_stop, None);
}

#[test]
fn a_row_without_its_finish_is_open_and_one_without_its_ending_has_none() {
    let mut open = row("r");
    open.finished_ms = None;
    assert_eq!(finished(&open), None);
    let mut unended = row("r");
    unended.ending = None;
    assert_eq!(finished(&unended), None);
}

fn site<'a>(started: Option<i64>, provider: Option<&'a str>) -> WindowSite<'a> {
    WindowSite {
        run_id: "r",
        opened: 0,
        realm: Some("alpha"),
        recipe: Some("recipe"),
        open: false,
        finished_ms: Some(10),
        phase: None,
        provider,
        model: Some("model"),
        started_ms: started,
        metering: Metering::Session,
        reported: true,
        usage: None,
    }
}

#[test]
fn a_site_row_needs_the_start_and_provider_a_whole_read_opens_it_by() {
    assert!(site_fact(&site(None, Some("claude")), "label").is_none());
    assert!(site_fact(&site(Some(1), None), "label").is_none());
    let fact = site_fact(&site(Some(1), Some("claude")), "label").unwrap();
    assert_eq!(
        (
            fact.realm.as_str(),
            fact.recipe.as_deref(),
            fact.at,
            fact.provider.as_str(),
            fact.model.as_deref(),
            fact.reported,
            fact.open,
            fact.ended,
        ),
        (
            "alpha",
            Some("recipe"),
            1,
            "claude",
            Some("model"),
            true,
            false,
            Some(10),
        )
    );
}
