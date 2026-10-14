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

/// A seat's form is read before its results are: a seat with no `results`
/// is refused for its form's defect first, and once the defect goes, for
/// the results it still lacks, so the results check stays armed.
#[test]
fn a_seat_meets_its_form_before_its_results() {
    let mut tools = panel(exec());
    tools["tools"] = json!({});
    let seats = [
        (
            json!({"agent": "office", "role": "roles/role.md"}),
            json!({"agent": "office"}),
            "bundle: seat 'work' combines 'agent' with 'role'; an agent reference is total — \
             'role' states what the agent IS, and a seat that could amend it would make \
             `brokkr agents show` a lie for that seat",
        ),
        (
            json!({"agent": "office", "panel": {}}),
            json!({"agent": "office"}),
            "bundle: seat 'work' must be exactly one of role+driver, agent, panel, sequence, \
             or select",
        ),
        (
            tools,
            panel(exec()),
            "bundle: seat 'work' declares 'tools' beside a panel, sequence or select; a local \
             declaration belongs to the site that executes — the member, step or case body — \
             and a container cannot own one, even an empty object, because it would either \
             become a shared grant or be ignored (decision 0065 slice one, design D5)",
        ),
    ];
    let fixture = Fixture::new();
    for (defective, repaired, refusal) in seats {
        assert_eq!(fixture.refusal(defective), refusal);
        assert_eq!(
            fixture.refusal(repaired),
            "bundle: seat 'work' needs non-empty 'results'"
        );
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
