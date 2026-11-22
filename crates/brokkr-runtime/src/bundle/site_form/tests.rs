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
    let first = |raw: &Value, own: Result<(), SiteRefusal>| {
        assert_eq!(
            read_form(WHAT, raw, form).unwrap_err(),
            own.unwrap_err(),
            "{raw}"
        );
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

/// The site every typed assert below names.
const WHAT: &str = "work:site";

/// The first refusal `raw` meets at a seat, as its typed cause.
fn first_at_seat(raw: &Value) -> SiteRefusal<'_> {
    read_form(WHAT, raw, &SEAT).unwrap_err()
}

#[test]
fn each_check_raises_its_own_variant() {
    let what = WHAT;
    let raw = json!({"boundary": "x", "consumes": [], "tools": {}});
    assert_eq!(
        refuse_boundary_key(what, &raw),
        Err(SiteRefusal::Boundary { what })
    );
    let key = "consumes";
    assert_eq!(
        refuse_crossing_keys(what, &raw),
        Err(SiteRefusal::Crossing { what, key })
    );
    let (unknown, key, known) = (json!({"class": "work", "bogus": 1}), "bogus", MEMBER_KEYS);
    assert_eq!(
        refuse_unknown_keys(what, &unknown, known),
        Err(SiteRefusal::UnknownKey { what, key, known })
    );
    let driver = json!({"driver": {"confine": true}});
    assert_eq!(
        refuse_confine(what, &driver),
        Err(SiteRefusal::ConfineRetired { what })
    );
    let key = "confine";
    assert_eq!(
        refuse_driver_keys(what, &driver),
        Err(SiteRefusal::DriverKey { what, key })
    );
    assert_eq!(
        refuse_tools_on_container(what, &raw),
        Err(SiteRefusal::ToolsOnContainer { what })
    );
    let words = vec!["role+driver", "agent"];
    assert_eq!(
        exactly_one(what, "seat", &[Shape::Inline, Shape::Agent]),
        SiteRefusal::ExactlyOne {
            what,
            noun: "seat",
            words
        }
    );
}

#[test]
fn an_agent_reference_refuses_each_amendment_by_its_own_variant() {
    let (what, key) = (WHAT, "hands");
    let raw = json!({"agent": "office", "hands": "workspace"});
    assert_eq!(
        refuse_amendments(what, &raw),
        Err(SiteRefusal::Amends { what, key })
    );
    let raw = json!({"agent": "office", "driver": "driver"});
    assert_eq!(
        refuse_amendments(what, &raw),
        Err(SiteRefusal::DriverNotObject { what })
    );
    let (raw, key) = (
        json!({"agent": "office", "driver": {"command": []}}),
        "command",
    );
    assert_eq!(
        refuse_amendments(what, &raw),
        Err(SiteRefusal::AmendsDriver { what, key })
    );
}

/// The text after a driver key's place sentence, written out once here.
const UNREAD: &str = " A key the compiler does not read is a declaration that was never made — \
    a capability placed there would compile, deliver nothing and run the seat at its harness \
    default — so it is refused rather than ignored (decision 0004; decision 0065 slice one, \
    rebuild unit 5e)";

#[test]
fn a_driver_key_is_named_when_short_and_placed_when_it_is_a_capability() {
    let long = "k".repeat(65);
    let hidden = "one that is not a short name and is not echoed";
    let site = " 'tools' is a site declaration, written on the seat beside its driver.";
    let tool = " 'sandbox' is a typed tool field, written as 'tools.sandbox' on the seat.";
    for (key, named, place) in [
        ("short-key_1.x", "'short-key_1.x'", ""),
        (long.as_str(), hidden, ""),
        ("two words", hidden, ""),
        ("tools", "'tools'", site),
        ("sandbox", "'sandbox'", tool),
        ("image", "'image'", ""),
    ] {
        let mut raw = json!({"driver": {}});
        raw["driver"][key] = json!(true);
        let refusal = refuse_driver_keys(WHAT, &raw).unwrap_err();
        assert_eq!(refusal, SiteRefusal::DriverKey { what: WHAT, key });
        let text = "seat 'work:site' driver has an unknown key, ";
        assert_eq!(
            refusal.to_string(),
            format!("{text}{named}; known: command.{place}{UNREAD}")
        );
    }
}

