//! Decision 0031's pin walk over the TYPED tree (#349): an unpinned inline
//! site is refused in 0031's own words wherever a body form puts it, nested
//! forms included, and a site the walk refuses is not refused first for a
//! consequence of the pin it lacks.

use super::*;

/// The typed seats `enforce_model_pins` reads, built from the JSON a test
/// writes: an inline command, an agent reference (which writes none), a
/// panel or a sequence, each site under the name the parse gives it.
pub(super) fn typed_seats(seats: &Value) -> BTreeMap<String, Seat> {
    let written = |raw: &Value| match raw.get("agent") {
        Some(_) => Vec::new(),
        None => command_parts(raw),
    };
    let members = |raw: &Value| {
        let member = |(name, raw): (&String, &Value)| PanelMember {
            name: name.clone(),
            role_path: PathBuf::new(),
            command: Vec::new(),
            candidates: Vec::new(),
            written: written(raw),
            class: SeatClass::Work,
        };
        let panel = raw.get("panel").and_then(Value::as_object);
        panel.map(|panel| panel.iter().map(member).collect::<Vec<_>>())
    };
    let step = |(index, raw): (usize, &Value)| SequenceStep {
        name: step_label(index, raw),
        class: SeatClass::Work,
        results: Vec::new(),
        body: match members(raw) {
            Some(members) => StepBody::Invocation(Invocation::Panel {
                members,
                aggregate: Aggregate::UnanimousPass,
            }),
            None => StepBody::Invocation(Invocation::Single {
                role_path: PathBuf::new(),
                command: Vec::new(),
                candidates: Vec::new(),
                written: written(raw),
                class: (),
            }),
        },
    };
    let body = |raw: &Value| match (members(raw), raw.get("sequence")) {
        (Some(members), _) => SeatBody::Invocation(Invocation::Panel {
            members,
            aggregate: Aggregate::UnanimousPass,
        }),
        (None, Some(steps)) => SeatBody::Sequence {
            steps: steps
                .as_array()
                .unwrap()
                .iter()
                .enumerate()
                .map(step)
                .collect(),
        },
        (None, None) => SeatBody::Invocation(Invocation::Single {
            role_path: PathBuf::new(),
            command: Vec::new(),
            candidates: Vec::new(),
            written: written(raw),
            class: SeatClass::Work,
        }),
    };
    let seats = seats.as_object().unwrap().iter();
    seats
        .map(|(phase, raw)| {
            let seat = Seat {
                gate: crate::bundle::GateAssessment::ContainsWork(crate::bundle::WorkSite::seat()),
                results: Vec::new(),
                limits: Limits::default(),
                inputs: Vec::new(),
                secrets: Vec::new(),
                body: body(raw),
            };
            (phase.clone(), seat)
        })
        .collect()
}

#[test]
fn a_selector_with_a_gate_default_is_whole_gate() {
    // Decision 0094: compile refuses empty panels, sequences and selectors;
    // their minimums are removal-controlled in bundle::tests, not here.
    let select = |default| SeatBody::Select {
        cases: BTreeMap::new(),
        default,
    };
    let gate = SeatBody::Invocation(Invocation::Single {
        role_path: PathBuf::new(),
        command: Vec::new(),
        candidates: Vec::new(),
        written: Vec::new(),
        class: SeatClass::Gate,
    });
    assert!(select(Some(Box::new(gate))).is_gate());
}

/// What lets a selected sequence's whole-effect fact be read off its
/// compiled steps (`SiteCompiler::parse_case`): it holds no dialect step, because
/// its `phase:case` label is no dialect phase, whichever operation it names.
#[test]
fn a_selected_sequence_holds_no_dialect_step() {
    let dialect = Dialect::load(&workspace().join("dialects/openspec.json"))
        .unwrap()
        .0;
    let dir = tempfile::tempdir().unwrap();
    let results = ["clear".to_string(), "ambiguous".to_string()];
    let refusals = [
        ("check", "sequence step 'clarify:engine:count' uses dialect operation 'check'; phase 'clarify:engine' needs 'validate'"),
        ("validate", "dialect 'openspec' declares phase 'clarify:engine' validate unsupported"),
    ];
    let (charters, case_origin) = (Charters::default(), BTreeMap::new());
    for (operation, expected) in refusals {
        let compiler = &mut SiteCompiler::new(
            crate::bundle::tests::unopened(),
            (Boundary::Namespace, &crate::argv::tests::ENGINE),
            Some(&dialect),
            &charters,
            &[],
            &case_origin,
        );
        let at = SeatScope {
            dir: dir.path(),
            results: &results,
            secrets: &[],
        };
        let raw = json!({"sequence": [
            {"name": "count", "results": ["clear"], "dialect": operation},
            {"name": "judge", "role": "roles/role.md", "driver": {"command": ["driver"]}}
        ]});
        match parse_sequence(compiler, at, "clarify:engine", &raw) {
            Err(CompileError::Invalid(refusal)) => assert_eq!(refusal, expected),
            other => panic!("a selected dialect step must be refused: {other:?}"),
        }
    }
}

