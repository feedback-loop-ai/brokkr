//! The spawn-time re-walk of a site that declares no hands (#823, the
//! operator's option a): such a site is unboxed, so the script its exec
//! dispatch runs is held to the bytes the compile pinned, in the leaf
//! layer and in an ancestor's, exactly as an unboxed site with hands is
//! (decision 0048). Any other hands-less argv spawns as it always has.

use super::*;
use crate::engine::capability_tests::provision_tests::survey_tests::installed;

/// A layer on disk whose pinned identity the bundle carries: the
/// bundle's own files map for the leaf, computed by the same digest.
pub(super) fn pinned_layer(dir: &Path) -> (PathBuf, Bundle) {
    let layer = dir.join("layer");
    std::fs::create_dir_all(layer.join("scripts")).unwrap();
    std::fs::write(layer.join("scripts/gate.sh"), "#!/bin/sh\ntrue\n").unwrap();
    std::fs::write(layer.join("scripts/lib.sh"), "helper\n").unwrap();
    std::fs::write(layer.join("bundle.json"), "{}").unwrap();
    let mut bundle = bundle(dir, single_body(vec!["must-not-run".into()]));
    bundle.dir = layer.clone();
    bundle.roots = vec![layer.clone()];
    let mut files = Map::new();
    for rel in ["bundle.json", "scripts/gate.sh", "scripts/lib.sh"] {
        files.insert(
            rel.to_string(),
            Value::String(sha256_bytes(&std::fs::read(layer.join(rel)).unwrap())),
        );
    }
    bundle.manifest["files"] = Value::Object(files);
    (layer, bundle)
}

/// The driver-protocol body of a seat ending in `outcome`, as a script.
fn speaking(outcome: AttemptOutcome) -> String {
    driver_command("effect", "attempt", outcome).swap_remove(2)
}

/// An engine over [`pinned_layer`] whose `scripts/gate.sh` is re-pinned
/// as a seat that completes, the layer, and a hands-less exec dispatch
/// of that script through an engine stand-in that runs what follows its
/// `driver exec --`. The network probe is pinned passing, so a prefix
/// composed for a site with no hands would show on every host.
fn hands_less_gate() -> (tempfile::TempDir, Engine, PathBuf, Vec<String>) {
    let (dir, mut engine) = super::super::tests::engine(single_body(vec!["driver".into()]));
    let (layer, mut pinned) = pinned_layer(dir.path());
    let complete = speaking(AttemptOutcome::Succeeded {
        result: json!({"result":"complete"}),
    });
    std::fs::write(layer.join("scripts/gate.sh"), &complete).unwrap();
    pinned.manifest["files"]["scripts/gate.sh"] = json!(sha256_bytes(complete.as_bytes()));
    engine.bundle = pinned;
    engine.network_prefix = Some(true);
    let source = dir.path().join("stand-in.sh");
    std::fs::write(&source, "#!/bin/sh\nshift 3\nexec \"$@\"\n").unwrap();
    let stand_in = installed(dir.path(), "brokkr", &source);
    let script = layer.join("scripts/gate.sh");
    let command = vec![
        stand_in.display().to_string(),
        "driver".into(),
        "exec".into(),
        "--".into(),
        "sh".into(),
        script.display().to_string(),
    ];
    (dir, engine, layer, command)
}

/// `command` composed for a site with no hands.
fn hands_less(engine: &mut Engine, command: &[String]) -> SiteSpawn {
    engine.compose("attempt", true, command.to_vec(), None, None, "result.json")
}

/// What a hands-less gate's `command` composed to before #823 — the
/// author's one segment, the inherited environment, no prefix — beside
/// the re-walk it now marks.
fn gate(command: &[String], rewalk: Option<PathBuf>) -> SiteSpawn {
    let authored = SiteSpawn::of(vec![Segment::new(Origin::Authored, command)]);
    let class = Some(SeatClass::Gate);
    SiteSpawn {
        rewalk,
        class,
        ..authored
    }
}

/// `command` composed for a site with no hands, then run.
fn compose_and_run(engine: &mut Engine, command: &[String]) -> (SiteSpawn, DriverRun) {
    let spawn = hands_less(engine, command);
    let deadline = std::time::Duration::from_secs(5);
    let run = engine
        .run_driver(
            "effect",
            "attempt",
            "work",
            &spawn,
            json!({}),
            deadline,
            None,
            None,
        )
        .unwrap();
    (spawn, run)
}

