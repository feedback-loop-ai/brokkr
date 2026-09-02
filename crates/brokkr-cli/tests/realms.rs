//! The map is the world, chosen at invocation (decision 0023, phase 1),
//! proved through the shipped binary: what an operator types is what is
//! opened, pinned and read back.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn brokkr() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["implement", "review", "done", "stop"],
  "initial": "implement",
  "terminal": ["done", "stop"],
  "rules": [
    {"id": "IMPL-OK", "from": "implement", "result": "complete", "next": "review",
     "reason": "Implementation complete."},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
     "reason": "Clean review; done."}
  ]
}"#;

struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    /// A workspace with a one-phase bundle, a git realm, and — when
    /// asked — a map at its root naming that realm and a journal that is
    /// NOT the default one, so "which journal was opened" is answerable.
    fn new(map: Option<Value>) -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        let bundle = ws.path().join("bundle");
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::create_dir_all(ws.path().join("state")).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        std::fs::write(bundle.join("roles/role.md"), "# role\n").unwrap();
        let script = ws.path().join("script.json");
        std::fs::write(
            &script,
            json!({"seats": {
                "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
                "review": [{"behavior": "succeed", "result": {"result": "clean"}}],
            }})
            .to_string(),
        )
        .unwrap();
        let seat = |results: Value| {
            json!({
                "role": "roles/role.md",
                "results": results,
                "driver": {"command": [
                    brokkr(), "fake-driver",
                    "--script", script.to_string_lossy(),
                    "--state", ws.path().join("state").to_string_lossy(),
                ]},
            })
        };
        std::fs::write(
            bundle.join("bundle.json"),
            json!({
                "name": "mapped",
                "policy": "policy.json",
                "seats": {
                    "implement": seat(json!(["complete"])),
                    "review": seat(json!(["clean"])),
                },
            })
            .to_string(),
        )
        .unwrap();
        if let Some(map) = map {
            std::fs::write(ws.path().join("realms.json"), map.to_string()).unwrap();
        }
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// Every invocation runs FROM the workspace, so `./realms.json` is
    /// discovered exactly as an operator standing there would find it.
    fn run(&self, args: &[&str]) -> (Option<i32>, String, String) {
        let out = Command::new(brokkr())
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn brokkr_run(&self, extra: &[&str]) -> (Option<i32>, String, String) {
        let mut args = vec!["run", "--bundle", "bundle", "--feature", "mapped feature"];
        args.extend_from_slice(extra);
        let (code, _, stderr) = self.run(&args);
        let run_id = stderr
            .lines()
            .find_map(|line| line.strip_prefix("run started: "))
            .unwrap_or_default()
            .trim()
            .to_string();
        (code, run_id, stderr)
    }
}

/// The map this workspace carries: one realm, this tree, and a journal
/// that is deliberately not `.forge/forge.db`.
fn map_over(realm_path: &str) -> Value {
    json!({
        "schema": "forge.realms/v1",
        "realms": [{"name": "brokkr", "path": realm_path, "default_branch": "main"}],
        "journal": "state/world.db",
    })
}

fn git(repo: &Path, args: &[&str]) {
    assert!(Command::new("git")
        .args(args)
        .current_dir(repo)
        .status()
        .unwrap()
        .success());
}

