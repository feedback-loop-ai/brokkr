//! Decision 0046 through the binary: the three verbs that start a run
//! refuse a boundary this machine cannot build before any journal row is
//! written (ruling 2); `rerun` compiles in the discovered realm as `run`
//! does (ruling 1); `compile` prints the boundary under each hands site
//! (ruling 1); `doctor` reads the realm's word (ruling 2); and a run whose
//! exec gate stands under `harness` runs unboxed end to end, its record
//! carrying the plain word and its export verifying (rulings 3 and 4).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["verify", "review", "done", "stop"],
  "initial": "verify",
  "terminal": ["done", "stop"],
  "rules": [
    {"id": "VERIFY-PASS", "from": "verify", "result": "pass", "next": "review",
     "reason": "the gate passed"},
    {"id": "VERIFY-FAIL", "from": "verify", "result": "fail", "next": "stop",
     "severity": "hard", "reason": "the gate failed"},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
     "reason": "the review passed"}
  ]
}"#;

/// The gate script: reads the result path off the prompt by line, as the
/// shipped verify seat does, writes the result its second argument names,
/// and reports what its environment holds.
const GATE: &str = r#"#!/bin/sh
prompt_file="$1"
verdict="$2"
result_path=""
while IFS= read -r line; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    case "$trimmed" in /*.json|?:*.json) result_path="$trimmed" ;; esac
done < "$prompt_file"
[ -n "$result_path" ] || exit 2
printf '{"result":"%s","notes":"home=%s token=%s marker=%s"}\n' "$verdict" "$(printf '%s' "$HOME" | sed 's|\\|/|g')" "${GH_TOKEN:-unset}" "${BROKKR_HANDS_BOX:-unset}" > "$result_path"
"#;

fn gate_seat(verdicts: &[&str]) -> Value {
    json!({
        "role": "roles/gate.md",
        "results": verdicts,
        "class": "gate",
        "driver": {"command": [
            "{brokkr}", "driver", "exec", "--", "sh", "./scripts/gate.sh", "{prompt_file}", verdicts[0]
        ]},
        "hands": "workspace",
    })
}

struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    /// A workspace holding a bundle whose one seat is an exec gate with
    /// hands running the bundle's own `./scripts/gate.sh`, and a v4 map
    /// naming the workspace itself as a realm under `boundary`.
    fn new(boundary: &str) -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        let bundle = ws.bundle_dir();
        std::fs::create_dir_all(bundle.join("scripts")).unwrap();
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        std::fs::write(bundle.join("roles/gate.md"), "# gate\n").unwrap();
        std::fs::write(bundle.join("scripts/gate.sh"), GATE).unwrap();
        std::fs::write(
            bundle.join("bundle.json"),
            json!({
                "name": "gated",
                "policy": "policy.json",
                "seats": {
                    "verify": gate_seat(&["pass", "fail"]),
                    "review": gate_seat(&["clean"]),
                }
            })
            .to_string(),
        )
        .unwrap();
        // A gate seat opens the workspace adapters (decision 0021): the
        // shipped exec adapter is copied verbatim rather than invented.
        std::fs::create_dir_all(ws.path().join("adapters")).unwrap();
        std::fs::copy(
            workspace_root().join("adapters/exec.json"),
            ws.path().join("adapters/exec.json"),
        )
        .unwrap();
        ws.map(boundary);
        ws
    }

    fn map(&self, boundary: &str) {
        std::fs::write(
            self.path().join("realms.json"),
            json!({
                "schema": "forge.realms/v4",
                "realms": [{"name": "app", "path": ".", "default_branch": "main", "boundary": boundary}],
                "journal": "forge.db",
            })
            .to_string(),
        )
        .unwrap();
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn bundle_dir(&self) -> PathBuf {
        self.path().join("bundle")
    }

    fn db(&self) -> PathBuf {
        self.path().join("forge.db")
    }

    fn brokkr(&self, args: &[&str]) -> (Option<i32>, String, String) {
        let out = Command::new(brokkr_bin())
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

    fn run(&self) -> (Option<i32>, String, String) {
        let bundle = self.bundle_dir();
        let db = self.db();
        self.brokkr(&[
            "run",
            "--bundle",
            bundle.to_str().unwrap(),
            "--feature",
            "boundary proof",
            "--db",
            db.to_str().unwrap(),
        ])
    }

    fn events(&self, run_id: &str) -> Vec<Value> {
        let store = brokkr_store::Store::open(&self.db()).unwrap();
        store
            .load(run_id)
            .unwrap()
            .into_iter()
            .map(|event| {
                json!({
                    "type": event.event_type,
                    "payload": event.payload,
                })
            })
            .collect()
    }
}

fn run_id(stderr: &str) -> String {
    stderr
        .lines()
        .find_map(|line| {
            line.strip_prefix("run started: ")
                .or_else(|| line.split_once(" as ").map(|(_, rest)| rest))
        })
        .map(|rest| rest.split_whitespace().next().unwrap().to_string())
        .unwrap_or_else(|| panic!("no run id in stderr: {stderr}"))
}

#[test]
fn init_in_the_realm_runs_unboxed_gates_after_journal_results_and_source_writes() {
    for boundary in ["harness", "open"] {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        let (code, _, stderr) = ws.brokkr(&["init", "."]);
        assert_eq!(code, Some(0), "{stderr}");
        ws.map(boundary);
        std::fs::create_dir_all(ws.path().join("src")).unwrap();
        std::fs::write(ws.path().join("src/implementation.txt"), "before\n").unwrap();
        std::fs::write(ws.path().join("scripts/gate.sh"), GATE).unwrap();
        std::fs::write(
            ws.path().join("scripts/implement.sh"),
            format!(
                "printf 'after\\n' > src/implementation.txt\nprintf 'new\\n' > src/new.txt\n{GATE}"
            ),
        )
        .unwrap();
        std::fs::create_dir_all(ws.path().join("roles")).unwrap();
        std::fs::write(ws.path().join("roles/gate.md"), "# gate\n").unwrap();
        let mut policy: Value = serde_json::from_str(POLICY).unwrap();
        policy["phases"]
            .as_array_mut()
            .unwrap()
            .push(json!("implement"));
        policy["initial"] = json!("implement");
        policy["rules"].as_array_mut().unwrap().push(json!({
            "id": "IMPLEMENT-PASS", "from": "implement", "result": "pass",
            "next": "verify", "reason": "implementation concluded"
        }));
        std::fs::write(ws.path().join("policy.json"), policy.to_string()).unwrap();
        let mut implement = gate_seat(&["pass"]);
        implement["class"] = json!("work");
        implement["driver"]["command"][5] = json!("./scripts/implement.sh");
        std::fs::write(
            ws.path().join("bundle.json"),
            json!({"name": "init-in-place", "policy": "policy.json", "seats": {
                "implement": implement,
                "verify": gate_seat(&["pass", "fail"]),
                "review": gate_seat(&["clean"])
            }})
            .to_string(),
        )
        .unwrap();
        for args in [
            vec!["init", "--initial-branch=main"],
            vec![
                "-c",
                "user.name=Boundary Test",
                "-c",
                "user.email=boundary@example.invalid",
                "add",
                ".",
            ],
            vec![
                "-c",
                "user.name=Boundary Test",
                "-c",
                "user.email=boundary@example.invalid",
                "-c",
                "commit.gpgsign=false",
                "commit",
                "-m",
                "initial",
            ],
        ] {
            let output = Command::new("git")
                .args(args)
                .current_dir(ws.path())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        // An existing journal is inside the bundle at compile, and every
        // attempt writes another result beneath that same root.
        drop(brokkr_store::Store::open(&ws.db()).unwrap());
        let (code, _, stderr) = ws.brokkr(&[
            "run",
            "--bundle",
            ".",
            "--repo",
            ws.path().to_str().unwrap(),
            "--feature",
            "init in place",
        ]);
        let events = ws.events(&run_id(&stderr));
        assert_eq!(code, Some(0), "{boundary}: {stderr}\n{events:#?}");
        assert_eq!(events.last().unwrap()["type"], "run/completed");
        assert_eq!(
            events
                .iter()
                .filter(|event| event["type"] == "effect/succeeded")
                .count(),
            3
        );
        assert_eq!(
            std::fs::read_to_string(ws.path().join("src/implementation.txt")).unwrap(),
            "after\n"
        );
        assert!(ws.path().join("src/new.txt").is_file());
        assert!(
            std::fs::read_dir(ws.path().join(".forge/results"))
                .unwrap()
                .count()
                >= 3
        );
    }
}

#[test]
fn a_plain_compile_has_no_boundary_map_and_resume_keeps_its_pinned_namespace() {
    let ws = Workspace::new("namespace");
    let path = ws.bundle_dir().join("bundle.json");
    let mut config: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    for seat in config["seats"].as_object_mut().unwrap().values_mut() {
        seat.as_object_mut().unwrap().remove("hands");
        seat["class"] = json!("work");
    }
    std::fs::write(path, config.to_string()).unwrap();
    let (code, stdout, stderr) = ws.brokkr(&["compile", "--bundle", "bundle"]);
    assert_eq!(code, Some(0), "{stderr}");
    let compiled: Value = serde_json::from_str(&stdout).unwrap();
    assert!(compiled["manifest"].get("hands").is_none());
    assert!(compiled["manifest"].get("boundary").is_none());
    assert!(compiled.get("boundary").is_none());
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "{stderr}");
    let id = run_id(&stderr);
    let before = ws.events(&id);
    assert_eq!(
        before[0]["payload"]["manifest"]["realms"]["map"]["realms"][0]["boundary"],
        "namespace"
    );
    ws.map("harness");
    let (code, _, stderr) = ws.brokkr(&[
        "resume",
        "--run",
        &id,
        "--bundle",
        "bundle",
        "--db",
        ws.db().to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(ws.events(&id), before);
}

/// Ruling 2, the three verbs: a `seatbelt` realm refuses `run`, `resume`
/// and `rerun` naming the boundary, its slice and the seat, and no
/// journal row is written and no seat spawned.
#[test]
fn run_resume_and_rerun_refuse_an_unbuilt_boundary_before_the_journal() {
    let ws = Workspace::new("seatbelt");
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(1), "{stderr}");
    assert!(
        stderr.contains("`seatbelt` boundary is built by slice (ii)"),
        "{stderr}"
    );
    assert!(stderr.contains("[\"review\", \"verify\"]"), "{stderr}");
    assert!(!ws.db().exists(), "no journal was opened");

    // A run started under `harness` and pinned; then the realm moves to
    // `seatbelt` and both `resume` and `rerun` refuse the same way.
    ws.map("harness");
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "{stderr}");
    let first = run_id(&stderr);
    let bundle = ws.bundle_dir();
    let db = ws.db();
    let bundle_arg = bundle.to_str().unwrap();
    let db_arg = db.to_str().unwrap();
    let config_path = bundle.join("bundle.json");
    let config = std::fs::read(&config_path).unwrap();
    std::fs::write(&config_path, "{}").unwrap();
    let (code, _, stderr) = ws.brokkr(&[
        "rerun", "--run", &first, "--bundle", bundle_arg, "--db", db_arg,
    ]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(stderr.contains("bundle"), "{stderr}");
    assert_eq!(
        brokkr_store::Store::open(&db)
            .unwrap()
            .list_runs()
            .unwrap()
            .len(),
        1
    );
    std::fs::write(config_path, config).unwrap();
    ws.map("seatbelt");
    let (code, _, stderr) = ws.brokkr(&[
        "rerun", "--run", &first, "--bundle", bundle_arg, "--db", db_arg,
    ]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(
        stderr.contains("`seatbelt` boundary is built by slice (ii)"),
        "{stderr}"
    );
    let store = brokkr_store::Store::open(&db).unwrap();
    assert_eq!(
        store.list_runs().unwrap().len(),
        1,
        "the rerun wrote no run"
    );

    // `resume` reads the pinned world, never the workspace's map: a run
    // pinned under `seatbelt` by hand refuses at resume by the same rule.
    let world = brokkr_runtime::realms::World::discover(ws.path(), None)
        .unwrap()
        .unwrap();
    let compiled = brokkr_runtime::Bundle::compile_under(
        &bundle,
        &workspace_root().join("agents"),
        &workspace_root().join("adapters"),
        brokkr_core::realms::Boundary::Seatbelt,
    )
    .unwrap();
    let pinned = world.pinned(&compiled.manifest, Some(ws.path())).unwrap();
    let mut store = brokkr_store::Store::open(&db).unwrap();
    store
        .create_run("pinned-seatbelt", "boundary proof", "gated", &pinned)
        .unwrap();
    drop(store);
    ws.map("harness");
    let (code, _, stderr) = ws.brokkr(&[
        "resume",
        "--run",
        "pinned-seatbelt",
        "--bundle",
        bundle_arg,
        "--db",
        db_arg,
    ]);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(
        stderr.contains("`seatbelt` boundary is built by slice (ii)"),
        "{stderr}"
    );
    let store = brokkr_store::Store::open(&db).unwrap();
    assert!(
        store.load("pinned-seatbelt").unwrap().is_empty(),
        "resume wrote no row"
    );
}

/// Rulings 1, 3 and 4 end to end: under a `harness` realm the exec gate
/// runs unboxed — no box, a private home, no token in its environment —
/// `effect/started` carries the boundary entry, the finishing checkpoint
/// and the result carry the plain word, the export verifies and carries
/// no adjective, `rerun` compiles in the discovered realm and pins it,
/// and `compile` prints the boundary under the hands site.
#[test]
fn a_harness_realm_runs_its_exec_gate_unboxed_and_records_the_word() {
    let ws = Workspace::new("harness");
    let (code, _, stderr) = ws.run();
    assert_eq!(code, Some(0), "{stderr}");
    let id = run_id(&stderr);
    let events = ws.events(&id);
    let started: Vec<&Value> = events
        .iter()
        .filter(|event| event["type"] == "effect/started")
        .collect();
    assert_eq!(started.len(), 2, "verify then review");
    for event in &started {
        assert_eq!(
            event["payload"]["boundary"],
            json!([{"member": null, "boundary": "harness", "gate": true}])
        );
    }
    let manifest = &events[0]["payload"]["manifest"];
    assert_eq!(
        manifest["boundary"],
        json!({"review": "harness", "verify": "harness"})
    );
    assert_eq!(manifest["realms"]["realm"], "app");
    let finished = events
        .iter()
        .filter(|event| event["type"] == "effect/checkpointed")
        .map(|event| &event["payload"]["checkpoint"])
        .find(|checkpoint| checkpoint["step"] == "exec-session-finished")
        .expect("the exec driver's finishing checkpoint");
    assert_eq!(finished["boundary"], "harness");
    assert_eq!(finished["model"], "not applicable");
    let results: Vec<&Value> = events
        .iter()
        .filter(|event| event["type"] == "effect/succeeded")
        .map(|event| &event["payload"]["result"])
        .collect();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0]["result"], "pass");
    assert_eq!(results[1]["result"], "clean");
    let result = results[0];
    assert_eq!(result["boundary"], "harness");
    let notes = result["notes"].as_str().unwrap();
    assert!(notes.contains("token=unset"), "{notes}");
    assert!(
        notes.contains("marker=unset") || std::env::var_os("BROKKR_HANDS_BOX").is_some(),
        "{notes}"
    );
    let reported_home = notes
        .strip_prefix("home=")
        .unwrap()
        .split(" token=")
        .next()
        .unwrap();
    let expected_scratch = ws.path().join(".forge/scratch").canonicalize().unwrap();
    let reported_home = Path::new(reported_home).canonicalize().unwrap();
    assert!(reported_home.starts_with(expected_scratch), "{notes}");
    assert!(
        !notes.contains("/runtime/home"),
        "no box was built: {notes}"
    );
    let words: Vec<&str> = events
        .iter()
        .flat_map(|event| event["payload"]["checkpoint"]["boundary"].as_str())
        .collect();
    assert!(words.iter().all(|word| *word == "harness"), "{words:?}");

    // The export is the record itself: the plain word, verified, no
    // adjective anywhere in it.
    let out = ws.path().join("export");
    let db = ws.db();
    let (code, _, stderr) = ws.brokkr(&[
        "export",
        "--run",
        &id,
        "--out",
        out.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "{stderr}");
    let journal = out.join(format!("{id}.ndjson"));
    let text = std::fs::read_to_string(&journal).unwrap();
    assert!(text.contains("\"boundary\":\"harness\""), "{text}");
    assert!(!text.contains("unboxed"), "the data carries the plain word");
    let (code, stdout, stderr) = ws.brokkr(&["verify-run", journal.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{stderr}");
    let verified: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(verified["chain"], "verified");
    assert_eq!(verified["state"]["status"], "completed");

    // `rerun` compiles in the discovered realm as `run` does: the new
    // run's manifest carries the realms pin and the boundary map.
    let bundle = ws.bundle_dir();
    let (code, _, stderr) = ws.brokkr(&[
        "rerun",
        "--run",
        &id,
        "--bundle",
        bundle.to_str().unwrap(),
        "--db",
        db.to_str().unwrap(),
    ]);
    assert_eq!(code, Some(0), "{stderr}");
    let rerun = run_id(&stderr);
    assert_ne!(rerun, id);
    let events = ws.events(&rerun);
    assert_eq!(
        events[0]["payload"]["manifest"]["boundary"],
        json!({"review": "harness", "verify": "harness"})
    );
    assert_eq!(events[0]["payload"]["manifest"]["realms"]["realm"], "app");

    // `compile` prints the manifest, whose `boundary` map sits under
    // `hands` with the same keys — and a bundle boxing nothing prints
    // neither key.
    let (code, stdout, stderr) = ws.brokkr(&["compile", "--bundle", bundle.to_str().unwrap()]);
    assert_eq!(code, Some(0), "{stderr}");
    let view: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        view["manifest"]["boundary"],
        json!({"review": "harness", "verify": "harness"})
    );
    assert_eq!(
        view["manifest"]["hands"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>(),
        view["manifest"]["boundary"]
            .as_object()
            .unwrap()
            .keys()
            .collect::<Vec<_>>()
    );
    assert!(
        view.get("boundary").is_none(),
        "no second copy beside the manifest"
    );
    let root = workspace_root();
    let out = Command::new(brokkr_bin())
        .args(["compile", "--bundle", "bundles/self"])
        .current_dir(&root)
        .output()
        .unwrap();
    let own: Value = serde_json::from_slice(&out.stdout).unwrap();
    let boundary = own["manifest"]["boundary"].as_object().unwrap();
    let hands = own["manifest"]["hands"].as_object().unwrap();
    assert_eq!(
        boundary.keys().collect::<Vec<_>>(),
        hands.keys().collect::<Vec<_>>()
    );
    assert!(
        boundary.values().all(|word| word == "namespace"),
        "{boundary:?}"
    );
}

/// Ruling 2, `doctor` through the binary: in a `harness` realm with no
/// bubblewrap the `boundaries` line offers `harness` and `open`, and the
/// bundle's hands line stays healthy.
#[test]
fn doctor_reads_the_realms_boundary_from_the_workspace_map() {
    let ws = Workspace::new("harness");
    let empty = ws.path().join("empty-path");
    std::fs::create_dir_all(&empty).unwrap();
    let bundle = ws.bundle_dir();
    let out = Command::new(brokkr_bin())
        .args(["doctor", "--bundle", bundle.to_str().unwrap()])
        .env("PATH", &empty)
        .current_dir(ws.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(
            "boundaries: harness · open offered; namespace needs bwrap on PATH (not found)"
        ),
        "{stdout}"
    );
    assert!(
        stdout.contains(
            "ok       hands: seats [\"review\", \"verify\"] declare hands and can run under `harness`"
        ),
        "{stdout}"
    );
    ws.map("container");
    let out = Command::new(brokkr_bin())
        .args(["doctor", "--bundle", bundle.to_str().unwrap()])
        .env("PATH", &empty)
        .current_dir(ws.path())
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(
            "warn     hands: seats [\"review\", \"verify\"] declare hands and will refuse to \
             spawn: `container` is built by slice (iii)"
        ),
        "{stdout}"
    );
}
