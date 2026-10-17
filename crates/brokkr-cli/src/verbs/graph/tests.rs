//! `brokkr graph`'s own part (#568, the Library): the realm it draws in,
//! where a recipe's table resolves, the journal it hands over, the lines it
//! paints, and its refusals. The journals are synthetic, written here.

use brokkr_core::EventType;
use brokkr_store::Store;
use clap::Parser;
use serde_json::{json, Value};

use super::*;
use crate::verbs::analytics::RangeRefused;
use crate::Cli;

/// The repository root: its `recipes/` holds `self`, which composes
/// `panel-review` over its base.
fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// A journal under `dir`, written one run at a time.
struct Journal {
    store: Store,
}

impl Journal {
    fn at(dir: &Path) -> Journal {
        Journal {
            store: Store::open(&dir.join("forge.db")).unwrap(),
        }
    }

    /// A run of the recipe `manifest` names, through `events`.
    fn run(&mut self, run_id: &str, manifest: &Value, events: &[(EventType, Value)]) {
        let recipe = manifest["bundle_name"].as_str().unwrap();
        self.store
            .create_run(run_id, "Draw the graph", recipe, manifest)
            .unwrap();
        let start = json!({"feature": "Draw the graph\nwith its counts", "manifest": manifest});
        let started = [(EventType::RunStarted, start)];
        for (event_type, payload) in started.iter().chain(events) {
            self.store
                .append_next(run_id, *event_type, payload.clone(), None, None)
                .unwrap();
        }
    }
}

fn entered(phase: &str) -> (EventType, Value) {
    (EventType::PhaseEntered, json!({"phase": phase}))
}

fn ruled(rule: &str, from: &str, next: Option<&str>) -> (EventType, Value) {
    let decided = json!({"from": from, "result": "resolved", "rule_id": rule, "next": next,
                         "severity": next.map(|_| "normal"), "inputs": {}, "problem": null});
    (EventType::TransitionDecided, decided)
}

/// One seat's attempt that succeeded with `result`.
fn seat(phase: &str, result: &str) -> [(EventType, Value); 3] {
    [
        (
            EventType::EffectRequested,
            json!({"effect_id": phase, "seat": phase, "phase": phase}),
        ),
        (
            EventType::EffectStarted,
            json!({"effect_id": phase, "attempt_id": phase}),
        ),
        (
            EventType::EffectSucceeded,
            json!({"effect_id": phase, "attempt_id": phase, "result": {"result": result}}),
        ),
    ]
}

fn asked<'a>(recipe: &'a str, now: &'a str) -> Asked<'a> {
    Asked {
        recipe,
        realm: UNMAPPED,
        range: Range::Day,
        now,
    }
}

fn hearth(dir: &Path) -> Vec<Hearth> {
    vec![Hearth {
        realms: Vec::new(),
        journal: dir.join("forge.db"),
    }]
}

#[test]
fn the_realm_is_the_named_one_the_workspaces_or_the_maps_first() {
    let dir = tempfile::tempdir().unwrap();
    let workspace = dir.path().join("tree");
    std::fs::create_dir(&workspace).unwrap();
    assert_eq!(
        place(None, None, &workspace),
        Ok((UNMAPPED.to_string(), workspace.clone()))
    );
    assert_eq!(
        place(None, Some("named"), &workspace),
        Ok(("named".to_string(), workspace.clone()))
    );
    let map = dir.path().join("realms.json");
    std::fs::write(
        &map,
        r#"{"schema":"forge.realms/v1",
            "realms":[{"name":"first","path":"elsewhere","default_branch":"main"},
                      {"name":"solo","path":"tree","default_branch":"main"}],
            "journal":"j.db"}"#,
    )
    .unwrap();
    let world = World::load(&map).unwrap();
    let solo = world.path_of(&world.map.realms[1]);
    let first = world.path_of(&world.map.realms[0]);
    assert_eq!(
        place(Some(&world), None, &workspace),
        Ok(("solo".to_string(), solo.clone()))
    );
    assert_eq!(
        place(Some(&world), None, dir.path()),
        Ok(("first".to_string(), first))
    );
    assert_eq!(
        place(Some(&world), Some("solo"), dir.path()),
        Ok(("solo".to_string(), solo))
    );
    let unknown = place(Some(&world), Some("nope"), dir.path()).unwrap_err();
    assert_eq!(
        unknown,
        UnknownRealm {
            realm: "nope".into(),
            known: vec!["first".into(), "solo".into()]
        }
    );
    assert_eq!(
        unknown.to_string(),
        "no realm 'nope' in the map: it names first, solo"
    );
}

