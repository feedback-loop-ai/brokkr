//! Decision 0065 through the verbs (design D2 and D7): `run`, `rerun` and
//! `resume` authorise against the OPERATED realm, read the operator's
//! definitions and tool dialects beside the map — or, with no map, under
//! the operated repository — and a resume stands on what the run pinned,
//! never on what the workspace's map says today.
//!
//! Driven through the real binary. The seats are plain `exec` sites, whose
//! native inventory is unmeasured: they can hold nothing, so what these
//! tests read is the AUTHORITY each verb compiled under, which is the
//! thing a verb could get wrong. What a Codex seat is launched with under
//! that authority is `brokkr-runtime/tests/capability_launch.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

const POLICY: &str = r#"{
  "schema": "forge.phase-machine/v1",
  "phases": ["work", "review", "done"],
  "initial": "work",
  "terminal": ["done"],
  "rules": [
    {"id": "WORK-DONE", "from": "work", "result": "complete", "next": "review",
     "reason": "the work is done"},
    {"id": "REVIEW-CLEAN", "from": "review", "result": "clean", "next": "done",
     "reason": "the review passed"}
  ]
}"#;

/// Reads the result path off the prompt by line, as the shipped seats do.
const SEAT: &str = r#"#!/bin/sh
result_path=""
while IFS= read -r line; do
    trimmed="${line#"${line%%[![:space:]]*}"}"
    trimmed="${trimmed%"${trimmed##*[![:space:]]}"}"
    case "$trimmed" in /*.json) result_path="$trimmed" ;; esac
done < "$1"
[ -n "$result_path" ] || exit 2
printf '{"result":"%s","notes":"done"}\n' "$2" > "$result_path"
"#;

const DEFINITION: &str = "capabilities/web-search.json";
const DIALECT: &str = "dialects/tools/codex-native-search.json";

struct Workspace {
    dir: tempfile::TempDir,
}

impl Workspace {
    /// A workspace holding one bundle whose one inline `exec` seat WANTS
    /// `web-search`, the shipped exec adapter, and an operated repository
    /// `repo/` beside them. No map and no operator data yet.
    fn new() -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        let bundle = ws.path().join("bundle");
        std::fs::create_dir_all(bundle.join("scripts")).unwrap();
        std::fs::create_dir_all(bundle.join("roles")).unwrap();
        std::fs::create_dir_all(ws.path().join("repo")).unwrap();
        std::fs::write(bundle.join("policy.json"), POLICY).unwrap();
        std::fs::write(bundle.join("roles/work.md"), "# work\n").unwrap();
        std::fs::write(bundle.join("scripts/seat.sh"), SEAT).unwrap();
        let seat = |verdict: &str| {
            json!({
                "role": "roles/work.md",
                "results": [verdict],
                "driver": {"command": [
                    "{brokkr}", "driver", "exec", "--", "sh", "./scripts/seat.sh", "{prompt_file}",
                    verdict
                ]},
            })
        };
        let mut work = seat("complete");
        work["capabilities"] = json!({"web-search": "wants"});
        std::fs::write(
            bundle.join("bundle.json"),
            json!({
                "name": "asking",
                "policy": "policy.json",
                "seats": {"work": work, "review": seat("clean")},
            })
            .to_string(),
        )
        .unwrap();
        std::fs::create_dir_all(ws.path().join("adapters")).unwrap();
        std::fs::copy(
            workspace_root().join("adapters/exec.json"),
            ws.path().join("adapters/exec.json"),
        )
        .unwrap();
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    /// The shipped definition and Codex dialect, verbatim, under `root`.
    fn operator_data(&self, root: &str) {
        for shipped in [DEFINITION, DIALECT] {
            let to = self.path().join(root).join(shipped);
            std::fs::create_dir_all(to.parent().unwrap()).unwrap();
            std::fs::copy(workspace_root().join(shipped), to).unwrap();
        }
    }

    /// A v6 map naming `repo/` as realm `app`, granting `capabilities`.
    fn map(&self, capabilities: Value) {
        std::fs::write(
            self.path().join("realms.json"),
            json!({
                "schema": "forge.realms/v6",
                "realms": [{"name": "app", "path": "repo", "default_branch": "main",
                            "capabilities": capabilities}],
                "journal": "forge.db",
            })
            .to_string(),
        )
        .unwrap();
    }

    fn brokkr(&self, args: &[&str]) -> (Option<i32>, String) {
        let out = Command::new(env!("CARGO_BIN_EXE_brokkr"))
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    /// `run`, `rerun --run <id>` or `resume --run <id>` over `repo/`.
    fn verb(&self, verb: &str, run: Option<&str>) -> (Option<i32>, String) {
        let mut args = vec![verb, "--bundle", "bundle", "--db", "forge.db", "--repo", "repo"];
        match run {
            Some(run) => args.extend(["--run", run]),
            None => args.extend(["--feature", "capability proof"]),
        }
        self.brokkr(&args)
    }

    /// The `capabilities` section the run pinned at `run/started`.
    fn pinned(&self, run: &str) -> Value {
        self.events(run)[0]["manifest"]["capabilities"].clone()
    }

    fn events(&self, run: &str) -> Vec<Value> {
        brokkr_store::Store::open(&self.path().join("forge.db"))
            .unwrap()
            .load(run)
            .unwrap()
            .into_iter()
            .map(|event| event.payload)
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

const GRANT: &str = r#"{"web-search": {"dialect": "codex-native-search"}}"#;

fn notices(section: &Value) -> Value {
    section["sites"]["work"]["candidates"][0]["notices"].clone()
}

/// `run` and `rerun` authorise against the operated realm as the map
/// stands when the verb is given; `resume` stands on the run's own pin.
#[test]
fn run_and_rerun_read_todays_realm_and_resume_reads_the_runs_pin() {
    let ws = Workspace::new();
    ws.operator_data(".");
    ws.map(json!({}));
    let (code, stderr) = ws.verb("run", None);
    assert_eq!(code, Some(0), "{stderr}");
    let first = run_id(&stderr);
    let pinned = ws.pinned(&first);
    assert_eq!(pinned["realm"], "app");
    assert_eq!(pinned["grants"], json!({}));
    // The want was consulted, so its definition is pinned — by a source
    // relative to the operator's directory, never a host path.
    assert_eq!(pinned["definitions"]["web-search"]["source"], DEFINITION);
    assert_eq!(pinned["dialects"], json!({}));
    assert_eq!(
        notices(&pinned),
        json!([
            "seat 'work' (office 'work') in realm 'app': dropped wanted capability 'web-search' \
             because the realm does not grant it to this office"
        ])
    );
    let before = ws.events(&first);

    // A MAP-ONLY edit grants the capability. The resume reproduces the
    // pinned no-grant context — it compiles to the pinned manifest exactly,
    // which it could not do had it read a single grant off today's map.
    ws.map(serde_json::from_str(GRANT).unwrap());
    let (code, stderr) = ws.verb("resume", Some(&first));
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(ws.events(&first), before);

    // A rerun is a NEW run and stands where `run` stands: under the grant.
    // An `exec` site still holds nothing, and claims no denial.
    let (code, stderr) = ws.verb("rerun", Some(&first));
    assert_eq!(code, Some(0), "{stderr}");
    let second = run_id(&stderr);
    let regranted = ws.pinned(&second);
    assert_eq!(regranted["grants"], serde_json::from_str::<Value>(GRANT).unwrap());
    assert_eq!(regranted["dialects"]["codex-native-search"]["source"], DIALECT);
    assert_eq!(
        regranted["sites"]["work"]["candidates"][0]["held"],
        json!({})
    );
    assert_eq!(
        notices(&regranted),
        json!([
            "seat 'work' (office 'work') in realm 'app': dropped wanted capability 'web-search' \
             through dialect 'codex-native-search' because provider 'exec' cannot carry a \
             binding to provider 'codex'; no native denial is claimed"
        ])
    );
    // And the map going back to granting nothing does not disturb the
    // second run's resume either: each run stands on its own pin.
    ws.map(json!({}));
    let (code, stderr) = ws.verb("resume", Some(&second));
    assert_eq!(code, Some(0), "{stderr}");
}

/// A pinned input that can no longer be reproduced leaves through the
/// manifest-mismatch door with capabilities named — whether the bytes
/// moved or the file is gone — and the run's journal is not touched.
#[test]
fn a_resume_that_cannot_reproduce_its_pinned_inputs_is_a_capability_mismatch() {
    let ws = Workspace::new();
    ws.operator_data(".");
    ws.map(serde_json::from_str(GRANT).unwrap());
    let (code, stderr) = ws.verb("run", None);
    assert_eq!(code, Some(0), "{stderr}");
    let run = run_id(&stderr);
    let before = ws.events(&run);
    let refusal = |stderr: &str| {
        stderr
            .lines()
            .find(|line| line.contains("pins a different bundle"))
            .unwrap_or_else(|| panic!("no mismatch refusal in: {stderr}"))
            .trim_start_matches("error: ")
            .to_string()
    };

    // The definition's BYTES move: one trailing newline.
    let definition = ws.path().join(DEFINITION);
    let original = std::fs::read(&definition).unwrap();
    std::fs::write(&definition, [original.as_slice(), b"\n"].concat()).unwrap();
    let (code, stderr) = ws.verb("resume", Some(&run));
    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        refusal(&stderr),
        format!(
            "run '{run}' pins a different bundle: capabilities differ: the run's pinned \
             definitions no longer match what the bundle compiles to here — a grant, an \
             abstract definition, a tool dialect or an adapter's native declaration was added, \
             removed or edited since the run started"
        )
    );

    // The granted dialect is GONE: today that is a refusal by authority,
    // and it still leaves through the same door, naming capabilities.
    std::fs::write(&definition, &original).unwrap();
    std::fs::remove_file(ws.path().join(DIALECT)).unwrap();
    let (code, stderr) = ws.verb("resume", Some(&run));
    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        refusal(&stderr),
        format!(
            "run '{run}' pins a different bundle: capabilities differ: the capability \
             authority the run was started under cannot be reproduced here — realm \
             'app': capability 'web-search': tool dialect 'codex-native-search' is not at \
             '{DIALECT}' in the operator configuration; a grant, an abstract definition or a \
             tool dialect was removed or edited since the run started"
        )
    );
    assert_eq!(ws.events(&run), before, "a refused resume appends nothing");

    // Restored byte for byte, the run resumes.
    ws.operator_data(".");
    let (code, stderr) = ws.verb("resume", Some(&run));
    assert_eq!(code, Some(0), "{stderr}");
}

/// With no map the operator's directory is the OPERATED repository: what
/// `--repo` names, not the workspace the verb is given in and not the
/// recipe's home. An unmapped run keeps that root when it resumes, and a
/// map that appears in the workspace afterwards lends it nothing.
#[test]
fn an_unmapped_run_reads_and_keeps_the_operated_repository_as_its_root() {
    let ws = Workspace::new();
    // Beside the workspace and beside the recipe: neither is the root.
    ws.operator_data(".");
    ws.operator_data("bundle");
    let (code, stderr) = ws.verb("run", None);
    assert_eq!(code, Some(1), "{stderr}");
    assert!(
        stderr.contains(
            "bundle: seat 'work' (office 'work') in realm '<unmapped>': capability \
             'web-search' has no abstract definition at 'capabilities/web-search.json' in the \
             operator configuration; declare its classes before requesting it"
        ),
        "{stderr}"
    );
    assert!(
        !ws.path().join("forge.db").exists(),
        "a refused compile opens no journal"
    );

    std::fs::remove_dir_all(ws.path().join("bundle/capabilities")).unwrap();
    std::fs::remove_dir_all(ws.path().join("bundle/dialects")).unwrap();
    ws.operator_data("repo");
    let (code, stderr) = ws.verb("run", None);
    assert_eq!(code, Some(0), "{stderr}");
    let run = run_id(&stderr);
    let pinned = ws.pinned(&run);
    assert_eq!(pinned["realm"], "<unmapped>");
    assert_eq!(pinned["grants"], json!({}));
    assert_eq!(pinned["definitions"]["web-search"]["source"], DEFINITION);
    let before = ws.events(&run);

    // The workspace's decoy goes, so that only `repo/` can reproduce the
    // pin; and a map appears, naming the repository and granting the
    // capability. The run pinned no world: it resumes unmapped, under
    // `repo/`.
    std::fs::remove_dir_all(ws.path().join("capabilities")).unwrap();
    ws.map(serde_json::from_str(GRANT).unwrap());
    let (code, stderr) = ws.verb("resume", Some(&run));
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(ws.events(&run), before);
}
