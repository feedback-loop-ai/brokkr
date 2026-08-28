use super::*;
use serde_json::json;

fn table(rule: Value) -> Value {
    json!({
        "phases": ["work", "done"],
        "initial": "work",
        "terminal": ["done"],
        "rules": [rule],
    })
}

fn rule() -> Value {
    json!({
        "id": "WORK-DONE",
        "from": "work",
        "result": "complete",
        "next": "done",
        "reason": "complete",
    })
}

#[test]
fn loader_refuses_unreachable_phase_and_required_field_defects() {
    assert!(Machine::from_table(&Value::Null)
        .unwrap_err()
        .0
        .contains("table must be an object"));

    let mut value = table(rule());
    value["initial"] = json!(2);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("initial must be a string"));

    let mut value = table(rule());
    value["rules"] = json!({});
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("rules must be an array"));

    let mut value = table(rule());
    value["phases"] = json!(["work", 2]);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("entries must be strings"));

    let mut value = table(rule());
    value["rules"] = json!([2]);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("rule must be an object"));

    let mut value = table(rule());
    value["initial"] = json!("elsewhere");
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("initial phase"));

    let mut value = table(rule());
    value["terminal"] = json!(["elsewhere"]);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("terminal phase"));

    let mut value = table(rule());
    value["rules"][0].as_object_mut().unwrap().remove("reason");
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("missing 'reason'"));

    let mut value = table(rule());
    value["rules"][0]["next"] = json!("elsewhere");
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("unknown phase"));

    let mut value = table(rule());
    value["rules"][0]["from"] = json!("elsewhere");
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("unknown phase"));

    let mut value = table(rule());
    value["rules"][0]["severity"] = json!(2);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("severity must be a string"));

    let mut value = table(rule());
    value["rules"][0]["when"] = json!(2);
    assert!(Machine::from_table(&value)
        .unwrap_err()
        .0
        .contains("'when' must be an object"));
}

#[test]
fn every_runtime_condition_shape_is_strict() {
    let counter = Condition::CounterGte {
        name: "consecutive_failures".into(),
        threshold: 2.0,
    };
    let severity = Condition::SeverityAbove {
        name: "max_residual_severity".into(),
        threshold_rank: severity_rank("medium").unwrap(),
    };
    let flag = Condition::Flag {
        name: "fixes_applied".into(),
        expected: true,
    };

    assert_eq!(
        conditions_met(std::slice::from_ref(&counter), &Map::new()),
        Ok(false)
    );
    assert_eq!(
        conditions_met(
            std::slice::from_ref(&counter),
            &json!({"consecutive_failures": 1})
                .as_object()
                .unwrap()
                .clone()
        ),
        Ok(false)
    );
    assert!(conditions_met(
        std::slice::from_ref(&counter),
        &json!({"consecutive_failures": "two"})
            .as_object()
            .unwrap()
            .clone()
    )
    .is_err());

    assert_eq!(
        conditions_met(
            std::slice::from_ref(&severity),
            &json!({"max_residual_severity": "low"})
                .as_object()
                .unwrap()
                .clone()
        ),
        Ok(false)
    );
    assert!(conditions_met(
        std::slice::from_ref(&severity),
        &json!({"max_residual_severity": "unknown"})
            .as_object()
            .unwrap()
            .clone()
    )
    .is_err());
    assert!(conditions_met(
        std::slice::from_ref(&severity),
        &json!({"max_residual_severity": 3})
            .as_object()
            .unwrap()
            .clone()
    )
    .is_err());

    assert_eq!(
        conditions_met(std::slice::from_ref(&flag), &Map::new()),
        Ok(false)
    );
    assert_eq!(
        conditions_met(
            std::slice::from_ref(&flag),
            &json!({"fixes_applied": false}).as_object().unwrap().clone()
        ),
        Ok(false)
    );
    assert!(conditions_met(
        std::slice::from_ref(&flag),
        &json!({"fixes_applied": "yes"}).as_object().unwrap().clone()
    )
    .is_err());
}