fn git_repo(repo: &Path) -> String {
    git(repo, &["init", "-q"]);
    git(repo, &["config", "user.name", "Brokkr Test"]);
    git(repo, &["config", "user.email", "brokkr@test"]);
    git(repo, &["config", "commit.gpgSign", "false"]);
    std::fs::write(repo.join("file.txt"), "content").unwrap();
    git(repo, &["add", "."]);
    git(repo, &["commit", "-q", "-m", "first"]);
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .unwrap();
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// `brokkr realms` lists the world: every realm with its path, default
/// branch and current HEAD, and the journal the world writes.
#[test]
fn the_realms_verb_lists_the_world_and_writes_nothing() {
    let ws = Workspace::new(Some(map_over("realm")));
    std::fs::create_dir(ws.path().join("realm")).unwrap();
    let head = git_repo(&ws.path().join("realm"));

    let (code, out, stderr) = ws.run(&["realms"]);
    assert_eq!(code, Some(0), "{stderr}");
    let lines: Vec<&str> = out.lines().collect();
    assert!(lines[0].starts_with("map      "), "{out}");
    assert!(lines[0].ends_with("realms.json"), "{out}");
    // Paths print platform-native; the pin normalizes for the compare.
    assert_eq!(lines[1].replace('\\', "/"), "journal  ./state/world.db");
    assert_eq!(lines[2], format!("realm    brokkr  realm  main  {head}"));
    assert_eq!(lines.len(), 3, "{out}");

    // `--json` is the same derivation, spelled for a script.
    let (code, json, stderr) = ws.run(&["realms", "--json"]);
    assert_eq!(code, Some(0), "{stderr}");
    let view: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        view["journal"].as_str().unwrap().replace('\\', "/"),
        "./state/world.db"
    );
    assert!(view["map"].as_str().unwrap().ends_with("realms.json"));
    // Every realm carries its effective journal (decision 0026 ruling
    // 1); this world has one hearth, so it is the world's own.
    let journal = view["realms"][0]["journal"]
        .as_str()
        .unwrap()
        .replace('\\', "/");
    assert_eq!(journal, "./state/world.db");
    assert_eq!(
        view["realms"],
        json!([{"name": "brokkr", "path": "realm", "default_branch": "main",
                "head": head, "journal": view["realms"][0]["journal"]}])
    );

    // A readout writes nothing: no journal was created by asking.
    assert!(!ws.path().join("state/world.db").exists());
    assert!(!ws.path().join(".forge").exists());

    // And the lore stays out of the machine's mouth (0019 law 4).
    assert!(!out.to_lowercase().contains("yggdrasil"), "{out}");
    assert!(!json.to_lowercase().contains("yggdrasil"), "{json}");
}

