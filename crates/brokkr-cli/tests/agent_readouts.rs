//! Provenance in every readout (decision 0016; spec AC-8, AC-17, AC-18).
//!
//! A run whose second attempt fell back to a different model must be
//! legible as exactly that, everywhere. These tests drive a real
//! fallback run and then read it back through `brokkr inspect`, the TUI's
//! model, the console's `/api/view` payload and `brokkr compare` — and
//! assert that none of them composes the sentence itself, because a
//! surface that formats provenance on its own is a surface that can stop
//! mentioning a fallback without anyone noticing.

use std::path::{Path, PathBuf};
use std::process::Command;

use serde_json::{json, Value};

fn brokkr_bin() -> &'static str {
    env!("CARGO_BIN_EXE_brokkr")
}

const POLICY: &str = r#"{
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
    /// A workspace whose agent's first model is served by a provider
    /// that is not installed, so attempt one fails to start and attempt
    /// two falls back — the run every readout below is asked to explain.
    fn new(models: Value) -> Workspace {
        let ws = Workspace {
            dir: tempfile::tempdir().unwrap(),
        };
        for sub in ["bundle", "agents/charters", "adapters", "state"] {
            std::fs::create_dir_all(ws.path().join(sub)).unwrap();
        }
        std::fs::write(ws.path().join("bundle/policy.json"), POLICY).unwrap();
        std::fs::write(ws.path().join("agents/charters/work.md"), "# work\n").unwrap();
        ws.write(
            "adapters/absent.json",
            json!({
                "provider": "absent",
                "binary": "brokkr-absent-driver",
                "driver": ["brokkr-absent-driver-that-is-not-installed"],
                "models": {"first": "absent/first"},
                "model_flag": "--model",
                "tool_permissions": "unsupported",
                "mcp": "unsupported",
            }),
        );
        for (provider, model) in [("fake", "second"), ("other", "third")] {
            ws.write(
                &format!("adapters/{provider}.json"),
                json!({
                    "provider": provider,
                    "binary": brokkr_bin(),
                    "driver": [
                        brokkr_bin(), "fake-driver",
                        "--script", ws.path().join("script.json").to_string_lossy(),
                        "--state", ws.path().join("state").to_string_lossy(),
                    ],
                    "models": {model: format!("{provider}/{model}")},
                    "model_flag": "--model",
                    "tool_permissions": "unsupported",
                    "mcp": "unsupported",
                }),
            );
        }
        ws.write(
            "script.json",
            json!({"seats": {
                "implement": [{"behavior": "succeed", "result": {"result": "complete"}}],
                "review": [{"behavior": "succeed", "result": {"result": "clean"}}],
            }}),
        );
        ws.write(
            "agents/worker.json",
            json!({
                "description": "the worker",
                "charter": "charters/work.md",
                "models": models,
                "limits": {"max_attempts": 2, "timeout_seconds": 60},
            }),
        );
        ws.write(
            "bundle/bundle.json",
            json!({
                "name": "readouts",
                "policy": "policy.json",
                "seats": {
                    "implement": {"results": ["complete"], "agent": "worker"},
                    "review": {
                        "results": ["clean"],
                        "role": "../agents/charters/work.md",
                        "driver": {"command": [
                            brokkr_bin(), "fake-driver",
                            "--script", ws.path().join("script.json").to_string_lossy(),
                            "--state", ws.path().join("state").to_string_lossy(),
                        ]},
                    },
                },
            }),
        );
        ws
    }

    fn path(&self) -> &Path {
        self.dir.path()
    }

    fn db(&self) -> PathBuf {
        self.path().join("forge.db")
    }

    fn write(&self, relative: &str, body: Value) {
        std::fs::write(
            self.path().join(relative),
            serde_json::to_vec_pretty(&body).unwrap(),
        )
        .unwrap();
    }

    fn brokkr(&self, args: &[&str]) -> (String, String) {
        let out = Command::new(brokkr_bin())
            .args(args)
            .current_dir(self.path())
            .output()
            .unwrap();
        (
            String::from_utf8_lossy(&out.stdout).into_owned(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }

    fn run(&self) -> String {
        let (_, stderr) = self.brokkr(&[
            "run",
            "--bundle",
            self.path().join("bundle").to_str().unwrap(),
            "--feature",
            "readouts",
            "--db",
            self.db().to_str().unwrap(),
        ]);
        stderr
            .lines()
            .find_map(|line| line.strip_prefix("run started: "))
            .unwrap_or_else(|| panic!("run id on stderr: {stderr}"))
            .trim()
            .to_string()
    }

    fn inspect_json(&self, run_id: &str) -> Value {
        let (stdout, _) = self.brokkr(&[
            "inspect",
            "--run",
            run_id,
            "--json",
            "--db",
            self.db().to_str().unwrap(),
        ]);
        serde_json::from_str(&stdout).unwrap()
    }

    fn inspect_text(&self, run_id: &str) -> String {
        self.brokkr(&[
            "inspect",
            "--run",
            run_id,
            "--db",
            self.db().to_str().unwrap(),
        ])
        .0
    }
}

/// AC-8 and AC-17: the fallback and its provenance reach the JSON view
/// model AND the human readout. A notice that only lands in JSON nobody
/// reads is the ruling's "never nothing" defeated.
#[test]
fn inspect_shows_the_fallback_in_json_and_in_prose() {
    let ws = Workspace::new(json!(["first", "second"]));
    let run_id = ws.run();

    let view = ws.inspect_json(&run_id);
    assert_eq!(view["view_version"], 4);
    let seat = view["participants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["label"] == "implement")
        .expect("the implement participant");
    assert_eq!(seat["provenance"]["agent"], "worker");
    assert_eq!(seat["provenance"]["model"], "second");
    assert_eq!(seat["provenance"]["provider"], "fake");
    assert_eq!(seat["provenance"]["chain_index"], 1);
    assert_eq!(seat["provenance"]["fallback"], json!(true));
    assert_eq!(seat["model"]["text"], "fake/second");

    let notices = view["notices"].as_array().unwrap();
    assert_eq!(notices.len(), 1);
    assert_eq!(notices[0]["kind"], "fallback");

    let text = ws.inspect_text(&run_id);
    assert!(text.contains("note  fallback"), "{text}");
    assert!(text.contains("worker · selected second via fake"), "{text}");
    assert!(text.contains("not the agent's first choice"), "{text}");

    // The inline review seat claims nothing: the journal carries no
    // model for it, and the readout invents none.
    let review = view["participants"]
        .as_array()
        .unwrap()
        .iter()
        .find(|part| part["label"] == "review")
        .expect("the review participant");
    assert!(review["provenance"].is_null());
}

/// A run that got its first choice says so plainly, with no notice and
/// no fallback language anywhere in the readout.
#[test]
fn a_first_choice_run_carries_no_notice_at_all() {
    let ws = Workspace::new(json!(["second"]));
    let run_id = ws.run();
    let view = ws.inspect_json(&run_id);
    assert!(view["notices"].as_array().unwrap().is_empty());
    let text = ws.inspect_text(&run_id);
    assert!(!text.contains("note  fallback"), "{text}");
    assert!(text.contains("worker · selected second via fake"), "{text}");
}

/// AC-18: `brokkr compare` reports a model difference as a FIRST-CLASS
/// divergence — reported unconditionally, and reported even when the two
/// runs pin the same recipe, because comparing pinned plans would hide
/// precisely the fallback this exists to expose.
#[test]
fn compare_reports_a_resolution_divergence_even_when_the_recipe_matches() {
    let ws = Workspace::new(json!(["first", "second"]));
    let fallen_back = ws.run();
    // The same bundle, same digest — but this time the first link is
    // reachable, so a different model actually ran.
    std::fs::remove_file(ws.path().join("adapters/absent.json")).unwrap();
    ws.write(
        "adapters/absent.json",
        json!({
            "provider": "absent",
            "binary": brokkr_bin(),
            "driver": [
                brokkr_bin(), "fake-driver",
                "--script", ws.path().join("script.json").to_string_lossy(),
                "--state", ws.path().join("state2").to_string_lossy(),
            ],
            "models": {"first": "absent/first"},
            "model_flag": "--model",
            "tool_permissions": "unsupported",
            "mcp": "unsupported",
        }),
    );
    let first_choice = ws.run();

    let (stdout, _) = ws.brokkr(&[
        "compare",
        &fallen_back,
        &first_choice,
        "--db",
        ws.db().to_str().unwrap(),
    ]);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    let divergence = &report["comparison"]["resolution_divergence"];
    assert_eq!(divergence["implement"]["a"]["model"], "fake/second");
    assert_eq!(divergence["implement"]["b"]["model"], "absent/first");
    assert_eq!(
        divergence["implement"]["a"]["selected"]["fallback"],
        json!(true)
    );
    assert_eq!(
        divergence["implement"]["b"]["selected"]["fallback"],
        json!(false)
    );
    // Each run's own section names what served it, too.
    assert_eq!(
        report["runs"][&fallen_back]["resolution"]["implement"]["selected"]["provider"],
        "fake"
    );

    // Two runs that resolved identically report an EMPTY divergence
    // rather than omitting the field: absence of a difference is itself
    // a reported fact.
    let (stdout, _) = ws.brokkr(&[
        "compare",
        &first_choice,
        &first_choice,
        "--db",
        ws.db().to_str().unwrap(),
    ]);
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(
        report["comparison"]["resolution_divergence"],
        json!({}),
        "the field is unconditional"
    );
}

/// AC-8's anti-drift half: the human sentence is composed in exactly one
/// place. No surface may build "… via …" or "fallback #…" itself, or a
/// readout could quietly stop saying the second choice was a second
/// choice.
#[test]
fn no_surface_composes_the_provenance_sentence_itself() {
    let crates_dir = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let derivation = crates_dir.join("brokkr-view/src/lib.rs");
    let source = std::fs::read_to_string(&derivation).unwrap();
    assert!(
        source.contains("via {provider}"),
        "the derivation composes it"
    );

    for surface in [
        "brokkr-cli/src/render.rs",
        "brokkr-cli/src/tui.rs",
        "brokkr-cli/src/ui.html",
        "brokkr-cli/src/compare.rs",
    ] {
        let body = std::fs::read_to_string(crates_dir.join(surface)).unwrap();
        for fragment in ["via {provider}", "fallback #", "first choice"] {
            assert!(
                !body.contains(fragment),
                "{surface} composes '{fragment}' itself; the sentence belongs \
                 to the single brokkr-view derivation"
            );
        }
    }
}

/// AC-10 through the binary: `brokkr agents list` and
/// `brokkr agents show` resolve their library roots exactly as
/// `--recipes-dir` does, and `show` prints a machine-readable object
/// without a `--json` flag.
#[test]
fn the_agents_verbs_run_against_the_default_roots() {
    let ws = Workspace::new(json!(["second"]));
    let (stdout, _) = ws.brokkr(&["agents", "list"]);
    assert!(stdout.starts_with("worker\tsecond\tthe worker"), "{stdout}");

    let (stdout, _) = ws.brokkr(&["agents", "show", "worker"]);
    let shown: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(shown["name"], "worker");
    assert_eq!(shown["resolution"]["chosen"]["provider"], "fake");
    assert_eq!(shown["resolution"]["chain"][0]["status"], "ok");

    // Explicit roots, and the refusal an unknown name earns.
    let (stdout, _) = ws.brokkr(&[
        "agents",
        "list",
        "--agents-dir",
        ws.path().join("agents").to_str().unwrap(),
    ]);
    assert!(stdout.contains("worker"), "{stdout}");
    let (_, stderr) = ws.brokkr(&[
        "agents",
        "show",
        "nobody",
        "--agents-dir",
        ws.path().join("agents").to_str().unwrap(),
        "--adapters-dir",
        ws.path().join("adapters").to_str().unwrap(),
    ]);
    assert!(stderr.contains("is not in the library"), "{stderr}");
}
