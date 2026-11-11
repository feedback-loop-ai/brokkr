//! The row mappings `brokkr analytics` folds: a run's row to its facts and a
//! site's row to its fact, including the rows a whole read opens no run or
//! site from.

use std::collections::{BTreeMap, BTreeSet};

use brokkr_core::ending::Ending;
use brokkr_core::seat::meter::Metering;
use brokkr_store::aggregate::{WindowReviews, WindowRun};
use brokkr_store::metered::WindowSite;

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

#[test]
fn a_run_row_maps_its_facts_and_its_costs() {
    let mut spends = BTreeMap::new();
    spends.insert("r".to_string(), vec![(5, 0.25)]);
    let whole = BTreeSet::new();
    let facts = run_facts(&row("r"), "label", &spends, None, &whole).unwrap();
    assert_eq!(
        (
            facts.realm.as_str(),
            facts.recipe.as_deref(),
            facts.started,
            facts.open,
            facts.implement_visits,
            facts.reviews.clone(),
            facts.costs.clone(),
        ),
        (
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
            vec![(5, 0.25)],
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
fn a_run_out_of_scope_or_left_whole_is_no_row() {
    let spends = BTreeMap::new();
    assert!(
        run_facts(&row("r"), "label", &spends, Some("beta"), &BTreeSet::new()).is_none(),
        "another realm's run"
    );
    let mut whole = BTreeSet::new();
    whole.insert("r".to_string());
    assert!(
        run_facts(&row("r"), "label", &spends, None, &whole).is_none(),
        "a run read whole"
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