/// The script edited after the compile: the spawn is refused with 0048's
/// text before the edited bytes run, and the refusal is journaled as the
/// attempt's failure, as every walk refusal is.
fn refused_after_an_edit(engine: &mut Engine, layer: &Path, command: &[String], refusal: &str) {
    let ran = speaking(AttemptOutcome::Failed {
        error: "the edited script ran".into(),
    });
    std::fs::write(layer.join("scripts/gate.sh"), ran).unwrap();
    let (_, run) = compose_and_run(engine, command);
    let error = match &run {
        DriverRun::SpawnFailed(error) => error,
        DriverRun::Ran(report) => panic!("the edited script ran: {:?}", report.outcome),
    };
    let failure = format!("driver did not spawn: {refusal}");
    assert_eq!(error, &failure);
    engine
        .conclude_single("effect", "attempt", run, &Selection::new(), None)
        .unwrap();
    let events = engine.store.load(&engine.run_id).unwrap();
    let failed = events.last().unwrap();
    assert_eq!(failed.event_type, EventType::EffectFailed);
    assert_eq!(failed.payload["error"], json!(failure));
}

#[test]
fn a_hands_less_exec_site_in_the_leaf_is_refused_when_its_script_moved() {
    let (_dir, mut engine, layer, command) = hands_less_gate();
    let refusal = "unboxed exec dispatch refused: layer 'test' moved since the compile \
                   (changed: scripts/gate.sh); the bytes an unboxed gate runs must be the \
                   bytes its digest names (decision 0046 ruling 4)";
    refused_after_an_edit(&mut engine, &layer, &command, refusal);
}

#[test]
fn a_hands_less_exec_site_in_an_ancestor_is_refused_when_its_script_moved() {
    let (dir, mut engine, layer, command) = hands_less_gate();
    // The pinned layer is the ancestor `base`; the leaf is another layer.
    let leaf = dir.path().join("child");
    engine.bundle.chain.push(crate::bundle::compose::Ancestor {
        name: "base".into(),
        reached_as: None,
        dir: layer.clone(),
        digest: engine.bundle.manifest_digest(),
        files: engine.bundle.manifest["files"].as_object().unwrap().clone(),
    });
    engine.bundle.manifest["files"] = json!({});
    engine.bundle.dir = leaf.clone();
    engine.bundle.roots = vec![leaf, layer.clone()];
    let refusal = "unboxed exec dispatch refused: layer 'base' moved since the compile \
                   (changed: scripts/gate.sh); the bytes an unboxed gate runs must be the \
                   bytes its digest names (decision 0046 ruling 4)";
    refused_after_an_edit(&mut engine, &layer, &command, refusal);
}

/// Unchanged, the same site spawns and completes, composed exactly as
/// it was before the re-walk was marked: the authored argv, the
/// inherited environment, no prefix — and now the script's directory.
#[test]
fn a_hands_less_exec_site_whose_script_is_unchanged_spawns_as_composed() {
    let (_dir, mut engine, layer, command) = hands_less_gate();
    let (spawn, run) = compose_and_run(&mut engine, &command);
    assert_eq!(spawn, gate(&command, Some(layer.join("scripts"))));
    assert_eq!(spawn.env, SpawnEnv::Inherit);
    let DriverRun::Ran(report) = run else {
        panic!("an unchanged script was refused")
    };
    assert!(report.accepted);
    let result = json!({"result":"complete"});
    assert_eq!(report.outcome, AttemptOutcome::Succeeded { result });
}

/// A hands-less site that is no exec dispatch — a model seat, a plain
/// command — marks no re-walk, even where a later token names a script
/// under a root, and spawns over a script that has since moved.
#[test]
fn a_hands_less_site_that_is_no_exec_dispatch_marks_no_rewalk() {
    let (_dir, mut engine, layer, exec) = hands_less_gate();
    let script = layer.join("scripts/gate.sh");
    let model = [&exec[..2], &["claude".into(), "--".into()], &exec[5..]].concat();
    let complete = speaking(AttemptOutcome::Succeeded {
        result: json!({"result":"complete"}),
    });
    // `sh -c <body> sh <script>`: the script is `$1`, never run.
    let plain = [&["sh".into(), "-c".into(), complete], &exec[4..]].concat();
    std::fs::write(&script, "moved\n").unwrap();
    for command in [model, plain.clone()] {
        assert_eq!(hands_less(&mut engine, &command), gate(&command, None));
    }
    let (_, run) = compose_and_run(&mut engine, &plain);
    let DriverRun::Ran(report) = run else {
        panic!("a plain command was refused")
    };
    let result = json!({"result":"complete"});
    assert_eq!(report.outcome, AttemptOutcome::Succeeded { result });
}