#[test]
fn a_bundle_directory_is_found_by_the_name_its_bundle_json_gives_it() {
    let dir = tempfile::tempdir().unwrap();
    let write = |path: &str, bytes: &str| {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, bytes).unwrap();
    };
    write("a/bundle.json", r#"{"name": "alpha"}"#);
    write("deep/b/bundle.json", r#"{"name": "beta", "cost": "low"}"#);
    // Three down, hidden, build output, unreadable, and a file: none.
    write("deep/b/c/bundle.json", r#"{"name": "too-deep"}"#);
    write(".hidden/bundle.json", r#"{"name": "hidden"}"#);
    write("target/x/bundle.json", r#"{"name": "built"}"#);
    write("node_modules/y/bundle.json", r#"{"name": "vendored"}"#);
    write("bad/bundle.json", "not json");
    write("file.txt", "");
    // The shelf the folder resolves by name, ahead of its bundles.
    std::fs::create_dir_all(dir.path().join("recipes/one")).unwrap();
    let found = shelf(dir.path()).unwrap();
    let named: Vec<(&str, PathBuf)> = found
        .iter()
        .map(|(name, path)| {
            (
                name.as_str(),
                path.strip_prefix(dir.path()).unwrap().to_path_buf(),
            )
        })
        .collect();
    assert_eq!(
        named,
        [
            ("one", PathBuf::from("recipes/one")),
            ("alpha", PathBuf::from("a")),
            ("beta", PathBuf::from("deep/b")),
        ]
    );
    assert_eq!(bundles(&dir.path().join("absent"), BUNDLE_DEPTH), []);
}

#[test]
fn a_composed_recipe_is_drawn_from_its_table_with_its_traffic_and_drift() {
    let repository = repository();
    let shelf = shelf(&repository).unwrap();
    let bundle = compiled(None, &repository, &shelf, "self")
        .unwrap()
        .unwrap();
    let first = &bundle.machine.rules[0];
    let next = first.next.as_deref().unwrap();
    // The same table, but its base layer as it was before it moved.
    let mut moved = bundle.manifest.clone();
    let files = moved["files"].as_object_mut().unwrap();
    let base = files
        .keys()
        .find(|key| key.starts_with("@compose/"))
        .unwrap()
        .clone();
    files.insert(base, json!("an earlier base"));
    let dir = tempfile::tempdir().unwrap();
    let mut journal = Journal::at(dir.path());
    // Each ruling follows the seat whose result it rules, as the fold
    // requires.
    let mut shipped = vec![entered(&first.from)];
    shipped.extend(seat(&first.from, &first.result));
    shipped.extend([ruled(&first.id, &first.from, Some(next)), entered(next)]);
    shipped.extend(seat(next, "resolved"));
    shipped.extend([ruled("OLD-RULE", next, Some("done")), entered("done")]);
    shipped.push((EventType::RunCompleted, json!({})));
    journal.run("shipped", &bundle.manifest, &shipped);
    let mut stopped = vec![entered(&first.from)];
    stopped.extend(seat(&first.from, &first.result));
    stopped.extend([ruled(&first.id, &first.from, Some("stop")), entered("stop")]);
    stopped.push((EventType::RunStopped, json!({})));
    journal.run("stopped", &moved, &stopped);
    let now = now_rfc3339();
    let view = derive(
        &hearth(dir.path()),
        Some(&bundle),
        &shelf,
        &asked("self", &now),
    )
    .unwrap();
    assert_eq!(
        (view.recipe.as_str(), &view.table),
        ("self", &Table::Resolved)
    );
    assert_eq!(view.drifted_runs, 1, "only the run under the earlier base");
    assert_eq!(
        serde_json::to_value(&view).unwrap()["view_version"],
        json!(1)
    );
    let painted = paint(&view);
    let lines: Vec<&str> = painted.lines().collect();
    assert_eq!(
        lines[0],
        format!("world / self · {} → {now} · 2 runs", view.from)
    );
    assert_eq!(lines.len(), 1 + view.states.len() + view.rules.len() + 3);
    let intake = format!(
        "state  {} · entered 2 · median 0m · p90 0m · not reported",
        first.from
    );
    assert!(lines.contains(&intake.as_str()), "{painted}");
    let rule = format!(
        "rule   {} · 2 · {} → {next} · forward",
        first.id, first.from
    );
    assert!(lines.contains(&rule.as_str()), "{painted}");
    assert_eq!(
        &lines[lines.len() - 3..],
        [
            "retired OLD-RULE · 1",
            "drift  1 run ran under an earlier version of this table",
            "ends   1 done · 1 stop by a rule · 0 stopped by you · 0 open",
        ]
    );
}

#[test]
fn a_recipe_only_runs_carry_is_drawn_from_them_and_its_live_run_named() {
    let dir = tempfile::tempdir().unwrap();
    let mut journal = Journal::at(dir.path());
    let gone = json!({"bundle_name": "gone"});
    journal.run("live", &gone, &[entered("work")]);
    let now = now_rfc3339();
    let view = derive(&hearth(dir.path()), None, &[], &asked("gone", &now)).unwrap();
    let painted = paint(&view);
    assert_eq!(
        painted,
        format!(
            "world / gone · {} → {now} · 1 runs\n\
             this recipe's table was not found in world\n\
             state  work · entered 1 · median — · p90 — · not reported\n\
             ends   0 done · 0 stop by a rule · 0 stopped by you · 1 open\n\
             live   Draw the graph · work\n",
            view.from
        )
    );
}

#[test]
fn an_unknown_recipe_is_refused_naming_the_recipes_the_realm_knows() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("recipes/one")).unwrap();
    std::fs::create_dir_all(dir.path().join("two")).unwrap();
    std::fs::write(dir.path().join("two/bundle.json"), r#"{"name": "two"}"#).unwrap();
    let shelf = shelf(dir.path()).unwrap();
    let nothing = compiled(None, dir.path(), &shelf, "nosuch").unwrap();
    assert!(
        nothing.is_none(),
        "no table resolves for a name nothing gives"
    );
    let refused = derive(
        &hearth(dir.path()),
        None,
        &shelf,
        &asked("nosuch", &now_rfc3339()),
    );
    assert_eq!(
        refused.unwrap_err().to_string(),
        "no recipe 'nosuch' in world: the recipes it knows are one and two"
    );
}

#[test]
fn the_verb_prints_the_view_as_lines_or_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    Journal::at(dir.path()).run("live", &json!({"bundle_name": "gone"}), &[entered("work")]);
    for json in [false, true] {
        let args = GraphArgs {
            recipe: "gone".into(),
            realm: None,
            range: Range::Week,
            realms: None,
            db: Some(dir.path().join("forge.db")),
            json,
        };
        assert_eq!(graph(dir.path(), args).unwrap(), Exit::Completed.into());
    }
    let unknown = GraphArgs {
        recipe: "nosuch".into(),
        realm: None,
        range: Range::Week,
        realms: None,
        db: Some(dir.path().join("forge.db")),
        json: true,
    };
    assert_eq!(
        graph(dir.path(), unknown).unwrap_err().to_string(),
        "no recipe 'nosuch' in world: the recipes it knows are gone"
    );
}

#[test]
fn clap_takes_the_recipe_and_refuses_a_range_outside_the_three() {
    let refused = Cli::try_parse_from(["brokkr", "graph", "--recipe", "self", "--range", "2d"])
        .err()
        .expect("2d is no range");
    assert_eq!(refused.kind(), clap::error::ErrorKind::ValueValidation);
    let source = std::error::Error::source(&refused)
        .and_then(|source| source.downcast_ref::<RangeRefused>())
        .map(|refused| refused.0.to_string());
    assert_eq!(
        source.as_deref(),
        Some("unknown range '2d': one of 24h, 7d or 30d")
    );
    let missing = Cli::try_parse_from(["brokkr", "graph"])
        .err()
        .expect("no recipe");
    assert_eq!(
        missing.kind(),
        clap::error::ErrorKind::MissingRequiredArgument
    );
}
