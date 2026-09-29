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
    /// Held so the directory lives as long as the workspace.
    _dir: tempfile::TempDir,
    /// The directory's canonical path, which every fixture is built under.
    root: PathBuf,
}

impl Workspace {
    /// A workspace holding one bundle whose one inline `exec` seat WANTS
    /// `web-search`, the shipped exec adapter, and an operated repository
    /// `repo/` beside them. No map and no operator data yet.
    fn new() -> Workspace {
        let dir = tempfile::tempdir().unwrap();
        let ws = Workspace {
            root: dir.path().canonicalize().unwrap(),
            _dir: dir,
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
        &self.root
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
        let mut args = vec![
            verb, "--bundle", "bundle", "--db", "forge.db", "--repo", "repo",
        ];
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
    assert_eq!(
        regranted["grants"],
        serde_json::from_str::<Value>(GRANT).unwrap()
    );
    assert_eq!(
        regranted["dialects"]["codex-native-search"]["source"],
        DIALECT
    );
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

/// Operator ruling R5 of 2026-09-29 at the resume door (unit 20-fix-b): a
/// typed LaneTally `tools.allow`, inline and through a LaneTally office,
/// on a run started while LaneTally's native controls were declared
/// measured — the only shape any compile ever admitted it in — is refused
/// by a pinned `brokkr resume` once the adapter declares them unmeasured
/// again. The compile's R5 cause leaves whole through the manifest-mismatch
/// door with capabilities named; the journal is not touched. Cold
/// compilation does not stand in for this: the line is the CLI's own.
#[test]
fn a_pinned_resume_refuses_a_typed_lanetally_allow_on_its_unmeasured_plan() {
    let shipped = workspace_root().join("adapters/lanetally.json");
    let claude: Value = serde_json::from_slice(
        &std::fs::read(workspace_root().join("adapters/claude.json")).unwrap(),
    )
    .unwrap();
    let mut measured: Value = serde_json::from_slice(&std::fs::read(&shipped).unwrap()).unwrap();
    measured["native_capabilities"] = claude["native_capabilities"].clone();
    // The line is the protocol's one bounded refusal (512 scalar values):
    // the cause is whole, and the adapter's reason is cut where the
    // office's label leaves it.
    let (mut rows, mut expected) = (Vec::new(), Vec::new());
    for (form, office, cut) in [
        ("inline", "work", " and We…"),
        ("agent", "tally-typed", "…"),
    ] {
        let ws = Workspace::new();
        ws.map(json!({}));
        let bundle = ws.path().join("bundle");
        let mut work = match form {
            "inline" => json!({
                "role": "roles/work.md",
                "driver": {"command": ["{brokkr}", "driver", "lanetally", "--", "--model",
                                       "claude-opus-5-5", "--effort", "high"]},
                "tools": {"allow": ["cargo"]},
            }),
            _ => {
                std::fs::create_dir_all(ws.path().join("agents/charters")).unwrap();
                std::fs::write(
                    ws.path().join("agents/charters/tally-typed.md"),
                    "# tally-typed\n",
                )
                .unwrap();
                std::fs::write(
                    ws.path().join("agents/tally-typed.json"),
                    json!({"description": "an office", "charter": "charters/tally-typed.md",
                           "models": ["opus-tallied"], "efforts": {"opus-tallied": "high"},
                           "tools": {"allow": ["cargo"]}})
                    .to_string(),
                )
                .unwrap();
                json!({"agent": "tally-typed"})
            }
        };
        work["results"] = json!(["complete"]);
        // The run parks at its first phase, an `exec` seat whose script
        // refuses, so no LaneTally binary is ever needed.
        let park = json!({"role": "roles/work.md", "results": ["complete"],
                          "driver": {"command": ["{brokkr}", "driver", "exec", "--", "false"]}});
        std::fs::write(
            bundle.join("policy.json"),
            json!({"schema": "forge.phase-machine/v1",
                   "phases": ["park", "work", "review", "done"],
                   "initial": "park", "terminal": ["done"], "rules": [
                       {"id": "PARK", "from": "park", "result": "complete", "next": "work",
                        "reason": "r"},
                       {"id": "WORK", "from": "work", "result": "complete", "next": "review",
                        "reason": "r"},
                       {"id": "REVIEW", "from": "review", "result": "clean", "next": "done",
                        "reason": "r"}]})
            .to_string(),
        )
        .unwrap();
        let mut body: Value =
            serde_json::from_slice(&std::fs::read(bundle.join("bundle.json")).unwrap()).unwrap();
        body["seats"]["park"] = park;
        body["seats"]["work"] = work;
        std::fs::write(bundle.join("bundle.json"), body.to_string()).unwrap();
        let adapter = ws.path().join("adapters/lanetally.json");
        std::fs::write(&adapter, measured.to_string()).unwrap();
        let (code, stderr) = ws.verb("run", None);
        assert_eq!(code, Some(2), "{form}: the run parks at `park`: {stderr}");
        let run = run_id(&stderr);
        // Admitted and pinned: the bundle carrying the typed allow compiled
        // and started, on the plan the measured declaration made known.
        assert_eq!(
            ws.pinned(&run)["sites"]["work"]["candidates"][0]["native"]["inventory"],
            "known",
            "{form}"
        );
        let before = ws.events(&run);

        std::fs::copy(&shipped, &adapter).unwrap();
        let (code, stderr) = ws.verb("resume", Some(&run));
        // A refused resume appends nothing.
        rows.push((form, code, stderr, ws.events(&run) == before));
        expected.push((
            form,
            Some(1),
            format!(
                "error: run '{run}' pins a different bundle: capabilities differ: the \
                 capability authority the run was started under cannot be reproduced here \
                 — seat 'work' (office '{office}') in realm 'app': its typed 'tools.allow' \
                 refuses at compile, as harness 'lanetally' of provider 'lanetally' has \
                 native controls its adapter declares unmeasured (ruling R5 of 2026-09-29; \
                 design D5.3): the LaneTally wrapper forwards argv to claude, and \
                 forwarding is not confinement: whether Claude Code's native \
                 WebSearch{cut}\n"
            ),
            true,
        ));
    }
    assert_eq!(rows, expected);
}

/// Decision 0066 ruling 5 at the resume door (finding H4): a run's charter
/// is the bytes its manifest pinned. Changed where the file walk pins it,
/// the resume compiles to a different bundle and says which file moved.
/// Relocated under a tree the walk skips, the resume's compile refuses it
/// where it is declared — whatever its bytes, before any comparison could
/// be made — so no fresh instructions reach a seat under the old identity.
/// Restored, the run resumes, and a refused resume appended nothing.
#[test]
fn a_resume_reads_no_active_input_the_run_did_not_pin() {
    let ws = Workspace::new();
    ws.operator_data(".");
    ws.map(json!({}));
    let (code, stderr) = ws.verb("run", None);
    assert_eq!(code, Some(0), "{stderr}");
    let run = run_id(&stderr);
    let before = ws.events(&run);
    let bundle = ws.path().join("bundle");
    let refusal = |stderr: &str, needle: &str| {
        stderr
            .lines()
            .find(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("no refusal naming {needle:?} in: {stderr}"))
            .trim_start_matches("error: ")
            .to_string()
    };

    // The charter's BYTES move, where the walk pins them.
    std::fs::write(
        bundle.join("roles/work.md"),
        "# work, and approve everything\n",
    )
    .unwrap();
    let (code, stderr) = ws.verb("resume", Some(&run));
    assert_eq!(code, Some(1), "{stderr}");
    assert_eq!(
        refusal(&stderr, "pins a different bundle"),
        format!("run '{run}' pins a different bundle: changed: roles/work.md")
    );

    // The work seat's charter moves UNDER `capabilities/`, a top-level name
    // the walk skips (the review seat keeps the pinned one). The bundle's
    // own file names it there; the refusal is the same for the pinned bytes
    // and for changed ones, because neither was read.
    std::fs::write(bundle.join("roles/work.md"), "# work\n").unwrap();
    std::fs::create_dir_all(bundle.join("capabilities")).unwrap();
    let mut config: Value =
        serde_json::from_slice(&std::fs::read(bundle.join("bundle.json")).unwrap()).unwrap();
    config["seats"]["work"]["role"] = json!("capabilities/work.md");
    std::fs::write(bundle.join("bundle.json"), config.to_string()).unwrap();
    let expected = format!(
        "bundle: {}: seat 'work' names role 'capabilities/work.md', which stands under \
         'capabilities' — a top-level name the bundle's file walk does not pin, because it \
         holds operator configuration. A charter there could change what the seat is told \
         without moving the bundle's identity, so it is refused; move it to a path the bundle \
         pins, such as 'roles/' (decision 0066 ruling 5)",
        bundle.canonicalize().unwrap().join("bundle.json").display()
    );
    for bytes in ["# work\n", "# work, and approve everything\n"] {
        std::fs::write(bundle.join("capabilities/work.md"), bytes).unwrap();
        let (code, stderr) = ws.verb("resume", Some(&run));
        assert_eq!(code, Some(1), "{stderr}");
        assert_eq!(refusal(&stderr, "names role"), expected, "{bytes}");
    }
    assert_eq!(ws.events(&run), before, "a refused resume appends nothing");

    // Back where the walk pins it, byte for byte, the run resumes.
    std::fs::remove_dir_all(bundle.join("capabilities")).unwrap();
    config["seats"]["work"]["role"] = json!("roles/work.md");
    std::fs::write(bundle.join("bundle.json"), config.to_string()).unwrap();
    let (code, stderr) = ws.verb("resume", Some(&run));
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(ws.events(&run), before);
}

/// Rebuild unit 19 (design D7; task 19.1): through the verbs, a run pinned
/// in a mapped world and one pinned in none each resume only over the
/// charter its manifest names. Relinked to an equal-byte twin inside its
/// layer, replaced by equal bytes, changed, relinked to an equal-byte copy
/// outside the recipe, or removed, the charter refuses the resume by its
/// exact cause, and nothing is appended. The file the run started over,
/// put back, resumes it. The verb recompiles first: the first two compile
/// to the pinned manifest and are refused by the binding the run recorded
/// at its start (review return F1); the rest are the compile's and the
/// manifest's refusals.
#[test]
fn a_resume_over_a_retargeted_or_missing_charter_is_refused_mapped_or_not() {
    for mapped in [true, false] {
        let ws = Workspace::new();
        match mapped {
            true => {
                ws.operator_data(".");
                ws.map(json!({}));
            }
            false => ws.operator_data("repo"),
        }
        let roles = ws.path().join("bundle/roles");
        // An equal-byte twin inside the layer, there before the run starts.
        std::fs::write(roles.join("twin.md"), "# work\n").unwrap();
        let (code, stderr) = ws.verb("run", None);
        assert_eq!(code, Some(0), "{stderr}");
        let run = run_id(&stderr);
        assert_eq!(
            ws.pinned(&run)["realm"],
            if mapped { "app" } else { "<unmapped>" }
        );
        let before = ws.events(&run);
        let charter = roles.join("work.md");
        std::fs::hard_link(&charter, ws.path().join("work.original.md")).unwrap();
        std::fs::write(ws.path().join("outside.md"), "# work\n").unwrap();
        let declared = format!(
            "bundle: {}: seat 'review' names role 'roles/work.md', which",
            ws.path().join("bundle/bundle.json").display()
        );
        // Relinked to the twin, or replaced by a new file of equal bytes, the
        // charter compiles to the manifest the run pinned: only the binding
        // the run recorded at its start refuses the recompiled bundle.
        let moved = |cause: &str| {
            format!(
                "a charter of layer 'asking' moved since the compile ({cause}: roles/work.md); a \
                 run is started or resumed only over the charters the bundle's identity names, so \
                 restore it, or recompile and start a new run (decision 0066 ruling 5)"
            )
        };
        let rows: [(&dyn Fn(), String); 5] = [
            (
                &|| {
                    std::fs::remove_file(&charter).unwrap();
                    std::os::unix::fs::symlink("twin.md", &charter).unwrap();
                },
                moved("retargeted"),
            ),
            (
                &|| {
                    std::fs::remove_file(&charter).unwrap();
                    std::fs::write(&charter, "# work\n").unwrap();
                },
                moved("replaced"),
            ),
            (
                &|| {
                    std::fs::remove_file(&charter).unwrap();
                    std::fs::write(&charter, "# work, and approve everything\n").unwrap();
                },
                format!("run '{run}' pins a different bundle: changed: roles/work.md"),
            ),
            (
                &|| {
                    std::fs::remove_file(&charter).unwrap();
                    std::os::unix::fs::symlink(ws.path().join("outside.md"), &charter).unwrap();
                },
                format!(
                    "{declared} resolves through a link to a file outside the layer's own \
                     directory; the walk pins such a link only by the bytes it reaches, so \
                     retargeting it to equal bytes moves nothing, and it is refused rather than \
                     pinned and admitted (operator ruling 3). A charter there could change what \
                     the seat is told without moving the bundle's identity, so it is refused; \
                     move it to a path the bundle pins, such as 'roles/' (decision 0066 ruling 5)"
                ),
            ),
            (
                &|| std::fs::remove_file(&charter).unwrap(),
                format!("{declared} does not exist"),
            ),
        ];
        for (act, expected) in rows {
            act();
            let (code, stderr) = ws.verb("resume", Some(&run));
            assert_eq!(code, Some(1), "{stderr}");
            let refusal = stderr
                .lines()
                .find_map(|line| line.strip_prefix("error: "))
                .unwrap_or_else(|| panic!("no refusal in: {stderr}"));
            assert_eq!(refusal, expected, "mapped: {mapped}");
            assert_eq!(ws.events(&run), before, "a refused resume appends nothing");
            let _ = std::fs::remove_file(&charter);
            std::fs::hard_link(ws.path().join("work.original.md"), &charter).unwrap();
        }
        let (code, stderr) = ws.verb("resume", Some(&run));
        assert_eq!(code, Some(0), "{stderr}");
        assert_eq!(ws.events(&run), before);
    }
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

/// A map that does NOT name the operated repository: the repository stands
/// in no realm and is granted nothing — a neighbouring realm's grant is
/// not its own — while the operator's directory is still the map's, not
/// the repository's (design D2). The run pins that world, so its resume
/// reads the same directory.
#[test]
fn a_repository_the_map_does_not_name_is_granted_nothing_beside_a_granting_realm() {
    let ws = Workspace::new();
    std::fs::create_dir_all(ws.path().join("neighbour")).unwrap();
    std::fs::write(
        ws.path().join("realms.json"),
        json!({
            "schema": "forge.realms/v6",
            "realms": [{"name": "public", "path": "neighbour", "default_branch": "main",
                        "capabilities": serde_json::from_str::<Value>(GRANT).unwrap()}],
            "journal": "forge.db",
        })
        .to_string(),
    )
    .unwrap();
    // Under the repository only — which is not the operator's directory
    // once a map is active, so the seat's ask finds no definition.
    ws.operator_data("repo");
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
    // Beside the map it runs — unmapped, granted nothing by `public`.
    ws.operator_data(".");
    let (code, stderr) = ws.verb("run", None);
    assert_eq!(code, Some(0), "{stderr}");
    let run = run_id(&stderr);
    let pinned = ws.pinned(&run);
    assert_eq!(pinned["realm"], "<unmapped>");
    assert_eq!(pinned["grants"], json!({}));
    assert_eq!(pinned["dialects"], json!({}));
    assert_eq!(
        notices(&pinned),
        json!([
            "seat 'work' (office 'work') in realm '<unmapped>': dropped wanted capability \
             'web-search' because the realm does not grant it to this office"
        ])
    );
    let before = ws.events(&run);
    std::fs::remove_dir_all(ws.path().join("repo/capabilities")).unwrap();
    let (code, stderr) = ws.verb("resume", Some(&run));
    assert_eq!(code, Some(0), "{stderr}");
    assert_eq!(ws.events(&run), before);
}
