//! Each site form reads its body form after one shared order of refusals.

use std::fmt::Debug;

use super::*;
use serde_json::json;

/// The refusal `classify` raises for `raw` at `form`, as its operator reads it.
fn refusal<S: Copy + Debug>(raw: &Value, form: &SiteForm<S>) -> String {
    classify("work:site", raw, form).unwrap_err().to_string()
}

#[test]
fn each_form_names_its_admitted_forms_when_two_are_written() {
    let two = json!({"agent": "office", "panel": {}});
    assert_eq!(
        refusal(&two, &SEAT),
        "bundle: seat 'work:site' must be exactly one of role+driver, agent, panel, sequence, \
         or select"
    );
    assert_eq!(
        refusal(&two, &CASE),
        "bundle: seat 'work:site' must be exactly one of role+driver, agent, panel, or sequence"
    );
    assert_eq!(
        refusal(&two, &STEP),
        "bundle: sequence step 'work:site' must be exactly one of role+driver, agent, panel, \
         or dialect"
    );
}

#[test]
fn a_select_case_that_writes_no_form_is_refused_where_a_seat_step_or_member_reads_inline() {
    let none = json!({"class": "work"});
    assert_eq!(
        refusal(&none, &CASE),
        "bundle: seat 'work:site' must be exactly one of role+driver, agent, panel, or sequence"
    );
    assert_eq!(classify("work", &none, &SEAT).unwrap(), SeatShape::Inline);
    assert_eq!(classify("work:a", &none, &STEP).unwrap(), StepShape::Inline);
    assert_eq!(
        classify("work:a", &none, &MEMBER).unwrap(),
        MemberShape::Inline
    );
}

/// A site carrying every shared defect at once. `panel` is a defect only
/// where a site may write a container.
fn every_defect(container: bool) -> Value {
    let mut raw = json!({
        "boundary": "namespace",
        "publishes": "lane",
        "bogus": true,
        "driver": {"confine": true, "tools": {}},
        "agent": "office",
        "role": "roles/role.md",
        "tools": {},
    });
    if container {
        raw["panel"] = json!({});
    }
    raw
}

/// Remove defects from `raw` one at a time, and see each refused first in
/// the one order, by the refusal its own check raises, until `form` admits
/// `admitted`.
fn assert_refused_in_order<S: Copy + Debug + PartialEq>(
    form: &SiteForm<S>,
    container: bool,
    admitted: S,
) {
    let mut raw = every_defect(container);
    let first = |raw: &Value, own: Result<(), CompileError>| {
        assert_eq!(refusal(raw, form), own.unwrap_err().to_string(), "{raw}");
    };
    let what = "work:site";
    first(&raw, refuse_boundary_key(what, &raw));
    raw.as_object_mut().unwrap().remove("boundary");
    first(&raw, refuse_crossing_keys(what, &raw));
    raw.as_object_mut().unwrap().remove("publishes");
    first(&raw, refuse_unknown_keys(what, &raw, form.keys));
    raw.as_object_mut().unwrap().remove("bogus");
    first(&raw, refuse_confine(what, &raw));
    raw["driver"].as_object_mut().unwrap().remove("confine");
    first(&raw, refuse_driver_keys(what, &raw));
    raw.as_object_mut().unwrap().remove("driver");
    first(&raw, refuse_amendments(what, &raw));
    raw.as_object_mut().unwrap().remove("role");
    if container {
        let shapes: Vec<Shape> = form.shapes.iter().map(|(shape, _)| *shape).collect();
        first(&raw, Err(exactly_one(what, form.noun, &shapes)));
        raw.as_object_mut().unwrap().remove("agent");
        first(&raw, refuse_tools_on_container(what, &raw));
        raw.as_object_mut().unwrap().remove("tools");
    }
    assert_eq!(classify(what, &raw, form).unwrap(), admitted);
}

#[test]
fn every_form_refuses_in_one_order() {
    assert_refused_in_order(&SEAT, true, SeatShape::Panel);
    assert_refused_in_order(&CASE, true, CaseShape::Panel);
    assert_refused_in_order(&STEP, true, StepShape::Panel);
    assert_refused_in_order(&MEMBER, false, MemberShape::Agent);
}
