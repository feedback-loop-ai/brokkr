//! The spawn-time re-walk of a site that declares no hands (#823, the
//! operator's option a): such a site is unboxed, so the script its exec
//! dispatch runs is held to the bytes the compile pinned, in the leaf
//! layer and in an ancestor's, exactly as an unboxed site with hands is
//! (decision 0048). Any other hands-less argv spawns as it always has.
//! A boxed exec site, hands under `namespace`, is held to the same walk
//! before its box is built (#832, operator ruling "accept all").

use super::*;
use crate::argv::Executable;
use crate::engine::capability_tests::provision_tests::survey_tests::installed;
use brokkr_protocol::hands::SANDBOX_BUNDLE;

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

/// `command` composed for a site with `hands`, or none, then run.
fn compose_and_run(
    engine: &mut Engine,
    command: &[String],
    hands: Option<&HandsSpec>,
) -> (SiteSpawn, DriverRun) {
    let spawn = engine.compose(
        "attempt",
        true,
        command.to_vec(),
        hands,
        None,
        "result.json",
    );
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

/// The script edited after the compile: the spawn of a site with `hands`,
/// or none, is refused with 0048's text before the edited bytes run, and
/// the refusal is journaled as the attempt's failure, as every walk
/// refusal is.
fn refused_after_an_edit(
    engine: &mut Engine,
    (layer, hands): (&Path, Option<&HandsSpec>),
    command: &[String],
    refusal: &str,
) {
    let ran = speaking(AttemptOutcome::Failed {
        error: "the edited script ran".into(),
    });
    std::fs::write(layer.join("scripts/gate.sh"), ran).unwrap();
    let (_, run) = compose_and_run(engine, command, hands);
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

/// Decision 0048's refusal of the leaf layer's edited `scripts/gate.sh`.
const LEAF_MOVED: &str = "unboxed exec dispatch refused: layer 'test' moved since the compile \
                          (changed: scripts/gate.sh); the bytes an unboxed gate runs must be \
                          the bytes its digest names (decision 0046 ruling 4)";

#[test]
fn a_hands_less_exec_site_in_the_leaf_is_refused_when_its_script_moved() {
    let (_dir, mut engine, layer, command) = hands_less_gate();
    refused_after_an_edit(&mut engine, (&layer, None), &command, LEAF_MOVED);
}

/// The escape is optional behind the verb (decision 0009), so a site
/// spelled `<engine> driver exec <script>` is the same exec dispatch: its
/// script is the tail's first token, not the argv's fifth, and it is
/// re-walked like the escaped spelling.
#[test]
fn a_hands_less_exec_site_without_the_escape_is_refused_when_its_script_moved() {
    let (dir, mut engine, layer, escaped) = hands_less_gate();
    let source = dir.path().join("unescaped.sh");
    std::fs::write(&source, "#!/bin/sh\nshift 2\nexec sh \"$@\"\n").unwrap();
    let stand_in = installed(dir.path(), "brokkr-unescaped", &source);
    let script = escaped[5].clone();
    let command = vec![
        stand_in.display().to_string(),
        "driver".into(),
        "exec".into(),
        script,
    ];
    refused_after_an_edit(&mut engine, (&layer, None), &command, LEAF_MOVED);
}

/// The pinned `layer` made the ancestor `base` of a composed bundle whose
/// leaf, `child`, is another layer.
fn under_an_ancestor(engine: &mut Engine, dir: &Path, layer: &Path) {
    let leaf = dir.join("child");
    engine.bundle.chain.push(crate::bundle::compose::Ancestor {
        name: "base".into(),
        reached_as: None,
        dir: layer.to_path_buf(),
        digest: engine.bundle.manifest_digest(),
        files: engine.bundle.manifest["files"].as_object().unwrap().clone(),
    });
    engine.bundle.manifest["files"] = json!({});
    engine.bundle.dir = leaf.clone();
    engine.bundle.roots = vec![leaf, layer.to_path_buf()];
}

#[test]
fn a_hands_less_exec_site_in_an_ancestor_is_refused_when_its_script_moved() {
    let (dir, mut engine, layer, command) = hands_less_gate();
    under_an_ancestor(&mut engine, dir.path(), &layer);
    let refusal = "unboxed exec dispatch refused: layer 'base' moved since the compile \
                   (changed: scripts/gate.sh); the bytes an unboxed gate runs must be the \
                   bytes its digest names (decision 0046 ruling 4)";
    refused_after_an_edit(&mut engine, (&layer, None), &command, refusal);
}

/// Unchanged, the same site spawns and completes, composed exactly as
/// it was before the re-walk was marked: the authored argv, the
/// inherited environment, no prefix — and now the script's directory.
#[test]
fn a_hands_less_exec_site_whose_script_is_unchanged_spawns_as_composed() {
    let (_dir, mut engine, layer, command) = hands_less_gate();
    let (spawn, run) = compose_and_run(&mut engine, &command, None);
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
    let (_, run) = compose_and_run(&mut engine, &plain, None);
    let DriverRun::Ran(report) = run else {
        panic!("a plain command was refused")
    };
    let result = json!({"result":"complete"});
    assert_eq!(report.outcome, AttemptOutcome::Succeeded { result });
}

/// A stand-in for `<engine> hands exec … --bundle-root <root> -- <command>`
/// that builds no namespace: it runs `<command>` with the box's bundle path
/// read back as `<root>`, the host directory the box binds read-only, so a
/// boxed spawn runs the host's bytes as the box would show them, on every
/// host and inside a box.
fn box_stand_in() -> String {
    let bundle = SANDBOX_BUNDLE;
    [
        "#!/bin/sh",
        "while [ \"$1\" != -- ]; do [ \"$1\" = --bundle-root ] && root=$2; shift; done",
        "shift",
        &format!(
            "for part; do shift; case $part in {bundle}/*) part=$root${{part#{bundle}}};; esac"
        ),
        "set -- \"$@\" \"$part\"; done",
        "exec \"$@\"",
        "",
    ]
    .join("\n")
}

/// [`hands_less_gate`] under `namespace` with hands: the engine's own
/// executable is [`box_stand_in`], and the probe is pinned failing, as the
/// workstation's is, so every host composes the same box.
fn boxed_gate() -> (tempfile::TempDir, Engine, PathBuf, Vec<String>) {
    let (dir, mut engine, layer, command) = hands_less_gate();
    let source = dir.path().join("box.sh");
    std::fs::write(&source, box_stand_in()).unwrap();
    let engine_box = installed(dir.path(), "brokkr-box", &source);
    engine.bundle.executable = Executable(Some(engine_box));
    engine.boundary = Boundary::Namespace;
    engine.network_prefix = Some(false);
    (dir, engine, layer, command)
}

/// The walk refusal of a boxed exec dispatch whose `layer` moved by `key`.
fn boxed_moved(layer: &str, key: &str) -> String {
    format!(
        "boxed exec dispatch refused: layer '{layer}' moved since the compile ({key}); the \
         bytes a gate runs must be the bytes its digest names (decision 0046 ruling 4, \
         decision 0048, #832)"
    )
}

#[test]
fn a_boxed_exec_site_in_the_leaf_is_refused_before_its_box_when_its_script_moved() {
    let (_dir, mut engine, layer, command) = boxed_gate();
    let refusal = boxed_moved("test", "changed: scripts/gate.sh");
    let hands = HandsSpec::default();
    refused_after_an_edit(&mut engine, (&layer, Some(&hands)), &command, &refusal);
}

/// The word is the spawn's: a site with hands under `harness`, led on Linux
/// by the engine's network prefix, is unboxed and keeps 0048's own text.
#[test]
fn an_unboxed_exec_site_behind_its_network_prefix_keeps_the_unboxed_text() {
    let (_dir, mut engine, layer, command) = hands_less_gate();
    (engine.boundary, engine.bundle.boundary) = (Boundary::Harness, Boundary::Harness);
    let hands = HandsSpec::default();
    refused_after_an_edit(&mut engine, (&layer, Some(&hands)), &command, LEAF_MOVED);
}

#[test]
fn a_boxed_exec_site_in_an_ancestor_is_refused_before_its_box_when_its_script_moved() {
    let (dir, mut engine, layer, command) = boxed_gate();
    under_an_ancestor(&mut engine, dir.path(), &layer);
    let refusal = boxed_moved("base", "changed: scripts/gate.sh");
    let hands = HandsSpec::default();
    refused_after_an_edit(&mut engine, (&layer, Some(&hands)), &command, &refusal);
}

/// A file removed from, or added to, the walked directory refuses the
/// boxed spawn too, before the box that would have run the unchanged
/// script starts.
#[test]
fn a_boxed_exec_site_is_refused_when_a_file_is_removed_or_added_beside_its_script() {
    let (_dir, mut engine, layer, command) = boxed_gate();
    let hands = HandsSpec::default();
    let scripts = layer.join("scripts");
    let refused = |engine: &mut Engine| match compose_and_run(engine, &command, Some(&hands)) {
        (_, DriverRun::SpawnFailed(error)) => error,
        (_, DriverRun::Ran(report)) => panic!("the box ran: {:?}", report.outcome),
    };
    std::fs::remove_file(scripts.join("lib.sh")).unwrap();
    let missing = boxed_moved("test", "missing: scripts/lib.sh");
    assert_eq!(
        refused(&mut engine),
        format!("driver did not spawn: {missing}")
    );
    std::fs::write(scripts.join("lib.sh"), "helper\n").unwrap();
    std::fs::write(scripts.join("extra.sh"), "new\n").unwrap();
    let added = boxed_moved("test", "added: scripts/extra.sh");
    assert_eq!(
        refused(&mut engine),
        format!("driver did not spawn: {added}")
    );
}

/// Untouched, the walk passes and the site spawns into its box exactly as
/// before #832: the box's argv token for token, the bundle's script at the
/// box's path, the engine's environment — and now the host's script
/// directory to re-walk. The box runs the script and the seat completes.
#[test]
fn a_boxed_exec_site_whose_script_is_unchanged_spawns_into_its_box_as_composed() {
    let (_dir, mut engine, layer, command) = boxed_gate();
    let hands = HandsSpec::default();
    let (spawn, run) = compose_and_run(&mut engine, &command, Some(&hands));
    let artifacts = artifact_root(&engine.run_local());
    let reach = engine.reach.sealed(&hands);
    let handed = (&engine.bundle.executable, artifacts.as_path(), reach);
    let workdir = engine.workdir();
    let mapped = hands_command(
        command.clone(),
        Some(&hands),
        &workdir,
        handed,
        &engine.bundle.roots,
    );
    let mapped = mapped.unwrap();
    let in_box = format!("{SANDBOX_BUNDLE}/scripts/gate.sh");
    let tail = [&command[..5], std::slice::from_ref(&in_box)].concat();
    assert_eq!(mapped[1..3], ["hands", "exec"]);
    assert_eq!(mapped[mapped.len() - 6..], tail[..]);
    let authored = [Segment::new(Origin::Authored, &command)];
    let expected = SiteSpawn {
        rewalk: Some(layer.join("scripts")),
        class: Some(SeatClass::Gate),
        ..SiteSpawn::of(behind(mapped, &authored))
    };
    assert_eq!(spawn, expected);
    assert_eq!(spawn.env, SpawnEnv::Inherit);
    let DriverRun::Ran(report) = run else {
        panic!("an unchanged boxed script was refused")
    };
    assert!(report.accepted);
    let result = json!({"result":"complete"});
    assert_eq!(report.outcome, AttemptOutcome::Succeeded { result });
}