/// An inline built-in that pins its effort and no model.
fn unpinned() -> Value {
    json!({"role": "roles/role.md",
           "driver": {"command": ["{brokkr}", "driver", "claude", "--", "--effort", "high"]}})
}

/// An `exec` site, which takes neither pin.
pub(super) fn exec() -> Value {
    json!({"role": "roles/role.md",
           "driver": {"command": ["{brokkr}", "driver", "exec", "--", "true"]}})
}

pub(super) fn panel(member: Value) -> Value {
    json!({"aggregate": "unanimous-pass", "panel": {"a": member, "b": exec()}})
}

pub(super) fn sequence(mut last: Value) -> Value {
    let mut first = exec();
    first["name"] = json!("first");
    first["results"] = json!(["pass", "fail"]);
    last["name"] = json!("last");
    json!({"sequence": [first, last]})
}

pub(super) fn select(cases: Value, default: Value) -> Value {
    json!({"select": {"on": "strategy", "cases": cases, "default": default}})
}

/// The `work` seat around `body`, refused for the one unpinned site
/// `label`, by decision 0031 and in its exact words.
fn assert_unpinned(body: Value, label: &str) {
    let mut work = body;
    work["results"] = json!(["pass", "fail"]);
    match Fixture::new().compile(work) {
        Err(CompileError::Invalid(refusal)) => assert_eq!(
            refusal,
            format!(
                "seats '{label}' do not pin a model; add '--model <concrete-model-id>' to each \
                 driver.command (decision 0031 ruling 2)"
            )
        ),
        other => panic!("'{label}' must be refused for its pin: {other:?}"),
    }
}

#[test]
fn an_unpinned_single_seat_is_refused_by_0031() {
    assert_unpinned(unpinned(), "work");
}

#[test]
fn an_unpinned_panel_member_is_refused_by_0031() {
    assert_unpinned(panel(unpinned()), "work:a");
}

#[test]
fn an_unpinned_sequence_step_is_refused_by_0031() {
    assert_unpinned(sequence(unpinned()), "work:last");
}

#[test]
fn an_unpinned_member_of_a_sequence_steps_panel_is_refused_by_0031() {
    assert_unpinned(sequence(panel(unpinned())), "work:last:a");
}

#[test]
fn an_unpinned_select_case_is_refused_by_0031() {
    assert_unpinned(select(json!({"chore": unpinned()}), exec()), "work:chore");
}

#[test]
fn an_unpinned_select_default_is_refused_by_0031() {
    assert_unpinned(select(json!({}), unpinned()), "work:default");
}

#[test]
fn an_unpinned_member_of_a_select_cases_panel_is_refused_by_0031() {
    let cases = json!({"design": panel(unpinned())});
    assert_unpinned(select(cases, exec()), "work:design:a");
}

#[test]
fn an_unpinned_step_of_a_select_cases_sequence_is_refused_by_0031() {
    let cases = json!({"engine": sequence(unpinned())});
    assert_unpinned(select(cases, exec()), "work:engine:last");
}

/// A gate's judges and a secret's destination are both read off the model
/// pin. A site that states none, or one no reader can make out, is refused
/// for the pin, which names the repair, and not for what the pin's absence
/// does to either reading.
#[test]
fn a_site_is_refused_for_its_missing_pin_before_its_route_is_judged() {
    let fixture = Fixture::new();
    let mut claude = adapter("claude", Some("trusted"), Some(false));
    claude["model_flag"] = json!("--model");
    claude["efforts"] = json!(["high"]);
    claude["effort_flag"] = json!("--effort");
    fixture.write_adapter(claude);
    let mut gate = unpinned();
    gate["class"] = json!("gate");
    let mut secret = unpinned();
    secret["driver"]["command"] =
        json!(["{brokkr}", "driver", "claude", "--", "--effort", "high", "--model"]);
    secret["secrets"] = json!(["TOKEN"]);
    for mut work in [gate, secret] {
        work["results"] = json!(["pass", "fail"]);
        match fixture.compile(work.clone()) {
            Err(CompileError::Invalid(refusal)) => assert_eq!(
                refusal,
                "seats 'work' do not pin a model; add '--model <concrete-model-id>' to each \
                 driver.command (decision 0031 ruling 2)",
                "{work}"
            ),
            other => panic!("{work} must be refused for its pin: {other:?}"),
        }
    }
}