/// The whole point, end to end: the map names the journal, the run
/// writes there, and every read surface opens the same one — with no
/// flag but the map itself.
#[test]
fn the_map_chooses_the_journal_for_the_run_and_for_every_read_surface() {
    let ws = Workspace::new(Some(map_over(".")));
    let (code, run_id, stderr) = ws.brokkr_run(&[]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(
        ws.path().join("state/world.db").is_file(),
        "the map's journal"
    );
    assert!(
        !ws.path().join(".forge/forge.db").exists(),
        "and not the default one"
    );

    // A map found rather than typed still moved where the journal is,
    // and every surface says so once, on stderr, before opening it.
    let announced = |stderr: &str| {
        stderr
            .lines()
            .any(|line| line.starts_with("note: the journal is ") && line.contains("world.db"))
    };
    assert!(announced(&stderr), "{stderr}");

    let (code, listed, stderr) = ws.run(&["runs", "--json"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(listed.contains(&run_id), "{listed}");
    assert!(announced(&stderr), "{stderr}");

    for surface in [
        vec!["inspect", "--run", "latest", "--json"],
        vec!["export", "--run", "latest", "--out", "exported"],
    ] {
        let (code, _, stderr) = ws.run(&surface);
        assert_eq!(code, Some(0), "{surface:?}: {stderr}");
    }
    assert!(ws
        .path()
        .join(format!("exported/{run_id}.ndjson"))
        .is_file());

    // `--db` outranks the map's journal, and the fleet there is empty —
    // and the operator's own answer is not announced back to them.
    let (code, listed, stderr) = ws.run(&["runs", "--json", "--db", "state/other.db"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(!listed.contains(&run_id), "{listed}");
    assert!(!announced(&stderr), "{stderr}");
}

/// Many hearths (decision 0026 rulings 1, 3 and 5), through the shipped
/// binary: a v2 map whose realms name different journals lists its fleet
/// GROUPED by realm on both surfaces, a world with one hearth lists what
/// it always listed, and reading a world creates no journal in it.
#[test]
fn a_many_hearth_world_lists_its_fleet_grouped_by_realm() {
    let ws = Workspace::new(Some(map_over(".")));
    let (code, alpha_run, stderr) = ws.brokkr_run(&[]);
    assert_eq!(code, Some(0), "{stderr}");
    // A second run in another journal, named outright: `--db` outranks
    // the map for a fleet read exactly as it does for a single run.
    let (code, beta_run, stderr) = ws.brokkr_run(&["--db", "state/beta.db"]);
    assert_eq!(code, Some(0), "{stderr}");

    // The v1 map is ONE hearth: a flat listing with no realm heading in
    // it anywhere, and only the runs that journal holds.
    let (code, flat, stderr) = ws.run(&["runs"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(flat.contains(&alpha_run), "{flat}");
    assert!(
        !flat.contains(&beta_run),
        "another journal is another hearth: {flat}"
    );
    assert!(
        !flat.contains(" · 1 run · "),
        "no heading, as before: {flat}"
    );

    // The degenerate many-hearth case — a v2 realm naming the journal
    // the world already names — is still one hearth, and still flat.
    let mut degenerate = map_over(".");
    degenerate["schema"] = json!("forge.realms/v2");
    degenerate["realms"][0]["journal"] = json!("state/world.db");
    std::fs::write(ws.path().join("realms.json"), degenerate.to_string()).unwrap();
    let (code, same, stderr) = ws.run(&["runs"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(same.contains(&alpha_run), "{same}");
    assert!(!same.contains(" · 1 run · "), "{same}");

    // Two hearths: one section per realm, each under its own heading.
    let many = json!({
        "schema": "forge.realms/v2",
        "realms": [
            {"name": "alpha", "path": ".", "default_branch": "main"},
            {"name": "beta", "path": ".", "default_branch": "main",
             "journal": "state/beta.db"},
        ],
        "journal": "state/world.db",
    });
    std::fs::write(ws.path().join("realms.json"), many.to_string()).unwrap();
    let (code, grouped, stderr) = ws.run(&["runs"]);
    assert_eq!(code, Some(0), "{stderr}");
    let lines: Vec<&str> = grouped.lines().collect();
    assert!(lines[0].starts_with("alpha · 1 run · "), "{grouped}");
    assert!(lines[1].starts_with(&alpha_run), "{grouped}");
    assert_eq!(lines[2], "", "a blank line parts the hearths: {grouped}");
    assert!(lines[3].starts_with("beta · 1 run · "), "{grouped}");
    assert!(lines[4].starts_with(&beta_run), "{grouped}");

    // `--json` is the SAME grouping: one derivation, two renderings.
    let (code, listed, stderr) = ws.run(&["runs", "--json"]);
    assert_eq!(code, Some(0), "{stderr}");
    let view: Value = serde_json::from_str(&listed).unwrap();
    assert_eq!(view["count"], json!(2));
    assert_eq!(view["realms"][0]["realm"], json!("alpha"));
    assert_eq!(view["realms"][0]["runs"][0]["run_id"], json!(alpha_run));
    assert_eq!(view["realms"][1]["realm"], json!("beta"));
    assert_eq!(view["realms"][1]["runs"][0]["run_id"], json!(beta_run));

    // Ruling 5: a fleet read writes to no journal it reads, and creates
    // none — a hearth whose journal is not there yet says so instead.
    let ghostly = json!({
        "schema": "forge.realms/v2",
        "realms": [
            {"name": "alpha", "path": ".", "default_branch": "main"},
            {"name": "ghost", "path": ".", "default_branch": "main",
             "journal": "state/ghost.db"},
        ],
        "journal": "state/world.db",
    });
    std::fs::write(ws.path().join("realms.json"), ghostly.to_string()).unwrap();
    let before = std::fs::read(ws.path().join("state/world.db")).unwrap();
    let (code, listed, stderr) = ws.run(&["runs"]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(listed.contains("ghost · 0 runs · "), "{listed}");
    assert!(
        !ws.path().join("state/ghost.db").exists(),
        "a read created a journal"
    );
    assert_eq!(
        std::fs::read(ws.path().join("state/world.db")).unwrap(),
        before,
        "a fleet read moved a byte of a journal it read"
    );

    // And `brokkr realms` names each realm's own hearth.
    let (code, world, stderr) = ws.run(&["realms", "--json"]);
    assert_eq!(code, Some(0), "{stderr}");
    let world: Value = serde_json::from_str(&world).unwrap();
    assert!(world["realms"][0]["journal"]
        .as_str()
        .unwrap()
        .ends_with("world.db"));
    assert!(world["realms"][1]["journal"]
        .as_str()
        .unwrap()
        .ends_with("ghost.db"));
}

/// Pinned AND embedded: the exported manifest carries the map's content
/// hash and the map itself, so the world a run believed in survives the
/// file it was read from.
#[test]
fn the_run_manifest_pins_the_maps_hash_and_embeds_the_map() {
    let ws = Workspace::new(Some(map_over(".")));
    let (code, run_id, stderr) = ws.brokkr_run(&[]);
    assert_eq!(code, Some(0), "{stderr}");
    let (code, _, stderr) = ws.run(&["export", "--run", &run_id, "--out", "exported"]);
    assert_eq!(code, Some(0), "{stderr}");

    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(ws.path().join(format!("exported/{run_id}.manifest.json")))
            .unwrap(),
    )
    .unwrap();
    let pin = &manifest["realms"];
    assert_eq!(pin["map"], map_over("."));
    assert_eq!(
        pin["source"].as_str().unwrap().replace('\\', "/"),
        "./realms.json"
    );
    assert_eq!(
        pin["sha256"].as_str().unwrap(),
        brokkr_core::canonical::sha256_hex(&map_over("."))
    );

    // The map is workspace data, not bundle data: the bundle's own
    // pinned files are untouched by adopting one.
    let (code, compiled, _) = ws.run(&["compile", "--bundle", "bundle"]);
    assert_eq!(code, Some(0));
    let compiled: Value = serde_json::from_str(&compiled).unwrap();
    assert_eq!(compiled["manifest"]["files"], manifest["files"]);
    assert!(compiled["manifest"].get("realms").is_none());

    // And the run stays resumable under that exact bundle.
    let (code, _, stderr) = ws.run(&[
        "resume",
        "--bundle",
        "bundle",
        "--run",
        &run_id,
        "--db",
        "state/world.db",
    ]);
    assert_eq!(code, Some(0), "{stderr}");
}

/// A world that never drew a map notices nothing: same default journal,
/// same manifest, no realm key anywhere in the journal.
#[test]
fn a_workspace_with_no_map_runs_exactly_as_it_always_did() {
    let ws = Workspace::new(None);
    let (code, run_id, stderr) = ws.brokkr_run(&[]);
    assert_eq!(code, Some(0), "{stderr}");
    assert!(ws.path().join(".forge/forge.db").is_file());
    ws.run(&["export", "--run", &run_id, "--out", "exported"]);
    let journal =
        std::fs::read_to_string(ws.path().join(format!("exported/{run_id}.ndjson"))).unwrap();
    for key in ["\"realms\"", "\"realm_facts\""] {
        assert!(!journal.contains(key), "an unmapped run journaled {key}");
    }
}

/// A map named and missing, or present and malformed, is a refusal
/// before any seat spawns — never a silent fallback to the default
/// world. The proof is that nothing was written at all.
#[test]
fn a_missing_or_malformed_map_refuses_before_any_seat_spawns() {
    let ws = Workspace::new(None);
    let (code, run_id, _) = ws.brokkr_run(&["--realms", "clientx.json"]);
    assert_eq!(code, Some(1));
    assert!(run_id.is_empty(), "no run was started");
    assert!(!ws.path().join(".forge").exists(), "no journal was opened");
    let (_, _, stderr) = ws.run(&[
        "run",
        "--bundle",
        "bundle",
        "--feature",
        "f",
        "--realms",
        "clientx.json",
    ]);
    assert!(stderr.contains("no realms map at"), "{stderr}");

    // The same refusal for the map this workspace carries, unnamed.
    std::fs::write(ws.path().join("realms.json"), "{ not a map").unwrap();
    let (code, _, stderr) = ws.run(&["runs", "--json"]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("not a readable realms map"), "{stderr}");
    assert!(stderr.contains("realms.json"), "{stderr}");

    // Including a map that names a version this build does not read:
    // an addition is a version, not drift inside one already published.
    let mut future = map_over(".");
    future["schema"] = json!("forge.realms/v3");
    std::fs::write(ws.path().join("realms.json"), future.to_string()).unwrap();
    let (code, _, stderr) = ws.run(&["realms"]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("forge.realms/v3"), "{stderr}");

    // And a v1 map reaching for v2's one new word: the version is the
    // promise, so the word is refused under the label that forbids it.
    let mut drifted = map_over(".");
    drifted["realms"][0]["journal"] = json!("other.db");
    std::fs::write(ws.path().join("realms.json"), drifted.to_string()).unwrap();
    let (code, _, stderr) = ws.run(&["realms"]);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("names its own journal"), "{stderr}");
}

/// This repository carries its own map (ruling 1), and it is the
/// bootstrap world: one realm, this repository, the journal this
/// repository has always written.
#[test]
fn the_repository_carries_the_bootstrap_map() {
    let root: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    let map: Value =
        serde_json::from_str(&std::fs::read_to_string(root.join("realms.json")).unwrap()).unwrap();
    assert_eq!(
        map,
        json!({
            "schema": "forge.realms/v1",
            "realms": [{"name": "brokkr", "path": ".", "default_branch": "main"}],
            "journal": ".forge/forge.db",
        })
    );
    let world = brokkr_runtime::World::load(&root.join("realms.json")).unwrap();
    assert_eq!(world.journal(), root.join(".forge/forge.db"));
    assert_eq!(world.realm_for(&root).unwrap().name, "brokkr");
}
