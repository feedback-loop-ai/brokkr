//! Each compiled site meets its own form (#349): the same body, written at
//! a seat, a select case, a panel member and a sequence step, is refused in
//! each site's own words.

use super::pin_walk_tests::{exec, panel, select, sequence};
use super::*;

#[test]
fn each_site_meets_its_own_form() {
    let body = || json!({"agent": "office", "panel": {}});
    let sites = [
        (
            body(),
            "bundle: seat 'work' must be exactly one of role+driver, agent, panel, sequence, \
             or select",
        ),
        (
            select(json!({"chore": body()}), exec()),
            "bundle: seat 'work:chore' must be exactly one of role+driver, agent, panel, or \
             sequence",
        ),
        (
            panel(body()),
            "bundle: seat 'work:a' has unknown key 'panel'; known: class, agent, role, driver, \
             hands, capabilities, tools. The site vocabulary is closed because a declaration \
             this compiler cannot see is a declaration that was never made — a misspelled \
             'class' would leave a gate reading as work (decision 0021 ruling 1)",
        ),
        (
            sequence(body()),
            "bundle: sequence step 'work:last' must be exactly one of role+driver, agent, \
             panel, or dialect",
        ),
    ];
    let fixture = Fixture::new();
    for (mut work, refusal) in sites {
        work["results"] = json!(["pass", "fail"]);
        assert_eq!(fixture.refusal(work), refusal);
    }
}

#[test]
fn an_unknown_aggregate_names_the_vocabulary() {
    let mut work = panel(exec());
    work["aggregate"] = json!("majority");
    work["results"] = json!(["pass", "fail"]);
    assert_eq!(
        Fixture::new().refusal(work),
        "bundle: seat 'work' unknown aggregate 'majority'; known: unanimous-pass, review-panel"
    );
}
