//! `brokkr graph`'s own part (#568, the Library): the realm it draws in,
//! where a recipe's table resolves, the journal it hands over, the lines it
//! paints, and its refusals. The journals are synthetic, written here.

use brokkr_core::policy::Machine;
use brokkr_core::EventType;
use brokkr_store::Store;
use clap::Parser;
use serde_json::{json, Value};

use super::*;
use crate::tui::drawn_links;
use crate::verbs::analytics::CrossRunCmd;
use crate::{Cli, Cmd, Exit};
use brokkr_view::RuleKind;

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

/// The bundle `compiles` holds for the recipe asked for.
fn bundle_of(compiles: Compiles) -> Bundle {
    let Compiled::Bundle(bundle) = compiles.asked else {
        panic!("the recipe asked for compiled");
    };
    *bundle
}

/// What compiling a shelf on which nothing resolves found.
fn unshelved() -> Compiles {
    Compiles {
        asked: Compiled::NotOnShelf,
        refused: Vec::new(),
    }
}

/// A folder `brokkr init` scaffolded, read through its links, with the
/// harness it seats found under `bin`.
fn adopter(dir: &Path) -> PathBuf {
    let repo = std::fs::canonicalize(dir).unwrap();
    let bins = repo.join("bin");
    std::fs::create_dir(&bins).unwrap();
    std::fs::write(bins.join("claude"), "").unwrap();
    crate::init::init(&repo, bins.as_os_str(), "linux", &repo, None).unwrap();
    repo
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
fn an_adopters_implicit_bundle_resolves_and_is_drawn_with_its_dormant_rules() {
    let dir = tempfile::tempdir().unwrap();
    let repo = adopter(dir.path());
    // A visible bundle beside the folder still resolves after it.
    std::fs::create_dir(repo.join("other")).unwrap();
    std::fs::write(repo.join("other/bundle.json"), r#"{"name": "other"}"#).unwrap();
    let shelf = shelf(&repo).unwrap();
    assert_eq!(
        shelf,
        [
            ("starter".to_string(), repo.join(".brokkr")),
            ("other".to_string(), repo.join("other")),
        ]
    );
    let compiles = compiled(None, &repo, &shelf, "starter").unwrap();
    // The bundle beside it is a name and no recipe, so it does not compile.
    assert_eq!(compiles.refused, ["other"]);
    let bundle = bundle_of(compiles);
    let journal = repo.join(".brokkr/state");
    let first = &bundle.machine.rules[0];
    let mut events = vec![entered(&first.from)];
    events.extend(seat(&first.from, &first.result));
    Journal::at(&journal).run("live", &bundle.manifest, &events);
    let now = now_rfc3339();
    let compiles = Compiles {
        asked: Compiled::Bundle(Box::new(bundle.clone())),
        refused: vec!["other".into()],
    };
    let view = derive(
        &hearth(&journal),
        &compiles,
        &shelf,
        &asked("starter", &now),
    )
    .unwrap();
    assert_eq!(view.does_not_compile, ["other"]);
    assert_eq!(
        (&view.table, view.runs, view.rules.len()),
        (&Table::Resolved, 1, bundle.machine.rules.len())
    );
    assert_eq!(view.never_fired.count, bundle.machine.rules.len());
}

#[test]
fn a_composed_recipe_is_drawn_from_its_table_with_its_traffic_and_drift() {
    let repository = repository();
    let shelf = shelf(&repository).unwrap();
    let bundle = bundle_of(compiled(None, &repository, &shelf, "self").unwrap());
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
    let compiles = Compiles {
        asked: Compiled::Bundle(Box::new(bundle.clone())),
        refused: Vec::new(),
    };
    let view = derive(&hearth(dir.path()), &compiles, &shelf, &asked("self", &now)).unwrap();
    assert_eq!(
        (view.recipe.as_str(), &view.table),
        ("self", &Table::Resolved)
    );
    assert_eq!(view.drifted_runs, 1, "only the run under the earlier base");
    assert_eq!(
        serde_json::to_value(&view).unwrap()["view_version"],
        json!(2)
    );
    let painted = paint(&view);
    let lines: Vec<&str> = painted.lines().collect();
    assert_eq!(
        lines[0],
        format!("world / self · {} → {now} · 2 runs", view.from)
    );
    assert_eq!(lines.len(), 1 + view.states.len() + view.rules.len() + 4);
    let intake = format!(
        "state  {} · entered 2 · median 0m · p90 0m · not reported · —",
        first.from
    );
    assert!(lines.contains(&intake.as_str()), "{painted}");
    let rule = format!(
        "rule   {} · 2 · {} → {next} · forward",
        first.id, first.from
    );
    assert!(lines.contains(&rule.as_str()), "{painted}");
    assert_eq!(
        &lines[lines.len() - 4..],
        [
            "retired OLD-RULE · 1",
            "drift  1 run ran under an earlier version of a base this recipe composes, \
             which may not have changed this table",
            "ends   1 done · 1 stop by a rule · 0 stopped by you · 0 open",
            "ledger 1 of 1 shipped (100%) · 0 excluded · 1 unclassified",
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
    let view = derive(&hearth(dir.path()), &unshelved(), &[], &asked("gone", &now)).unwrap();
    let painted = paint(&view);
    assert_eq!(
        painted,
        format!(
            "world / gone · {} → {now} · 1 runs\n\
             this recipe's table was not found in world\n\
             state  work · entered 1 · median — · p90 — · not reported · —\n\
             ends   0 done · 0 stop by a rule · 0 stopped by you · 1 open\n\
             ledger 0 of 0 shipped (—) · 0 excluded · 0 unclassified\n\
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
        matches!(nothing.asked, Compiled::NotOnShelf),
        "no table resolves for a name nothing gives"
    );
    let refused = derive(
        &hearth(dir.path()),
        &nothing,
        &shelf,
        &asked("nosuch", &now_rfc3339()),
    );
    assert_eq!(
        refused.unwrap_err().to_string(),
        "no recipe 'nosuch' in world: the recipes it knows are one and two"
    );
}

#[test]
fn a_recipe_that_does_not_compile_is_drawn_with_its_refusal_and_named_among_the_realms() {
    let dir = tempfile::tempdir().unwrap();
    let repo = adopter(dir.path());
    // The adapter as a library written for an older Brokkr carries it: a
    // key this Brokkr does not know.
    let adapter = repo.join(".brokkr/adapters/claude.json");
    let mut written: Value = serde_json::from_slice(&std::fs::read(&adapter).unwrap()).unwrap();
    written["binding_grant"] = json!({"network": true});
    std::fs::write(&adapter, serde_json::to_vec(&written).unwrap()).unwrap();
    let shelf = shelf(&repo).unwrap();
    let compiles = compiled(None, &repo, &shelf, "starter").unwrap();
    assert_eq!(compiles.refused, ["starter"]);
    let Compiled::Refused { reason, bundle } = &compiles.asked else {
        panic!("the starter does not compile");
    };
    let named = format!(
        "adapter 'claude' ({}) has unknown key 'binding_grant'; known keys: ",
        adapter.display()
    );
    assert!(reason.starts_with(&named), "{reason}");
    assert_eq!(bundle, ".brokkr");
    let journal = tempfile::tempdir().unwrap();
    Journal::at(journal.path());
    let now = now_rfc3339();
    let view = derive(
        &hearth(journal.path()),
        &compiles,
        &shelf,
        &asked("starter", &now),
    )
    .unwrap();
    let text = "starter doesn't compile against this Brokkr";
    assert_eq!(
        (&view.table, &view.recipes, &view.does_not_compile),
        (
            &Table::Refused {
                text: text.into(),
                reason: reason.clone(),
                bundle: ".brokkr".into(),
            },
            &vec!["starter".to_string()],
            &vec!["starter".to_string()],
        )
    );
    assert_eq!((view.runs, view.states.len()), (0, 0));
    // No run carries it, so there is no state to draw: the refusal is
    // printed as it was, with no drawing above it.
    assert_eq!(shown(&view, PIPED_WIDTH), paint(&view));
    assert_eq!(
        paint(&view),
        format!(
            "world / starter · {} → {now} · 0 runs\n{text}\n  {reason}\n  bundle .brokkr\n\
             ends   0 done · 0 stop by a rule · 0 stopped by you · 0 open\n\
             ledger 0 of 0 shipped (—) · 0 excluded · 0 unclassified\n",
            view.from
        )
    );
}

#[test]
fn a_name_resolves_to_its_first_directory_and_is_compiled_once() {
    let dir = tempfile::tempdir().unwrap();
    let repo = adopter(dir.path());
    // A bundle beside the folder giving the starter's name: no recipe, and
    // never the one the name resolves to.
    std::fs::create_dir(repo.join("again")).unwrap();
    std::fs::write(repo.join("again/bundle.json"), r#"{"name": "starter"}"#).unwrap();
    let shelf = shelf(&repo).unwrap();
    assert_eq!(shelf.len(), 2);
    let compiles = compiled(None, &repo, &shelf, "starter").unwrap();
    assert_eq!(compiles.refused, Vec::<String>::new());
    assert!(matches!(compiles.asked, Compiled::Bundle(_)));
}

#[test]
fn a_refusal_reads_without_the_bundle_words_that_wrap_it() {
    let composed = CompileError::Invalid("bundle: dialect 'speckit' declares phase 'specify' validate unsupported (composed: triage -> fast)".into());
    assert_eq!(
        reason_of(&composed),
        "dialect 'speckit' declares phase 'specify' validate unsupported (composed: triage -> fast)"
    );
    let io = CompileError::Io(std::io::Error::other("gone"));
    assert_eq!(reason_of(&io), "bundle io: gone");
}

#[test]
fn a_refused_bundle_is_named_in_the_realms_folder_however_the_two_are_spelled() {
    let dir = tempfile::tempdir().unwrap();
    let real = std::fs::canonicalize(dir.path()).unwrap().join("real");
    std::fs::create_dir_all(real.join("recipes/x")).unwrap();
    let link = dir.path().join("link");
    std::os::unix::fs::symlink(&real, &link).unwrap();
    assert_eq!(within(&real.join("recipes/x"), &link), "recipes/x");
    let elsewhere = dir.path().join("elsewhere");
    assert_eq!(within(&elsewhere, &link), elsewhere.display().to_string());
}

#[test]
fn a_realm_that_cannot_compile_any_recipe_fails_the_graph_rather_than_naming_them() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("tree");
    std::fs::create_dir_all(folder.join("recipes/one")).unwrap();
    let map = dir.path().join("realms.json");
    std::fs::write(
        &map,
        r#"{"schema":"forge.realms/v3",
            "realms":[{"name":"solo","path":"tree","default_branch":"main","dialect":"nosuch"}],
            "journal":"j.db"}"#,
    )
    .unwrap();
    let world = World::load(&map).unwrap();
    let folder = world.path_of(&world.map.realms[0]);
    let shelf = shelf(&folder).unwrap();
    let failed = compiled(Some(&world), &folder, &shelf, "one")
        .err()
        .unwrap();
    assert!(
        matches!(
            failed.downcast_ref::<LaunchError>(),
            Some(LaunchError::World(_))
        ),
        "{failed}"
    );
}

#[test]
fn the_verb_prints_the_view_as_lines_or_verbatim() {
    let dir = tempfile::tempdir().unwrap();
    Journal::at(dir.path()).run("live", &json!({"bundle_name": "gone"}), &[entered("work")]);
    for json in [false, true] {
        let args = GraphArgs {
            recipe: Some("gone".into()),
            realm: None,
            range: Range::Week,
            realms: None,
            db: Some(dir.path().join("forge.db")),
            json,
        };
        assert_eq!(graph(dir.path(), args).unwrap(), Exit::Completed.into());
    }
    // Through the command line, naming no recipe: the realm's busiest.
    let db = dir.path().join("forge.db");
    let cli = Cli::try_parse_from([
        "brokkr".as_ref(),
        "graph".as_ref(),
        "--db".as_ref(),
        db.as_os_str(),
    ]);
    let drawn = crate::run_with(
        cli.unwrap(),
        dir.path(),
        crate::ui::serve,
        None,
        None,
        crate::run_tui,
    );
    assert_eq!(drawn.unwrap(), Exit::Completed.into());
    let unknown = GraphArgs {
        recipe: Some("nosuch".into()),
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
    // The range is analytics' own parser, refusal and all (its tests).
    let refused = Cli::try_parse_from(["brokkr", "graph", "--recipe", "self", "--range", "2d"]);
    assert!(refused.is_err(), "2d is no range");
    let unnamed = Cli::try_parse_from(["brokkr", "graph"]).unwrap();
    let Cmd::CrossRun(CrossRunCmd::Graph(args)) = unnamed.command else {
        panic!("brokkr graph is the graph verb");
    };
    assert_eq!(args.recipe, None, "Brokkr picks the recipe");
}

#[test]
fn with_no_recipe_named_the_busiest_is_drawn_else_the_first_known() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("recipes/zeta")).unwrap();
    std::fs::create_dir_all(dir.path().join("recipes/alpha")).unwrap();
    let shelf = shelf(dir.path()).unwrap();
    let now = now_rfc3339();
    let chose = |hearths: &[Hearth]| chosen(hearths, &shelf, UNMAPPED, Range::Week, &now);
    // No run yet: the first the realm knows by name.
    assert_eq!(chose(&hearth(dir.path())).unwrap(), "alpha");
    let mut journal = Journal::at(dir.path());
    // Two runs of beta, one of zeta and one of alpha; beta is the busiest.
    for (run_id, recipe) in [
        ("b1", "beta"),
        ("z1", "zeta"),
        ("b2", "beta"),
        ("a1", "alpha"),
    ] {
        journal.run(run_id, &json!({"bundle_name": recipe}), &[entered("work")]);
    }
    assert_eq!(chose(&hearth(dir.path())).unwrap(), "beta");
    let unclocked = chosen(&hearth(dir.path()), &shelf, UNMAPPED, Range::Week, "noon");
    assert_eq!(
        unclocked.unwrap_err().to_string(),
        "the clock read 'noon', which is not an RFC 3339 time"
    );
    let empty = tempfile::tempdir().unwrap();
    let none = chosen(&hearth(empty.path()), &[], "brokkr", Range::Week, &now);
    let none = none.unwrap_err();
    assert_eq!(
        none.downcast_ref::<NoRecipe>(),
        Some(&NoRecipe {
            realm: "brokkr".into()
        })
    );
    assert_eq!(
        none.to_string(),
        "no recipe in brokkr: none is installed and no run in the range names one"
    );
}

// ------------------------------------------------------------- the drawing

/// A table whose rows are `states` in order, ending in `done` and beneath
/// it `stop`, ruled by `rules` of `(id, from, result, next)`; a rule with
/// no next parks.
fn table(states: &[&str], rules: &[(&str, &str, &str, Option<&str>)]) -> Machine {
    let rules: Vec<Value> = (rules.iter())
        .map(|(id, from, result, next)| {
            let mut rule = json!({"id": id, "from": from, "result": result, "reason": id});
            match next {
                Some(next) => rule["next"] = json!(next),
                None => rule["park"] = json!(true),
            }
            rule
        })
        .collect();
    let mut phases: Vec<&str> = states.to_vec();
    phases.extend(["done", "stop"]);
    Machine::from_table(&json!({
        "schema": brokkr_core::policy::TABLE_SCHEMA_V2,
        "phases": phases,
        "initial": states[0],
        "terminal": ["done", "stop"],
        "rules": rules,
    }))
    .unwrap()
}

/// The reforge loop's table: implement, review and ship, review sending
/// the work back, holding it, or stopping it, and ship retrying itself.
fn reforging() -> Machine {
    table(
        &["implement", "review", "ship"],
        &[
            ("IMPL-OK", "implement", "complete", Some("review")),
            ("REVIEW-CLEAN", "review", "clean", Some("ship")),
            ("REVIEW-REFORGE", "review", "residual", Some("implement")),
            ("REVIEW-HOLD", "review", "hold", None),
            ("REVIEW-STOP", "review", "blocked", Some("stop")),
            ("SHIP-RETRY", "ship", "retry", Some("ship")),
            ("SHIP-OK", "ship", "complete", Some("done")),
        ],
    )
}

/// One step of a run: the state it is in, its seat's result, and the rule
/// that moves it on, or parks it when there is no next.
type Step<'a> = (&'a str, &'a str, &'a str, Option<&'a str>);

/// The events of a run through `steps`: it enters each state, its seat
/// says the result, and the rule moves it.
fn walked(steps: &[Step]) -> Vec<(EventType, Value)> {
    let mut events = vec![entered(steps[0].0)];
    for (state, result, rule, next) in steps {
        events.extend(seat(state, result));
        events.push(ruled(rule, state, *next));
        events.extend(next.map(entered));
    }
    events
}

/// `recipe`'s graph over the journal in `dir`, drawn from `machine`.
fn view_of(dir: &Path, machine: &Machine, recipe: &str) -> GraphView {
    let manifest = json!({"bundle_name": recipe});
    let now = now_rfc3339();
    let view = read_hearths(&hearth(dir), |grouped| {
        brokkr_view::graph(&GraphQuery {
            hearths: grouped,
            recipe,
            realm: UNMAPPED,
            table: GraphResolution::Compiled(GraphTable {
                machine,
                manifest: &manifest,
            }),
            shelved: &[],
            does_not_compile: &[],
            range: Range::Day,
            now: &now,
        })
    });
    view.unwrap()
}

/// The reforge table's journal: one run reforged twice and shipped on a
/// retry, one reforged once and shipped.
fn reforged(dir: &Path) -> GraphView {
    let manifest = json!({"bundle_name": "reforge"});
    let reforge = ("review", "residual", "REVIEW-REFORGE", Some("implement"));
    let implement = ("implement", "complete", "IMPL-OK", Some("review"));
    let clean = ("review", "clean", "REVIEW-CLEAN", Some("ship"));
    let ship = ("ship", "complete", "SHIP-OK", Some("done"));
    let retry = ("ship", "retry", "SHIP-RETRY", Some("ship"));
    let mut journal = Journal::at(dir);
    let mut twice = walked(&[
        implement, reforge, implement, reforge, implement, clean, retry, ship,
    ]);
    twice.push((EventType::RunCompleted, json!({})));
    journal.run("twice", &manifest, &twice);
    let mut once = walked(&[implement, reforge, implement, clean, ship]);
    once.push((EventType::RunCompleted, json!({})));
    journal.run("once", &manifest, &once);
    view_of(dir, &reforging(), "reforge")
}

#[test]
fn a_linear_recipe_is_drawn_on_one_rail_with_each_rules_runs_above_its_arrow() {
    let dir = tempfile::tempdir().unwrap();
    let machine = table(
        &["plan", "build", "ship"],
        &[
            ("PLAN-OK", "plan", "complete", Some("build")),
            ("BUILD-OK", "build", "complete", Some("ship")),
            ("BUILD-FAIL", "build", "fail", Some("stop")),
            ("SHIP-OK", "ship", "complete", Some("done")),
        ],
    );
    let manifest = json!({"bundle_name": "linear"});
    let mut journal = Journal::at(dir.path());
    for run_id in ["first", "second"] {
        let mut events = walked(&[
            ("plan", "complete", "PLAN-OK", Some("build")),
            ("build", "complete", "BUILD-OK", Some("ship")),
            ("ship", "complete", "SHIP-OK", Some("done")),
        ]);
        events.push((EventType::RunCompleted, json!({})));
        journal.run(run_id, &manifest, &events);
    }
    let view = view_of(dir.path(), &machine, "linear");
    let drawing = drawn(&view, PIPED_WIDTH);
    insta::assert_snapshot!("graph_linear", drawing);
    // The drawing comes first, and today's lines follow it unchanged.
    let shown = shown(&view, PIPED_WIDTH);
    assert_eq!(shown.strip_prefix(&drawing), Some(paint(&view).as_str()));
}

/// `view` drawn at `width`, every line of it held within `width`, and
/// every link the range took either drawn by a rail or listed under it
/// with its count, never both and never neither: none is lost.
fn drawn(view: &GraphView, width: usize) -> String {
    let (drawing, drawn) = drawn_links(view, width);
    for line in drawing.lines() {
        assert!(line.chars().count() <= width, "{line}\n{drawing}");
    }
    let read = unbroken(&drawing);
    let taken = (view.links.iter().enumerate()).filter(|(_, link)| link.count > 0);
    for (at, link) in taken {
        let (from, next, count) = (&link.from, &link.next, link.count);
        let listed = match link.kind {
            RuleKind::Back => format!(" {next} ᐸ── {from} ×{count} "),
            RuleKind::Terminal => format!(" {from} ──ᐳ ⊗ {next} {count} "),
            RuleKind::Forward | RuleKind::SelfLoop => format!(" {from} ──ᐳ {next} {count} "),
        };
        assert_ne!(
            drawn.contains(&at),
            read.contains(&listed),
            "{listed}\n{drawing}"
        );
    }
    drawing
}

/// `drawing`'s lines read as one: a word continued `…` to `…` joined, and
/// every run of blanks one blank.
fn unbroken(drawing: &str) -> String {
    let mut read = String::new();
    for line in drawing.lines() {
        let line = line.trim_start();
        match (read.strip_suffix('…'), line.strip_prefix('…')) {
            (Some(kept), Some(rest)) => read = format!("{kept}{rest}"),
            _ => read = format!("{read} {line}"),
        }
    }
    read.split_whitespace().collect::<Vec<_>>().join(" ") + " "
}

/// `recipe`'s graph over one run through `steps` to completion, planted in
/// `dir` and drawn from `machine`.
fn one_run(dir: &Path, machine: &Machine, recipe: &str, steps: &[Step]) -> GraphView {
    let mut events = walked(steps);
    events.push((EventType::RunCompleted, json!({})));
    Journal::at(dir).run(recipe, &json!({"bundle_name": recipe}), &events);
    view_of(dir, machine, recipe)
}

/// The back links `view` took, each drawn as a return arc at `width`.
fn arcs_drawn(view: &GraphView, width: usize) {
    let (drawing, drawn) = drawn_links(view, width);
    for (at, link) in view.links.iter().enumerate() {
        if link.kind == RuleKind::Back && link.count > 0 {
            assert!(drawn.contains(&at), "{link:?}\n{drawing}");
        }
    }
}

#[test]
fn a_reforge_loop_taken_more_than_once_is_a_return_arc_with_its_count() {
    let dir = tempfile::tempdir().unwrap();
    let view = reforged(dir.path());
    insta::assert_snapshot!("graph_reforged", drawn(&view, PIPED_WIDTH));
}

/// The row `a` to `d` over one run that `c` sends back to `c_back` once
/// and `d` to `d_back` once, each walked forward again from there.
fn sent_back(dir: &Path, c_back: &'static str, d_back: &'static str) -> GraphView {
    let machine = table(
        &["a", "b", "c", "d"],
        &[
            ("A-OK", "a", "complete", Some("b")),
            ("B-OK", "b", "complete", Some("c")),
            ("C-OK", "c", "complete", Some("d")),
            ("C-BACK", "c", "residual", Some(c_back)),
            ("D-BACK", "d", "residual", Some(d_back)),
            ("D-OK", "d", "complete", Some("done")),
            ("D-STOP", "d", "blocked", Some("stop")),
        ],
    );
    let on: [Step; 3] = [
        ("a", "complete", "A-OK", Some("b")),
        ("b", "complete", "B-OK", Some("c")),
        ("c", "complete", "C-OK", Some("d")),
    ];
    let back = |at: &str| (on.iter()).position(|step| step.0 == at).unwrap();
    let mut steps = on[..2].to_vec();
    steps.push(("c", "residual", "C-BACK", Some(c_back)));
    steps.extend(&on[back(c_back)..]);
    steps.push(("d", "residual", "D-BACK", Some(d_back)));
    steps.extend(&on[back(d_back)..]);
    steps.push(("d", "complete", "D-OK", Some("done")));
    one_run(dir, &machine, "sent", &steps)
}

#[test]
fn a_return_arc_nested_in_another_rides_a_row_of_its_own_with_its_count() {
    let dir = tempfile::tempdir().unwrap();
    let view = sent_back(dir.path(), "b", "a");
    // b ᐸ c rides the arc row, and a ᐸ d, whose span holds it, the row
    // under it, rising past it to its states: both arcs, both counts.
    let drawing = drawn(&view, PIPED_WIDTH);
    insta::assert_snapshot!("graph_nested", drawing);
    arcs_drawn(&view, PIPED_WIDTH);
    assert_eq!(drawing.matches(" ×1 ").count(), 2, "{drawing}");
}

/// The delivery recipe's shape: implement, verify, review and ship, verify
/// sending the work back three times and review once, both to implement.
fn delivered(dir: &Path) -> GraphView {
    let states = ["implement", "verify", "review", "ship"];
    let machine = table(
        &states,
        &[
            ("IMPL-OK", "implement", "complete", Some("verify")),
            ("VERIFY-OK", "verify", "pass", Some("review")),
            ("VERIFY-FAIL", "verify", "fail", Some("implement")),
            ("REVIEW-CLEAN", "review", "clean", Some("ship")),
            ("REVIEW-REFORGE", "review", "residual", Some("implement")),
            ("REVIEW-STOP", "review", "blocked", Some("stop")),
            ("SHIP-OK", "ship", "complete", Some("done")),
        ],
    );
    let implement = ("implement", "complete", "IMPL-OK", Some("verify"));
    let pass = ("verify", "pass", "VERIFY-OK", Some("review"));
    let fail = ("verify", "fail", "VERIFY-FAIL", Some("implement"));
    let reforge = ("review", "residual", "REVIEW-REFORGE", Some("implement"));
    let clean = ("review", "clean", "REVIEW-CLEAN", Some("ship"));
    let ship = ("ship", "complete", "SHIP-OK", Some("done"));
    let manifest = json!({"bundle_name": "delivery"});
    let mut journal = Journal::at(dir);
    let runs = [
        vec![implement, fail, implement, pass, reforge, implement, pass],
        vec![implement, fail, implement, fail, implement, pass],
    ];
    for (index, mut steps) in runs.into_iter().enumerate() {
        steps.extend([clean, ship]);
        let mut events = walked(&steps);
        events.push((EventType::RunCompleted, json!({})));
        journal.run(&format!("run-{index}"), &manifest, &events);
    }
    view_of(dir, &machine, "delivery")
}

#[test]
fn the_delivery_shape_draws_its_two_reforges_as_nested_return_arcs() {
    let dir = tempfile::tempdir().unwrap();
    let view = delivered(dir.path());
    let drawing = drawn(&view, PIPED_WIDTH);
    insta::assert_snapshot!("graph_delivery", drawing);
    arcs_drawn(&view, PIPED_WIDTH);
    // verify ᐸ implement ×3 rides the arc row, review's ×1 the row under.
    let rows: Vec<&str> = drawing.lines().filter(|line| line.contains('ᐸ')).collect();
    assert_eq!(rows.len(), 2, "{drawing}");
    assert!(
        rows[0].contains(" ×3 ") && rows[1].contains(" ×1 "),
        "{drawing}"
    );
}

#[test]
fn a_self_loops_count_and_a_forward_links_count_never_share_a_column() {
    let dir = tempfile::tempdir().unwrap();
    let machine = table(
        &["a", "b"],
        &[
            ("A-AGAIN", "a", "retry", Some("a")),
            ("A-OK", "a", "complete", Some("b")),
            ("B-OK", "b", "complete", Some("done")),
            ("B-STOP", "b", "blocked", Some("stop")),
        ],
    );
    let mut events = walked(&[
        ("a", "retry", "A-AGAIN", Some("a")),
        ("a", "complete", "A-OK", Some("b")),
        ("b", "complete", "B-OK", Some("done")),
    ]);
    events.push((EventType::RunCompleted, json!({})));
    Journal::at(dir.path()).run("loud", &json!({"bundle_name": "loud"}), &events);
    let mut view = view_of(dir.path(), &machine, "loud");
    // Review 2's case: entered `1 · 0m`, ten thousand runs to itself and
    // ten thousand on: the two counts would share a column on one row.
    let leaving = |view: &mut GraphView, count| {
        for link in view.links.iter_mut().filter(|link| link.from == "a") {
            link.count = count;
        }
    };
    leaving(&mut view, 10_000);
    let drawing = drawn(&view, PIPED_WIDTH);
    insta::assert_snapshot!("graph_counts_apart", drawing);
    let words: Vec<&str> = drawing.split_whitespace().collect();
    assert!(
        words.contains(&"↻10000") && words.contains(&"10000"),
        "{drawing}"
    );
    // Where the count is wider than the room before its arrow, the link
    // is listed whole.
    leaving(&mut view, 1_000_000_000_000);
    let drawing = drawn(&view, 24);
    assert!(drawing.contains("  a ──ᐳ b  1000000000000\n"), "{drawing}");
}

#[test]
fn a_parked_run_is_live_on_its_state_and_a_stopped_one_takes_the_road_down_to_stop() {
    let dir = tempfile::tempdir().unwrap();
    let manifest = json!({"bundle_name": "reforge"});
    let implement = ("implement", "complete", "IMPL-OK", Some("review"));
    let mut journal = Journal::at(dir.path());
    let held = walked(&[implement, ("review", "hold", "REVIEW-HOLD", None)]);
    journal.run("held", &manifest, &held);
    let blocked = ("review", "blocked", "REVIEW-STOP", Some("stop"));
    let mut stopped = walked(&[implement, blocked]);
    stopped.push((EventType::RunStopped, json!({})));
    journal.run("stopped", &manifest, &stopped);
    let view = view_of(dir.path(), &reforging(), "reforge");
    let drawing = drawn(&view, PIPED_WIDTH);
    insta::assert_snapshot!("graph_parked", drawing);
    assert!(drawing.contains("╰─ 1 ─ᐳ ⊗ stop"), "{drawing}");
    // At 12 columns the road has no room for its head and a letter past
    // its state: the rule is listed whole instead (`drawn` checks it).
    let narrow = drawn(&view, 12);
    assert!(!narrow.contains("╰─ 1 ─ᐳ"), "{narrow}");
}

#[test]
fn two_return_arcs_that_cross_ride_two_rows_one_passing_through_the_other() {
    let dir = tempfile::tempdir().unwrap();
    // a ᐸ c and b ᐸ d: each holds one end of the other.
    let view = sent_back(dir.path(), "a", "b");
    let drawing = drawn(&view, PIPED_WIDTH);
    insta::assert_snapshot!("graph_crossed", drawing);
    arcs_drawn(&view, PIPED_WIDTH);
    assert!(drawing.contains('┼'), "{drawing}");
}

#[test]
fn a_range_with_no_runs_draws_the_table_with_no_traffic() {
    let dir = tempfile::tempdir().unwrap();
    Journal::at(dir.path());
    let view = view_of(dir.path(), &reforging(), "reforge");
    assert_eq!(view.runs, 0);
    insta::assert_snapshot!("graph_no_runs", drawn(&view, PIPED_WIDTH));
}

#[test]
fn at_80_columns_a_wide_recipe_folds_whole_states_and_draws_its_arcs_across_the_fold() {
    let dir = tempfile::tempdir().unwrap();
    let states = [
        "specification-intake",
        "implementation-planning",
        "implementation",
        "verification-and-review",
        "release-shipping",
    ];
    let rule = |id, at: usize, result, next: usize| (id, states[at], result, Some(states[next]));
    let machine = table(
        &states,
        &[
            rule("INTAKE-COMPLETE", 0, "complete", 1),
            rule("PLANNING-COMPLETE", 1, "complete", 2),
            rule("IMPLEMENTATION-COMPLETE", 2, "complete", 3),
            rule("REVIEW-CLEAN", 3, "clean", 4),
            rule("REVIEW-RESIDUAL", 3, "residual", 2),
            rule("REVIEW-RESPECIFY", 3, "unclear", 0),
            ("REVIEW-BLOCKED", states[3], "blocked", Some("stop")),
            ("PLANNING-ABANDONED", states[1], "abandoned", Some("stop")),
            ("RELEASE-SHIPPED", states[4], "complete", Some("done")),
            ("RELEASE-WITHDRAWN", states[4], "withdrawn", Some("stop")),
        ],
    );
    let step =
        |at: usize, result, rule, next: usize| (states[at], result, rule, Some(states[next]));
    let steps = [
        step(0, "complete", "INTAKE-COMPLETE", 1),
        step(1, "complete", "PLANNING-COMPLETE", 2),
        step(2, "complete", "IMPLEMENTATION-COMPLETE", 3),
        step(3, "residual", "REVIEW-RESIDUAL", 2),
        step(2, "complete", "IMPLEMENTATION-COMPLETE", 3),
        step(3, "unclear", "REVIEW-RESPECIFY", 0),
        step(0, "complete", "INTAKE-COMPLETE", 1),
        step(1, "complete", "PLANNING-COMPLETE", 2),
        step(2, "complete", "IMPLEMENTATION-COMPLETE", 3),
        step(3, "clean", "REVIEW-CLEAN", 4),
        (states[4], "complete", "RELEASE-SHIPPED", Some("done")),
    ];
    let view = one_run(dir.path(), &machine, "wide", &steps);
    let drawing = drawn(&view, 80);
    insta::assert_snapshot!("graph_wide_80", drawing);
    // No label is cut: every state's whole name is drawn, and nothing is
    // clamped.
    for state in states {
        assert!(drawing.contains(state), "{state}\n{drawing}");
    }
    assert!(!drawing.contains('…'), "{drawing}");
    // Both back links leave the second rail for the first: each is drawn
    // as its landing half out to `›` and its departing half in from `‹`.
    arcs_drawn(&view, 80);
    let lines = || drawing.lines().filter(|line| !line.contains('⏺'));
    let landing = lines().filter(|line| line.contains('ᐸ') && line.ends_with('›'));
    let leaving = lines().filter(|line| line.starts_with('‹'));
    assert_eq!((landing.count(), leaving.count()), (2, 2), "{drawing}");
}

#[test]
fn at_80_columns_a_long_link_across_the_fold_wraps_and_a_long_id_continues() {
    let dir = tempfile::tempdir().unwrap();
    let (first, second) = (
        "the-specification-intake-and-its-triage",
        "the-implementation-planning-and-staffing",
    );
    // The longest id a table admits: 64 columns.
    let long = "A-RULE-WHOSE-ID-RUNS-AS-WIDE-AS-THE-TABLE-ADMITS-AND-STILL-HOLDS";
    let machine = table(
        &[first, second],
        &[
            ("INTAKE-OK", first, "complete", Some(second)),
            ("PLANNING-OK", second, "complete", Some("done")),
            (long, second, "abandoned", Some("stop")),
        ],
    );
    let mut events = walked(&[
        (first, "complete", "INTAKE-OK", Some(second)),
        (second, "complete", "PLANNING-OK", Some("done")),
    ]);
    events.push((EventType::RunCompleted, json!({})));
    Journal::at(dir.path()).run("long", &json!({"bundle_name": "long"}), &events);
    let view = view_of(dir.path(), &machine, "long");
    let drawing = drawn(&view, 80);
    insta::assert_snapshot!("graph_long_80", drawing);
    // The link the fold split runs out to `›` and in from `‹`.
    assert!(drawing.contains("‹──────────────────ᐳ⏺"), "{drawing}");
    // At the narrowest width the id is wider than a line: it is continued,
    // `…` to `…`, and reads whole with its breaks taken out.
    let narrowest = drawn(&view, 40);
    assert!(!narrowest.contains(long), "{narrowest}");
    assert!(
        narrowest.replace("…\n   …", "").contains(long),
        "{narrowest}"
    );
}

#[test]
fn the_terminals_width_is_the_drawings_and_a_pipe_draws_at_the_stated_width() {
    assert_eq!(drawing_width(Some(Ok((80, 24)))), 80);
    let unread = Some(Err(std::io::Error::other("no size")));
    assert_eq!((drawing_width(unread), drawing_width(None)), (100, 100));
    // A known positive width is fitted, however narrow; a pty with no size
    // set answers 0 columns, and is drawn as a pipe is.
    let narrow = [39, 12, 1].map(|columns| drawing_width(Some(Ok((columns, 24)))));
    assert_eq!(
        (narrow, drawing_width(Some(Ok((0, 24))))),
        ([39, 12, 1], 100)
    );
}

#[test]
fn a_narrow_terminal_is_fitted_one_state_to_a_rail_with_every_arc_still_drawn() {
    let dir = tempfile::tempdir().unwrap();
    let (delivery, reforge) = (dir.path().join("delivery"), dir.path().join("reforge"));
    let views = [delivered(&delivery), reforged(&reforge)];
    for width in [39, 24, 12] {
        for view in &views {
            // `drawn` holds every line, the legend's too, within the width.
            drawn(view, width);
            arcs_drawn(view, width);
        }
    }
    insta::assert_snapshot!("graph_delivery_24", drawn(&views[0], 24));
    insta::assert_snapshot!("graph_delivery_12", drawn(&views[0], 12));
    // Under the twelve columns a node and a clamped label need, the states
    // are written as lines, held to the width like the lines under them.
    Journal::at(&dir.path().join("none"));
    let none = view_of(&dir.path().join("none"), &reforging(), "reforge");
    insta::assert_snapshot!("graph_no_runs_8", drawn(&none, 8));
    // Down to one column, the indents yield and no line passes the width;
    // from the three a continued piece needs, every taken link still reads
    // whole.
    for width in 1..=12 {
        for view in views.iter().chain([&none]) {
            let drawing = recipe_graph(view, width);
            let widest = drawing.lines().map(|line| line.chars().count()).max();
            assert!(widest <= Some(width), "{width}\n{drawing}");
            if width >= 3 {
                drawn(view, width);
            }
        }
    }
    insta::assert_snapshot!("graph_reforged_3", drawn(&views[1], 3));
}

#[test]
fn a_state_named_wider_than_the_terminal_is_clamped_on_its_rail() {
    let dir = tempfile::tempdir().unwrap();
    let wide = "a-state-whose-name-runs-wider-than-the-terminal";
    let machine = table(
        &[wide, "ship"],
        &[
            ("WIDE-OK", wide, "complete", Some("ship")),
            ("WIDE-AGAIN", "ship", "residual", Some(wide)),
            ("SHIP-OK", "ship", "complete", Some("done")),
            ("SHIP-STOP", "ship", "blocked", Some("stop")),
        ],
    );
    let steps = [
        (wide, "complete", "WIDE-OK", Some("ship")),
        ("ship", "residual", "WIDE-AGAIN", Some(wide)),
        (wide, "complete", "WIDE-OK", Some("ship")),
        ("ship", "complete", "SHIP-OK", Some("done")),
    ];
    let view = one_run(dir.path(), &machine, "named", &steps);
    let drawing = drawn(&view, 30);
    insta::assert_snapshot!("graph_name_wider_30", drawing);
    arcs_drawn(&view, 30);
    assert!(drawing.contains("a-state-whose-name-runs…"), "{drawing}");
}

/// `json` with the stamps of this run's clock, and the stays it measured
/// to the millisecond, put in words: the rest is the wire's own bytes.
fn steadied(view: &GraphView, json: &str) -> String {
    let mut json = json.to_string();
    let stamps = (view.rules.iter()).flat_map(|rule| rule.latest.iter().map(|run| &run.at));
    for stamp in stamps.chain([&view.from, &view.to]) {
        json = json.replace(stamp.as_str(), "<stamp>");
    }
    let lines = json.lines().map(|line| {
        let millis = line.trim_start().strip_prefix("\"millis\": ");
        match millis.filter(|millis| !millis.starts_with("null")) {
            Some(_) => format!("{}\"millis\": <millis>,", &line[..line.find('"').unwrap()]),
            None => line.to_string(),
        }
    });
    lines.collect::<Vec<String>>().join("\n")
}

#[test]
fn the_json_is_the_view_model_verbatim_as_before_the_drawing() {
    let dir = tempfile::tempdir().unwrap();
    let view = reforged(dir.path());
    let json = serde_json::to_string_pretty(&view).unwrap();
    insta::assert_snapshot!("graph_json", steadied(&view, &json));
}