#[test]
fn the_amendments_and_the_crossings_refuse_in_their_own_order() {
    let (what, key) = (WHAT, "role");
    let raw = json!({"agent": "office", "role": "roles/role.md", "driver": {"command": []}});
    assert_eq!(first_at_seat(&raw), SiteRefusal::Amends { what, key });
    let (raw, key) = (
        json!({"agent": "office", "inputs": {}, "hands": "workspace"}),
        "inputs",
    );
    assert_eq!(first_at_seat(&raw), SiteRefusal::Amends { what, key });
    let (raw, key) = (
        json!({"agent": "office", "hands": "workspace", "driver": "d"}),
        "hands",
    );
    assert_eq!(first_at_seat(&raw), SiteRefusal::Amends { what, key });
    let raw = json!({"agent": "office", "driver": "driver"});
    assert_eq!(first_at_seat(&raw), SiteRefusal::DriverNotObject { what });
    let (raw, key) = (json!({"publishes": [], "consumes": []}), "publishes");
    assert_eq!(first_at_seat(&raw), SiteRefusal::Crossing { what, key });
}

#[test]
fn what_the_shared_checks_admit_stays_admitted() {
    let beside = json!({"agent": "office", "results": [], "secrets": [], "class": "work",
                        "limits": {}});
    assert_eq!(read_form(WHAT, &beside, &SEAT), Ok(SeatShape::Agent));
    assert_eq!(refuse_amendments(WHAT, &beside), Ok(()));
    let empty = json!({"agent": "office", "driver": {}});
    assert_eq!(read_form(WHAT, &empty, &SEAT), Ok(SeatShape::Agent));
    assert_eq!(refuse_unknown_keys(WHAT, &json!("site"), SEAT_KEYS), Ok(()));
    assert_eq!(refuse_driver_keys(WHAT, &json!({"driver": "d"})), Ok(()));
}

#[test]
fn every_site_form_refusal_reads_as_it_always_has() {
    let (what, key) = (WHAT, "bogus");
    let known = &["class", "agent"][..];
    let words = vec!["role+driver", "agent", "panel"];
    let pins = [
        (
            SiteRefusal::Boundary { what },
            "seat 'work:site' declares boundary; the boundary is declared by the realm \
             (realms.json, forge.realms/v4) and never by a bundle or an agent, because the \
             machine a realm runs on is the realm's fact (decision 0046 ruling 1)",
        ),
        (
            SiteRefusal::Crossing {
                what,
                key: "publishes",
            },
            "seat 'work:site' declares publishes; a crossing is declared by the realm \
             (realms.json, forge.realms/v5) and never by a bundle, because the file one realm \
             publishes and the digest another pins it at is the realm's fact and not a \
             recipe's (decision 0057 ruling 1)",
        ),
        (
            SiteRefusal::UnknownKey { what, key, known },
            "seat 'work:site' has unknown key 'bogus'; known: class, agent. The site vocabulary \
             is closed because a declaration this compiler cannot see is a declaration that was \
             never made — a misspelled 'class' would leave a gate reading as work (decision 0021 \
             ruling 1)",
        ),
        (
            SiteRefusal::ConfineRetired { what },
            "seat 'work:site' declares driver.confine; decision 0008's container confinement \
             retired into the `container` boundary a realm declares (realms.json, \
             forge.realms/v4), and the field is refused until that boundary is measured in \
             decision 0046's slice (iii) (decision 0046 ruling 5)",
        ),
        (
            SiteRefusal::DriverKey { what, key: "hands" },
            "seat 'work:site' driver has an unknown key, 'hands'; known: command. 'hands' is a \
             site declaration, written on the seat beside its driver. A key the compiler does \
             not read is a declaration that was never made — a capability placed there would \
             compile, deliver nothing and run the seat at its harness default — so it is \
             refused rather than ignored (decision 0004; decision 0065 slice one, rebuild unit \
             5e)",
        ),
        (
            SiteRefusal::Amends {
                what,
                key: "inputs",
            },
            "seat 'work:site' combines 'agent' with 'inputs'; an agent reference is total — \
             'inputs' states what the agent IS, and a seat that could amend it would make \
             `brokkr library agents show` a lie for that seat",
        ),
        (
            SiteRefusal::DriverNotObject { what },
            "seat 'work:site' driver must be an object",
        ),
        (
            SiteRefusal::AmendsDriver {
                what,
                key: "command",
            },
            "seat 'work:site' combines 'agent' with 'driver.command'; an agent reference is \
             total — 'driver.command' states what the agent IS, and a seat that could amend it \
             would make `brokkr library agents show` a lie for that seat",
        ),
        (
            SiteRefusal::ExactlyOne {
                what,
                noun: "sequence step",
                words,
            },
            "sequence step 'work:site' must be exactly one of role+driver, agent, or panel",
        ),
        (
            SiteRefusal::ToolsOnContainer { what },
            "seat 'work:site' declares 'tools' beside a panel, sequence or select; a \
             local declaration belongs to the site that executes — the member, step or \
             case body — and a container cannot own one, even an empty object, because it \
             would either become a shared grant or be ignored (decision 0065 slice one, design D5)",
        ),
    ];
    for (refusal, text) in pins {
        assert_eq!(refusal.to_string(), text);
    }
}
